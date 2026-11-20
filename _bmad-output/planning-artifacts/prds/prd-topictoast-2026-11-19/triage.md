---
title: topictoast — triage delle revisioni del PRD
created: 2026-11-19
oggetto: prd.md (draft 2026-11-19), review-rubric.md, review-adversarial.md, review-security.md, review-edge-cases.md, .memlog.md
---

# Triage delle revisioni del PRD

Elenco unico, senza doppioni, dei rilievi di gravità critica e alta delle quattro revisioni. Ogni
voce ha una sola classe:

- **sicurezza** — invariante di sicurezza o di perdita di dati, a livello di requisito, che non
  chiede scelte di prodotto: si applica il testo proposto. I meccanismi stanno nella riga «Per
  l'addendum».
- **decisione** — scelta di prodotto del product owner. «già in corso» = domanda già posta, senza
  opzioni qui.
- **rinvio** — appartiene all'architettura o alle spec delle storie, non al PRD.

Regole seguite: nessuna voce rovescia decisioni o override del `.memlog.md` (stream processor nella
v1, pipeline stream dei clienti, nodi precompilati, sandbox su due livelli con pool, obiettivi
numerici); dove le revisioni propongono correzioni diverse per lo stesso buco, il testo contiene
solo l'invariante comune e la divergenza va in nota o diventa una decisione; nessun valore
predefinito singolo (T-20). Citazioni: nome del file più ID del rilievo; la rubric non ha ID e si
cita con sezione, gravità e titolo. Nuovi requisiti: solo NFR-14.

## Indice

| T | Titolo | Classe |
|---|---|---|
| T-1 | Autenticazione dei worker e accesso al bus | sicurezza |
| T-2 | Trasporto cifrato | sicurezza |
| T-3 | Cliente ricavato solo dalla credenziale; controllo verticale | sicurezza |
| T-4 | Rete in uscita per conto dei clienti (`call http` e nodi) | sicurezza |
| T-5 | Logica dei clienti valutata dal motore: jq, byte dei nodi, limiti | sicurezza |
| T-6 | Dialetto SQL delle pipeline stream a lista chiusa | sicurezza |
| T-7 | Sandbox: nessuna convivenza tra clienti, a entrambi i livelli | sicurezza |
| T-8 | Segreti mai in chiaro su disco | sicurezza |
| T-9 | Un'esecuzione resta nel contesto del suo cliente | sicurezza |
| T-10 | Nessuna chiave o credenziale con valore predefinito noto | sicurezza |
| T-11 | Contenuto dei clienti mostrato solo come testo | sicurezza |
| T-12 | Quote su spazio, ingresso e API; arretrato con tetto; disco pieno | sicurezza |
| T-13 | Timeout del task e attesa prima della consegna | sicurezza |
| T-14 | Durabilità: cosa vuol dire accettato e completato | sicurezza |
| T-15 | Idempotenza: durata, concorrenza, righe distinte | sicurezza |
| T-16 | Versioni: nessuna sovrascrittura silenziosa, nodi stabili | sicurezza |
| T-17 | Watermark: eventi dal futuro e stream silenzioso | sicurezza |
| T-18 | Non interferenza tra clienti misurabile | sicurezza |
| T-19 | Eliminazione di un cliente: dove ed entro quando | sicurezza |
| T-20 | Valori predefiniti e limiti (NFR-14) | sicurezza |
| T-21 | Confronto SM-2/NFR-4 con le ~7.700 istruzioni di Hatchet | decisione: già in corso |
| T-22 | Linea di taglio e ripiego se gli spike falliscono | decisione: già in corso |
| T-23 | Modello di identità (FR-27, FR-28, FR-29, FR-45) | decisione: già in corso |
| T-24 | Join e limite di 1 MB | decisione |
| T-25 | Guasto coperto da NFR-6 | decisione |
| T-26 | Rotazione della chiave della cassaforte nella v1 | decisione |
| T-27 | Numero di processi: FR-50 e perimetro di NFR-1 | decisione |
| T-28 | Condizioni di misura degli NFR | decisione |
| T-29 | Sottoinsieme OWS di FR-1 (`call http`, jq) | decisione |
| T-30 | Semantica del DAG con dipendenze fallite o saltate | rinvio |
| T-31 | Meccanismo del bus scritto nei requisiti | rinvio |
| T-32 | Pianificazioni durante un fermo | rinvio |
| T-33 | Limite di 1 MB: definizione esatta | rinvio |
| T-34 | Annullamento: gare con retry e completamento | rinvio |
| T-35 | Worker lento ma vivo: doppio risultato e tentativi | rinvio |
| T-36 | Eventi duplicati e partenza di una nuova versione di query | rinvio |

Conteggio: **sicurezza 20 · decisione 6 nuove + 3 già in corso · rinvio 7**.

---

## Sicurezza

### T-1 · Autenticazione dei worker e accesso al bus

- **Fonti:** review-security.md C1 (critica); review-adversarial.md C1 (critica, prima metà:
  autenticazione); review-rubric.md, Scope honesty, high, «Confine di fiducia verso i worker non
  specificato».
- **Dove nel PRD:** FR-12, FR-33, §3 «Worker», NFR-8.
- **Classe:** sicurezza.
- **Testo — conseguenze da aggiungere a FR-12:**
  - Solo un processo con una credenziale di worker valida, distinta dalle credenziali dei clienti,
    può collegarsi, registrare funzioni e ricevere attività. Un processo senza credenziale, o con
    una credenziale di cliente, che registra una funzione esistente viene rifiutato e non riceve
    alcuna attività né alcun segreto.
  - Chi raggiunge il bus senza una credenziale valida non legge né scrive attività ed eventi.
  - Entrambi i casi entrano nel test di NFR-8.
- **Nota:** la credenziale di worker è distinta da quelle dei clienti con qualunque risposta a
  T-23.
- **Per l'addendum:** una credenziale per worker, revocabile una per una; eventuale limitazione di
  una credenziale di worker a certe funzioni o a certi clienti; forma della credenziale e accesso
  dei worker al bus (diretto o solo tramite il motore) in architettura.

### T-2 · Trasporto cifrato

- **Fonti:** review-security.md A2 (alta); review-adversarial.md C1 (critica, seconda metà:
  cifratura in transito).
