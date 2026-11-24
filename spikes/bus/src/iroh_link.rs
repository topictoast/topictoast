//! Collegamento dei worker su QUIC con iroh (fase 3): stessa forma della variante gRPC, cioè una connessione per
//! worker e 4 sessioni bidirezionali che portano gli stessi messaggi `Delivery` e `FromWorker`, con un prefisso di
//! lunghezza (u32 big-endian) al posto del flusso di gRPC.
//!
//! Sicurezza (C-11): relay e scoperta esterna sono spenti (`presets::Minimal` + `RelayMode::Disabled` +
//! `clear_address_lookup`), il motore indica al worker identità e indirizzo da configurazione, e l'identità di nodo del
//! worker è DERIVATA dalla sua credenziale: il motore accetta una connessione solo se l'identità del nodo è tra quelle
//! derivate dalle credenziali note, e ogni sessione ripete la credenziale nel saluto. Il worker controlla che il motore
//! abbia l'identità attesa (iroh la verifica nell'handshake TLS).

use crate::engine::Msg;
use crate::{now_us, pb};
use iroh::endpoint::{RecvStream, SendStream, presets};
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, SecretKey};
use prost::Message;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use tokio::sync::mpsc;

pub const ALPN: &[u8] = b"topictoast/worker/1";
const MAX_FRAME: usize = 64 << 20;

/// Chiave del nodo del worker derivata dalla credenziale: legame tra identità e credenziale.
pub fn key_from_cred(cred: &[u8]) -> SecretKey {
    let h: [u8; 32] = Sha256::new().chain_update(b"iroh-worker-v1").chain_update(cred).finalize().into();
    SecretKey::from_bytes(&h)
}

fn cred_hash(cred: &[u8]) -> [u8; 32] {
    Sha256::digest(cred).into()
}

/// Endpoint senza relay e senza scoperta esterna.
pub async fn endpoint(sk: SecretKey, bind: &str) -> Endpoint {
    let ep = Endpoint::builder(presets::Minimal)
        .secret_key(sk)
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(RelayMode::Disabled)
        .clear_address_lookup()
        // solo l'indirizzo indicato: senza questa riga iroh apre anche prese UDP su tutte le interfacce (0.0.0.0 e [::])
        .clear_ip_transports()
        .bind_addr(bind.parse::<SocketAddr>().unwrap())
        .unwrap()
        .bind()
        .await
        .expect("bind iroh");
    ep
}

async fn read_frame(r: &mut RecvStream) -> Option<Vec<u8>> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).await.ok()?;
    let n = u32::from_be_bytes(len) as usize;
    if n > MAX_FRAME {
        return None;
    }
    let mut b = vec![0u8; n];
    r.read_exact(&mut b).await.ok()?;
    Some(b)
}

async fn write_frame(w: &mut SendStream, body: &[u8]) -> bool {
    let mut b = Vec::with_capacity(4 + body.len());
    b.extend_from_slice(&(body.len() as u32).to_be_bytes());
    b.extend_from_slice(body);
    w.write_all(&b).await.is_ok()
}

// ---------------------------------------------------------------- lato motore

/// Accetta connessioni dei worker. `allowed`: identità di nodo attese → impronta della credenziale.
pub async fn serve(ep: Endpoint, allowed: Arc<HashMap<EndpointId, [u8; 32]>>, tx: mpsc::Sender<Msg>, next: Arc<AtomicU64>) {
    while let Some(incoming) = ep.accept().await {
        let (allowed, tx, next) = (allowed.clone(), tx.clone(), next.clone());
        tokio::spawn(async move {
            let Ok(acc) = incoming.accept() else { return };
            let Ok(conn) = acc.await else { return };
            // controllo a ogni collegamento: l'identità del nodo (provata dall'handshake) deve essere derivata da una credenziale nota
            let Some(want) = allowed.get(&conn.remote_id()).copied() else {
                conn.close(1u32.into(), b"E_IDENTITA");
                return;
            };
            while let Ok((send, recv)) = conn.accept_bi().await {
                let (tx, next, conn2) = (tx.clone(), next.clone(), conn.clone());
                tokio::spawn(async move {
                    if !session(send, recv, want, tx, next.fetch_add(1, Relaxed)).await {
                        conn2.close(2u32.into(), b"E_CREDENZIALE");
                    }
                });
            }
        });
    }
}

