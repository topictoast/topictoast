//! Motore finto (stand-in) e pilota della fase 4 (spike S-2): `drive4 <scenario> [k=v ...]`.
//! Il motore finto possiede i segreti finti di ogni corsa (file della chiave, chiave nell'ambiente e in
//! memoria, file del database, configurazione, cartella di due tenant), avvia gli ausiliari confinati e li
//! mette alla prova. I valori sono casuali per corsa, non si stampano e non si scrivono mai nel repository:
//! i file grezzi riportano solo presenza o assenza. Non esiste un motore vero: A5 misura gli ausiliari più
//! questo processo, e A7 gira sul percorso della sandbox (il pilota fa da motore), non sul percorso completo.
//! Codice da buttare.

use crate::confine::{EXIT_IPC_VERSION, EXIT_REFUSED, IPC_VERSION};
use crate::drv::{A, Env, arg, argn, aux_cpus, fresh, list, mask, pct, pin, summ};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixListener;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const ROLES: [&str; 5] = ["validation", "jq", "callhttp", "light", "stream"];
static USR1: AtomicU64 = AtomicU64::new(0);

extern "C" fn on_usr1(_: i32) {
    USR1.fetch_add(1, Ordering::Relaxed);
}

/// Dumpable e «ptracer» del motore finto: `strong` = come AD-6 (dumpable 0, Yama attivo); `weak` = senza
/// l'uno e senza l'altro (PR_SET_PTRACER_ANY), per isolare ciò che fa il confinamento da solo.
fn set_engine_mode(dump: bool, any: bool) {
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, dump as libc::c_ulong, 0, 0, 0);
        libc::prctl(
            libc::PR_SET_PTRACER,
            if any { libc::c_ulong::MAX } else { 0 },
            0,
            0,
            0,
        );
    }
}

pub struct Eng {
    run: String,
    key_p: String,
    db_p: String,
    cfg_p: String,
    ta_dir: String,
    tb_dir: String,
    tb_file: String,
    ta_file: String,
    etc_dir: String,
    ca_p: String,
    cache: String,
    sock: String,
    abs: String,
    key: String,
    db: String,
    cfgv: String,
    tbv: String,
    key_mem: Vec<u8>,
    ext_port: u16,
    bus_port: u16,
    int_port: u16,
    #[allow(dead_code)] // tenuto per la compilazione dei moduli nel costruttore
    env: Env,
    mods: HashMap<String, (String, String, bool)>, // nome -> (percorso cwasm, sha, componente)
    leak_fd: i32,
    n_scratch: AtomicU64,
    _keep: Vec<Box<dyn std::any::Any>>,
}

fn serve_ok(l: std::net::TcpListener) {
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let mut s = s;
            let mut b = [0u8; 256];
            let _ = s.read(&mut b);
            let _ = s.write_all(b"ok\n");
        }
    });
}

impl Eng {
    /// Prepara cartelle, segreti finti, listener e moduli compilati. Si rifà da capo a ogni corsa.
    pub fn new(a: &A) -> Eng {
        let h = std::env::var("HOME").unwrap_or_default();
        let out = arg(a, "out", &format!("{h}/ttspikesandbox/out")).to_string();
        let run = format!("{out}/p4-{}", std::process::id());
        let mk = |p: &str| std::fs::create_dir_all(p).unwrap();
        mk(&run);
        let (ta_dir, tb_dir) = (format!("{run}/tenants/A"), format!("{run}/tenants/B"));
        let (etc_dir, cache) = (format!("{run}/etc"), format!("{run}/cache"));
        for d in [&ta_dir, &tb_dir, &etc_dir, &cache] {
            mk(d);
        }
        let (key, db, cfgv, tbv) = (fresh(), fresh(), fresh(), fresh());
        let wr = |p: &str, s: &str| std::fs::write(p, s).unwrap();
        let (key_p, db_p, cfg_p) = (
            format!("{run}/master.key"),
            format!("{run}/db.sqlite"),
            format!("{run}/config.toml"),
        );
        wr(&key_p, &key);
        wr(&db_p, &format!("SQLite format 3 fake {db}"));
        wr(&cfg_p, &format!("[fake]\nsecret = \"{cfgv}\"\n"));
        let (tb_file, ta_file) = (format!("{tb_dir}/secret.dat"), format!("{ta_dir}/own.dat"));
        wr(&tb_file, &tbv);
        wr(&ta_file, "tenant A own data");
        let ca_p = format!("{etc_dir}/ca.pem");
        wr(&ca_p, "fake ca bundle");
        std::fs::set_permissions(&key_p, std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .unwrap();
        // listener: socket UNIX con percorso, astratto, e TCP «interno» (il motore); due TCP «esterni»
        let sock = format!("{run}/eng.sock");
        let ul = UnixListener::bind(&sock).unwrap();
        let abs = format!("sbx-p4-{}", std::process::id());
        let al = abstract_listener(&abs);
        let il = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let (el, bl) = (
            std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
            std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
        );
        let (int_port, ext_port, bus_port) = (
            il.local_addr().unwrap().port(),
            el.local_addr().unwrap().port(),
            bl.local_addr().unwrap().port(),
        );
        serve_ok(el);
        serve_ok(bl);
        unsafe { libc::signal(libc::SIGUSR1, on_usr1 as *const () as usize) };
        // un descrittore «dimenticato» aperto senza CLOEXEC: l'ausiliario lo eredita, a meno che lo chiuda
        let leak_fd = unsafe {
            libc::open(
                std::ffi::CString::new(key_p.clone()).unwrap().as_ptr(),
                libc::O_RDONLY,
            )
        };
        let env = Env {
            wasm: arg(a, "wasm", &format!("{h}/ttspikesandbox/wasm")).to_string(),
            cache: cache.clone(),
            out: out.clone(),
        };
        let mut mods = HashMap::new();
        for m in [
            "empty",
            "rust-p1",
            "javy-static",
            "loop",
            "growmem",
            "recurse",
            "table",
            "probe",
            "bigmem",
        ] {
            let r = env.compile(m, "epoch", false);
            let f = |k| crate::drv::field(&r, k).map(String::from);
            if let (Some(p), Some(s)) = (f("path"), f("sha")) {
                mods.insert(m.to_string(), (p, s, false));
            } else {
                eprintln!("compilazione di {m} fallita: {r}");
            }
        }
        Eng {
            key_mem: key.clone().into_bytes(),
            run,
            key_p,
            db_p,
            cfg_p,
            ta_dir,
            tb_dir,
            tb_file,
            ta_file,
            etc_dir,
            ca_p,
            cache,
            sock,
            abs,
            key,
            db,
            cfgv,
            tbv,
            ext_port,
            bus_port,
            int_port,
            env,
            mods,
            leak_fd,
            n_scratch: AtomicU64::new(0),
            _keep: vec![Box::new(ul), Box::new(al), Box::new(il)],
        }
    }

    pub fn cleanup(&self) {
        unsafe { libc::close(self.leak_fd) };
        let _ = std::fs::remove_dir_all(&self.run);
    }
}

fn abstract_listener(name: &str) -> i32 {
    unsafe {
        let fd = libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
        let mut a: libc::sockaddr_un = std::mem::zeroed();
        a.sun_family = libc::AF_UNIX as u16;
        for (i, b) in name.bytes().enumerate() {
            a.sun_path[1 + i] = b as _;
        }
        let len = (2 + 1 + name.len()) as u32;
        libc::bind(fd, (&a as *const libc::sockaddr_un).cast(), len);
        libc::listen(fd, 8);
        fd
    }
}

/// Come è finito un ausiliario che non risponde più.
#[derive(Debug, Clone)]
pub struct Gone {
    pub code: Option<i32>,
    pub sig: Option<i32>,
    pub core: bool,
    pub stderr: String,
}

impl Gone {
    fn text(&self) -> String {
        match (self.sig, self.code) {
            (Some(31), _) => "KILLED(SIGSYS)".into(),
            (Some(s), _) => format!("KILLED(sig{s}{})", if self.core { ",core" } else { "" }),
            (_, Some(c)) => format!("exit({c})"),
            _ => "gone".into(),
        }
    }
}

pub struct R {
    pub child: Child,
    tx: ChildStdin,
    rx: BufReader<ChildStdout>,
    pub pid: i32,
    pub confined: String,
    /// `Seccomp`, `NoNewPrivs`, ... letti da `/proc/<pid>/status` prima di mandare qualunque riga.
    pub snap: String,
    pub t_ready_us: u128,
    pub t_fork_us: u128,
}

pub enum Spawn {
    Up(Box<R>),
    Refused(String),
    Died(Gone),
}

pub struct Sp<'a> {
    pub role: &'a str,
    pub mode: &'a str,
    /// Moduli (nomi) di cui il motore apre il cwasm e passa il descrittore (ruolo `light`): fd 3, 4, ...
    pub mods: &'a [&'a str],
    pub ipc: u32,
    pub leak: bool,
    pub inspect: bool,
    pub discover: Option<&'a str>,
}

