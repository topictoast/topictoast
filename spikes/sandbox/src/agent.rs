//! Agente del guest (fase 3 dello spike S-2): gira nella microVM come PID 1, ascolta sul vsock (porta 5000) e
//! serve lo stesso protocollo a righe dell'ausiliario leggero (`aux::serve`), più il trasferimento di byte.
//! Contiene anche i comandi `native`: codice ostile *nativo* dentro il guest, che simula l'esito peggiore di
//! una fuga dal modulo wasm (codice arbitrario nel processo dell'agente). Esistono solo qui dentro: sull'host
//! `native` è rifiutato due volte (flag `guest` e `sbx_guest=1` nella riga di comando del kernel).
//! Codice da buttare.

use crate::aux::{Rig, serve};
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::os::fd::FromRawFd;
use std::time::{Duration, Instant};

const PORT: u32 = 5000;

fn mount(src: &str, tgt: &str, fs: &str) {
    let (s, t, f) = (cstr(src), cstr(tgt), cstr(fs));
    unsafe { libc::mount(s.as_ptr(), t.as_ptr(), f.as_ptr(), 0, std::ptr::null()) };
}

fn cstr(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).unwrap()
}

fn vsock_addr(cid: u32, port: u32) -> libc::sockaddr_vm {
    let mut a: libc::sockaddr_vm = unsafe { std::mem::zeroed() };
    a.svm_family = libc::AF_VSOCK as libc::sa_family_t;
    a.svm_cid = cid;
    a.svm_port = port;
    a
}

/// Punto d'ingresso del guest: monta `/proc` e `/tmp` (tmpfs), crea il motore una volta sola (prima dello
/// snapshot), ascolta e serve una connessione alla volta.
pub fn main() {
    mount("proc", "/proc", "proc");
    mount("tmpfs", "/tmp", "tmpfs");
    let rig = Rig::new("epoch");
    let fd = unsafe { libc::socket(libc::AF_VSOCK, libc::SOCK_STREAM, 0) };
    assert!(fd >= 0, "socket vsock: {}", std::io::Error::last_os_error());
    let a = vsock_addr(libc::VMADDR_CID_ANY, PORT);
    let rc = unsafe {
        libc::bind(
            fd,
            (&a as *const libc::sockaddr_vm).cast(),
            std::mem::size_of::<libc::sockaddr_vm>() as u32,
        )
    };
    assert_eq!(rc, 0, "bind vsock: {}", std::io::Error::last_os_error());
    assert_eq!(unsafe { libc::listen(fd, 8) }, 0);
    loop {
        let c = unsafe { libc::accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) };
        if c < 0 {
            continue;
        }
        let mut w = unsafe { std::fs::File::from_raw_fd(c) };
        let mut r = BufReader::with_capacity(1 << 20, w.try_clone().unwrap());
        serve(&rig, &mut r, &mut w, c, true);
        drop(w);
        // raccoglie i figli lasciati dai comandi `native`
        while unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) } > 0 {}
    }
}

fn kv_num(m: &HashMap<&str, &str>, k: &str, d: u64) -> u64 {
    m.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn in_guest() -> bool {
    std::fs::read_to_string("/proc/cmdline").is_ok_and(|s| s.contains("sbx_guest=1"))
}

/// Comando `native sub=<nome> ...`: una riga di risposta.
pub fn native(m: &HashMap<&str, &str>, guest: bool) -> String {
    if !guest || !in_guest() {
        return "err native refused: not inside the microVM".into();
    }
    match m.get("sub").copied().unwrap_or("") {
        "net" => net(),
        "fs" => fs_scan(m),
        "write" => write_try(),
        "forkbomb" => forkbomb(kv_num(m, "secs", 5)),
        "oom" => oom(m.get("child") != Some(&"0"), kv_num(m, "mb", 4096)),
        "spin" => spin(kv_num(m, "secs", 5)),
        "info" => info(),
        _ => "err unknown native".into(),
    }
}

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn proc_count() -> usize {
    std::fs::read_dir("/proc")
        .map(|d| {
            d.flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .bytes()
                        .all(|b| b.is_ascii_digit())
                })
                .count()
        })
        .unwrap_or(0)
}

fn info() -> String {
    let mem = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let g = |k: &str| {
        mem.lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("?")
            .to_string()
    };
    format!(
        "ok procs={} memtotal_kb={} memavail_kb={} uid={} cpus={}",
        proc_count(),
        g("MemTotal:"),
        g("MemAvailable:"),
        unsafe { libc::getuid() },
        std::thread::available_parallelism().map_or(0, |n| n.get())
    )
}

