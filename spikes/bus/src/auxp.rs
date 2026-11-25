//! Processo ausiliario (AD-6), blocco E2: il sottocomando `aux` dello stesso binario, avviato dal motore come figlio con
//! l'ambiente VUOTO. Valuta il filtro `if`, la mappatura jq dell'ingresso e controlla ingressi e uscite contro schemi JSON
//! (2020-12), senza riferimenti esterni (AD-10: `jsonschema` senza `resolve-http` né `resolve-file`).
//!
//! **IPC:** pipe stdin/stdout, frame con prefisso di lunghezza `[u32 LE: byte che seguono][u64 LE: id richiesta][u8: tipo][carico]`.
//! Il motore manda per primo il frame `HELLO` con la versione dell'IPC; l'ausiliario esce con codice 3 se è diversa e risponde
//! `HELLO` se uguale. Poi richieste in pipeline (un processo, un task scrittore e un task lettore nel motore): `PREPARE` (carico =
//! ingresso JSON; risposta = stato + ingresso mappato) e `CHECK` (carico = uscita JSON; risposta = stato + messaggio). Stato: 0 ok,
//! 1 passo saltato (`if` falso), 2 errore (messaggio con il percorso del campo).
//!
//! Il contenuto del manifest della prova è fisso in costanti di questo file (`IF_PROG`, `MAP_PROG`, `INPUT_SCHEMA`,
//! `OUTPUT_SCHEMA`): una storia vera lo riceverebbe dal motore.
//!
//! **Se l'ausiliario cade:** le richieste in corso tornano con errore al chiamante; `dispatch` rimette l'attività in testa alla
//! coda dei pronti e non consegna nulla (nessun lavoro perso, ma nessun avanzamento: il riavvio dell'ausiliario non c'è, non
//! coperto). Il controllo delle uscite, in quel caso, segna il tentativo come fallito (si ritenta).
// ponytail: un solo ausiliario seriale (legge, elabora, risponde in ordine); più processi o un thread per richiesta se la CPU di
// un core non basta (riga a 10.000 att./s).

use jaq_core::load::{Arena, File, Loader};
use jaq_core::{Compiler, Ctx, Vars, data, unwrap_valr};
use jaq_json::{Val, read};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};

pub const IPC_VERSION: u32 = 1;
const T_HELLO: u8 = 0;
const T_PREPARE: u8 = 1;
const T_CHECK: u8 = 2;

/// Filtro `if` del passo (un booleano jq sull'ingresso).
pub const IF_PROG: &str = ".flag == true";
/// Mappatura dell'ingresso: 7 campi, con un'aritmetica (`+ 1`, `qty * price`) e operazioni su array (`map`, `add`, `length`).
pub const MAP_PROG: &str = "{id: .id, seq: (.seq + 1), name: .name, tags: (.tags | map(ascii_upcase)), total: ([.items[] | .qty * .price] | add), n: (.items | length), data: .data}";
/// Schema dell'ingresso MAPPATO: sottoinsieme 2020-12 con `type`, `properties`, `required`, `items`, `minimum`, `maxLength`.
pub const INPUT_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"id":{"type":"string","maxLength":32},"seq":{"type":"integer","minimum":0},"name":{"type":"string","maxLength":64},"tags":{"type":"array","items":{"type":"string","maxLength":16}},"total":{"type":"number","minimum":0},"n":{"type":"integer","minimum":1},"data":{"type":"string","maxLength":4096},"flag":{"type":"boolean"},"kind":{"type":"string"},"priority":{"type":"integer","minimum":0}},"required":["id","seq","name","tags","total","n","data"]}"#;
/// Schema dell'uscita del passo.
pub const OUTPUT_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"id":{"type":"string","maxLength":32},"seq":{"type":"integer","minimum":0},"flag":{"type":"boolean"},"status":{"type":"string","maxLength":16},"name":{"type":"string","maxLength":64},"tags":{"type":"array","items":{"type":"string","maxLength":16}},"items":{"type":"array","items":{"type":"object","properties":{"sku":{"type":"string"},"qty":{"type":"integer","minimum":1},"price":{"type":"number","minimum":0}},"required":["sku","qty","price"]}},"owner":{"type":"object","properties":{"team":{"type":"string"},"email":{"type":"string","maxLength":64},"level":{"type":"integer","minimum":0}},"required":["team"]},"meta":{"type":"object"},"data":{"type":"string","maxLength":4096}},"required":["id","seq","status","items","data"]}"#;

