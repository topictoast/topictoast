//! Motore minimo, proiettore e archivio dei payload in un solo processo.
//!
//! Regola d'oro (AD-2): ogni decisione si scrive in modo durevole sul bus PRIMA di qualunque
//! effetto (risposta all'accettazione, consegna al worker). Il motore è l'unico scrittore del bus;
//! il proiettore è l'unico scrittore del database e salva la posizione applicata nella stessa transazione.

use crate::bus::Bus;
use crate::crypto::{Keys, Uid, hex, sha};
use crate::db::Db;
use crate::ev::{Ev, Exec, Hash, Loc};
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
    /// Durata di ogni singola richiesta di scrittura (una per tenant e connessione).
    pub append_sub: Histogram<u64>,
    /// Istogrammi del minuto corrente, azzerati dalla riga al minuto della diagnosi.
    pub min_append: Histogram<u64>,
    pub min_sub: Histogram<u64>,
    pub min_lag: Histogram<u64>,
    /// Eventi per richiesta, in unità (non in tempo).
    pub sub_events: Histogram<u64>,
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
            append_sub: hist(),
            min_append: hist(),
            min_sub: hist(),
            min_lag: hist(),
            sub_events: hist(),
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

/// Segmento aperto di un tenant (uno scrittore: il suo proiettore).
pub struct Seg {
    pub file: Option<std::fs::File>,
    pub id: u64,
    pub len: u64,
    pub opened: Instant,
}

pub struct Shared {
    pub segs: [Mutex<Seg>; 2],
    /// Posizioni già scritte ma non ancora recepite dal ciclo del motore (che può essere in coda): evita di leggere una posizione che manca.
    pub pending_locs: Mutex<HashMap<(Uid, Hash), Loc>>,
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

    fn seg_path(&self, t: &Uid, seg: u64) -> String {
        format!("{}/archive/{}/{}.seg", self.dir, hex(t), seg)
    }

    /// Accoda un gruppo di payload (uno per impronta, ognuno sigillato da solo come in C-12) al segmento aperto del
    /// tenant e fa un solo `fdatasync` del segmento. Il segmento si chiude per dimensione o dopo 1 s e non si riscrive mai.
    /// Torna la posizione di ogni payload. Un crash lascia al più byte non referenziati in coda: le posizioni nel
    /// database sono solo quelle già sincronizzate, quindi nessun payload confermato si perde.
    // ponytail: un payload già presente in un segmento precedente si riscrive (nessuna deduplicazione tra gruppi).
    fn archive_put_group(&self, ti: usize, items: &[(Uid, Hash, Bytes)]) -> HashMap<Hash, Loc> {
        use std::io::Write;
        let mut locs = HashMap::new();
        let mut buf = Vec::new();
        let mut rel = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (t, h, b) in items {
            if !seen.insert(*h) {
                continue;
            }
            let sealed = self.keys.seal(t, &format!("arc/{}", hex(h)), h[..16].try_into().unwrap(), b);
            rel.push((*h, buf.len() as u64, sealed.len() as u32));
            buf.extend_from_slice(&sealed);
        }
        let mut s = self.segs[ti].lock().unwrap();
        if s.file.is_none() || s.len > (32 << 20) || s.opened.elapsed() > Duration::from_secs(1) {
            let dir = format!("{}/archive/{}", self.dir, hex(&TENANTS[ti]));
            std::fs::create_dir_all(&dir).unwrap();
            s.id = now_us();
            s.file = Some(std::fs::OpenOptions::new().append(true).create_new(true).open(self.seg_path(&TENANTS[ti], s.id)).unwrap());
            s.len = 0;
            s.opened = Instant::now();
            // la voce di directory del nuovo segmento deve essere durevole prima di referenziarlo
            std::fs::File::open(&dir).unwrap().sync_all().unwrap();
        }
        s.file.as_mut().unwrap().write_all(&buf).unwrap();
        s.file.as_ref().unwrap().sync_data().unwrap();
        for (h, off, len) in rel {
            locs.insert(h, (s.id, s.len + off, len));
        }
        s.len += buf.len() as u64;
        locs
    }

    async fn archive_get(&self, t: &Uid, h: &Hash, loc: Loc) -> Bytes {
        use std::os::unix::fs::FileExt;
        let t0 = Instant::now();
        let p = self.seg_path(t, loc.0);
        let raw = tokio::task::spawn_blocking(move || {
            let mut b = vec![0u8; loc.2 as usize];
            std::fs::File::open(p).expect("segmento assente").read_exact_at(&mut b, loc.1).expect("lettura del segmento");
            b
        })
        .await
        .unwrap();
        let (_, _, plain) = self.keys.open(&format!("arc/{}", hex(h)), &raw);
        self.m.lock().unwrap().archive_read.record(t0.elapsed().as_micros() as u64 + 1).ok();
        Bytes::from(plain)
    }

