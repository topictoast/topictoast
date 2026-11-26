//! Ruolo `engine`: API di FR-18 a blocchi, avvii con idempotenza (C-4) in SQLite, segnali durevoli degli
//! stream fermi (PIANO 1.9) e supervisore dei processi per tenant (PIANO 1.12).
//!
//! Rotte (tutte con credenziale `Authorization: Bearer`, nessuna rotta senza, C-14):
//! - `POST /v1/streams/{stream}/blocks`: token di ingresso del tenant; header `Idempotency-Key`;
//! - `POST /v1/starts`: token di servizio del tenant (lo conosce solo il suo processo `stream`);
//! - `GET /v1/stats`: token di amministrazione.
//!
//! Il processo per tenant parte con una coppia di socket ereditata (mai un socket in ascolto) e riceve
//! solo la chiave del suo tenant e la credenziale di sola lettura del bus.

use crate::bus::Bus;
use crate::crypto::{Master, TenantKey, hex, new_id, tenant_uid};
use crate::http::{Req, Resp, serve};
use crate::spec::*;
use crate::wire;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Primo messaggio sulla coppia di socket: tutto quello che il processo per tenant puo' sapere.
#[derive(Serialize, Deserialize)]
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
}

struct TenantRt {
    def: TenantDef,
    uid: [u8; 16],
    key: TenantKey,
    ingest_token: String,
    svc_token: String,
    /// blocchi arrivati nell'ultimo secondo, per stream
    ticks: HashMap<String, AtomicU64>,
}

struct Eng {
    tenants: Vec<TenantRt>,
    bus: Bus,
    db: SqlitePool,
    admin_token: String,
    stats: Mutex<HashMap<String, Value>>,
}

fn j(code: u16, v: Value) -> Resp {
    Resp::json(code, v)
}

