//! Sentinella (prototipo): un processo di sola lettura che segue i topic `state-*` di Iggy con un gruppo di consumatori
//! (`sentinel`, posizione salvata dal server) e, dalle sole intestazioni utente in chiaro degli eventi, vede se un workflow
//! avanza, è bloccato o ha errori e se il motore è fermo. Non ha chiavi: ingressi e uscite restano sigillati per tenant.
//! Resta fuori dal percorso caldo: non scrive sul bus né sul database e non tocca la lettura né la conservazione del motore
//! (che restano sulla posizione del proiettore).
//!
//! Intestazione `m` (35 byte grezzi, scritta dal motore con `meta=1`): tipo (1 Accepted, 2 Timer, 3 Ready, 4 Done), esecuzione
//! (16), passo (4), tentativo (4), tipo del timer (1), scadenza in µs (8), ultimo passo (1).
//!
//! Costanti fissate nelle regole di lettura (`risultati/iggy-sqlite-sent-regola-4-ms.txt`): scoperta dei topic ogni 1 s,
//! tolleranza di 2 s dopo la scadenza di un tentativo, 2 s di silenzio con un'esecuzione aperta.
// ponytail: le scadenze si cercano scorrendo tutti i timer a ogni passo (100 ms): fino a qualche migliaio va bene; con
// centinaia di migliaia di timer serve un indice per scadenza (come `Engine::timers`). Ceiling dichiarato, non misurato.

use crate::crypto::{Uid, hex};
use crate::ev::Ev;
use hdrhistogram::Histogram;
use std::collections::{HashMap, HashSet};

pub const GRACE_US: u64 = 2_000_000;
pub const SILENT_US: u64 = 2_000_000;

/// Intestazione di un evento di stato letta dal sentinella.
#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    pub kind: u8,
    pub exec: Uid,
    pub step: u32,
    pub attempt: u32,
    pub tkind: u8,
    pub due: u64,
    pub last: bool,
}

/// Intestazione in chiaro di un evento (la scrive il motore con `meta=1`).
pub fn encode(e: &Ev) -> [u8; crate::bus::META_LEN] {
    let m = match e {
        Ev::Accepted { exec, .. } => Meta {
            kind: 1,
            exec: *exec,
            step: 0,
            attempt: 0,
            tkind: 0,
            due: 0,
            last: false,
        },
        Ev::Timer {
            exec,
            step,
            kind,
            attempt,
            due_us,
            ..
        } => Meta {
            kind: 2,
            exec: *exec,
            step: *step,
            attempt: *attempt,
            tkind: *kind,
            due: *due_us,
            last: false,
        },
        Ev::Ready {
            exec, step, last, ..
        } => Meta {
            kind: 3,
            exec: *exec,
            step: *step,
            attempt: 0,
            tkind: 0,
            due: 0,
            last: *last,
        },
        Ev::Done {
            exec, step, last, ..
        } => Meta {
            kind: 4,
            exec: *exec,
            step: *step,
            attempt: 0,
            tkind: 0,
            due: 0,
            last: *last,
        },
    };
    let mut v = [0u8; crate::bus::META_LEN];
    v[0] = m.kind;
    v[1..17].copy_from_slice(&m.exec);
    v[17..21].copy_from_slice(&m.step.to_le_bytes());
    v[21..25].copy_from_slice(&m.attempt.to_le_bytes());
    v[25] = m.tkind;
    v[26..34].copy_from_slice(&m.due.to_le_bytes());
    v[34] = m.last as u8;
    v
}

pub fn decode(b: &[u8]) -> Option<Meta> {
    (b.len() == crate::bus::META_LEN).then(|| Meta {
        kind: b[0],
        exec: b[1..17].try_into().unwrap(),
        step: u32::from_le_bytes(b[17..21].try_into().unwrap()),
        attempt: u32::from_le_bytes(b[21..25].try_into().unwrap()),
        tkind: b[25],
        due: u64::from_le_bytes(b[26..34].try_into().unwrap()),
        last: b[34] == 1,
    })
}