    /// Payload per hash: prima la cache di quelli non ancora proiettati, poi l'archivio con la posizione nota.
    async fn payload(&self, t: &Uid, h: &Hash, loc: Option<Loc>) -> Bytes {
        let hit = self.cache.lock().unwrap().get(&(*t, *h)).map(|x| x.0.clone());
        match hit {
            Some(b) => b,
            None => self.archive_get(t, h, loc.expect("payload fuori cache senza posizione")).await,
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
    /// Posizioni nei segmenti di payload appena archiviati: (esecuzione, impronta, posizione).
    Located(usize, Vec<(Uid, Hash, Loc)>),
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
    /// Istante comune a tutti gli eventi di una iterazione: con scritture in parallelo il bus può mostrare al proiettore
    /// una parte della iterazione prima del resto; la cache si svuota solo per istanti strettamente più vecchi.
    bts: u64,
    /// Code di scrittura sul bus, una per tenant e connessione (indice tenant * n + connessione).
    jobs: Vec<mpsc::UnboundedSender<Job>>,
    /// Gruppi di eventi al più in volo sul bus.
    k: usize,
    /// Messaggi al più per gruppo.
    maxb: usize,
    /// Messaggi in coda necessari per aprire un gruppo mentre un altro è già in volo.
    minb: usize,
}

impl Engine {
    /// Decide i passi pronti di un'esecuzione: li marca emessi e produce gli eventi `Ready`.
    fn decide(&mut self, exec: Uid, evs: &mut Vec<(usize, Ev)>, newly: &mut Vec<(usize, Uid, u32, u64)>) {
        let ts = self.bts;
        let e = self.execs.get_mut(&exec).unwrap();
        let ti = tenant_idx(&e.tenant).unwrap();
        for s in e.ready_unissued() {
            e.issued[s as usize] = true;
            evs.push((ti, Ev::Ready { exec, tenant: e.tenant, step: s, last: s + 1 == e.steps(), ts }));
            newly.push((ti, exec, s, ts));
        }
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
                let refs: Vec<(Hash, Option<Loc>)> = e.input_refs(step).into_iter().map(|h| (h, e.locs.get(&h).copied().or_else(|| self.sh.pending_locs.lock().unwrap().get(&(tenant, h)).copied()))).collect();
                let input = if refs.len() == 1 {
                    self.sh.payload(&tenant, &refs[0].0, refs[0].1).await
                } else {
                    let mut v = Vec::new();
                    for (h, l) in &refs {
                        v.extend_from_slice(&self.sh.payload(&tenant, h, *l).await);
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

    /// Elabora un gruppo di messaggi: aggiorna lo stato e produce gli eventi da scrivere, i passi pronti e le risposte.
    /// Nessun effetto esterno qui: risposte e consegne partono solo quando il gruppo è durevole (AD-2).
    fn handle(&mut self, batch: Vec<Msg>) -> (Vec<(usize, Ev)>, Vec<(usize, Uid, u32, u64)>, Vec<(oneshot::Sender<Result<Uid, Status>>, Uid, Instant)>) {
        self.bts = now_us();
        let mut evs: Vec<(usize, Ev)> = vec![];
        let mut newly = vec![];
        let mut replies = vec![];
        for msg in batch {
                match msg {
                    Msg::Located(ti, v) => {
                        for (exec, h, loc) in v {
                            self.sh.pending_locs.lock().unwrap().remove(&(TENANTS[ti], h));
                            if let Some(e) = self.execs.get_mut(&exec) {
                                e.locs.insert(h, loc);
                            }
                        }
                    }
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
                        let ts = self.bts;
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
                        let ts = self.bts;
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
        (evs, newly, replies)
    }

    /// Manda gli eventi alle code di scrittura, una per tenant e connessione, ognuna in ordine di arrivo (FIFO).
    /// L'esecuzione usa sempre la connessione scelta dal suo id: l'ordine dei suoi eventi sul bus non cambia, anche
    /// con più gruppi in volo. Torna il numero di richieste inviate.
    fn submit(&self, gid: u64, evs: &[(usize, Ev)]) -> usize {
        let n = self.jobs.len() / 2;
        let mut sent = 0;
        for ti in 0..2 {
            let topic = topic_of(&TENANTS[ti]);
            for ci in 0..n {
                let msgs: Vec<Bytes> = evs.iter().filter(|(i, e)| *i == ti && e.exec_hash() % n as u64 == ci as u64).map(|(_, e)| self.sh.seal_ev(&topic, e)).collect();
                if msgs.is_empty() {
                    continue;
                }
                let _ = self.jobs[ti * n + ci].send(Job { gid, topic: topic.clone(), msgs });
                sent += 1;
            }
        }
        sent
    }

    /// Effetti di un gruppo diventato durevole: risposte alle accettazioni, passi pronti, consegne.
    async fn finalize(&mut self, g: Group) {
        {
            let mut m = self.sh.m.lock().unwrap();
            for (_, _, t0) in &g.replies {
                m.accept.record(t0.elapsed().as_micros() as u64 + 1).ok();
            }
            m.accepted += g.replies.len() as u64;
            if g.remaining_total > 0 {
                m.append.record(g.t0.elapsed().as_micros() as u64 + 1).ok();
                m.min_append.record(g.t0.elapsed().as_micros() as u64 + 1).ok();
            }
        }
        for (reply, exec, _) in g.replies {
            let _ = reply.send(Ok(exec));
        }
        for (ti, exec, step, ts) in g.newly {
            self.pending[ti].push_back((exec, step, ts));
        }
        // le esecuzioni concluse escono dallo stato quando i loro eventi sono durevoli
        self.execs.retain(|_, e| !e.finished());
        self.dispatch().await;
    }

    /// Ciclo del motore con fino a `k` gruppi in volo sul bus. Il ciclo non aspetta più la scrittura: continua a
    /// raccogliere messaggi e a decidere; gli effetti di un gruppo partono quando le sue richieste sono durevoli.
    async fn run(mut self, mut rx: mpsc::Receiver<Msg>, mut drx: mpsc::UnboundedReceiver<u64>) {
        let mut groups: HashMap<u64, Group> = HashMap::new();
        let mut gid = 0u64;
        loop {
            let mut batch = Vec::new();
            // Raccolta a gruppi pieni: con un gruppo già in volo se ne apre un altro solo con almeno `minb` messaggi in coda,
            // altrimenti si aspetta (raggruppamento naturale). Senza questa regola il ciclo emette tanti gruppi minuscoli,
            // la coda FIFO della connessione si riempie e la latenza di ogni gruppo cresce di k volte senza più throughput.
            let can_take = groups.len() < self.k && (groups.is_empty() || rx.len() >= self.minb);
            tokio::select! {
                n = rx.recv_many(&mut batch, self.maxb), if can_take => {
                    if n == 0 {
                        return;
                    }
                    let (evs, newly, replies) = self.handle(batch);
                    if evs.is_empty() {
                        self.finalize(Group { remaining: 0, remaining_total: 0, newly, replies, t0: Instant::now() }).await;
                    } else {
                        gid += 1;
                        let sent = self.submit(gid, &evs);
                        groups.insert(gid, Group { remaining: sent, remaining_total: sent, newly, replies, t0: Instant::now() });
                    }
                }
                Some(id) = drx.recv() => {
                    let g = groups.get_mut(&id).unwrap();
                    g.remaining -= 1;
                    if g.remaining == 0 {
                        let g = groups.remove(&id).unwrap();
                        self.finalize(g).await;
                    }
                }
                // riesamina la regola di raccolta quando i messaggi in coda crescono
                _ = tokio::time::sleep(Duration::from_micros(500)), if !groups.is_empty() && !can_take => {}
            }
        }
    }
}

/// Gruppo di eventi in volo sul bus.
struct Group {
    remaining: usize,
    remaining_total: usize,
    newly: Vec<(usize, Uid, u32, u64)>,
    replies: Vec<(oneshot::Sender<Result<Uid, Status>>, Uid, Instant)>,
    t0: Instant,
}

/// Richiesta di scrittura per una coda (tenant, connessione).
struct Job {
    gid: u64,
    topic: String,
    msgs: Vec<Bytes>,
}

/// Una connessione al bus e una coda FIFO: scrive le richieste in ordine e segnala il gruppo quando sono durevoli.
/// Un errore del bus ferma il processo (ponytail: rifiuto con codice e ripresa sono della fase 4).
async fn conn_worker(bus: Bus, mut rx: mpsc::UnboundedReceiver<Job>, dtx: mpsc::UnboundedSender<u64>, sh: Arc<Shared>) {
    while let Some(j) = rx.recv().await {
        let ts = Instant::now();
        let nev = j.msgs.len() as u64;
        // richieste di al più 8 MB (Iggy rifiuta oltre 64 MB), in ordine
        let mut from = 0;
        while from < j.msgs.len() {
            let (mut to, mut bytes) = (from, 0);
            while to < j.msgs.len() && (to == from || bytes + j.msgs[to].len() <= (8 << 20)) {
                bytes += j.msgs[to].len();
                to += 1;
            }
            if let Err(e) = bus.append(&j.topic, &j.msgs[from..to]).await {
                eprintln!("append sul bus fallita: {e}");
                std::process::exit(2);
            }
            from = to;
        }
        {
            let mut m = sh.m.lock().unwrap();
            m.append_sub.record(ts.elapsed().as_micros() as u64 + 1).ok();
            m.min_sub.record(ts.elapsed().as_micros() as u64 + 1).ok();
            m.sub_events.record(nev).ok();
        }
        let _ = dtx.send(j.gid);
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

async fn projector(sh: Arc<Shared>, ti: usize, start: u64, tx: mpsc::Sender<Msg>) {
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
        // payload nel segmento del tenant e un solo fdatasync PRIMA della transazione (la posizione fa scartare il bus)
        let sh2 = sh.clone();
        let evs2 = evs.clone();
        let noarc = std::env::var("DIAG_NOARCHIVE").is_ok();
        let (locs, located) = tokio::task::spawn_blocking(move || {
            if noarc {
                return (HashMap::new(), vec![]);
            }
            let items: Vec<(Uid, Hash, Bytes)> = evs2
                .iter()
                .filter_map(|e| match e {
                    Ev::Accepted { tenant, input_hash, input, .. } => Some((*tenant, *input_hash, input.clone())),
                    Ev::Done { tenant, out_hash, output, .. } => Some((*tenant, *out_hash, output.clone())),
                    _ => None,
                })
                .collect();
            let locs = sh2.archive_put_group(ti, &items);
            // posizioni per il motore: solo per le esecuzioni ancora vive (le altre le scarta lui)
            let located = evs2
                .iter()
                .filter_map(|e| match e {
                    Ev::Accepted { exec, input_hash, .. } => Some((*exec, *input_hash, locs[input_hash])),
                    Ev::Done { exec, out_hash, .. } => Some((*exec, *out_hash, locs[out_hash])),
                    _ => None,
                })
                .collect();
            (locs, located)
        })
        .await
        .unwrap();
        {
            let mut p = sh.pending_locs.lock().unwrap();
            for (_, h, l) in &located {
                p.insert((TENANTS[ti], *h), *l);
            }
        }
        let _ = tx.send(Msg::Located(ti, located)).await;
        // transazione; con il database giù si ritenta per sempre, rifiutando lavoro nuovo (AD-2)
        loop {
            match sh.db.apply(&topic, next, &evs, &locs).await {
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
                m.min_lag.record(now.saturating_sub(e.ts()).max(1)).ok();
            }
        }
        sh.applied_ts[ti].store(evs.iter().map(|e| e.ts()).max().unwrap(), Relaxed);
        // la cache si svuota con i payload appena proiettati (non per istante: con più connessioni l'ordine sul bus non segue gli istanti)
        if !noarc_flag() {
            let mut c = sh.cache.lock().unwrap();
            for e in &evs {
                match e {
                    Ev::Accepted { tenant, input_hash, .. } => c.remove(&(*tenant, *input_hash)),
                    Ev::Done { tenant, out_hash, .. } => c.remove(&(*tenant, *out_hash)),
                    _ => None,
                };
            }
        }
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
            format!("n={} p50={:.2}ms p99={:.2}ms p99.9={:.2}ms max={:.2}ms over100ms={}", h.len(), h.value_at_quantile(0.5) as f64 / 1e3, h.value_at_quantile(0.99) as f64 / 1e3, h.value_at_quantile(0.999) as f64 / 1e3, h.max() as f64 / 1e3, h.count_between(100_000, u64::MAX))
        };
        let stm = self.sh.db.stmts.load(Relaxed) - m.stmts0;
        let rows = self.sh.db.rows.load(Relaxed) - m.rows0;
        let text = format!(
            "secs={secs:.1} accepted={} refused={} done={} rejected={} groups={}\nactivities_per_s={:.0} accepted_per_s={:.0}\naccept_engine {}\nappend {}\nappend_sub {}\nsub_events_p50={} sub_events_p99={}\nstart_ready {}\nstart_sent {}\nstart_join {}\nstart_ready_tenant_b {}\narchive_read {}\nproj_lag {}\nsql_stmts={stm} sql_rows={rows} sql_per_activity={:.4} rows_per_activity={:.3} wal_bytes_per_activity={:.1}\nengine_cpu_s={:.2} cache_entries={}",
            m.accepted,
            m.refused,
            m.done,
            m.rejected,
            m.groups,
            m.done as f64 / secs,
            m.accepted as f64 / secs,
            f(&m.accept),
            f(&m.append),
            f(&m.append_sub),
            m.sub_events.value_at_quantile(0.5),
            m.sub_events.value_at_quantile(0.99),
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

/// Diagnostica: senza archivio i payload non si proiettano, quindi la cache non si svuota.
fn noarc_flag() -> bool {
    std::env::var("DIAG_NOARCHIVE").is_ok()
}

/// `n` connessioni di scrittura al bus (parallelismo delle scritture; l'esecuzione sceglie con il suo id).
async fn futures_n(kind: &str, n: usize) -> Vec<Bus> {
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(Bus::connect(kind).await.unwrap());
    }
    v
}

pub async fn main(bus_kind: &str, db_url: &str, listen: &str, dir: &str, cap: usize, proj_batch: u32, trim: bool, window_ms: u64, wconn: usize, kgroups: usize, maxb: usize, minb: usize) {
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
        bus_w: futures_n(bus_kind, 2 * wconn).await,
        pending_locs: Mutex::new(HashMap::new()),
        segs: [0, 1].map(|_| Mutex::new(Seg { file: None, id: 0, len: 0, opened: Instant::now() })),
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
    let (tx, rx) = mpsc::channel(65536);
    let (execs, redeliver) = recover(&sh).await;
    let pos = sh.db.positions().await.unwrap();
    for ti in 0..2 {
        tokio::spawn(projector(sh.clone(), ti, pos.get(&topic_of(&TENANTS[ti])).copied().unwrap_or(0), tx.clone()));
    }
    // riga al minuto per la diagnosi (fase 2d): attività/s, scrittura sul bus, ritardo del proiettore, pressione I/O
    let sh3 = sh.clone();
    tokio::spawn(async move {
        let (mut last_done, mut t) = (0u64, 0u32);
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            t += 1;
            let io = std::fs::read_to_string("/proc/pressure/io").ok().and_then(|s| s.lines().next().map(String::from)).unwrap_or_default();
            let mut m = sh3.m.lock().unwrap();
            let d = m.done - last_done.min(m.done);
            last_done = m.done;
            let ms = |h: &Histogram<u64>, q: f64| h.value_at_quantile(q) as f64 / 1e3;
            eprintln!("MIN t={t} act_s={:.0} append_p50_ms={:.1} sub_p50_ms={:.1} lag_p50_ms={:.0} lag_p99_ms={:.0} io=[{io}]", d as f64 / 60.0, ms(&m.min_append, 0.5), ms(&m.min_sub, 0.5), ms(&m.min_lag, 0.5), ms(&m.min_lag, 0.99));
            m.min_append.reset();
            m.min_sub.reset();
            m.min_lag.reset();
        }
    });
    // code di scrittura: una connessione al bus e una coda FIFO per tenant e connessione
    let (dtx, mut drx) = mpsc::unbounded_channel::<u64>();
    let mut jobs = vec![];
    for b in sh.bus_w.iter() {
        let (jtx, jrx) = mpsc::unbounded_channel();
        tokio::spawn(conn_worker(b.clone(), jrx, dtx.clone(), sh.clone()));
        jobs.push(jtx);
    }
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
        bts: now_us(),
        jobs,
        k: kgroups,
        maxb,
        minb,
    };
    // decisioni mancanti dopo la ripresa (Ready non scritti), poi riconsegna delle emesse e non concluse
    let ids: Vec<Uid> = eng.execs.keys().copied().collect();
    let (mut evs, mut newly) = (vec![], vec![]);
    for id in ids {
        eng.decide(id, &mut evs, &mut newly);
    }
    if !evs.is_empty() {
        for _ in 0..eng.submit(0, &evs) {
            drx.recv().await;
        }
    }
    for (ti, exec, step, ts) in newly {
        eng.pending[ti].push_back((exec, step, ts));
    }
    for (ti, exec, step) in redeliver {
        eng.pending[ti].push_back((exec, step, now_us()));
    }
    tokio::spawn(eng.run(rx, drx));

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
