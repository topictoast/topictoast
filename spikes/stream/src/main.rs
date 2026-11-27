//! Spike S-3 (pipeline stream su DataFusion): percorso principale (parte 1) e ciclo di vita, dialetto e barriere
//! (parte 2a). Codice da buttare.
//! Sottocomandi: `engine`, `stream` (processo per tenant, lanciato dal motore), `load`, `check`, `probe`.

// Punti di innesto per la parte 2b (non costruiti qui):
// - A1/A2: confinamento all'inizio di `stream::run` (dopo la lettura del messaggio iniziale sul descrittore 3);
// - I1/M1: isolamento tra tenant e RAM: `stream::run` espone gia' `cpu_ns`, `rss_kb` e `state_bytes` nelle statistiche;
// - S1: sentinella = campo di prova nel generatore `evgen` e lettura dei dati di Iggy e dei checkpoint.
mod bus;
mod check;
mod crypto;
mod dialect;
mod engine;
mod evgen;
mod http;
mod load;
mod pipeline;
mod probe;
mod spec;
mod stream;
mod wire;

#[tokio::main]
async fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("engine") => engine::run().await,
        Some("stream") => stream::run().await,
        Some("load") => load::run().await,
        Some("check") => check::run().await,
        Some("probe") => probe::run().await,
        _ => eprintln!("uso: streamspike engine|stream|load|check|probe [--opzioni]"),
    }
}
