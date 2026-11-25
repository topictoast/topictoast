//! Worker di prova (processo separato) e canale verso il motore. Il collegamento si sceglie da configurazione:
//! `link=grpc` (gRPC con TLS) o `link=iroh` (QUIC con iroh, `iroh_link.rs`); il lavoro di una consegna è comune.

use crate::{arg, now_us, pb};
use tokio::sync::mpsc;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

/// Canale TLS verso il motore; il certificato è quello generato da `gen-cert`.
pub fn channel(url: &str, dir: &str) -> Channel {
    let ca = std::fs::read(format!("{dir}/tls/cert.pem")).unwrap();
    let tls = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(ca))
        .domain_name("localhost");
    Channel::from_shared(url.to_string())
        .unwrap()
        .tls_config(tls)
        .unwrap()
        .http2_adaptive_window(true)
        .tcp_nodelay(true)
        .connect_lazy()
}

/// CPU del processo in secondi, come `engine_cpu_s` del motore.
fn cpu_s() -> f64 {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) };
    t.tv_sec as f64 + t.tv_nsec as f64 / 1e9
}

/// Credenziale del worker: argomento `cred=` oppure file `out/worker.cred` (esadecimale).
pub fn credential(dir: &str) -> Vec<u8> {
    let s = std::env::args()
        .rev()
        .find_map(|a| a.strip_prefix("cred=").map(String::from))
        .unwrap_or_else(|| {
            std::fs::read_to_string(format!("{dir}/worker.cred"))
                .unwrap()
                .trim()
                .to_string()
        });
    s.into_bytes()
}

