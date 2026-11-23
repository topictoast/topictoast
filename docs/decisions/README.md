# Decisioni architetturali (ADR)

Un file per decisione: `YYYY-MM-DD-<slug>.md`, in italiano. Intestazione con `Data`, `Stato`
(`proposta` → `accettata` → `sostituita da <file>`) e, se serve, `Sostituisce`. Un ADR accettato
non si riscrive: una decisione nuova è un ADR nuovo, e nel vecchio cambia solo lo `Stato`. Le
regole AD-n stanno nella
[spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md); gli ADR ne
registrano bilancio e alternative.

Gli ADR del 2026-11-21 sono `accettata`: Marco ha approvato in chat la spine e il secondo blocco
E-1…E-9 il 2026-11-21 (memlog, voci 62 e 63), e la PR della spine li porta su `main`.

| File | Decisione | AD | Stato |
| --- | --- | --- | --- |
| [2026-11-21-runtime-units.md](2026-11-21-runtime-units.md) | Unità di runtime della v1: motore a nodo singolo, bus separato, SQLite o Postgres | AD-1 | accettata |
| [2026-11-21-bus-source-of-truth.md](2026-11-21-bus-source-of-truth.md) | Verità sul log del bus, database come proiezione, payload fuori dal database, tempo reale dal motore | AD-2, AD-3, AD-18 | accettata |
| [2026-11-21-typed-data-and-tenant-isolation.md](2026-11-21-typed-data-and-tenant-isolation.md) | Costituzione V alla lettera; isolamento tra tenant nell'unico adattatore del repository; PATCH 1.0.1 | AD-4, AD-5 | accettata |
| [2026-11-21-tenant-content-keys-and-deletion.md](2026-11-21-tenant-content-keys-and-deletion.md) | Chiave nel motore, contenuto dei tenant negli ausiliari; cassaforte a busta ed eliminazione crittografica | AD-6, AD-7 | accettata |
| [2026-11-21-dag-task-and-propagation.md](2026-11-21-dag-task-and-propagation.md) | Task `dag` di prima classe, `switch` come `do` annidato, regola di propagazione | AD-8, AD-9 | accettata |
| [2026-11-21-schema-language.md](2026-11-21-schema-language.md) | Linguaggio degli schemi: sottoinsieme chiuso di JSON Schema 2020-12 | AD-10 | accettata |
| [2026-11-21-single-validator-napi-rs.md](2026-11-21-single-validator-napi-rs.md) | Un solo validatore Rust, esposto a JS con napi-rs (browser: `wasm32-wasip1` senza thread) | AD-11 | accettata |
| [2026-11-21-public-api-and-kit-embedding.md](2026-11-21-public-api-and-kit-embedding.md) | API pubblica HTTP con JSON, OpenAPI e SSE; kit nelle pagine dell'ospite | AD-12, AD-19 | accettata |
| [2026-11-21-stream-pipelines-datafusion.md](2026-11-21-stream-pipelines-datafusion.md) | Pipeline stream su DataFusion, un processo ausiliario per tenant | AD-13 | accettata |
| [2026-11-21-payload-store-and-history.md](2026-11-21-payload-store-and-history.md) | Archivio dei payload su disco o RustFS facoltativo, storico in Parquet | AD-17 | accettata |
| [2026-11-21-bdd-boundary.md](2026-11-21-bdd-boundary.md) | Confine degli scenari BDD su due livelli, entrambi con cucumber-rs | AD-14 | accettata |
| [2026-11-21-repo-layout-and-dependencies.md](2026-11-21-repo-layout-and-dependencies.md) | Layout del repository e direzione delle dipendenze | AD-15, AD-16 | accettata |
| [2026-11-23-sql-driver.md](2026-11-23-sql-driver.md) | Driver SQL: sqlx per SQLite e Postgres, 0.9.1 con l'eccezione di sicurezza | AD-5 (adattatore unico), Stack | accettata |
| [2026-11-23-configuration-format.md](2026-11-23-configuration-format.md) | Formato della configurazione: TOML facoltativo e variabili d'ambiente, nessuna ricarica a caldo | C-19 | accettata |
| [2026-11-23-dependency-licenses.md](2026-11-23-dependency-licenses.md) | Licenze ammesse per le dipendenze: permissive e MPL-2.0 non modificata | Stack | accettata |
| [2026-11-23-task-logs.md](2026-11-23-task-logs.md) | Log dei task: pezzi per tentativo nell'archivio dei payload, segnaposto dei segreti all'origine e nel motore | AD-17 (esteso) | accettata |

Gli spike S-1…S-4 chiudono ciascuno con un ADR misurato (`bus`, `sandbox`, `stream-pipelines`,
`kit-browser`) che aggiorna gli AD indicati nella spine.
