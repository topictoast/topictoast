//! Pilota della fase 3 (livello forte): avvia microVM Firecracker come processi figli, parla con la loro API
//! (HTTP su socket Unix, scritto a mano) e con l'agente del guest sul vsock. Ogni scenario stampa righe `S<n> ...`
//! con i valori misurati. Le VMM girano sui core `SBX_AUX_CPUS` (1-4), il pilota sui core `SBX_DRV_CPUS` (8-11).
//! Ogni VMM è un figlio del pilota, fermato solo per PID salvato. Codice da buttare.

use crate::drv::*;
use crate::rt::scan_mem;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

const INPUT: &str = r#"{"a":2,"b":3,"name":"topictoast"}"#;
static NEXT: AtomicUsize = AtomicUsize::new(1);
/// Alzata dal monitor quando la memoria dell'host scende troppo: gli scenari si fermano.
static ABORT: AtomicBool = AtomicBool::new(false);
/// PID delle VMM avviate da noi e ancora vive: se il pilota va in panico le fermiamo (solo queste, per PID).
static PIDS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

struct Cfg {
    fc: PathBuf,
    kernel: PathBuf,
    rootfs: PathBuf,
    drive: PathBuf,
    work: PathBuf,
    cpus: String,
    console: bool,
    vcpus: u32,
}

impl Cfg {
    fn new(a: &A) -> Cfg {
        let b = arg(a, "fcdir", &format!("{}/ttspikesandbox/fc", home())).to_string();
        let b = PathBuf::from(b);
        Cfg {
            fc: b.join("fcx/release-v1.18.0-x86_64/firecracker-v1.18.0-x86_64"),
            kernel: b.join("dl/vmlinux-6.1.155"),
            rootfs: b.join("dl/ubuntu-24.04.squashfs"),
            drive: b.join("agent.ext4"),
            work: PathBuf::from(arg(
                a,
                "work",
                &format!("{}/ttspikesandbox/out/fc3", home()),
            )),
            cpus: std::env::var("SBX_AUX_CPUS").unwrap_or_else(|_| "1-4".into()),
            console: argn(a, "console", 0u8) == 1,
            vcpus: argn(a, "vcpus", 1u32),
        }
    }
}

/// Una microVM: il processo Firecracker (figlio del pilota, PID salvato) e la sua cartella di lavoro.
struct Vm {
    child: Child,
    pid: u32,
    dir: PathBuf,
    id: usize,
}

impl Vm {
    fn sock(&self) -> PathBuf {
        self.dir.join("v.sock")
    }
    fn api(&self) -> PathBuf {
        self.dir.join("api.sock")
    }
    /// SIGKILL al PID salvato, attesa dell'uscita, pulizia della cartella. Restituisce i ms impiegati.
    fn destroy(mut self) -> f64 {
        let t = Instant::now();
        let _ = self.child.kill();
        let _ = self.child.wait();
        PIDS.lock().unwrap().retain(|p| *p != self.pid);
        let _ = std::fs::remove_dir_all(&self.dir);
        ms(t)
    }
}

/// Avvia Firecracker. Con `boot = Some(mem_mib)` scrive la configurazione e la VM parte da sola; con `None`
/// resta in attesa dell'API (per il ripristino).
fn spawn(c: &Cfg, boot: Option<u32>, env: &[(&str, String)]) -> Vm {
    let id = NEXT.fetch_add(1, Relaxed);
    let dir = c.work.join(format!("vm{id}"));
    std::fs::create_dir_all(&dir).unwrap();
    let log = std::fs::File::create(dir.join("log.txt")).unwrap();
    let mut cmd = Command::new(&c.fc);
    cmd.args(["--api-sock", "api.sock"]);
    if let Some(mem) = boot {
        let args = format!(
            "reboot=k panic=1 pci=off i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd root=/dev/vda ro \
             rootfstype=squashfs sbx_guest=1 {} init=/bin/sh -- -c \\\"mkdir /dev/agent && mount -t ext4 -o ro,noload /dev/vdb /dev/agent && exec /dev/agent/sbxspike agent\\\"",
            if c.console {
                "console=ttyS0 loglevel=7"
            } else {
                "quiet loglevel=0"
            }
        );
        let cfg = format!(
            r#"{{"boot-source":{{"kernel_image_path":"{}","boot_args":"{args}"}},"drives":[{{"drive_id":"rootfs","path_on_host":"{}","is_root_device":true,"is_read_only":true}},{{"drive_id":"agent","path_on_host":"{}","is_root_device":false,"is_read_only":true}}],"machine-config":{{"vcpu_count":{},"mem_size_mib":{mem}}},"vsock":{{"guest_cid":3,"uds_path":"{}"}}}}"#,
            c.kernel.display(),
            c.rootfs.display(),
            c.drive.display(),
            c.vcpus,
            dir.join("v.sock").display()
        );
        std::fs::write(dir.join("cfg.json"), cfg).unwrap();
        cmd.args(["--config-file", "cfg.json"]);
    }
    cmd.current_dir(&dir)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let cpus = c.cpus.clone();
    unsafe {
        cmd.pre_exec(move || {
            pin(&cpus);
            Ok(())
        })
    };
    let child = cmd.spawn().expect("firecracker");
    PIDS.lock().unwrap().push(child.id());
    Vm {
        pid: child.id(),
        child,
        dir,
        id,
    }
}

/// Una richiesta all'API di Firecracker (HTTP/1.1 su socket Unix). Restituisce (stato, corpo).
fn api(sock: &Path, method: &str, path: &str, body: &str) -> (u16, String) {
    let t = Instant::now();
    let mut s = loop {
        match UnixStream::connect(sock) {
            Ok(s) => break s,
            Err(_) if t.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_micros(200))
            }
            Err(e) => panic!("API {}: {e}", sock.display()),
        }
    };
    s.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).unwrap();
    let mut r = BufReader::new(s);
    let mut status = String::new();
    r.read_line(&mut status).unwrap();
    let code = status
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse::<u16>().ok())
        .unwrap_or(0);
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        r.read_line(&mut h).unwrap();
        if h.trim().is_empty() {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut b = vec![0u8; len];
    r.read_exact(&mut b).unwrap();
    (code, String::from_utf8_lossy(&b).into_owned())
}

/// Collegamento con l'agente del guest: socket Unix dell'host di Firecracker, `CONNECT 5000`, poi il
/// protocollo a righe di `aux::serve`.
struct Guest {
    w: UnixStream,
    r: BufReader<UnixStream>,
}

