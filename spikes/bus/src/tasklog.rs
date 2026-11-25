//! Log dei task (FR-59), blocco D della fase 4. Il motore è l'unico scrittore dei pezzi (AD-17).
//!
//! Percorso di una riga: il worker la scrive nella sessione gRPC con un numero progressivo per tentativo (`seq`, lo dà lui) →
//! il motore sostituisce il valore esatto del segreto consegnato col segnaposto → tetto per tentativo (righe e byte) → le righe
//! del tentativo si raggruppano in un pezzo, chiuso al più dopo `close_ms` → il pezzo si cifra per tenant nel formato di C-12 e
//! si scrive nell'archivio con creazione solo se assente → sul bus va solo il riferimento al pezzo (topic `log-<tenant>`, mai
//! proiettato nel database: NFR-4) → solo dopo, conferma al worker, che fino ad allora tiene le righe e le rimanda a ogni
//! ricollegamento.
//!
//! **Numero del pezzo = primo numero di riga del pezzo** (`<esecuzione>-<passo>-<tentativo>-<primo seq>`), mai un contatore del
//! motore: dopo un crash il worker rimanda le righe non confermate, che possono essere raggruppate in modo diverso da prima; con
//! un contatore in memoria la riscrittura potrebbe cadere su una chiave già presa da un contenuto DIVERSO e perdere righe in
//! silenzio. Qui, se la chiave esiste, si legge il pezzo esistente (primo, ultimo): le righe fino al suo ultimo sono già salvate
//! e si scartano, le altre si scrivono sotto la chiave `ultimo + 1`, e così via. Ogni numero di riga finisce in un solo pezzo.
//!
//! Un pezzo copre l'intervallo contiguo `[first, last]`; le righe nell'intervallo ma non salvate (oltre il tetto) sono il
//! `discarded` dichiarato nell'intestazione del pezzo: salvate + scartate = scritte dal worker.
// ponytail: il tetto in byte e il rilevamento dei doppioni stanno in memoria del motore: dopo un riavvio il conteggio dei byte
// riparte da zero (il tetto in righe no: dipende dal numero di riga). Uscita: portare nell'intestazione del pezzo i byte
// cumulati e leggere l'ultimo pezzo del tentativo alla prima riga dopo il riavvio.

use crate::bus::Bus;
use crate::crypto::{Keys, Uid, hex, sha};
use crate::{hist, pb};
use bytes::Bytes;
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tonic::Status;

/// Segnaposto che prende il posto del valore esatto di un segreto consegnato.
pub const PLACEHOLDER: &[u8] = b"[REDACTED]";

pub type Tx = mpsc::Sender<Result<pb::Delivery, Status>>;
/// Esecuzione, passo, tentativo.
type Key = (Uid, u32, u32);

/// Righe di un tentativo arrivate dalla sessione di un worker, con il canale per confermarle.
pub struct LogIn {
    pub tenant: Uid,
    pub exec: Uid,
    pub step: u32,
    pub attempt: u32,
    pub lines: Vec<(u64, Vec<u8>)>,
    pub tx: Tx,
}

pub struct Cfg {
    pub dir: String,
    pub max_lines: u64,
    pub max_bytes: usize,
    pub close_ms: u64,
    /// Ritardo della conferma al worker dopo che il pezzo è durevole: allarga la finestra di un crash con righe non
    /// confermate (solo per la scena del crash, come `rec_delay_ms`); 0 = subito.
    pub ack_hold_ms: u64,
    /// Valore del segreto consegnato (vuoto = nessuno) e interruttore della sostituzione (spento = corsa di controllo).
    pub secret: Vec<u8>,
    pub redact: bool,
}

#[derive(Default)]
pub struct Counters {
    pub lines_in: AtomicU64,
    pub dups: AtomicU64,
    pub stored: AtomicU64,
    pub discarded: AtomicU64,
    pub redacted: AtomicU64,
    pub chunks: AtomicU64,
    pub conflicts: AtomicU64,
    /// Conflitti in cui il pezzo esistente copriva meno righe di quelle da scrivere (contenuto diverso: il caso del riavvio).
    pub conflicts_differ: AtomicU64,
    pub refs: AtomicU64,
    pub acks: AtomicU64,
    pub write_errors: AtomicU64,
    /// Durata di un ciclo di scrittura (pezzi in parallelo, fsync della directory, riferimenti sul bus), in microsecondi.
    pub flush: Mutex<Option<Histogram<u64>>>,
}

