//! Generatore di carico a ritmo fisso (open-loop: la latenza parte dall'istante previsto, non da
//! quello d'invio, per non nascondere le code), registro delle accettazioni e verifiche finali.

use crate::engine::{TENANTS, topic_of};
use crate::{arg, fmt, hist, now_us, pb, worker};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// CPU cumulata del worker, scritta dal worker su `out/worker.cpu`.
fn worker_cpu(dir: &str) -> f64 {
    std::fs::read_to_string(format!("{dir}/worker.cpu"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0.0)
}

pub async fn stats(url: &str, dir: &str, reset: bool) -> String {
    let mut c = pb::control_client::ControlClient::new(worker::channel(url, dir));
    c.stats(pb::StatsReq { reset })
        .await
        .map(|r| r.into_inner().text)
        .unwrap_or_else(|e| format!("stats non disponibili: {e}"))
}

pub async fn load(url: &str, dir: &str) {
    let rate: f64 = arg("rate", "100").parse().unwrap();
    let kind: u32 = arg("kind", "0").parse().unwrap();
    let n: u32 = arg("n", "1").parse().unwrap();
    let size: usize = arg("size", "1024").parse().unwrap();
    let work_ms: u32 = arg("work_ms", "0").parse().unwrap();
    let retry_ms: u32 = arg("retry_ms", "0").parse().unwrap();
    let secs: f64 = arg("secs", "10").parse().unwrap();
    let warm: f64 = arg("warm", "2").parse().unwrap();
    let tenants = arg("tenants", "a");
    let ledger_path = arg("ledger", "");
    let do_stats = arg("stats", "1") == "1";
    let max_out: usize = arg("max_out", "20000").parse().unwrap();
    let chans: Vec<_> = (0..4)
        .map(|_| {
            pb::control_client::ControlClient::new(worker::channel(url, dir).clone())
                .max_encoding_message_size(64 << 20)
        })
        .collect();
    let h = Arc::new(Mutex::new(hist()));
    let ledger = Arc::new(Mutex::new(Vec::<[u8; 16]>::new()));
    let (offered, ok, refused, errs, overload) = (
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicU64::new(0)),
    );
    let sem = Arc::new(tokio::sync::Semaphore::new(max_out));
    // tipo di ogni risposta non riuscita: codice gRPC e messaggio (blocco B: nessun errore senza tipo)
    let err_types = Arc::new(Mutex::new(std::collections::BTreeMap::<String, u64>::new()));
    // pausa vista dal client: il più lungo intervallo senza accettazioni riuscite (gap, inizio, fine in microsecondi)
    let (last_ok, gap) = (
        Arc::new(AtomicU64::new(0)),
        Arc::new(Mutex::new((0u64, 0u64, 0u64))),
    );
    let start = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_millis(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);
    let mut acc = 0.0;
    let mut i = 0usize;
    let mut reset_done = false;
    let mut w0 = 0.0;
    let mut tasks = tokio::task::JoinSet::new();
    let mut input = vec![0u8; size];
    let json = arg("payload", "bytes") == "json";
    let inbad: usize = arg("inbad", "0").parse().unwrap();
    let inflag = arg("inflag", "1") != "0";
    // intervallo di misura = [warm, warm+secs]; l'ultimo istante di invio è warm+secs
    while start.elapsed().as_secs_f64() < warm + secs {
        tick.tick().await;
        let el = start.elapsed().as_secs_f64();
        if !reset_done && el >= warm {
            reset_done = true;
            if do_stats {
                stats(url, dir, true).await;
                w0 = worker_cpu(dir);
            }
        }
        acc += rate / 1000.0;
        while acc >= 1.0 {
            acc -= 1.0;
            i += 1;
            let tenant = match tenants.as_str() {
                "a" => 0,
                "b" => 1,
                _ => i % 2,
            };
            if json {
                // blocco E2: documento JSON fisso di circa `size` byte (uguale nei due bracci); `inbad=N`: ogni N-esimo senza `name`; `inflag=0`: `flag` falso
                input = crate::auxp::doc(
                    i as u64,
                    size,
                    false,
                    inbad > 0 && i.is_multiple_of(inbad),
                    inflag,
                );
            } else {
                input[..8].copy_from_slice(&(i as u64).to_le_bytes());
            }
            let req = pb::AcceptReq {
                tenant: TENANTS[tenant].to_vec(),
                kind,
                n,
                work_ms,
                out_size: size as u32,
                input: input.clone(),
                retry_ms,
            };
            let Ok(permit) = sem.clone().try_acquire_owned() else {
                overload.fetch_add(1, Relaxed);
                continue;
            };
            offered.fetch_add(1, Relaxed);
            let intended = Instant::now();
            let measured = reset_done;
            let mut c = chans[i % chans.len()].clone();
            let (h, ledger, ok, refused, errs, last_ok, gap, err_types) = (
                h.clone(),
                ledger.clone(),
                ok.clone(),
                refused.clone(),
                errs.clone(),
                last_ok.clone(),
                gap.clone(),
                err_types.clone(),
            );
            tasks.spawn(async move {
                match c.accept(req).await {
                    Ok(r) => {
                        if measured {
                            h.lock()
                                .unwrap()
                                .record(intended.elapsed().as_micros() as u64 + 1)
                                .ok();
                        }
                        ok.fetch_add(1, Relaxed);
                        let now = now_us();
                        let prev = last_ok.swap(now, Relaxed);
                        if prev != 0 && now.saturating_sub(prev) > 50_000 {
                            let mut g = gap.lock().unwrap();
                            if now.saturating_sub(prev) > g.0 {
                                *g = (now - prev, prev, now);
                            }
                        }
                        ledger
                            .lock()
                            .unwrap()
                            .push(r.into_inner().exec.try_into().unwrap());
                    }
                    Err(e)
                        if matches!(
                            e.message(),
                            "E_PROJECTION_DOWN"
                                | "E_UPGRADE_DRAIN"
                                | "E_BUS_DOWN"
                                | "E_STORAGE_FULL"
                        ) =>
                    {
                        *err_types
                            .lock()
                            .unwrap()
                            .entry(format!("{:?}:{}", e.code(), e.message()))
                            .or_default() += 1;
                        refused.fetch_add(1, Relaxed);
                    }
                    Err(e) => {
                        let msg: String = e.message().chars().take(100).collect();
                        *err_types
                            .lock()
                            .unwrap()
                            .entry(format!("{:?}:{msg}", e.code()))
                            .or_default() += 1;
                        errs.fetch_add(1, Relaxed);
                    }
                }
                drop(permit);
            });
        }
        while tasks.try_join_next().is_some() {}
    }
    let st = if do_stats {
        stats(url, dir, false).await
    } else {
        String::new()
    };
    while tasks.join_next().await.is_some() {}
    println!(
        "load tenants={tenants} rate={rate}/s kind={kind} n={n} size={size} work_ms={work_ms} warm={warm}s measured={secs}s"
    );
    println!(
        "offered={} accepted={} refused_E_PROJECTION_DOWN={} errors={} overload_skipped={}",
        offered.load(Relaxed),
        ok.load(Relaxed),
        refused.load(Relaxed),
        errs.load(Relaxed),
        overload.load(Relaxed)
    );
    println!("errors_by_type {:?}", err_types.lock().unwrap());
    println!("accept_rtt_open_loop {}", fmt(&h.lock().unwrap()));
    let g = *gap.lock().unwrap();
    println!(
        "accept_max_gap_ms={:.0} from_us={} to_us={}",
        g.0 as f64 / 1e3,
        g.1,
        g.2
    );
    println!("-- statistiche del motore a fine finestra (prima dello svuotamento)\n{st}");
    // CPU di motore e worker per 1.000 attività (fase 3): il worker scrive la propria CPU su file, il motore la riporta nelle statistiche
    let num = |k: &str| {
        st.split(k)
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    let (done, ecpu, wcpu) = (num("done="), num("engine_cpu_s="), worker_cpu(dir) - w0);
    println!(
        "worker_cpu_s={wcpu:.2} cpu_per_1000_att engine={:.3} worker={:.3}",
        ecpu * 1000.0 / done.max(1.0),
        wcpu * 1000.0 / done.max(1.0)
    );
    if !ledger_path.is_empty() {
        let mut f = std::io::BufWriter::new(std::fs::File::create(&ledger_path).unwrap());
        for id in ledger.lock().unwrap().iter() {
            writeln!(f, "{}", crate::crypto::hex(id)).unwrap();
        }
    }
}

/// Verifica dopo il guasto: ogni accettata è conclusa una volta sola nella proiezione e sul bus.
pub async fn verify(db_url: &str, bus_kind: &str, dir: &str) {
    let ledger_path = arg("ledger", "");
    let db = crate::db::Db::open(db_url).await.unwrap();
    let accepted: Vec<String> = std::fs::read_to_string(&ledger_path)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    let done = db.done_ids().await.unwrap();
    let done_hex: std::collections::HashSet<String> =
        done.iter().map(|d| crate::crypto::hex(d)).collect();
    let missing = accepted.iter().filter(|a| !done_hex.contains(*a)).count();
    let bad = db
        .count("SELECT count(*) FROM activity WHERE done_us IS NOT NULL AND done_events <> 1")
        .await
        .unwrap();
    let open = db.count("SELECT count(*) FROM activity a JOIN execution e ON e.id = a.exec_id WHERE e.done_us IS NOT NULL AND e.status IS NULL AND a.done_us IS NULL").await.unwrap();
    let tot = db
        .count("SELECT count(*) FROM activity WHERE done_us IS NOT NULL")
        .await
        .unwrap();
    // ogni payload di uscita proiettato si legge dal segmento e si decifra (nessun payload confermato perso)
    let keys = crate::crypto::Keys::load(&format!("{dir}/master.key"));
    let (mut pay_ok, mut pay_bad) = (0u64, 0u64);
    let light = arg("light", "0") == "1";
    for (t, h, (seg, off, len)) in db.out_locs(if light { 997 } else { 1 }).await.unwrap() {
        use std::os::unix::fs::FileExt;
        let mut b = vec![0u8; len as usize];
        let r = std::fs::File::open(format!(
            "{dir}/archive/{}/{seg}.seg",
            crate::crypto::hex(&t)
        ))
        .and_then(|f| f.read_exact_at(&mut b, off));
        let good = r.is_ok()
            && std::panic::catch_unwind(|| {
                keys.open(&format!("arc/{}", crate::crypto::hex(&h)), &b)
            })
            .is_ok();
        if good { pay_ok += 1 } else { pay_bad += 1 }
    }
    // bus: conta gli eventi Done per attività (richiede trim=0)
    let bus = crate::bus::Bus::connect(bus_kind).await.unwrap();
    // light=1 (corse da 10.000 att./s, con taglio attivo): niente scansione del bus, payload a campione
    let mut dup = 0u64;
    let mut seen = std::collections::HashSet::new();
    let mut dones = 0u64;
    for t in TENANTS.iter().filter(|_| !light) {
        let topic = topic_of(t);
        let mut p = 0;
        loop {
            let r = bus.read(&topic, p, 1000).await.unwrap_or_default();
            if r.is_empty() {
                break;
            }
            for (pp, raw) in r {
                if let crate::ev::Ev::Done { exec, step, .. } =
                    crate::ev::Ev::decode(&keys.open(&format!("bus/{topic}"), &raw).2)
                {
                    dones += 1;
                    if !seen.insert((exec, step)) {
                        dup += 1;
                    }
                }
                p = pp + 1;
            }
        }
    }
    // passi `wait` (blocco C): quanti, quanti prima della scadenza (devono essere 0), scarto di scatto p50/p99/max
    let wd = db.wait_delays().await.unwrap();
    if !wd.is_empty() {
        let mut h = crate::hist();
        for d in &wd {
            h.record((*d).max(1) as u64).ok();
        }
        println!(
            "wait_delay: n={} scattati_prima_della_scadenza={} p50={:.1}ms p99={:.1}ms max={:.1}ms",
            wd.len(),
            wd.iter().filter(|d| **d < 0).count(),
            h.value_at_quantile(0.5) as f64 / 1e3,
            h.value_at_quantile(0.99) as f64 / 1e3,
            h.max() as f64 / 1e3
        );
    }
    println!(
        "verify: accettate={} concluse_nella_proiezione={} accettate_perse={missing} attivita_concluse={tot} done_events!=1:{bad} attivita_aperte_in_esecuzioni_concluse={open} eventi_Done_sul_bus={dones} duplicati_sul_bus={dup} payload_decifrati={pay_ok} payload_illeggibili={pay_bad} light={light}",
        accepted.len(),
        done.len()
    );
    let ok = missing == 0 && bad == 0 && open == 0 && dup == 0 && pay_bad == 0;
    println!("verify {}", if ok { "OK" } else { "FALLITA" });
    if !ok {
        std::process::exit(1);
    }
}

/// Tiene un lock esclusivo sul file SQLite per `secs` secondi (emulazione di "database fermo").
pub async fn lock_db(url: &str, secs: u64) {
    use sqlx::Connection;
    sqlx::any::install_default_drivers();
    let mut c = sqlx::AnyConnection::connect(url).await.unwrap();
    sqlx::query("BEGIN EXCLUSIVE")
        .execute(&mut c)
        .await
        .unwrap();
    println!("lock esclusivo preso per {secs}s (now_us={})", now_us());
    tokio::time::sleep(Duration::from_secs(secs)).await;
    sqlx::query("ROLLBACK").execute(&mut c).await.unwrap();
    println!("lock rilasciato (now_us={})", now_us());
}

/// Certificato autofirmato per localhost (C-14: generato, mai valori predefiniti).
pub fn gen_cert(dir: &str) {
    std::fs::create_dir_all(format!("{dir}/tls")).unwrap();
    let c = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    std::fs::write(format!("{dir}/tls/cert.pem"), c.cert.pem()).unwrap();
    std::fs::write(format!("{dir}/tls/key.pem"), c.signing_key.serialize_pem()).unwrap();
}
