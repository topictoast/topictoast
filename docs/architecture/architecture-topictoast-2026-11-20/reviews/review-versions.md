# Revisione: versioni e fatti della spine

**Data:** 2026-11-21 · **Oggetto:** `ARCHITECTURE-SPINE.md` (aggiornata il 2026-11-21) · **Lente:** ogni decisione
è stata verificata sulla realtà (registri, sorgenti, specifiche) o è solo affermata?

**Regola applicata:** versione esatta pubblicata entro il **2026-11-07** (due settimane prima di oggi); eccezione per
le versioni che correggono un avviso pubblicato.

**Metodo:** API di crates.io (`/api/v1/crates/<nome>/versions`, User-Agent generico), registry npm (`time`,
`versions[<pin>].deprecated`, `license`, `peerDependencies`), release GitHub (`gh api repos/<repo>/releases`), avvisi
per repository (`gh api repos/<repo>/security-advisories`, senza troncare), OSV (`api.osv.dev/v1/querybatch` sulle
versioni fissate, provato prima su versioni note vulnerabili), docs.rs, tarball pubblicati (napi-build 2.6.0, @napi-rs/cli
3.10.12, emnapi), schema OWS al tag `v1.1.0`, blog di Iggy 0.10.0, HTML Living Standard (WHATWG), dati di compatibilità
MDN, rfc-editor.org, postgresql.org, Docker Hub, bundlephobia. Tutto letto il 2026-11-21.

**Limiti della verifica:**

- Per `apache/datafusion` e `apache/iggy` la lista degli avvisi GitHub vuota è una prova debole: la ASF annuncia i CVE
  anche su liste di posta. Scritto "nessun avviso trovato su <fonte>", non "nessun avviso".
- OSV è in ritardo sugli avvisi più recenti: non riporta GHSA-2rq8 e GHSA-qhgm di react-router, né quelli di iroh
  del 2026-11-19. Per le ultime due settimane fa fede la lista degli avvisi del repository.
- Dove una conclusione è un'inferenza, la riga lo dice.

## Esito in breve

Le 26 righe dello Stack reggono tutte. Ogni versione esiste, non è yanked né deprecata e ha una licenza compatibile con
Apache-2.0; per nessuna è stato trovato un avviso aperto. 25 righe su 26 rispettano già la regola delle due settimane:
la ventiseiesima è OWS 1.1.0, fissabile dal 2026-11-26, e la spine lo dice già. Le correzioni sono **10** e quasi
tutte cadono fuori dalla tabella:

- Postgres, che manca e ha un CVE sulla RLS (1);
- il perimetro dell'eccezione di sicurezza (2);
- S-4: un'impostazione non rilasciata e la versione di emnapi (3, 4);
- il valore di `alg` del JWT e gli avvisi della libreria JWT (5, 6);
- una domanda aperta che si può chiudere (7);
- due formulazioni (8, 10);
- i candidati degli spike senza versione (9).

## 1. Righe dello Stack

