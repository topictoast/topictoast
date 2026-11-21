# ADR: API pubblica HTTP con JSON e kit nelle pagine dell'ospite

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-12, AD-19 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-26, FR-27, FR-35…FR-39, FR-41, FR-44…FR-49, NFR-8, §6; CLAUDE.md («BDD and
  tests»); [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 37, 39,
  48, 51, 53, 56, 59; `review-decisions.md` (D-1, E-2, E-5, E-6); revisioni `reconcile-prd.md`,
  `reconcile-ux.md` (R-3), `reconcile-brief-rules.md` (R23), `review-security.md` (SEC-M7),
  `review-adversarial.md` (R23, R25)

## Contesto

Nucleo JS del kit, dashboard, SDK TS e Rust e CLI parlano con il motore. L'API e il protocollo
dei worker sono superfici pubbliche, documentate in modo da permettere SDK di terzi (PRD §6). Il
tempo reale arriva entro 1 s (FR-41). Gli scenari BDD confrontano il JSON esatto, tipi compresi:
`3` non è `3.0` (CLAUDE.md).

Il kit vive nelle pagine della piattaforma ospite:

- la credenziale firmata ha scadenza breve (FR-27, C-15);
- il bundle non porta il marchio (FR-44);
- ogni connessione fuori da localhost è cifrata (NFR-8).

## Decisione

- **[AD-12](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)** (memlog,
  voce 39): una sola API per tutti i client, a risorse HTTP con JSON.
  - Forma dell'API secondo C-16, C-17 e C-18 (E-6): campi JSON in camelCase, percorsi `/v1/…`,
    paginazione a cursore con `cursor` e `limit`, filtri come parametri di query, chiave di
    idempotenza nell'header `Idempotency-Key`, risorse identificate da UUIDv7 (C-1), posizione
    del bus come token opaco nel campo `position` e nell'header `Last-Event-ID`.
  - Lo schema OpenAPI si genera dai tipi Rust di `crates/protocol` (E-5); un solo generatore
    ricava dallo schema i tipi TS del nucleo JS (una volta, in `packages/core`) e dell'SDK TS, e lo
    fissa la prima storia che lo usa. La dashboard non ha un bersaglio proprio: usa quei tipi
    tramite il client dell'API di `packages/core` (AD-15).
  - Il tempo reale passa da SSE secondo C-8 e AD-18; gli eventi degli stream si inviano in
    blocchi.
  - Errori secondo C-7.
  - Il protocollo dei worker è separato, con i tipi in `crates/protocol`, e lo decide S-1.
- **AD-19:** il kit raggiunge il motore direttamente, con CORS limitato alle origini che
  l'integratore configura, oppure tramite un proxy dell'ospite.
  - La lista CORS rifiuta `*`, `null` e le corrispondenze per suffisso; le risorse dell'operatore
    non accettano le origini del kit e non mandano mai `Access-Control-Allow-Credentials`.
  - JS e `.wasm` del kit arrivano dal pacchetto installato dall'ospite, serviti dalla sua origine,
    mai scaricati dal motore.
  - Un'installazione di produzione fissa in modo esplicito indirizzo d'ascolto e TLS (C-14); il
    predefinito resta solo localhost (AD-1).
  - Nel nucleo JS l'SSE passa da `fetch` con l'header `Authorization`, mai con un token nell'URL o
    in un cookie; il motore chiude uno stream quando la sua credenziale scade.
  - La credenziale del kit segue i controlli di C-15 (E-2): `aud` verificato per risorsa, `iss`
    configurato, `exp` obbligatorio con una durata massima imposta dal motore, nessuna revoca
    nella v1.
  - La CSP della pagina ospite deve permettere `connect-src` verso il motore e
    `'wasm-unsafe-eval'` (S-4).

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:**
  - un solo dialetto per kit, dashboard, SDK e CLI, con la stessa forma per ogni risorsa: campi,
    percorsi, liste, filtri, idempotenza e posizione non cambiano da storia a storia (E-6);
  - il JSON si confronta esatto negli scenari;
  - SSE riprende da `Last-Event-ID` (C-8);
  - API leggibile da chi scrive un SDK di terzi;
  - il kit funziona sia con CORS sia dietro un proxy dell'ospite;
  - le origini dell'ospite non leggono le risposte delle risorse dell'operatore, e il motore non
    serve codice nelle pagine dell'ospite.
- **Costi:**
  - i due generatori (OpenAPI dai tipi Rust, TS da OpenAPI) sono ancora da scegliere (Deferred);
  - il trasporto SSE con `fetch` e header è codice del nucleo JS;
  - l'integratore configura origini CORS esatte o proxy, TLS e CSP della pagina ospite, e serve
    dalla propria origine JS e `.wasm` del kit.
- **Facilità di manutenzione:** i tipi Rust di `crates/protocol` sono l'unica fonte dello schema
  e dei tipi dell'SDK Rust, quindi i client non divergono (E-5). Il protocollo dei worker evolve a parte, con la sua versione (C-6).

## Alternative scartate

- **Connect con protobuf** (memlog, voce 39): gli `int64` diventano stringhe nel JSON, contro il
  confronto del JSON esatto; è anche meno immediato per chi integra.
- **ts-rs per i tipi TS** (`reconcile-brief-rules.md` R23; Stack della spine, «Da non usare»): una
  sola strada per i tipi TS di tutti i client, quella da OpenAPI.
- **Convenzioni dell'API fissate dalla prima storia che crea una risorsa, con un Update della
  spine** (`review-rubric.md`, seconda strada proposta): nessun motivo registrato; Marco ha scelto
  di fissarle subito (E-6, memlog voce 63). La rubrica segnala il rischio del silenzio: le storie
  delle liste di esecuzioni (FR-46) e di workflow (FR-58) divergerebbero.
- **Per AD-19 nessuna alternativa registrata nel memlog.** Le revisioni (`reconcile-prd.md`,
  `reconcile-ux.md` R-3, `review-security.md` SEC-M7) hanno segnalato i rischi che AD-19 previene:
  - un token nell'URL finisce nei log di accesso;
  - uno stream aperto può sopravvivere alla scadenza della credenziale o al cambio di tenant;
  - manca una politica CORS per il kit;
  - spesso l'ospite sta sullo stesso sito del motore: se la lista CORS del kit valesse anche per le
    risorse dell'operatore e mandasse `Access-Control-Allow-Credentials`, le risposte della
    dashboard diventerebbero leggibili dalle origini dell'ospite.

## Conseguenze e criteri di revisione

- **S-1** decide il protocollo dei worker fra tre varianti: worker diretti sui topic del bus,
  worker dal motore su gRPC con TLS, worker dal motore su QUIC con iroh. Per le varianti (2) e (3)
  fissa e prova: credenziale di worker legata alla connessione e controllata a ogni collegamento,
  verifica del server obbligatoria nell'SDK; con iroh, relay e scoperta esterni spenti nella
  configurazione predefinita e identità del nodo legata a C-11. L'ADR del bus aggiorna AD-12.
- **D-1** (memlog, voce 53): la variante (1) è esclusa, qualunque siano i numeri, se dà ai worker
  chiavi dei tenant o segreti, se C-11 non si applica dal bus o se gli eventi di stato non li
  scrive solo il motore.
- **S-4** prova la CSP dell'ospite con `connect-src`, `'wasm-unsafe-eval'` e il fetch del `.wasm`,
  e il bundle dell'ospite senza «topictoast» (FR-44). L'ADR del kit aggiorna AD-11 e AD-19.
- **Open Questions e rinvii della spine:**
  - contratto pubblico del kit (M-45, M-46, M-47, T-63), prima della prima storia del kit;
  - chi serve i file della dashboard, sessione dell'operatore e CSRF, prima della storia d'accesso;
  - granularità degli eventi e battito del server, prima della prima storia del tempo reale;
  - livello BDD di SDK TS e CLI.
- I generatori (OpenAPI dai tipi Rust, TS da OpenAPI) li fissa la prima storia che li usa
  (Deferred della spine). AD-19 si riapre se S-4 mostra che il validatore non regge la CSP
  richiesta all'ospite.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63), applicato sopra:
  - E-6 in AD-12: convenzioni dell'API C-16, C-17 e C-18;
  - E-5 nella sorgente dello schema OpenAPI: il crate puro `crates/protocol` con i tipi dell'API
    e del protocollo dei worker;
  - E-2 nei controlli della credenziale del kit: `aud` verificato per risorsa, `iss` configurato,
    `exp` obbligatorio con durata massima (C-15).

  Restano da valutare nelle Open Questions della spine `If-Match` obbligatorio con 428
  (`review-adversarial.md` R25), i predefiniti applicati solo dal motore (`review-adversarial.md`
  R23) e l'autorizzazione per operazione in una tabella con un test per operazione (rubrica).
