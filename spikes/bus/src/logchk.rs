//! Controlli del log dei task (blocco D): `logcheck` rilegge TUTTI i pezzi dell'archivio decifrandoli e li confronta con la
//! verità di base del worker, cerca il segreto sintetico negli eventi del bus e nei pezzi; `logtail` misura la visibilità
//! (dalla riga scritta nel worker al pezzo leggibile e decifrato nell'archivio). Solo conteggi: il segreto non si stampa mai.

use crate::crypto::{Keys, Uid, hex};
use crate::engine::{TENANTS, topic_of};
use crate::tasklog::{chunk_addr, decode, log_topic};
use crate::{arg, fmt, hist, now_us};
use std::collections::{BTreeMap, HashMap, HashSet};

fn count(hay: &[u8], needle: &[u8]) -> u64 {
    if needle.is_empty() || hay.len() < needle.len() {
        return 0;
    }
    hay.windows(needle.len()).filter(|w| *w == needle).count() as u64
}

/// Pezzi dell'archivio: nome, tenant e contenuto decifrato (ignora i file temporanei, che iniziano con un punto).
fn chunks(keys: &Keys, dir: &str) -> Vec<(String, Uid, crate::tasklog::Decoded)> {
    let mut v = vec![];
    for t in &TENANTS {
        let d = format!("{dir}/logs/{}", hex(t));
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let raw = std::fs::read(e.path()).unwrap();
            let (_, _, plain) = keys.open(&chunk_addr(t, &name), &raw);
            v.push((name, *t, decode(&plain)));
        }
    }
    v
}

pub async fn check(dir: &str, bus_kind: &str) {
    let keys = Keys::load(&format!("{dir}/master.key"));
    let marker = std::env::var("SPIKE_SECRET")
        .unwrap_or_default()
        .into_bytes();
    let (max_lines, max_bytes): (u64, usize) = (
        arg("log_lines", "100").parse().unwrap(),
        arg("log_bytes", "16384").parse().unwrap(),
    );
    // verità di base del worker: esecuzione, passo, tentativo -> righe scritte
    let mut written: HashMap<(String, u32, u32), u64> = HashMap::new();
    for l in std::fs::read_to_string(arg("logw", ""))
        .unwrap_or_default()
        .lines()
    {
        let f: Vec<&str> = l.split(' ').collect();
        written.insert(
            (f[0].into(), f[1].parse().unwrap(), f[2].parse().unwrap()),
            f[3].parse().unwrap(),
        );
    }
    // pezzi per tentativo
    type Rng = (u64, u64, u32, Vec<(u64, usize)>);
    let mut by: BTreeMap<(String, u32, u32), Vec<Rng>> = BTreeMap::new();
    let (mut n_chunks, mut marker_chunks, mut placeholder_chunks) = (0u64, 0u64, 0u64);
    let mut hdr_bad = 0u64;
    let all = chunks(&keys, dir);
    let on_disk: HashSet<String> = all.iter().map(|c| c.0.clone()).collect();
    for (name, _, d) in &all {
        n_chunks += 1;
        let f: Vec<&str> = name.split('-').collect();
        let key = (
            f[0].to_string(),
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
        );
        for (_, t) in &d.lines {
            marker_chunks += count(t, &marker);
            placeholder_chunks += count(t, crate::tasklog::PLACEHOLDER);
        }
        if d.discarded as u64 != d.last - d.first + 1 - d.lines.len() as u64 {
            hdr_bad += 1;
        }
        by.entry(key).or_default().push((
            d.first,
            d.last,
            d.discarded,
            d.lines.iter().map(|(s, t)| (*s, t.len())).collect(),
        ));
    }
    let (mut missing, mut overlap, mut beyond, mut dup_seq, mut over_cap) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut stored, mut discarded, mut w_total) = (0u64, 0u64, 0u64);
    for (k, n) in &written {
        w_total += n;
        if !by.contains_key(k) {
            missing += n;
        }
    }
    for (k, rs) in by {
        // copertura dei numeri 1..=n da parte degli intervalli dei pezzi: ognuno esattamente una volta
        let n = written.get(&k).copied().unwrap_or(0);
        let mut cover = vec![0u32; n as usize + 1];
        let (mut seen, mut bytes, mut st) = (HashSet::new(), 0usize, 0u64);
        for (first, last, disc, lines) in &rs {
            for s in *first..=*last {
                match cover.get_mut(s as usize) {
                    Some(c) => *c += 1,
                    None => beyond += 1,
                }
            }
            discarded += *disc as u64;
            for (s, l) in lines {
                dup_seq += !seen.insert(*s) as u64;
                bytes += l;
                st += 1;
            }
        }
        missing += cover[1..].iter().filter(|c| **c == 0).count() as u64;
        overlap += cover[1..].iter().filter(|c| **c > 1).count() as u64;
        stored += st;
        over_cap += (st > max_lines || bytes > max_bytes) as u64;
    }
    // bus: eventi di stato e riferimenti ai pezzi; il segreto non deve comparire in nessuno, dopo la decifratura
    let bus = crate::bus::Bus::connect(bus_kind).await.unwrap();
    let (mut ev_n, mut marker_bus, mut incomplete) = (0u64, 0u64, 0u64);
    let (mut refs, mut distinct, mut refs_no_chunk) = (0u64, HashSet::new(), 0u64);
    for t in &TENANTS {
        for topic in [topic_of(t), log_topic(t)] {
            let mut p = 0;
            loop {
                let r = bus.read(&topic, p, 1000).await.unwrap_or_default();
                let Some(first) = r.first().map(|x| x.0) else {
                    break;
                };
                if p == 0 && first != 0 {
                    incomplete += 1;
                }
                for (pp, raw) in r {
                    let plain = keys.open(&format!("bus/{topic}"), &raw).2;
                    ev_n += 1;
                    marker_bus += count(&plain, &marker);
                    if topic.starts_with("log-") {
                        refs += 1;
                        // tenant 16, esecuzione 16, passo 4, tentativo 4, primo 8
                        let ex: Uid = plain[16..32].try_into().unwrap();
                        let st = u32::from_le_bytes(plain[32..36].try_into().unwrap());
                        let at = u32::from_le_bytes(plain[36..40].try_into().unwrap());
                        let fi = u64::from_le_bytes(plain[40..48].try_into().unwrap());
                        let name = crate::tasklog::chunk_name(&ex, st, at, fi);
                        if distinct.insert(name.clone()) && !on_disk.contains(&name) {
                            refs_no_chunk += 1;
                        }
                    }
                    p = pp + 1;
                }
            }
        }
    }
    println!(
        "logcheck: attempts_written={} lines_written={w_total} chunks={n_chunks} lines_stored={stored} lines_discarded={discarded} stored_plus_discarded={} missing={missing} overlap={overlap} beyond_written={beyond} stored_twice={dup_seq} header_inconsistent={hdr_bad} attempts_over_cap={over_cap}",
        written.len(),
        stored + discarded
    );
    println!(
        "logcheck: bus_events_scanned={ev_n} marker_in_bus_events={marker_bus} marker_in_chunks={marker_chunks} placeholder_in_chunks={placeholder_chunks} refs={refs} refs_distinct={} refs_without_chunk={refs_no_chunk} bus_scan_incomplete={incomplete}",
        distinct.len()
    );
    // verità di base: ogni numero scritto compare una sola volta, salvato o in uno scarto contato
    let ok = missing == 0
        && overlap == 0
        && beyond == 0
        && dup_seq == 0
        && hdr_bad == 0
        && stored + discarded == w_total;
    println!("logcheck {}", if ok { "OK" } else { "FALLITA" });
    if !ok {
        std::process::exit(1);
    }
}

