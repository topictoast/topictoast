//! Ruolo `stream`: un processo per tenant (AD-6, AD-13). Riceve sul descrittore 3 (coppia di socket
//! ereditata) la chiave del suo tenant e la credenziale di sola lettura del bus; legge i topic del tenant,
//! decifra, esegue le pipeline a micro-lotti, scrive i checkpoint e consegna le righe delle finestre chiuse come
//! avvii al motore (o come blocchi su uno stream di destinazione, C7).
//!
//! Nessun orologio di sistema nella semantica: il tempo viene solo dal log (arrivo nei blocchi e nei
//! segnali). I timer `Instant` servono solo a decidere quando raggruppare e quando fare il checkpoint.
//!
//! Parte 2a:
//! - ogni pipeline ha la sua posizione applicata e le sue chiavi di blocco viste (`PipeState`); il checkpoint le porta;
//!   alla ripresa di una pipeline sospesa si rilegge dal minimo delle posizioni e ognuna salta quello che ha applicato;
//! - ciclo di vita (C5/V3): comandi del motore sul socket (`suspend`, `resume`, `set_pipeline`, `validate`); una
//!   pipeline sospesa non si alimenta e, se sono tutte sospese, il processo non legge il bus;
//! - quota di stato per pipeline (64 MiB) contata sugli accumulatori dopo ogni micro-lotto: oltre la quota la pipeline
//!   si sospende con il codice `state_quota_exceeded`, le altre continuano;
//! - perdita di log oltre la conservazione: un salto di posizione nella lettura sospende le pipeline colpite con il codice
//!   `retention_exceeded`, mai una perdita silenziosa.
//!
//! Confinamento (descrittori, namespace, Landlock, seccomp: PIANO 1.12): parte 2b.

// spike: gli indici servono a prendere in prestito `pipes` in modo disgiunto dai flussi
#![allow(clippy::needless_range_loop)]

use crate::bus::Bus;
use crate::crypto::{TenantKey, new_id, tenant_uid, unhex};
use crate::dialect::{Barriers, DErr, derr, validate_set};
use crate::engine::Hello;
use crate::http::Client;
use crate::pipeline::{Pipe, PipeHead, PipeState, Row};
use crate::spec::*;
use crate::wire::{self, Msg};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::io::Write;
use std::os::fd::FromRawFd;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Notify, mpsc};

type Pending = Arc<Mutex<VecDeque<Row>>>;

/// Contatori della consegna, letti dalle statistiche.
#[derive(Default)]
struct Counters {
    delivered: AtomicU64,
    refused: AtomicU64,
}

struct SRt {
    def: StreamDef,
    topic: String,
    /// prossima posizione da leggere dal bus
    pos: u64,
    /// posizione piu' alta mai letta (le statistiche contano gli eventi solo oltre questa)
    hw: u64,
    events: u64,
    bad: u64,
}

/// Una pipeline in esecuzione (un ramo di `UNION ALL` e' una `PRt` a parte).
struct PRt {
    pipe: Pipe,
    st: PipeState,
    sidx: usize,
    /// `Some(codice)` se sospesa
    suspended: Option<String>,
    bytes: usize,
}

#[derive(Serialize, Deserialize)]
struct StreamCk {
    name: String,
    pos: u64,
    hw: u64,
    events: u64,
}

#[derive(Serialize, Deserialize)]
struct CkHead {
    streams: Vec<StreamCk>,
    pipes: Vec<PipeHead>,
    pending: Vec<Row>,
    /// (id pipeline, codice) delle sospensioni
    #[serde(default)]
    suspended: Vec<(String, String)>,
}