pub async fn run(url: &str, dir: &str, sessions: usize) {
    // la CPU del worker si scrive su file ogni 200 ms; il generatore di carico ne legge la differenza sulla finestra
    let cpu_path = format!("{dir}/worker.cpu");
    tokio::spawn(async move {
        loop {
            let _ = std::fs::write(&cpu_path, format!("{:.3}", cpu_s()));
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    });
    let once = arg("once", "0") == "1";
    let cred = credential(dir);
    if arg("link", "grpc") == "iroh" {
        let engine: iroh::EndpointId = arg(
            "engine_id",
            &std::fs::read_to_string(format!("{dir}/engine.nodeid"))
                .unwrap_or_default()
                .trim()
                .to_string(),
        )
        .parse()
        .expect("identità del motore");
        let sk = if arg("badkey", "0") == "1" {
            iroh::SecretKey::generate()
        } else {
            crate::iroh_link::key_from_cred(&cred)
        };
        crate::iroh_link::worker(
            sk,
            "127.0.0.1:0",
            engine,
            arg("iroh_addr", "127.0.0.1:47011").parse().unwrap(),
            cred.clone(),
            sessions,
            once,
        )
        .await;
        if once {
            return;
        }
        std::future::pending::<()>().await;
    }
    let ch = channel(url, &arg("tlsdir", dir));
    // Esiti finiti e non ancora confermati dal motore (fase 4, blocco B): sopravvivono alla sessione, si rimandano
    // a ogni nuovo collegamento della stessa sessione (indice) e si tolgono solo con la conferma durevole `ack`.
    let outbox: Outbox = Default::default();
    let logbox: crate::worklog::LogBox = Default::default();
    for idx in 0..sessions {
        let (ch, cred, outbox, logbox) = (ch.clone(), cred.clone(), outbox.clone(), logbox.clone());
        tokio::spawn(async move {
            let slot: Slot = Default::default();
            loop {
                session(
                    ch.clone(),
                    cred.clone(),
                    once,
                    idx,
                    slot.clone(),
                    outbox.clone(),
                    logbox.clone(),
                )
                .await;
                if once {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });
    }
    if once {
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        return;
    }
    std::future::pending::<()>().await;
}

/// Il lavoro di una consegna: durata simulata, poi un'uscita unica per attività (intestazione con esecuzione, passo e consegna).
pub async fn work(d: pb::Delivery, recv: u64) -> pb::FromWorker {
    if d.work_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(d.work_ms as u64)).await;
    }
    let mut out = vec![(d.delivery_id & 0xff) as u8; d.out_size as usize];
    let hdr = [
        d.exec.as_slice(),
        &d.step.to_le_bytes(),
        &d.delivery_id.to_le_bytes(),
    ]
    .concat();
    let n = hdr.len().min(out.len());
    out[..n].copy_from_slice(&hdr[..n]);
    pb::FromWorker {
        exec: d.exec,
        step: d.step,
        delivery_id: d.delivery_id,
        output: if d.fail { vec![] } else { out },
        lat_ready_us: recv.saturating_sub(d.ready_us),
        lat_sent_us: recv.saturating_sub(d.sent_us),
        hello: false,
        credential: vec![],
        failed: d.fail,
        heartbeat: false,
        ..Default::default()
    }
}

async fn session(
    ch: Channel,
    cred: Vec<u8>,
    once: bool,
    idx: usize,
    slot: Slot,
    outbox: Outbox,
    logbox: crate::worklog::LogBox,
) {
    let mut c = pb::worker_client::WorkerClient::new(ch)
        .max_decoding_message_size(64 << 20)
        .max_encoding_message_size(64 << 20);
    let (tx, rx) = mpsc::channel::<pb::FromWorker>(8192);
    let _ = tx
        .send(pb::FromWorker {
            hello: true,
            credential: cred,
            ..Default::default()
        })
        .await;
    let resp = match c
        .session(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if once {
                println!("RIFIUTATO: {e}");
            }
            return;
        }
    };
    let mut inb = resp.into_inner();
    if once {
        // con il saluto rifiutato il motore chiude il flusso con un errore entro poco
        match tokio::time::timeout(std::time::Duration::from_millis(1500), inb.message()).await {
            Ok(Err(e)) => println!("RIFIUTATO: {e}"),
            _ => println!("ACCETTATO"),
        }
        return;
    }
    // da qui i nuovi esiti escono da questa sessione; quelli rimasti nella cassetta si rimandano subito (il motore
    // scarta e conferma i doppioni)
    *slot.lock().unwrap() = Some(tx.clone());
    let pending: Vec<_> = outbox
        .lock()
        .unwrap()
        .values()
        .filter(|(i, _)| *i == idx)
        .map(|(_, r)| r.clone())
        .collect();
    for r in pending {
        let _ = tx.send(r).await;
    }
    // righe di log non confermate di questa sessione: si rimandano tutte (il motore scarta i doppioni per numero di riga)
    for m in crate::worklog::resend(&logbox, idx) {
        let _ = tx.send(m).await;
    }
    while let Ok(Some(d)) = inb.message().await {
        if d.log_ack != 0 {
            crate::worklog::confirm(&logbox, &d.exec, d.step, d.attempt, d.log_ack);
            continue;
        }
        if d.ack != 0 {
            outbox.lock().unwrap().remove(&d.ack);
            continue;
        }
        let recv = now_us();
        let (slot, outbox) = (slot.clone(), outbox.clone());
        tokio::spawn(crate::worklog::task(
            d.clone(),
            idx,
            slot.clone(),
            logbox.clone(),
        ));
        tokio::spawn(async move {
            log_start(&d);
            // battiti (blocco C): con `lease_ms` il worker segnala ogni lease_ms/3 che il tentativo è vivo
            let hb = (d.lease_ms > 0).then(|| {
                let (slot, exec, step, did, every) = (
                    slot.clone(),
                    d.exec.clone(),
                    d.step,
                    d.delivery_id,
                    std::time::Duration::from_millis((d.lease_ms / 3).max(1) as u64),
                );
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(every).await;
                        let tx = slot.lock().unwrap().clone();
                        if let Some(tx) = tx {
                            let _ = tx
                                .send(pb::FromWorker {
                                    exec: exec.clone(),
                                    step,
                                    delivery_id: did,
                                    heartbeat: true,
                                    ..Default::default()
                                })
                                .await;
                        }
                    }
                })
            });
            let r = work(d, recv).await;
            if let Some(h) = hb {
                h.abort();
            }
            outbox
                .lock()
                .unwrap()
                .insert(r.delivery_id, (idx, r.clone()));
            let tx = slot.lock().unwrap().clone();
            if let Some(tx) = tx {
                let _ = tx.send(r).await;
            }
        });
    }
    *slot.lock().unwrap() = None;
}

/// Sessione di un collegamento: il suo mittente corrente (vuoto tra un collegamento e il successivo).
pub(crate) type Slot = std::sync::Arc<std::sync::Mutex<Option<mpsc::Sender<pb::FromWorker>>>>;
/// Esiti non confermati, per `delivery_id`, con l'indice della sessione che li rimanda.
type Outbox =
    std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u64, (usize, pb::FromWorker)>>>;

/// Registro degli inizi di tentativo (argomento `worklog=`): una riga `esecuzione passo consegna istante_us tentativo` per
/// tentativo, scritta subito (sopravvive a `kill -9`). Serve a contare i tentativi eseguiti due volte.
fn log_start(d: &pb::Delivery) {
    use std::io::Write;
    static LOG: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
        std::sync::OnceLock::new();
    let log = LOG.get_or_init(|| {
        let p = arg("worklog", "");
        (!p.is_empty()).then(|| {
            std::sync::Mutex::new(
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(p)
                    .unwrap(),
            )
        })
    });
    if let Some(f) = log {
        // una sola write per riga: due worker scrivono nello stesso file in append e le righe non devono mescolarsi
        let _ = f.lock().unwrap().write_all(
            format!(
                "{} {} {} {} {}\n",
                crate::crypto::hex(&d.exec),
                d.step,
                d.delivery_id,
                now_us(),
                d.attempt
            )
            .as_bytes(),
        );
    }
}
