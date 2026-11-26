//! Runtime condiviso dall'ausiliario e dal compilatore: configurazione del motore, funzioni
//! dell'host (`topictoast.*`), limiti di risorse, esecuzione di un'istanza e classificazione degli
//! errori. Codice da buttare (spike S-2, fase 2).

use sha2::{Digest, Sha256};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;
use wasmtime::component::{Component, Linker as CLinker};
use wasmtime::{
    Caller, Config, Engine, Error, InstanceAllocationStrategy, InstancePre, Linker, Module,
    PoolingAllocationConfig, ResourceLimiter, Result, Store, StoreContextMut, Trap,
};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::{I32Exit, ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Scelte di configurazione del motore (stringa tipo `pool+nocow+epoch+fuel`).
#[derive(Clone, Debug)]
pub struct Opts {
    pub pool: bool,
    pub cow: bool,
    pub epoch: bool,
    pub fuel: bool,
}

impl Opts {
    pub fn parse(s: &str) -> Opts {
        let has = |t: &str| s.split('+').any(|x| x == t);
        Opts {
            pool: has("pool"),
            cow: !has("nocow"),
            epoch: has("epoch"),
            fuel: has("fuel"),
        }
    }
}

/// Motore con la configurazione scelta. Tetti fissi: pila di 512 KiB (predefinita di wasmtime).
pub fn make_engine(o: &Opts) -> Result<Engine> {
    let mut c = Config::new();
    c.epoch_interruption(o.epoch);
    c.consume_fuel(o.fuel);
    c.memory_init_cow(o.cow);
    if o.pool {
        let mut p = PoolingAllocationConfig::new();
        p.total_core_instances(256)
            .total_memories(256)
            .total_tables(256);
        p.max_memory_size(64 << 20).table_elements(100_000);
        c.allocation_strategy(InstanceAllocationStrategy::Pooling(p));
    }
    Engine::new(&c)
}

/// Impronta della configurazione compilata: entra nel nome del file della cache.
pub fn cfg_hash(e: &Engine) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    e.precompile_compatibility_hash().hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn sha_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Limiti di memoria e tabelle di uno store; negare la crescita fa trappola con un errore che nomina il limite.
pub struct Lim {
    pub mem_max: usize,
    pub tbl_max: usize,
}

impl ResourceLimiter for Lim {
    fn memory_growing(&mut self, _cur: usize, desired: usize, _max: Option<usize>) -> Result<bool> {
        if desired > self.mem_max {
            return Err(Error::msg(format!(
                "memory limit exceeded: requested {desired} bytes, cap {} MiB",
                self.mem_max >> 20
            )));
        }
        Ok(true)
    }
    fn table_growing(&mut self, _cur: usize, desired: usize, _max: Option<usize>) -> Result<bool> {
        if desired > self.tbl_max {
            return Err(Error::msg(format!(
                "table limit exceeded: requested {desired} elements, cap {}",
                self.tbl_max
            )));
        }
        Ok(true)
    }
}

/// Stato dell'host comune ai moduli core e ai componenti.
pub struct Common {
    pub lim: Lim,
    pub logs: Vec<(u32, String)>,
    pub log_bytes: usize,
    pub log_cap: usize,
    pub emits: Vec<(String, String)>,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
}

impl Common {
    fn log(&mut self, level: u32, msg: String) -> Result<()> {
        self.log_bytes += msg.len();
        if self.log_bytes > self.log_cap {
            return Err(Error::msg(format!(
                "log limit exceeded: more than {} bytes",
                self.log_cap
            )));
        }
        self.logs.push((level, msg));
        Ok(())
    }
}

pub struct CoreHost {
    pub c: Common,
    pub wasi: WasiP1Ctx,
}

pub struct CompHost {
    pub c: Common,
    pub ctx: WasiCtx,
    pub table: ResourceTable,
}

impl WasiView for CompHost {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

fn guest_bytes<T>(caller: &mut Caller<'_, T>, ptr: i32, len: i32) -> Result<Vec<u8>> {
    let mem = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| Error::msg("guest without exported memory"))?;
    mem.data(&caller)
        .get(ptr as u32 as usize..(ptr as u32 as usize).saturating_add(len as u32 as usize))
        .map(|s| s.to_vec())
        .ok_or_else(|| Error::msg("guest pointer out of bounds"))
}

/// Funzioni dell'host offerte ai moduli core: `log`, `emit`, `input`, `output`. Nient'altro.
fn add_core_host_fns(l: &mut Linker<CoreHost>) -> Result<()> {
    l.func_wrap(
        "topictoast",
        "log",
        |mut c: Caller<'_, CoreHost>, level: i32, ptr: i32, len: i32| {
            let m = String::from_utf8_lossy(&guest_bytes(&mut c, ptr, len)?).into_owned();
            c.data_mut().c.log(level as u32, m)
        },
    )?;
    l.func_wrap(
        "topictoast",
        "emit",
        |mut c: Caller<'_, CoreHost>, sp: i32, sl: i32, ep: i32, el: i32| {
            let s = String::from_utf8_lossy(&guest_bytes(&mut c, sp, sl)?).into_owned();
            let e = String::from_utf8_lossy(&guest_bytes(&mut c, ep, el)?).into_owned();
            c.data_mut().c.emits.push((s, e));
            Ok(())
        },
    )?;
    l.func_wrap(
        "topictoast",
        "input",
        |mut c: Caller<'_, CoreHost>, ptr: i32, cap: i32| -> Result<i32> {
            let inp = c.data().c.input.clone();
            let n = inp.len().min(cap as u32 as usize);
            let mem = c
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| Error::msg("no memory"))?;
            mem.write(&mut c, ptr as u32 as usize, &inp[..n])?;
            Ok(n as i32)
        },
    )?;
    l.func_wrap(
        "topictoast",
        "output",
        |mut c: Caller<'_, CoreHost>, ptr: i32, len: i32| {
            let b = guest_bytes(&mut c, ptr, len)?;
            c.data_mut().c.output.extend_from_slice(&b);
            Ok(())
        },
    )?;
    Ok(())
}

