//! Motore minimo, proiettore e archivio dei payload in un solo processo.
//!
//! Regola d'oro (AD-2): ogni decisione si scrive in modo durevole sul bus PRIMA di qualunque
//! effetto (risposta all'accettazione, consegna al worker). Il motore è l'unico scrittore del bus;
//! il proiettore è l'unico scrittore del database e salva la posizione applicata nella stessa transazione.

use crate::bus::Bus;
use crate::crypto::{Keys, Uid, hex, sha};
use crate::db::Db;
use crate::ev::{Ev, Exec, Hash};
use crate::pb;
use crate::{hist, now_us};
use bytes::Bytes;
use hdrhistogram::Histogram;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tonic::Status;

pub const TENANTS: [Uid; 2] = [
    [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x0a],
    [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x0b],
];

/// Il nome del topic usa solo l'identificativo interno del tenant (AD-5).
pub fn topic_of(t: &Uid) -> String {
    format!("state-{}", hex(t))
}

fn tenant_idx(t: &[u8]) -> Option<usize> {
    TENANTS.iter().position(|x| x == t)
}

// ---------------------------------------------------------------- metriche

pub struct M {
    pub accept: Histogram<u64>,
    pub append: Histogram<u64>,
    pub start_ready: Histogram<u64>,
    pub start_sent: Histogram<u64>,
    pub start_join: Histogram<u64>,
    pub start_b: Histogram<u64>,
    pub archive_read: Histogram<u64>,
    pub proj_lag: Histogram<u64>,
    pub accepted: u64,
    pub refused: u64,
    pub done: u64,
    pub rejected: u64,
    pub groups: u64,
    pub since: Instant,
    pub stmts0: u64,
    pub rows0: u64,
    pub wal0: i64,
    pub cpu0: f64,
}

fn cpu_s() -> f64 {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) };
    t.tv_sec as f64 + t.tv_nsec as f64 / 1e9
}

impl M {
    fn new() -> M {
        M {
            accept: hist(),
            append: hist(),
            start_ready: hist(),
            start_sent: hist(),
            start_join: hist(),
            start_b: hist(),
            archive_read: hist(),
            proj_lag: hist(),
            accepted: 0,
            refused: 0,
            done: 0,
            rejected: 0,
            groups: 0,
            since: Instant::now(),
            stmts0: 0,
            rows0: 0,
            wal0: 0,
            cpu0: cpu_s(),
        }
    }
}

pub struct Shared {
    pub keys: Keys,
    pub bus: Bus,
    /// Una connessione per scrittore (un tenant ciascuno), così le scritture dei tenant non si accodano sulla stessa connessione.
    pub bus_w: Vec<Bus>,
    pub db: Arc<Db>,
    pub dir: String,
    pub failing: AtomicU32,
    pub applied_ts: [AtomicU64; 2],
    pub m: Mutex<M>,
    pub cache: Mutex<HashMap<(Uid, Hash), (Bytes, u64)>>,
    pub trim: bool,
    pub proj_batch: u32,
    /// Attesa prima di applicare un gruppo piccolo: raccoglie più eventi per istruzione (NFR-4, SM-2), restando sotto 1 s di ritardo.
    pub window_ms: u64,
}

impl Shared {
    fn put_cache(&self, t: Uid, h: Hash, b: Bytes, ts: u64) {
        self.cache.lock().unwrap().insert((t, h), (b, ts));
    }

    fn arc_path(&self, t: &Uid, h: &Hash) -> String {
        format!("{}/archive/{}/{}/{}", self.dir, hex(t), hex(&h[..1]), hex(h))
    }

