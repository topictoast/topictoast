//! Motore minimo, proiettore e archivio dei payload in un solo processo.
//!
//! Regola d'oro (AD-2): ogni decisione si scrive in modo durevole sul bus PRIMA di qualunque
//! effetto (risposta all'accettazione, consegna al worker). Il motore è l'unico scrittore del bus;
//! il proiettore è lo scrittore del database e salva la posizione applicata nella stessa transazione. Eccezione di H3
//! (blocco D, terza parte): il task `rec_writer` scrive nella tabella `timer` i record di consegna di tipo 0, che non
//! passano dal bus (AD-2 vale per la decisione «pronta», non per la scadenza del tentativo).

use crate::bus::Bus;
use crate::crypto::{Keys, Uid, hex, sha};
use crate::db::Db;
use crate::ev::{Ev, Exec, Hash, Loc};
use crate::pb;
use crate::{hist, now_us};
use bytes::Bytes;
use hdrhistogram::Histogram;
use sha2::Digest;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
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
    /// Tempi del proiettore nel minuto: lettura dal bus, scrittura dei payload nell'archivio, transazione sul database (fase 4).
    pub min_pread: Histogram<u64>,
    pub min_parc: Histogram<u64>,
    pub min_pdb: Histogram<u64>,
    /// Eventi per richiesta, in unità (non in tempo).
    pub sub_events: Histogram<u64>,
    pub start_ready: Histogram<u64>,
    pub start_sent: Histogram<u64>,
    pub start_join: Histogram<u64>,
    pub start_b: Histogram<u64>,
    pub archive_read: Histogram<u64>,
    pub proj_lag: Histogram<u64>,
    /// Ritardo di scatto dei timer (blocco C): istante in cui il motore li prende in carico meno la loro scadenza.
    pub timer_delay: Histogram<u64>,
    pub accepted: u64,
    pub refused: u64,
    pub done: u64,
    pub rejected: u64,
    pub expired: u64,
    pub late_ok: u64,
    pub groups: u64,
    pub since: Instant,
    pub stmts0: u64,
    pub rows0: u64,
    pub wal0: i64,
    pub cpu0: f64,
}

fn cpu_s() -> f64 {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
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
            min_pread: hist(),
            min_parc: hist(),
            min_pdb: hist(),
            sub_events: hist(),
            start_ready: hist(),
            start_sent: hist(),
            start_join: hist(),
            start_b: hist(),
            archive_read: hist(),
            proj_lag: hist(),
            timer_delay: hist(),
            accepted: 0,
            refused: 0,
            done: 0,
            rejected: 0,
            expired: 0,
            late_ok: 0,
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
    /// Aggiornamento in corso: le accettazioni si rifiutano con `E_UPGRADE_DRAIN` e il motore si svuota (fase 4).
    pub draining: AtomicBool,
    /// Bus non raggiungibile (sonda periodica) o scritture che non riescono (blocco B): le accettazioni si rifiutano con `E_BUS_DOWN`.
    pub bus_down: AtomicBool,
    pub append_fail: AtomicU32,
    /// Disco pieno su archivio o database: le accettazioni si rifiutano con `E_STORAGE_FULL`.
    pub full: AtomicU32,
    /// Il prossimo esito durevole scrive la riga `FIRST_DONE` (all avvio e a ogni ritorno del bus).
    pub log_first: AtomicBool,
    /// Prossima posizione che il proiettore leggerà, per topic: limite inferiore per cercare una scrittura ambigua.
    pub proj_pos: [AtomicU64; 2],
    /// Margine oltre `work_ms` prima che un tentativo senza esito scada e torni disponibile.
    pub grace_ms: u64,
    /// Tempo massimo di una append sul bus prima di sospettare che sia giù.
    pub append_timeout_ms: u64,
    /// Timer in sospeso in memoria (per le statistiche).
    pub timers_pending: AtomicU64,
    /// Scrive una riga per ogni timer programmato e scattato (variabile `TIMER_LOG`, solo per le corse con pochi timer).
    pub timer_log: bool,
    /// Contratto di battito (0 = spento): con un valore, la scadenza di ogni tentativo è `lease_ms` dall'ultimo battito del worker.
    pub lease_ms: u64,
    /// Battiti accettati (ognuno è un evento `Timer` sul bus).
    pub heartbeats: AtomicU64,
    /// Blocco D, parte seconda: consegne inviate il cui `Timer` di scadenza non è ancora durevole (ora e massimo della corsa).
    pub undurable: AtomicU64,
    pub undurable_max: AtomicU64,
    /// Attività pronte ma trattenute dal tetto per tenant: ultimo valore e massimo del minuto (riga `MIN`).
    pub held: AtomicU64,
    pub held_max: AtomicU64,
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
    fn archive_put_group(
        &self,
        ti: usize,
        items: &[(Uid, Hash, Bytes)],
    ) -> std::io::Result<HashMap<Hash, Loc>> {
        use std::io::Write;
        let mut locs = HashMap::new();
        let mut buf = Vec::new();
        let mut rel = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (t, h, b) in items {
            if !seen.insert(*h) {
                continue;
            }
            let sealed = self.keys.seal(
                t,
                &format!("arc/{}", hex(h)),
                h[..16].try_into().unwrap(),
                b,
            );
            rel.push((*h, buf.len() as u64, sealed.len() as u32));
            buf.extend_from_slice(&sealed);
        }
        let mut s = self.segs[ti].lock().unwrap();
        if s.file.is_none() || s.len > (32 << 20) || s.opened.elapsed() > Duration::from_secs(1) {
            let dir = format!("{}/archive/{}", self.dir, hex(&TENANTS[ti]));
            std::fs::create_dir_all(&dir)?;
            s.id = now_us();
            s.file = None;
            s.file = Some(
                std::fs::OpenOptions::new()
                    .append(true)
                    .create_new(true)
                    .open(self.seg_path(&TENANTS[ti], s.id))?,
            );
            s.len = 0;
            s.opened = Instant::now();
            // la voce di directory del nuovo segmento deve essere durevole prima di referenziarlo
            std::fs::File::open(&dir)?.sync_all()?;
        }
        // un errore a metà scrittura (disco pieno) lascia byte non referenziati in coda: il segmento si abbandona, il prossimo gruppo ne apre uno nuovo
        let f = s.file.as_mut().unwrap();
        if let Err(e) = f.write_all(&buf).and_then(|_| f.sync_data()) {
            s.file = None;
            return Err(e);
        }
        for (h, off, len) in rel {
            locs.insert(h, (s.id, s.len + off, len));
        }
        s.len += buf.len() as u64;
        Ok(locs)
    }

    async fn archive_get(&self, t: &Uid, h: &Hash, loc: Loc) -> Bytes {
        use std::os::unix::fs::FileExt;
        let t0 = Instant::now();
        let p = self.seg_path(t, loc.0);
        let raw = tokio::task::spawn_blocking(move || {
            let mut b = vec![0u8; loc.2 as usize];
            std::fs::File::open(p)
                .expect("segmento assente")
                .read_exact_at(&mut b, loc.1)
                .expect("lettura del segmento");
            b
        })
        .await
        .unwrap();
        let (_, _, plain) = self.keys.open(&format!("arc/{}", hex(h)), &raw);
        self.m
            .lock()
            .unwrap()
            .archive_read
            .record(t0.elapsed().as_micros() as u64 + 1)
            .ok();
        Bytes::from(plain)
    }

    /// Payload per hash: prima la cache di quelli non ancora proiettati, poi l'archivio con la posizione nota.
    async fn payload(&self, t: &Uid, h: &Hash, loc: Option<Loc>) -> Bytes {
        let hit = self
            .cache
            .lock()
            .unwrap()
            .get(&(*t, *h))
            .map(|x| x.0.clone());
        match hit {
            Some(b) => b,
            None => {
                self.archive_get(t, h, loc.expect("payload fuori cache senza posizione"))
                    .await
            }
        }
    }

    fn seal_ev(&self, topic: &str, e: &Ev) -> Bytes {
        let t = match e {
            Ev::Accepted { tenant, .. }
            | Ev::Ready { tenant, .. }
            | Ev::Done { tenant, .. }
            | Ev::Timer { tenant, .. } => tenant,
        };
        let id = *uuid::Uuid::now_v7().as_bytes();
        Bytes::from(self.keys.seal(t, &format!("bus/{topic}"), &id, &e.encode()))
    }
}

// ---------------------------------------------------------------- messaggi del motore

pub enum Msg {
    /// Posizioni nei segmenti di payload appena archiviati: (esecuzione, impronta, posizione).
    Located(usize, Vec<(Uid, Hash, Loc)>),
    Accept {
        tenant: usize,
        req: pb::AcceptReq,
        t0: Instant,
        reply: oneshot::Sender<Result<Uid, Status>>,
    },
    Result {
        sess: u64,
        r: pb::FromWorker,
    },
    Register {
        sess: u64,
        tx: mpsc::Sender<Result<pb::Delivery, Status>>,
    },
    Unregister {
        sess: u64,
    },
    /// Interno (blocco C): il timer di un passo `wait` è scaduto, il passo si conclude. Non arriva mai dal canale esterno.
    WaitDone {
        exec: Uid,
        step: u32,
    },
}