/// Checkpoint: file temporaneo, `fdatasync`, rinomina, `fsync` della cartella (PIANO 1.10), cifrato C-12.
fn write_ckpt(
    dir: &str,
    key: &TenantKey,
    tid: &[u8; 16],
    tenant: &str,
    head: &CkHead,
    blobs: &[Vec<u8>],
) {
    let h = serde_json::to_vec(head).unwrap();
    let mut plain = (h.len() as u32).to_le_bytes().to_vec();
    plain.extend_from_slice(&h);
    for b in blobs {
        plain.extend_from_slice(&(b.len() as u32).to_le_bytes());
        plain.extend_from_slice(b);
    }
    let sealed = key.seal(tid, &format!("ckpt/{tenant}"), &new_id(), &plain);
    let tmp = format!("{dir}/ckpt.tmp");
    let mut f = std::fs::File::create(&tmp).unwrap();
    f.write_all(&sealed).unwrap();
    f.sync_data().unwrap();
    drop(f);
    std::fs::rename(&tmp, format!("{dir}/ckpt.bin")).unwrap();
    std::fs::File::open(dir).unwrap().sync_all().unwrap();
}

fn read_ckpt(dir: &str, key: &TenantKey, tenant: &str) -> Option<(CkHead, Vec<Vec<u8>>)> {
    let sealed = std::fs::read(format!("{dir}/ckpt.bin")).ok()?;
    let plain = key.open(&format!("ckpt/{tenant}"), &sealed)?;
    let hl = u32::from_le_bytes(plain[..4].try_into().unwrap()) as usize;
    let head: CkHead = serde_json::from_slice(&plain[4..4 + hl]).ok()?;
    let mut blobs = vec![];
    let mut p = 4 + hl;
    while p < plain.len() {
        let l = u32::from_le_bytes(plain[p..p + 4].try_into().unwrap()) as usize;
        blobs.push(plain[p + 4..p + 4 + l].to_vec());
        p += 4 + l;
    }
    Some((head, blobs))
}

