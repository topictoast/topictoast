//! Sottocomando `role <nome> k=v ...` (spike S-2, fase 4): un ruolo ausiliario di AD-6. Si confina PRIMA di
//! leggere una sola riga di input (`confine::apply`), poi serve il protocollo a righe su stdin/stdout.
//! `light` è la sandbox leggera vera (stesso `serve` della fase 2, con wasmtime); gli altri quattro ruoli
//! (`validation`, `jq`, `callhttp`, `stream`) sono **stub**: misurano il confinamento, non il lavoro del ruolo.
//! Codice da buttare.

use crate::aux::{Rig, serve};
use crate::confine::{self, Cfg, EXIT_IPC_VERSION, EXIT_REFUSED, IPC_VERSION, Mode};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::sync::OnceLock;

static CFG: OnceLock<Cfg> = OnceLock::new();
static USERNS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn list(s: Option<&&str>) -> Vec<String> {
    s.map(|v| {
        v.split(',')
            .filter(|x| !x.is_empty())
            .map(String::from)
            .collect()
    })
    .unwrap_or_default()
}

/// Percorso come lo vede il ruolo: nel ripiego con namespace le cartelle concesse stanno in `/r<n>` e `/d<n>`.
fn mapped(p: &str) -> String {
    let c = CFG.get().unwrap();
    if USERNS.load(std::sync::atomic::Ordering::Relaxed) {
        for (i, d) in c.rw.iter().enumerate() {
            if let Some(rest) = p.strip_prefix(d.as_str()) {
                return format!("/d{i}{rest}");
            }
        }
        for (i, d) in c.ro.iter().enumerate() {
            if let Some(rest) = p.strip_prefix(d.as_str()) {
                return format!("/r{i}{rest}");
            }
        }
    }
    p.to_string()
}

pub fn main(a: &[String]) -> ! {
    let m: HashMap<&str, &str> = a[1..].iter().filter_map(|t| t.split_once('=')).collect();
    let cfg = Cfg {
        role: a[0].clone(),
        mode: m.get("mode").map(|s| Mode::parse(s)),
        ro: list(m.get("ro")),
        rw: list(m.get("rw")),
        tcp: list(m.get("tcp"))
            .iter()
            .filter_map(|p| p.parse().ok())
            .collect(),
        keep: list(m.get("keep"))
            .iter()
            .filter_map(|p| p.parse().ok())
            .collect(),
        scratch: m.get("scratch").map(|s| s.to_string()).unwrap_or_default(),
        discover: m.get("discover") == Some(&"1"),
    };
    // La sandbox leggera crea il motore wasmtime (e il thread dell'epoca) PRIMA del confinamento: il filtro
    // seccomp e Landlock si applicano poi a tutti i thread, e dopo nessuna `clone` è consentita.
    // Prima i descrittori ereditati (altrimenti si chiuderebbero anche epoll ed eventfd del runtime di wasmtime-wasi)
    let pre = confine::harden(&cfg);
    let rig = (cfg.role == "light").then(|| {
        let r = Rig::new("epoch");
        r.rt.warmup();
        r
    });
    CFG.set(cfg.clone()).unwrap();
    let rep = match confine::apply(&cfg, pre) {
        Ok(r) => r,
        Err(why) => {
            eprintln!(
                "refused code=E_AUX_CONFINEMENT_UNAVAILABLE role={} reason={}",
                cfg.role,
                why.replace(' ', "_")
            );
            std::process::exit(EXIT_REFUSED);
        }
    };
    if rep.contains("userns_active=1") {
        USERNS.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let out = std::io::stdout();
    let mut out = out.lock();
    // Da qui in poi si può leggere: l'unico input prima di questo punto è l'argomento di avvio del motore.
    writeln!(out, "confined {rep}").unwrap();
    writeln!(out, "ready").unwrap();
    out.flush().unwrap();
    let inp = std::io::stdin();
    let mut inp = inp.lock();
    let mut hello = String::new();
    inp.read_line(&mut hello).unwrap_or(0);
    let hm: HashMap<&str, &str> = hello
        .split_whitespace()
        .skip(1)
        .filter_map(|t| t.split_once('='))
        .collect();
    if hm.get("ipc").and_then(|v| v.parse::<u32>().ok()) != Some(IPC_VERSION) {
        eprintln!(
            "ipc version mismatch: want {IPC_VERSION}, got {:?}",
            hm.get("ipc")
        );
        std::process::exit(EXIT_IPC_VERSION);
    }
    writeln!(out, "ok hello").unwrap();
    out.flush().unwrap();
    if let Some(rig) = &rig {
        serve(rig, &mut inp, &mut out, 1, false);
    } else {
        stub(&cfg, &mut inp, &mut out);
    }
    std::process::exit(0)
}

/// Ciclo dei ruoli stub. `work` esercita le syscall che il ruolo vero userebbe (file, socket), non il suo lavoro.
fn stub(c: &Cfg, r: &mut dyn BufRead, out: &mut dyn Write) {
    let mut raw = String::new();
    loop {
        raw.clear();
        if r.read_line(&mut raw).unwrap_or(0) == 0 {
            return;
        }
        let line = raw.trim_end();
        let m: HashMap<&str, &str> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|t| t.split_once('='))
            .collect();
        let reply = match line.split_whitespace().next().unwrap_or("") {
            "ping" => "ok".to_string(),
            "quit" => return,
            "work" => work(c, &m),
            "probe" => probe(&m),
            _ => "err unknown command".into(),
        };
        if writeln!(out, "{reply}").and_then(|_| out.flush()).is_err() {
            return;
        }
    }
}