| Elemento | Affermato nella spine | Verificato (fonte e data) | Esito | Correzione |
| --- | --- | --- | --- | --- |
| Rust (toolchain stabile) | 1.99.1 | GitHub `rust-lang/rust` releases, 2026-11-21: 1.99.1 del 2026-10-16; 1.100.0 del 2026-11-12, non fissabile. MSRV dichiarate su crates.io: datafusion 55.2.0 → 1.94.0, iggy 0.12.0 → 1.95, napi, cucumber e testcontainers → 1.88, rustls → 1.71. Tutte sotto 1.99. | ok | — |
| Open Workflow Specification | 1.1.0, del 2026-11-12, fissabile dal 2026-11-26 | GitHub `open-workflow-specification/specification` releases, 2026-11-21: v1.1.0 del 2026-11-12T20:51Z, non pre-release; nessuna 1.2.0. Licenza del repository: Apache-2.0. | ok | Non rispetta ancora la regola, ma la spine lo dice già e la data cade prima dello sprint 1. |
| datafusion | 55.2.0 | crates.io, 2026-11-21: 55.2.0 del 2026-10-07, Apache-2.0, non yanked. 55.3.0 (2026-11-17) contiene bugfix e "chore: fix cargo audit" (#27396). Quella PR cambia solo il `Cargo.lock` del repository, e lo fa per avvisi su dipendenze transitive: zstd 0.13.3 (RUSTSEC-2026-0391), rustls-webpki 0.103.15 (RUSTSEC-2026-0382), quinn-proto, ctutils. Nessun avviso trovato su `apache/datafusion` né su OSV. | ok | Vedi la riga "Eccezione di sicurezza" nella sezione 2. |
| napi | 3.16.1 | crates.io: 2026-10-31, MIT. Avvisi del repository `napi-rs/napi-rs` (5 high, agosto-settembre 2026): tutti corretti entro la 3.12.4. 3.16.2 (2026-11-11) non fissabile. | ok | — |
| napi-derive | 3.7.0 | crates.io: 2026-10-30, MIT; dipende da `napi-derive-backend ^7`. | ok | — |
| napi-build | 2.6.0 | crates.io: 2026-10-01, MIT. Nel sorgente del crate pubblicato (`src/wasi.rs`), per `wasm32-wasip1` senza thread fa il link di `emnapi-basic-napi-rs` e fissa `-zstack-size=64000000`, fuori dal ramo `has_threads`. | ok | Vedi S-4 nella sezione 3 (`stackSizeBytes`). |
| @napi-rs/cli | 3.10.12 | npm: 2026-11-03, MIT, `latest`. Nel `dist` pubblicato ci sono `threadlessInitialMemory`, `asyncInit` e `wasm32-wasip1`; **manca `stackSizeBytes`**. | ok | Vedi S-4 nella sezione 3. |
| @napi-rs/wasm-runtime | 1.2.7 | npm: 2026-11-03, MIT. Peer: `@emnapi/core` e `@emnapi/runtime` in `^1.7.1 \|\| 2.0.0-beta.2 \|\| 2.0.0-beta.3`. | ok | — |
| rustls | 0.23.46 | crates.io: 2026-10-14, licenza `Apache-2.0 OR ISC OR MIT`. Avvisi del repository `rustls/rustls` del 2026-10-14: GHSA-c8m5-c952-6cm4, GHSA-w44r-93j7-qv28 e GHSA-h2v8-394m-p7vv sono corretti da ≥ 0.23.46; GHSA-vp88-798v-h67f riguarda solo 0.23.44 e 0.23.45. OSV su 0.23.46: nessun avviso (su 0.23.45: 8). Dipende da `rustls-webpki ^0.103.14`, e RUSTSEC-2026-0382 (2026-10-20) è corretto solo in 0.103.16. | ok | Il pin di rustls non basta: il `Cargo.lock` deve risolvere `rustls-webpki` ≥ 0.103.16 (sezione 2). |
| cucumber | 0.23.0 | crates.io: 2026-04-23, `MIT OR Apache-2.0`. Repository attivo (push 2026-07-27). Nessun avviso su repository e OSV. | ok | — |
| testcontainers | 0.28.0 | crates.io: 2026-08-06, `MIT OR Apache-2.0`. Push 2026-11-21. Nessun avviso. | ok | — |
| wiremock | 0.6.5, manutenzione ferma dal 2025-08 | crates.io: 2025-08-24, `MIT/Apache-2.0` (sintassi SPDX vecchia, vale `MIT OR Apache-2.0`). Ultimo commit 2025-08-24, repository non archiviato. Nessun avviso. | ok | — |
| Apache Iggy server | server-0.10.0, candidato | GitHub `apache/iggy` releases: server-0.10.0 del 2026-11-07T09:32Z, fissabile da oggi. Immagine Docker `apache/iggy:0.10.0` del 2026-11-07. Apache-2.0. Nessun avviso trovato sul repository. | ok | — |
| iggy (crate client) | 0.12.0, candidato | crates.io: 2026-11-07T08:52Z, Apache-2.0, MSRV 1.95. Il blog della 0.10.0 lo indica come SDK Rust del server 0.10.0. OSV: nessun avviso. | ok | — |
| RustFS | 1.0.4, Apache-2.0 | GitHub `rustfs/rustfs` releases: 1.0.4 del 2026-10-25. `LICENSE` Apache-2.0. Immagine Docker `rustfs/rustfs:1.0.4` presente. Il repository ha **43 avvisi in un anno** (dal 2025-12-30), il più recente corretto in 1.0.2: nessuno tocca 1.0.4. Le note della 1.0.5 (2026-11-10) contengono "fix(auth): restrict root-derived accounts and credential logs" e "authorize warehouse location claims", senza avviso. | ok | Nota per l'ADR: storia di avvisi molto fitta. Riverificare la 1.0.5 quando diventa fissabile (2026-11-24). |
| react, react-dom | 19.3.1 | npm: 2026-10-14, MIT. È la versione che corregge GHSA-xqc2-xf4c-x3h3 (critical, RCE nei React Server Components) e GHSA-h3gv-c5rc-p453 (avvisi di `facebook/react`). | ok | — |
| @xyflow/react | 12.12.1, candidato | npm: 2026-11-02, MIT. La major 13.0.0 è del 2026-11-16, non fissabile. Peer `react >=17`. | ok | — |
| react-router | 8.5.1 | npm: 2026-10-27, MIT. Lista completa degli avvisi di `remix-run/react-router`: 8.5.1 è la versione corretta per GHSA-2rq8-3rm6-2w8f e GHSA-qhgm-jj2j-75wj (high, 2026-11-18). Nessun avviso del ramo 8.x ha una correzione sopra 8.5.1, quindi 8.5.2 non è una correzione di sicurezza. | ok | Se mai si usano `@react-router/node`, `serve` o `dev`, servono almeno 8.3.1 (GHSA-mp5r-736v-j8x9). |
| @tanstack/react-query | 5.107.1 | npm: 2026-11-06T23:04Z, MIT. | ok | — |
| @vanilla-extract/css | 1.21.3 | npm: 2026-10-08, MIT. | ok | — |
| storybook | 10.6.6 | npm: 2026-11-05, MIT. Gli avvisi GHSA-f2ph-cmg5-wxc8 (critical) e GHSA-px85-f4q5-jh77 sono corretti da 10.6.2. | ok | — |
| vitest | 5.1.0 | npm: 2026-11-02, MIT. GHSA-p458-7748-mcp3 riguarda le versioni < 5.0.5; GHSA-82fw-gwwq-j7x9 è corretto da 5.0.0-rc.2. Peer `vite ^6.4 \|\| ^7 \|\| ^8`. | ok | — |
| @testing-library/react | 16.3.3 | npm: 2026-08-27, MIT. Peer React `^18 \|\| ^19`. | ok | — |
| @playwright/test | 1.65.0 | npm: 2026-11-05, Apache-2.0. Nessun avviso su `microsoft/playwright`. | ok | — |
| playwright-bdd | 9.2.1 | npm: 2026-09-06, MIT. Peer `@playwright/test >=1.44`: la compatibilità con 1.65, "non verificata" in `research-versions.md`, è ora dichiarata. | ok | — |
| @biomejs/biome | 2.6.6 | npm: 2026-11-06, `MIT OR Apache-2.0`. | ok | — |

Per ogni pin npm è stato letto `versions[<pin>].deprecated`: nessuno è deprecato. Su crates.io nessun pin è yanked.

**Versione fissabile più nuova:** nessuna. Per ogni riga la versione successiva al pin è uscita dopo il 2026-11-07:

- datafusion 55.3.0 (11-17), napi 3.16.2 e napi-derive 3.7.1 (11-11);
- Rust 1.100.0 (11-12), RustFS 1.0.5 (11-10);
- react-router 8.5.2 (11-18), @tanstack/react-query 5.107.2 (11-17), storybook 10.6.7 (11-10), vitest 5.1.1 (11-18);
- @playwright/test 1.65.1 (11-13), @biomejs/biome 2.6.7 (11-11), @xyflow/react 13.0.0 (11-16).

Le altre righe fissano già l'ultima versione stabile.

## 2. Elementi su cui la spine si appoggia ma senza versione

| Elemento | Affermato nella spine | Verificato (fonte e data) | Esito | Correzione |
| --- | --- | --- | --- | --- |
| Postgres | AD-1, AD-5: Postgres in alternativa a SQLite, RLS con utente non superuser e senza `BYPASSRLS`; nessuna versione nello Stack | postgresql.org `versions.json` e pagina sicurezza, 2026-11-21. 18.7, 17.12, 16.16 e 15.20 sono del 2026-11-12; 19.0 del 2026-11-19. Una di quelle release corregge **CVE-2026-120154: "INSERT ... ON CONFLICT can read rows despite row security policies"**. Tocca la RLS di AD-5 e un proiettore idempotente, che con buona probabilità scrive con upsert. Corregge anche CVE-2026-120153 (7.2, extension script). | da correggere | Postgres è dell'integratore, non lo distribuisce topictoast. Scrivere quindi, per ogni major supportata, la minor minima corretta (18.7, 17.12, 16.16, 15.20), più un pin dell'immagine dei test (testcontainers), con l'eccezione di sicurezza. Quali major supportare lo decide Marco. Il test a due tenant di AD-5 deve coprire anche `ON CONFLICT`. |
| Eccezione di sicurezza | "una versione che corregge una vulnerabilità pubblicata (RustSec o GHSA) si fissa subito" | Il CVE di Postgres sopra non ha né RustSec né GHSA. DataFusion #27396 mostra avvisi su dipendenze transitive (`rustls-webpki`, `zstd`) che nessun pin diretto copre. Senza `cargo update --precise`, Cargo risolve `rustls-webpki` a 0.103.17 (2026-11-12), che non rispetta la regola. | da correggere | Allargare l'eccezione: "avviso pubblicato (RustSec, GHSA o CVE del progetto)". Dire che la regola e l'eccezione valgono anche per le dipendenze transitive in `Cargo.lock` e `package-lock.json`. Oggi servono `rustls-webpki` ≥ 0.103.16 (2026-10-20) e `zstd` ≥ 0.13.4. |
| emnapi | S-4: "dipende da emnapi 2.0.0-beta (pre-release: versione esatta nell'ADR)" | Tarball su unpkg. `emnapi@2.0.0-beta.3` contiene `lib/wasm32-wasip1/libemnapi-basic-napi-rs.a`, l'archivio che serve a napi-build 2.6.0 senza thread; `1.11.3` (stabile) no. beta.2 non è stata controllata. L'esempio upstream `examples/custom-async-runtime` fissa `@emnapi/core` e `@emnapi/runtime` a 2.0.0-beta.3. Pubblicata il 2026-11-02, MIT, fissabile; nessun avviso su OSV. | da correggere | Precisazione: la dipendenza è confermata. La versione si può scrivere già ora: `emnapi`, `@emnapi/core` e `@emnapi/runtime` **2.0.0-beta.3**. Il peer di wasm-runtime 1.2.7 nomina proprio beta.2 e beta.3: una beta più nuova rompe il peer. |
| Libreria JWT (rinviata) | C-15 si appoggia a una libreria JWT Rust con EdDSA | Avvisi di `Keats/jsonwebtoken`, pubblicati il 2026-11-13. GHSA-698m-q5wh-jf73 (high, chiave HMAC vuota) riguarda ≤ 11.1.0. GHSA-4gj5-m54m-9gq6 (medium, `aud`/`iss`/`sub` di tipo sbagliato saltano la validazione) riguarda ≤ 11.0.0. Nessuno dei due indica la versione corretta. Il CHANGELOG al tag v12.0.0 (2026-11-13) contiene "BREAKING: `EncodingKey` and `DecodingKey` reject empty HMAC keys" e "Fix remaining type confusion with sub, iss, aud". | da correggere | In `research-versions.md`, non nella spine. La nota "correzioni da changelog senza GHSA" e il pin 11.1.0 sono superati: 11.1.0 ricade in GHSA-698m (l'avviso su `aud`, `iss` e `sub` si ferma a 11.0.0). Con l'eccezione, la scelta va su 12.0.0, che il CHANGELOG conferma come correzione. |
| Candidati dello spike del bus (S-1) | Valkey, gRPC, iroh: nessuna versione | Valkey: avviso GHSA-3885-pq5f-788j / CVE-2026-130822 (high), corretto in 9.1.4 (2026-11-18). iroh: 5 avvisi del 2026-11-19 (2 high) corretti in 1.4.2; GHSA-9v2v-w4v3-3qqq (low) solo in 1.5. tonic 0.14.9 del 2026-10-29, MIT, nessun avviso su OSV. `redis` (client per Valkey) 1.8.1 del 2026-10-30, BSD-3-Clause. | da correggere | Iggy è nello Stack come candidato, Valkey no: aggiungere i candidati con i pin. Con l'eccezione: Valkey **9.1.4** e iroh **1.4.2**. Fissabili: tonic **0.14.9** e redis **1.8.1**. |
| Candidati dello spike della sandbox (S-2) | wasmtime e Firecracker come punti di partenza, cloud-hypervisor citato | wasmtime: OSV su 50.0.1 restituisce RUSTSEC-2026-0566…0573; 50.0.3 (2026-11-19) è senza avvisi. Firecracker: v1.17.0 del 2026-09-10 (GHSA-48ww-rqpp-2mgm corretto in 1.17.0); v1.18.0 del 2026-11-09. cloud-hypervisor: GHSA-3r3x-6hgg-2m38 (high) corretto in v54.1 (2026-11-11). | da correggere | Pin nell'ADR di S-2. Con l'eccezione: wasmtime **50.0.3** e cloud-hypervisor **v54.1**. Fissabile: Firecracker **v1.17.0**. |

## 3. Fatti su cui poggiano le AD

| Elemento | Affermato nella spine | Verificato (fonte e data) | Esito | Correzione |
| --- | --- | --- | --- | --- |
| Schema OWS 1.1 chiuso (AD-8) | Un documento con `dag` è rifiutato dai validatori ufficiali; senza `dag` resta OWS puro | `schema/workflow.yaml` al tag v1.1.0, 2026-11-21. `$defs.task` ha `unevaluatedProperties: false` e un `oneOf` sui 12 tipi (righe 231-247). `taskBase.metadata` ha `additionalProperties: true`. La radice non ha `additionalProperties`. Un task `dag` non passa nessun ramo del `oneOf`. | ok | — |
| Linguaggio degli schemi (AD-10) | "È lo stesso linguaggio di `input.schema` di OWS" | Nello stesso schema, `$defs.schema` ha `format` (predefinito `json`, qualsiasi `{format}:{version}`) e `document` libero oppure `resource` esterna. OWS accetta quindi JSON Schema completo, altri formati e schemi esterni. | da correggere | Formulazione: "È un sottoinsieme chiuso di `input.schema` di OWS con `format: json`, senza `resource` esterne." |
| Bersaglio napi-rs `wasm32-wasip1` senza thread (AD-11, S-4) | Supportato da @napi-rs/cli 3.8.0, nessun COOP/COEP, upstream prova solo Chromium | PR napi-rs #3353 "support non-threaded WASI targets", mergiata il 2026-07-21. Changelog della CLI: 3.8.0 del 2026-07-28. `examples/custom-async-runtime/vite.config.mjs` dice "deliberately sets NO COOP/COEP headers" e prova solo `chromium`. | ok | — |
| Impostazioni da provare in S-4 | `threadlessInitialMemory`, `stackSizeBytes`, `asyncInit: true` | `threadlessInitialMemory` (#3516, CLI 3.10.0) e `asyncInit` ci sono nel dist di 3.10.12. **`stackSizeBytes` non c'è** né in @napi-rs/cli 3.10.12 né in napi-build 2.6.0: lo introduce la PR #3727 ("make WASI `-zstack-size` configurable"), mergiata il **2026-11-21T05:45Z** e non ancora rilasciata. In napi-build 2.6.0 lo stack wasm è fisso a `-zstack-size=64000000` (circa 61 MiB), anche senza thread. | da correggere | Togliere `stackSizeBytes` dalle impostazioni, o segnarlo "dopo il primo rilascio che contiene #3727, più due settimane". S-4 deve misurare la memoria iniziale del browser con lo stack fisso di 64.000.000 byte. |
| Peso della colla JS (S-4) | Stima di 60-90 KB gzip senza jq, più il `.wasm` | bundlephobia, 2026-11-21: @napi-rs/wasm-runtime 1.2.7 20,4 KB, @emnapi/core 2.0.0-beta.3 25,9 KB, @emnapi/runtime 11,8 KB, @tybys/wasm-util 11,9 KB. In totale circa 58-70 KB gzip, prima del tree-shaking. | ok | Stima coerente: la misura resta allo spike. |
| `'wasm-unsafe-eval'` (AD-11, AD-19) | La CSP dell'ospite lo deve permettere | Dati di compatibilità MDN (`script-src` → `wasm-unsafe-eval`): Chrome 97, Firefox 102, Safari 16; Chrome Android e Safari iOS rispecchiano il desktop. | ok | — |
| EventSource senza header (AD-19) | Nel nucleo JS l'SSE passa da `fetch` con `Authorization` | WHATWG HTML, IDL: `dictionary EventSourceInit { boolean withCredentials = false; }`. Nessun modo di passare header. | ok | Nota: con `fetch` il nucleo perde la riconnessione automatica e l'invio automatico di `Last-Event-ID` di EventSource, e deve rifarli (C-8). |
| DataFusion senza finestre, watermark e checkpoint per gli stream (AD-13) | Stato, finestre, watermark e checkpoint li costruisce topictoast | Issue di `apache/datafusion`. #11366 (StreamingWindowExec) è chiusa il 2026-08-01 con il rimando a `BoundedWindowAggExec`, che lavora su input ordinato o in tempo di elaborazione, senza watermark. #24603 (rimozione dei gruppi per TTL o watermark) è chiusa come *not planned* il 2026-11-17. L'epica #24265 (aperta) dice che le release recenti peggiorano lo streaming, per esempio con il buffering in `RepartitionExec` e `FilterExec`. Primitive presenti: `Boundedness`, `EmissionType`, `SymmetricHashJoinExec`. Nessun checkpoint. | ok | Nota per S-3: misurare la latenza aggiunta dal buffering degli operatori della 55.x. |
| Cifratura modulare dei Parquet in DataFusion (Open Questions, AD-17) | "Non verificato" | crates.io: datafusion 55.2.0 ha la feature `parquet_encryption`. docs.rs 55.2.0: esistono `datafusion::config::ParquetEncryptionOptions` e `datafusion::execution::parquet_encryption::EncryptionFactory` (HTTP 200). | da correggere | Aggiornare la domanda: l'API c'è (verificata solo a livello di feature e di tipi). Resta da provare in S-3 una chiave per tenant tramite `EncryptionFactory`. |
| Iggy 0.10 non legge i dati della 0.9 (S-1) | Sì | Blog "Apache Iggy 0.10.0 Release" (9 novembre 2026), sezione "Breaking changes". Il server rifiuta tutti i client più vecchi al login. "no migration for 0.9.x data directories … until 1.0.0 an upgrade starts from a fresh data directory". | ok | Nota: fino alla 1.0 ogni aggiornamento del bus riparte da vuoto, quindi FR-51 senza perdite chiede uno svuotamento coordinato. Il criterio è già in S-1. |
| Credenziali del bus senza valori predefiniti (S-1) | Credenziali generate all'installazione, nessun valore predefinito del candidato accettato | Stesso blog: "A fresh server no longer generates a root password". Servono `IGGY_ROOT_USERNAME` e `IGGY_ROOT_PASSWORD`, oppure `--with-default-root-credentials`. | ok | Nota: le credenziali le deve generare topictoast. `--with-default-root-credentials` va vietato. |
| RustFS 1.0.x Apache-2.0 (AD-17) | Sì | Vedi la sezione 1. | ok | — |
| JWT EdDSA, Ed25519 (C-15) | "JWT EdDSA (Ed25519)" | RFC 9864 (ottobre 2025, aggiorna RFC 8037), §2.2 e §4.1.2: registra `alg: "Ed25519"` e porta `"EdDSA"` a **Deprecated** nel registro IANA di JOSE. `jose` (JS, 6.2.18) accetta sia `EdDSA` sia `Ed25519`. `jsonwebtoken` (Rust, 11.1.0 e 12.0.0, `src/algorithms.rs`) conosce solo `EdDSA`. | da correggere | Scrivere in C-15 i valori di `alg` ammessi: per esempio `Ed25519` (RFC 9864) e `EdDSA` solo con chiave OKP `crv: Ed25519`, tutto il resto rifiutato. La libreria JWT Rust scelta deve gestire quei valori. |
| RFC 9457 (C-7) e UUIDv7 (C-1) | problem+json, UUIDv7 | rfc-editor.org: RFC 9457 (luglio 2023, sostituisce la 7807); RFC 9562 (maggio 2024, sostituisce la 4122, definisce UUIDv7). | ok | — |
| SDK Rust ufficiale di OWS da non usare | Alpha, DSL 1.0, senza validazione | crates.io: `serverless_workflow_core` e `_builders` fermi a 1.0.0-alpha6.3 (2025-01-27), senza versioni stabili. Push del repository `sdk-rust` il 2025-12-12. | ok | — |
| jtd, sled, fred, denormalized fermi | Sì | crates.io: jtd 0.3.1 (2021-01-31); sled 0.34.7, con la 1.0.0-alpha.124 del 2024-10-11; fred 10.1.0 (2025-02-27). denormalized: push del 2024-12-18. | ok | — |
| elkjs | "licenza non compatibile con il kit Apache-2.0" | npm: elkjs 0.12.1, `EPL-2.0 OR GPL-3.0-or-later`. La [politica ASF](https://www.apache.org/legal/resolved.html) mette EPL 2.0 in Categoria B: incluso solo in forma binaria e con etichetta. Copyleft debole, quindi non "non compatibile" in senso stretto. | da correggere | Formulazione: "Copyleft (EPL-2.0 debole o GPL-3.0): esclusa per scelta, per gli obblighi su un kit incorporabile." |
| Interprete jq compilabile per `wasm32-wasip1` (rinvio M-29) | Vincolo | Upstream jaq prova solo `wasm-pack --target web` (jaq-play), non `wasm32-wasip1`. | non verificabile | Resta un vincolo da provare nello spike S-4 (peso "con jq compreso"). |
| Bersaglio senza thread su Firefox, Safari e telefoni (S-4) | Da provare | Upstream prova solo Chromium (sopra). | non verificabile | Già aperto in S-4. |

## Correzioni, in ordine di priorità

1. **Postgres nello Stack**: per ogni major supportata, la minor minima corretta (18.7, 17.12, 16.16, 15.20), più un
   pin dell'immagine dei test. Correggono CVE-2026-120154, un `ON CONFLICT` che legge righe nonostante la RLS di AD-5.
   Le major le sceglie Marco.
2. **Eccezione di sicurezza**: estenderla ai CVE dei progetti e alle dipendenze transitive dei lockfile. Oggi servono `rustls-webpki` ≥ 0.103.16 e `zstd` ≥ 0.13.4.
3. **S-4**: togliere `stackSizeBytes`, che non è rilasciato (PR #3727 del 2026-11-21). Lo stack wasm resta fisso a 64.000.000 byte.
4. **S-4**: scrivere già la versione di emnapi, 2.0.0-beta.3.
5. **C-15**: fissare i valori di `alg`. RFC 9864 porta `EdDSA` a deprecato e registra `Ed25519`.
6. **`research-versions.md`, jsonwebtoken**: 11.1.0 ricade in GHSA-698m-q5wh-jf73 (high, 2026-11-13). Con
   l'eccezione si va su 12.0.0, che il CHANGELOG conferma come correzione.
7. **Open Questions, cifratura dei Parquet**: la feature `parquet_encryption` di DataFusion esiste; manca solo la prova per tenant.
8. **AD-10**: scrivere "sottoinsieme" al posto di "stesso linguaggio" di `input.schema`.
9. **Candidati di S-1 e S-2 nello Stack o negli ADR**:
   - con l'eccezione: Valkey 9.1.4, iroh 1.4.2, wasmtime 50.0.3, cloud-hypervisor v54.1;
   - fissabili: tonic 0.14.9, redis 1.8.1, Firecracker v1.17.0.
10. **elkjs**: scrivere "copyleft, esclusa per scelta" al posto di "non compatibile".
