# ADR: linguaggio degli schemi, sottoinsieme chiuso di JSON Schema 2020-12

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-10 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-37, FR-39, FR-55, FR-56, §11 n.16; addendum del PRD («Dopo la revisione
  dell'aggiornamento»); `extract-prd.md` (M-05); `research-ows.md`; `research-versions.md` (jtd);
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 12, 32, 42, 56;
  `conventions.md` (C-10); `review-decisions.md`; revisione `review-versions.md`

## Contesto

Manifest delle funzioni (FR-55) e schema di input del workflow (FR-56) hanno bisogno di un
linguaggio degli schemi chiuso (PRD §11 n.16, M-05), con tre vincoli:

- nessun riferimento esterno (FR-55);
- derivabile dai tipi TS e Rust (FR-37);
- stessi esiti in motore, nucleo JS ed editor (FR-39).

I candidati dell'addendum del PRD erano due: un sottoinsieme chiuso di JSON Schema senza
riferimenti esterni, oppure JSON Type Definition (RFC 8927). Lo schema di OWS è JSON Schema 2020-12,
e OWS ha già `input.schema` (`research-ows.md`). Quello schema ha però un `format` (predefinito
`json`, ammesso qualsiasi `{format}:{version}`) e un documento libero oppure una `resource`
esterna: OWS accetta JSON Schema completo, altri formati e schemi esterni (`review-versions.md`).

## Decisione

Registra [AD-10](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
(memlog, voce 32): un sottoinsieme chiuso di JSON Schema 2020-12.

- Le parole chiave ammesse e quelle vietate sono elencate nella spine; ogni altra parola chiave è
  rifiutata con un errore chiaro.
- È un sottoinsieme chiuso di `input.schema` di OWS con `format: json`: JSON Schema completo, altri
  formati e `resource` esterne, che OWS accetta, sono rifiutati. È la correzione dei revisori del
  2026-11-21 (memlog, voce 56; `review-versions.md`) al posto di «stesso linguaggio di
  `input.schema`».
- Una sola implementazione, nel validatore del dominio.
- I dettagli (oggetti chiusi, `integer` distinto da `number`, `null` diverso da assente,
  predefiniti dopo la validazione) sono in C-10, approvata in blocco (voce 42).

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:** è un sottoinsieme chiuso di `input.schema` di OWS con `format: json`. Si
  deriva dai tipi TS e Rust. Il linguaggio chiuso dà errori
  chiari, e la parità tra motore e kit c'è per costruzione (AD-11).
- **Costi:**
  - senza `$ref`, `oneOf`, `anyOf`, `allOf`, `if`/`then` non si esprimono schemi con varianti o
    riusi;
  - uno schema JSON esistente che usa quelle parole chiave è rifiutato;
  - un documento OWS il cui `input.schema` usa JSON Schema completo, un altro formato o una
    `resource` esterna è rifiutato;
  - la regola di compatibilità tra campi è ancora da scrivere (Deferred).
- **Facilità di manutenzione:** una sola implementazione da provare, su un sottoinsieme piccolo.
  Si mappa su tabelle di campi (AD-4).

## Alternative scartate

- **JSON Type Definition (RFC 8927)** (memlog, voce 32), per tre motivi:
  - il crate `jtd` è fermo dal 2021, come `jtd-codegen` e `jtd-infer`, e su crates.io non c'è
    un'alternativa mantenuta (`research-versions.md`; memlog, voce 12);
  - è diverso dal linguaggio di OWS;
  - ha pochi strumenti.

## Conseguenze e criteri di revisione

Rinvii della spine:

- la regola di compatibilità tra campi, alla spec delle storie delle versioni (FR-5, FR-55);
- la forma del campo di estensione del manifest (M-06);
- la derivazione del manifest dai tipi (M-07).

Nessuno spike la misura. AD-10 si regge sul fatto che è un sottoinsieme di `input.schema` di OWS
con `format: json`: un cambio di OWS su quel punto la riapre. Allargare il sottoinsieme tocca il
formato pubblico del manifest (PRD §6) e chiede un ADR nuovo.

**Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63): nessuna voce E-1…E-9
cambia AD-10. Resta da valutare nelle Open Questions della spine l'elenco esatto delle parole
chiave per minimi, massimi e lunghezze (rubrica).
