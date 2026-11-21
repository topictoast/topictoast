# ADR: verità sul log del bus, database come proiezione

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-2, AD-3, AD-18 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-6, FR-9, FR-11, FR-41, FR-48, NFR-2, NFR-4, NFR-6, NFR-15, SM-2; brief «Il
  problema»; [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 6, 19,
  20, 28, 48, 51, 53, 55, 56, 59; `review-decisions.md` (D-1, D-2, E-1, E-3, E-4, E-6); revisioni
  `reconcile-prd.md`, `reconcile-ux.md`, `reconcile-brief-rules.md`, `review-adversarial.md` (R2,
  R4, R5, R13), `review-security.md` (SEC-B1)

## Contesto

Il PRD mette le attività sul bus e solo lo stato nel database (memlog, voce 6). Il motivo è nel
brief: su Hatchet con Postgres come coda un utente ha misurato circa 7.700 istruzioni SQL per
task. Vincoli:

- «accettato» deve voler dire al sicuro (FR-9, NFR-6), con ripresa entro 30 s (NFR-15);
- avvio delle attività p50 ≤ 5 ms, p99 ≤ 25 ms (NFR-2);
- ≤ 5 istruzioni SQL per attività (NFR-4) e confronto con Hatchet (SM-2);
- payload fino a 1 MB tra task (FR-11);
- tempo reale entro 1 s (FR-41).

Restava da fissare dove sta la verità tra bus e database e in che ordine si riprende.

## Decisione

- **[AD-2](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)** (memlog,
  voce 19): la verità è il log del bus. «Accettato» = scrittura durevole sul bus prima della
  risposta. Il database è una proiezione scritta da un solo proiettore, indietro al massimo di 1 s.
  La ripresa riparte dall'ultima posizione del bus applicata nel database. Dopo la verifica degli
  input e le revisioni la regola fissa anche:
  - il bus non elimina mai posizioni dei topic di stato non ancora applicate; i topic degli stream
    seguono la conservazione di FR-18 e il proiettore non li legge;
  - con il database fermo o il disco pieno il motore rifiuta il lavoro nuovo con un codice (C-7) e
    avvisa l'operatore (FR-48);
  - un evento che il proiettore non riesce ad applicare è un guasto, non un dato: il proiettore si
    ferma su quella posizione senza saltarla, e il motore rifiuta il lavoro nuovo come sopra;
  - sul bus gli eventi di stato li scrive solo il motore, con ogni variante di collegamento di
    S-1: i messaggi dei worker sono richieste che il motore giudica;
  - un solo motore per installazione: all'avvio prende un lock esclusivo prima di scrivere su bus
    o database, e un secondo motore esce con un codice;
  - segreti e chiavi avvolte dei tenant non passano dal bus: li scrive il motore tramite
    l'adattatore del repository, prima della risposta.
- **AD-3** (memlog, voce 28): input e output dei task stanno fuori dal database. Viaggiano sul
  bus, e il proiettore scrive ogni payload nell'archivio dei payload (AD-17) quando lo applica,
  non solo per lo storico (E-4); il database tiene solo un riferimento tipizzato (impronta,
  dimensione). Le uscite a monte si leggono dall'archivio, quindi il bus può scartare le posizioni
  già applicate anche mentre un'esecuzione lunga è viva. L'indirizzo comprende l'identificativo
  interno del tenant, mai un nome del JWT (E-1); nessuna deduplicazione tra tenant. I payload
  nell'archivio sono cifrati con la chiave del tenant nel formato di C-12 (AD-7, E-3).
- **AD-18:** gli eventi SSE nascono dal motore subito dopo la scrittura durevole sul bus, mai
  dalla proiezione.
  - La posizione del bus è per i client un token opaco, che solo il dominio confronta; l'id
    dell'evento SSE è quel token, e una risposta con un'istantanea porta il token della sua
    posizione nel campo `position` (C-16, E-6).
  - Un `Last-Event-ID` fuori dalla conservazione riceve un errore C-7 che chiede di ricaricare
    l'istantanea.
  - Lettura dopo una scrittura: l'API aspetta fino a 1 s che la proiezione raggiunga il token;
    scaduta l'attesa risponde 503 con un codice C-7 e `Retry-After`, mai 404.
  - L'1 s di FR-41 si misura dalla scrittura durevole all'evento consegnato.
  - Eventi filtrati per tenant e ruolo in un solo punto, con la regola di AD-5 e lo stesso tipo di
    vista per ruolo delle risposte.
  - I controlli di idempotenza (FR-6), di C-3, di C-4 e di `If-Match` (C-5) li fa il motore su
    tutto lo stato accettato: la proiezione fino alla posizione applicata più il bus riletto da
    lì, mai la sola rilettura.
  - Le righe e gli avvii del processo delle pipeline stream passano dall'IPC del motore con un id
    deterministico (C-4).

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:** una sola verità. Il lavoro accettato non si perde se il database è in
  ritardo o fermo, né per un evento che il proiettore non sa applicare. La ripresa rilegge il bus.
  Il database porta solo stato tipizzato, i payload stanno fuori (NFR-4, SM-2). Gli eventi in
  tempo reale non aspettano la proiezione (FR-41), e un client non vede mai «Esecuzione non
  trovata» subito dopo «Esegui». Proiezione ed esiti delle attività non possono divergere, perché
  lo stato sul bus lo scrive solo il motore. Le uscite a monte restano leggibili dall'archivio
  anche quando il bus scarta le posizioni applicate durante un'esecuzione lunga (E-4).
- **Costi:**
  - il database può essere indietro fino a 1 s, quindi serve l'attesa del token dopo una
    scrittura; scaduta l'attesa il client riceve un 503 e riprova;
  - tre supporti da tenere coerenti: bus, database, archivio dei payload;
  - ogni payload si scrive nell'archivio appena il proiettore lo applica, non solo a esecuzione
    chiusa (E-4);
  - la conservazione del bus dipende dal proiettore; un evento non applicabile ferma il proiettore
    e il motore smette di accettare lavoro nuovo finché il guasto non si risolve;
  - i controlli di AD-18 leggono proiezione e rilettura insieme, con un costo da far stare in
    NFR-4 e SM-2 (S-1);
  - i segreti sono un'eccezione al percorso del bus;
  - backup e ripristino coerenti di database e bus restano fuori dalla v1.
- **Facilità di manutenzione:** un solo scrittore dello stato proiettato, un solo scrittore degli
  eventi di stato sul bus, un solo motore per installazione; la proiezione è idempotente e si può
  rigiocare. La regola «nessun'altra unità scrive lo stato proiettato» va difesa in ogni storia.
  Il token opaco lascia al dominio la forma della posizione, che S-1 fissa per partizione.

## Alternative scartate

- **Database prima, bus dopo** (memlog, voce 19): due scritture durevoli in fila, rischio sul p50
  di NFR-2, 2-4 istruzioni SQL per attività, vicino al limite di NFR-4.
- **Payload nel database come albero tipizzato** (memlog, voce 28): migliaia di righe per 1 MB,
  NFR-4 e SM-2 a rischio.
- **Uscite a monte lette dal bus finché l'esecuzione è viva, archivio solo per lo storico**, la
  regola precedente di AD-3 (`review-adversarial.md` R6): il bus può eliminare le posizioni già
  applicate (AD-2), quindi un task a valle che diventa pronto dopo un `wait` lungo, un fan-out
  lungo o un'attesa per quota trova cancellata l'uscita a monte, e i test brevi passano. La
  correzione minima proposta da R6, un bus che conserva anche le posizioni con input e output di
  esecuzioni vive (conservazione non più corta del timeout massimo di un'esecuzione, NFR-14),
  aveva un costo in disco da misurare in S-1; nessun altro motivo registrato. Marco ha scelto E-4
  (memlog, voce 63).
- **Per AD-18 nessuna alternativa registrata nel memlog.** Le revisioni (`reconcile-ux.md` R-1,
  `reconcile-prd.md`) hanno segnalato solo il buco. Il rischio dell'altra strada è nel Prevents di
  AD-18: eventi letti da una proiezione in ritardo, «Esecuzione non trovata» subito dopo
  «Esegui», filtri di tenant diversi tra risorse ed eventi, controlli fatti su uno stato vecchio.
- **Per le correzioni dei revisori nessuna alternativa registrata nel memlog.** Le revisioni
  descrivono il rischio della regola precedente o del silenzio:
  - evento non applicabile registrato e saltato (`review-adversarial.md` R4): l'esecuzione
    accettata gira sul bus ma nel database non esiste, e API e dashboard non la vedono mai;
  - risultati dei worker scritti sul bus come stato (`review-adversarial.md` R5, con la variante 1
    di S-1): proiezione e motore danno due esiti diversi alla stessa attività;
  - nessun lock all'avvio (`review-adversarial.md` R13): con due motori ci sono due proiettori, due
    pianificatori che controllano C-3 ciascuno per sé e due consegne della stessa attività;
  - controlli di AD-18 sul solo stato vivo del motore (`review-adversarial.md` R2): dopo un riavvio
    lo stato vivo contiene solo la rilettura, le chiavi d'idempotenza più vecchie mancano e un
    rilancio crea una seconda esecuzione;
  - posizione del bus esposta come id dell'evento (`review-security.md` SEC-B1): un tenant misura
    il traffico degli altri dai salti tra i propri eventi.

## Conseguenze e criteri di revisione

- **S-1** decide la tecnologia del bus: Iggy contro Valkey come livello caldo con scarico sul
  database. Con Valkey lo scarico porta i payload nell'archivio e nel database solo lo stato
  tipizzato. Misura:
  - scrittura durevole entro NFR-2;
  - NFR-3 e NFR-4 con SQLite e con Postgres, righe e WAL contati per attività;
  - NFR-6 con guasto iniettato, database fermo e disco pieno, lavoro nuovo rifiutato con codice;
  - NFR-15 con rilettura del bus e proiezione idempotente;
  - aggiornamento senza perdite (Iggy 0.10 non legge i dati della 0.9);
  - timer durevoli sul log, ricostruiti in ripresa, con un retry con attesa attraverso un crash
    iniettato.

  Da fissare e provare nell'ADR del bus, per candidato e variante: posizione codificata per
  partizione e salvata per partizione nella stessa transazione del gruppo, token che non rivela il
  volume degli altri tenant, ordine tra eventi; conservazione legata alla posizione del
  proiettore; database lento ma attivo, oltre 1 s; come i controlli di AD-18 raggiungono lo
  stato più vecchio della rilettura, dentro NFR-4 e SM-2; con SQLite, scritture del motore e gruppi
  del proiettore senza `SQLITE_BUSY` né chiavi esterne verso righe non proiettate; nessun effetto
  esterno prima della scrittura durevole della decisione che lo causa; consegna
  dell'annullamento (FR-10) e arresto per timeout.

  L'ADR del bus aggiorna AD-1, AD-2, AD-3, AD-17, AD-18, C-8 e il protocollo dei worker (AD-12).
- **D-1** (memlog, voce 53): la variante 1 di S-1 (worker diretti sul bus) è esclusa,
  qualunque siano i numeri, se non lascia al solo motore la scrittura degli eventi di stato, se dà
  chiavi dei tenant o segreti ai worker o se C-11 non si applica dal bus. La regola di AD-2 non si
  riapre per la variante 1: cade la variante.
- Se la scrittura durevole sul bus non sta in NFR-2, o la proiezione a gruppi non sta in NFR-4,
  AD-2 si riapre. AD-18 si rivede se i controlli su proiezione più rilettura non stanno in NFR-4.
- **Open Questions della spine:**
  - da chiudere in S-1: quali risorse oltre alle esecuzioni passano dal bus e dove si fanno gli
    altri controlli sullo stato più recente; il proiettore sotto RLS con Postgres; la versione del
    formato dei messaggi del bus;
  - da chiudere prima delle storie che le toccano: la fase operativa del motore durante la
    ripresa; la granularità degli eventi.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63), applicato sopra:
  - E-4 in AD-3: il proiettore scrive ogni payload nell'archivio quando lo applica, le uscite a
    monte si leggono da lì e il bus può scartare le posizioni applicate anche durante
    un'esecuzione lunga;
  - E-1 nell'indirizzo dei payload: identificativo interno del tenant, mai un nome del JWT;
  - E-3 nella cifratura di payload ed eventi: busta versionata con l'id della chiave, chiave
    derivata con HKDF, un solo formato (C-12), costo misurato in S-1;
  - E-6 nel trasporto del token di AD-18: campo `position` e header `Last-Event-ID` (C-16).
