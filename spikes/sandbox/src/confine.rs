//! Confinamento degli ausiliari (spike S-2, fase 4): ciò che un sottocomando applica a sé stesso PRIMA di
//! leggere contenuto dei tenant. Ordine: chiusura dei descrittori ereditati, `PR_SET_DUMPABLE=0`,
//! `PR_SET_NO_NEW_PRIVS`, limiti di risorse, poi Landlock (filesystem, porte TCP, segnali, socket astratti) e
//! seccomp con lista consentita per ruolo. Dove Landlock manca: seccomp senza `open` per i ruoli che non
//! ne hanno bisogno, namespace utente e mount con radice vuota per gli altri, altrimenti rifiuto di avviare.
//! Codice da buttare (spike): `Mode::{Ll, Sc, None}` esistono solo per attribuire un rifiuto allo strato che
//! lo produce; il prodotto avrebbe solo `Full` (con il ripiego automatico) e nessuna modalità senza confinamento.

use landlock::{
    ABI, Access, AccessFs, AccessNet, BitFlags, LandlockStatus, NetPort, PathBeneath, PathFd,
    RestrictSelfAttr, Ruleset, RulesetAttr, RulesetCreatedAttr, Scope, make_bitflags,
};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompFilter, SeccompRule, apply_filter_all_threads,
};
use std::collections::BTreeMap;
use std::time::Instant;

/// Versione dell'IPC tra motore e ausiliario (AD-6): con una versione diversa l'ausiliario esce.
pub const IPC_VERSION: u32 = 1;
/// Codice di uscita del rifiuto di avvio (EX_CONFIG): il motore lo traduce in errore con codice.
pub const EXIT_REFUSED: i32 = 78;
/// Codice di uscita per versione dell'IPC diversa.
pub const EXIT_IPC_VERSION: i32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Produzione: Landlock + seccomp, con il ripiego automatico dove Landlock manca.
    Full,
    /// Solo prova: Landlock senza seccomp.
    Ll,
    /// Solo prova: seccomp senza Landlock (e senza ripiego).
    Sc,
    /// Solo prova: nessun confinamento (controllo positivo delle prove ostili).
    None,
    /// Prova: Landlock finto assente (percorso del ripiego forzato).
    Fb,
    /// Prova: Landlock finto assente e namespace utente finti vietati.
    FbNoUserns,
}

impl Mode {
    pub fn parse(s: &str) -> Mode {
        match s {
            "ll" => Mode::Ll,
            "sc" => Mode::Sc,
            "none" => Mode::None,
            "fb" => Mode::Fb,
            "fb-nouserns" => Mode::FbNoUserns,
            _ => Mode::Full,
        }
    }
}

/// Configurazione di un ruolo, risolta dal motore e passata sulla riga di comando (mai segreti: AD-6).
#[derive(Clone, Debug, Default)]
pub struct Cfg {
    pub role: String,
    pub mode: Option<Mode>,
    /// Cartelle leggibili (Landlock), per esempio la CA di `call http`.
    pub ro: Vec<String>,
    /// Cartelle leggibili e scrivibili, per esempio la cartella del tenant della pipeline stream.
    pub rw: Vec<String>,
    /// Porte TCP a cui il ruolo può collegarsi (Landlock vede le porte, non gli indirizzi: M-30 resta).
    pub tcp: Vec<u16>,
    /// Descrittori ereditati da tenere (aperti dal motore per l'ausiliario); gli altri si chiudono.
    pub keep: Vec<i32>,
    /// Cartella vuota per la radice del ripiego con namespace.
    pub scratch: String,
    /// Prova di scoperta delle syscall: al posto dei filtri emette la syscall marcatore.
    pub discover: bool,
}

impl Cfg {
    pub fn mode(&self) -> Mode {
        self.mode.unwrap_or(Mode::Full)
    }
}

/// I ruoli che devono aprire file o socket: per loro seccomp da solo non basta.
pub fn needs_open(role: &str) -> bool {
    matches!(role, "callhttp" | "stream")
}