/// Documento JSON fisso di circa `size` byte (12 campi, un oggetto annidato e due array), uguale nei due bracci:
/// `output` aggiunge `status`; `no_name` toglie `name` (per la prova di schema); `flag` è il valore del campo `flag`.
pub fn doc(seq: u64, size: usize, output: bool, no_name: bool, flag: bool) -> Vec<u8> {
    let items: Vec<String> = (1..=6)
        .map(|i| format!(r#"{{"sku":"sku-{i}","qty":{i},"price":{}}}"#, 10 + i))
        .collect();
    let name = if no_name {
        String::new()
    } else {
        r#""name":"topictoast-load","#.to_string()
    };
    let status = if output { r#""status":"ok","# } else { "" };
    let head = format!(
        r#"{{"id":"ex-{seq}","seq":{seq},"flag":{flag},{name}{status}"kind":"chain","priority":3,"ratio":0.5,"tags":["alfa","beta","gamma"],"owner":{{"team":"t1","email":"ops@example.com","level":2}},"items":[{}],"meta":{{"region":"eu","retries":0}},"data":""#,
        items.join(",")
    );
    let pad = size.saturating_sub(head.len() + 2).max(1);
    format!("{head}{}\"}}", "x".repeat(pad)).into_bytes()
}

// ---------------------------------------------------------------- frame

/// Un frame intero: lunghezza, id, tipo, carico.
pub fn encode(id: u64, ty: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(13 + payload.len());
    v.extend_from_slice(&((9 + payload.len()) as u32).to_le_bytes());
    v.extend_from_slice(&id.to_le_bytes());
    v.push(ty);
    v.extend_from_slice(payload);
    v
}

/// Legge un frame (bloccante); `None` alla fine del flusso. Una lunghezza assurda (oltre 64 MB) è un errore.
pub fn read_frame(r: &mut impl Read) -> std::io::Result<Option<(u64, u8, Vec<u8>)>> {
    let mut l = [0u8; 4];
    if let Err(e) = r.read_exact(&mut l) {
        return if e.kind() == std::io::ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(e)
        };
    }
    let n = u32::from_le_bytes(l) as usize;
    if !(9..=64 << 20).contains(&n) {
        return Err(std::io::Error::other("frame di lunghezza non valida"));
    }
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)?;
    Ok(Some((
        u64::from_le_bytes(b[..8].try_into().unwrap()),
        b[8],
        b[9..].to_vec(),
    )))
}

async fn read_frame_async(
    r: &mut (impl AsyncReadExt + Unpin),
) -> std::io::Result<Option<(u64, u8, Vec<u8>)>> {
    let mut l = [0u8; 4];
    if let Err(e) = r.read_exact(&mut l).await {
        return if e.kind() == std::io::ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(e)
        };
    }
    let n = u32::from_le_bytes(l) as usize;
    if !(9..=64 << 20).contains(&n) {
        return Err(std::io::Error::other("frame di lunghezza non valida"));
    }
    let mut b = vec![0u8; n];
    r.read_exact(&mut b).await?;
    Ok(Some((
        u64::from_le_bytes(b[..8].try_into().unwrap()),
        b[8],
        b[9..].to_vec(),
    )))
}

// ---------------------------------------------------------------- lato ausiliario

type JqFilter = jaq_core::compile::Filter<jaq_core::Native<data::JustLut<Val>>>;

fn compile(code: &str) -> JqFilter {
    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());
    // l'arena vive quanto il processo: un solo `compile` per filtro all'avvio
    let arena: &'static Arena = Box::leak(Box::new(Arena::default()));
    let modules = Loader::new(defs)
        .load(arena, File { code, path: () })
        .expect("filtro jq non valido");
    Compiler::default()
        .with_funs(funs)
        .compile(modules)
        .expect("filtro jq non compilabile")
}

fn run_first(f: &JqFilter, input: Val) -> Result<Option<Val>, String> {
    let ctx = Ctx::<data::JustLut<Val>>::new(&f.lut, Vars::new([]));
    f.id.run((ctx, input))
        .map(unwrap_valr)
        .next()
        .transpose()
        .map_err(|e| format!("jq: {e}"))
}

