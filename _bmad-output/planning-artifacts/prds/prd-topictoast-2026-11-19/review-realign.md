---
title: Revisione del riallineamento del PRD (2026-11-23)
created: 2026-11-23
oggetto: git diff main su prd.md, addendum.md, epics.md, CLAUDE.md (branch docs/prd-realign)
fonti di confronto: ARCHITECTURE-SPINE.md e conventions.md (architecture-topictoast-2026-11-20), .memlog.md del PRD
---

# Revisione del riallineamento del PRD

**Verdetto:** il riallineamento è corretto nella sostanza: AD-1, AD-8, AD-9, AD-10, AD-13 sono
citate bene, il bus resta aperto e le 19 domande di §11 coincidono con la tabella delle epiche. Va
corretto prima del merge su tre fronti. (1) La storia 2.11 introduce dipendenze in avanti: il
canale SSE nasce nell'Epic 3, i tenant multipli nell'Epic 4, i nodi nell'Epic 10. (2) §11 n. 8 e
l'addendum chiudono più di quanto chiuda la spine. (3) Il segnaposto dei segreti di FR-59
contraddice FR-33 e NFR-8, chiude T-67 senza dirlo e vincola il trasporto di S-1.

Gravità: **A** alta (da correggere prima del merge), **M** media, **B** bassa.

## Rilievi

### 1. [A] Storia 2.11: il canale in tempo reale non esiste ancora nell'Epic 2

- **Dove:** `epics.md:1933` (criterio SSE), `epics.md:1915` (prova per l'utente),
  `epics.md:1010` (Epic 2 «Bloccata da»), `epics.md:541` e `epics.md:1140` (AR-99 assegnata
  all'Epic 3).
- **Problema:** nessuna storia delle epiche 1 e 2 costruisce lo stream SSE (C-8): AD-18 compare
  solo per le letture dopo una scrittura (1.6). Il primo canale in tempo reale è la storia 3.5, e
  AR-99 (granularità degli eventi e battito, «prima della prima storia del tempo reale») è
  assegnata all'Epic 3. La 2.11 diventa così la prima storia in tempo reale senza che AR-99 la
  blocchi: è una dipendenza in avanti. Il criterio dice anche «il canale riprende dalla posizione
  (C-8)». Questo presuppone che le righe abbiano una posizione del bus, cioè che viaggino sul bus:
  è proprio il trasporto che S-1 deve ancora misurare (spine L433-434). Se le righe vanno
  direttamente nell'archivio dei payload, AD-18 («eventi SSE subito dopo la scrittura durevole
  sul bus») non le copre, e il token di C-8 non vale per loro.
- **Correzione:** togliere dalla 2.11 il criterio SSE e la frase della prova sul canale; leggere
  il log solo via API. Spostare «righe in tempo reale entro 1 s» nella 3.5, che nasce con il
  canale. Per la ripresa scrivere «riprende senza perdere righe, nel modo fissato da S-1» invece
  di «dalla posizione (C-8)». In alternativa: aggiungere AR-99 e il trasporto di S-1 a «Bloccata
  da» dell'Epic 2 e assegnare AR-99 anche all'Epic 2 in `epics.md:1140`.

### 2. [A] Storia 6.5: righe dei nodi prima che esistano i nodi

- **Dove:** `epics.md:2815`.
- **Problema:** «le righe del worker e dei suoi nodi» nell'Epic 6: i nodi personalizzati arrivano
  con l'Epic 10 (storia 10.2). Il criterio non si può verificare quando la storia si chiude. In più
  «suoi» è ambiguo: i nodi del worker o del tenant?
- **Correzione:** «vede in tempo reale gli eventi del motore e le righe del worker, come testo
  (NFR-7)». Nella 10.2 aggiungere che le righe dei nodi compaiono nella tab Log del kit e della
  dashboard già costruite (storie 3.5 e 6.5).

### 3. [A] Storia 2.11: isolamento tra tenant provato sull'adattatore sbagliato e con tenant che non esistono ancora

- **Dove:** `epics.md:1945-1947`, `epics.md:1943`.
- **Problema:**
  - L'adattatore del repository di AD-5 è l'unico accesso al **database**. Le righe del log
    «non passano dal database» (FR-59, criterio a `epics.md:1937`), quindi il test a due tenant
    di quell'adattatore non tocca mai la lettura dei log. L'isolamento delle righe sta
    nell'indirizzo dell'archivio dei payload, con l'identificativo interno del tenant (AD-3,
    AD-17), e nel filtro unico di risposte ed eventi (AD-18).
  - «non porta le attività degli altri tenant oltre NFR-2» richiede più tenant con attività.
    L'Epic 2 non li ha: le storie sorelle dicono «le altre esecuzioni» (`epics.md:1616`) o «il
    worker sano», e le quote per tenant arrivano con l'Epic 4 (`epics.md:1692`).
