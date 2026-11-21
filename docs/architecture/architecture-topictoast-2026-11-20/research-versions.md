# Ricerca versioni delle tecnologie candidate

**Data della verifica:** 2026-11-20 · **Soglia della regola "≥ 2 settimane":** rilasciata entro il 2026-11-06 (UTC).

Metodo: dati letti direttamente dalle API pubbliche di crates.io (`/api/v1/crates/<nome>`), dal registry npm
(`registry.npmjs.org/<pacchetto>`, campi `time` e `dist-tags`), dalle release e dagli advisory di sicurezza GitHub
(`gh api repos/<repo>/releases`, `.../security-advisories`) e dai README o dalla documentazione ufficiale al tag
indicato. Date in UTC. Le versioni yanked e le pre-release (`-rc`, `-edge`, `-dev`) sono escluse da "Ultima" e da
"Fissabile".

Lettura delle colonne:

- **Ultima / Data:** ultima versione stabile e data di pubblicazione.
- **Fissabile (≥ 2 settimane):** `sì — X` se l'ultima rispetta la regola; altrimenti `no → X (data)`, dove X è la
  versione più alta pubblicata entro il 2026-11-06.
- **Nome:** il link è la fonte della riga; altre fonti sono in Note.
- "non verificato" = non trovato nelle fonti lette; "non pubblicato" = la fonte ufficiale non riporta il dato.

## 1. Rust

