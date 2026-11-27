//! Confinamento del ruolo `stream` (A1, A2), ordine dell'ADR della sandbox, adattato dallo spike S-2 fase 4:
//! 1. descrittori ereditati e limiti (`prologue`, prima che esista il runtime: altrimenti si chiuderebbero anche
//!    epoll ed eventfd di tokio);
//! 2. runtime caldo (fatto da `stream::run`: contesto di DataFusion, connessione a Iggy, comunicazione con il motore);
//! 3. nessun `RLIMIT_FSIZE` per questo ruolo (scrive i checkpoint);
//! 4. Landlock su tutti i thread: file solo la cartella dei checkpoint del tenant (lettura e scrittura) e poche
//!    letture di `/proc/self`, rete solo `connect` TCP verso le porte del bus e del motore;
//! 5. `PR_SET_DUMPABLE` a 0;
//! 6. seccomp su tutti i thread con la lista scoperta su minisheep (`allow/stream.txt`) piu' la base (`allow/base.txt`).
//!
//! Non costruito (ponytail): il ripiego con namespace utente dove Landlock manca. L'ADR lo prevede per questo ruolo e lo
//! spike della sandbox (fase 4) lo ha misurato; qui Landlock c'e' su tutte e due le macchine, e il namespace chiede un
//! processo con un solo thread, cioe' un avvio sincrono prima del runtime. Dove Landlock manca questo codice rifiuta di
//! partire (il motore lo traduce in errore con codice), che e' il comportamento «fallisce chiuso» dell'ADR.

use landlock::{
    ABI, Access, AccessFs, AccessNet, BitFlags, NetPort, PathBeneath, PathFd, RestrictSelfAttr,
    Ruleset, RulesetAttr, RulesetCreatedAttr, Scope, make_bitflags,
};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompFilter, SeccompRule, apply_filter_all_threads,
};
use std::collections::BTreeMap;
use std::time::Instant;

/// Codice di uscita del rifiuto di avvio (EX_CONFIG): il motore lo traduce in errore con codice.
pub const EXIT_REFUSED: i32 = 78;

/// Parametri del confinamento, dal messaggio iniziale del motore.
#[derive(Clone, Debug, Default)]
pub struct Cfg {
    /// cartella dei checkpoint del tenant (lettura e scrittura)
    pub rw: Vec<String>,
    /// file letti da `/proc/self` (lettura sola)
    pub ro: Vec<String>,
    /// porte TCP verso cui il processo puo' collegarsi: bus e motore
    pub tcp: Vec<u16>,
    /// scoperta delle syscall (gdb): al posto dei filtri il processo si ferma con SIGSTOP e si fa agganciare da gdb
    pub discover: bool,
    /// solo M1: non si mette `PR_SET_DUMPABLE` a 0 (si legge il PSS da fuori)
    pub keep_dumpable: bool,
}

fn set_rlimit(res: u32, v: u64) {
    let l = libc::rlimit {
        rlim_cur: v,
        rlim_max: v,
    };
    // SAFETY: `l` e' valido per la lettura
    unsafe { libc::setrlimit(res as _, &l) };
}

/// Passo 1, prima del runtime: chiude ogni descrittore oltre 2 che non e' il 3 (la coppia di socket del motore),
/// nessun privilegio nuovo, niente core, limite dei file. Restituisce `(ereditati, chiusi)`.
pub fn prologue() -> (usize, usize) {
    let fds: Vec<i32> = std::fs::read_dir("/proc/self/fd")
        .map(|d| {
            d.filter_map(|e| e.ok()?.file_name().to_str()?.parse().ok())
                .collect()
        })
        .unwrap_or_default();
    let (mut inh, mut closed) = (0usize, 0usize);
    for fd in fds {
        if fd > 2 {
            inh += 1;
            // SAFETY: chiusura di un descrittore ereditato
            if fd != 3 && unsafe { libc::close(fd) } == 0 {
                closed += 1;
            }
        }
    }
    // SAFETY: chiamate prctl/setrlimit senza puntatori
    unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    set_rlimit(libc::RLIMIT_CORE, 0);
    // ponytail: 512 descrittori; il limite vero per processo lo fissa il prodotto (connessioni di Iggy e del motore, thread)
    set_rlimit(libc::RLIMIT_NOFILE, 512);
    // l'iteratore della cartella teneva un descrittore proprio: gia' chiuso, non conta
    (inh.saturating_sub(1), closed)
}

