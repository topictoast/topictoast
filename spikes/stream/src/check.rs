//! Ruolo `check`: l'oracolo indipendente (fuori dal percorso misurato). Dal seme (`run.json`), dal registro dei
//! blocchi (`blocks.jsonl`) e dall'ordine del log di Iggy ricalcola le finestre chiuse con un ciclo semplice sugli
//! eventi rigenerati, SENZA DataFusion e senza il codice delle pipeline, e le confronta con gli avvii che il
//! motore ha accettato (database SQLite) e con i contatori del processo per tenant.
//!
//! Dal log prende solo l'ordine e le intestazioni (arrivo, chiave, segnali); gli eventi si rigenerano.
//! Le regole sono quelle scritte in `pipeline.rs`, riscritte qui in modo indipendente.

use crate::bus::Bus;
use crate::crypto::{Master, tenant_uid};
use crate::evgen::{Ev, Gen, RunCfg};
use crate::spec::*;
use crate::wire::{self, Msg};
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::sqlite::SqliteConnectOptions;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Default, Clone)]
struct Agg {
    n: i64,
    asns: BTreeSet<i64>,
    sum: i64,
    min: i64,
    max: i64,
}

struct O {
    def: PipelineDef,
    size: i64,
    slide: i64,
    wm: Option<i64>,
    last_wm: i64,
    last_arrival: i64,
    late: u64,
    fut: u64,
    /// finestra -> gruppo -> aggregati
    wins: BTreeMap<i64, BTreeMap<String, Agg>>,
    /// righe senza finestra
    rows: Vec<(String, Value)>,
}

impl O {
    fn new(def: &PipelineDef) -> O {
        O {
            def: def.clone(),
            size: def.oracle_size_s * 1000,
            slide: def.oracle_slide_s * 1000,
            wm: None,
            last_wm: 0,
            last_arrival: 0,
            late: 0,
            fut: 0,
            wins: BTreeMap::new(),
            rows: vec![],
        }
    }

    fn signal(&mut self, s: i64) {
        if let Some(w) = self.wm
            && s - self.last_arrival >= self.def.quiet_s * 1000
        {
            self.wm = Some(w.max(self.last_wm + (s - self.last_arrival)));
        }
    }

    /// Gruppo e filtro (WHERE) per tipo di prova; `None` = l'evento non passa il filtro.
    fn group(&self, e: &Ev) -> Option<String> {
        match self.def.oracle.as_str() {
            "uj3" | "uj3v" => Some(e.asn.to_string()),
            "hop" => e.ip.starts_with("10.").then(|| e.kind.to_string()),
            "distinct" => (e.asn < 1100).then(|| format!("{}|{}", e.kind, e.asn)),
            "ipdistinct" => (e.kind == "malware" && e.asn < 1020).then(|| e.ip.clone()),
            _ => None,
        }
    }

    fn block(&mut self, off: u64, arrival: i64, evs: &[Ev]) {
        if self.def.oracle == "filter" {
            for (i, e) in evs.iter().enumerate() {
                if e.asn == 1020 {
                    self.rows.push((
                        format!("{}:{}:{}:{}", self.def.id, self.def.version, off, i),
                        json!({"asn":e.asn,"ip":e.ip,"kind":e.kind.to_uppercase()}),
                    ));
                }
            }
            return;
        }
        let lb = self.wm.map(|w| w - self.size - self.def.lateness_s * 1000);
        let mut maxt: Option<i64> = None;
        for e in evs {
            let t = e.ts;
            if t > arrival + self.def.future_s * 1000 {
                self.fut += 1;
                continue;
            }
            maxt = Some(maxt.map_or(t, |m| m.max(t)));
            let mut starts = vec![];
            let mut s = t.div_euclid(self.slide) * self.slide;
            while s > t - self.size {
                starts.push(s);
                s -= self.slide;
            }
            let closed = |s: i64| lb.is_some_and(|l| s < l);
            if starts.iter().any(|s| closed(*s)) {
                self.late += 1;
            }
            let Some(g) = self.group(e) else { continue };
            for s in starts.into_iter().filter(|s| !closed(*s)) {
                let a = self.wins.entry(s).or_default().entry(g.clone()).or_insert_with(|| Agg {
                    min: i64::MAX,
                    max: i64::MIN,
                    ..Agg::default()
                });
                a.n += 1;
                a.asns.insert(e.asn);
                a.sum += e.asn;
                a.min = a.min.min(e.asn);
                a.max = a.max.max(e.asn);
            }
        }
        if let Some(m) = maxt {
            self.wm = Some(self.wm.map_or(m, |w| w.max(m)));
        }
        self.last_wm = self.wm.unwrap_or(0);
        self.last_arrival = arrival;
    }

