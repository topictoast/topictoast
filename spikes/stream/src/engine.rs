//! Ruolo `engine`: API di FR-18 a blocchi, avvii con idempotenza (C-4) in SQLite, segnali durevoli degli
//! stream fermi (PIANO 1.9) e supervisore dei processi per tenant (PIANO 1.12).
//!
//! Rotte (tutte con credenziale `Authorization: Bearer`, nessuna rotta senza, C-14):
//! - `POST /v1/streams/{stream}/blocks`: token di ingresso del tenant, o token di servizio per gli stream di destinazione;
//!   header `Idempotency-Key`;
//! - `POST /v1/starts`: token di servizio del tenant (lo conosce solo il suo processo `stream`);
//! - `GET /v1/stats`: token di amministrazione;
//! - rotte di amministrazione (token di amministrazione), parte 2a: `POST /v1/admin/{tenant}/pipelines/{id}/suspend|resume`,
//!   `PUT /v1/admin/{tenant}/pipelines/{id}` (nuova versione), `POST /v1/admin/{tenant}/validate` (validazione a secco),
//!   `POST /v1/admin/refuse-starts?seconds=N` (C6).
//!
//! Il processo per tenant parte con una coppia di socket ereditata (mai un socket in ascolto) e riceve
//! solo la chiave del suo tenant e la credenziale di sola lettura del bus. Sulla stessa coppia il motore manda i
//! comandi di ciclo di vita e riceve le risposte e le statistiche.

use crate::bus::Bus;
use crate::crypto::{Master, TenantKey, ct_eq, hex, new_id, tenant_uid};
use crate::http::{Req, Resp, serve};
use crate::spec::*;
use crate::wire;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

/// Primo messaggio sulla coppia di socket: tutto quello che il processo per tenant puo' sapere.
#[derive(Clone, Serialize, Deserialize)]
pub struct Hello {
    pub tenant: TenantDef,
    pub key_hex: String,
    pub iggy_addr: String,
    pub iggy_user: String,
    pub iggy_pass: String,
    pub api_addr: String,
    pub api_token: String,
    pub ckpt_dir: String,
    pub state_quota: usize,
    /// quota di stato per pipeline contata da topictoast (V3); spenta solo nel controllo con il solo pool di DataFusion
    #[serde(default)]
    pub quota_enabled: bool,
    /// limite del sistema (`RLIMIT_DATA`) sul processo, in MiB; 0 = nessun limite (solo controllo di V3)
    #[serde(default)]
    pub rlimit_data_mb: u64,
    /// pipeline sospese dall'operatore
    #[serde(default)]
    pub suspended: Vec<String>,
    /// solo controlli positivi (file `-ctrl-`): barriera 1 spenta nel processo per tenant
    #[serde(default)]
    pub ctrl_off_syntax: bool,
}

struct TenantRt {
    def: TenantDef,
    /// pipeline correnti (le versioni cambiate dall'API). ponytail: in memoria; il prodotto le tiene in database
    pipelines: Mutex<Vec<PipelineDef>>,
    suspended: Mutex<HashSet<String>>,
    uid: [u8; 16],
    key: TenantKey,
    ingest_token: String,
    svc_token: String,
    /// blocchi arrivati nell'ultimo secondo, per stream
    ticks: HashMap<String, AtomicU64>,
    /// comandi verso il processo per tenant
    ctl: mpsc::UnboundedSender<String>,
}

struct Eng {
    tenants: Vec<TenantRt>,
    bus: Bus,
    db: SqlitePool,
    admin_token: String,
    stats: Mutex<HashMap<String, Value>>,
    /// richieste in attesa di risposta dal processo per tenant: id -> (indice tenant, canale)
    replies: Mutex<HashMap<u64, (usize, oneshot::Sender<Value>)>>,
    req_seq: AtomicU64,
    /// rifiuto degli avvii per quota fino a questo istante (C6)
    refuse_until: AtomicI64,
    refused_starts: AtomicU64,
}

fn j(code: u16, v: Value) -> Resp {
    Resp::json(code, v)
}