fn allow_text(role: &str) -> &'static str {
    match role {
        "validation" => include_str!("../allow/validation.txt"),
        "jq" => include_str!("../allow/jq.txt"),
        "callhttp" => include_str!("../allow/callhttp.txt"),
        "light" => include_str!("../allow/light.txt"),
        "stream" => include_str!("../allow/stream.txt"),
        _ => "",
    }
}

/// Lista consentita del ruolo: una riga `numero nome`, `#` per i commenti.
pub fn allowlist(role: &str) -> Vec<i64> {
    let mut v: Vec<i64> = [include_str!("../allow/base.txt"), allow_text(role)]
        .iter()
        .flat_map(|t| t.lines())
        .filter_map(|l| l.split_whitespace().next()?.parse().ok())
        .filter(|n: &i64| *n >= 0)
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// ABI Landlock del kernel letta con la syscall (0 se manca); `errno` in `Err`.
pub fn kernel_abi() -> Result<i64, i32> {
    let r = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<u8>(),
            0usize,
            1u32, // LANDLOCK_CREATE_RULESET_VERSION
        )
    };
    if r < 0 {
        Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0))
    } else {
        Ok(r)
    }
}

fn set_rlimit(res: u32, v: u64) {
    let l = libc::rlimit {
        rlim_cur: v,
        rlim_max: v,
    };
    unsafe { libc::setrlimit(res as _, &l) };
}

/// Chiude ogni descrittore oltre 2 che non è in `keep`; restituisce `(ereditati, chiusi)`.
fn close_inherited(keep: &[i32]) -> (usize, usize) {
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
            if !keep.contains(&fd) && unsafe { libc::close(fd) } == 0 {
                closed += 1;
            }
        }
    }
    // l'iteratore della cartella teneva un descrittore proprio: già chiuso, non conta
    (inh.saturating_sub(1), closed)
}

/// Ruleset Landlock del ruolo. Restituisce lo stato e la descrizione delle regole per il rapporto.
fn landlock(c: &Cfg) -> Result<(String, String), String> {
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
    let mut rules = vec![];
    for p in &c.ro {
        let fd = PathFd::new(p).map_err(|e| format!("ro {p}: {e}"))?;
        rs = rs
            .add_rule(PathBeneath::new(fd, fs_ro))
            .map_err(|e| format!("rule ro: {e}"))?;
        rules.push("ro");
    }
    for p in &c.rw {
        let fd = PathFd::new(p).map_err(|e| format!("rw {p}: {e}"))?;
        rs = rs
            .add_rule(PathBeneath::new(fd, fs_all))
            .map_err(|e| format!("rule rw: {e}"))?;
        rules.push("rw");
    }
    for port in &c.tcp {
        rs = rs
            .add_rule(NetPort::new(*port, AccessNet::ConnectTcp))
            .map_err(|e| format!("rule tcp: {e}"))?;
        rules.push("tcp");
    }
    let st = rs
        .all_threads(true)
        .map_err(|e| format!("all_threads: {e}"))?
        .restrict_self()
        .map_err(|e| format!("restrict: {e}"))?;
    let abi_s = match st.landlock {
        LandlockStatus::Available {
            effective_abi,
            kernel_abi,
        } => format!(
            "effective={effective_abi:?} kernel={}",
            kernel_abi
                .map(|k| k.to_string())
                .unwrap_or("=effective".into())
        ),
        o => format!("{o:?}"),
    };
    Ok((
        format!(
            "{:?} nnp={} all_threads={} {abi_s}",
            st.ruleset, st.no_new_privs as u8, st.all_threads as u8
        ),
        format!(
            "fs_handled=all net_handled=bind+connect scope=signal+abstract_unix rules={}",
            rules.join(",")
        ),
    ))
}

/// Filtro seccomp: solo le syscall della lista, qualunque altra uccide il processo.
fn seccomp(role: &str) -> Result<usize, String> {
    let nrs = allowlist(role);
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

fn w(path: &str, s: &str) -> Result<(), String> {
    std::fs::write(path, s).map_err(|e| format!("{path}: {e}"))
}

fn sys_err(what: &str) -> String {
    format!("{what}: {}", std::io::Error::last_os_error())
}

fn cs(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).unwrap()
}