/// Consegna le righe in attesa, con la chiave C-4: come avvio (`/v1/starts`) o, per una pipeline con destinazione
/// stream (C7), come blocco di un evento sullo stream di destinazione con la stessa chiave come `Idempotency-Key`
/// (i doppioni dopo un kill si assorbono alla lettura, come per ogni blocco). Un rifiuto per quota (429/503) o
/// un errore trattiene la riga e la riprova; la riga esce dalla coda solo dopo il 200.
async fn deliver(
    pending: Pending,
    api: String,
    token: String,
    wake: Arc<Notify>,
    cnt: Arc<Counters>,
) {
    let mut c = Client::new(&api);
    loop {
        let row = pending.lock().unwrap().front().cloned();
        let Some(row) = row else {
            // nessun sondaggio periodico: una pipeline sospesa non deve svegliare il processo
            wake.notified().await;
            continue;
        };
        let auth = format!("Bearer {token}");
        let r = match &row.dest {
            Some(d) => {
                let body = json!({"events":[row.body]}).to_string();
                c.post(
                    &format!("/v1/streams/{d}/blocks"),
                    &[("Authorization", &auth), ("Idempotency-Key", &row.key)],
                    body.as_bytes(),
                )
                .await
            }
            None => {
                c.post(
                    "/v1/starts",
                    &[("Authorization", &auth), ("Idempotency-Key", &row.key)],
                    row.body.to_string().as_bytes(),
                )
                .await
            }
        };
        match r {
            Ok((200, _)) => {
                pending.lock().unwrap().pop_front();
                cnt.delivered.fetch_add(1, Ordering::Relaxed);
            }
            Ok((code, _)) => {
                if code == 429 {
                    cnt.refused.fetch_add(1, Ordering::Relaxed);
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
        }
    }
}

/// Prima posizione ancora «in conservazione» di uno stream: la prima il cui arrivo e' entro `retention_ms` dall'arrivo dell'ultimo
/// messaggio (il tempo del log, che avanza con i segnali anche a stream fermo). Ricerca binaria sugli arrivi, che nel log non
/// decrescono.
/// ponytail: conservazione simulata dal lettore; Iggy 0.10 qui tiene tutto (nessuna scadenza per segmenti pilotabile senza
/// cambiare la configurazione condivisa). Il prodotto la configura sul log e la rilevazione resta il salto di posizione.
async fn log_floor(bus: &Bus, key: &TenantKey, topic: &str, retention_ms: i64) -> u64 {
    let arrival = |m: Option<(u64, bytes::Bytes)>| -> i64 {
        m.and_then(|(_, p)| key.open(topic, &p)).map_or(0, |plain| {
            i64::from_le_bytes(plain[1..9].try_into().unwrap())
        })
    };
    let head = bus.head(topic).await.unwrap_or(0);
    if head == 0 {
        return 0;
    }
    let last = arrival(
        bus.read(topic, head - 1, 1)
            .await
            .ok()
            .and_then(|v| v.into_iter().next()),
    );
    let (mut lo, mut hi) = (0u64, head - 1);
    while lo < hi {
        let mid = (lo + hi) / 2;
        let a = arrival(
            bus.read(topic, mid, 1)
                .await
                .ok()
                .and_then(|v| v.into_iter().next()),
        );
        if a >= last - retention_ms {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

/// Tempo di CPU del processo (utente + sistema, tutti i thread, anche quelli finiti), in nanosecondi.
fn cpu_ns() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` e' valido per la scrittura
    unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) };
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// Memoria residente (KiB) da `/proc/self/statm`.
fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| {
            s.split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<u64>().ok())
        })
        .map_or(0, |p| p * 4)
}

/// Costruisce e valida un insieme di pipeline; per la validazione a secco esegue anche lo stadio di ingresso su un
/// lotto sintetico.
async fn build_checked(
    tenant: &TenantDef,
    def: &PipelineDef,
    quota: usize,
    dry: bool,
    bars: &Barriers,
) -> Result<Vec<Pipe>, DErr> {
    let mut defs: Vec<PipelineDef> = tenant
        .pipelines
        .iter()
        .filter(|p| p.id != def.id)
        .cloned()
        .collect();
    defs.push(def.clone());
    validate_set(&defs, &tenant.streams, bars.syntax).map_err(|(_, e)| e)?;
    let pipes = Pipe::build(def, &tenant.streams, quota, bars).await?;
    if dry {
        for p in &pipes {
            let sd = tenant
                .streams
                .iter()
                .find(|s| s.name == p.stream)
                .expect("stream gia' verificato");
            p.dry_run(sd, 64).await?;
        }
    }
    Ok(pipes)
}

pub async fn run() {
    // coppia di socket ereditata: descrittore 3
    let std_sock = unsafe { std::os::unix::net::UnixStream::from_raw_fd(3) };
    std_sock.set_nonblocking(true).unwrap();
    let sock = tokio::net::UnixStream::from_std(std_sock).unwrap();
    let (rd, mut wr) = sock.into_split();
    let mut lines = BufReader::new(rd).lines();
    let hello: Hello = serde_json::from_str(
        &lines
            .next_line()
            .await
            .unwrap()
            .expect("nessun messaggio iniziale"),
    )
    .unwrap();
    // limite del sistema sul processo (V3, barriera 4): oltre, le allocazioni falliscono
    if hello.rlimit_data_mb > 0 {
        let l = libc::rlimit {
            rlim_cur: hello.rlimit_data_mb << 20,
            rlim_max: hello.rlimit_data_mb << 20,
        };
        // SAFETY: `l` e' valido per la lettura
        unsafe { libc::setrlimit(libc::RLIMIT_DATA, &l) };
    }
    // comandi del motore sul socket; se il motore sparisce (EOF) il processo termina
    let (ctx_tx, mut ctrl) = mpsc::unbounded_channel::<Value>();
    tokio::spawn(async move {
        while let Ok(Some(l)) = lines.next_line().await {
            if let Ok(v) = serde_json::from_str::<Value>(&l) {
                let _ = ctx_tx.send(v);
            }
        }
        std::process::exit(0);
    });
    let key = TenantKey(unhex(&hello.key_hex).try_into().unwrap());
    let mut tenant = hello.tenant.clone();
    let tid = tenant_uid(&tenant.id);
    let bus = Bus::connect(&hello.iggy_addr, &hello.iggy_user, &hello.iggy_pass)
        .await
        .expect("bus");
    let quota = hello.state_quota;
    let bars = Barriers {
        syntax: !hello.ctrl_off_syntax,
        ..Barriers::ALL
    };

    let mut srt: Vec<SRt> = tenant
        .streams
        .iter()
        .map(|s| SRt {
            topic: topic_of(&tenant.id, &s.name),
            def: s.clone(),
            pos: 0,
            hw: 0,
            events: 0,
            bad: 0,
        })
        .collect();
    let mut pipes: Vec<PRt> = vec![];
    for d in &tenant.pipelines {
        match build_checked(&tenant, d, quota, false, &bars).await {
            Ok(ps) => {
                for p in ps {
                    let sidx = srt.iter().position(|s| s.def.name == p.stream).unwrap();
                    let st = p.new_state(0);
                    pipes.push(PRt {
                        pipe: p,
                        st,
                        sidx,
                        suspended: None,
                        bytes: 0,
                    });
                }
            }
            Err(e) => {
                eprintln!("stream: pipeline {} rifiutata: {e}", d.id);
                std::process::exit(2);
            }
        }
    }
    let pending: Pending = Arc::new(Mutex::new(VecDeque::new()));

    // ripresa dall'ultimo checkpoint
    if let Some((head, blobs)) = read_ckpt(&hello.ckpt_dir, &key, &tenant.id) {
        for s in &mut srt {
            if let Some(c) = head.streams.iter().find(|c| c.name == s.def.name) {
                s.pos = c.pos;
                s.hw = c.hw;
                s.events = c.events;
            }
        }
        let mut bi = 0;
        for h in &head.pipes {
            let n = h.wins.len();
            if let Some(p) = pipes.iter_mut().find(|p| {
                p.pipe.def.id == h.id
                    && p.pipe.def.version == h.version
                    && p.pipe.branch == h.branch
            }) {
                let refs: Vec<&[u8]> = blobs[bi..bi + n].iter().map(|b| b.as_slice()).collect();
                p.st = p
                    .pipe
                    .restore(h, &refs)
                    .expect("stato del checkpoint illeggibile");
            }
            bi += n;
        }
        for (id, code) in &head.suspended {
            for p in pipes.iter_mut().filter(|p| &p.pipe.def.id == id) {
                p.suspended = Some(code.clone());
            }
        }
        // si riparte dal minimo delle posizioni applicate delle pipeline di ogni stream
        for (i, s) in srt.iter_mut().enumerate() {
            if let Some(m) = pipes
                .iter()
                .filter(|p| p.sidx == i)
                .map(|p| p.st.applied)
                .min()
            {
                s.pos = m;
            }
        }
        *pending.lock().unwrap() = head.pending.into_iter().collect();
        eprintln!(
            "stream {}: ripreso dal checkpoint, righe in attesa {}",
            tenant.id,
            pending.lock().unwrap().len()
        );
    }
    for id in &hello.suspended {
        for p in pipes
            .iter_mut()
            .filter(|p| &p.pipe.def.id == id && p.suspended.is_none())
        {
            p.suspended = Some("operator".into());
        }
    }
    let wake = Arc::new(Notify::new());
    let cnt = Arc::new(Counters::default());
    wake.notify_one();
    tokio::spawn(deliver(
        pending.clone(),
        hello.api_addr.clone(),
        hello.api_token.clone(),
        wake.clone(),
        cnt.clone(),
    ));

    let ck_every = Duration::from_secs(10);
    let mut last_ck = Instant::now();
    let mut last_stat = Instant::now();
    let mut last_delivered = 0u64;
    let mut dirty = true;
    let mut force_ck = false;
    let mut rr = 0usize;
    let mut stash: Option<Value> = None;
    loop {
        // comandi del motore
        let mut cmds: Vec<Value> = stash.take().into_iter().collect();
        while let Ok(c) = ctrl.try_recv() {
            cmds.push(c);
        }
        for c in cmds {
            let mut reply = json!({"reply": c["req"], "ok": true});
            match c["cmd"].as_str().unwrap_or("") {
                "suspend" => {
                    let id = c["id"].as_str().unwrap_or("");
                    for p in pipes
                        .iter_mut()
                        .filter(|p| p.pipe.def.id == id && p.suspended.is_none())
                    {
                        p.suspended = Some("operator".into());
                    }
                    dirty = true;
                }
                "resume" => {
                    let id = c["id"].as_str().unwrap_or("").to_string();
                    // la ripresa controlla la conservazione: se il log non ha piu' la posizione, errore con codice
                    let mut lost = false;
                    for pi in 0..pipes.len() {
                        if pipes[pi].pipe.def.id != id || pipes[pi].suspended.is_none() {
                            continue;
                        }
                        let (applied, si) = (pipes[pi].st.applied, pipes[pi].sidx);
                        let head = bus.head(&srt[si].topic).await.unwrap_or(0);
                        if let Some(r) = srt[si].def.retention_s
                            && applied < log_floor(&bus, &key, &srt[si].topic, r * 1000).await
                        {
                            lost = true;
                        }
                        if applied < head {
                            match bus.read(&srt[si].topic, applied, 1).await {
                                Ok(g) if g.first().is_some_and(|(o, _)| *o > applied) => {
                                    lost = true
                                }
                                _ => {}
                            }
                        }
                    }
                    if lost {
                        for p in pipes
                            .iter_mut()
                            .filter(|p| p.pipe.def.id == id && p.suspended.is_some())
                        {
                            p.suspended = Some("retention_exceeded".into());
                        }
                        dirty = true;
                        reply = json!({"reply": c["req"], "ok": false, "error": derr("retention_exceeded", format!("la posizione della pipeline {id} non e' piu' nel log"))});
                    } else {
                        for pi in 0..pipes.len() {
                            if pipes[pi].pipe.def.id == id && pipes[pi].suspended.take().is_some() {
                                let (si, a) = (pipes[pi].sidx, pipes[pi].st.applied);
                                srt[si].pos = srt[si].pos.min(a);
                            }
                        }
                        dirty = true;
                    }
                }
                "set_pipeline" => match serde_json::from_value::<PipelineDef>(c["def"].clone()) {
                    Err(e) => {
                        reply = json!({"reply": c["req"], "ok": false, "error": derr("bad_request", e.to_string())})
                    }
                    Ok(def) => match build_checked(&tenant, &def, quota, true, &bars).await {
                        Err(e) => reply = json!({"reply": c["req"], "ok": false, "error": e}),
                        Ok(ps) => {
                            pipes.retain(|p| p.pipe.def.id != def.id);
                            tenant.pipelines.retain(|p| p.id != def.id);
                            tenant.pipelines.push(def.clone());
                            for p in ps {
                                let sidx = srt.iter().position(|s| s.def.name == p.stream).unwrap();
                                // una nuova versione parte con lo stato vuoto dalla posizione di lettura corrente
                                let st = p.new_state(srt[sidx].pos);
                                eprintln!(
                                    "stream: set_pipeline {} v{} start_pos={}",
                                    def.id, def.version, srt[sidx].pos
                                );
                                pipes.push(PRt {
                                    pipe: p,
                                    st,
                                    sidx,
                                    suspended: None,
                                    bytes: 0,
                                });
                            }
                            dirty = true;
                            force_ck = true;
                        }
                    },
                },
                "validate" => match serde_json::from_value::<PipelineDef>(c["def"].clone()) {
                    Err(e) => {
                        reply = json!({"reply": c["req"], "ok": false, "error": derr("bad_request", e.to_string())})
                    }
                    Ok(def) => {
                        if let Err(e) = build_checked(&tenant, &def, quota, true, &bars).await {
                            reply = json!({"reply": c["req"], "ok": false, "error": e});
                        }
                    }
                },
                _ => {
                    reply = json!({"reply": c["req"], "ok": false, "error": derr("bad_request", "comando sconosciuto")})
                }
            }
            let _ = wr.write_all(format!("{reply}\n").as_bytes()).await;
        }
        // stream con almeno una pipeline attiva
        let active: Vec<usize> = (0..srt.len())
            .filter(|s| pipes.iter().any(|p| p.sidx == *s && p.suspended.is_none()))
            .collect();
        if active.is_empty() {
            // tutte sospese: nessuna lettura del bus; ci si sveglia per i comandi, il checkpoint e le statistiche
            let next = (last_stat + Duration::from_secs(1))
                .min(last_ck + ck_every)
                .saturating_duration_since(Instant::now());
            tokio::select! {
                c = ctrl.recv() => stash = c,
                _ = tokio::time::sleep(next) => {}
            }
        } else {
            let si = active[rr % active.len()];
            rr += 1;
            // raggruppa: fino a 8.192 righe o 50 ms
            let start = Instant::now();
            let mut msgs: Vec<(u64, Msg)> = vec![];
            let mut rows = 0usize;
            loop {
                let got = match bus.read(&srt[si].topic, srt[si].pos, 8).await {
                    Ok(g) => g,
                    Err(e) => {
                        eprintln!("stream: lettura fallita: {e}");
                        tokio::time::sleep(Duration::from_millis(200)).await;
                        break;
                    }
                };
                if got.is_empty() {
                    if msgs.is_empty() || start.elapsed() >= Duration::from_millis(50) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    continue;
                }
                // salto di posizione: il log ha perso dati oltre la conservazione, nessuna perdita silenziosa
                if let Some((o, _)) = got.first()
                    && *o > srt[si].pos
                {
                    let o = *o;
                    for p in pipes
                        .iter_mut()
                        .filter(|p| p.sidx == si && p.suspended.is_none() && p.st.applied < o)
                    {
                        eprintln!(
                            "stream: pipeline {} sospesa: retention_exceeded (posizione {} perduta, log da {o})",
                            p.pipe.def.id, p.st.applied
                        );
                        p.suspended = Some("retention_exceeded".into());
                    }
                    srt[si].pos = o;
                    dirty = true;
                    if !pipes.iter().any(|p| p.sidx == si && p.suspended.is_none()) {
                        break;
                    }
                }
                for (off, payload) in got {
                    srt[si].pos = off + 1;
                    match key.open(&srt[si].topic, &payload) {
                        Some(plain) => {
                            let m = wire::decode(&plain, true);
                            if let Msg::Block { events, .. } = &m {
                                rows += *events as usize;
                                if off >= srt[si].hw {
                                    srt[si].events += *events as u64;
                                }
                            }
                            msgs.push((off, m));
                        }
                        None => srt[si].bad += 1,
                    }
                    srt[si].hw = srt[si].hw.max(off + 1);
                }
                if rows >= 8192 || start.elapsed() >= Duration::from_millis(50) {
                    break;
                }
            }
            if !msgs.is_empty() {
                let refs: Vec<(u64, &Msg)> = msgs.iter().map(|(o, m)| (*o, m)).collect();
                for pi in 0..pipes.len() {
                    if pipes[pi].sidx != si || pipes[pi].suspended.is_some() {
                        continue;
                    }
                    let PRt { pipe, st, .. } = &mut pipes[pi];
                    match pipe.process(st, &refs).await {
                        Ok(r) => {
                            if !r.is_empty() {
                                pending.lock().unwrap().extend(r);
                                wake.notify_one();
                            }
                        }
                        Err(e) => {
                            eprintln!("stream: pipeline {} in errore: {e}", pipe.def.id);
                            std::process::exit(3);
                        }
                    }
                    pipes[pi].bytes = pipes[pi].pipe.state_bytes(&pipes[pi].st);
                }
                // quota di stato per pipeline (somma dei rami), dopo ogni micro-lotto
                if hello.quota_enabled {
                    let ids: Vec<String> = pipes
                        .iter()
                        .filter(|p| p.sidx == si)
                        .map(|p| p.pipe.def.id.clone())
                        .collect();
                    for id in ids {
                        let total: usize = pipes
                            .iter()
                            .filter(|p| p.pipe.def.id == id)
                            .map(|p| p.bytes)
                            .sum();
                        if total > quota {
                            for p in pipes
                                .iter_mut()
                                .filter(|p| p.pipe.def.id == id && p.suspended.is_none())
                            {
                                eprintln!(
                                    "stream: pipeline {id} sospesa: state_quota_exceeded ({total} byte oltre {quota})"
                                );
                                p.suspended = Some("state_quota_exceeded".into());
                            }
                        }
                    }
                }
                dirty = true;
            } else if rr.is_multiple_of(active.len()) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
        // checkpoint tra due lotti, solo se qualcosa e' cambiato (una pipeline sospesa e ferma non scrive)
        let delivered = cnt.delivered.load(Ordering::Relaxed);
        if delivered != last_delivered {
            dirty = true;
            last_delivered = delivered;
        }
        if force_ck || (last_ck.elapsed() >= ck_every && dirty) {
            let mut heads = vec![];
            let mut blobs = vec![];
            for p in pipes.iter_mut() {
                let (h, b) = p.pipe.save(&mut p.st).expect("salvataggio dello stato");
                heads.push(h);
                blobs.extend(b);
            }
            let head = CkHead {
                streams: srt
                    .iter()
                    .map(|s| StreamCk {
                        name: s.def.name.clone(),
                        pos: s.pos,
                        hw: s.hw,
                        events: s.events,
                    })
                    .collect(),
                pipes: heads,
                pending: pending.lock().unwrap().iter().cloned().collect(),
                suspended: pipes
                    .iter()
                    .filter_map(|p| p.suspended.clone().map(|c| (p.pipe.def.id.clone(), c)))
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            };
            write_ckpt(&hello.ckpt_dir, &key, &tid, &tenant.id, &head, &blobs);
            last_ck = Instant::now();
            dirty = false;
            force_ck = false;
        } else if last_ck.elapsed() >= ck_every {
            last_ck = Instant::now();
        }
        if last_stat.elapsed() >= Duration::from_secs(1) {
            let v = json!({
                "streams": srt.iter().map(|s| json!({"name":s.def.name,"pos":s.pos,"events":s.events,"bad":s.bad})).collect::<Vec<_>>(),
                "pipes": pipes.iter().map(|p| json!({
                    "id":p.pipe.def.id,"version":p.pipe.def.version,"branch":p.pipe.branch,
                    "late":p.st.late,"future":p.st.future,"wm":p.st.wm,"open_windows":p.st.wins.len(),
                    "state": if p.suspended.is_some() {"suspended"} else {"running"}, "code": p.suspended,
                    "state_bytes":p.bytes,"applied":p.st.applied,"dup_blocks":p.st.dup_blocks})).collect::<Vec<_>>(),
                "pending": pending.lock().unwrap().len(),
                "delivered": cnt.delivered.load(Ordering::Relaxed),
                "refused": cnt.refused.load(Ordering::Relaxed),
                "cpu_ns": cpu_ns(),
                "rss_kb": rss_kb(),
            });
            let _ = wr.write_all(format!("{v}\n").as_bytes()).await;
            last_stat = Instant::now();
        }
    }
}