/// Interfaccia dell'host per i componenti (`topictoast:node/host`): `log` ed `emit`.
fn add_comp_host_fns(l: &mut CLinker<CompHost>) -> Result<()> {
    let mut i = l.instance("topictoast:node/host@1.0.0")?;
    i.func_wrap(
        "log",
        |mut c: StoreContextMut<'_, CompHost>, (level, msg): (u32, String)| {
            c.data_mut().c.log(level, msg)
        },
    )?;
    i.func_wrap(
        "emit",
        |mut c: StoreContextMut<'_, CompHost>, (s, e): (String, String)| {
            c.data_mut().c.emits.push((s, e));
            Ok(())
        },
    )?;
    Ok(())
}

thread_local! {
    /// Fd su cui scrivere `started\n` subito prima di entrare nel guest (-1: nessuno). Serve alla fase 3 per
    /// misurare, dall'host, l'istante in cui il nodo inizia.
    pub static START_FD: std::cell::Cell<i32> = const { std::cell::Cell::new(-1) };
}

fn mark_start() {
    let fd = START_FD.get();
    if fd >= 0 {
        unsafe { libc::write(fd, b"started\n".as_ptr().cast(), 8) };
    }
}

pub enum Loaded {
    Core(InstancePre<CoreHost>),
    /// Modulo javy a collegamento dinamico: il plugin di QuickJS si istanzia a ogni esecuzione, nello stesso store.
    Dyn {
        plugin: Module,
        node: Module,
    },
    Comp(wasmtime::component::InstancePre<CompHost>),
}

pub struct Rt {
    pub engine: Engine,
    pub opts: Opts,
    core: Linker<CoreHost>,
    comp: CLinker<CompHost>,
}

/// Parametri di una richiesta di esecuzione.
pub struct RunReq {
    pub input: Vec<u8>,
    pub args: Vec<String>,
    pub limit_ms: u64,
    pub fuel: u64,
    pub mem_mb: usize,
    pub log_cap: usize,
    pub out_cap: usize,
}

/// Risultato: tempi in microsecondi dalla ricezione della richiesta.
#[derive(Default)]
pub struct RunRes {
    pub t_inst_us: u128,
    pub t_start_us: u128,
    pub t_total_us: u128,
    /// `None` se il nodo è finito bene, altrimenti `(classe, messaggio)`.
    pub failure: Option<(String, String)>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub host_output: Vec<u8>,
    pub logs: Vec<(u32, String)>,
    pub emits: Vec<(String, String)>,
}

