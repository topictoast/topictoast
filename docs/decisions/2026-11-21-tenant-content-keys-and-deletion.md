# ADR: chiave nel motore, contenuto dei tenant fuori, eliminazione crittografica

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-6, AD-7 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-13, FR-16, FR-17, FR-25, FR-31, FR-32, FR-33, FR-34, NFR-8; addendum del PRD
  («Meccanismi suggeriti dalla revisione»); `extract-prd.md` (M-29, M-38, M-39, AMB-11);
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 35, 42, 48, 50, 51,
  53, 56; `conventions.md` (C-12, C-13); `review-decisions.md` (D-1, E-1, E-3); revisioni
  `review-security.md` (SEC-C1, SEC-A1, SEC-A2, SEC-A5, SEC-A11, SEC-M2), `review-rubric.md`

## Contesto

topictoast tiene i segreti di più tenant ed esegue contenuto scritto dai tenant.

- Sandbox e byte dei nodi stanno fuori dal processo che custodisce la chiave della cassaforte, e
  nessuna istanza contiene dati di due tenant (FR-16), con limiti di risorse (FR-17).
- Il PRD non dice quale processo custodisce la chiave (AMB-11).
- Eliminare un tenant rende i suoi dati illeggibili su ogni supporto, verificato con un valore
  sentinella (FR-31).
- La chiave della cassaforte si ruota nella v1 (FR-34).
- Le revisioni del PRD hanno suggerito tre meccanismi, come input e non come decisioni:
  - jq senza ambiente né file, oppure valutato fuori dal processo della chiave;
  - segreti risolti alla consegna e mai scritti sul bus;
  - per l'eliminazione, stream separati per tenant oppure cancellazione crittografica (M-39).

## Decisione

- **[AD-6](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)** (memlog,
  voce 35): il motore custodisce la chiave principale e non interpreta mai contenuto dei tenant.
  Documenti, manifest, schemi, jq, `call http` e SQL delle pipeline stream li interpretano processi
  ausiliari senza chiave, con limiti di CPU, memoria e tempo. Il motore riceve strutture tipizzate
  e passa all'ausiliario solo il segreto della singola chiamata. Chiude AMB-11. Dopo la verifica
  degli input e le revisioni la regola fissa anche:
  - unica eccezione: il processo delle pipeline stream di un tenant riceve la chiave di quel
    tenant (AD-13); le due sandbox e quel processo stanno fuori dal motore, e nessuna loro istanza
    contiene dati di due tenant, né insieme né in sequenza;
  - le funzioni che interpretano contenuto dei tenant (validatore, jq, lettura dei manifest) si
    chiamano solo tramite un permesso che esiste solo in modalità ausiliario, e un test prova che
    la modalità motore non le chiama mai;
  - il motore avvia ogni ausiliario con l'ambiente vuoto e gli passa sull'IPC la configurazione
    risolta e la versione dell'IPC; con una versione diversa l'ausiliario esce. Il motore toglie
    dal proprio ambiente la variabile della chiave principale dopo averla letta;
  - motore e ausiliari non sono ispezionabili (`PR_SET_DUMPABLE` a 0) e non scrivono core dump;
    chiavi e segreti stanno in tipi che si azzerano al rilascio, con `Debug` mascherato;
  - le risposte degli ausiliari sono input esterno (costituzione IV): messaggi tipizzati con
    tetti di dimensione e profondità, validati dal motore. I corpi delle richieste dell'API hanno
    tetti di dimensione, profondità e decompressione, applicati prima dell'analisi;
  - `call http`: nessun archivio di cookie né pool di connessioni condiviso tra tenant;
    `Authorization` non passa mai su un redirect verso un'altra origine.