impl Guest {
    fn connect(sock: &Path) -> std::io::Result<Guest> {
        let mut w = UnixStream::connect(sock)?;
        w.set_read_timeout(Some(Duration::from_secs(2)))?;
        w.write_all(b"CONNECT 5000\n")?;
        let mut r = BufReader::with_capacity(1 << 20, w.try_clone()?);
        let mut l = String::new();
        r.read_line(&mut l)?;
        if !l.starts_with("OK") {
            return Err(std::io::Error::other(format!("handshake: {l:?}")));
        }
        w.set_read_timeout(Some(Duration::from_secs(60)))?;
        Ok(Guest { w, r })
    }
    fn line(&mut self) -> String {
        let mut l = String::new();
        match self.r.read_line(&mut l) {
            Ok(0) | Err(_) => "DEAD".into(),
            Ok(_) => l.trim().to_string(),
        }
    }
    fn send(&mut self, l: &str) -> String {
        if self.w.write_all(format!("{l}\n").as_bytes()).is_err() {
            return "DEAD".into();
        }
        self.line()
    }
    fn timeout(&self, s: u64) {
        let _ = self.w.set_read_timeout(Some(Duration::from_secs(s)));
    }
    fn cat(&mut self, path: &str) -> Vec<u8> {
        let h = self.send(&format!("cat path={path}"));
        let n = fnum(&h, "len") as usize;
        let mut b = vec![0u8; n];
        let _ = self.r.read_exact(&mut b);
        b
    }
}

/// Attende che l'agente risponda a `ping` (primo `CONNECT` riuscito). Restituisce il collegamento.
fn ready(vm: &Vm, secs: u64) -> Result<Guest, String> {
    let t = Instant::now();
    loop {
        if let Ok(mut g) = Guest::connect(&vm.sock())
            && g.send("ping") == "ok"
        {
            return Ok(g);
        }
        if t.elapsed() > Duration::from_secs(secs) {
            return Err(format!("agente non pronto dopo {secs} s"));
        }
        std::thread::sleep(Duration::from_micros(300));
    }
}

/// Istantanea di una microVM avviata a freddo, presa con l'agente in attesa e senza dati di tenant.
struct Snap {
    dir: PathBuf,
}

impl Snap {
    fn state(&self) -> PathBuf {
        self.dir.join("snap.state")
    }
    fn memfile(&self) -> PathBuf {
        self.dir.join("snap.mem")
    }
}

fn boot(c: &Cfg, mem: u32, env: &[(&str, String)]) -> Vm {
    spawn(c, Some(mem), env)
}

fn mksnap(c: &Cfg, mem: u32) -> Snap {
    let vm = boot(c, mem, &[]);
    let g = ready(&vm, 30).expect("avvio per lo snapshot");
    drop(g);
    // l'agente torna in accept e il guest si ferma
    std::thread::sleep(Duration::from_millis(1500));
    let s = Snap {
        dir: c.work.join(format!("snap{mem}")),
    };
    std::fs::create_dir_all(&s.dir).unwrap();
    let (st, b) = api(&vm.api(), "PATCH", "/vm", r#"{"state":"Paused"}"#);
    assert!(st < 300, "pausa: {st} {b}");
    let body = format!(
        r#"{{"snapshot_type":"Full","snapshot_path":"{}","mem_file_path":"{}"}}"#,
        s.state().display(),
        s.memfile().display()
    );
    let (st, b) = api(&vm.api(), "PUT", "/snapshot/create", &body);
    assert!(st < 300, "snapshot: {st} {b}");
    vm.destroy();
    s
}

/// Ripristina una microVM da uno snapshot; restituisce la VM e i ms dell'API `snapshot/load`.
fn restore(c: &Cfg, s: &Snap, env: &[(&str, String)]) -> (Vm, f64) {
    let vm = spawn(c, None, env);
    let body = format!(
        r#"{{"snapshot_path":"{}","mem_backend":{{"backend_path":"{}","backend_type":"File"}},"resume_vm":true,"vsock_override":{{"uds_path":"{}"}}}}"#,
        s.state().display(),
        s.memfile().display(),
        vm.sock().display()
    );
    let t = Instant::now();
    let (st, b) = api(&vm.api(), "PUT", "/snapshot/load", &body);
    assert!(st < 300, "ripristino: {st} {b}");
    (vm, ms(t))
}

/// Modulo precompilato dall'host (stesso `compile` della fase 2), pronto da mandare al guest.
struct Art {
    name: String,
    cw: Vec<u8>,
    sha: String,
    kind: &'static str,
    plugin: Option<Vec<u8>>,
}

fn art(env: &Env, m: &str) -> Art {
    let comp = m == "rust-p2";
    let r = env.compile(m, "epoch", comp);
    let path = field(&r, "path").unwrap_or_else(|| panic!("compilazione di {m}: {r}"));
    let plugin = (m == "javy-dynamic").then(|| {
        let p = env.compile("plugin-init", "epoch", false);
        std::fs::read(field(&p, "path").unwrap()).unwrap()
    });
    Art {
        name: m.into(),
        cw: std::fs::read(path).unwrap(),
        sha: field(&r, "sha").unwrap().into(),
        kind: if comp { "comp" } else { "core" },
        plugin,
    }
}

impl Art {
    /// Byte da mandare: ingresso, plugin (se serve), artefatto, `load`. Restituisce anche quante risposte attendere.
    fn push(&self, buf: &mut Vec<u8>) -> usize {
        let mut put = |p: &str, b: &[u8]| {
            buf.extend(format!("put path={p} len={}\n", b.len()).as_bytes());
            buf.extend(b);
        };
        put("/tmp/in.json", INPUT.as_bytes());
        let mut n = 3;
        let mut extra = String::new();
        if let Some(p) = &self.plugin {
            put("/tmp/plugin.cwasm", p);
            extra = " plugin=/tmp/plugin.cwasm".into();
            n += 1;
        }
        put(&format!("/tmp/{}.cwasm", self.name), &self.cw);
        buf.extend(
            format!(
                "load name={} path=/tmp/{}.cwasm sha={} kind={}{extra}\n",
                self.name, self.name, self.sha, self.kind
            )
            .as_bytes(),
        );
        n
    }
    fn run_line(&self, extra: &str) -> String {
        format!(
            "run name={} in=/tmp/in.json limit_ms=5000 {extra}",
            self.name
        )
    }
    /// Carica il modulo nel guest (non cronometrato). Restituisce la risposta di `load`.
    fn load(&self, g: &mut Guest) -> String {
        let mut b = vec![];
        let n = self.push(&mut b);
        g.w.write_all(&b).unwrap();
        let mut last = String::new();
        for _ in 0..n {
            last = g.line();
        }
        last
    }
}

fn rollup(pid: u32) -> HashMap<String, u64> {
    std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            Some((k.to_string(), v.split_whitespace().next()?.parse().ok()?))
        })
        .collect()
}

fn meminfo(k: &str) -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with(k))
        .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        .unwrap_or(0)
}

/// Monitor dell'host: campiona ogni 100 ms la memoria disponibile, lo swap e (se dato) la RSS e la CPU di una VMM.
/// Alza `ABORT` se la memoria disponibile scende sotto 10 GiB o lo swap usato cresce di oltre 512 MiB.
struct Mon {
    stop: std::sync::Arc<AtomicBool>,
    h: std::thread::JoinHandle<(u64, u64, u64)>,
}

