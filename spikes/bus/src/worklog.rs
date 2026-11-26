//! Log dei task, lato worker (blocco D): scrive righe nella sessione gRPC, con un numero progressivo per tentativo dato dal
//! worker (la verità di base per perdite e doppioni), le tiene finché il motore non conferma che il loro pezzo è durevole e le
//! rimanda a ogni ricollegamento.

use crate::worker::Slot;
use crate::{arg, now_us, pb};

/// Righe di un tentativo non ancora confermate dal motore: restano qui finché il loro pezzo non è durevole e si rimandano
/// a ogni ricollegamento della sessione che ha ricevuto la consegna. `generated == total` e nessuna riga = tutto confermato.
pub struct LogEntry {
    idx: usize,
    tenant: Vec<u8>,
    delivery_id: u64,
    lines: Vec<(u64, Vec<u8>)>,
    generated: u64,
    total: u64,
}

pub type LogBox =
    std::sync::Arc<std::sync::Mutex<std::collections::HashMap<(Vec<u8>, u32, u32), LogEntry>>>;

/// Profilo delle righe per tenant (argomenti `loga=NxL`, `logb=NxL`: N righe lunghe L byte per tentativo), pausa tra le due
/// metà delle righe (`logsplit_ms`, 0 = tutte insieme) e file della verità di base (`logw=`: una riga per tentativo con
/// esecuzione, passo, tentativo e righe scritte).
struct LogCfg {
    prof: [(u64, usize); 2],
    split_ms: u64,
    written: Option<std::sync::Mutex<std::fs::File>>,
}

fn log_cfg() -> &'static LogCfg {
    static C: std::sync::OnceLock<LogCfg> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let p = |k: &str| {
            let v = arg(k, "0x0");
            let (n, l) = v.split_once('x').expect("NxL");
            (n.parse().unwrap(), l.parse().unwrap())
        };
        let w = arg("logw", "");
        LogCfg {
            prof: [p("loga"), p("logb")],
            split_ms: arg("logsplit_ms", "0").parse().unwrap(),
            written: (!w.is_empty()).then(|| {
                std::sync::Mutex::new(
                    std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(w)
                        .unwrap(),
                )
            }),
        }
    })
}

/// Una riga: l'ora di scrittura (microsecondi, 16 cifre: la prova di visibilità la rilegge), l'identità del tentativo, il
/// numero; ogni decima riga porta il segreto consegnato (la prova della sostituzione), poi riempimento fino alla lunghezza.
fn line(d: &pb::Delivery, seq: u64, len: usize) -> Vec<u8> {
    let mut t = format!(
        "{:016} {} {} {} {seq:06} ",
        now_us(),
        &crate::crypto::hex(&d.exec)[..8],
        d.step,
        d.attempt
    )
    .into_bytes();
    if seq % 10 == 1 && !d.secret.is_empty() {
        t.extend_from_slice(b"token=");
        t.extend_from_slice(&d.secret);
        t.push(b' ');
    }
    if t.len() < len {
        t.resize(len, b'x');
    }
    t
}

/// Scrive le righe di un tentativo nella sessione: le tiene nella cassetta, le manda se la sessione c'è.
pub async fn task(d: pb::Delivery, idx: usize, slot: Slot, lb: LogBox) {
    let cfg = log_cfg();
    let Some(ti) = crate::engine::TENANTS
        .iter()
        .position(|t| t.as_slice() == d.tenant)
    else {
        return;
    };
    let (n, len) = cfg.prof[ti];
    if n == 0 {
        return;
    }
    let key = (d.exec.clone(), d.step, d.attempt);
    lb.lock().unwrap().insert(
        key.clone(),
        LogEntry {
            idx,
            tenant: d.tenant.clone(),
            delivery_id: d.delivery_id,
            lines: vec![],
            generated: 0,
            total: n,
        },
    );
    // una sola scrittura, oppure metà subito e metà dopo `split_ms`
    let first = if cfg.split_ms == 0 { n } else { n.div_ceil(2) };
    for (from, to, wait) in [(1, first, 0), (first + 1, n, cfg.split_ms)] {
        if from > to {
            continue;
        }
        if wait > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        }
        let lines: Vec<(u64, Vec<u8>)> = (from..=to).map(|s| (s, line(&d, s, len))).collect();
        let msg = {
            let mut g = lb.lock().unwrap();
            // l'entrata non può mancare: `generated < total` fino all'ultima scrittura, quindi non si rimuove prima
            let e = g.get_mut(&key).unwrap();
            e.lines.extend(lines.iter().cloned());
            e.generated = to;
            to_msg(&key, e, lines)
        };
        let tx = slot.lock().unwrap().clone();
        if let Some(tx) = tx {
            let _ = tx.send(msg).await;
        }
    }
    if let Some(w) = &cfg.written {
        use std::io::Write;
        let _ = w.lock().unwrap().write_all(
            format!(
                "{} {} {} {}\n",
                crate::crypto::hex(&d.exec),
                d.step,
                d.attempt,
                n
            )
            .as_bytes(),
        );
    }
}

fn to_msg(key: &(Vec<u8>, u32, u32), e: &LogEntry, lines: Vec<(u64, Vec<u8>)>) -> pb::FromWorker {
    pb::FromWorker {
        exec: key.0.clone(),
        step: key.1,
        attempt: key.2,
        delivery_id: e.delivery_id,
        tenant: e.tenant.clone(),
        lines: lines
            .into_iter()
            .map(|(seq, text)| pb::LogLine { seq, text })
            .collect(),
        ..Default::default()
    }
}

/// Messaggi con tutte le righe non confermate delle entrate di una sessione (al ricollegamento).
pub fn resend(lb: &LogBox, idx: usize) -> Vec<pb::FromWorker> {
    lb.lock()
        .unwrap()
        .iter()
        .filter(|(_, e)| e.idx == idx && !e.lines.is_empty())
        .map(|(k, e)| to_msg(k, e, e.lines.clone()))
        .collect()
}

/// Conferma del motore: le righe fino a `upto` sono durevoli e si tolgono; l'entrata finita sparisce.
pub fn confirm(lb: &LogBox, exec: &[u8], step: u32, attempt: u32, upto: u64) {
    let mut g = lb.lock().unwrap();
    let k = (exec.to_vec(), step, attempt);
    if let Some(e) = g.get_mut(&k) {
        e.lines.retain(|l| l.0 > upto);
        if e.lines.is_empty() && e.generated == e.total {
            g.remove(&k);
        }
    }
}