/// Messaggio che nomina il campo (percorso JSON) e la regola violata.
fn invalid(v: &jsonschema::Validator, doc: &serde_json::Value) -> Option<String> {
    v.validate(doc)
        .err()
        .map(|e| format!("{}: {e}", e.instance_path()))
}

struct Manifest {
    cond: JqFilter,
    map: JqFilter,
    input: jsonschema::Validator,
    output: jsonschema::Validator,
}

impl Manifest {
    fn new() -> Manifest {
        let schema = |s: &str| {
            jsonschema::validator_for(&serde_json::from_str(s).unwrap()).expect("schema non valido")
        };
        Manifest {
            cond: compile(IF_PROG),
            map: compile(MAP_PROG),
            input: schema(INPUT_SCHEMA),
            output: schema(OUTPUT_SCHEMA),
        }
    }

    /// `if` → mappatura → controllo dello schema d'ingresso. Stato e carico della risposta.
    fn prepare(&self, input: &[u8]) -> (u8, Vec<u8>) {
        let val = match read::parse_single(input) {
            Ok(v) => v,
            Err(e) => return (2, format!("ingresso non JSON: {e}").into_bytes()),
        };
        match run_first(&self.cond, val.clone()) {
            Ok(Some(Val::Bool(true))) => {}
            Ok(_) => return (1, vec![]),
            Err(e) => return (2, e.into_bytes()),
        }
        let mapped = match run_first(&self.map, val) {
            Ok(Some(v)) => v.to_string(),
            Ok(None) => return (2, b"mappatura senza risultato".to_vec()),
            Err(e) => return (2, e.into_bytes()),
        };
        match serde_json::from_str::<serde_json::Value>(&mapped) {
            Ok(doc) => match invalid(&self.input, &doc) {
                None => (0, mapped.into_bytes()),
                Some(m) => (2, m.into_bytes()),
            },
            Err(e) => (2, format!("mappatura non JSON: {e}").into_bytes()),
        }
    }

    fn check(&self, output: &[u8]) -> (u8, Vec<u8>) {
        match serde_json::from_slice::<serde_json::Value>(output) {
            Ok(doc) => match invalid(&self.output, &doc) {
                None => (0, vec![]),
                Some(m) => (2, m.into_bytes()),
            },
            Err(e) => (2, format!("uscita non JSON: {e}").into_bytes()),
        }
    }
}

/// Il sottocomando `aux`: legge i frame da stdin, risponde su stdout, in ordine. Esce a fine del flusso (motore morto) o con
/// codice 3 se la versione dell'IPC non è la sua.
pub fn serve() {
    let (mut i, mut o) = (
        std::io::BufReader::new(std::io::stdin().lock()),
        std::io::stdout().lock(),
    );
    match read_frame(&mut i) {
        Ok(Some((id, T_HELLO, p))) if p == IPC_VERSION.to_le_bytes() => {
            o.write_all(&encode(id, T_HELLO, &p)).unwrap();
            o.flush().unwrap();
        }
        _ => {
            eprintln!("AUX_VERSIONE: attesa {IPC_VERSION}");
            std::process::exit(3);
        }
    }
    let m = Manifest::new();
    while let Ok(Some((id, ty, p))) = read_frame(&mut i) {
        let (st, mut body) = match ty {
            T_PREPARE => m.prepare(&p),
            T_CHECK => m.check(&p),
            _ => (2, b"tipo di richiesta sconosciuto".to_vec()),
        };
        body.insert(0, st);
        o.write_all(&encode(id, ty, &body)).unwrap();
        o.flush().unwrap();
    }
}

// ---------------------------------------------------------------- lato motore

/// Esito di `PREPARE`.
pub enum Prepared {
    /// Ingresso mappato e valido, da consegnare al worker.
    Mapped(Vec<u8>),
    /// `if` falso: il passo si salta.
    Skip,
    /// Errore di validazione (o di jq): messaggio con il campo.
    Invalid(String),
    /// L'ausiliario non risponde (caduto): niente si perde, l'attività resta da consegnare.
    Down,
}

struct Req {
    id: u64,
    ty: u8,
    payload: Vec<u8>,
    reply: oneshot::Sender<Vec<u8>>,
}

#[derive(Default)]
pub struct Counters {
    pub prepare: AtomicU64,
    pub check: AtomicU64,
    pub skipped: AtomicU64,
    pub input_invalid: AtomicU64,
    pub output_invalid: AtomicU64,
    pub errors: AtomicU64,
}