/// Classe dell'errore in base al testo o al tipo di trappola, in modo che nomini il limite superato.
pub fn classify(e: &Error) -> (String, String) {
    let msg = format!("{e:#}");
    let class = if let Some(t) = e.downcast_ref::<Trap>() {
        match t {
            Trap::Interrupt => "cpu_time_limit".to_string(),
            Trap::OutOfFuel => "cpu_fuel_limit".to_string(),
            Trap::StackOverflow => "stack_limit".to_string(),
            o => format!("trap_{o:?}"),
        }
    } else if msg.contains("memory limit exceeded") {
        "memory_limit".into()
    } else if msg.contains("table limit exceeded") {
        "table_limit".into()
    } else if msg.contains("log limit exceeded") {
        "log_limit".into()
    } else if msg.contains("write beyond capacity") {
        "output_limit".into()
    } else if msg.contains("unknown import") || msg.contains("incompatible import") {
        "unoffered_import".into()
    } else {
        "error".into()
    };
    (class, msg.replace('\n', " | "))
}

impl Rt {
    pub fn new(opts: Opts) -> Result<Rt> {
        let engine = make_engine(&opts)?;
        let mut core = Linker::new(&engine);
        p1::add_to_linker_sync(&mut core, |h: &mut CoreHost| &mut h.wasi)?;
        add_core_host_fns(&mut core)?;
        let mut comp = CLinker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut comp)?;
        add_comp_host_fns(&mut comp)?;
        Ok(Rt {
            engine,
            opts,
            core,
            comp,
        })
    }

    /// Carica un artefatto nativo. `unsafe` perché wasmtime non lo valida: l'integrità si verifica prima
    /// (impronta registrata dal compilatore), vedi `aux::cmd_load`.
    pub fn load(&self, path: &Path, comp: bool, plugin: Option<&Path>) -> Result<Loaded> {
        if let Some(p) = plugin {
            let plugin = unsafe { Module::deserialize_file(&self.engine, p)? };
            let node = unsafe { Module::deserialize_file(&self.engine, path)? };
            Ok(Loaded::Dyn { plugin, node })
        } else if comp {
            let c = unsafe { Component::deserialize_file(&self.engine, path)? };
            Ok(Loaded::Comp(self.comp.instantiate_pre(&c)?))
        } else {
            let m = unsafe { Module::deserialize_file(&self.engine, path)? };
            Ok(Loaded::Core(self.core.instantiate_pre(&m)?))
        }
    }

    /// Carica un artefatto già aperto dal motore (fase 4: l'ausiliario confinato non apre file).
    pub fn load_file(&self, f: std::fs::File, _comp: bool) -> Result<Loaded> {
        // ponytail: solo moduli core (wasmtime 50.0.3 non ha `Component::deserialize_open_file`); i componenti
        // si caricherebbero con `deserialize` dai byte già letti per la verifica.
        let m = unsafe { Module::deserialize_open_file(&self.engine, f)? };
        Ok(Loaded::Core(self.core.instantiate_pre(&m)?))
    }

    /// Fase 4: esegue una volta, prima del confinamento, un nodo minimo che scrive su stdout. Alla prima
    /// scrittura wasmtime-wasi crea il suo runtime tokio (thread di lavoro, epoll, lettura dei cgroup per contare
    /// i core): così quelle syscall restano fuori dalla lista consentita dell'ausiliario. Nessun dato di tenant.
    pub fn warmup(&self) {
        let wat = r#"(module
  (import "wasi_snapshot_preview1" "fd_write" (func $w (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1) (data (i32.const 8) "x")
  (func (export "_start")
    (i32.store (i32.const 0) (i32.const 8)) (i32.store (i32.const 4) (i32.const 1))
    (drop (call $w (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 16)))))"#;
        let bytes = wat::parse_str(wat).expect("wat");
        let m = Module::new(&self.engine, &bytes).expect("warmup module");
        let l = Loaded::Core(self.core.instantiate_pre(&m).expect("pre"));
        let q = RunReq {
            input: vec![],
            args: vec![],
            limit_ms: 1_000_000,
            fuel: u64::MAX / 4,
            mem_mb: 64,
            log_cap: 1 << 20,
            out_cap: 1 << 20,
        };
        let r = self.run(&l, &q, Instant::now(), None);
        assert!(r.failure.is_none(), "warmup: {:?}", r.failure);
    }

    pub fn run(
        &self,
        l: &Loaded,
        q: &RunReq,
        t0: Instant,
        hold: Option<&mut Vec<Box<dyn std::any::Any>>>,
    ) -> RunRes {
        let stdout = MemoryOutputPipe::new(q.out_cap);
        let stderr = MemoryOutputPipe::new(q.out_cap);
        let mut b = WasiCtxBuilder::new();
        b.stdin(MemoryInputPipe::new(q.input.clone()))
            .stdout(stdout.clone())
            .stderr(stderr.clone());
        b.arg("node");
        for a in &q.args {
            b.arg(a);
        }
        let common = Common {
            lim: Lim {
                mem_max: q.mem_mb << 20,
                tbl_max: 10_000,
            },
            logs: vec![],
            log_bytes: 0,
            log_cap: q.log_cap,
            emits: vec![],
            input: q.input.clone(),
            output: vec![],
        };
        let mut res = RunRes::default();
        let ticks = if self.opts.epoch {
            q.limit_ms.max(1)
        } else {
            0
        };
        let outcome: Result<()> = match l {
            Loaded::Core(_) | Loaded::Dyn { .. } => {
                let mut store = Store::new(
                    &self.engine,
                    CoreHost {
                        c: common,
                        wasi: b.build_p1(),
                    },
                );
                store.limiter(|h| &mut h.c.lim);
                self.arm(&mut store, ticks, q.fuel);
                let r = (|| -> Result<()> {
                    let inst = match l {
                        Loaded::Core(pre) => pre.instantiate(&mut store)?,
                        Loaded::Dyn { plugin, node } => {
                            let mut lk = self.core.clone();
                            let pi = lk.instantiate(&mut store, plugin)?;
                            lk.instance(&mut store, "sbx_node_plugin_v1", pi)?;
                            lk.instantiate(&mut store, node)?
                        }
                        _ => unreachable!(),
                    };
                    res.t_inst_us = t0.elapsed().as_micros();
                    let f = match inst.get_typed_func::<(), ()>(&mut store, "_start") {
                        Ok(f) => f,
                        Err(_) => inst.get_typed_func::<(), ()>(&mut store, "run")?,
                    };
                    res.t_start_us = t0.elapsed().as_micros();
                    mark_start();
                    f.call(&mut store, ())
                })();
                let r = match r {
                    Err(e) if e.downcast_ref::<I32Exit>().is_some_and(|x| x.0 == 0) => Ok(()),
                    o => o,
                };
                res.t_total_us = t0.elapsed().as_micros();
                let h = store.data_mut();
                res.logs = std::mem::take(&mut h.c.logs);
                res.emits = std::mem::take(&mut h.c.emits);
                res.host_output = std::mem::take(&mut h.c.output);
                if let Some(v) = hold {
                    v.push(Box::new(store));
                }
                r
            }
            Loaded::Comp(pre) => {
                let h = CompHost {
                    c: common,
                    ctx: b.build(),
                    table: ResourceTable::new(),
                };
                let mut store = Store::new(&self.engine, h);
                store.limiter(|h| &mut h.c.lim);
                self.arm(&mut store, ticks, q.fuel);
                let r = (|| -> Result<()> {
                    let cmd = wasmtime_wasi::p2::bindings::sync::CommandPre::new(pre.clone())?
                        .instantiate(&mut store)?;
                    res.t_inst_us = t0.elapsed().as_micros();
                    res.t_start_us = res.t_inst_us;
                    mark_start();
                    cmd.wasi_cli_run()
                        .call_run(&mut store)?
                        .map_err(|_| Error::msg("guest returned error"))
                })();
                let r = match r {
                    Err(e) if e.downcast_ref::<I32Exit>().is_some_and(|x| x.0 == 0) => Ok(()),
                    o => o,
                };
                res.t_total_us = t0.elapsed().as_micros();
                let h = store.data_mut();
                res.logs = std::mem::take(&mut h.c.logs);
                res.emits = std::mem::take(&mut h.c.emits);
                if let Some(v) = hold {
                    v.push(Box::new(store));
                }
                r
            }
        };
        res.failure = outcome.err().map(|e| classify(&e));
        res.stdout = stdout.contents().to_vec();
        res.stderr = stderr.contents().to_vec();
        res
    }

    fn arm<T>(&self, s: &mut Store<T>, ticks: u64, fuel: u64) {
        if self.opts.epoch {
            s.set_epoch_deadline(ticks);
        }
        if self.opts.fuel {
            s.set_fuel(fuel).unwrap();
        }
    }
}

