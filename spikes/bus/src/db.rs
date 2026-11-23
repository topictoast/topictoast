//! Adattatore del database (SQLite o Postgres) usato dal proiettore, con il contatore delle istruzioni.
//!
//! Ogni gruppo di eventi è una transazione: righe tipizzate e posizione applicata insieme.
//! ponytail: i valori entrano nel testo SQL come numeri e letterali esadecimali costruiti da tipi
//! interni (mai da testo esterno), così lo stesso codice serve i due dialetti senza segnaposto;
//! in produzione sono parametri legati.

use crate::bus::Res;
use crate::crypto::{Uid, hex};
use crate::ev::{Ev, Exec, Hash};
use sqlx::{AnyPool, Executor, Row, any::AnyPoolOptions};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

pub struct Db {
    pool: AnyPool,
    pg: bool,
    /// Istruzioni SQL inviate (BEGIN e COMMIT compresi).
    pub stmts: AtomicU64,
    /// Righe toccate dalle istruzioni di scrittura.
    pub rows: AtomicU64,
}

const CHUNK: usize = 500;

impl Db {
    pub async fn open(url: &str) -> Res<Db> {
        sqlx::any::install_default_drivers();
        let pg = url.starts_with("postgres");
        let o = AnyPoolOptions::new().max_connections(4).acquire_timeout(std::time::Duration::from_secs(2));
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
        let db = Db { pool, pg, stmts: 0.into(), rows: 0.into() };
        let b = if pg { "BYTEA" } else { "BLOB" };
        for ddl in [
            "CREATE TABLE IF NOT EXISTS projection_pos(topic TEXT PRIMARY KEY, pos BIGINT NOT NULL)".to_string(),
            format!("CREATE TABLE IF NOT EXISTS execution(id {b} PRIMARY KEY, tenant {b} NOT NULL, kind BIGINT NOT NULL, n BIGINT NOT NULL, work_ms BIGINT NOT NULL, out_size BIGINT NOT NULL, input_hash {b} NOT NULL, accepted_us BIGINT NOT NULL, done_us BIGINT)"),
            format!("CREATE TABLE IF NOT EXISTS activity(exec_id {b} NOT NULL, step BIGINT NOT NULL, tenant {b} NOT NULL, is_last BIGINT NOT NULL, ready_us BIGINT, done_us BIGINT, out_hash {b}, done_events BIGINT NOT NULL DEFAULT 0, PRIMARY KEY(exec_id, step))"),
            "CREATE INDEX IF NOT EXISTS execution_live ON execution(id) WHERE done_us IS NULL".to_string(),
        ] {
            db.pool.execute(sqlx::AssertSqlSafe(ddl)).await?;
        }
        Ok(db)
    }

    fn lit(&self, b: &[u8]) -> String {
        if self.pg { format!("'\\x{}'", hex(b)) } else { format!("X'{}'", hex(b)) }
    }

    async fn run(&self, c: &mut sqlx::AnyConnection, sql: &str) -> Res<()> {
        self.stmts.fetch_add(1, Relaxed);
        let r = c.execute(sqlx::AssertSqlSafe(sql.to_string())).await?;
        self.rows.fetch_add(r.rows_affected(), Relaxed);
        Ok(())
    }

    /// Applica un gruppo di eventi del topic e la nuova posizione (la prossima da leggere), in una transazione.
    pub async fn apply(&self, topic: &str, next_pos: u64, evs: &[Ev]) -> Res<()> {
        let mut c = self.pool.acquire().await?;
        self.run(&mut c, "BEGIN").await?;
        let r = self.apply_in_tx(&mut c, topic, next_pos, evs).await;
        match r {
            Ok(()) => self.run(&mut c, "COMMIT").await,
            Err(e) => {
                let _ = c.execute("ROLLBACK").await;
                Err(e)
            }
        }
    }