- **Correzione:**
  - Scrivere «Dato il test a due tenant dell'adattatore dell'archivio dei payload e della risorsa
    API del log (AD-5, AD-17)».
  - Nell'Epic 2 usare «le altre esecuzioni»; il caso «altri tenant» va nella storia delle quote
    dell'Epic 4 (FR-30, NFR-2) o nella corsa a carico misto dell'Epic 13 (rilievo 9).

### 4. [A] §11 n. 8 «Chiusa» chiude più della spine

- **Dove:** `prd.md:1584-1587` (§11 n. 8), `prd.md:356-359` (FR-2), `prd.md:299-300`
  (glossario), `epics.md:606` (riga n. 8); spine L801.
- **Problema:**
  - La spine lascia aperta la politica d'errore del fan-out «il join riceve l'errore» **rispetto
    ad AD-9**: l'elemento fallito conta come dipendenza fallita? rende fallita l'esecuzione? Il
    proprietario è «architettura; Marco se tocca AD-9».
  - FR-3 (`prd.md:375-378`) lo dice bene. §11 n. 8 però dà la regola di propagazione per
    «Chiusa» e cita come residuo solo «join dei dati e sintassi del fan-out»; tralascia la
    politica d'errore e il `$context` concorrente. FR-2 e il glossario enunciano AD-9 senza
    riserve e aggiungono «nessun task può dichiarare un'eccezione», mentre la seconda politica di
    FR-3 è proprio un'eccezione dichiarata.
  - In più, «`then` e `switch` stanno in un `do` annidato, fuori dal `dag`» è sbagliato: secondo
    AD-8 il `do` annidato sta **dentro** il `dag` e conta come un solo nodo.
- **Correzione:**
  - §11 n. 8: «**Chiusa in parte:** la regola è in FR-2 (AD-9); `then`, `exit`, `end` e `switch`
    stanno in un `do` annidato dentro il `dag`, che conta come un solo task (AD-8). Aperti, fino
    all'aggiornamento della spine prima della prima storia del `dag`: join dei dati, `$context`
    concorrente, sintassi del fan-out e incrocio della politica "il join riceve l'errore" con la
    regola (Marco, se cambia AD-9).»
  - Allineare la riga n. 8 di `epics.md:606`.
  - FR-2: chiudere il punto con «salvo quanto fisserà FR-3 per il fan-out».
  - FR-3: aggiungere «con Marco, se cambia la regola di FR-2».

### 5. [A] Addendum: chiusura in blocco di tutte le scelte rimandate

- **Dove:** `addendum.md:143-144`.
- **Problema:** «Le scelte rimandate all'architettura nelle sezioni sopra sono chiuse dalla spine
  accettata» non è vero. Diverse voci delle sezioni sopra la spine le ha riassegnate o le tiene
  aperte:
  - forma degli schemi del manifest dai tipi TS e Rust (`addendum.md:93-95`): Deferred M-07,
    alle spec degli SDK;
  - scadenza della credenziale e controlli del kit (`addendum.md:99-104`): Deferred M-45…M-47,
    sezione contratti del kit;
  - come si salvano le pianificazioni (`addendum.md:105-110`): M-04 aperta;
  - connessioni del sotto-workflow (`addendum.md:129-131`): M-37 aperta, alla spec;
  - processi ausiliari a freddo (`addendum.md:41-47`): S-2;
  - iroh (`addendum.md:48-`): S-1.

  Chi legge l'addendum conclude che non resta niente di aperto.
