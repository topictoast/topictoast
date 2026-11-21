# ADR: layout del repository e direzione delle dipendenze

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-15, AD-16 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: costituzione III, IV, VIII; CLAUDE.md («Repo layout», «Gate»); `agenda.md` (C.9);
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 40, 45, 48, 50,
  51, 53; `review-decisions.md` (D-1, E-5, E-7); revisioni `reconcile-ux.md` (R-4),
  `reconcile-brief-rules.md` (R2, R17, R22, R34), `review-rubric.md`, `review-adversarial.md` (R32)

## Contesto

Prima di questa PR il repository prevedeva `crates/` e `dashboard/`. L'architettura aggiunge
altre unità:

- un nucleo JS e i componenti React del kit;
- un SDK TS;
- l'output di napi-rs del validatore (AD-11);
- un design system condiviso tra kit e dashboard;
- il codice degli spike.

La costituzione III vuole un dominio puro e una sola composition root; la IV, prima della PATCH
1.0.1, un solo modulo per il client HTTP. Le regole della skill `dashboard-react` valgono per la
dashboard, ma il kit vive in pagine altrui.

## Decisione

- **[AD-15](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)** (memlog,
  voce 40):
  - `crates/` è il workspace Cargo: modello OWS, dominio, motore, adattatori, SDK Rust, CLI,
    collegamento napi-rs, benchmark;
  - `crates/ows` è un crate a parte, scritto da noi, con il modello del sottoinsieme OWS 1.1 più il
    task `dag`: puro, compila per `wasm32-wasip1`, non dipende dal resto del workspace (memlog,
    voce 57). L'SDK Rust ufficiale di OWS è solo un riferimento: né dipendenza né fork;
  - `crates/protocol` è un crate puro, senza I/O, con i tipi dell'API pubblica e del protocollo
    dei worker: non dipende dal resto del workspace, lo usano il motore e l'SDK Rust, ed è la
    sorgente dello schema OpenAPI (E-5);
  - `packages/` è il workspace npm: `ui`, `core`, `react`, `sdk`, `validator`;
  - `packages/ui`, il design system, lo usano sia `packages/react` sia `dashboard/`; la dashboard
    dipende da `packages/ui` e dal client dell'API di `packages/core`, che carica il validatore
    wasm in modo pigro alla prima validazione: la dashboard non lo istanzia mai;
  - `dashboard/` resta invariata;
  - `spikes/<nome>/` sta fuori dai workspace e dal gate;
  - le regole di `dashboard-react` non valgono per il kit, e la skill del kit si scrive prima della
    prima storia del kit.
- **Nomi fissati:** i nomi dei crate e dei pacchetti sono fissati nella spine, perché regole,
  `purity.rs` e gate li citano (Design Paradigm; `review-rubric.md`).
- **AD-16:**
  - `crates/ows` e `crates/protocol` non dipendono da nessun crate del workspace; `crates/domain`
    dipende solo da `crates/ows`; nessuno dei tre dipende da I/O, runtime async, client HTTP o
    driver di database (test `purity.rs`);
  - solo `crates/engine` sceglie e collega gli adattatori; i processi ausiliari sono sottocomandi
    del suo binario, senza una seconda composition root, e il percorso del binario per gli
    ausiliari è configurabile (AD-14);
  - il client HTTP vive in un solo modulo per artefatto (costituzione IV, PATCH 1.0.1; E-7): per
    il motore in `crates/adapters`, da cui passano `call http` e RustFS; per l'SDK Rust in
    `crates/sdk`, che CLI e benchmark usano;
  - nessun client, Rust o JS, dipende da `crates/engine` o `crates/adapters`: usa solo l'API del
    motore e il protocollo dei worker, che va al motore o al bus secondo S-1;
  - i processi ausiliari non ricevono mai la chiave principale; quello delle pipeline stream
    riceve solo la chiave del suo tenant (AD-6, AD-13);
  - la validazione in JS passa solo da `packages/validator` (AD-11); la dashboard usa solo
    `packages/ui` e il client dell'API di `packages/core`.

  Motore e ausiliari sono lo stesso binario, che linka il validatore del dominio: per questo AD-6
  fa chiamare le funzioni che interpretano contenuto dei tenant solo tramite un permesso che esiste
  solo in modalità ausiliario, con un test che la modalità motore non le chiama mai.

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:**
  - un solo design system per kit e dashboard;
  - il validatore si costruisce una volta e si consuma da `packages/validator`;
  - i tipi dell'API e del protocollo dei worker si scrivono una volta, in `crates/protocol`:
    l'OpenAPI nasce da lì e l'SDK Rust non li ridichiara (E-5);
  - l'SDK Rust chiama l'API con il proprio client HTTP, senza dipendere da `crates/adapters`
    (E-7);
  - gli spike non entrano nel gate.
