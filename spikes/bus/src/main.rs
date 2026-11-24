//! Spike S-1 fase 2: Iggy contro Valkey. Codice da buttare; un solo binario con sottocomandi.
mod bench;
mod bus;
mod crypto;
mod db;
mod engine;
mod ev;
mod load;
mod iroh_link;
mod worker;

pub mod pb {
    tonic::include_proto!("spike");
}

use bus::Bus;
use bytes::Bytes;
use hdrhistogram::Histogram;
use std::time::{Duration, Instant};

/// Argomenti `chiave=valore` dopo il sottocomando.
pub fn arg(k: &str, d: &str) -> String {
    let p = format!("{k}=");
    // vince l'ultima occorrenza, così gli argomenti dopo i predefiniti dello script li sovrascrivono
    std::env::args().rev().find_map(|a| a.strip_prefix(&p).map(String::from)).unwrap_or(d.into())
}

pub fn hist() -> Histogram<u64> {
    Histogram::new_with_bounds(1, 120_000_000, 3).unwrap()
}

/// Microsecondi dall'epoca Unix: stesso orologio tra i processi della stessa macchina.
pub fn now_us() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64
}

/// p50/p99/max in ms, con il numero dei campioni.
pub fn fmt(h: &Histogram<u64>) -> String {
    format!(
        "n={} p50={:.2}ms p99={:.2}ms p99.9={:.2}ms max={:.2}ms over100ms={}",
        h.len(),
        h.value_at_quantile(0.5) as f64 / 1e3,
        h.value_at_quantile(0.99) as f64 / 1e3,
        h.value_at_quantile(0.999) as f64 / 1e3,
        h.max() as f64 / 1e3,
        h.count_between(100_000, u64::MAX)
    )
}

#[tokio::main]
async fn main() {
    // due provider (ring con tonic, aws-lc con rcgen): si sceglie ring in modo esplicito
    rustls::crypto::ring::default_provider().install_default().unwrap();
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let dir = arg("dir", "out");
    let url = arg("engine", "https://localhost:47010");
    match cmd.as_str() {
        "fsync-floor" => fsync_floor(),
        "bench-crypto" => bench_crypto(),
        "bench-append" => bench::append().await,
        "retention-proof" => bench::retention_proof().await,
        "bus-info" => bench::info().await,
        "gen-cert" => load::gen_cert(&dir),
        // credenziale del worker (C-11): 32 byte casuali in esadecimale, una per riga
        "gen-nodeid" => println!("{}", iroh::SecretKey::generate().public()),
        "gen-cred" => {
            let b = [*uuid::Uuid::new_v4().as_bytes(), *uuid::Uuid::new_v4().as_bytes()].concat();
            std::fs::write(format!("{dir}/worker.cred"), format!("{}\n", crypto::hex(&b))).unwrap()
        }
        "engine" => {
            engine::main(
                &arg("bus", "valkey"),
                &arg("db", "sqlite:out/spike.db?mode=rwc"),
                &arg("listen", "127.0.0.1:47010"),
                &dir,
                arg("cap", "512").parse().unwrap(),
                arg("proj_batch", "1000").parse().unwrap(),
                arg("trim", "1") == "1",
                arg("window_ms", "200").parse().unwrap(),
                arg("wconn", "1").parse().unwrap(),
                arg("kgroups", "4").parse().unwrap(),
                arg("maxb", "1024").parse().unwrap(),
                arg("minb", "64").parse().unwrap(),
                &arg("link", "grpc"),
                &arg("iroh_listen", "127.0.0.1:47011"),
                arg("busgen", "1").parse().unwrap(),
                arg("upgrade", "0") == "1",
            )
            .await
        }
        "worker" => worker::run(&url, &dir, arg("sessions", "4").parse().unwrap()).await,
        "load" => load::load(&url, &dir).await,
        "stats" => println!("{}", load::stats(&url, &dir, arg("reset", "0") == "1").await),
        "verify" => load::verify(&arg("db", ""), &arg("bus", "valkey"), &dir).await,
        "lock-db" => load::lock_db(&arg("db", ""), arg("secs", "10").parse().unwrap()).await,
        _ => eprintln!("sottocomandi: fsync-floor bench-append gen-cert engine worker load stats verify lock-db"),
    }
}

/// Costo di cifratura per oggetto (C-12): due HKDF + AES-256-GCM, più SHA-256 del payload.
fn bench_crypto() {
    let keys = crypto::Keys::load("out/bench.key");
    let t = [1u8; 16];
    for size in [1024usize, 65536, 1 << 20] {
        let data = vec![9u8; size];
        let n = if size >= 1 << 20 { 300 } else { 20000 };
        let (mut hs, mut ho, mut hh) = (hist(), hist(), hist());
        for i in 0..n {
            let id = [(i & 255) as u8; 16];
            let t0 = Instant::now();
            let env = keys.seal(&t, "bench", &id, &data);
            hs.record(t0.elapsed().as_nanos() as u64 / 100 + 1).unwrap();
            let t0 = Instant::now();
            let _ = keys.open("bench", &env);
            ho.record(t0.elapsed().as_nanos() as u64 / 100 + 1).unwrap();
            let t0 = Instant::now();
            let _ = crypto::sha(&data);
            hh.record(t0.elapsed().as_nanos() as u64 / 100 + 1).unwrap();
        }
        let f = |h: &Histogram<u64>| format!("p50={:.1}us p99={:.1}us", h.value_at_quantile(0.5) as f64 / 10.0, h.value_at_quantile(0.99) as f64 / 10.0);
        println!("bench-crypto size={size} n={n}: seal {} | open {} | sha256 {}", f(&hs), f(&ho), f(&hh));
    }
}

/// Soglia minima fisica: scrittura di 4 KB + fdatasync sul disco dei dati.
fn fsync_floor() {
    use std::os::unix::fs::FileExt;
    let dir = arg("dir", "out");
    let f = std::fs::File::create(format!("{dir}/floor.tmp")).unwrap();
    let buf = vec![7u8; 4096];
    let mut h = hist();
    let n: u64 = arg("n", "3000").parse().unwrap();
    for i in 0..n {
        let t = Instant::now();
        f.write_at(&buf, i * 4096).unwrap();
        f.sync_data().unwrap();
        h.record(t.elapsed().as_micros() as u64).unwrap();
    }
    println!("fsync-floor {dir}: {}", fmt(&h));
    std::fs::remove_file(format!("{dir}/floor.tmp")).unwrap();
}

