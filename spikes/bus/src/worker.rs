//! Worker di prova (processo separato) e canale verso il motore. Il collegamento si sceglie da configurazione:
//! `link=grpc` (gRPC con TLS) o `link=iroh` (QUIC con iroh, `iroh_link.rs`); il lavoro di una consegna è comune.

use crate::{arg, now_us, pb};
use tokio::sync::mpsc;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

/// Canale TLS verso il motore; il certificato è quello generato da `gen-cert`.
pub fn channel(url: &str, dir: &str) -> Channel {
    let ca = std::fs::read(format!("{dir}/tls/cert.pem")).unwrap();
    let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca)).domain_name("localhost");
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
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) };
    t.tv_sec as f64 + t.tv_nsec as f64 / 1e9
}

/// Credenziale del worker: argomento `cred=` oppure file `out/worker.cred` (esadecimale).
pub fn credential(dir: &str) -> Vec<u8> {
    let s = std::env::args().rev().find_map(|a| a.strip_prefix("cred=").map(String::from)).unwrap_or_else(|| std::fs::read_to_string(format!("{dir}/worker.cred")).unwrap().trim().to_string());
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
        let engine: iroh::EndpointId = arg("engine_id", &std::fs::read_to_string(format!("{dir}/engine.nodeid")).unwrap_or_default().trim().to_string()).parse().expect("identità del motore");
        let sk = if arg("badkey", "0") == "1" { iroh::SecretKey::generate() } else { crate::iroh_link::key_from_cred(&cred) };
        crate::iroh_link::worker(sk, "127.0.0.1:0", engine, arg("iroh_addr", "127.0.0.1:47011").parse().unwrap(), cred.clone(), sessions, once).await;
        if once {
            return;
        }
        std::future::pending::<()>().await;
    }
    let ch = channel(url, &arg("tlsdir", dir));
    for _ in 0..sessions {
        let (ch, cred) = (ch.clone(), cred.clone());
        tokio::spawn(async move {
            loop {
                session(ch.clone(), cred.clone(), once).await;
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
    let hdr = [d.exec.as_slice(), &d.step.to_le_bytes(), &d.delivery_id.to_le_bytes()].concat();
    let n = hdr.len().min(out.len());
    out[..n].copy_from_slice(&hdr[..n]);
    pb::FromWorker {
        exec: d.exec,
        step: d.step,
        delivery_id: d.delivery_id,
        output: out,
        lat_ready_us: recv.saturating_sub(d.ready_us),
        lat_sent_us: recv.saturating_sub(d.sent_us),
        hello: false,
        credential: vec![],
    }
}

async fn session(ch: Channel, cred: Vec<u8>, once: bool) {
    let mut c = pb::worker_client::WorkerClient::new(ch).max_decoding_message_size(64 << 20).max_encoding_message_size(64 << 20);
    let (tx, rx) = mpsc::channel::<pb::FromWorker>(8192);
    let _ = tx.send(pb::FromWorker { hello: true, credential: cred, ..Default::default() }).await;
    let resp = match c.session(tokio_stream::wrappers::ReceiverStream::new(rx)).await {
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
    while let Ok(Some(d)) = inb.message().await {
        let recv = now_us();
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(work(d, recv).await).await;
        });
    }
}