struct Infl {
    /// 0 = consegna di un'incarnazione precedente del motore (ripresa): vale l'esito con qualunque `delivery_id`.
    delivery_id: u64,
    /// Numero del tentativo in corso. La scadenza non è qui: è un timer durevole (blocco C), in `Engine::timers`; è l'unico
    /// modo in cui un tentativo perso (worker morto, sessione persa) torna disponibile.
    attempt: u32,
}

struct Engine {
    sh: Arc<Shared>,
    execs: HashMap<Uid, Exec>,
    sessions: Vec<(u64, mpsc::Sender<Result<pb::Delivery, Status>>)>,
    rr: usize,
    inflight: HashMap<(Uid, u32), Infl>,
    /// Attività da consegnare: (esecuzione, passo, istante di «pronta», numero del tentativo).
    pending: [VecDeque<(Uid, u32, u64, u32)>; 2],
    tenant_inflight: [usize; 2],
    /// Timer durevoli (blocco C) in memoria: l'indice per scadenza e il valore per attività (tipo, tentativo, scadenza).
    /// Si ricostruiscono dal database e dal bus alla ripresa; un timer entra qui solo quando il suo evento è durevole.
    timers: BTreeSet<(u64, Uid, u32)>,
    tmap: HashMap<(Uid, u32), (u8, u32, u64)>,
    /// Timer da armare quando il gruppo che li scrive è durevole (AD-2: prima il timer, poi l'effetto).
    arm: Vec<(Uid, u32, u8, u32, u64)>,
    /// Record di consegna di tipo 0 verso il task scrittore del database (H3): (esecuzione, passo, tentativo, scadenza).
    rec_tx: mpsc::UnboundedSender<(Uid, u32, u32, u64)>,
    /// Gruppi di eventi in volo sul bus.
    groups: HashMap<u64, Group>,
    gid: u64,
    /// Gruppi in volo nati da messaggi in ingresso (quelli di consegne e timer non contano per il limite `k`).
    intake_n: usize,
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
    fn decide(
        &mut self,
        exec: Uid,
        evs: &mut Vec<(usize, Ev)>,
        newly: &mut Vec<(usize, Uid, u32, u64)>,
    ) {
        let ts = self.bts;
        let e = self.execs.get_mut(&exec).unwrap();
        let ti = tenant_idx(&e.tenant).unwrap();
        for s in e.ready_unissued() {
            e.issued[s as usize] = true;
            evs.push((
                ti,
                Ev::Ready {
                    exec,
                    tenant: e.tenant,
                    step: s,
                    last: s + 1 == e.steps(),
                    ts,
                },
            ));
            if e.kind == 2 {
                // passo `wait`: nessun worker, solo un timer (scritto con la decisione, armato a gruppo durevole)
                let due = ts + e.work_ms as u64 * 1000;
                evs.push((
                    ti,
                    Ev::Timer {
                        exec,
                        tenant: e.tenant,
                        step: s,
                        kind: 2,
                        attempt: 0,
                        due_us: due,
                        ts,
                    },
                ));
                self.arm.push((exec, s, 2, 0, due));
            } else {
                newly.push((ti, exec, s, ts));
            }
        }
    }

    /// Programma (o sostituisce) il timer di un passo: al più uno per attività.
    fn set_timer(&mut self, exec: Uid, step: u32, kind: u8, attempt: u32, due: u64) {
        if let Some((_, _, old)) = self.tmap.insert((exec, step), (kind, attempt, due)) {
            self.timers.remove(&(old, exec, step));
        }
        self.timers.insert((due, exec, step));
        self.sh
            .timers_pending
            .store(self.tmap.len() as u64, Relaxed);
    }

    fn cancel_timer(&mut self, exec: Uid, step: u32) {
        if let Some((_, _, old)) = self.tmap.remove(&(exec, step)) {
            self.timers.remove(&(old, exec, step));
            self.sh
                .timers_pending
                .store(self.tmap.len() as u64, Relaxed);
        }
    }

    /// Prende in carico i timer scaduti (al più 4.096 per passata, per non fermare il ciclo): una scadenza di tentativo
    /// rimette l'attività in coda con il tentativo successivo, un'attesa di ritentativo la mette in coda con lo stesso
    /// tentativo, un passo `wait` conclude il passo (messaggio interno `WaitDone`, che il ciclo elabora). Nulla di durevole
    /// si scrive qui: l'effetto durevole è la consegna (timer di scadenza nuovo) o l'esito `Done`; se il motore muore prima, il
    /// timer è ancora nel database e riscatta alla ripresa.
    async fn fire_due(&mut self) -> Vec<Msg> {
        let now = now_us();
        let mut waits = vec![];
        let mut queued = false;
        let mut n = 0;
        while let Some(&(due, exec, step)) = self.timers.first() {
            if due > now || n >= 4096 {
                break;
            }
            self.timers.pop_first();
            let Some((kind, attempt, _)) = self.tmap.remove(&(exec, step)) else {
                continue;
            };
            n += 1;
            {
                let mut m = self.sh.m.lock().unwrap();
                m.timer_delay.record(now.saturating_sub(due).max(1)).ok();
            }
            if self.sh.timer_log {
                eprintln!("TFIRE {kind} {} {step} {attempt} {due} {now}", hex(&exec));
            }
            match kind {
                0 => {
                    // scadenza del tentativo: vale solo se è ancora quello in volo
                    if self.inflight.get(&(exec, step)).map(|f| f.attempt) == Some(attempt) {
                        self.inflight.remove(&(exec, step));
                        let ti = tenant_idx(&self.execs[&exec].tenant).unwrap();
                        self.tenant_inflight[ti] -= 1;
                        self.pending[ti].push_front((exec, step, now, attempt + 1));
                        self.sh.m.lock().unwrap().expired += 1;
                        queued = true;
                    }
                }
                1 => {
                    if let Some(e) = self.execs.get(&exec) {
                        let ti = tenant_idx(&e.tenant).unwrap();
                        self.pending[ti].push_back((exec, step, now, attempt));
                        queued = true;
                    }
                }
                _ => waits.push(Msg::WaitDone { exec, step }),
            }
        }
        self.sh
            .timers_pending
            .store(self.tmap.len() as u64, Relaxed);
        if queued {
            self.dispatch().await;
        }
        waits
    }

    /// Consegna le attività pronte ai worker, a turno tra i tenant, rispettando il tetto per tenant.
    async fn dispatch(&mut self) {
        // aggiornamento in corso: nessuna consegna nuova, le attività pronte si riconsegnano dopo la ripresa
        if self.sessions.is_empty() || self.sh.draining.load(Relaxed) {
            return;
        }
        let mut progress = true;
        while progress {
            progress = false;
            for ti in 0..2 {
                if self.tenant_inflight[ti] >= self.cap {
                    continue;
                }
                let Some((exec, step, ready_us, attempt)) = self.pending[ti].pop_front() else {
                    continue;
                };
                let Some(e) = self.execs.get(&exec) else {
                    continue;
                };
                // già concluso o già in volo (una ripresa e una scadenza possono accodare lo stesso passo): niente doppia consegna
                if e.done[step as usize].is_some() || self.inflight.contains_key(&(exec, step)) {
                    continue;
                }
                let kind = e.kind;
                // fan-out: solo il ramo 0 è lento, così le uscite degli altri rami sono già proiettate quando serve il join
                let (tenant, work_ms, out_size) = (
                    e.tenant,
                    if e.kind == 1 && step != 0 {
                        0
                    } else {
                        e.work_ms
                    },
                    e.out_size,
                );
                let refs: Vec<(Hash, Option<Loc>)> = e
                    .input_refs(step)
                    .into_iter()
                    .map(|h| {
                        (
                            h,
                            e.locs.get(&h).copied().or_else(|| {
                                self.sh
                                    .pending_locs
                                    .lock()
                                    .unwrap()
                                    .get(&(tenant, h))
                                    .copied()
                            }),
                        )
                    })
                    .collect();
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
                let (_, tx) = self.sessions[self.rr].clone();
                self.next_delivery += 1;
                let mut d = pb::Delivery {
                    exec: exec.to_vec(),
                    step,
                    tenant: tenant.to_vec(),
                    delivery_id: self.next_delivery,
                    input: input.to_vec(),
                    work_ms,
                    out_size,
                    ready_us,
                    sent_us: 0,
                    ack: 0,
                    attempt,
                    // kind 3: il primo tentativo di ogni passo fallisce
                    fail: kind == 3 && attempt == 0,
                    lease_ms: self.sh.lease_ms as u32,
                };
                // Blocco D, terza parte (H3): «invia prima, registra dopo, nel database». L'attività è già durevolmente pronta
                // (il gruppo di `Ready` o di un timer di ritentativo/scadenza che l'ha messa in coda è durevole: AD-2 vale per
                // quella decisione). La consegna parte subito e la scadenza si arma in memoria ora; il record di consegna
                // (esecuzione, passo, tentativo, scadenza) va al task scrittore, che lo scrive nella tabella `timer` in modo
                // asincrono: nessuno lo aspetta e non passa dal bus. Se il motore muore prima che il record sia durevole, alla
                // ripresa l'attività risulta «mai consegnata» e si riconsegna: doppione accettato e contato (`undurable_deliveries`).
                let now = now_us();
                let lease = self.sh.lease_ms;
                let due = now
                    + if lease > 0 {
                        lease
                    } else {
                        work_ms as u64 + self.sh.grace_ms
                    } * 1000;
                let u = self.sh.undurable.fetch_add(1, Relaxed) + 1;
                self.sh.undurable_max.fetch_max(u, Relaxed);
                let _ = self.rec_tx.send((exec, step, attempt, due));
                self.set_timer(exec, step, 0, attempt, due);
                if self.sh.timer_log {
                    eprintln!("TSET 0 {} {step} {attempt} {due} {}", hex(&exec), now_us());
                }
                d.sent_us = now_us();
                let _ = tx.try_send(Ok(d));
                self.inflight.insert(
                    (exec, step),
                    Infl {
                        delivery_id: self.next_delivery,
                        attempt,
                    },
                );
                self.tenant_inflight[ti] += 1;
                progress = true;
            }
        }
        // pronte ma trattenute dal tetto per tenant (con sessioni e senza aggiornamento in corso), per la riga `MIN`
        let held = (self.pending[0].len() + self.pending[1].len()) as u64;
        self.sh.held.store(held, Relaxed);
        self.sh.held_max.fetch_max(held, Relaxed);
    }

