//! Ruolo `stream`: un processo per tenant (AD-6, AD-13). Riceve sul descrittore 3 (coppia di socket
//! ereditata) la chiave del suo tenant e la credenziale di sola lettura del bus; legge i topic del tenant,
//! decifra, deduplica i blocchi per `Idempotency-Key`, esegue le pipeline a micro-lotti, scrive i checkpoint
//! e consegna le righe delle finestre chiuse come avvii al motore.
//!
//! Nessun orologio di sistema nella semantica: il tempo viene solo dal log (arrivo nei blocchi e nei
//! segnali). I timer `Instant` servono solo a decidere quando raggruppare e quando fare il checkpoint.
//!
//! Confinamento (descrittori, namespace, Landlock, seccomp: PIANO 1.12) e quota di stato: parte 2.

use crate::bus::Bus;
use crate::crypto::{TenantKey, new_id, tenant_uid, unhex};
use crate::engine::Hello;
use crate::http::Client;
use crate::pipeline::{Pipe, PipeHead, PipeState, Row};
use crate::spec::*;
use crate::wire::{self, Msg};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::os::fd::FromRawFd;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

type Pending = Arc<Mutex<VecDeque<Row>>>;

struct SRt {
    def: StreamDef,
    topic: String,
    pos: u64,
    /// chiavi dei blocchi viste. ponytail: cresce con lo stream e si salva intero nel checkpoint; il prodotto
    /// la pota con la conservazione (le chiavi piu' vecchie della conservazione non servono), parte 2.
    seen: HashSet<String>,
    dup_blocks: u64,
    events: u64,
    bad: u64,
}

#[derive(Serialize, Deserialize)]
struct StreamCk {
    name: String,
    pos: u64,
    seen: Vec<String>,
    dup_blocks: u64,
    events: u64,
}

#[derive(Serialize, Deserialize)]
struct CkHead {
    streams: Vec<StreamCk>,
    pipes: Vec<PipeHead>,
    pending: Vec<Row>,
}