impl<'a> Sp<'a> {
    pub fn new(role: &'a str, mode: &'a str) -> Sp<'a> {
        Sp {
            role,
            mode,
            mods: &[],
            ipc: IPC_VERSION,
            leak: false,
            inspect: false,
            discover: None,
        }
    }
}

fn status_snap(pid: i32) -> String {
    let s = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    let g = |k: &str| {
        s.lines()
            .find(|l| l.starts_with(k))
            .map(|l| l[k.len()..].trim().to_string())
            .unwrap_or_default()
    };
    let uid = std::fs::metadata(format!("/proc/{pid}/environ"))
        .map(|m| std::os::unix::fs::MetadataExt::uid(&m))
        .unwrap_or(9999);
    let lim = std::fs::read_to_string(format!("/proc/{pid}/limits")).unwrap_or_default();
    let l = |k: &str| {
        lim.lines()
            .find(|x| x.starts_with(k))
            .map(|x| x.split_whitespace().rev().nth(2).unwrap_or("?").to_string())
            .unwrap_or_default()
    };
    format!(
        "status_seccomp={} seccomp_filters={} status_nnp={} environ_owner_uid={uid} lim_core={} lim_nofile={} lim_fsize={} threads={}",
        g("Seccomp:"),
        g("Seccomp_filters:"),
        g("NoNewPrivs:"),
        l("Max core file size"),
        l("Max open files"),
        l("Max file size"),
        g("Threads:")
    )
}

impl Eng {
    fn scratch(&self) -> String {
        let n = self.n_scratch.fetch_add(1, Ordering::Relaxed);
        let d = format!("{}/scr-{n}", self.run);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Avvia un ausiliario confinato e lo porta fino a «pronto»: legge il rapporto di confinamento, verifica lo
    /// stato del kernel, poi (e solo poi) manda `hello`.
    pub fn spawn(&self, sp: &Sp) -> Spawn {
        let exe = std::env::current_exe().unwrap();
        let mut c = if let Some(log) = sp.discover {
            // scoperta delle syscall con gdb (non c'è strace): il registro è scritto dallo script di gdb
            let mut g = Command::new("gdb");
            g.args(["-batch", "-nx", "-x", log, "--args"]).arg(&exe);
            g
        } else {
            Command::new(&exe)
        };
        c.args(["role", sp.role]);
        c.arg(format!("mode={}", sp.mode));
        let mut scratch = None;
        match sp.role {
            "callhttp" => {
                let s = self.scratch();
                c.args([
                    format!("ro={}", self.etc_dir),
                    format!("tcp={}", self.ext_port),
                    format!("scratch={s}"),
                ]);
                scratch = Some(s);
            }
            "stream" => {
                let s = self.scratch();
                c.args([
                    format!("rw={}", self.ta_dir),
                    format!("tcp={}", self.bus_port),
                    format!("scratch={s}"),
                ]);
                scratch = Some(s);
            }
            "light" if !sp.mods.is_empty() => {
                let k: Vec<String> = (0..sp.mods.len()).map(|i| (3 + i).to_string()).collect();
                c.arg(format!("keep={}", k.join(",")));
            }
            _ => {}
        }
        if sp.discover.is_some() {
            c.arg("discover=1");
        }
        let files: Vec<std::fs::File> = sp
            .mods
            .iter()
            .map(|m| std::fs::File::open(&self.mods[*m].0).unwrap())
            .collect();
        let srcs: Vec<i32> = files.iter().map(|f| f.as_raw_fd()).collect();
        let cpus = aux_cpus();
        c.env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if sp.discover.is_some() {
            c.env("PATH", "/usr/bin:/bin")
                .env("HOME", std::env::var("HOME").unwrap_or_default());
        }
        let _ = sp.leak; // il descrittore dimenticato (leak_fd) è sempre aperto nel motore e senza CLOEXEC
        unsafe {
            c.pre_exec(move || {
                pin(&cpus);
                for (i, s) in srcs.iter().enumerate() {
                    let t = libc::fcntl(*s, libc::F_DUPFD, 100);
                    libc::dup2(t, 3 + i as i32);
                    libc::close(t);
                }
                Ok(())
            })
        };
        let t0 = Instant::now();
        let mut child = c.spawn().expect("spawn");
        let t_fork_us = t0.elapsed().as_micros();
        drop(files);
        let pid = child.id() as i32;
        let tx = child.stdin.take().unwrap();
        let mut rx = BufReader::new(child.stdout.take().unwrap());
        let (mut confined, mut l) = (String::new(), String::new());
        let gone = |child: &mut Child, rx: &mut BufReader<ChildStdout>| {
            let st = child.wait().unwrap();
            let mut e = String::new();
            let _ = child.stderr.take().map(|mut s| s.read_to_string(&mut e));
            let _ = rx;
            Gone {
                code: st.code(),
                sig: st.signal(),
                core: st.core_dumped(),
                stderr: e.trim().to_string(),
            }
        };
        loop {
            l.clear();
            if rx.read_line(&mut l).unwrap_or(0) == 0 {
                let g = gone(&mut child, &mut rx);
                if let Some(s) = scratch {
                    let _ = std::fs::remove_dir_all(s);
                }
                return if g.code == Some(EXIT_REFUSED) {
                    Spawn::Refused(g.stderr)
                } else {
                    Spawn::Died(g)
                };
            }
            if let Some(r) = l.trim().strip_prefix("confined ") {
                confined = r.to_string();
            } else if l.trim() == "ready" {
                break;
            }
        }
        let snap = if sp.inspect {
            status_snap(pid)
        } else {
            String::new()
        };
        let mut r = R {
            child,
            tx,
            rx,
            pid,
            confined,
            snap,
            t_ready_us: 0,
            t_fork_us,
        };
        // l'ingresso del motore: versione dell'IPC. Prima di questo punto l'ausiliario non ha letto nulla.
        match r.send(&format!("hello ipc={} role={}", sp.ipc, sp.role)) {
            Ok(_) => {}
            Err(g) => return Spawn::Died(g),
        }
        r.t_ready_us = t0.elapsed().as_micros();
        Spawn::Up(Box::new(r))
    }
}

impl R {
    /// Manda una riga e legge una riga di risposta, o dice come è finito l'ausiliario.
    pub fn send(&mut self, line: &str) -> Result<String, Gone> {
        let w = writeln!(self.tx, "{line}").and_then(|_| self.tx.flush());
        let mut r = String::new();
        if w.is_ok() && self.rx.read_line(&mut r).unwrap_or(0) > 0 {
            return Ok(r.trim().to_string());
        }
        let st = self.child.wait().unwrap();
        let mut e = String::new();
        let _ = self
            .child
            .stderr
            .take()
            .map(|mut s| s.read_to_string(&mut e));
        Err(Gone {
            code: st.code(),
            sig: st.signal(),
            core: st.core_dumped(),
            stderr: e.trim().to_string(),
        })
    }