/// Passo 4: Landlock su tutti i thread.
fn landlock(c: &Cfg) -> Result<String, String> {
    let abi = ABI::V9;
    let fs_all = AccessFs::from_all(abi) & !BitFlags::from(AccessFs::Execute);
    let fs_ro = make_bitflags!(AccessFs::{ReadFile | ReadDir});
    let mut rs = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| format!("handle fs: {e}"))?
        .handle_access(AccessNet::from_all(abi))
        .map_err(|e| format!("handle net: {e}"))?
        .scope(make_bitflags!(Scope::{AbstractUnixSocket | Signal}))
        .map_err(|e| format!("scope: {e}"))?
        .create()
        .map_err(|e| format!("create: {e}"))?;
    for p in &c.ro {
        let fd = PathFd::new(p).map_err(|e| format!("ro {p}: {e}"))?;
        rs = rs
            .add_rule(PathBeneath::new(fd, fs_ro))
            .map_err(|e| format!("rule ro: {e}"))?;
    }
    for p in &c.rw {
        let fd = PathFd::new(p).map_err(|e| format!("rw {p}: {e}"))?;
        rs = rs
            .add_rule(PathBeneath::new(fd, fs_all))
            .map_err(|e| format!("rule rw: {e}"))?;
    }
    for port in &c.tcp {
        rs = rs
            .add_rule(NetPort::new(*port, AccessNet::ConnectTcp))
            .map_err(|e| format!("rule tcp: {e}"))?;
    }
    let st = rs
        .all_threads(true)
        .map_err(|e| format!("all_threads: {e}"))?
        .restrict_self()
        .map_err(|e| format!("restrict: {e}"))?;
    Ok(format!(
        "{:?} nnp={} all_threads={}",
        st.ruleset, st.no_new_privs as u8, st.all_threads as u8
    ))
}