async fn handle(e: Arc<Eng>, r: Req) -> Resp {
    let Some(tok) = r.bearer() else {
        return j(401, json!({"error":"unauthorized"}));
    };
    let key = r.header("idempotency-key").unwrap_or("").to_string();
    match (r.method.as_str(), r.path.as_str()) {
        ("POST", p) if p.starts_with("/v1/streams/") && p.ends_with("/blocks") => {
            let Some(t) = e.tenants.iter().find(|t| t.ingest_token == tok) else {
                return j(401, json!({"error":"unauthorized"}));
            };
            let name = &p["/v1/streams/".len()..p.len() - "/blocks".len()];
            let Some(sd) = t.def.streams.iter().find(|s| s.name == name) else {
                return j(404, json!({"error":"unknown_stream"}));
            };
            if key.is_empty() || key.len() > 128 || !key.chars().all(|c| c.is_ascii_alphanumeric() || "-_.:".contains(c)) {
                return j(400, json!({"error":"idempotency_key_required"}));
            }
            let arrival = now_ms();
            let batch = match wire::ingest(sd, &r.body, arrival) {
                Ok(b) => b,
                Err(x) => {
                    return j(400, json!({"error":"invalid_event","index":x.index,"field":x.field,"message":x.msg}));
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
            if !e.tenants.iter().any(|t| t.svc_token == tok) {
                return j(401, json!({"error":"unauthorized"}));
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
            let ins = sqlx::query("INSERT OR IGNORE INTO starts(key, pipeline, body, accepted_ms) VALUES (?,?,?,?)")
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
                let _ = sqlx::query("UPDATE meta SET v = v + 1 WHERE k = 'dupes'").execute(&mut *tx).await;
            }
            // commit con synchronous=FULL: durevole prima della risposta
            if tx.commit().await.is_err() {
                return j(503, json!({"error":"db"}));
            }
            j(200, json!({"duplicate":dup}))
        }
        ("GET", "/v1/stats") => {
            if tok != e.admin_token {
                return j(401, json!({"error":"unauthorized"}));
            }
            let starts: i64 = sqlx::query("SELECT count(*) FROM starts").fetch_one(&e.db).await.map(|r| r.get(0)).unwrap_or(-1);
            let dupes: i64 = sqlx::query("SELECT v FROM meta WHERE k='dupes'").fetch_one(&e.db).await.map(|r| r.get(0)).unwrap_or(-1);
            let st = e.stats.lock().unwrap().clone();
            j(200, json!({"tenants":st,"starts":starts,"dupes":dupes}))
        }
        _ => j(404, json!({"error":"not_found"})),
    }
}

/// Supervisore di un tenant: un processo `stream`, riavviato quando muore.
async fn supervise(e: Arc<Eng>, idx: usize, hello: Hello, pid_dir: String) {
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
        let _ = std::fs::write(format!("{pid_dir}/stream-{}.pid", t.def.id), ch.id().unwrap_or(0).to_string());
        parent.set_nonblocking(true).unwrap();
        let sock = tokio::net::UnixStream::from_std(parent).unwrap();
        let (rd, mut wr) = sock.into_split();
        let _ = wr.write_all(format!("{}\n", serde_json::to_string(&hello).unwrap()).as_bytes()).await;
        let mut lines = BufReader::new(rd).lines();
        loop {
            tokio::select! {
                l = lines.next_line() => match l {
                    Ok(Some(l)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&l) {
                            e.stats.lock().unwrap().insert(t.def.id.clone(), v);
                        }
                    }
                    _ => { let _ = ch.wait().await; break; }
                },
                _ = ch.wait() => break,
            }
        }
        eprintln!("engine: stream {} terminato, riavvio", t.def.id);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

pub async fn run() {
    let spec = Spec::load(&arg("--spec").expect("--spec"));
    let listen = arg("--listen").unwrap_or("127.0.0.1:47111".into());
    let data = arg("--data").expect("--data");
    let iggy_addr = arg("--iggy").unwrap_or("127.0.0.1:47101".into());
    for d in ["payload", "pids"] {
        std::fs::create_dir_all(format!("{data}/{d}")).unwrap();
    }
    let master = Master::load(&format!("{data}/master.key"));
    let bus = Bus::connect(&iggy_addr, &env("IGGY_ROOT_USERNAME"), &env("IGGY_ROOT_PASSWORD")).await.expect("bus");
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
    sqlx::query("CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v INTEGER NOT NULL)").execute(&db).await.unwrap();
    sqlx::query("INSERT OR IGNORE INTO meta VALUES('dupes', 0)").execute(&db).await.unwrap();

    let mut tenants = vec![];
    let mut hellos = vec![];
    for t in &spec.tenants {
        let uid = tenant_uid(&t.id);
        let key = master.tenant_key(&uid);
        let topics: Vec<String> = t.streams.iter().map(|s| topic_of(&t.id, &s.name)).collect();
        for tp in &topics {
            bus.ensure_topic(tp).await.unwrap();
        }
        let up = bus.ro_user(&format!("ro-{}", t.id), &topics).await.expect("utente di sola lettura");
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
            state_quota: 64 << 20,
        });
        tenants.push(TenantRt {
            def: t.clone(),
            uid,
            key,
            ingest_token: env(&format!("TT_TOKEN_{}", t.id.to_ascii_uppercase())),
            svc_token: svc,
            ticks: t.streams.iter().map(|s| (s.name.clone(), AtomicU64::new(0))).collect(),
        });
    }
    let e = Arc::new(Eng {
        tenants,
        bus,
        db,
        admin_token: env("TT_ADMIN_TOKEN"),
        stats: Mutex::new(HashMap::new()),
    });
    let listener = tokio::net::TcpListener::bind(&listen).await.expect("porta occupata");
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
                            let sealed = t.key.seal(&t.uid, &topic, &new_id(), &wire::encode_signal(now_ms()));
                            let _ = e.bus.append(&topic, Bytes::from(sealed)).await;
                        }
                    }
                }
            }
        });
    }
    for (i, h) in hellos.into_iter().enumerate() {
        tokio::spawn(supervise(e.clone(), i, h, format!("{data}/pids")));
    }
    let e2 = e.clone();
    serve(listener, move |r| handle(e2.clone(), r)).await;
}