    /// Elabora un gruppo di messaggi: aggiorna lo stato e produce gli eventi da scrivere, i passi pronti e le risposte.
    /// Nessun effetto esterno qui: risposte e consegne partono solo quando il gruppo è durevole (AD-2).
    fn handle(
        &mut self,
        batch: Vec<Msg>,
    ) -> (
        Vec<(usize, Ev)>,
        Vec<(usize, Uid, u32, u64)>,
        Vec<(oneshot::Sender<Result<Uid, Status>>, Uid, Instant)>,
        Vec<(u64, u64)>,
    ) {
        self.bts = now_us();
        let mut evs: Vec<(usize, Ev)> = vec![];
        let mut newly = vec![];
        let mut replies = vec![];
        let mut acks = vec![];
        for msg in batch {
            match msg {
                Msg::Located(ti, v) => {
                    for (exec, h, loc) in v {
                        self.sh
                            .pending_locs
                            .lock()
                            .unwrap()
                            .remove(&(TENANTS[ti], h));
                        if let Some(e) = self.execs.get_mut(&exec) {
                            e.locs.insert(h, loc);
                        }
                    }
                }
                Msg::Register { sess, tx } => self.sessions.push((sess, tx)),
                // Blocco B: una sessione persa non rimette in coda le sue consegne (il worker può ricollegarsi e
                // consegnare l'esito): tornano disponibili solo alla scadenza del tentativo (`expire`).
                Msg::Unregister { sess } => self.sessions.retain(|s| s.0 != sess),
                Msg::Accept {
                    tenant,
                    req,
                    t0,
                    reply,
                } => {
                    if self.sh.full.load(Relaxed) > 0 {
                        self.sh.m.lock().unwrap().refused += 1;
                        let _ = reply.send(Err(Status::unavailable("E_STORAGE_FULL")));
                        continue;
                    }
                    if self.sh.bus_down.load(Relaxed) || self.sh.append_fail.load(Relaxed) > 0 {
                        self.sh.m.lock().unwrap().refused += 1;
                        let _ = reply.send(Err(Status::unavailable("E_BUS_DOWN")));
                        continue;
                    }
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
                    self.execs.insert(
                        exec,
                        Exec::new(
                            t,
                            req.kind as u8,
                            req.n,
                            req.work_ms,
                            req.out_size,
                            req.retry_ms,
                            h,
                        ),
                    );
                    evs.push((
                        tenant,
                        Ev::Accepted {
                            exec,
                            tenant: t,
                            kind: req.kind as u8,
                            n: req.n,
                            work_ms: req.work_ms,
                            out_size: req.out_size,
                            retry_ms: req.retry_ms,
                            ts,
                            input_hash: h,
                            input,
                        },
                    ));
                    self.decide(exec, &mut evs, &mut newly);
                    replies.push((reply, exec, t0));
                }
                Msg::Result { sess, r } => {
                    let exec: Uid = r.exec.as_slice().try_into().unwrap();
                    let key = (exec, r.step);
                    if r.heartbeat {
                        // Battito (blocco C): il tentativo è vivo, la sua scadenza si sposta in avanti con un nuovo evento `Timer`
                        // (stessa riga del passo, vince l'ultimo). Un battito di un tentativo che non è più quello in volo si ignora.
                        if let Some(f) = self.inflight.get(&key)
                            && (f.delivery_id == r.delivery_id || f.delivery_id == 0)
                            && let Some(e) = self.execs.get(&exec)
                        {
                            let (tenant, attempt) = (e.tenant, f.attempt);
                            let ti = tenant_idx(&tenant).unwrap();
                            let due = self.bts + self.sh.lease_ms * 1000;
                            evs.push((
                                ti,
                                Ev::Timer {
                                    exec,
                                    tenant,
                                    step: r.step,
                                    kind: 0,
                                    attempt,
                                    due_us: due,
                                    ts: self.bts,
                                },
                            ));
                            self.arm.push((exec, r.step, 0, attempt, due));
                            self.sh.heartbeats.fetch_add(1, Relaxed);
                        }
                        continue;
                    }
                    // C-2: vale solo il primo esito del tentativo corrente, dalla sessione a cui è stato consegnato
                    // Blocco B: l'esito vale da qualunque sessione (il worker può essersi ricollegato) se porta la
                    // consegna corrente, o se la consegna è di prima della ripresa (`delivery_id` 0 in `inflight`).
                    // Gli altri si scartano e si confermano (il worker smette di rimandarli).
                    match self.inflight.get(&key) {
                        Some(f) if f.delivery_id == r.delivery_id || f.delivery_id == 0 => {
                            if f.delivery_id == 0 {
                                self.sh.m.lock().unwrap().late_ok += 1;
                            }
                        }
                        _ => {
                            self.sh.m.lock().unwrap().rejected += 1;
                            self.ack_now(sess, r.delivery_id);
                            continue;
                        }
                    }
                    acks.push((sess, r.delivery_id));
                    let attempt = self.inflight.remove(&key).map_or(0, |f| f.attempt);
                    let e = self.execs.get_mut(&exec).unwrap();
                    let ti = tenant_idx(&e.tenant).unwrap();
                    self.tenant_inflight[ti] -= 1;
                    // l'esito (riuscito o fallito) chiude il tentativo: la sua scadenza non serve più
                    self.cancel_timer(exec, r.step);
                    if r.failed {
                        // Blocco C: tentativo fallito, attesa durevole prima del successivo. Il timer di ritentativo si scrive
                        // nello stesso gruppo che conferma l'esito al worker; sostituisce quello di scadenza nel database.
                        let e = &self.execs[&exec];
                        let (tenant, retry_ms) = (e.tenant, e.retry_ms);
                        let ts = self.bts;
                        let due = ts + retry_ms as u64 * 1000;
                        evs.push((
                            ti,
                            Ev::Timer {
                                exec,
                                tenant,
                                step: r.step,
                                kind: 1,
                                attempt: attempt + 1,
                                due_us: due,
                                ts,
                            },
                        ));
                        self.arm.push((exec, r.step, 1, attempt + 1, due));
                        continue;
                    }
                    let e = &self.execs[&exec];
                    let is_join = e.kind == 1 && r.step == e.n;
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
                    }
                    self.complete(exec, r.step, Bytes::from(r.output), &mut evs, &mut newly);
                }
                Msg::WaitDone { exec, step } => {
                    // il passo `wait` si conclude con un'uscita vuota (il timer è già tolto da `fire_due`)
                    if self
                        .execs
                        .get(&exec)
                        .is_some_and(|e| e.done[step as usize].is_none())
                    {
                        self.complete(exec, step, Bytes::new(), &mut evs, &mut newly);
                    }
                }
            }
        }
        (evs, newly, replies, acks)
    }

    /// Conclude un passo con la sua uscita: stato, evento `Done`, decisione dei passi che ne dipendono.
    fn complete(
        &mut self,
        exec: Uid,
        step: u32,
        output: Bytes,
        evs: &mut Vec<(usize, Ev)>,
        newly: &mut Vec<(usize, Uid, u32, u64)>,
    ) {
        let e = self.execs.get_mut(&exec).unwrap();
        let ti = tenant_idx(&e.tenant).unwrap();
        let h = sha(&output);
        let ts = self.bts;
        e.done[step as usize] = Some(h);
        let (tenant, last) = (e.tenant, step + 1 == e.steps());
        self.sh.put_cache(tenant, h, output.clone(), ts);
        self.sh.m.lock().unwrap().done += 1;
        evs.push((
            ti,
            Ev::Done {
                exec,
                tenant,
                step,
                last,
                ts,
                out_hash: h,
                output,
            },
        ));
        if !last {
            self.decide(exec, evs, newly);
        }
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
                let msgs: Vec<Bytes> = evs
                    .iter()
                    .filter(|(i, e)| *i == ti && e.exec_hash() % n as u64 == ci as u64)
                    .map(|(_, e)| self.sh.seal_ev(&topic, e))
                    .collect();
                if msgs.is_empty() {
                    continue;
                }
                let _ = self.jobs[ti * n + ci].send(Job {
                    gid,
                    topic: topic.clone(),
                    msgs,
                });
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
                m.min_append
                    .record(g.t0.elapsed().as_micros() as u64 + 1)
                    .ok();
            }
        }
        for (reply, exec, _) in g.replies {
            let _ = reply.send(Ok(exec));
        }
        // esiti durevoli: conferma ai worker; il primo dopo l'avvio o dopo il ritorno del bus si scrive nel log
        if !g.acks.is_empty() && self.sh.log_first.swap(false, Relaxed) {
            eprintln!("FIRST_DONE {}", now_us());
        }
        for (sess, id) in g.acks {
            self.ack_now(sess, id);
        }
        for (ti, exec, step, ts) in g.newly {
            self.pending[ti].push_back((exec, step, ts, 0));
        }
        // timer durevoli: ora che i loro eventi sono sul bus si armano in memoria, e solo dopo parte la consegna che li usa
        for (exec, step, kind, attempt, due) in g.arm {
            // solo se il timer serve ancora: il tentativo è ancora in volo (scadenza) o il passo non è concluso (attese)
            let live = if kind == 0 {
                self.inflight.get(&(exec, step)).map(|f| f.attempt) == Some(attempt)
            } else {
                self.execs
                    .get(&exec)
                    .is_some_and(|e| e.done[step as usize].is_none())
            };
            if live {
                self.set_timer(exec, step, kind, attempt, due);
                if self.sh.timer_log && kind != 2 {
                    eprintln!(
                        "TSET {kind} {} {step} {attempt} {due} {}",
                        hex(&exec),
                        now_us()
                    );
                }
            }
        }
        // le esecuzioni concluse escono dallo stato quando i loro eventi sono durevoli
        self.execs.retain(|_, e| !e.finished());
        self.dispatch().await;
    }

    /// Manda un gruppo di eventi al bus e lo registra tra quelli in volo (con i timer da armare e le consegne da inviare
    /// accumulati finora). Solo con almeno un evento: un gruppo senza eventi si finalizza subito, da chi lo ha creato.
    fn launch(
        &mut self,
        evs: Vec<(usize, Ev)>,
        newly: Vec<(usize, Uid, u32, u64)>,
        replies: Vec<(oneshot::Sender<Result<Uid, Status>>, Uid, Instant)>,
        acks: Vec<(u64, u64)>,
        intake: bool,
    ) {
        if intake {
            self.intake_n += 1;
        }
        self.gid += 1;
        let sent = self.submit(self.gid, &evs);
        let g = Group {
            remaining: sent,
            remaining_total: sent,
            newly,
            replies,
            acks,
            t0: Instant::now(),
            arm: std::mem::take(&mut self.arm),
            intake,
        };
        self.groups.insert(self.gid, g);
    }

    /// Conferma al worker che l'esito con questo `delivery_id` è stato ricevuto (durevole, o scartato come doppione).
    /// Se la sessione non c'è più il worker lo rimanderà al ricollegamento e lo riceverà come doppione.
    fn ack_now(&self, sess: u64, id: u64) {
        if let Some((_, tx)) = self.sessions.iter().find(|s| s.0 == sess) {
            let _ = tx.try_send(Ok(pb::Delivery {
                ack: id,
                ..Default::default()
            }));
        }
    }

    /// Posizione applicata uguale alla testa del bus su ogni topic (passo 1 dell'aggiornamento).
    async fn drained(&self) -> bool {
        let pos = self.sh.db.positions().await.unwrap();
        for t in &TENANTS {
            let topic = topic_of(t);
            if self.sh.bus.head(&topic).await.unwrap() != pos.get(&topic).copied().unwrap_or(0) {
                return false;
            }
        }
        true
    }

    /// Elabora messaggi (in ingresso o interni): eventi in un gruppo sul bus, oppure subito gli effetti se non c'è niente da scrivere.
    async fn process(&mut self, batch: Vec<Msg>) {
        let (evs, newly, replies, acks) = self.handle(batch);
        if evs.is_empty() {
            let g = Group {
                remaining: 0,
                remaining_total: 0,
                newly,
                replies,
                acks,
                t0: Instant::now(),
                arm: vec![],
                intake: false,
            };
            self.finalize(g).await;
        } else {
            self.launch(evs, newly, replies, acks, true);
        }
    }

    /// Ciclo del motore con fino a `k` gruppi in volo sul bus. Il ciclo non aspetta più la scrittura: continua a
    /// raccogliere messaggi e a decidere; gli effetti di un gruppo partono quando le sue richieste sono durevoli.
    async fn run(mut self, mut rx: mpsc::Receiver<Msg>, mut drx: mpsc::UnboundedReceiver<u64>) {
        // passata dei timer ogni 20 ms (blocco C; prima 100 ms per le sole scadenze dei tentativi)
        let mut sweep = tokio::time::interval(Duration::from_millis(20));
        loop {
            let mut batch = Vec::new();
            // Raccolta a gruppi pieni: con un gruppo già in volo se ne apre un altro solo con almeno `minb` messaggi in coda,
            // altrimenti si aspetta (raggruppamento naturale). Senza questa regola il ciclo emette tanti gruppi minuscoli,
            // la coda FIFO della connessione si riempie e la latenza di ogni gruppo cresce di k volte senza più throughput.
            // aggiornamento in corso: non si prende più niente dalla coda, si finiscono i gruppi in volo
            let draining = self.sh.draining.load(Relaxed);
            let can_take = !draining
                && self.intake_n < self.k
                && (self.intake_n == 0 || rx.len() >= self.minb);
            tokio::select! {
                n = rx.recv_many(&mut batch, self.maxb), if can_take => {
                    if n == 0 {
                        return;
                    }
                    self.process(batch).await;
                }
                _ = sweep.tick() => {
                    if !draining {
                        let waits = self.fire_due().await;
                        if !waits.is_empty() {
                            self.process(waits).await;
                        }
                    }
                }
                Some(id) = drx.recv() => {
                    let g = self.groups.get_mut(&id).unwrap();
                    g.remaining -= 1;
                    if g.remaining == 0 {
                        let g = self.groups.remove(&id).unwrap();
                        if g.intake {
                            self.intake_n -= 1;
                        }
                        self.finalize(g).await;
                    }
                }
                // riesamina la regola di raccolta quando i messaggi in coda crescono
                _ = tokio::time::sleep(Duration::from_micros(500)), if !self.groups.is_empty() && !can_take => {}
                // svuotamento completo: nessun gruppo in volo e posizione applicata uguale alla testa su ogni topic.
                // Da qui in poi il motore non scrive più sul bus (la coda non si consuma): si può uscire.
                _ = tokio::time::sleep(Duration::from_millis(20)) => {
                    // H3: anche la coda del task scrittore deve essere vuota e durevole (`undurable` = record inviati e non
                    // ancora confermati), prima che il motore nuovo riversi lo stato: i timer di consegna stanno solo nel database.
                    if draining && self.groups.is_empty() && self.sh.undurable.load(Relaxed) == 0 && self.drained().await {
                        // le risposte alle accettazioni già durevoli escono dal socket prima della fine del processo
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        eprintln!("DRAINED {} accepted={} in_flight_deliveries={}", now_us(), self.sh.m.lock().unwrap().accepted, self.inflight.len());
                        std::process::exit(0);
                    }
                }
            }
        }
    }
}