    /// Scrive il payload cifrato per tenant solo se assente (stesso contenuto = stesso nome, AD-17).
    fn archive_put(&self, t: &Uid, h: &Hash, b: &[u8]) {
        use std::io::Write;
        let p = self.arc_path(t, h);
        let sealed = self.keys.seal(t, &format!("arc/{}", hex(h)), h[..16].try_into().unwrap(), b);
        // creazione solo se assente (O_EXCL); un file esistente ma troncato da un crash si riscrive via tmp + rename
        let open = || std::fs::OpenOptions::new().write(true).create_new(true).open(&p);
        match open() {
            Ok(mut f) => f.write_all(&sealed).unwrap(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
                let _ = open().and_then(|mut f| f.write_all(&sealed));
            }
            Err(_) => {
                if std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) != sealed.len() as u64 {
                    let tmp = format!("{p}.tmp{}", std::process::id());
                    std::fs::write(&tmp, &sealed).unwrap();
                    std::fs::rename(&tmp, &p).unwrap();
                }
            }
        }
    }

    async fn archive_get(&self, t: &Uid, h: &Hash) -> Bytes {
        let t0 = Instant::now();
        let raw = tokio::fs::read(self.arc_path(t, h)).await.expect("payload assente nell'archivio");
        let (_, _, plain) = self.keys.open(&format!("arc/{}", hex(h)), &raw);
        self.m.lock().unwrap().archive_read.record(t0.elapsed().as_micros() as u64 + 1).ok();
        Bytes::from(plain)
    }

    /// Payload per hash: prima la cache di quelli non ancora proiettati, poi l'archivio.
    async fn payload(&self, t: &Uid, h: &Hash) -> Bytes {
        let hit = self.cache.lock().unwrap().get(&(*t, *h)).map(|x| x.0.clone());
        match hit {
            Some(b) => b,
            None => self.archive_get(t, h).await,
        }
    }

    fn seal_ev(&self, topic: &str, e: &Ev) -> Bytes {
        let t = match e {
            Ev::Accepted { tenant, .. } | Ev::Ready { tenant, .. } | Ev::Done { tenant, .. } => tenant,
        };
        let id = *uuid::Uuid::now_v7().as_bytes();
        Bytes::from(self.keys.seal(t, &format!("bus/{topic}"), &id, &e.encode()))
    }
}

// ---------------------------------------------------------------- messaggi del motore

pub enum Msg {
    Accept { tenant: usize, req: pb::AcceptReq, t0: Instant, reply: oneshot::Sender<Result<Uid, Status>> },
    Result { sess: u64, r: pb::FromWorker },
    Register { sess: u64, tx: mpsc::Sender<Result<pb::Delivery, Status>> },
    Unregister { sess: u64 },
}

struct Infl {
    sess: u64,
    delivery_id: u64,
}

struct Engine {
    sh: Arc<Shared>,
    execs: HashMap<Uid, Exec>,
    sessions: Vec<(u64, mpsc::Sender<Result<pb::Delivery, Status>>)>,
    rr: usize,
    inflight: HashMap<(Uid, u32), Infl>,
    pending: [VecDeque<(Uid, u32, u64)>; 2],
    tenant_inflight: [usize; 2],
    cap: usize,
    next_delivery: u64,
}

impl Engine {
    /// Decide i passi pronti di un'esecuzione: li marca emessi e produce gli eventi `Ready`.
    fn decide(&mut self, exec: Uid, evs: &mut Vec<(usize, Ev)>, newly: &mut Vec<(usize, Uid, u32, u64)>) {
        let ts = now_us();
        let e = self.execs.get_mut(&exec).unwrap();
        let ti = tenant_idx(&e.tenant).unwrap();
        for s in e.ready_unissued() {
            e.issued[s as usize] = true;
            evs.push((ti, Ev::Ready { exec, tenant: e.tenant, step: s, last: s + 1 == e.steps(), ts }));
            newly.push((ti, exec, s, ts));
        }
    }

    /// Scrive in modo durevole, per tenant in parallelo. Un errore del bus è fatale per il processo.
    // ponytail: un errore del bus ferma il motore; il rifiuto con codice e la ripresa sono della fase 4.
    async fn append(&self, evs: &[(usize, Ev)]) {
        let t0 = Instant::now();
        let mut set: tokio::task::JoinSet<crate::bus::Res<()>> = tokio::task::JoinSet::new();
        for ti in 0..2 {
            let topic = topic_of(&TENANTS[ti]);
            let msgs: Vec<Bytes> = evs.iter().filter(|(i, _)| *i == ti).map(|(_, e)| self.sh.seal_ev(&topic, e)).collect();
            if msgs.is_empty() {
                continue;
            }
            let bus = self.sh.bus_w[ti].clone();
            set.spawn(async move {
                // gruppi di al più 8 MB (Iggy rifiuta richieste oltre 64 MB), in ordine, tutti durevoli prima di tornare
                let mut from = 0;
                while from < msgs.len() {
                    let (mut to, mut bytes) = (from, 0);
                    while to < msgs.len() && (to == from || bytes + msgs[to].len() <= (8 << 20)) {
                        bytes += msgs[to].len();
                        to += 1;
                    }
                    bus.append(&topic, &msgs[from..to]).await?;
                    from = to;
                }
                Ok(())
            });
        }
        while let Some(r) = set.join_next().await {
            if let Err(e) = r.unwrap() {
                eprintln!("append sul bus fallita: {e}");
                std::process::exit(2);
            }
        }
        self.sh.m.lock().unwrap().append.record(t0.elapsed().as_micros() as u64 + 1).ok();
    }