impl Eng {
    /// Manda un comando al processo del tenant `ti` e attende la risposta; se il processo muore prima, errore con codice.
    async fn ask(&self, ti: usize, mut cmd: Value, wait_s: u64) -> Value {
        let id = self.req_seq.fetch_add(1, Ordering::Relaxed);
        cmd["req"] = json!(id);
        let (tx, rx) = oneshot::channel();
        self.replies.lock().unwrap().insert(id, (ti, tx));
        let _ = self.tenants[ti].ctl.send(cmd.to_string());
        match tokio::time::timeout(std::time::Duration::from_secs(wait_s), rx).await {
            Ok(Ok(v)) => v,
            _ => {
                self.replies.lock().unwrap().remove(&id);
                json!({"ok": false, "error": {"code": "timeout", "message": "nessuna risposta dal processo per tenant"}})
            }
        }
    }
}

async fn admin(e: &Eng, r: &Req) -> Resp {
    let path: Vec<&str> = r
        .path
        .split('?')
        .next()
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let q = |k: &str| {
        r.path.split('?').nth(1).and_then(|s| {
            s.split('&')
                .find_map(|kv| kv.strip_prefix(&format!("{k}=")))
                .map(str::to_string)
        })
    };
    let tenant = |n: &str| e.tenants.iter().position(|t| t.def.id == n);
    match (r.method.as_str(), path.as_slice()) {
        ("POST", ["v1", "admin", "refuse-starts"]) => {
            let s: i64 = q("seconds").and_then(|v| v.parse().ok()).unwrap_or(20);
            e.refuse_until.store(now_ms() + s * 1000, Ordering::Relaxed);
            j(200, json!({"refusing_for_s": s}))
        }
        (
            "POST",
            [
                "v1",
                "admin",
                t,
                "pipelines",
                id,
                act @ ("suspend" | "resume"),
            ],
        ) => {
            let Some(ti) = tenant(t) else {
                return j(404, json!({"error":"unknown_tenant"}));
            };
            if *act == "suspend" {
                e.tenants[ti]
                    .suspended
                    .lock()
                    .unwrap()
                    .insert(id.to_string());
            }
            let v = e.ask(ti, json!({"cmd": act, "id": id}), 30).await;
            if *act == "resume" && v["ok"] == true {
                e.tenants[ti].suspended.lock().unwrap().remove(*id);
            }
            j(if v["ok"] == true { 200 } else { 409 }, v)
        }
        ("PUT", ["v1", "admin", t, "pipelines", id]) => {
            let Some(ti) = tenant(t) else {
                return j(404, json!({"error":"unknown_tenant"}));
            };
            let Ok(def) = serde_json::from_slice::<PipelineDef>(&r.body) else {
                return j(400, json!({"error":"bad_request"}));
            };
            if def.id != *id {
                return j(400, json!({"error":"bad_request","message":"id diverso"}));
            }
            let v = e
                .ask(ti, json!({"cmd": "set_pipeline", "def": def}), 60)
                .await;
            if v["ok"] == true {
                let mut ps = e.tenants[ti].pipelines.lock().unwrap();
                ps.retain(|p| p.id != def.id);
                ps.push(def);
            }
            j(if v["ok"] == true { 200 } else { 422 }, v)
        }
        ("POST", ["v1", "admin", t, "validate"]) => {
            let Some(ti) = tenant(t) else {
                return j(404, json!({"error":"unknown_tenant"}));
            };
            let Ok(def) = serde_json::from_slice::<PipelineDef>(&r.body) else {
                return j(400, json!({"error":"bad_request"}));
            };
            let v = e.ask(ti, json!({"cmd": "validate", "def": def}), 120).await;
            j(if v["ok"] == true { 200 } else { 422 }, v)
        }
        _ => j(404, json!({"error":"not_found"})),
    }
}

