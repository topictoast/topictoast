//! Spike S-2, fase 2: livello leggero della sandbox. Un solo binario con i sottocomandi:
//!   aux <cfg>                       processo ausiliario di un tenant (protocollo a righe su stdin/stdout)
//!   compile <in> <outdir> [cfg=..] [kind=core|comp]   valida e compila nella cache nativa (solo questo la scrive)
//!   mkguests <dir>                  scrive i moduli WAT (vuoto e ostili) come .wasm
//!   mkbig <kind> <n> <out>          modulo sintetico grande per L10
//!   drive <scenario> [k=v ...]      pilota di carico e di prova (vedi drv.rs)
//! Codice da buttare: non è un prototipo del prodotto.

mod aux;
mod drv;
mod hostile;
mod rt;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("aux") => aux::main(a.get(2).map(String::as_str).unwrap_or("epoch")),
        Some("compile") => compile_cmd(&a[2..]),
        Some("mkguests") => {
            std::fs::create_dir_all(&a[2]).unwrap();
            for (n, w) in hostile::GUESTS {
                std::fs::write(format!("{}/{n}.wasm", a[2]), wat::parse_str(w).unwrap()).unwrap();
            }
        }
        Some("mkbig") => {
            let n: usize = a[3].parse().unwrap();
            let src = if a[2] == "ifs" {
                hostile::ifs_module(n)
            } else {
                hostile::big_module(&a[2], n)
            };
            std::fs::write(&a[4], wat::parse_str(src).unwrap()).unwrap();
        }
        Some("imports") => {
            // importazioni ed esportazioni di un modulo core (per L9 e L12)
            let e = rt::make_engine(&rt::Opts::parse("epoch")).unwrap();
            match wasmtime::Module::new(&e, std::fs::read(&a[2]).unwrap()) {
                Ok(m) => {
                    for i in m.imports() {
                        println!("import {}.{}", i.module(), i.name());
                    }
                    for x in m.exports() {
                        println!("export {}", x.name());
                    }
                }
                Err(e) => println!("err {e:#}"),
            }
        }
        Some("drive") => drv::main(&a[2..]),
        _ => {
            eprintln!("uso: sbxspike aux|compile|mkguests|mkbig|drive ...");
            std::process::exit(2);
        }
    }
}

/// `compile`: legge i byte del tenant, valida, compila, scrive la cache. Una riga di risultato su stdout.
fn compile_cmd(a: &[String]) {
    let kv: std::collections::HashMap<_, _> =
        a[2..].iter().filter_map(|t| t.split_once('=')).collect();
    if let Some(mb) = kv.get("as_mb").and_then(|v| v.parse::<u64>().ok()) {
        // tetto di memoria virtuale del processo che compila (setrlimit, senza root)
        let l = libc::rlimit {
            rlim_cur: mb << 20,
            rlim_max: mb << 20,
        };
        unsafe { libc::setrlimit(libc::RLIMIT_AS, &l) };
    }
    let rt_opts = rt::Opts::parse(kv.get("cfg").copied().unwrap_or("epoch"));
    let engine = rt::make_engine(&rt_opts).expect("engine");
    let bytes = std::fs::read(&a[0]).expect("lettura del modulo");
    let t = std::time::Instant::now();
    match rt::compile(
        &engine,
        &bytes,
        kv.get("kind") == Some(&"comp"),
        std::path::Path::new(&a[1]),
    ) {
        Ok(c) => println!(
            "ok path={} sha={} in_len={} cwasm_len={} validate_us={} compile_us={} total_us={}",
            c.path.display(),
            c.cwasm_sha,
            bytes.len(),
            c.cwasm_len,
            c.validate_us,
            c.compile_us,
            t.elapsed().as_micros()
        ),
        Err(e) => {
            println!("err {}", format!("{e:#}").replace('\n', " | "));
            std::process::exit(1);
        }
    }
}