/// Rete: interfacce, connessioni TCP/UDP verso indirizzi interni e pubblici, vsock verso l'host (CID 2).
fn net() -> String {
    let ifs: Vec<String> = std::fs::read_dir("/sys/class/net")
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    let mut out = format!("ok ifaces={}", ifs.join(","));
    for (ip, port) in [
        ("127.0.0.1", 9u16),
        ("169.254.169.254", 80),
        ("10.0.0.1", 80),
        ("192.168.0.1", 80),
        ("93.184.216.34", 80),
    ] {
        let addr: std::net::SocketAddr = format!("{ip}:{port}").parse().unwrap();
        let r = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(500));
        out += &format!(
            " tcp[{ip}]={}",
            match r {
                Ok(_) => "CONNECTED".to_string(),
                Err(e) => format!("err{}", e.raw_os_error().unwrap_or(-1)),
            }
        );
    }
    let u = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|s| s.send_to(b"x", "8.8.8.8:53"));
    out += &format!(
        " udp[8.8.8.8]={}",
        match u {
            Ok(_) => "SENT".to_string(),
            Err(e) => format!("err{}", e.raw_os_error().unwrap_or(-1)),
        }
    );
    // vsock verso l'host: risponde solo dove l'host ascolta su `<uds>_<porta>` (Firecracker)
    let (mut ok, mut refused, mut other) = (vec![], 0, 0);
    for port in (1u32..=64).chain([1024, 5000, 5001, 7000, 9999]) {
        let s = unsafe { libc::socket(libc::AF_VSOCK, libc::SOCK_STREAM, 0) };
        let a = vsock_addr(2, port);
        let rc = unsafe {
            libc::connect(
                s,
                (&a as *const libc::sockaddr_vm).cast(),
                std::mem::size_of::<libc::sockaddr_vm>() as u32,
            )
        };
        if rc == 0 {
            ok.push(port);
        } else if errno() == libc::ECONNRESET || errno() == libc::ECONNREFUSED {
            refused += 1;
        } else {
            other += 1;
        }
        unsafe { libc::close(s) };
    }
    out += &format!(" vsock_host_connected_ports={ok:?} refused={refused} other_err={other}");
    out
}

/// Cerca i valori sentinella (mascherati con XOR 0x5A, come in fase 2) in un flusso; aggiorna i conteggi.
fn scan_stream(r: &mut dyn Read, needles: &[Vec<u8>], hits: &mut [usize], bytes: &mut u64) {
    let mut buf = vec![0u8; 1 << 20];
    let keep = needles.iter().map(|n| n.len()).max().unwrap_or(1) - 1;
    let mut carry = 0usize;
    while let Ok(n) = r.read(&mut buf[carry..]) {
        if n == 0 {
            break;
        }
        *bytes += n as u64;
        let tot = carry + n;
        let w = &buf[..tot];
        for (k, nd) in needles.iter().enumerate() {
            if tot >= nd.len() {
                for i in 0..=tot - nd.len() {
                    if w[i] ^ 0x5A == nd[0]
                        && w[i..i + nd.len()]
                            .iter()
                            .zip(nd)
                            .all(|(x, y)| x ^ 0x5A == *y)
                    {
                        hits[k] += 1;
                    }
                }
            }
        }
        carry = keep.min(tot);
        buf.copy_within(tot - carry..tot, 0);
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0))
        .collect()
}

/// Tutto ciò che il guest può leggere: i due dischi grezzi, ogni file regolare sotto `/` (tranne `/proc`,
/// `/sys`, `/dev`), `/dev/mem`, `/dev/kvm` e `/proc/kcore` (solo apertura). Cerca i sentinella dell'host
/// (`s=`, elenco separato da virgole, mascherati) e un controllo positivo scritto qui in `/tmp` (`ctl=`).
fn fs_scan(m: &HashMap<&str, &str>) -> String {
    let mut needles: Vec<Vec<u8>> = m
        .get("s")
        .map(|s| s.split(',').filter(|x| !x.is_empty()).map(unhex).collect())
        .unwrap_or_default();
    let host_n = needles.len();
    if let Some(c) = m.get("ctl") {
        let clear: Vec<u8> = unhex(c).iter().map(|b| b ^ 0x5A).collect();
        let _ = std::fs::write("/tmp/ctl.txt", clear);
        needles.push(unhex(c));
    }
    let mut hits = vec![0usize; needles.len()];
    let (mut bytes, mut files) = (0u64, 0u64);
    let deadline = Instant::now() + Duration::from_secs(kv_num(m, "secs", 60));
    for dev in ["/dev/vda", "/dev/vdb"] {
        if let Ok(mut f) = std::fs::File::open(dev) {
            scan_stream(&mut f, &needles, &mut hits, &mut bytes);
            files += 1;
        }
    }
    let mut stack = vec![std::path::PathBuf::from("/")];
    let mut cut = false;
    while let Some(d) = stack.pop() {
        if Instant::now() > deadline {
            cut = true;
            break;
        }
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if matches!(p.to_str(), Some("/proc" | "/sys" | "/dev")) {
                continue;
            }
            let Ok(md) = std::fs::symlink_metadata(&p) else {
                continue;
            };
            if md.is_dir() {
                stack.push(p);
            } else if md.is_file()
                && md.len() < (512 << 20)
                && let Ok(mut f) = std::fs::File::open(&p)
            {
                scan_stream(&mut f, &needles, &mut hits, &mut bytes);
                files += 1;
            }
        }
    }
    let probe = |p: &str| match std::fs::File::open(p) {
        Ok(_) => "opened".to_string(),
        Err(e) => format!("err{}", e.raw_os_error().unwrap_or(-1)),
    };
    format!(
        "ok files={files} bytes={bytes} cut={cut} host_hits={:?} ctl_hits={} dev_mem={} dev_kvm={} proc_kcore={}",
        &hits[..host_n],
        if needles.len() > host_n {
            hits[host_n]
        } else {
            0
        },
        probe("/dev/mem"),
        probe("/dev/kvm"),
        probe("/proc/kcore"),
    )
}