/// Esito della compilazione di un modulo caricato dal tenant.
pub struct Compiled {
    pub path: PathBuf,
    pub cwasm_sha: String,
    pub validate_us: u128,
    pub compile_us: u128,
    pub cwasm_len: usize,
}

/// Valida e compila i byte del tenant; scrive `<sha modulo>-<cfg>.cwasm` con `rename` (mai sul posto) e
/// il file `.sha` con l'impronta dell'artefatto, che il motore conserva e riverifica al caricamento.
pub fn compile(engine: &Engine, bytes: &[u8], comp: bool, outdir: &Path) -> Result<Compiled> {
    let t = Instant::now();
    if comp {
        // ponytail: i componenti si validano compilandoli; nessun passaggio separato.
    } else {
        Module::validate(engine, bytes)?;
    }
    let validate_us = t.elapsed().as_micros();
    let t = Instant::now();
    let cw = if comp {
        engine.precompile_component(bytes)?
    } else {
        engine.precompile_module(bytes)?
    };
    let compile_us = t.elapsed().as_micros();
    std::fs::create_dir_all(outdir)?;
    let stem = format!("{}-{}", sha_hex(bytes), cfg_hash(engine));
    let path = outdir.join(format!("{stem}.cwasm"));
    let tmp = outdir.join(format!("{stem}.tmp"));
    std::fs::write(&tmp, &cw)?;
    std::fs::rename(&tmp, &path)?;
    let sha = sha_hex(&cw);
    std::fs::write(outdir.join(format!("{stem}.sha")), &sha)?;
    Ok(Compiled {
        path,
        cwasm_sha: sha,
        validate_us,
        compile_us,
        cwasm_len: cw.len(),
    })
}

