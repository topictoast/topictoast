//! Pilota di carico e di prova (spike S-2, fase 2). `drive <scenario> k=v ...`; ogni scenario stampa righe
//! `Lnn ...` con i valori misurati. Gli ausiliari girano sui core `SBX_AUX_CPUS` (1-4), il pilota sui core
//! `SBX_DRV_CPUS` (8-11), come nello spike del bus. Codice da buttare.

use crate::rt::{mem_stat, scan_mem};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

pub(crate) type A = HashMap<String, String>;

pub(crate) fn arg<'a>(a: &'a A, k: &str, d: &'a str) -> &'a str {
    a.get(k).map(String::as_str).unwrap_or(d)
}

pub(crate) fn argn<T: std::str::FromStr>(a: &A, k: &str, d: T) -> T {
    a.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
}

pub(crate) fn list<'a>(a: &'a A, k: &str, d: &'a str) -> Vec<&'a str> {
    arg(a, k, d).split(',').filter(|s| !s.is_empty()).collect()
}

/// Lega il processo ai core indicati (`1-4`, `8-11`, `3`).
pub(crate) fn pin(cpus: &str) {
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    for part in cpus.split(',') {
        let (lo, hi) = part
            .split_once('-')
            .map(|(a, b)| (a.parse().unwrap(), b.parse().unwrap()))
            .unwrap_or_else(|| {
                let n: usize = part.parse().unwrap();
                (n, n)
            });
        for c in lo..=hi {
            unsafe { libc::CPU_SET(c, &mut set) };
        }
    }
    unsafe { libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) };
}

pub(crate) fn aux_cpus() -> String {
    std::env::var("SBX_AUX_CPUS").unwrap_or_else(|_| "1-4".into())
}

/// Un processo ausiliario pilotato dal pilota (stand-in del motore).
pub(crate) struct Aux {
    pub(crate) child: Child,
    pub(crate) tx: ChildStdin,
    pub(crate) rx: BufReader<ChildStdout>,
    pub(crate) pid: String,
}

impl Aux {
    /// Avvia l'ausiliario e attende `ready`; restituisce anche i microsecondi da fork a `ready`.
    pub(crate) fn spawn(
        cfg: &str,
        extra: impl FnOnce(&mut Command),
    ) -> std::io::Result<(Aux, u128)> {
        let exe = std::env::current_exe()?;
        let mut c = Command::new(exe);
        c.args(["aux", cfg])
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        extra(&mut c);
        let cpus = aux_cpus();
        unsafe {
            c.pre_exec(move || {
                pin(&cpus);
                Ok(())
            })
        };
        let t = Instant::now();
        let mut child = c.spawn()?;
        let tx = child.stdin.take().unwrap();
        let mut rx = BufReader::new(child.stdout.take().unwrap());
        let mut l = String::new();
        rx.read_line(&mut l)?;
        let us = t.elapsed().as_micros();
        if l.trim() != "ready" {
            return Err(std::io::Error::other(format!("ausiliario non pronto: {l}")));
        }
        let pid = child.id().to_string();
        Ok((Aux { child, tx, rx, pid }, us))
    }

    /// Invia una riga e legge la risposta; `None` se l'ausiliario è morto (EOF).
    pub(crate) fn try_send(&mut self, line: &str) -> Option<String> {
        writeln!(self.tx, "{line}").ok()?;
        self.tx.flush().ok()?;
        let mut r = String::new();
        match self.rx.read_line(&mut r) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(r.trim().to_string()),
        }
    }

    pub(crate) fn send(&mut self, line: &str) -> String {
        self.try_send(line).unwrap_or_else(|| "DEAD".into())
    }

    pub(crate) fn alive(&mut self) -> bool {
        self.send("ping") == "ok"
    }

    /// Ferma l'ausiliario: `quit` e attesa; il PID è quello salvato all'avvio.
    pub(crate) fn stop(mut self) {
        let _ = writeln!(self.tx, "quit");
        let _ = self.tx.flush();
        let _ = self.child.wait();
    }
}

pub(crate) fn field<'a>(r: &'a str, k: &str) -> Option<&'a str> {
    let p = format!("{k}=");
    r.split_whitespace()
        .find_map(|t| t.strip_prefix(p.as_str()))
}

pub(crate) fn fnum(r: &str, k: &str) -> u128 {
    field(r, k).and_then(|v| v.parse().ok()).unwrap_or(0)
}

pub(crate) fn fail_of(r: &str) -> String {
    r.split(" fail=").nth(1).unwrap_or("?").to_string()
}

pub(crate) fn pct(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[(((v.len() - 1) as f64) * p).round() as usize]
}

/// Riga di riepilogo in millisecondi.
pub(crate) fn summ(label: &str, us: &[u128]) -> String {
    let mut v: Vec<f64> = us.iter().map(|x| *x as f64 / 1000.0).collect();
    format!(
        "{label} n={} p50={:.3} p90={:.3} p99={:.3} max={:.3}",
        v.len(),
        pct(&mut v, 0.5),
        pct(&mut v, 0.9),
        pct(&mut v, 0.99),
        pct(&mut v, 1.0)
    )
}