async fn handle(e: Arc<Eng>, r: Req) -> Resp {
    let Some(tok) = r.bearer() else {
        return j(401, json!({"error":"unauthorized"}));
    };
    let key = r.header("idempotency-key").unwrap_or("").to_string();
    match (r.method.as_str(), r.path.as_str()) {
        ("POST", p) if p.starts_with("/v1/streams/") && p.ends_with("/blocks") => {
            let name = &p["/v1/streams/".len()..p.len() - "/blocks".len()];
            // token di ingresso del tenant, oppure token di servizio ma solo per gli stream di destinazione delle sue pipeline
            let Some(t) = e.tenants.iter().find(|t| {
                ct_eq(&t.ingest_token, tok)
                    || (ct_eq(&t.svc_token, tok)
                        && t.pipelines
                            .lock()
                            .unwrap()
                            .iter()
                            .any(|p| p.dest_stream.as_deref() == Some(name)))
            }) else {
                return j(401, json!({"error":"unauthorized"}));
            };
            let Some(sd) = t.def.streams.iter().find(|s| s.name == name) else {
                return j(404, json!({"error":"unknown_stream"}));
            };
            if key.is_empty()
                || key.len() > 128
                || !key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.:|".contains(c))
            {
                return j(400, json!({"error":"idempotency_key_required"}));
            }
            let arrival = now_ms();
            let batch = match wire::ingest(sd, &r.body, arrival) {
                Ok(b) => b,
                Err(x) => {
                    return j(
                        400,
                        json!({"error":"invalid_event","index":x.index,"field":x.field,"message":x.msg}),
                    );
                }
            };
            let topic = topic_of(&t.def.id, name);
            let frame = wire::encode_block(arrival, &key, &batch);
            let sealed = t.key.seal(&t.uid, &topic, &new_id(), &frame);
            if e.bus.append(&topic, Bytes::from(sealed)).await.is_err() {
                return j(503, json!({"error":"bus_unavailable"}));
            }
            t.ticks[name].fetch_add(1, Ordering::Relaxed);
            // 200 solo dopo la scrittura durevole: «accettato» vuol dire questo
            j(200, json!({"arrival_ms":arrival,"events":batch.num_rows()}))
        }
        ("POST", "/v1/starts") => {
            if !e.tenants.iter().any(|t| ct_eq(&t.svc_token, tok)) {
                return j(401, json!({"error":"unauthorized"}));
            }
            // rifiuto per quota del motore (C6): la riga resta nel processo per tenant, che riprova
            if now_ms() < e.refuse_until.load(Ordering::Relaxed) {
                e.refused_starts.fetch_add(1, Ordering::Relaxed);
                return j(429, json!({"error":"quota_exceeded"}));
            }
            if key.is_empty() {
                return j(400, json!({"error":"idempotency_key_required"}));
            }
            let pipeline = key.split(':').next().unwrap_or("").to_string();
            let body = String::from_utf8_lossy(&r.body).to_string();
            // ponytail: il corpo e' testo JSON; il prodotto avrebbe colonne tipizzate (costituzione V)
            let mut tx = match e.db.begin().await {
                Ok(t) => t,
                Err(_) => return j(503, json!({"error":"db"})),
            };
            let ins = sqlx::query(
                "INSERT OR IGNORE INTO starts(key, pipeline, body, accepted_ms) VALUES (?,?,?,?)",
            )
            .bind(&key)
            .bind(&pipeline)
            .bind(&body)
            .bind(now_ms())
            .execute(&mut *tx)
            .await;
            let Ok(ins) = ins else {
                return j(503, json!({"error":"db"}));
            };
            let dup = ins.rows_affected() == 0;
            if dup {
                let _ = sqlx::query("UPDATE meta SET v = v + 1 WHERE k = 'dupes'")
                    .execute(&mut *tx)
                    .await;
            }
            // commit con synchronous=FULL: durevole prima della risposta
            if tx.commit().await.is_err() {
                return j(503, json!({"error":"db"}));
            }
            j(200, json!({"duplicate":dup}))
        }
        ("GET", "/v1/stats") => {
            if !ct_eq(tok, &e.admin_token) {
                return j(401, json!({"error":"unauthorized"}));
            }
            let starts: i64 = sqlx::query("SELECT count(*) FROM starts")
                .fetch_one(&e.db)
                .await
                .map(|r| r.get(0))
                .unwrap_or(-1);
            let dupes: i64 = sqlx::query("SELECT v FROM meta WHERE k='dupes'")
                .fetch_one(&e.db)
                .await
                .map(|r| r.get(0))
                .unwrap_or(-1);
            let st = e.stats.lock().unwrap().clone();
            j(
                200,
                json!({"tenants":st,"starts":starts,"dupes":dupes,"refused_starts":e.refused_starts.load(Ordering::Relaxed)}),
            )
        }
        (_, p) if p.starts_with("/v1/admin/") => {
            if !ct_eq(tok, &e.admin_token) {
                return j(401, json!({"error":"unauthorized"}));
            }
            admin(&e, &r).await
        }
        _ => j(404, json!({"error":"not_found"})),
    }
}

