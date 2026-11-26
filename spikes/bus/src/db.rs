//! Adattatore del database (SQLite o Postgres) usato dal proiettore, con il contatore delle istruzioni.
//!
//! Ogni gruppo di eventi è una transazione: righe tipizzate e posizione applicata insieme.
//! ponytail: i valori entrano nel testo SQL come numeri e letterali esadecimali costruiti da tipi
//! interni (mai da testo esterno), così lo stesso codice serve i due dialetti senza segnaposto;
//! in produzione sono parametri legati.

use crate::bus::Res;
use crate::crypto::{Uid, hex};
use crate::ev::{EXEC_STEP, Ev, Exec, Hash, Loc};
use sqlx::{AnyPool, Executor, Row, any::AnyPoolOptions};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

tokio::task_local! {
    /// Indice del tenant il cui proiettore sta applicando (solo per contare le istruzioni per tenant).
    pub static TENANT: usize;
}

pub struct Db {
    pool: AnyPool,
    pg: bool,
    /// Istruzioni SQL inviate (BEGIN e COMMIT compresi).
    pub stmts: AtomicU64,
    /// Blocco D: istruzioni per tenant (il proiettore di un tenant imposta `TENANT` per la sua `apply`).
    pub stmts_t: [AtomicU64; 2],
    /// Righe toccate dalle istruzioni di scrittura.
    pub rows: AtomicU64,
    /// Generazione del bus corrente, scritta con ogni posizione applicata (fase 4).
    pub generation: AtomicU64,
    /// Blocco E4: visto almeno un annullamento / un timer di scadenza dell'esecuzione. Finché sono falsi la proiezione non fa istruzioni in più.
    pub cancelled_any: AtomicBool,
    pub exec_timers: AtomicBool,
    /// Blocco E4c: ritardo dentro la transazione di scrittura del proiettore, prima del `COMMIT` (SQLite tiene il lock di scrittura per tutto il
    /// ritardo). `(ms, inizio_s, durata_s)`, con `inizio_s` contato dalla prima applicazione; `(0, ..)` = nessun ritardo.
    delay: (u64, u64, u64),
    t0: std::sync::OnceLock<std::time::Instant>,
    delay_on: AtomicBool,
}

const CHUNK: usize = 500;

impl Db {
    pub async fn open(url: &str) -> Res<Db> {
        sqlx::any::install_default_drivers();
        let pg = url.starts_with("postgres");
        let o = AnyPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(std::time::Duration::from_secs(2));
        let o = if pg {
            o
        } else {
            // busy_timeout breve: con il file bloccato il proiettore deve fallire in fretta
            o.after_connect(|c, _| {
                Box::pin(async move {
                    c.execute("PRAGMA journal_mode=WAL").await?;
                    c.execute("PRAGMA synchronous=FULL").await?;
                    c.execute("PRAGMA busy_timeout=300").await?;
                    Ok(())
                })
            })
        };
        let pool = o.connect(url).await?;
        let db = Db {
            pool,
            pg,
            stmts: 0.into(),
            stmts_t: [0.into(), 0.into()],
            rows: 0.into(),
            generation: 1.into(),
            cancelled_any: false.into(),
            exec_timers: false.into(),
            delay: (
                crate::arg("proj_delay_ms", "0").parse().unwrap(),
                crate::arg("proj_delay_from_s", "0").parse().unwrap(),
                crate::arg("proj_delay_secs", "0").parse().unwrap(),
            ),
            t0: Default::default(),
            delay_on: false.into(),
        };
        let b = if pg { "BYTEA" } else { "BLOB" };
        for ddl in [
            "CREATE TABLE IF NOT EXISTS projection_pos(topic TEXT PRIMARY KEY, pos BIGINT NOT NULL, gen BIGINT NOT NULL DEFAULT 1)".to_string(),
            format!("CREATE TABLE IF NOT EXISTS execution(id {b} PRIMARY KEY, tenant {b} NOT NULL, kind BIGINT NOT NULL, n BIGINT NOT NULL, work_ms BIGINT NOT NULL, out_size BIGINT NOT NULL, retry_ms BIGINT NOT NULL DEFAULT 0, input_hash {b} NOT NULL, input_seg BIGINT NOT NULL DEFAULT 0, input_off BIGINT NOT NULL DEFAULT 0, input_len BIGINT NOT NULL DEFAULT 0, accepted_us BIGINT NOT NULL, done_us BIGINT, status BIGINT, end_role TEXT, end_name TEXT, end_us BIGINT)"),
            format!("CREATE TABLE IF NOT EXISTS activity(exec_id {b} NOT NULL, step BIGINT NOT NULL, tenant {b} NOT NULL, is_last BIGINT NOT NULL, ready_us BIGINT, done_us BIGINT, out_hash {b}, done_events BIGINT NOT NULL DEFAULT 0, out_seg BIGINT, out_off BIGINT, out_len BIGINT, PRIMARY KEY(exec_id, step))"),
            format!("CREATE TABLE IF NOT EXISTS timer(exec_id {b} NOT NULL, step BIGINT NOT NULL, kind BIGINT NOT NULL, attempt BIGINT NOT NULL, due_us BIGINT NOT NULL, PRIMARY KEY(exec_id, step))"),
            "CREATE INDEX IF NOT EXISTS execution_live ON execution(id) WHERE done_us IS NULL".to_string(),
            // blocco E5: scritture del motore (segreti, chiavi avvolte) simulate; FK non applicabile nel prototipo: un segreto appartiene al tenant, non a un'esecuzione
            format!("CREATE TABLE IF NOT EXISTS tenant_secret(tenant {b} NOT NULL, name TEXT NOT NULL, value {b} NOT NULL, ts BIGINT NOT NULL, PRIMARY KEY(tenant, name))"),
        ] {
            db.pool.execute(sqlx::AssertSqlSafe(ddl)).await?;
        }
        Ok(db)
    }

