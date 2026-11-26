//! Moduli di prova scritti in WAT: il nodo vuoto e i moduli ostili noti (L5, L6, L12). Il sottocomando
//! `mkguests` li traduce in `.wasm`; i moduli Rust e JavaScript si costruiscono con `guests/build.sh`.

pub const GUESTS: &[(&str, &str)] = &[
    (
        "empty",
        r#"(module (memory (export "memory") 1) (func (export "_start")))"#,
    ),
    // L5: ciclo infinito
    (
        "loop",
        r#"(module (memory (export "memory") 1) (func (export "_start") (loop $l (br $l))))"#,
    ),
    // L5: una sola copia in blocco da 1 GiB (l'epoca è controllata solo all'inizio dell'istruzione)
    (
        "copybig",
        r#"(module (memory (export "memory") 17000)
            (func (export "_start")
              (memory.copy (i32.const 0) (i32.const 4096) (i32.const 1073741824))
              (loop $l (br $l))))"#,
    ),
    // L5: copie da 16 MiB senza fine, dentro il tetto di memoria
    (
        "copyloop",
        r#"(module (memory (export "memory") 512)
            (func (export "_start")
              (loop $l (memory.copy (i32.const 0) (i32.const 16777216) (i32.const 16777216)) (br $l))))"#,
    ),
    // L6: memory.grow a blocchi da 16 MiB, toccando le pagine, finché il tetto lo ferma
    (
        "growmem",
        r#"(module (memory (export "memory") 1)
            (func (export "_start") (local $r i32)
              (loop $l
                (local.set $r (memory.grow (i32.const 256)))
                (if (i32.ne (local.get $r) (i32.const -1))
                  (then (memory.fill (i32.mul (local.get $r) (i32.const 65536)) (i32.const 1) (i32.const 16777216))))
                (br $l))))"#,
    ),
    // L6: crescita della tabella senza fine
    (
        "table",
        r#"(module (memory (export "memory") 1) (table 1 funcref)
            (func (export "_start")
              (loop $l (drop (table.grow (ref.null func) (i32.const 1000))) (br $l))))"#,
    ),
    // L6: ricorsione senza fine (pila)
    (
        "recurse",
        r#"(module (memory (export "memory") 1) (func $f (call $f)) (func (export "_start") (call $f)))"#,
    ),
    // L6: memoria iniziale dichiarata da 2,6 GB con un segmento di dati
    (
        "bigmem",
        r#"(module (memory (export "memory") 40000) (data (i32.const 0) "x") (func (export "_start")))"#,
    ),
    // L12: importazioni che l'host non offre
    (
        "evilimport",
        r#"(module (import "env" "escape" (func)) (import "topictoast" "shell" (func (param i32)))
            (memory (export "memory") 1) (func (export "_start") (call 0)))"#,
    ),
    // L12: socket e apertura di file con descrittori che il guest non ha; scrive gli errno su stdout
    (
        "sock",
        r#"(module
            (import "wasi_snapshot_preview1" "sock_accept" (func $accept (param i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "sock_send" (func $send (param i32 i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "path_open" (func $open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_write" (func $write (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 100) "/etc/passwd")
            (func (export "_start")
              (i32.store8 (i32.const 0) (call $accept (i32.const 3) (i32.const 0) (i32.const 200)))
              (i32.store8 (i32.const 1) (call $send (i32.const 3) (i32.const 300) (i32.const 0) (i32.const 0) (i32.const 200)))
              (i32.store8 (i32.const 2) (call $open (i32.const 3) (i32.const 0) (i32.const 100) (i32.const 11) (i32.const 0) (i64.const 2) (i64.const 0) (i32.const 0) (i32.const 200)))
              (i32.store (i32.const 16) (i32.const 0)) (i32.store (i32.const 20) (i32.const 3))
              (drop (call $write (i32.const 1) (i32.const 16) (i32.const 1) (i32.const 24)))))"#,
    ),
    // L12: righe di log enormi tramite la funzione dell'host
    (
        "logflood",
        r#"(module (import "topictoast" "log" (func $log (param i32 i32 i32))) (memory (export "memory") 2)
            (func (export "_start") (loop $l (call $log (i32.const 2) (i32.const 0) (i32.const 65536)) (br $l))))"#,
    ),
    // L12: scritture enormi su stderr tramite WASI
    (
        "errflood",
        r#"(module (import "wasi_snapshot_preview1" "fd_write" (func $w (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 2)
            (func (export "_start")
              (i32.store (i32.const 65540) (i32.const 0)) (i32.store (i32.const 65544) (i32.const 65536))
              (loop $l (drop (call $w (i32.const 2) (i32.const 65540) (i32.const 1) (i32.const 65560))) (br $l))))"#,
    ),
];

/// Moduli sintetici per L10: `n` funzioni di aritmetica e cicli, con dimensione circa `bytes`.
pub fn big_module(kind: &str, n: usize) -> String {
    let mut s = String::from("(module (memory (export \"memory\") 1)\n");
    match kind {
        // tante funzioni piccole e varie
        "funcs" => {
            for i in 0..n {
                s += &format!(
                    "(func $f{i} (param i32 i32) (result i32) (local i32) (local.set 2 (i32.add (local.get 0) (i32.const {i})))\n  (loop $l (local.set 2 (i32.xor (i32.mul (local.get 2) (i32.const 31)) (local.get 1))) (br_if $l (i32.lt_u (local.get 2) (i32.const {})))) (local.get 2))\n",
                    i + 1000
                );
            }
        }
        // una funzione con blocchi annidati a n livelli
        "nest" => {
            s += "(func (export \"f\") (param i32) (result i32)\n";
            for _ in 0..n {
                s += "(block ";
            }
            s += "(br_if 0 (local.get 0))";
            for _ in 0..n {
                s += ")";
            }
            s += "(local.get 0))\n";
        }
        // br_table con n destinazioni
        "brtable" => {
            s += "(func (export \"f\") (param i32) (result i32)\n(block $out\n";
            for _ in 0..8 {
                s += "(block ";
            }
            s += "(br_table";
            for i in 0..n {
                s += &format!(" {}", i % 8);
            }
            s += " 0 (local.get 0))";
            for _ in 0..8 {
                s += ")";
            }
            s += ")\n(i32.const 0))\n";
        }
        // una funzione lunga con n istruzioni in linea retta e molti locali
        "straight" => {
            s += "(func (export \"f\") (param i32) (result i32)\n";
            for i in 0..200 {
                s += "(local i32) ";
                let _ = i;
            }
            s += "(local.get 0)\n";
            for i in 0..n {
                s += &format!("(i32.const {}) (i32.add) (local.tee {}) \n", i, 1 + i % 200);
            }
            s += ")\n";
        }
        _ => {}
    }
    s += ")";
    s
}

/// Una funzione con `n` livelli di `if` annidati e molti rami (stress dell'allocatore dei registri).
pub fn ifs_module(n: usize) -> String {
    let mut s = String::from(
        "(module (func (export \"f\") (param i32) (result i32) (local i32 i32 i32 i32)\n",
    );
    for i in 0..n {
        s += &format!(
            "(if (i32.gt_u (local.get 0) (i32.const {i})) (then (local.set {} (i32.add (local.get {}) (local.get 0)))))\n",
            1 + i % 4,
            1 + (i + 1) % 4
        );
    }
    s += "(local.get 1))\n)";
    s
}