pub(crate) fn fresh() -> String {
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .unwrap()
        .read_exact(&mut b)
        .unwrap();
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn mask(s: &str) -> String {
    s.bytes().map(|b| format!("{:02x}", b ^ 0x5A)).collect()
}

pub(crate) struct Env {
    pub(crate) wasm: String,
    pub(crate) cache: String,
    pub(crate) out: String,
}

impl Env {
    pub(crate) fn new(a: &A) -> Env {
        let h = std::env::var("HOME").unwrap_or_default();
        Env {
            wasm: arg(a, "wasm", &format!("{h}/ttspikesandbox/wasm")).to_string(),
            cache: arg(a, "cache", &format!("{h}/ttspikesandbox/out/cache")).to_string(),
            out: arg(a, "out", &format!("{h}/ttspikesandbox/out")).to_string(),
        }
    }

    /// Compila `<wasm>/<file>.wasm` con il sottocomando `compile` (processo a parte, come in produzione).
    pub(crate) fn compile(&self, file: &str, cfg: &str, comp: bool) -> String {
        let exe = std::env::current_exe().unwrap();
        let mut c = Command::new(exe);
        c.args([
            "compile",
            &format!("{}/{file}.wasm", self.wasm),
            &self.cache,
        ]);
        c.arg(format!("cfg={cfg}"))
            .arg(format!("kind={}", if comp { "comp" } else { "core" }));
        let cpus = aux_cpus();
        unsafe {
            c.pre_exec(move || {
                pin(&cpus);
                Ok(())
            })
        };
        let o = c.output().expect("compile");
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    /// Prepara il modulo `m` per la configurazione `cfg` e restituisce la riga `load` per l'ausiliario.
    pub(crate) fn load_line(&self, m: &str, cfg: &str) -> String {
        let comp = m == "rust-p2";
        let r = self.compile(m, cfg, comp);
        let path = field(&r, "path").unwrap_or_else(|| panic!("compilazione di {m}: {r}"));
        let sha = field(&r, "sha").unwrap();
        let mut l = format!(
            "load name={m} path={path} sha={sha} kind={}",
            if comp { "comp" } else { "core" }
        );
        if m == "javy-dynamic" {
            let p = self.compile("plugin-init", cfg, false);
            l += &format!(" plugin={}", field(&p, "path").unwrap());
        }
        l
    }

    pub(crate) fn input(&self) -> String {
        let p = format!("{}/input.json", self.out);
        std::fs::create_dir_all(&self.out).unwrap();
        std::fs::write(&p, r#"{"a":2,"b":3,"name":"topictoast"}"#).unwrap();
        p
    }
}

pub(crate) const MODS: &str = "empty,javy-static,javy-dynamic,rust-p1,rust-unknown,rust-p2";

pub fn main(args: &[String]) {
    let scen = args[0].as_str();
    let a: A = args[1..]
        .iter()
        .filter_map(|t| t.split_once('='))
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    pin(&std::env::var("SBX_DRV_CPUS").unwrap_or_else(|_| "8-11".into()));
    let env = Env::new(&a);
    match scen {
        "warm" => warm(&env, &a),
        "cold" => cold(&env, &a),
        "mem" => mem(&env, &a),
        "idle" => idle(&env, &a),
        "limits" => limits(&env, &a),
        "residue" => residue(&env, &a),
        "sentinel" => sentinel(&env, &a),
        "probe" => probe(&env, &a),
        "compile" => compile_rows(&env, &a),
        "cache" => cache(&env, &a),
        "l9" => l9(&env, &a),
        "mkbigs" => mkbigs(&env, &a),
        _ => eprintln!("scenario sconosciuto"),
    }
}

/// L1: avvio a caldo (processo ausiliario già avviato) per modulo e configurazione.
fn warm(env: &Env, a: &A) {
    let (n, w) = (argn(a, "n", 1000usize), argn(a, "warmup", 100usize));
    let inp = env.input();
    for cfg in list(a, "cfgs", "epoch") {
        for m in list(a, "mods", MODS) {
            let (mut x, _) = Aux::spawn(cfg, |_| {}).unwrap();
            let r = x.send(&env.load_line(m, cfg));
            if !r.starts_with("ok") {
                println!("L1 cfg={cfg} mod={m} LOAD {r}");
                continue;
            }
            let (mut st, mut tt, mut ti, mut rtt, mut bad) = (vec![], vec![], vec![], vec![], 0);
            for i in 0..w + n {
                let t = Instant::now();
                let r = x.send(&format!("run name={m} in={inp} limit_ms=5000"));
                let d = t.elapsed().as_micros();
                if fail_of(&r) != "none" {
                    bad += 1;
                }
                if i >= w {
                    st.push(fnum(&r, "t_start_us"));
                    ti.push(fnum(&r, "t_inst_us"));
                    tt.push(fnum(&r, "t_total_us"));
                    rtt.push(d);
                }
            }
            println!("L1 cfg={cfg} mod={m} warmup={w} failures={bad}");
            println!("   {}", summ("t_start_ms", &st));
            println!("   {}", summ("t_inst_ms", &ti));
            println!("   {}", summ("t_total_ms", &tt));
            println!("   {}", summ("rtt_ms", &rtt));
            x.stop();
        }
    }
}

/// L2: avvio a freddo di un ausiliario per tenant (fork/exec, Engine, deserialize, prima esecuzione).
fn cold(env: &Env, a: &A) {
    let n = argn(a, "n", 200usize);
    let inp = env.input();
    for cfg in list(a, "cfgs", "epoch") {
        for m in list(a, "mods", MODS) {
            let line = env.load_line(m, cfg);
            let (mut ready, mut load, mut first, mut total, mut bad) =
                (vec![], vec![], vec![], vec![], 0);
            for _ in 0..n {
                let (mut x, r_us) = Aux::spawn(cfg, |_| {}).unwrap();
                let t = Instant::now();
                let r = x.send(&line);
                let l_us = t.elapsed().as_micros();
                let t = Instant::now();
                let r2 = x.send(&format!("run name={m} in={inp} limit_ms=5000"));
                let rtt = t.elapsed().as_micros();
                if !r.starts_with("ok") || fail_of(&r2) != "none" {
                    bad += 1;
                }
                let pipe = rtt.saturating_sub(fnum(&r2, "t_total_us"));
                ready.push(r_us);
                load.push(l_us);
                first.push(r_us + l_us + pipe + fnum(&r2, "t_start_us"));
                total.push(r_us + l_us + rtt);
                x.stop();
            }
            println!("L2 cfg={cfg} mod={m} failures={bad}");
            println!("   {}", summ("spawn_to_ready_ms", &ready));
            println!("   {}", summ("load_ms", &load));
            println!("   {}", summ("cold_to_node_start_ms", &first));
            println!("   {}", summ("cold_to_node_done_ms", &total));
        }
    }
}

/// L3 e L9 (memoria): N istanze tenute vive nello stesso processo di un tenant.
fn mem(env: &Env, a: &A) {
    let inp = env.input();
    for cfg in list(a, "cfgs", "epoch") {
        for m in list(a, "mods", MODS) {
            for n in list(a, "ns", "1,10,50")
                .iter()
                .map(|s| s.parse::<usize>().unwrap())
            {
                let (mut x, _) = Aux::spawn(cfg, |_| {}).unwrap();
                x.send(&env.load_line(m, cfg));
                let b = x.send("stat");
                let mut bad = 0;
                for _ in 0..n {
                    if fail_of(&x.send(&format!("run name={m} in={inp} limit_ms=5000 hold=1")))
                        != "none"
                    {
                        bad += 1;
                    }
                }
                let s = x.send("stat");
                let d = |k: &str| (fnum(&s, k) as f64 - fnum(&b, k) as f64) / n as f64 / 1024.0;
                println!(
                    "L3 cfg={cfg} mod={m} n={n} failures={bad} base_rss_mb={:.1} base_pss_mb={:.1} rss_mb={:.1} pss_mb={:.1} hwm_mb={:.1} per_instance_rss_mb={:.2} per_instance_pss_mb={:.2}",
                    fnum(&b, "rss_kb") as f64 / 1024.0,
                    fnum(&b, "pss_kb") as f64 / 1024.0,
                    fnum(&s, "rss_kb") as f64 / 1024.0,
                    fnum(&s, "pss_kb") as f64 / 1024.0,
                    fnum(&s, "hwm_kb") as f64 / 1024.0,
                    d("rss_kb"),
                    d("pss_kb")
                );
                x.stop();
            }
        }
    }
}

/// L4: ausiliari fermi, uno per tenant, per N = 1, 5, 20.
fn idle(env: &Env, a: &A) {
    let inp = env.input();
    for cfg in list(a, "cfgs", "epoch") {
        for m in list(a, "mods", "empty,javy-static,rust-p1") {
            let mut prev_sum = 0.0;
            for n in list(a, "ns", "1,5,20")
                .iter()
                .map(|s| s.parse::<usize>().unwrap())
            {
                let line = env.load_line(m, cfg);
                let mut xs = vec![];
                for _ in 0..n {
                    let (mut x, _) = Aux::spawn(cfg, |_| {}).unwrap();
                    x.send(&line);
                    x.send(&format!("run name={m} in={inp} limit_ms=5000"));
                    xs.push(x);
                }
                std::thread::sleep(Duration::from_secs(2));
                let st: Vec<_> = xs.iter().map(|x| mem_stat(&x.pid)).collect();
                let rss: Vec<f64> = st.iter().map(|s| s.0 as f64 / 1024.0).collect();
                let pss: Vec<f64> = st.iter().map(|s| s.1 as f64 / 1024.0).collect();
                let (sr, sp) = (rss.iter().sum::<f64>(), pss.iter().sum::<f64>());
                println!(
                    "L4 cfg={cfg} mod={m} n={n} rss_max_mb={:.1} pss_max_mb={:.1} rss_sum_mb={sr:.1} pss_sum_mb={sp:.1} threads={}",
                    rss.iter().cloned().fold(0.0, f64::max),
                    pss.iter().cloned().fold(0.0, f64::max),
                    st[0].3
                );
                if n > 1 && prev_sum > 0.0 {
                    println!(
                        "   marginal_pss_mb_per_tenant(vs previous N)={:.2}",
                        (sp - prev_sum) / (n as f64 - 1.0)
                    );
                }
                if n == 1 {
                    prev_sum = sp;
                }
                for x in xs {
                    x.stop();
                }
            }
        }
    }
    // riga di confronto: ausiliario avviato e mai usato (solo Engine)
    let (x, _) = Aux::spawn("epoch", |_| {}).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    let s = mem_stat(&x.pid);
    println!(
        "L4 bare(engine only, no module) rss_mb={:.1} pss_mb={:.1}",
        s.0 as f64 / 1024.0,
        s.1 as f64 / 1024.0
    );
    x.stop();
}

/// Carico di un secondo tenant (B) in un thread: esecuzioni a ritmo costante finché `stop` non è vero.
fn bg_tenant(
    env: &Env,
    stop: &std::sync::atomic::AtomicBool,
    rate_hz: u64,
) -> (Vec<u128>, Vec<u128>) {
    let inp = env.input();
    let (mut x, _) = Aux::spawn("epoch", |_| {}).unwrap();
    x.send(&env.load_line("rust-p1", "epoch"));
    let (mut st, mut rtt) = (vec![], vec![]);
    let mut next = Instant::now();
    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        let t = Instant::now();
        let r = x.send(&format!("run name=rust-p1 in={inp} limit_ms=5000"));
        rtt.push(t.elapsed().as_micros());
        st.push(fnum(&r, "t_start_us"));
        next += Duration::from_micros(1_000_000 / rate_hz);
        if let Some(d) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(d);
        }
    }
    x.stop();
    (st, rtt)
}

fn series(
    x: &mut Aux,
    cfg_label: &str,
    label: &str,
    name: &str,
    extra: &str,
    n: usize,
    limit_ms: u64,
) -> Vec<String> {
    let mut over = vec![];
    let mut classes = std::collections::BTreeSet::new();
    let mut walls = vec![];
    for _ in 0..n {
        let t = Instant::now();
        let r = x.send(&format!("run name={name} limit_ms={limit_ms} {extra}"));
        walls.push(t.elapsed().as_micros());
        let tt = fnum(&r, "t_total_us");
        over.push((tt as f64 / 1000.0 - limit_ms as f64).max(-1e9));
        classes.insert(fail_of(&r).chars().take(110).collect::<String>());
    }
    let mut o = over.clone();
    println!(
        "{label} cfg={cfg_label} limit_ms={limit_ms} n={n} overshoot_ms p50={:.2} p99={:.2} max={:.2} min={:.2} alive={}",
        pct(&mut o, 0.5),
        pct(&mut o, 0.99),
        pct(&mut o, 1.0),
        pct(&mut o, 0.0),
        x.alive()
    );
    for c in &classes {
        println!("   outcome: {c}");
    }
    classes.into_iter().collect()
}

/// L5 e L6: limiti di CPU e di memoria con i moduli ostili; L5 anche con un secondo tenant sotto carico.
fn limits(env: &Env, a: &A) {
    let n = argn(a, "n", 30usize);
    // L5: ciclo infinito con epoca; poi copie da 16 MiB; poi la copia da 1 GiB (solo riportata)
    let (mut x, _) = Aux::spawn("epoch", |_| {}).unwrap();
    for m in ["loop", "copyloop", "copybig"] {
        println!("{}", x.send(&env.load_line(m, "epoch")));
    }
    for lim in [100u64, 1000] {
        series(&mut x, "epoch", "L5 loop", "loop", "", n, lim);
    }
    series(
        &mut x,
        "epoch",
        "L5 copyloop(16MiB copies)",
        "copyloop",
        "mem_mb=64",
        n,
        100,
    );
    series(
        &mut x,
        "epoch",
        "L5 copybig(one 1GiB copy, mem_mb=2048)",
        "copybig",
        "mem_mb=2048",
        5,
        100,
    );
    // L5 fuel: velocità del combustibile (ns per unità) con un ciclo, e precisione in tempo
    x.stop();
    let (mut f, _) = Aux::spawn("fuel", |_| {}).unwrap();
    println!("{}", f.send(&env.load_line("loop", "fuel")));
    for fuel in [100_000_000u64, 1_000_000_000] {
        let t = Instant::now();
        let r = f.send(&format!("run name=loop fuel={fuel}"));
        let us = t.elapsed().as_micros();
        println!(
            "L5 fuel={fuel} wall_ms={:.1} ns_per_fuel={:.3} outcome={}",
            us as f64 / 1000.0,
            us as f64 * 1000.0 / fuel as f64,
            fail_of(&r)
        );
    }
    f.stop();
    // L5 epoca+fuel acceso: chi scatta per primo
    let (mut b, _) = Aux::spawn("epoch+fuel", |_| {}).unwrap();
    println!("{}", b.send(&env.load_line("loop", "epoch+fuel")));
    series(
        &mut b,
        "epoch+fuel",
        "L5 loop(fuel huge)",
        "loop",
        "",
        10,
        100,
    );
    b.stop();

    // L6: memoria, tabelle, pila, memoria iniziale enorme; stesso ausiliario per tutta la serie
    let (mut x, _) = Aux::spawn("epoch", |_| {}).unwrap();
    for m in ["growmem", "table", "recurse", "bigmem"] {
        println!("{}", x.send(&env.load_line(m, "epoch")));
    }
    let b0 = x.send("stat");
    for m in ["growmem", "table", "recurse", "bigmem"] {
        series(
            &mut x,
            "epoch",
            &format!("L6 {m}"),
            m,
            "mem_mb=64",
            10,
            2000,
        );
    }
    let b1 = x.send("stat");
    println!(
        "L6 aux_rss_mb before={:.1} after={:.1} threads before={} after={}",
        fnum(&b0, "rss_kb") as f64 / 1024.0,
        fnum(&b1, "rss_kb") as f64 / 1024.0,
        fnum(&b0, "threads"),
        fnum(&b1, "threads")
    );
    x.stop();

    // L5/L6 con un altro tenant: B esegue nodi a 200/s mentre A esegue il codice ostile
    let dur = argn(a, "bgsecs", 20u64);
    for phase in ["control", "hostile"] {
        let stop = std::sync::atomic::AtomicBool::new(false);
        let (st, rtt) = std::thread::scope(|s| {
            let h = s.spawn(|| bg_tenant(env, &stop, 200));
            if phase == "hostile" {
                let (mut x, _) = Aux::spawn("epoch", |_| {}).unwrap();
                for m in ["loop", "growmem", "recurse", "copyloop"] {
                    x.send(&env.load_line(m, "epoch"));
                }
                let t = Instant::now();
                while t.elapsed() < Duration::from_secs(dur) {
                    for m in ["loop", "growmem", "recurse", "copyloop"] {
                        x.send(&format!("run name={m} limit_ms=100 mem_mb=64"));
                    }
                }
                x.stop();
            } else {
                std::thread::sleep(Duration::from_secs(dur));
            }
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            h.join().unwrap()
        });
        println!("L5/L6 other-tenant phase={phase} secs={dur}");
        println!("   {}", summ("B t_start_ms", &st));
        println!("   {}", summ("B rtt_ms", &rtt));
    }
}

fn children_of(pid: &str) -> usize {
    let mut n = 0;
    if let Ok(d) = std::fs::read_dir(format!("/proc/{pid}/task")) {
        for t in d.flatten() {
            n += std::fs::read_to_string(t.path().join("children"))
                .unwrap_or_default()
                .split_whitespace()
                .count();
        }
    }
    n
}

/// L7: nessun residuo dopo l'arresto del nodo; kill del processo; rete di sicurezza con RLIMIT_CPU.
fn residue(env: &Env, _a: &A) {
    let (mut x, _) = Aux::spawn("epoch", |_| {}).unwrap();
    for m in ["loop", "growmem", "recurse", "table"] {
        x.send(&env.load_line(m, "epoch"));
    }
    let s0 = x.send("stat");
    let mut snaps = vec![];
    for round in [10usize, 100, 100] {
        for _ in 0..round {
            for m in ["loop", "growmem", "recurse", "table"] {
                x.send(&format!("run name={m} limit_ms=20 mem_mb=64"));
            }
        }
        let s = x.send("stat");
        snaps.push(s.clone());
        println!(
            "L7 after_extra_rounds={round} {s} children={}",
            children_of(&x.pid)
        );
    }
    println!("L7 start {s0}");
    // la memoria non cresce tra il giro 2 e il giro 3 (100 giri ciascuno)
    println!(
        "L7 rss_growth_between_100_round_snapshots_kb={}",
        fnum(&snaps[2], "rss_kb") as i128 - fnum(&snaps[1], "rss_kb") as i128
    );
    // arresto del processo dal «motore»: PID salvato all'avvio
    let pid = x.pid.clone();
    let t = Instant::now();
    let _ = x.child.kill();
    let _ = x.child.wait();
    println!(
        "L7 killed pid={pid} gone={} children_left={} wait_ms={:.1}",
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        children_of(&pid),
        t.elapsed().as_secs_f64() * 1000.0
    );
    // rete di sicurezza: RLIMIT_CPU di 2 s su un ausiliario SENZA epoca né combustibile, con un ciclo infinito
    let (mut y, _) = Aux::spawn("none", |c| {
        unsafe {
            c.pre_exec(|| {
                let l = libc::rlimit {
                    rlim_cur: 2,
                    rlim_max: 3,
                };
                libc::setrlimit(libc::RLIMIT_CPU, &l);
                Ok(())
            })
        };
    })
    .unwrap();
    y.send(&env.load_line("loop", "none"));
    let t = Instant::now();
    let r = y.try_send("run name=loop");
    let sig = y
        .child
        .wait()
        .ok()
        .and_then(|s| std::os::unix::process::ExitStatusExt::signal(&s));
    println!(
        "L7 rlimit_cpu=2s response={:?} wall_ms={:.0} killed_by_signal={:?}",
        r,
        t.elapsed().as_secs_f64() * 1000.0,
        sig
    );
}

/// L8: un tenant, un processo; il segreto del tenant B non compare nella memoria del processo del tenant A.
fn sentinel(env: &Env, a: &A) {
    let (sa, sb) = (fresh(), fresh());
    for cfg in list(a, "cfgs", "epoch,pool+epoch") {
        let line = env.load_line("rust-p1", cfg);
        let (mut xa, _) = Aux::spawn(cfg, |_| {}).unwrap();
        let (mut xb, _) = Aux::spawn(cfg, |_| {}).unwrap();
        xa.send(&line);
        xb.send(&line);
        let fa = format!("{}/in-a.json", env.out);
        let fb = format!("{}/in-b.json", env.out);
        std::fs::write(&fa, format!(r#"{{"a":1,"b":2,"name":"{sa}"}}"#)).unwrap();
        std::fs::write(&fb, format!(r#"{{"a":1,"b":2,"name":"{sb}"}}"#)).unwrap();
        // i due nodi restano vivi (hold=1) mentre si legge la memoria
        let ra = xa.send(&format!("run name=rust-p1 in={fa} hold=1"));
        let rb = xb.send(&format!("run name=rust-p1 in={fb} hold=1"));
        let (ma, mb) = (mask(&sa), mask(&sb));
        let ext = |pid: &str, m: &str| match scan_mem(pid, &hex(m)) {
            Ok((h, r)) => format!("hits={h} read_mb={}", r >> 20),
            Err(e) => format!("denied({e})"),
        };
        println!(
            "L8 cfg={cfg} nodes_ran: A={} B={}",
            fail_of(&ra),
            fail_of(&rb)
        );
        println!(
            "L8 cfg={cfg} control: scan B for B's value, from B itself:  {}",
            xb.send(&format!("scan x={mb}"))
        );
        println!(
            "L8 cfg={cfg} control: scan B for B's value, from the driver: {}",
            ext(&xb.pid, &mb)
        );
        println!(
            "L8 cfg={cfg} control: scan A for A's value, from A itself:  {}",
            xa.send(&format!("scan x={ma}"))
        );
        println!(
            "L8 cfg={cfg} TEST: scan A for B's value, from A itself:   {}",
            xa.send(&format!("scan x={mb}"))
        );
        println!(
            "L8 cfg={cfg} TEST: scan A for B's value, from the driver: {}",
            ext(&xa.pid, &mb)
        );
        println!(
            "L8 cfg={cfg} TEST: scan B for A's value, from B itself:   {}",
            xb.send(&format!("scan x={ma}"))
        );
        println!(
            "L8 cfg={cfg} TEST: scan B for A's value, from the driver: {}",
            ext(&xb.pid, &ma)
        );
        // solo riportato: l'ausiliario A (non antenato di B) legge la memoria di B: lo nega Yama, non topictoast
        println!(
            "L8 cfg={cfg} REPORTED: A (sibling process) reads B's memory for B's value: {}",
            xa.send(&format!("scan x={mb} pid={}", xb.pid))
        );
        // dopo il rilascio delle istanze: residuo del valore dentro lo stesso processo (stesso tenant)
        xa.send("free");
        xb.send("free");
        println!(
            "L8 cfg={cfg} after free, A's value still in A (same tenant): {}",
            xa.send(&format!("scan x={ma}"))
        );
        println!(
            "L8 cfg={cfg} after free, B's value in A: {}",
            xa.send(&format!("scan x={mb}"))
        );
        xa.stop();
        xb.stop();
    }
}

pub(crate) fn hex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// L12: codice ostile noto. Mai si stampa un segreto: si riporta solo se compare nell'uscita.
fn probe(env: &Env, a: &A) {
    let d = format!("{}/l12", env.out);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(format!("{d}/tenantB")).unwrap();
    let secrets: Vec<(&str, String)> = ["key", "db", "cfg", "tenantB", "env_aux", "env_engine"]
        .into_iter()
        .map(|n| (n, fresh()))
        .collect();
    let sv = |n: &str| secrets.iter().find(|s| s.0 == n).unwrap().1.clone();
    std::fs::write(format!("{d}/key"), sv("key")).unwrap();
    std::fs::write(format!("{d}/db"), sv("db")).unwrap();
    std::fs::write(format!("{d}/cfg"), sv("cfg")).unwrap();
    std::fs::write(format!("{d}/tenantB/data"), sv("tenantB")).unwrap();
    // il pilota fa da «motore»: ha un segreto nel proprio ambiente
    unsafe { std::env::set_var("SBX_ENGINE_SECRET", sv("env_engine")) };
    let key_file = std::fs::File::open(format!("{d}/key")).unwrap();
    let kfd = std::os::fd::AsRawFd::as_raw_fd(&key_file);
    let (mut x, _) = Aux::spawn("epoch", |c| {
        c.env("SBX_AUX_SECRET", sv("env_aux"));
        unsafe {
            c.pre_exec(move || {
                // il descrittore del file della chiave resta aperto e ereditato dall'ausiliario
                libc::fcntl(kfd, libc::F_SETFD, 0);
                Ok(())
            })
        };
    })
    .unwrap();
    let me = std::process::id();
    let paths = [
        format!("{d}/key"),
        format!("{d}/db"),
        format!("{d}/cfg"),
        format!("{d}/tenantB/data"),
        format!("/proc/{me}/environ"),
        format!("/proc/{me}/mem"),
        format!("/proc/{me}/maps"),
        format!("/proc/{}/environ", x.pid),
        format!("/proc/{}/mem", x.pid),
        "/proc/self/environ".into(),
        "/proc/self/mem".into(),
        format!("/proc/self/fd/{kfd}"),
        format!("/proc/self/fd/{}", kfd),
        "/etc/passwd".into(),
        "/".into(),
        ".".into(),
    ];
    println!("{}", x.send(&env.load_line("probe", "epoch")));
    let o = format!("{d}/probe.out");
    let r = x.send(&format!(
        "run name=probe args={} out={o} limit_ms=2000",
        paths.join(",")
    ));
    println!("L12 probe outcome: {}", fail_of(&r));
    let text = std::fs::read_to_string(&o).unwrap_or_default();
    let mut shown = text.clone();
    for (n, v) in &secrets {
        shown = shown.replace(v.as_str(), &format!("<SENTINEL {n}>"));
    }
    for (n, v) in &secrets {
        println!(
            "L12 sentinel {n}: present_in_guest_output={}",
            text.contains(v.as_str())
        );
    }
    for l in shown.replace("\\n", "\n").lines().take(60) {
        println!(
            "   {}",
            l.chars()
                .take(200)
                .collect::<String>()
                .replace(d.as_str(), "<dir>")
                .replace(&me.to_string(), "<engine>")
        );
    }
    // moduli ostili in WAT
    for m in ["sock", "evilimport", "logflood", "errflood"] {
        let l = env.load_line(m, "epoch");
        if !l.contains("path=") {
            continue;
        }
        let lr = x.send(&l);
        if !lr.starts_with("ok") {
            println!(
                "L12 {m}: load refused: {}",
                lr.chars().take(160).collect::<String>()
            );
            continue;
        }
        let o2 = format!("{d}/{m}.out");
        let t = Instant::now();
        let r = x.send(&format!(
            "run name={m} out={o2} limit_ms=3000 logcap=1048576 outcap=1048576"
        ));
        println!(
            "L12 {m}: {} outlen={} errlen={} logs={} wall_ms={:.1} alive={}",
            fail_of(&r).chars().take(150).collect::<String>(),
            fnum(&r, "outlen"),
            fnum(&r, "errlen"),
            fnum(&r, "logs"),
            t.elapsed().as_secs_f64() * 1000.0,
            x.alive()
        );
        if m == "sock" {
            let o = std::fs::read_to_string(&o2).unwrap_or_default();
            let bytes: Vec<u8> = o
                .lines()
                .next()
                .unwrap_or("")
                .trim_start_matches("stdout=")
                .bytes()
                .take(3)
                .collect();
            println!("   errno bytes (sock_accept, sock_send, path_open dirfd=3): {bytes:?}");
        }
    }
    let _ = a;
    x.stop();
    drop(key_file);
}

fn run_file(args: &[&str], cpus: &str, timeout: Duration) -> (String, bool, i64, f64, String) {
    let exe = std::env::current_exe().unwrap();
    let mut c = Command::new(exe);
    c.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
    let cpus = cpus.to_string();
    unsafe {
        c.pre_exec(move || {
            pin(&cpus);
            Ok(())
        })
    };
    let t = Instant::now();
    let child = c.spawn().unwrap();
    let pid = child.id() as i32;
    let mut st = 0i32;
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    let mut killed = false;
    loop {
        let r = unsafe { libc::wait4(pid, &mut st, libc::WNOHANG, &mut ru) };
        if r == pid {
            break;
        }
        if t.elapsed() > timeout && !killed {
            unsafe { libc::kill(pid, libc::SIGKILL) };
            killed = true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let wall = t.elapsed().as_secs_f64() * 1000.0;
    let mut out = String::new();
    let mut ch = child;
    if let Some(mut o) = ch.stdout.take() {
        let _ = o.read_to_string(&mut out);
    }
    let status = if libc::WIFSIGNALED(st) {
        format!("signal {}", libc::WTERMSIG(st))
    } else {
        format!("exit {}", libc::WEXITSTATUS(st))
    };
    (out.trim().to_string(), killed, ru.ru_maxrss, wall, status)
}

/// Genera i moduli sintetici di dimensione data (100 KB, 1 MB, 10 MB) e quelli ostili per il compilatore.
fn mkbigs(env: &Env, _a: &A) {
    let exe = std::env::current_exe().unwrap();
    let mk = |kind: &str, n: usize, name: &str| -> u64 {
        let p = format!("{}/{name}.wasm", env.wasm);
        Command::new(&exe)
            .args(["mkbig", kind, &n.to_string(), &p])
            .status()
            .unwrap();
        std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
    };
    for (name, target) in [
        ("syn-100k", 100_000u64),
        ("syn-1m", 1_000_000),
        ("syn-10m", 10_000_000),
    ] {
        let mut n = (target / 70) as usize;
        let s = mk("funcs", n, name);
        n = (n as u64 * target / s.max(1)) as usize;
        let s2 = mk("funcs", n, name);
        println!("mkbig {name} funcs={n} bytes={s2}");
    }
    for (name, kind, n) in [
        ("hostile-nest", "nest", 20_000usize),
        ("hostile-brtable", "brtable", 300_000),
        ("hostile-straight", "straight", 300_000),
        ("hostile-ifs", "ifs", 100_000),
    ] {
        println!("mkbig {name} n={n} bytes={}", mk(kind, n, name));
    }
}

/// L10: validazione e compilazione di moduli di varie dimensioni e di moduli costruiti per rallentare il compilatore.
fn compile_rows(env: &Env, a: &A) {
    let reps = argn(a, "reps", 3usize);
    let cap = Duration::from_secs(argn(a, "cap_s", 10u64));
    let cpus = aux_cpus();
    let cfg = arg(a, "cfg", "epoch");
    let hostile = [
        "hostile-nest",
        "hostile-brtable",
        "hostile-straight",
        "hostile-ifs",
    ];
    let stop = std::sync::atomic::AtomicBool::new(false);
    let rows = |names: &[&str], label: &str, extra: &str| {
        for m in names {
            for _ in 0..reps {
                let p = format!("{}/{m}.wasm", env.wasm);
                let (o, killed, rss, wall, status) = run_file(
                    &["compile", &p, &env.cache, &format!("cfg={cfg}"), extra],
                    &cpus,
                    cap,
                );
                println!(
                    "{label} mod={m} in_bytes={} wall_ms={wall:.0} validate_ms={:.0} compile_ms={:.0} maxrss_mb={:.0} killed_at_cap={killed} status={status} result={}",
                    fnum_s(&o, "in_len"),
                    fnum_s(&o, "validate_us") as f64 / 1000.0,
                    fnum_s(&o, "compile_us") as f64 / 1000.0,
                    rss as f64 / 1024.0,
                    o.chars()
                        .take(100)
                        .collect::<String>()
                        .split(" path=")
                        .next()
                        .unwrap_or("")
                );
            }
        }
    };
    rows(
        &["javy-static", "rust-p1", "syn-100k", "syn-1m", "syn-10m"],
        "L10",
        "kind=core",
    );
    // il tetto di memoria virtuale del processo che compila (setrlimit RLIMIT_AS) sui moduli grandi
    rows(
        &["syn-1m", "hostile-straight", "hostile-ifs"],
        "L10 as=512MiB",
        "as_mb=512",
    );
    // il motore (qui un tenant B) non si ferma mentre si compilano moduli ostili
    std::thread::scope(|s| {
        let h = s.spawn(|| bg_tenant(env, &stop, 200));
        rows(&hostile, "L10 hostile", "kind=core");
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let (st, rtt) = h.join().unwrap();
        println!("L10 other-tenant during hostile compiles:");
        println!("   {}", summ("B t_start_ms", &st));
        println!("   {}", summ("B rtt_ms", &rtt));
    });
    // byte malformati: devono essere rifiutati senza fermare nulla
    let js = std::fs::read(format!("{}/javy-static.wasm", env.wasm)).unwrap();
    let bad: Vec<(&str, Vec<u8>)> = vec![
        ("truncated-javy", js[..js.len() / 2].to_vec()),
        ("zeros-1m", vec![0u8; 1 << 20]),
        ("header-only", b"\0asm\x01\0\0\0".to_vec()),
        (
            "garbage-after-header",
            [
                b"\0asm\x01\0\0\0".to_vec(),
                js[8..8 + 5000].iter().map(|b| b ^ 0xFF).collect(),
            ]
            .concat(),
        ),
    ];
    for (n, b) in bad {
        let p = format!("{}/bad-{n}.wasm", env.wasm);
        std::fs::write(&p, &b).unwrap();
        let (o, killed, rss, wall, status) = run_file(
            &["compile", &p, &env.cache, &format!("cfg={cfg}")],
            &cpus,
            cap,
        );
        println!(
            "L10 malformed {n} bytes={} wall_ms={wall:.0} maxrss_mb={:.0} killed={killed} status={status} -> {}",
            b.len(),
            rss as f64 / 1024.0,
            o.chars().take(120).collect::<String>()
        );
    }
}

fn fnum_s(o: &str, k: &str) -> u128 {
    fnum(o, k)
}

/// L11: cache nativa manomessa o incompatibile.
fn cache(env: &Env, _a: &A) {
    let inp = env.input();
    let r = env.compile("rust-p1", "epoch", false);
    let path = field(&r, "path").unwrap().to_string();
    let sha = field(&r, "sha").unwrap().to_string();
    let good = std::fs::read(&path).unwrap();
    let tdir = format!("{}/l11", env.out);
    std::fs::create_dir_all(&tdir).unwrap();
    println!("L11 cwasm bytes={} sha={}", good.len(), &sha[..16]);
    let version = b"50.0.3";
    let ver_off = good.windows(version.len()).position(|w| w == version);
    println!(
        "L11 wasmtime version string in artifact at offset {:?}",
        ver_off
    );
    let mut cases: Vec<(String, Vec<u8>)> = vec![("intact".into(), good.clone())];
    let mut flip = |name: &str, off: usize| {
        let mut b = good.clone();
        b[off] ^= 0x01;
        cases.push((format!("{name} (byte {off} flipped)"), b));
    };
    flip("header", 16);
    flip("middle", good.len() / 2);
    flip("last-byte", good.len() - 1);
    if let Some(o) = ver_off {
        let mut b = good.clone();
        b[o + 5] = b'2';
        cases.push(("version string 50.0.3 -> 50.0.2".into(), b));
    }
    cases.push(("truncated to half".into(), good[..good.len() / 2].to_vec()));
    cases.push((
        "plain wasm bytes in place of cwasm".into(),
        std::fs::read(format!("{}/rust-p1.wasm", env.wasm)).unwrap(),
    ));
    for (name, bytes) in &cases {
        let p = format!("{tdir}/case.cwasm");
        std::fs::write(&p, bytes).unwrap();
        for (mode, shaarg) in [("verified", sha.as_str()), ("UNVERIFIED", "-")] {
            let (mut x, _) = Aux::spawn("epoch", |_| {}).unwrap();
            let l = x.send(&format!("load name=n path={p} sha={shaarg} kind=core"));
            let mut s = format!(
                "L11 {name} [{mode}] load: {}",
                l.chars().take(110).collect::<String>()
            );
            if l.starts_with("ok") {
                let o = format!("{tdir}/out.txt");
                let r = x.send(&format!("run name=n in={inp} out={o} limit_ms=2000"));
                let out = std::fs::read_to_string(&o).unwrap_or_default();
                s += &format!(
                    " | run: {} stdout_ok={}",
                    fail_of(&r).chars().take(80).collect::<String>(),
                    out.contains(r#"{"greeting":"hello topictoast","sum":5}"#)
                );
            }
            println!("{s}");
            x.stop();
        }
    }
    // configurazione diversa: artefatto compilato con `epoch` caricato da un motore `fuel` o `pool+epoch`
    for cfg in ["fuel", "pool+epoch", "epoch+nocow"] {
        let (mut x, _) = Aux::spawn(cfg, |_| {}).unwrap();
        let l = x.send(&format!("load name=n path={path} sha={sha} kind=core"));
        println!(
            "L11 artifact(epoch) loaded by engine cfg={cfg}: {}",
            l.chars().take(140).collect::<String>()
        );
        x.stop();
    }
    // un cwasm caricato dal tenant al posto del modulo: il compilatore lo rifiuta
    let up = format!("{tdir}/upload.wasm");
    std::fs::write(&up, &good).unwrap();
    let (o, _, _, _, _) = run_file(
        &["compile", &up, &format!("{tdir}/cache2"), "cfg=epoch"],
        &aux_cpus(),
        Duration::from_secs(10),
    );
    println!(
        "L11 cwasm uploaded as if it were a module -> compile: {}",
        o.chars().take(140).collect::<String>()
    );
}

/// L9: lo stesso nodo in sei forme: funziona (uscita esatta, tre righe di log, una scrittura sullo stream).
fn l9(env: &Env, a: &A) {
    let inp = env.input();
    let cfg = arg(a, "cfg", "epoch");
    for m in list(a, "mods", MODS) {
        let (mut x, _) = Aux::spawn(cfg, |_| {}).unwrap();
        let l = env.load_line(m, cfg);
        let size = std::fs::metadata(format!("{}/{m}.wasm", env.wasm))
            .map(|m| m.len())
            .unwrap_or(0);
        let cw = std::fs::metadata(field(&l, "path").unwrap())
            .map(|m| m.len())
            .unwrap_or(0);
        x.send(&l);
        let o = format!("{}/l9-{m}.txt", env.out);
        let r = x.send(&format!("run name={m} in={inp} out={o} limit_ms=5000"));
        let t = std::fs::read_to_string(&o).unwrap_or_default();
        let want = r#"{"greeting":"hello topictoast","sum":5}"#;
        let out_ok =
            t.contains(&format!("stdout={want}\n")) || t.contains(&format!("host_output={want}\n"));
        let logs: Vec<&str> = t.lines().filter(|l| l.starts_with("log ")).collect();
        let emits: Vec<&str> = t.lines().filter(|l| l.starts_with("emit ")).collect();
        let levels_ok = logs.len() == 3
            && logs[0].starts_with("log 0 ")
            && logs[1].starts_with("log 1 ")
            && logs[2].starts_with("log 2 ");
        println!(
            "L9 mod={m} wasm_bytes={size} cwasm_bytes={cw} outcome={} output_exact={out_ok} log_lines={} levels_0_1_2={levels_ok} emits={:?}",
            fail_of(&r).chars().take(100).collect::<String>(),
            logs.len(),
            emits
        );
        x.stop();
    }
}
