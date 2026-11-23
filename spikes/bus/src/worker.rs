//! Worker di prova (processo separato, gRPC con TLS) e canale verso il motore.

use crate::{now_us, pb};
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

pub async fn run(url: &str, dir: &str, sessions: usize) {
    let ch = channel(url, dir);
    for _ in 0..sessions {
        let ch = ch.clone();
        tokio::spawn(async move {
            loop {
                session(ch.clone()).await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });
    }
    std::future::pending::<()>().await;
}

async fn session(ch: Channel) {
    let mut c = pb::worker_client::WorkerClient::new(ch).max_decoding_message_size(64 << 20).max_encoding_message_size(64 << 20);
    let (tx, rx) = mpsc::channel::<pb::FromWorker>(8192);
    let _ = tx.send(pb::FromWorker { hello: true, ..Default::default() }).await;
    let Ok(resp) = c.session(tokio_stream::wrappers::ReceiverStream::new(rx)).await else { return };
    let mut inb = resp.into_inner();
    while let Ok(Some(d)) = inb.message().await {
        let recv = now_us();
        let tx = tx.clone();
        tokio::spawn(async move {
            if d.work_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(d.work_ms as u64)).await;
            }
            // uscita unica per attività: intestazione con esecuzione, passo e consegna, poi riempimento
            let mut out = vec![(d.delivery_id & 0xff) as u8; d.out_size as usize];
            let hdr = [d.exec.as_slice(), &d.step.to_le_bytes(), &d.delivery_id.to_le_bytes()].concat();
            let n = hdr.len().min(out.len());
            out[..n].copy_from_slice(&hdr[..n]);
            let _ = tx
                .send(pb::FromWorker {
                    exec: d.exec,
                    step: d.step,
                    delivery_id: d.delivery_id,
                    output: out,
                    lat_ready_us: recv.saturating_sub(d.ready_us),
                    lat_sent_us: recv.saturating_sub(d.sent_us),
                    hello: false,
                })
                .await;
        });
    }
}