    fn lit(&self, b: &[u8]) -> String {
        if self.pg {
            format!("'\\x{}'", hex(b))
        } else {
            format!("X'{}'", hex(b))
        }
    }

    async fn run(&self, c: &mut sqlx::AnyConnection, sql: &str) -> Res<()> {
        self.stmts.fetch_add(1, Relaxed);
        let _ = TENANT.try_with(|t| self.stmts_t[*t].fetch_add(1, Relaxed));
        let r = c.execute(sqlx::AssertSqlSafe(sql.to_string())).await?;
        self.rows.fetch_add(r.rows_affected(), Relaxed);
        Ok(())
    }

    /// Blocco E5: una riga di `tenant_secret` in una transazione breve sullo stesso pool (scrittura del motore simulata: segreto o chiave avvolta
    /// opaca). Nessuna chiave esterna: il segreto appartiene al tenant, non a un'esecuzione, quindi non punta a righe non ancora proiettate.
    pub async fn put_secret(&self, tenant: &Uid, name: &str, value: &[u8], ts: u64) -> Res<()> {
        let mut c = self.pool.acquire().await?;
        self.run(&mut c, "BEGIN").await?;
        let sql = format!(
            "INSERT INTO tenant_secret(tenant,name,value,ts) VALUES (X'{}','{name}',X'{}',{ts})",
            crate::crypto::hex(tenant),
            crate::crypto::hex(value)
        );
        match self.run(&mut c, &sql).await {
            Ok(()) => self.run(&mut c, "COMMIT").await,
            Err(e) => {
                let _ = c.execute("ROLLBACK").await;
                Err(e)
            }
        }
    }

