//! Ruolo `load`: generatore deterministico dal seme, N connessioni, blocchi da 1.000 eventi, registro di
//! ogni blocco con la sua `Idempotency-Key` e l'esito. Un blocco senza risposta si riprova con la stessa
//! chiave fino al 200 (accettato = risposta 200).
//!
//! Il tempo dell'evento puo' partire nel passato e avanzare piu' in fretta dell'orologio, mai oltre
//! arrivo + 60 s (il generatore aspetta se e' troppo avanti), salvo gli eventi iniettati apposta.

use crate::evgen::{Gen, RunCfg};
use crate::http::Client;
use crate::spec::*;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub async fn run() {
    let addr = arg("--addr").unwrap_or("127.0.0.1:47111".into());
    let dir = arg("--run-dir").expect("--run-dir");
    let tenant = arg("--tenant").unwrap_or("acme".into());
    let stream = arg("--stream").unwrap_or("indicators".into());
    let num = |k: &str, d: i64| arg(k).map(|v| v.parse::<i64>().unwrap()).unwrap_or(d);
    let seed = num("--seed", 1) as u64;
    let events = num("--events", 100_000) as u64;
    let rate = num("--rate", 0);
    let flush_s = num("--flush-s", 0);
    let conns = num("--conns", 8) as usize;
    // 0 = fino alla fine degli eventi; altrimenti smette di emettere blocchi dopo N secondi (corsa a tempo)
    let duration = num("--duration-s", 0) as u64;
    let past_ms = num("--past-s", 0) * 1000;
    let evrate = num("--evrate", if rate > 0 { rate } else { 10_000 });
    // origine del tempo dell'evento: `--origin-ms` la fissa (stessi eventi, stesse righe a ogni corsa con lo stesso seme);
    // `--phase-s` aspetta che l'orologio raggiunga quella fase della griglia di `--phase-period-s` (predefinito 300 s), cosi' due
    // corse in tempo reale differiscono per un numero intero di periodi (`check --compare` le confronta dopo lo scarto)
    let (t0, wall0) = if let Some(o) = arg("--origin-ms") {
        let o: i64 = o.parse().unwrap();
        (o, o + past_ms)
    } else if let Some(ph) = arg("--phase-s") {
        let period = num("--phase-period-s", 300) * 1000;
        let phase = ph.parse::<i64>().unwrap() * 1000;
        let target = (now_ms() - phase).div_euclid(period) * period + phase + period;
        println!(
            "fase: attesa fino a {target} ({} s)",
            (target - now_ms()) / 1000
        );
        while now_ms() < target {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        (target - past_ms, target)
    } else {
        (now_ms() - past_ms, now_ms())
    };
    // C3, caso peggiore: l'ultimo blocco si trattiene fino a E + X, con E la fine della finestra (`--win-s`) dell'ultimo evento
    let hold_last_until: Option<i64> = arg("--hold-last-s").map(|x| {
        let win = num("--win-s", 20) * 1000;
        let t_last = t0 + ((events as i64 - 1 + if flush_s > 0 { 1 } else { 0 }) * 1000) / evrate;
        let e = (t_last.div_euclid(win) + 1) * win;
        println!(
            "hold-last: E={e} B_target={}",
            e + x.parse::<i64>().unwrap() * 1000
        );
        e + x.parse::<i64>().unwrap() * 1000
    });
    let cfg = RunCfg {
        seed,
        t0,
        wall0,
        rate,
        evrate,
        total: events + if flush_s > 0 { 1 } else { 0 },
        block: 1000,
        late_ppm: num("--late-ppm", 0) as u64,
        late_ms: num("--late-s", 400) * 1000,
        fut_ppm: num("--fut-ppm", 0) as u64,
        fut_ms: num("--fut-s", 120) * 1000,
        far_n: num("--far-n", -1),
        flush_ms: flush_s * 1000,
        asns: num("--asns", 5000) as usize,
        zipf_s: arg("--zipf").map(|v| v.parse().unwrap()).unwrap_or(1.0),
        prefix: arg("--prefix").unwrap_or(format!("r{seed}")),
        stream: stream.clone(),
    };
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        format!("{dir}/run.json"),
        serde_json::to_vec_pretty(&cfg).unwrap(),
    )
    .unwrap();
    let g = Arc::new(Gen::new(cfg.clone()));
    let log = Arc::new(Mutex::new(std::io::BufWriter::new(
        std::fs::File::create(format!("{dir}/blocks.jsonl")).unwrap(),
    )));
    let token = env(&format!("TT_TOKEN_{}", tenant.to_ascii_uppercase()));
    let admin = std::env::var("TT_ADMIN_TOKEN").unwrap_or_default();
    println!(
        "load seed={seed} eventi={events} blocchi={} conn={conns} rate={rate} evrate={evrate} t0={t0} past_ms={past_ms}",
        g.blocks()
    );
    let next = Arc::new(AtomicU64::new(0));
    let acc_events = Arc::new(AtomicU64::new(0));
    let acc_blocks = Arc::new(AtomicU64::new(0));
    let retries = Arc::new(AtomicU64::new(0));
    let noresp = Arc::new(AtomicU64::new(0));
    let fatal = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let mut hs = vec![];
    for _ in 0..conns {
        let (g, next, log, addr, token) = (
            g.clone(),
            next.clone(),
            log.clone(),
            addr.clone(),
            token.clone(),
        );
        let (acc_events, acc_blocks, retries, noresp, fatal) = (
            acc_events.clone(),
            acc_blocks.clone(),
            retries.clone(),
            noresp.clone(),
            fatal.clone(),
        );
        let stream = stream.clone();
        hs.push(tokio::spawn(async move {
            let mut c = Client::new(&addr);
            let path = format!("/v1/streams/{stream}/blocks");
            let auth = format!("Bearer {token}");
            loop {
                let b = next.fetch_add(1, Ordering::Relaxed);
                if b >= g.blocks() || (duration > 0 && start.elapsed().as_secs() >= duration) {
                    return;
                }
                let (a, z) = g.block_range(b);
                if rate > 0 {
                    let due = start + Duration::from_secs_f64((a as f64) / rate as f64);
                    tokio::time::sleep_until(tokio::time::Instant::from_std(due)).await;
                }
                // il tempo dell'evento non va oltre arrivo + 30 s (margine sul limite di 60 s)
                let last_nominal = g.cfg.t0 + ((z - 1) as i64) * 1000 / g.cfg.evrate;
                while last_nominal > now_ms() + 30_000 && !(g.cfg.flush_ms > 0 && z == g.cfg.total) {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                if let Some(until) = hold_last_until
                    && b + 1 == g.blocks()
                {
                    while now_ms() < until {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
                let body = g.body(b);
                let key = g.key(b);
                let mut attempts = 0;
                let mut no_resp = 0;
                let (status, arrival) = loop {
                    attempts += 1;
                    match c.post(&path, &[("Authorization", &auth), ("Idempotency-Key", &key)], body.as_bytes()).await {
                        Ok((200, r)) => {
                            let v: serde_json::Value = serde_json::from_slice(&r).unwrap_or_default();
                            break (200, v["arrival_ms"].as_i64().unwrap_or(0));
                        }
                        Ok((s, r)) if (400..500).contains(&s) => {
                            eprintln!("load: blocco {b} rifiutato {s}: {}", String::from_utf8_lossy(&r));
                            fatal.fetch_add(1, Ordering::Relaxed);
                            break (s, 0);
                        }
                        Ok(_) => tokio::time::sleep(Duration::from_millis(200)).await,
                        Err(_) => {
                            // nessuna risposta: l'esito e' incerto, si riprova con la stessa chiave
                            no_resp += 1;
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                    }
                };
                if attempts > 1 {
                    retries.fetch_add(attempts - 1, Ordering::Relaxed);
                }
                noresp.fetch_add(no_resp, Ordering::Relaxed);
                if status == 200 {
                    acc_events.fetch_add(z - a, Ordering::Relaxed);
                    acc_blocks.fetch_add(1, Ordering::Relaxed);
                }
                let line = format!(
                    "{{\"b\":{b},\"key\":\"{key}\",\"events\":{},\"status\":{status},\"arrival\":{arrival},\"done\":{},\"attempts\":{attempts},\"noresp\":{no_resp}}}\n",
                    z - a,
                    now_ms()
                );
                log.lock().unwrap().write_all(line.as_bytes()).unwrap();
            }
        }));
    }
    // ritardo del consumatore, campionato ogni 5 s: eventi accettati meno eventi elaborati
    let sampler = {
        let (acc_events, tenant, addr, admin) = (
            acc_events.clone(),
            tenant.clone(),
            addr.clone(),
            admin.clone(),
        );
        tokio::spawn(async move {
            let mut c = Client::new(&addr);
            let mut maxlag = 0i64;
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let acc = acc_events.load(Ordering::Relaxed) as i64;
                let auth = format!("Bearer {admin}");
                if let Ok((200, b)) = c
                    .request("GET", "/v1/stats", &[("Authorization", &auth)], b"")
                    .await
                {
                    let v: serde_json::Value = serde_json::from_slice(&b).unwrap_or_default();
                    let done: i64 = v["tenants"][&tenant]["streams"]
                        .as_array()
                        .map(|a| a.iter().map(|s| s["events"].as_i64().unwrap_or(0)).sum())
                        .unwrap_or(0);
                    let lag = acc - done;
                    maxlag = maxlag.max(lag);
                    println!(
                        "LAG t={:.0}s accettati={acc} elaborati={done} ritardo={lag} max={maxlag}",
                        start.elapsed().as_secs_f64()
                    );
                }
            }
        })
    };
    for h in hs {
        h.await.unwrap();
    }
    sampler.abort();
    log.lock().unwrap().flush().unwrap();
    let secs = start.elapsed().as_secs_f64();
    let ev = acc_events.load(Ordering::Relaxed);
    println!(
        "DONE blocchi_accettati={} eventi_accettati={ev} secondi={secs:.2} ritmo={:.0}/s riprove={} senza_risposta={} rifiutati={}",
        acc_blocks.load(Ordering::Relaxed),
        ev as f64 / secs,
        retries.load(Ordering::Relaxed),
        noresp.load(Ordering::Relaxed),
        fatal.load(Ordering::Relaxed)
    );
}
