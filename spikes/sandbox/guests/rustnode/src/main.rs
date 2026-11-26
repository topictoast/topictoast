//! Entrata WASI (p1 e p2): ingresso su stdin, uscita su stdout.
use std::io::{Read, Write};

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let out = rustnode::node(&s);
    std::io::stdout().write_all(out.as_bytes()).unwrap();
}
