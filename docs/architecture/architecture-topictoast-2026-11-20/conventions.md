---
title: topictoast — convenzioni proposte in blocco
created: 2026-11-21
stato: approvata da Marco il 2026-11-21, estensioni comprese; C-16…C-18 da E-6 (memlog, voci 42, 62 e 63)
fonti: agenda.md (gruppo D), extract-prd.md (M-15…M-43), decisioni nel memlog, review-decisions.md (E-1, E-2, E-3, E-6)
---

# Convenzioni proposte in blocco

Una sola approvazione per tutto il blocco. Ogni voce indica il meccanismo del PRD che chiude.

## Identità e chiavi

- **C-1 Identificativi e tempi.** UUIDv7 per esecuzioni, task, attività, eventi delle
  esecuzioni, risorse dell'API (C-16) e identificativo interno dei tenant (AD-5). L'id di un evento SSE è il token della posizione del bus (AD-18); l'id degli eventi
  degli stream lo fissa lo spike delle pipeline stream (M-35). Tempi in UTC, RFC 3339 nell'API.
  Durate in ISO 8601, come in OWS.
- **C-2 Tentativo (M-16).** Ogni consegna porta `attemptId`, cioè l'attività più il numero del
  tentativo. Un risultato di un tentativo non più corrente viene rifiutato e registrato.
  Risultato, battito e scritture di un tentativo si accettano solo dalla sessione del worker a
  cui il motore lo ha consegnato; per ogni `attemptId` vale il primo esito registrato, i
  successivi sono rifiutati e registrati.
- **C-3 Scadenze (M-20).** Chiave di scadenza = pianificazione + versione + istante previsto in
  UTC. La controllano il motore e un vincolo di unicità nella proiezione, così ogni scadenza dà
  una sola esecuzione anche dopo un fermo. Le scadenze rifiutate si registrano con il motivo.
- **C-4 Riga → avvio (M-21).** Chiave = pipeline + versione + fine della finestra + chiave del
  gruppo; per le righe senza finestra, la posizione nello stream. Si usa come chiave di
  idempotenza dell'avvio (FR-6): righe uguali in finestre diverse danno due avvii.
- **C-5 Versioni (M-22).** Una pubblicazione porta la versione di base (`If-Match` con ETag) e
  viene rifiutata se nel frattempo un'altra l'ha superata. La versione di un nodo compilato è
  l'impronta SHA-256 del contenuto.

## Protocollo e interfacce

- **C-6 Versione del protocollo dei worker (M-17).** Il worker la dichiara alla registrazione.
  In 0.y.z serve la stessa minor, dopo la 1.0 la stessa major; se è incompatibile, il worker
  riceve un rifiuto con codice.
- **C-7 Errori.** `application/problem+json` (RFC 9457) con `errors[]`. Ogni errore ha `code`
  (stabile, in inglese, a punti), `params` (per i messaggi in italiano e inglese del kit) e
  `path` (JSON Pointer nel documento). Il validatore Rust usa lo stesso formato nel motore e nel
  kit. Campi facoltativi `line` e `column` per SQL, jq, JSON e cron; un errore che riguarda un
  task porta anche il percorso dei nomi dei task, stabile tra le versioni. Un solo catalogo di
  codici, nel dominio, copre errori, motivi di attesa e motivi di salto; arriva ai tipi TS
  tramite lo schema OpenAPI (AD-12).
- **C-8 Tempo reale.** SSE; un client che si ricollega riprende da `Last-Event-ID`. L'id
  dell'evento è il token opaco della posizione del bus; origine, filtro e trasporto degli eventi
  seguono AD-18 e AD-19.
- **C-9 Metriche e log (FR-52).** Metriche in formato OpenMetrics su `/metrics`. Log JSON su
  stdout, una riga per evento. Mai segreti, token, chiavi o cookie. Mai nemmeno nomi visualizzati
  né input e output delle attività (FR-52); fuori da localhost `/metrics` segue C-14. Lo stderr
  degli ausiliari non entra nei log così com'è: il motore registra solo il codice C-7 e gli
  identificativi. Le righe dei log dei task (FR-59) non sono log del motore: non vanno mai su
  stdout né in `/metrics`, stanno nell'archivio dei payload (AD-17).
- **C-10 Dettagli degli schemi (M-05).** Gli oggetti sono chiusi per impostazione predefinita:
  un campo in più viene rifiutato, salvo `additionalProperties: true`. `integer` è distinto da
  `number`: `3` non è `3.0`. `null` è diverso da un campo assente ed è ammesso solo se il tipo lo
  include. I valori predefiniti si applicano dopo la validazione.

## Sicurezza

- **C-11 Credenziali dei worker (M-15).** Una per worker, revocabile una per una, limitabile a
  funzioni e tenant, distinta dalle credenziali dei tenant e dell'integratore.
- **C-12 Cassaforte (M-36, M-38).** Cifratura a busta: chiave principale dell'installazione →
  una chiave per tenant, cifrata dalla principale → segreti cifrati con AES-256-GCM. Ogni oggetto
  cifrato porta una busta versionata con l'id della chiave; per ogni oggetto o segmento si deriva
  dalla chiave del tenant, con HKDF, la chiave che lo cifra in AES-256-GCM; l'`info` di HKDF
  contiene l'indirizzo completo dell'oggetto o del pezzo (per i log dei task anche il numero del
  pezzo, AD-17), così due oggetti non condividono mai chiave e nonce. Così non si raggiunge
  il limite dei nonce di AES-GCM, e la chiave del tenant ruota tramite l'id. Il formato è uno
  solo, nel dominio, e lo usano motore, proiettore, processo delle pipeline stream e archivio dei
  payload; S-1 ne misura il costo. I segreti si risolvono alla consegna e non vengono mai scritti nei messaggi del bus. La chiave principale
  viene da un file o da una variabile d'ambiente; si genera solo se il deposito delle chiavi è
  vuoto, e il motore ne conserva l'impronta: se la chiave presente non corrisponde, rifiuta di
  partire con un codice (AD-7). La chiave generata sta fuori dalle directory dati di topictoast e
  del bus. Ruotare la chiave principale (FR-34) vuol dire cifrare di nuovo le chiavi dei tenant,
  senza conservare la vecchia busta.