/// Ripiego senza Landlock per i ruoli che aprono file o socket: namespace utente e mount con radice vuota
/// (tmpfs) in cui compaiono solo le cartelle `rw` (in `/d<n>`) e `ro` (in `/r<n>`, sola lettura). Va fatto
/// con un solo thread (`unshare(CLONE_NEWUSER)` fallisce altrimenti). Non è un confine di rete: il `call http`
/// ha bisogno della rete, quindi niente `CLONE_NEWNET`; i segnali e `ptrace` li chiude seccomp.
fn enter_userns(c: &Cfg) -> Result<(), String> {
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
    if unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) } != 0 {
        return Err(sys_err("unshare"));
    }
    w("/proc/self/setgroups", "deny")?;
    w("/proc/self/uid_map", &format!("{uid} {uid} 1"))?;
    w("/proc/self/gid_map", &format!("{gid} {gid} 1"))?;
    let mnt = |src: &str, tgt: &str, fs: &str, fl: libc::c_ulong| -> Result<(), String> {
        let (s, t, f) = (cs(src), cs(tgt), cs(fs));
        let r = unsafe { libc::mount(s.as_ptr(), t.as_ptr(), f.as_ptr(), fl, std::ptr::null()) };
        if r != 0 {
            Err(sys_err(&format!("mount {tgt}")))
        } else {
            Ok(())
        }
    };
    mnt("none", "/", "", libc::MS_REC | libc::MS_PRIVATE)?;
    let root = c.scratch.as_str();
    mnt("tmpfs", root, "tmpfs", libc::MS_NOSUID | libc::MS_NODEV)?;
    std::fs::create_dir(format!("{root}/old")).map_err(|e| e.to_string())?;
    for (i, p) in c.rw.iter().enumerate() {
        let t = format!("{root}/d{i}");
        std::fs::create_dir(&t).map_err(|e| e.to_string())?;
        mnt(p, &t, "", libc::MS_BIND | libc::MS_REC)?;
    }
    for (i, p) in c.ro.iter().enumerate() {
        let t = format!("{root}/r{i}");
        std::fs::create_dir(&t).map_err(|e| e.to_string())?;
        mnt(p, &t, "", libc::MS_BIND | libc::MS_REC)?;
        mnt(
            "none",
            &t,
            "",
            libc::MS_BIND | libc::MS_REMOUNT | libc::MS_RDONLY | libc::MS_NOSUID | libc::MS_NODEV,
        )?;
    }
    std::env::set_current_dir(root).map_err(|e| e.to_string())?;
    let (n, o) = (cs("."), cs("old"));
    if unsafe { libc::syscall(libc::SYS_pivot_root, n.as_ptr(), o.as_ptr()) } != 0 {
        return Err(sys_err("pivot_root"));
    }
    std::env::set_current_dir("/").map_err(|e| e.to_string())?;
    if unsafe { libc::umount2(cs("/old").as_ptr(), libc::MNT_DETACH) } != 0 {
        return Err(sys_err("umount old"));
    }
    std::fs::remove_dir("/old").map_err(|e| e.to_string())
}

/// Applica il confinamento. `Ok(riga di rapporto)` oppure `Err(motivo)` = rifiuto di avviare (esce 78).
pub fn harden(c: &Cfg) -> Vec<(String, String)> {
    let mode = c.mode();
    let mut r: Vec<(String, String)> = vec![];
    let mut put = |k: &str, v: String| r.push((k.to_string(), v));
    put("role", c.role.clone());
    put("mode", format!("{mode:?}").to_lowercase());
    put("env_vars", std::env::vars().count().to_string());
    if mode != Mode::None {
        // 1. descrittori ereditati: il motore passa solo ciò che serve
        let (inh, closed) = close_inherited(&c.keep);
        put("inherited_fds", inh.to_string());
        put("closed_fds", closed.to_string());
        // 2. non ispezionabile, nessun privilegio nuovo, limiti di risorse (senza root)
        unsafe {
            libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0);
        }
        set_rlimit(libc::RLIMIT_CORE, 0);
        set_rlimit(libc::RLIMIT_NOFILE, 64);
        // ponytail: RLIMIT_CPU come rete di sicurezza lunga; il limite vero è l'epoca nel guest.
        set_rlimit(libc::RLIMIT_CPU, 3600);
        put(
            "rlimits",
            "core=0 nofile=64 cpu=3600 fsize=0|stream:unlimited".into(),
        );
        put("dumpable", "0".into());
        put("no_new_privs", "1".into());
    }
    r
}