pub struct Client {
    tx: mpsc::UnboundedSender<Req>,
    next: AtomicU64,
    pub ctr: Counters,
    pub pid: u32,
    /// Andata e ritorno di una richiesta (µs), misurata nel motore dall'invio alla risposta.
    pub rtt: Arc<Mutex<hdrhistogram::Histogram<u64>>>,
}

impl Client {
    /// Avvia l'ausiliario con l'ambiente vuoto e la stessa versione dell'IPC; panica se non risponde con la stessa versione.
    pub async fn spawn() -> Client {
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .arg("aux")
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("avvio dell'ausiliario");
        let pid = child.id().unwrap_or(0);
        let (mut cin, mut cout) = (child.stdin.take().unwrap(), child.stdout.take().unwrap());
        cin.write_all(&encode(0, T_HELLO, &IPC_VERSION.to_le_bytes()))
            .await
            .unwrap();
        match read_frame_async(&mut cout).await {
            Ok(Some((_, T_HELLO, p))) if p == IPC_VERSION.to_le_bytes() => {}
            r => panic!("l'ausiliario non è compatibile con l'IPC {IPC_VERSION}: {r:?}"),
        }
        let pending: Arc<Mutex<HashMap<u64, (oneshot::Sender<Vec<u8>>, Instant)>>> =
            Default::default();
        let rtt = Arc::new(Mutex::new(crate::hist()));
        let (tx, mut rx) = mpsc::unbounded_channel::<Req>();
        // scrittore: registra la richiesta e scrive il frame
        let p2 = pending.clone();
        tokio::spawn(async move {
            while let Some(r) = rx.recv().await {
                p2.lock().unwrap().insert(r.id, (r.reply, Instant::now()));
                if cin
                    .write_all(&encode(r.id, r.ty, &r.payload))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        // lettore: completa la richiesta con lo stesso id; alla fine del flusso scarta tutte le attese (i chiamanti vedono l'errore)
        let (p3, rtt2) = (pending.clone(), rtt.clone());
        tokio::spawn(async move {
            while let Ok(Some((id, _, body))) = read_frame_async(&mut cout).await {
                if let Some((s, t0)) = p3.lock().unwrap().remove(&id) {
                    rtt2.lock()
                        .unwrap()
                        .record(t0.elapsed().as_micros() as u64 + 1)
                        .ok();
                    let _ = s.send(body);
                }
            }
            eprintln!("AUX_CADUTO {}", crate::now_us());
            p3.lock().unwrap().clear();
        });
        tokio::spawn(async move {
            let st = child.wait().await;
            eprintln!("AUX_EXIT {st:?}");
        });
        Client {
            tx,
            next: 1.into(),
            ctr: Counters::default(),
            pid,
            rtt,
        }
    }

    /// Manda una richiesta senza aspettare: si possono lanciare molte richieste e poi attendere le risposte (pipeline).
    pub fn call(&self, ty: u8, payload: Vec<u8>) -> oneshot::Receiver<Vec<u8>> {
        let (reply, rx) = oneshot::channel();
        let id = self.next.fetch_add(1, Relaxed);
        if self
            .tx
            .send(Req {
                id,
                ty,
                payload,
                reply,
            })
            .is_err()
        {
            self.ctr.errors.fetch_add(1, Relaxed);
        }
        rx
    }

    /// `PREPARE` di un ingresso.
    pub async fn prepare(&self, input: Vec<u8>) -> Prepared {
        self.ctr.prepare.fetch_add(1, Relaxed);
        match self.call(T_PREPARE, input).await {
            Ok(b) if b.first() == Some(&0) => Prepared::Mapped(b[1..].to_vec()),
            Ok(b) if b.first() == Some(&1) => {
                self.ctr.skipped.fetch_add(1, Relaxed);
                Prepared::Skip
            }
            Ok(b) => {
                self.ctr.input_invalid.fetch_add(1, Relaxed);
                Prepared::Invalid(String::from_utf8_lossy(b.get(1..).unwrap_or_default()).into())
            }
            Err(_) => {
                self.ctr.errors.fetch_add(1, Relaxed);
                Prepared::Down
            }
        }
    }

    /// `CHECK` di un'uscita: `None` se valida, altrimenti il messaggio (anche se l'ausiliario non risponde).
    pub async fn check(&self, rx: oneshot::Receiver<Vec<u8>>) -> Option<String> {
        match rx.await {
            Ok(b) if b.first() == Some(&0) => None,
            Ok(b) => {
                self.ctr.output_invalid.fetch_add(1, Relaxed);
                Some(String::from_utf8_lossy(b.get(1..).unwrap_or_default()).into())
            }
            Err(_) => {
                self.ctr.errors.fetch_add(1, Relaxed);
                Some("ausiliario non raggiungibile".into())
            }
        }
    }

    /// Lancia il controllo di un'uscita (conta la chiamata) e torna l'attesa.
    pub fn check_start(&self, output: Vec<u8>) -> oneshot::Receiver<Vec<u8>> {
        self.ctr.check.fetch_add(1, Relaxed);
        self.call(T_CHECK, output)
    }

    /// CPU (secondi) e RSS (kB) del processo ausiliario, da `/proc` (nessuna libreria in più).
    pub fn cpu_rss(&self) -> (f64, u64) {
        let stat = std::fs::read_to_string(format!("/proc/{}/stat", self.pid)).unwrap_or_default();
        // dopo la parentesi chiusa del nome: stato è il campo 3, utime il 14 e stime il 15
        let f: Vec<&str> = stat
            .rsplit(')')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        let tick = |i: usize| f.get(i).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
        let rss = std::fs::read_to_string(format!("/proc/{}/status", self.pid))
            .unwrap_or_default()
            .lines()
            .find_map(|l| {
                l.strip_prefix("VmRSS:")
                    .and_then(|v| v.split_whitespace().next().and_then(|n| n.parse().ok()))
            })
            .unwrap_or(0);
        ((tick(11) + tick(12)) / hz, rss)
    }

    pub fn text(&self) -> String {
        let c = |a: &AtomicU64| a.load(Relaxed);
        let (cpu, rss) = self.cpu_rss();
        format!(
            "aux calls_prepare={} calls_check={} skipped={} input_invalid={} output_invalid={} errors={} cpu_s={cpu:.2} rss_kb={rss}\naux_rtt {}",
            c(&self.ctr.prepare),
            c(&self.ctr.check),
            c(&self.ctr.skipped),
            c(&self.ctr.input_invalid),
            c(&self.ctr.output_invalid),
            c(&self.ctr.errors),
            crate::fmt(&self.rtt.lock().unwrap())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trip() {
        let f = encode(42, T_CHECK, b"{\"a\":1}");
        let (id, ty, p) = read_frame(&mut &f[..]).unwrap().unwrap();
        assert_eq!(
            (id, ty, p.as_slice()),
            (42, T_CHECK, b"{\"a\":1}".as_slice())
        );
        assert!(
            read_frame(&mut &[][..]).unwrap().is_none(),
            "fine del flusso"
        );
        assert!(
            read_frame(&mut &[1u8, 0, 0, 0, 0][..]).is_err(),
            "lunghezza non valida"
        );
    }

    /// Il documento fisso passa `if`, mappatura e schemi; senza `name` l'errore nomina il campo; con `flag` falso il passo si salta.
    #[test]
    fn manifest_ok_skip_and_named_error() {
        let m = Manifest::new();
        let (st, mapped) = m.prepare(&doc(7, 1024, false, false, true));
        assert_eq!(st, 0, "{}", String::from_utf8_lossy(&mapped));
        let v: serde_json::Value = serde_json::from_slice(&mapped).unwrap();
        assert_eq!((v["seq"].as_u64(), v["n"].as_u64()), (Some(8), Some(6)));
        assert_eq!(
            v["total"].as_f64(),
            Some((1..=6).map(|i| (i * (10 + i)) as f64).sum())
        );
        assert_eq!(v["tags"][0], "ALFA");
        let (st, msg) = m.prepare(&doc(7, 1024, false, true, true));
        assert_eq!(st, 2);
        assert!(
            String::from_utf8_lossy(&msg).contains("/name"),
            "{}",
            String::from_utf8_lossy(&msg)
        );
        assert_eq!(m.prepare(&doc(7, 1024, false, false, false)).0, 1);
        assert_eq!(m.check(&doc(9, 1024, true, false, true)).0, 0);
        assert_eq!(
            m.check(&doc(9, 1024, false, false, true)).0,
            2,
            "manca `status`"
        );
        let n = doc(1, 1024, false, false, true).len();
        assert!((1000..=1050).contains(&n), "{n}");
    }
}
