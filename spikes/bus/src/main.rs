//! Spike S-1 fase 2: Iggy contro Valkey. Codice da buttare; un solo binario con sottocomandi.
mod bus;

use bus::Bus;
use bytes::Bytes;
use hdrhistogram::Histogram;
use std::time::{Duration, Instant};

/// Argomenti `chiave=valore` dopo il sottocomando.
pub fn arg(k: &str, d: &str) -> String {
    let p = format!("{k}=");
    std::env::args().find_map(|a| a.strip_prefix(&p).map(String::from)).unwrap_or(d.into())
}

pub fn hist() -> Histogram<u64> {
    Histogram::new_with_bounds(1, 120_000_000, 3).unwrap()
}

/// p50/p99/max in ms, con il numero dei campioni.
pub fn fmt(h: &Histogram<u64>) -> String {
    format!(
        "n={} p50={:.2}ms p99={:.2}ms p99.9={:.2}ms max={:.2}ms",
        h.len(),
        h.value_at_quantile(0.5) as f64 / 1e3,
        h.value_at_quantile(0.99) as f64 / 1e3,
        h.value_at_quantile(0.999) as f64 / 1e3,
        h.max() as f64 / 1e3
    )
}

#[tokio::main]
async fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    match cmd.as_str() {
        "fsync-floor" => fsync_floor(),
        "bench-append" => bench_append().await,
        _ => eprintln!("sottocomandi: fsync-floor bench-append"),
    }
}

/// Soglia minima fisica: scrittura di 4 KB + fdatasync sul disco dei dati.
fn fsync_floor() {
    use std::io::Write;
    use std::os::unix::fs::FileExt;
    let dir = arg("dir", "data");
    let f = std::fs::File::create(format!("{dir}/floor.tmp")).unwrap();
    let buf = vec![7u8; 4096];
    let mut h = hist();
    for i in 0..3000u64 {
        let t = Instant::now();
        f.write_at(&buf, i * 4096).unwrap();
        f.sync_data().unwrap();
        h.record(t.elapsed().as_micros() as u64).unwrap();
    }
    std::io::stdout().flush().unwrap();
    println!("fsync-floor {dir}: {}", fmt(&h));
    std::fs::remove_file(format!("{dir}/floor.tmp")).unwrap();
}

/// Solo il bus: `conc` task, ognuno scrive gruppi di `batch` messaggi da `size` byte.
async fn bench_append() {
    let kind = arg("bus", "iggy");
    let size: usize = arg("size", "1024").parse().unwrap();
    let batch: usize = arg("batch", "1").parse().unwrap();
    let conc: usize = arg("conc", "1").parse().unwrap();
    let secs: u64 = arg("secs", "10").parse().unwrap();
    let warm: u64 = arg("warmup", "2").parse().unwrap();
    let own = arg("own", "0") == "1";
    let bus = Bus::connect(&kind).await.unwrap();
    let topic = format!("bench-{}", std::process::id());
    bus.ensure_topic(&topic).await.unwrap();
    let payload = Bytes::from(vec![42u8; size]);
    let start = Instant::now();
    let mut tasks = vec![];
    for _ in 0..conc {
        // own=1: ogni task ha connessione e topic propri; own=0: tutti sulla stessa connessione e topic
        let (bus, payload) = (bus.clone(), payload.clone());
        let topic = if own { format!("{topic}-{}", tasks.len()) } else { topic.clone() };
        tasks.push(tokio::spawn(async move {
            let bus = if own { Bus::connect(bus.name()).await.unwrap() } else { bus };
            bus.ensure_topic(&topic).await.unwrap();
            let (mut h, mut n) = (hist(), 0u64);
            let msgs = vec![payload; batch];
            while start.elapsed() < Duration::from_secs(warm + secs) {
                let t = Instant::now();
                bus.append(&topic, &msgs).await.unwrap();
                if start.elapsed() > Duration::from_secs(warm) {
                    h.record(t.elapsed().as_micros() as u64).unwrap();
                    n += batch as u64;
                }
            }
            if own {
                bus.drop_topic(&topic).await.ok();
            }
            (h, n)
        }));
    }
    let (mut h, mut n) = (hist(), 0);
    for t in tasks {
        let (a, b) = t.await.unwrap();
        h.add(a).unwrap();
        n += b;
    }
    bus.drop_topic(&topic).await.ok();
    println!(
        "bench-append bus={kind} size={size} batch={batch} conc={conc}: {} msgs/s={:.0}",
        fmt(&h),
        n as f64 / secs as f64
    );
}