/// Lista consentita: base a mano piu' le syscall scoperte, una riga `numero nome`, `#` per i commenti.
pub fn allowlist() -> Vec<i64> {
    let mut v: Vec<i64> = [
        include_str!("../allow/base.txt"),
        include_str!("../allow/stream.txt"),
    ]
    .iter()
    .flat_map(|t| t.lines())
    .filter_map(|l| l.split_whitespace().next()?.parse().ok())
    .filter(|n: &i64| *n >= 0)
    .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Passo 6: seccomp con la terminazione del processo per ogni syscall fuori lista.
fn seccomp() -> Result<usize, String> {
    let nrs = allowlist();
    let rules: BTreeMap<i64, Vec<SeccompRule>> = nrs.iter().map(|n| (*n, vec![])).collect();
    let arch = std::env::consts::ARCH
        .try_into()
        .map_err(|e| format!("arch: {e:?}"))?;
    let f = SeccompFilter::new(
        rules,
        SeccompAction::KillProcess,
        SeccompAction::Allow,
        arch,
    )
    .map_err(|e| format!("filter: {e}"))?;
    let prog: BpfProgram = f.try_into().map_err(|e| format!("bpf: {e}"))?;
    apply_filter_all_threads(&prog).map_err(|e| format!("apply: {e}"))?;
    Ok(nrs.len())
}

/// Passi 4-6 dopo il runtime caldo. `Ok(riga di rapporto)` oppure `Err(motivo)` = rifiuto di avviare (esce 78).
pub fn apply(c: &Cfg, pre: (usize, usize)) -> Result<String, String> {
    let t = Instant::now();
    let mut r = format!("inherited_fds={} closed_fds={}", pre.0, pre.1);
    if c.discover {
        // scoperta: nessun filtro; il processo si ferma da solo e `discover.sh` lo aggancia con gdb e lo fa ripartire
        // (la sandbox usava una syscall marcatore con gdb dall'avvio; qui, con il runtime caldo, gdb dall'avvio non finiva mai)
        // SAFETY: raise senza puntatori
        unsafe { libc::raise(libc::SIGSTOP) };
        r += " level=discover";
    } else {
        r += &format!(" landlock={}", landlock(c)?);
        // SAFETY: prctl senza puntatori
        if !c.keep_dumpable {
            unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
        }
        r += &format!(" seccomp_syscalls={}", seccomp()?);
        r += " level=landlock+seccomp";
    }
    r += &format!(" t_confine_us={}", t.elapsed().as_micros());
    Ok(r)
}

/// Probe ostili di A2 eseguiti dentro il processo confinato (comando `probe` del motore, solo con `--allow-probe`).
/// Ogni risposta dice se l'accesso e' riuscito (`leak=1`) o l'errno che lo ha negato. Un processo ucciso da seccomp non
/// risponde: lo vede il motore dallo stato di uscita (SIGSYS).
pub fn probe(id: &str, path: &str, pid: i32, port: u16) -> String {
    use std::io::Read;
    let den = |e: &std::io::Error| format!("leak=0 err={}", errno_name(e));
    match id {
        "file" => match std::fs::File::open(path) {
            Err(e) => den(&e),
            Ok(mut f) => {
                let mut b = vec![];
                // ponytail: massimo 1 MiB per prova, basta a vedere che il file si legge
                let _ = (&mut f).take(1 << 20).read_to_end(&mut b);
                format!("leak=1 bytes={}", b.len())
            }
        },
        "dir" => match std::fs::read_dir(path) {
            Err(e) => den(&e),
            Ok(d) => format!("leak=1 entries={}", d.count()),
        },
        "write" => match std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
        {
            Err(e) => den(&e),
            Ok(_) => "leak=1".into(),
        },
        "ptrace" => {
            // SEIZE non ferma il motore, ma prova lo stesso permesso di ptrace
            // SAFETY: ptrace su un pid altrui
            let r = unsafe { libc::ptrace(libc::PTRACE_SEIZE, pid, 0, 0) };
            if r < 0 {
                den(&std::io::Error::last_os_error())
            } else {
                // SAFETY: stacca quello che si e' appena agganciato
                unsafe { libc::ptrace(libc::PTRACE_DETACH, pid, 0, 0) };
                "leak=1".into()
            }
        }
        "pvm" => {
            let mut b = vec![0u8; 64];
            let l = libc::iovec {
                iov_base: b.as_mut_ptr().cast(),
                iov_len: 64,
            };
            let r = libc::iovec {
                iov_base: 0x1000 as *mut _,
                iov_len: 64,
            };
            // SAFETY: `l` e' valido; l'indirizzo remoto non si dereferenzia qui
            let n = unsafe { libc::process_vm_readv(pid, &l, 1, &r, 1, 0) };
            if n < 0 {
                let e = std::io::Error::last_os_error();
                // EFAULT = il permesso c'e' (l'indirizzo e' inventato): conta come fuga del permesso
                if e.raw_os_error() == Some(libc::EFAULT) {
                    "leak=1 err=EFAULT(permesso concesso)".into()
                } else {
                    den(&e)
                }
            } else {
                "leak=1".into()
            }
        }
        "kill" => {
            // SAFETY: segnale 0 = solo il controllo del permesso
            if unsafe { libc::kill(pid, 0) } == 0 {
                "leak=1".into()
            } else {
                den(&std::io::Error::last_os_error())
            }
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
            // SAFETY: socket e connect su un indirizzo locale
            unsafe {
                let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
                if fd < 0 {
                    return den(&std::io::Error::last_os_error());
                }
                let r = libc::connect(
                    fd,
                    (&a as *const libc::sockaddr_in).cast(),
                    std::mem::size_of::<libc::sockaddr_in>() as u32,
                );
                let e = std::io::Error::last_os_error();
                libc::close(fd);
                if r == 0 { "leak=1".into() } else { den(&e) }
            }
        }
        "exec" => {
            let p = std::ffi::CString::new("/bin/true").unwrap();
            let argv = [p.as_ptr(), std::ptr::null()];
            // SAFETY: argv terminato da null; se riesce non si torna
            unsafe { libc::execv(p.as_ptr(), argv.as_ptr()) };
            den(&std::io::Error::last_os_error())
        }
        "fork" => {
            // SAFETY: il figlio esce subito
            let p = unsafe { libc::fork() };
            if p == 0 {
                unsafe { libc::_exit(0) };
            }
            if p < 0 {
                den(&std::io::Error::last_os_error())
            } else {
                unsafe { libc::waitpid(p, std::ptr::null_mut(), 0) };
                "leak=1".into()
            }
        }
        "env" => format!("leak=0 env_vars={}", std::env::vars().count()),
        "dumpable" => format!("leak=0 dumpable={}", unsafe {
            libc::prctl(libc::PR_GET_DUMPABLE)
        }),
        "fds" => {
            // descrittori ereditati oltre il 3: devono essere tutti chiusi
            let mut open = 0;
            for fd in 4..64 {
                // SAFETY: fcntl di sola lettura
                if unsafe { libc::fcntl(fd, libc::F_GETFD) } >= 0 {
                    open += 1;
                }
            }
            // i descrittori aperti dal runtime stesso (epoll, eventfd, socket) ci sono: si riporta il numero, non e' una fuga
            format!("leak=0 open_fds_4_63={open}")
        }
        _ => "err unknown probe".into(),
    }
}

fn errno_name(e: &std::io::Error) -> String {
    match e.raw_os_error() {
        Some(libc::EACCES) => "EACCES".into(),
        Some(libc::EPERM) => "EPERM".into(),
        Some(libc::ENOENT) => "ENOENT".into(),
        Some(libc::ESRCH) => "ESRCH".into(),
        Some(libc::ECONNREFUSED) => "ECONNREFUSED".into(),
        Some(n) => format!("errno{n}"),
        None => "?".into(),
    }
}