/// Finestra di raccolta del task scrittore (ms): i record si scrivono a lotti ogni 20 ms al più, così le transazioni del
/// proiettore non devono aspettare scritture continue (il gestore di blocco di SQLite non è equo) e un lotto è un commit.
const REC_WINDOW_MS: u64 = 20;

/// Task scrittore dei record di consegna (H3): raccoglie i record, li scrive a lotti nella tabella `timer` (stesso
/// `Db`, quindi stesse istruzioni contate) e solo dopo il commit li toglie da `undurable`. Nessuno lo aspetta.
/// Un errore del database si riprova (il lotto resta contato come non durevole). `delay_ms` è il ritardo di
/// guasto della prova del doppione provocato: i commit partono con quel ritardo in più (0 = nessuno).
async fn rec_writer(
    sh: Arc<Shared>,
    mut rx: mpsc::UnboundedReceiver<(Uid, u32, u32, u64)>,
    delay_ms: u64,
) {
    let mut batch = vec![];
    while rx.recv_many(&mut batch, 8192).await > 0 {
        tokio::time::sleep(Duration::from_millis(REC_WINDOW_MS + delay_ms)).await;
        while let Ok(r) = rx.try_recv() {
            batch.push(r);
        }
        let mut reported = false;
        while let Err(e) = sh.db.write_delivery_records(&batch).await {
            if !reported {
                eprintln!("REC_ERR {e}");
                reported = true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        sh.undurable.fetch_sub(batch.len() as u64, Relaxed);
        batch.clear();
    }
}

/// Gruppo di eventi in volo sul bus.
struct Group {
    remaining: usize,
    remaining_total: usize,
    newly: Vec<(usize, Uid, u32, u64)>,
    replies: Vec<(oneshot::Sender<Result<Uid, Status>>, Uid, Instant)>,
    /// Esiti accettati in questo gruppo (sessione, consegna): si confermano al worker quando il gruppo è durevole.
    acks: Vec<(u64, u64)>,
    t0: Instant,
    /// Timer da armare quando il gruppo è durevole (blocco C: ritentativi, attese, battiti).
    arm: Vec<(Uid, u32, u8, u32, u64)>,
    /// Gruppo nato da messaggi in ingresso (conta per il limite `k`), non da una consegna o da un timer.
    intake: bool,
}

/// Richiesta di scrittura per una coda (tenant, connessione).
struct Job {
    gid: u64,
    topic: String,
    msgs: Vec<Bytes>,
}

/// Scrive un gruppo di messaggi e torna solo quando sono tutti durevoli sul bus. Con il bus giù il client `iggy` non
/// torna con un errore: aspetta e ritenta da sé (misurato: una chiamata ha impiegato 8,4 s attraverso un crash); qui
/// ogni tentativo ha un tempo massimo, allo scadere si segna `append_fail` (le accettazioni si rifiutano con
/// `E_BUS_DOWN`) e, prima di riscrivere, si cerca nella coda del topic ciò che è già durevole: un'append senza risposta
/// può essere arrivata sul disco, e riscriverla duplicherebbe `Done` e `Ready`.
// ponytail: la ricerca parte dalla posizione del proiettore, quindi costa quanto il suo ritardo; una scrittura ambigua con
// il proiettore molto indietro rilegge molto. Soglia per un ritardo di minuti: posizione del gruppo salvata prima di scrivere.
async fn append_durable(bus: &Bus, sh: &Shared, topic: &str, msgs: &[Bytes]) {
    let limit = Duration::from_millis(sh.append_timeout_ms);
    let mut todo = msgs.to_vec();
    let mut failed = false;
    loop {
        // L'append non si annulla mai (annullarla lascia un'operazione incerta che blocca la partizione, v. `Bus::append`):
        // oltre il tempo massimo si segna soltanto `append_fail` (rifiuto di lavoro nuovo) e si continua ad aspettare.
        let r = {
            let fut = bus.append(topic, &todo);
            tokio::pin!(fut);
            tokio::select! {
                r = &mut fut => r,
                _ = tokio::time::sleep(limit) => {
                    if !failed {
                        failed = true;
                        sh.append_fail.fetch_add(1, Relaxed);
                    }
                    eprintln!("APPEND_SLOW {} {topic}: nessuna risposta in {} ms", now_us(), sh.append_timeout_ms);
                    fut.await
                }
            }
        };
        let Err(e) = r else { break };
        if !failed {
            failed = true;
            sh.append_fail.fetch_add(1, Relaxed);
        }
        eprintln!("APPEND_FAIL {} {topic}: {e}", now_us());
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(Ok(missing)) = tokio::time::timeout(
                Duration::from_secs(3),
                missing_from_tail(bus, sh, topic, &todo),
            )
            .await
            {
                todo = missing;
                break;
            }
        }
        if todo.is_empty() {
            break;
        }
    }
    if failed {
        sh.append_fail.fetch_sub(1, Relaxed);
        eprintln!(
            "APPEND_OK {} {topic}: {} messaggi riscritti, {} già durevoli",
            now_us(),
            todo.len(),
            msgs.len() - todo.len()
        );
    }
}

/// I messaggi di `todo` che non sono già nella coda del topic (dalla posizione del proiettore), nell'ordine dato.
async fn missing_from_tail(
    bus: &Bus,
    sh: &Shared,
    topic: &str,
    todo: &[Bytes],
) -> crate::bus::Res<Vec<Bytes>> {
    let ti = TENANTS
        .iter()
        .position(|t| topic_of(t) == topic)
        .ok_or("topic sconosciuto")?;
    let mut present = std::collections::HashSet::new();
    let mut p = sh.proj_pos[ti].load(Relaxed);
    loop {
        let r = bus.read(topic, p, 1000).await?;
        let Some(last) = r.last().map(|m| m.0) else {
            break;
        };
        present.extend(r.into_iter().map(|(_, b)| b));
        p = last + 1;
    }
    Ok(todo
        .iter()
        .filter(|m| !present.contains(*m))
        .cloned()
        .collect())
}

/// Una connessione al bus e una coda FIFO: scrive le richieste in ordine e segnala il gruppo quando sono durevoli.
async fn conn_worker(
    bus: Bus,
    mut rx: mpsc::UnboundedReceiver<Job>,
    dtx: mpsc::UnboundedSender<u64>,
    sh: Arc<Shared>,
) {
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
            append_durable(&bus, &sh, &j.topic, &j.msgs[from..to]).await;
            from = to;
        }
        {
            let mut m = sh.m.lock().unwrap();
            m.append_sub
                .record(ts.elapsed().as_micros() as u64 + 1)
                .ok();
            m.min_sub.record(ts.elapsed().as_micros() as u64 + 1).ok();
            m.sub_events.record(nev).ok();
        }
        let _ = dtx.send(j.gid);
    }
}