/// Una sessione: il saluto porta la credenziale, che deve corrispondere a quella legata al nodo. Torna false se rifiutata.
async fn session(mut send: SendStream, mut recv: RecvStream, want: [u8; 32], tx: mpsc::Sender<Msg>, sess: u64) -> bool {
    let Some(first) = read_frame(&mut recv).await else { return true };
    let Ok(h) = pb::FromWorker::decode(&first[..]) else { return false };
    if !h.hello || cred_hash(&h.credential) != want {
        return false;
    }
    let (dtx, mut drx) = mpsc::channel::<Result<pb::Delivery, tonic::Status>>(8192);
    if tx.send(Msg::Register { sess, tx: dtx }).await.is_err() {
        return true;
    }
    tokio::spawn(async move {
        while let Some(Ok(d)) = drx.recv().await {
            if !write_frame(&mut send, &d.encode_to_vec()).await {
                break;
            }
        }
    });
    while let Some(b) = read_frame(&mut recv).await {
        if let Ok(m) = pb::FromWorker::decode(&b[..])
            && !m.hello
        {
            let _ = tx.send(Msg::Result { sess, r: m }).await;
        }
    }
    let _ = tx.send(Msg::Unregister { sess }).await;
    true
}

// ---------------------------------------------------------------- lato worker

/// Connessione del worker: una connessione, `sessions` sessioni bidirezionali. In modalità `once` prova il collegamento
/// e riporta se è stato rifiutato, senza ritentare.
pub async fn worker(sk: SecretKey, bind: &str, engine: EndpointId, addr: SocketAddr, cred: Vec<u8>, sessions: usize, once: bool) {
    let ep = endpoint(sk, bind).await;
    if once {
        // prova di spegnimento di relay e scoperta: nessun indirizzo relay, nessun servizio di scoperta
        println!("iroh: relay_urls={} address_lookup={:?}", ep.addr().relay_urls().count(), ep.address_lookup().map(|s| format!("{s:?}")).map_err(|e| e.to_string()));
    }
    loop {
        let r = ep.connect(EndpointAddr::new(engine).with_ip_addr(addr), ALPN).await;
        match r {
            Err(e) => {
                if once {
                    println!("RIFIUTATO: connessione fallita ({e})");
                    return;
                }
            }
            Ok(conn) => {
                let mut tasks = vec![];
                for _ in 0..sessions {
                    let Ok((send, recv)) = conn.open_bi().await else { break };
                    tasks.push(tokio::spawn(wsession(send, recv, cred.clone())));
                }
                if once {
                    tokio::select! {
                        _ = conn.closed() => println!("RIFIUTATO: connessione chiusa dal motore"),
                        _ = tokio::time::sleep(std::time::Duration::from_millis(1500)) => println!("ACCETTATO"),
                    }
                    return;
                }
                conn.closed().await;
                for t in tasks {
                    t.abort();
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

async fn wsession(mut send: SendStream, mut recv: RecvStream, cred: Vec<u8>) {
    let hello = pb::FromWorker { hello: true, credential: cred, ..Default::default() };
    if !write_frame(&mut send, &hello.encode_to_vec()).await {
        return;
    }
    let (tx, mut rx) = mpsc::channel::<pb::FromWorker>(8192);
    tokio::spawn(async move {
        while let Some(m) = rx.recv().await {
            if !write_frame(&mut send, &m.encode_to_vec()).await {
                break;
            }
        }
    });
    while let Some(b) = read_frame(&mut recv).await {
        let recv_us = now_us();
        let Ok(d) = pb::Delivery::decode(&b[..]) else { continue };
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(crate::worker::work(d, recv_us).await).await;
        });
    }
}
