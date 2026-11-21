# ADR: confine degli scenari BDD su due livelli

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-14 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: costituzione II; CLAUDE.md («BDD and tests»); PRD §6; `agenda.md` (C.9);
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 38, 50, 55, 56;
  `review-decisions.md` (D-2); revisioni `reconcile-brief-rules.md` (R15, R16, R31),
  `review-adversarial.md` (R32)

## Contesto

La costituzione II vuole ogni storia dentro scenari Gherkin in italiano, eseguiti con `cucumber`
al confine pubblico del sistema, con i servizi esterni simulati. Prima di questa PR CLAUDE.md
diceva: «the public boundary of the system (API, CLI or SDK), decided by the architecture».

Le superfici pubbliche del PRD sono diverse (§6):

- API del motore e protocollo dei worker;
- SDK TS e Rust, e CLI;
- nucleo JS e componenti React.

## Decisione

Registra [AD-14](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
(memlog, voce 38): due livelli, entrambi con cucumber-rs.

1. **Scenari di contratto:** API pubblica del motore più un worker di prova in Rust sul
   protocollo dei worker.
2. **Scenari dell'SDK Rust:** guidano il sistema come un integratore Rust.

Su entrambi i livelli:

- bus e database reali via testcontainers, servizi esterni simulati (`reconcile-brief-rules.md`
  R31; CLAUDE.md di questa PR);
- gli scenari avviano come processo il binario del motore già costruito, mai il motore dentro il
  test; il percorso del binario per gli ausiliari è configurabile (AD-16).

Con Postgres l'immagine di test è la 18.7 fino al 2026-11-30, poi la 19.x (D-2, memlog voce 55).
Kit e dashboard restano a Playwright BDD.

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:**
  - gli scenari di contratto coprono l'API e il protocollo dei worker, entrambi superfici
    pubbliche;
  - gli scenari dell'SDK coprono l'SDK Rust come lo usa un integratore;
  - un solo esecutore per il Rust;
  - gli scenari provano il binario vero, con gli ausiliari avviati come in produzione.
- **Costi:**
  - due insiemi di step da mantenere e un worker di prova da scrivere;
  - bus e database reali via testcontainers su entrambi i livelli;
  - il binario del motore va costruito prima degli scenari;
  - nel repository ci sono due esecutori BDD, cucumber-rs e Playwright BDD;
  - il livello di SDK TS e CLI è ancora aperto.
- **Facilità di manutenzione:** il confine è lo stesso per ogni storia del motore. Gli step del
  livello di contratto parlano l'API e il protocollo dei worker, senza passare dall'SDK.

## Alternative scartate

- **Solo l'API** (memlog, voce 50): l'SDK non viene provato, eppure è una superficie pubblica
  (PRD §6).
- **Solo l'SDK Rust** (memlog, voce 50): un errore dell'SDK può nascondere un errore dell'API,
  che è documentata per permettere SDK di terzi (PRD §6).
- **Motore avviato dentro il processo di test** (`review-adversarial.md` R32; nessuna alternativa
  registrata nel memlog): è la via più rapida, ma l'ausiliario si avvia rilanciando l'eseguibile
  corrente con un sottocomando, e dentro il test l'eseguibile corrente è il binario di cucumber.

## Conseguenze e criteri di revisione

Open Questions della spine:

- il livello BDD di SDK TS e CLI, prima della prima storia che li tocca. Proposta: un worker di
  prova in TS al livello di contratto, e la CLI guidata da scenari cucumber-rs
  (`reconcile-brief-rules.md` R16);
- la PATCH 1.0.2 della costituzione II, oppure la deroga in AD-14 (Marco);
- il primo avvio (credenziale `aud = sdk`, prima credenziale di worker, certificati), prima dello
  sprint 1: serve agli scenari di contratto e dell'SDK.

Il protocollo che il worker di prova parla lo decide lo spike del bus (S-1). AD-14 si riapre se
la scelta di Marco sulla costituzione II cambia gli esecutori, o se SDK TS e CLI non trovano
posto in uno dei due livelli.

**Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63): nessuna voce E-1…E-9
cambia AD-14.