- **AD-7** (memlog, voce 42; C-12 e C-13 approvate in blocco):
  - cassaforte a busta: chiave principale → chiave per tenant → segreti in AES-256-GCM;
    l'integratore ha una propria chiave, come un tenant (E-1);
  - segreti risolti alla consegna, mai nei messaggi del bus;
  - input, output ed eventi dei tenant cifrati con la chiave del tenant su bus, file e checkpoint;
  - formato dei dati cifrati (E-3): ogni oggetto cifrato porta una busta versionata con l'id della
    chiave; per ogni oggetto o segmento si deriva dalla chiave del tenant, con HKDF, la chiave che
    lo cifra in AES-256-GCM; la chiave del tenant ruota tramite l'id. Il formato è uno solo, nel
    dominio, e lo usano motore, proiettore, processo delle pipeline stream e archivio dei payload
    (C-12, C-13);
  - la chiave principale si genera solo con il deposito delle chiavi vuoto; il motore ne conserva
    l'impronta e, se la chiave presente non corrisponde, rifiuta di partire con un codice;
  - cartelle dei dati 0700 e file 0600; il motore non parte, con un codice, se la chiave
    principale o la cartella dei dati sono leggibili da altri utenti;
  - eliminare un tenant, in quest'ordine: fermare pipeline stream, esecuzioni, pianificazioni,
    sandbox riservate e stream SSE del tenant, e rifiutarne le richieste → scrivere sul bus
    l'evento lapide del tenant → cancellarne la chiave → cancellarne le righe del database tramite
    l'adattatore del repository e i nodi caricati → pulizia asincrona di bus e archivio dei
    payload;
  - la rilettura del bus salta solo gli eventi dei tenant con lapide; un evento che non si decifra
    senza lapide è un guasto che ferma proiettore e ripresa (AD-2), mai un'eliminazione;
  - cancellazione senza residui nel database: con SQLite `secure_delete` attivo e checkpoint del
    WAL (`TRUNCATE`) dopo l'eliminazione; con Postgres VACUUM delle tabelle toccate, con il
    residuo che resta documentato;
  - il test del valore sentinella copre database (file compresi), bus, archivio dei payload,
    checkpoint e nodi.

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:**
  - il contenuto ostile non arriva mai nel processo della chiave, e il permesso con il suo test
    rende il confine verificabile anche se motore e ausiliari sono lo stesso binario;
  - un ausiliario compromesso non manda al motore strutture senza tetti, e non eredita né
    l'ambiente né la chiave;
  - un tenant si rende illeggibile su ogni supporto cancellando una chiave, e la lapide distingue
    un tenant eliminato da un evento indecifrabile;
  - una chiave principale mancante o sbagliata ferma l'avvio invece di generarne una nuova;
  - la rotazione della chiave principale cifra di nuovo solo le chiavi dei tenant (FR-34);
  - con la chiave derivata per oggetto o segmento nessuna chiave raggiunge il limite dei nonce di
    AES-GCM, e l'id nella busta permette di ruotare la chiave del tenant senza rompere la
    rilettura; motore, proiettore, processo delle pipeline stream e archivio leggono lo stesso
    formato (E-3).
- **Costi:**
  - una chiamata tra processi sul percorso caldo (`if`, jq, controllo degli schemi), da far stare
    in NFR-2;
  - NFR-1 deve reggere con gli ausiliari attivi;
  - la cifratura per tenant su bus, file e checkpoint, con una derivazione HKDF per oggetto o
    segmento e una busta su ogni cifrato (E-3), il cui costo si misura in S-1;
  - le righe del database non si cancellano per via crittografica: le elimina l'adattatore del
    repository, con `secure_delete` e checkpoint del WAL in SQLite e VACUUM in Postgres; con
    Postgres un residuo resta, ed è documentato;
  - un evento indecifrabile senza lapide ferma proiettore e ripresa, quindi anche il lavoro nuovo
    (AD-2);
  - ogni ausiliario va confinato e provato da solo (S-2), e i tetti dell'IPC e dei corpi dell'API
    vanno fissati e mantenuti.
- **Facilità di manutenzione:** un solo processo tiene la chiave. Gli ausiliari sono sottocomandi
  dello stesso binario (AD-16), separati dal permesso della modalità ausiliario. Il confine tra
  processi è fatto di strutture tipizzate e versionate da mantenere. L'ordine dell'eliminazione va
  provato con un test.

## Alternative scartate

- **Cassaforte in un processo separato, interpretazione nel motore** (memlog, voce 35): i
  segreti decifrati finirebbero nella memoria del motore, e i limiti di memoria dentro il processo
  sono difficili.
- **Eliminazione con stream separati per tenant, senza cifrare i payload** (M-39): scartata con
  l'approvazione di C-13, che sceglie la cancellazione crittografica (memlog, voce 42). Motivo
  (memlog, voce 50): molti stream, e file e checkpoint da pulire a mano, perché i dati di un
  tenant stanno anche nell'archivio dei payload e nei checkpoint (AD-3, AD-17), non solo sul bus.

  Gli stream separati non sono scartati come struttura: gli eventi stream di ogni tenant stanno su
  topic propri (AD-13), e lo spazio per tenant sul bus resta aperto in S-1 (M-24). Sono scartati
  solo come meccanismo di eliminazione.
- **Chiave principale «generata al primo avvio se manca» ed eventi indecifrabili trattati come
  eliminati**, le regole precedenti di C-12 e AD-7 (`review-security.md` SEC-A11): il motore non
  distingue un primo avvio da una chiave che manca per un volume non montato, un percorso sbagliato
  o un container ricreato; ne genererebbe una nuova e le chiavi avvolte non si aprirebbero più.
  Trattare «indecifrabile» come «eliminato» farebbe saltare in silenzio gli eventi di tutti i
  tenant, perdendo lavoro accettato senza errore (NFR-6, FR-9), e lo stesso con un cifrato
  corrotto.