- **Correzione:** «Le scelte rimandate all'architettura nelle sezioni sopra sono decise dalla
  spine accettata o riassegnate da lei (tabelle Deferred e Open Questions, spike S-1…S-4); in caso
  di dubbio vale la spine.»

### 6. [A] Segnaposto dei segreti: contraddice FR-33 e NFR-8, chiude T-67 in silenzio e vincola S-1

- **Dove:** `prd.md:549-551` (FR-59), `prd.md:999` (FR-33), `prd.md:1365` (NFR-8),
  `addendum.md:155-158`, `epics.md:562` e `epics.md:1158` (AR-118, T-67), spine L787 (T-67),
  C-12.
- **Problema:**
  - FR-33 dice che i segreti non compaiono «nei log … di topictoast», e NFR-8 dice «mai nei
    log». Il log del task è un log che topictoast conserva e mostra. FR-59 maschera solo il
    valore esatto e ammette che un valore trasformato resti visibile: FR-33 e NFR-8 non sono
    riformulati di conseguenza.
  - La spine rinvia T-67 («segreti nelle righe di log del codice dei tenant») alla spec della
    consegna dei segreti e dei log. FR-59 decide il comportamento ma non lo dice, e AR-118 cita
    ancora T-67 come rinviata.
  - Il `.memlog.md` registra come decisioni di Marco solo il perimetro e la visibilità per il
    tenant; il segnaposto è un'aggiunta di chi ha scritto il testo.
  - Il vincolo tecnico non è scritto da nessuna parte. C-12 vuole i segreti «mai nei messaggi del
    bus», e una riga non mascherata con il segreto dentro, se viaggiasse sul bus (variante (1) di
    S-1, o righe trasportate come eventi), lo violerebbe. Il mascheramento deve quindi avvenire
    prima del bus, nell'SDK o nell'host della sandbox, oppure le righe non devono passare dal bus.
- **Correzione:**
  - In FR-33 e NFR-8 scrivere «nei log del motore (FR-52); nei log dei task vale FR-59».
  - In FR-59 aggiungere «(chiude il lato di prodotto di T-67)» e farlo confermare a Marco, oppure
    marcarlo `[ASSUMPTION]`.
  - Nell'addendum aggiungere: «il mascheramento avviene prima che la riga entri nel bus o
    nell'archivio (C-12); S-1 sceglie il trasporto di conseguenza».
  - In AR-118 annotare che il lato di prodotto di T-67 è chiuso da FR-59.

### 7. [M] «Scaduta» come evento di un tentativo contraddice il glossario

- **Dove:** `prd.md:535-536` (FR-59), `epics.md:120` (inventario), `epics.md:1927-1929`
  (storia 2.11); glossario `prd.md:296-302`, FR-48 `prd.md:1234-1235`.
- **Problema:** il glossario dice «Il timeout è un motivo, non uno stato. "Scaduta" vale solo
  per un'esecuzione». Gli stati dell'attività e le fasi di FR-48 non hanno «scaduta». FR-59 la
  elenca tra gli eventi del tentativo, e la 2.11 parla di «un task … scaduto». Anche «ricevuta dal
  worker» non vale per i tentativi di un nodo, che girano nella sandbox.
- **Correzione:** «ricevuta (dal worker o dalla sandbox), in esecuzione, completata, fallita con
  il motivo (timeout compreso), ritentata con l'attesa, annullata». Nella 2.11: «Dato un task
  ritentato, fallito per timeout o annullato», con uno scenario per caso. L'elenco attuale mette
  insieme «completata» e «annullata» nello stesso log, e così non si può verificare.

### 8. [M] Impegni di FR-59 che Marco non ha deciso

