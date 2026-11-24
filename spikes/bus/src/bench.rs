//! Prove sul solo bus (fase 4): scrittura a ritmo fisso con la forma del traffico del motore (blocco A,
//! stalli di Iggy) e prova delle primitive di conservazione (`retention-proof`).

use crate::bus::Bus;
use crate::{arg, fmt, hist};
use bytes::Bytes;
use hdrhistogram::Histogram;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Riga della pressione I/O del sistema, come la stampa il motore.
fn io_pressure() -> String {
    std::fs::read_to_string("/proc/pressure/io").ok().and_then(|s| s.lines().next().map(String::from)).unwrap_or_default()
}

/// Metrica del minuto corrente di `append`.
struct Min {
    h: Histogram<u64>,
    msgs: u64,
    batches: u64,
    lag_max_us: u64,
}

/// Solo il bus: `conc` task, ognuno scrive gruppi di `batch` messaggi da `size` byte.
///
/// Opzioni oltre a quelle della fase 2: `rate` (messaggi/s totali, 0 = a ciclo chiuso), `mix=1` (ogni 21
/// messaggi, 11 grandi e 10 da 80 byte: un `Accepted`, dieci `Done` e dieci `Ready` di un'esecuzione a 10 passi),
/// `topics=N` (il task i scrive sul topic i mod N, connessioni proprie con `own=1`), `minsecs` (periodo della riga
/// `BMIN`, 60). Il ritmo è a scadenze fisse: se un'append supera il periodo le successive partono subito e il
/// ritardo rispetto alla scadenza è riportato (`lagmax_ms`), così una coda che si accumula non si nasconde.
pub async fn append() {
    let kind = arg("bus", "iggy");
    let size: usize = arg("size", "1024").parse().unwrap();
    let batch: usize = arg("batch", "1").parse().unwrap();
    let conc: usize = arg("conc", "1").parse().unwrap();
    let secs: u64 = arg("secs", "10").parse().unwrap();
    let warm: u64 = arg("warmup", "2").parse().unwrap();
    let own = arg("own", "0") == "1";
    // same=1: connessioni proprie ma stesso topic (come il motore, che scrive tutti sullo stesso topic del tenant)
    let same = arg("same", "0") == "1";
    let rate: f64 = arg("rate", "0").parse().unwrap();
    let mix = arg("mix", "0") == "1";
    let topics: usize = arg("topics", "0").parse().unwrap();
    let minsecs: u64 = arg("minsecs", "60").parse().unwrap();
    let bus = Bus::connect(&kind).await.unwrap();
    let topic = format!("bench-{}", std::process::id());
    bus.ensure_topic(&topic).await.unwrap();
    for k in 0..topics {
        bus.ensure_topic(&format!("{topic}-{k}")).await.unwrap();
    }
    let big = Bytes::from(vec![42u8; size]);
    let small = Bytes::from(vec![43u8; 80]);
    let start = Instant::now();
    let total = Duration::from_secs(warm + secs);
    let min = Arc::new(Mutex::new(Min { h: hist(), msgs: 0, batches: 0, lag_max_us: 0 }));
    let mon = {
        let min = min.clone();
        tokio::spawn(async move {
            let mut t = 0;
            loop {
                tokio::time::sleep(Duration::from_secs(minsecs)).await;
                t += 1;
                let mut m = min.lock().unwrap();
                let ms = |q: f64| m.h.value_at_quantile(q) as f64 / 1e3;
                println!("BMIN t={t} msgs_s={:.0} batches={} append_p50_ms={:.1} append_p99_ms={:.1} append_max_ms={:.1} lagmax_ms={:.0} io=[{}]", m.msgs as f64 / minsecs as f64, m.batches, ms(0.5), ms(0.99), m.h.max() as f64 / 1e3, m.lag_max_us as f64 / 1e3, io_pressure());
                m.h.reset();
                m.msgs = 0;
                m.batches = 0;
                m.lag_max_us = 0;
            }
        })
    };
    let mut tasks = vec![];
    for i in 0..conc {
        // own=1: ogni task ha connessione propria; il topic è proprio salvo same=1 o topics=N
        let (bus, big, small, min) = (bus.clone(), big.clone(), small.clone(), min.clone());
        let topic = if topics > 0 {
            format!("{topic}-{}", i % topics)
        } else if own && !same {
            format!("{topic}-{}", tasks.len())
        } else {
            topic.clone()
        };
        tasks.push(tokio::spawn(async move {
            let bus = if own { Bus::connect(bus.name()).await.unwrap() } else { bus };
            if topics == 0 && !(own && same) {
                bus.ensure_topic(&topic).await.unwrap();
            }
            let (mut h, mut n) = (hist(), 0u64);
            let period = (rate > 0.0).then(|| Duration::from_secs_f64(batch as f64 * conc as f64 / rate));
            let mut next = Instant::now();
            let mut k = i * 7;
            while start.elapsed() < total {
                let mut lag = 0;
                if let Some(p) = period {
                    next += p;
                    let now = Instant::now();
                    if next > now {
                        tokio::time::sleep_until(tokio::time::Instant::from_std(next)).await;
                    } else {
                        lag = (now - next).as_micros() as u64;
                    }
                }
                let msgs: Vec<Bytes> = (0..batch)
                    .map(|j| if !mix || (k + j) % 21 < 11 { big.clone() } else { small.clone() })
                    .collect();
                k += batch;
                let t = Instant::now();
                bus.append(&topic, &msgs).await.unwrap();
                let us = t.elapsed().as_micros() as u64;
                {
                    let mut m = min.lock().unwrap();
                    m.h.record(us.max(1)).ok();
                    m.msgs += batch as u64;
                    m.batches += 1;
                    m.lag_max_us = m.lag_max_us.max(lag);
                }
                if start.elapsed() > Duration::from_secs(warm) {
                    h.record(us.max(1)).unwrap();
                    n += batch as u64;
                }
            }
            if own && !same && topics == 0 {
                bus.drop_topic(&topic).await.ok();
            }
            (h, n, topic)
        }));
    }
    let (mut h, mut n) = (hist(), 0);
    let mut used = std::collections::BTreeSet::new();
    for t in tasks {
        let (a, b, tp) = t.await.unwrap();
        h.add(a).unwrap();
        n += b;
        used.insert(tp);
    }
    mon.abort();
    for tp in used {
        bus.drop_topic(&tp).await.ok();
    }
    bus.drop_topic(&topic).await.ok();
    println!("bench-append bus={kind} size={size} batch={batch} conc={conc} rate={rate} mix={mix}: {} msgs/s={:.0}", fmt(&h), n as f64 / secs as f64);
}