- **C-13 Cancellazione del tenant (M-39).** Input, output ed eventi dei tenant sono cifrati con
  la chiave del tenant, nel formato di C-12, su bus, file e checkpoint. Eliminare un tenant vuol dire cancellarne la
  chiave (cancellazione crittografica), seguita da una pulizia asincrona. Il costo della
  cifratura si misura nello spike del bus. Ordine (AD-7, FR-31): si fermano pipeline stream,
  esecuzioni, pianificazioni, sandbox riservate e stream SSE del tenant, e se ne rifiutano le
  richieste; si scrive sul bus l'evento lapide del tenant; si cancella la chiave; si cancellano
  le righe del database tramite l'adattatore del repository e i nodi caricati; poi la pulizia
  asincrona di bus e archivio dei payload. La rilettura del bus salta solo gli eventi dei tenant
  con lapide; un evento che non si decifra senza lapide è un guasto, mai un'eliminazione. La
  cancellazione nel database non lascia residui secondo AD-7 (SQLite e Postgres). Il test del
  valore sentinella di FR-31 copre database (file compresi), bus, archivio dei payload,
  checkpoint e nodi.
- **C-14 TLS e valori predefiniti (M-40, M-41).** TLS (rustls) su ogni connessione fuori da
  localhost, con certificati forniti dall'integratore oppure generati al primo avvio, stampando
  l'impronta. L'SDK verifica sempre il
  certificato e non esiste una modalità senza autenticazione. Nessuna chiave o credenziale
  predefinita: si generano al primo avvio, si mostrano una sola volta, e i valori degli esempi
  vengono rifiutati. Le credenziali del primo avvio non vanno mai su stdout né nei log (FR-45,
  FR-52).
- **C-15 Credenziale firmata (M-42, M-43).** JWT firmato dalla piattaforma ospite. Sono ammessi
  solo gli algoritmi `EdDSA` e `Ed25519`, con chiave Ed25519 (RFC 9864). `iss` è configurato e
  verificato. `exp` è obbligatorio e il motore impone una durata massima configurabile, con un
  valore predefinito breve. Nella v1 non c'è revoca. Campi: `tenant`, `role`, `displayName`
  facoltativo, `selectedTenant` (il tenant scelto dall'integratore) e `aud`, che vale `kit` o
  `sdk`, distingue le due credenziali dell'integratore (FR-28) ed è verificato per risorsa
  (risorse del kit contro risorse dell'SDK). `tenant` è un nome esterno che il motore traduce
  nell'identificativo interno del tenant (AD-5); `default` è riservato al tenant predefinito. Il
  motore conosce solo la chiave pubblica.

## API

- **C-16 Forma dell'API.** Campi JSON in camelCase. Percorsi `/v1/…`. Ogni risorsa si identifica
  con un UUIDv7 (C-1). La posizione del bus è un token opaco (C-8, AD-18): sta nel campo
  `position` delle risposte e nell'header `Last-Event-ID`.
- **C-17 Liste.** Paginazione a cursore con `cursor` e `limit`; filtri come parametri di query.
- **C-18 Idempotenza.** La chiave di idempotenza (FR-6) viaggia nell'header `Idempotency-Key`.

## Configurazione

- **C-19 Formato della configurazione (superficie pubblica, PRD §6).** Un file TOML facoltativo,
  indicato da `TOPICTOAST_CONFIG`; senza file valgono i predefiniti e l'esempio gira senza
  modifiche (FR-50). Chiavi in snake_case inglese, raggruppate in sezioni (nomi esatti nelle spec
  delle storie, secondo `docs/glossary.md`).
  - **Variabili:** ogni chiave si può dare come `TOPICTOAST_<SEZIONE>__<CHIAVE>` in maiuscolo, con
    il doppio trattino basso tra sezione e chiave; la variabile vince sul file. Sono configurazione
    solo queste: le altre `TOPICTOAST_*`, senza doppio trattino basso, sono dei client (CLI, SDK,
    worker) e il motore le ignora.
  - **Validazione all'avvio:** chiavi sconosciute, nel file o tra le variabili di configurazione, e
    valori fuori tipo sono rifiutati; i predefiniti si applicano dopo la validazione (C-10) e ogni
    valore sta entro il massimo dell'installazione (NFR-14). Con una configurazione non valida il
    motore non parte: scrive una riga C-7 su stdout (C-9) con il percorso della chiave e la regola,
    mai il valore né il messaggio del parser così com'è, ed esce con un codice diverso da zero.
  - **Segreti:** le chiavi segrete sono un elenco chiuso, marcato nel tipo della configurazione:
    chiave principale, URL e password del database, credenziali del bus, credenziali di RustFS,
    chiavi private TLS. Nel file compaiono solo come `<chiave>_file`, il percorso di un file;
    il valore arriva da quel file o da una variabile. Un valore segreto scritto nel file fa
    rifiutare l'avvio.
  - **Processi ausiliari:** il motore passa a ogni ausiliario (AD-6) solo le chiavi che servono al
    suo ruolo, mai un segreto.
  - **Cambi:** nessuna ricarica a caldo; ogni cambio richiede un riavvio, entro NFR-15. Rinominare
    o togliere una chiave è una rottura di contratto.