impl Counters {
    pub fn text(&self) -> String {
        let g = |a: &AtomicU64| a.load(Relaxed);
        let mut f = self.flush.lock().unwrap();
        let h = f.get_or_insert_with(hist);
        format!(
            "tasklog lines_in={} dups={} stored={} discarded={} redacted={} chunks={} conflicts={} conflicts_differ={} refs={} acks={} write_errors={} flush_cycle {}",
            g(&self.lines_in),
            g(&self.dups),
            g(&self.stored),
            g(&self.discarded),
            g(&self.redacted),
            g(&self.chunks),
            g(&self.conflicts),
            g(&self.conflicts_differ),
            g(&self.refs),
            g(&self.acks),
            g(&self.write_errors),
            crate::fmt(h)
        )
    }
}

/// Sostituisce ogni occorrenza esatta di `secret` in `text` con il segnaposto; torna il testo e il numero di sostituzioni.
fn redact(text: Vec<u8>, secret: &[u8]) -> (Vec<u8>, u64) {
    if secret.is_empty() || text.len() < secret.len() {
        return (text, 0);
    }
    let (mut out, mut n, mut i) = (Vec::with_capacity(text.len()), 0, 0);
    while i < text.len() {
        if text[i..].starts_with(secret) {
            out.extend_from_slice(PLACEHOLDER);
            i += secret.len();
            n += 1;
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    (out, n)
}

/// Pezzo chiuso, da scrivere: righe salvate nell'intervallo `[first, last]`.
struct Chunk {
    key: Key,
    tenant: Uid,
    first: u64,
    last: u64,
    lines: Vec<(u64, Vec<u8>)>,
}

/// Stato di un tentativo nel motore.
struct Att {
    tenant: Uid,
    /// Prossimo numero atteso; sotto questo valore una riga è un doppione (rimandata dal worker).
    next: u64,
    /// Byte salvati finora (tetto in byte).
    bytes: usize,
    /// Pezzo aperto: primo e ultimo numero coperto (0 = nessun pezzo aperto), righe salvate, apertura.
    first: u64,
    last: u64,
    buf: Vec<(u64, Vec<u8>)>,
    opened: Instant,
    /// Ultimo numero di riga durevole (pezzo scritto e riferimento sul bus).
    durable: u64,
    tx: Tx,
    touched: Instant,
}

impl Att {
    fn take(&mut self, key: Key) -> Chunk {
        let c = Chunk {
            key,
            tenant: self.tenant,
            first: self.first,
            last: self.last,
            lines: std::mem::take(&mut self.buf),
        };
        self.first = 0;
        c
    }
}

struct State {
    cfg: Cfg,
    keys: Keys,
    ctr: Arc<Counters>,
    atts: HashMap<Key, Att>,
    /// Pezzi chiusi per un buco nei numeri o da riscrivere dopo un errore.
    ready: Vec<Chunk>,
    reack: Vec<Key>,
    buses: Vec<Bus>,
}

impl State {
    fn ingest(&mut self, m: LogIn) {
        let key = (m.exec, m.step, m.attempt);
        let now = Instant::now();
        let a = self.atts.entry(key).or_insert_with(|| Att {
            tenant: m.tenant,
            next: 0,
            bytes: 0,
            first: 0,
            last: 0,
            buf: vec![],
            opened: now,
            durable: 0,
            tx: m.tx.clone(),
            touched: now,
        });
        a.tx = m.tx;
        a.touched = now;
        for (seq, text) in m.lines {
            self.ctr.lines_in.fetch_add(1, Relaxed);
            // primo messaggio del tentativo (anche dopo un riavvio): si parte dal primo numero ricevuto
            if a.next == 0 {
                a.next = seq;
            }
            if seq < a.next {
                self.ctr.dups.fetch_add(1, Relaxed);
                if seq <= a.durable {
                    self.reack.push(key);
                }
                continue;
            }
            // buco nei numeri (non deve succedere: lo stream di una sessione è in ordine): il pezzo aperto si chiude lì
            if a.first != 0 && seq != a.last + 1 {
                self.ready.push(a.take(key));
            }
            if a.first == 0 {
                a.first = seq;
                a.opened = now;
            }
            a.last = seq;
            a.next = seq + 1;
            let (text, n) = if self.cfg.redact {
                redact(text, &self.cfg.secret)
            } else {
                (text, 0)
            };
            self.ctr.redacted.fetch_add(n, Relaxed);
            // tetto per tentativo: in righe (dal numero, quindi uguale dopo un riavvio) e in byte
            if seq > self.cfg.max_lines || a.bytes + text.len() > self.cfg.max_bytes {
                self.ctr.discarded.fetch_add(1, Relaxed);
            } else {
                a.bytes += text.len();
                self.ctr.stored.fetch_add(1, Relaxed);
                a.buf.push((seq, text));
            }
        }
    }

    /// Chiude i pezzi scaduti, li scrive tutti in parallelo, fa l'fsync delle directory, scrive i riferimenti sul bus e
    /// solo dopo conferma i worker. Un tentativo ha al più un pezzo per ciclo, e il ciclo finisce prima del successivo:
    /// la conferma «fino a N» vale quindi per tutte le righe sotto N.
    async fn flush(&mut self) {
        let now = Instant::now();
        let close = Duration::from_millis(self.cfg.close_ms);
        let mut chunks = std::mem::take(&mut self.ready);
        for (k, a) in self.atts.iter_mut() {
            if a.first != 0 && now.duration_since(a.opened) >= close {
                chunks.push(a.take(*k));
            }
        }
        let t0 = Instant::now();
        let mut acks: HashMap<Key, u64> = HashMap::new();
        if !chunks.is_empty() {
            let mut set = tokio::task::JoinSet::new();
            for c in chunks {
                let (keys, dir, ctr) = (self.keys.clone(), self.cfg.dir.clone(), self.ctr.clone());
                set.spawn_blocking(move || {
                    let r = write_chunk(&keys, &dir, &c, &ctr);
                    (c, r)
                });
            }
            let mut refs: [Vec<Bytes>; 2] = [vec![], vec![]];
            let mut done: Vec<(Key, u64)> = vec![];
            let mut failed: Vec<Key> = vec![];
            while let Some(r) = set.join_next().await {
                let (c, r) = r.unwrap();
                match r {
                    Ok(w) => {
                        let ti = crate::engine::TENANTS
                            .iter()
                            .position(|t| *t == c.tenant)
                            .unwrap();
                        for r in w {
                            let id = *uuid::Uuid::now_v7().as_bytes();
                            let addr = format!("bus/{}", log_topic(&c.tenant));
                            refs[ti].push(Bytes::from(self.keys.seal(&c.tenant, &addr, &id, &r)));
                        }
                        done.push((c.key, c.last));
                    }
                    Err(e) => {
                        // disco pieno o simili: il pezzo si riscrive al ciclo dopo, le righe restano in memoria e nel worker
                        self.ctr.write_errors.fetch_add(1, Relaxed);
                        eprintln!("LOG_WRITE_ERROR {e}");
                        failed.push(c.key);
                        self.ready.push(c);
                    }
                }
            }
            // directory sincronizzate prima dei riferimenti e delle conferme (le voci create con il collegamento sono durevoli)
            for t in &crate::engine::TENANTS {
                let d = format!("{}/logs/{}", self.cfg.dir, hex(t));
                tokio::task::spawn_blocking(move || {
                    std::fs::File::open(d).and_then(|f| f.sync_all())
                })
                .await
                .unwrap()
                .ok();
            }
            let (b0, b1) = (self.buses[0].clone(), self.buses[1].clone());
            let (r0, r1) = (std::mem::take(&mut refs[0]), std::mem::take(&mut refs[1]));
            self.ctr
                .refs
                .fetch_add((r0.len() + r1.len()) as u64, Relaxed);
            tokio::join!(
                append_refs(&b0, log_topic(&crate::engine::TENANTS[0]), r0),
                append_refs(&b1, log_topic(&crate::engine::TENANTS[1]), r1)
            );
            // un tentativo con un pezzo non scritto non si conferma nemmeno per i pezzi successivi (la conferma è cumulativa)
            for (k, last) in done.into_iter().filter(|d| !failed.contains(&d.0)) {
                let e = acks.entry(k).or_default();
                *e = (*e).max(last);
                if let Some(a) = self.atts.get_mut(&k) {
                    a.durable = a.durable.max(last);
                }
            }
            self.ctr
                .flush
                .lock()
                .unwrap()
                .get_or_insert_with(hist)
                .record(t0.elapsed().as_micros() as u64 + 1)
                .ok();
        }
        // doppioni di righe già durevoli (rimandate dopo un ricollegamento): si riconferma
        for k in std::mem::take(&mut self.reack) {
            if let Some(a) = self.atts.get(&k) {
                acks.entry(k).or_insert(a.durable);
            }
        }
        for ((exec, step, attempt), upto) in acks {
            let Some(a) = self.atts.get(&(exec, step, attempt)) else {
                continue;
            };
            let (tx, hold) = (a.tx.clone(), self.cfg.ack_hold_ms);
            self.ctr.acks.fetch_add(1, Relaxed);
            let d = pb::Delivery {
                exec: exec.to_vec(),
                step,
                attempt,
                log_ack: upto,
                ..Default::default()
            };
            if hold == 0 {
                let _ = tx.try_send(Ok(d));
            } else {
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(hold)).await;
                    let _ = tx.send(Ok(d)).await;
                });
            }
        }
    }
}