/// Sostituisce 8 byte iniziali del payload con l'indice del messaggio: dopo una cancellazione di segmenti
/// l'offset letto deve coincidere con l'indice scritto (nessuno slittamento).
fn payload(i: u64, size: usize) -> Bytes {
    let mut v = vec![7u8; size];
    v[..8].copy_from_slice(&i.to_le_bytes());
    Bytes::from(v)
}

/// Aspetta che il numero di segmenti si fermi (la cancellazione sul disco avviene al passo successivo del server).
async fn settle(bus: &Bus, topic: &str) -> (u32, u64, u64) {
    let mut last = bus.segments(topic).await.unwrap();
    let mut same = 0;
    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let now = bus.segments(topic).await.unwrap();
        if now == last {
            same += 1;
            if same >= 3 {
                break;
            }
        } else {
            same = 0;
        }
        last = now;
    }
    last
}

/// Primo offset leggibile e verifica che da lì alla testa gli offset siano contigui e i payload coincidano con l'indice.
async fn first_and_check(bus: &Bus, topic: &str, head: u64) -> (u64, bool) {
    let mut from = 0u64;
    let (mut first, mut next) = (None, None);
    let mut ok = true;
    loop {
        let r = match bus.read(topic, from, 500).await {
            Ok(r) => r,
            Err(e) => {
                println!("  lettura da {from} fallita: {e}");
                break;
            }
        };
        if r.is_empty() {
            break;
        }
        for (o, b) in &r {
            first.get_or_insert(*o);
            if next.is_some_and(|n| n != *o) || u64::from_le_bytes(b[..8].try_into().unwrap()) != *o {
                ok = false;
            }
            next = Some(o + 1);
            from = o + 1;
        }
    }
    // nulla di leggibile è corretto solo se tutto fino alla testa è stato applicato e cancellato (primo = testa)
    (first.unwrap_or(head), ok && (next == Some(head) || first.is_none()))
}

