# Revisione di sicurezza e separazione tra clienti del PRD topictoast

**Verdetto:** il PRD non è ancora pronto per l'architettura sul piano della sicurezza, perché la separazione tra clienti poggia su quattro assunzioni non confermate (FR-27, FR-28, FR-29, FR-45) e lascia senza requisiti il confine dei worker, che ricevono i segreti di tutti, e tre superfici in cui la logica scritta dal cliente gira nel motore fuori dalla sandbox (`call http`, espressioni jq, SQL delle pipeline stream).

Conteggio: **critica 6 · alta 10 · media 13 · bassa 5**.

Ambito: solo requisiti (cosa il PRD deve chiedere), non come costruirli. Ogni correzione è la più piccola che chiude il buco e porta una conseguenza verificabile. Dove una revisione parallela tocca lo stesso punto c'è un rimando di una riga; ogni voce qui resta leggibile da sola.

Prima dell'architettura: C1–C6 e A1. Le altre si applicano nella stessa passata sul PRD.

| ID | Gravità | Dove | Titolo |
|---|---|---|---|
| C1 | critica | FR-12, FR-33 | Worker senza autenticazione: chi registra una funzione riceve i segreti |
| C2 | critica | FR-27, FR-28, NFR-8 | Identità del cliente non legata alla credenziale; nessun controllo verticale |
| C3 | critica | FR-1, FR-16 | `call http` eseguita dal motore fuori dalla politica di rete |
| C4 | critica | FR-1, FR-34 | Espressioni jq del cliente nel processo che custodisce la chiave |
| C5 | critica | FR-19 | Dialetto SQL delle pipeline senza lista chiusa |
| C6 | critica | FR-16, FR-54 | Sandbox leggera: nessuna regola di non convivenza tra clienti |
| A1 | alta | FR-33, FR-34 | Segreti con le attività su un bus persistente |
| A2 | alta | NFR-8 | Trasporto cifrato non richiesto |
| A3 | alta | FR-40, FR-37 | Funzioni dell'integratore invocabili da ogni cliente |
| A4 | alta | FR-29 | Risoluzione dei nomi e origine del contesto nei workflow dell'integratore |
| A5 | alta | FR-18, FR-24 | Chi scrive su uno stream ne sceglie il cliente |
| A6 | alta | FR-15, FR-16 | Byte dei nodi trattati nel processo della chiave |
| A7 | alta | FR-50, FR-45 | La configurazione predefinita spinge verso segreti predefiniti |
| A8 | alta | FR-46 | Testo dei clienti interpretato nella dashboard dell'operatore |
| A9 | alta | FR-30 | Quote mancanti su spazio, ingresso eventi, API, pool |
| A10 | alta | FR-34 | Ciclo di vita della chiave della cassaforte assente |
| M1 | media | FR-16 | Lista di destinazioni dichiarata dal nodo stesso |
| M2 | media | FR-54 | Sandbox riservata: residui tra attività |
| M3 | media | FR-33 | Segreti riportati in errori e output da codice esterno |
| M4 | media | FR-52 | Metriche: endpoint ed etichette |
| M5 | media | FR-10, FR-27 | Nessun registro delle azioni |
| M6 | media | FR-26, FR-41 | Connessioni in tempo reale oltre la credenziale |
| M7 | media | FR-18, FR-21 | Watermark avvelenato dal tempo dell'evento |
| M8 | media | FR-28 | Nessuna sospensione di un cliente |
| M9 | media | FR-19 | Pipeline dell'integratore su più clienti |
| M10 | media | FR-31 | Eliminazione e conservazione fuori dal database |
| M11 | media | FR-3, FR-4 | Limiti di dimensione e complessità |
| M12 | media | FR-15, FR-27 | Credenziali dei clienti per SDK e CLI |
| M13 | media | FR-15 | Nodi caricati: impronta, versioni, blocco |
| B1 | bassa | FR-39 | Origini ammesse per il kit |
| B2 | bassa | FR-41, FR-29 | Interni dei workflow dell'integratore visibili al cliente |
| B3 | bassa | FR-32 | Uso dei segreti da parte dei workflow dell'integratore |
| B4 | bassa | FR-1, FR-16 | Rete in uscita usata contro terzi |
| B5 | bassa | NFR-11 | Artefatti di rilascio verificabili |

---

## Gravità critica

Qui un solo difetto, o nessun difetto ma una funzione prevista, basta a rompere la separazione tra clienti o a esporre i segreti di tutti.

### C1. Worker senza autenticazione: chi registra una funzione riceve i segreti
- **Gravità:** critica
- **Dove:** FR-12, FR-33, FR-37; §3 «Worker»
- **Citazione:** «Un worker si collega con l'SDK, registra le sue funzioni e riceve le attività che le invocano.»
- **Attacco:** nessun requisito dice chi può collegarsi come worker. Un processo che raggiunge il motore (un container compromesso sulla stessa rete, o un cliente che usa le credenziali SDK con cui carica i nodi, FR-15) registra `censys_search`. Per FR-12 le attività «si distribuiscono» anche a lui; per FR-33 ognuna arriva con i segreti decifrati del cliente dell'esecuzione. Restando collegato raccoglie le chiavi API di tutti i clienti che usano quella funzione; se il worker legittimo è fermo, le riceve tutte. Vale anche per chi legge direttamente il bus, se i worker vi accedono.
- **Correzione:** un FR "Autenticazione dei worker": solo una credenziale di worker emessa dall'integratore registra funzioni e riceve attività; una credenziale di cliente non può farlo; il bus non accetta connessioni senza credenziale né con credenziali di cliente. **Conseguenza verificabile:** un processo senza credenziale di worker, o con una credenziale di cliente, che registra una funzione esistente viene rifiutato e non riceve alcuna attività.
- Vedi anche `review-rubric.md`, «Confine di fiducia verso i worker».