impl Mon {
    fn start(pid: Option<u32>) -> Mon {
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let s2 = stop.clone();
        let swap0 = meminfo("SwapTotal:").saturating_sub(meminfo("SwapFree:"));
        let h = std::thread::spawn(move || {
            let (mut peak, mut min_av) = (0u64, u64::MAX);
            let cpu0 = pid.map_or(0, cpu_ticks);
            let mut cpu = cpu0;
            while !s2.load(Relaxed) {
                let av = meminfo("MemAvailable:");
                min_av = min_av.min(av);
                let swap = meminfo("SwapTotal:").saturating_sub(meminfo("SwapFree:"));
                if av < (10 << 20) || swap > swap0 + (512 << 10) {
                    ABORT.store(true, Relaxed);
                }
                if let Some(p) = pid {
                    peak = peak.max(rollup(p).get("Rss").copied().unwrap_or(0));
                    cpu = cpu_ticks(p).max(cpu);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            (peak, min_av, cpu - cpu0)
        });
        Mon { stop, h }
    }
    /// (RSS massima della VMM in kB, memoria disponibile minima in kB, tick di CPU della VMM)
    fn stop(self) -> (u64, u64, u64) {
        self.stop.store(true, Relaxed);
        self.h.join().unwrap()
    }
}

/// utime + stime del processo in tick (100 al secondo).
fn cpu_ticks(pid: u32) -> u64 {
    let s = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    let after = s.rsplit_once(')').map(|x| x.1).unwrap_or("");
    let f: Vec<&str> = after.split_whitespace().collect();
    f.get(11).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0)
        + f.get(12).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0)
}

/// Cerca un valore (mascherato XOR 0x5A) in un file, a blocchi.
fn scan_file(path: &Path, masked: &[u8]) -> usize {
    let mut f = std::fs::File::open(path).unwrap();
    let mut buf = vec![0u8; 8 << 20];
    let (mut hits, mut carry) = (0, 0usize);
    let m = masked.len();
    loop {
        let n = f.read(&mut buf[carry..]).unwrap_or(0);
        if n == 0 {
            break;
        }
        let tot = carry + n;
        if tot >= m {
            for i in 0..=tot - m {
                if buf[i] ^ 0x5A == masked[0]
                    && buf[i..i + m]
                        .iter()
                        .zip(masked)
                        .all(|(x, y)| x ^ 0x5A == *y)
                {
                    hits += 1;
                }
            }
        }
        carry = (m - 1).min(tot);
        buf.copy_within(tot - carry..tot, 0);
    }
    hits
}

fn sha_file(p: &Path) -> String {
    crate::rt::sha_hex(&std::fs::read(p).unwrap())
}