/// Prova delle primitive di conservazione di Iggy 0.10.0 (fase 4, blocco A). Ogni riga: posizione del proiettore
/// (la prossima da leggere) passata a `trim`, stato dopo l'assestamento, e se la regola «mai oltre la posizione
/// applicata» regge. Va lanciata con `IGGY_SEGMENT_SIZE=1048576` (segmenti da 1 MiB, il minimo).
pub async fn retention_proof() {
    let bus = Bus::connect("iggy").await.unwrap();
    let size: usize = arg("size", "20000").parse().unwrap();
    let per: usize = arg("batch", "10").parse().unwrap();
    let nb: usize = arg("batches", "60").parse().unwrap();
    let topic = format!("ret-{}", std::process::id());
    bus.ensure_topic(&topic).await.unwrap();
    let mut i = 0u64;
    for _ in 0..nb {
        let msgs: Vec<Bytes> = (0..per).map(|_| { i += 1; payload(i - 1, size) }).collect();
        bus.append(&topic, &msgs).await.unwrap();
    }
    let head = bus.head(&topic).await.unwrap();
    let (s0, m0, b0) = settle(&bus, &topic).await;
    println!("scritti {i} messaggi da {size} byte, testa={head}; segmenti={s0} messaggi={m0} byte={b0}");
    // mappa dei segmenti: dopo ogni trim si vede il primo offset retenuto; i confini si ricavano dalle cancellazioni
    let mut violations = 0;
    let mut prev_first = 0u64;
    let steps = [0u64, 1, head / 8, head / 4 + 3, head / 2, head / 2 + 1, head * 3 / 4, head - 1, head];
    for pos in steps {
        if pos > 0 {
            bus.trim(&topic, pos).await.unwrap();
        }
        let (segs, msgs, bytes) = settle(&bus, &topic).await;
        let (first, intact) = first_and_check(&bus, &topic, head).await;
        let stored = bus.consumer_offset(&topic).await.unwrap();
        // regola: ogni posizione >= pos resta leggibile, quindi il primo offset retenuto è <= pos;
        // e nulla di retenuto è stato ricopiato male (offset contigui fino alla testa, payload = indice)
        let rule = first <= pos && intact && first >= prev_first;
        if !rule {
            violations += 1;
        }
        println!("trim(pos={pos}) soglia salvata={stored:?} -> primo offset leggibile={first} segmenti={segs} messaggi={msgs} byte={bytes} contigui={intact} regola={}", if rule { "ok" } else { "VIOLATA" });
        prev_first = first;
    }
    println!("violazioni={violations}");
    // controllo negativo: senza posizione salvata il server non ha soglia e cancella ogni segmento chiuso
    let t2 = format!("ret2-{}", std::process::id());
    bus.ensure_topic(&t2).await.unwrap();
    let mut j = 0u64;
    for _ in 0..nb {
        let msgs: Vec<Bytes> = (0..per).map(|_| { j += 1; payload(j - 1, size) }).collect();
        bus.append(&t2, &msgs).await.unwrap();
    }
    let h2 = bus.head(&t2).await.unwrap();
    let (a, _, _) = settle(&bus, &t2).await;
    // chiamata diretta senza trim(): nessun consumatore registrato
    #[cfg(not(feature = "old"))]
    {
        use iggy::prelude::*;
        if let Bus::Iggy(c) = &bus {
            let (s, t) = (Identifier::named("tt").unwrap(), Identifier::named(&t2).unwrap());
            c.delete_segments(&s, &t, 0, u32::MAX).await.unwrap();
        }
    }
    let (b, _, _) = settle(&bus, &t2).await;
    let (first2, _) = first_and_check(&bus, &t2, h2).await;
    println!("controllo negativo, nessun consumatore registrato: segmenti {a} -> {b}, primo offset leggibile={first2} su testa {h2} (senza soglia si perde tutto ciò che è chiuso)");
    bus.drop_topic(&topic).await.ok();
    bus.drop_topic(&t2).await.ok();
    if violations > 0 {
        std::process::exit(1);
    }
}

/// Stato dei topic dei tenant (testa, segmenti, messaggi, byte, posizione del consumatore): `busspike bus-info`.
pub async fn info() {
    let bus = Bus::connect("iggy").await.unwrap();
    for t in &crate::engine::TENANTS {
        let topic = crate::engine::topic_of(t);
        let (segs, msgs, bytes) = bus.segments(&topic).await.unwrap();
        let head = bus.head(&topic).await.unwrap();
        let stored = bus.consumer_offset(&topic).await.unwrap();
        let first = bus.read(&topic, 0, 1).await.ok().and_then(|r| r.first().map(|m| m.0));
        println!("bus-info {topic}: testa={head} primo_offset_leggibile={first:?} segmenti={segs} messaggi={msgs} byte={bytes} soglia_consumatore={stored:?}");
    }
}