/// Stato per esecuzione ricavato dalle sole intestazioni, e logica degli allarmi (pura: gli istanti si passano da fuori).
pub struct State {
    /// Esecuzioni aperte: `Accepted` senza il `Done` dell'ultimo passo.
    open: HashSet<Uid>,
    /// Ultimo timer per (esecuzione, passo): tipo, tentativo, scadenza.
    timers: HashMap<(Uid, u32), (u8, u32, u64)>,
    alarmed: HashSet<(Uid, u32, u32, u8)>,
    last_event: u64,
    silent: bool,
    /// Allarmi per tipo: `overdue`, `retry`, `engine-silent`; ripartenze del motore viste.
    pub counts: [u64; 3],
    pub recovered: u64,
    /// Tempo dall'append del timer di ritentativo (ora del server di Iggy) all'allarme, in µs.
    pub retry_lat: Histogram<u64>,
    /// Scadenze di tentativo risolte (timer nuovo o `Done`) dopo la loro scadenza: quante, e il margine che restava prima della
    /// fine della tolleranza (µs). Sono le scadenze in cui il motore ha agito e il sentinella non ha dato l'allarme.
    pub resolved_after_due: u64,
    pub margin: Histogram<u64>,
}

impl State {
    pub fn new(now: u64) -> State {
        State {
            open: HashSet::new(),
            timers: HashMap::new(),
            alarmed: HashSet::new(),
            last_event: now,
            silent: false,
            counts: [0; 3],
            recovered: 0,
            retry_lat: crate::hist(),
            resolved_after_due: 0,
            margin: crate::hist(),
        }
    }

    /// Un timer di scadenza (tipo 0) sostituito o chiuso dopo la sua scadenza: il motore ha agito entro la tolleranza o oltre.
    fn resolved(&mut self, prev: Option<(u8, u32, u64)>, now: u64) {
        if let Some((0, _, due)) = prev
            && now > due
        {
            self.resolved_after_due += 1;
            self.margin
                .record((due + GRACE_US).saturating_sub(now).max(1))
                .ok();
        }
    }

    /// Un evento letto: `now` è l'istante di lettura, `ts` l'ora di append data da Iggy. Torna le righe da scrivere.
    pub fn event(&mut self, m: &Meta, now: u64, ts: u64) -> Vec<String> {
        let mut out = vec![];
        self.last_event = now;
        if std::mem::take(&mut self.silent) {
            self.recovered += 1;
            out.push("RECOVERED engine".into());
        }
        match m.kind {
            1 => {
                self.open.insert(m.exec);
            }
            2 => {
                let prev = self
                    .timers
                    .insert((m.exec, m.step), (m.tkind, m.attempt, m.due));
                self.resolved(prev, now);
                // tipo 1: un tentativo è fallito e un ritentativo è in programma (il tentativo mostrato è quello fallito)
                if m.tkind == 1 && self.alarmed.insert((m.exec, m.step, m.attempt, 1)) {
                    self.counts[1] += 1;
                    self.retry_lat.record(now.saturating_sub(ts).max(1)).ok();
                    out.push(format!(
                        "ALARM retry {} {} {}",
                        hex(&m.exec),
                        m.step,
                        m.attempt.saturating_sub(1)
                    ));
                }
            }
            4 => {
                let prev = self.timers.remove(&(m.exec, m.step));
                self.resolved(prev, now);
                if m.last {
                    self.open.remove(&m.exec);
                }
            }
            _ => {}
        }
        out
    }

    /// Controllo periodico: scadenze rimaste senza risposta e silenzio del motore.
    pub fn tick(&mut self, now: u64) -> Vec<String> {
        let mut out = vec![];
        for ((exec, step), (tk, att, due)) in &self.timers {
            if *tk == 0 && *due + GRACE_US < now && self.alarmed.insert((*exec, *step, *att, 0)) {
                self.counts[0] += 1;
                out.push(format!("ALARM overdue {} {} {}", hex(exec), step, att));
            }
        }
        if !self.silent && !self.open.is_empty() && now.saturating_sub(self.last_event) > SILENT_US
        {
            self.silent = true;
            self.counts[2] += 1;
            out.push("ALARM engine-silent".into());
        }
        out
    }
}