/// Memoria del processo: (Rss, Pss, VmHWM, thread) in KiB e conteggio, da /proc.
pub fn mem_stat(pid: &str) -> (u64, u64, u64, u64) {
    let kv = |path: String, key: &str| -> u64 {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with(key))
                    .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
            })
            .unwrap_or(0)
    };
    (
        kv(format!("/proc/{pid}/smaps_rollup"), "Rss:"),
        kv(format!("/proc/{pid}/smaps_rollup"), "Pss:"),
        kv(format!("/proc/{pid}/status"), "VmHWM:"),
        kv(format!("/proc/{pid}/status"), "Threads:"),
    )
}

/// Cerca nella memoria leggibile di un processo il valore sentinella (passato mascherato con XOR 0x5A, così
/// il processo che cerca non tiene mai una copia in chiaro). Restituisce `(occorrenze, byte letti)`.
pub fn scan_mem(pid: &str, masked: &[u8]) -> std::io::Result<(usize, u64)> {
    use std::os::unix::fs::FileExt;
    let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
    let mem = std::fs::File::open(format!("/proc/{pid}/mem"))?;
    let (mut hits, mut read) = (0usize, 0u64);
    let mut buf = vec![0u8; 1 << 20];
    let m = masked.len();
    for line in maps.lines() {
        let mut it = line.split_whitespace();
        let (range, perms) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
        let name = it.nth(3).unwrap_or("");
        if !perms.starts_with('r') || name.starts_with("[vvar") || name.starts_with("[vsys") {
            continue;
        }
        let (a, b) = range.split_once('-').unwrap_or(("0", "0"));
        let (mut pos, end) = (
            u64::from_str_radix(a, 16).unwrap_or(0),
            u64::from_str_radix(b, 16).unwrap_or(0),
        );
        let mut carry = 0usize;
        while pos < end {
            let n = ((end - pos) as usize).min(buf.len() - carry);
            let Ok(got) = mem.read_at(&mut buf[carry..carry + n], pos) else {
                break;
            };
            if got == 0 {
                break;
            }
            read += got as u64;
            let tot = carry + got;
            let w = &buf[..tot];
            if tot >= m {
                for i in 0..=tot - m {
                    if w[i] ^ 0x5A == masked[0]
                        && w[i..i + m].iter().zip(masked).all(|(x, y)| x ^ 0x5A == *y)
                    {
                        hits += 1;
                    }
                }
            }
            carry = (m - 1).min(tot);
            buf.copy_within(tot - carry..tot, 0);
            pos += got as u64;
        }
    }
    Ok((hits, read))
}