/// Visibilità: segue la creazione dei pezzi con inotify per `secs` secondi; per ogni riga, ora di lettura meno ora scritta nel
/// worker (le prime 16 cifre della riga). Un pezzo si crea con `hard_link` già completo: la lettura non vede pezzi a metà.
pub fn tail(dir: &str, secs: u64) {
    let keys = Keys::load(&format!("{dir}/master.key"));
    let fd = unsafe { libc::inotify_init1(0) };
    let mut wds: HashMap<i32, Uid> = HashMap::new();
    for t in &TENANTS {
        let p = std::ffi::CString::new(format!("{dir}/logs/{}", hex(t))).unwrap();
        let wd = unsafe { libc::inotify_add_watch(fd, p.as_ptr(), libc::IN_CREATE) };
        assert!(wd >= 0, "inotify_add_watch");
        wds.insert(wd, *t);
    }
    println!("logtail pronto (now_us={})", now_us());
    let (mut h, mut n_chunks, mut over1s) = (hist(), 0u64, 0u64);
    let end = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut buf = vec![0u8; 1 << 16];
    while std::time::Instant::now() < end {
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut pfd, 1, 200) } <= 0 {
            continue;
        }
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        let (mut o, n) = (0usize, n.max(0) as usize);
        while o + 16 <= n {
            let wd = i32::from_ne_bytes(buf[o..o + 4].try_into().unwrap());
            let len = u32::from_ne_bytes(buf[o + 12..o + 16].try_into().unwrap()) as usize;
            let name = String::from_utf8_lossy(&buf[o + 16..o + 16 + len])
                .trim_end_matches('\0')
                .to_string();
            o += 16 + len;
            let (Some(t), false) = (wds.get(&wd), name.starts_with('.')) else {
                continue;
            };
            let Ok(raw) = std::fs::read(format!("{dir}/logs/{}/{name}", hex(t))) else {
                continue;
            };
            let (_, _, plain) = keys.open(&chunk_addr(t, &name), &raw);
            let now = now_us();
            n_chunks += 1;
            for (_, text) in decode(&plain).lines {
                let w: u64 = String::from_utf8_lossy(&text[..16]).parse().unwrap_or(now);
                let d = now.saturating_sub(w).max(1);
                over1s += (d > 1_000_000) as u64;
                h.record(d).ok();
            }
        }
    }
    println!(
        "log_visibility chunks={n_chunks} {} over1s={over1s}",
        fmt(&h)
    );
}