fn tcp_roundtrip(port: u16) -> std::io::Result<usize> {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port))?;
    s.write_all(b"GET / HTTP/1.0\r\n\r\n")?;
    let mut b = String::new();
    s.read_to_string(&mut b)?;
    Ok(b.len())
}

fn work(c: &Cfg, m: &HashMap<&str, &str>) -> String {
    let len: usize = m.get("len").and_then(|v| v.parse().ok()).unwrap_or(0);
    match c.role.as_str() {
        // stub: nessun lavoro vero, solo una risposta in funzione dell'ingresso
        "validation" | "jq" => format!("ok role={} len={len}", c.role),
        "callhttp" => {
            let ca = m
                .get("ca")
                .map(|p| std::fs::read(mapped(p)).map(|b| b.len()))
                .unwrap_or(Ok(0));
            let port: u16 = m.get("port").and_then(|v| v.parse().ok()).unwrap_or(0);
            match (ca, tcp_roundtrip(port)) {
                (Ok(a), Ok(b)) => format!("ok role=callhttp ca_bytes={a} reply_bytes={b}"),
                (a, b) => format!("err ca={a:?} net={b:?}"),
            }
        }
        "stream" => {
            let dir = m.get("dir").map(|p| mapped(p)).unwrap_or_default();
            let f = format!("{dir}/state.bin");
            let res = std::fs::write(&f, vec![7u8; 4096])
                .and_then(|_| std::fs::File::open(&f)?.sync_all())
                .and_then(|_| std::fs::read(&f))
                .and_then(|b| {
                    let port: u16 = m.get("port").and_then(|v| v.parse().ok()).unwrap_or(0);
                    tcp_roundtrip(port).map(|n| (b.len(), n))
                });
            match res {
                Ok((a, b)) => format!("ok role=stream file_bytes={a} reply_bytes={b}"),
                Err(e) => format!("err {e}"),
            }
        }
        _ => "err role".into(),
    }
}

