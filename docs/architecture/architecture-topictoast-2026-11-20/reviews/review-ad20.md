# Revisione di AD-20 (chiamata di funzioni e nodi dal catalogo predefinito)

Oggetto: `git diff main`, cioè AD-20 della spine, ADR `2026-11-23-function-invocation`, note di FR-1 e §11 n.7 del PRD, epiche (AR-86, AR-102, AR-106, AR-110, AR-111). Fonti OWS lette: `schema/workflow.yaml` e `dsl.md` del tag v1.1.0 (copia in scratchpad).

Esito: la forma è corretta su OWS 1.1 e la direzione regge, ma la grammatica dei nomi, il momento in cui `latest` si lega e il contesto di risoluzione dei sotto-workflow sono lasciati a «spec della storia», e su questi punti due unità conformi ad AD-20 possono costruire cose incompatibili.

## A. Lente avversaria

### A1. Grammatica del nome e prefisso `node.` (alta)
Dove: AD-20, Rule, primo punto.
Problema: l'unico vincolo scritto è «niente `:` né `@`». Restano indefiniti:
- punti nel nome (`node.a.b`: nodo `a.b` o funzione `node` con sotto-nome?). Il validatore Rust taglia al primo punto, un parser TS al secondo, un editor mostra un altro chip;
- maiuscole (`Foo` e `foo` sono la stessa funzione?), Unicode (NFC/NFD, `char::is_alphanumeric` di Rust contro `\w` di JS), lunghezza massima, `/` (senza il divieto, `https://x/y@v1` è indistinguibile da un nome: OWS ha davvero la forma `call: https://…@v1.0.0`, dsl.md:697);
- cosa succede se un worker registra una funzione che si chiama `node.x` o `node` (la riservatezza del prefisso è dichiarata, il rifiuto in registrazione no);
- l'enum riservato dello schema (`http`, `grpc`, `openapi`, `asyncapi`, `a2a`, `mcp`) confronta la stringa intera: `http:latest@default` passa lo schema OWS. Una funzione chiamata `http` è quindi accettata dallo schema e va riservata dal validatore.
Correzione: una sola grammatica ASCII nella spine (per esempio `[a-z][a-z0-9_-]{0,62}`, senza punti, minuscole), nomi riservati = i sei di OWS + `node`, una sola funzione `parse_call` nel dominio esposta da `packages/validator` (come per il gruppo `switch` in AD-8), vettori di test condivisi tra Rust e TS. Il «dettaglio nella spec della storia di registrazione» non può restare aperto, perché serve già al validatore del browser.

### A2. Quando `latest` si lega, e cosa salva la versione pubblicata (alta)
Dove: AD-20 Rule, secondo punto; FR-5; FR-15; C-5; M-09.
Problema:
- «Un nodo si lega alla sua versione all'avvio dell'esecuzione» regge per i nodi (FR-15). Per le funzioni dei worker non c'è una versione: il manifest di FR-55 non ha un campo versione e FR-12 permette a un secondo worker di registrare la stessa funzione con un manifest diverso. Quale manifest vale per controllare input/output (FR-55) se due worker divergono? Quando ci si lega: avvio dell'esecuzione o consegna del task?
- La versione pubblicata (FR-5) conserva solo la stringa `latest`: il workflow salvato cambia comportamento quando cambia un manifest o si carica un nodo nuovo, senza una nuova versione di workflow e senza rivalidazione. FR-5 dice «le esecuzioni in corso finiscono con la versione con cui sono partite», ma la versione del workflow non basta a fissarle: nessuna regola dice che l'esecuzione registra, per ogni task, il manifest o l'impronta SHA-256 risolti all'avvio (FR-15 lo dice solo per il nodo, a posteriori «ogni attività registra quale versione ha eseguito»).
- M-09 (effetti di un cambio di manifest) è assegnata alle storie delle versioni, ma AD-20 sposta la dipendenza da «versione fissata» a «sempre l'ultima», cioè ha già deciso la direzione più rischiosa. Pianificazioni (FR-57) e pipeline stream (FR-24) avviano workflow salvati: un manifest nuovo li rompe in silenzio al prossimo avvio.
- `latest` non è definito da OWS per i cataloghi (il layout usa directory semver, `functions/log/1.0.0/function.yaml`; `latest` compare solo in `run.workflow.version`, default `latest`). È una convenzione di topictoast e va dichiarata tale.
Correzione: scrivere in AD-20 che (1) la risoluzione avviene una volta all'avvio dell'esecuzione e produce una mappa task → (nome, impronta del manifest o del nodo) conservata con l'esecuzione e usata per tutte le consegne e i retry; (2) per le funzioni il manifest vigente è quello dell'ultima registrazione, con regola esplicita se due worker divergono (rifiuto della seconda registrazione incompatibile, come FR-55 già vuole una regola di compatibilità); (3) M-09 resta alla spec, ma con il vincolo minimo che un cambio di manifest non rende non valida un esecuzione già partita.