pub fn log_topic(t: &Uid) -> String {
    format!("log-{}", hex(t))
}

/// Scrive i riferimenti sul bus, ritentando finché ci riesce: un riferimento doppio dopo un esito incerto è innocuo
/// (chi legge deduplica per chiave del pezzo).
async fn append_refs(bus: &Bus, topic: String, msgs: Vec<Bytes>) {
    if msgs.is_empty() {
        return;
    }
    while let Err(e) = bus.append(&topic, &msgs).await {
        eprintln!("LOG_APPEND_FAIL {topic}: {e}");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Corpo in chiaro di un pezzo: primo, ultimo, scartate, numero di righe salvate, poi (numero, lunghezza, testo) per riga.
fn encode(first: u64, last: u64, lines: &[&(u64, Vec<u8>)]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&first.to_le_bytes());
    v.extend_from_slice(&last.to_le_bytes());
    v.extend_from_slice(&((last - first + 1 - lines.len() as u64) as u32).to_le_bytes());
    v.extend_from_slice(&(lines.len() as u32).to_le_bytes());
    for (seq, t) in lines.iter().map(|l| (l.0, &l.1)) {
        v.extend_from_slice(&seq.to_le_bytes());
        v.extend_from_slice(&(t.len() as u32).to_le_bytes());
        v.extend_from_slice(t);
    }
    v
}

/// Pezzo letto: intervallo coperto, righe scartate dichiarate, righe salvate.
pub struct Decoded {
    pub first: u64,
    pub last: u64,
    pub discarded: u32,
    pub lines: Vec<(u64, Vec<u8>)>,
}

pub fn decode(b: &[u8]) -> Decoded {
    let u64at = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
    let u32at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let (n, mut o) = (u32at(20) as usize, 24);
    let mut lines = Vec::with_capacity(n);
    for _ in 0..n {
        let len = u32at(o + 8) as usize;
        lines.push((u64at(o), b[o + 12..o + 12 + len].to_vec()));
        o += 12 + len;
    }
    Decoded {
        first: u64at(0),
        last: u64at(8),
        discarded: u32at(16),
        lines,
    }
}

/// Nome del file del pezzo e indirizzo per la cifratura (`info` di HKDF): il nome porta il primo numero di riga.
pub fn chunk_name(exec: &Uid, step: u32, attempt: u32, first: u64) -> String {
    format!("{}-{step}-{attempt}-{first:010}", hex(exec))
}

pub fn chunk_addr(tenant: &Uid, name: &str) -> String {
    format!("log/{}/{name}", hex(tenant))
}

/// Riferimento al pezzo (va sul bus, mai proiettato): tenant, esecuzione, passo, tentativo, primo, ultimo, salvate, scartate.
fn reference(c: &Chunk, first: u64, last: u64, stored: u32, discarded: u32) -> Vec<u8> {
    let mut v = [c.tenant.as_slice(), c.key.0.as_slice()].concat();
    v.extend_from_slice(&c.key.1.to_le_bytes());
    v.extend_from_slice(&c.key.2.to_le_bytes());
    v.extend_from_slice(&first.to_le_bytes());
    v.extend_from_slice(&last.to_le_bytes());
    v.extend_from_slice(&stored.to_le_bytes());
    v.extend_from_slice(&discarded.to_le_bytes());
    v
}

/// Scrive il pezzo con creazione solo se assente. Sul disco: file temporaneo `create_new`, `fdatasync`, poi
/// `hard_link` sulla chiave (la libreria standard lo rifiuta se la chiave esiste, ed è atomico: la chiave compare solo
/// completa, anche se il processo muore a metà; con una `create_new` diretta sulla chiave un crash lascerebbe un pezzo
/// troncato che la riscrittura troverebbe «esistente»). Se la chiave esiste vedi il commento del modulo. Torna i riferimenti.
fn write_chunk(keys: &Keys, dir: &str, c: &Chunk, ctr: &Counters) -> std::io::Result<Vec<Vec<u8>>> {
    let tdir = format!("{dir}/logs/{}", hex(&c.tenant));
    let (mut first, mut refs) = (c.first, vec![]);
    loop {
        let part: Vec<&(u64, Vec<u8>)> = c.lines.iter().filter(|l| l.0 >= first).collect();
        let name = chunk_name(&c.key.0, c.key.1, c.key.2, first);
        let addr = chunk_addr(&c.tenant, &name);
        let sealed = keys.seal(
            &c.tenant,
            &addr,
            sha(addr.as_bytes())[..16].try_into().unwrap(),
            &encode(first, c.last, &part),
        );
        let tmp = format!("{tdir}/.tmp-{}", uuid::Uuid::new_v4().simple());
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)?;
            f.write_all(&sealed)?;
            f.sync_data()?;
        }
        let path = format!("{tdir}/{name}");
        let linked = std::fs::hard_link(&tmp, &path);
        let _ = std::fs::remove_file(&tmp);
        match linked {
            Ok(()) => {
                ctr.chunks.fetch_add(1, Relaxed);
                let (n, d) = (
                    part.len() as u32,
                    (c.last - first + 1) as u32 - part.len() as u32,
                );
                refs.push(reference(c, first, c.last, n, d));
                return Ok(refs);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // chiave già presa (riscrittura dopo un riavvio): vale il contenuto esistente; si prosegue dopo il suo ultimo numero
                ctr.conflicts.fetch_add(1, Relaxed);
                let (_, _, plain) = keys.open(&addr, &std::fs::read(&path)?);
                let e = decode(&plain);
                refs.push(reference(
                    c,
                    e.first,
                    e.last,
                    e.lines.len() as u32,
                    e.discarded,
                ));
                if e.last < c.last {
                    ctr.conflicts_differ.fetch_add(1, Relaxed);
                    first = e.last + 1;
                } else {
                    return Ok(refs);
                }
            }
            Err(e) => return Err(e),
        }
    }
}