### C2. Identità del cliente non legata alla credenziale; nessun controllo verticale
- **Gravità:** critica
- **Dove:** FR-27, FR-28, FR-38, FR-39, NFR-8
- **Citazione:** «Una richiesta senza identità valida viene rifiutata.»
- **Attacco:** FR-27 è l'unico confine tra clienti, è un'assunzione e ha una sola conseguenza di sicurezza. Non dice:
  1. che il cliente si ricava solo dalla credenziale e mai da un campo della richiesta (identificativo nel percorso, nel corpo, in un filtro di lista): un solo endpoint che legge il parametro dà a una credenziale valida i dati di un altro cliente;
  2. che la credenziale porta il ruolo e che una credenziale di cliente non apre le operazioni dell'integratore (FR-28: clienti e quote; FR-38: tutto via SDK). Il kit gira nel browser del cliente, quindi la credenziale è in mano al cliente, che può chiamare l'API a mano;
  3. scadenza, destinatario e revoca.

  Il test di NFR-8 prova solo l'accesso orizzontale («un cliente non legge né modifica risorse di un altro»), non quello verticale.
- **Correzione:** in FR-27/FR-28: cliente e ruolo si ricavano solo dalla credenziale verificata; la credenziale ha una scadenza e vale per un solo cliente. **Conseguenza verificabile:** il test di NFR-8 si estende: per ogni punto di accesso, una credenziale valida del cliente A con l'identificativo di B in un parametro agisce su A o viene rifiutata; su ogni operazione riservata all'integratore riceve un rifiuto; una credenziale scaduta viene rifiutata.
- Sfasamento d'orologio e rotazione: `review-edge-cases.md` M5.

