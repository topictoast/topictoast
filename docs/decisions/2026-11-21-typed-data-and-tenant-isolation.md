# ADR: dati tipizzati e isolamento tra tenant nel repository

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-4, AD-5 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md) e la PATCH 1.0.1 della costituzione
- Fonti: costituzione V; PRD FR-5, FR-26, FR-31, FR-40, NFR-1, NFR-8;
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 2, 27, 30, 48, 49,
  50, 51, 55, 56, 58, 59; `review-decisions.md` (D-2, E-1, E-2, E-8); revisioni
  `reconcile-brief-rules.md` (R29), `reconcile-ux.md` (R-13, R-16), `review-security.md` (SEC-M1),
  `review-adversarial.md` (R22)

## Contesto

- La costituzione V chiede colonne tipizzate e tabelle figlie per i dati che il sistema
  interpreta, niente `jsonb` né `TEXT` JSON. L'utente applicativo del database non è superuser e
  non ha `BYPASSRLS`.
- Il workflow salvato deve essere valido secondo lo schema di topictoast (FR-40) e ha versioni
  (FR-5).
- Ogni tenant vede solo le proprie risorse (FR-26). NFR-8 chiede un test per ogni risorsa e ogni
  punto di accesso.
- SQLite incorporato è una promessa del PRD e la base di NFR-1 (≤ 80 MB). SQLite non ha utenti
  del database né RLS.

## Decisione

- **[AD-4](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)** (memlog,
  voce 27): la costituzione V si legge alla lettera. Documenti OWS, schemi dei manifest e posizioni
  dei nodi si scompongono in tabelle tipizzate. Il documento salvato si rigenera in forma canonica
  e la forma originale non si conserva. La forma canonica conserva l'ordine dei task in ogni
  contenitore, `dag` compreso (memlog, voce 59). Ogni cambio della spec OWS è una migrazione.
- **AD-5** (memlog, voce 30): ogni accesso al database passa da un solo adattatore del
  repository, che aggiunge sempre il filtro del tenant. Un test con due tenant copre ogni metodo,
  con SQLite e con Postgres. Con Postgres, in più, RLS con un utente non superuser e senza
  `BYPASSRLS`. Dopo la verifica degli input e le revisioni la regola fissa anche:
  - ogni tenant ha un identificativo interno UUIDv7; il campo `tenant` del JWT è un nome esterno
    che il motore traduce in quell'identificativo, e `default` è riservato al tenant predefinito;
    né i nomi né i valori del JWT finiscono mai in percorsi, nomi di topic o chiavi (E-1);
  - l'integratore è un proprietario con un identificativo interno riservato e una propria chiave,
    e segue lo stesso filtro (E-1);
  - tenant e permessi di una richiesta vengono solo dalla credenziale verificata: C-15, sessione
    dell'operatore o credenziale di worker (C-11), ognuna con il proprio ambito. Per il JWT
    «verificata» vuol dire i controlli di C-15: solo `EdDSA` e `Ed25519`, `iss` verificato, `aud`
    verificato per risorsa, `exp` obbligatorio con una durata massima imposta dal motore, nessuna
    revoca nella v1 (E-2);
  - il tenant predefinito esiste dal primo avvio e segue lo stesso percorso;
  - ogni chiave d'idempotenza, di versione o d'indirizzo comprende il tenant, e nessun supporto
    deduplica tra tenant: il test a due tenant usa la stessa chiave d'idempotenza e lo stesso
    modulo;
  - lo stesso filtro, con il test a due tenant, vale per gli eventi in tempo reale (AD-18).
- **Versioni di Postgres** (D-2, memlog voce 55): major 18 e 19, minime 18.7 e 19.0; il test a due
  tenant con Postgres gira sull'immagine 18.7 fino al 2026-11-30, poi sulla 19.x.
- **PATCH 1.0.1 della costituzione.** Marco ha deciso una PATCH che scrive che con SQLite
  l'isolamento sta nell'adattatore (voce 30). Dopo la verifica degli input (`reconcile-brief-rules.md`
  R29, memlog voce 49) è stata riformulata per tutti i database:
  - un solo adattatore del repository applica sempre il filtro del tenant, con un test a due
    tenant per ogni metodo;
  - con Postgres l'RLS è una seconda barriera;
  - con SQLite l'adattatore è l'unica barriera.

  Il testo è nella costituzione di questa PR. La riformulazione va oltre la frase su SQLite che
  Marco aveva scelto, e Marco l'ha approvata con E-8 di `review-decisions.md` (memlog, voce 63).

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:** nessun JSON opaco nel database; ogni campo si interroga e si migra. FR-40
  resta soddisfatto. Un solo percorso di isolamento per SQLite e Postgres, tenant predefinito e
  integratore compresi; con Postgres l'RLS è una seconda barriera (E-8). Nessuna chiave condivisa
  tra tenant, quindi nessuna deduplicazione che passi il confine. Un nome scelto dall'ospite non
  finisce mai in un percorso, un topic o una chiave: un tenant chiamato `default` o con `/` o `..`
  nel nome non tocca il predefinito né esce dalla propria cartella (E-1). Un token rubato vale al
  massimo la durata imposta dal motore (E-2).