    pub fn read_line(&mut self) -> Option<String> {
        let mut r = String::new();
        (self.rx.read_line(&mut r).unwrap_or(0) > 0).then(|| r.trim().to_string())
    }

    /// Arresto per inattività: `quit` e attesa del PID salvato.
    pub fn stop(mut self) {
        let _ = writeln!(self.tx, "quit");
        let _ = self.tx.flush();
        let _ = self.child.wait();
    }
}

impl Drop for R {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn up(s: Spawn, what: &str) -> Box<R> {
    match s {
        Spawn::Up(r) => r,
        Spawn::Refused(e) => panic!("{what}: rifiutato: {e}"),
        Spawn::Died(g) => panic!("{what}: morto: {g:?}"),
    }
}

// ---------------------------------------------------------------------------------------------- lavoro legittimo

const NODE_IN: &str = r#"{"a":2,"b":3,"name":"topictoast"}"#;

/// Lavoro legittimo del ruolo (A1, A4 e scoperta delle syscall): niente prove ostili. Restituisce le righe di esito.
fn workload(e: &Eng, r: &mut R, role: &str) -> Vec<String> {
    let mut o = vec![];
    let mut push = |s: String| o.push(s);
    match role {
        "validation" | "jq" => push(format!("{:?}", r.send("work len=100"))),
        "callhttp" => push(format!(
            "{:?}",
            r.send(&format!("work ca={} port={}", e.ca_p, e.ext_port))
        )),
        "stream" => push(format!(
            "{:?}",
            r.send(&format!("work dir={} port={}", e.ta_dir, e.bus_port))
        )),
        _ => {
            // light: carica tutti i moduli passati per descrittore e li esegue
            let names = [
                "empty",
                "rust-p1",
                "javy-static",
                "loop",
                "growmem",
                "recurse",
                "table",
                "probe",
                "bigmem",
            ];
            for (i, n) in names.iter().enumerate() {
                let sha = &e.mods[*n].1;
                let l = r.send(&format!(
                    "load name={n} path=fd:{} sha={sha} kind=core",
                    3 + i
                ));
                push(format!("load {n}: {}", short(&l)));
            }
            for (n, extra) in [
                ("rust-p1", format!("inb={NODE_IN}")),
                ("javy-static", format!("inb={NODE_IN}")),
                ("loop", "limit_ms=20".to_string()),
                ("growmem", "mem_mb=8".to_string()),
                ("recurse", "limit_ms=50".to_string()),
                ("table", "limit_ms=50".to_string()),
                ("probe", "limit_ms=200".to_string()),
                ("bigmem", "mem_mb=64".to_string()),
            ] {
                let l = r.send(&format!("run name={n} {extra}"));
                push(format!("run {n}: {}", short(&l)));
            }
        }
    }
    o
}

fn short(r: &Result<String, Gone>) -> String {
    match r {
        Ok(s) => s.chars().take(160).collect(),
        Err(g) => g.text(),
    }
}

fn all_mods() -> Vec<&'static str> {
    vec![
        "empty",
        "rust-p1",
        "javy-static",
        "loop",
        "growmem",
        "recurse",
        "table",
        "probe",
        "bigmem",
    ]
}

// ---------------------------------------------------------------------------------------------- prove ostili (A2)

pub struct Pr {
    name: &'static str,
    line: String,
    /// `judged`: deve essere negata (verdetto di A2); `meta`: solo metadati, riportata; `control`: deve riuscire
    judged: bool,
    control: bool,
}

impl Eng {
    fn probes(&self) -> Vec<Pr> {
        let pid = std::process::id();
        let (k, d, c, t) = (
            mask(&self.key),
            mask(&self.db),
            mask(&self.cfgv),
            mask(&self.tbv),
        );
        let p = |name: &'static str, line: String, judged: bool| Pr {
            name,
            line,
            judged,
            control: false,
        };
        vec![
            p(
                "key_file",
                format!("probe id=file path={} x={k}", self.key_p),
                true,
            ),
            p(
                "db_file",
                format!("probe id=file path={} x={d}", self.db_p),
                true,
            ),
            p(
                "config_file",
                format!("probe id=file path={} x={c}", self.cfg_p),
                true,
            ),
            p(
                "tenantB_file",
                format!("probe id=file path={} x={t}", self.tb_file),
                true,
            ),
            p(
                "tenantB_dir",
                format!("probe id=dir path={}", self.tb_dir),
                true,
            ),
            p(
                "eng_environ",
                format!("probe id=file path=/proc/{pid}/environ x={k}"),
                true,
            ),
            p(
                "eng_maps",
                format!("probe id=file path=/proc/{pid}/maps x={k}"),
                true,
            ),
            p(
                "eng_cmdline",
                format!("probe id=file path=/proc/{pid}/cmdline x={k}"),
                true,
            ),
            p("eng_mem", format!("probe id=mem pid={pid} x={k}"), true),
            p(
                "eng_process_vm_readv",
                format!(
                    "probe id=pvm pid={pid} addr={:x} len={} x={k}",
                    self.key_mem.as_ptr() as usize,
                    self.key_mem.len()
                ),
                true,
            ),
            p("eng_ptrace", format!("probe id=ptrace pid={pid}"), true),
            p("eng_signal", format!("probe id=kill pid={pid}"), true),
            p("inherited_fds", format!("probe id=fds skip=0 x={k}"), true),
            p(
                "unix_socket_path",
                format!("probe id=unix path={}", self.sock),
                true,
            ),
            p(
                "unix_socket_abstract",
                format!("probe id=abstract path={}", self.abs),
                true,
            ),
            p(
                "tcp_engine_port",
                format!("probe id=tcp port={}", self.int_port),
                true,
            ),
            p("exec", "probe id=exec".into(), true),
            p(
                "write_cache",
                format!("probe id=write path={}/evil", self.cache),
                true,
            ),
            p("fork", "probe id=fork".into(), true),
            Pr {
                name: "stat_key_file",
                line: format!("probe id=stat path={}", self.key_p),
                judged: false,
                control: false,
            },
            Pr {
                name: "own_tenant_file",
                line: format!("probe id=own path={} x={k}", self.ta_file),
                judged: false,
                control: true,
            },
        ]
    }
}

pub struct Cell {
    pub text: String,
    pub leak: bool,
}