- **Dove nel PRD:** NFR-8; FR-27, FR-33, FR-39.
- **Classe:** sicurezza.
- **Testo — da aggiungere a NFR-8, prima dell'elenco dei test:**
  Ogni connessione che porta segreti, credenziali o dati dei clienti (tra worker e motore, tra
  motore e bus, dal browser, dall'SDK e dalla CLI verso il motore) è cifrata e autentica il server;
  una connessione in chiaro è accettata solo da localhost. Con la configurazione predefinita, un
  worker che si collega in chiaro da un altro host viene rifiutato.
- **Per l'addendum:** TLS; certificati generati al primo avvio o forniti dall'integratore, con una
  procedura che resti nei tempi di NFR-13.

### T-3 · Cliente ricavato solo dalla credenziale; controllo verticale

- **Fonti:** review-security.md C2 (critica; punti 1 e 2 e il test verticale; scadenza, revoca e
  ruoli restano in T-23).
- **Dove nel PRD:** FR-26, NFR-8.
- **Classe:** sicurezza.
- **Testo — conseguenza da aggiungere a FR-26:**
  - Il cliente di una richiesta, e ciò che la richiesta può fare, si ricavano solo dalla credenziale
    verificata, mai da un campo della richiesta (percorso, corpo, filtro di una lista): una
    credenziale del cliente A con l'identificativo del cliente B in un parametro agisce su A o viene
    rifiutata.
- **Testo — NFR-8, secondo test:** dopo «che un cliente non legge né modifica risorse di un altro»
  aggiungere «, e che una credenziale di cliente riceve un rifiuto su ogni operazione riservata
  all'integratore».
- **Nota:** vale con qualunque modello di identità scelto in T-23.
- **Per l'addendum:** nessun meccanismo da fissare qui.

### T-4 · Rete in uscita per conto dei clienti (`call http` e nodi)

- **Fonti:** review-security.md C3 (critica); review-adversarial.md C3 (critica);
  review-edge-cases.md A13 (alta, parte `call http`). Assorbite: review-security.md M1 (media),
  review-edge-cases.md M7 (media), review-rubric.md, Done-ness clarity, medium, «Formato del nodo
  personalizzato non definito» (parte «chi approva le destinazioni»).
- **Dove nel PRD:** FR-16 (terza conseguenza), FR-1 (`call http`), NFR-8, §12.
- **Classe:** sicurezza.
- **Testo — sostituisce in FR-16** «L'accesso alla rete esterna è negato salvo una lista di
  destinazioni dichiarata nel nodo `[ASSUMPTION]`.» **con:**
  - Ogni connessione di rete aperta per conto di un cliente, da un nodo personalizzato o da una
    `call http`, raggiunge solo le destinazioni che l'integratore consente a quel cliente; la lista
    dichiarata in un nodo può restringerle, mai allargarle. Loopback, indirizzi privati, link-local
    e indirizzi dell'installazione (motore, bus, database) restano irraggiungibili anche quando un
    nome consentito vi si risolve o un redirect vi porta. Una `call http` verso `127.0.0.1`,
    `169.254.169.254` o un nome che si risolve a un indirizzo privato fallisce con un errore di
    politica di rete senza aprire la connessione; il caso entra nel test di NFR-8.
- **Nota:** il valore predefinito delle destinazioni consentite ricade in NFR-14 (T-20); la voce
  di FR-16 nel §12 cambia di conseguenza.
- **Per l'addendum:** controllo sull'indirizzo IP risolto a ogni connessione e a ogni redirect
  (contro il DNS rebinding); `call http` con lo stesso filtro dei nodi, dentro o fuori dal processo
  della chiave (vedi T-27); dimensione massima della risposta in NFR-14.

### T-5 · Logica dei clienti valutata dal motore: jq, byte dei nodi, limiti

- **Fonti:** review-security.md C4 (critica); review-adversarial.md C2 (critica, punti 1 e 3 e
  correzione di FR-17); review-security.md A6 (alta); review-edge-cases.md A13 (alta, parte jq e
  ricorsione dei sotto-workflow).
- **Dove nel PRD:** FR-1 (espressioni jq, sotto-workflow), FR-4, FR-16, FR-17, NFR-7.
- **Classe:** sicurezza.
- **Testo — enunciato di FR-17, sostituisce** «Ogni esecuzione di un nodo personalizzato ha limiti
  di CPU, memoria e tempo.» **con:**
  Ogni esecuzione di un nodo personalizzato e ogni espressione jq di un cliente ha limiti di CPU,
  memoria e tempo; ogni query di pipeline stream ha limiti di CPU e di memoria, compresa la memoria
  di esecuzione oltre a quella di stato (FR-25).
- **Testo — prima conseguenza di FR-17, sostituisce** «Un nodo che supera un limite viene fermato
  e l'attività fallisce con un errore che nomina il limite superato.» **con:**
  - Superare un limite fa fallire l'attività con un errore che nomina il limite superato, oppure
    rallenta o sospende la pipeline stream con un errore visibile al cliente e all'operatore; il
    processo del motore non si ferma e le attività degli altri clienti restano entro l'obiettivo
    di NFR-2.
- **Testo — conseguenza da aggiungere a FR-17:**
  - Un'espressione jq non legge l'ambiente del processo, il file system, moduli esterni né la rete:
    un'espressione che usa `$ENV`, `env`, `input_filename`, `import` o `include` viene rifiutata
    alla validazione (FR-4).
- **Testo — conseguenza da aggiungere a FR-16:**
  - Il trattamento dei byte di un nodo caricato (validazione, compilazione, istanziazione) non
    raggiunge la chiave della cassaforte: un modulo costruito per far crashare il validatore non
    ferma il motore né le esecuzioni in corso.
- **Nota:** i passi di controllo senza fine (`then` all'indietro, sotto-workflow ricorsivi) li
  chiude NFR-14 (T-20): timeout dell'esecuzione sempre presente e con un massimo, profondità di
  ricorsione limitata.
- **Per l'addendum:** interprete jq senza accesso ad ambiente e file, oppure valutazione fuori dal
  processo della chiave; validazione e compilazione dei moduli fuori dal processo della chiave, come
  la loro esecuzione (FR-16). Ogni processo in più ricade in T-27.

### T-6 · Dialetto SQL delle pipeline stream a lista chiusa

- **Fonti:** review-security.md C5 (critica); review-adversarial.md C2 (critica, punto 2);
  review-edge-cases.md A16 (alta).
- **Dove nel PRD:** FR-19, FR-25, NFR-7, NFR-8.
- **Classe:** sicurezza.
- **Testo — conseguenze da aggiungere a FR-19:**
  - Il dialetto delle pipeline stream dei clienti è una lista chiusa e documentata: interrogazioni
    sugli stream del cliente con le funzioni elencate. DDL, tabelle esterne, connettori, lettura o
    scrittura di file, accesso alla rete e funzioni definite dall'utente vengono rifiutati al
    salvataggio con un errore di validazione. Un insieme di query ostili (tabella esterna su
    `/etc/passwd`, `COPY` verso un file, connettore verso l'indirizzo del bus, funzione definita
    dall'utente) entra nel test di NFR-8.
  - Rielaborata dal checkpoint sugli stessi eventi, una pipeline stream emette le stesse righe
    (condizione di FR-24).
  - Un ciclo tra pipeline stream dello stesso cliente (una scrive sullo stream da cui un'altra
    legge, fino a tornare alla prima) viene rifiutato al salvataggio.
- **Testo — conseguenza da aggiungere a FR-25:**
  - Una pipeline stream oltre la quota di CPU viene rallentata o sospesa con un errore visibile al
    cliente e all'operatore.
- **Per l'addendum:** lista bianca di funzioni sul motore SQL scelto dallo spike; funzioni non
  deterministiche rifiutate o legate al tempo dell'evento (scelta in architettura); nessun accesso
  a cataloghi, sorgenti o destinazioni del motore sottostante.

### T-7 · Sandbox: nessuna convivenza tra clienti, a entrambi i livelli

- **Fonti:** review-security.md C6 (critica). Assorbita: review-adversarial.md M14 (media, seconda
  parte: segreti in transito nella sandbox condivisa).
- **Dove nel PRD:** FR-16, FR-54, NFR-8.
- **Classe:** sicurezza.
- **Testo — conseguenza da aggiungere a FR-16:**
  - A entrambi i livelli, un'istanza della sandbox, il suo processo e la memoria che ha usato non
    contengono mai dati o segreti di due clienti, né insieme né uno dopo l'altro. Nel test di
    NFR-8, mentre gira un nodo del cliente B con un segreto sentinella, il processo che esegue il
    nodo del cliente A non contiene quel segreto.
- **Nota:** compatibile con la sandbox forte riservata a un cliente (`.memlog.md`, riga 23); la
  seconda conseguenza di FR-54 resta. La pulizia tra attività dello stesso cliente
  (review-security.md M2, media) non è inclusa: vedi i temi in fondo.
- **Per l'addendum:** istanze monouso o un processo per cliente nella sandbox leggera; costo in RAM
  e tempo di avvio da misurare nello spike (NFR-12).

### T-8 · Segreti mai in chiaro su disco

- **Fonti:** review-adversarial.md C4 (critica); review-security.md A1 (alta).
- **Dove nel PRD:** FR-34, FR-33, §1.
- **Classe:** sicurezza.
- **Testo — sostituisce in FR-34** «Una copia del database senza la chiave di cifratura non rivela
  alcun segreto.» **con:**
  - Nessun supporto persistente di topictoast (database, bus, checkpoint, log, file temporanei)
    contiene un segreto in chiaro: una copia di questi supporti senza la chiave di cifratura non
    rivela alcun segreto. Dopo un'esecuzione con un segreto sentinella, una ricerca del valore nelle
    directory dati di topictoast, del bus e dei checkpoint non trova nulla.
- **Per l'addendum:** segreto risolto alla consegna e mai scritto nel messaggio sul bus, oppure
  cifrato dentro il messaggio; scelta in architettura.

### T-9 · Un'esecuzione resta nel contesto del suo cliente

- **Fonti:** review-security.md A3 (alta); review-security.md A4 (alta); review-security.md A5
  (alta). Assorbita: review-edge-cases.md M4 (media).
- **Dove nel PRD:** FR-26, FR-4, FR-18, FR-29, FR-37, FR-40, FR-6, FR-24.
- **Classe:** sicurezza.
- **Testo — conseguenze da aggiungere a FR-26:**
  - Un'esecuzione resta nel contesto del suo cliente. Funzioni, nodi personalizzati e
    sotto-workflow citati da un workflow si risolvono tra le risorse del proprietario del workflow
    e, se il proprietario è un cliente, tra quelle che l'integratore gli rende disponibili; i
    segreti si prendono dal cliente dell'esecuzione; nessun riferimento si risolve mai tra le
    risorse di un altro cliente. Un workflow scrive
    solo sugli stream del cliente dell'esecuzione; un worker, mentre serve un'attività, scrive solo
    sugli stream del cliente di quell'attività. Una scrittura verso lo stream di un altro cliente
    riceve lo stesso errore di uno stream inesistente e a quel cliente non arriva alcun evento.
  - Ogni trigger (pianificazione, pipeline stream) ha un cliente fissato alla creazione e avvia
    esecuzioni solo nel suo contesto.
- **Testo — conseguenza da aggiungere a FR-4:**
  - L'integratore dichiara quali funzioni sono invocabili dai workflow dei clienti. Un documento di
    un cliente che invoca una funzione non disponibile viene rifiutato all'invio come se la
    funzione non esistesse, e il controllo si ripete all'esecuzione.
- **Testo — se FR-29 resta dopo T-23, conseguenza da aggiungere a FR-29:**
  - Un cliente che crea un nodo personalizzato o un sotto-workflow con lo stesso nome di uno usato
    da un workflow dell'integratore non cambia ciò che quel workflow esegue quando gira per lui.
- **Nota:** il valore predefinito della disponibilità delle funzioni ricade in NFR-14 (T-20).
- **Per l'addendum:** credenziale legata all'attività per le scritture dei worker; la
  documentazione per l'integratore chiede di ricavare il cliente dal contesto della funzione
  (FR-37), mai dall'input.

### T-10 · Nessuna chiave o credenziale con valore predefinito noto

- **Fonti:** review-security.md A7 (alta); review-adversarial.md A12 (alta).
- **Dove nel PRD:** FR-50; FR-27, FR-34, FR-45, NFR-13.
- **Classe:** sicurezza.
- **Testo — conseguenza da aggiungere a FR-50:**
  - Nessuna chiave o credenziale che protegge identità, segreti o dashboard ha un valore predefinito
    noto: due installazioni nuove hanno valori diversi, e topictoast rifiuta di partire con un
    valore presente negli esempi della documentazione. Con la configurazione predefinita topictoast
    accetta connessioni solo da localhost.
- **Nota:** formulato senza nominare il meccanismo di FR-27 e FR-45, quindi vale con qualunque
  risposta a T-23.
- **Per l'addendum:** generazione casuale al primo avvio con i valori mostrati una volta, oppure
  rifiuto di partire finché non vengono forniti; la guida rapida resta nei tempi di NFR-13.

### T-11 · Contenuto dei clienti mostrato solo come testo

- **Fonti:** review-security.md A8 (alta).
- **Dove nel PRD:** NFR-7; FR-41, FR-46, FR-47, FR-48.
- **Classe:** sicurezza.
- **Testo — da aggiungere a NFR-7:**
  Il contenuto fornito dai clienti (nomi, descrizioni, errori, input, output, righe emesse) è
  mostrato sempre come testo, mai interpretato come HTML o script, nella dashboard e nei componenti
  React. Un test salva un payload di script in ogni campo testuale di un cliente, apre ogni pagina
  della dashboard e ogni componente, e verifica che nessuno script venga eseguito.
- **Per l'addendum:** CSP e nessun HTML grezzo, secondo la skill `dashboard-react`.

### T-12 · Quote su spazio, ingresso e API; arretrato con tetto; disco pieno

- **Fonti:** review-adversarial.md C5 (critica); review-security.md A9 (alta);
  review-edge-cases.md A17 (alta); review-edge-cases.md A15 (alta, parte conservazione degli
  stream). Assorbite: review-security.md M10 (media, parte conservazione), review-edge-cases.md M2
  (media, parte del pool per cliente).
- **Dove nel PRD:** FR-30, FR-18, FR-23, FR-24, FR-54, NFR-6.
- **Classe:** sicurezza.
- **Testo — enunciato di FR-30, sostituisce** «L'integratore assegna a ogni cliente quote su
  attività concorrenti, esecuzioni al minuto, risorse della sandbox e risorse delle pipeline
  stream.» **con:**
  L'integratore assegna a ogni cliente quote su attività concorrenti, esecuzioni al minuto, risorse
  della sandbox (compresa la sua parte del pool della sandbox forte), risorse delle pipeline stream,
  eventi scritti al secondo, richieste API al secondo e spazio occupato (stream, storico, code, nodi
  caricati).
- **Testo — conseguenze da aggiungere a FR-30:**
  - L'arretrato di un cliente (attività ed esecuzioni in coda, righe in attesa di FR-24, eventi
    conservati) ha un tetto. Oltre il tetto o la quota di spazio, una nuova richiesta riceve un
    errore di quota prima di essere accettata, e una pipeline stream smette di emettere
    segnalandolo con un errore visibile al cliente e all'operatore; il lavoro già accettato non va
    perso.
  - Un cliente che scrive eventi oltre la sua quota riceve errori di quota, mentre gli altri
    continuano a scrivere e ad avviare esecuzioni entro l'obiettivo di NFR-2.
- **Testo — conseguenza da aggiungere a FR-18:**
  - Ogni stream ha una conservazione per durata o per dimensione, con valore predefinito
    documentato (NFR-14).
- **Testo — sostituisce in FR-23** «Una pipeline sospesa non consuma CPU; alla ripresa elabora gli
  eventi arrivati nel frattempo.» **con:**
  - Una pipeline stream sospesa non consuma CPU; alla ripresa elabora gli eventi arrivati nel
    frattempo e ancora conservati. Se una parte è già uscita dalla conservazione, la pipeline
    stream lo segnala con un errore visibile al cliente e all'operatore, mai in silenzio.
- **Testo — sostituisce in FR-24** «Il numero di esecuzioni avviate da una pipeline rispetta la
  quota del cliente (FR-30); le righe in eccesso restano in attesa, non vanno perse.» **con:**
  - Il numero di esecuzioni avviate da una pipeline stream rispetta la quota del cliente (FR-30);
    le righe in eccesso restano in attesa entro il tetto dell'arretrato e non vanno perse.
- **Testo — da aggiungere a NFR-6:** A disco pieno topictoast rifiuta lavoro nuovo con un errore
  esplicito e non perde quello accettato.
- **Per l'addendum:** valori di quote, tetti e conservazione nelle spec delle storie (NFR-14);
  come il motore misura lo spazio per cliente sul bus: architettura.

### T-13 · Timeout del task e attesa prima della consegna

- **Fonti:** review-edge-cases.md A4 (alta); review-adversarial.md A7 (alta).
- **Dove nel PRD:** FR-8, FR-12, FR-30, FR-54, UJ-1.
- **Classe:** sicurezza.
- **Testo — conseguenza da aggiungere a FR-8:**
  - Il timeout del task conta dalla consegna al worker o alla sandbox. L'attesa prima della
    consegna (nessun worker collegato, cliente oltre quota, pool della sandbox vuoto) non fa
    fallire l'attività per timeout del task e non consuma tentativi; resta comunque limitata da un
    tempo documentato (NFR-14). Un'attività rimasta in coda per quota più a lungo del suo timeout
    del task, una volta consegnata, si completa normalmente.
- **Testo — sostituisce in FR-12** «restano in attesa, entro il timeout del task, e partono quando
  un worker la registra» **con** «restano in attesa, entro il limite di attesa di FR-8, e partono
  quando un worker la registra».
- **Testo — sostituisce in FR-54** «le attività attendono una sandbox libera entro il timeout del
  task» **con** «le attività attendono una sandbox libera entro il limite di attesa di FR-8».
- **Nota:** le due revisioni concordano sull'invariante e divergono sul limite dell'attesa:
  review-edge-cases.md A4 vuole un timeout di attesa separato, review-adversarial.md A7 il solo
  timeout dell'esecuzione. La scelta va nella spec della storia.
- **Per l'addendum:** due orologi per attività (attesa e esecuzione) se la spec sceglie il timeout
  di attesa separato.

### T-14 · Durabilità: cosa vuol dire accettato e completato

- **Fonti:** review-edge-cases.md A7 (alta); review-adversarial.md A11 (alta, parte «il benchmark
  non lo misura»); review-rubric.md, Done-ness clarity, high, «Condizioni di benchmark non pinnano
  la durabilità» (parte NFR-6). Assorbita: review-adversarial.md M2 (media).
- **Dove nel PRD:** FR-9, NFR-6, FR-53, SM-C1.
- **Classe:** sicurezza.
- **Testo — sostituisce in FR-9** «Dopo un riavvio, nessun task completato viene rieseguito e
  nessuna attività accettata va persa.» **con:**
  - Un'esecuzione, un'attività o un evento risulta accettato solo quando la risposta parte dopo che
    è al sicuro dal guasto dichiarato in NFR-6. Dopo quel guasto nessuna esecuzione, attività o
    evento accettato va perso, e nessun task il cui completamento è stato registrato viene
    rieseguito; un'attività consegnata senza esito registrato può essere consegnata di nuovo
    (consegna almeno una volta).
- **Testo — uscita da aggiungere a FR-53:** attività, esecuzioni ed eventi accettati e poi persi
  dopo un guasto iniettato durante i test di NFR-3 e NFR-5 (obiettivo: zero, NFR-6).
- **Nota:** quale guasto vale (crash del processo o anche spegnimento dell'host) è T-25.
- **Per l'addendum:** modalità di scrittura (sincrona, a gruppi) dichiarata nel benchmark, dopo
  T-25.

### T-15 · Idempotenza: durata, concorrenza, righe distinte

- **Fonti:** review-edge-cases.md A2 (alta). Assorbite: review-adversarial.md M8 (media),
  review-rubric.md, Done-ness clarity, medium, «Idempotenza ed esattamente-una-volta senza durata».
- **Dove nel PRD:** FR-6, FR-24, FR-31.
- **Classe:** sicurezza.
- **Testo — sostituisce in FR-6** «Un avvio con una chiave di idempotenza già usata dallo stesso
  cliente restituisce l'esecuzione esistente invece di crearne una nuova.» **con:**
  - Un avvio con una chiave di idempotenza già usata dallo stesso cliente restituisce l'esecuzione
    esistente invece di crearne una nuova, anche quando le richieste arrivano in contemporanea, per
    almeno la durata di conservazione dell'esecuzione (FR-31). Cento richieste concorrenti con la
    stessa chiave producono una sola esecuzione.
- **Testo — sostituisce in FR-24** «Con destinazione workflow, ogni riga avvia esattamente
  un'esecuzione anche dopo un riavvio dal checkpoint (chiave di idempotenza derivata dalla riga,
  FR-6).» **con:**
  - Con destinazione workflow, ogni riga avvia esattamente un'esecuzione anche dopo un riavvio dal
    checkpoint, e due righe distinte con lo stesso contenuto (per esempio lo stesso conteggio in
    due finestre) avviano due esecuzioni.
- **Per l'addendum:** chiave composta da pipeline stream, versione e finestra o posizione nello
  stream; stessa chiave con input diverso (restituire l'esistente o errore) nella spec della storia.

### T-16 · Versioni: nessuna sovrascrittura silenziosa, nodi stabili

- **Fonti:** review-edge-cases.md A11 (alta); review-edge-cases.md A12 (alta). Assorbita:
  review-security.md M13 (media).
- **Dove nel PRD:** FR-5, FR-15, FR-40.
- **Classe:** sicurezza.
- **Testo — conseguenza da aggiungere a FR-5:**
  - Una pubblicazione preparata su una versione che nel frattempo non è più l'ultima viene
    rifiutata con un errore che indica la versione corrente; non sostituisce mai in silenzio il
    lavoro di un altro.
- **Testo — conseguenza da aggiungere a FR-15:**
  - Caricare un nodo con un nome già usato crea una versione nuova: un'esecuzione in corso esegue
    fino alla fine la versione del nodo con cui è partita, e ogni attività registra quale versione
    ha eseguito.
- **Per l'addendum:** controllo ottimistico sulla versione di base; versione del nodo identificata
  dall'impronta del contenuto. Ripubblicazione identica, versioni dei sotto-workflow, eliminazione
  di un nodo in uso: spec delle storie.

### T-17 · Watermark: eventi dal futuro e stream silenzioso

- **Fonti:** review-edge-cases.md A14 (alta). Assorbite: review-security.md M7 (media),
  review-adversarial.md M10 (media).
- **Dove nel PRD:** FR-21, FR-18, UJ-3.
- **Classe:** sicurezza.
- **Testo — conseguenze da aggiungere a FR-21:**
  - Un evento con tempo dell'evento oltre una tolleranza nel futuro rispetto all'arrivo non sposta
    il watermark e non fa scartare gli eventi successivi; è contato in una metrica visibile nella
    dashboard. Un evento datato un anno nel futuro non chiude alcuna finestra.
  - Su uno stream che smette di ricevere eventi, le finestre aperte si chiudono comunque entro un
    tempo documentato (NFR-14): l'ultima finestra di un'ondata emette le sue righe (UJ-3).
- **Nota:** le tre revisioni divergono su cosa fare dell'evento (rifiuto all'ingresso, scarto, tempo
  di arrivo al posto del tempo dichiarato): spec della storia.
- **Per l'addendum:** avanzamento del watermark di uno stream inattivo con l'orologio di sistema.

### T-18 · Non interferenza tra clienti misurabile

- **Fonti:** review-adversarial.md A5 (alta). Assorbita: review-rubric.md, Done-ness clarity,
  medium, «Aggettivi di non-interferenza senza soglia».
- **Dove nel PRD:** FR-13, FR-17, FR-25, FR-30, UJ-4, FR-53.
- **Classe:** sicurezza.
- **Testo — sostituisce in FR-13** «Un worker lento non rallenta le attività consegnate ad altri
  worker, anche quando condividono la stessa partizione del bus.» **con:**
  - Mentre un worker è lento o bloccato, le attività consegnate ad altri worker restano entro
    l'obiettivo di NFR-2, anche quando sono accodate insieme alle sue.
- **Testo — sostituisce in FR-17** «Un nodo fermato non lascia processi né memoria occupata e non
  rallenta gli altri clienti.» **con:**
  - Un nodo fermato non lascia processi né memoria occupata; mentre un nodo satura i propri limiti,
    le attività degli altri clienti restano entro l'obiettivo di NFR-2.
- **Testo — conseguenza da aggiungere a FR-25:**
  - Mentre una pipeline stream di un cliente è al limite delle sue quote, le pipeline stream degli
    altri clienti elaborano gli eventi al ritmo con cui arrivano, senza ritardo crescente sul
    watermark, e le attività degli altri clienti restano entro l'obiettivo di NFR-2.
- **Testo — sostituisce in FR-30** «Un cliente oltre quota vede le sue nuove attività in coda; gli
  altri clienti non rallentano, anche quando condividono partizioni del bus (UJ-4).» **con:**
  - Un cliente oltre quota vede le sue nuove attività in coda; le attività degli altri clienti
    restano entro l'obiettivo di NFR-2, anche quando sono accodate insieme alle sue (UJ-4).
- **Testo — uscita da aggiungere a FR-53:** una corsa a carico misto, con la latenza di NFR-2
  misurata mentre una pipeline stream gira a un ritmo dichiarato e un cliente è oltre quota.
- **Nota:** la soglia riusa l'obiettivo di NFR-2, come review-security.md C4 e A9; nessun numero
  nuovo. La rubric citava come esempio una soglia relativa. Le sostituzioni tolgono anche le
  «partizioni del bus» (T-31).
- **Per l'addendum:** nessuno.

### T-19 · Eliminazione di un cliente: dove ed entro quando

- **Fonti:** review-adversarial.md A13 (alta). Assorbita: review-security.md M10 (media, parte
  eliminazione); sequenza di arresto da review-edge-cases.md A17 (alta, già in T-12).
- **Dove nel PRD:** FR-31.
- **Classe:** sicurezza.
- **Testo — sostituisce in FR-31** «Eliminare un cliente elimina tutte le sue risorse.» **con:**
  - Eliminare un cliente ferma prima le sue esecuzioni, pipeline stream e pianificazioni, poi rende
    i suoi dati illeggibili in ogni supporto (database, bus, checkpoint, nodi caricati, sandbox
    riservate) entro un tempo massimo documentato. Passato quel tempo, una ricerca di un valore
    sentinella del cliente nelle directory dati di topictoast e del bus non trova nulla; un
    identificativo di cliente eliminato non viene riusato.
- **Per l'addendum:** stream separati per cliente o cancellazione crittografica (una chiave per
  cliente); scelta in architettura.

### T-20 · Valori predefiniti e limiti (NFR-14)

- **Fonti:** review-edge-cases.md A3 (alta); review-edge-cases.md A5 (alta, parte numero massimo di
  elementi); review-edge-cases.md A13 (alta, parte limiti di documento, task, profondità e risposta
  HTTP); review-edge-cases.md A10 (alta, parte «nessuna attesa senza fine»); review-adversarial.md
  C2 (critica, correzione di FR-8: timeout predefinito e massimo); review-adversarial.md A8 (alta,
  parte numero massimo di elementi); review-adversarial.md A9 (alta, parte esecuzione appesa).
  Assorbite: review-security.md M11 (media), review-adversarial.md M3 (media, parte retry
  predefinito), review-rubric.md, Done-ness clarity, medium, «FR-47, FR-48, FR-49 senza
  conseguenze» (parte valori predefiniti di FR-14 e FR-17).
- **Dove nel PRD:** §5 «Altri requisiti trasversali» (nuovo NFR-14); FR-2; FR-7, FR-8, FR-13,
  FR-14, FR-17, FR-21, FR-22, FR-30, FR-3, FR-6.
- **Classe:** sicurezza.
- **Testo — da aggiungere al §5, «Altri requisiti trasversali»:**
  - **NFR-14. Valori predefiniti e limiti.** Ogni campo che un workflow, una pipeline stream, un
    worker, un nodo personalizzato o la configurazione di un nuovo cliente può omettere ha un valore
    predefinito documentato e limitato, applicato dopo la validazione. Vale almeno per: retry, timeout del task
    e dell'esecuzione, limite di attesa prima della consegna, intervallo di battito dei worker,
    concorrenza di un worker, limiti dei nodi personalizzati, ritardo tollerato e tolleranza sul
    tempo dell'evento, chiusura delle finestre di uno stream inattivo, intervallo dei checkpoint,
    conservazione degli stream, recupero delle scadenze perse di una pianificazione, quote di un
    nuovo cliente, funzioni e destinazioni di rete disponibili ai clienti. Ogni tempo e ogni limite
    che un cliente può dichiarare ha un massimo fissato dall'installazione, e nessuna combinazione
    di valori predefiniti e dichiarati lascia un'esecuzione, un'attività o una finestra in attesa
    senza fine. Le grandezze che un cliente controlla (elementi di un fan-out, dimensione del
    documento e della query, numero di task, profondità di annidamento e di ricorsione dei
    sotto-workflow, dimensione della risposta di una `call http`) hanno un limite con valore
    predefinito documentato: superarlo è un errore di validazione o fa fallire il task con limite e
    valore, senza creare attività e senza fermare il motore. Un workflow senza campi facoltativi
    gira con i valori predefiniti documentati e termina; una pipeline stream senza campi
    facoltativi gira con i valori predefiniti documentati e chiude le sue finestre.
- **Testo — conseguenza da aggiungere a FR-2:**
  - Un task la cui dipendenza è fallita o è stata saltata non resta in attesa: raggiunge uno stato
    terminale secondo una regola documentata.
- **Nota:** nessun valore nel PRD. La regola di propagazione è T-30.
- **Per l'addendum:** ogni spec di storia fissa i valori dei campi che tocca e li riporta nella sua
  pagina di `docs/user/`; il caso «workflow minimo senza campi facoltativi» diventa uno scenario BDD.

---

## Decisioni già in corso

### T-21 · Confronto SM-2/NFR-4 con le ~7.700 istruzioni di Hatchet

- **Fonti:** review-adversarial.md A1 (alta); review-rubric.md, Done-ness clarity, high,
  «Condizioni di benchmark non pinnano la durabilità» (parte normalizzazione di SM-2).
- **Dove nel PRD:** §1, NFR-4, SM-2.
- **Classe:** decisione: già in corso.
- **Nota:** dalla risposta dipende anche la definizione di «istruzione SQL» e della forma del
  carico di NFR-4 (review-edge-cases.md M14, media).

### T-22 · Linea di taglio e ripiego se gli spike falliscono

- **Fonti:** review-rubric.md, Decision-readiness, high, «Nessuna linea di taglio né ripiego per v1
  e obiettivi»; review-adversarial.md A14 (alta).
- **Dove nel PRD:** §8.1, §10 punti 5 e 6, §1.1 punto 5.
- **Classe:** decisione: già in corso.

### T-23 · Modello di identità (FR-27, FR-28, FR-29, FR-45)

- **Fonti:** review-security.md C2 (critica; parti che dipendono dal modello: ruolo nella
  credenziale, scadenza, destinatario, revoca); review-rubric.md, Scope honesty, high, «Fondamenta
  di sicurezza lasciate come assunzioni» (parte FR-27 e FR-28); review-adversarial.md A10 (alta:
  credenziali della CLI dei clienti, scadenza, ruolo, revoca).
- **Dove nel PRD:** FR-27, FR-28, FR-29, FR-45; §4.8; UJ-2.
- **Classe:** decisione: già in corso.
- **Nota:** T-3, T-9 (salvo la frase su FR-29) e T-10 valgono con qualunque risposta.

---

## Decisioni nuove

### T-24 · Join e limite di 1 MB

- **Fonti:** review-rubric.md, Done-ness clarity, high, «FR-11 (1 MB) contro FR-3 e FR-41»;
  review-adversarial.md A8 (alta); review-edge-cases.md A5 (alta, parte join).
- **Dove nel PRD:** FR-11, FR-3, FR-41, UJ-2.
- **Classe:** decisione.
- **Domanda:** il limite di 1 MB di FR-11 vale anche per l'input aggregato del join?
- **Opzioni:**
  1. **Limite per elemento più tetto aggregato del join**, configurabile con valore predefinito
     documentato; oltre, il join fallisce con dimensione e limite. UJ-2 regge; il picco di RAM di
     un join cresce e va riportato sotto carico (T-28).
  2. **1 MB anche per il join intero.** Semplice e coerente con la RAM minima; UJ-2 (300
     arricchimenti DNS e Censys) fallisce se gli output non restano piccoli.
  3. **Join che riceve i risultati a pagine o per riferimento.** Nessun tetto aggregato; contratto
     pubblico nuovo (§6) e più lavoro nella v1, naturale quando arriva lo storage per payload
     grandi.
- **Raccomandazione:** opzione 1 nella v1, opzione 3 con lo storage per payload grandi. In tutte le
  opzioni il numero massimo di elementi di un fan-out è un limite di NFR-14 (T-20), e la misura
  esatta del limite è T-33.

### T-25 · Guasto coperto da NFR-6

- **Fonti:** review-rubric.md, Done-ness clarity, high, «Condizioni di benchmark non pinnano la
  durabilità» (parte modalità di flush); review-adversarial.md A11 (alta, parte «crash non
  definito» e backup); review-edge-cases.md A7 (alta, parte momento dell'accettazione).
- **Dove nel PRD:** NFR-6, §3 «Macchina di riferimento», §5, §8.2, SM-C1.
- **Classe:** decisione.
- **Domanda:** NFR-6 vale per il crash di un processo (`kill -9`) o anche per lo spegnimento
  improvviso dell'host?
- **Opzioni:**
  1. **Entrambi.** Coerente con SM-C1 (durabilità prima del carico sul database); NFR-2 e NFR-3
     dipendono dalla latenza delle scritture sincrone, quindi la macchina di riferimento deve dire
     «SSD NVMe locale» (review-adversarial.md B3).
  2. **Solo crash del processo**, con lo spegnimento dell'host come limite documentato e la
     finestra di perdita dichiarata. Numeri più facili; promessa più debole, da scrivere nel
     benchmark pubblico.
  3. **Configurabile**, con l'opzione 1 come predefinita e il benchmark pubblicato in quella
     modalità. Flessibile; due modalità da provare.
- **Raccomandazione:** opzione 1. Da confermare insieme: backup e ripristino coerenti di database e
  bus fuori dalla v1, documentati come limite in §8.2 (review-adversarial.md A11). In tutte le
  opzioni il benchmark dichiara la modalità di scrittura usata.

### T-26 · Rotazione della chiave della cassaforte nella v1

- **Fonti:** review-security.md A10 (alta, rotazione nella v1); review-adversarial.md M13 (media,
  rotazione fuori dalla v1); review-rubric.md, Scope honesty, medium, «Gestione della chiave della
  cassaforte non coperta».
- **Dove nel PRD:** FR-34, §8.2.
- **Classe:** decisione.
- **Domanda:** la rotazione della chiave della cassaforte entra nella v1?
- **Opzioni:**
  1. **Sì:** si ruota senza fermare il motore e senza perdere segreti, e dopo la rotazione la
     chiave vecchia non decifra nulla di ciò che è salvato. Una chiave trapelata si neutralizza;
     più lavoro e test nella v1.
  2. **No, dopo la v1.** Meno perimetro; una chiave trapelata obbliga ogni cliente a reinserire
     tutti i segreti.
- **Raccomandazione:** opzione 1: la cassaforte è il punto 4 di §1.1 e il lancio è pubblico; con
  una chiave principale che cifra le chiavi dei dati, la rotazione riscrive poche chiavi (per
  l'addendum).
- **In tutte le opzioni, conseguenza da aggiungere a FR-34:**
  - La chiave di cifratura non sta nel database né nelle sue copie; cosa succede se si perde è
    documentato per l'integratore.

### T-27 · Numero di processi: FR-50 e perimetro di NFR-1

- **Fonti:** review-adversarial.md A2 (alta). Collegate: T-4, T-5, T-7 possono aggiungere processi
  che eseguono logica dei clienti.
- **Dove nel PRD:** FR-50, NFR-1, §1.1 «Cosa non lo distingue», §8.1; FR-16, §1.2 punto 8.
- **Classe:** decisione (ridefinisce cosa misura un obiettivo deciso: `.memlog.md`, riga 20).
- **Domanda:** NFR-1 conta solo il motore e il bus, o tutti i processi che topictoast avvia?
- **Opzioni:**
  1. **Il gruppo «processi di topictoast» più il bus**: il motore e ogni processo che avvia, a
     riposo (sandbox leggera e, se l'architettura li separa, processi che eseguono jq, query delle
     pipeline stream e `call http`); esclusi worker, stato delle pipeline stream e pool della
     sandbox forte, come già deciso. Misura onesta, non si aggira spostando lavoro nei processi
     figli; gli 80 MB diventano più difficili.
  2. **Solo motore e bus**, con i processi figli riportati a parte senza obiettivo. Gli 80 MB
     restano raggiungibili; il numero si abbassa spostando lavoro nei figli.
- **Raccomandazione:** opzione 1.
- **Testo, uguale in entrambe le opzioni:**
  - Enunciato di FR-50, sostituisce «Nella v1 l'integratore installa topictoast con due processi,
    topictoast e il bus, più il database a scelta.» con: «Nella v1 l'integratore installa due
    servizi, topictoast e il bus, più il database a scelta. topictoast avvia e ferma da sé i
    processi di cui ha bisogno (la sandbox leggera e, se l'architettura li separa, i processi che
    eseguono espressioni jq, query delle pipeline stream e `call http`), che non si installano né
    si configurano a parte.» La frase sul numero di processi che non è la differenza resta.
  - Prima conseguenza di FR-50, sostituisce «Con SQLite incorporato, topictoast e il bus sono gli
    unici processi.» con: «Con SQLite incorporato, topictoast e il bus sono gli unici servizi da
    installare e avviare; ogni altro processo lo avvia e lo ferma topictoast.»
  - §1.1, sostituisce «Con SQLite topictoast ha due processi» con «Con SQLite topictoast si
    installa con due servizi».
  - §8.1, sostituisce «Installazione con due processi più il database» con «Installazione con due
    servizi più il database».
- **Testo — cella «Come si misura» di NFR-1:**
  - Opzione 1: «somma dell'RSS dei processi di topictoast (il motore e ogni processo che avvia,
    compresa la sandbox leggera, a riposo) e del bus, con SQLite, 60 s dopo l'avvio, nessuna esecuzione né pipeline stream attiva;
    esclusi worker, stato delle pipeline stream e pool della sandbox forte (FR-54)».
  - Opzione 2: «RSS del motore e del bus, con SQLite, 60 s dopo l'avvio, nessuna esecuzione né
    pipeline stream attiva; esclusi worker e stato delle pipeline stream; i processi avviati da
    topictoast riportati a parte, senza obiettivo».

### T-28 · Condizioni di misura degli NFR

- **Fonti:** review-adversarial.md A3 (alta); review-adversarial.md A4 (alta).
- **Dove nel PRD:** NFR-1, NFR-3, NFR-5, FR-53, §1 e §1.1 punto 3.
- **Classe:** decisione (nessuna opzione abbassa un numero del `.memlog.md`, riga 20).
- **Domanda:** gli NFR si misurano in condizioni che il pubblico non possa contestare?
- **Opzioni:**
  1. **Condizioni fissate nel §5:** NFR-1 con un insieme di dati dichiarato (clienti, workflow,
     storico); NFR-3 con il p99 di avvio riportato anche a 10.000 attività/s; eventi di NFR-5
     scritti tramite l'API o l'SDK di FR-18; forma del carico e dimensione dei payload dichiarate
     in FR-53; RSS sotto carico e per pipeline stream attiva riportati senza obiettivo. Benchmark
     pubblico difendibile; gli obiettivi diventano più difficili.
  2. **Solo i valori aggiuntivi riportati senza obiettivo** (p99 a 10.000/s, RSS sotto carico);
     insieme di dati, forma del carico e percorso di ingresso nella spec della storia del
     benchmark. NFR invariati; le condizioni restano contestabili al lancio.
  3. **§5 com'è.** Nessun lavoro; la critica di review-adversarial.md A4 resta aperta.
- **Raccomandazione:** opzione 1. In tutte le opzioni il §1.1 punto 3 dice «scala aggiungendo
  worker; il motore è a nodo singolo nella v1», coerente con l'alta disponibilità fuori dalla v1
  (§8.2).

### T-29 · Sottoinsieme OWS di FR-1 (`call http`, jq)

- **Fonti:** review-rubric.md, Scope honesty, high, «Fondamenta di sicurezza lasciate come
  assunzioni» (parte FR-1, NFR-5, NFR-12). Collegata: review-adversarial.md M15 (media).
- **Dove nel PRD:** FR-1, §12.
- **Classe:** decisione.
- **Domanda:** confermi il sottoinsieme OWS di FR-1, compresa `call http`, che apre connessioni di
  rete dal motore per conto dei clienti?
- **Opzioni:**
  1. **Confermato com'è**, con T-4 e T-5 nella v1. L'editor chiama un'API direttamente (UJ-2);
     più lavoro di sicurezza e di test.
  2. **Senza `call http` nella v1:** le chiamate HTTP passano da funzioni dei worker o da nodi
     personalizzati. Nessuna connessione aperta dal motore; l'editor perde le chiamate HTTP dirette.
- **Raccomandazione:** opzione 1. Le espressioni jq non sono un'opzione da togliere: OWS le usa per
  trasformare input e output e per le condizioni (da verificare su `dsl.md`). In tutte le opzioni il §12 marca come portanti,
  da confermare prima dell'architettura, FR-1, FR-27, FR-28, NFR-5 e NFR-12.

---

## Rinvii

### T-30 · Semantica del DAG con dipendenze fallite o saltate

- **Fonti:** review-adversarial.md A9 (alta); review-edge-cases.md A10 (alta). Assorbita:
  review-rubric.md, Strategic coherence, medium, «Perimetro del DAG ancora da inventare».
- **Dove nel PRD:** FR-2, FR-3, FR-1 (`switch`, `try`, `then`).
- **Classe:** rinvio — architettura, con il progetto dell'estensione DAG (§4.1, `CLAUDE.md`).
- **Nel PRD:** una voce nel §11: «regola di propagazione di fallimenti e salti tra dipendenze DAG,
  e incrocio con `then` e `switch`: da chiudere in architettura prima delle storie»; FR-4 nomina
  anche i task inesistenti tra i riferimenti segnalati all'invio. L'invariante «nessuna attesa
  senza fine» è già in T-20.

### T-31 · Meccanismo del bus scritto nei requisiti

- **Fonti:** review-adversarial.md A6 (alta).
- **Dove nel PRD:** FR-13, FR-30, FR-9, FR-51, NFR-1, NFR-6, §4.11.
- **Classe:** rinvio — architettura.
- **Nel PRD:** T-18 toglie già le «partizioni del bus» da FR-13 e FR-30; in NFR-6 «di topictoast o
  del bus» diventa «di qualunque processo dell'installazione»; FR-51 dice «l'aggiornamento di
  qualunque componente non perde attività accettate» (review-rubric.md, Shape fit, low); Iggy resta
  solo nel paragrafo dei vincoli del §4.11.

### T-32 · Pianificazioni durante un fermo

- **Fonti:** review-edge-cases.md A1 (alta). Assorbita: review-adversarial.md M9 (media).
- **Dove nel PRD:** FR-6.
- **Classe:** rinvio — spec della storia delle pianificazioni (recupero delle scadenze perse,
  sovrapposizione, fuso orario, intervallo minimo). Il valore predefinito del recupero, con un
  tetto, ricade in NFR-14 (T-20).

### T-33 · Limite di 1 MB: definizione esatta

- **Fonti:** review-edge-cases.md A6 (alta).
- **Dove nel PRD:** FR-11.
- **Classe:** rinvio — spec della storia di FR-11: cosa si misura (byte del JSON serializzato,
  metadati compresi o no), limite incluso, errore di dimensione non ritentabile. Dipende da T-24.

### T-34 · Annullamento: gare con retry e completamento

- **Fonti:** review-edge-cases.md A8 (alta).
- **Dove nel PRD:** FR-10.
- **Classe:** rinvio — spec della storia di FR-10: stato terminale unico, annullamento ripetibile
  senza effetto, risultato arrivato dopo registrato come ignorato, nessun tentativo dopo un
  annullamento durante l'attesa del retry.

### T-35 · Worker lento ma vivo: doppio risultato e tentativi

- **Fonti:** review-edge-cases.md A9 (alta). Assorbita: review-adversarial.md M3 (media, parte
  riassegnazione).
- **Dove nel PRD:** FR-14, FR-7, UJ-4.
- **Classe:** rinvio — architettura (protocollo dei worker: identificativo di tentativo, risultato
  di un tentativo non più corrente rifiutato e registrato) e spec di FR-14 (la riassegnazione
  consuma o no un tentativo: oggi «contando un tentativo» contraddice UJ-4 quando il task ha un
  solo tentativo).

### T-36 · Eventi duplicati e partenza di una nuova versione di query

- **Fonti:** review-edge-cases.md A15 (alta, parte non coperta da T-12).
- **Dove nel PRD:** FR-18, FR-23.
- **Classe:** rinvio — architettura (identificativo facoltativo dell'evento e deduplicazione
  all'ingresso) e spec di FR-23 (da quale punto dello stream parte la nuova versione di una query).

---

## Rilievi medi e bassi non elencati

| Revisione | Critica + alta (sopra) | Media | Bassa | Medie assorbite sopra |
|---|---|---|---|---|
| review-rubric.md | 0 + 5 | 9 | 7 (più 5 note meccaniche senza gravità) | 6, in tutto o in parte: non interferenza (T-18), idempotenza (T-15), perimetro del DAG (T-30), chiave della cassaforte (T-26), destinazioni del nodo (T-4), valori predefiniti di FR-14 e FR-17 (T-20) |
| review-edge-cases.md | 0 + 17 (l'intestazione dice 16, l'elenco va da A1 ad A17) | 14 | 4 | M2 (T-12, parte), M4 (T-9), M7 (T-4) |
| review-security.md | 6 + 10 | 13 | 5 | M1 (T-4), M7 (T-17), M10 (T-12, T-19), M11 (T-20), M13 (T-16) |
| review-adversarial.md | 5 + 14 | 19 | 8 | M2 (T-14), M3 (T-20, T-35), M8 (T-15), M9 (T-32), M10 (T-17), M13 (T-26), M14 seconda parte (T-7) |

Temi dei rilievi restanti:

1. **Credenziali, registro delle azioni, sospensione di un cliente** (review-security.md M5, M6,
   M8, M12, B1; review-edge-cases.md M5, B4; review-adversarial.md M5) — PRD, dopo T-23.
2. **Segreti nei risultati del codice esterno**: mascherarli (review-security.md M3) o dichiarare
   che non è garantito (review-adversarial.md M14); segreto cambiato tra due tentativi, ultimo uso
   con workflow e task (review-edge-cases.md M3; review-security.md B3) — PRD.
3. **Dettagli della sandbox**: declassamento della forte, parte del pool e pulizia tra attività
   dello stesso cliente, limite del nodo contro timeout del task, numeri di NFR-12, formato del
   modulo TypeScript su entrambi i livelli (review-edge-cases.md M1, M2, M6; review-security.md M2;
   review-adversarial.md M11, M12) — architettura (spike della sandbox).
4. **Protocollo e aggiornamenti**: firme di funzione durante l'aggiornamento dei worker,
   concorrenza 0 o enorme, versione del protocollo, 0.y.z contro «stessa major», procedura di
   aggiornamento del bus (review-edge-cases.md M8, M9, M10; review-adversarial.md M1) —
   architettura.
5. **Funzionamento delle pipeline stream**: destinazione eliminata, checkpoint che fallisce e suo
   carico sul database, dimensioni delle finestre, pipeline stream dell'integratore su più clienti,
   modifica che perde le finestre aperte (review-edge-cases.md M11, M12, B3; review-adversarial.md
   M18, B5; review-security.md M9) — spec delle storie.
6. **Kit frontend e dashboard**: conseguenze di FR-47–FR-49, vista in tempo reale per fan-out
   enormi, editor sui documenti che non ha creato, errori del motore senza marchio e traducibili,
   WCAG con temi personalizzati, interni dei workflow dell'integratore (review-edge-cases.md M13;
   review-adversarial.md M6, M7, B1, B4; review-security.md B2) — PRD.
7. **Metriche, benchmark e osservabilità**: metrica sul kit e su un secondo integratore,
   «primo rilascio pubblico» da definire, condizioni di SM-4 e SM-5, NFR-12 fuori dalle SM e da
   FR-53, endpoint e cardinalità delle metriche, riferimento Temporal (review-adversarial.md M16,
   B2, B6, B8; review-security.md M4; review-edge-cases.md B2, M14) — PRD.
8. **Coerenza editoriale e perimetro taciuto**: «pipeline» senza «stream», task e attività in
   NFR-4, formato di UJ-3/4/5, ordine di FR-54 e degli NFR, trigger da webhook e versioni dei
   sotto-workflow in §8.2, elenco di FR-38, avviso o rifiuto in FR-4, `document.dsl`, artefatti
   firmati, quota di rete in uscita verso terzi, domanda sulle pipeline stream da validare
   (review-adversarial.md M4, M15, M17, M19, B7; review-edge-cases.md B1; review-security.md B4,
   B5) — PRD.