fn errno_name(e: &std::io::Error) -> String {
    match e.raw_os_error() {
        Some(libc::EACCES) => "EACCES".into(),
        Some(libc::EPERM) => "EPERM".into(),
        Some(libc::ENOENT) => "ENOENT".into(),
        Some(libc::ENOSYS) => "ENOSYS".into(),
        Some(libc::EOPNOTSUPP) => "EOPNOTSUPP".into(),
        Some(libc::ESRCH) => "ESRCH".into(),
        Some(libc::ECONNREFUSED) => "ECONNREFUSED".into(),
        Some(libc::EBADF) => "EBADF".into(),
        Some(libc::EMFILE) => "EMFILE".into(),
        Some(n) => format!("errno{n}"),
        None => "?".into(),
    }
}

fn denied(e: &std::io::Error) -> String {
    format!("ok leak=0 err={}", errno_name(e))
}

fn unmask(x: &str) -> Vec<u8> {
    (0..x.len() / 2)
        .filter_map(|i| u8::from_str_radix(&x[2 * i..2 * i + 2], 16).ok())
        .collect()
}

/// Conta le occorrenze del valore sentinella (passato mascherato con XOR 0x5A) in `b`.
fn hits(b: &[u8], masked: &[u8]) -> usize {
    let m = masked.len();
    if m == 0 || b.len() < m {
        return 0;
    }
    (0..=b.len() - m)
        .filter(|&i| b[i..i + m].iter().zip(masked).all(|(x, y)| x ^ 0x5A == *y))
        .count()
}

fn read_file(path: &str, masked: &[u8]) -> String {
    match std::fs::File::open(path) {
        Err(e) => denied(&e),
        Ok(mut f) => {
            let mut b = vec![];
            // ponytail: massimo 8 MiB per prova (basta per i file finti e per environ/maps)
            let _ = (&mut f).take(8 << 20).read_to_end(&mut b);
            format!("ok leak=1 bytes={} hits={}", b.len(), hits(&b, masked))
        }
    }
}