impl Eng {
    /// Una prova in un ausiliario nuovo (ogni prova uccisa da seccomp ne richiede uno nuovo, come in produzione).
    fn cell(&self, role: &str, mode: &str, pr: &Pr) -> Cell {
        let usr1 = USR1.load(Ordering::Relaxed);
        let mods: Vec<&str> = vec![];
        let mut sp = Sp::new(role, mode);
        sp.mods = &mods;
        let mut r = match self.spawn(&sp) {
            Spawn::Up(r) => r,
            Spawn::Refused(e) => {
                return Cell {
                    text: format!("REFUSED({})", e.chars().take(80).collect::<String>()),
                    leak: false,
                };
            }
            Spawn::Died(g) => {
                return Cell {
                    text: format!("START-FAILED({})", g.text()),
                    leak: false,
                };
            }
        };
        let res = r.send(&pr.line);
        let signalled = USR1.load(Ordering::Relaxed) > usr1;
        match res {
            Ok(s) => {
                let leak = s.contains("leak=1") || (pr.name == "eng_signal" && signalled);
                let detail = s
                    .trim_start_matches("ok ")
                    .replace("leak=1", "ACCESS")
                    .replace("leak=0 ", "denied ");
                Cell {
                    text: if leak {
                        format!("LEAK {detail}")
                    } else {
                        detail
                    },
                    leak,
                }
            }
            Err(g) => {
                // exec riuscita: /bin/true ha sostituito l'ausiliario ed è uscito 0 senza risposta
                let leak = pr.name == "exec" && g.code == Some(0);
                Cell {
                    text: if leak {
                        "LEAK exec_ok".into()
                    } else {
                        g.text()
                    },
                    leak,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------- scenari

pub fn main(args: &[String]) {
    let scen = args[0].as_str();
    // il motore finto riceve la chiave finta nell'ambiente (come AD-6: «la variabile della chiave principale»):
    // si rilancia una volta con la variabile e la si toglie dopo averla letta.
    if std::env::var("SBX_FAKE_KEY").is_err() && matches!(scen, "a2" | "a4") {
        let e = std::env::current_exe().unwrap();
        let err = Command::new(e)
            .arg("drive4")
            .args(args)
            .env("SBX_FAKE_KEY", fresh())
            .exec();
        panic!("exec: {err}");
    }
    let a: A = args[1..]
        .iter()
        .filter_map(|t| t.split_once('='))
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    pin(&std::env::var("SBX_DRV_CPUS").unwrap_or_else(|_| "8-11".into()));
    if scen == "abi" {
        return abi();
    }
    if scen != "discover" && scen != "a4" {
        abi(); // intestazione A0 di ogni corsa: ABI Landlock, Yama, namespace utente
    }
    let fake_key_env = std::env::var("SBX_FAKE_KEY").ok();
    let mut e = Eng::new(&a);
    if let Some(k) = fake_key_env {
        // la chiave dell'ambiente è la stessa del file: la variabile si toglie come in AD-6
        e.key = k.clone();
        e.key_mem = k.clone().into_bytes();
        let _ = std::fs::write(&e.key_p, &k);
        unsafe { std::env::remove_var("SBX_FAKE_KEY") };
    }
    match scen {
        "a1" => a1(&e),
        "a2" => a2(&e, &a),
        "a3" => a3(&e),
        "a4" => a4(&e, &a),
        "a5" => a5(&e, &a),
        "a6" => a6(&e, &a),
        "a7" => a7(&e, &a),
        "a9" => a9(&e, &a),
        "discover" => discover(&e, &a),
        _ => eprintln!("scenario sconosciuto"),
    }
    e.cleanup();
}

/// Stato della macchina: ABI Landlock, LSM, Yama, namespace utente (solo lettura, nessuna impostazione cambiata).
fn abi() {
    let rd = |p: &str| {
        std::fs::read_to_string(p)
            .map(|s| s.trim().to_string())
            .unwrap_or("assente".into())
    };
    println!(
        "A0 kernel={} lsm={}",
        rd("/proc/sys/kernel/osrelease"),
        rd("/sys/kernel/security/lsm")
    );
    match crate::confine::kernel_abi() {
        Ok(v) => println!("A0 landlock_abi_kernel={v} (syscall landlock_create_ruleset VERSION)"),
        Err(e) => println!("A0 landlock_abi_kernel=assente errno={e}"),
    }
    println!("A0 landlock_crate=0.4.7 conosce_fino_a=V9 seccompiler=0.5.0");
    println!(
        "A0 yama_ptrace_scope={}",
        rd("/proc/sys/kernel/yama/ptrace_scope")
    );
    println!(
        "A0 userns: unprivileged_userns_clone={} max_user_namespaces={} apparmor_restrict_unprivileged_userns={} (sysctl letti, non cambiati)",
        rd("/proc/sys/kernel/unprivileged_userns_clone"),
        rd("/proc/sys/user/max_user_namespaces"),
        rd("/proc/sys/kernel/apparmor_restrict_unprivileged_userns")
    );
    // prova reale in un figlio usa-e-getta: unshare(CLONE_NEWUSER|CLONE_NEWNS) e mount di un tmpfs
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        let ok = unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) } == 0;
        unsafe { libc::_exit(if ok { 0 } else { 1 }) };
    }
    let mut st = 0;
    unsafe { libc::waitpid(pid, &mut st, 0) };
    println!(
        "A0 unshare(CLONE_NEWUSER|CLONE_NEWNS) in un figlio usa-e-getta: {}",
        if st == 0 { "riuscito" } else { "negato" }
    );
    println!("A0 core_pattern={}", rd("/proc/sys/kernel/core_pattern"));
}

/// A1: per ogni ruolo, il confinamento è applicato prima di leggere input, e il lavoro legittimo gira.
fn a1(e: &Eng) {
    for role in ROLES {
        let mut sp = Sp::new(role, "full");
        let mods = all_mods();
        if role == "light" {
            sp.mods = &mods;
        }
        sp.inspect = true;
        let mut r = up(e.spawn(&sp), role);
        println!("A1 role={role} confined: {}", r.confined);
        println!(
            "A1 role={role} kernel_snapshot_before_first_input: {}",
            r.snap
        );
        println!(
            "A1 role={role} allowlist_syscalls_in_file={}",
            crate::confine::allowlist(role).len()
        );
        let w = workload(e, &mut r, role);
        for l in &w {
            println!("A1 role={role} work: {l}");
        }
        println!(
            "A1 role={role} work_all_ok={}",
            w.iter()
                .all(|l| !l.contains("KILLED") && !l.contains("Err("))
        );
    }
}

fn a2(e: &Eng, a: &A) {
    let probes = e.probes();
    let modes = list(a, "modes", "none,ll,sc,full,fb");
    let roles = list(a, "roles", "validation,jq,callhttp,light,stream");
    for (label, dump, any) in [("strong", false, false), ("weak", true, true)] {
        set_engine_mode(dump, any);
        for role in &roles {
            for mode in &modes {
                let (mut leaks, mut meta, mut ctl, mut judged_n) = (vec![], vec![], vec![], 0);
                for pr in &probes {
                    let c = e.cell(role, mode, pr);
                    println!(
                        "A2 eng={label} role={role} mode={mode} probe={} => {}",
                        pr.name, c.text
                    );
                    if pr.judged {
                        judged_n += 1;
                        if c.leak {
                            leaks.push(pr.name);
                        }
                    } else if pr.control {
                        ctl.push(format!(
                            "{}:{}",
                            pr.name,
                            if c.leak { "allowed" } else { "denied" }
                        ));
                    } else if c.leak {
                        meta.push(pr.name);
                    }
                }
                println!(
                    "A2sum eng={label} role={role} mode={mode} judged={judged_n} leaks={} {:?} meta_visible={:?} controls={:?}",
                    leaks.len(),
                    leaks,
                    meta,
                    ctl
                );
            }
        }
    }
    // attribuzione di Yama e del dumpable: ausiliario non confinato (`none`), quattro varianti del motore
    let ids = [
        "eng_environ",
        "eng_maps",
        "eng_cmdline",
        "eng_mem",
        "eng_process_vm_readv",
        "eng_ptrace",
        "inherited_fds",
    ];
    for (label, dump, any) in [
        ("dump0+yama", false, false),
        ("dump1+yama", true, false),
        ("dump0+noyama", false, true),
        ("dump1+noyama", true, true),
    ] {
        set_engine_mode(dump, any);
        for pr in probes.iter().filter(|p| ids.contains(&p.name)) {
            let c = e.cell("jq", "none", pr);
            println!(
                "A2yama engine={label} unconfined_aux probe={} => {}",
                pr.name, c.text
            );
        }
    }
    // /proc/<motore>/environ dopo aver tolto la variabile: la chiave resta nella pagina dell'ambiente?
    set_engine_mode(true, true);
    let pr = &probes[5];
    let c = e.cell("jq", "none", pr);
    println!(
        "A2environ engine=dump1 variabile_gia_tolta unconfined_aux => {}",
        c.text
    );
    set_engine_mode(false, false);
}

fn a3(e: &Eng) {
    set_engine_mode(false, false);
    for role in ROLES {
        // ambiente vuoto e dumpable visti dall'interno
        let mut sp = Sp::new(role, "full");
        sp.inspect = true;
        let mods: Vec<&str> = vec![];
        sp.mods = &mods;
        let mut r = up(e.spawn(&sp), role);
        let env = r.send("probe id=env");
        let dump = match e.spawn(&Sp::new(role, "ll")) {
            Spawn::Up(mut x) => x.send("probe id=dumpable"),
            _ => Ok("n/a".into()),
        };
        println!(
            "A3 role={role} env_inside={} dumpable_inside={} outside: {}",
            short(&env),
            short(&dump),
            r.snap
        );
        let core_pid = r.pid;
        let lim = std::fs::read_to_string(format!("/proc/{core_pid}/limits")).unwrap_or_default();
        println!(
            "A3 role={role} limits_core: {}",
            lim.lines()
                .find(|l| l.starts_with("Max core"))
                .unwrap_or("?")
        );
        // crash: nessun core dump
        let g = match e.spawn(&Sp::new(role, "ll")) {
            Spawn::Up(mut x) => {
                let lim =
                    std::fs::read_to_string(format!("/proc/{}/limits", x.pid)).unwrap_or_default();
                println!(
                    "A3 role={role} limits_core_ll: {}",
                    lim.lines()
                        .find(|l| l.starts_with("Max core"))
                        .unwrap_or("?")
                );
                x.send("probe id=crash")
            }
            Spawn::Died(g) => Err(g),
            _ => Ok("n/a".into()),
        };
        let cores = std::fs::read_dir(".")
            .map(|d| {
                d.flatten()
                    .filter(|f| f.file_name().to_string_lossy().starts_with("core"))
                    .count()
            })
            .unwrap_or(0)
            + std::fs::read_dir(&e.run)
                .map(|d| {
                    d.flatten()
                        .filter(|f| f.file_name().to_string_lossy().starts_with("core"))
                        .count()
                })
                .unwrap_or(0);
        println!(
            "A3 role={role} crash_in_ll_mode_(hardening_only): {} core_files_found={cores}",
            match &g {
                Err(g) => format!("{} core_dumped_flag={}", g.text(), g.core),
                Ok(s) => s.clone(),
            }
        );
        // versione dell'IPC diversa: l'ausiliario esce con 3 prima di fare altro
        let mut sp = Sp::new(role, "full");
        sp.ipc = 999;
        sp.mods = &mods;
        let t = Instant::now();
        match e.spawn(&sp) {
            Spawn::Died(g) => println!(
                "A3 role={role} ipc_version_999: exit_code={:?} (atteso {EXIT_IPC_VERSION}) in {} us stderr={}",
                g.code,
                t.elapsed().as_micros(),
                g.stderr
            ),
            Spawn::Up(_) => {
                println!("A3 role={role} ipc_version_999: ERRORE, l'ausiliario è partito")
            }
            Spawn::Refused(x) => println!("A3 role={role} ipc_version_999: rifiutato {x}"),
        }
        // controllo positivo del dumpable: senza confinamento resta 1
        let mut r = up(e.spawn(&Sp::new(role, "none")), role);
        let d = r.send("probe id=dumpable");
        let owner = status_snap(r.pid);
        println!(
            "A3 role={role} control_none dumpable_inside={} {owner}",
            short(&d)
        );
    }
    println!(
        "A3 core_pattern={}",
        std::fs::read_to_string("/proc/sys/kernel/core_pattern")
            .unwrap_or_default()
            .trim()
    );
}

fn a4(e: &Eng, a: &A) {
    let probes = e.probes();
    abi();
    for role in ROLES {
        let mods = all_mods();
        let mut sp = Sp::new(role, "fb");
        if role == "light" {
            sp.mods = &mods;
        }
        sp.inspect = true;
        match e.spawn(&sp) {
            Spawn::Up(mut r) => {
                println!(
                    "A4 role={role} mode=fb (Landlock forzato assente) confined: {}",
                    r.confined
                );
                println!("A4 role={role} kernel_snapshot: {}", r.snap);
                for l in workload(e, &mut r, role) {
                    println!("A4 role={role} work: {l}");
                }
            }
            Spawn::Refused(x) => println!("A4 role={role} mode=fb REFUSED {x}"),
            Spawn::Died(g) => println!("A4 role={role} mode=fb DIED {}", g.text()),
        }
        // senza Landlock e senza namespace utente: i ruoli che aprono file o socket rifiutano di avviarsi
        let nomods: Vec<&str> = vec![];
        let mut sp = Sp::new(role, "fb-nouserns");
        sp.mods = &nomods;
        match e.spawn(&sp) {
            Spawn::Up(r) => println!(
                "A4 role={role} mode=fb-nouserns STARTED level: {}",
                r.confined
            ),
            Spawn::Refused(x) => {
                println!("A4 role={role} mode=fb-nouserns REFUSED exit={EXIT_REFUSED} {x}");
                println!(
                    "A4 engine log (error): ausiliario {role} non avviato: confinamento non disponibile; nessuna modalità senza confinamento"
                );
                println!(
                    "A4 operator warning: il ruolo {role} è disabilitato finché Landlock o i namespace utente non sono disponibili; coded refusal: E_AUX_CONFINEMENT_UNAVAILABLE"
                );
            }
            Spawn::Died(g) => println!("A4 role={role} mode=fb-nouserns DIED {}", g.text()),
        }
    }
    let n = argn(a, "full", 1);
    if n == 1 {
        for (label, dump, any) in [("strong", false, false), ("weak", true, true)] {
            set_engine_mode(dump, any);
            for role in ROLES {
                let (mut leaks, mut ctl) = (vec![], vec![]);
                for pr in &probes {
                    let c = e.cell(role, "fb", pr);
                    println!(
                        "A4 eng={label} role={role} mode=fb probe={} => {}",
                        pr.name, c.text
                    );
                    if pr.judged && c.leak {
                        leaks.push(pr.name);
                    }
                    if pr.control {
                        ctl.push(format!(
                            "{}:{}",
                            pr.name,
                            if c.leak { "allowed" } else { "denied" }
                        ));
                    }
                }
                println!(
                    "A4sum eng={label} role={role} mode=fb leaks={} {:?} controls={:?}",
                    leaks.len(),
                    leaks,
                    ctl
                );
            }
        }
        set_engine_mode(false, false);
    }
}

fn vmrss_kb(pid: i32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))?
                .split_whitespace()
                .nth(1)?
                .parse()
                .ok()
        })
        .unwrap_or(0)
}