- **Dove:** `prd.md:535-556`; `.memlog.md` (decisioni del 2026-11-23).
- **Problema:** Marco ha deciso tre cose: perimetro, contenuto (eventi del motore, righe dei
  worker e dei nodi) e visibilità per il tenant. Sono state aggiunte senza attribuzione:
  - il segnaposto dei segreti (rilievo 6);
  - il tetto per tentativo, con lo scarto e il conteggio;
  - i tre livelli;
  - l'ora al millesimo;
  - 1 s anche a tentativo in corso;
  - il caso nella corsa a carico misto.

  Sono scelte ragionevoli, ma sono impegni di prodotto nuovi in un PRD `final`.
- **Correzione:** farle confermare a Marco in un'unica voce del memlog, oppure marcare
  `[ASSUMPTION]` quelle non confermate e aggiungerle all'indice di §12.

### 9. [M] Corsa a carico misto: FR-53 e la storia 13.3 non sono aggiornati

- **Dove:** `prd.md:554-556` (FR-59), `prd.md:1316-1318` (FR-53), `epics.md:4092-4094`
  (storia 13.3), `epics.md:955-961` (mappa di copertura).
- **Problema:** FR-59 dice che il caso «entra nella corsa a carico misto di FR-53», ma FR-53
  elenca ancora solo la pipeline stream, il tenant oltre quota e gli schemi complessi. La 13.3
  non ha il caso, e la mappa di copertura non ha una voce per l'Epic 13. È una lacuna di
  copertura di FR-59.
- **Correzione:** in FR-53 aggiungere «e un'attività scrive righe di log fino al tetto (FR-59)».
  Aggiungere lo stesso alla 13.3, e nella mappa la voce «FR-59: Epic 13 - log fino al tetto nella
  corsa a carico misto».

### 10. [M] Tetto del log contro NFR-14 e quote di spazio

- **Dove:** `prd.md:547-548` (FR-59), `prd.md:1402-1407` (NFR-14, terzo punto),
  `prd.md:940-942` (FR-30).
- **Problema:**
  - NFR-14 dice che superare il limite di una grandezza controllata dal tenant «è un errore di
    validazione o fa fallire il task». Il volume di righe di un nodo è una grandezza del tenant,
    ma FR-59 scarta le righe e le conta: le due regole si contraddicono alla lettera.
  - FR-30 elenca lo spazio occupato «(stream, storico, code, nodi caricati)» e non dice se i log
    contano. Un tenant con molti tentativi resta sotto il tetto per tentativo e riempie comunque
    il disco.
  - Non è detto se il tetto conta anche gli eventi del motore.
- **Correzione:**
  - In NFR-14: «…o fa fallire il task con limite e valore; le righe del log oltre il tetto si
    scartano e si contano (FR-59)».
  - In FR-59: «Le righe contano nello spazio occupato del tenant (FR-30); il tetto vale solo per
    le righe scritte dal codice, mai per gli eventi del motore».

### 11. [M] Ruolo tenant: identificativi dei worker nel log

- **Dove:** `prd.md:1124` (FR-41), `prd.md:535` (evento «ricevuta dal worker»),
  `epics.md:2813-2816` (storia 6.5).
- **Problema:** FR-41 vieta gli identificativi dei worker con il ruolo tenant, e AD-18 nasconde i
  campi per ruolo «in un solo punto». FR-59 e la 6.5 non dicono che gli eventi del motore nel log
  seguono la stessa regola.
- **Correzione:** in FR-59 aggiungere «Con il ruolo tenant gli eventi del log non mostrano
  identificativi dei worker (FR-41)». Nella 6.5 aggiungere «E gli eventi del motore non mostrano
  identificativi dei worker».

### 12. [M] Storia 2.11: manca il legame delle righe con il tentativo e la sessione (C-2)

- **Dove:** `epics.md:1913` («Rispetta»), criteri `epics.md:1920-1951`.
- **Problema:** C-2 dice che le scritture di un tentativo si accettano solo dalla sessione del
  worker che lo ha ricevuto, e i risultati di un tentativo non più corrente sono rifiutati. Una
  riga è una scrittura del tentativo. Senza un criterio, un worker con credenziale valida potrebbe
  scrivere nel log di un tentativo altrui, anche di un altro tenant.
