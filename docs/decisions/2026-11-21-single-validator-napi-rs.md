# ADR: un solo validatore Rust, esposto a JS con napi-rs

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-11 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-4, FR-17, FR-19, FR-39, FR-40, FR-44; `agenda.md` (C.3); `research-versions.md`;
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 25, 26, 29, 32, 33,
  34, 48, 50, 51, 52, 56, 59; `review-decisions.md` (E-9); revisioni `reconcile-ux.md` (R-6),
  `review-security.md` (SEC-M13)

## Contesto

Il nucleo JS del kit valida con gli stessi esiti del motore (FR-39), e la validazione avviene
all'invio (FR-4). Il validatore copre documento, `dag`, schemi (AD-10), jq e cron, e deve girare
nel browser dentro pagine altrui. Le strade in agenda erano due: una sola implementazione Rust
compilata in Wasm, oppure due implementazioni con una suite comune (`agenda.md`, C.3).

## Decisione

Registra [AD-11](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md): la
validazione ha una sola implementazione, in Rust nel dominio, e arriva a JS solo tramite napi-rs.
Per Node il collegamento è nativo; per il browser il bersaglio è `wasm32-wasip1` senza thread.

La scelta si è chiusa in tre passi:

1. **Voce 25.** Marco sceglie la parità con una sola implementazione Rust esposta con napi-rs. Il
   bersaglio nel browser resta aperto: secondo la documentazione di napi.rs letta quel giorno,
   napi-rs genera per il browser solo `wasm32-wasip1-threads`, che richiede `SharedArrayBuffer` e
   quindi COOP/COEP sulla pagina ospite.
2. **Voce 29.** Marco vuole solo napi-rs, anche nel browser, e chiede di verificarne i limiti:
   target senza thread, isolamento cross-origin, peso del runtime, Safari, CSP.
3. **Voce 33.** Verificato il bersaglio `wasm32-wasip1` senza thread (`@napi-rs/cli` dalla 3.8.0,
   changelog del 2026-07-28, #3353): niente Web Worker, niente `SharedArrayBuffer`, niente
   COOP/COEP. Upstream lo prova con `examples/custom-async-runtime`. Il bersaglio a thread è
   escluso, e niente `wasm-bindgen`.

Resta **da confermare nello spike del kit (S-4)**.

La regola fissa anche il perimetro:

- il validatore del browser copre il documento del workflow: formato, `dag`, schemi (AD-10),
  espressioni jq con la stessa implementazione Rust di jq, cron. Rifiuta anche `$secrets` e
  `$authorization` (AD-8), ed espone la funzione del dominio che riconosce il gruppo `switch`;
- il SQL delle pipeline stream lo valida solo un processo ausiliario del motore, tramite una
  validazione a secco dell'API;
- il contesto della validazione (manifest, segreti per connessione, limiti, stream disponibili) è
  un tipo del dominio servito da una risorsa dell'API limitata al tenant della credenziale;
- la pagina ospite deve permettere `'wasm-unsafe-eval'`, il fetch del `.wasm` e `connect-src`
  verso il motore; JS e `.wasm` del kit arrivano dal pacchetto installato dall'ospite, serviti
  dalla sua origine, mai scaricati dal motore (AD-19).

Versioni nello Stack della spine: emnapi, `@emnapi/core` e `@emnapi/runtime` 2.0.0-beta.3
(pre-release, dipendenza della build wasm di napi-rs). `@napi-rs/cli` 3.10.12 ha
`threadlessInitialMemory` e `asyncInit`, ma non `stackSizeBytes` (memlog, voce 52); lo stack wasm
è fisso a 64 MB in napi-build 2.6.0 (S-4 della spine).

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:** gli stessi esiti in editor e motore per costruzione (FR-39). Funziona in
  pagine senza isolamento cross-origin, se S-4 lo conferma. Un solo strumento di collegamento per
  Node e browser.
- **Costi:**
  - peso stimato di circa 60-90 KB gzip di colla JS più il `.wasm`, jq escluso: è una stima, la
    misura la fa S-4;
  - dipendenza da emnapi, `@emnapi/core` e `@emnapi/runtime` 2.0.0-beta.3, una pre-release che
    finisce nelle pagine di ogni ospite;
  - impostazioni da provare: `threadlessInitialMemory`, `asyncInit: true`, bundle ESM
    preconfezionato;
  - lo stack wasm fisso a 64 MB non si regola dalla CLI: la memoria iniziale nel browser la misura
    S-4;
  - la pagina ospite deve permettere `'wasm-unsafe-eval'` e il fetch del `.wasm`;
  - upstream prova solo Chromium;
  - la documentazione di napi.rs è indietro rispetto alla CLI.
- **Facilità di manutenzione:** un solo validatore. Il dominio deve restare compilabile per
  `wasm32-wasip1`, e il gate lo verifica (CLAUDE.md di questa PR). La validazione del SQL è fatta
  da un processo ausiliario del motore (AD-6: il motore non interpreta mai contenuto dei tenant).

## Alternative scartate

- **Due implementazioni, Rust e TS, con una suite comune** (`agenda.md`, C.3). Motivi (memlog,
  voce 50):
  - ogni regola scritta due volte;
  - una deriva si vede solo se la suite copre il caso;
  - jq con un'altra libreria, ed errori diversi.

  Marco ha scelto una sola implementazione (voce 25), con la parità per costruzione (voce 32).
- **Bersaglio napi-rs a thread, `wasm32-wasip1-threads`** (memlog, voce 33): richiede header
  COOP/COEP sulla pagina ospite e un worker same-origin, e Safari non ha `credentialless`.
- **`wasm-bindgen` per il browser** (memlog, voci 29 e 33): due toolchain di collegamento, e Marco
  vuole solo napi-rs (memlog, voce 50). La scelta è diventata definitiva solo
  dopo la verifica del bersaglio senza thread. `wasm-bindgen` è mantenuto (0.2.130 del 2026-10-30,
  `research-versions.md`): non è scartato per la manutenzione.

## Conseguenze e criteri di revisione

- **S-4** prova il validatore senza COOP/COEP su Chromium, Firefox e Safari, anche su telefono
  (Safari iOS, Chrome Android), e sotto una CSP restrittiva con `connect-src` verso il motore,
  `'wasm-unsafe-eval'` e il fetch del `.wasm`. Misura e riporta peso gzip con jq, tempo di avvio,
  latenza di una validazione e memoria iniziale nel browser con lo stack fisso. Verifica anche:
  - esiti identici al validatore nativo;
  - nessun «topictoast» nel bundle dell'ospite: JS, CSS, `.wasm`, nomi dei file (FR-44);
  - il comportamento con `'wasm-unsafe-eval'` negato (errore C-7 o validazione tramite l'API),
    scelto nell'ADR.

  L'ADR del kit aggiorna AD-11 e AD-19, e conferma la versione di emnapi fissata nello Stack. Le
  soglie di peso e di avvio le fissa Marco in quell'ADR.
- AD-11 si riapre se il bersaglio senza thread non gira su uno dei browser elencati o non regge la
  CSP dell'ospite.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63): nessuna voce E-1…E-9
  cambia la regola di AD-11. E-9 cambia solo come si fissano emnapi e le altre dipendenze del
  validatore: una correzione di una vulnerabilità pubblicata, anche solo come CVE o in una
  dipendenza transitiva nei lockfile, si fissa subito (Stack della spine). Restano da valutare
  nelle Open Questions della spine provenienza verificata, `npm ci --ignore-scripts` e
  `cargo audit` nel pre-push (sicurezza M13).