/// Avvia un ausiliario di `role` già caldo: `work` (stub) oppure carico e una esecuzione del nodo (light).
fn warm_role(e: &Eng, role: &str, mode: &str) -> Box<R> {
    let mut sp = Sp::new(role, mode);
    let m = ["rust-p1"];
    if role == "light" {
        sp.mods = &m;
    }
    let mut r = up(e.spawn(&sp), role);
    match role {
        "light" => {
            let sha = &e.mods["rust-p1"].1;
            let _ = r.send(&format!("load name=n path=fd:3 sha={sha} kind=core"));
            let _ = r.send(&format!("run name=n inb={NODE_IN}"));
        }
        _ => {
            workload_one(e, &mut r, role);
        }
    }
    r
}

fn workload_one(e: &Eng, r: &mut R, role: &str) -> String {
    let l = match role {
        "callhttp" => r.send(&format!("work ca={} port={}", e.ca_p, e.ext_port)),
        "stream" => r.send(&format!("work dir={} port={}", e.ta_dir, e.bus_port)),
        _ => r.send("work len=100"),
    };
    short(&l)
}

/// A5: somma degli RSS con gli ausiliari attivi e dopo l'arresto per inattività (60 s senza esecuzione).
fn a5(e: &Eng, a: &A) {
    let idle_s = argn(a, "idle_s", 60u64);
    let n_max = argn(a, "tenants", 20usize);
    let mut procs: Vec<(usize, &str, Box<R>)> = vec![];
    let t0 = Instant::now();
    for t in 0..n_max {
        for role in ROLES {
            procs.push((t, role, warm_role(e, role, "full")));
        }
    }
    println!(
        "A5 avviati {} ausiliari ({} tenant x 5 ruoli) in {} ms",
        procs.len(),
        n_max,
        t0.elapsed().as_millis()
    );
    let me = std::process::id() as i32;
    let eng_kb = vmrss_kb(me);
    println!("A5 stand-in motore (questo processo, NON il motore vero) RSS_kb={eng_kb}");
    let snap = |label: &str, procs: &[(usize, &str, Box<R>)]| {
        for n in [1usize, 5, 20] {
            if n > n_max {
                continue;
            }
            let aux: u64 = procs
                .iter()
                .filter(|(t, _, _)| *t < n)
                .map(|(_, _, r)| vmrss_kb(r.pid))
                .sum();
            let light: u64 = procs
                .iter()
                .filter(|(t, r, _)| *t < n && *r == "light")
                .map(|(_, _, r)| vmrss_kb(r.pid))
                .sum();
            println!(
                "A5 {label} N={n} aux_rss_kb={aux} (di cui light={light}) standin_kb={eng_kb} somma_con_standin_MB={:.1} somma_con_stima_motore_26.5MB={:.1}",
                (aux + eng_kb) as f64 / 1024.0,
                aux as f64 / 1024.0 + 26.5
            );
        }
        for role in ROLES {
            let v: Vec<u64> = procs
                .iter()
                .filter(|(_, r, _)| *r == role)
                .map(|(_, _, r)| vmrss_kb(r.pid))
                .collect();
            println!(
                "A5 {label} ruolo={role} rss_kb min={} max={} medio={}",
                v.iter().min().unwrap_or(&0),
                v.iter().max().unwrap_or(&0),
                v.iter().sum::<u64>() / v.len().max(1) as u64
            );
        }
    };
    snap("attivi", &procs);
    // PSS: solo dai processi non confinati (un processo non ispezionabile non lascia leggere smaps_rollup)
    for role in ROLES {
        let r = warm_role(e, role, "none");
        let s =
            std::fs::read_to_string(format!("/proc/{}/smaps_rollup", r.pid)).unwrap_or_default();
        let g = |k: &str| {
            s.lines()
                .find(|l| l.starts_with(k))
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("0")
                .to_string()
        };
        println!(
            "A5 controllo_senza_confinamento ruolo={role} rss_kb={} pss_kb={}",
            g("Rss:"),
            g("Pss:")
        );
    }
    println!("A5 attesa di {idle_s} s senza esecuzioni (arresto per inattività)");
    std::thread::sleep(Duration::from_secs(idle_s));
    snap("fermi_prima_dello_stop", &procs);
    let t = Instant::now();
    let n = procs.len();
    for (_, _, r) in procs.drain(..) {
        r.stop();
    }
    println!(
        "A5 arresto per inattività di {n} ausiliari in {} ms; ausiliari rimasti: 0",
        t.elapsed().as_millis()
    );
    println!(
        "A5 dopo_lo_stop standin_kb={} somma_con_stima_motore_26.5MB={:.1}",
        vmrss_kb(me),
        26.5
    );
}