### A3. Funzione che sparisce (media)
Dove: AD-20, FR-4 («il controllo si ripete all'esecuzione»), FR-12.
Problema: sono tre casi diversi con esiti diversi e nessuno è scritto per AD-20: (a) nessun worker connesso (FR-12: resta in coda con il motivo); (b) funzione ritirata dall'elenco dell'integratore per il tenant (FR-4: come inesistente); (c) nodo eliminato o sostituito. Per (b) e (c) a esecuzione già partita non si dice se il task fallisce, resta in attesa o salta, né con quale codice C-7 e quale motivo di attesa. Un editor che mostra «in attesa» e un motore che fa fallire sono entrambi conformi.
Correzione: una riga in AD-20 con la tabella dei tre casi, i codici nel catalogo C-7 e l'effetto (attesa con tetto di FR-8 solo per (a); fallimento del task per (b) e (c)).

### A4. Contesti integratore e tenant, sotto-workflow (alta)
Dove: AD-20 Rule, quarto punto; PRD FR-29 e §4.x (L868-L870); FR-15.
Problema:
- AD-20 dice «contesto del proprietario del workflow»; il PRD distingue proprietario del workflow e proprietario dell'esecuzione (L862-L866). Per un sotto-workflow dell'integratore chiamato da un tenant, i nomi dentro il sotto-workflow si risolvono per l'integratore (proprietario del documento) mentre i segreti sono del tenant. AD-20 non dice che la risoluzione è per documento e non per esecuzione; due unità possono scegliere ciascuna una lettura. M-37 lo lascia aperto solo per le connessioni, non per i nomi.
- Un workflow dell'integratore «non vede mai i nodi dei tenant» e FR-15 dice che i nodi li carica il tenant: quindi `node.` in un documento dell'integratore è sempre un errore. Va scritto (e l'editor non deve offrire nodi nella palette dell'integratore), altrimenti il contesto di validazione dell'integratore (AD-11) può includere o no i nodi.
- Un tenant che chiama un sotto-workflow dell'integratore che contiene `node.x`: vale lo stesso rifiuto, ma la validazione all'invio del tenant non guarda dentro il sotto-workflow dell'integratore.
- FR-4 chiede lo stesso errore per «inesistente» e «non disponibile» (nessun dettaglio del manifest). AD-20 non lo ripete: un tenant che scrive `node.x` con `x` caricato da un altro tenant deve ricevere lo stesso errore, e con tempi simili, di un nome mai esistito. Il test a due tenant di AD-5 deve coprire la ricerca per nome.
- AD-5 impone un solo adattatore con filtro di tenant: il registro delle funzioni è globale (integratore), quello dei nodi è per tenant. La query «cerca nome» unisce i due insiemi; non è scritto quale metodo del repository la serve né che il test a due tenant la copre.
Correzione: in AD-20 «la risoluzione è per documento, nel contesto del suo proprietario; i segreti restano del proprietario dell'esecuzione»; elenco chiuso di chi vede cosa; stesso codice di errore per non esiste / non disponibile; nuovo metodo del repository elencato per il test di AD-5.

### A5. `call http` contro le chiamate a funzione (media)
Dove: AD-20 Rule, quinto punto; ADR «Decisione», secondo sotto-punto; FR-29; FR-16.
Problema: AD-20 dice «una chiamata a funzione o nodo non nomina segreti», l'ADR dice «una chiamata non nomina segreti» (senza «a funzione o nodo»), mentre FR-29 vuole che un task senza manifest dichiari nel documento la connessione: l'ADR, letto alla lettera, vieta ciò che FR-29 richiede. FR-16 dice ancora che il nodo vede «i segreti che il suo task dichiara»; con AD-20 li dichiara il manifest, non il task. Il vincolo «chiave API in un header deve essere esprimibile» è verificato (le politiche OWS sono basic, bearer, digest, oauth2, oidc, nessuna chiave API), ma resta a M-37 senza scadenza prima del validatore: il validatore del dominio, che AD-20 dice proprietario unico, sarà scritto nell'epica 1 prima della storia delle connessioni (epica 8). Due unità possono costruire il campo in modi diversi nel frattempo.
Correzione: allineare l'ADR («una chiamata a funzione o nodo»), riscrivere FR-16 come «i segreti delle connessioni del suo manifest», e fissare in AD-20 che fino a M-37 il validatore rifiuta qualunque dichiarazione di connessione in un task `call http` (un solo comportamento per tutti).

### A6. Scritture sugli stream (media)
Dove: AD-20 Rule, ultimo punto; PRD L872-L874; AD-17; C-2.
Problema:
- PRD: «un workflow scrive solo sugli stream del proprietario dell'esecuzione». AD-20: mai dal documento (`emit` fuori sottoinsieme). Come scrive allora un workflow? Via una funzione o un nodo, via `call http` verso l'API del motore, o non lo fa? È un buco: il documento non ha un modo scritto per realizzare ciò che il PRD afferma.
- C-2 dice che le scritture di un tentativo sono accettate solo dalla sessione del worker assegnato, e che per `attemptId` vale il primo esito. Una scrittura sullo stream non è un esito: un retry dello stesso task riscrive gli stessi eventi. Idempotenza e deduplicazione delle scritture tra tentativi non sono decise (si ereditano gli eventi di un tentativo fallito?).
- «Sulla stessa strada dei risultati» (AD-17) vale per cifratura e archivio, ma gli eventi di stream sono letti da altri (pipeline, FR-18), non dal motore solo: la semantica di visibilità durante il tentativo è diversa. È spostata in S-1 («come un worker scrive su uno stream»), va bene, ma il trasporto è già vincolato in AD-20 in due modi (SDK o host della sandbox).
Correzione: dichiarare come un workflow scrive su uno stream (o correggere la frase del PRD), e mettere deduplicazione tra tentativi nei criteri di S-1.

### A7. Contraddizioni e incoerenze con regole esistenti
- **PRD FR-1** (rischio, media): «un documento OWS 1.1 che usa solo il sottoinsieme viene accettato». Un documento con `call: foo:1.0.0@default`, `use.functions` o `use.catalogs.default` è OWS valido (la spec permette perfino di sovrascrivere `default`) e AD-20 lo rifiuta. Va elencato come fuori dal sottoinsieme nella nota di FR-1 e nel testo del sottoinsieme, non solo nella spine («fuori dal sottoinsieme di FR-1» in AD-20 è circolare: FR-1 oggi non nomina `use.functions`).
- **AD-11 / browser** (bassa): il contesto di validazione è limitato al tenant della credenziale; per un workflow dell'integratore (editor dell'integratore) il contesto deve essere costruito in un solo modo; non scritto.
- **AD-8** (bassa): coerente (`$secrets` vietati), ma `use.secrets` e `use.authentications` non sono citati: sono fuori dal sottoinsieme? Una riga basta.
- **C-5** (bassa): «versione di un nodo = SHA-256»; AD-20 usa `latest` e non dice come si ordina «ultimo» (tempo di caricamento? due caricamenti concorrenti dello stesso nome?). Serve un ordine totale (ordine di commit nel motore).
- **C-7** (bassa): AD-20 introduce almeno sei errori nuovi (catalogo non `default`, versione diversa da `latest`, nome non valido, nome riservato, URL, nome non disponibile) senza elencarli; il catalogo vive nel dominio ma l'epica 1 non ha l'elenco.
- **Residui non decisi né rinviati**: formato del nome per i sotto-workflow (la forma `run: workflow` ha `version` con default `latest` in OWS, quindi lì una versione fissa è ammessa e per le funzioni no: asimmetria da giustificare); clic dall'editor «sposta il nodo da `node.a` a `node.b`» (rinomina: aggiorna i documenti salvati?); elenco ordinato di chi vede quali funzioni per tenant (`L'integratore dichiara quali funzioni sono invocabili`: dove vive questa dichiarazione e se è versionata).

## B. Verifica sulle fonti OWS 1.1

Verificato corretto:
1. `call` di una funzione personalizzata: `type: string`, `not: enum [asyncapi, grpc, http, openapi, a2a, mcp]` (workflow.yaml, CallFunction): l'elenco dell'ADR coincide. Nota: `grpc` e gli altri compaiono nell'enum, non `run` né `node`.
2. Lo schema non lega `call` a `use.functions` (nessun riferimento incrociato): vero. «La spec non dice cosa succede con un nome non dichiarato»: sostanzialmente vero (dsl.md dice solo come si definiscono le funzioni: `use.functions`, `function.yaml`, catalogo); è un'inferenza, non una frase della spec.
3. Catalogo predefinito: dsl.md «Default Catalog»: «Runtimes may optionally define a "default" catalog… implicitly by any and all workflows», forma `{functionName}:{functionVersion}@default`, nome `default` riservato; come le risorse sono salvate e risolte «is entirely up to the runtime». L'ADR lo riporta bene. La forma `name:version@default` è quindi corretta.
4. Le politiche di autenticazione sono basic, bearer, digest, oauth2, oidc (workflow.yaml, AuthenticationPolicy); nessuna chiave API: affermazione dell'ADR corretta.

Da correggere o precisare nell'ADR:
5. `latest` come versione non esiste in OWS per le funzioni di catalogo (solo `run.workflow.version`, default `latest`). L'ADR «Bilancio» dice che il documento «resta OWS sensato»: per un altro runtime `foo:latest@default` dipende dal suo catalogo predefinito, e la versione `latest` potrebbe non esistere. Va scritto che `latest` è una convenzione di topictoast (e che a un runtime terzo servono nomi e versioni, non i manifest).
6. La spec ammette di sovrascrivere il catalogo predefinito con `use.catalogs.default`, e ammette `call: https://…@v1.0.0` (URL di un `function.yaml`, dsl.md:697). L'ADR rifiuta entrambi: coerente con FR-55, ma va citato come deviazione da OWS (e rientra nel fatto che topictoast non è un runtime conforme, FR-1), non come caso già «verificato».
7. «Il nome si risolve nel contesto del proprietario»: non c'è un equivalente in OWS (è scelta del runtime); l'ADR non lo attribuisce alla spec, va bene.

## Ordine di correzione consigliato
1. A1 (grammatica, un solo parser), perché blocca il validatore del dominio nell'epica 1.
2. A2 (momento del legame e registrazione per task), perché cambia FR-5 e M-09.
3. A4 (risoluzione per documento, stesso errore per non esiste / non disponibile).
4. A7, nota di FR-1 sul sottoinsieme; B5, B6 nell'ADR.
5. A5, A6, A3.