/// Crea le directory dei pezzi e toglie i file temporanei lasciati da un crash.
pub fn prepare(dir: &str) {
    for t in &crate::engine::TENANTS {
        let d = format!("{dir}/logs/{}", hex(t));
        std::fs::create_dir_all(&d).unwrap();
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            if e.file_name().to_string_lossy().starts_with(".tmp-") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

pub async fn run(
    cfg: Cfg,
    keys: Keys,
    bus_kind: &'static str,
    ctr: Arc<Counters>,
    mut rx: mpsc::UnboundedReceiver<LogIn>,
) {
    let buses = vec![
        Bus::connect(bus_kind).await.unwrap(),
        Bus::connect(bus_kind).await.unwrap(),
    ];
    let mut st = State {
        cfg,
        keys,
        ctr,
        atts: HashMap::new(),
        ready: vec![],
        reack: vec![],
        buses,
    };
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut n = 0u32;
    loop {
        tokio::select! {
            Some(m) = rx.recv() => {
                st.ingest(m);
                while let Ok(m) = rx.try_recv() {
                    st.ingest(m);
                }
            }
            _ = tick.tick() => {
                st.flush().await;
                n += 1;
                // ponytail: uno stato di tentativo inattivo da 2 minuti si butta; un doppione arrivato dopo lo trova il
                // conflitto di creazione (stessa chiave), non il controllo in memoria
                if n.is_multiple_of(100) {
                    st.atts.retain(|_, a| a.first != 0 || a.touched.elapsed() < Duration::from_secs(120));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Uid = [7; 16];

    /// Directory di prova sotto `~/.cache`, mai in `/tmp` (tmpfs in RAM); chi la crea la toglie con il nome esatto.
    fn tdir(name: &str) -> String {
        let d = format!(
            "{}/.cache/ttspike-logtest-{name}",
            std::env::var("HOME").unwrap()
        );
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(format!("{d}/logs/{}", hex(&T))).unwrap();
        d
    }

    fn chunk(first: u64, last: u64) -> Chunk {
        Chunk {
            key: ([1; 16], 0, 0),
            tenant: T,
            first,
            last,
            lines: (first..=last)
                .map(|s| (s, format!("riga {s}").into_bytes()))
                .collect(),
        }
    }

    #[test]
    fn redact_replaces_every_exact_occurrence() {
        let (t, n) = redact(b"a SEGRETO b SEGRETO".to_vec(), b"SEGRETO");
        assert_eq!(
            (t.as_slice(), n),
            (b"a [REDACTED] b [REDACTED]".as_slice(), 2)
        );
        assert_eq!(redact(b"nulla".to_vec(), b"SEGRETO").1, 0);
        assert_eq!(redact(b"SEGRETO".to_vec(), b"").1, 0);
    }

    /// Il caso del riavvio: il pezzo `[1..5]` è già durevole (conferma persa), il worker rimanda 1..8 e il motore nuovo li
    /// raggruppa in un solo pezzo `[1..8]` sulla stessa chiave. Nessuna riga persa né doppia: 6..8 finiscono sotto `6`.
    #[test]
    fn resent_chunk_with_different_content_loses_nothing() {
        let dir = tdir("resend");
        let keys = Keys::load(&format!("{dir}/master.key"));
        let ctr = Counters::default();
        write_chunk(&keys, &dir, &chunk(1, 5), &ctr).unwrap();
        let refs = write_chunk(&keys, &dir, &chunk(1, 8), &ctr).unwrap();
        assert_eq!(refs.len(), 2, "riferimento al pezzo esistente e al nuovo");
        assert_eq!(ctr.conflicts.load(Relaxed), 1);
        assert_eq!(ctr.conflicts_differ.load(Relaxed), 1);
        let mut seqs = vec![];
        for e in std::fs::read_dir(format!("{dir}/logs/{}", hex(&T)))
            .unwrap()
            .flatten()
        {
            let n = e.file_name().to_string_lossy().to_string();
            let (_, _, p) = keys.open(&chunk_addr(&T, &n), &std::fs::read(e.path()).unwrap());
            seqs.extend(decode(&p).lines.into_iter().map(|l| l.0));
        }
        seqs.sort();
        assert_eq!(seqs, (1..=8).collect::<Vec<_>>());
        // la stessa riscrittura con lo stesso contenuto non cambia nulla
        write_chunk(&keys, &dir, &chunk(1, 8), &ctr).unwrap();
        assert_eq!(ctr.chunks.load(Relaxed), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Tetto: oltre le righe consentite la riga si scarta e il pezzo dichiara quante; un doppione non cambia il conto.
    #[tokio::test]
    async fn cap_discards_and_chunk_states_the_count() {
        let dir = tdir("cap");
        let keys = Keys::load(&format!("{dir}/master.key"));
        let (tx, _rx) = mpsc::channel(8);
        let mut st = State {
            cfg: Cfg {
                dir: dir.clone(),
                max_lines: 3,
                max_bytes: 1 << 20,
                close_ms: 0,
                ack_hold_ms: 0,
                secret: vec![],
                redact: true,
            },
            keys: keys.clone(),
            ctr: Arc::new(Default::default()),
            atts: HashMap::new(),
            ready: vec![],
            reack: vec![],
            buses: vec![],
        };
        let lines =
            |r: std::ops::RangeInclusive<u64>| r.map(|s| (s, b"x".to_vec())).collect::<Vec<_>>();
        let m = |l| LogIn {
            tenant: T,
            exec: [1; 16],
            step: 0,
            attempt: 0,
            lines: l,
            tx: tx.clone(),
        };
        st.ingest(m(lines(1..=5)));
        st.ingest(m(lines(1..=5)));
        assert_eq!(st.ctr.dups.load(Relaxed), 5);
        let c = st
            .atts
            .get_mut(&([1; 16], 0, 0))
            .unwrap()
            .take(([1; 16], 0, 0));
        write_chunk(&keys, &dir, &c, &st.ctr).unwrap();
        let name = chunk_name(&[1; 16], 0, 0, 1);
        let (_, _, p) = keys.open(
            &chunk_addr(&T, &name),
            &std::fs::read(format!("{dir}/logs/{}/{name}", hex(&T))).unwrap(),
        );
        let d = decode(&p);
        assert_eq!((d.first, d.last, d.discarded, d.lines.len()), (1, 5, 2, 3));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
