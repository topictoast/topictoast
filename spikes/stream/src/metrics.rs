//! Istogramma a secchi logaritmici per le latenze di T3, scritto nei processi e letto dalle statistiche (parte 2b).
//!
//! Secchi: 16 per ogni potenza di due, quindi un errore relativo massimo di circa 4,4 % sul valore riportato (il centro
//! del secchio). I valori sono in microsecondi. Nessun campione si conserva: costo fisso per `record`.

use serde_json::{Value, json};
use std::sync::Mutex;

const SUB: usize = 16;
const BUCKETS: usize = 64 * SUB;

pub struct Hist {
    n: u64,
    max: u64,
    b: Vec<u64>,
}

impl Default for Hist {
    fn default() -> Hist {
        Hist {
            n: 0,
            max: 0,
            b: vec![0; BUCKETS],
        }
    }
}

fn bucket(v: u64) -> usize {
    if v < SUB as u64 {
        return v as usize;
    }
    let e = 63 - v.leading_zeros() as usize;
    let frac = ((v >> (e - 4)) & 15) as usize;
    ((e - 3) * SUB + frac).min(BUCKETS - 1)
}

/// Valore di mezzo del secchio `i`.
fn mid(i: usize) -> u64 {
    if i < SUB {
        return i as u64;
    }
    let e = i / SUB + 3;
    let frac = (i % SUB) as u64;
    let lo = (16 + frac) << (e - 4);
    lo + (1u64 << (e - 4)) / 2
}

impl Hist {
    pub fn record(&mut self, us: u64) {
        self.n += 1;
        self.max = self.max.max(us);
        self.b[bucket(us)] += 1;
    }

    /// Percentile `p` (0..100) in microsecondi; 0 se vuoto.
    pub fn pct(&self, p: f64) -> u64 {
        if self.n == 0 {
            return 0;
        }
        let want = ((self.n as f64) * p / 100.0).ceil().max(1.0) as u64;
        let mut acc = 0;
        for (i, c) in self.b.iter().enumerate() {
            acc += c;
            if acc >= want {
                return mid(i).min(self.max);
            }
        }
        self.max
    }

    pub fn json(&self) -> Value {
        json!({"n": self.n, "p50_us": self.pct(50.0), "p99_us": self.pct(99.0), "max_us": self.max})
    }
}

/// Istogramma condiviso tra il ciclo di lettura e il consegnatore.
pub type Shared = Mutex<Hist>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentili_entro_il_5_per_cento() {
        let mut h = Hist::default();
        for v in 1..=10_000u64 {
            h.record(v);
        }
        let p50 = h.pct(50.0) as f64;
        let p99 = h.pct(99.0) as f64;
        assert!((p50 - 5000.0).abs() / 5000.0 < 0.05, "p50={p50}");
        assert!((p99 - 9900.0).abs() / 9900.0 < 0.05, "p99={p99}");
    }
}