    /// Righe attese: finestre chiuse al watermark finale.
    fn expected(&self) -> BTreeMap<String, Value> {
        let mut out: BTreeMap<String, Value> = self.rows.iter().cloned().collect();
        let Some(wm) = self.wm else { return out };
        let lim = wm - self.size - self.def.lateness_s * 1000;
        let key = |we: i64, g: &str| format!("{}:{}:{}:{}", self.def.id, self.def.version, we, g);
        for (ws, groups) in self.wins.range(..lim) {
            let we = ws + self.size;
            match self.def.oracle.as_str() {
                "uj3" | "uj3v" => {
                    for (g, a) in groups {
                        if self.def.oracle == "uj3" && a.n <= 50 {
                            continue;
                        }
                        out.insert(key(we, g), json!({"window_start":ws,"window_end":we,"asn":g.parse::<i64>().unwrap(),"n":a.n}));
                    }
                }
                "hop" => {
                    for (g, a) in groups {
                        out.insert(
                            key(we, g),
                            json!({"window_start":ws,"window_end":we,"kind":g,"n":a.n,"asns":a.asns.len(),"s":a.sum,"lo":a.min,"hi":a.max,"av":a.sum as f64 / a.n as f64}),
                        );
                    }
                }
                "distinct" => {
                    let kinds: BTreeSet<&str> = groups.keys().map(|g| g.split('|').next().unwrap()).collect();
                    for k in kinds {
                        out.insert(key(we, k), json!({"window_start":ws,"window_end":we,"kind":k}));
                    }
                }
                "ipdistinct" => {
                    for g in groups.keys() {
                        out.insert(key(we, g), json!({"window_start":ws,"window_end":we,"ip":g}));
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// Uguaglianza esatta dei JSON, tranne i decimali (`avg`): DataFusion accumula in un ordine proprio e
/// l'ultima cifra puo' differire; si accetta uno scarto relativo di 1e-12 e solo per i numeri non interi.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
        (Value::Number(x), Value::Number(y)) if x.is_f64() && y.is_f64() => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() <= 1e-12 * x.abs().max(y.abs())
        }
        _ => a == b,
    }
}

pub async fn run() {
    let spec = Spec::load(&arg("--spec").expect("--spec"));
    let dir = arg("--run-dir").expect("--run-dir");
    let data = arg("--data").expect("--data");
    let tenant = arg("--tenant").unwrap_or("acme".into());
    let addr = arg("--addr").unwrap_or("127.0.0.1:47111".into());
    let iggy = arg("--iggy").unwrap_or("127.0.0.1:47101".into());
    let wait_s: u64 = arg("--wait-s").map(|v| v.parse().unwrap()).unwrap_or(120);
    let cfg: RunCfg = serde_json::from_slice(&std::fs::read(format!("{dir}/run.json")).unwrap()).unwrap();
    let g = Gen::new(cfg.clone());
    let td = spec.tenants.iter().find(|t| t.id == tenant).expect("tenant");
    let topic = topic_of(&tenant, &cfg.stream);
    // registro dei blocchi accettati
    let mut accepted: HashMap<String, u64> = HashMap::new();
    let mut noresp = 0;
    for l in std::fs::read_to_string(format!("{dir}/blocks.jsonl")).unwrap().lines() {
        let v: Value = serde_json::from_str(l).unwrap();
        noresp += v["noresp"].as_u64().unwrap_or(0);
        if v["status"] == 200 {
            accepted.insert(v["key"].as_str().unwrap().to_string(), v["events"].as_u64().unwrap());
        }
    }
    let master = Master::load(&format!("{data}/master.key"));
    let tkey = master.tenant_key(&tenant_uid(&tenant));
    let bus = Bus::connect(&iggy, &env("IGGY_ROOT_USERNAME"), &env("IGGY_ROOT_PASSWORD")).await.unwrap();
    let mut http = crate::http::Client::new(&addr);
    let auth = format!("Bearer {}", env("TT_ADMIN_TOKEN"));
    let db = sqlx::SqlitePool::connect_with(SqliteConnectOptions::new().filename(format!("{data}/starts.db")).read_only(true))
        .await
        .unwrap();
    let stats = |b: Vec<u8>| -> Value { serde_json::from_slice(&b).unwrap_or_default() };
    let head0 = bus.head(&topic).await.unwrap();
    println!("CHECK tenant={tenant} seed={} blocchi_accettati={} testa_log={head0} senza_risposta={noresp}", cfg.seed, accepted.len());

    // attende che il processo abbia letto tutto il log e consegnato le righe in attesa
    let t0 = std::time::Instant::now();
    let mut verdict_all = false;
    let mut last_report = String::new();
    while t0.elapsed().as_secs() < wait_s {
        let Ok((200, b)) = http.request("GET", "/v1/stats", &[("Authorization", &auth)], b"").await else {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        };
        let s1 = stats(b);
        let t = &s1["tenants"][tenant.as_str()];
        let pos1: u64 = t["streams"][0]["pos"].as_u64().unwrap_or(0);
        if pos1 < head0 || t["pending"].as_u64().unwrap_or(1) != 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }
        // righe di avvio accettate dal motore
        let mut actual: HashMap<String, (String, Value, i64)> = HashMap::new();
        for r in sqlx::query("SELECT key, pipeline, body, accepted_ms FROM starts").fetch_all(&db).await.unwrap() {
            let k: String = r.get(0);
            let p: String = r.get(1);
            let body: String = r.get(2);
            actual.insert(k, (p, serde_json::from_str(&body).unwrap(), r.get(3)));
        }
        let Ok((200, b2)) = http.request("GET", "/v1/stats", &[("Authorization", &auth)], b"").await else { continue };
        let s2 = stats(b2);
        let t2 = &s2["tenants"][tenant.as_str()];
        let pos2: u64 = t2["streams"][0]["pos"].as_u64().unwrap_or(0);

        // replay del log fino a `pos2`, nell'ordine del log; primo blocco per chiave vince
        let mut os: Vec<O> = td.pipelines.iter().map(O::new).collect();
        let mut seen: HashSet<String> = HashSet::new();
        let mut in_log: HashSet<String> = HashSet::new();
        let mut dup_blocks = 0;
        let mut from = 0u64;
        while from < pos2 {
            let got = bus.read(&topic, from, 200).await.unwrap();
            if got.is_empty() {
                break;
            }
            for (off, payload) in got {
                from = off + 1;
                if off >= pos2 {
                    break;
                }
                let plain = tkey.open(&topic, &payload).expect("messaggio illeggibile nel log");
                match wire::decode(&plain, false) {
                    Msg::Signal { arrival } => os.iter_mut().for_each(|o| o.signal(arrival)),
                    Msg::Block { arrival, key, .. } => {
                        in_log.insert(key.clone());
                        if !seen.insert(key.clone()) {
                            dup_blocks += 1;
                            continue;
                        }
                        let b: u64 = key.rsplit('-').next().unwrap().parse().unwrap();
                        let (a, z) = g.block_range(b);
                        let evs: Vec<Ev> = (a..z).map(|n| g.event(n)).collect();
                        os.iter_mut().for_each(|o| o.block(off, arrival, &evs));
                    }
                }
            }
        }
        let lost_blocks: Vec<&String> = accepted.keys().filter(|k| !in_log.contains(*k)).collect();
        let lost_block_events: u64 = lost_blocks.iter().map(|k| accepted[*k]).sum();
        println!("LOG blocchi_distinti={} duplicati_nel_log={dup_blocks} accettati_assenti_dal_log={} eventi_accettati_assenti={lost_block_events} posizione_elaborata={pos2}", seen.len(), lost_blocks.len());
        let mut report = String::new();
        let mut all_ok = lost_blocks.is_empty();
        let (mut lost, mut dbl) = (0i64, 0i64);
        for o in &os {
            let exp = o.expected();
            let id = &o.def.id;
            let act: HashMap<&String, &(String, Value, i64)> = actual.iter().filter(|(_, v)| &v.0 == id).collect();
            let missing: Vec<&String> = exp.keys().filter(|k| !act.contains_key(*k)).collect();
            let extra: Vec<&&String> = act.keys().filter(|k| !exp.contains_key(**k)).collect();
            let mismatch = exp.iter().filter(|(k, v)| act.get(k).is_some_and(|a| !same(&a.1, v))).count();
            let (mut l, mut d) = (0i64, 0i64);
            if o.def.oracle == "uj3v" {
                for (k, v) in &exp {
                    let n = v["n"].as_i64().unwrap();
                    match act.get(k) {
                        Some(a) => {
                            let m = a.1["n"].as_i64().unwrap_or(0);
                            l += (n - m).max(0);
                            d += (m - n).max(0);
                        }
                        None => l += n,
                    }
                }
                for k in &extra {
                    d += act[**k].1["n"].as_i64().unwrap_or(0);
                }
            }
            lost += l;
            dbl += d;
            // contatori del processo per tenant
            let pp = t2["pipes"].as_array().and_then(|a| a.iter().find(|p| p["id"] == id.as_str())).cloned().unwrap_or_default();
            let (pl, pf) = (pp["late"].as_u64().unwrap_or(u64::MAX), pp["future"].as_u64().unwrap_or(u64::MAX));
            let windowed = o.def.oracle != "filter";
            let cnt_ok = !windowed || (pl == o.late && pf == o.fut);
            let close_delay = exp
                .iter()
                .filter_map(|(k, v)| act.get(k).zip(v["window_end"].as_i64()).map(|(a, we)| a.2 - we))
                .max();
            let ok = missing.is_empty() && extra.is_empty() && mismatch == 0 && cnt_ok && l == 0 && d == 0;
            all_ok &= ok;
            report.push_str(&format!(
                "PIPELINE {id} oracolo={} righe_attese={} righe_avvio={} mancanti={} in_piu={} diverse={mismatch} perdute={l} contate_due_volte={d} in_ritardo(oracolo/pipeline)={}/{} futuri(oracolo/pipeline)={}/{} ritardo_massimo_chiusura_ms={:?} {}\n",
                o.def.oracle, exp.len(), act.len(), missing.len(), extra.len(), o.late, pl, o.fut, pf, close_delay,
                if ok { "PASS" } else { "FAIL" }
            ));
            if let Some((k, v)) = exp.iter().find(|(k, v)| act.get(k).is_some_and(|a| !same(&a.1, v))) {
                report.push_str(&format!("  esempio diverso: {k} atteso={v} avvio={}\n", act[k].1));
            }
            if let Some(k) = missing.first() {
                report.push_str(&format!("  esempio mancante: {k}\n"));
            }
            if let Some(k) = extra.first() {
                report.push_str(&format!("  esempio in piu': {k}\n"));
            }
        }
        report.push_str(&format!(
            "TOTALE perdute={lost} contate_due_volte={dbl} avvii_doppi_assorbiti={} avvii_totali={}\n",
            s2["dupes"], s2["starts"]
        ));
        // si riprova (le chiusure da segnali possono ancora arrivare) finche' l'esito e' verde o scade il tempo
        last_report = report;
        verdict_all = all_ok;
        if all_ok {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    print!("{last_report}");
    println!("VERDETTO {}", if verdict_all { "PASS" } else { "FAIL" });
}