- **Costi:**
  - il motore traduce il nome esterno di ogni credenziale nell'identificativo interno (E-1);
  - nessuna revoca nella v1: un token rubato resta valido fino alla scadenza (E-2);
  - la forma originale del documento salvato si perde;
  - ogni cambio della spec OWS diventa una migrazione: la 1.1.0 ha 8 breaking change pur essendo
    una minor (memlog, voce 2);
  - con SQLite l'isolamento ha una sola barriera;
  - un test a due tenant per ogni metodo del repository, ripetuto su SQLite e su Postgres.
- **Facilità di manutenzione:** un solo adattatore da verificare per il filtro del tenant. Le
  tabelle seguono il modello OWS e il linguaggio degli schemi (AD-4, AD-10), quindi cambiano con
  loro.

## Alternative scartate

- **Documenti verbatim, con una PATCH della costituzione** (memlog, voce 27): scartata perché
  Marco ha scelto la lettura alla lettera della costituzione V (memlog, voce 50).
- **Deroga scritta in ogni piano** (memlog, voce 27): deroghe ripetute in molte storie (memlog,
  voce 50).
- **Un file SQLite per tenant** (memlog, voce 30): file e migrazioni moltiplicati, viste
  dell'operatore costose.
- **Solo Postgres** (memlog, voce 30): rompe NFR-1 e la promessa di SQLite incorporato del PRD.
- **PATCH 1.0.1 solo per SQLite**, la frase che Marco aveva scelto (memlog, voce 30;
  `reconcile-brief-rules.md` R29): con Postgres la costituzione non imporrebbe né il filtro né il
  test a due tenant. Sostituita da E-8 (memlog, voce 63).
- **Valore del campo `tenant` del JWT usato direttamente in percorsi, nomi di topic e chiavi**
  (`review-adversarial.md` R10): una storia lo usa come nome di cartella e di topic, un'altra lo
  traduce in un id interno; un tenant dell'ospite chiamato `default` coincide con il predefinito;
  un valore con `/` o `..` esce dalla cartella del tenant; cartelle, chiavi S3 e nomi di topic
  ammettono caratteri diversi.
- **Integratore senza un proprietario esplicito, per esempio con il tenant a NULL**
  (`review-rubric.md`, `review-adversarial.md` R15): una storia mette NULL, un'altra un valore
  riservato, e la politica RLS e il filtro SQLite trattano il NULL in modo diverso.
- **C-15 con i soli campi, senza verifiche fissate** (`review-security.md` SEC-A8): l'algoritmo
  resta aperto (`none`, o un HMAC che usa la chiave pubblica come segreto), e senza durata
  massima una credenziale `sdk` firmata per un anno è una chiave padrona senza scadenza.
- **Per le altre correzioni dei revisori in AD-5 nessuna alternativa registrata nel memlog.**

## Conseguenze e criteri di revisione

Open Questions della spine:

- gli altri JSON dei tenant (letterali dei documenti, input delle esecuzioni e delle
  pianificazioni, errori conservati, righe delle pipeline stream, moduli dei nodi): da chiudere
  prima della prima storia che salva uno qualunque di questi dati;
- il proiettore sotto RLS con Postgres: S-1, misurato dentro NFR-4. Da provare con `FORCE ROW
  LEVEL SECURITY` su ogni tabella dei tenant, perché il proprietario delle tabelle altrimenti salta
  l'RLS (`review-security.md` SEC-M1), e con il tenant impostato solo per la transazione;
- le posizioni dei nodi nella versione e nell'export: prima della prima storia dell'editor.

Criteri di revisione:

- AD-5 si riapre se S-1 mostra che il proiettore sotto RLS non sta in NFR-4;
- AD-4 si rivede quando si chiude la domanda sugli altri JSON dei tenant;
- ogni cambio della costituzione passa da una PR con versione e data (Governance).

**Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63), applicato sopra:

- E-8 nel testo della PATCH 1.0.1: con ogni database un solo adattatore applica il filtro con il
  test a due tenant, e con Postgres l'RLS è una seconda barriera;
- E-1 in AD-5: identificativo interno UUIDv7 al posto del nome del JWT, `default` riservato,
  integratore come proprietario con un identificativo riservato, una propria chiave e lo stesso
  filtro;
- E-2 in «credenziale verificata»: algoritmi, `iss`, `aud` per risorsa, `exp` obbligatorio con
  durata massima, nessuna revoca nella v1 (C-15).

Restano da valutare nelle Open Questions della spine il tipo di ambito del tenant per ogni metodo
del repository (sicurezza B2), il tipo del dominio per il proprietario con il caso integratore
contro tenant nel test (`review-adversarial.md` R15) e la canonicalizzazione esposta ai client
(`review-adversarial.md` R22), che tocca la forma canonica di AD-4.