| Nome | Ultima | Data | Fissabile (≥ 2 settimane) | Licenza | Note |
|---|---|---|---|---|---|
| [Rust stable (toolchain)](https://github.com/rust-lang/rust/releases) | 1.100.0 | 2026-11-12 | no → 1.99.1 (2026-10-16) | MIT OR Apache-2.0 | Rilascio ogni 6 settimane. MSRV dei crate sotto rispetto a 1.99: non verificato. |
| [tokio](https://crates.io/crates/tokio) | 1.54.1 | 2026-10-21 | sì — 1.54.1 | MIT | 1.54.1 corregge GHSA-5wjr-9356-4wc8 (race in `time::Interval`, medium): la versione fissabile lo include. |
| [axum](https://crates.io/crates/axum) | 0.8.9 | 2026-04-14 | sì — 0.8.9 | MIT | Pre-1.0. Ultimo rilascio 7 mesi fa; repo attivo (push 2026-11-19). |
| [hyper](https://crates.io/crates/hyper) | 1.12.0 | 2026-10-06 | sì — 1.12.0 | MIT | 1.x stabile. |
| [tonic](https://crates.io/crates/tonic) | 0.14.9 | 2026-10-29 | sì — 0.14.9 | MIT | Pre-1.0. Repo spostato in [`grpc/grpc-rust`](https://github.com/grpc/grpc-rust). Codegen in `tonic-prost-build`. |
| [tonic-prost](https://crates.io/crates/tonic-prost) | 0.14.9 | 2026-10-29 | sì — 0.14.9 | MIT | Allineato a tonic. |
| [tonic-prost-build](https://crates.io/crates/tonic-prost-build) | 0.14.9 | 2026-10-29 | sì — 0.14.9 | MIT | Allineato a tonic. |
| [prost](https://crates.io/crates/prost) | 0.14.6 | 2026-11-12 | no → 0.14.4 (2026-06-07) | Apache-2.0 | Pre-1.0. |
| [prost-build](https://crates.io/crates/prost-build) | 0.14.6 | 2026-11-12 | no → 0.14.4 (2026-06-07) | Apache-2.0 | Allineato a prost. |
| [rustls](https://crates.io/crates/rustls) | 0.23.46 | 2026-10-14 | sì — 0.23.46 | Apache-2.0 OR ISC OR MIT | 0.23.46 corregge i 4 advisory del 2026-10-27 (es. GHSA-c8m5-c952-6cm4, segreti TLS nel `Debug`): la fissabile li include. 0.24 solo in pre-release (`0.24.0-dev.3`). |
| [sqlx](https://crates.io/crates/sqlx) | 0.9.1 | 2026-11-18 | no → 0.9.0 (2026-05-21) | MIT OR Apache-2.0 | Postgres, MySQL/MariaDB, SQLite (feature `postgres`, `sqlite`, `sqlite-unbundled`). Controlli a compile-time offline: `cargo sqlx prepare` salva i metadati in `.sqlx/`, `SQLX_OFFLINE=true` forza l'offline ([README](https://github.com/launchbadge/sqlx/blob/v0.9.0/README.md), [sqlx-cli](https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-cli/README.md)). `sqlx-sqlite`, `sqlx-postgres` e `sqlx-cli` hanno stessa versione e data. Pre-1.0. |
| [rusqlite](https://crates.io/crates/rusqlite) | 0.43.0 | 2026-11-15 | no → 0.41.1 (2026-10-28) | MIT | Alternativa sincrona per SQLite. Due minor (0.42.0 il 2026-11-11, 0.43.0) in 4 giorni: pre-1.0, minor = breaking. |
| [serde](https://crates.io/crates/serde) | 1.0.229 | 2026-07-18 | sì — 1.0.229 | MIT OR Apache-2.0 | Stabile. |
| [serde_json](https://crates.io/crates/serde_json) | 1.0.154 | 2026-10-11 | sì — 1.0.154 | MIT OR Apache-2.0 | Stabile. |
| [thiserror](https://crates.io/crates/thiserror) | 2.0.21 | 2026-09-23 | sì — 2.0.21 | MIT OR Apache-2.0 | Stabile. |
| [tracing](https://crates.io/crates/tracing) | 0.1.44 | 2025-12-18 | sì — 0.1.44 | MIT | Ultimo rilascio 11 mesi fa; repo attivo (push 2026-11-12). |
| [tracing-subscriber](https://crates.io/crates/tracing-subscriber) | 0.3.23 | 2026-03-13 | sì — 0.3.23 | MIT | — |
| [opentelemetry](https://crates.io/crates/opentelemetry) | 0.33.1 | 2026-10-09 | sì — 0.33.1 | Apache-2.0 | Pre-1.0: per le regole semver di Cargo, da 0.33 a 0.34 è un cambio incompatibile. `opentelemetry`, `_sdk` e `-otlp` escono con la stessa versione. |
| [opentelemetry_sdk](https://crates.io/crates/opentelemetry_sdk) | 0.33.1 | 2026-10-09 | sì — 0.33.1 | Apache-2.0 | Vedi sopra. |
| [opentelemetry-otlp](https://crates.io/crates/opentelemetry-otlp) | 0.33.1 | 2026-10-09 | sì — 0.33.1 | Apache-2.0 | Vedi sopra. |
| [tracing-opentelemetry](https://crates.io/crates/tracing-opentelemetry) | 0.34.0 | 2026-09-23 | sì — 0.34.0 | MIT | Abbinamento esatto con opentelemetry 0.33: non verificato. |
| [prometheus-client](https://crates.io/crates/prometheus-client) | 0.26.0 | 2026-11-11 | no → 0.25.2 (2026-10-30) | Apache-2.0 OR MIT | Client ufficiale (`prometheus/client_rust`). Pre-1.0. |
| [metrics](https://crates.io/crates/metrics) | 0.24.6 | 2026-05-13 | sì — 0.24.6 | MIT | Facade alternativa. Pre-1.0. |
| [metrics-exporter-prometheus](https://crates.io/crates/metrics-exporter-prometheus) | 0.18.3 | 2026-04-30 | sì — 0.18.3 | MIT AND Apache-2.0 | Licenza dichiarata su crates.io come `MIT AND Apache-2.0`. |
| [jsonwebtoken](https://crates.io/crates/jsonwebtoken) | 12.0.0 | 2026-11-13 | no → 11.1.0 (2026-09-16) | MIT | EdDSA supportato; obbligatorio scegliere un backend: feature `aws_lc_rs` oppure `rust_crypto` ([README 11.1.0](https://github.com/Keats/jsonwebtoken/blob/v11.1.0/README.md)). 12.0.0 è breaking e contiene correzioni di sicurezza assenti in 11.1.0: "Fix remaining type confusion with sub, iss, aud", "Reject unknown critical extensions" ([CHANGELOG](https://github.com/Keats/jsonwebtoken/blob/v12.0.0/CHANGELOG.md)). |
| [pasetors](https://crates.io/crates/pasetors) | 0.8.1 | 2026-08-30 | sì — 0.8.1 | MIT | PASETO v4 (v4.public = Ed25519 per specifica), v3.public, v2; PASERK parziale; `forbid(unsafe_code)`, no_std/wasm ([README](https://github.com/brycx/pasetors)). Pre-1.0. |
| [age](https://crates.io/crates/age) | 0.12.1 | 2026-07-14 | sì — 0.12.1 | MIT OR Apache-2.0 | Libreria di `rage` (formato age). Pre-1.0. Push 2026-08-20. |
| [chacha20poly1305](https://crates.io/crates/chacha20poly1305) | 0.11.0 | 2026-06-28 | sì — 0.11.0 | Apache-2.0 OR MIT | RustCrypto AEADs, pre-1.0; repo attivo (push 2026-11-13). |
| [aes-gcm](https://crates.io/crates/aes-gcm) | 0.11.1 | 2026-08-21 | sì — 0.11.1 | Apache-2.0 OR MIT | Come sopra. |
| [wasmtime](https://crates.io/crates/wasmtime) | 51.0.0 | 2026-11-20 | no → 50.0.1 (2026-10-29) | Apache-2.0 WITH LLVM-exception | In 50.0.1 ci sono pooling allocator (`Config::allocation_strategy` + `PoolingAllocationConfig`), fuel (`Config::consume_fuel`), epoch (`Config::epoch_interruption`), component model ([docs.rs](https://docs.rs/wasmtime/50.0.1/wasmtime/struct.Config.html)). `component-model` e `component-model-async` sono **Tier 1** ([stability tiers](https://docs.wasmtime.dev/stability-tiers.html)). I backport di sicurezza arrivano su più rami insieme (36, 48, 49, 50). **50.0.2 (11-16) e 50.0.3 (11-19) correggono 9 advisory assenti in 50.0.1**: vedi § Sicurezza. |
| [wasmtime-wasi](https://crates.io/crates/wasmtime-wasi) | 51.0.0 | 2026-11-20 | no → 50.0.1 (2026-10-29) | Apache-2.0 WITH LLVM-exception | Allineato a wasmtime. |
| [jaq-core](https://crates.io/crates/jaq-core) | 3.1.2 | 2026-10-21 | sì — 3.1.2 | MIT | Libreria. `import`/`include` disattivati di default: `Loader::new` risponde "module loading not supported" finché l'host non passa `with_read`. Dettagli in § jaq. |
| [jaq-std](https://crates.io/crates/jaq-std) | 3.0.4 | 2026-10-21 | sì — 3.0.4 | MIT | `env` è un filtro nativo della feature `std`: c'è in `funs()`/`extra_funs()`, non in `base_funs()`. L'host sceglie quali nativi registrare. |
| [jaq-json](https://crates.io/crates/jaq-json) | 2.0.3 | 2026-08-28 | sì — 2.0.3 | MIT | Tipo valore JSON per jaq-core. |
| [jaq-all](https://crates.io/crates/jaq-all) | 0.4.0 | 2026-10-21 | sì — 0.4.0 | MIT | Raccoglie formati e std. Lo usa il playground wasm. Attenzione: `jaq_all::base_funs()` registra `env` (tramite `jaq_std::funs`) e `input_filename`. La CLI `jaq` è 3.2.0 (2026-11-19), fissabile 3.1.2. |
| [sqlparser](https://crates.io/crates/sqlparser) | 0.64.0 | 2026-11-09 | no → 0.63.0 (2026-09-13) | Apache-2.0 | Pre-1.0, minor = breaking. Repo `apache/datafusion-sqlparser-rs`. |
| [datafusion](https://crates.io/crates/datafusion) | 55.3.0 | 2026-11-17 | no → 55.2.0 (2026-10-07) | Apache-2.0 | Major frequenti. |
| [arroyo](https://github.com/ArroyoSystems/arroyo/releases) | v0.16.0 (GitHub) | 2026-11-17 | no → v0.15.0 (2025-12-01) | MIT OR Apache-2.0 (README) | **Nessun crate del motore pubblicato**: si distribuisce come binario singolo o Docker ([README](https://github.com/ArroyoSystems/arroyo/blob/v0.16.0/README.md)). Su crates.io non c'è il motore: il crate `arroyo` 0.7.0 (2023, owner `mwylde`, 15 download recenti) non è il prodotto; gli altri crate pubblicati sono solo `arroyo-udf-common`, `-macros` e `-plugin` (0.3.0). Embedding come libreria: non documentato. Usa un fork di DataFusion. Cloudflare: secondo le [note di v0.16.0](https://github.com/ArroyoSystems/arroyo/releases/tag/v0.16.0) lo gestisce come servizio (Pipelines) e la 0.17 sarà "the last before 1.0". |
| [denormalized](https://github.com/probably-nothing-labs/denormalized) | nessuna release | — | no — nessuna | Apache-2.0 | Crate 0.1.0 (2024-07-25) yanked. Ultimo push 2024-12-18. README: "work-in-progress". Fermo. |
| [dbsp](https://crates.io/crates/dbsp) (Feldera) | 0.385.0 | 2026-11-20 | no → 0.378.0 (2026-11-06, al limite) | MIT OR Apache-2.0 | Una 0.x quasi ogni giorno (sette minor dal 2026-11-07). [Repo](https://github.com/feldera/feldera/blob/main/LICENSE): edizione open-source MIT più parti "Enterprise" sotto un'altra licenza; il crate dichiara MIT OR Apache-2.0. |
| [cucumber](https://crates.io/crates/cucumber) | 0.23.0 | 2026-04-23 | sì — 0.23.0 | MIT OR Apache-2.0 | Pre-1.0. Ultimo push 2026-07-27. |
| [testcontainers](https://crates.io/crates/testcontainers) | 0.28.0 | 2026-08-06 | sì — 0.28.0 | MIT OR Apache-2.0 | Push 2026-11-16. |
| [testcontainers-modules](https://crates.io/crates/testcontainers-modules) | 0.15.0 | 2026-02-21 | sì — 0.15.0 | MIT | Compatibilità con testcontainers 0.28: non verificata. |
| [wiremock](https://crates.io/crates/wiremock) | 0.6.5 | 2025-08-24 | sì — 0.6.5 | MIT OR Apache-2.0 | **Ultimo rilascio e ultimo push 2025-08-24 (15 mesi).** Manutenzione ferma o minima. |
| [ts-rs](https://crates.io/crates/ts-rs) | 12.0.1 | 2026-01-31 | sì — 12.0.1 | MIT | Push 2026-11-09. |
| [wasm-bindgen](https://crates.io/crates/wasm-bindgen) | 0.2.130 | 2026-10-30 | sì — 0.2.130 | MIT OR Apache-2.0 | Repo nell'org [`wasm-bindgen`](https://github.com/wasm-bindgen/wasm-bindgen); rilasci regolari (8 tra maggio e ottobre 2026). |
| [wasm-pack](https://crates.io/crates/wasm-pack) | 0.15.1 | 2026-10-30 | sì — 0.15.1 | MIT OR Apache-2.0 | Ora in org [`wasm-bindgen`](https://github.com/wasm-bindgen/wasm-pack). Rilasci ripresi dopo 0.13.1 (2024-10-29): 0.14.0 (2026-01-20), 0.15.0 (2026-05-15), 0.15.1. |
| [jsonschema](https://crates.io/crates/jsonschema) | 0.61.2 | 2026-11-12 | no → 0.60.2 (2026-10-26) | MIT | Draft 2020-12, 2019-09, 7 ([README](https://github.com/Stranger6667/jsonschema), conformità su bowtie). Pre-1.0 con minor frequenti. |
| [jtd](https://crates.io/crates/jtd) (JSON Type Definition, RFC 8927) | 0.3.1 | 2021-01-31 | sì — 0.3.1 | MIT | **Fermo dal 2021** (ultimo push 2023-02-15); anche `jtd-codegen` e `jtd-infer` sono fermi al 2021. Su crates.io nessuna alternativa JTD mantenuta. |
| [croner](https://crates.io/crates/croner) | 4.0.3 | 2026-11-04 | sì — 4.0.3 | MIT | POSIX/Vixie più `L`, `#`, `W` e fusi orari ([README](https://github.com/Hexagon/croner-rust)). Push 2026-11-19. |
| [cron](https://crates.io/crates/cron) | 0.17.0 | 2026-06-18 | sì — 0.17.0 | MIT OR Apache-2.0 | Push 2026-06-21. Sintassi esatta (campo secondi, DST): non verificato. |
| [iroh](https://crates.io/crates/iroh) | 1.5.0 | 2026-11-19 | no → 1.4.1 (2026-10-21) | MIT OR Apache-2.0 | 1.x stabile. **1.4.2 (2026-11-19) corregge 5 advisory su `iroh` e `iroh-relay` (2 high, 3 medium) assenti in 1.4.1; un sesto (low) è corretto solo in 1.5** ([advisory](https://github.com/n0-computer/iroh/security/advisories)). |

### jaq: compatibilità con jq e sandbox

Fonti: [README v3.1.2](https://github.com/01mf02/jaq/blob/v3.1.2/README.md), sorgenti al tag `v3.1.2` (`jaq-std/src/lib.rs`,
`jaq-core/src/load/mod.rs`, `jaq/src/main.rs`), manuale `docs/stdlib.dj`.

- **Compatibilità:** il README parla di compatibilità con jq "in most cases" e confronta i benchmark con jq 1.8.1.
  Un confronto puntuale con jq 1.7 non è pubblicato.
- **Assenti rispetto a jq:** `$__loc__`, `modulemeta`, `have_literal_numbers`, `have_decnum`,
  `$JQ_BUILD_CONFIGURATION`, `builtins`, `input_line_number`; gli operatori SQL-style `INDEX`, `JOIN`, `IN`;
  `--stream` con `tostream`, `fromstream`, `truncate_stream`.
- **Semantica diversa** (sezioni "Compatibility" del manuale): assegnamenti e path update, `limit` dentro un update,
  `reduce`/`foreach`, NaN, divisione per zero, chiavi non stringa, `input_filename`.
- **Disattivare l'ambiente e il file system:**
  - `$ENV` è definito solo dalla CLI (`jaq/src/main.rs:228`), come variabile globale: chi usa le librerie non lo
    vede se l'host non lo passa.
  - `input_filename` non è in jaq-core, jaq-std o jaq-json: lo definisce `jaq-all` (`src/data.rs`) e restituisce il
    valore che l'host gli passa.
  - `env` è un nativo di jaq-std (feature `std`). Ci sono due modi per escluderlo: usare `jaq_std::base_funs()`
    invece di `funs()`, oppure filtrare per nome l'iteratore dei nativi. `jaq_all::base_funs()` invece lo include.
  - `import`/`include` restano disattivati se l'host non fornisce un reader.
  - Ricerca fatta su tutto il sorgente v3.1.2 per `input_filename`, `ENV` ed `env`.
- **Garanzie:** niente panic, memory safety, nessun I/O avviato da filtri o dati (escluso il caricamento dei file
  prima dell'esecuzione). **Nessuna difesa dall'esaurimento delle risorse** (tempo, memoria, stack): il README lo
  dichiara. Due audit di Radically Open Security.
- **wasm32:** il crate `jaq-play` (playground su gedenkt.at/jaq) si compila con
  `wasm-pack build --target web` ([README jaq-play](https://github.com/01mf02/jaq/blob/v3.1.2/jaq-play/README.md)).

## 2. Bus

| Nome | Ultima | Data | Fissabile (≥ 2 settimane) | Licenza | Note |
|---|---|---|---|---|---|
| [Apache Iggy server](https://github.com/apache/iggy/releases) | server-0.10.0 | 2026-11-07 | no → server-0.9.0 (2026-09-18) | Apache-2.0 | Un giorno oltre la soglia. Dal [blog 0.10.0](https://iggy.apache.org/blogs/2026/11/09/release-0.10.0/): il server 0.10.0 rifiuta al login tutti i client più vecchi (0.9.x compresi) e non migra le data dir 0.9.x ("until 1.0.0" si riparte da vuoto); tutti i nodi del cluster devono avere la stessa build. **RAM a riposo: non pubblicata** (il blog riporta solo la CPU). |
| [iggy (crate client)](https://crates.io/crates/iggy) | 0.12.0 | 2026-11-07 | no → 0.11.0 (2026-09-18) | Apache-2.0 | 0.12.0 è l'SDK del server 0.10.0 (blog). Coppia fissabile: crate 0.11.0 con server-0.9.0, entrambi del 2026-09-18. **Il crate 0.10.0 esiste ma è del 2026-04-22, stessa data di server-0.8.0**: client e server hanno numerazioni sfasate, e non esiste un crate 0.10.x con la data del server 0.10.0. Molte versioni `-edge`. |
| [Valkey](https://github.com/valkey-io/valkey/releases) | 9.2.0 | 2026-11-19 | no → 9.1.3 (2026-10-26) | BSD-3-Clause | **9.1.4 (2026-11-18, uscita insieme a 9.0.8, 8.1.12, 8.0.13 e 7.2.16) è un rilascio "Upgrade urgency SECURITY"** per CVE-2026-130822 (use-after-free con `maxmemory-clients`), assente in 9.1.3. AOF con `appendfsync always`: fsync a ogni append, "Very very slow, very safe", con group commit; AOF multi-part (`appenddirname`) ([persistence](https://valkey.io/topics/persistence/)). Streams presenti (`XADD`, "append-only log", [streams](https://valkey.io/topics/streams-intro/)). |
| [redis (crate client)](https://crates.io/crates/redis) | 1.8.2 | 2026-11-14 | no → 1.8.1 (2026-10-30) | BSD-3-Clause | Client per Valkey; compatibilità ufficiale con Valkey 9: non verificata. 2.0 in RC (`2.0.0-rc.3`). |
| [fred (crate client)](https://crates.io/crates/fred) | 10.1.0 | 2025-02-27 | sì — 10.1.0 | MIT | Client alternativo. **Ultimo rilascio e ultimo push 2025-02-27**: fermo. |
| [NATS server (JetStream)](https://github.com/nats-io/nats-server/releases) | v2.15.3 | 2026-11-11 | no → v2.15.2 (2026-10-26) | Apache-2.0 | JetStream è dentro il server. **2.15.3 corregge 6 advisory (5 high, 1 medium) assenti in 2.15.2**, tra cui GHSA-p849-62j6-rhc5 (permessi leafnode) ([advisory](https://github.com/nats-io/nats-server/security/advisories)). Il 2026-11-12 NATS ha pubblicato circa 30 advisory; quelli corretti fino alla 2.15.2 sono già nella versione fissabile. Ramo 2.14.x mantenuto in parallelo. |
| [async-nats](https://crates.io/crates/async-nats) | 0.53.0 | 2026-11-17 | no → 0.52.0 (2026-11-06, al limite) | Apache-2.0 | Pre-1.0. |
| [fjall](https://crates.io/crates/fjall) | 3.1.13 | 2026-10-25 | sì — 3.1.13 | MIT OR Apache-2.0 | KV LSM embedded, **non un log né una coda**: andrebbe costruito sopra. Journal con `persist` esplicito; transazioni serializzabili opzionali ([README](https://github.com/fjall-rs/fjall)). Serie 3.x stabile, rilasci regolari. |
| [redb](https://crates.io/crates/redb) | 5.0.0 | 2026-11-18 | no → 4.5.0 (2026-11-05) | MIT OR Apache-2.0 | KV B-tree ACID embedded. 5.0.0 cambia la codifica di `Option<&str>`, `Option<String>` e simili, ed è breaking ([CHANGELOG](https://github.com/cberner/redb/blob/master/CHANGELOG.md)). |
| [slatedb](https://crates.io/crates/slatedb) | 0.18.1 | 2026-11-07 | no → 0.18.0 (2026-11-03) | Apache-2.0 | LSM su object storage. Pre-1.0. |
| [apalis](https://crates.io/crates/apalis) | 1.2.0 | 2026-11-14 | no → 1.0.0 (2026-10-12) | MIT OR Apache-2.0 | Libreria per code di job con backend separati; 1.x dal 2026-10-12. Maturità dei backend: non verificata. |
| [sled](https://crates.io/crates/sled) | 0.34.7 | 2021-09-12 | sì — 0.34.7 | MIT OR Apache-2.0 | Ultima stabile del 2021; la 1.0 è in alpha (ultima 2024-10-11). Sconsigliabile. |
| [okaywal](https://crates.io/crates/okaywal) | 0.3.1 | 2023-11-26 | sì — 0.3.1 | MIT OR Apache-2.0 | WAL; fermo dal 2023. |
| [yaque](https://crates.io/crates/yaque) | 0.6.6 | 2023-11-04 | sì — 0.6.6 | Apache-2.0 | Coda persistente su file; ferma dal 2023. |
| [commitlog](https://crates.io/crates/commitlog) | 0.2.0 | 2021-11-06 | sì — 0.2.0 | MIT | Fermo dal 2021. |
| [walrus-rust](https://crates.io/crates/walrus-rust) | 0.2.0 | 2025-10-19 | sì — 0.2.0 | MIT | WAL; 390 download recenti: immaturo. |

## 3. Sandbox

| Nome | Ultima | Data | Fissabile (≥ 2 settimane) | Licenza | Note |
|---|---|---|---|---|---|
| [Firecracker](https://github.com/firecracker-microvm/firecracker/releases) | v1.18.0 | 2026-11-09 | no → v1.17.0 (2026-09-10) | Apache-2.0 | Requisiti: accesso r/w a `/dev/kvm`, Linux x86_64 o aarch64; su EC2 solo istanze `.metal` ([getting started](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md)). La [kernel policy di v1.17.0](https://github.com/firecracker-microvm/firecracker/blob/v1.17.0/docs/kernel-policy.md) elenca i kernel host v5.10, v6.1 e v6.18, con fine supporto minima rispettivamente al 2024-01-31, al 2025-10-12 e al 2028-05-28; per il guest v6.1 la fine supporto minima è il 2026-09-02, per v6.18 il 2028-06-01. 1.17.0 include la correzione di GHSA-48ww-rqpp-2mgm (high). |
| [cloud-hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor/releases) | v55.0 | 2026-11-19 | no → v54.0 (2026-10-14) | Apache-2.0 e BSD-3-Clause (cartella `LICENSES`) | **v54.1 (2026-11-11) è un rilascio di sicurezza** (GHSA-3r3x-6hgg-2m38, use-after-free in virtio-pmem) assente in v54.0. Requisiti host: non verificati. GitHub non rileva la licenza (`null`). |
| [javy (CLI)](https://github.com/bytecodealliance/javy/releases) | v9.1.0 | 2026-07-30 | sì — v9.1.0 | Apache-2.0 WITH LLVM-exception | Motore QuickJS (`rquickjs` nel Cargo.toml). Moduli da 1–16 KB con linking dinamico, almeno 869 KB con linking statico ([README](https://github.com/bytecodealliance/javy/blob/v9.1.0/README.md)). Riceve JavaScript, quindi il TypeScript va prima compilato in JS. **Startup e memoria: non pubblicati.** Crate [`javy`](https://crates.io/crates/javy) 8.1.0 (2026-07-30). |
| [ComponentizeJS](https://www.npmjs.com/package/@bytecodealliance/componentize-js) | 0.23.0 | 2026-09-21 | sì — 0.23.0 | Apache-2.0 WITH LLVM-exception ([LICENSE](https://github.com/bytecodealliance/ComponentizeJS/blob/main/LICENSE); npm non la dichiara) | README: "**experimental project**, no guarantees … breaking changes may be made". SpiderMonkey tramite StarlingMonkey, pre-inizializzazione con Wizer, embedding di circa 8 MB. Riceve JS e un mondo WIT e produce un Wasm Component per Wasmtime. **Startup e memoria: non pubblicati.** |
| [StarlingMonkey](https://github.com/bytecodealliance/StarlingMonkey/releases) | v0.5.1 | 2026-11-13 | no → v0.4.0 (2026-10-26) | Apache-2.0 (GitHub) | Runtime JS su SpiderMonkey per Wasm Components, builtins WASI 0.2.0. Il README lo dice in produzione su Fastly Compute (JS) e nel Spin JS SDK di Fermyon. Pre-1.0. |

## 4. Frontend

| Nome | Ultima | Data | Fissabile (≥ 2 settimane) | Licenza | Note |
|---|---|---|---|---|---|
| [react](https://www.npmjs.com/package/react) | 19.3.1 | 2026-10-14 | sì — 19.3.1 | MIT | — |
| [react-dom](https://www.npmjs.com/package/react-dom) | 19.3.1 | 2026-10-14 | sì — 19.3.1 | MIT | — |
| [@xyflow/react](https://www.npmjs.com/package/@xyflow/react) | 13.0.2 | 2026-11-17 | no → 12.12.1 (2026-11-02) | MIT | **Major 13.0.0 uscita il 2026-11-16**: troppo recente. |
| [@dagrejs/dagre](https://www.npmjs.com/package/@dagrejs/dagre) | 3.1.3 | 2026-10-24 | sì — 3.1.3 | MIT | Il vecchio pacchetto `dagre` (0.8.5, 2019) è fermo. |
| [elkjs](https://www.npmjs.com/package/elkjs) | 0.12.1 | 2026-10-28 | sì — 0.12.1 | EPL-2.0 OR GPL-3.0-or-later | `latest` = 0.12.1; la 0.13.0 è uscita lo stesso giorno senza dist-tag né release GitHub, quindi va ignorata. **Licenza copyleft (EPL-2.0 debole o GPL-3.0)**: da valutare per un kit embeddable. |
| [@codemirror/view](https://www.npmjs.com/package/@codemirror/view) | 6.43.15 | 2026-10-19 | sì — 6.43.15 | MIT | Sorgenti spostati su `code.haverbeke.berlin`. |
| [@codemirror/state](https://www.npmjs.com/package/@codemirror/state) | 6.7.6 | 2026-09-22 | sì — 6.7.6 | MIT | Come sopra. |
| [@codemirror/lang-sql](https://www.npmjs.com/package/@codemirror/lang-sql) | 6.10.0 | 2025-09-16 | sì — 6.10.0 | MIT | Stabile, rilasci rari. |
| [@codemirror/lang-json](https://www.npmjs.com/package/@codemirror/lang-json) | 6.0.2 | 2025-06-19 | sì — 6.0.2 | MIT | Stabile, rilasci rari. |
| [codemirror](https://www.npmjs.com/package/codemirror) (basic setup) | 6.0.2 | 2025-06-19 | sì — 6.0.2 | MIT | La 6.65.7 del 2022 è una pubblicazione spuria, da ignorare. La 5.x è sul tag `version5`. |
| [@vanilla-extract/css](https://www.npmjs.com/package/@vanilla-extract/css) | 1.21.3 | 2026-10-08 | sì — 1.21.3 | MIT | — |
| [@vanilla-extract/vite-plugin](https://www.npmjs.com/package/@vanilla-extract/vite-plugin) | 5.2.6 | 2026-07-27 | sì — 5.2.6 | MIT | Supporto di Vite 8: non verificato. |
| [@tanstack/react-query](https://www.npmjs.com/package/@tanstack/react-query) | 5.108.2 | 2026-11-20 | no → 5.107.1 (2026-11-06, al limite) | MIT | Rilasci molto frequenti. |
| [react-router](https://www.npmjs.com/package/react-router) | 8.5.2 | 2026-11-18 | no → 8.5.1 (2026-10-27) | MIT | v8; il ramo 7.x è ancora mantenuto (7.18.6 il 2026-11-18). |
| [vite](https://www.npmjs.com/package/vite) | 8.3.8 | 2026-11-11 | no → 8.3.6 (2026-11-05) | MIT | 8.3.6 include le correzioni di 8.3.3 per tre advisory del 2026-10-21. |
| [vitest](https://www.npmjs.com/package/vitest) | 5.1.1 | 2026-11-18 | no → 5.1.0 (2026-11-02) | MIT | GHSA-82fw-gwwq-j7x9 è corretta dalla 5.0.0-rc.2. |
| [@testing-library/react](https://www.npmjs.com/package/@testing-library/react) | 16.3.3 | 2026-08-27 | sì — 16.3.3 | MIT | — |
| [@playwright/test](https://www.npmjs.com/package/@playwright/test) | 1.65.1 | 2026-11-13 | no → 1.65.0 (2026-11-05) | Apache-2.0 | — |
| [playwright-bdd](https://www.npmjs.com/package/playwright-bdd) | 9.2.1 | 2026-09-06 | sì — 9.2.1 | MIT | Push 2026-11-16. Compatibilità con Playwright 1.65: non verificata. |
| [storybook](https://www.npmjs.com/package/storybook) | 10.6.8 | 2026-11-20 | no → 10.6.6 (2026-11-05) | MIT | 10.6.6 include le correzioni di 10.6.2 per GHSA-f2ph-cmg5-wxc8 (critical) e GHSA-px85-f4q5-jh77 (high) del 2026-10-21/22. |
| [@biomejs/biome](https://www.npmjs.com/package/@biomejs/biome) | 2.6.8 | 2026-11-15 | no → 2.6.6 (2026-11-06, al limite) | MIT OR Apache-2.0 | — |
| [typescript](https://www.npmjs.com/package/typescript) | 7.0.2 | 2026-07-08 | sì — 7.0.2 | Apache-2.0 | La 7.x si distribuisce con binari nativi per piattaforma (`@typescript/typescript-<os>-<arch>`). Ultima 6.x: 6.0.3 (2026-04-16). Compatibilità dei tool (Vite, Storybook, vanilla-extract, ts-rs) con TS 7: non verificata. |

## Sicurezza: correzioni uscite dopo la soglia

La regola delle due settimane esclude queste correzioni; per ognuna serve una deroga esplicita o una mitigazione.

| Componente | Versione fissabile | Correzione dopo il 2026-11-06 | Advisory |
|---|---|---|---|
| wasmtime / wasmtime-wasi | 50.0.1 | 50.0.2 (11-16), 50.0.3 (11-19) | 9 advisory. 1 high: [GHSA-f2pq-268w-886c](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-f2pq-268w-886c) (escape dalla sandbox WASI filesystem, **solo Windows**). 3 medium: GHSA-43j2-gcp4-m22g (un guest Wasm può far abortire l'host tramite il GC heap), GHSA-5m8g-xhq4-4h93 (wasmtime-wasi, memoria host senza limite), GHSA-f8mm-ggr3-89wh (wasi-http). 5 low (abort o panic). |
| Valkey | 9.1.3 | 9.1.4 (11-18) | CVE-2026-130822 / [GHSA-3885-pq5f-788j](https://github.com/valkey-io/valkey/security/advisories/GHSA-3885-pq5f-788j) high (use-after-free con `maxmemory-clients`). |
| NATS server | v2.15.2 | v2.15.3 (11-11) | 6 advisory. 5 high: [GHSA-p849-62j6-rhc5](https://github.com/nats-io/nats-server/security/advisories/GHSA-p849-62j6-rhc5) (leafnode), GHSA-p29q-ccgc-hcg7 e GHSA-7mrx-vpq7-fhmp (authorization callout), GHSA-fwrx-9r9q-h7p4 (ack JetStream da leafnode), GHSA-4fpp-8grw-j976 (panic sul route pool). 1 medium: GHSA-rr37-7hx9-8xw5. |
| iroh | 1.4.1 | 1.4.2 (11-19); 1.5.0 per l'ultimo | 5 advisory corretti in 1.4.2, tra cui GHSA-rqw2-29g8-jwxw e GHSA-2gf7-pq49-93p2 (high, esaurimento memoria su iroh-relay); GHSA-9v2v-w4v3-3qqq (low, credenziali proxy nel log di debug) corretto solo in 1.5. |
| cloud-hypervisor | v54.0 | v54.1 (11-11) | GHSA-3r3x-6hgg-2m38 high (use-after-free in virtio-pmem). |
| jsonwebtoken | 11.1.0 | 12.0.0 (11-13, breaking) | Correzioni da changelog senza GHSA: type confusion su `sub`/`iss`/`aud`, rifiuto delle critical extension sconosciute. |

Perimetro della verifica, letta il 2026-11-20:

- **Advisory pubblicati sul repo** (`gh api repos/<repo>/security-advisories`, tutti quelli dopo il 2026-11-06):
  wasmtime, iroh, nats-server, valkey, cloud-hypervisor, firecracker, iggy (per questi ultimi due nessuno).
- **[Database advisory GitHub](https://github.com/advisories)** (ecosistemi `rust` e `npm`, pubblicati dal
  2026-09-01) per tutti gli altri crate e pacchetti delle tabelle. Non ha restituito correzioni uscite dopo la
  soglia; questo database però è in ritardo: gli advisory wasmtime del 2026-11-16/19 non ci sono ancora.
- jsonwebtoken: solo il CHANGELOG.
- Gli altri componenti con "no →" non hanno avuto un controllo a livello di repo: un advisory pubblicato solo sul
  loro repo può essere sfuggito.
