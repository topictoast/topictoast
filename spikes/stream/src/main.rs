//! Spike S-3 (pipeline stream su DataFusion): percorso principale (parte 1), ciclo di vita, dialetto e barriere
//! (parte 2a), confinamento, isolamento, latenze e RAM (parte 2b). Codice da buttare.
//! Sottocomandi: `engine`, `stream` (processo per tenant, lanciato dal motore), `load`, `check`, `probe`.
//!
//! Il ruolo `stream` non parte con `#[tokio::main]`: il passo 1 del confinamento (chiudere i descrittori ereditati) deve
//! avvenire PRIMA che il runtime apra i suoi (epoll, eventfd), altrimenti li chiuderebbe.
mod bus;
mod check;
mod confine;
mod crypto;
mod dialect;
mod engine;
mod evgen;
mod http;
mod load;
mod metrics;
mod pipeline;
mod probe;
mod spec;
mod stream;
mod wire;

fn main() {
    let role = std::env::args().nth(1);
    if role.as_deref() == Some("stream") && !std::env::args().any(|a| a == "--no-confine") {
        let _ = stream::PRE.set(confine::prologue());
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        match role.as_deref() {
            Some("engine") => engine::run().await,
            Some("stream") => stream::run().await,
            Some("load") => load::run().await,
            Some("check") => check::run().await,
            Some("probe") => probe::run().await,
            _ => eprintln!("uso: streamspike engine|stream|load|check|probe [--opzioni]"),
        }
    });
}
