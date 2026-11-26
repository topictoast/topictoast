//! Spike S-3 (pipeline stream su DataFusion), parte 1: percorso principale. Codice da buttare.
//! Sottocomandi: `engine`, `stream` (processo per tenant, lanciato dal motore), `load`, `check`.

mod bus;
mod check;
mod crypto;
mod engine;
mod evgen;
mod http;
mod load;
mod pipeline;
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
        _ => eprintln!("uso: streamspike engine|stream|load|check [--opzioni]"),
    }
}