    async fn apply_in_tx(&self, c: &mut sqlx::AnyConnection, topic: &str, next_pos: u64, evs: &[Ev]) -> Res<()> {
        let (mut ex, mut rd, mut dn, mut fin) = (vec![], vec![], vec![], vec![]);
        let mut fin_ts = 0;
        for e in evs {
            match e {
                Ev::Accepted { exec, tenant, kind, n, work_ms, out_size, ts, input_hash, .. } => ex.push(format!(
                    "({},{},{},{},{},{},{},{},NULL)",
                    self.lit(exec),
                    self.lit(tenant),
                    kind,
                    n,
                    work_ms,
                    out_size,
                    self.lit(input_hash),
                    ts
                )),
                Ev::Ready { exec, tenant, step, last, ts } => {
                    rd.push(format!("({},{},{},{},{},NULL,NULL,0)", self.lit(exec), step, self.lit(tenant), *last as u8, ts))
                }
                Ev::Done { exec, tenant, step, last, ts, out_hash, .. } => {
                    dn.push(format!("({},{},{},{},{},{},{},1)", self.lit(exec), step, self.lit(tenant), *last as u8, ts, ts, self.lit(out_hash)));
                    if *last {
                        fin.push(self.lit(exec));
                        fin_ts = *ts;
                    }
                }
            }
        }
        for ch in ex.chunks(CHUNK) {
            self.run(c, &format!("INSERT INTO execution(id,tenant,kind,n,work_ms,out_size,input_hash,accepted_us,done_us) VALUES {} ON CONFLICT(id) DO NOTHING", ch.join(","))).await?;
        }
        for ch in rd.chunks(CHUNK) {
            self.run(c, &format!("INSERT INTO activity(exec_id,step,tenant,is_last,ready_us,done_us,out_hash,done_events) VALUES {} ON CONFLICT(exec_id,step) DO NOTHING", ch.join(","))).await?;
        }
        for ch in dn.chunks(CHUNK) {
            self.run(c, &format!("INSERT INTO activity(exec_id,step,tenant,is_last,ready_us,done_us,out_hash,done_events) VALUES {} ON CONFLICT(exec_id,step) DO UPDATE SET done_us=excluded.done_us, out_hash=excluded.out_hash, done_events=activity.done_events+1", ch.join(","))).await?;
        }
        for ch in fin.chunks(CHUNK) {
            self.run(c, &format!("UPDATE execution SET done_us={} WHERE id IN ({})", fin_ts, ch.join(","))).await?;
        }
        self.run(c, &format!("INSERT INTO projection_pos(topic,pos) VALUES ('{topic}',{next_pos}) ON CONFLICT(topic) DO UPDATE SET pos=excluded.pos")).await
    }

    /// Prossima posizione da leggere per ogni topic.
    pub async fn positions(&self) -> Res<HashMap<String, u64>> {
        let r = sqlx::query("SELECT topic, pos FROM projection_pos").fetch_all(&self.pool).await?;
        Ok(r.iter().map(|x| (x.get::<String, _>(0), x.get::<i64, _>(1) as u64)).collect())
    }

    /// Esecuzioni vive (non concluse) con lo stato dei passi, come proiettato fino alla posizione applicata.
    pub async fn live(&self) -> Res<HashMap<Uid, Exec>> {
        let mut m = HashMap::new();
        let r = sqlx::query("SELECT id, tenant, kind, n, work_ms, out_size, input_hash FROM execution WHERE done_us IS NULL")
            .fetch_all(&self.pool)
            .await?;
        for x in r {
            let id: Uid = x.get::<Vec<u8>, _>(0).try_into().unwrap();
            let tenant: Uid = x.get::<Vec<u8>, _>(1).try_into().unwrap();
            let h: Hash = x.get::<Vec<u8>, _>(6).try_into().unwrap();
            m.insert(id, Exec::new(tenant, x.get::<i64, _>(2) as u8, x.get::<i64, _>(3) as u32, x.get::<i64, _>(4) as u32, x.get::<i64, _>(5) as u32, h));
        }
        let r = sqlx::query("SELECT a.exec_id, a.step, a.out_hash FROM activity a JOIN execution e ON e.id = a.exec_id WHERE e.done_us IS NULL")
            .fetch_all(&self.pool)
            .await?;
        for x in r {
            let id: Uid = x.get::<Vec<u8>, _>(0).try_into().unwrap();
            let step = x.get::<i64, _>(1) as usize;
            if let Some(e) = m.get_mut(&id) {
                e.issued[step] = true;
                e.done[step] = x.get::<Option<Vec<u8>>, _>(2).map(|h| h.try_into().unwrap());
            }
        }
        Ok(m)
    }

    /// Byte di WAL scritti finora (solo Postgres; SQLite: 0, non misurato).
    pub async fn wal_bytes(&self) -> Res<i64> {
        if self.pg { self.count("SELECT (pg_current_wal_lsn() - '0/0'::pg_lsn)::bigint").await } else { Ok(0) }
    }

    /// Un intero da una query di conteggio (verifiche).
    pub async fn count(&self, sql: &str) -> Res<i64> {
        Ok(sqlx::query(sqlx::AssertSqlSafe(sql.to_string())).fetch_one(&self.pool).await?.get::<i64, _>(0))
    }

    /// Identificativi delle esecuzioni concluse.
    pub async fn done_ids(&self) -> Res<std::collections::HashSet<Uid>> {
        let r = sqlx::query("SELECT id FROM execution WHERE done_us IS NOT NULL").fetch_all(&self.pool).await?;
        Ok(r.iter().map(|x| x.get::<Vec<u8>, _>(0).try_into().unwrap()).collect())
    }
}
