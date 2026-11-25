//! Pilota delle scene dell'annullamento (blocco E4, FR-10): `cancelctl phase=<fase> dir=out`. Accetta esecuzioni con la porta dell'accettazione
//! e le annulla con quella dell'annullamento, stampando una riga `CANCELCTL` per ogni richiesta (codice, ruolo, tempo dalla richiesta alla
//! risposta, cioè all'evento `Cancelled` durevole). Tutte le esecuzioni accettate si aggiungono a `out/ledger.txt` (la verifica finale le
//! conta: concluse o annullate). Lo stato in cui si trovava ogni esecuzione alla richiesta lo scrive il motore (`CANCEL_RECV … state=`).
//!
//! Fasi: `never` (senza worker collegato: passi in coda mai consegnati), `rest` (con il worker: tetto, in volo, ritentativo in attesa,
//! fan-out, già concluse, annullamento ripetuto, tutte insieme), `crash` (per la scena del crash: 100 esecuzioni lunghe annullate
//! tutte insieme, poi altre 100 non annullate).

use crate::engine::TENANTS;
use crate::{arg, now_us, pb, worker};
use std::io::Write;

type Client = pb::control_client::ControlClient<tonic::transport::Channel>;

/// Ingresso di 64 byte diverso a ogni chiamata: il motore indirizza i payload per impronta, due esecuzioni con lo stesso ingresso
/// si contendono la stessa voce della cache (come il generatore di carico, il primo byte cambia a ogni esecuzione).
fn input() -> Vec<u8> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut v = vec![7u8; 64];
    v[..8].copy_from_slice(
        &N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_le_bytes(),
    );
    v[8..16].copy_from_slice(&now_us().to_le_bytes());
    v
}

async fn accept(
    c: &mut Client,
    tenant: usize,
    kind: u32,
    n: u32,
    work_ms: u32,
    retry_ms: u32,
) -> Vec<u8> {
    let r = c
        .accept(pb::AcceptReq {
            tenant: TENANTS[tenant].to_vec(),
            kind,
            n,
            work_ms,
            out_size: 64,
            input: input(),
            retry_ms,
        })
        .await
        .expect("accettazione");
    r.into_inner().exec
}

async fn cancel(mut c: Client, phase: &str, exec: Vec<u8>, role: &str, name: &str) -> String {
    let t0 = now_us();
    let r = c
        .cancel(pb::CancelReq {
            exec: exec.clone(),
            role: role.into(),
            name: name.into(),
        })
        .await;
    let code = r.map_or_else(|e| format!("ERR:{}", e.message()), |r| r.into_inner().code);
    println!(
        "CANCELCTL phase={phase} exec={} code={code} role={role} latency_us={} at_us={t0}",
        crate::crypto::hex(&exec),
        now_us() - t0
    );
    code
}

fn ledger(dir: &str, ids: &[Vec<u8>]) {
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{dir}/ledger.txt"))
        .unwrap();
    for id in ids {
        writeln!(f, "{}", crate::crypto::hex(id)).unwrap();
    }
}

pub async fn run(url: &str, dir: &str) {
    let ch = worker::channel(url, dir);
    let mut c: Client = pb::control_client::ControlClient::new(ch);
    let phase = arg("phase", "rest");
    let mut all: Vec<Vec<u8>> = vec![];
    match phase.as_str() {
        // sei esecuzioni da due passi, tenant B; senza worker collegato nessun passo è consegnato; le prime tre si annullano
        "never" => {
            let mut ids = vec![];
            for _ in 0..6 {
                ids.push(accept(&mut c, 1, 0, 2, 300, 0).await);
            }
            for id in &ids[..3] {
                cancel(c.clone(), "never", id.clone(), "integrator", "Maria Rossi").await;
            }
            all = ids;
        }
        "rest" => {
            let ten = |c: &Client| c.clone();
            // tetto (tenant A, `cap` piccolo): 20 esecuzioni da un passo; la prima è in volo, le ultime sono trattenute dal tetto
            let cap = {
                let mut c = ten(&c);
                async move {
                    let mut ids = vec![];
                    for _ in 0..20 {
                        ids.push(accept(&mut c, 0, 0, 1, 1500, 0).await);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    cancel(c.clone(), "inflight", ids[0].clone(), "tenant", "").await;
                    cancel(c.clone(), "held", ids[19].clone(), "tenant", "").await;
                    cancel(
                        c.clone(),
                        "held",
                        ids[18].clone(),
                        "integrator",
                        "Luca Bianchi",
                    )
                    .await;
                    // annullamento ripetuto: nessun effetto, codice, idempotente
                    cancel(c.clone(), "twice", ids[0].clone(), "tenant", "").await;
                    cancel(c.clone(), "twice", ids[19].clone(), "integrator", "").await;
                    ids
                }
            };
            // ritentativo in attesa (tenant B): il primo tentativo fallisce subito, l'attesa è di 30 s; si annullano tutte e tre
            let retry = {
                let mut c = ten(&c);
                async move {
                    let mut ids = vec![];
                    for _ in 0..3 {
                        ids.push(accept(&mut c, 1, 3, 1, 50, 30_000).await);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    for id in &ids {
                        cancel(c.clone(), "retrywait", id.clone(), "tenant", "Anna Verdi").await;
                    }
                    ids
                }
            };
            // fan-out (tenant B): quattro rami lenti insieme (`fan_slow`), si annulla una delle due esecuzioni
            let fan = {
                let mut c = ten(&c);
                async move {
                    let a = accept(&mut c, 1, 1, 4, 2000, 0).await;
                    let b = accept(&mut c, 1, 1, 4, 2000, 0).await;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    cancel(c.clone(), "fanout", a.clone(), "tenant", "").await;
                    vec![a, b]
                }
            };
            // già concluse (tenant B): passi senza lavoro, poi si annullano a esecuzione finita
            let fin = {
                let mut c = ten(&c);
                async move {
                    let mut ids = vec![];
                    for _ in 0..3 {
                        ids.push(accept(&mut c, 1, 0, 1, 0, 0).await);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    for id in &ids {
                        cancel(c.clone(), "finished", id.clone(), "tenant", "").await;
                    }
                    ids
                }
            };
            // le tre fasi del tenant B girano una dopo l'altra (il tetto di 4 è per tenant: insieme si trattengono a vicenda e lo stato
            // alla richiesta non sarebbe quello voluto); il tenant A (tetto) gira insieme
            let tenant_b = async move {
                let g = fin.await;
                let b = retry.await;
                let f = fan.await;
                (b, f, g)
            };
            let (a, (b, f, g)) = tokio::join!(cap, tenant_b);
            all = [a, b, f, g].concat();
        }
        // 100 esecuzioni lunghe annullate tutte insieme (con consegne in volo), poi altre 100 non annullate
        "crash" => {
            let mut ids = vec![];
            for _ in 0..100 {
                ids.push(accept(&mut c, 0, 0, 3, 5000, 0).await);
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let mut hs = vec![];
            for id in ids.clone() {
                hs.push(tokio::spawn(cancel(
                    c.clone(),
                    "crash",
                    id,
                    "integrator",
                    "Carla Neri",
                )));
            }
            for h in hs {
                h.await.unwrap();
            }
            for _ in 0..100 {
                ids.push(accept(&mut c, 0, 0, 3, 5000, 0).await);
            }
            all = ids;
        }
        p => println!("fasi: never rest crash (ricevuto {p})"),
    }
    ledger(dir, &all);
    println!("CANCELCTL phase={phase} accettate={}", all.len());
}
