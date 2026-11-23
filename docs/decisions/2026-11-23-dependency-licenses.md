# ADR: licenze ammesse per le dipendenze

- Data: 2026-11-23
- Stato: accettata
- Registra: paragrafo sulle licenze nello Stack della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md); chiude la domanda aperta
  sulle licenze (proprietario Marco), da chiudere prima della prima dipendenza dello sprint 1
- Fonti: PRD NFR-11; costituzione; [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voce 72

## Contesto

Tutto il codice di topictoast è Apache-2.0, senza parti a pagamento, e le dipendenze devono avere
licenze compatibili (NFR-11). Il kit gira dentro le pagine di altri. Finora la spine escludeva solo
elkjs (EPL-2.0 o GPL-3.0), caso per caso.

## Decisione

Decisioni di Marco del 2026-11-23 (memlog, voci 72 e 76), per le dipendenze Rust e npm, sia del
motore sia del kit:
- **ammesse:** MIT, Apache-2.0 (anche WITH LLVM-exception, la licenza di wasmtime e cranelift),
  BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, Unicode-3.0, CDLA-Permissive-2.0, BSL-1.0,
  CC0-1.0, bzip2-1.0.6 (DataFusion), BlueOak-1.0.0 (glob, minimatch), e MPL-2.0 per pacchetti non
  modificati;
- **solo nelle dipendenze di sviluppo:** CC-BY-4.0 (caniuse-lite) e PSF-2.0 (argparse), che non
  finiscono nel motore né nel kit distribuiti;
- **escluse:** EPL, LGPL, GPL, AGPL, SSPL, BUSL e i pacchetti senza licenza;
- **espressioni SPDX:** con `OR` basta una licenza ammessa, con `AND` servono tutte, con `WITH`
  solo un'eccezione elencata;
- **perimetro:** i lockfile Cargo e npm di `crates/`, `packages/` e `dashboard/`; `spikes/` è
  fuori;
- **eccezioni:** per un pacchetto e una versione, con un ADR, elencate nella configurazione dello
  strumento;
- **controllo:** licenze e avvisi di sicurezza in automatico nel pre-push; lo strumento lo fissa la
  prima storia che aggiunge una dipendenza.

La verifica dei fatti (`reviews/review-pre-s1-reality.md`) ha provato l'elenco su un albero di 218
crate (sqlx, tokio, rustls, axum, tonic): tutte ammesse valutando le espressioni `OR`; `ring` è
`Apache-2.0 AND ISC`.

## Bilancio

- **Funzioni:** chi incorpora il kit o distribuisce topictoast non riceve obblighi di copyleft.
- **Costi:** qualche libreria utile resta fuori (oggi elkjs); MPL-2.0 chiede di rendere disponibile
  il sorgente dei file MPL modificati, e per questo è ammessa solo non modificata.
- **Manutenzione:** un elenco chiuso e un controllo automatico, invece di un giudizio per ogni
  dipendenza.

## Alternative scartate

- **Solo licenze permissive:** è la regola più semplice, ma MPL-2.0 compare tra le dipendenze
  indirette di crate comuni e richiederebbe più eccezioni.
- **Anche il copyleft debole (MPL, EPL, LGPL):** dà più libertà nella scelta, ma porta obblighi a chi
  incorpora il kit, e farebbe rientrare elkjs.
