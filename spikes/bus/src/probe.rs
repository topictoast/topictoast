//! Sonda del bus (fase 4, blocco B): una append ogni `period_ms` su un topic proprio, stampa esito e durata di ognuna
//! (`P <ms dall'inizio> ok|ERR <durata ms> <errore>`), senza mai fermarsi su un errore. Serve a vedere come rispondono
//! il client e il server a un crash o a un disco pieno.

use crate::arg;
use crate::bus::Bus;
use bytes::Bytes;
use std::time::{Duration, Instant};

pub async fn run() {
    let size: usize = arg("size", "1024").parse().unwrap();
    let batch: usize = arg("batch", "1").parse().unwrap();
    let secs: u64 = arg("secs", "30").parse().unwrap();
    let period: u64 = arg("period_ms", "100").parse().unwrap();
    let bus = Bus::connect(&arg("bus", "iggy")).await.unwrap();
    let topic = "probe".to_string();
    bus.ensure_topic(&topic).await.unwrap();
    let msgs: Vec<Bytes> = (0..batch).map(|_| Bytes::from(vec![5u8; size])).collect();
    let start = Instant::now();
    let mut acked = 0u64;
    while start.elapsed() < Duration::from_secs(secs) {
        let t = Instant::now();
        let r = tokio::time::timeout(Duration::from_secs(20), bus.append(&topic, &msgs)).await;
        let (el, ms) = (start.elapsed().as_millis(), t.elapsed().as_millis());
        match r {
            Ok(Ok(())) => {
                acked += batch as u64;
                println!("P {el} ok {ms}")
            }
            Ok(Err(e)) => println!("P {el} ERR {ms} {e}"),
            Err(_) => println!("P {el} ERR {ms} timeout 20 s"),
        }
        tokio::time::sleep(Duration::from_millis(period)).await;
    }
    // messaggi riconosciuti dal server contro testa del topic (la testa può superarli: un'append senza risposta può essere durevole)
    println!(
        "END acked={acked} head={:?}",
        bus.head(&topic).await.map_err(|e| e.to_string())
    );
}