- **Correzione:** aggiungere C-2 e C-11 a «Rispetta», e il criterio «Dato un tentativo non più
  corrente o una sessione diversa da quella che lo ha ricevuto, Quando arriva una riga, Allora è
  rifiutata e registrata». Aggiungere NFR-8 ai requisiti.

### 13. [M] Storia 2.11 troppo grande per un solo agente

- **Dove:** `epics.md:1907-1951`.
- **Problema:** una sola storia contiene:
  - l'API di log dell'SDK Rust;
  - il messaggio del protocollo dei worker, che dipende da S-1;
  - gli eventi del motore per tentativo;
  - il salvataggio a righe aggiunte nell'archivio dei payload, cifrato, quando AD-17 ha solo uno
    spazio immutabile per impronta e uno con nome sovrascrivibile;
  - la risorsa API con i tipi OpenAPI;
  - l'SSE con la ripresa;
  - tetto e conteggio;
  - due misure del benchmark (NFR-2, NFR-4);
  - i test a due tenant e il controllo C-9.

  Sono almeno due storie.
- **Correzione:**
  - Dividere in **2.11 «Log del tentativo: eventi del motore e lettura via API»** (eventi,
    archivio, API, isolamento).
  - Poi **2.12 «Righe del worker Rust»** (SDK, protocollo secondo S-1, tetto, C-2, NFR-2 e NFR-4,
    C-9).
  - Il tempo reale va nella 3.5 (rilievo 1).
  - Aggiornare il conteggio delle storie in `CLAUDE.md` (91).

### 14. [M] FR-59: da dove si misura 1 s e con quale orologio

- **Dove:** `prd.md:538-541`, `epics.md:1925-1926`.
- **Problema:**
  - AD-18 misura l'1 s di FR-41 dalla scrittura durevole sul bus all'evento consegnato. Per una
    riga del worker l'inizio non è definito (scrittura nell'SDK? arrivo al motore?), e l'SDK di
    solito raggruppa le righe.
  - «Ora al millesimo» e «in ordine di tempo» non dicono di chi è l'orologio. I worker possono
    stare su altri host (AD-1), quindi mettere in un solo ordine le righe dei worker e gli eventi
    del motore non è ben definito.
- **Correzione:** in FR-59 scrivere «entro 1 secondo da quando il codice scrive la riga». Nella
  2.11 rimandare alla spec chi fissa l'ora della riga e l'ordine (per esempio l'arrivo al motore,
  con l'ora del worker come campo in più).

### 15. [M] FR-55: elenco delle parole chiave di AD-10 senza `description`

- **Dove:** `prd.md:608-611` (FR-55), `prd.md:1608-1610` (§11 n. 16).
- **Problema:** AD-10 ammette `description`. FR-55 elenca le parole chiave ammesse senza
  `description` e aggiunge «ogni altra parola chiave è rifiutata»: letto alla lettera, il PRD
  rifiuta le descrizioni dei campi, che però NFR-7 cita («nomi e descrizioni dei campi degli
  schemi»). Inoltre §11 n. 16 cita «(AD-10, FR-55)», ma le risposte alle sotto-domande (intero e
  decimale, campi in più, `null` e campo assente, predefiniti) stanno in C-10, che la tabella
  delle epiche cita.
- **Correzione:** in FR-55 aggiungere «descrizioni» all'elenco. In §11 n. 16 citare «(AD-10,
  C-10, FR-55)».

### 16. [M] FR-45: riferimento sbagliato e punto doppio

- **Dove:** `prd.md:1206-1209`.
- **Problema:**
  - «Dove viene scritta lo fissa l'architettura prima dello sprint 1 (C-14)»: C-14 fissa la
    regola («mai su stdout né nei log»). Dove si scrivono le credenziali è una Open Question della
    spine (primo avvio, L809; AR-96 nelle epiche).
  - «Output del processo» è più largo di «stdout», quindi comprende anche stderr: va bene se è
    voluto, ma va detto.
  - Il punto seguente, «non compare mai nei log (FR-52)», ora ripete la frase appena aggiunta.
- **Correzione:** «La credenziale non compare mai su stdout, su stderr né nei log (C-14, FR-52);
  dove viene scritta lo fissa l'architettura prima dello sprint 1 (domanda sul primo avvio)».
  Cancellare il punto doppio a `prd.md:1209`.