/// Supervisore di un tenant: un processo `stream`, riavvitato quando muore.
async fn supervise(
    e: Arc<Eng>,
    idx: usize,
    base: Hello,
    pid_dir: String,
    mut ctl: mpsc::UnboundedReceiver<String>,
) {
    let t = &e.tenants[idx];
    loop {
        let (parent, child) = std::os::unix::net::UnixStream::pair().unwrap();
        let cfd = child.as_raw_fd();
        let mut cmd = tokio::process::Command::new(std::env::current_exe().unwrap());
        cmd.arg("stream").env_clear();
        // SAFETY: solo chiamate async-signal-safe tra fork ed exec
        unsafe {
            cmd.pre_exec(move || {
                if libc::dup2(cfd, 3) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                // muore con il motore: nessun processo per tenant orfano con la chiave in memoria
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
        let mut ch = match cmd.spawn() {
            Ok(c) => c,
            Err(err) => {
                eprintln!("engine: spawn stream {} fallito: {err}", t.def.id);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };
        drop(child);
        let _ = std::fs::write(
            format!("{pid_dir}/stream-{}.pid", t.def.id),
            ch.id().unwrap_or(0).to_string(),
        );
        parent.set_nonblocking(true).unwrap();
        let sock = tokio::net::UnixStream::from_std(parent).unwrap();
        let (rd, mut wr) = sock.into_split();
        // le pipeline e le sospensioni correnti, non quelle della configurazione iniziale
        let mut hello = base.clone();
        hello.tenant.pipelines = t.pipelines.lock().unwrap().clone();
        hello.suspended = t.suspended.lock().unwrap().iter().cloned().collect();
        let _ = wr
            .write_all(format!("{}\n", serde_json::to_string(&hello).unwrap()).as_bytes())
            .await;
        let mut lines = BufReader::new(rd).lines();
        let status = loop {
            tokio::select! {
                l = lines.next_line() => match l {
                    Ok(Some(l)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&l) {
                            if let Some(id) = v.get("reply").and_then(|x| x.as_u64()) {
                                if let Some((_, tx)) = e.replies.lock().unwrap().remove(&id) {
                                    let _ = tx.send(v);
                                }
                            } else {
                                e.stats.lock().unwrap().insert(t.def.id.clone(), v);
                            }
                        }
                    }
                    _ => break ch.wait().await.ok(),
                },
                c = ctl.recv() => if let Some(c) = c { let _ = wr.write_all(format!("{c}\n").as_bytes()).await; },
                s = ch.wait() => break s.ok(),
            }
        };
        // le richieste senza risposta (es. una validazione a secco che ha fatto morire il processo) ricevono un errore con codice
        let sig = status.as_ref().and_then(|s| s.signal());
        let code = status.as_ref().and_then(|s| s.code());
        let dead: Vec<oneshot::Sender<Value>> = {
            let mut m = e.replies.lock().unwrap();
            let ids: Vec<u64> = m
                .iter()
                .filter(|(_, (ti, _))| *ti == idx)
                .map(|(k, _)| *k)
                .collect();
            ids.into_iter()
                .filter_map(|k| m.remove(&k).map(|(_, tx)| tx))
                .collect()
        };
        for tx in dead {
            let _ = tx.send(json!({"ok": false, "error": {"code": "validation_process_died", "message": format!("il processo per tenant e' terminato (segnale {sig:?}, codice {code:?})")}}));
        }
        eprintln!(
            "engine: stream {} terminato (segnale {sig:?}, codice {code:?}), riavvio",
            t.def.id
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

pub async fn run() {
    let spec = Spec::load(&arg("--spec").expect("--spec"));
    let listen = arg("--listen").unwrap_or("127.0.0.1:47111".into());
    let data = arg("--data").expect("--data");
    let iggy_addr = arg("--iggy").unwrap_or("127.0.0.1:47101".into());
    // controllo di V3: `--no-quota` spegne la quota di stato, `--rlimit-data-mb 0` il limite del sistema
    let quota_enabled = !std::env::args().any(|a| a == "--no-quota");
    let rlimit_data_mb: u64 = arg("--rlimit-data-mb")
        .map(|v| v.parse().unwrap())
        .unwrap_or(1024);
    let ctrl_off_syntax = std::env::args().any(|a| a == "--ctrl-off-syntax");
    if ctrl_off_syntax && !arg("--label").unwrap_or_default().contains("-ctrl-") {
        eprintln!(
            "rifiuto: --ctrl-off-syntax solo con --label di un file di controllo positivo (nome con `-ctrl-`)"
        );
        std::process::exit(2);
    }
    let state_quota: usize = arg("--state-quota-mb")
        .map(|v| v.parse::<usize>().unwrap())
        .unwrap_or(64)
        << 20;
    for d in ["payload", "pids"] {
        std::fs::create_dir_all(format!("{data}/{d}")).unwrap();
    }
    let master = Master::load(&format!("{data}/master.key"));
    let bus = Bus::connect(
        &iggy_addr,
        &env("IGGY_ROOT_USERNAME"),
        &env("IGGY_ROOT_PASSWORD"),
    )
    .await
    .expect("bus");
    bus.ensure_stream().await.unwrap();
    // database degli avvii: synchronous=FULL, chiave unica
    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(format!("{data}/starts.db"))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full),
    )
    .await
    .unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS starts(key TEXT PRIMARY KEY, pipeline TEXT NOT NULL, body TEXT NOT NULL, accepted_ms INTEGER NOT NULL) WITHOUT ROWID")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v INTEGER NOT NULL)")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT OR IGNORE INTO meta VALUES('dupes', 0)")
        .execute(&db)
        .await
        .unwrap();

    let mut tenants = vec![];
    let mut hellos = vec![];
    let mut ctls = vec![];
    for t in &spec.tenants {
        let uid = tenant_uid(&t.id);
        let key = master.tenant_key(&uid);
        let topics: Vec<String> = t.streams.iter().map(|s| topic_of(&t.id, &s.name)).collect();
        for tp in &topics {
            bus.ensure_topic(tp).await.unwrap();
        }
        let up = bus
            .ro_user(&format!("ro-{}", t.id), &topics)
            .await
            .expect("utente di sola lettura");
        let svc = hex(&new_id());
        let ckpt = format!("{data}/payload/{}", t.id);
        std::fs::create_dir_all(&ckpt).unwrap();
        hellos.push(Hello {
            tenant: t.clone(),
            key_hex: hex(&key.0),
            iggy_addr: iggy_addr.clone(),
            iggy_user: format!("ro-{}", t.id),
            iggy_pass: up,
            api_addr: listen.clone(),
            api_token: svc.clone(),
            ckpt_dir: ckpt,
            state_quota,
            quota_enabled,
            rlimit_data_mb,
            suspended: vec![],
            ctrl_off_syntax,
        });
        let (tx, rx) = mpsc::unbounded_channel();
        ctls.push(rx);
        tenants.push(TenantRt {
            def: t.clone(),
            pipelines: Mutex::new(t.pipelines.clone()),
            suspended: Mutex::new(HashSet::new()),
            uid,
            key,
            ingest_token: env(&format!("TT_TOKEN_{}", t.id.to_ascii_uppercase())),
            svc_token: svc,
            ticks: t
                .streams
                .iter()
                .map(|s| (s.name.clone(), AtomicU64::new(0)))
                .collect(),
            ctl: tx,
        });
    }
    let e = Arc::new(Eng {
        tenants,
        bus,
        db,
        admin_token: env("TT_ADMIN_TOKEN"),
        stats: Mutex::new(HashMap::new()),
        replies: Mutex::new(HashMap::new()),
        req_seq: AtomicU64::new(1),
        refuse_until: AtomicI64::new(0),
        refused_starts: AtomicU64::new(0),
    });
    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .expect("porta occupata");
    println!("engine pronto su {listen}");
    // segnali di stream fermo: ogni secondo, uno per stream senza blocchi (PIANO 1.9)
    {
        let e = e.clone();
        tokio::spawn(async move {
            let mut iv = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                iv.tick().await;
                for t in &e.tenants {
                    for s in &t.def.streams {
                        if t.ticks[&s.name].swap(0, Ordering::Relaxed) == 0 {
                            let topic = topic_of(&t.def.id, &s.name);
                            let sealed = t.key.seal(
                                &t.uid,
                                &topic,
                                &new_id(),
                                &wire::encode_signal(now_ms()),
                            );
                            let _ = e.bus.append(&topic, Bytes::from(sealed)).await;
                        }
                    }
                }
            }
        });
    }
    for ((i, h), rx) in hellos.into_iter().enumerate().zip(ctls) {
        tokio::spawn(supervise(e.clone(), i, h, format!("{data}/pids"), rx));
    }
    let e2 = e.clone();
    serve(listener, move |r| handle(e2.clone(), r)).await;
}