- **Costi:**
  - un crate puro in più da coprire con `purity.rs` (E-5);
  - due moduli del client HTTP, uno per artefatto (E-7);
  - due workspace con due gate: Cargo e `(cd packages && npm run gate)`;
  - il dominio deve compilare per `wasm32-wasip1`, con una riga in più nel gate (CLAUDE.md di
    questa PR);
  - una skill del kit da scrivere prima della prima storia del kit;
  - lo stesso binario per motore e ausiliari chiede il permesso della modalità ausiliario e il suo
    test (AD-6).
- **Facilità di manutenzione:**
  - le frecce sono verificabili: `purity.rs` per i crate puri, `clippy.toml` per il client HTTP
    di ogni artefatto quando arriva;
  - una sola composition root;
  - i nomi dei crate e dei pacchetti sono fissati, quindi regole, `purity.rs` e gate non cambiano
    da storia a storia.

## Alternative scartate

- **Fork mantenuto dell'SDK Rust ufficiale di OWS** (memlog, voce 57): andrebbe riscritto comunque
  dalla 1.0 alla 1.1, modella 12 tipi di task oltre il sottoinsieme supportato, crea un impegno
  pubblico di manutenzione, e l'upstream è fermo dal 2025-12-12.
- **SDK ufficiale come dipendenza diretta** (memlog, voce 57): è un'alpha, sul DSL 1.0, senza
  validazione.
- **Modello OWS dentro `crates/domain`** (memlog, voce 57): Marco ha scelto un crate a parte, così
  il modello del formato resta separato dalle regole del dominio.
- **Cartelle separate in radice, senza un workspace npm** (memlog, voce 50): dipendenze e
  versioni da gestire a mano tra pacchetti che si usano a vicenda: `packages/react` usa
  `packages/core` e `packages/ui`, `packages/core` usa `packages/validator` (AD-16). `dashboard/`
  resta fuori dal workspace e usa `packages/ui` e `packages/core` come dipendenze (AD-15).
- **Processi ausiliari con un proprio cablaggio degli adattatori**, cioè una seconda composition
  root (`reconcile-brief-rules.md` R2): viola la costituzione III. Qualunque separazione scelga
  S-2, il cablaggio resta in `crates/engine` e gli ausiliari sono sottocomandi dello stesso
  binario.
- **Tipi dell'API nel motore, accanto al server HTTP** (`review-adversarial.md` R11): l'SDK Rust
  non può dipendere dal motore (AD-16) e li riscriverebbe a mano, e il protocollo dei worker
  finirebbe scritto due volte negli SDK TS e Rust; nomi di campo, `3` contro `3.0`, campi
  facoltativi e codici C-7 divergerebbero tra i due SDK.
- **Tipi dell'API in un modulo del dominio** (`review-adversarial.md` R11, `review-rubric.md`):
  nessun motivo registrato; Marco ha scelto un crate a parte (E-5, memlog voce 63).
- **Un solo modulo del client HTTP in tutto il workspace, in `crates/adapters`**, la regola
  precedente di AD-16 (`review-rubric.md`): l'SDK Rust chiama l'API in HTTP, quindi dovrebbe
  dipendere da `crates/adapters`, che AD-16 vieta, oppure avere un secondo client, che la regola
  stessa vieta. Sostituita da E-7 e dalla PATCH 1.0.1 della costituzione IV.
- **Nomi dei crate indicativi, fissati dalla prima storia che li crea** (`review-rubric.md`;
  nessuna alternativa registrata nel memlog): AD-15, AD-16, `purity.rs` e il gate legano le regole
  ai nomi.

## Conseguenze e criteri di revisione

- **S-2** decide quali processi ausiliari separare, sempre come sottocomandi del binario del
  motore, e prova la «lettura della chiave principale» per ogni sottocomando ausiliario.
- **Open Questions della spine:** le licenze ammesse per le dipendenze di nucleo e kit (Marco),
  prima di fissare la prima dipendenza dello sprint 1; la distribuzione delle librerie client e le
  piattaforme del validatore nativo per Node, prima della prima storia dell'SDK o del kit.
- **Criteri di revisione:** se S-2 richiede un processo che non può essere un sottocomando dello
  stesso binario, serve un ADR nuovo, perché tocca la costituzione III. Se il collegamento dei
  worker scelto in S-1 cambia le frecce dei client verso motore o bus, AD-16 si aggiorna; con D-1
  (memlog, voce 53) la variante (1), quella dei worker diretti sul bus, è esclusa se manca una
  delle sue condizioni di sicurezza.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63), applicato sopra:
  - E-5 in AD-15 e nelle frecce di AD-16: crate puro `crates/protocol` con i tipi dell'API e del
    protocollo dei worker, usato da motore e SDK Rust, sorgente dello schema OpenAPI;
  - E-7 nella regola del client HTTP di AD-16: un solo modulo per artefatto, motore e SDK Rust,
    invece di uno in tutto il workspace.

  Resta da valutare nelle Open Questions della spine da cosa dipende `packages/sdk` (rubrica).