fn connect_ok(fam: i32, addr: *const libc::sockaddr, len: u32) -> String {
    let fd = unsafe { libc::socket(fam, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return denied(&std::io::Error::last_os_error());
    }
    let r = unsafe { libc::connect(fd, addr, len) };
    let e = std::io::Error::last_os_error();
    unsafe { libc::close(fd) };
    if r == 0 {
        "ok leak=1".into()
    } else {
        denied(&e)
    }
}

/// Prove ostili di A2: ogni risposta dice se l'accesso è riuscito (`leak=1`) o l'errno che lo ha negato. Un
/// processo ucciso da seccomp non risponde: lo vede il motore dallo stato di uscita (SIGSYS).
pub fn probe(m: &HashMap<&str, &str>) -> String {
    let id = m.get("id").copied().unwrap_or("");
    let x = unmask(m.get("x").copied().unwrap_or(""));
    let pid: i32 = m.get("pid").and_then(|v| v.parse().ok()).unwrap_or(0);
    let path = m.get("path").copied().unwrap_or("");
    let port: u16 = m.get("port").and_then(|v| v.parse().ok()).unwrap_or(0);
    match id {
        "file" => read_file(path, &x),
        "own" => read_file(&mapped(path), &x),
        "dir" => match std::fs::read_dir(path) {
            Err(e) => denied(&e),
            Ok(d) => format!("ok leak=1 entries={}", d.count()),
        },
        "stat" => match std::fs::metadata(path) {
            Err(e) => denied(&e),
            Ok(md) => format!("ok leak=1 len={}", md.len()),
        },
        "mem" => match crate::rt::scan_mem(&pid.to_string(), &x) {
            Ok((h, r)) => format!("ok leak=1 hits={h} read_bytes={r}"),
            Err(e) => denied(&e),
        },
        "pvm" => {
            let addr =
                usize::from_str_radix(m.get("addr").copied().unwrap_or("0"), 16).unwrap_or(0);
            let len: usize = m.get("len").and_then(|v| v.parse().ok()).unwrap_or(0);
            let mut b = vec![0u8; len];
            let l = libc::iovec {
                iov_base: b.as_mut_ptr().cast(),
                iov_len: len,
            };
            let r = libc::iovec {
                iov_base: addr as *mut _,
                iov_len: len,
            };
            let n = unsafe { libc::process_vm_readv(pid, &l, 1, &r, 1, 0) };
            if n < 0 {
                denied(&std::io::Error::last_os_error())
            } else {
                format!("ok leak=1 bytes={n} hits={}", hits(&b, &x))
            }
        }
        "ptrace" => {
            // SEIZE non manda il SIGSTOP dell'ATTACH: non ferma il motore, ma prova lo stesso permesso di ptrace
            let r = unsafe { libc::ptrace(libc::PTRACE_SEIZE, pid, 0, 0) };
            if r < 0 {
                denied(&std::io::Error::last_os_error())
            } else {
                unsafe { libc::ptrace(libc::PTRACE_DETACH, pid, 0, 0) };
                "ok leak=1".into()
            }
        }
        "kill" => {
            if unsafe { libc::kill(pid, libc::SIGUSR1) } == 0 {
                "ok leak=1".into()
            } else {
                denied(&std::io::Error::last_os_error())
            }
        }
        "fds" => {
            // descrittori ereditati: i `skip` sono quelli che il motore ha passato apposta
            let skip: Vec<i32> = list(m.get("skip"))
                .iter()
                .filter_map(|v| v.parse().ok())
                .collect();
            let (mut readable, mut h) = (0, 0);
            for fd in 3..64 {
                if skip.contains(&fd) {
                    continue;
                }
                let mut b = vec![0u8; 1 << 16];
                let n = unsafe { libc::read(fd, b.as_mut_ptr().cast(), b.len()) };
                if n > 0 {
                    readable += 1;
                    h += hits(&b[..n as usize], &x);
                }
            }
            format!(
                "ok leak={} readable_fds={readable} hits={h}",
                (readable > 0) as u8
            )
        }
        "unix" | "abstract" => {
            let mut a: libc::sockaddr_un = unsafe { std::mem::zeroed() };
            a.sun_family = libc::AF_UNIX as u16;
            let name = path.as_bytes();
            let off = if id == "abstract" { 1 } else { 0 };
            for (i, b) in name.iter().enumerate().take(100) {
                a.sun_path[off + i] = *b as _;
            }
            let len = (2 + off + name.len()) as u32;
            connect_ok(libc::AF_UNIX, (&a as *const libc::sockaddr_un).cast(), len)
        }
        "tcp" => {
            let a = libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: port.to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes([127, 0, 0, 1]),
                },
                sin_zero: [0; 8],
            };
            connect_ok(
                libc::AF_INET,
                (&a as *const libc::sockaddr_in).cast(),
                std::mem::size_of::<libc::sockaddr_in>() as u32,
            )
        }
        "exec" => {
            let p = std::ffi::CString::new("/bin/true").unwrap();
            let argv = [p.as_ptr(), std::ptr::null()];
            unsafe { libc::execv(p.as_ptr(), argv.as_ptr()) };
            denied(&std::io::Error::last_os_error())
        }
        "write" => match std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
        {
            Err(e) => denied(&e),
            Ok(_) => "ok leak=1".into(),
        },
        "fork" => {
            let p = unsafe { libc::fork() };
            if p == 0 {
                unsafe { libc::_exit(0) };
            }
            if p < 0 {
                denied(&std::io::Error::last_os_error())
            } else {
                unsafe { libc::waitpid(p, std::ptr::null_mut(), 0) };
                "ok leak=1".into()
            }
        }
        "env" => format!("ok leak=0 env_vars={}", std::env::vars().count()),
        "dumpable" => format!("ok leak=0 dumpable={}", unsafe {
            libc::prctl(libc::PR_GET_DUMPABLE)
        }),
        "crash" => {
            unsafe { std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 1) };
            "unreachable".into()
        }
        _ => "err unknown probe".into(),
    }
}
