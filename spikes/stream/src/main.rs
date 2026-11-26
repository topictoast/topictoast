//! Spike S-3 (pipeline stream su DataFusion), parte 1: percorso principale. Codice da buttare.
//! Sottocomandi: `engine`, `stream` (processo per tenant, lanciato dal motore), `load`, `check`.

// Punti di innesto per la parte 2 (non costruiti qui):
// - V1/V2: `pipeline::dialect_check` (elenco completo dei rifiuti, limiti di dimensione, funzioni ammesse) e
//   gli strati 1-3 in `Pipe::build` (contesto senza archivi di oggetti: gia' fatto; manca il controllo funzioni);
// - V3/I1/M1: la quota di stato per pipeline (64 MiB, `Hello::state_quota`) si misura sugli accumulatori in
//   `Pipe::process` (oggi passa solo come limite del pool di DataFusion, che non vede gli accumulatori);
// - A1/A2: confinamento all'inizio di `stream::run` (dopo la lettura del messaggio iniziale sul descrittore 3);
// - C5: sospensione/ripresa/modifica della query = `stream::run` costruisce le `Pipe` dalla versione e scarta lo
//   stato di una versione diversa (oggi il checkpoint si applica solo a id e versione uguali);
// - C6: rifiuto per quota = rotta del motore da pilotare in `engine::handle` (`POST /v1/starts`); il lato
//   `stream::deliver` gia' trattiene la riga e riprova su ogni risposta diversa da 200;
// - C7: destinazione stream = secondo tipo di consegna in `stream::deliver`;
// - S1: sentinella = campo di prova nel generatore `evgen` e lettura dei dati di Iggy e dei checkpoint.
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