/// Per la scena `newtenant`: crea un topic `state-*` nuovo e ci scrive un evento `Ready` con la sola intestazione; stampa gli istanti.
pub async fn new_topic(bus_kind: &str) {
    let bus = crate::bus::Bus::connect(bus_kind).await.unwrap();
    let t: Uid = *uuid::Uuid::new_v4().as_bytes();
    let topic = crate::engine::topic_of(&t);
    let t0 = crate::now_us();
    bus.ensure_topic(&topic).await.unwrap();
    let t1 = crate::now_us();
    let mut v = vec![crate::bus::META_MAGIC];
    v.extend_from_slice(&encode(&Ev::Ready {
        exec: t,
        tenant: t,
        step: 0,
        last: false,
        ts: t1,
    }));
    v.extend_from_slice(b"x");
    bus.append(&topic, &[bytes::Bytes::from(v)]).await.unwrap();
    println!(
        "NEWTOPIC {topic} create_us={t0} created_us={t1} append_done_us={}",
        crate::now_us()
    );
}

#[cfg(not(feature = "old"))]
pub use run::run;

#[cfg(not(feature = "old"))]
mod run {
    use super::*;
    use crate::bus::{Bus, STREAM};
    use crate::{hist, now_us};
    use iggy::prelude::*;
    use std::io::Write;
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Default)]
    struct Stats {
        events: u64,
        gaps: u64,
        gap_events: u64,
        hdr_bytes: u32,
        topics: u64,
    }

    struct Shared {
        st: Mutex<State>,
        stats: Mutex<Stats>,
        lag: Mutex<Histogram<u64>>,
        delay: Mutex<Histogram<u64>>,
        log: Mutex<std::fs::File>,
    }

    impl Shared {
        fn lines(&self, now: u64, v: Vec<String>) {
            let mut f = self.log.lock().unwrap();
            for l in v {
                let _ = writeln!(f, "{now} {l}");
            }
        }
    }

    fn cpu_s() -> f64 {
        let mut t = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) };
        t.tv_sec as f64 + t.tv_nsec as f64 / 1e9
    }

    /// Segue un topic con il gruppo `sentinel`: la posizione la salva il server a ogni lettura (`CommitPolicy::OnRead`).
    async fn follow(c: Arc<IggyClient>, topic: String, sh: Arc<Shared>) {
        let (s, t) = (
            Identifier::named(STREAM).unwrap(),
            Identifier::named(&topic).unwrap(),
        );
        let g = Identifier::named("sentinel").unwrap();
        let _ = c.create_consumer_group(&s, &t, "sentinel").await;
        let _ = c.join_consumer_group(&s, &t, &g).await;
        sh.lines(now_us(), vec![format!("FOLLOW {topic}")]);
        let key = HeaderKey::from_str("m").unwrap();
        let (mut expected, mut first) = (None::<u64>, true);
        loop {
            let r = c
                .poll_messages(
                    &s,
                    &t,
                    0,
                    &Consumer::group(g.clone()),
                    &PollingStrategy::next(),
                    PollOptions {
                        count: 1000,
                        commit: CommitPolicy::OnRead,
                        ..PollOptions::default()
                    },
                )
                .await;
            let r = match r {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("SENTINEL_ERR {topic}: {e}");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let _ = c.join_consumer_group(&s, &t, &g).await;
                    continue;
                }
            };
            if r.messages.is_empty() {
                tokio::time::sleep(Duration::from_millis(20)).await;
                continue;
            }
            let now = now_us();
            let (last, last_ts) = {
                let h = &r.messages.last().unwrap().header;
                (h.offset, h.timestamp)
            };
            // ritardo di lettura: ora di lettura meno ora di append dell'ultimo messaggio del gruppo (criterio di 1 s delle regole)
            sh.delay
                .lock()
                .unwrap()
                .record(now.saturating_sub(last_ts).max(1))
                .ok();
            sh.lag
                .lock()
                .unwrap()
                .record(r.current_offset.saturating_sub(last).max(1))
                .ok();
            let mut lines = vec![];
            {
                let (mut st, mut stats) = (sh.st.lock().unwrap(), sh.stats.lock().unwrap());
                for m in &r.messages {
                    let off = m.header.offset;
                    if let Some(e) = expected.filter(|e| *e != off) {
                        stats.gaps += 1;
                        stats.gap_events += off.saturating_sub(e);
                    }
                    expected = Some(off + 1);
                    stats.events += 1;
                    stats.hdr_bytes = m.header.user_headers_length;
                    if first {
                        first = false;
                        lines.push(format!("FIRST {topic} offset={off}"));
                    }
                    let meta =
                        m.user_headers_map().ok().flatten().and_then(|h| {
                            h.get(&key).and_then(|v| v.as_raw().ok().and_then(decode))
                        });
                    if let Some(meta) = meta {
                        lines.extend(st.event(&meta, now, m.header.timestamp));
                    }
                }
            }
            sh.lines(now, lines);
        }
    }

    /// `sentinel dir=out secs=N`: gira fino a `secs` secondi o fino alla comparsa di `out/sentinel.stop`, poi stampa le statistiche.
    pub async fn run(dir: &str, secs: u64) {
        let c = match Bus::connect("iggy").await.unwrap() {
            Bus::Iggy(c) => c,
            Bus::Valkey(_) => unreachable!(),
        };
        let start = now_us();
        let sh = Arc::new(Shared {
            st: Mutex::new(State::new(start)),
            stats: Mutex::new(Stats::default()),
            lag: Mutex::new(hist()),
            delay: Mutex::new(hist()),
            log: Mutex::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(format!("{dir}/sentinel.log"))
                    .unwrap(),
            ),
        });
        let (cpu0, mut known) = (cpu_s(), HashSet::<String>::new());
        let s = Identifier::named(STREAM).unwrap();
        let end = std::time::Instant::now() + Duration::from_secs(secs);
        let (mut since_tick, stop) = (0u32, format!("{dir}/sentinel.stop"));
        println!("sentinel pronto (now_us={start})");
        while std::time::Instant::now() < end && !std::path::Path::new(&stop).exists() {
            // scoperta dei topic ogni 1 s (10 passi da 100 ms), controllo degli allarmi a ogni passo
            if since_tick % 10 == 0 {
                for t in c.get_topics(&s).await.unwrap_or_default() {
                    if t.name.starts_with("state-") && known.insert(t.name.clone()) {
                        sh.stats.lock().unwrap().topics += 1;
                        tokio::spawn(follow(c.clone(), t.name.clone(), sh.clone()));
                    }
                }
            }
            since_tick += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
            let now = now_us();
            let v = sh.st.lock().unwrap().tick(now);
            sh.lines(now, v);
        }
        let (st, stats, lag) = (
            sh.st.lock().unwrap(),
            sh.stats.lock().unwrap(),
            sh.lag.lock().unwrap(),
        );
        println!(
            "sentinel_stats events={} topics={} lag_events p50={} p99={} max={} gaps={} gap_events={} alarms overdue={} retry={} engine_silent={} recovered={} header_bytes={} cpu_s={:.2}",
            stats.events,
            stats.topics,
            lag.value_at_quantile(0.5),
            lag.value_at_quantile(0.99),
            lag.max(),
            stats.gaps,
            stats.gap_events,
            st.counts[0],
            st.counts[1],
            st.counts[2],
            st.recovered,
            stats.hdr_bytes,
            cpu_s() - cpu0
        );
        println!(
            "sentinel_read_delay {}",
            crate::fmt(&sh.delay.lock().unwrap())
        );
        println!("sentinel_retry_latency {}", crate::fmt(&st.retry_lat));
        println!(
            "sentinel_deadlines_resolved_after_due={} margin_lt_1s={} margin_to_grace_end {}",
            st.resolved_after_due,
            st.margin.count_between(0, 1_000_000),
            crate::fmt(&st.margin)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::TENANTS;

    const E: Uid = [3; 16];

    fn meta(kind: u8, tkind: u8, attempt: u32, due: u64, last: bool) -> Meta {
        Meta {
            kind,
            exec: E,
            step: 0,
            attempt,
            tkind,
            due,
            last,
        }
    }

    #[test]
    fn header_roundtrip() {
        let ev = Ev::Timer {
            exec: E,
            tenant: TENANTS[0],
            step: 7,
            kind: 1,
            attempt: 2,
            due_us: 99,
            ts: 5,
        };
        let m = decode(&encode(&ev)).unwrap();
        assert_eq!(
            (m.kind, m.step, m.attempt, m.tkind, m.due),
            (2, 7, 2, 1, 99)
        );
        assert_eq!(encode(&ev).len(), crate::bus::META_LEN);
    }

    /// Scadenza senza risposta: un solo allarme dopo scadenza + 2 s; un `Done` o un timer nuovo lo evita.
    #[test]
    fn overdue_once_and_only_without_answer() {
        let mut s = State::new(0);
        s.event(&meta(1, 0, 0, 0, false), 2_000_000, 1);
        s.event(&meta(2, 0, 0, 1_000_000, false), 2_000_100, 1);
        assert!(s.tick(2_900_000).is_empty());
        assert_eq!(s.tick(3_100_000).len(), 1);
        assert!(
            s.tick(9_000_000).iter().all(|l| !l.contains("overdue")),
            "un allarme per tentativo"
        );
        // un timer nuovo dello stesso passo (l'engine ha agito) e poi un Done: nessun nuovo allarme
        s.event(&meta(2, 0, 1, 20_000_000, false), 9_000_100, 1);
        s.event(&meta(4, 0, 0, 0, true), 9_000_200, 1);
        assert!(s.tick(30_000_000).iter().all(|l| !l.contains("overdue")));
        assert_eq!(s.counts[0], 1);
    }

    /// Errore: il timer di ritentativo (tipo 1) dà un allarme con il tentativo fallito, una volta sola.
    #[test]
    fn retry_alarm_names_the_failed_attempt() {
        let mut s = State::new(0);
        let out = s.event(&meta(2, 1, 1, 5_000_000, false), 100, 40);
        assert_eq!(out, vec![format!("ALARM retry {} 0 0", hex(&E))]);
        assert!(
            s.event(&meta(2, 1, 1, 5_000_000, false), 200, 41)
                .is_empty()
        );
        assert_eq!(s.counts[1], 1);
    }

    /// Silenzio con un'esecuzione aperta: allarme, poi `RECOVERED` al primo evento; senza esecuzioni aperte nessun allarme.
    #[test]
    fn engine_silent_and_recovered() {
        let mut s = State::new(0);
        assert!(
            s.tick(10_000_000).is_empty(),
            "niente aperto, niente allarme"
        );
        s.event(&meta(1, 0, 0, 0, false), 10_000_000, 1);
        assert!(s.tick(11_900_000).is_empty());
        assert_eq!(s.tick(12_100_000), vec!["ALARM engine-silent".to_string()]);
        assert!(s.tick(13_000_000).is_empty(), "una sola volta");
        assert_eq!(
            s.event(&meta(3, 0, 0, 0, false), 14_000_000, 1),
            vec!["RECOVERED engine".to_string()]
        );
        // chiusa l'esecuzione (Done dell'ultimo passo) il silenzio non è un allarme
        s.event(&meta(4, 0, 0, 0, true), 14_000_100, 1);
        assert!(s.tick(30_000_000).is_empty());
    }
}