/// Secondo passo: Landlock, ripiego e seccomp, dopo la creazione dei thread. `Ok(riga di rapporto)` oppure
/// `Err(motivo)` = rifiuto di avviare (esce 78).
pub fn apply(c: &Cfg, pre: Vec<(String, String)>) -> Result<String, String> {
    let t = Instant::now();
    let mode = c.mode();
    let mut r = pre;
    if mode != Mode::None && c.role != "stream" {
        // dopo la creazione del motore wasmtime (che usa memfd e ftruncate): nessun file si può più scrivere
        set_rlimit(libc::RLIMIT_FSIZE, 0);
    }
    let mut put = |k: &str, v: String| r.push((k.to_string(), v));
    let ll_wanted = matches!(mode, Mode::Full | Mode::Ll);
    let sc_wanted = matches!(mode, Mode::Full | Mode::Sc | Mode::Fb | Mode::FbNoUserns);
    let abi = kernel_abi();
    put(
        "abi_raw",
        match &abi {
            Ok(v) => v.to_string(),
            Err(e) => format!("errno{e}"),
        },
    );
    let ll_ok = abi.is_ok() && !matches!(mode, Mode::Fb | Mode::FbNoUserns);
    let mut level = "none";
    if c.discover {
        // scoperta delle syscall: il marcatore (`sysinfo`) segna il punto in cui i filtri sarebbero caricati
        let mut si: libc::sysinfo = unsafe { std::mem::zeroed() };
        unsafe { libc::sysinfo(&mut si) };
        put("level", "discover".into());
    } else {
        if ll_wanted && ll_ok {
            let (st, rules) = landlock(c)?;
            put("landlock", st);
            put("landlock_rules", rules);
            level = "landlock";
        } else if ll_wanted {
            if mode == Mode::Ll {
                return Err("Landlock assente e modalità solo-Landlock".into());
            }
            put("landlock", "absent".into());
        } else if matches!(mode, Mode::Fb | Mode::FbNoUserns) {
            put("landlock", "forced_off".into());
        }
        // ripiego: serve solo dove Landlock non c'è (o è finto assente) e il ruolo apre file o socket
        let fallback =
            (mode == Mode::Full && !ll_ok) || matches!(mode, Mode::Fb | Mode::FbNoUserns);
        let mut userns = false;
        if fallback && needs_open(&c.role) {
            if mode == Mode::FbNoUserns {
                return Err("Landlock assente e namespace utente non consentiti (simulato)".into());
            }
            enter_userns(c)
                .map_err(|e| format!("Landlock assente e namespace utente non riusciti: {e}"))?;
            userns = true;
            put("userns", "root_vuota".into());
            level = "userns+seccomp";
        } else if fallback {
            level = "seccomp-senza-open";
        }
        if mode != Mode::None {
            // dopo il ripiego con namespace (con dumpable 0 i file `/proc/self/*_map` sono di root) e prima di seccomp
            unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
        }
        if sc_wanted {
            let n = seccomp(&c.role)?;
            put("seccomp_syscalls", n.to_string());
            put("seccomp_action", "KILL_PROCESS, TSYNC".into());
            if level == "landlock" {
                level = "landlock+seccomp";
            } else if level == "none" {
                level = "seccomp";
            }
        }
        put("userns_active", (userns as u8).to_string());
        put("level", level.to_string());
    }
    put("t_confine_us", t.elapsed().as_micros().to_string());
    Ok(r.iter()
        .map(|(k, v)| format!("{k}={}", v.replace(' ', "_")))
        .collect::<Vec<_>>()
        .join(" "))
}