### 17. [M] Meccanismo nel PRD (criterio: §6, superfici pubbliche)

- **Dove:** `prd.md:716-718` (§4.5: «Il SQL gira su DataFusion; … li costruisce topictoast»),
  `prd.md:1540-1542` (rischio 2), `prd.md:1262-1264` (FR-50: «separati dal motore»),
  `prd.md:545-546` (FR-59: «non passano dal database»), `prd.md:340`, `378`, `1587`, `1597`,
  `1601` («aggiornamento della spine»).
- **Problema:**
  - Formato dei workflow, linguaggio degli schemi e dialetto SQL sono contratti pubblici (§6):
    `dag`, `input.schema` e il sottoinsieme di JSON Schema stanno bene nel PRD.
  - Il motore SQL interno, chi costruisce stato e finestre, la separazione dei processi e il
    «non passano dal database» sono il come. L'addendum li ha già (`addendum.md:146-158`).
  - FR-50 dice «separati dal motore»: AD-6 lo regge, ma quali processi separare e come
    raggrupparli lo decide S-2 (spine L50, L487-489).
  - «Spine» è gergo del processo: il lettore del PRD non sa cosa sia.
- **Correzione:**
  - §4.5: «Le pipeline stream si scrivono in un dialetto SQL chiuso (§6); motore e costruzione
    dello stato li fissa l'architettura (AD-13), e lo spike delle pipeline stream li verifica.»
  - FR-50: «…separati dal motore (AD-6); quali e come raggrupparli lo fissa lo spike della
    sandbox».
  - FR-59: «Il log non aggiunge carico al database: NFR-4 vale con il log attivo».
    La formulazione attuale, «non contano nelle istruzioni SQL», si legge come un'esenzione dal
    conteggio.
  - Nel PRD sostituire «aggiornamento della spine» con «aggiornamento dell'architettura».

### 18. [B] Rischio 3: fatti di Iggy attribuiti al bus in generale

- **Dove:** `prd.md:1543-1547`; anche `prd.md:1491-1493` (§8.2, cluster «oggi non è ancora
  maturo», riga non toccata).
- **Problema:** «Il bus è un log, non una coda» e «il bus è pre-1.0 e cambia formato su disco»
  valgono per Iggy (brief, `brief.md:174`). Il bus però non è ancora scelto, e Valkey 9.1.4 non è
  pre-1.0.
- **Correzione:** «**Il bus candidato Iggy è un log, non una coda** … Iggy è pre-1.0 e cambia
  formato su disco … Lo spike del bus sceglie tra Iggy e Valkey e misura questi punti per
  entrambi.» Allo stesso modo, in §8.2: «il cluster del bus candidato».

### 19. [B] §11 n. 14: una richiesta della revisione diventa una regola

- **Dove:** `prd.md:1603-1605`; fonte `addendum.md:135-136`.
- **Problema:** «la scelta non è mai implicita» è una richiesta della revisione di sicurezza
  («chiede che…»). Nessun FR la contiene, e la spine la lascia alla spec (M-37). Scritta così in
  §11, sembra una regola decisa.
- **Correzione:** «**Assegnata:** spec della storia delle connessioni; la revisione di sicurezza
  chiede che la scelta non sia mai implicita». In alternativa: portarla in FR-29 con la conferma
  di Marco.

### 20. [B] Riferimenti superati o incoerenti nelle epiche

- **Dove e problema:**
  - `epics.md:438` (AR-9): «trasporto log dei task se nel perimetro». Ora sono nel perimetro.
  - `epics.md:987`: la riga AR-92 elenca le epiche «2, 3, 5, 6, 8, 10», mentre `epics.md:1135`
    elenca anche l'Epic 4, che ha i criteri 4.7 e 4.8.
  - `epics.md:979` (riga S-1): non cita il trasporto delle righe di log, che `CLAUDE.md` invece
    aggiunge, e tra le epiche da ricontrollare mancano la 5, la 6 e la 10.
  - `epics.md:1010`: tra i blocchi dell'Epic 2 manca il trasporto di S-1, e AR-99 se resta il
    tempo reale nella 2.11.
  - La spine dice ancora «se entrano nel perimetro» (L433-434), e la sua Open Question sui log
    (L805) è ancora aperta.
