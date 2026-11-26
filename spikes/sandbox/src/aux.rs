//! Processo ausiliario di un tenant: tiene un `Engine` e i moduli precompilati, esegue istanze su richiesta.
//! Protocollo a righe su stdin/stdout, una risposta per richiesta:
//!   load name=<n> path=<cwasm> sha=<hex|-> kind=core|comp
//!   run name=<n> in=<file|-> limit_ms=<n> fuel=<n> mem_mb=<n> logcap=<n> outcap=<n> args=<a,b> hold=0|1 out=<file|->
//!   ping | stat | scan x=<hex mascherato> | free | quit

use crate::rt::*;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn kv(line: &str) -> HashMap<&str, &str> {
    line.split_whitespace()
        .skip(1)
        .filter_map(|t| t.split_once('='))
        .collect()
}

fn num<T: std::str::FromStr>(m: &HashMap<&str, &str>, k: &str, d: T) -> T {
    m.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
}

/// Cronometro dell'epoca: avanza l'epoca ogni millisecondo, ma solo mentre c'è una richiesta in corso
/// (altrimenti il thread dorme: nessun risveglio a vuoto). I tick sono ancorati a un istante iniziale, senza deriva.
fn ticker_loop(engine: wasmtime::Engine, active: Arc<AtomicBool>) {
    loop {
        while !active.load(Ordering::Acquire) {
            std::thread::park();
        }
        let t0 = Instant::now();
        let mut n = 0u32;
        while active.load(Ordering::Acquire) {
            n += 1;
            let target = t0 + Duration::from_millis(n as u64);
            if let Some(d) = target.checked_duration_since(Instant::now()) {
                std::thread::sleep(d);
            }
            engine.increment_epoch();
        }
    }
}

pub fn main(cfg: &str) {
    let rt = Rt::new(Opts::parse(cfg)).expect("engine");
    let active = Arc::new(AtomicBool::new(false));
    let ticker = if rt.opts.epoch {
        let h = std::thread::Builder::new().name("ticker".into());
        let (e, a) = (rt.engine.clone(), active.clone());
        Some(h.spawn(move || ticker_loop(e, a)).unwrap())
    } else {
        None
    };
    let mut mods: HashMap<String, Loaded> = HashMap::new();
    let mut held: Vec<Box<dyn std::any::Any>> = vec![];
    let out = std::io::stdout();
    let mut out = out.lock();
    writeln!(out, "ready").unwrap();
    out.flush().unwrap();
    for line in std::io::stdin().lock().lines() {
        let t0 = Instant::now();
        let line = line.unwrap();
        let cmd = line.split_whitespace().next().unwrap_or("");
        let m = kv(&line);
        let reply = match cmd {
            "ping" => "ok".to_string(),
            "quit" => break,
            "free" => {
                held.clear();
                "ok".into()
            }
            "stat" => {
                let (rss, pss, hwm, th) = mem_stat("self");
                format!(
                    "ok rss_kb={rss} pss_kb={pss} hwm_kb={hwm} threads={th} held={}",
                    held.len()
                )
            }
            "scan" => {
                let masked: Vec<u8> = (0..m["x"].len() / 2)
                    .map(|i| u8::from_str_radix(&m["x"][2 * i..2 * i + 2], 16).unwrap())
                    .collect();
                match scan_mem("self", &masked) {
                    Ok((h, r)) => format!("ok hits={h} read_bytes={r}"),
                    Err(e) => format!("err {e}"),
                }
            }
            "load" => {
                let path = std::path::Path::new(m["path"]);
                let t = Instant::now();
                let verify = m.get("sha").copied().unwrap_or("-");
                // l'impronta registrata dal compilatore si riverifica prima di toccare l'artefatto
                let vr = if verify == "-" {
                    Ok(())
                } else {
                    std::fs::read(path)
                        .map_err(|e| e.to_string())
                        .and_then(|b| {
                            if sha_hex(&b) == verify {
                                Ok(())
                            } else {
                                Err("sha mismatch".to_string())
                            }
                        })
                };
                let verify_us = t.elapsed().as_micros();
                match vr.and_then(|_| {
                    rt.load(
                        path,
                        m.get("kind") == Some(&"comp"),
                        m.get("plugin").map(std::path::Path::new),
                    )
                    .map_err(|e| format!("{e:#}"))
                }) {
                    Ok(l) => {
                        mods.insert(m["name"].to_string(), l);
                        format!(
                            "ok load_us={} verify_us={verify_us}",
                            t.elapsed().as_micros()
                        )
                    }
                    Err(e) => format!("err {}", e.replace('\n', " | ")),
                }
            }
            "run" => {
                let input = match m.get("in") {
                    Some(&"-") | None => vec![],
                    Some(p) => std::fs::read(p).unwrap_or_default(),
                };
                let q = RunReq {
                    input,
                    args: m
                        .get("args")
                        .map(|a| a.split(',').map(String::from).collect())
                        .unwrap_or_default(),
                    limit_ms: num(&m, "limit_ms", 1_000_000),
                    fuel: num(&m, "fuel", u64::MAX / 4),
                    mem_mb: num(&m, "mem_mb", 64),
                    log_cap: num(&m, "logcap", 1 << 20),
                    out_cap: num(&m, "outcap", 1 << 20),
                };
                let hold = m.get("hold") == Some(&"1");
                active.store(true, Ordering::Release);
                if let Some(t) = &ticker {
                    t.thread().unpark();
                }
                let r = match mods.get(m["name"]) {
                    Some(l) => rt.run(l, &q, t0, if hold { Some(&mut held) } else { None }),
                    None => RunRes {
                        failure: Some(("error".into(), "module not loaded".into())),
                        ..Default::default()
                    },
                };
                active.store(false, Ordering::Release);
                if let Some(p) = m.get("out").filter(|p| **p != "-") {
                    let mut s = format!(
                        "stdout={}\nstderr={}\nhost_output={}\n",
                        lossy(&r.stdout),
                        lossy(&r.stderr),
                        lossy(&r.host_output)
                    );
                    for (l, t) in &r.logs {
                        s += &format!("log {l} {t}\n");
                    }
                    for (st, e) in &r.emits {
                        s += &format!("emit {st} {e}\n");
                    }
                    std::fs::write(p, s).unwrap();
                }
                let fail = match &r.failure {
                    None => "none".to_string(),
                    Some((c, msg)) => {
                        format!("{c} msg={}", msg.chars().take(200).collect::<String>())
                    }
                };
                format!(
                    "ok t_inst_us={} t_start_us={} t_total_us={} outlen={} errlen={} logs={} emits={} fail={fail}",
                    r.t_inst_us,
                    r.t_start_us,
                    r.t_total_us,
                    r.stdout.len(),
                    r.stderr.len(),
                    r.logs.len(),
                    r.emits.len()
                )
            }
            _ => "err unknown command".into(),
        };
        writeln!(out, "{reply}").unwrap();
        out.flush().unwrap();
    }
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}