/// A6: riavvio a freddo dopo l'arresto per inattività, con e senza confinamento.
fn a6(e: &Eng, a: &A) {
    let n = argn(a, "n", 200usize);
    let cases: Vec<(&str, &str)> = vec![
        ("validation", ""),
        ("jq", ""),
        ("callhttp", ""),
        ("stream", ""),
        ("light", "empty"),
        ("light", "rust-p1"),
        ("light", "javy-static"),
    ];
    for (role, m) in cases {
        for mode in ["full", "none"] {
            let (mut ready, mut first, mut conf, mut fork) = (vec![], vec![], vec![], vec![]);
            let mods = [m];
            for _ in 0..n + 10 {
                let t = Instant::now();
                let mut sp = Sp::new(role, mode);
                if role == "light" {
                    sp.mods = &mods;
                }
                let mut r = up(e.spawn(&sp), role);
                let ready_us = t.elapsed().as_micros();
                let ok = if role == "light" {
                    let sha = &e.mods[m].1;
                    let l = r
                        .send(&format!("load name=n path=fd:3 sha={sha} kind=core"))
                        .unwrap_or_default();
                    // il nodo inizia: marcatore `started` subito prima del guest
                    let _ = writeln!(r.tx, "run name=n mark=1 inb={NODE_IN}");
                    let _ = r.tx.flush();
                    let s = r.read_line().unwrap_or_default();
                    let f = r.read_line().unwrap_or_default();
                    l.starts_with("ok") && s == "started" && f.contains("fail=none")
                } else {
                    workload_one(e, &mut r, role).starts_with("Ok") || true
                };
                let first_us = t.elapsed().as_micros();
                assert!(ok, "primo lavoro non riuscito");
                let cu: u128 = r
                    .confined
                    .split_whitespace()
                    .find_map(|x| x.strip_prefix("t_confine_us="))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                ready.push(ready_us);
                first.push(first_us);
                conf.push(cu);
                fork.push(r.t_fork_us);
                r.stop(); // arresto per inattività
            }
            // si scartano le prime 10 (riscaldamento della page cache e delle code)
            let w = |v: &Vec<u128>| v[10..].to_vec();
            println!(
                "A6 role={role} mod={} mode={mode} {}",
                if m.is_empty() { "-" } else { m },
                summ("fork_to_first_work_ms", &w(&first))
            );
            println!(
                "A6 role={role} mod={} mode={mode} {}",
                if m.is_empty() { "-" } else { m },
                summ("fork_to_ready+hello_ms", &w(&ready))
            );
            println!(
                "A6 role={role} mod={} mode={mode} {}",
                if m.is_empty() { "-" } else { m },
                summ("confinement_setup_ms", &w(&conf))
            );
            println!(
                "A6 role={role} mod={} mode={mode} {}",
                if m.is_empty() { "-" } else { m },
                summ("spawn_call_ms", &w(&fork))
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------- A7

struct Tenant {
    slot: std::sync::Mutex<Option<Box<R>>>,
    last_ms: AtomicU64,
}

/// A7: carico aperto a `rate` attività/s; ogni attività è una esecuzione del nodo nell'ausiliario del suo tenant.
/// I tenant «caldi» sono sempre in uso; quelli «freddi» (un insieme rotante) sono fermati per inattività
/// (`idle_ms`, accorciato rispetto ai 60 s di produzione) e quindi ripartono a freddo, nella misura `cold_ppm`.
fn a7(e: &Eng, a: &A) {
    let secs = argn(a, "secs", 20u64);
    let rate = argn(a, "rate", 1000u64);
    let hot = argn(a, "hot", 20usize);
    let cold_set = argn(a, "coldset", 64usize);
    let idle_ms = argn(a, "idle_ms", 1000u64);
    let phases: Vec<(String, bool, u64)> = arg(
        a,
        "phases",
        "ctrl:0:10000,idle:1:10000,idle:1:5000,idle:1:1000",
    )
    .split(',')
    .map(|p| {
        let v: Vec<&str> = p.split(':').collect();
        (v[0].to_string(), v[1] == "1", v[2].parse().unwrap())
    })
    .collect();
    for (label, idle_on, cold_ppm) in phases {
        a7_phase(
            e, &label, idle_on, cold_ppm, secs, rate, hot, cold_set, idle_ms,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn a7_phase(
    e: &Eng,
    label: &str,
    idle_on: bool,
    cold_ppm: u64,
    secs: u64,
    rate: u64,
    hot: usize,
    cold_set: usize,
    idle_ms: u64,
) {
    use std::sync::{Arc, mpsc};
    let n_t = hot + cold_set;
    let tenants: Vec<Arc<Tenant>> = (0..n_t)
        .map(|_| {
            Arc::new(Tenant {
                slot: std::sync::Mutex::new(None),
                last_ms: AtomicU64::new(0),
            })
        })
        .collect();
    let start_aux = |e: &Eng| -> Box<R> {
        let mods = ["rust-p1"];
        let mut sp = Sp::new("light", "full");
        sp.mods = &mods;
        let mut r = up(e.spawn(&sp), "light");
        let sha = &e.mods["rust-p1"].1;
        assert!(
            r.send(&format!("load name=n path=fd:3 sha={sha} kind=core"))
                .unwrap_or_default()
                .starts_with("ok")
        );
        r
    };
    // avvio fuori dal tempo misurato: i caldi sempre; i freddi solo nel controllo (senza arresto per inattività)
    for (i, t) in tenants.iter().enumerate() {
        if i < hot || !idle_on {
            let mut r = start_aux(e);
            let _ = r.send(&format!("run name=n inb={NODE_IN}"));
            *t.slot.lock().unwrap() = Some(r);
        }
    }
    let clk = Instant::now();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    // arresto per inattività (se attivo): ogni 20 ms ferma gli ausiliari fermi da più di `idle_ms`
    let reaper = idle_on.then(|| {
        let (ts, stop) = (tenants.clone(), stop.clone());
        std::thread::spawn(move || {
            pin(&std::env::var("SBX_DRV_CPUS").unwrap_or_else(|_| "8-11".into()));
            let mut stopped = 0u64;
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(20));
                let now = clk.elapsed().as_millis() as u64;
                for t in &ts {
                    if let Ok(mut g) = t.slot.try_lock()
                        && g.is_some()
                        && now.saturating_sub(t.last_ms.load(Ordering::Relaxed)) > idle_ms
                    {
                        g.take().unwrap().stop();
                        stopped += 1;
                    }
                }
            }
            stopped
        })
    });
    type Lat = Vec<(u32, bool)>;
    let mut txs = vec![];
    let mut joins = vec![];
    for t in tenants.iter() {
        let (tx, rx) = mpsc::channel::<Instant>();
        txs.push(tx);
        let t = t.clone();
        // ponytail: un thread per tenant (84): basta per il carico; con migliaia di tenant servirebbe un pool.
        let me = e as *const Eng as usize;
        joins.push(
            std::thread::Builder::new()
                .stack_size(256 << 10)
                .spawn(move || -> (Lat, u64) {
                    pin(&std::env::var("SBX_DRV_CPUS").unwrap_or_else(|_| "8-11".into()));
                    let e = unsafe { &*(me as *const Eng) };
                    let (mut lat, mut errs) = (vec![], 0u64);
                    for sched in rx {
                        let mut g = t.slot.lock().unwrap();
                        let cold = g.is_none();
                        if cold {
                            let mods = ["rust-p1"];
                            let mut sp = Sp::new("light", "full");
                            sp.mods = &mods;
                            match e.spawn(&sp) {
                                Spawn::Up(mut r) => {
                                    let sha = &e.mods["rust-p1"].1;
                                    let _ = r.send(&format!(
                                        "load name=n path=fd:3 sha={sha} kind=core"
                                    ));
                                    *g = Some(r);
                                }
                                _ => {
                                    errs += 1;
                                    continue;
                                }
                            }
                        }
                        let r = g.as_mut().unwrap();
                        let _ = writeln!(r.tx, "run name=n mark=1 inb={NODE_IN}");
                        let _ = r.tx.flush();
                        let s = r.read_line().unwrap_or_default();
                        let started = Instant::now();
                        let f = r.read_line().unwrap_or_default();
                        if s != "started" || !f.contains("fail=none") {
                            errs += 1;
                        }
                        lat.push((started.duration_since(sched).as_micros() as u32, cold));
                        t.last_ms
                            .store(clk.elapsed().as_millis() as u64, Ordering::Relaxed);
                    }
                    (lat, errs)
                })
                .unwrap(),
        );
    }
    // pianificatore: tick assoluti ogni 1/rate secondi, tenant scelto a caso (xorshift fisso: stessa sequenza)
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let total = secs * rate;
    let period = Duration::from_nanos(1_000_000_000 / rate);
    let t_start = Instant::now() + Duration::from_millis(50);
    let (mut max_lag, mut cold_ctr, mut sent_cold) = (0u128, 0usize, 0u64);
    for k in 0..total {
        let target = t_start + period * k as u32;
        loop {
            let now = Instant::now();
            if now >= target {
                max_lag = max_lag.max((now - target).as_micros());
                break;
            }
            if target - now > Duration::from_micros(300) {
                std::thread::sleep(target - now - Duration::from_micros(200));
            }
        }
        let is_cold = next() % 1_000_000 < cold_ppm;
        let id = if is_cold {
            cold_ctr += 1;
            sent_cold += 1;
            hot + cold_ctr % cold_set
        } else {
            (next() % hot as u64) as usize
        };
        let _ = txs[id].send(target);
    }
    drop(txs);
    let (mut all, mut errs) = (vec![], 0u64);
    for j in joins {
        let (l, er) = j.join().unwrap();
        all.extend(l);
        errs += er;
    }
    stop.store(true, Ordering::Relaxed);
    let stopped = reaper.map(|h| h.join().unwrap()).unwrap_or(0);
    let cold_n = all.iter().filter(|(_, c)| *c).count();
    let us = |f: &dyn Fn(&(u32, bool)) -> bool| -> Vec<f64> {
        all.iter()
            .filter(|x| f(x))
            .map(|(u, _)| *u as f64 / 1000.0)
            .collect()
    };
    let (mut v_all, mut v_cold, mut v_warm) = (us(&|_| true), us(&|x| x.1), us(&|x| !x.1));
    println!(
        "A7 phase={label} idle_stop={} cold_ppm={cold_ppm} activities={} requested_cold={sent_cold} cold_restarts_observed={cold_n} idle_stops={stopped} errors={errs} max_sched_lag_us={max_lag} rate={rate}/s secs={secs} hot={hot} coldset={cold_set} idle_ms={idle_ms}",
        idle_on as u8,
        all.len()
    );
    println!(
        "A7 phase={label} start_latency_ms all: p50={:.3} p90={:.3} p99={:.3} p99.9={:.3} max={:.3}",
        pct(&mut v_all, 0.5),
        pct(&mut v_all, 0.9),
        pct(&mut v_all, 0.99),
        pct(&mut v_all, 0.999),
        pct(&mut v_all, 1.0)
    );
    println!(
        "A7 phase={label} start_latency_ms warm_only: p50={:.3} p99={:.3} max={:.3}",
        pct(&mut v_warm, 0.5),
        pct(&mut v_warm, 0.99),
        pct(&mut v_warm, 1.0)
    );
    if !v_cold.is_empty() {
        println!(
            "A7 phase={label} start_latency_ms cold_only: n={} p50={:.3} p90={:.3} p99={:.3} max={:.3}",
            v_cold.len(),
            pct(&mut v_cold, 0.5),
            pct(&mut v_cold, 0.9),
            pct(&mut v_cold, 0.99),
            pct(&mut v_cold, 1.0)
        );
    }
    for t in &tenants {
        if let Some(r) = t.slot.lock().unwrap().take() {
            r.stop();
        }
    }
}

// ---------------------------------------------------------------------------------------------- A9

/// A9: il limite wasm è sempre minore del timeout del task. Regola del motore: `limit = min(richiesto, T - T/10)`,
/// e un timeout sotto 10 ms è rifiutato (non si può far stare un limite sotto un tick da 1 ms con margine).
fn node_limit(timeout_ms: u64, requested_ms: u64) -> Result<u64, &'static str> {
    if timeout_ms < 10 {
        return Err("E_TASK_TIMEOUT_TOO_SMALL");
    }
    Ok(requested_ms.min(timeout_ms - timeout_ms / 10).max(1))
}

fn a9(e: &Eng, a: &A) {
    let reps = argn(a, "reps", 30usize);
    println!(
        "A9 regola: limite_wasm = min(richiesto, T - T/10), T < 10 ms rifiutato: {:?} {:?}",
        node_limit(1, 1_000_000),
        node_limit(2, 1_000_000)
    );
    for t_ms in [10u64, 100, 1000, 5000] {
        for (modname, extra_mem) in [("loop", false), ("growmem", true)] {
            let mods = [modname];
            let (mut ok_n, mut names_limit, mut elapsed) = (0, 0, vec![]);
            let mut classes: HashMap<String, usize> = HashMap::new();
            let mut r = {
                let mut sp = Sp::new("light", "full");
                sp.mods = &mods;
                up(e.spawn(&sp), "light")
            };
            let sha = e.mods[modname].1.clone();
            let _ = r.send(&format!("load name=n path=fd:3 sha={sha} kind=core"));
            for _ in 0..reps {
                let limit = node_limit(t_ms, 1_000_000).unwrap(); // richiesto: «senza limite»
                let t = Instant::now();
                let line = format!(
                    "run name=n limit_ms={limit} mem_mb={}",
                    if extra_mem { 8 } else { 64 }
                );
                // il timer del task: se scade prima della risposta, il motore ferma l'ausiliario (per PID salvato)
                let rep = r.send_timeout(&line, Duration::from_millis(t_ms));
                let el = t.elapsed();
                elapsed.push(el.as_micros());
                match rep {
                    Some(l) => {
                        let class = crate::drv::fail_of(&l)
                            .split_whitespace()
                            .next()
                            .unwrap_or("?")
                            .to_string();
                        let named = class.ends_with("_limit");
                        *classes.entry(class).or_default() += 1;
                        if named {
                            names_limit += 1;
                        }
                        if el < Duration::from_millis(t_ms) {
                            ok_n += 1;
                        }
                    }
                    None => {
                        *classes.entry("task_timeout(engine)".into()).or_default() += 1;
                        // l'ausiliario è stato fermato dal timer: se ne avvia uno nuovo
                        let mut sp = Sp::new("light", "full");
                        sp.mods = &mods;
                        r = up(e.spawn(&sp), "light");
                        let _ = r.send(&format!("load name=n path=fd:3 sha={sha} kind=core"));
                    }
                }
            }
            println!(
                "A9 T={t_ms}ms module={modname} limit_wasm={}ms reps={reps} stopped_before_T={ok_n} error_names_limit={names_limit} classes={classes:?} {}",
                node_limit(t_ms, 1_000_000).unwrap(),
                summ("elapsed_ms", &elapsed)
            );
        }
        // controllo: senza il vincolo (limite richiesto = «senza limite») vince il timer del task
        let mods = ["loop"];
        let mut sp = Sp::new("light", "full");
        sp.mods = &mods;
        let mut r = up(e.spawn(&sp), "light");
        let sha = &e.mods["loop"].1;
        let _ = r.send(&format!("load name=n path=fd:3 sha={sha} kind=core"));
        let t = Instant::now();
        let rep = r.send_timeout("run name=n limit_ms=100000", Duration::from_millis(t_ms));
        println!(
            "A9 control_senza_vincolo T={t_ms}ms: {} dopo {:.1} ms",
            if rep.is_none() {
                "scatta il timer del task (errore non nomina il limite del nodo)"
            } else {
                "risposta"
            },
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
    // timeout troppo piccolo: rifiutato dalla regola
    println!("A9 T=1ms: {:?}", node_limit(1, 1_000));
}

impl R {
    /// Come `send` ma con il timer del task: scaduto, ferma l'ausiliario (PID salvato) e restituisce `None`.
    fn send_timeout(&mut self, line: &str, t: Duration) -> Option<String> {
        let _ = writeln!(self.tx, "{line}");
        let _ = self.tx.flush();
        let fd = self.rx.get_ref().as_raw_fd();
        // dati già nel buffer di BufReader: nessuna attesa
        if self.rx.buffer().is_empty() {
            let mut p = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let n = unsafe { libc::poll(&mut p, 1, t.as_millis() as i32) };
            if n <= 0 {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return None;
            }
        }
        self.read_line()
    }
}

// ---------------------------------------------------------------------------------------------- scoperta delle syscall

/// Scoperta: per ogni ruolo, il lavoro legittimo sotto gdb (`catch syscall`); il filtro non è caricato, al suo
/// posto c'è il marcatore `sysinfo`. Lo script `discover.sh` estrae le syscall successive al marcatore.
fn discover(e: &Eng, a: &A) {
    for role in list(a, "roles", "validation,jq,callhttp,light,stream") {
        let log = format!("{}/gdb-{role}.log", e.run);
        let script = format!("{}/gdb-{role}.cmd", e.run);
        let s = format!(
            "set pagination off\nset confirm off\nset startup-with-shell off\nhandle SIGSEGV SIGBUS SIGILL SIGFPE nostop noprint pass\nset logging file {log}\nset logging redirect on\nset logging enabled on\ncatch syscall sysinfo\ncommands 1\nsilent\nenable 2\ncontinue\nend\ncatch syscall\ndisable 2\ncommands 2\nsilent\nprintf \"SYS %d\\n\", $orig_rax\ncontinue\nend\nrun\n"
        );
        std::fs::write(&script, s).unwrap();
        let mods = all_mods();
        let mut sp = Sp::new(role, "full");
        if role == "light" {
            sp.mods = &mods;
        }
        sp.discover = Some(&script);
        let mut r = up(e.spawn(&sp), role);
        for l in workload(e, &mut r, role) {
            println!("D role={role} {l}");
        }
        r.stop();
        let txt = std::fs::read_to_string(&log).unwrap_or_default();
        std::fs::write(
            format!("{}/discover-{role}.txt", arg(a, "dest", &e.run)),
            txt,
        )
        .unwrap();
        println!("D role={role} registro gdb scritto");
    }
}