- **Correzione:**
  - AR-9: «trasporto delle righe di log dei worker (FR-59)».
  - Riga AR-92: «2, 3, 4, 5, 6, 8, 10».
  - Riga S-1: aggiungere il trasporto delle righe di FR-59 e le epiche 5, 6, 10.
  - Annotare come seguito un Update della spine, con l'intento `bmad-architecture` Update, che
    chiuda la riga L805 e tolga «se entrano nel perimetro».

### 21. [B] Livelli del log diversi tra PRD e UX

- **Dove:** `prd.md:538-539`, `epics.md:753` (UX-DR-33).
- **Problema:** FR-59 ha tre livelli (info, avviso, errore), UX-DR-33 ne ha quattro: c'è anche
  «ok», probabilmente per gli eventi del motore.
- **Correzione:** dire in FR-59 che «ok» è solo un livello di presentazione degli eventi del
  motore, oppure toglierlo dalla UX nell'Update `bmad-ux` già previsto (AR-7).

### 22. [B] Allineamenti minori con FR-31 e NFR-7

- **Dove:** `prd.md:962` (FR-31), `prd.md:1358-1361` (NFR-7).
- **Problema:** FR-31 elimina le esecuzioni vecchie «con i loro input e output» e non nomina i
  log, che invece FR-59 e la 4.7 includono. L'elenco di NFR-7 dei contenuti mostrati come testo
  non contiene le righe dei log; FR-59 le definisce contenuto del tenant o dell'integratore, e il
  test XSS di NFR-7 copre solo «campi testuali di un tenant».
- **Correzione:** FR-31: «…con i loro input, output e log dei task (FR-59)». NFR-7: aggiungere
  «righe dei log dei task, anche quelle dei worker».

### 23. [B] Prosa nelle righe cambiate

- `prd.md:252-253` e `prd.md:530-531`: «le righe scritte dal codice che lo esegue, la funzione
  del worker o il nodo personalizzato» si legge come un elenco di tre cose. Meglio «dal codice che
  lo esegue (la funzione del worker o il nodo personalizzato)».
- `prd.md:352-353` (FR-2): «fuori dal `dag`, `for` e `then` mantengono la semantica di OWS» fa
  pensare che dentro il `dag` `for` cambi significato, ma AD-8 vieta solo `then`, `exit`, `end`
  e `switch`. Meglio «`for` e `then` mantengono la semantica di OWS; `then` non si usa dentro il
  `dag` (punto seguente)».
- `prd.md:1272-1273` (FR-50): «la verifica lo spike della sandbox» si può leggere come «la
  verifica [nome]». Meglio «lo spike della sandbox la verifica e fissa anche…».
- `prd.md:1241` (FR-48): «lo decide la registrazione» non è chiaro. Meglio «lo fissa la spec
  della storia di registrazione (§11 n. 10)».
- `prd.md:1548-1549` (rischio 4): l'accordo «Chiuso» va con «Forma», che è femminile. «Come dice
  già §7» esagera §7, che dice solo «non garantisce che girino su altri runtime OWS». Meglio
  «Chiusa dall'architettura … i validatori ufficiali di OWS lo rifiutano (AD-8); §7 non promette
  che giri su altri runtime». Lo stesso vale per il rimando «(§7)» di `prd.md:318`.
- `prd.md:1253` (§4.11): in «Come fissato dall'architettura (AD-1), sono topictoast e il bus»
  manca il soggetto. Meglio «i componenti sono topictoast e il bus…».
- `prd.md:1566-1567` (§11 n. 1): «che fino ad allora usa UJ-3» lascia ambigui il riferimento e
  il momento. Meglio «Marco, prima o durante lo spike delle pipeline stream; fino al caso reale lo
  spike usa UJ-3 come assunzione».