/// Scritture che devono fallire: radice, dischi grezzi (letti solo dalla VMM), cartelle in sola lettura.
fn write_try() -> String {
    let mut out = "ok".to_string();
    for p in [
        "/etc/sbx-write-test",
        "/dev/agent/sbx-write-test",
        "/dev/vda",
        "/dev/vdb",
    ] {
        let r = std::fs::OpenOptions::new()
            .write(true)
            .create(p.contains("sbx"))
            .open(p);
        let s = match r {
            Err(e) => format!("open_err{}", e.raw_os_error().unwrap_or(-1)),
            Ok(f) => {
                use std::os::unix::fs::FileExt;
                match f.write_at(&[0u8; 512], 0) {
                    Ok(n) => format!("WROTE{n}"),
                    Err(e) => format!("write_err{}", e.raw_os_error().unwrap_or(-1)),
                }
            }
        };
        out += &format!(" {p}={s}");
    }
    out
}

fn now_ms() -> u64 {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut t) };
    t.tv_sec as u64 * 1000 + t.tv_nsec as u64 / 1_000_000
}

/// Bomba di fork per `secs` secondi: ogni processo biforca in continuazione. Dopo la scadenza tutti escono.
/// Conta fork riusciti e falliti in memoria condivisa; il padre campiona il numero di processi.
fn forkbomb(secs: u64) -> String {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
    let shared = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            4096,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    } as *mut AtomicU64;
    let (forks, fails) = unsafe { (&*shared, &*shared.add(1)) };
    let deadline = now_ms() + secs * 1000;
    if unsafe { libc::fork() } == 0 {
        // nel figlio: niente allocazioni, solo fork e orologio
        while now_ms() < deadline {
            let p = unsafe { libc::fork() };
            if p < 0 {
                fails.fetch_add(1, Relaxed);
            } else if p > 0 {
                forks.fetch_add(1, Relaxed);
            }
        }
        unsafe { libc::_exit(0) };
    }
    let mut peak = 0usize;
    while now_ms() < deadline {
        std::thread::sleep(Duration::from_millis(250));
        peak = peak.max(proc_count());
    }
    // attende che i figli escano (fino a 20 s), poi riporta
    let t = now_ms();
    while now_ms() - t < 20_000 {
        if unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) } < 0
            && errno() == libc::ECHILD
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    format!(
        "ok forks={} fails={} procs_peak={peak} drain_ms={} {}",
        forks.load(Relaxed),
        fails.load(Relaxed),
        now_ms() - t,
        info()
    )
}

/// Esaurimento di memoria: alloca e tocca blocchi da 1 MiB fino al limite (`mb`) o all'uccisione. Con
/// `child=1` lo fa un figlio (l'OOM killer del guest lo uccide e l'agente risponde), con `child=0` lo fa
/// l'agente stesso, che è il PID 1.
fn oom(child: bool, mb: u64) -> String {
    let t = now_ms();
    let grab = move || {
        for _ in 0..mb {
            let p = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    1 << 20,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if p == libc::MAP_FAILED {
                return;
            }
            unsafe { libc::memset(p, 0xA5, 1 << 20) };
        }
    };
    if !child {
        grab();
        return format!("ok oom_in_agent_survived ms={}", now_ms() - t);
    }
    let p = unsafe { libc::fork() };
    if p == 0 {
        grab();
        unsafe { libc::_exit(0) };
    }
    let mut st = 0;
    unsafe { libc::waitpid(p, &mut st, 0) };
    format!(
        "ok oom_child exited={} signaled={} sig={} ms={} {}",
        libc::WIFEXITED(st),
        libc::WIFSIGNALED(st),
        if libc::WIFSIGNALED(st) {
            libc::WTERMSIG(st)
        } else {
            0
        },
        now_ms() - t,
        info()
    )
}

/// Un figlio per ogni vCPU, in ciclo per `secs` secondi.
fn spin(secs: u64) -> String {
    let n = std::thread::available_parallelism().map_or(1, |n| n.get());
    let deadline = now_ms() + secs * 1000;
    let mut pids = vec![];
    for _ in 0..n {
        let p = unsafe { libc::fork() };
        if p == 0 {
            while now_ms() < deadline {}
            unsafe { libc::_exit(0) };
        }
        pids.push(p);
    }
    for p in &pids {
        unsafe { libc::waitpid(*p, std::ptr::null_mut(), 0) };
    }
    format!("ok spun children={n} secs={secs}")
}