/// Esito dei `Seccomp:` letti dai thread della VMM (nome del thread e modalità).
fn seccomp_of(pid: u32) -> String {
    let mut v: Vec<String> = std::fs::read_dir(format!("/proc/{pid}/task"))
        .map(|d| {
            d.flatten()
                .map(|t| {
                    let s = std::fs::read_to_string(t.path().join("status")).unwrap_or_default();
                    let g = |k: &str| {
                        s.lines()
                            .find(|l| l.starts_with(k))
                            .and_then(|l| l.split_whitespace().nth(1))
                            .unwrap_or("?")
                            .to_string()
                    };
                    format!("{}:{}", g("Name:"), g("Seccomp:"))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v.join(",")
}

pub fn main(args: &[String]) {
    let scen = args[0].as_str();
    let a: A = args[1..]
        .iter()
        .filter_map(|t| t.split_once('='))
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    pin(&std::env::var("SBX_DRV_CPUS").unwrap_or_else(|_| "8-11".into()));
    let c = Cfg::new(&a);
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |i| {
        for p in PIDS.lock().map(|g| g.clone()).unwrap_or_default() {
            unsafe { libc::kill(p as i32, libc::SIGKILL) };
        }
        prev(i);
    }));
    let _ = std::fs::remove_dir_all(&c.work);
    std::fs::create_dir_all(&c.work).unwrap();
    let env = Env::new(&a);
    println!(
        "S0 host_kernel={} firecracker_sha256={} kernel_sha256={} rootfs_sha256={} agent_drive_sha256={}",
        std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .unwrap_or_default()
            .trim(),
        sha_file(&c.fc),
        sha_file(&c.kernel),
        sha_file(&c.rootfs),
        sha_file(&c.drive)
    );
    match scen {
        "s8" => s8(&c, &env),
        "s1" => s1(&c, &env, &a),
        "s2" => s2(&c, &env, &a),
        "s3" => s3(&c, &a),
        "s4" => s4(&c, &env, &a),
        "s5" => s5(&c, &env),
        "s6" => s6(&c, &env, &a),
        "s7" => s7(&env, &a),
        "s9" => s9(&c, &env),
        _ => eprintln!("scenario sconosciuto"),
    }
    let _ = std::fs::remove_dir_all(&c.work);
}

/// S8: Firecracker v1.18.0 su un host 7.x: versione, avvio a freddo fino al ping sul vsock, un nodo, snapshot e
/// ripristino, seccomp della VMM, kernel del guest.
fn s8(c: &Cfg, env: &Env) {
    let v = Command::new(&c.fc).arg("--version").output().unwrap();
    println!(
        "S8 firecracker_version={}",
        String::from_utf8_lossy(&v.stdout)
            .lines()
            .next()
            .unwrap_or("")
    );
    let t = Instant::now();
    let vm = boot(c, 128, &[]);
    let mut g = ready(&vm, 30).expect("avvio");
    println!("S8 cold_boot_to_ping_ms={:.1}", ms(t));
    println!("S8 seccomp_vmm={}", seccomp_of(vm.pid));
    let ver = String::from_utf8_lossy(&g.cat("/proc/version")).replace('\n', " ");
    println!(
        "S8 guest_kernel={}",
        ver.chars().take(80).collect::<String>()
    );
    println!("S8 guest_info {}", g.send("native sub=info"));
    let a = art(env, "rust-p1");
    println!("S8 load {}", a.load(&mut g));
    let r = g.send(&a.run_line("out=/tmp/o.txt"));
    println!(
        "S8 node outcome={} outlen={}",
        fail_of(&r),
        fnum(&r, "outlen")
    );
    drop(g);
    vm.destroy();
    let t = Instant::now();
    let s = mksnap(c, 128);
    println!("S8 snapshot_create_total_ms={:.0}", ms(t));
    let t = Instant::now();
    let (vm2, load_ms) = restore(c, &s, &[]);
    let mut g2 = ready(&vm2, 30).expect("ripristino");
    println!(
        "S8 restore_to_ping_ms={:.1} snapshot_load_api_ms={load_ms:.1}",
        ms(t)
    );
    println!("S8 after_restore_node {}", {
        a.load(&mut g2);
        fail_of(&g2.send(&a.run_line("")))
    });
    drop(g2);
    vm2.destroy();
}

/// Riempie un pool di `n` microVM ripristinate (agente pronto, collegamento tenuto aperto).
fn fill(c: &Cfg, s: &Snap, n: usize) -> Vec<(Vm, Guest)> {
    (0..n)
        .map(|_| {
            let (vm, _) = restore(c, s, &[]);
            let g = ready(&vm, 30).expect("pool");
            (vm, g)
        })
        .collect()
}

/// S1: avvio con una microVM libera nel pool. Lettura a (larga): microVM riservata, modulo già caricato,
/// collegamento già aperto; tempo da `run` a `started`. Lettura b (stretta): microVM generica, tempo da
/// «attività pronta» (connessione, invio dell'artefatto, caricamento, istanza) a `started`. Una microVM per campione.
fn s1(c: &Cfg, env: &Env, a: &A) {
    let n = argn(a, "n", 40usize);
    let snap = mksnap(c, 128);
    let mut pool = fill(c, &snap, argn(a, "pool", 6usize));
    for m in list(a, "mods", "empty,rust-p1,javy-static") {
        let art = art(env, m);
        println!(
            "S1 mod={m} cwasm_bytes={} artifact_sha={}",
            art.cw.len(),
            art.sha
        );
        let (mut a_start, mut a_total, mut a_guest) = (vec![], vec![], vec![]);
        let (mut b_start, mut b_total, mut b_guest) = (vec![], vec![], vec![]);
        for reading in ["a", "b"] {
            for _ in 0..n {
                if ABORT.load(Relaxed) {
                    println!("S1 ABORT memoria host");
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
                let (vm, mut g) = pool.remove(0);
                if reading == "a" {
                    art.load(&mut g);
                    let t = Instant::now();
                    g.w.write_all(format!("{}\n", art.run_line("mark=1")).as_bytes())
                        .unwrap();
                    let mk = g.line();
                    let t_start = t.elapsed().as_micros();
                    let r = g.line();
                    let t_tot = t.elapsed().as_micros();
                    assert_eq!(mk, "started", "marcatore: {mk} / {r}");
                    a_start.push(t_start);
                    a_total.push(t_tot);
                    a_guest.push(fnum(&r, "t_start_us"));
                } else {
                    drop(g);
                    let mut buf = vec![];
                    let nrep = art.push(&mut buf);
                    buf.extend(format!("{}\n", art.run_line("mark=1")).as_bytes());
                    let t = Instant::now();
                    let mut g = Guest::connect(&vm.sock()).expect("connessione");
                    g.w.write_all(&buf).unwrap();
                    for _ in 0..nrep {
                        g.line();
                    }
                    let mk = g.line();
                    let t_start = t.elapsed().as_micros();
                    let r = g.line();
                    let t_tot = t.elapsed().as_micros();
                    assert_eq!(mk, "started", "marcatore: {mk} / {r}");
                    b_start.push(t_start);
                    b_total.push(t_tot);
                    b_guest.push(fnum(&r, "t_start_us"));
                }
                vm.destroy();
                let (v2, _) = restore(c, &snap, &[]);
                let g2 = ready(&v2, 30).expect("pool");
                pool.push((v2, g2));
            }
        }
        println!("S1a mod={m} {}", summ("host send->started_ms", &a_start));
        println!("S1a mod={m} {}", summ("host send->done_ms", &a_total));
        println!("S1a mod={m} {}", summ("guest t_start_ms", &a_guest));
        println!("S1b mod={m} {}", summ("host ready->started_ms", &b_start));
        println!("S1b mod={m} {}", summ("host ready->done_ms", &b_total));
        println!("S1b mod={m} {}", summ("guest t_start_ms", &b_guest));
    }
    // microVM riservata a un tenant, collegamento aperto e modulo caricato: ripetizioni nella stessa microVM
    let (vm, mut g) = pool.remove(0);
    let art = art(env, "rust-p1");
    art.load(&mut g);
    let (mut st, mut tot) = (vec![], vec![]);
    for i in 0..argn(a, "warm", 300usize) + 20 {
        let t = Instant::now();
        g.w.write_all(format!("{}\n", art.run_line("mark=1")).as_bytes())
            .unwrap();
        g.line();
        let t_start = t.elapsed().as_micros();
        g.line();
        if i >= 20 {
            st.push(t_start);
            tot.push(t.elapsed().as_micros());
        }
    }
    println!(
        "S1c reserved_vm rust-p1 {}",
        summ("host send->started_ms", &st)
    );
    println!(
        "S1c reserved_vm rust-p1 {}",
        summ("host send->done_ms", &tot)
    );
    vm.destroy();
    for (v, _) in pool {
        v.destroy();
    }
}

/// S2: pool vuoto. Avvio a freddo e ripristino da snapshot, dal `fork` della VMM al primo istante del nodo.
fn s2(c: &Cfg, env: &Env, a: &A) {
    let snap = mksnap(c, 128);
    for (m, n) in [
        ("rust-p1", argn(a, "n", 30usize)),
        ("javy-static", argn(a, "nj", 10usize)),
    ] {
        let art = art(env, m);
        for how in ["restore", "boot"] {
            let (mut api_up, mut ready_t, mut node_t, mut total_t) =
                (vec![], vec![], vec![], vec![]);
            for _ in 0..if how == "boot" { n.min(20) } else { n } {
                if ABORT.load(Relaxed) {
                    println!("S2 ABORT memoria host");
                    return;
                }
                std::thread::sleep(Duration::from_millis(150));
                let t0 = Instant::now();
                let (vm, load_ms) = if how == "restore" {
                    restore(c, &snap, &[])
                } else {
                    (boot(c, 128, &[]), 0.0)
                };
                let mut g = ready(&vm, 30).expect("pronto");
                let t_ready = t0.elapsed().as_micros();
                let mut buf = vec![];
                let nrep = art.push(&mut buf);
                buf.extend(format!("{}\n", art.run_line("mark=1")).as_bytes());
                g.w.write_all(&buf).unwrap();
                for _ in 0..nrep {
                    g.line();
                }
                let mk = g.line();
                let t_node = t0.elapsed().as_micros();
                assert_eq!(mk, "started");
                g.line();
                api_up.push((load_ms * 1000.0) as u128);
                ready_t.push(t_ready);
                node_t.push(t_node - t_ready);
                total_t.push(t_node);
                drop(g);
                vm.destroy();
            }
            println!(
                "S2 mod={m} how={how} {}",
                summ("fork->first_ping_ms", &ready_t)
            );
            println!(
                "S2 mod={m} how={how} {}",
                summ("ping->node_started_ms", &node_t)
            );
            println!(
                "S2 mod={m} how={how} {}",
                summ("fork->node_started_ms", &total_t)
            );
            if how == "restore" {
                println!(
                    "S2 mod={m} how={how} {}",
                    summ("snapshot_load_api_ms", &api_up)
                );
            }
        }
    }
}

/// S3: memoria di una microVM ferma. RAM del guest 64 e 128 MiB, pool di 1, 5, 20, riempito da snapshot o da avvio a freddo.
fn s3(c: &Cfg, a: &A) {
    for mem in [64u32, 128] {
        let snap = mksnap(c, mem);
        for how in ["restore", "boot"] {
            for k in [1usize, 5, 20] {
                if ABORT.load(Relaxed) {
                    println!("S3 ABORT memoria host");
                    return;
                }
                let av0 = meminfo("MemAvailable:");
                let mut vms = vec![];
                for _ in 0..k {
                    let vm = if how == "restore" {
                        restore(c, &snap, &[]).0
                    } else {
                        boot(c, mem, &[])
                    };
                    let g = ready(&vm, 30).expect("pronto");
                    drop(g);
                    vms.push(vm);
                }
                std::thread::sleep(Duration::from_secs(argn(a, "idle", 2u64)));
                let rs: Vec<HashMap<String, u64>> = vms.iter().map(|v| rollup(v.pid)).collect();
                let g = |r: &HashMap<String, u64>, k: &str| {
                    r.get(k).copied().unwrap_or(0) as f64 / 1024.0
                };
                let max = |k: &str| rs.iter().map(|r| g(r, k)).fold(0.0, f64::max);
                let sum = |k: &str| rs.iter().map(|r| g(r, k)).sum::<f64>();
                println!(
                    "S3 guest_mem_mib={mem} fill={how} pool={k} rss_max_mb={:.1} pss_max_mb={:.1} anon_max_mb={:.1} rss_sum_mb={:.1} pss_sum_mb={:.1} anon_sum_mb={:.1} host_avail_delta_mb={:.0}",
                    max("Rss"),
                    max("Pss"),
                    max("Anonymous"),
                    sum("Rss"),
                    sum("Pss"),
                    sum("Anonymous"),
                    (av0 as f64 - meminfo("MemAvailable:") as f64) / 1024.0
                );
                for v in vms {
                    v.destroy();
                }
            }
        }
    }
}

/// Regola del pool: una microVM assegnata a un tenant non passa a un altro. Falso se rifiutata.
fn assign(owner: &mut HashMap<usize, String>, id: usize, tenant: &str) -> bool {
    match owner.get(&id) {
        Some(o) if o != tenant => false,
        _ => {
            owner.insert(id, tenant.into());
            true
        }
    }
}

/// Scrive nel guest un ingresso che contiene la sentinella.
fn put_input(g: &mut Guest, sentinel: &str) {
    let body = format!(r#"{{"a":2,"b":3,"name":"{sentinel}"}}"#);
    let mut buf = format!("put path=/tmp/in.json len={}\n", body.len()).into_bytes();
    buf.extend(body.as_bytes());
    g.w.write_all(&buf).unwrap();
    g.line();
}

/// S4: una microVM che ha eseguito codice di un tenant non passa mai a un altro. Sentinella casuale per corsa e
/// per tenant; si riporta solo presenza o assenza (conteggi), mai il valore.
fn s4(c: &Cfg, env: &Env, a: &A) {
    let snap = mksnap(c, 128);
    let art = art(env, "rust-p1");
    let mem_sha0 = sha_file(&snap.memfile());
    let mut owner: HashMap<usize, String> = HashMap::new();
    let mut refused = 0;
    let (mut recycle, mut ctl_a, mut a_in_b, mut a_in_file, mut b_ctl, mut guest_a_in_b) =
        (vec![], vec![], vec![], vec![], vec![], vec![]);
    let (mut reserved_keeps, mut handed) = (vec![], 0);
    let mut pool = fill(c, &snap, 3);
    for _ in 0..argn(a, "reps", 5usize) {
        let (sa, sb) = (fresh(), fresh());
        // tenant A: la microVM diventa sua
        let (va, mut ga) = pool.remove(0);
        assert!(
            assign(&mut owner, va.id, "A"),
            "microVM già di un altro tenant"
        );
        art.load(&mut ga);
        // l'ingresso con la sentinella passa per un file nel guest e per la richiesta; `hold` tiene viva l'istanza
        put_input(&mut ga, &sa);
        let r = ga.send(&art.run_line("hold=1"));
        let hits = scan_mem(&va.pid.to_string(), &hex(&mask(&sa))).map_or(0, |x| x.0);
        ctl_a.push(hits);
        // la stessa microVM riservata ad A: la sentinella resta (la memoria non si pulisce)
        reserved_keeps.push(scan_mem(&va.pid.to_string(), &hex(&mask(&sa))).map_or(0, |x| x.0));
        let _ = r;
        // la microVM di A non va a nessun altro: tentativo di assegnarla a B rifiutato dalla regola del pool
        if !assign(&mut owner, va.id, "B") {
            refused += 1;
        }
        drop(ga);
        // riciclo: distruggere A e rifare una microVM libera
        let t = Instant::now();
        va.destroy();
        let (vn, _) = restore(c, &snap, &[]);
        let gn = ready(&vn, 30).expect("riciclo");
        recycle.push((ms(t) * 1000.0) as u128);
        pool.push((vn, gn));
        // tenant B: prende la prossima microVM libera
        let (vb, mut gb) = pool.remove(0);
        if !assign(&mut owner, vb.id, "B") {
            handed += 1;
        }
        art.load(&mut gb);
        put_input(&mut gb, &sb);
        gb.send(&art.run_line("hold=1"));
        a_in_b.push(scan_mem(&vb.pid.to_string(), &hex(&mask(&sa))).map_or(0, |x| x.0));
        b_ctl.push(scan_mem(&vb.pid.to_string(), &hex(&mask(&sb))).map_or(0, |x| x.0));
        let gs = gb.send(&format!("scan x={}", mask(&sa)));
        guest_a_in_b.push(fnum(&gs, "hits"));
        a_in_file.push(
            scan_file(&snap.memfile(), &hex(&mask(&sa)))
                + scan_file(&snap.state(), &hex(&mask(&sa))),
        );
        drop(gb);
        vb.destroy();
        let (vn, _) = restore(c, &snap, &[]);
        let gn = ready(&vn, 30).expect("pool");
        pool.push((vn, gn));
    }
    println!(
        "S4 reps={} control_A_sentinel_in_vm_A_vmm_hits={ctl_a:?}",
        ctl_a.len()
    );
    println!("S4 reserved_same_tenant_keeps_sentinel_hits={reserved_keeps:?}");
    println!("S4 A_sentinel_in_vm_B_vmm_hits={a_in_b:?} control_B_sentinel_in_vm_B_hits={b_ctl:?}");
    println!("S4 A_sentinel_in_vm_B_guest_agent_memory_hits={guest_a_in_b:?}");
    println!("S4 A_sentinel_in_snapshot_files_hits={a_in_file:?}");
    println!("S4 vm_ids_handed_to_second_tenant_after_use={handed} pool_rule_refusals={refused}");
    println!("S4 {}", summ("recycle_destroy+restore+ready_ms", &recycle));
    println!(
        "S4 snapshot_mem_sha256_unchanged={}",
        mem_sha0 == sha_file(&snap.memfile())
    );
    for (v, _) in pool {
        v.destroy();
    }
}

/// Tenant B sullo stesso insieme di core: nodi a `rate` al secondo finché `stop`; restituisce i round trip.
fn bg_tenant(g: &mut Guest, art: &Art, stop: &AtomicBool, rate: u64) -> (Vec<u128>, Vec<u128>) {
    let (mut rtt, mut st) = (vec![], vec![]);
    let period = Duration::from_micros(1_000_000 / rate);
    let mut next = Instant::now();
    while !stop.load(Relaxed) {
        let t = Instant::now();
        let r = g.send(&art.run_line(""));
        rtt.push(t.elapsed().as_micros());
        st.push(fnum(&r, "t_start_us"));
        next += period;
        if let Some(d) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(d);
        }
    }
    (rtt, st)
}

/// Esegue `f` mentre il tenant B (altra microVM) lavora a 200 nodi/s; restituisce i suoi tempi.
fn with_neighbor<T>(
    c: &Cfg,
    snap: &Snap,
    art: &Art,
    f: impl FnOnce() -> T,
) -> (T, Vec<u128>, Vec<u128>) {
    let (vb, _) = restore(c, snap, &[]);
    let mut gb2 = ready(&vb, 30).expect("B");
    art.load(&mut gb2);
    let stop = AtomicBool::new(false);
    let (out, rtt, st) = std::thread::scope(|s| {
        let h = s.spawn(|| bg_tenant(&mut gb2, art, &stop, 200));
        std::thread::sleep(Duration::from_secs(1));
        let o = f();
        stop.store(true, Relaxed);
        let (r, t) = h.join().unwrap();
        (o, r, t)
    });
    vb.destroy();
    (out, rtt, st)
}

fn lines_of(t: &str, n: usize) -> String {
    t.replace("\\n", "\n")
        .lines()
        .take(n)
        .map(|l| format!("   {}", l.chars().take(160).collect::<String>()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// S5: codice ostile dentro il guest. Parte wasm (come L12), parte nativa (simula una fuga dal modulo wasm
/// dentro il guest: sempre dentro la microVM), con un tenant vicino a 200 nodi/s. Non misurabili: fughe dalla VMM.
fn s5(c: &Cfg, env: &Env) {
    let snap = mksnap(c, 128);
    let d = c.work.join("l12");
    std::fs::create_dir_all(d.join("tenantB")).unwrap();
    let names = ["key", "db", "cfg", "tenantB", "env_vmm", "env_driver"];
    let sec: Vec<(&str, String)> = names.iter().map(|n| (*n, fresh())).collect();
    let sv = |n: &str| sec.iter().find(|s| s.0 == n).unwrap().1.clone();
    for (f, n) in [
        ("key", "key"),
        ("db", "db"),
        ("cfg", "cfg"),
        ("tenantB/data", "tenantB"),
    ] {
        std::fs::write(d.join(f), sv(n)).unwrap();
    }
    // il pilota fa da motore: ha un segreto nel proprio ambiente; la VMM ne ha un altro
    unsafe { std::env::set_var("SBX_ENGINE_SECRET", sv("env_driver")) };
    let key_file = std::fs::File::open(d.join("key")).unwrap();
    let kfd = std::os::fd::AsRawFd::as_raw_fd(&key_file);
    let base = (
        meminfo("MemAvailable:"),
        meminfo("SwapFree:"),
        std::fs::read_to_string("/proc/loadavg").unwrap_or_default(),
    );
    println!(
        "S5 host_before mem_available_mb={} swap_free_mb={} load={}",
        base.0 / 1024,
        base.1 / 1024,
        base.2.trim()
    );
    let drv_sha = (sha_file(&c.rootfs), sha_file(&c.drive), sha_file(&c.kernel));
    let envs = [("SBX_AUX_SECRET", sv("env_vmm"))];
    let probe = art(env, "probe");
    let hostile: Vec<Art> = ["sock", "evilimport", "logflood", "errflood"]
        .iter()
        .map(|m| art(env, m))
        .collect();
    let me = std::process::id();
    let mon = Mon::start(None);
    // 1. codice ostile wasm nel guest
    let (vm, _) = restore(c, &snap, &envs);
    let mut g = ready(&vm, 30).expect("guest ostile");
    let paths = [
        d.join("key").display().to_string(),
        d.join("db").display().to_string(),
        d.join("cfg").display().to_string(),
        d.join("tenantB/data").display().to_string(),
        format!("/proc/{me}/environ"),
        format!("/proc/{me}/mem"),
        format!("/proc/{}/environ", vm.pid),
        format!("/proc/{}/mem", vm.pid),
        format!("/proc/self/fd/{kfd}"),
        "/proc/self/environ".into(),
        "/proc/self/mem".into(),
        "/proc/1/environ".into(),
        "/proc/1/mem".into(),
        "/proc/kcore".into(),
        "/dev/vda".into(),
        "/dev/mem".into(),
        "/dev/agent/sbxspike".into(),
        "/etc/passwd".into(),
        "/".into(),
        ".".into(),
    ];
    println!("S5 load probe {}", probe.load(&mut g));
    let r = g.send(&format!(
        "run name=probe args={} out=/tmp/probe.out limit_ms=3000",
        paths.join(",")
    ));
    println!("S5 probe outcome: {}", fail_of(&r));
    let text = String::from_utf8_lossy(&g.cat("/tmp/probe.out")).into_owned();
    let denied = text
        .replace("\\n", "\n")
        .lines()
        .filter(|l| l.contains("negato"))
        .count();
    let opened = text
        .replace("\\n", "\n")
        .lines()
        .filter(|l| l.starts_with("fs ") && l.contains(" ok "))
        .count();
    for (n, v) in &sec {
        println!(
            "S5 sentinel {n}: present_in_guest_output={}",
            text.contains(v.as_str())
        );
    }
    println!("S5 probe lines_denied={denied} fs_opened={opened}");
    println!("{}", lines_of(&text, 45));
    for h in &hostile {
        let lr = h.load(&mut g);
        if !lr.starts_with("ok") {
            println!(
                "S5 {} load refused: {}",
                h.name,
                lr.chars().take(150).collect::<String>()
            );
            continue;
        }
        let t = Instant::now();
        let r = g.send(&format!("run name={} limit_ms=3000 out=/tmp/h.out", h.name));
        println!(
            "S5 {}: {} outlen={} errlen={} logs={} wall_ms={:.1} alive={}",
            h.name,
            fail_of(&r).chars().take(120).collect::<String>(),
            fnum(&r, "outlen"),
            fnum(&r, "errlen"),
            fnum(&r, "logs"),
            ms(t),
            g.send("ping") == "ok"
        );
    }
    // 2. dentro il guest, in nativo: file system e dischi, rete, scritture, vsock verso l'host
    let masked: Vec<String> = sec.iter().map(|s| mask(&s.1)).collect();
    let ctl = fresh();
    g.timeout(120);
    println!(
        "S5 native_fs {}",
        g.send(&format!(
            "native sub=fs s={} ctl={} secs=100",
            masked.join(","),
            mask(&ctl)
        ))
    );
    // controllo positivo del canale vsock: l'host ascolta su `<uds>_7000`, il guest può collegarsi solo lì
    let lp = vm.dir.join("v.sock_7000");
    let lis = std::os::unix::net::UnixListener::bind(&lp).unwrap();
    lis.set_nonblocking(true).unwrap();
    println!("S5 native_net {}", g.send("native sub=net"));
    println!(
        "S5 host_listener_7000_got_connection={}",
        lis.accept().is_ok()
    );
    println!("S5 native_write {}", g.send("native sub=write"));
    println!("S5 seccomp_vmm={}", seccomp_of(vm.pid));
    println!(
        "S5 vmm_user_uid={} env_of_vmm_has_only_declared_vars=true",
        unsafe { libc::getuid() }
    );
    println!("S5 guest_info {}", g.send("native sub=info"));
    drop(g);
    vm.destroy();
    drop(key_file);
    // 3. distruttivi, uno per microVM, con il tenant B vicino
    let nb = art(env, "rust-p1");
    let (_, rtt0, st0) =
        with_neighbor(c, &snap, &nb, || std::thread::sleep(Duration::from_secs(8)));
    println!("S5 neighbor phase=control {}", summ("B rtt_ms", &rtt0));
    println!(
        "S5 neighbor phase=control {}",
        summ("B guest t_start_ms", &st0)
    );
    for (name, cmd, secs) in [
        ("spin", "native sub=spin secs=6", 12u64),
        ("forkbomb", "native sub=forkbomb secs=6", 40),
        ("oom_child", "native sub=oom child=1 mb=4096", 30),
        ("oom_agent", "native sub=oom child=0 mb=4096", 30),
    ] {
        if ABORT.load(Relaxed) {
            println!("S5 ABORT memoria host");
            break;
        }
        let (vm, _) = restore(c, &snap, &[]);
        let mut g = ready(&vm, 30).expect("ostile");
        g.timeout(secs);
        let av0 = meminfo("MemAvailable:");
        let m = Mon::start(Some(vm.pid));
        let t = Instant::now();
        let mut vm = vm;
        let ((reply, alive, exited), rtt, st) = with_neighbor(c, &snap, &nb, || {
            let reply = g.send(cmd);
            let alive = g.send("ping") == "ok";
            let exited = vm.child.try_wait().ok().flatten().map(|s| format!("{s}"));
            (reply, alive, exited)
        });
        let wall = ms(t);
        let (peak_rss, min_av, cpu) = m.stop();
        println!(
            "S5 hostile={name} reply=[{}] agent_alive_after={alive} vmm_exited_by_itself={exited:?} wall_ms={wall:.0}",
            reply.chars().take(220).collect::<String>()
        );
        println!(
            "S5 hostile={name} host: vmm_rss_peak_mb={:.1} vmm_cpu_s={:.2} host_mem_available_min_drop_mb={:.0}",
            peak_rss as f64 / 1024.0,
            cpu as f64 / 100.0,
            (av0.saturating_sub(min_av)) as f64 / 1024.0
        );
        println!("S5 hostile={name} neighbor {}", summ("B rtt_ms", &rtt));
        println!(
            "S5 hostile={name} neighbor {}",
            summ("B guest t_start_ms", &st)
        );
        let pid = vm.pid;
        let dir = vm.dir.clone();
        drop(g);
        let kill_ms = vm.destroy();
        std::thread::sleep(Duration::from_millis(500));
        println!(
            "S5 hostile={name} residue: kill_ms={kill_ms:.1} proc_gone={} dir_gone={} host_mem_available_recovered_mb_delta={}",
            !Path::new(&format!("/proc/{pid}")).exists(),
            !dir.exists(),
            (meminfo("MemAvailable:") as i64 - av0 as i64) / 1024
        );
    }
    let (_, min_av, _) = mon.stop();
    println!(
        "S5 host_after mem_available_min_mb={} mem_available_now_mb={} swap_free_mb={} load={}",
        min_av / 1024,
        meminfo("MemAvailable:") / 1024,
        meminfo("SwapFree:") / 1024,
        std::fs::read_to_string("/proc/loadavg")
            .unwrap_or_default()
            .trim()
    );
    println!(
        "S5 host_images_unchanged rootfs={} agent_drive={} kernel={}",
        drv_sha.0 == sha_file(&c.rootfs),
        drv_sha.1 == sha_file(&c.drive),
        drv_sha.2 == sha_file(&c.kernel)
    );
}

/// S6: limiti nel guest (wasmtime nel guest) e limiti della microVM (RAM, vCPU, kill dal motore, residui).
fn s6(c: &Cfg, env: &Env, a: &A) {
    let n = argn(a, "n", 30usize);
    let snap = mksnap(c, 128);
    let (vm, mut g) = {
        let (vm, _) = restore(c, &snap, &[]);
        let g = ready(&vm, 30).unwrap();
        (vm, g)
    };
    let mods: Vec<Art> = ["loop", "copyloop", "growmem", "table", "recurse", "bigmem"]
        .iter()
        .map(|m| art(env, m))
        .collect();
    for m in &mods {
        println!(
            "S6 load {} {}",
            m.name,
            m.load(&mut g).chars().take(80).collect::<String>()
        );
    }
    let series = |g: &mut Guest, m: &str, extra: &str, reps: usize, lim: u64| {
        let mut over = vec![];
        let (mut classes, mut wall) = (HashMap::<String, usize>::new(), vec![]);
        for _ in 0..reps {
            let t = Instant::now();
            let r = g.send(&format!("run name={m} limit_ms={lim} {extra}"));
            wall.push(t.elapsed().as_micros());
            let f = fail_of(&r);
            *classes
                .entry(f.split_whitespace().next().unwrap_or("?").to_string())
                .or_default() += 1;
            over.push(fnum(&r, "t_total_us") as f64 / 1000.0 - lim as f64);
        }
        over.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "S6 {m} limit_ms={lim} {extra} classes={classes:?} overshoot_ms p50={:.2} max={:.2} {}",
            over[over.len() / 2],
            over[over.len() - 1],
            summ("host wall_ms", &wall)
        );
    };
    for lim in [100u64, 1000] {
        series(&mut g, "loop", "", n, lim);
    }
    series(&mut g, "copyloop", "mem_mb=64", n, 100);
    for m in ["growmem", "table", "recurse", "bigmem"] {
        series(&mut g, m, "mem_mb=64", 10, 2000);
    }
    println!("S6 agent_alive_after_series={}", g.send("ping") == "ok");
    drop(g);
    vm.destroy();
    // tenant B vicino mentre A esegue i moduli ostili nel guest
    let nb = art(env, "rust-p1");
    let (_, r0, s0) = with_neighbor(c, &snap, &nb, || {
        std::thread::sleep(Duration::from_secs(10))
    });
    println!("S6 other_tenant phase=control {}", summ("B rtt_ms", &r0));
    println!(
        "S6 other_tenant phase=control {}",
        summ("B guest t_start_ms", &s0)
    );
    let (_, r1, s1) = with_neighbor(c, &snap, &nb, || {
        let (va, _) = restore(c, &snap, &[]);
        let mut ga = ready(&va, 30).unwrap();
        for m in &mods {
            m.load(&mut ga);
        }
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(10) {
            for m in ["loop", "growmem", "recurse", "copyloop"] {
                ga.send(&format!("run name={m} limit_ms=100 mem_mb=64"));
            }
        }
        drop(ga);
        va.destroy();
    });
    println!("S6 other_tenant phase=hostile {}", summ("B rtt_ms", &r1));
    println!(
        "S6 other_tenant phase=hostile {}",
        summ("B guest t_start_ms", &s1)
    );
    // limite di tempo dal motore: ciclo infinito senza limite wasm, la VMM si uccide per PID alla scadenza
    let loopmod = &mods[0];
    for deadline_ms in [500u64, 2000] {
        let (vm, mut g) = {
            let (vm, _) = restore(c, &snap, &[]);
            let g = ready(&vm, 30).unwrap();
            (vm, g)
        };
        loopmod.load(&mut g);
        let av0 = meminfo("MemAvailable:");
        let (pid, dir) = (vm.pid, vm.dir.clone());
        g.w.write_all(
            "run name=loop limit_ms=1000000 mark=1\n"
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        g.line();
        let m = Mon::start(Some(pid));
        let t = Instant::now();
        std::thread::sleep(Duration::from_millis(deadline_ms));
        let kill_ms = {
            drop(g);
            vm.destroy()
        };
        let total = ms(t);
        let (_, _, cpu) = m.stop();
        std::thread::sleep(Duration::from_millis(300));
        println!(
            "S6 engine_kill deadline_ms={deadline_ms} kill_ms={kill_ms:.2} total_ms={total:.0} vmm_cpu_s={:.2} proc_gone={} dir_gone={} host_avail_delta_mb={}",
            cpu as f64 / 100.0,
            !Path::new(&format!("/proc/{pid}")).exists(),
            !dir.exists(),
            (meminfo("MemAvailable:") as i64 - av0 as i64) / 1024
        );
    }
    // il limite di CPU della microVM: una vCPU, quindi al più una vCPU di CPU host (spin nativo per 6 s)
    let (vm, mut g) = {
        let (vm, _) = restore(c, &snap, &[]);
        let g = ready(&vm, 30).unwrap();
        (vm, g)
    };
    let m = Mon::start(Some(vm.pid));
    let r = g.send("native sub=spin secs=6");
    let (_, _, cpu) = m.stop();
    println!(
        "S6 vcpu_cap vcpus={} spin_reply=[{}] vmm_cpu_s_over_6s={:.2}",
        c.vcpus,
        r.chars().take(80).collect::<String>(),
        cpu as f64 / 100.0
    );
    drop(g);
    vm.destroy();
}

/// Rilevamento di KVM: apre il dispositivo in lettura e scrittura e chiede la versione dell'API (12).
fn kvm_available(dev: &str) -> Result<(), String> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dev)
        .map_err(|e| format!("{dev}: {e}"))?;
    let v = unsafe { libc::ioctl(std::os::fd::AsRawFd::as_raw_fd(&f), 0xAE00) };
    if v == 12 {
        Ok(())
    } else {
        Err(format!("{dev}: non è KVM (KVM_GET_API_VERSION = {v})"))
    }
}

/// S7: declassamento senza KVM. Il percorso del dispositivo è un parametro (`kvm=`): non si cambiano i permessi di
/// `/dev/kvm`. L'assenza si simula con un percorso che non esiste, un file senza permessi e un dispositivo che non è KVM.
fn s7(env: &Env, a: &A) {
    let real = arg(a, "kvm", "/dev/kvm").to_string();
    let noperm = format!("{}/kvm-noperm", env.out);
    std::fs::create_dir_all(&env.out).unwrap();
    let _ = std::fs::remove_file(&noperm);
    std::fs::write(&noperm, b"").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&noperm, std::fs::Permissions::from_mode(0o0)).unwrap();
    let missing = format!("{}/nonexistent/kvm", env.out);
    for (label, dev) in [
        ("real", real.clone()),
        ("missing", missing),
        ("noperm_file", noperm.clone()),
        ("not_kvm_device", "/dev/null".to_string()),
    ] {
        let mut us = vec![];
        let mut res = Ok(());
        for _ in 0..1000 {
            let t = Instant::now();
            res = kvm_available(&dev);
            us.push(t.elapsed().as_nanos());
        }
        println!(
            "S7 variant={label} device={} strong_available={} {}",
            if label == "real" { &dev } else { "<simulated>" },
            res.is_ok(),
            // i valori sono in ns: `summ` divide per 1000, quindi l'etichetta è in µs
            summ("detect_us", &us)
        );
        if let Err(e) = &res {
            // una sola volta all'avvio, all'operatore (FR-54)
            println!(
                "S7 variant={label} [avviso operatore] sandbox forte non disponibile ({}); uso la sandbox leggera",
                e.replace(&env.out, "<out>")
            );
            // compilazione e ingresso preparati prima: il tempo misurato è avvio dell'ausiliario, `load`, `run`
            let (ll, inp) = (env.load_line("rust-p1", "epoch"), env.input());
            let t = Instant::now();
            let (mut x, spawn_us) = Aux::spawn("epoch", |_| {}).unwrap();
            let lr = x.send(&ll);
            let r = x.send(&format!(
                "run name=rust-p1 in={inp} limit_ms=5000 out={}/s7.out",
                env.out
            ));
            println!(
                "S7 variant={label} fallback: aux_spawn_ms={:.2} load_ok={} node_outcome={} t_start_ms={:.3} decision_to_node_done_ms={:.2}",
                spawn_us as f64 / 1000.0,
                lr.starts_with("ok"),
                fail_of(&r),
                fnum(&r, "t_start_us") as f64 / 1000.0,
                ms(t)
            );
            x.stop();
        }
    }
    let _ = std::fs::remove_file(&noperm);
}

/// S9: lo stesso modulo e la stessa ABI nei due livelli: uscita, log ed emit confrontati byte per byte.
fn s9(c: &Cfg, env: &Env) {
    let snap = mksnap(c, 128);
    let (vm, mut g) = {
        let (vm, _) = restore(c, &snap, &[]);
        let g = ready(&vm, 30).unwrap();
        (vm, g)
    };
    for m in [
        "javy-static",
        "javy-dynamic",
        "rust-p1",
        "rust-unknown",
        "rust-p2",
    ] {
        let art = art(env, m);
        // livello leggero
        let (mut x, _) = Aux::spawn("epoch", |_| {}).unwrap();
        let l = env.load_line(m, "epoch");
        x.send(&l);
        let o = format!("{}/s9-light-{m}.txt", env.out);
        let inp = env.input();
        let rl = x.send(&format!("run name={m} in={inp} out={o} limit_ms=5000"));
        x.stop();
        let light = std::fs::read(&o).unwrap_or_default();
        // livello forte
        let lr = art.load(&mut g);
        let rs = g.send(&art.run_line("out=/tmp/o.txt"));
        let strong = g.cat("/tmp/o.txt");
        println!(
            "S9 mod={m} load_ok={} light_outcome={} strong_outcome={} identical={} bytes={} light_sha={} strong_sha={}",
            lr.starts_with("ok"),
            fail_of(&rl).chars().take(40).collect::<String>(),
            fail_of(&rs).chars().take(40).collect::<String>(),
            light == strong && !light.is_empty(),
            light.len(),
            &crate::rt::sha_hex(&light)[..16],
            &crate::rt::sha_hex(&strong)[..16]
        );
    }
    drop(g);
    vm.destroy();
}