### C3. `call http` eseguita dal motore fuori dalla politica di rete
- **Gravità:** critica
- **Dove:** FR-1, FR-16, NFR-8
- **Citazione:** «fork, for, try con retry, wait, raise, call http, invocazione di funzioni e nodi personalizzati,»
- **Attacco:** FR-16 vieta la rete interna solo ai nodi personalizzati («Non raggiunge la rete interna dell'installazione»). Una `call http` in un workflow del cliente parte dal motore, il processo che custodisce la chiave e vede database e bus. Giulia, o chiunque abbia accesso all'editor di un cliente, mette una `call http` verso `http://169.254.169.254/latest/meta-data/iam/security-credentials/`, verso l'indirizzo del bus o di Postgres, o verso un servizio interno della piattaforma ospite, e legge la risposta nell'output del task (FR-41). Non serve alcuna falla: è una funzione prevista. Il test con codice ostile di NFR-8 riguarda la sandbox e non la copre.
- **Correzione:** in FR-16: ogni richiesta di rete fatta per conto di un cliente, da qualsiasi parte del motore, segue la politica dei nodi personalizzati; loopback, indirizzi privati, link-local e indirizzi dell'installazione sono negati sull'indirizzo risolto e a ogni redirect. **Conseguenza verificabile:** una `call http` di un cliente verso `127.0.0.1`, `169.254.169.254`, l'indirizzo del bus o un nome DNS che risolve a un indirizzo privato fallisce con un errore di politica di rete e nessuna connessione parte; il caso entra nel test di NFR-8.
- Stesso tema, come caso limite: `review-edge-cases.md` A13.

### C4. Espressioni jq del cliente nel processo che custodisce la chiave
- **Gravità:** critica
- **Dove:** FR-1, FR-16, FR-17, FR-34, NFR-7
- **Citazione:** «sotto-workflow, espressioni jq»
- **Attacco:** le espressioni jq le scrive il cliente e le valuta il motore. Il linguaggio jq prevede l'accesso all'ambiente del processo (`$ENV`, `env`) e, secondo l'implementazione, a file e moduli (`input_filename`, `import`, `include`). Se la chiave della cassaforte, la chiave dell'integratore o la password del database stanno in una variabile d'ambiente, un'espressione `$ENV` nell'input di un task le copia nell'output, visibile al cliente. jq è anche un linguaggio completo: `[range(1e10)]` occupa la memoria e `last(range(1e12))` la CPU del motore, che serve tutti i clienti. FR-16 e FR-17 limitano solo i nodi personalizzati.
- **Correzione:** un requisito sulle espressioni dei clienti: non leggono ambiente, file system, moduli né rete; hanno limiti di tempo e memoria; superarli fa fallire il task, non il motore. **Conseguenza verificabile:** un'espressione che usa `$ENV`, `env`, `input_filename` o `import` è rifiutata alla validazione (FR-4); `last(range(1e12))` fa fallire il task entro il limite dichiarato e intanto le attività degli altri clienti restano nella latenza di NFR-2.

### C5. Dialetto SQL delle pipeline senza lista chiusa
- **Gravità:** critica
- **Dove:** FR-19, FR-25, NFR-7; §4.5
- **Citazione:** «Una query che fa riferimento a stream di un altro cliente viene rifiutata come se lo stream non»
- **Attacco:** FR-19 vieta solo i riferimenti agli stream di altri clienti. I motori indicati come base (§4.5) accettano istruzioni che leggono e scrivono file o aprono connessioni: DataFusion ha `CREATE EXTERNAL TABLE ... LOCATION` e `COPY ... TO`, Arroyo definisce sorgenti e destinazioni con `CREATE TABLE ... WITH (connector = ...)`. Una cliente salva dall'editor una pipeline che legge un file del server (il database SQLite, la configurazione con le chiavi) o apre un connettore verso il bus e ne legge le partizioni condivise, poi manda le righe su uno stream suo. Ogni costrutto valido per il motore sottostante ma non elencato è una porta aperta.
- **Correzione:** in FR-19: il dialetto delle pipeline dei clienti è una lista chiusa, documentata: solo interrogazioni sugli stream del cliente con le funzioni elencate; DDL, tabelle esterne, connettori, lettura o scrittura di file, rete e funzioni definite dall'utente sono rifiutati al salvataggio. **Conseguenza verificabile:** un insieme di query ostili (tabella esterna su `/etc/passwd`, `COPY` verso un file, connettore verso l'indirizzo del bus, funzione definita dall'utente) è rifiutato al salvataggio con un errore di validazione; l'insieme entra nel test di NFR-8.
- Join tra stream: FR-19 li limita già agli stream dello stesso cliente, e lo stato di un join senza finestra ricade nella quota di memoria di FR-25, che sospende la pipeline. Resta scoperta la CPU: FR-25 elenca la quota ma non dice cosa succede quando una pipeline la supera. Conseguenza da aggiungere: la pipeline oltre quota di CPU viene rallentata o sospesa con un errore visibile, e le pipeline degli altri clienti mantengono il proprio throughput.
- Vedi `review-edge-cases.md` A16 (determinismo e stato illimitato).

### C6. Sandbox leggera: nessuna regola di non convivenza tra clienti
- **Gravità:** critica
- **Dove:** FR-16, FR-54, FR-50; §4.4
- **Citazione:** «Una sandbox che ha eseguito codice di un cliente non passa mai a un altro cliente: è monouso o»
- **Attacco:** la regola vale solo per la sandbox forte (FR-54). Per la sandbox leggera, che è il livello predefinito e quello di quasi tutte le installazioni (FR-50: niente KVM), FR-16 protegge la chiave («gira fuori dal processo che custodisce la chiave»), non i segreti già decifrati. Se un processo della sandbox leggera esegue insieme i nodi di più clienti, o riusa istanze e memoria tra clienti, una sola fuga dal runtime («un bug del runtime è una fuga», addendum del brief, §6) legge input e segreti in transito di ogni cliente che gira lì in quel momento, senza mai toccare la chiave.
- **Correzione:** estendere a entrambi i livelli la regola di FR-54: un'istanza della sandbox, il suo processo e la memoria che ha usato non contengono mai dati o segreti di due clienti, né insieme né uno dopo l'altro. **Conseguenza verificabile:** nel test di NFR-8, mentre gira un nodo del cliente B con un segreto sentinella, la memoria del processo che esegue il nodo del cliente A non contiene il segreto sentinella.

---

## Gravità alta

### A1. Segreti con le attività su un bus persistente
- **Gravità:** alta
- **Dove:** FR-33, FR-34, FR-12, FR-22; §1; §10 rischio 3
- **Citazione:** «Una copia del database senza la chiave di cifratura non rivela alcun segreto.»
- **Attacco:** le attività viaggiano sul bus (§1: «le attività viaggiano su un bus di messaggi»), il bus è un log con segmenti su disco (rischio 3), un'attività può attendere fino al timeout (FR-12) e FR-33 consegna i segreti «insieme all'attività». Se il segreto sta nel messaggio, finisce in chiaro nei file del bus, nelle loro copie e nei backup, per tutta la ritenzione del log. FR-34 protegge solo la «copia del database»: una copia della directory del bus rivela le chiavi API di tutti i clienti. Lo stesso vale per i checkpoint (FR-22) e i file temporanei.
- **Correzione:** FR-34 diventa: nessun supporto persistente di topictoast (database, bus, checkpoint, log, file temporanei) contiene un segreto in chiaro. **Conseguenza verificabile:** dopo un'esecuzione con un segreto sentinella, una ricerca del valore nelle directory dati di topictoast, del bus e dei checkpoint non trova nulla.

### A2. Trasporto cifrato non richiesto
- **Gravità:** alta
- **Dove:** NFR-8, FR-27, FR-33, FR-39
- **Citazione:** «segreti cifrati e mai nei log»
- **Attacco:** NFR-8 parla di cifratura solo a riposo. I segreti decifrati vanno dal motore ai worker, che sono processi dell'integratore anche su altre macchine; la credenziale del cliente va dal browser al motore (FR-39); le attività vanno dal motore al bus. Senza un requisito, una configurazione in chiaro è conforme al PRD, e chi osserva la rete tra motore e worker raccoglie le chiavi API di tutti.
- **Correzione:** in NFR-8: ogni connessione che porta segreti, credenziali o dati dei clienti è cifrata e autentica il server; la configurazione predefinita accetta connessioni in chiaro solo da localhost. **Conseguenza verificabile:** con la configurazione predefinita, un worker che si collega in chiaro da un altro host viene rifiutato.

### A3. Funzioni dell'integratore invocabili da ogni cliente
- **Gravità:** alta
- **Dove:** FR-40, FR-37, FR-36, FR-39, FR-4
- **Citazione:** «L'editor mostra solo funzioni, nodi personalizzati e segreti disponibili al cliente.»
- **Attacco:** nessun FR dice chi decide quali funzioni sono «disponibili al cliente», né che il limite vale all'invio del documento e non solo nell'editor. Il worker di Marco registra anche funzioni pensate per i workflow dell'integratore, per esempio una che scrive nel database della piattaforma ospite per il cliente indicato nell'input. Un cliente invia a mano, con il nucleo JS (FR-39), un documento OWS che invoca quella funzione con l'identificativo di un altro cliente nell'input. L'editor non entra in gioco e FR-4 controlla solo che la funzione esista.
- **Correzione:** un FR: l'integratore dichiara per ogni funzione se è usabile dai workflow dei clienti (tutti o alcuni); il motore applica la regola all'invio e all'esecuzione. La documentazione per l'integratore chiede di ricavare il cliente dal contesto (FR-37), mai dall'input. **Conseguenza verificabile:** un documento di un cliente che invoca una funzione riservata all'integratore è rifiutato all'invio come se la funzione non esistesse.

### A4. Risoluzione dei nomi e origine del contesto nei workflow dell'integratore
- **Gravità:** alta
- **Dove:** FR-29, FR-1 (sotto-workflow), FR-15, FR-6, FR-24
- **Citazione:** «Un workflow dell'integratore eseguito per un cliente gira nel contesto di quel cliente»
- **Attacco:** un workflow dell'integratore gira con i segreti del cliente e richiama per nome funzioni, nodi e sotto-workflow. Se i nomi si risolvono nel contesto del cliente, il cliente carica un nodo o crea un sotto-workflow con lo stesso nome di un passo dell'integratore (`enrich`) e il suo codice gira dentro il workflow dell'integratore, con i dati che l'integratore gli passa. Nell'altro verso, nessun divieto esplicito impedisce a un sotto-workflow di un cliente di nominare un workflow di un altro cliente. FR-29 non dice poi da dove viene il cliente quando l'avvio è una pianificazione (FR-6) o una riga di pipeline (FR-24) create dall'integratore.
- **Correzione:** in FR-29: i riferimenti dentro un workflow si risolvono solo tra le risorse del suo proprietario (integratore: solo le sue; cliente: le sue più quelle che l'integratore gli rende disponibili), mai tra quelle di un altro cliente; ogni trigger ha un cliente fissato alla creazione. **Conseguenza verificabile:** un cliente che crea un nodo o un sotto-workflow con lo stesso nome di uno usato da un workflow dell'integratore non cambia cosa esegue quel workflow quando gira per lui.
- Collisione di nomi anche in `review-edge-cases.md` M4.

### A5. Chi scrive su uno stream ne sceglie il cliente
- **Gravità:** alta
- **Dove:** FR-18, FR-24, FR-37
- **Citazione:** «L'integratore, i worker e i workflow possono scrivere eventi sugli stream di un cliente tramite API»
- **Attacco:** FR-18 vincola chi legge, non chi scrive. Un workflow del cliente A che scrive indicando uno stream del cliente B, o un worker che serve un'attività di A e scrive su B per un errore nel codice dell'integratore, porta eventi scelti da A nella pipeline di B. Per FR-24 ogni riga emessa avvia un workflow di B, con input deciso da A e segreti di B; con una `call http` costruita dall'input, A fa spedire i segreti di B a un suo indirizzo.
- **Correzione:** in FR-18: un workflow scrive solo sugli stream del cliente dell'esecuzione; un worker, mentre serve un'attività, scrive sul cliente dell'attività; scrivere sugli stream di un altro cliente richiede la credenziale dell'integratore fuori da un'attività. **Conseguenza verificabile:** un workflow del cliente A che scrive su uno stream del cliente B riceve lo stesso errore di uno stream inesistente e a B non arriva alcun evento.

### A6. Byte dei nodi trattati nel processo della chiave
- **Gravità:** alta
- **Dove:** FR-15, FR-16, NFR-7
- **Citazione:** «Un nodo che non rispetta il formato richiesto (modulo compilato, schema) viene rifiutato con»
- **Attacco:** FR-16 tiene fuori dal processo della chiave solo l'esecuzione. Validare un modulo compilato, e con molte tecnologie di sandbox compilarlo in codice macchina prima di eseguirlo, vuol dire dare byte ostili a un parser e a un compilatore. Se avviene nel processo del motore, un bug del validatore o del compilatore è esecuzione di codice accanto alla chiave, prima ancora che il nodo giri.
- **Correzione:** in FR-16: ogni trattamento dei byte di un nodo caricato (validazione, compilazione, istanziazione, esecuzione) avviene fuori dal processo che custodisce la chiave. **Conseguenza verificabile:** nel test di NFR-8, un modulo costruito per far crashare il validatore non ferma il processo che custodisce la chiave né le esecuzioni in corso.

### A7. La configurazione predefinita spinge verso segreti predefiniti
- **Gravità:** alta
- **Dove:** FR-50, FR-45, FR-27, FR-34, NFR-13
- **Citazione:** «Con la configurazione predefinita, un workflow di esempio gira senza modificare nulla.»
- **Attacco:** FR-50 e NFR-13 (15 minuti) premiano un'installazione che parte senza toccare nulla. FR-45 vuole credenziali della dashboard «configurate all'installazione», FR-27 una chiave dell'integratore, FR-34 una chiave di cifratura. La strada più corta per rispettare FR-50 è un file d'esempio con password della dashboard, chiave dell'integratore e chiave della cassaforte già scritte, che poi finisce in produzione. Con la chiave dell'integratore pubblica nel repository, chiunque firma una credenziale per qualsiasi cliente o per l'integratore.
- **Correzione:** in FR-50: nessun segreto ha un valore predefinito noto; al primo avvio topictoast genera i segreti che servono, o rifiuta di partire finché non vengono forniti. **Conseguenza verificabile:** due installazioni nuove hanno chiavi diverse, e topictoast rifiuta di partire con una chiave presente negli esempi della documentazione.

### A8. Testo dei clienti interpretato nella dashboard dell'operatore
- **Gravità:** alta
- **Dove:** FR-46, FR-47, FR-48, FR-41, NFR-7
- **Citazione:** «L'operatore vede le esecuzioni di tutti i clienti, con filtri per cliente, workflow e stato.»
- **Attacco:** nomi di workflow, task e nodi, errori, input, output e righe delle pipeline li scrivono i clienti, e la dashboard li mostra all'operatore, che vede tutti. Un cliente mette uno script nel nome di un task o nell'errore restituito dal suo nodo; se la dashboard lo interpreta, lo script gira nella sessione dell'operatore e da lì legge e modifica le risorse di tutti i clienti. È il passaggio da un cliente a tutti che non richiede falle nel motore. Lo stesso vale per i componenti React nella piattaforma ospite.
- **Correzione:** in NFR-7: il contenuto fornito dai clienti è sempre mostrato come testo, mai interpretato come HTML o script, nella dashboard e nei componenti. **Conseguenza verificabile:** un test salva un payload di script in ogni campo testuale di un cliente (nome, descrizione, errore, output, riga emessa), apre ogni pagina della dashboard e verifica che nessuno script venga eseguito.

### A9. Quote mancanti su spazio, ingresso eventi, API, pool
- **Gravità:** alta
- **Dove:** FR-30, FR-25, FR-18, FR-24, FR-54
- **Citazione:** «L'integratore assegna a ogni cliente quote su attività concorrenti, esecuzioni al minuto, risorse»
- **Attacco:** le quote coprono concorrenza, esecuzioni al minuto, sandbox e pipeline. Restano fuori: eventi scritti al secondo e spazio degli stream (FR-18; lo stream è una «sequenza ordinata e illimitata di eventi di un cliente»), righe in attesa (FR-24: «in eccesso restano in attesa, non vanno perse»), attività in coda, numero e dimensione di workflow, nodi, segreti e storico, richieste API al secondo. Un cliente che scrive eventi senza sosta, o una pipeline che emette milioni di righe contro una quota di 10 esecuzioni al minuto, riempie il disco del bus o il database; per NFR-6 il motore deve allora smettere di accettare lavoro, e si ferma per tutti. Con SQLite c'è un solo scrittore: un cliente che salva di continuo rallenta tutti. Il pool della sandbox forte (FR-54: «quando il pool è vuoto, le attività attendono una sandbox libera entro il timeout del task») non ha una parte per cliente: un cliente con nodi lunghi lo svuota.
- **Correzione:** in FR-30: quote per cliente anche su ingresso eventi, spazio occupato (stream, storico, code, oggetti caricati), richieste API e quota del pool; oltre quota la nuova richiesta riceve un errore di quota, il lavoro già accettato no. **Conseguenza verificabile:** un cliente che scrive eventi oltre la sua quota di spazio riceve errori di quota, mentre gli altri continuano a scrivere e ad avviare esecuzioni nella latenza di NFR-2.
- Code senza tetto: `review-edge-cases.md` A17.

### A10. Ciclo di vita della chiave della cassaforte assente
- **Gravità:** alta
- **Dove:** FR-34, FR-27, NFR-8
- **Citazione:** «I segreti sono cifrati a riposo.»
- **Attacco:** FR-34 non dice dove vive la chiave, come si ruota, cosa succede se si perde o trapela. Se sta accanto al database, il backup porta entrambi. Se trapela, non c'è modo di renderla inutile senza far reinserire a ogni cliente tutti i segreti. Se si perde, si perdono tutti i segreti. La stessa lacuna vale per la chiave con cui la piattaforma ospite garantisce le identità (FR-27).
- **Correzione:** in FR-34: la chiave si ruota senza fermare il motore e senza perdere segreti; dopo la rotazione la chiave vecchia non decifra nulla di ciò che è salvato; la chiave non sta nel database né nel suo backup. **Conseguenza verificabile:** dopo una rotazione, i segreti esistenti funzionano nelle esecuzioni nuove, e una copia del database fatta dopo la rotazione e aperta con la chiave vecchia non rivela alcun segreto.

---

## Gravità media

### M1. Lista di destinazioni dichiarata dal nodo stesso
- **Gravità:** media
- **Dove:** FR-16
- **Citazione:** «L'accesso alla rete esterna è negato salvo una lista di destinazioni dichiarata nel nodo»
- **Attacco:** la lista la scrive chi carica il nodo, cioè la parte non fidata: un nodo ostile dichiara il proprio server, o tutto. Un nome in lista che risolve a un indirizzo interno (DNS controllato dal cliente, rebinding) porta il nodo nella rete dell'installazione nonostante la prima conseguenza di FR-16. L'integratore non ha un tetto sulle destinazioni.
- **Correzione:** in FR-16: la lista del nodo è un sottoinsieme di quella consentita dall'integratore; gli indirizzi interni sono negati sull'indirizzo risolto, qualunque cosa dica la lista. **Conseguenza verificabile:** un nodo con in lista un nome che risolve a `10.0.0.1` non apre la connessione.
- Vedi `review-edge-cases.md` M7.

### M2. Sandbox riservata: residui tra attività
- **Gravità:** media
- **Dove:** FR-54, FR-16
- **Citazione:** «Un nodo personalizzato in esecuzione vede solo il proprio input e i segreti che il suo task»
- **Attacco:** una sandbox forte «riservata a quel cliente» passa da un'attività all'altra dello stesso cliente. Il nodo B trova in memoria o su disco i segreti dichiarati per il nodo A e l'input dell'esecuzione precedente, contro FR-16. Conta perché nodi diversi dello stesso cliente possono venire da sviluppatori diversi, o uno può essere compromesso.
- **Correzione:** in FR-54: tra due attività una sandbox riservata non conserva file, memoria né variabili dell'attività precedente. **Conseguenza verificabile:** un nodo che cerca il segreto sentinella usato dall'attività precedente nella stessa sandbox non lo trova.

### M3. Segreti riportati in errori e output da codice esterno
- **Gravità:** media
- **Dove:** FR-33, FR-7, FR-41, FR-46
- **Citazione:** «I segreti non compaiono nei log, nello storico, negli output né negli errori di topictoast.»
- **Attacco:** «di topictoast» lascia fuori i casi frequenti: un worker che restituisce `401 per la chiave abc123`, un nodo che stampa il segreto, un'espressione jq che fallisce citando il valore. FR-7 conserva l'errore dell'ultimo tentativo, FR-41 e FR-46 lo mostrano: l'operatore vede così i segreti di tutti i clienti. Scritta così, la conseguenza non si può verificare.
- **Correzione:** in FR-33: topictoast sostituisce con un segnaposto i valori dei segreti consegnati a un'attività in tutto ciò che salva o mostra di quell'attività (output, errori, log, uscita dei nodi). **Conseguenza verificabile:** un worker che restituisce come errore il segreto ricevuto produce storico e vista delle esecuzioni con il segnaposto al posto del valore.

### M4. Metriche: endpoint ed etichette
- **Gravità:** media
- **Dove:** FR-52, FR-47
- **Citazione:** «Le metriche coprono attività al secondo, latenza di avvio delle attività, code per cliente,»
- **Attacco:** le metriche per cliente espongono elenco dei clienti, volumi e nomi di workflow e pipeline a chi raggiunge l'endpoint, che di solito non chiede credenziali. Se le etichette contengono nomi scelti dai clienti, un cliente crea migliaia di nomi e fa esplodere il numero di serie, con la memoria del motore e del sistema di raccolta.
- **Correzione:** in FR-52: l'endpoint delle metriche chiede una credenziale o ascolta solo sull'indirizzo configurato; le etichette usano identificativi opachi dei clienti e nessun testo scelto dai clienti. **Conseguenza verificabile:** una richiesta all'endpoint senza credenziale viene rifiutata e nessuna etichetta contiene nomi di workflow, pipeline o task.
- Cardinalità: `review-edge-cases.md` B2.

### M5. Nessun registro delle azioni
- **Gravità:** media
- **Dove:** FR-10, FR-27, FR-32, FR-46
- **Citazione:** «L'esecuzione risulta annullata nello storico, con chi l'ha annullata e quando.»
- **Attacco:** FR-27 garantisce il cliente, non la persona: il «chi» di FR-10 non si può ricavare. Non c'è registro per sostituzione ed eliminazione dei segreti, caricamento dei nodi, modifica dei workflow dell'integratore, cambi di quota, apertura da parte dell'operatore di input e output dei clienti. Dopo un incidente (un nodo sostituito che esfiltra, un operatore che legge i dati di un cliente) non si ricostruisce cosa è successo né a chi.
- **Correzione:** un FR "Registro delle azioni": ogni azione che cambia segreti, nodi, workflow, pipeline, clienti o quote, e ogni apertura del dettaglio di un'esecuzione dalla dashboard, è registrata con cliente, attore e ora, senza valori dei segreti; il cliente vede le voci che lo riguardano. L'attore è un identificativo opaco passato dalla piattaforma ospite per le richieste dei clienti e, per la dashboard, l'operatore: quindi ogni operatore ha una credenziale propria, non una condivisa (FR-45). **Conseguenza verificabile:** dopo la sostituzione di un segreto, il registro del cliente mostra azione, attore e ora, senza il valore.

### M6. Connessioni in tempo reale oltre la credenziale
- **Gravità:** media
- **Dove:** FR-26, FR-41, FR-27, FR-31
- **Citazione:** «Liste, ricerche, conteggi ed eventi in tempo reale non contengono mai risorse di altri clienti.»
- **Attacco:** la vista in tempo reale tiene aperta una connessione. Se l'identità si controlla solo all'apertura, un utente tolto dalla piattaforma ospite, o un cliente eliminato, continua a ricevere stati, input e output finché la connessione resta aperta.
- **Correzione:** in FR-41: una connessione in tempo reale si chiude alla scadenza della credenziale con cui è stata aperta e all'eliminazione o sospensione del cliente. **Conseguenza verificabile:** una connessione aperta con una credenziale valida 60 secondi non riceve eventi dopo il sessantesimo secondo.

### M7. Watermark avvelenato dal tempo dell'evento
- **Gravità:** media
- **Dove:** FR-18, FR-21, UJ-3
- **Citazione:** «Gli eventi arrivati dopo la chiusura della loro finestra vengono scartati e contati in una»
- **Attacco:** nel caso di Marco gli eventi arrivano da feed di indicatori, in parte influenzabili da chi vuole nascondersi. Un evento con tempo nel futuro (anno 2099) spinge avanti il watermark: tutte le finestre si chiudono e gli eventi veri che seguono vengono scartati come in ritardo. Con un solo evento la pipeline di rilevamento di UJ-3 diventa cieca.
- **Correzione:** in FR-21: un evento con tempo oltre una tolleranza configurabile rispetto all'arrivo viene scartato e contato, e non sposta il watermark. **Conseguenza verificabile:** un evento con tempo un anno nel futuro non chiude alcuna finestra e compare nella metrica degli eventi scartati.
- Vedi `review-edge-cases.md` A14.

### M8. Nessuna sospensione di un cliente
- **Gravità:** media
- **Dove:** FR-28, FR-31
- **Citazione:** «Solo l'integratore crea e configura clienti e quote.»
- **Attacco:** davanti a un cliente che abusa (un nodo che attacca, una pipeline che satura) l'unico strumento è eliminarlo (FR-31), che cancella anche le prove. Pianificazioni, pipeline attive e credenziali emesse continuano a lavorare finché il cliente esiste.
- **Correzione:** un FR: l'integratore sospende un cliente; la sospensione ferma pianificazioni, pipeline e nuove esecuzioni, rifiuta le sue credenziali e conserva i dati. **Conseguenza verificabile:** dopo la sospensione nessuna attività del cliente parte, le sue richieste vengono rifiutate e i suoi dati restano leggibili dall'integratore.

### M9. Pipeline dell'integratore su più clienti
- **Gravità:** media
- **Dove:** FR-19, FR-24; §4.5
- **Citazione:** «L'integratore e il cliente possono definire una pipeline stream con una query SQL su uno o più»
- **Attacco:** FR-19 vieta i riferimenti tra clienti solo alle query dei clienti. Una pipeline dell'integratore che unisce gli stream di A e B e manda le righe a un workflow di A porta dati di B nello storico di A. Il PRD non dice a chi appartiene una pipeline dell'integratore né in quale contesto avvia i workflow. Un'analisi su tutti i clienti è plausibile per Marco (indicatori comuni), quindi il caso va regolato, non vietato.
- **Correzione:** in FR-19: una pipeline che legge stream di più clienti appartiene all'integratore e scrive solo su destinazioni dell'integratore; una pipeline di un cliente legge e scrive solo i suoi stream. **Conseguenza verificabile:** una pipeline che legge stream di A e di B con destinazione un workflow di A è rifiutata al salvataggio.

### M10. Eliminazione e conservazione fuori dal database
- **Gravità:** media
- **Dove:** FR-31, FR-18, FR-22, FR-13; §3 «Stream»
- **Citazione:** «Eliminare un cliente elimina tutte le sue risorse.»
- **Attacco:** la conservazione di FR-31 riguarda solo lo storico. Gli stream sono illimitati e senza durata; checkpoint, righe emesse, nodi caricati, log e sandbox riservate non sono citati. Il bus è un log con partizioni condivise tra clienti (FR-13, FR-30): un messaggio di un cliente non si cancella da solo, resta finché scade il segmento. «elimina tutte le sue risorse» non dice entro quando né dove: dati ed eventi di un cliente eliminato restano su disco per un tempo non dichiarato. Se l'identificativo del cliente viene riusato, un cliente nuovo può ritrovarsi risorse del vecchio.
- **Correzione:** in FR-31: durata di conservazione configurabile anche per stream e checkpoint; l'eliminazione di un cliente rende i suoi dati illeggibili in ogni supporto (database, bus, checkpoint, oggetti caricati) entro un tempo massimo dichiarato; un identificativo eliminato non si riusa. **Conseguenza verificabile:** passato il tempo dichiarato, una ricerca di un valore sentinella del cliente eliminato nelle directory dati di topictoast e del bus non trova nulla, e un cliente nuovo con lo stesso nome non vede alcuna risorsa del precedente.

### M11. Limiti di dimensione e complessità
- **Gravità:** media
- **Dove:** FR-3, FR-4, FR-11, FR-19, FR-1
- **Citazione:** «Con un elenco di N elementi nascono N attività; il join riceve N risultati nell'ordine»
- **Attacco:** FR-11 limita input e output a 1 MB, non la cardinalità: un elenco da 1 MB di elementi minimi genera centinaia di migliaia di attività da una sola esecuzione, tutte da salvare. Mancano limiti su dimensione del documento, numero di task, profondità di annidamento, ricorsione tra sotto-workflow, lunghezza della query SQL. Ognuno occupa validatore, pianificatore e database con una sola richiesta, prima che le quote di esecuzione intervengano.
- **Correzione:** in FR-4 e FR-19: limiti configurabili, con valori predefiniti, su dimensione del documento e della query, numero di task, profondità, ricorsione dei sotto-workflow ed elementi di un fan-out; superarli è un errore di validazione o fa fallire il task. **Conseguenza verificabile:** un fan-out oltre il limite di elementi fa fallire il task con un errore che indica limite e valore, senza creare attività.
- Vedi `review-edge-cases.md` A5, A13.

### M12. Credenziali dei clienti per SDK e CLI
- **Gravità:** media
- **Dove:** FR-15, FR-27, FR-32, FR-38; UJ-2
- **Citazione:** «Il cliente può caricare un nodo personalizzato già compilato tramite SDK o CLI, con nome e schema di»
- **Attacco:** FR-27 descrive l'identità per le richieste che passano dalla piattaforma ospite; UJ-2 fa caricare il nodo da «un collega sviluppatore con la CLI», fuori dall'app. Serve quindi una credenziale del cliente di lunga durata, su macchine di sviluppo e in CI. Il PRD non dice chi la emette, che ambito ha, quanto dura e come si revoca. Una credenziale trapelata da un log di CI permette di sostituire un nodo e di leggere tutto il cliente, senza scadenza.
- **Correzione:** in FR-27: le credenziali non interattive dei clienti le emette l'integratore, scadono, si limitano alle operazioni necessarie (per esempio solo caricare nodi) e si revocano una per una. **Conseguenza verificabile:** una credenziale della CLI revocata viene rifiutata alla richiesta successiva, e una credenziale limitata al caricamento dei nodi non legge segreti né esecuzioni.

### M13. Nodi caricati: impronta, versioni, blocco
- **Gravità:** media
- **Dove:** FR-15, FR-5
- **Citazione:** «Il nodo è visibile solo ai workflow del cliente che l'ha caricato.»
- **Attacco:** i workflow hanno versioni (FR-5), i nodi no. Ricaricare un nodo con lo stesso nome cambia in silenzio il codice che gira in tutti i workflow che lo usano, con i loro segreti, e non resta traccia di quale binario ha eseguito un'attività. Non c'è un limite di dimensione del modulo né un modo per ritirare subito un nodo ostile.
- **Correzione:** in FR-15: ogni nodo caricato ha un'impronta del contenuto, immutabile; un caricamento con lo stesso nome crea una versione nuova; ogni attività registra l'impronta eseguita; l'integratore può bloccare un nodo. **Conseguenza verificabile:** lo storico di un'attività mostra l'impronta del nodo eseguito, e dopo il blocco nessuna nuova attività esegue quel nodo.
- Versioni dei nodi: `review-edge-cases.md` A12.

---

## Gravità bassa

### B1. Origini ammesse per il kit
- **Gravità:** bassa
- **Dove:** FR-39, FR-27
- **Citazione:** «Il kit frontend usato dentro la piattaforma ospite non chiede un secondo login all'utente.»
- **Attacco:** se il motore accetta chiamate dal browser da qualsiasi origine, una pagina terza che dispone della credenziale (o, se l'identità viaggia in un cookie, qualsiasi pagina) chiama il motore a nome del cliente. Bassa perché dipende da come viaggia la credenziale, che decide l'architettura.
- **Correzione:** l'integratore configura le origini da cui il motore accetta chiamate dal browser. **Conseguenza verificabile:** una richiesta dal browser da un'origine non configurata viene rifiutata.

### B2. Interni dei workflow dell'integratore visibili al cliente
- **Gravità:** bassa
- **Dove:** FR-41, FR-29
- **Citazione:** «Il cliente può seguire un'esecuzione in tempo reale: stato di ogni task, attività del fan-out,»
- **Attacco:** quando un workflow dell'integratore gira per un cliente, la vista mostra input, output ed errori di ogni task, compresi i passi interni dell'integratore (indirizzi interni, errori dei worker, dati di altri sistemi).
- **Correzione:** in FR-29: l'integratore può rendere non visibili ai clienti i task di un suo workflow. **Conseguenza verificabile:** un task reso non visibile non mostra input, output ed errori nella vista del cliente.

### B3. Uso dei segreti da parte dei workflow dell'integratore
- **Gravità:** bassa
- **Dove:** FR-29, FR-32
- **Citazione:** «vede solo nome, data e ultimo uso.»
- **Attacco:** un workflow dell'integratore usa i segreti del cliente (FR-29) e il cliente vede solo la data dell'ultimo uso, non chi l'ha usato. Un workflow dell'integratore sbagliato o compromesso che manda la chiave alla destinazione sbagliata non lascia traccia visibile al cliente.
- **Correzione:** in FR-32: l'ultimo uso indica anche workflow e task. **Conseguenza verificabile:** dopo un'esecuzione di un workflow dell'integratore, il segreto mostra al cliente il nome del workflow e del task che l'ha usato.

### B4. Rete in uscita usata contro terzi
- **Gravità:** bassa
- **Dove:** FR-1, FR-3, FR-16, FR-30
- **Citazione:** «Il parallelismo massimo di un fan-out si può limitare nel workflow; senza limite vale la quota»
- **Attacco:** un fan-out da migliaia di `call http`, o nodi con destinazioni in lista, colpisce un servizio terzo dall'indirizzo dell'installazione. Il danno ricade sull'integratore (indirizzo bloccato, segnalazioni di abuso), non su altri clienti.
- **Correzione:** in FR-30: quota per cliente sulle richieste di rete in uscita al secondo. **Conseguenza verificabile:** un fan-out di `call http` oltre la quota viene rallentato al valore della quota.

### B5. Artefatti di rilascio verificabili
- **Gravità:** bassa
- **Dove:** NFR-11, §6
- **Citazione:** «Tutto il codice di topictoast è Apache-2.0»
- **Attacco:** chi integra installa topictoast e il bus accanto ai segreti dei suoi clienti; senza artefatti firmati e un elenco delle dipendenze, un rilascio manomesso o una dipendenza vulnerabile non si riconoscono.
- **Correzione:** ogni rilascio pubblica artefatti firmati e l'elenco delle dipendenze (SBOM). **Conseguenza verificabile:** la guida rapida verifica la firma dell'artefatto prima dell'avvio.

---

## Già coperto, da non perdere

- FR-26: una risorsa di un altro cliente risponde «come se la risorsa non esistesse», niente oracolo; FR-19 fa lo stesso per gli stream.
- FR-13 e FR-30: separazione anche con partizioni del bus condivise. Manca solo un numero per «non rallentano» (per esempio: gli altri clienti restano nella latenza di NFR-2 mentre uno satura la propria quota).
- FR-54: la sandbox forte non passa mai tra clienti (C6 chiede lo stesso per la leggera).
- FR-16 e NFR-8: chiave fuori dalla sandbox, test con codice ostile prima del primo rilascio, test automatico di isolamento orizzontale (C2 lo estende al verticale).
- FR-6 e FR-32: chiave di idempotenza per cliente; valore del segreto non rileggibile dopo il salvataggio.