- **Dati dei tenant cifrati in AES-GCM direttamente con la chiave del tenant**, senza id della
  chiave né derivazione, la lettura possibile della regola precedente (`review-security.md`
  SEC-A6, `review-rubric.md`): con nonce casuali di 96 bit il limite raccomandato è di 2^32
  cifrature per chiave (NIST SP 800-38D), che un tenant a 100.000 eventi al secondo (NFR-5)
  supera in circa 12 ore; oltre quel limite un nonce ripetuto rivela i testi e permette di
  falsificare i messaggi. Senza id della chiave nel formato la rotazione non si aggiunge dopo
  senza rompere la rilettura (FR-51), e bus, archivio e checkpoint sceglierebbero ciascuno il
  proprio schema.
- **AEAD a nonce lungo (XChaCha20-Poly1305 o AES-GCM-SIV)** al posto della derivazione, proposto
  da `review-security.md` SEC-A6 e da `review-rubric.md`: nessun motivo registrato; Marco ha
  scelto HKDF con AES-256-GCM (E-3, memlog voce 63).
- **Per le altre correzioni dei revisori nessuna alternativa registrata nel memlog.** Le revisioni
  descrivono il rischio del silenzio: un ausiliario dello stesso utente che legge la chiave da file,
  database o `/proc` del motore (SEC-C1); un ausiliario compromesso che manda al motore strutture
  senza tetti (SEC-A1); cookie o `Authorization` che passano da un tenant all'altro in `call http`
  (SEC-A2); chiavi e segreti in core dump e messaggi di panic (SEC-M2); chiave avvolta e righe in
  chiaro nelle pagine libere e nel WAL dopo la cancellazione (SEC-A5); validazione chiamata dal
  motore perché è la strada più corta (`review-rubric.md`).

## Conseguenze e criteri di revisione

- **S-1** misura:
  - il costo della chiamata verso gli ausiliari entro NFR-2;
  - il costo della cifratura per tenant nel formato di C-12 (E-3);
  - per i worker diretti sui topic del bus, come arrivano i segreti (mai sul bus) e la
    decifratura con la chiave del tenant, con il giro in più dentro NFR-2.

  **D-1** (memlog, voce 53): la variante 1 è esclusa, qualunque siano i numeri, se dà ai worker
  chiavi dei tenant o segreti, se C-11 non si applica dal bus o se gli eventi di stato non li
  scrive solo il motore. AD-6 e AD-7 non si riaprono per la variante 1: cade la variante.
- **S-2** prova le sandbox con codice ostile (fuga, risorse, rete interna, lettura della chiave)
  e decide quali ausiliari separare. Fissa e prova inoltre:
  - come jq, `call http`, validazione e validazione a secco del SQL evitano di mescolare tenant
    (istanze per tenant o altro), misurato contro i riavvii a freddo di NFR-2 e contro NFR-1;
  - il confinamento con Landlock e seccomp di ogni ausiliario prima di toccare contenuto dei
    tenant, e il ripiego dove Landlock manca;
  - il test «lettura della chiave principale» per ogni sottocomando ausiliario: file della
    chiave, `/proc` del motore, file del database, configurazione, cartella di un altro tenant;
  - il tenant carica solo bytecode, che topictoast valida e compila nell'ausiliario; la cache del
    codice nativo la scrive solo topictoast.

  L'ADR della sandbox aggiorna AD-6.
- **S-3** misura la RAM di ogni processo per tenant contro NFR-1, e tiene spento il versamento su
  disco o lo mette in una cartella del tenant cifrata.
- **Criteri di revisione:** AD-6 si riapre se la chiamata tra processi non sta in NFR-2, o se il
  test «lettura della chiave principale» di S-2 non passa per un sottocomando ausiliario; AD-7 si
  riapre se la cifratura per tenant non sta in NFR-2 o NFR-3.
- **Rinvii della spine:** segreti nelle righe di log del codice dei tenant (T-67); dove vive il
  rinnovo dei token OAuth, dopo la v1.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63), applicato sopra:
  - E-3 nello schema di cifratura di AD-7, C-12 e C-13: busta versionata con l'id della chiave,
    chiave per oggetto o segmento derivata con HKDF, AES-256-GCM, rotazione della chiave del
    tenant tramite l'id, un solo formato nel dominio, costo misurato in S-1;
  - E-1 nei proprietari delle chiavi: identificativo interno del tenant, integratore come
    proprietario con una propria chiave.

  Restano da valutare nelle Open Questions della spine il rinnovo delle chiavi dei tenant alla
  rotazione della principale e i dati associati (AAD) (sicurezza M4).
