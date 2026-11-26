//! Generatore deterministico degli eventi (forma di UJ-3: indicatori con `asn`, `ip`, `kind` e tempo
//! dell'evento). Lo usano `load` (per inviarli) e `check` (per rigenerarli dal seme: l'oracolo non si fida
//! del percorso misurato).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunCfg {
    pub seed: u64,
    /// tempo dell'evento del primo evento (ms)
    pub t0: i64,
    /// eventi per secondo di tempo dell'evento
    pub evrate: i64,
    /// eventi totali, compreso quello di chiusura se `flush_ms > 0`
    pub total: u64,
    pub block: u64,
    /// ritardatari iniettati (parti per milione) e di quanto
    pub late_ppm: u64,
    pub late_ms: i64,
    /// eventi nel futuro oltre la tolleranza (ppm) e di quanto
    pub fut_ppm: u64,
    pub fut_ms: i64,
    /// indice dell'evento a un anno nel futuro (-1 nessuno)
    pub far_n: i64,
    /// se > 0 l'ultimo evento ha tempo = penultimo + `flush_ms` (chiude le finestre aperte)
    pub flush_ms: i64,
    pub asns: usize,
    pub zipf_s: f64,
    pub prefix: String,
    pub stream: String,
}

fn mix(seed: u64, n: u64, salt: u64) -> u64 {
    let mut z = seed
        .wrapping_add(n.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(salt.wrapping_mul(0xD1B5_4A32_D192_ED03));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub struct Ev {
    pub asn: i64,
    pub ip: String,
    pub kind: &'static str,
    pub ts: i64,
}

pub const KINDS: [&str; 4] = ["malware", "phishing", "c2", "scanner"];

pub struct Gen {
    pub cfg: RunCfg,
    cdf: Vec<f64>,
}

impl Gen {
    pub fn new(cfg: RunCfg) -> Gen {
        let w: Vec<f64> = (1..=cfg.asns).map(|r| 1.0 / (r as f64).powf(cfg.zipf_s)).collect();
        let tot: f64 = w.iter().sum();
        let mut acc = 0.0;
        let cdf = w
            .iter()
            .map(|x| {
                acc += x / tot;
                acc
            })
            .collect();
        Gen { cfg, cdf }
    }

    fn nominal(&self, n: u64) -> i64 {
        self.cfg.t0 + (n as i64) * 1000 / self.cfg.evrate
    }

    pub fn event(&self, n: u64) -> Ev {
        let c = &self.cfg;
        let mut ts = if c.flush_ms > 0 && n == c.total - 1 {
            self.nominal(n - 1) + c.flush_ms
        } else {
            self.nominal(n)
        };
        let r = mix(c.seed, n, 1) % 1_000_000;
        if c.flush_ms > 0 && n == c.total - 1 {
            // l'evento di chiusura non si altera
        } else if r < c.late_ppm {
            ts -= c.late_ms;
        } else if r < c.late_ppm + c.fut_ppm {
            ts += c.fut_ms;
        }
        if n as i64 == c.far_n {
            ts += 365 * 86_400_000;
        }
        let u = (mix(c.seed, n, 2) >> 11) as f64 / (1u64 << 53) as f64;
        let rank = self.cdf.partition_point(|x| *x < u).min(self.cdf.len() - 1);
        let h = mix(c.seed, n, 3);
        Ev {
            asn: 1000 + rank as i64,
            ip: format!("{}.{}.{}.{}", if h & 1 == 0 { 10 } else { 192 }, (h >> 8) & 255, (h >> 16) & 255, (h >> 24) % 400),
            kind: KINDS[(mix(c.seed, n, 4) % 4) as usize],
            ts,
        }
    }

    /// Intervallo `[da, a)` degli indici degli eventi del blocco `b`.
    pub fn block_range(&self, b: u64) -> (u64, u64) {
        (b * self.cfg.block, ((b + 1) * self.cfg.block).min(self.cfg.total))
    }

    pub fn blocks(&self) -> u64 {
        self.cfg.total.div_ceil(self.cfg.block)
    }

    pub fn key(&self, b: u64) -> String {
        format!("{}-{b}", self.cfg.prefix)
    }

    /// Corpo JSON del blocco `b`.
    pub fn body(&self, b: u64) -> String {
        let (a, z) = self.block_range(b);
        let mut s = String::with_capacity(((z - a) * 90) as usize + 16);
        s.push_str("{\"events\":[");
        for n in a..z {
            let e = self.event(n);
            if n > a {
                s.push(',');
            }
            s.push_str(&format!("{{\"asn\":{},\"ip\":\"{}\",\"kind\":\"{}\",\"ts\":{}}}", e.asn, e.ip, e.kind, e.ts));
        }
        s.push_str("]}");
        s
    }
}
