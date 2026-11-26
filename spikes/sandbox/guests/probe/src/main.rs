//! Modulo ostile: tenta ogni cosa che un nodo non deve poter fare. Per ogni tentativo stampa una riga
//! `<tentativo> ok <byte>` oppure `<tentativo> negato <errore>`; i byte letti con successo sono stampati
//! in chiaro, così l'host può cercarci il valore sentinella senza che il guest lo conosca.
//! Argomenti: i percorsi da aprire (uno per argomento).
use std::io::Read;

fn main() {
    // 1. file system: ogni percorso passato come argomento
    for p in std::env::args().skip(1) {
        match std::fs::File::open(&p) {
            Ok(mut f) => {
                let mut b = vec![0u8; 4096];
                let n = f.read(&mut b).unwrap_or(0);
                println!("fs {p} ok {n} {}", String::from_utf8_lossy(&b[..n]));
            }
            Err(e) => println!("fs {p} negato {e}"),
        }
    }
    // 2. ambiente: quante variabili vede il guest
    println!("env count {}", std::env::vars().count());
    // 3. rete
    match std::net::TcpStream::connect("127.0.0.1:9") {
        Ok(_) => println!("tcp ok"),
        Err(e) => println!("tcp negato {e}"),
    }
    match std::net::UdpSocket::bind("127.0.0.1:0") {
        Ok(_) => println!("udp ok"),
        Err(e) => println!("udp negato {e}"),
    }
    // 4. processi
    match std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("id")
        .output()
    {
        Ok(_) => println!("proc ok"),
        Err(e) => println!("proc negato {e}"),
    }
    // 5. descrittori ereditati: gli indici WASI oltre stdio non esistono
    for fd in 3..16 {
        let mut b = [0u8; 1];
        let r = unsafe { fd_read_probe(fd, &mut b) };
        println!("fd {fd} {}", if r == 0 { "ok" } else { "negato" });
    }
}

/// Lettura diretta di un fd WASI (restituisce 0 se riuscita, altrimenti l'errno).
unsafe fn fd_read_probe(fd: u32, buf: &mut [u8]) -> u16 {
    #[link(wasm_import_module = "wasi_snapshot_preview1")]
    unsafe extern "C" {
        fn fd_read(fd: u32, iovs: *const [u32; 2], n: usize, out: *mut usize) -> u16;
    }
    let iov = [buf.as_mut_ptr() as u32, buf.len() as u32];
    let mut n = 0usize;
    unsafe { fd_read(fd, &iov, 1, &mut n) }
}