// ---------------------------------------------------------------- ripresa

/// Ricostruisce lo stato: database fino alla posizione applicata, poi rilettura del bus da lì.
/// Torna le esecuzioni vive, i timer in sospeso (database + coda del bus) e, per ogni attività emessa e non conclusa, il suo timer
/// (se ne ha uno): la ripresa deduce da lì se è in volo (scadenza), in attesa di ritentare o ancora da consegnare (nessun timer).
async fn recover(
    sh: &Arc<Shared>,
) -> (
    HashMap<Uid, Exec>,
    HashMap<(Uid, u32), (u8, u32, u64)>,
    Vec<(usize, Uid, u32)>,
) {
    let mut execs = sh
        .db
        .live()
        .await
        .expect("lettura dello stato dal database");
    let mut timers = sh
        .db
        .timers()
        .await
        .expect("lettura dei timer dal database");
    let pos = sh.db.positions().await.unwrap();
    let mut replayed = 0u64;
    for (ti, t) in TENANTS.iter().enumerate() {
        let topic = topic_of(t);
        let mut p = pos.get(&topic).copied().unwrap_or(0);
        loop {
            let r = sh
                .bus
                .read(&topic, p, 1000)
                .await
                .expect("rilettura del bus");
            if r.is_empty() {
                break;
            }
            for (pp, raw) in &r {
                let (_, _, plain) = sh.keys.open(&format!("bus/{topic}"), raw);
                match Ev::decode(&plain) {
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
                        input,
                    } => {
                        sh.put_cache(tenant, input_hash, input, ts);
                        execs.entry(exec).or_insert_with(|| {
                            Exec::new(tenant, kind, n, work_ms, out_size, retry_ms, input_hash)
                        });
                    }
                    Ev::Timer {
                        exec,
                        step,
                        kind,
                        attempt,
                        due_us,
                        ..
                    } => {
                        // stessa regola d'ordine del proiettore (`Db::apply`): un evento non ancora proiettato non sostituisce
                        // il record di consegna del database (H3) se ha un tentativo più vecchio, o lo stesso con un tipo più alto
                        if timers
                            .get(&(exec, step))
                            .is_none_or(|&(k, a, _)| attempt > a || (attempt == a && kind <= k))
                        {
                            timers.insert((exec, step), (kind, attempt, due_us));
                        }
                    }
                    Ev::Ready { exec, step, .. } => {
                        if let Some(e) = execs.get_mut(&exec) {
                            e.issued[step as usize] = true;
                        }
                    }
                    Ev::Done {
                        exec,
                        tenant,
                        step,
                        ts,
                        out_hash,
                        output,
                        ..
                    } => {
                        timers.remove(&(exec, step));
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
    timers.retain(|(id, _), _| execs.contains_key(id));
    // attività emesse e non concluse: la ripresa le classifica con il loro timer (v. `main`)
    let redeliver: Vec<(usize, Uid, u32)> = execs
        .iter()
        .flat_map(|(id, e)| {
            (0..e.steps())
                .filter(|&s| e.issued[s as usize] && e.done[s as usize].is_none())
                .map(move |s| (tenant_idx(&e.tenant).unwrap(), *id, s))
        })
        .collect();
    eprintln!(
        "ripresa: {} esecuzioni vive, {replayed} eventi riletti dal bus, {} attività emesse e non concluse, {} timer",
        execs.len(),
        redeliver.len(),
        timers.len()
    );
    (execs, timers, redeliver)
}

// ---------------------------------------------------------------- proiettore

/// Errore di archivio o database nel proiettore: si segna `failing` (le accettazioni si rifiutano con
/// `E_PROJECTION_DOWN`) e, se il testo dice spazio finito, anche `full` (`E_STORAGE_FULL`). Il log riporta ogni passaggio una volta.
fn storage_fail(sh: &Shared, failing: &mut bool, full: &mut bool, topic: &str, e: &str) {
    let lower = e.to_lowercase();
    let is_full = lower.contains("no space")
        || lower.contains("disk is full")
        || lower.contains("could not extend");
    if !*failing {
        *failing = true;
        sh.failing.fetch_add(1, Relaxed);
        eprintln!("proiezione ferma su {topic} {}: {e}", now_us());
    }
    if is_full && !*full {
        *full = true;
        sh.full.fetch_add(1, Relaxed);
        eprintln!("STORAGE_FULL {} {topic}: {e}", now_us());
    }
}

async fn projector(sh: Arc<Shared>, ti: usize, start: u64, tx: mpsc::Sender<Msg>) {
    // connessione propria del proiettore: le letture non si mettono in coda dietro le scritture del motore
    let bus = Bus::connect(sh.bus.name()).await.unwrap();
    let topic = topic_of(&TENANTS[ti]);
    let mut pos = start;
    sh.proj_pos[ti].store(pos, Relaxed);
    let mut last_trim = Instant::now();
    let (mut failing, mut full) = (false, false);
    loop {
        let tr = Instant::now();
        let raw = match bus.read(&topic, pos, sh.proj_batch).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("lettura del bus fallita: {e}");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };
        if !raw.is_empty() {
            sh.m.lock()
                .unwrap()
                .min_pread
                .record(tr.elapsed().as_micros() as u64 + 1)
                .ok();
        }
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
        // Prova della conservazione (fase 4): la posizione applicata più alta è `pos`; se la prima lettura
        // non parte esattamente da lì, una posizione non ancora applicata è stata cancellata (o saltata).
        if bus.name() == "iggy" && raw[0].0 != pos {
            eprintln!(
                "E_POSIZIONE_PERSA: {topic} letto da {pos}, primo offset disponibile {}",
                raw[0].0
            );
            std::process::exit(4);
        }
        let next = raw.last().unwrap().0 + 1;
        let evs: Vec<Ev> = raw
            .iter()
            .map(|(_, b)| Ev::decode(&sh.keys.open(&format!("bus/{topic}"), b).2))
            .collect();
        // payload nel segmento del tenant e un solo fdatasync PRIMA della transazione (la posizione fa scartare il bus)
        let noarc = std::env::var("DIAG_NOARCHIVE").is_ok();
        let ta = Instant::now();
        // errore dell'archivio (disco pieno): si ritenta lo stesso gruppo rifiutando lavoro nuovo, come per il database
        let (locs, located) = loop {
            let (sh2, evs2) = (sh.clone(), evs.clone());
            let r = tokio::task::spawn_blocking(move || {
                if noarc {
                    return Ok((HashMap::new(), vec![]));
                }
                let items: Vec<(Uid, Hash, Bytes)> = evs2
                    .iter()
                    .filter_map(|e| match e {
                        Ev::Accepted {
                            tenant,
                            input_hash,
                            input,
                            ..
                        } => Some((*tenant, *input_hash, input.clone())),
                        Ev::Done {
                            tenant,
                            out_hash,
                            output,
                            ..
                        } => Some((*tenant, *out_hash, output.clone())),
                        _ => None,
                    })
                    .collect();
                let locs = sh2.archive_put_group(ti, &items)?;
                // posizioni per il motore: solo per le esecuzioni ancora vive (le altre le scarta lui)
                let located = evs2
                    .iter()
                    .filter_map(|e| match e {
                        Ev::Accepted {
                            exec, input_hash, ..
                        } => Some((*exec, *input_hash, locs[input_hash])),
                        Ev::Done { exec, out_hash, .. } => Some((*exec, *out_hash, locs[out_hash])),
                        _ => None,
                    })
                    .collect();
                std::io::Result::Ok((locs, located))
            })
            .await
            .unwrap();
            match r {
                Ok(v) => break v,
                Err(e) => {
                    storage_fail(
                        &sh,
                        &mut failing,
                        &mut full,
                        &topic,
                        &format!("archivio: {e}"),
                    );
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        };
        sh.m.lock()
            .unwrap()
            .min_parc
            .record(ta.elapsed().as_micros() as u64 + 1)
            .ok();
        {
            let mut p = sh.pending_locs.lock().unwrap();
            for (_, h, l) in &located {
                p.insert((TENANTS[ti], *h), *l);
            }
        }
        let _ = tx.send(Msg::Located(ti, located)).await;
        // transazione; con il database giù si ritenta per sempre, rifiutando lavoro nuovo (AD-2)
        let td = Instant::now();
        loop {
            match sh.db.apply(&topic, next, &evs, &locs).await {
                Ok(()) => break,
                Err(e) => {
                    storage_fail(
                        &sh,
                        &mut failing,
                        &mut full,
                        &topic,
                        &format!("database: {e}"),
                    );
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        }
        sh.m.lock()
            .unwrap()
            .min_pdb
            .record(td.elapsed().as_micros() as u64 + 1)
            .ok();
        if failing {
            failing = false;
            sh.failing.fetch_sub(1, Relaxed);
            if full {
                full = false;
                sh.full.fetch_sub(1, Relaxed);
            }
            sh.log_first.store(true, Relaxed);
            eprintln!("proiezione ripresa su {topic} {}", now_us());
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
                    Ev::Accepted {
                        tenant, input_hash, ..
                    } => c.remove(&(*tenant, *input_hash)),
                    Ev::Done {
                        tenant, out_hash, ..
                    } => c.remove(&(*tenant, *out_hash)),
                    _ => None,
                };
            }
        }
        pos = next;
        sh.proj_pos[ti].store(pos, Relaxed);
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
    async fn accept(
        &self,
        r: tonic::Request<pb::AcceptReq>,
    ) -> Result<tonic::Response<pb::AcceptResp>, Status> {
        let t0 = Instant::now();
        let req = r.into_inner();
        if self.sh.draining.load(Relaxed) {
            self.sh.m.lock().unwrap().refused += 1;
            return Err(Status::unavailable("E_UPGRADE_DRAIN"));
        }
        let tenant = tenant_idx(&req.tenant).ok_or_else(|| Status::invalid_argument("E_TENANT"))?;
        if req.n == 0 || req.n > 10_000 || req.kind > 3 {
            return Err(Status::invalid_argument("E_SHAPE"));
        }
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Msg::Accept {
                tenant,
                req,
                t0,
                reply,
            })
            .await
            .map_err(|_| Status::unavailable("E_ENGINE"))?;
        let exec = rx.await.map_err(|_| Status::unavailable("E_ENGINE"))??;
        Ok(tonic::Response::new(pb::AcceptResp {
            exec: exec.to_vec(),
        }))
    }

    async fn stats(
        &self,
        r: tonic::Request<pb::StatsReq>,
    ) -> Result<tonic::Response<pb::StatsResp>, Status> {
        let wal = self.sh.db.wal_bytes().await.unwrap_or(0);
        let mut m = self.sh.m.lock().unwrap();
        let secs = m.since.elapsed().as_secs_f64();
        let f = |h: &Histogram<u64>| {
            format!(
                "n={} p50={:.2}ms p99={:.2}ms p99.9={:.2}ms max={:.2}ms over100ms={}",
                h.len(),
                h.value_at_quantile(0.5) as f64 / 1e3,
                h.value_at_quantile(0.99) as f64 / 1e3,
                h.value_at_quantile(0.999) as f64 / 1e3,
                h.max() as f64 / 1e3,
                h.count_between(100_000, u64::MAX)
            )
        };
        let stm = self.sh.db.stmts.load(Relaxed) - m.stmts0;
        let rows = self.sh.db.rows.load(Relaxed) - m.rows0;
        let text = format!(
            "secs={secs:.1} accepted={} refused={} done={} rejected={} groups={} expired={} late_ok={}\nactivities_per_s={:.0} accepted_per_s={:.0}\naccept_engine {}\nappend {}\nappend_sub {}\nsub_events_p50={} sub_events_p99={}\nstart_ready {}\nstart_sent {}\nstart_join {}\nstart_ready_tenant_b {}\narchive_read {}\nproj_lag {}\ntimer_fire_delay {} timers_pending={} heartbeats={}\nundurable_deliveries={} undurable_max={} held={}\nsql_stmts={stm} sql_rows={rows} sql_per_activity={:.4} rows_per_activity={:.3} wal_bytes_per_activity={:.1}\nengine_cpu_s={:.2} cache_entries={}",
            m.accepted,
            m.refused,
            m.done,
            m.rejected,
            m.groups,
            m.expired,
            m.late_ok,
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
            f(&m.timer_delay),
            self.sh.timers_pending.load(Relaxed),
            self.sh.heartbeats.load(Relaxed),
            self.sh.undurable.load(Relaxed),
            self.sh.undurable_max.load(Relaxed),
            self.sh.held.load(Relaxed),
            stm as f64 / m.done.max(1) as f64,
            rows as f64 / m.done.max(1) as f64,
            (wal - m.wal0) as f64 / m.done.max(1) as f64,
            cpu_s() - m.cpu0,
            self.sh.cache.lock().unwrap().len(),
        );
        if r.into_inner().reset {
            let (stmts, rows) = (
                self.sh.db.stmts.load(Relaxed),
                self.sh.db.rows.load(Relaxed),
            );
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
    /// Impronte delle credenziali dei worker note al motore (C-11).
    creds: Vec<[u8; 32]>,
}

#[tonic::async_trait]
impl pb::worker_server::Worker for Wrk {
    type SessionStream = tokio_stream::wrappers::ReceiverStream<Result<pb::Delivery, Status>>;

    async fn session(
        &self,
        r: tonic::Request<tonic::Streaming<pb::FromWorker>>,
    ) -> Result<tonic::Response<Self::SessionStream>, Status> {
        let mut inb = r.into_inner();
        // la credenziale nel saluto si controlla a ogni collegamento, prima di registrare la sessione
        let h = inb
            .message()
            .await?
            .ok_or_else(|| Status::unauthenticated("E_CREDENZIALE"))?;
        let hh: [u8; 32] = sha2::Sha256::digest(&h.credential).into();
        if !h.hello || !self.creds.contains(&hh) {
            return Err(Status::unauthenticated("E_CREDENZIALE"));
        }
        let sess = self.next.fetch_add(1, Relaxed);
        let (dtx, drx) = mpsc::channel(8192);
        self.tx
            .send(Msg::Register { sess, tx: dtx })
            .await
            .map_err(|_| Status::unavailable("E_ENGINE"))?;
        let tx = self.tx.clone();
        tokio::spawn(async move {
            while let Ok(Some(m)) = inb.message().await {
                if !m.hello {
                    let _ = tx.send(Msg::Result { sess, r: m }).await;
                }
            }
            let _ = tx.send(Msg::Unregister { sess }).await;
        });
        Ok(tonic::Response::new(
            tokio_stream::wrappers::ReceiverStream::new(drx),
        ))
    }
}

// ---------------------------------------------------------------- aggiornamento del bus

/// Passo 2 dell'aggiornamento: riscrive sul bus nuovo e vuoto lo stato vivo (esecuzioni non concluse con i
/// loro passi emessi e conclusi), letto dal database e dall'archivio dei payload. Poi, in una transazione,
/// porta le posizioni salvate alla nuova generazione, subito dopo gli eventi riscritti: il database ne
/// rappresenta già lo stato, quindi il proiettore non li riapplica (un `Done` riapplicato conterrebbe due volte).
/// Un crash prima della transazione lascia il bus nuovo non vuoto e la generazione del database vecchia:
/// al riavvio i topic si svuotano e si ricomincia (ponytail: riversamento ripetibile, non ripresa a metà).
async fn rewrite_live(sh: &Arc<Shared>, bgen: u64) {
    let t0 = Instant::now();
    let execs = sh.db.live().await.expect("lettura dello stato vivo");
    let ts = now_us();
    let mut per: [Vec<Bytes>; 2] = [vec![], vec![]];
    let (mut n_exec, mut n_ready, mut n_done) = (0u64, 0u64, 0u64);
    for (id, e) in &execs {
        let ti = tenant_idx(&e.tenant).unwrap();
        let topic = topic_of(&e.tenant);
        let input = sh
            .archive_get(&e.tenant, &e.input_hash, e.locs[&e.input_hash])
            .await;
        per[ti].push(sh.seal_ev(
            &topic,
            &Ev::Accepted {
                exec: *id,
                tenant: e.tenant,
                kind: e.kind,
                n: e.n,
                work_ms: e.work_ms,
                out_size: e.out_size,
                retry_ms: e.retry_ms,
                ts,
                input_hash: e.input_hash,
                input,
            },
        ));
        n_exec += 1;
        for s in 0..e.steps() {
            if !e.issued[s as usize] {
                continue;
            }
            per[ti].push(sh.seal_ev(
                &topic,
                &Ev::Ready {
                    exec: *id,
                    tenant: e.tenant,
                    step: s,
                    last: s + 1 == e.steps(),
                    ts,
                },
            ));
            n_ready += 1;
            if let Some(h) = e.done[s as usize] {
                let output = sh.archive_get(&e.tenant, &h, e.locs[&h]).await;
                per[ti].push(sh.seal_ev(
                    &topic,
                    &Ev::Done {
                        exec: *id,
                        tenant: e.tenant,
                        step: s,
                        last: s + 1 == e.steps(),
                        ts,
                        out_hash: h,
                        output,
                    },
                ));
                n_done += 1;
            }
        }
    }
    // timer in sospeso (blocco C): ogni riga del database torna un evento `Timer` con la scadenza originale
    let mut n_timers = 0u64;
    for ((id, step), (kind, attempt, due)) in sh.db.timers().await.expect("lettura dei timer") {
        let Some(e) = execs.get(&id) else { continue };
        let ti = tenant_idx(&e.tenant).unwrap();
        per[ti].push(sh.seal_ev(
            &topic_of(&e.tenant),
            &Ev::Timer {
                exec: id,
                tenant: e.tenant,
                step,
                kind,
                attempt,
                due_us: due,
                ts,
            },
        ));
        n_timers += 1;
    }
    // i due topic si scrivono in parallelo su due connessioni (con 100.000 esecuzioni vive la scrittura in serie dominava la pausa)
    let (t0n, t1n) = (topic_of(&TENANTS[0]), topic_of(&TENANTS[1]));
    let (r0, r1) = tokio::join!(
        append_all(&sh.bus_w[0], &t0n, &per[0]),
        append_all(&sh.bus_w[1 % sh.bus_w.len()], &t1n, &per[1])
    );
    r0.expect("scrittura dello stato vivo sul bus nuovo");
    r1.expect("scrittura dello stato vivo sul bus nuovo");
    let mut positions = vec![];
    for t in &TENANTS {
        let topic = topic_of(t);
        positions.push((topic.clone(), sh.bus.head(&topic).await.unwrap()));
    }
    sh.db
        .reset_positions(bgen, &positions)
        .await
        .expect("posizioni della nuova generazione");
    eprintln!(
        "RIVERSAMENTO gen={bgen}: {n_exec} esecuzioni vive, {n_ready} Ready, {n_done} Done, {n_timers} timer riscritti in {} ms, posizioni {positions:?}",
        t0.elapsed().as_millis()
    );
}

/// Scrive i messaggi in richieste da 1.024, in ordine.
async fn append_all(bus: &Bus, topic: &str, msgs: &[Bytes]) -> crate::bus::Res<()> {
    for ch in msgs.chunks(1024) {
        bus.append(topic, ch).await?;
    }
    Ok(())
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

pub async fn main(
    bus_kind: &str,
    db_url: &str,
    listen: &str,
    dir: &str,
    cap: usize,
    proj_batch: u32,
    trim: bool,
    window_ms: u64,
    wconn: usize,
    kgroups: usize,
    maxb: usize,
    minb: usize,
    link: &str,
    iroh_listen: &str,
    busgen: u64,
    upgrade: bool,
    grace_ms: u64,
    append_timeout_ms: u64,
    lease_ms: u64,
    rec_delay_ms: u64,
) {
    let t_start = Instant::now();
    // AD-2: un solo motore; il lock si libera da solo se il processo muore (kill -9)
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(format!("{dir}/engine.lock"))
        .unwrap();
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
    // Generazione del bus (fase 4): gli offset ripartono da zero a ogni bus nuovo, quindi la posizione salvata
    // vale solo con la generazione del bus su cui è stata presa. Bus vuoto: nasce con `busgen`.
    let bus_gen = bus.generation(busgen).await.unwrap();
    let rewrite = match db.generation().await.unwrap() {
        None => false,
        Some(d) if d == bus_gen => false,
        Some(d) if upgrade && bus_gen > d => true,
        Some(d) => {
            eprintln!(
                "E_BUS_GENERATION: posizioni del database di generazione {d}, bus di generazione {bus_gen} (upgrade={upgrade}): il motore non parte"
            );
            std::process::exit(5);
        }
    };
    db.generation.store(bus_gen, Relaxed);
    if rewrite {
        // un riversamento interrotto lascia eventi sul bus nuovo: si riparte da topic vuoti
        for t in &TENANTS {
            let topic = topic_of(t);
            if bus.head(&topic).await.unwrap() > 0 {
                eprintln!("bus nuovo non vuoto su {topic}: si svuota e si rifà il riversamento");
                bus.drop_topic(&topic).await.unwrap();
                bus.ensure_topic(&topic).await.unwrap();
            }
        }
    }
    let sh = Arc::new(Shared {
        keys,
        bus_w: futures_n(bus_kind, 2 * wconn).await,
        pending_locs: Mutex::new(HashMap::new()),
        segs: [0, 1].map(|_| {
            Mutex::new(Seg {
                file: None,
                id: 0,
                len: 0,
                opened: Instant::now(),
            })
        }),
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
        draining: AtomicBool::new(false),
        bus_down: AtomicBool::new(false),
        append_fail: 0.into(),
        full: 0.into(),
        log_first: AtomicBool::new(true),
        proj_pos: [0.into(), 0.into()],
        grace_ms,
        append_timeout_ms,
        timers_pending: 0.into(),
        timer_log: std::env::var("TIMER_LOG").is_ok(),
        lease_ms,
        heartbeats: 0.into(),
        undurable: 0.into(),
        undurable_max: 0.into(),
        held: 0.into(),
        held_max: 0.into(),
    });
    // Sonda del bus (blocco B): una connessione propria chiede il topic ogni 500 ms con tempo massimo di 1,5 s; il client
    // non torna con un errore a bus giù (aspetta), quindi è il tempo massimo a dire «giù». Con il bus giù le accettazioni
    // si rifiutano con `E_BUS_DOWN`; al ritorno `log_first` riarma la riga `FIRST_DONE`.
    if bus_kind == "iggy" {
        let sh = sh.clone();
        let mon = Bus::connect(bus_kind).await.unwrap();
        tokio::spawn(async move {
            let topic = topic_of(&TENANTS[0]);
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let up = matches!(
                    tokio::time::timeout(Duration::from_millis(1500), mon.head(&topic)).await,
                    Ok(Ok(_))
                );
                if !up && !sh.bus_down.swap(true, Relaxed) {
                    eprintln!("BUS_DOWN {}", now_us());
                } else if up && sh.bus_down.swap(false, Relaxed) {
                    sh.log_first.store(true, Relaxed);
                    eprintln!("BUS_UP {}", now_us());
                }
            }
        });
    }
    // SIGUSR1 = inizio dell'aggiornamento: rifiuto delle accettazioni con codice e svuotamento (passo 1)
    {
        let sh = sh.clone();
        tokio::spawn(async move {
            let mut s =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())
                    .unwrap();
            s.recv().await;
            eprintln!("DRAIN_START {}", now_us());
            sh.draining.store(true, Relaxed);
        });
    }
    if rewrite {
        rewrite_live(&sh, bus_gen).await;
    }
    let (tx, rx) = mpsc::channel(65536);
    let (execs, timers, redeliver) = recover(&sh).await;
    let pos = sh.db.positions().await.unwrap();
    for ti in 0..2 {
        tokio::spawn(projector(
            sh.clone(),
            ti,
            pos.get(&topic_of(&TENANTS[ti])).copied().unwrap_or(0),
            tx.clone(),
        ));
    }
    // riga al minuto per la diagnosi (fase 2d): attività/s, scrittura sul bus, ritardo del proiettore, pressione I/O
    let sh3 = sh.clone();
    tokio::spawn(async move {
        let (mut last_done, mut t) = (0u64, 0u32);
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            t += 1;
            let io = std::fs::read_to_string("/proc/pressure/io")
                .ok()
                .and_then(|s| s.lines().next().map(String::from))
                .unwrap_or_default();
            let mut m = sh3.m.lock().unwrap();
            let d = m.done - last_done.min(m.done);
            last_done = m.done;
            let ms = |h: &Histogram<u64>, q: f64| h.value_at_quantile(q) as f64 / 1e3;
            eprintln!(
                "MIN t={t} act_s={:.0} append_p50_ms={:.1} sub_p50_ms={:.1} lag_p50_ms={:.0} lag_p99_ms={:.0} proj_read_p50_ms={:.1} proj_arch_p50_ms={:.1} proj_db_p50_ms={:.1} proj_groups={} held_max={} io=[{io}]",
                d as f64 / 60.0,
                ms(&m.min_append, 0.5),
                ms(&m.min_sub, 0.5),
                ms(&m.min_lag, 0.5),
                ms(&m.min_lag, 0.99),
                ms(&m.min_pread, 0.5),
                ms(&m.min_parc, 0.5),
                ms(&m.min_pdb, 0.5),
                m.min_pdb.len(),
                sh3.held_max.swap(0, Relaxed)
            );
            m.min_append.reset();
            m.min_sub.reset();
            m.min_lag.reset();
            m.min_pread.reset();
            m.min_parc.reset();
            m.min_pdb.reset();
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
    let (rec_tx, rec_rx) = mpsc::unbounded_channel();
    tokio::spawn(rec_writer(sh.clone(), rec_rx, rec_delay_ms));
    let mut eng = Engine {
        rec_tx,
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
        timers: BTreeSet::new(),
        tmap: HashMap::new(),
        arm: vec![],
        groups: HashMap::new(),
        gid: 0,
        intake_n: 0,
    };
    // Blocco C: ogni attività emessa e non conclusa si classifica con il suo timer durevole. Scadenza di tentativo = in volo
    // presso un worker (che può ricollegarsi con l'esito, consegna 0): l'esito vale fino alla scadenza originale, poi si rifà.
    // Attesa di ritentativo o di `wait` = il timer scatta alla sua scadenza (se è già passata, alla prima passata dopo la ripresa).
    // Nessun timer = mai consegnata (la consegna scrive prima il suo timer): si consegna subito. Nel blocco B restava «in volo»
    // per `work_ms` + margine dalla ripresa.
    let n_recovered = redeliver.len();
    let (mut n_fly, mut n_wait, mut n_new) = (0, 0, 0);
    for (ti, exec, step) in redeliver {
        match timers.get(&(exec, step)).copied() {
            Some((0, attempt, due)) => {
                eng.inflight.insert(
                    (exec, step),
                    Infl {
                        delivery_id: 0,
                        attempt,
                    },
                );
                eng.tenant_inflight[ti] += 1;
                // una scadenza già passata (o vicina) si sposta in avanti di `grace_ms` dalla ripresa, solo in memoria: il worker che
                // ha finito durante la pausa deve poter ricollegarsi e consegnare l'esito prima che il tentativo si rifaccia
                eng.set_timer(exec, step, 0, attempt, due.max(now_us() + grace_ms * 1000));
                n_fly += 1;
            }
            Some((kind, attempt, due)) => {
                eng.set_timer(exec, step, kind, attempt, due);
                n_wait += 1;
            }
            None if eng.execs[&exec].kind == 2 => {
                eng.set_timer(exec, step, 2, 0, now_us());
                n_wait += 1;
            }
            None => {
                eng.pending[ti].push_back((exec, step, now_us(), 0));
                n_new += 1;
            }
        }
    }
    eprintln!(
        "ripresa: {n_recovered} attività emesse e non concluse: {n_fly} in volo (scadenza ripristinata), {n_wait} in attesa di un timer, {n_new} da consegnare subito"
    );
    // decisioni mancanti dopo la ripresa (Ready non scritti)
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
        eng.pending[ti].push_back((exec, step, ts, 0));
    }
    for (exec, step, kind, attempt, due) in std::mem::take(&mut eng.arm) {
        eng.set_timer(exec, step, kind, attempt, due);
    }
    tokio::spawn(eng.run(rx, drx));

    let cert = std::fs::read(format!("{dir}/tls/cert.pem")).unwrap();
    let key = std::fs::read(format!("{dir}/tls/key.pem")).unwrap();
    // credenziali dei worker (C-11) e, con iroh, collegamento QUIC: identità del nodo del motore fissata da file, relay e scoperta spenti
    let creds: Vec<Vec<u8>> = std::fs::read_to_string(format!("{dir}/worker.cred"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().as_bytes().to_vec())
        .collect();
    let cred_hashes: Vec<[u8; 32]> = creds
        .iter()
        .map(|c| sha2::Sha256::digest(c).into())
        .collect();
    if link == "iroh" {
        let kp = format!("{dir}/iroh-engine.key");
        let sk = match std::fs::read(&kp) {
            Ok(b) if b.len() == 32 => iroh::SecretKey::from_bytes(&b.try_into().unwrap()),
            _ => {
                let k = iroh::SecretKey::generate();
                std::fs::write(&kp, k.to_bytes()).unwrap();
                k
            }
        };
        let ep = crate::iroh_link::endpoint(sk, iroh_listen).await;
        std::fs::write(format!("{dir}/engine.nodeid"), ep.id().to_string()).unwrap();
        let allowed: std::collections::HashMap<iroh::EndpointId, [u8; 32]> = creds
            .iter()
            .map(|c| {
                (
                    crate::iroh_link::key_from_cred(c).public(),
                    sha2::Sha256::digest(c).into(),
                )
            })
            .collect();
        tokio::spawn(crate::iroh_link::serve(
            ep,
            std::sync::Arc::new(allowed),
            tx.clone(),
            std::sync::Arc::new(AtomicU64::new(1)),
        ));
    }
    let tls = tonic::transport::ServerTlsConfig::new()
        .identity(tonic::transport::Identity::from_pem(cert, key));
    println!(
        "READY recovered_ms={} now_us={} gen={bus_gen}",
        t_start.elapsed().as_millis(),
        now_us()
    );
    tonic::transport::Server::builder()
        .tls_config(tls)
        .unwrap()
        .add_service(
            pb::control_server::ControlServer::new(Ctl {
                tx: tx.clone(),
                sh: sh.clone(),
            })
            .max_decoding_message_size(64 << 20),
        )
        .add_optional_service((link == "grpc").then(|| {
            pb::worker_server::WorkerServer::new(Wrk {
                tx: tx.clone(),
                next: 1.into(),
                creds: cred_hashes.clone(),
            })
            .max_decoding_message_size(64 << 20)
            .max_encoding_message_size(64 << 20)
        }))
        .serve(listen.parse().unwrap())
        .await
        .unwrap();
}