    /// Consegna le attività pronte ai worker, a turno tra i tenant, rispettando il tetto per tenant.
    async fn dispatch(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let mut progress = true;
        while progress {
            progress = false;
            for ti in 0..2 {
                if self.tenant_inflight[ti] >= self.cap {
                    continue;
                }
                let Some((exec, step, ready_us)) = self.pending[ti].pop_front() else { continue };
                let Some(e) = self.execs.get(&exec) else { continue };
                // fan-out: solo il ramo 0 è lento, così le uscite degli altri rami sono già proiettate quando serve il join
                let (tenant, work_ms, out_size) = (e.tenant, if e.kind == 1 && step != 0 { 0 } else { e.work_ms }, e.out_size);
                let refs = e.input_refs(step);
                let input = if refs.len() == 1 {
                    self.sh.payload(&tenant, &refs[0]).await
                } else {
                    let mut v = Vec::new();
                    for h in &refs {
                        v.extend_from_slice(&self.sh.payload(&tenant, h).await);
                    }
                    Bytes::from(v)
                };
                self.rr = (self.rr + 1) % self.sessions.len();
                let (sess, tx) = self.sessions[self.rr].clone();
                self.next_delivery += 1;
                let d = pb::Delivery {
                    exec: exec.to_vec(),
                    step,
                    tenant: tenant.to_vec(),
                    delivery_id: self.next_delivery,
                    input: input.to_vec(),
                    work_ms,
                    out_size,
                    ready_us,
                    sent_us: now_us(),
                };
                if tx.try_send(Ok(d)).is_err() {
                    // sessione piena o chiusa: il passo resta in coda, la sessione chiusa si toglie con Unregister
                    self.pending[ti].push_front((exec, step, ready_us));
                    continue;
                }
                self.inflight.insert((exec, step), Infl { sess, delivery_id: self.next_delivery });
                self.tenant_inflight[ti] += 1;
                progress = true;
            }
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<Msg>) {
        const MAXB: usize = 256;
        let mut batch = Vec::with_capacity(MAXB);
        loop {
            if rx.recv_many(&mut batch, MAXB).await == 0 {
                return;
            }
            let mut evs: Vec<(usize, Ev)> = vec![];
            let mut newly = vec![];
            let mut replies = vec![];
            for msg in batch.drain(..) {
                match msg {
                    Msg::Register { sess, tx } => self.sessions.push((sess, tx)),
                    Msg::Unregister { sess } => {
                        self.sessions.retain(|s| s.0 != sess);
                        // C-2: le consegne di una sessione persa tornano in coda, con un nuovo tentativo
                        let lost: Vec<_> = self.inflight.iter().filter(|(_, f)| f.sess == sess).map(|(k, _)| *k).collect();
                        for (exec, step) in lost {
                            self.inflight.remove(&(exec, step));
                            let ti = tenant_idx(&self.execs[&exec].tenant).unwrap();
                            self.tenant_inflight[ti] -= 1;
                            self.pending[ti].push_front((exec, step, now_us()));
                        }
                    }
                    Msg::Accept { tenant, req, t0, reply } => {
                        if self.sh.failing.load(Relaxed) > 0 {
                            self.sh.m.lock().unwrap().refused += 1;
                            let _ = reply.send(Err(Status::unavailable("E_PROJECTION_DOWN")));
                            continue;
                        }
                        let ts = now_us();
                        let exec = *uuid::Uuid::now_v7().as_bytes();
                        let input = Bytes::from(req.input);
                        let h = sha(&input);
                        let t = TENANTS[tenant];
                        self.sh.put_cache(t, h, input.clone(), ts);
                        self.execs.insert(exec, Exec::new(t, req.kind as u8, req.n, req.work_ms, req.out_size, h));
                        evs.push((tenant, Ev::Accepted { exec, tenant: t, kind: req.kind as u8, n: req.n, work_ms: req.work_ms, out_size: req.out_size, ts, input_hash: h, input }));
                        self.decide(exec, &mut evs, &mut newly);
                        replies.push((reply, exec, t0));
                    }
                    Msg::Result { sess, r } => {
                        let exec: Uid = r.exec.as_slice().try_into().unwrap();
                        let key = (exec, r.step);
                        // C-2: vale solo il primo esito del tentativo corrente, dalla sessione a cui è stato consegnato
                        match self.inflight.get(&key) {
                            Some(f) if f.sess == sess && f.delivery_id == r.delivery_id => {}
                            _ => {
                                self.sh.m.lock().unwrap().rejected += 1;
                                continue;
                            }
                        }
                        self.inflight.remove(&key);
                        let e = self.execs.get_mut(&exec).unwrap();
                        let ti = tenant_idx(&e.tenant).unwrap();
                        self.tenant_inflight[ti] -= 1;
                        let output = Bytes::from(r.output);
                        let h = sha(&output);
                        let ts = now_us();
                        e.done[r.step as usize] = Some(h);
                        let (tenant, last) = (e.tenant, r.step + 1 == e.steps());
                        let is_join = e.kind == 1 && r.step == e.n;
                        self.sh.put_cache(tenant, h, output.clone(), ts);
                        {
                            let mut m = self.sh.m.lock().unwrap();
                            m.start_ready.record(r.lat_ready_us.max(1)).ok();
                            m.start_sent.record(r.lat_sent_us.max(1)).ok();
                            if is_join {
                                m.start_join.record(r.lat_ready_us.max(1)).ok();
                            }
                            if ti == 1 {
                                m.start_b.record(r.lat_ready_us.max(1)).ok();
                            }
                            m.done += 1;
                        }
                        evs.push((ti, Ev::Done { exec, tenant, step: r.step, last, ts, out_hash: h, output }));
                        if !last {
                            self.decide(exec, &mut evs, &mut newly);
                        }
                    }
                }
            }
            if !evs.is_empty() {
                self.append(&evs).await;
            }
            {
                let mut m = self.sh.m.lock().unwrap();
                for (_, _, t0) in &replies {
                    m.accept.record(t0.elapsed().as_micros() as u64 + 1).ok();
                }
                m.accepted += replies.len() as u64;
            }
            for (reply, exec, _) in replies {
                let _ = reply.send(Ok(exec));
            }
            for (ti, exec, step, ts) in newly {
                self.pending[ti].push_back((exec, step, ts));
            }
            // le esecuzioni concluse escono dallo stato dopo la scrittura durevole dei loro eventi
            self.execs.retain(|_, e| !e.finished());
            self.dispatch().await;
        }
    }
}

// ---------------------------------------------------------------- ripresa

/// Ricostruisce lo stato: database fino alla posizione applicata, poi rilettura del bus da lì.
async fn recover(sh: &Arc<Shared>) -> (HashMap<Uid, Exec>, Vec<(usize, Uid, u32)>) {
    let mut execs = sh.db.live().await.expect("lettura dello stato dal database");
    let pos = sh.db.positions().await.unwrap();
    let mut replayed = 0u64;
    for (ti, t) in TENANTS.iter().enumerate() {
        let topic = topic_of(t);
        let mut p = pos.get(&topic).copied().unwrap_or(0);
        loop {
            let r = sh.bus.read(&topic, p, 1000).await.expect("rilettura del bus");
            if r.is_empty() {
                break;
            }
            for (pp, raw) in &r {
                let (_, _, plain) = sh.keys.open(&format!("bus/{topic}"), raw);
                match Ev::decode(&plain) {
                    Ev::Accepted { exec, tenant, kind, n, work_ms, out_size, ts, input_hash, input } => {
                        sh.put_cache(tenant, input_hash, input, ts);
                        execs.entry(exec).or_insert_with(|| Exec::new(tenant, kind, n, work_ms, out_size, input_hash));
                    }
                    Ev::Ready { exec, step, .. } => {
                        if let Some(e) = execs.get_mut(&exec) {
                            e.issued[step as usize] = true;
                        }
                    }
                    Ev::Done { exec, tenant, step, ts, out_hash, output, .. } => {
                        sh.put_cache(tenant, out_hash, output, ts);
                        if let Some(e) = execs.get_mut(&exec) {
                            e.issued[step as usize] = true;
                            e.done[step as usize] = Some(out_hash);
                        }
                    }
                }
                p = pp + 1;
                replayed += 1;
            }
        }
        let _ = ti;
    }
    execs.retain(|_, e| !e.finished());
    eprintln!("ripresa: {} esecuzioni vive, {replayed} eventi riletti dal bus", execs.len());
    // attività emesse e non concluse: da riconsegnare con un nuovo tentativo
    let redeliver = execs
        .iter()
        .flat_map(|(id, e)| (0..e.steps()).filter(|&s| e.issued[s as usize] && e.done[s as usize].is_none()).map(move |s| (tenant_idx(&e.tenant).unwrap(), *id, s)))
        .collect();
    (execs, redeliver)
}

// ---------------------------------------------------------------- proiettore

async fn projector(sh: Arc<Shared>, ti: usize, start: u64) {
    // connessione propria del proiettore: le letture non si mettono in coda dietro le scritture del motore
    let bus = Bus::connect(sh.bus.name()).await.unwrap();
    let topic = topic_of(&TENANTS[ti]);
    let mut pos = start;
    let mut last_trim = Instant::now();
    let mut failing = false;
    loop {
        let raw = match bus.read(&topic, pos, sh.proj_batch).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("lettura del bus fallita: {e}");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };
        if raw.is_empty() {
            tokio::time::sleep(Duration::from_millis(2)).await;
            continue;
        }
        let raw = if (raw.len() as u32) < sh.proj_batch / 2 && sh.window_ms > 0 {
            tokio::time::sleep(Duration::from_millis(sh.window_ms)).await;
            bus.read(&topic, pos, sh.proj_batch).await.unwrap_or(raw)
        } else {
            raw
        };
        let next = raw.last().unwrap().0 + 1;
        let evs: Vec<Ev> = raw.iter().map(|(_, b)| Ev::decode(&sh.keys.open(&format!("bus/{topic}"), b).2)).collect();
        // payload nell'archivio e fsync del file system PRIMA della transazione (la posizione fa scartare il bus)
        let sh2 = sh.clone();
        let evs2 = evs.clone();
        let noarc = std::env::var("DIAG_NOARCHIVE").is_ok();
        tokio::task::spawn_blocking(move || {
            if noarc {
                return;
            }
            let items: Vec<(Uid, Hash, Bytes)> = evs2
                .into_iter()
                .filter_map(|e| match e {
                    Ev::Accepted { tenant, input_hash, input, .. } => Some((tenant, input_hash, input)),
                    Ev::Done { tenant, out_hash, output, .. } => Some((tenant, out_hash, output)),
                    _ => None,
                })
                .collect();
            std::thread::scope(|s| {
                for ch in items.chunks(items.len().div_ceil(4).max(1)) {
                    let sh3 = &sh2;
                    s.spawn(move || ch.iter().for_each(|(t, h, b)| sh3.archive_put(t, h, b)));
                }
            });
            let f = std::fs::File::open(format!("{}/archive", sh2.dir)).unwrap();
            use std::os::fd::AsRawFd;
            unsafe { libc::syncfs(f.as_raw_fd()) };
        })
        .await
        .unwrap();
        // transazione; con il database giù si ritenta per sempre, rifiutando lavoro nuovo (AD-2)
        loop {
            match sh.db.apply(&topic, next, &evs).await {
                Ok(()) => break,
                Err(e) => {
                    if !failing {
                        failing = true;
                        sh.failing.fetch_add(1, Relaxed);
                        eprintln!("proiezione ferma su {topic}: {e}");
                    }
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        }
        if failing {
            failing = false;
            sh.failing.fetch_sub(1, Relaxed);
            eprintln!("proiezione ripresa su {topic}");
        }
        let now = now_us();
        {
            let mut m = sh.m.lock().unwrap();
            m.groups += 1;
            for e in &evs {
                m.proj_lag.record(now.saturating_sub(e.ts()).max(1)).ok();
            }
        }
        sh.applied_ts[ti].store(evs.iter().map(|e| e.ts()).max().unwrap(), Relaxed);
        pos = next;
        if sh.trim && last_trim.elapsed() > Duration::from_millis(500) {
            last_trim = Instant::now();
            let _ = bus.trim(&topic, pos).await;
        }
    }
}

// ---------------------------------------------------------------- gRPC

struct Ctl {
    tx: mpsc::Sender<Msg>,
    sh: Arc<Shared>,
}

#[tonic::async_trait]
impl pb::control_server::Control for Ctl {
    async fn accept(&self, r: tonic::Request<pb::AcceptReq>) -> Result<tonic::Response<pb::AcceptResp>, Status> {
        let t0 = Instant::now();
        let req = r.into_inner();
        let tenant = tenant_idx(&req.tenant).ok_or_else(|| Status::invalid_argument("E_TENANT"))?;
        if req.n == 0 || req.n > 10_000 {
            return Err(Status::invalid_argument("E_SHAPE"));
        }
        let (reply, rx) = oneshot::channel();
        self.tx.send(Msg::Accept { tenant, req, t0, reply }).await.map_err(|_| Status::unavailable("E_ENGINE"))?;
        let exec = rx.await.map_err(|_| Status::unavailable("E_ENGINE"))??;
        Ok(tonic::Response::new(pb::AcceptResp { exec: exec.to_vec() }))
    }

    async fn stats(&self, r: tonic::Request<pb::StatsReq>) -> Result<tonic::Response<pb::StatsResp>, Status> {
        let wal = self.sh.db.wal_bytes().await.unwrap_or(0);
        let mut m = self.sh.m.lock().unwrap();
        let secs = m.since.elapsed().as_secs_f64();
        let f = |h: &Histogram<u64>| {
            format!("n={} p50={:.2}ms p99={:.2}ms p99.9={:.2}ms max={:.2}ms", h.len(), h.value_at_quantile(0.5) as f64 / 1e3, h.value_at_quantile(0.99) as f64 / 1e3, h.value_at_quantile(0.999) as f64 / 1e3, h.max() as f64 / 1e3)
        };
        let stm = self.sh.db.stmts.load(Relaxed) - m.stmts0;
        let rows = self.sh.db.rows.load(Relaxed) - m.rows0;
        let text = format!(
            "secs={secs:.1} accepted={} refused={} done={} rejected={} groups={}\nactivities_per_s={:.0} accepted_per_s={:.0}\naccept_engine {}\nappend {}\nstart_ready {}\nstart_sent {}\nstart_join {}\nstart_ready_tenant_b {}\narchive_read {}\nproj_lag {}\nsql_stmts={stm} sql_rows={rows} sql_per_activity={:.4} rows_per_activity={:.3} wal_bytes_per_activity={:.1}\nengine_cpu_s={:.2} cache_entries={}",
            m.accepted,
            m.refused,
            m.done,
            m.rejected,
            m.groups,
            m.done as f64 / secs,
            m.accepted as f64 / secs,
            f(&m.accept),
            f(&m.append),
            f(&m.start_ready),
            f(&m.start_sent),
            f(&m.start_join),
            f(&m.start_b),
            f(&m.archive_read),
            f(&m.proj_lag),
            stm as f64 / m.done.max(1) as f64,
            rows as f64 / m.done.max(1) as f64,
            (wal - m.wal0) as f64 / m.done.max(1) as f64,
            cpu_s() - m.cpu0,
            self.sh.cache.lock().unwrap().len(),
        );
        if r.into_inner().reset {
            let (stmts, rows) = (self.sh.db.stmts.load(Relaxed), self.sh.db.rows.load(Relaxed));
            *m = M::new();
            m.stmts0 = stmts;
            m.rows0 = rows;
            m.wal0 = wal;
        }
        Ok(tonic::Response::new(pb::StatsResp { text }))
    }
}

struct Wrk {
    tx: mpsc::Sender<Msg>,
    next: AtomicU64,
}

#[tonic::async_trait]
impl pb::worker_server::Worker for Wrk {
    type SessionStream = tokio_stream::wrappers::ReceiverStream<Result<pb::Delivery, Status>>;

    async fn session(&self, r: tonic::Request<tonic::Streaming<pb::FromWorker>>) -> Result<tonic::Response<Self::SessionStream>, Status> {
        let sess = self.next.fetch_add(1, Relaxed);
        let (dtx, drx) = mpsc::channel(8192);
        self.tx.send(Msg::Register { sess, tx: dtx }).await.map_err(|_| Status::unavailable("E_ENGINE"))?;
        let tx = self.tx.clone();
        let mut inb = r.into_inner();
        tokio::spawn(async move {
            while let Ok(Some(m)) = inb.message().await {
                if !m.hello {
                    let _ = tx.send(Msg::Result { sess, r: m }).await;
                }
            }
            let _ = tx.send(Msg::Unregister { sess }).await;
        });
        Ok(tonic::Response::new(tokio_stream::wrappers::ReceiverStream::new(drx)))
    }
}

// ---------------------------------------------------------------- avvio

pub async fn main(bus_kind: &str, db_url: &str, listen: &str, dir: &str, cap: usize, proj_batch: u32, trim: bool, window_ms: u64) {
    let t_start = Instant::now();
    // AD-2: un solo motore; il lock si libera da solo se il processo muore (kill -9)
    let lock = std::fs::OpenOptions::new().create(true).truncate(true).write(true).open(format!("{dir}/engine.lock")).unwrap();
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            eprintln!("E_ENGINE_LOCKED: un altro motore è attivo");
            std::process::exit(3);
        }
    }
    std::fs::create_dir_all(format!("{dir}/archive")).unwrap();
    let keys = Keys::load(&format!("{dir}/master.key"));
    let bus = Bus::connect(bus_kind).await.unwrap();
    for t in &TENANTS {
        bus.ensure_topic(&topic_of(t)).await.unwrap();
    }
    let db = Arc::new(Db::open(db_url).await.unwrap());
    let sh = Arc::new(Shared {
        keys,
        bus_w: vec![Bus::connect(bus_kind).await.unwrap(), Bus::connect(bus_kind).await.unwrap()],
        bus,
        db,
        dir: dir.into(),
        failing: 0.into(),
        applied_ts: [0.into(), 0.into()],
        m: Mutex::new(M::new()),
        cache: Mutex::new(HashMap::new()),
        trim,
        proj_batch,
        window_ms,
    });
    let (execs, redeliver) = recover(&sh).await;
    let pos = sh.db.positions().await.unwrap();
    for ti in 0..2 {
        tokio::spawn(projector(sh.clone(), ti, pos.get(&topic_of(&TENANTS[ti])).copied().unwrap_or(0)));
    }
    // eviction della cache dei payload non ancora proiettati
    let sh2 = sh.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let w = [sh2.applied_ts[0].load(Relaxed), sh2.applied_ts[1].load(Relaxed)];
            if std::env::var("DIAG_NOARCHIVE").is_err() {
            sh2.cache.lock().unwrap().retain(|(t, _), (_, ts)| *ts > w[tenant_idx(t).unwrap()]);
            }
        }
    });
    let (tx, rx) = mpsc::channel(65536);
    let mut eng = Engine {
        sh: sh.clone(),
        execs,
        sessions: vec![],
        rr: 0,
        inflight: HashMap::new(),
        pending: [VecDeque::new(), VecDeque::new()],
        tenant_inflight: [0, 0],
        cap,
        next_delivery: now_us(),
    };
    // decisioni mancanti dopo la ripresa (Ready non scritti), poi riconsegna delle emesse e non concluse
    let ids: Vec<Uid> = eng.execs.keys().copied().collect();
    let (mut evs, mut newly) = (vec![], vec![]);
    for id in ids {
        eng.decide(id, &mut evs, &mut newly);
    }
    if !evs.is_empty() {
        eng.append(&evs).await;
    }
    for (ti, exec, step, ts) in newly {
        eng.pending[ti].push_back((exec, step, ts));
    }
    for (ti, exec, step) in redeliver {
        eng.pending[ti].push_back((exec, step, now_us()));
    }
    tokio::spawn(eng.run(rx));

    let cert = std::fs::read(format!("{dir}/tls/cert.pem")).unwrap();
    let key = std::fs::read(format!("{dir}/tls/key.pem")).unwrap();
    let tls = tonic::transport::ServerTlsConfig::new().identity(tonic::transport::Identity::from_pem(cert, key));
    println!("READY recovered_ms={}", t_start.elapsed().as_millis());
    tonic::transport::Server::builder()
        .tls_config(tls)
        .unwrap()
        .add_service(pb::control_server::ControlServer::new(Ctl { tx: tx.clone(), sh: sh.clone() }).max_decoding_message_size(64 << 20))
        .add_service(pb::worker_server::WorkerServer::new(Wrk { tx, next: 1.into() }).max_decoding_message_size(64 << 20).max_encoding_message_size(64 << 20))
        .serve(listen.parse().unwrap())
        .await
        .unwrap();
}