/// Checkpoint: file temporaneo, `fdatasync`, rinomina, `fsync` della cartella (PIANO 1.10), cifrato C-12.
fn write_ckpt(dir: &str, key: &TenantKey, tid: &[u8; 16], tenant: &str, head: &CkHead, blobs: &[Vec<u8>]) {
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

/// Consegna le righe in attesa come avvii, con la chiave C-4; riprova finche' il motore accetta.
/// Un rifiuto per quota (429/503) trattiene la riga e la riprova.
async fn deliver(pending: Pending, api: String, token: String) {
    let mut c = Client::new(&api);
    loop {
        let row = pending.lock().unwrap().front().cloned();
        let Some(row) = row else {
            tokio::time::sleep(Duration::from_millis(20)).await;
            continue;
        };
        let auth = format!("Bearer {token}");
        match c
            .post("/v1/starts", &[("Authorization", &auth), ("Idempotency-Key", &row.key)], row.body.to_string().as_bytes())
            .await
        {
            Ok((200, _)) => {
                pending.lock().unwrap().pop_front();
            }
            _ => tokio::time::sleep(Duration::from_millis(200)).await,
        }
    }
}

pub async fn run() {
    // coppia di socket ereditata: descrittore 3
    let std_sock = unsafe { std::os::unix::net::UnixStream::from_raw_fd(3) };
    std_sock.set_nonblocking(true).unwrap();
    let sock = tokio::net::UnixStream::from_std(std_sock).unwrap();
    let (rd, mut wr) = sock.into_split();
    let mut lines = BufReader::new(rd).lines();
    let hello: Hello = serde_json::from_str(&lines.next_line().await.unwrap().expect("nessun messaggio iniziale")).unwrap();
    // se il motore sparisce (EOF) il processo termina
    tokio::spawn(async move {
        while let Ok(Some(_)) = lines.next_line().await {}
        std::process::exit(0);
    });
    let key = TenantKey(unhex(&hello.key_hex).try_into().unwrap());
    let tenant = hello.tenant.clone();
    let tid = tenant_uid(&tenant.id);
    let bus = Bus::connect(&hello.iggy_addr, &hello.iggy_user, &hello.iggy_pass).await.expect("bus");

    let mut pipes = vec![];
    for p in &tenant.pipelines {
        match Pipe::build(p, &tenant.streams, hello.state_quota).await {
            Ok(x) => pipes.push(x),
            Err(e) => {
                eprintln!("stream: pipeline {} rifiutata: {e}", p.id);
                std::process::exit(2);
            }
        }
    }
    let mut srt: Vec<SRt> = tenant
        .streams
        .iter()
        .map(|s| SRt {
            topic: topic_of(&tenant.id, &s.name),
            def: s.clone(),
            pos: 0,
            seen: HashSet::new(),
            dup_blocks: 0,
            events: 0,
            bad: 0,
        })
        .collect();
    let mut states: Vec<PipeState> = pipes.iter().map(|p| p.new_state()).collect();
    let pending: Pending = Arc::new(Mutex::new(VecDeque::new()));

    // ripresa dall'ultimo checkpoint
    if let Some((head, blobs)) = read_ckpt(&hello.ckpt_dir, &key, &tenant.id) {
        for s in &mut srt {
            if let Some(c) = head.streams.iter().find(|c| c.name == s.def.name) {
                s.pos = c.pos;
                s.seen = c.seen.iter().cloned().collect();
                s.dup_blocks = c.dup_blocks;
                s.events = c.events;
            }
        }
        let mut bi = 0;
        for h in &head.pipes {
            let n = h.wins.len();
            if let Some(i) = pipes.iter().position(|p| p.def.id == h.id && p.def.version == h.version) {
                let refs: Vec<&[u8]> = blobs[bi..bi + n].iter().map(|b| b.as_slice()).collect();
                states[i] = pipes[i].restore(h, &refs).expect("stato del checkpoint illeggibile");
            }
            bi += n;
        }
        *pending.lock().unwrap() = head.pending.into_iter().collect();
        eprintln!("stream {}: ripreso dal checkpoint, righe in attesa {}", tenant.id, pending.lock().unwrap().len());
    }
    tokio::spawn(deliver(pending.clone(), hello.api_addr.clone(), hello.api_token.clone()));

    let ck_every = Duration::from_secs(10);
    let mut last_ck = Instant::now();
    let mut last_stat = Instant::now();
    let mut rr = 0usize;
    loop {
        let si = rr % srt.len();
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
            for (off, payload) in got {
                srt[si].pos = off + 1;
                match key.open(&srt[si].topic, &payload) {
                    Some(plain) => {
                        let m = wire::decode(&plain, true);
                        if let Msg::Block { events, .. } = &m {
                            rows += *events as usize;
                        }
                        msgs.push((off, m));
                    }
                    None => srt[si].bad += 1,
                }
            }
            if rows >= 8192 || start.elapsed() >= Duration::from_millis(50) {
                break;
            }
        }
        if !msgs.is_empty() {
            // deduplica dei blocchi per Idempotency-Key entro la conservazione (il primo vince)
            let mut keep: Vec<(u64, &Msg)> = vec![];
            for (off, m) in &msgs {
                if let Msg::Block { key, events, .. } = m {
                    if !srt[si].seen.insert(key.clone()) {
                        srt[si].dup_blocks += 1;
                        continue;
                    }
                    srt[si].events += *events as u64;
                }
                keep.push((*off, m));
            }
            for (pi, p) in pipes.iter().enumerate() {
                if p.stream != srt[si].def.name {
                    continue;
                }
                match p.process(&mut states[pi], &keep).await {
                    Ok(r) => pending.lock().unwrap().extend(r),
                    Err(e) => {
                        eprintln!("stream: pipeline {} in errore: {e}", p.def.id);
                        std::process::exit(3);
                    }
                }
            }
        } else if si == srt.len() - 1 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        // checkpoint tra due lotti
        if last_ck.elapsed() >= ck_every {
            let mut heads = vec![];
            let mut blobs = vec![];
            for (pi, p) in pipes.iter().enumerate() {
                let (h, b) = p.save(&mut states[pi]).expect("salvataggio dello stato");
                heads.push(h);
                blobs.extend(b);
            }
            let head = CkHead {
                streams: srt
                    .iter()
                    .map(|s| StreamCk {
                        name: s.def.name.clone(),
                        pos: s.pos,
                        seen: s.seen.iter().cloned().collect(),
                        dup_blocks: s.dup_blocks,
                        events: s.events,
                    })
                    .collect(),
                pipes: heads,
                pending: pending.lock().unwrap().iter().cloned().collect(),
            };
            write_ckpt(&hello.ckpt_dir, &key, &tid, &tenant.id, &head, &blobs);
            last_ck = Instant::now();
        }
        if last_stat.elapsed() >= Duration::from_secs(1) {
            let v = json!({
                "streams": srt.iter().map(|s| json!({"name":s.def.name,"pos":s.pos,"events":s.events,"dup_blocks":s.dup_blocks,"bad":s.bad})).collect::<Vec<_>>(),
                "pipes": pipes.iter().zip(&states).map(|(p, s)| json!({"id":p.def.id,"late":s.late,"future":s.future,"wm":s.wm,"open_windows":s.wins.len()})).collect::<Vec<_>>(),
                "pending": pending.lock().unwrap().len(),
            });
            let _ = wr.write_all(format!("{v}\n").as_bytes()).await;
            last_stat = Instant::now();
        }
    }
}