- `epics.md:1915` (prova della 2.11): «tre log con la riga della funzione, il fallimento con il
  motivo» suggerisce un fallimento anche al terzo tentativo. Meglio «i primi due con il
  fallimento e "ritentata" con l'attesa, il terzo con "completata"».

## Confronto §11 del PRD con la tabella delle epiche (`epics.md:596-618`)

| n. | PRD (`prd.md:1565-1621`) | Tabella delle epiche | Esito |
| --- | --- | --- | --- |
| 1 | Aperta: Marco, prima o durante S-3; UJ-3 come assunzione | aperta; S-3 usa UJ-3; Marco | coincide |
| 2 | Aperta: ADR di S-3 | aperta; ADR di S-3 | coincide |
| 3 | Aperta: ADR di S-2 | aperta; ADR di S-2 | coincide |
| 4 | Chiusa (AD-17) | chiusa; AD-17 | coincide |
| 5 | Fuori dalla v1 | fuori dalla v1 | coincide |
| 6 | Chiusa in parte (AD-7); spec delle connessioni | «busta e chiavi decise (AD-7, C-12)», senza etichetta | stessa sostanza; etichetta diversa, C-12 solo nella tabella |
| 7 | Chiusa in parte: `input.schema` (AD-10); il resto da un aggiornamento della spine, proposta prima di S-1 | chiusa in parte; proposta prima di S-1, Update prima della prima storia che la tocca | coincide; il PRD non dice la scadenza «prima della prima storia che la tocca» |
| 8 | Chiusa | chiusa | coincidono, ma **chiudono troppo entrambi** (rilievo 4) |
| 9 | Aperta: ADR di S-2 | aperta (M-28); ADR di S-2 | coincide |
| 10 | Assegnata: registrazione, spec della storia di registrazione | assegnata (M-08); stessa spec | coincide |
| 11 | Assegnata: spec delle versioni | assegnata (M-09); stessa | coincide |
| 12 | Chiusa in parte: forma nel documento all'aggiornamento della spine | chiusa in parte; M-04 | coincide |
| 13 | Chiusa | chiusa | coincide |
| 14 | Assegnata: spec delle connessioni; «mai implicita» | assegnata (M-37) | coincide; «mai implicita» solo nel PRD (rilievo 19) |
| 15 | Assegnata: spec di FR-8 | assegnata (M-19) | coincide |
| 16 | Chiusa (AD-10, FR-55); compatibilità nelle spec delle versioni | chiusa; AD-10, C-10 | coincide; il PRD non cita C-10 (rilievo 15) |
| 17 | Assegnata: spec delle pianificazioni | assegnata | coincide |
| 18 | Assegnata: spec delle pianificazioni | assegnata | coincide |
| 19 | Assegnata: spec delle quote | assegnata (M-23) | coincide |

Il conteggio del memlog (chiuse 4, 8, 13, 16; in parte 6, 7, 12; aperte 1, 2, 3, 9; assegnate 10,
11, 14, 15, 17, 18, 19; fuori 5) torna. Con la correzione del rilievo 4, la n. 8 passa a «in parte».

## Cosa è corretto

- AD-1 (bus servizio separato, Iggy o Valkey da S-1), AD-9 (testo della regola), AD-10 (salvo
  `description`) e AD-13 sono citate fedelmente; nessun punto dà per scelto il bus o la forma di
  M-02 e M-04.
- FR-3 lascia aperto in modo esplicito l'incrocio della politica d'errore con AD-9.
- Gli obiettivi degli spike (NFR-5, NFR-12, riavvio a freddo) restano `[ASSUMPTION]`, con lo spike
  che li chiude.
- La numerazione delle storie è corretta: la 2.11 è l'ultima dell'Epic 2 e il totale è 90, come in
  `CLAUDE.md`.
- La copertura di FR-59 nelle epiche 2, 3, 4, 5, 6, 8, 10 è coerente con le liste «FRs
  covered». Mancano l'Epic 13 (rilievo 9) e il caso del ruolo tenant (rilievo 11).
- La lettura dei log dagli SDK è coperta dal test di parità della storia 5.3
  (`epics.md:2603-2606`), che impone un metodo per ogni operazione dell'API.