    /// Applica un gruppo di eventi del topic e la nuova posizione (la prossima da leggere), in una transazione.
    pub async fn apply(
        &self,
        topic: &str,
        next_pos: u64,
        evs: &[Ev],
        locs: &HashMap<Hash, Loc>,
    ) -> Res<()> {
        let mut c = self.pool.acquire().await?;
        self.run(&mut c, "BEGIN").await?;
        let r = self.apply_in_tx(&mut c, topic, next_pos, evs, locs).await;
        // blocco E4c: database lento ma attivo: scritture fatte (lock di scrittura preso), commit rimandato
        if r.is_ok() {
            let d = self.delay_now();
            if d > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(d)).await;
            }
        }
        match r {
            Ok(()) => self.run(&mut c, "COMMIT").await,
            Err(e) => {
                let _ = c.execute("ROLLBACK").await;
                Err(e)
            }
        }
    }

    /// Ritardo da applicare ora (ms): `delay.0` tra `inizio_s` e `inizio_s + durata_s` dalla prima applicazione, altrimenti 0. Scrive una riga
    /// `PROJ_DELAY_ON` / `PROJ_DELAY_OFF` all'inizio e alla fine (istante in µs).
    fn delay_now(&self) -> u64 {
        let (ms, from, secs) = self.delay;
        if ms == 0 {
            return 0;
        }
        let t = self
            .t0
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64();
        let on = t >= from as f64 && t < (from + secs) as f64;
        if on != self.delay_on.swap(on, std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "PROJ_DELAY_{} {}",
                if on { "ON" } else { "OFF" },
                crate::now_us()
            );
        }
        if on { ms } else { 0 }
    }

    async fn apply_in_tx(
        &self,
        c: &mut sqlx::AnyConnection,
        topic: &str,
        next_pos: u64,
        evs: &[Ev],
        locs: &HashMap<Hash, Loc>,
    ) -> Res<()> {
        let (mut ex, mut rd, mut dn, mut fin) = (vec![], vec![], vec![], vec![]);
        // timer (blocco C): l ultimo di ogni passo nel gruppo vince (Postgres rifiuta due upsert della stessa riga in un
        // solo INSERT); gli esiti cancellano il timer del loro passo
        let mut tm: HashMap<(Uid, u32), String> = HashMap::new();
        let mut dl = vec![];
        let mut cn: Vec<(String, String)> = vec![];
        let mut fin_ts = 0;
        for e in evs {
            match e {
                Ev::Accepted {
                    exec,
                    tenant,
                    kind,
                    n,
                    work_ms,
                    out_size,
                    retry_ms,
                    ts,
                    input_hash,
                    ..
                } => ex.push(format!(
                    "({},{},{},{},{},{},{},{},{},{},{},{},NULL)",
                    self.lit(exec),
                    self.lit(tenant),
                    kind,
                    n,
                    work_ms,
                    out_size,
                    retry_ms,
                    self.lit(input_hash),
                    locs.get(input_hash).map_or(0, |l| l.0),
                    locs.get(input_hash).map_or(0, |l| l.1),
                    locs.get(input_hash).map_or(0, |l| l.2),
                    ts
                )),
                Ev::Ready {
                    exec,
                    tenant,
                    step,
                    last,
                    ts,
                } => rd.push(format!(
                    "({},{},{},{},{},NULL,NULL,0,NULL,NULL,NULL)",
                    self.lit(exec),
                    step,
                    self.lit(tenant),
                    *last as u8,
                    ts
                )),
                Ev::Done {
                    exec,
                    tenant,
                    step,
                    last,
                    ts,
                    out_hash,
                    ..
                } => {
                    let l = locs.get(out_hash).copied().unwrap_or_default();
                    dl.push(format!("({},{})", self.lit(exec), step));
                    dn.push(format!(
                        "({},{},{},{},{},{},{},1,{},{},{})",
                        self.lit(exec),
                        step,
                        self.lit(tenant),
                        *last as u8,
                        ts,
                        ts,
                        self.lit(out_hash),
                        l.0,
                        l.1,
                        l.2
                    ));
                    if *last {
                        fin.push(self.lit(exec));
                        fin_ts = *ts;
                    }
                }
                Ev::Timer {
                    exec,
                    tenant: _,
                    step,
                    kind,
                    attempt,
                    due_us,
                    ..
                } => {
                    if *step == EXEC_STEP {
                        self.exec_timers.store(true, Relaxed);
                    }
                    tm.insert(
                        (*exec, *step),
                        format!(
                            "({},{},{},{},{})",
                            self.lit(exec),
                            step,
                            kind,
                            attempt,
                            due_us
                        ),
                    );
                }
                Ev::Cancelled {
                    exec,
                    status,
                    role,
                    name,
                    ts,
                    ..
                } => {
                    self.cancelled_any.store(true, Relaxed);
                    // storico (FR-10): stato, chi (ruolo e nome visualizzato) e quando; l'esecuzione esce dalle vive
                    let q = |s: &str| s.replace('\'', "''");
                    cn.push((
                        self.lit(exec),
                        format!(
                            "UPDATE execution SET done_us={ts}, status={status}, end_role='{}', end_name='{}', end_us={ts} WHERE id={}",
                            q(role),
                            q(name),
                            self.lit(exec)
                        ),
                    ));
                }
            }
        }
        for ch in ex.chunks(CHUNK) {
            self.run(c, &format!("INSERT INTO execution(id,tenant,kind,n,work_ms,out_size,retry_ms,input_hash,input_seg,input_off,input_len,accepted_us,done_us) VALUES {} ON CONFLICT(id) DO NOTHING", ch.join(","))).await?;
        }
        for ch in rd.chunks(CHUNK) {
            self.run(c, &format!("INSERT INTO activity(exec_id,step,tenant,is_last,ready_us,done_us,out_hash,done_events,out_seg,out_off,out_len) VALUES {} ON CONFLICT(exec_id,step) DO NOTHING", ch.join(","))).await?;
        }
        for ch in dn.chunks(CHUNK) {
            self.run(c, &format!("INSERT INTO activity(exec_id,step,tenant,is_last,ready_us,done_us,out_hash,done_events,out_seg,out_off,out_len) VALUES {} ON CONFLICT(exec_id,step) DO UPDATE SET done_us=excluded.done_us, out_hash=excluded.out_hash, out_seg=excluded.out_seg, out_off=excluded.out_off, out_len=excluded.out_len, done_events=activity.done_events+1", ch.join(","))).await?;
        }
        let tm_ids: Vec<String> = if self.cancelled_any.load(Relaxed) {
            let mut ids: Vec<Uid> = tm.keys().map(|k| k.0).collect();
            ids.sort();
            ids.dedup();
            ids.iter().map(|e| self.lit(e)).collect()
        } else {
            vec![]
        };
        let tm: Vec<String> = tm.into_values().collect();
        for ch in tm.chunks(CHUNK) {
            self.run(c, &format!("INSERT INTO timer(exec_id,step,kind,attempt,due_us) VALUES {} ON CONFLICT(exec_id,step) DO UPDATE SET kind=excluded.kind, attempt=excluded.attempt, due_us=excluded.due_us", ch.join(","))).await?;
        }
        for ch in dl.chunks(CHUNK) {
            self.run(
                c,
                &format!(
                    "DELETE FROM timer WHERE (exec_id, step) IN ({})",
                    ch.join(",")
                ),
            )
            .await?;
        }
        for ch in fin.chunks(CHUNK) {
            self.run(
                c,
                &format!(
                    "UPDATE execution SET done_us={} WHERE id IN ({})",
                    fin_ts,
                    ch.join(",")
                ),
            )
            .await?;
        }
        // Blocco E4. Annullamenti: storico e uscita dalle vive, poi via tutti i timer dell'esecuzione (anche quello di scadenza).
        for (id, upd) in &cn {
            self.run(c, upd).await?;
            self.run(c, &format!("DELETE FROM timer WHERE exec_id={id}"))
                .await?;
        }
        // Un `Timer` può arrivare dopo `Cancelled` (consegne fatte tra la richiesta e l'evento durevole: il loro timer segue nel bus):
        // si toglie, per non lasciare righe di timer di esecuzioni annullate. Solo se si è visto un annullamento: nessun costo prima.
        if self.cancelled_any.load(Relaxed) && !tm_ids.is_empty() {
            self.run(c, &format!("DELETE FROM timer WHERE exec_id IN (SELECT id FROM execution WHERE status IS NOT NULL AND id IN ({}))", tm_ids.join(","))).await?;
        }
        // con timer di scadenza dell'esecuzione in uso, la sua riga si toglie quando l'esecuzione conclude
        if self.exec_timers.load(Relaxed) && !fin.is_empty() {
            self.run(
                c,
                &format!("DELETE FROM timer WHERE exec_id IN ({})", fin.join(",")),
            )
            .await?;
        }
        self.run(c, &format!("INSERT INTO projection_pos(topic,pos,gen) VALUES ('{topic}',{next_pos},{}) ON CONFLICT(topic) DO UPDATE SET pos=excluded.pos, gen=excluded.gen", self.generation.load(Relaxed))).await
    }

    /// Generazione del bus a cui si riferiscono le posizioni salvate (`None` se non ce n'è nessuna).
    pub async fn generation(&self) -> Res<Option<u64>> {
        let r = sqlx::query("SELECT min(gen) FROM projection_pos")
            .fetch_one(&self.pool)
            .await?;
        Ok(r.get::<Option<i64>, _>(0).map(|g| g as u64))
    }

    /// Dopo il riversamento dello stato vivo su un bus nuovo: posizioni e generazione nuove, in una transazione.
    pub async fn reset_positions(&self, bgen: u64, pos: &[(String, u64)]) -> Res<()> {
        let mut c = self.pool.acquire().await?;
        self.run(&mut c, "BEGIN").await?;
        for (topic, p) in pos {
            self.run(&mut c, &format!("INSERT INTO projection_pos(topic,pos,gen) VALUES ('{topic}',{p},{bgen}) ON CONFLICT(topic) DO UPDATE SET pos=excluded.pos, gen=excluded.gen")).await?;
        }
        self.run(&mut c, "COMMIT").await
    }

    /// Prossima posizione da leggere per ogni topic.
    pub async fn positions(&self) -> Res<HashMap<String, u64>> {
        let r = sqlx::query("SELECT topic, pos FROM projection_pos")
            .fetch_all(&self.pool)
            .await?;
        Ok(r.iter()
            .map(|x| (x.get::<String, _>(0), x.get::<i64, _>(1) as u64))
            .collect())
    }

    /// Esecuzioni annullate o scadute già proiettate (blocco E4): servono a contare gli esiti tardivi dopo una ripresa.
    pub async fn cancelled_ids(&self) -> Res<std::collections::HashSet<Uid>> {
        let r = sqlx::query("SELECT id FROM execution WHERE status IS NOT NULL")
            .fetch_all(&self.pool)
            .await?;
        Ok(r.into_iter()
            .map(|x| x.get::<Vec<u8>, _>(0).try_into().unwrap())
            .collect())
    }

    /// Esecuzioni vive (non concluse) con lo stato dei passi, come proiettato fino alla posizione applicata.
    pub async fn live(&self) -> Res<HashMap<Uid, Exec>> {
        let mut m = HashMap::new();
        let r = sqlx::query("SELECT id, tenant, kind, n, work_ms, out_size, input_hash, input_seg, input_off, input_len, retry_ms FROM execution WHERE done_us IS NULL")
            .fetch_all(&self.pool)
            .await?;
        for x in r {
            let id: Uid = x.get::<Vec<u8>, _>(0).try_into().unwrap();
            let tenant: Uid = x.get::<Vec<u8>, _>(1).try_into().unwrap();
            let h: Hash = x.get::<Vec<u8>, _>(6).try_into().unwrap();
            let mut ex = Exec::new(
                tenant,
                x.get::<i64, _>(2) as u8,
                x.get::<i64, _>(3) as u32,
                x.get::<i64, _>(4) as u32,
                x.get::<i64, _>(5) as u32,
                x.get::<i64, _>(10) as u32,
                h,
            );
            ex.locs.insert(
                h,
                (
                    x.get::<i64, _>(7) as u64,
                    x.get::<i64, _>(8) as u64,
                    x.get::<i64, _>(9) as u32,
                ),
            );
            m.insert(id, ex);
        }
        let r = sqlx::query("SELECT a.exec_id, a.step, a.out_hash, a.out_seg, a.out_off, a.out_len FROM activity a JOIN execution e ON e.id = a.exec_id WHERE e.done_us IS NULL")
            .fetch_all(&self.pool)
            .await?;
        for x in r {
            let id: Uid = x.get::<Vec<u8>, _>(0).try_into().unwrap();
            let step = x.get::<i64, _>(1) as usize;
            if let Some(e) = m.get_mut(&id) {
                e.issued[step] = true;
                e.done[step] = x
                    .get::<Option<Vec<u8>>, _>(2)
                    .map(|h| h.try_into().unwrap());
                if let (Some(h), Some(s), Some(o), Some(l)) = (
                    e.done[step],
                    x.get::<Option<i64>, _>(3),
                    x.get::<Option<i64>, _>(4),
                    x.get::<Option<i64>, _>(5),
                ) {
                    e.locs.insert(h, (s as u64, o as u64, l as u32));
                }
            }
        }
        Ok(m)
    }

    /// Timer in sospeso delle esecuzioni vive: (esecuzione, passo) -> (tipo, tentativo, scadenza in microsecondi).
    pub async fn timers(&self) -> Res<HashMap<(Uid, u32), (u8, u32, u64)>> {
        let r = sqlx::query("SELECT t.exec_id, t.step, t.kind, t.attempt, t.due_us FROM timer t JOIN execution e ON e.id = t.exec_id WHERE e.done_us IS NULL")
            .fetch_all(&self.pool)
            .await?;
        Ok(r.iter()
            .map(|x| {
                (
                    (
                        x.get::<Vec<u8>, _>(0).try_into().unwrap(),
                        x.get::<i64, _>(1) as u32,
                    ),
                    (
                        x.get::<i64, _>(2) as u8,
                        x.get::<i64, _>(3) as u32,
                        x.get::<i64, _>(4) as u64,
                    ),
                )
            })
            .collect())
    }

    /// Payload di uscita proiettati: (tenant, impronta, segmento, scostamento, lunghezza), per la verifica dopo il guasto.
    pub async fn out_locs(&self, sample: u64) -> Res<Vec<(Uid, Hash, Loc)>> {
        // sample > 1: solo le righe con scostamento multiplo di `sample` (campione, per le corse da milioni di attività)
        let r = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT tenant, out_hash, out_seg, out_off, out_len FROM activity WHERE done_us IS NOT NULL AND out_off % {} = 0", sample.max(1)))).fetch_all(&self.pool).await?;
        Ok(r.iter()
            .map(|x| {
                (
                    x.get::<Vec<u8>, _>(0).try_into().unwrap(),
                    x.get::<Vec<u8>, _>(1).try_into().unwrap(),
                    (
                        x.get::<i64, _>(2) as u64,
                        x.get::<i64, _>(3) as u64,
                        x.get::<i64, _>(4) as u32,
                    ),
                )
            })
            .collect())
    }

    /// Byte di WAL scritti finora (solo Postgres; SQLite: 0, non misurato).
    pub async fn wal_bytes(&self) -> Res<i64> {
        if self.pg {
            self.count("SELECT (pg_current_wal_lsn() - '0/0'::pg_lsn)::bigint")
                .await
        } else {
            Ok(0)
        }
    }

    /// Un intero da una query di conteggio (verifiche).
    pub async fn count(&self, sql: &str) -> Res<i64> {
        Ok(sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>(0))
    }

    /// Scarto di scatto dei passi `wait` conclusi: istante dell'esito meno scadenza (accettazione + attesa), in microsecondi.
    /// Negativo = scattato prima della scadenza. L'istante dell'esito è quello della decisione, prima della scrittura durevole.
    pub async fn wait_delays(&self) -> Res<Vec<i64>> {
        let r = sqlx::query("SELECT a.done_us - e.accepted_us - e.work_ms * 1000 FROM activity a JOIN execution e ON e.id = a.exec_id WHERE e.kind = 2 AND a.done_us IS NOT NULL")
            .fetch_all(&self.pool)
            .await?;
        Ok(r.iter().map(|x| x.get::<i64, _>(0)).collect())
    }

    /// Identificativi delle esecuzioni concluse.
    pub async fn done_ids(&self) -> Res<std::collections::HashSet<Uid>> {
        let r = sqlx::query("SELECT id FROM execution WHERE done_us IS NOT NULL")
            .fetch_all(&self.pool)
            .await?;
        Ok(r.iter()
            .map(|x| x.get::<Vec<u8>, _>(0).try_into().unwrap())
            .collect())
    }
}
