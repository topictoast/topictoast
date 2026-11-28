---
stepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-create-stories]
inputDocuments:
  - _bmad-output/planning-artifacts/prds/prd-topictoast-2026-11-19/prd.md
  - _bmad-output/planning-artifacts/prds/prd-topictoast-2026-11-19/addendum.md
  - docs/architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md
  - docs/architecture/architecture-topictoast-2026-11-20/conventions.md
  - docs/architecture/architecture-topictoast-2026-11-20/review-decisions.md
  - docs/decisions/2026-11-21-runtime-units.md
  - docs/decisions/2026-11-21-bus-source-of-truth.md
  - docs/decisions/2026-11-21-typed-data-and-tenant-isolation.md
  - docs/decisions/2026-11-21-tenant-content-keys-and-deletion.md
  - docs/decisions/2026-11-21-dag-task-and-propagation.md
  - docs/decisions/2026-11-21-schema-language.md
  - docs/decisions/2026-11-21-single-validator-napi-rs.md
  - docs/decisions/2026-11-21-public-api-and-kit-embedding.md
  - docs/decisions/2026-11-21-stream-pipelines-datafusion.md
  - docs/decisions/2026-11-21-payload-store-and-history.md
  - docs/decisions/2026-11-21-bdd-boundary.md
  - docs/decisions/2026-11-21-repo-layout-and-dependencies.md
  - _bmad-output/planning-artifacts/ux-designs/ux-topictoast-2026-11-19/DESIGN.md
  - _bmad-output/planning-artifacts/ux-designs/ux-topictoast-2026-11-19/EXPERIENCE.md
  - .specify/memory/constitution.md
  - _bmad-output/planning-artifacts/briefs/brief-topictoast-2026-11-18/brief.md
  - _bmad-output/planning-artifacts/briefs/brief-topictoast-2026-11-18/addendum.md
---

# topictoast - Epic Breakdown

## Overview

Questo documento scompone in epiche e storie realizzabili i requisiti del PRD, del design UX e
dell'architettura di topictoast.

Regole di lettura:

- Gli id del PRD (`FR-n`, `NFR-n`, `UJ-n`, `SM-n`) restano quelli del PRD, mai rinumerati; `AR-n`
  (architettura), `UX-DR-n` (UX) e `BR-n` (brief) sono id di questo documento.
- Il riferimento `prd.md:Lx-Ly` indica le righe del PRD da cui viene la voce; il PRD resta la fonte
  in caso di dubbio. Righe e parafrasi si riferiscono al PRD del 2026-11-21: l'aggiornamento del 2026-11-23
  (domande aperte, decisioni dell'architettura, FR-59) sposta le righe e cambia alcuni testi, e in
  quel caso vale il PRD. FR-59 non ha riferimento di riga.
- Dove il PRD dà per aperta una questione che l'architettura accettata ha già deciso, vale
  l'architettura: la voce porta la nota «→ deciso da …».
- `[DA ALLINEARE]` segna una voce UX scritta prima dell'architettura (2026-11-20) che va riallineata
  con l'Update di `bmad-ux` prima della prima storia del design system (AR-7).

## Requirements Inventory

### Functional Requirements

#### 4.1 Definizione dei workflow
- **FR-1 — Sottoinsieme OWS della v1** (prd.md:L318-L337; parziale: costrutti fuori sottoinsieme restano fuori v1): topictoast implementa un sottoinsieme di OWS 1.1 (non è un runtime OWS conforme) per integratore e tenant.
  - Costrutti ammessi: do, set, switch, fork, for, try con retry, wait, raise, call http, invocazione di funzioni e nodi personalizzati, sotto-workflow, espressioni jq, `schedule` del documento cron o a intervallo (FR-6).
  - Documento OWS 1.1 che usa solo il sottoinsieme: accettato ed eseguito con la semantica della specifica.
  - Costrutto fuori sottoinsieme: rifiutato all'invio con errore che nomina costrutto e percorso nel documento.
  - Accettati `document.dsl` 1.1.x; altri valori rifiutati con errore che indica le versioni accettate `[ASSUMPTION]`.
  - Fuori perimetro: `call` gRPC, OpenAPI, AsyncAPI, A2A; `run` di container, script, shell; `emit` e `listen`; conformità completa al CTK.
  - Come un task invoca funzione/nodo e dichiara i segreti in OWS: lo fissa l'architettura (§11).
- **FR-2 — Dipendenze arbitrarie tra task** (prd.md:L339-L351; v1): un task può dipendere da uno o più task qualsiasi del workflow (UJ-2).
  - Il task parte quando tutte le dipendenze sono completate, non prima; task senza dipendenze reciproche partono in parallelo.
  - Ciclo nelle dipendenze: rifiutato all'invio con i task coinvolti nel messaggio; il divieto vale per le dipendenze dell'estensione DAG, `for` e `then` mantengono la semantica OWS.
  - Task con dipendenza fallita o saltata: non resta in attesa, raggiunge uno stato terminale secondo una regola documentata (§11).
  - ⚠ La regola di propagazione di fallimenti e salti è domanda aperta (§11, glossario Stati): non specificata. → deciso da AD-9 (AR-30); join e fan-out dentro `dag`: AR-88.
- **FR-3 — Fan-out dinamico con join** (prd.md:L353-L368; v1): un task (o sotto-grafo) si ripete in parallelo per ogni elemento di un elenco prodotto a runtime e un task successivo attende tutti i risultati (UJ-2).
  - N elementi: N attività; il join riceve N risultati nell'ordine dell'elenco.
  - Elenco vuoto: nessuna attività, il join riceve un elenco vuoto.
  - Input singolo al posto dell'elenco = elenco di un elemento (UJ-2, variante).
  - Parallelismo massimo limitabile nel workflow; senza limite vale la quota del tenant (FR-30).
  - Elemento fallito dopo i retry: segue la politica dichiarata, o fallisce tutto il fan-out o il join riceve l'errore al posto del risultato.
- **FR-4 — Validazione all'invio** (prd.md:L370-L390; v1): ogni workflow è validato alla ricezione, prima di salvarlo, con lo schema proprio (sottoinsieme OWS + estensione DAG).
  - Documento non valido: errore di validazione con percorso e motivo; mai crash, mai salvataggio parziale.
  - Riferimenti inesistenti a funzioni, nodi personalizzati, segreti, task: segnalati all'invio.
  - L'integratore dichiara quali funzioni sono invocabili dai workflow dei tenant; documento di tenant che invoca funzione non disponibile: rifiutato come se non esistesse, senza dettagli del manifest ([kit] il nucleo JS non lo riceve); il controllo si ripete all'esecuzione.
  - Input di ogni task controllato sul manifest della funzione/nodo (FR-55); quello di un sotto-workflow sullo schema di input del workflow chiamato (FR-56).
  - Errori all'invio: tipi sbagliati, campi obbligatori mancanti, e (nel workflow di un tenant) connessioni senza segreto; stessi esiti dell'editor (FR-40) e del nucleo JS (FR-39).
  - Connessioni di un workflow dell'integratore: risolte all'avvio (FR-29).
  - Segreto legato a una connessione che non soddisfa, o in più rispetto alle connessioni del manifest del task: errore all'invio.
- **FR-5 — Versioni dei workflow** (prd.md:L392-L408; v1): si può pubblicare una nuova versione mentre la precedente ha esecuzioni in corso.
  - [kit] Dall'editor salvare = pubblicare, nessun passo separato; lavoro incompleto o non valido non si salva e resta solo nel kit (FR-27); ogni salvataggio cambia subito ciò che avviano pianificazione e pipeline stream che usano il workflow (FR-57, FR-24).
  - [kit] Prima di un salvataggio che cambia lo schema di input (FR-56) l'editor avvisa delle pianificazioni e pipeline stream che avviano il workflow.
  - Le esecuzioni in corso finiscono con la versione con cui sono partite.
  - Le nuove esecuzioni usano l'ultima versione pubblicata, salvo richiesta esplicita di una versione.
  - Pubblicazione preparata su una versione non più ultima: rifiutata con errore che indica la versione corrente; mai sostituzione silenziosa del lavoro di un altro.
- **FR-56 — Schema di input del workflow** (prd.md:L410-L422; v1): si dichiara lo schema dell'input (campi, tipi, obbligatori) da editor, SDK o nel documento (UJ-2).
  - Avvio con input non conforme: rifiutato prima di essere accettato, con percorso e regola violata, mai con il valore ricevuto.
  - Valore singolo per un campo elenco = elenco di un elemento (FR-3).
  - Workflow senza schema: accetta qualunque input JSON.
  - Lo schema usa il linguaggio di FR-55; come si scrive nel documento OWS lo fissa l'architettura (§11).
#### 4.2 Esecuzione
- **FR-6 — Trigger** (prd.md:L430-L457; v1): un'esecuzione parte da chiamata API/SDK, pianificazione o riga emessa da una pipeline stream (FR-24) (UJ-1, UJ-3).
  - Ogni avvio restituisce subito l'identificativo dell'esecuzione.
  - Ogni esecuzione registra l'origine: API o SDK (con ruolo di chi l'ha avviata e, se presente, nome visualizzato, FR-27); pianificazione; pipeline stream.
  - Pianificazione = quella del documento OWS (`schedule`, FR-1) del proprietario del workflow, oppure quella che un tenant aggiunge dal kit (FR-57); le esecuzioni avviate appartengono al proprietario della pianificazione (FR-26).
  - La pianificazione porta l'input delle esecuzioni, controllato sullo schema di input (FR-56) al salvataggio e a ogni scadenza.
  - Cron o intervallo: un'esecuzione per ogni scadenza accettata, una sola anche se il motore si ferma o riparte mentre scatta.
  - Scadenza o riga di pipeline stream (FR-24) che non diventa esecuzione accettata (es. input fuori schema, connessione senza segreto): registrata sulla pianificazione/pipeline stream con il motivo, visibile al proprietario e all'operatore; mai in silenzio.
  - Pianificazione sospendibile e riprendibile senza modificare il workflow; le scadenze cadute durante la sospensione non si recuperano.
  - Dopo un fermo, scadenze perse e recuperate visibili al proprietario della pianificazione.
  - Chiave di idempotenza già usata dallo stesso tenant, per almeno la durata di conservazione (FR-31): restituisce l'esecuzione esistente, anche con richieste concorrenti; 100 richieste concorrenti con la stessa chiave = una sola esecuzione.
- **FR-7 — Retry** (prd.md:L459-L467; v1): il workflow dichiara per ogni task una politica di retry: numero di tentativi, attesa tra i tentativi, crescita dell'attesa.
  - Attività fallita ritentata secondo la politica; ogni tentativo è un'attività distinta visibile nello storico.
  - Tentativi esauriti: il task fallisce e l'errore dell'ultimo tentativo è conservato.
- **FR-8 — Timeout** (prd.md:L469-L481; v1): il workflow dichiara timeout per task e per l'intera esecuzione.
  - Attività oltre il timeout: considerata fallita, segue il retry.
  - Esecuzione oltre il timeout: termina come scaduta; le attività in corso sono annullate.
  - Il timeout del task conta dalla consegna al worker/sandbox; l'attesa prima della consegna (nessun worker, tenant oltre quota, pool sandbox vuoto) non fa fallire per timeout del task e non consuma tentativi, ma è limitata da un tempo documentato (NFR-14).
  - Attività rimasta in coda per quota più del timeout del task: una volta consegnata si completa normalmente.
- **FR-9 — Durabilità** (prd.md:L483-L494; v1): un'esecuzione accettata arriva alla fine anche se topictoast o il bus si riavviano.
  - Esecuzione, attività o evento è "accettato" solo se, alla partenza della risposta, è già al sicuro dal guasto dichiarato in NFR-6.
  - Dopo quel guasto nessuna esecuzione/attività/evento accettato va perso; nessun task con completamento registrato è rieseguito; un'attività consegnata senza esito registrato può essere riconsegnata.
  - Consegna ai worker almeno una volta; la documentazione per l'integratore spiega come rendere idempotenti le funzioni.
- **FR-10 — Annullamento** (prd.md:L496-L504; v1): integratore e tenant possono annullare un'esecuzione in coda o in esecuzione.
  - Nessuna nuova attività parte dopo l'annullamento; worker e sandbox ricevono richiesta di fermare le attività in corso.
  - Storico: esecuzione annullata, con chi (ruolo e, se presente, nome visualizzato) e quando.
- **FR-11 — Dati tra task** (prd.md:L506-L516; v1): l'output di un task è input dei task che ne dipendono.
  - Limite di dimensione di input e output di un'attività `[ASSUMPTION: 1 MB nella v1]` (storage per payload grandi dopo la v1).
  - Output oltre il limite: l'attività fallisce con errore che indica dimensione e limite.
  - Fan-out: il limite vale per ogni elemento; l'input aggregato del join ha tetto proprio, configurabile, con predefinito documentato (NFR-14); oltre il tetto il join fallisce con dimensione e limite.
- **FR-59 — Log dei task** (PRD del 2026-11-23, §4.2; v1): per ogni tentativo di un task, gli eventi del motore e le righe scritte dal codice che lo esegue, funzione del worker o nodo personalizzato.
  - Eventi del motore almeno: ricevuta (dal worker o dalla sandbox), in esecuzione, completata, fallita con il motivo (timeout compreso), ritentata con l'attesa, annullata; con il ruolo tenant senza identificativi dei worker.
  - Righe dal worker tramite l'SDK (TypeScript e Rust) e dal nodo dalla sandbox: ora al millesimo, livello (info, avviso, errore; «ok» solo per gli eventi del motore), messaggio, tentativo; orologio e ordine nella spec.
  - Mostrate nella vista delle esecuzioni del kit e nel dettaglio della dashboard entro 1 s da quando il codice scrive la riga, anche a tentativo in corso.
  - Il tenant vede il log delle proprie esecuzioni, righe dei worker comprese; nessun tenant vede quello di un altro (test di NFR-8).
  - Nessun carico in più sul database (NFR-4 vale con il log attivo); contano nello spazio del tenant (FR-30), restano quanto lo storico (FR-31) e spariscono con l'esecuzione o il tenant.
  - Tetto di righe e byte per tentativo sulle righe del codice, mai sugli eventi del motore (NFR-14): le righe in più si scartano e il log dice quante.
  - Valore esatto di un segreto consegnato sostituito da un segnaposto prima del bus e dell'archivio (NFR-8, C-12; confermato da Marco); valori trasformati non riconosciuti, detto nella documentazione.
  - Mostrate come testo (NFR-7), mai nei log del motore (FR-52); scrivere fino al tetto non porta gli altri tenant oltre NFR-2 (corsa a carico misto di FR-53).
#### 4.3 Worker
- **FR-12 — Registrazione e consegna** (prd.md:L525-L539; v1): un worker si collega con l'SDK, registra le funzioni con il manifest (FR-55) e riceve le attività che le invocano.
  - Attività per funzione senza worker collegati: restano in coda con il motivo, entro il limite di attesa di FR-8, e partono quando un worker la registra.
  - Più worker con la stessa funzione: le attività si distribuiscono tra loro.
  - Solo un processo con credenziale di worker valida, distinta da quelle dei tenant, può collegarsi, registrare funzioni, ricevere attività. Senza credenziale o con credenziale di tenant che registra una funzione esistente: rifiutato, nessuna attività né segreto.
  - Chi raggiunge il bus senza credenziale valida non legge né scrive attività ed eventi.
  - Entrambi i casi entrano nel test di NFR-8.
- **FR-13 — Concorrenza e contropressione** (prd.md:L541-L549; v1): ogni worker dichiara quante attività accetta in parallelo.
  - Mai più attività consegnate di quante dichiarate.
  - Con un worker lento o bloccato, le attività consegnate ad altri worker restano entro l'obiettivo di NFR-2, anche se accodate insieme alle sue `[ASSUMPTION: soglia di non interferenza = obiettivo NFR-2, qui e in FR-17, FR-25, FR-30]`.
- **FR-14 — Worker che non risponde** (prd.md:L551-L558; v1): topictoast rileva un worker che smette di rispondere.
  - Attività di un worker senza segni di vita entro l'intervallo configurato: riassegnate ad altri worker, contando un tentativo.
  - [dashboard] Il worker risulta "non risponde" (FR-48).
- **FR-55 — Manifest delle funzioni** (prd.md:L560-L586; v1): alla registrazione (FR-12) il worker dichiara il manifest: nome, descrizione, gruppo, connessioni richieste (piattaforma esterna + tipo di credenziale), schema di input, schema di output, campo di estensione per altri metadati; stessi campi per un nodo personalizzato (FR-15) (UJ-1, UJ-2).
  - Manifest non valido: rifiutato con l'errore del validatore.
  - Schemi del manifest e schema di input di FR-56: linguaggio chiuso e documentato (campi, tipi, obbligatori, elenchi, oggetti annidati), una sola regola di compatibilità comune a motore, nucleo JS ed editor (§11).
  - Nessun riferimento fuori dal manifest/documento: schema con riferimento esterno rifiutato alla validazione senza aprire connessioni di rete.
  - Manifest e schemi ostili (riferimento esterno, oltre i limiti di NFR-14) entrano nel test di NFR-8.
  - Mentre un tenant invia e avvia a ripetizione workflow con schemi e input più complessi consentiti (NFR-14), le attività degli altri tenant restano entro NFR-2; il caso entra nella corsa a carico misto di FR-53.
  - All'esecuzione: input di ogni attività controllato sullo schema di input del manifest, output sullo schema di output. Input non conforme: il task fallisce prima della consegna, senza consumare tentativi. Output non conforme: l'attività fallisce con percorso e motivo e non arriva mai ai task a valle.
  - [kit] Nell'editor funzioni dei worker e nodi personalizzati hanno gli stessi campi dal manifest; la categoria (funzione del worker o nodo personalizzato) li distingue, il gruppo raggruppa le voci.
  - Il formato del manifest, con elenco chiuso dei campi, è superficie pubblica (§6).
#### 4.4 Nodi personalizzati e sandbox
- **FR-15 — Caricamento di un nodo personalizzato** (prd.md:L597-L609; v1): il tenant carica via SDK o CLI un nodo già compilato (TypeScript o Rust) con il manifest (FR-55). [kit] L'editor lo mostra tra i task disponibili ma non ne ospita il codice.
  - Visibile solo ai workflow del tenant che l'ha caricato.
  - Nodo non conforme (modulo compilato, manifest): rifiutato con l'errore del validatore.
  - Il server non richiede compilatori installati.
  - Caricare un nodo con nome già usato crea una versione nuova; l'esecuzione in corso finisce con la versione con cui è partita; ogni attività registra quale versione ha eseguito.
- **FR-16 — Isolamento della sandbox** (prd.md:L611-L636; v1): un nodo personalizzato in esecuzione vede solo il proprio input e i segreti che il suo task dichiara.
  - Non raggiunge rete interna dell'installazione, file system dell'host, dati o segreti di altri tenant.
  - Non raggiunge la chiave della cassaforte né segreti non dichiarati: la sandbox, a entrambi i livelli, gira fuori dal processo che custodisce la chiave.
  - Ogni connessione di rete aperta per un tenant (nodo personalizzato o `call http`) raggiunge solo le destinazioni che l'integratore consente a quel tenant; la lista dichiarata in un nodo può restringere, mai allargare.
  - Loopback, indirizzi privati, link-local e indirizzi dell'installazione (motore, bus, database) irraggiungibili anche se un nome consentito vi si risolve o un redirect vi porta.
  - `call http` verso `127.0.0.1`, `169.254.169.254` o nome che si risolve a indirizzo privato: fallisce con errore di politica di rete senza aprire la connessione; entra nel test di NFR-8.
  - Il trattamento dei byte di un nodo caricato (validazione, compilazione, istanziazione), manifest compreso, avviene fuori dal processo che custodisce la chiave: un modulo che fa crashare il validatore non ferma motore né esecuzioni in corso.
  - A entrambi i livelli: un'istanza della sandbox, il suo processo e la memoria usata non contengono mai dati o segreti di due tenant, né insieme né in sequenza. Test NFR-8: mentre gira un nodo del tenant B con segreto sentinella, il processo del nodo del tenant A non lo contiene.
  - Fuori v1 (§8.2): destinazioni di rete legate a una connessione; nella v1 valgono le destinazioni per tenant.
- **FR-17 — Limiti di risorse** (prd.md:L638-L653; v1): ogni esecuzione di nodo personalizzato e ogni espressione jq di un tenant ha limiti di CPU, memoria, tempo; ogni query di pipeline stream ha limiti di CPU e memoria (memoria di esecuzione oltre a quella di stato, FR-25).
  - Limite superato: l'attività fallisce con errore che nomina il limite, oppure la pipeline stream è rallentata/sospesa con errore visibile a tenant e operatore; il processo del motore non si ferma; attività degli altri tenant entro NFR-2.
  - Nodo fermato: nessun processo né memoria occupata; mentre un nodo satura i limiti, le attività degli altri tenant restano entro NFR-2.
  - jq non legge ambiente, file system, moduli esterni, rete: espressione con `$ENV`, `env`, `input_filename`, `import`, `include` rifiutata alla validazione (FR-4).
- **FR-54 — Sandbox forte opzionale** (prd.md:L655-L667; v1): dove l'host offre KVM, l'integratore può attivare la sandbox forte (microVM) per tutti i tenant o per alcuni.
  - Pool configurabile di sandbox forti già avviate: attività senza avvio a freddo; pool vuoto: le attività attendono una sandbox libera entro il limite di FR-8.
  - Sandbox che ha eseguito codice di un tenant non passa mai a un altro: monouso o riservata a quel tenant.
  - La RAM del pool si misura a parte, fuori da NFR-1.
  - Host senza KVM: sandbox forte non attivabile; i nodi dei tenant per cui è attivata si rifiutano con un codice, con avviso all'operatore [dashboard]; gli altri tenant usano la leggera (PRD riallineato dopo S-2, 2026-11-26).
#### 4.5 Pipeline stream
- **FR-18 — Stream e ingresso degli eventi** (prd.md:L678-L692; v1): integratore, worker e workflow scrivono eventi sugli stream di un tenant via API o SDK. `[ASSUMPTION: nessun connettore verso sistemi esterni come Kafka nella v1]`
  - Evento di uno stream di un tenant leggibile solo dalle pipeline stream di quel tenant.
  - Evento senza tempo dell'evento: riceve il tempo di arrivo.
  - Conservazione per durata o dimensione per stream, con predefinito documentato (NFR-14); breve (pochi minuti): copre la finestra più lunga delle pipeline che leggono lo stream + ritardo tollerato + intervallo dei checkpoint.
  - Pipeline con finestra oltre la conservazione dello stream: rifiutata al salvataggio con errore che indica i due valori.
- **FR-19 — Definizione in SQL** (prd.md:L694-L713; v1): integratore e tenant definiscono una pipeline stream con query SQL su uno o più stream.
  - Validazione al salvataggio; errore di sintassi o stream inesistente: messaggio con riga e colonna.
  - Query su stream di un altro tenant: rifiutata come se lo stream non esistesse.
  - Dialetto dei tenant = lista chiusa e documentata (interrogazioni sugli stream del tenant con le funzioni elencate); DDL, tabelle esterne, connettori, lettura/scrittura di file, accesso alla rete, funzioni definite dall'utente: rifiutati al salvataggio con errore di validazione. Query ostili (tabella esterna su `/etc/passwd`, `COPY` verso un file, connettore verso l'indirizzo del bus, UDF) nel test di NFR-8.
  - Ciclo tra pipeline dello stesso tenant (una scrive sullo stream da cui un'altra legge, fino a tornare alla prima): rifiutato al salvataggio.
  - Rielaborata dal checkpoint sugli stessi eventi, emette le stesse righe (condizione per FR-24).
- **FR-20 — Finestre e aggregazioni** (prd.md:L715-L723; v1): aggregazione su finestre di tempo dell'evento, deduplicazione, filtro. `[ASSUMPTION: finestre fisse e scorrevoli nella v1; di sessione dopo]`
  - Conteggio su finestre fisse di 5 minuti: una riga per gruppo alla chiusura della finestra.
  - Deduplicazione su chiave: ogni chiave emessa una sola volta per finestra.
- **FR-21 — Watermark ed eventi in ritardo** (prd.md:L725-L737; v1): la pipeline dichiara quanto ritardo tollera.
  - Finestra chiusa quando il watermark supera la sua fine + il ritardo tollerato.
  - Eventi dopo la chiusura della loro finestra: scartati e contati in una metrica visibile [dashboard].
  - Evento con tempo oltre una tolleranza nel futuro rispetto all'arrivo: non sposta il watermark, non fa scartare gli eventi successivi, è contato in una metrica visibile [dashboard]; un evento datato un anno nel futuro non chiude alcuna finestra.
  - Stream che smette di ricevere eventi: le finestre aperte si chiudono comunque entro un tempo documentato (NFR-14); l'ultima finestra di un'ondata emette le righe (UJ-3).
- **FR-22 — Stato e checkpoint** (prd.md:L739-L747; v1): lo stato della pipeline sopravvive ai riavvii.
  - Dopo il riavvio riparte dall'ultimo checkpoint e rielabora gli eventi successivi; nessuna finestra perde eventi già accettati.
  - Intervallo tra checkpoint configurabile, predefinito `[ASSUMPTION: 10 secondi]`.
- **FR-23 — Ciclo di vita** (prd.md:L749-L759; v1): integratore e tenant attivano, sospendono, modificano, eliminano una pipeline stream.
  - Sospesa: non consuma CPU; alla ripresa elabora gli eventi arrivati e ancora conservati; se una parte è uscita dalla conservazione: errore visibile a tenant e operatore, mai in silenzio.
  - Modificare la query di una pipeline attiva crea una nuova versione con stato vuoto `[ASSUMPTION]`.
- **FR-24 — Destinazioni** (prd.md:L761-L772; v1): le righe emesse vanno su uno stream del tenant oppure avviano un workflow del tenant, un avvio per riga, con la riga come input (UJ-3).
  - Destinazione workflow: ogni riga dà esattamente un avvio anche dopo riavvio dal checkpoint: un'esecuzione accettata o un rifiuto registrato con il motivo (FR-6). Due righe distinte con lo stesso contenuto (es. stesso conteggio in due finestre) = due avvii.
  - Esecuzioni avviate rispettano la quota del tenant (FR-30); righe in eccesso trattenute entro il tetto dell'arretrato, non perse.
- **FR-25 — Isolamento e quote delle pipeline stream** (prd.md:L774-L786; v1): le pipeline di un tenant non degradano quelle degli altri.
  - Quote per tenant su numero di pipeline attive, memoria di stato, CPU.
  - Memoria di stato superata: pipeline sospesa con errore visibile a tenant e operatore.
  - CPU oltre quota: rallentata o sospesa con errore visibile a tenant e operatore.
  - Con una pipeline al limite, le pipeline degli altri tenant elaborano al ritmo di arrivo, senza ritardo crescente sul watermark, e le attività degli altri restano entro NFR-2.
#### 4.6 Separazione tra tenant
- **FR-26 — Visibilità per tenant** (prd.md:L799-L826; v1): ogni richiesta per conto di un tenant vede e modifica solo le sue risorse.
  - Risorsa di altro tenant: risposta come se non esistesse.
  - Liste, ricerche, conteggi, eventi in tempo reale mai con risorse di altri tenant.
  - Tenant e permessi si ricavano solo dalla credenziale verificata, mai da un campo della richiesta (percorso, corpo, filtro): credenziale di A con id di B in un parametro agisce su A o è rifiutata.
  - Un'esecuzione appartiene a chi la avvia: avviata da un tenant, o per suo conto con credenziale dell'integratore (FR-28), a mano, con la sua pianificazione o da una sua pipeline stream = di quel tenant, visibile solo a lui; avviata dalla pianificazione del documento (FR-6) = del proprietario del workflow (per workflow dell'integratore, l'integratore, FR-29).
  - Workflow di un tenant gira sempre nel contesto del tenant, anche se lo avvia l'integratore; un'esecuzione dell'integratore esegue solo workflow dell'integratore. Test NFR-8.
  - Funzioni, nodi personalizzati e sotto-workflow citati si risolvono tra le risorse del proprietario del workflow e, se è un tenant, tra quelle rese disponibili dall'integratore; segreti dal proprietario dell'esecuzione; mai tra risorse di altro tenant.
  - Un workflow scrive solo su stream del proprietario dell'esecuzione; un worker scrive solo su stream del proprietario dell'attività servita; scrittura su stream di altro tenant: stesso errore di stream inesistente, nessun evento arriva.
  - Ogni trigger (pianificazione, pipeline stream) ha proprietario fissato alla creazione (tenant o integratore) e avvia esecuzioni solo nel suo contesto.
- **FR-27 — Identità garantita dalla piattaforma ospite** (prd.md:L828-L851; v1; la rotazione della chiave di firma è dopo la v1 `[ASSUMPTION]`, §8.2): la piattaforma ospite garantisce per ogni richiesta l'identità del tenant con credenziale firmata, a scadenza breve, con tenant, ruolo e, se voluto, nome visualizzato.
  - Richiesta senza identità valida o con credenziale scaduta: rifiutata.
  - [kit] Il kit nella piattaforma ospite non chiede un secondo login.
  - Nome visualizzato = dato personale: sempre testo (NFR-7), conservato quanto il record che lo contiene, cancellato con lui o con il tenant (FR-31); documentato per l'integratore.
  - Con credenziale dell'integratore, il tenant scelto nel kit è un campo della credenziale firmata; la piattaforma firma una credenziale nuova a ogni cambio di tenant.
  - Rifiutate: credenziale di ruolo tenant con il campo del tenant scelto; con tenant scelto inesistente o eliminato (FR-31); con nome visualizzato oltre la lunghezza massima o fuori dai caratteri documentati (NFR-14). I tre casi nel test NFR-8.
  - [kit] Avvisa la piattaforma prima della scadenza e alla scadenza.
  - [kit] Lavoro non salvato torna dopo il rinnovo solo se la credenziale nuova ha stesso tenant e stesso ruolo; altrimenti scartato senza mostrarlo; resta solo in memoria della pagina (un ricaricamento lo perde, mai salvato nel browser).
  - [kit] A un cambio di tenant scarta lo stato del tenant precedente, eventi in tempo reale compresi; test NFR-8.
- **FR-28 — Due ruoli** (prd.md:L853-L873; v1): ruoli: integratore (con l'operatore), che vede tutti i tenant; tenant, che vede solo sé stesso.
  - Solo l'integratore crea e configura tenant e quote.
  - [kit] Con credenziale dell'integratore il kit agisce per il tenant nella credenziale (FR-27): ogni sezione mostra solo le risorse di quel tenant e i workflow dell'integratore; "Esegui" lancia nel suo contesto.
  - Credenziale dell'integratore per il kit con tenant scelto vale solo per: risorse di quel tenant, lettura dei workflow dell'integratore, elenco e ricerca dei tenant; senza tenant scelto solo elenco e ricerca dei tenant. Ogni altra operazione riservata (tenant, quote, pubblicazione di workflow dell'integratore, risorse di altri tenant, operazioni dell'operatore): rifiutata con credenziali del kit (NFR-8). Elenco dei tenant riservato all'integratore.
  - [kit] Workflow creato nell'editor con credenziale dell'integratore appartiene al tenant indicato; nel kit non si crea un workflow dell'integratore: si creano dall'SDK (FR-35, FR-36) e nel kit sono in sola lettura.
  - Fuori perimetro: ruoli e permessi tra utenti dello stesso tenant (li gestisce la piattaforma ospite).
- **FR-29 — Workflow dell'integratore e del tenant** (prd.md:L876-L894; v1): un workflow appartiene all'integratore (disponibile a tutti i tenant) o a un singolo tenant.
  - Workflow dell'integratore eseguito per un tenant: nel contesto del tenant (suoi segreti, quote, storico).
  - Avviato dalla pianificazione del documento (FR-6): nel contesto dell'integratore, con i suoi segreti; nessun tenant vede l'esecuzione (FR-26).
  - Connessioni del workflow dell'integratore risolte all'avvio tra i segreti di chi lo avvia; nessun segreto soddisfa: avvio rifiutato prima di essere accettato, con la connessione mancante nel messaggio.
  - Task senza manifest (es. `call http`) in un workflow dell'integratore dichiara nel documento la connessione, risolta allo stesso modo; senza dichiarazione non riceve segreti.
  - Un tenant non può modificare un workflow dell'integratore.
  - Nodo personalizzato o sotto-workflow di un tenant con lo stesso nome di uno usato da un workflow dell'integratore non cambia ciò che questo esegue quando gira per lui.
- **FR-30 — Quote per tenant** (prd.md:L896-L912; v1): l'integratore assegna quote su attività concorrenti, esecuzioni al minuto, risorse della sandbox (inclusa la parte del pool sandbox forte), risorse delle pipeline stream, eventi scritti al secondo, richieste API al secondo, spazio occupato (stream, storico, code, nodi caricati).
  - Tenant oltre quota: nuove attività in coda; attività degli altri tenant entro NFR-2 anche se accodate insieme (UJ-4).
  - Superare una quota non perde lavoro accettato.
  - L'arretrato (attività ed esecuzioni in coda, righe trattenute di FR-24, eventi conservati) ha un tetto. Oltre tetto o quota di spazio: nuova richiesta con errore di quota prima di essere accettata; la pipeline stream smette di emettere segnalandolo con errore visibile a tenant e operatore; lavoro già accettato non perso.
  - Tenant che scrive eventi oltre quota: errori di quota; gli altri continuano a scrivere e avviare esecuzioni entro NFR-2.
- **FR-31 — Conservazione dello storico** (prd.md:L914-L925; v1): durata di conservazione configurabile per tenant `[ASSUMPTION: 30 giorni predefiniti]`.
  - Esecuzioni più vecchie della durata eliminate con input e output.
  - Eliminare un tenant: prima ferma esecuzioni, pipeline stream, pianificazioni; poi rende i dati illeggibili in ogni supporto (database, bus, checkpoint, nodi caricati, sandbox riservate) entro un tempo massimo documentato.
  - Passato quel tempo, la ricerca di un valore sentinella del tenant nelle directory dati di topictoast e del bus non trova nulla; un identificativo di tenant eliminato non viene riusato.
  - ⚠ Il tempo massimo di eliminazione "documentato" non ha valore nel testo (NFR-14 non letto in questo estratto). → il valore lo fissa la spec della storia (NFR-14); ordine di eliminazione e prova sentinella in AD-7 (AR-28).

#### 4.7 Cassaforte dei segreti

- **FR-32 — Salvataggio di un segreto** (prd.md:L933-L947; parziale: OAuth fuori, nella v1 i segreti sono solo chiavi API): il tenant salva un segreto con un nome e la connessione che soddisfa (FR-55), dall'editor (componente React) o tramite SDK.
  - Dopo il salvataggio il valore non è più leggibile da nessuna API, componente o dashboard; si vedono solo nome, connessione, data e ultimo uso.
  - [kit] Il selettore dei segreti mostra solo i segreti che soddisfano la connessione richiesta; per un task senza manifest (es. `call http`) mostra tutti i segreti del tenant e l'attività riceve solo quello scelto (FR-33).
  - Le connessioni tra cui il tenant sceglie sono solo quelle dei manifest a lui disponibili: funzioni rese disponibili dall'integratore e suoi nodi personalizzati. L'elenco entra nel test di NFR-8.
  - Sostituire il valore di un segreto non richiede di modificare i workflow che lo usano.
- **FR-33 — Consegna al momento dell'esecuzione** (prd.md:L949-L966; v1): un task dichiara i segreti che gli servono; il worker o la sandbox li ricevono insieme all'attività.
  - Un'attività riceve solo i segreti dichiarati dal suo task e appartenenti al proprietario dell'esecuzione (FR-26).
  - I segreti non compaiono in log, storico, output né errori di topictoast.
  - Le connessioni di ogni task sono fissate nella versione pubblicata del workflow (FR-5) e controllate sui manifest alla pubblicazione; il segreto si sceglie come dicono FR-29 e FR-40.
  - Un manifest registrato dopo non aggiunge connessioni né segreti a quella versione: se chiede una connessione in più, il task fallisce con l'errore e non riceve altri segreti. Il caso entra nel test di NFR-8.
  - Un avvio con una connessione senza segreto è rifiutato prima di essere accettato, con la connessione nel messaggio (FR-29).
  - Un segreto eliminato durante un'esecuzione fa fallire alla consegna l'attività che lo usa, con il motivo.
- **FR-34 — Protezione a riposo** (prd.md:L968-L981; v1, rotazione chiave inclusa §8.2): i segreti sono cifrati a riposo.
  - Nessun supporto persistente (database, bus, checkpoint, log, file temporanei) contiene un segreto in chiaro; una copia senza la chiave non rivela alcun segreto.
  - Criterio: dopo un'esecuzione con un segreto sentinella, la ricerca del valore nelle directory dati di topictoast, del bus e dei checkpoint non trova nulla.
  - La chiave di cifratura non è raggiungibile dai nodi personalizzati (FR-16).
  - La chiave non sta nel database né nelle sue copie; cosa succede se si perde è documentato per l'integratore.
  - L'integratore ruota la chiave senza fermare il motore e senza perdere segreti; dopo la rotazione la chiave vecchia non decifra nulla di ciò che è salvato.

#### 4.8 SDK

- **FR-35 — Workflow come codice** (prd.md:L990-L998; v1): l'integratore definisce un workflow in TypeScript o Rust e lo pubblica.
  - [SDK] Il workflow come codice produce un documento OWS equivalente, valido secondo lo schema di topictoast, leggibile ed esportabile.
  - [dashboard/kit] Nell'editor è in sola lettura per tutti, con provenienza visibile; si modifica solo da codice.
- **FR-36 — Documenti OWS a runtime** (prd.md:L1000-L1006; v1): l'integratore invia un documento OWS, con estensione DAG, e ottiene un workflow eseguibile senza riavviare nulla.
  - Il workflow è eseguibile appena l'invio risponde con successo.
- **FR-37 — Worker e funzioni** (prd.md:L1008-L1017; v1): l'integratore scrive un worker che registra funzioni tipizzate, in TypeScript o Rust.
  - Una funzione tipizzata dichiara il suo manifest (FR-55).
  - [SDK] Una funzione riceve input, segreti dichiarati e contesto dell'esecuzione (proprietario, esecuzione, tentativo) e restituisce un output o un errore.
  - La documentazione spiega di ricavare il proprietario dal contesto, mai dall'input; l'SDK espone il proprietario solo da lì.
- **FR-38 — Amministrazione** (prd.md:L1019-L1025; v1): l'integratore crea e configura tenant, quote, segreti e pipeline stream tramite SDK.
  - [SDK] Tutto ciò che si fa da editor e dashboard si può fare anche tramite SDK.
  - Ogni tenant ha un nome, con cui il kit lo mostra e lo cerca (FR-28).

#### 4.9 Kit frontend

- **FR-39 — Nucleo JS** (prd.md:L1037-L1045; v1): lo sviluppatore della piattaforma ospite legge, costruisce, valida, invia workflow e segue le esecuzioni dal nucleo JS, senza React.
  - [kit] La validazione del nucleo dà gli stessi esiti del motore (FR-4) sugli stessi documenti, a parità di contesto (manifest, segreti, funzioni disponibili); se il contesto cambia tra le due, vale l'esito del motore.
- **FR-40 — Editor dei workflow** (prd.md:L1047-L1067; v1): il tenant compone un workflow come grafo: aggiunge task (funzioni dei worker, nodi personalizzati, ogni altro costrutto di FR-1), li collega, configura fan-out e join, sceglie i segreti (UJ-2).
  - [kit] L'editor mostra solo funzioni, nodi personalizzati e segreti disponibili al tenant.
  - Il tenant definisce lo schema di input del workflow (FR-56) dall'editor.
  - Un campo dello schema di input di un task prende il valore da: valore fisso; campo compatibile dell'input del workflow (FR-56), dell'output di un task a monte o dell'elemento corrente di un fan-out (FR-3); oppure espressione jq (modo avanzato).
  - Un campo obbligatorio senza fonte è un errore. Per un sotto-workflow lo schema di input è quello del workflow chiamato (FR-56).
  - Criterio: il workflow di UJ-2 si compone senza scrivere jq.
  - Ogni connessione richiesta dalla funzione o dal nodo personalizzato di un task (FR-55) ha un segreto scelto tra quelli che la soddisfano (FR-32).
  - Un grafo non valido mostra l'errore sul task che lo causa prima del salvataggio; tipi, campi obbligatori e connessioni senza segreto si controllano con gli stessi esiti dell'invio (FR-4).
  - Il workflow salvato è un documento OWS valido secondo lo schema di topictoast (sottoinsieme OWS più estensione DAG).
- **FR-41 — Vista delle esecuzioni** (prd.md:L1069-L1085; v1): il tenant segue un'esecuzione in tempo reale: stato di ogni task, attività del fan-out, tentativi, input, output, errori.
  - [kit] Un cambio di stato appare nella vista entro 1 secondo.
  - Un fan-out da migliaia di elementi resta leggibile (attività raggruppate con conteggi per stato).
  - Il tenant lancia un workflow dal kit con un modulo generato dallo schema di input (FR-56), JSON come modo avanzato; un workflow senza schema si lancia solo con il JSON.
  - Un lancio dal kit porta una chiave di idempotenza per ogni intenzione di lancio (FR-6): ripeterlo, anche dopo errore di rete o rinnovo della credenziale, restituisce la stessa esecuzione.
  - Con il ruolo tenant la vista non mostra identificativi dei worker.
  - Durante la ripresa dopo un crash (NFR-15) la vista dice in modo neutro che gli aggiornamenti sono in pausa e riprendono da soli; stati e righe restano visibili; nessun testo nomina il motore o il crash.
- **FR-42 — Gestione dei segreti** (prd.md:L1087-L1092; v1): il tenant aggiunge, sostituisce ed elimina i propri segreti da un componente React.
  - Il componente non mostra mai il valore di un segreto salvato (FR-32).
- **FR-43 — Editor delle pipeline stream** (prd.md:L1094-L1103; v1): il tenant scrive, valida, attiva e sospende le proprie pipeline stream, sceglie la destinazione e vede stato e ultime righe emesse (UJ-3).
  - Gli errori di validazione della query compaiono con riga e colonna prima del salvataggio.
  - Durante la ripresa dopo un crash (NFR-15) l'editor dice in modo neutro che gli aggiornamenti sono in pausa e riprendono da soli; stato e ultime righe emesse restano visibili; nessun testo nomina il motore o il crash.
- **FR-44 — Aspetto personalizzabile** (prd.md:L1105-L1116; v1): lo sviluppatore della piattaforma ospite adatta colori, caratteri, testi e lingua dei componenti React.
  - Nessun componente mostra nome o logo di topictoast senza un'opzione esplicita; un controllo automatico sul bundle lo verifica.
  - I testi sono tutti sostituibili; un testo mancante o vuoto ricade sul predefinito della lingua; la v1 include italiano e inglese.
  - Il kit segnala le coppie di colori della piattaforma ospite sotto la soglia di contrasto (NFR-9): errore in sviluppo, avviso in produzione.
- **FR-57 — Pianificazioni nel kit** (prd.md:L1118-L1131; v1): il tenant crea, vede, modifica, sospende, riprende ed elimina dal kit la propria pianificazione di un workflow (FR-6), con input e prossima scadenza.
  - La pianificazione del kit è un trigger del tenant (FR-26), non parte del workflow: cambiarla non modifica il workflow.
  - È distinta dalla pianificazione del documento (FR-1, FR-6), che il tenant vede nel kit in sola lettura, come il resto di un workflow definito come codice.
  - Al massimo una pianificazione per workflow per tenant.
  - Il tenant pianifica anche i workflow dell'integratore e quelli definiti come codice, che restano in sola lettura (FR-29, FR-35).
  - [SDK] Le stesse operazioni si fanno tramite SDK (FR-38).
- **FR-58 — Area workflow ed elenco dei workflow** (prd.md:L1133-L1148; v1): lo sviluppatore della piattaforma ospite incorpora un componente unico, l'area workflow, con navigazione interna tra elenco dei workflow, editor con pianificazione, esecuzioni, segreti e pipeline stream; ogni sezione esiste anche come componente singolo (UJ-2).
  - [kit] L'elenco mostra i workflow del tenant e quelli dell'integratore, con la provenienza.
  - Dall'elenco il tenant apre un workflow, ne crea uno o elimina uno dei suoi; l'integratore elimina i propri tramite SDK (FR-38).
  - Eliminare un workflow lascia finire le esecuzioni in corso, elimina le sue pianificazioni, lascia lo storico (FR-31).
  - Se un altro workflow lo usa come sotto-workflow o una pipeline stream lo ha come destinazione, l'eliminazione è rifiutata con l'elenco di chi lo usa.
  - L'area mostra solo le risorse del tenant e i workflow dell'integratore (FR-26, FR-29), e nessun marchio di topictoast (FR-44).
  - Finché esiste solo il tenant predefinito (§4.6), l'area non mostra la scelta del tenant.

#### 4.10 Dashboard

- **FR-45 — Accesso** (prd.md:L1156-L1165; v1): l'operatore accede con una credenziale generata al primo avvio; nessuna password predefinita.
  - [dashboard] Senza credenziali valide non mostra alcun dato.
  - [dashboard] Alla scadenza della sessione torna alla pagina d'accesso senza mostrare dati.
  - La pagina d'accesso dice dove trovare la credenziale del primo avvio, senza mostrarla.
  - La credenziale del primo avvio non compare mai nei log (FR-52).
- **FR-46 — Esecuzioni** (prd.md:L1167-L1180; v1): l'operatore vede le esecuzioni di tutti i tenant, con filtri per tenant, workflow e stato (compreso "in coda") e ordinamento per tempo in coda.
  - Tempo in coda = età del più vecchio tra task e attività in coda dell'esecuzione, anche per un'esecuzione già "in esecuzione"; senza task né attività in coda non ha tempo in coda.
  - [dashboard] Da un'esecuzione si arriva al dettaglio di ogni task, come in FR-41, ma senza i valori dei segreti.
  - [dashboard] Dal dettaglio l'operatore annulla l'esecuzione (FR-10, FR-28).
  - Un'azione che cambia stato è accettata solo se parte dalla dashboard stessa: una richiesta di annullamento da una pagina di un'altra origine è rifiutata.
- **FR-47 — Pipeline stream** (prd.md:L1182-L1185; v1): l'operatore vede le pipeline stream attive, rallentate e sospese.
  - Per ciascuna: causa, ritardo sul watermark, eventi al secondo, memoria di stato, eventi in ritardo scartati, ultimo checkpoint.
- **FR-48 — Worker** (prd.md:L1187-L1197; v1): l'operatore vede i worker collegati, le funzioni registrate, le attività in corso per worker e da quanto tempo, i worker che non rispondono, e la cronologia recente delle attività di ogni worker.
  - Fasi della cronologia: ricevuta, in esecuzione, completata, fallita, ritentata, annullata.
  - [dashboard] Segnala disco pieno e rifiuto di lavoro nuovo (NFR-6) con un avviso distinto dalla ripresa dopo un crash del motore (NFR-15).
  - Un worker che registra una funzione già registrata con connessioni diverse è segnalato all'operatore (FR-33).
  - ⚠ Il rimando a FR-33 per il caso "funzione già registrata con connessioni diverse" non trova quella regola nel testo di FR-33 (che tratta solo il manifest registrato dopo la pubblicazione). → la regola vicina è in FR-33 (manifest registrato dopo la pubblicazione con una connessione in più: task fallito); la segnalazione all'operatore è propria di FR-48.
- **FR-49 — Quote** (prd.md:L1199-L1204; v1): l'operatore vede per ogni tenant l'uso delle quote e i tenant vicini al limite e al limite.
  - La soglia di "vicino al limite" ha un valore predefinito documentato (NFR-14).

#### 4.11 Installazione e funzionamento

- **FR-50 — Installazione minima** (prd.md:L1214-L1238; v1; sandbox forte opzionale FR-54 richiede KVM): l'integratore installa due servizi, topictoast e il bus, più il database a scelta; topictoast avvia e ferma da sé i processi ausiliari (sandbox leggera e, separati dal motore, processi per espressioni jq, query delle pipeline stream e `call http`: AD-6).
  - Il numero di processi non è la differenza (§1.1): lo sono RAM e carico sul database.
  - Con SQLite incorporato, topictoast e il bus sono gli unici servizi da installare e avviare; ogni altro processo lo gestisce topictoast.
  - Un processo ausiliario inattivo oltre un tempo configurabile è fermato e riavviato a freddo alla prima richiesta; il tempo di riavvio è riportato nel benchmark (FR-53); NFR-2 e NFR-12 si misurano a regime.
  - ⚠ Il testo è marcato `[ASSUMPTION: idea di Marco da studiare in architettura, con NFR-1 che conta tutti i processi attivi; obiettivo del riavvio a freddo in §11]`. → si studia in S-2 (M-28, AR-14).
  - Con la configurazione predefinita un workflow di esempio gira senza modificare nulla.
  - Il tenant predefinito esiste dal primo avvio (§4.6). Con credenziale dell'integratore un workflow pubblicato dall'SDK senza tenant va al tenant predefinito; con credenziale di un tenant va sempre al tenant della credenziale (FR-26). Renderlo disponibile a tutti i tenant come workflow dell'integratore (FR-29) è scelta esplicita.
  - L'installazione predefinita non richiede Kubernetes, KVM né root; solo la sandbox forte opzionale (FR-54) richiede KVM.
  - Nessuna chiave o credenziale che protegge identità, segreti o dashboard ha un valore predefinito noto: due installazioni nuove hanno valori diversi; topictoast rifiuta di partire con un valore presente negli esempi della documentazione.
  - Con la configurazione predefinita accetta connessioni di rete solo da localhost.
  - ⚠ §4.11 descrive "due servizi più il database"; la descrizione di sezione cita bus Iggy con SQLite o Postgres, "da confermare in architettura". → deciso da AD-1 (bus servizio separato, SQLite o Postgres); il bus, Iggy o Valkey, lo sceglie S-1 (AR-9).
- **FR-51 — Aggiornamenti** (prd.md:L1240-L1248; v1): l'integratore aggiorna topictoast a una versione nuova senza perdere stato.
  - L'aggiornamento migra lo stato da solo; le esecuzioni in corso riprendono dopo il riavvio.
  - L'aggiornamento di qualunque componente, bus incluso (che oggi cambia formato su disco tra versioni senza migrazione), non perde attività accettate né esecuzioni in corso.
- **FR-52 — Osservabilità** (prd.md:L1250-L1257; v1): topictoast espone metriche e log strutturati in formati standard.
  - Le metriche coprono attività al secondo, latenza di avvio delle attività, code per tenant, pipeline stream (stesse grandezze di FR-47).
  - I log non contengono segreti, credenziali, nomi visualizzati né input e output delle attività.
- **FR-53 — Benchmark riproducibile** (prd.md:L1259-L1278; v1, "Benchmark pubblico riproducibile" §8.1): il repository contiene il benchmark dei requisiti del §5, eseguibile da chiunque sulla macchina di riferimento con un solo comando.
  - Produce: RAM a riposo, latenza p50 e p99 di avvio delle attività, attività al secondo, istruzioni SQL per attività, istruzioni SQL al giorno con il carico di hatchet-dev/hatchet#5153 (SM-2), eventi al secondo delle pipeline stream.
  - Riporta attività, esecuzioni ed eventi accettati e poi persi dopo un guasto iniettato durante i test di NFR-3 e NFR-5 (obiettivo: zero, NFR-6).
  - Include una corsa a carico misto: latenza di NFR-2 misurata mentre una pipeline stream gira a ritmo dichiarato, un tenant è oltre quota e un tenant avvia a ripetizione con gli schemi più complessi consentiti (FR-55).
  - Dichiara forma del carico, dimensione dei payload, insieme di dati, modalità di scrittura; riporta senza obiettivo la RAM sotto carico, la RAM per pipeline stream attiva e il tempo di riavvio a freddo dei processi ausiliari (FR-50).
  - Misura il tempo di ripresa dopo un crash iniettato del motore (NFR-15).
  - Le corse di NFR-2 e NFR-3 comprendono il controllo di input e output delle attività sugli schemi del manifest (FR-55).

### NonFunctional Requirements

Premessa (prd.md:L1282-L1285): tutti misurati con il benchmark di FR-53 sulla macchina di riferimento, con SQLite e con Postgres sulla stessa macchina, con forma del carico, dimensione dei payload, insieme di dati e modalità di scrittura dichiarati nel benchmark. RAM sotto carico e per pipeline stream attiva: riportata senza obiettivo. Riferimenti concorrenti (research-footprint.md, benchmark vendor non confrontabili, L1297-L1301): Temporal dev server ~150 MB a riposo (2023); Windmill ~12 ms overhead con worker dedicato; Restate p50 15 ms; Hatchet 2.000 run/s con DB da 8 CPU.

- **NFR-1 — RAM a riposo** (prd.md:L1289): obiettivo v1 ≤ 80 MB.
  - Misura: somma RSS dei processi di topictoast in esecuzione (motore e ogni processo ausiliario attivo) e del bus, con SQLite, 60 s dopo l'avvio, insieme di dati dichiarato (tenant, workflow, storico), nessuna esecuzione né pipeline stream attiva.
  - Esclusi: worker, stato delle pipeline stream, pool della sandbox forte (FR-54).
- **NFR-2 — Latenza di avvio delle attività** (prd.md:L1290): p50 ≤ 5 ms, p99 ≤ 25 ms.
  - Misura: da attività pronta (dipendenze soddisfatte) a ricezione da parte del worker; worker sulla stessa macchina; carico 1.000 attività/s.
- **NFR-3 — Throughput** (prd.md:L1291): ≥ 10.000 attività/s.
  - Misura: attività vuote, worker sulla stessa macchina, sostenuto 10 minuti; il p99 di avvio è riportato anche a questo ritmo.
- **NFR-4 — Carico sul database** (prd.md:L1292): ≤ 5 istruzioni SQL per attività.
  - Misura: media sull'intero ciclo di vita dell'attività durante il test di NFR-3; confronto con Hatchet sullo stesso carico in SM-2.
- **NFR-5 — Pipeline stream** (prd.md): ≥ 100.000 eventi/s per pipeline con filtro e conteggio su finestra fissa, confermato dallo spike S-3 (105.000 eventi/s per 10 minuti, ADR del 2026-11-28).
  - Misura: eventi scritti via API o SDK di FR-18 e consumati dalla pipeline, sostenuto 10 minuti.
- **NFR-6 — Nessuna perdita di dati** (prd.md:L1305-L1310): nessuna attività, esecuzione o evento accettato va perso per riavvio, crash di qualunque processo o spegnimento improvviso dell'host.
  - Vale anche durante i test di NFR-3 e NFR-5.
  - Backup e ripristino coerenti di DB e bus: fuori v1 (§8.2), documentati come limite.
  - A disco pieno: rifiuta lavoro nuovo con errore esplicito, non perde l'accettato; ogni scrittura rifiutata lascia intatto lo stato precedente.
- **NFR-7 — Confini stretti** (prd.md:L1311-L1318): ogni input esterno (API, SDK, documenti, manifest, query SQL, codice dei nodi personalizzati) validato; input non valido = errore di validazione, mai crash del motore.
  - Contenuto fornito dai tenant (nomi, descrizioni, errori, input, output, righe emesse, campi e metadati dei manifest, nomi e descrizioni dei campi degli schemi, nomi visualizzati) sempre mostrato come testo, mai HTML o script, in dashboard e componenti React.
  - Test: salva un payload di script in ogni campo testuale di un tenant, apre ogni pagina della dashboard e ogni componente, con entrambi i ruoli del kit, verifica che nessuno script giri.
- **NFR-8 — Sicurezza** (prd.md:L1319-L1332): isolamento tra tenant (FR-26), sandbox contro codice ostile (FR-16, FR-17), segreti cifrati e mai nei log (FR-33, FR-34).
  - Ogni connessione di rete con segreti, credenziali o dati dei tenant (worker-motore, motore-bus, motore-database se di rete, browser, SDK e CLI verso il motore) è cifrata e autentica il server; in chiaro accettata solo da localhost.
  - Config predefinita: un worker che si collega in chiaro da altro host è rifiutato.
  - Prima del primo rilascio pubblico: (a) sandbox a entrambi i livelli passa test con codice ostile noto (fuga, esaurimento risorse, accesso alla rete interna, lettura della chiave della cassaforte); (b) test automatico per ogni tipo di risorsa e punto di accesso (API, SDK, eventi in tempo reale, liste e conteggi): un tenant non legge né modifica risorse di un altro; una credenziale di tenant è rifiutata su ogni operazione riservata all'integratore; una credenziale integratore firmata per il kit è rifiutata su ogni operazione fuori dal suo ambito (FR-28).
- **NFR-9 — Accessibilità** (prd.md:L1333-L1338): tutti i componenti del kit (area workflow compresa) e la dashboard: WCAG 2.1 AA, compreso uso da tastiera dell'editor del grafo.
  - AA garantito con i colori predefiniti; con i colori della piattaforma ospite vale il controllo di FR-44.
  - Workflow ed esecuzione leggibili anche senza grafo, in vista alternativa in sola lettura; la modifica resta sul canvas, usabile da tastiera.
  - Verifica: controllo automatico su ogni componente + prova con NVDA e VoiceOver sui percorsi UJ-2 e UJ-4.
- **NFR-10 — Piattaforme** (prd.md:L1339): Linux x86_64 e arm64 `[ASSUMPTION]`.
- **NFR-11 — Licenza** (prd.md:L1340-L1341): tutto il codice Apache-2.0, nessuna parte a pagamento; dipendenze con licenze compatibili.
- **NFR-12 — Sandbox** (prd.md:L1294): `[ASSUMPTION: avvio p50 ≤ 10 ms e memoria ≤ 64 MB per istanza della sandbox leggera; ≤ 10 ms per la sandbox forte quando il pool ha una sandbox libera; da confermare dopo lo spike]`.
  - Misura: da attività pronta a inizio esecuzione del nodo personalizzato; memoria per istanza durante un nodo vuoto.
- **NFR-13 — Tempo al primo workflow** (prd.md:L1295): `[ASSUMPTION: ≤ 15 minuti]` da download o clone a workflow di esempio completato, seguendo la guida rapida, macchina pulita, persona che non conosce topictoast (UJ-5).
- **NFR-14 — Valori predefiniti e limiti** (prd.md:L1342-L1363): i valori li fissano le spec delle storie.
  - Ogni campo omettibile (workflow, pipeline stream, worker, nodo personalizzato, config nuovo tenant) ha valore predefinito documentato e limitato, applicato dopo la validazione. Almeno: retry; timeout di task ed esecuzione; limite di attesa prima della consegna; intervallo di battito dei worker; concorrenza di un worker; limiti dei nodi personalizzati; ritardo tollerato e tolleranza sul tempo dell'evento; chiusura finestre di uno stream inattivo; intervallo dei checkpoint; conservazione degli stream; recupero scadenze perse di una pianificazione; quote di un nuovo tenant; soglia "vicino al limite" (FR-49); funzioni e destinazioni di rete per i tenant; inattività prima di fermare un processo ausiliario.
  - Ogni tempo e limite dichiarabile da un tenant ha un massimo fissato dall'installazione; nessuna combinazione di predefiniti e dichiarati lascia un'esecuzione, attività o finestra ferma senza fine.
  - Grandezze controllate dal tenant (elementi di un fan-out, dimensione di documento e query, numero di task, profondità di annidamento e ricorsione dei sotto-workflow, dimensione risposta `call http`, dimensione/profondità/numero campi di manifest e schemi) hanno limite con predefinito documentato: superarlo = errore di validazione o task fallito con limite e valore, senza creare attività né fermare il motore.
  - Workflow o pipeline stream senza campi facoltativi gira coi predefiniti, termina / chiude le finestre.
- **NFR-15 — Ripresa dopo un crash** (prd.md:L1364-L1368): motore a nodo singolo nella v1; dopo crash riparte da solo e torna a consegnare attività entro 30 s, senza intervento.
  - I worker si ricollegano da soli, finiscono le attività che avevano; i risultati scritti nel frattempo non si perdono.
  - Durante la ripresa non partono nuove esecuzioni né task successivi (pausa, non perdita, NFR-6).
  - Misura: benchmark con crash iniettato (FR-53).
- **NFR-16 — Schermi** (prd.md:L1369-L1371): kit frontend e dashboard usabili per intero su desktop e telefono, da 360 px di larghezza, input touch, compresa composizione nell'editor del grafo e nell'editor SQL delle pipeline stream; sul telefono la disposizione del grafo è automatica.

### Additional Requirements

#### Avvio del progetto
- **AR-1** Nessuno starter template: la spine non ne nomina nessuno (cercato "starter" in spine e ADR: zero occorrenze). Greenfield scaffold. Verificato sul repo: esistono solo `.githooks/` (commit-msg, post-commit, pre-commit, pre-push), `scripts/review-diff.sh`, `docs/`, `_bmad*`; NON esistono `Cargo.toml`, `crates/`, `packages/`, `dashboard/`. (spine "Albero dei sorgenti" L719-746; AD-15)
- **AR-2** La prima storia crea il workspace Cargo `crates/`: `ows`, `domain`, `protocol`, `engine`, `adapters`, `sdk`, `cli`, `napi`, `bench`; `spikes/<nome>/` fuori dai workspace via `exclude` nel `Cargo.toml` di radice. Test `purity.rs` sulle dipendenze vietate di `ows`/`domain`/`protocol`. `missing_docs` attivo. (spine AD-15, AD-16, L739; costituzione III, VIII)
- **AR-3** La prima storia crea il workspace npm `packages/`: `ui`, `core`, `react`, `sdk`, `validator`; e `dashboard/` (skill `dashboard-react`). Nomi di crate e pacchetti fissati: regole, `purity.rs` e gate li citano. (spine AD-15, L55)
- **AR-4** Gate (CLAUDE.md): `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `(cd dashboard && npm run gate)`; `(cd packages && npm run gate)`; `cargo build --manifest-path crates/domain/Cargo.toml --target wasm32-wasip1`. Gli ultimi tre scattano quando la cartella esiste: lo scaffold deve definire `npm run gate` (Biome 2.6.6) in `packages/` e `dashboard/`. Il pre-push li esegue (tetto di tempo, `SKIP_*` per bypass); nessuna CI fino alla produzione. (CLAUDE.md "Gate"; `.githooks/pre-push`)
- **AR-5** Dev locale: un solo `docker compose`, senza profili né politiche di riavvio, avvia bus, Postgres e RustFS insieme; il motore sceglie gli adattatori dalla configurazione. Test: testcontainers + wiremock. (CLAUDE.md "Builds"; spine "Dispiegamento")
- **AR-6** Ogni clone: `git config core.hooksPath .githooks`; `CARGO_TARGET_DIR` su `/home`, mai `/tmp`; dipendenze pinned a versione esatta pubblicata da almeno 2 settimane (eccezione: fix di vulnerabilità RustSec/GHSA/CVE, anche transitiva nei lockfile, citando l'avviso nel commit, E-9). Stack fissato nella spine (Rust 1.99.1, OWS 1.1.0, datafusion 55.2.0, napi 3.16.1, napi-derive 3.7.0, napi-build 2.6.0, @napi-rs/cli 3.10.12, @napi-rs/wasm-runtime 1.2.7, rustls 0.23.46, cucumber 0.23.0, testcontainers 0.28.0, wiremock 0.6.5, react/react-dom 19.3.1, react-router 8.5.1, @tanstack/react-query 5.107.1, @vanilla-extract/css 1.21.3, storybook 10.6.6, vitest 5.1.0, @testing-library/react 16.3.3, @playwright/test 1.65.0, playwright-bdd 9.2.1, @biomejs/biome 2.6.6; PostgreSQL 18.7 e 19.0). OWS 1.1.0 compie 2 settimane il 2026-11-26. Da non usare: SDK Rust ufficiale OWS (né dipendenza né fork), jtd, denormalized, sled, fred, elkjs, ts-rs. (spine "Stack", L581-626; CLAUDE.md)
- **AR-7** Le UX spines (`DESIGN.md`, `EXPERIENCE.md`) vanno allineate a `packages/ui` e a T-57 con un Update di `bmad-ux` prima della prima storia del design system. (CLAUDE.md "Current state")

#### Spike prima dello sprint 1
- **AR-8** Ordine e regole: bus (3-4 giorni) → sandbox (2-3) → pipeline stream (~3) → kit nel browser (mezza giornata), ~2 settimane. Sprint 1 solo dopo tutti e quattro. Codice in `spikes/<nome>/` (fuori dal gate). Ogni spike chiude con ADR misurato in `docs/decisions/` + PR che aggiorna la spine (intento Update, AD-n stabili); ogni ADR controlla la licenza delle tecnologie contro NFR-11. (spine L409-416; CLAUDE.md)
- **AR-9** S-1 Bus, scope: Apache Iggy (server-0.10.0, crate `iggy` 0.12.0) contro Valkey 9.1.4 come livello caldo con scarico (payload in archivio AD-17, nel DB solo stato tipizzato; Iggy 0.10 non legge i dati della 0.9). Collegamento worker, attività sempre sul log: (1) diretti sui topic, ack/retry/timeout dal motore; (2) dal motore su gRPC+TLS (tonic 0.14.9); (3) dal motore su QUIC p2p iroh 1.4.2 (solo con eccezione di sicurezza). Anche: modalità di scrittura durevole (M-18), upgrade senza perdite, spazio per tenant con id interno (M-24, T-66), costo cifratura per tenant (C-12), costo chiamata verso ausiliari, timer durevoli (retry con attesa attraverso crash iniettato, `wait`, C-3, battiti), trasporto delle righe di log dei worker (FR-59), benchmark base (M-49), redis 1.8.1. (spine L418-436)
- **AR-10** S-1 misure per variante: latenza p50/p99 di un passo del DAG, throughput al ritmo di NFR-3, CPU del motore, payload 1 KB-1 MB, attività lenta; le corse NFR-2/NFR-3 includono `if`, mappatura jq e controllo input/output nell'ausiliario (FR-53). (spine L437-440)
- **AR-11** S-1 criteri pass/fail (decidono i numeri): upstream da archivio con proiezione indietro fino a 1 s (E-4, AD-3) misurato contro NFR-2; scrittura durevole p50 ≤ 5 ms, p99 ≤ 25 ms; NFR-3 ≥ 10.000 attività/s; NFR-4 ≤ 5 istruzioni SQL per attività (SQLite e Postgres, righe e WAL contati) e SM-2; NFR-1 ≤ 80 MB con SQLite, bus e ausiliari attivi; NFR-6 zero perdite con guasto iniettato (DB fermo e disco pieno: lavoro nuovo rifiutato con codice, AD-2); NFR-15 ripresa entro 30 s; proiezione idempotente; FR-51 upgrade senza perdite; chiamata tra processi sul percorso caldo entro NFR-2; attività lenta non porta altri tenant oltre NFR-2 (FR-13); autenticazione bus obbligatoria anche da localhost, credenziali generate, nessun predefinito (FR-12, FR-50). (spine L441-452)
- **AR-12** S-1 variante (1): criteri non numerici nell'ADR (segreti e decifratura per il worker diretto, mai sul bus, giro in più dentro NFR-2; chi applica concorrenza, quote, credenziali FR-12/FR-13/FR-30; compatibilità upgrade FR-51). Condizioni di sicurezza: nessuna chiave tenant né segreto ai worker; C-11 applicabile dal bus; eventi di stato scritti solo dal motore (AD-2). Se ne manca anche una, la variante 1 è esclusa qualunque siano i numeri (D-1). (spine L453-459)
- **AR-13** S-1 da fissare e provare nell'ADR per candidato e variante: posizione bus per partizione (topic, partizione, offset) salvata per partizione nella stessa transazione del gruppo, token opaco che non rivela il volume di altri tenant; conservazione legata alla posizione del proiettore; DB lento con proiezione > 1 s; controlli AD-18 sullo stato accettato più vecchio entro NFR-4/SM-2; SQLite senza `SQLITE_BUSY` né FK verso righe non proiettate; nessun effetto esterno prima della scrittura durevole della decisione; annullamento (FR-10) e arresto per timeout; credenziale bus per tenant in sola lettura per il processo stream (se il bus non la impone, il motore alimenta il processo); varianti (2)/(3): credenziale worker legata alla connessione, verifica server obbligatoria nell'SDK, iroh con relay e discovery esterni spenti e identità nodo legata a C-11. ADR `docs/decisions/YYYY-MM-DD-bus.md`; aggiorna AD-1, AD-2, AD-3, AD-17, AD-18, C-8 e protocollo worker (AD-12). Sblocca: FR-6…FR-14, FR-18, FR-41, FR-50, FR-51, FR-53, NFR-1…NFR-6, NFR-15, SM-2. (spine L460-479)
- **AR-14** S-2 Sandbox, scope: tecnologie dei due livelli (M-25): leggera senza KVM né root (wasmtime 50.0.3), forte microVM con pool (Firecracker v1.17.0; cloud-hypervisor v54.1 solo con eccezione); nessuna convivenza tra tenant nella leggera, costo RAM/avvio (M-26); formato modulo compilato TS e Rust (non esclude Python/Go dopo la v1, costituzione VII da riesaminare), declassamento della forte, parte del pool per tenant, pulizia, limite nodo vs timeout task (M-27); quali ausiliari separare, arresto per inattività, riavvio a freddo (M-28, PRD §11 n.9). (spine L481-492)
- **AR-15** S-2 criteri: NFR-12 (leggera: avvio p50 ≤ 10 ms e ≤ 64 MB per istanza; forte: ≤ 10 ms con sandbox libera nel pool; [ASSUMPTION]); NFR-1 con ausiliari attivi; arresto per inattività non peggiora p99 NFR-2 (SM-C2); FR-16 (fuori dal processo della chiave, mai due tenant in un'istanza); FR-17 (CPU, memoria, tempo); TS e Rust compilati girano nella leggera; codice ostile noto a entrambi i livelli (NFR-8: fuga, esaurimento, rete interna, lettura chiave); x86_64 e arm64 (NFR-10); default senza KVM né root (FR-50). (spine L493-499)
- **AR-16** S-2 da provare: come jq, `call http`, validazione e dry-run SQL evitano mix di tenant (misurato vs NFR-2 e NFR-1); confinamento Landlock + seccomp di ogni ausiliario prima di toccare contenuto tenant, con ripiego dove manca Landlock; test "lettura della chiave principale" per ogni sottocomando ausiliario (file chiave, `/proc` del motore, file DB, configurazione, cartella di altro tenant); il tenant carica solo bytecode, validato e compilato nell'ausiliario, cache nativa scritta solo da topictoast. ADR `…-sandbox.md`; aggiorna AD-6. Sblocca FR-15…FR-17, FR-54, NFR-7, NFR-8, NFR-10, NFR-12. (spine L500-510)
- **AR-17** S-3 Pipeline stream, scope: costo di stato, finestre, watermark e checkpoint costruiti sopra DataFusion (M-31), checkpoint nell'archivio payload (AD-17); dialetto SQL chiuso (M-33); chiusura finestre su stream inattivo (M-34); identità dell'evento e dedup dei blocchi nel processo delle pipeline (M-35); finestra/ritardo/checkpoint nel dialetto (T-56). Caso: UJ-3 del PRD (conteggio per ASN, finestre 5 min, soglia > 50, un avvio per riga, riavvio a metà finestra dall'ultimo checkpoint), [ASSUMPTION] finché non arriva un caso reale; un secondo tenant con processo proprio e query ostile prova FR-25. (spine L512-521)
- **AR-18** S-3 criteri: NFR-5 ≥ 100.000 eventi/s ([ASSUMPTION]) misurato dall'API FR-18 (a blocchi) con scrittura durevole, cifratura per tenant e guasto iniettato senza eventi accettati persi; FR-19…FR-25 sul caso; riavvio dal checkpoint senza perdite (FR-22), un solo avvio per riga (FR-24, C-4); SQL dei tenant non raggiunge tabelle esterne né funzioni di file/rete di DataFusion; NFR-1 con DataFusion nel binario e nessuna pipeline attiva; RAM per processo-tenant vs NFR-1; latenza del buffering operatori DataFusion 55.x. (spine L522-528)
- **AR-19** S-3 da provare: SQL controllato sull'AST contro il dialetto chiuso prima della pianificazione; niente DDL/DML/istruzioni; nessun catalogo file né object store registrato; spill su disco spento o in cartella tenant cifrata; limite memoria di sistema oltre al pool DataFusion (anche per il dry-run); casi ostili: `repeat` enorme, prodotto cartesiano, costanti enormi. ADR `…-stream-pipelines.md`; aggiorna AD-13. Sblocca FR-18…FR-25, NFR-5. (spine L529-535)
- **AR-20** S-4 Kit nel browser, scope: validatore Rust napi-rs `wasm32-wasip1` senza thread, da pagina senza isolamento cross-origin, su Chromium, Firefox, Safari, anche telefono (Safari iOS, Chrome Android; upstream prova solo Chromium); provare `threadlessInitialMemory`, `asyncInit: true`, bundle ESM preconfezionato; stack wasm fisso a 64 MB in napi-build 2.6.0; dipende da emnapi/@emnapi/core/@emnapi/runtime 2.0.0-beta.3 (pre-release). (spine L537-543)
- **AR-21** S-4 misure e criteri: peso gzip con jq (stima senza jq: 60-90 KB di colla JS + `.wasm`), tempo di avvio, latenza di una validazione, memoria iniziale. Pass: gira senza COOP/COEP sui tre browser; regge CSP restrittiva con `connect-src` verso il motore, `'wasm-unsafe-eval'` e fetch del `.wasm` (AD-19); con `'wasm-unsafe-eval'` negato, comportamento scelto nell'ADR (errore C-7 o validazione via API); esiti identici al validatore nativo (FR-39); bundle host senza la stringa "topictoast" (FR-44). ADR `…-kit-browser.md`; aggiorna AD-11 e AD-19; soglie di peso/avvio le fissa Marco nell'ADR. Sblocca FR-39…FR-44, FR-57, FR-58. (spine L544-552, L820)

#### Invarianti che ogni storia rispetta
- **AR-22** AD-1: motore Rust a nodo singolo, nessuna HA nella v1, riparte da solo entro 30 s (NFR-15); bus servizio separato; DB SQLite o Postgres; ogni altro processo (sandbox, ausiliari, pool microVM) lo avvia/ferma il motore; default senza Kubernetes/KVM/root, ascolto solo localhost; worker sempre attivi, anche remoti su TLS. (spine L59-69)
- **AR-23** AD-2: "accettato" = scrittura durevole sul log del bus prima della risposta; DB proiezione scritta a gruppi da un solo scrittore (proiettore), indietro ≤ 1 s, idempotente; ripresa dall'ultima posizione applicata; bus non scarta posizioni non applicate; DB fermo/disco pieno = lavoro nuovo rifiutato con codice C-7 + avviso FR-48; evento non applicabile = guasto (proiettore si ferma, non salta); eventi di stato scritti solo dal motore; un solo motore per installazione (lock esclusivo all'avvio, il secondo esce con codice); segreti e chiavi avvolte non passano dal bus (C-12). (spine L71-90)
- **AR-24** AD-3: input/output dei task sul bus; il proiettore scrive ogni payload nell'archivio (AD-17) all'applicazione, indirizzato dall'impronta, nel DB solo riferimento tipizzato (impronta, dimensione); uscite a monte lette dall'archivio; indirizzo con id interno del tenant, nessuna dedup tra tenant; payload cifrati con chiave del tenant. (spine L92-102)
- **AR-25** AD-4: documenti OWS, schemi dei manifest e posizioni dei nodi in tabelle tipizzate, mai JSON; documento rigenerato dalle tabelle in forma canonica (ordine dei task conservato, `dag` compreso); ogni cambio di spec OWS è una migrazione. (spine L104-112)
- **AR-26** AD-5: tenant = UUIDv7 interno; `tenant` del JWT è nome esterno tradotto (`default` riservato); mai nomi/valori JWT in percorsi, topic, chiavi; integratore = proprietario con id riservato e chiave; un solo adattatore del repository con filtro tenant sempre applicato, test a due tenant per ogni metodo con SQLite e Postgres, RLS Postgres (utente non superuser, senza `BYPASSRLS`) come seconda barriera; tenant e permessi solo dalla credenziale verificata; chiavi di idempotenza/versione/indirizzo includono l'id interno (test con stessa chiave in due tenant); stesso filtro per eventi (AD-18). (spine L114-132)
- **AR-27** AD-6: il processo motore tiene la chiave principale e non interpreta mai contenuto tenant (documenti, manifest, schemi, jq, `call http`, SQL stream): lo fanno ausiliari senza chiave, con limiti CPU/memoria/tempo; eccezione: processo stream del tenant con la chiave di quel tenant; sandbox e processo stream fuori dal motore, mai dati di due tenant in un'istanza (FR-16); funzioni che interpretano contenuto tenant dietro un permesso presente solo in modalità ausiliario (test: la modalità motore non le chiama); ausiliario avviato con ambiente vuoto, config e versione IPC via IPC (versione diversa = esce); variabile della chiave rimossa dall'ambiente dopo la lettura; `PR_SET_DUMPABLE` 0, niente core dump, segreti in tipi che si azzerano con `Debug` mascherato; risposte degli ausiliari = input esterno con tetti di dimensione/profondità; corpi API con tetti di dimensione, profondità, decompressione prima dell'analisi; `call http` senza cookie store né pool condiviso, `Authorization` mai su redirect verso altra origine. (spine L134-157)
- **AR-28** AD-7: chiave principale → chiave per tenant → AES-256-GCM; busta versionata con id chiave, chiave per oggetto/segmento derivata con HKDF, formato unico nel dominio (motore, proiettore, processo stream, archivio); segreti risolti alla consegna, mai sul bus; chiave principale generata solo con deposito vuoto, impronta conservata, avvio rifiutato con codice se non corrisponde; cartelle dati 0700 e file 0600, avvio rifiutato con codice se chiave o cartella leggibili da altri; eliminazione tenant in ordine: stop (pipeline, esecuzioni, pianificazioni, sandbox, SSE, richieste) → evento lapide sul bus → chiave cancellata → righe DB e nodi → pulizia asincrona bus e archivio; rilettura salta solo tenant con lapide, indecifrabile senza lapide = guasto; SQLite `secure_delete` + checkpoint WAL `TRUNCATE`, Postgres VACUUM (residuo documentato); test sentinella FR-31 su DB (file compresi), bus, archivio, checkpoint, nodi. (spine L159-183)
- **AR-29** AD-8: `dag` contenitore distinto da `do`, dipendenze in `dependsOn`; schema = sottoinsieme FR-1 di OWS 1.1 + `dag`, costrutto fuori sottoinsieme = errore con costrutto e percorso (C-7); documento senza `dag` resta OWS puro, con `dag` rifiutato dai validatori ufficiali; in `dag` vietati `then`, `exit`, `end`, `switch` (condizioni con `if`); `switch` dell'editor = `do` annidato con `switch` OWS, un solo nodo nel `dag`, forma riconosciuta da una funzione del dominio esposta da `packages/validator`; `$secrets` e `$authorization` vietati nei documenti (anche nel browser); segreti solo al confine della chiamata. (spine L185-202)
- **AR-30** AD-9: dipendenza fallita (dopo i retry) o saltata → task saltato con motivo, e i discendenti; rami indipendenti arrivano in fondo; esecuzione fallita se almeno un task fallito; nessuna eccezione per task nella v1; regola solo nel dominio. (spine L204-211)
- **AR-31** AD-10: ammessi solo `type`, `properties`, `required`, `items`, `enum`, `default`, `description`, minimi/massimi, lunghezze; vietati `$ref`, `oneOf`, `anyOf`, `allOf`, `if`/`then`; altra parola chiave = errore chiaro; sottoinsieme chiuso di `input.schema` OWS con `format: json`; derivabile da tipi TS e Rust; una sola implementazione (validatore del dominio); dettagli in C-10. (spine L213-222)
- **AR-32** AD-11: una sola validazione, Rust nel dominio, a JS solo via napi-rs (nativo Node; browser `wasm32-wasip1` senza thread); niente `wasm-bindgen`, niente `SharedArrayBuffer`/COOP/COEP; il validatore browser copre formato, `dag`, schemi, jq (stessa implementazione Rust), cron; SQL stream validato solo da un ausiliario via dry-run API; contesto di validazione = tipo del dominio servito da risorsa API limitata al tenant; pagina host con `'wasm-unsafe-eval'`, fetch `.wasm`, `connect-src`. (spine L224-237)
- **AR-33** AD-12: una sola API (HTTP+JSON, forma C-16/17/18) per kit, dashboard, SDK, CLI; OpenAPI generato dai tipi Rust di `crates/protocol`; un solo generatore ricava i tipi TS (una volta, in `packages/core`, e per l'SDK TS), fissato dalla prima storia che lo usa; dashboard senza bersaglio proprio; SSE per il tempo reale (FR-41); eventi stream a blocchi; errori C-7; protocollo worker separato, tipi in `crates/protocol`, deciso da S-1. (spine L239-250)
- **AR-34** AD-13: SQL stream su DataFusion, stato/finestre/watermark/checkpoint costruiti da topictoast; nessun altro motore (DBSP escluso, Arroyo solo esempio); un processo ausiliario per tenant, attivo solo con pipeline attive, con la sola chiave del tenant; eventi stream su topic propri del tenant con id interno, letti direttamente solo quelli. (spine L252-262)
- **AR-35** AD-14: scenari cucumber-rs su due livelli: (1) contratto = API pubblica + worker di prova Rust sul protocollo; (2) SDK Rust come integratore; bus e DB reali via testcontainers, servizi esterni con wiremock; avviano il binario del motore già costruito (mai il motore nel test), percorso del binario per gli ausiliari configurabile; kit e dashboard con Playwright BDD. (spine L264-273)
- **AR-36** AD-15: layout `crates/`, `packages/`, `dashboard/`, `spikes/`; `crates/ows` e `crates/protocol` puri, senza dipendenze dal workspace; SDK Rust ufficiale OWS solo riferimento; `packages/ui` usato da `react` e `dashboard/`; la dashboard non istanzia mai il validatore wasm (lo carica pigro `packages/core`); spike fuori da workspace e gate; skill del kit scritta prima della prima storia del kit; tipi dell'API mai ridichiarati a mano nell'SDK Rust. (spine L275-292)
- **AR-37** AD-16: `ows` e `protocol` senza dipendenze dal workspace; `domain` dipende solo da `ows`; nessuno dei tre dipende da I/O, async, HTTP, driver DB (test `purity.rs`); solo `crates/engine` sceglie gli adattatori; ausiliari = sottocomandi dello stesso binario (nessuna seconda composition root); client HTTP in un solo modulo per artefatto (`crates/adapters` per `call http` e RustFS; `crates/sdk` per CLI e bench); nessun client dipende da `engine` o `adapters`; validazione JS solo da `packages/validator`; dashboard solo `packages/ui` + client API di `packages/core`. (spine L294-356)
- **AR-38** AD-17: una sola porta "archivio dei payload" con, fin dalla prima storia, due spazi sotto l'id interno del tenant (immutabile per impronta; con nome sovrascrivibile per checkpoint ed export); adattatore predefinito disco, RustFS (1.0.4) facoltativo; servizi obbligatori restano due (FR-50); storico in Parquet cifrato per tenant solo quando il PRD definisce la funzione, nessun export in chiaro nemmeno provvisorio, DuckDB escluso. (spine L358-371)
- **AR-39** AD-18: SSE emesso subito dopo la scrittura durevole, mai dalla proiezione; posizione bus = token opaco (id SSE, campo `position`, `Last-Event-ID`; fuori conservazione = errore C-7 che chiede ricarica); l'API aspetta fino a 1 s la proiezione del token passato, poi 503 con codice C-7 e `Retry-After`, mai 404; 1 s di FR-41 misurato da capo a capo; eventi filtrati per tenant e ruolo in un solo punto, stessa vista per ruolo di risposte ed eventi; controlli di idempotenza FR-6, C-3, C-4, `If-Match` su tutto lo stato accettato (proiezione + bus riletto); righe/avvii del processo stream via IPC con id deterministico (C-4). (spine L373-392)
- **AR-40** AD-19: kit raggiunge il motore direttamente con CORS limitato alle origini configurate (rifiuta `*`, `null`, match per suffisso) o via proxy host; risorse operatore non accettano origini del kit né `Access-Control-Allow-Credentials`; JS e `.wasm` serviti dall'origine dell'host, mai dal motore; produzione fissa esplicitamente indirizzo e TLS (C-14); SSE via `fetch` con header `Authorization`, mai token in URL o cookie; il motore chiude lo stream alla scadenza della credenziale; CSP host con `connect-src` e `'wasm-unsafe-eval'`. (spine L394-407)

#### Convenzioni
- **AR-41** C-1: UUIDv7 per esecuzioni, task, attività, eventi delle esecuzioni, risorse API, id tenant; id SSE = token posizione bus; id eventi stream: posizione del blocco più indice nel blocco (AD-13, M-35); tempi UTC RFC 3339; durate ISO 8601. (conv. C-1)
- **AR-42** C-2: ogni consegna ha `attemptId` (attività + numero tentativo); tentativo non corrente rifiutato e registrato; risultato/battito/scritture solo dalla sessione del worker destinatario; vale il primo esito per `attemptId`. (C-2)
- **AR-43** C-3: chiave scadenza = pianificazione + versione + istante UTC, controllata dal motore e da vincolo di unicità nella proiezione; scadenze rifiutate registrate con motivo. (C-3)
- **AR-44** C-4: chiave riga→avvio = pipeline + versione + fine finestra + chiave gruppo (senza finestra: posizione nello stream) = chiave di idempotenza FR-6. (C-4)
- **AR-45** C-5: pubblicazione con `If-Match` (ETag), rifiutata se superata; versione nodo compilato = SHA-256 del contenuto. (C-5)
- **AR-46** C-6: versione del protocollo worker dichiarata alla registrazione; 0.y.z stessa minor, dopo 1.0 stessa major; incompatibile = rifiuto con codice. (C-6)
- **AR-47** C-7: `application/problem+json` (RFC 9457) con `errors[]` (`code` stabile inglese a punti, `params`, `path` JSON Pointer, `line`/`column` facoltativi, percorso dei nomi dei task); un solo catalogo codici nel dominio (errori, motivi di attesa e di salto) esportato in TS via OpenAPI; stesso formato in motore e kit. (C-7)
- **AR-48** C-8: SSE, ripresa da `Last-Event-ID` = token opaco di posizione; origine/filtro/trasporto secondo AD-18 e AD-19. (C-8)
- **AR-49** C-9: OpenMetrics su `/metrics` (fuori da localhost segue C-14); log JSON su stdout, una riga per evento; mai segreti, token, chiavi, cookie, nomi visualizzati, input/output delle attività; stderr degli ausiliari non nei log (solo codice C-7 e id). (C-9, FR-52)
- **AR-50** C-10: oggetti chiusi salvo `additionalProperties: true`; `integer` ≠ `number` (`3` non è `3.0`); `null` ≠ assente, ammesso solo se il tipo lo include; predefiniti applicati dopo la validazione. (C-10)
- **AR-51** C-11: credenziale per worker, revocabile singolarmente, limitabile a funzioni e tenant, distinta da quelle di tenant e integratore. (C-11)
- **AR-52** C-12: cassaforte a busta (vedi AR-28), principale da file o variabile d'ambiente, generata solo a deposito vuoto e fuori dalle directory dati di topictoast e del bus; ruotarla cifra di nuovo le chiavi dei tenant senza conservare la vecchia busta (FR-34). (C-12)
- **AR-53** C-13: eliminazione tenant = cancellazione crittografica + pulizia asincrona, nell'ordine di AD-7; test sentinella FR-31 su ogni supporto. (C-13)
- **AR-54** C-14: TLS (rustls) su ogni connessione fuori da localhost, certificati dell'integratore o generati al primo avvio con impronta stampata; l'SDK verifica sempre il certificato, nessuna modalità senza autenticazione; nessuna chiave/credenziale predefinita: generate al primo avvio, mostrate una volta, mai su stdout né nei log; valori degli esempi rifiutati. (C-14)
- **AR-55** C-15: JWT firmato dall'host; solo `EdDSA`/`Ed25519` (RFC 9864); `iss` configurato e verificato; `exp` obbligatorio con durata massima configurabile (breve di default); nessuna revoca nella v1; campi `tenant`, `role`, `displayName` facoltativo, `selectedTenant`, `aud` = `kit`|`sdk` verificato per risorsa; il motore conosce solo la chiave pubblica. (C-15)
- **AR-56** C-16: JSON camelCase, percorsi `/v1/…`, risorse con UUIDv7, `position` nelle risposte e `Last-Event-ID`. (C-16)
- **AR-57** C-17: liste con paginazione a cursore (`cursor`, `limit`), filtri come parametri di query. (C-17)
- **AR-58** C-18: chiave di idempotenza FR-6 nell'header `Idempotency-Key`. (C-18)

#### Sicurezza
- **AR-59** E-1: id tenant interno UUIDv7, nome JWT tradotto, `default` riservato, test a due tenant (= AR-26). (review E-1)
- **AR-60** E-2: validare JWT solo `EdDSA`/`Ed25519`, `iss`, `aud` per risorsa, `exp` con tetto di durata; test di rifiuto per ogni claim (= AR-55). (E-2)
- **AR-61** E-3: busta versionata + HKDF per oggetto/segmento + AES-256-GCM, formato unico nel dominio; il costo si misura in S-1. (E-3)
- **AR-62** E-4: il proiettore scrive ogni payload nell'archivio all'applicazione, uscite a monte lette di lì; criterio di S-1 (= AR-24). (E-4)
- **AR-63** E-5: crate puro `crates/protocol` (tipi API e protocollo worker, sorgente OpenAPI) usato da motore e SDK Rust (= AR-36). (E-5)
- **AR-64** E-6: convenzioni API camelCase, `/v1/…`, cursore, `Idempotency-Key`, UUIDv7, `position` (= C-16…C-18). (E-6)
- **AR-65** E-7: costituzione IV, "client HTTP in un solo modulo per artefatto" (motore, SDK Rust), imposto da `clippy.toml` "once it arrives". (E-7; costituzione IV)
- **AR-66** E-8: costituzione V: con ogni DB un solo adattatore del repository con filtro tenant e test a due tenant; con Postgres RLS come seconda barriera (utente non superuser, senza `BYPASSRLS`); aperta la prova `FORCE ROW LEVEL SECURITY` (vedi Open Questions). (E-8)
- **AR-67** E-9: eccezione di sicurezza alla regola delle due settimane vale anche per CVE senza avviso RustSec/GHSA e per dipendenze transitive nei lockfile (= AR-6). (E-9)
- **AR-68** D-1: variante 1 di S-1 esclusa se manca anche una sola delle tre condizioni di sicurezza (AR-12). (D-1)
- **AR-69** D-2: Postgres 18 e 19, minime 18.7 e 19.0; immagine di test 18.7 fino al 2026-11-30, poi 19.x (19.0 stabile dal 2026-11-16). (D-2)

#### Dispiegamento, ambienti, operatività
- **AR-70** Produzione v1: un host Linux x86_64 o arm64 (NFR-10, [ASSUMPTION]); un binario per motore e ausiliari + bus; SQLite incorporato o Postgres dell'integratore su TLS; archivio payload su directory (o RustFS facoltativo); pool microVM solo con KVM; worker remoti su TLS; indirizzo d'ascolto e TLS fissati esplicitamente. HA del centro e cluster del bus fuori dalla v1 (PRD §8.2). (spine L668-710)
- **AR-71** Ambienti: sviluppo con un solo `docker compose` (AR-5); test con testcontainers (bus e DB reali) + wiremock; nessuna CI fino alla produzione. (spine L708-710)
- **AR-72** Aggiornamenti: migrazione automatica dello stato (FR-51); upgrade del bus senza perdite = criterio S-1; ogni cambio della spec OWS = migrazione (AD-4). (spine L712-713)
- **AR-73** Backup e ripristino coerenti di DB e bus: fuori dalla v1, limite documentato (NFR-6). (spine L714)
- **AR-74** Osservabilità e trasporto: C-9, C-14 (`/metrics` OpenMetrics, log JSON su stdout, endpoint di salute/prontezza ancora aperto). (spine L715)
- **AR-75** Formato di distribuzione, formato di configurazione, supervisione del riavvio: domande aperte (vedi sotto). (spine L716-717)
- **AR-76** Migrazione già consegnata per la prova non si modifica più: nuova migrazione (CLAUDE.md "Sprint pipeline" 5).

#### Processo e gate
- **AR-77** Ogni storia cita le AD-n che rispetta; passa da bmad (epiche) → spec-kit (`speckit-specify` → `clarify` → `plan` con Constitution Check e riga "macro candidate: yes/no" → `tasks`) → BDD rosso → implementazione con subagent Sonnet → `ponytail:ponytail-review` → gate → PR. Non usare `bmad-build`, `bmad-build-auto`, `bmad-spec`, `bmad-project-context`, `speckit-implement`. (CLAUDE.md)
- **AR-78** BDD rosso per primo: `features/<area>/<storia>.feature` in italiano (`# language: it`, `Funzionalità`/`Scenario`/`Dato`/`Quando`/`Allora`), visto fallire prima del codice; JSON confrontato esatto, tipi inclusi; dentro, TDD sul dominio; nessun merge con scenari o test rossi; bug = scenario che lo riproduce. (costituzione II; CLAUDE.md)
- **AR-79** Due livelli BDD (AD-14) + Playwright BDD per kit e dashboard; mock solo nei test automatici, le prove manuali dell'utente su servizi reali. (CLAUDE.md "BDD e test")
- **AR-80** Glossario prima: ogni nuovo concetto entra in `docs/glossary.md` prima dell'uso; identificatori in inglese, commenti e rustdoc in italiano; `//!` per ogni crate/modulo, `///` per ogni item pubblico, `missing_docs` on, `cargo doc` senza warning. (costituzione VIII)
- **AR-81** Pagina di docs utente in `docs/user/` nella stessa PR per ogni feature visibile (task "User docs" in `tasks.md`; passi eseguibili alla lettera, esempi con `example.com/.org/.net`, dati sensibili con `| mascherare: <cosa>`); contratto pubblico cambiato = commit con `!` + aggiornare `docs/user/`. (CLAUDE.md; costituzione)
- **AR-82** Purezza del dominio (`purity.rs`); `crates/domain` deve compilare per `wasm32-wasip1` (build nel gate quando esiste); tipi dei dati interpretati sempre tipizzati, niente `jsonb`/`TEXT JSON`; niente `unwrap()` su input esterno; mai loggare token, chiavi, cookie, segreti; nessun Python nel prodotto. (costituzione III, IV, V, VII)
- **AR-83** Ponytail livello `full` in sessione principale e in ogni subagent; scorciatoie con tetto marcate `ponytail:`; macro sempre candidata nel plan. Dashboard: skill `dashboard-react` (TDD Vitest, Storybook, vanilla-extract, pagina nuova prima in Penpot approvata, `npm run gate`); storie visive: screenshot a 1280/1440/1920 px e 390/360 vs prototipo approvato. (CLAUDE.md; costituzione VI)
- **AR-84** Git: un branch per lavoro (`<tipo>/<slug>` o `NNN-slug`), mai commit su `main`; Conventional Commits, mai attribuzione a Claude; fine lavoro PR → merge → `git switch main && git pull && git branch -d`, branch remoto mai cancellato; spec-kit sempre con `SPECIFY_FEATURE_DIRECTORY=specs/<NNN-slug>`; a merge aggiornare `_bmad-output/implementation-artifacts/sprint-status.yaml`; fine sprint: `bmad-sprint-planning`, `bmad-retrospective`, `ponytail:ponytail-debt`, tag semver. (CLAUDE.md)
- **AR-85** Prima di `speckit-tasks`: ogni ancora della spec (file, tipo, funzione) esiste nel codice; edge case sempre su valori estremi, predefiniti dopo validazione, ripetizioni, concorrenza; servizio esterno = spike sul servizio reale + stub derivato + smoke test prima della PR. (CLAUDE.md)

#### Domande aperte con scadenza
- **AR-86** M-02 (invocazione funzioni/nodi custom, segreti e connessioni nel formato, scrittura su stream con tenant da `attemptId`, rifiuto di risorse esterne): Update spine prima della prima storia che lo tocca. ⚠ un'altra riga ("Da valutare… sicurezza e processo") dice "la proposta su M-02 prima di S-1". (OQ L799, L823)
  → deciso il 2026-11-23: AD-20 (catalogo predefinito, prefisso `node.`, rifiuto senza rete, scrittura sugli stream dall'SDK); connessione e segreti dei task senza manifest alla spec delle connessioni (AR-89).
- **AR-87** M-04 (input delle pianificazioni nel formato): Update spine prima della prima storia delle pianificazioni. (OQ L800)
- **AR-88** Join dei dati, `$context` concorrente, sintassi fan-out dinamico in `dag` e politica d'errore vs AD-9 (se tocca AD-9 decide Marco): Update spine prima della prima storia `dag`. (OQ L801)
- **AR-89** M-37 (forma delle connessioni): spec della storia delle connessioni (FR-29, FR-33). (OQ L802)
- **AR-90** T-58 (canvas `@xyflow/react` 12.12.1 o 13.x dal 2026-11-30, `@dagrejs/dagre`, CodeMirror 6; elkjs escluso; tastiera/NVDA/telefono, max task NFR-14, CSP AD-19): Update spine o skill del kit prima della prima storia del kit. (OQ L803)
- **AR-91** Editor: posizioni dei nodi nella versione (FR-5) e nell'export, a cosa si legano, unità; moduli dei costrutti generati dal dominio o a mano con test di parità: prima della prima storia dell'editor. (OQ L804)
- **AR-92** Log dei task (tab Log UX): nota nel PRD che li porti nel perimetro, proprietario Marco, prima di S-1 (mai nel DB, NFR-4). (OQ L805)
  → deciso da Marco (2026-11-23): nel perimetro come FR-59, con le righe di worker e nodi visibili anche al tenant.
- **AR-93** Ambito della verità sul bus (quali risorse, altri controlli sullo stato recente, lettura di I/O di esecuzione viva, stato solo in memoria): S-1, costo dentro NFR-4. (OQ L806)
- **AR-94** Proiettore sotto RLS Postgres (gruppi multi-tenant senza `BYPASSRLS`, lettura operatore FR-46, `FORCE ROW LEVEL SECURITY`, tenant per transazione): S-1, dentro NFR-4. (OQ L807)
- **AR-95** Versione del formato di messaggi bus, payload, checkpoint, export riletti dopo upgrade (FR-51): S-1, con upgrade a esecuzioni in corso. (OQ L808)
- **AR-96** Primo avvio (chi firma `aud = sdk`, prima credenziale worker C-11, credenziale per caricare un nodo FR-15, riuso del nome di tenant eliminato, dove si scrivono le credenziali 0600 con impronta lenta, rinnovo certificati C-14): prima dello sprint 1. (OQ L809)
- **AR-97** Altri JSON dei tenant (letterali, input esecuzioni/pianificazioni, errori, righe stream, moduli compilati): proposta archivio payload + riferimento tipizzato + cifratura; prima della prima storia che li salva (FR-15, FR-24, FR-40, FR-57). (OQ L810)
- **AR-98** Fase operativa del motore come risorsa/evento, API aperta prima della rilettura, "Esegui" in ripresa (T-60): prima della prima storia che la usa. (OQ L811)
- **AR-99** Granularità eventi delle esecuzioni e battito del server: prima della prima storia del tempo reale (FR-41). (OQ L812)
- **AR-100** Chi serve i file della dashboard (proposta: il motore, stessa origine), sessione operatore, CSRF: prima della storia d'accesso (FR-45, M-44). (OQ L813)
- **AR-101** Salvataggio e pulizia dell'archivio payload insieme al DB: prima della storia che scrive il primo file. (OQ L814)
- **AR-102** Formato di configurazione (file, chiavi, variabili d'ambiente, validazione con predefiniti dopo C-10, massimi NFR-14, ricarica): prima di S-1. (OQ L815)
  → deciso il 2026-11-23: C-19 (ADR `2026-11-23-configuration-format`).
- **AR-103** Formato di distribuzione e chi riavvia il motore (NFR-15) con endpoint salute/prontezza: riavvio ed endpoint nell'ADR di S-2 con M-28; distribuzione nella spec della prima storia di installazione (FR-50, NFR-13). (OQ L816)
- **AR-104** Distribuzione librerie client (npm, crates.io) e piattaforme del validatore nativo per Node: prima della prima storia SDK o kit. (OQ L817)
- **AR-105** Livello BDD di SDK TS e CLI (proposta: worker di prova TS al livello contratto, CLI guidata da scenari cucumber-rs): prima della prima storia SDK TS o CLI. (OQ L818)
- **AR-106** Licenze ammesse (Apache-2.0; oggi escluso solo elkjs): Marco, prima della prima dipendenza dello sprint 1. (OQ L819)
  → deciso da Marco il 2026-11-23: elenco nello Stack della spine (ADR `2026-11-23-dependency-licenses`).
- **AR-107** Soglie di peso e avvio del kit: Marco, ADR di S-4. (OQ L820)
- **AR-108** Rilievi dei revisori, identità e chiavi (tipo proprietario e caso integratore nel test AD-5, "versione" in C-3, chiave C-4 e `idempotency.conflict`, escape RFC 6901, tempi al microsecondo, impronta `sha256:` del nodo, indirizzo payload HMAC, parole chiave AD-10): prima della prima storia che li tocca. (OQ L821)
- **AR-109** Rilievi, API e client (`If-Match` obbligatorio con 428 e `If-None-Match: *`, canonicalizzazione, predefiniti solo dal motore, payload a byte invariati con limite 1 MB, `cron` UTC, dipendenze di `packages/sdk`, autorizzazione per operazione con test): prima della prima storia che li tocca. (OQ L822)
- **AR-110** Rilievi, sicurezza e processo (rinnovo chiavi tenant e AAD, `npm ci --ignore-scripts`, `cargo audit` nel pre-push, `/metrics` fuori da localhost con credenziale operatore, definizione di "localhost", ambito tenant per metodo del repository, consegne con segreti a DB fermo, controllo citato per le Rule di AD-1, 3, 4, 8, 9, 10, 12, 15, 17, 19, campi di correlazione nei log): prima della prima storia che li tocca; M-02 prima di S-1. (OQ L823)
- **AR-111** Driver SQL e versione di SQLite (assente dallo Stack): prima di S-1. (OQ L824)
  → deciso il 2026-11-23: sqlx 0.9.1 con SQLite 3.54 (ADR `2026-11-23-sql-driver`).
- **AR-112** Dove vivono nello schema OpenAPI il catalogo codici C-7 e il tipo del contesto di validazione AD-11 (stanno nel dominio, `protocol` non dipende dal workspace): prima della prima storia dell'API. (OQ L825)

#### Rimandato
- **AR-113** M-06 forma del campo di estensione del manifest: spec storia manifest (FR-55). M-07 derivazione manifest da tipi TS/Rust: spec storie SDK (FR-37), con test di parità TS/Rust. (Deferred)
- **AR-114** M-08 manifest senza worker o conteso: spec registrazione (FR-12, FR-55). M-09 effetti di cambio manifest/schema di un sotto-workflow: spec versioni (FR-5, FR-15, FR-55). Compatibilità tra campi degli schemi: stessa spec versioni. (Deferred)
- **AR-115** M-10 controllo input/output a ogni attività: benchmark S-1 e storia FR-55. M-19 due orologi per attività: spec FR-8. M-23 quote e storico integratore: spec quote (FR-30, FR-46). (Deferred)
- **AR-116** M-29 interprete jq: Rust puro, compilabile `wasm32-wasip1`, elenco chiuso di funzioni (FR-17 vieta `$ENV`, `env`, `input_filename`, `import`, `include`), limiti di profondità: spec prima storia jq. M-30 filtro rete in uscita (un filtro per sandbox e `call http`, indirizzi dell'installazione esclusi, proxy env ignorati, resolver del motore, controllo di ogni indirizzo IPv4/IPv6/mappato a ogni connessione e redirect, microVM dallo stesso filtro): spec prima storia `call http` o sandbox (FR-16, FR-17). (Deferred)
- **AR-117** M-44 pagina d'accesso dashboard e limite tentativi: spec accesso (FR-45). M-45/M-46/M-47/T-63 contratto pubblico del kit (credenziale scaduta, contrasto/marchio, idempotenza lanci, codici/dizionario, cambio tenant, personalizzazione, nomi neutri, `peerDependencies` React, componenti solo client): sezione contratti del kit prima della prima storia del kit; nomi neutri e temi di `packages/ui` prima della prima storia che lo crea. (Deferred)
- **AR-118** Conteggio workflow che usano un segreto: spec segreti (FR-32). T-57 unità componenti (rem/em) e T-59 isolamento CSS/colori forzati: prima della prima storia di design system, kit o dashboard. T-65 prestazioni grafi grandi: quando si fissa il limite task (NFR-14). T-67 segreti nei log del codice tenant: spec consegna segreti e log (FR-33, FR-52); lato di prodotto chiuso da FR-59 (segnaposto del valore esatto, confermato da Marco il 2026-11-23). T-68 dashboard con canale interrotto: spec dashboard. (Deferred)
- **AR-119** Ricerca per valore tra elementi di fan-out (A-5 UX): spec storia fan-out. AMB-1, AMB-5, AMB-6, AMB-9: spec delle storie che le toccano. Versioni delle librerie non scelte (runtime async, server HTTP, driver SQL, JWT, gRPC/iroh, jq, generatori OpenAPI/TS, TypeScript, Vite): prima storia che le usa, regola delle due settimane (driver SQL prima di S-1). (Deferred)
- **AR-120** Dopo la v1 (PRD §8.2): HA del centro, cluster del bus, backup/ripristino coerenti, rotazione della chiave di firma, OAuth. Funzione di analisi dello storico + export Parquet (prova cifratura per tenant, `parquet_encryption` in datafusion 55.2.0): quando il PRD la definisce. (Deferred)

#### Mappa FR → AD
| FR o capability | AD / C / S | Nota |
| --- | --- | --- |
| Definizione dei workflow: FR-1…FR-5, FR-56 | AD-4, AD-8, AD-9, AD-10, AD-11, C-5 | `crates/ows` (modello), `crates/domain` (validatore); motore (versioni) |
| Esecuzione: FR-6…FR-11, FR-59 | AD-2, AD-3, AD-7, AD-9, AD-17, AD-18, C-2, C-3, C-4, C-9, S-1 | motore, bus, proiettore, archivio payload; S-1 per l'annullamento di FR-10 e il trasporto delle righe di log dei worker |
| Worker: FR-12…FR-14, FR-55 | AD-1, AD-10, C-6, C-11, S-1 | motore, adattatore collegamento worker, SDK |
| Nodi personalizzati e sandbox: FR-15…FR-17, FR-54 | AD-6, AD-16, C-5, S-2 | processi ausiliari, pool microVM |
| Pipeline stream: FR-18…FR-25 | AD-6, AD-7, AD-13, AD-17, C-4, S-3 | ausiliario per tenant con DataFusion, bus, archivio payload |
| Separazione tra tenant: FR-26…FR-31 | AD-5, AD-7, AD-13, AD-18, C-13, C-15 | adattatore repository, API, eventi in tempo reale |
| Cassaforte dei segreti: FR-32…FR-34 | AD-2, AD-6, AD-7, C-12 | motore (chiave), adattatore cassaforte |
| SDK: FR-35…FR-38 | AD-12, AD-14, AD-15, AD-16, C-16, C-17, C-18 | `crates/protocol`, `crates/sdk`, `crates/cli`, `packages/sdk` |
| Kit frontend: FR-39…FR-44, FR-57, FR-58 | AD-4, AD-5, AD-11, AD-12, AD-15, AD-18, AD-19, C-3, C-5, C-7, C-8, C-15, S-4 | `packages/ui`, `core`, `react`, `validator` |
| Dashboard: FR-45…FR-49 | AD-5, AD-12, AD-15, AD-18, C-7, C-8, C-14, skill `dashboard-react` | `dashboard/`, `packages/ui`, client API di `packages/core` |
| Installazione e funzionamento: FR-50…FR-53 | AD-1, AD-2, AD-16, AD-19, C-9, C-14 | motore, `crates/bench` |
| Requisiti trasversali: NFR-1…NFR-16, SM-2 | AD-1, AD-2, AD-6, S-1…S-4 | benchmark (FR-53), spike |
| Contenuto come testo, accessibilità, schermi: NFR-7, NFR-9, NFR-16 | AD-6 (input del motore), AD-15, skill `dashboard-react`, skill del kit | `packages/ui`, `packages/react`, `dashboard/` |
| Licenza: NFR-11 | ADR degli spike, Open Questions (licenze) | ogni dipendenza |

#### Dal brief: punti non ancora coperti

- **BR-1** Workflow Builder: competere o integrarsi (per esempio un suo adattatore verso
  topictoast al posto di Temporal) resta da decidere (brief.md:L180-L181). Il PRD lo cita solo come
  riferimento (prd.md:L55, L1034); non è né in §11 né nella spine. Proprietario: Marco. Tocca il
  perimetro del kit (FR-39…FR-44) solo se la scelta è «integrarsi».
  → deciso da Marco (2026-11-22): rimandato a dopo la v1; nessuna storia della v1 lo tocca.

#### Dove si chiudono le domande aperte del PRD (§11)

Dal 2026-11-23 il PRD riporta in §11 lo stato di ogni domanda, uguale a questa tabella, che resta
come indice.

| §11 | Stato dopo l'architettura | Dove si chiude |
| --- | --- | --- |
| n.1 caso reale di pipeline stream | aperta; S-3 usa UJ-3 come assunzione | Marco, prima o durante S-3 |
| n.2 obiettivo NFR-5 | chiusa il 2026-11-28: NFR-5 confermato a ≥ 100.000 eventi/s; stato della pipeline di UJ-3 0,6-1,3 MB | ADR di S-3 (AR-18) |
| n.3 conferma NFR-12 | aperta | ADR di S-2 (AR-15) |
| n.4 checkpoint | chiusa | AD-17 |
| n.5 progetto separato con i nodi pronti | fuori dalla v1 | nessuna storia |
| n.6 salvataggio di segreti e connessioni | busta e chiavi decise (AD-7, C-12) | forma delle connessioni: spec della storia delle connessioni (M-37, AR-89) |
| n.7 invocazione di funzioni e nodi nel formato | chiusa in parte: schema di input in `input.schema` (AD-10); chiamata dal catalogo predefinito e scrittura sugli stream (AD-20); connessione e segreti dei task senza manifest alla spec (M-37) | AD-20 (2026-11-23); resto: spec della storia delle connessioni, AR-89 | storia che la tocca (AR-86, AR-110: vale la scadenza più stretta) |
| n.8 propagazione nel DAG | chiusa in parte: regola in AD-9; aperti join dei dati, `$context`, sintassi del fan-out e politica «il join riceve l'errore» | AD-9; join e fan-out dentro `dag`: Update della spine prima della prima storia `dag` (AR-88) |
| n.9 riavvio a freddo degli ausiliari | aperta (M-28) | ADR di S-2 (AR-14) |
| n.10 manifest senza worker o conteso | assegnata (M-08) | spec della storia di registrazione (AR-114) |
| n.11 cambio di manifest o di schema | assegnata (M-09) | spec delle storie delle versioni (AR-114) |
| n.12 input della pianificazione nel formato | chiusa in parte: la pianificazione porta il proprio input (PRD); la forma nel documento aperta (M-04) | Update della spine prima della prima storia delle pianificazioni (AR-87) |
| n.13 rifiuto registrato con il motivo | chiusa | PRD (FR-6, FR-24), C-3, C-4 |
| n.14 scelta del segreto tra più candidati | assegnata (M-37) | spec della storia delle connessioni (AR-89) |
| n.15 attività oltre il limite di attesa | assegnata (M-19) | spec della storia di FR-8 (AR-115) |
| n.16 linguaggio degli schemi | chiusa | AD-10, C-10; compatibilità tra campi: spec delle storie delle versioni (AR-114) |
| n.17 scadenze rifiutate | assegnata | spec della storia delle pianificazioni |
| n.18 chi sospende la pianificazione condivisa | assegnata | spec della storia delle pianificazioni |
| n.19 quote e storico dell'integratore | assegnata (M-23) | spec della storia delle quote (AR-115) |

#### Contesto dal PRD

#### Superfici pubbliche e compatibilità (§6)

- Contratti pubblici (prd.md:L1377-L1381): API del motore; protocollo worker-motore; SDK TypeScript e Rust; CLI; formato dei nodi personalizzati compilati; formato del manifest di funzioni e nodi personalizzati (FR-55); nucleo JS; componenti React (proprietà ed eventi); formato dei workflow (sottoinsieme OWS + estensione DAG); dialetto SQL delle pipeline stream; formato di configurazione; formato della credenziale firmata dalla piattaforma ospite (FR-27); metriche.
- SDK di terzi (L1382-L1383): API e protocollo worker-motore documentati per permettere SDK in altri linguaggi, anche della comunità.
- Versioni (L1384-L1386): semver; fino al primo stabile (0.y.z) una rottura alza la minor, dopo solo la major; ogni rottura annunciata nelle note di rilascio con via di migrazione.
- Documenti salvati (L1387-L1388): un workflow accettato da una versione resta eseguibile dalle successive della stessa major.
- Documentazione (L1389): ogni superficie pubblica ha pagine per l'integratore con esempi eseguibili.

#### Percorsi utente (§2.3)

- **UJ-1** (prd.md:L143-L156): Marco (sviluppatore/sicurezza, oggi Hatchet su RabbitMQ 4.2.9) installa topictoast+bus con config predefinita e SQLite, scrive un worker TS (`dns_lookup`, `censys_search`), incorpora i componenti React, pubblica un workflow di esempio per un tenant di prova. Climax: lancia dalla sua app e vede le attività arrivare al worker e completarsi. Limite: worker non avviato = attività in coda con il motivo, partono alla registrazione senza errori. FR citati: nessuno.
- **UJ-2** (L158-L177; NFR-16): Giulia (analista SOC, tenant di Marco) compone nell'editor: elenco indicatori, DNS e Censys in parallelo, arricchimento, inserimento; collega credenziali Censys; usa nodo personalizzato TS caricato via CLI da un collega; lancia con 300 indicatori. Climax: DAG con 300 rami paralleli e join finale. Esito: salvato e rilanciabile, storico solo del suo tenant. Varianti: 1 indicatore = elenco di un elemento; da telefono (NFR-16). Limite: nodo in loop infinito, la sandbox lo ferma al timeout, task fallito con messaggio leggibile, altri tenant non influenzati.
- **UJ-3** (L179-L190) `[ASSUMPTION: esempio dedotto, da confermare con caso reale di Marco]`: Giulia crea una pipeline stream: query che conta indicatori per ASN su finestre di 5 minuti, tiene righe con > 50 indicatori, destinazione "avvia il workflow di arricchimento per ogni riga". Climax: durante un'ondata emette una riga per l'ASN sospetto e il workflow parte da solo, una volta per ASN. Limite: riavvio durante una finestra, riparte dall'ultimo checkpoint senza perdere il conteggio. FR citati: nessuno.
- **UJ-4** (L192-L206): Marco operatore apre la dashboard con la credenziale del primo avvio; vede worker con funzioni e attività, esecuzioni ferme in coda da più tempo, pipeline stream attive/rallentate/sospese con ritardo e causa, tenant vicini al limite. Climax: individua un worker che non risponde, attività già riassegnate. Limite: tenant oltre quota di attività concorrenti = al limite, nuove attività in coda, altri non rallentano. FR citati: nessuno.
- **UJ-5** (L208-L221; FR-50, FR-45, NFR-13, §4.6): Luca (non conosce topictoast) segue la guida rapida su macchina pulita, avvia con config predefinita (FR-50), worker e workflow di esempio, entra in dashboard con la credenziale del primo avvio, nel tenant predefinito. Climax: vede l'esecuzione di esempio "completata". Esito: < 15 minuti (NFR-13). Limite: credenziale smarrita, la pagina d'accesso dice dove trovarla (FR-45).

#### Perimetro v1

- Dentro (§8.1, prd.md:L1406-L1422):
  - Motore OWS (sottoinsieme FR-1) + estensione DAG: dipendenze arbitrarie, fan-out dinamico con join, retry, timeout, durabilità.
  - Worker come funzioni sempre attive, SDK TypeScript e Rust.
  - Nodi personalizzati dei tenant in TypeScript e Rust, già compilati, in sandbox leggera o, con KVM, forte con pool.
  - Pipeline stream SQL con stato, finestre, watermark, checkpoint, scritte da integratore e tenant, con quote.
  - Separazione tra tenant, quote, cassaforte con chiavi API.
  - Kit frontend: nucleo JS + componenti React (area workflow con elenco, editor workflow, pianificazioni, vista esecuzioni, segreti, editor pipeline stream), anche da telefono (NFR-16).
  - Dashboard dell'operatore, anche da telefono (NFR-16).
  - Installazione con due servizi più il database (SQLite o Postgres), guida rapida per il primo workflow.
  - Benchmark pubblico riproducibile.
- Fuori (§8.2, L1426-L1448): OAuth gestito dal motore (v1 = chiavi API); storage per payload grandi e file (valgono limite e tetto del join di FR-11; RustFS già nella v1 come archivio facoltativo di storico payload e checkpoint, AD-17, senza cambiare il limite di FR-11); backup/ripristino coerenti di DB e bus (NFR-6); rotazione chiave di firma identità (FR-27) `[ASSUMPTION]` (la rotazione chiave cassaforte è nella v1, FR-34); nodi personalizzati Python e Go; SDK Python e Go `[ASSUMPTION]`; connettori stream verso sistemi esterni (Kafka e simili); destinazioni di rete legate a una connessione (v1: per tenant, FR-16); costrutti OWS fuori sottoinsieme (gRPC, OpenAPI, AsyncAPI, A2A, `emit`, `listen`) e conformità CTK completa; alta disponibilità di motore e bus (worker già HA, FR-12/FR-14; motore riparte entro NFR-15; cluster bus dopo lo spike); nodi pronti, passi LLM, utenti stile n8n `[ASSUMPTION]` (progetto separato); proposta dell'estensione DAG a OWS (dopo prova in produzione).
- Non-obiettivi (§7, L1393-L1402): non alternativa a n8n per utenti diretti (niente catalogo nodi/connettori nella v1 `[ASSUMPTION]`); non piattaforma di analisi dati/data warehouse; stream non sono archivio eventi (conservati pochi minuti, FR-18); non gestisce utenti e permessi interni ai tenant; non richiede né sfrutta Kubernetes; nessuna edizione a pagamento né funzioni riservate; nessuna garanzia che workflow con estensione DAG girino su altri runtime OWS.
- Metriche (§9, L1454-L1483):
  - SM-1: ≥ 500 stelle GitHub a 6 mesi dal primo rilascio pubblico (Workflow Builder 448, Zigflow 242).
  - SM-2: benchmark pubblicato col primo rilascio, riproducibile con FR-53; carico di hatchet-dev/hatchet#5153 (~680 task/giorno): ≤ 5.000 istruzioni SQL/giorno (vs ~5,2 milioni misurati su Hatchet; ~7.700 per task per polling a basso volume); sotto carico NFR-3: ≤ 5 istruzioni SQL per attività (NFR-4).
  - SM-3: NFR-1, NFR-2, NFR-3, NFR-5 rispettati nel benchmark di ogni rilascio.
  - SM-4: UJ-1, UJ-2, UJ-3, UJ-4 eseguiti end-to-end prima del primo rilascio pubblico, sull'app di Marco.
  - SM-5: NFR-13 rispettato da ≥ 3 persone che non conoscono topictoast prima del primo rilascio `[ASSUMPTION]`.
  - SM-6: 3 persone profilo Giulia compongono, salvano e lanciano il workflow di UJ-2 (≥ 5 task, fan-out e join) senza documentazione, da sole, su desktop, entro 20 minuti; 3 su 3.
  - Contro-metriche: SM-C1 attività perse dopo crash = zero (NFR-6), niente meno SQL a costo di durabilità; SM-C2 non sotto 80 MB a riposo se peggiora il p99 sotto carico (NFR-2); SM-C3 niente nodi pronti/connettori per le stelle.
- Rischi con mitigazione da costruire (§10, L1487-L1508): sandbox (spike prima dello sprint 1: tecnologie dei due livelli, TS e Rust compilati nella leggera); stato pipeline stream (spike prima dello sprint 1; isolamento e quote per pipeline); bus = log, non coda (ack, retry, timeout, ritardi da costruire; attività lenta può bloccare una partizione, FR-13, FR-30; bus pre-1.0 cambia formato su disco, FR-51 aggiornamento senza perdita); forma estensione OWS (in `metadata` resta OWS valido, con chiavi nuove no; decide l'architettura); ampiezza v1 (nessun taglio; se uno spike fallisce slitta il rilascio); NFR-1…NFR-4 sotto ogni riferimento pubblico (il benchmark di base negli spike dice se reggono).

#### Assunzioni del PRD

Le domande aperte di §11 sono nella tabella «Dove si chiudono le domande aperte del PRD» qui sopra.

Assunzioni (§12, L1558-L1577):
- §1/§1.2/§2.2/§7/§8.2: nodi pronti, passi LLM e utenti stile n8n in progetto separato (nome di lavoro flowtoast).
- §2.3 UJ-3: esempio conteggio per ASN su finestre di 5 minuti.
- FR-1: `document.dsl` 1.1.x accettato.
- FR-11: limite 1 MB per input e output.
- FR-13: soglia di non interferenza = obiettivo NFR-2 (anche FR-17, FR-25, FR-30).
- FR-18: nessun connettore esterno per gli stream nella v1.
- FR-20: finestre fisse e scorrevoli nella v1, sessione dopo.
- FR-22: checkpoint ogni 10 secondi predefinito.
- FR-23: modifica della query = nuova versione con stato vuoto.
- FR-31: storico conservato 30 giorni predefinito.
- FR-50: processi ausiliari fermati se inattivi e riavviati a freddo; NFR-1 conta tutti i processi attivi.
- NFR-5: 100.000 eventi/s per pipeline.
- NFR-12: avvio ≤ 10 ms, ≤ 64 MB per istanza leggera.
- NFR-13: ≤ 15 minuti.
- NFR-10: Linux x86_64 e arm64.
- §8.2: SDK Python e Go dopo la v1.
- §8.2: rotazione chiave di firma identità dopo la v1.
- SM-5: tre persone esterne per misurare NFR-13.

#### Dall'addendum

- addendum.md:L18-L19: worker = funzioni serverless sempre attive, si registrano e ricevono task, nessun avvio a freddo per task.
- L20-L28: stream stile Arroyo (SQL continuo con stato, finestre, watermark, checkpoint); DataFusion da solo non ha finestre, watermark né checkpoint; tecnologia scelta dall'architettura con spike (riuso Arroyo, DataFusion + stato proprio, altro).
- L34-L39: meccanismi proposti (input per l'architettura, non decisioni): credenziale per worker revocabile; TLS con certificati generati al primo avvio; controllo dell'IP risolto a ogni connessione e redirect contro DNS rebinding; jq senza accesso ad ambiente e file o fuori dal processo della chiave; lista bianca di funzioni SQL; segreti risolti alla consegna e mai scritti sul bus; cancellazione crittografica per tenant; controllo ottimistico delle versioni.
- L43-L46: processi ausiliari (sandbox leggera, jq, SQL, `call http`) fermati dopo inattività e riavviati a freddo, così a riposo restano motore e bus (NFR-1); da studiare costo di avvio a freddo contro NFR-2 e NFR-12 e quali tenere caldi.
- L52-L56: iroh è solo trasporto, non log: senza log durevole scoperti NFR-6, FR-9, FR-18, FR-22, FR-51; eventi nel DB romperebbero NFR-4 e SM-2. Varianti da spike: (A) iroh motore-worker a stella, log resta; (B) iroh + log incorporato al posto di Iggy. P2P diretto tra worker per payload grandi dopo la v1.
- L58-L63: Valkey Streams con consumer group (`XACK`, `XPENDING`, `XAUTOCLAIM`), durabilità con AOF `appendfsync always`; limiti: tutto in RAM, servizio separato, costo fsync su NFR-2 e NFR-3 da misurare; confronto nello stesso spike con NATS JetStream e log incorporato in Rust.
- L64-L71: dati caldi in Valkey poi scaricati su Postgres/SQLite per capacità e conservazione, non durabilità (AOF con fsync protegge il lavoro accettato, NFR-6); scarico a gruppi e idempotente (chiave naturale per evento ed esecuzione), eliminazione da Valkey solo dopo il commit, letture dello storico su entrambi i livelli, tabelle tipizzate (costituzione V); il benchmark conta anche le istruzioni dello scarico e riporta righe e WAL per attività (NFR-4, SM-2).
- L72-L77: stima da misurare: coda attività Valkey pari o migliore (p99); stream ad alto volume meglio Iggy; con fsync a ogni scrittura entrambi limitati dal disco; domanda chiave dello spike: per quanto conservare gli eventi.
- L78-L80: decisione di Marco: eventi degli stream conservati pochi minuti (finestra + ritardo tollerato + checkpoint); Valkey candidato anche per gli stream; su Postgres/SQLite solo checkpoint e storico esecuzioni.
- L81-L86: Valkey resta privato dietro al motore, mai raggiunto dai worker (segreti alla consegna, quote, contropressione al motore); scelta vera = trasporto motore-worker: gRPC/HTTP/2 su TLS o iroh; raccomandazione v1: protocollo standard; iroh se worker o motore dietro NAT o presso i tenant, o per p2p su payload grandi.
- L93-L97: manifest delle funzioni tipizzate (FR-37, FR-55): l'SDK ricava gli schemi di input/output dai tipi della funzione (TS e Rust); credenziale firmata (FR-27) con tenant, ruolo, nome visualizzato facoltativo (+ tenant scelto nel kit per credenziale integratore); forma, campi e nome del tenant predefinito (§4.6) in architettura.
- L99-L102: scadenza credenziale nel kit (FR-27): evento verso la piattaforma ospite ≥ 60 s prima della scadenza e uno alla scadenza, lavoro non salvato tenuto in memoria; valori, nomi eventi, API del kit in architettura.
- L103-L104: controlli del kit (FR-44): coppie di contrasto con errore in sviluppo e avviso in console in produzione (vince la piattaforma ospite); marchio verificato con ricerca di "topictoast" nel bundle e proprietà di opt-in.
- L105-L110: `schedule` nel documento accettato (FR-1), trigger del proprietario; esecuzioni dell'integratore con i suoi segreti (FR-6, FR-26, FR-29); il tenant aggiunge dal kit la propria pianificazione con proprio input (FR-57); storage in architettura.
- L111-L113: workflow definito come codice (FR-35) in sola lettura nell'editor per tutti (decisione di Marco).
- L114-L115: stato attività in coda oltre il limite di attesa (FR-8): spec della storia con il valore del limite.
- L121-L124: linguaggio degli schemi (FR-55, FR-56, §11 n. 16): sottoinsieme chiuso di JSON Schema senza riferimenti esterni, o JSON Type Definition (RFC 8927); verificare se l'`input` di OWS 1.1 porta già uno schema e se ammette schema esterno (FR-55 lo esclude).
- L125-L126: controllo input/output a ogni attività (FR-55): dove gira e costo, misurato nelle corse NFR-2/NFR-3 (FR-53); se un output non conforme si ritenta: spec della storia.
- L127-L128: registro di scadenze e righe rifiutate (FR-6, FR-24) con chiave per pianificazione e scadenza che garantisca una sola esecuzione per scadenza: architettura.
- L129-L136: connessioni (FR-29, FR-33): quando si risolvono per un sotto-workflow dell'integratore, nomi di piattaforma, due connessioni della stessa piattaforma in un manifest: architettura; cambio di connessione di un segreto dopo il salvataggio e se l'ultimo uso indica workflow e task: spec della storia; con più segreti sulla stessa connessione (§11 n. 14) la scelta non deve mai essere implicita.
- L137: chiave di idempotenza dei lanci dal kit (FR-41): nome e API nel contratto del kit.
- L138-L139: credenziale integratore firmata per il kit (FR-28): come il motore la distingue da quella usata con l'SDK: architettura.

### UX Design Requirements

# Requisiti di design UX (UX-DR) di topictoast

Fonti: DESIGN.md (D) ed EXPERIENCE.md (E), datati 2026-11-20; letti insieme ad AD-15, AD-16, AD-19, T-57 e reconcile-ux.md. Percorso base: `_bmad-output/planning-artifacts/ux-designs/ux-topictoast-2026-11-19/`.
Nota: il design system oggi vive in `packages/ui` (AD-15), condiviso da kit e dashboard; D/E parlano ancora di `src/theme/tokens.ts` e di una skill `dashboard-react` valida per tutto. T-57 (rem o em) è aperta.

#### Token
- UX-DR-1 [entrambi] (D:L27-45,L114-122,L551-559) Neutri, chiaro/scuro: canvas #f6f6f6/#151517, canvas-dot #d5d4d9/#2a2a2d, surface #ffffff/#1d1c1e, border #e6e6e8/#2f2f32, text #1a191f/#e7e6ea, text-secondary #585661/#a5a3ae, chip #eae9ec/#2d2c30, surface-subtle #fafafa/#19181b, surface-error #f8e1df/#493131, border-control #8a8991/#706f77, port-border #8e8d95/#74737b, edge #8e8d95/#74737b, edge-active #5489c9/#86b7f3. Scuro = suffisso `-dark`; i componenti citano solo il nome chiaro. Testo solo `text` o `text-secondary`.
- UX-DR-2 [entrambi] (D:L46-56,L117-122,L561-566) Accenti: accent-kit #4a4854/#c9c7d1 (grafite, kit); accent-dashboard #705ab5/#ae9ce2 (dashboard); nav-active-dashboard #cec3ef/#3f3658; button-primary #302e38/#d9d8df con on-button-primary #fff/#18171c; selection-halo #d4d3d6/#3d3c40 e selection-halo-dashboard #dbd7e9/#373344. Nella dashboard accent-kit diventa accent-dashboard e nient'altro cambia. [DA ALLINEARE: i nomi `accent-dashboard` nel pacchetto condiviso devono restare neutri (FR-44, M-45..47)]
- UX-DR-3 [entrambi] (D:L62-80,L571-575) Categorie, tre famiglie (primo piano, fill, border; chiaro/scuro): worker-function #774bc3/#bb99f5, #ccb8ef/#433658, #ba9fea/#534171; custom-node #b03b85/#f48bcd, #efb8db/#57364c, #ea9fce/#704160; flow-control #2e8a8a/#7df2f2, #b8efef/#365758, #9feaea/#417071. Distanza di tinta minima da stati: 28°.
- UX-DR-4 [entrambi] (D:L81-112,L577-583) Stati (primo piano; fill; border; chiaro/scuro): completed #2e8a5f/#7df2bc, #b8efd5/#365749, #9feac7/#41705b; running #3b6fb0/#86b7f3, #b8d1ef/#364558, #9fc1ea/#415671; retried #8e782f/#f2d77d, #efe2b8/#574f37, #ead89f/#706542; failed #b4443c/#f4928b, #efbcb8/#573837, #eaa49f/#704442; neutral #82808e/#8b8994, #ececef/#2a292e, #dfdee3/#38373e (in attesa, in coda, ricevuta, saltata, annullata, scaduta, sospesa, vicino al limite; si distinguono per icona e parola). Su ogni `-fill` solo `text`.
- UX-DR-5 [entrambi] (D:L125-139,L618-648) Tipografia: `fonts.sans` system-ui stack, `fonts.mono` ui-monospace stack (nessun file di font, T-54). Token: label-xs 0.6875rem/500/1.45, label-sm 0.75/500/1.5, label-caps 0.75/500/1.45 +0.04em, body-sm 0.8125/400/1.45, body 0.875/400/1.45, body-strong 0.875/500/1.45, heading-sm 1/500/1.4, heading-md 1.0625/500/1.4, heading-lg 1.25/500/1.3, wordmark 1.0625/600/1.3 -0.01em, mono-sm 0.75/400/1.55, mono-md 0.8125/400/1.45, mono-strong 0.875/500/1.45. Pesi 400/500, 600 solo wordmark; funzioni e ID sempre mono, nomi del tenant sans; cifre tabulari; maiuscolo solo in label-caps. [DA ALLINEARE: scala in rem base 16 px, T-57 aperta; la regola contraddice un host con `html{font-size:62.5%}`]
- UX-DR-6 [entrambi] (D:L148-196,L650-698) Spaziatura base 4: 1=0.25, 2=0.5, 3=0.75, 4=1, 5=1.5, 6=2 rem; dot-grid 1rem; layout: rail-width 4, palette-width 15, side-panel-width 22.5, log-panel-height 12.5, workflow-bar-height 4, dashboard-bar-height 3.5, breakpoint-compact 48, breakpoint-compact-height 30 rem; controlli: control-height 2.25, target-min 2.75, nav-item-height 2, rail-item-height 2.75, handle 2x0.25 rem; nodo: min 8.75x4.5, max-width 17.5, category-tag-offset 1.75em, icon-tile 1.625, status-bar-height 0.625, gap 0.125, min-segment 0.375 rem; icone lg 1.25/md 0.875/sm 0.75 rem, icon-stroke 1.6; selection-width 0.09375, halo-width 0.25, focus-ring-width 0.09375, focus-ring-offset 0.125, indicator-width 0.125, indicator-width-table 0.1875, log-row-min-height 1.375 rem, log-time-column 13ch, log-level-column 9ch. [DA ALLINEARE: tutto in rem (T-57 aperta; `category-tag-offset` è già in em); i token vivono in `packages/ui`, non in `src/theme/tokens.ts`] ⚠ larghezza palette e max-width nodo provvisorie (A-36, A-30), da fissare in Penpot.
- UX-DR-7 [entrambi] (D:L140-147,L707-710) Rounded: xs 0.25, sm 0.375, md 0.5, lg 0.625, xl 0.75, 2xl 1 rem, full 9999px (solo `state-badge`). [DA ALLINEARE: unità rem, T-57]
- UX-DR-8 [entrambi] (D:L700-705) Elevazione: nessuna ombra su nodi, card, pannelli (A-33); profondità con tono e filo `border`; fan-out e task non partito su `surface-subtle`.
- UX-DR-9 [entrambi] (E:L551-553) Movimento: ⚠ nessun token di durata o easing in D/E; esiste solo la regola `prefers-reduced-motion` (zoom, adattamento, apertura fogli, cambio badge senza transizione; icona in esecuzione, righe del segmento e `skeleton` fermi; max 3 lampeggi/s).
- UX-DR-10 [entrambi] (D:L715-742) Icone: SVG in linea (CSP vieta risorse esterne), tratto 1.6, estremità arrotondate; mappa: graffe/`</>`/ramo per le categorie; spunta, arco d'orologio, freccia circolare, croce, cerchio tratteggiato, righe, cerchio barrato, pausa per gli stati; quote con icone proprie; il disegno si fa in Penpot.

#### Componenti
(I 44 componenti hanno chiave in D; tutti vivono in `packages/ui` o `packages/react` secondo AD-15: ⚠ la ripartizione per componente non è decisa.)
- UX-DR-11 [entrambi] canvas (D:L198-201,L763; E:L241) Fondo `canvas` con punti `canvas-dot` ogni `dot-grid`; trascinamento e zoom, tre `icon-button` «Ingrandisci», «Rimpicciolisci», «Adatta»; regione con nome («Grafo del workflow, 7 task»); sola lettura nelle esecuzioni; «Riordina» solo editor desktop.
- UX-DR-12 [entrambi] list-view (D:L202-210,L765-770; E:L242) Alternativa non visiva al grafo, sola lettura; riga: icon-tile, nome (`name`/`name-function`), state-badge, sotto `detail` (categoria, «dipende da», «precede», conteggi); elenco annidato in ordine topologico, figli del fan-out con `indent`, rami di switch con etichetta; `Invio` apre il panel; «Grafo» riporta il fuoco al nodo. [DA ALLINEARE: la dashboard la riusa nel dettaglio ma può dipendere solo da `packages/ui` e core (AD-16)]
- UX-DR-13 [entrambi] node-card (D:L211-225,L771-776; E:L243) Card surface/border/radius xl/padding 3, min 8.75x4.5 max 17.5 rem; riga 1 icon-tile e nome (label o label-function mono), riga 2 state-badge nelle esecuzioni e chip `TS`/`RS`; in attesa `surface-subtle`; errore: croce in `icon-error` + «Errore»/«N errori» con `border-error` solo rinforzo; selezionato accent-kit + halo; nome mai troncato, a capo; nome accessibile con categoria, stato, errori, predecessori, successori, conteggio.
- UX-DR-14 [entrambi] category-tag (D:L226-230,L777; E:L244) Sopra la card, senza fondo, label-xs, icona di categoria e parola («Funzione del worker», «Nodo personalizzato», «Controllo di flusso») letta prima del nome. [DA ALLINEARE: offset in em, T-57]
- UX-DR-15 [entrambi] icon-tile (D:L231-234,L779; E:L245) 1.625rem, radius md, fill/border/icona della categoria; `aria-hidden`; su node-card, palette-item, intestazione del fan-out.
- UX-DR-16 [entrambi] state-badge (D:L235-239,L780; E:L246) Pillola radius full, label-sm, fill/border/icona dello stato, testo `text`; icona e parola sempre; aggiornamento live senza regione live propria; non interattivo.
- UX-DR-17 [entrambi] chip (D:L240-245,L782; E:L247) Fondo `chip`, radius sm, label-xs; usi: `TS`/`RS`, «versione 3», «Dell'integratore», «da codice», provenienza nella palette, funzioni dei worker con icona di categoria; solo informazione.
- UX-DR-18 [entrambi] fanout-group (D:L246-252,L785-789; E:L248) Contenitore surface-subtle, radius 2xl, padding 4, titolo heading-sm; intestazione tessera + nome con conteggio («Per ogni indicatore · 300») + badge; status-bar e conteggi su due colonne; mai un ramo per elemento; «· 1» e «· 0» validi; conteggi e «altri N falliti» sono `button`.
- UX-DR-19 [entrambi] status-bar (D:L253-259,L790-792; E:L249) Altezza 0.625rem, gap 0.125, segmento minimo 0.375, radius xs; segmenti nel primo piano dello stato, running a righe diagonali con `segment-running-stripe`; `role="img"` con `aria-label` dei conteggi, cambio non annunciato.
- UX-DR-20 [entrambi] error-card (D:L260-266,L793-795; E:L250) Fondo `surface-error`, bordo `state-failed-border`, radius xl; etichetta «Elemento fallito», badge «Fallita», indicatore mono, «elemento 87 di 300», funzione, tentativi, messaggio; il clic apre tentativi/input/output; «altri N falliti» apre una `data-table` cercabile con filtro per stato e `role="status"` («12 elementi falliti su 300»), `aria-rowcount`/`aria-rowindex` se virtualizzata.
- UX-DR-21 [entrambi] edge (D:L267-276,L796-799; E:L251) Curva Bézier, stroke 1.25px `edge`; attivo 2px `edge-active`; tratteggio `5 4` per task non partiti e rami non presi; porte raggio 3.5px, fill surface, stroke `port-border`; `hit-area` 2.75rem; ciclo = errore con i task coinvolti e niente salvataggio; da uno switch un arco per ramo.
- UX-DR-22 [entrambi] condition-chip (D:L277-283,L800-802; E:L252) Etichetta del ramo a metà arco, surface/border/text, radius sm, label-xs, max-width 8.75rem, a capo senza puntini; sempre neutra anche «altrimenti»; statica e senza fuoco; resta sul ramo non preso.
- UX-DR-23 [entrambi] workflow-bar (D:L284-290,L803-807; E:L253) Altezza minima 4rem, titolo heading-md, chip di versione/origine, badge di stato, metadati («In esecuzione · Avviata alle 14:22 dal tenant · durata 1 min 48 s»), azioni a destra: «Salva» primary e «Esegui» secondary con play in `text`, mai verde (T-52); «Annulla esecuzione», «Elenco»/«Grafo»; nome del workflow è un `button` che apre il pannello; ospita `pause-notice`; sul telefono due righe.
- UX-DR-24 [entrambi] button-primary (D:L291-296,L808; E:L254) Fondo `button-primary`, radius lg, min-height 2.25rem, body-strong, icona sinistra facoltativa; al massimo uno per area; etichetta verbo; «Salva» resta attivo con errori.
- UX-DR-25 [entrambi] button-secondary (D:L297-303,L809; E:L255) surface/text/border, stesse misure del primario; azioni di contorno («Aggiorna», «Sostituisci», «Riprova»).
- UX-DR-26 [entrambi] icon-button (D:L304-310,L810; E:L256) `button-secondary` quadrato 2.25rem, icona 1.25rem, nome accessibile obbligatorio; X del panel, zoom, «Riordina», «Comprimi palette»/«Apri palette», testa del bottom-sheet.
- UX-DR-27 [entrambi] confirm-dialog (D:L311-317,L813-814; E:L257) Modale, radius 2xl, padding 5; titolo domanda, frase di conseguenza, `button-primary` con l'azione e `button-secondary` «Chiudi»; fuoco iniziale su «Chiudi», `Esc` chiude, fuoco torna al controllo; usi: elimina segreto/workflow, «Salva e attiva» query modificata, cambio schema di input, uscita con modifiche non salvate.
- UX-DR-28 [entrambi] panel (D:L318-327,L815-826; E:L258) Sinistro (palette, comprimibile a colonna 4rem), destro (22.5rem: chip categoria, nome, sottotitolo, X; segue la selezione, `Esc` chiude), basso (12.5rem, tab Log/Input/Output, solo vista esecuzioni, sotto il solo canvas); nel kit agganciato ai bordi senza margine né raggio con filo verso il canvas (T-51), nella dashboard riquadri radius 2xl; sul telefono diventa bottom-sheet; titoli sezione `label-caps`.
- UX-DR-29 [entrambi] tabs (D:L328-333,L827-828; E:L259) Attiva con linea 0.125rem `accent-kit`, body-sm, contesto mono a destra; schema WAI-ARIA (frecce cambiano tab, `Tab` entra nel contenuto).
- UX-DR-30 [entrambi] attempt-card (D:L334-339,L829-830; E:L260) surface-subtle, radius xl; una per tentativo in ordine di tempo: badge «Tentativo 1 · fallito», orario mono, versione del nodo, dettaglio; l'ultimo fallito conserva l'errore; ID worker solo dove consentito.
- UX-DR-31 [entrambi] code-block (D:L340-345,L831; E:L261) JSON in sola lettura, mono-sm, a capo automatico, scorre nel blocco; sempre testo, mai valori di segreti.
- UX-DR-32 [kit] code-editor (D:L346-354,L832; E:L262) Editor SQL, JSON, jq; bordo `border-control`, focus `accent-kit`, gutter `text-secondary`, riga errore `surface-error` + marcatore `state-failed`; modo `Tab` che sposta il fuoco («Esc, poi Tab per uscire»); `autocapitalize/autocorrect off`, `spellcheck=false`, virgolette dritte; libreria T-58. [DA ALLINEARE: E dice «valida con il nucleo JS prima del salvataggio» anche per l'SQL, ma AD-11 limita il validatore del browser al documento; l'SQL è validato solo dal motore con validazione a secco]
- UX-DR-33 [entrambi] log-line (D:L355-366,L834-836; E:L263) Griglia tre colonne (orario 13ch, livello 9ch, messaggio), mono-sm, altezza minima 1.375rem, orario al millesimo; livello parola neutra («info», «avviso», «errore», «ok») con icona colorata; `role="log"`.
- UX-DR-34 [kit] palette-item (D:L367-374,L837-840; E:L264) Voce a riquadro con bordo, radius lg, icon-tile + nome, descrizione del manifest a capo mai troncata; gruppi con `label-caps`; clic/`Invio`/trascinamento aggiungono; omonimi distinti dal chip di provenienza; solo funzioni, nodi personalizzati e segreti del tenant più costrutti FR-1.
- UX-DR-35 [entrambi] form-field (D:L375-384,L841-843; E:L265) Etichetta sopra sempre visibile, bordo `border-control`, min 2.25rem, radius lg; aiuto «predefinito: 30 s, massimo: 10 min» letto dall'installazione; errore con `border-error`, `validation-message`, `aria-invalid`, `aria-describedby`; segreto: «Valore salvato, non visibile» + «Sostituisci»; Accesso: `autocomplete="current-password"`, incolla consentito, `role="alert"`.
- UX-DR-36 [entrambi] select (D:L385-391,L844-845; E:L266) `form-field` con indicatore a destra; segreti di una connessione (solo nomi; vuoto: «Nessun segreto per Censys.»), filtri «Tenant»/«Workflow»/«Stato», «Ripeti», tenant-selector.
- UX-DR-37 [kit] data-mapping (D:L392-398,L846-850; E:L267) Sezione «Dati»: una riga per campo del manifest (nome, tipo mono, scelta «Valore»/«Da un task»/«jq»), fonte `task · campo`; join mostra cosa riceve; errori su campo mancante, tipo incompatibile, jq non valido o con `$ENV`, `env`, `input_filename`, `import`, `include`.
- UX-DR-38 [kit] schedule-section (D:L399-403,L851-854; E:L268) «Ripeti» (Nessuna/Ogni/Cron), espressione mono-md, «Prossima scadenza», badge neutro «Sospesa», «Sospendi»/«Riprendi»; scadenze perse/recuperate; scadenza non accettata col motivo; modificabile anche su workflow dell'integratore o da codice; pianificazione del documento OWS in sola lettura e distinta.
- UX-DR-39 [kit] input-form (D:L404-409,L855-858; E:L269) Modulo di «Esegui» nel pannello destro (foglio sul telefono), un form-field per campo dello schema di input; elenco incollato una riga per elemento con conteggio in `role="status"` («300 elementi») ed errore «Riga 12: atteso un numero»; «Mostra JSON»/«Mostra modulo»; senza schema si apre il JSON; «Esegui» bloccato con errori.
- UX-DR-40 [entrambi] validation-message (D:L410-416,L859-860; E:L270) Fondo `surface-error`, icona `state-failed`, body-sm, percorso mono-md, radius lg; nomina task e campo, suggerimento se noto; SQL con riga e colonna; ciclo con i task coinvolti.
- UX-DR-41 [entrambi] tooltip (D:L417-421,L861; E:L271,L556-557) Fondo `button-primary`, testo `on-button-primary`, radius md, body-sm; aggiunge al nome, `Esc` lo chiude, sorvolabile (WCAG 1.4.13).
- UX-DR-42 [kit] rail-item (D:L422-431,L862-864; E:L272) Barra icone larga 4rem, voce min 2.75rem con icona 1.25rem e etichetta sempre visibile (A-34); cinque voci Workflow, Editor, Esecuzioni, Segreti, Pipeline stream; attiva: fondo `canvas`, icona `accent-kit`, `aria-current="page"`.
- UX-DR-43 [kit] tenant-selector (D:L432-437,L865-867; E:L273) Fascia a tutta larghezza in cima all'area, filo inferiore, padding 2, etichetta «Tenant» + `select` con ricerca; solo ruolo integratore con più di un tenant; cambio con modifiche non salvate apre il confirm-dialog e scarta stato ed eventi del tenant precedente; assente nei pezzi singoli.
- UX-DR-44 [dashboard] nav-item (D:L438-446,L868-869; E:L274) Min 2rem, radius md, body; attiva: fondo `nav-active-dashboard`, testo `text`, linea 0.125rem `accent-dashboard`, `aria-current="page"`; `NavLink`, mai `<a href>`.
- UX-DR-45 [dashboard] dashboard-bar (D:L447-452,L870-871; E:L275) Min 3.5rem; wordmark «topictoast» a sinistra, nav-item al centro, operatore a destra; titolo di pagina heading-lg è l'intestazione principale.
- UX-DR-46 [dashboard] login-card (D:L453-458,L872-873; E:L276) Radius 2xl, padding 5, titolo heading-md; form-field per la credenziale, «Accedi», frase su dove è stampata la credenziale del primo avvio (testo dall'architettura, T-63).
- UX-DR-47 [entrambi] data-table (D:L459-468,L874-876; E:L277) Intestazione `surface-subtle` label-sm, celle body-sm, ID mono-md; prima cella link o `button` (niente clic sulla `tr`), riga corrente `aria-current`/`aria-selected` con linea 0.1875rem; intestazioni ordinabili `button` con `aria-sort`; sul telefono card impilate; filtri e colonne di Esecuzioni, Segreti, Quote (barra d'uso, «Vicino al limite»/«Al limite») e Pipeline stream della dashboard.
- UX-DR-48 [dashboard] history-list (D:L469-473,L877-878; E:L278) Cronologia del worker: orario mono-sm a sinistra, badge della fase, sotto funzione e indicatore; fasi ricevuta, in esecuzione, completata, fallita, ritentata, annullata; la più recente in alto.
- UX-DR-49 [dashboard] recovery-banner (D:L474-481,L879-881; E:L279) Riquadro a tutta larghezza sotto il titolo, famiglia retried, radius 2xl, in cima a ogni pagina; non chiudibile a mano; regione live annuncia solo inizio e fine, il contatore sta fuori; stesso riquadro per disco pieno; due avvisi impilati se coincidono.
- UX-DR-50 [kit] pause-notice (D:L482-488,L882-883; E:L280) Avviso in linea nella workflow-bar, famiglia neutra, radius lg, `role="status"`, scompare da solo; mai «motore» né «crash».
- UX-DR-51 [entrambi] load-error (D:L489-492,L884-885; E:L281) Messaggio con icona `state-failed` nell'area non caricata + `button-secondary` «Riprova»; la cornice resta; risorsa di altro tenant non è un errore di caricamento.
- UX-DR-52 [kit] bottom-sheet (D:L493-499,L886-889; E:L282) Solo telefono, surface, radius 2xl, filo superiore; testa con «Chiudi» ed «Espandi»/«Comprimi»; maniglia 2x0.25rem aggiuntiva; mezza o tutta altezza; fuoco al nodo alla chiusura; chiusura con «Chiudi», maniglia, `Esc`, gesto indietro solo se abilitato dall'ospite.
- UX-DR-53 [entrambi] empty-state (D:L500-502,L890; E:L283) Una frase centrata `text-secondary` che dice cosa fare più al massimo un `button-secondary`.
- UX-DR-54 [entrambi] skeleton (D:L503-505,L891; E:L284) Fill `chip`, radius md, forma del contenuto atteso, `aria-busy="true"` sull'area, «Caricamento di …» in `role="status"` fuori dall'area; fermo con reduced motion.
- UX-DR-55 [entrambi] (E:L585-602) Nessun componente definito solo da E: ⚠ la «regione live per area» e il link «Salta al canvas» non hanno chiave in D.

#### Superfici (IA)
- UX-DR-56 [kit] (E:L72-87) Area workflow: contenitore con rail-item, `tenant-selector` condizionale, tre aree (palette, canvas, pannello destro); ogni sezione esiste anche come pezzo singolo; nessuna intestazione propria, nessun `main`; nessuna superficie per caricare nodi personalizzati (solo SDK/CLI, FR-15); FR-58.
- UX-DR-57 [kit] (E:L82,L344) Workflow (elenco): data-table con chip «Dell'integratore»/«da codice»; vuoto «Nessun workflow salvato.» + «Nuovo workflow»; «Elimina» con confirm-dialog e rifiuto con l'elenco di chi lo usa. Nessun mock.
- UX-DR-58 [kit] (E:L84,L345,L420-437) Editor dei workflow: workflow-bar, palette, canvas/list-view, pannello del task («Dati», «Connessioni», retry+timeout, politica d'errore e parallelismo del fan-out, «Collegamenti», «Elimina task», schema di output) e del workflow («Input del workflow», «Pianificazione»); vuoto «Canvas vuoto: aggiungere un task dalla palette.».
- UX-DR-59 [kit] (E:L85,L346) Vista delle esecuzioni: elenco («Nessuna esecuzione negli ultimi N giorni.») e dettaglio con grafo/list-view, fanout-group, pannello destro (tentativi, input, output, versione del nodo) e pannello basso Log/Input/Output; sola lettura; «Annulla esecuzione».
- UX-DR-60 [kit] (E:L86,L347) Segreti: data-table (nome, connessione, data, ultimo uso, mai il valore), «Aggiungi segreto» (nome, connessione, valore); vuoto «Nessun segreto salvato.»; eliminazione con confirm-dialog.
- UX-DR-61 [kit] (E:L87,L348,L483-490) Pipeline stream: elenco, editor con code-editor SQL, «Ultime righe emesse» in data-table, pannello «Parametri» (stream del tenant, conservazione, destinazione «Stream del tenant»/«Workflow»), «Salva e attiva»/«Sospendi»; vuoto «Nessuna pipeline stream.» + «Nuova pipeline stream».
- UX-DR-62 [dashboard] (E:L98,L349) Accesso: login-card, prima di ogni dato; errori «Credenziale non valida.» e sessione scaduta.
- UX-DR-63 [dashboard] (E:L99,L352,L778) Worker (pagina iniziale): worker collegati, funzioni, attività in corso, history-list, «Non risponde da 2 min · 4 attività riassegnate», riga fissa «Sandbox forte non attiva…», avviso funzione registrata con connessioni diverse; vuoto «Nessun worker collegato.» + «Apri la guida rapida». Esplorazione `.working/direction-c-carta-chiara.html`; nessun mock approvato.
- UX-DR-64 [dashboard] (E:L100,L350) Esecuzioni: elenco con filtri «Tenant» (solo con più tenant), «Workflow», «Stato», filtro «in coda», colonna «In coda da»; dettaglio come la vista delle esecuzioni del kit senza valori dei segreti; «Annulla esecuzione» con conferma. [DA ALLINEARE: riuso di canvas e vista del kit, vedi AD-15/AD-16]
- UX-DR-65 [dashboard] (E:L101,L351,L277) Pipeline stream: stato attiva/rallentata/sospesa con causa; «Ritardo sul watermark», «Eventi al secondo», «Memoria di stato», «Eventi in ritardo scartati», «Ultimo checkpoint»; righe rifiutate con motivo.
- UX-DR-66 [dashboard] (E:L102,L353,L277) Quote: una colonna per quota di FR-30 con barra d'uso, stati «Vicino al limite»/«Al limite»; con il solo tenant predefinito niente nome né filtro; «Nessun tenant vicino al limite.».

#### Stati, interazioni, voce
- UX-DR-67 [entrambi] (E:L329-333) Caricamento = `skeleton`; errore di caricamento = `load-error`; risorsa di altro tenant o eliminata = «Workflow non trovato.» (e analoghi), mai «accesso negato».
- UX-DR-68 [entrambi] (E:L296-325) Elenco chiuso degli stati per oggetto (esecuzione, task, attività, worker, pipeline stream, tenant, pianificazione) con famiglia di colore; «in attesa» ≠ «in coda»; timeout è motivo, non stato; saltato spiega perché con link.
- UX-DR-69 [entrambi] (E:L606-618) Tempo reale: cambio di stato entro 1 s; fan-out aggiorna i conteggi aggregati; in pausa i valori restano fermi; una regione live per area, fallimenti raccolti ogni pochi secondi, stato finale subito, nessun annuncio al montaggio. [DA ALLINEARE: AD-18/C-8 fanno passare l'SSE da `fetch` con `Authorization`, chiuso alla scadenza della credenziale; il ricollegamento è visibile come `pause-notice`]
- UX-DR-70 [entrambi] (E:L616-618) Log: `role="log"` `aria-live="off"`, focalizzabile con nome, segue le righe nuove; «Pausa aggiornamenti»/«Riprendi» sempre visibile (WCAG 2.2.2).
- UX-DR-71 [kit] (E:L358-361) Quota raggiunta: task «In coda» con motivo; richiesta oltre arretrato/spazio rifiutata accanto a «Esegui».
- UX-DR-72 [kit] (E:L362-380,L466-481) Salvare ed eseguire: «Salva» valida e crea una versione (niente «pubblica»), con errori non salva e mostra riepilogo `role="alert"` («3 errori in 2 task») con link; `aria-busy` e «Salvataggio…»; errore di rete «Salvataggio non riuscito.» + «Riprova»; conflitto di versione con «Ricarica ultima versione» e «Scarica le modifiche»; «Esegui» apre subito la vista in «In coda».
- UX-DR-73 [kit] (E:L374-384) Workflow dell'integratore e definito come codice: editor in sola lettura per tutti, «Esegui» attivo, messaggio dedicato, pianificazione del tenant modificabile.
- UX-DR-74 [entrambi] (E:L385-389) Ripresa del motore: kit `pause-notice` «Aggiornamenti in pausa, riprendono da soli.»; dashboard `recovery-banner` esplicito con tempo trascorso; canale interrotto: stesso notice nel kit, T-68 per la dashboard.
- UX-DR-75 [entrambi] (E:L390-396) Sessione scaduta: kit avvisa l'ospite con evento almeno 60 s prima e alla scadenza, mantiene in memoria grafo/SQL/JSON (ripristino solo con stesso tenant e ruolo), `role="alert"`; dashboard torna ad Accesso senza dati.
- UX-DR-76 [kit] (E:L400-415) Canvas: trascinamento sul vuoto, rotella/pizzico, direzione adattiva (sinistra-destra su desktop, alto-basso sul telefono); «Riordina» disposizione automatica; posizioni desktop salvate col workflow (dove: architettura, T-46); telefono sempre automatico; più radici e foglie ammesse.
- UX-DR-77 [kit] (E:L417-437) Selezione e configurazione: un clic seleziona (contorno + halo) e apre il panel destro; senza selezione l'editor mostra il pannello del workflow; nelle esecuzioni tentativi, input, output.
- UX-DR-78 [kit] (E:L439-449) Collegare: trascinare da porta d'uscita a ingresso o clic su porta poi task; «Collega a…», «Rimuovi collegamento a <task>», «Elimina task»; annuncio «Collegamento da <task>: scegliere la destinazione», `Esc` annulla; bersaglio valido con anello tratteggiato + etichetta; ciclo segnalato con i task.
- UX-DR-79 [kit] (E:L451-464) Palette e costrutti: inserimento (desktop: dove lasciato, clic accanto al selezionato, su un arco tra due task; telefono dopo il selezionato); costrutti `switch`, `for`/`fork`, `wait`, `raise`, `set`, `call http`, sotto-workflow; `switch` con sezione «Rami» e «altrimenti».
- UX-DR-80 [kit] (E:L483-490) Pipeline stream: query nel code-editor con errori riga/colonna prima del salvataggio, «Salva e attiva» bloccato con errori, conferma su query attiva modificata («La modifica azzera lo stato della pipeline stream.»). [DA ALLINEARE: la validazione SQL prima del salvataggio passa da API a secco (AD-11), non dal nucleo JS]
- UX-DR-81 [kit] (E:L492-513) Tastiera del canvas: una sola fermata di `Tab`; frecce seguono gli archi, `Shift+frecce` fratelli; `Spazio` seleziona, `Invio` apre il pannello, `Esc` torna al nodo; `Canc` elimina, `C` collega, `A` palette, `Alt+frecce` sposta di un passo di griglia, `+`/`−` zoom, `0` adatta; attivi solo con fuoco nel canvas (A-17); meccanismo T-58.
- UX-DR-82 [kit] (E:L515-525) Telefono: un dito sposta, pizzico zoom, tocco seleziona; mai `user-scalable=no`; con `pointer: coarse` ogni bersaglio ≥ `target-min` (porte e archi con area di tocco più larga); verifica contro la distanza minima tra nodi a 360 px.
- UX-DR-83 [entrambi] (E:L116-126) Voce: impersonale, infinito per istruzioni, nome + participio per esiti; pulsanti verbo breve, «Chiudi»/«Indietro», mai «Annulla» da solo; errori dicono dove e perché con suggerimento; mai solo «Errore».
- UX-DR-84 [entrambi] (E:L121-123,L220-227) Glossario del PRD §3 sempre (workflow, task, esecuzione, attività, worker, funzione, nodo personalizzato, segreto, quota); «Pipeline stream» per intero; tabella di microcopy predefinita (E:L150-198) con Do/Don't.
- UX-DR-85 [entrambi] (E:L127-132) Errori come codice + parametri resi da un dizionario sostituibile (FR-44); kit senza «topictoast» nel bundle (stringhe, icone, file, titoli), verificato da una ricerca nel gate. [DA ALLINEARE: formato del dizionario T-63; i codici seguono C-7 (`code`, `params`, `path`, `line`, `column`)]
- UX-DR-86 [entrambi] (E:L133-145) Chi ha fatto cosa = ruolo («dal tenant», «dall'integratore») + nome visualizzato facoltativo; trigger («da pianificazione», «da pipeline stream»); ID worker nascosto al ruolo tenant, visibile all'integratore e in dashboard; contenuto del tenant sempre testo; numeri/date con `Intl`; accordo degli aggettivi di stato.
- UX-DR-87 [kit] (E:L585-602) Personalizzazione dell'ospite: colori, caratteri, testi, lingua; predefinito neutro accent-kit; verifica coppie di contrasto (errore visibile in sviluppo, avviso in console in produzione); testi mancanti ricadono sul predefinito; nessuna proprietà nasconde icone o parole di stato; tema scelto dall'ospite, altrimenti `prefers-color-scheme`; dashboard non personalizzabile. [DA ALLINEARE: meccanismo e nomi neutri di `packages/ui` (M-45..47, T-63, T-59)]
- UX-DR-88 [entrambi] (E:L55-57,L58-62) Temi chiaro e scuro, italiano e inglese; il contenitore del kit porta `lang`, il contenuto del tenant `lang` e `dir="auto"`; Storybook per ogni componente, nuove pagine disegnate prima in Penpot. [DA ALLINEARE: tokens letti da `src/theme/tokens.ts`, oggi `packages/ui`]

#### Accessibilità
- UX-DR-89 [entrambi] (E:L531-532) Livello WCAG 2.1 AA (NFR-9) per kit e dashboard.
- UX-DR-90 [entrambi] (D:L590-616) Contrasti: 4,5:1 per ogni testo, 3:1 per indicatori non testuali (border-control, port-border, archi, segmenti, icone); matrice completa come test generato dai token nel gate; nessuna deroga sotto 3:1 (T-44); coppie vietate: `text-secondary` su `-fill`.
- UX-DR-91 [entrambi] (E:L533-535) Tastiera: editor del grafo tutto da tastiera, ordine di `Tab` = ordine di lettura (WCAG 2.4.3), filtri e tabs inclusi.
- UX-DR-92 [entrambi] (E:L536-543) Alternativa al grafo: list-view in sola lettura; modifica da tastiera e azioni del pannello; canvas regione con nome, niente `role="application"` (T-58); prova con NVDA e VoiceOver include la costruzione del workflow di UJ-2.
- UX-DR-93 [kit] (E:L544-546) Kit incorporato: mai `main`; regioni e `nav` con `aria-label` («Sezioni workflow»); livello delle intestazioni come proprietà; link «Salta al canvas» dopo la barra icone (WCAG 2.4.1).
- UX-DR-94 [entrambi] (E:L547-548) Mai solo colore: ogni stato con icona e parola, `aria-label` sulle barre dense.
- UX-DR-95 [entrambi] (D:L757-761; E:L549-550) Focus visibile: anello doppio filo interno `surface`, esterno `accent-kit` (dashboard `accent-dashboard`), 0.09375rem a 0.125rem; distinto dal contorno di selezione. [DA ALLINEARE: T-59, il focus del kit non deve essere spento da `*:focus{outline:none}` dell'ospite né tagliato da `overflow:hidden`]
- UX-DR-96 [entrambi] (E:L551-553) Movimento ridotto (vedi UX-DR-9).
- UX-DR-97 [entrambi] (E:L554-558) Elementi nativi (`button`, `nav`, link del router), nome accessibile su ogni controllo, `aria-expanded`/`aria-controls` su pannelli e fogli; tooltip WCAG 1.4.13; contenuto del tenant mai HTML (NFR-7).
- UX-DR-98 [entrambi] (D:L679-688,L696-698) Bersagli: `target-min` 2.75rem con `pointer: coarse`, maniglia compresa; testo ridimensionabile (1.4.4, 1.4.10, 1.4.12): altezze minime non fisse, colonne log in `ch`, nodi che crescono col testo; prove a 320 px, 200 % e spaziatura 1.4.12. [DA ALLINEARE: T-57, la prova obbligatoria a radice 62,5 %]
- UX-DR-99 [entrambi] (D:L664; E:L551) Colori forzati: ⚠ non specificati in D/E; T-59 prescrive colori di sistema per contorni, archi, porte, bordi e `color-scheme` nel tema scuro.

#### Responsive e piattaforme
- UX-DR-100 [entrambi] (E:L562-564; D:L656-680) Desktop (board 1280, 1440, 1920): kit con barra icone, palette, canvas, pannello destro fisso, pannello basso nelle esecuzioni, grafo sinistra-destra; dashboard con barra in alto e pannelli affiancati (margini 5/6, spazio 5).
- UX-DR-101 [entrambi] (E:L565; D:L682-688) Telefono (board 390 e 360): canvas a tutta larghezza, grafo dall'alto in basso, bottom-sheet, workflow-bar su due righe.
- UX-DR-102 [entrambi] (E:L566) Soglia a `breakpoint-compact` 48rem (768 px); tra 768 e 1279 px layout desktop con pannello destro sopra il canvas (A-21). [DA ALLINEARE: media query in rem, T-57]
- UX-DR-103 [entrambi] (E:L567) Orizzontale: layout compatto anche con `(pointer: coarse) and (max-height: 30rem)`, il pannello basso diventa foglio; la direzione del grafo segue la larghezza.
- UX-DR-104 [entrambi] (E:L569-575) Telefono: campo col fuoco sopra la tastiera virtuale, foglio a tutta altezza mentre si scrive; rail-item e nav-item in una fila in fondo; code-editor SQL a tutto schermo; data-table come card impilate (A-22).
- UX-DR-105 [entrambi] (E:L577) Prove: 1280, 1440, 1920, 390, 360, più 844x390 e 740x360 in orizzontale; mock a 1440 (desktop) e 390 chiaro/scuro (telefono).
- UX-DR-106 [kit] (E:L577) ⚠ Browser supportati non dichiarati in D/E; il browser compare solo nello spike 4 (validatore wasm su Chromium, Firefox, Safari).

#### Flussi chiave
- UX-DR-107 [kit] (E:L658-681) UJ-1 Marco incorpora il kit: area workflow nella sua app con credenziale firmata, `tenant-selector` «Tenant di prova», workflow da codice (chip «da codice», sola lettura), «Esegui» con input-form; climax: vede le attività arrivare al worker e i badge «Completato»; limite: worker assente, «In coda: nessun worker collegato per `dns_lookup`» senza consumare tentativi.
- UX-DR-108 [kit] (E:L683-735) UJ-2 Giulia compone, lancia e segue: «Nuovo workflow», palette, fan-out, «Dati» da task, segreto Censys, «Salva», «Esegui» con 300 indicatori; climax: il DAG prende vita con conteggi 287/9/3/1; varianti 1 elemento, ciclo, telefono 390 px, eliminazione segreto; limite: nodo in loop fermato dal timeout.
- UX-DR-109 [kit] (E:L737-763) UJ-3 Giulia crea una pipeline stream: «Nuova pipeline stream», query con errori riga/colonna, destinazione «Workflow», «Salva e attiva»; climax: la riga per l'ASN sospetto avvia il workflow; limite: riavvio durante una finestra, notice di pausa e ripartenza dal checkpoint. ⚠ Esempio dedotto dal PRD (A-25).
- UX-DR-110 [dashboard] (E:L765-784) UJ-4 Marco controlla l'installazione: accesso, Worker, Esecuzioni per «In coda da», Pipeline stream, Quote; climax: individua `worker-ti-03` «Non risponde da 2 min · 4 attività riassegnate»; limite: tenant «Al limite», le altre quote non cambiano.
- UX-DR-111 [dashboard] (E:L786-800) UJ-5 Luca prova topictoast: guida rapida, configurazione predefinita, workflow di esempio (FR-50); esito entro 15 minuti (NFR-13) visto in Esecuzioni come «Completata»; `login-card` dice dove sta la credenziale, i vuoti offrono «Apri la guida rapida».

#### Allineamento con l'architettura
- UX-DR-2: `accent-dashboard`, `nav-active-dashboard`, `selection-halo-dashboard` vivono ora nel pacchetto condiviso `packages/ui`, che per M-45..47 deve avere nomi neutri (FR-44).
- UX-DR-5, UX-DR-6, UX-DR-7, UX-DR-14, UX-DR-98, UX-DR-102: T-57 aperta (rem o em); la scala in rem base 16 px della skill `dashboard-react` e di D è da rivedere prima della prima storia del design system; `category-tag-offset` è già in em.
- UX-DR-6, UX-DR-88: i token non si leggono più da `src/theme/tokens.ts` ma dal pacchetto `packages/ui` (AD-15); anche Storybook e componenti presentazionali stanno lì.
- UX-DR-12, UX-DR-64: la dashboard può dipendere solo da `packages/ui` e dal client API di `packages/core` (AD-16): il riuso di list-view, canvas e vista delle esecuzioni del kit richiede che stiano in `packages/ui` o che il dettaglio sia ricostruito; ⚠ ripartizione ui/react non decisa.
- UX-DR-32, UX-DR-80: AD-11 limita il validatore del browser al documento (formato, dag, schemi, jq, cron); l'SQL è validato solo dal motore con validazione a secco, non dal nucleo JS (E:L262 e L491 vanno riletti).
- UX-DR-69, UX-DR-74: tempo reale = SSE via `fetch` con `Authorization`, chiusura a `exp` e ricollegamento (AD-18, C-8, AD-19); il caso «canale interrotto» e il cambio tenant devono combaciare con questo comportamento.
- UX-DR-85, UX-DR-87: formato del dizionario, personalizzazione e marchio (T-63, M-45..47); i codici di errore seguono C-7.
- UX-DR-95: T-59 (isolamento CSS, colori forzati, `z-index` dei fogli) aperta, vale per i componenti condivisi di `packages/ui`.
- UX-DR-106: AD-19 impone alla pagina ospite `connect-src` verso il motore e `'wasm-unsafe-eval'`; la dashboard non istanzia il validatore (AD-15) e mantiene la CSP `default-src 'self'`.

Chiavi components in DESIGN.md: canvas, list-view, node-card, category-tag, icon-tile, state-badge, chip, fanout-group, status-bar, error-card, edge, condition-chip, workflow-bar, button-primary, button-secondary, icon-button, confirm-dialog, panel, tabs, attempt-card, code-block, code-editor, log-line, palette-item, form-field, select, data-mapping, schedule-section, input-form, validation-message, tooltip, rail-item, tenant-selector, nav-item, dashboard-bar, login-card, data-table, history-list, recovery-banner, pause-notice, load-error, bottom-sheet, empty-state, skeleton

### FR Coverage Map

Un FR che serve a più epiche ha una riga per fetta; nessuna fetta ha bisogno di un'epica successiva.

FR-1: Epic 1 - nucleo del sottoinsieme (`do`, `set`, invocazione di funzione, `document.dsl`, rifiuto dei costrutti fuori sottoinsieme)
FR-1: Epic 2 - espressioni jq, `try` con `catch.retry`
FR-1: Epic 7 - `switch`, `fork`, `for`, `try` con `catch`, `wait`, `raise`, `call http`, sotto-workflow
FR-1: Epic 9 - `schedule` del documento
FR-1: Epic 10 - invocazione di nodi personalizzati
FR-2: Epic 2 - dipendenze nel task `dag`, cicli rifiutati, propagazione di AD-9
FR-3: Epic 2 - fan-out dinamico con join
FR-4: Epic 1 - validazione all'invio del nucleo (schema, riferimenti, input sul manifest)
FR-4: Epic 2 - espressioni jq non valide o vietate
FR-4: Epic 4 - funzioni rese disponibili ai tenant
FR-4: Epic 7 - costrutti del resto del sottoinsieme
FR-4: Epic 8 - connessioni senza segreto e segreti che non soddisfano
FR-5: Epic 2 - versioni nel motore e pubblicazione con `If-Match`
FR-5: Epic 12 - salvare = pubblicare nell'editor, avvisi sul cambio di schema
FR-6: Epic 1 - avvio da API e SDK, origine registrata, idempotenza
FR-6: Epic 9 - pianificazioni del documento e del tenant
FR-6: Epic 11 - avvii dalle righe delle pipeline stream
FR-7: Epic 2 - retry
FR-8: Epic 2 - timeout e limite di attesa
FR-9: Epic 1 - durabilità
FR-10: Epic 2 - annullamento
FR-11: Epic 1 - limite per input e output di un'attività
FR-11: Epic 2 - tetto dell'input aggregato del join
FR-12: Epic 1 - registrazione dei worker e consegna
FR-12: Epic 4 - credenziali dei worker limitate e revocabili
FR-13: Epic 2 - concorrenza e contropressione
FR-14: Epic 2 - worker che non risponde
FR-15: Epic 10 - caricamento di un nodo da SDK e CLI
FR-15: Epic 12 - nodi del tenant nell'editor
FR-16: Epic 7 - filtro di rete di `call http`
FR-16: Epic 10 - isolamento della sandbox
FR-17: Epic 2 - limiti di jq
FR-17: Epic 10 - limiti dei nodi personalizzati
FR-17: Epic 11 - limiti delle query delle pipeline stream
FR-18: Epic 11 - stream e ingresso degli eventi
FR-19: Epic 11 - definizione in SQL
FR-20: Epic 11 - finestre e aggregazioni
FR-21: Epic 11 - watermark ed eventi in ritardo
FR-22: Epic 11 - stato e checkpoint
FR-23: Epic 11 - ciclo di vita
FR-24: Epic 11 - destinazioni
FR-25: Epic 11 - isolamento e quote delle pipeline stream
FR-26: Epic 1 - filtro del tenant nel repository con test a due tenant, tenant predefinito
FR-26: Epic 4 - visibilità completa tra tenant, workflow e trigger
FR-26: Epic 11 - scrittura sugli stream solo del proprietario
FR-27: Epic 4 - verifica del JWT nel motore
FR-27: Epic 6 - scadenza, rinnovo e cambio di tenant nel kit
FR-28: Epic 4 - due ruoli e permessi nel motore
FR-28: Epic 6 - kit con credenziale dell'integratore
FR-29: Epic 4 - workflow dell'integratore eseguiti per un tenant
FR-29: Epic 8 - connessioni risolte all'avvio
FR-30: Epic 4 - quote su attività, esecuzioni, API, spazio e arretrato
FR-30: Epic 10 - quote della sandbox
FR-30: Epic 11 - quote di pipeline ed eventi
FR-31: Epic 4 - conservazione dello storico ed eliminazione del tenant
FR-31: Epic 10 - nodi e sandbox nella prova sentinella
FR-31: Epic 11 - checkpoint nella prova sentinella
FR-32: Epic 8 - salvataggio di un segreto
FR-32: Epic 12 - selettore dei segreti nell'editor
FR-33: Epic 8 - consegna al momento dell'esecuzione
FR-34: Epic 8 - protezione a riposo e rotazione della chiave
FR-35: Epic 5 - workflow come codice in TypeScript e Rust
FR-36: Epic 1 - documenti OWS a runtime
FR-37: Epic 1 - worker e funzioni con l'SDK Rust
FR-37: Epic 5 - worker e funzioni con l'SDK TypeScript
FR-38: Epic 4 - tenant e quote via SDK
FR-38: Epic 5 - parità dell'SDK TypeScript
FR-38: Epic 8 - segreti via SDK
FR-38: Epic 11 - pipeline stream via SDK
FR-39: Epic 6 - nucleo JS
FR-40: Epic 12 - editor dei workflow
FR-41: Epic 6 - vista delle esecuzioni e lancio
FR-42: Epic 12 - gestione dei segreti
FR-43: Epic 12 - editor delle pipeline stream
FR-44: Epic 6 - aspetto personalizzabile
FR-45: Epic 3 - accesso alla dashboard
FR-46: Epic 3 - esecuzioni nella dashboard
FR-47: Epic 11 - pipeline stream nella dashboard
FR-48: Epic 3 - worker nella dashboard
FR-49: Epic 4 - quote nella dashboard
FR-50: Epic 1 - installazione minima, tenant predefinito, credenziali generate, solo localhost, SQLite o Postgres
FR-50: Epic 10 - processi ausiliari fermati per inattività e riavviati a freddo
FR-51: Epic 13 - aggiornamenti senza perdite
FR-52: Epic 1 - log JSON e `/metrics` di base (C-9)
FR-52: Epic 13 - metriche complete
FR-53: Epic 1 - benchmark di base nel workspace
FR-53: Epic 13 - benchmark pubblico riproducibile
FR-54: Epic 10 - sandbox forte opzionale
FR-55: Epic 1 - manifest delle funzioni e controllo di input e output
FR-55: Epic 10 - manifest dei nodi personalizzati
FR-56: Epic 1 - schema di input del workflow
FR-57: Epic 9 - pianificazione del tenant via API e SDK
FR-57: Epic 12 - pianificazioni nel kit
FR-58: Epic 2 - eliminazione di un workflow nel motore
FR-58: Epic 6 - area workflow ed elenco dei workflow
FR-59: Epic 2 - eventi del motore letti via API (storia 2.11), righe del worker Rust con il tetto (storia 2.12)
FR-59: Epic 3 - tab Log nel dettaglio della dashboard
FR-59: Epic 4 - log nella conservazione e nell'eliminazione dei tenant
FR-59: Epic 5 - righe dal worker TypeScript
FR-59: Epic 6 - tab Log nel kit
FR-59: Epic 8 - segreti sostituiti nelle righe di log
FR-59: Epic 10 - righe dei nodi personalizzati
FR-59: Epic 13 - log fino al tetto nella corsa a carico misto

NFR: NFR-1…NFR-5 nel benchmark dell'Epic 13 (soglie verificate prima negli spike); NFR-6 ed NFR-15 dall'Epic 1, con i guasti iniettati dell'Epic 13; NFR-7 dall'Epic 1 (input) e nelle epiche con interfaccia (3, 6, 12); NFR-8 nei casi di ogni epica e nella suite completa dell'Epic 13; NFR-9 ed NFR-16 nelle epiche 3, 6, 12, con la prova NVDA e VoiceOver nell'Epic 13; NFR-10 ed NFR-12 nell'Epic 10; NFR-11 su ogni dipendenza; NFR-13 nell'Epic 13; NFR-14 nella spec di ogni storia.

## Epic List

Le epiche sono numerate nell'ordine di realizzazione. Ordine deciso da Marco il 2026-11-22: SDK TypeScript e kit subito dopo tenant e identità, così UJ-1 completo arriva prima.

Regola: le voci «Bloccata da» di un'epica si chiudono nello sprint precedente al suo inizio, con la PR di Update della spine o della skill che le decide; `bmad-sprint-planning` le mette tra i lavori di quello sprint.

### Prima dello sprint 1 (fase di architettura, non è un'epica)

Lavoro che chiude l'architettura: non passa da spec-kit né dal BDD. Ogni spike si chiude con un ADR
misurato e una PR di Update della spine (AR-8); dopo ogni Update si ricontrollano le epiche indicate.
Lo sprint 1 parte solo quando tutte le voci sono chiuse.

| Voce | Cosa | Chi | Epiche da ricontrollare |
| --- | --- | --- | --- |
| S-1 Bus (AR-9…AR-13) | Iggy o Valkey; tre collegamenti dei worker; ripresa e aggiornamento; trasporto delle righe di log dei worker (FR-59); chiusa il 2026-11-26 (ADR del bus), AR-93, AR-94 e AR-95 restano aperte | architettura | 1, 2, 3, 5, 6, 10, 11, 13 |
| S-2 Sandbox (AR-14…AR-16) | livello leggero e microVM; riavvio a freddo; chiusa il 2026-11-26 (ADR della sandbox), AR-103 resta aperta (prima storia di installazione) | architettura | 1, 10, 13 |
| S-3 Pipeline stream (AR-17…AR-19) | DataFusion con stato, finestre, watermark, checkpoint sul caso UJ-3; chiusa il 2026-11-28 (ADR delle pipeline stream): validazione a secco del SQL in un ruolo ausiliario proprio, il controllo sull'albero porta la lista delle query ostili come test di regressione, il budget di CPU va riprovato nella storia dell'isolamento | architettura | 1, 10, 11, 13 |
| S-4 Kit nel browser (AR-20, AR-21) | validatore `wasm32-wasip1` su Chromium, Firefox, Safari; soglie di peso e avvio (AR-107) | architettura, Marco | 6, 12 |
| AR-86 | chiusa il 2026-11-23: AD-20; resto alla spec delle connessioni (AR-89) | architettura | 1, 7, 8, 10 |
| AR-96 | primo avvio: credenziali, certificati, chi firma `aud = sdk` | architettura | 1, 3, 4 |
| AR-102 | chiusa il 2026-11-23: C-19 | architettura | 1 |
| AR-111 | chiusa il 2026-11-23: sqlx 0.9.1, SQLite 3.54 | architettura | 1 |
| AR-92 | chiusa il 2026-11-23: log dei task nel perimetro (FR-59) | Marco | 2, 3, 4, 5, 6, 8, 10, 13 |
| AR-106 | chiusa il 2026-11-23: elenco nello Stack della spine | Marco | tutte |

### Epic 1: Primo workflow eseguito

L'integratore installa topictoast e il bus con SQLite e la configurazione predefinita, pubblica via
API un documento OWS che chiama in sequenza le funzioni di un worker Rust, avvia l'esecuzione e ne
legge l'esito; un crash del motore a metà esecuzione non perde nulla.

**FRs covered:** FR-1 (nucleo), FR-4 (nucleo), FR-6 (avvio da API e SDK), FR-9, FR-11 (limite per attività), FR-12, FR-26 (fondamenta), FR-36, FR-37 (Rust), FR-50, FR-52 (base), FR-53 (benchmark di base), FR-55, FR-56
**NFR:** NFR-6, NFR-7, NFR-8 (TLS, credenziale di worker), NFR-14, NFR-15
**Invarianti dal primo giorno:** AD-2 verità sul bus; AD-3, AD-7, AD-17 cifratura per tenant e archivio dei payload con due spazi; AD-5 filtro del tenant con test a due tenant; AD-6 contenuto dei tenant solo nei processi ausiliari; C-7, C-9, C-11, C-14, C-16…C-18.
**Bloccata da:** S-1; AR-97, AR-98, AR-101, AR-103 (distribuzione), AR-104, AR-112; le parti di AR-108, AR-109, AR-110 che tocca.
**Note:** la prima storia è lo scaffold greenfield, senza template di partenza (AR-1…AR-6): workspace Cargo e npm, `dashboard/`, gate, hook, `docker compose`. Prova di UJ-1 lato motore.

### Epic 2: DAG, fan-out e controllo delle esecuzioni

L'integratore esegue workflow con task `dag` e dipendenze arbitrarie, un fan-out dinamico con join
su centinaia di elementi, retry, timeout e annullamento; pubblica versioni nuove mentre le vecchie
girano; un worker lento o muto non blocca gli altri.

**FRs covered:** FR-1 (espressioni jq, `try` con retry), FR-2, FR-3, FR-4 (jq), FR-5 (motore), FR-7, FR-8, FR-10, FR-11 (tetto del join), FR-13, FR-14, FR-17 (limiti di jq), FR-58 (eliminazione nel motore), FR-59 (eventi del motore e righe del worker Rust)
**NFR:** NFR-2 (non interferenza), NFR-14
**Bloccata da:** AR-88 (Update della spine su join, fan-out e forma del retry dentro `dag`, prima della prima storia `dag`); M-29 interprete jq (AR-116); AR-109 (`If-Match`); M-09 e M-19 nelle spec (AR-114, AR-115).; trasporto delle righe di log dei worker deciso da S-1 (storia 2.12).
**Note:** il cuore di UJ-2 senza interfaccia: 300 rami paralleli e un join.

### Epic 3: Dashboard dell'operatore

L'operatore entra con la credenziale del primo avvio e vede le esecuzioni (filtri, tempo in coda,
dettaglio dei task, annullamento) e i worker (funzioni, attività in corso, chi non risponde,
cronologia), da desktop e da telefono.

**FRs covered:** FR-45, FR-46, FR-48, FR-59 (tab Log)
**NFR:** NFR-7, NFR-9, NFR-16
**UX-DR:** token, shell della dashboard e componenti che queste pagine usano per prime; UJ-4 (worker ed esecuzioni) e UJ-5 (accesso).
**Bloccata da:** AR-7 (Update di `bmad-ux` sulle 16 voci DA ALLINEARE); T-57 e T-59 (AR-118); divisione dei componenti tra `packages/ui` e `packages/react`, perché il dettaglio dell'esecuzione è condiviso con il kit (AD-16); AR-90 (T-58, canvas); AR-99 (eventi in tempo reale); AR-100 (chi serve la dashboard, sessione, CSRF); AR-117 (pagina d'accesso, nomi neutri di `packages/ui`); pagine disegnate prima in Penpot.
**Note:** prima superficie con interfaccia: paga le decisioni comuni con il kit. Da qui ogni storia del motore si prova anche guardandola. Le pagine delle quote (FR-49) e delle pipeline stream (FR-47) arrivano con le epiche 4 e 11.

### Epic 4: Tenant, identità e quote

L'integratore crea tenant e quote dall'SDK; ogni richiesta porta un JWT firmato dalla piattaforma
ospite con tenant e ruolo; i tenant usano i workflow dell'integratore nel proprio contesto; un
tenant oltre quota non rallenta gli altri; lo storico scade e un tenant eliminato diventa
illeggibile ovunque.

**FRs covered:** FR-4 (funzioni rese disponibili ai tenant), FR-12 (credenziali dei worker), FR-26 (completo), FR-27 (motore), FR-28 (motore), FR-29 (senza connessioni), FR-30 (attività, esecuzioni, API, spazio, arretrato), FR-31, FR-38 (tenant e quote), FR-49, FR-59 (conservazione ed eliminazione)
**NFR:** NFR-8 (test tra tenant e tra ruoli), NFR-14
**Bloccata da:** M-23 nella spec (AR-115).
**Note:** UJ-4, parte delle quote.

### Epic 5: SDK TypeScript, workflow come codice e CLI

L'integratore scrive worker e workflow in TypeScript, definisce workflow come codice anche in Rust,
amministra tutto dall'SDK TypeScript e usa la CLI.

**FRs covered:** FR-35, FR-37 (TypeScript), FR-38 (parità dell'SDK TypeScript), FR-59 (righe dal worker TypeScript)
**Bloccata da:** AR-105 (livello BDD di SDK TS e CLI); AR-104 (distribuzione npm); AR-109 (dipendenze di `packages/sdk`); M-07 per TypeScript (AR-113).
**Note:** completa UJ-1 lato worker (`dns_lookup`, `censys_search` in TypeScript).

### Epic 6: Kit: seguire e lanciare i workflow

Lo sviluppatore della piattaforma ospite incorpora l'area workflow; il tenant vede l'elenco dei
workflow, li lancia con un modulo generato dallo schema di input e segue le esecuzioni in tempo
reale; colori, testi e lingua si adattano alla piattaforma ospite.

**FRs covered:** FR-27 (kit), FR-28 (kit), FR-39, FR-41, FR-44, FR-58, FR-59 (tab Log)
**NFR:** NFR-7, NFR-9, NFR-16
**Bloccata da:** S-4; skill del kit (AD-15); contratto pubblico del kit (AR-117); AR-104 (npm e validatore nativo).
**Note:** chiude UJ-1 (incorporare i componenti React).

### Epic 7: Linguaggio OWS completo e chiamate HTTP

Integratore e tenant usano tutto il sottoinsieme OWS della v1: `switch`, `fork`, `for`, `try` con
`catch`, `wait`, `raise`, sotto-workflow e `call http` verso le sole destinazioni consentite.

**FRs covered:** FR-1 (resto), FR-4 (costrutti), FR-16 (filtro di rete di `call http`)
**NFR:** NFR-8 (rete interna irraggiungibile), NFR-14
**Bloccata da:** M-30 filtro di rete nella spec (AR-116).

### Epic 8: Segreti e connessioni

Il tenant salva segreti legati a una connessione; l'attività li riceve solo alla consegna; non
compaiono mai in log, storico o errori; l'integratore ruota la chiave senza fermare nulla.

**FRs covered:** FR-4 (connessioni e segreti all'invio), FR-29 (connessioni risolte all'avvio), FR-32, FR-33, FR-34, FR-38 (segreti), FR-59 (segreti nelle righe)
**NFR:** NFR-8 (prova sentinella), NFR-14
**Bloccata da:** M-37 nella spec (AR-89); rinnovo delle chiavi dei tenant e consegne a database fermo (AR-110).

### Epic 9: Pianificazioni

Il proprietario di un workflow lo fa partire da solo con un cron o un intervallo, dal `schedule` del
documento o con la pianificazione propria del tenant via API e SDK; una sola esecuzione per scadenza,
rifiuti registrati con il motivo.

**FRs covered:** FR-1 (`schedule`), FR-6 (pianificazioni), FR-57 (API e SDK)
**Bloccata da:** AR-87 (M-04); C-3 e `cron` in UTC (AR-108, AR-109); §11 n.17 e n.18 nelle spec.

### Epic 10: Nodi personalizzati in sandbox

Il tenant carica dalla CLI o dall'SDK un nodo compilato TypeScript o Rust e lo usa nei suoi workflow;
la sandbox leggera, o la microVM dove c'è KVM, lo isola, lo limita e lo ferma senza toccare gli
altri tenant.

**FRs covered:** FR-1 (invocazione di nodi), FR-15, FR-16 (sandbox), FR-17 (limiti dei nodi), FR-30 (quote della sandbox), FR-31 (nodi e sandbox nella prova sentinella), FR-50 (processi ausiliari fermati e riavviati), FR-54, FR-55 (manifest dei nodi), FR-59 (righe dei nodi)
**NFR:** NFR-8 (codice ostile), NFR-10, NFR-12
**Bloccata da:** S-2; impronta `sha256:` del nodo (AR-108).
**Note:** il limite di UJ-2: un nodo in loop fermato al timeout.

### Epic 11: Pipeline stream

Integratore e tenant scrivono eventi sugli stream e definiscono pipeline in SQL con finestre,
watermark e checkpoint; le righe emesse vanno su uno stream o avviano un workflow, una volta per
riga; l'operatore vede lo stato delle pipeline.

**FRs covered:** FR-6 (righe delle pipeline), FR-17 (limiti delle query), FR-18, FR-19, FR-20, FR-21, FR-22, FR-23, FR-24, FR-25, FR-26 (scrittura sugli stream), FR-30 (pipeline ed eventi), FR-31 (checkpoint nella prova sentinella), FR-38 (pipeline), FR-47
**NFR:** NFR-5, NFR-8 (query ostili)
**Bloccata da:** S-3; C-4 (AR-108); caso reale di Marco (§11 n.1), altrimenti UJ-3 come assunzione.
**Note:** UJ-3 senza editor.

### Epic 12: Kit: comporre i workflow

Il tenant compone un workflow come grafo, sceglie i segreti, gestisce pianificazioni e pipeline
stream dal kit, anche da telefono.

**FRs covered:** FR-5 (salvare = pubblicare, avvisi), FR-15 (nodi nell'editor), FR-32 (selettore), FR-40, FR-42, FR-43, FR-57 (kit)
**NFR:** NFR-7, NFR-9, NFR-16
**Bloccata da:** AR-91 (posizioni dei nodi, moduli dei costrutti).
**Note:** chiude UJ-2 e UJ-3; SM-6.

### Epic 13: Pronto per il rilascio

L'integratore aggiorna topictoast senza perdite, una persona nuova arriva al primo workflow in 15
minuti con la guida rapida, e il benchmark pubblico dimostra gli obiettivi.

**FRs covered:** FR-51, FR-52 (metriche complete), FR-53, FR-59 (log nella corsa a carico misto)
**NFR:** NFR-1…NFR-5 (benchmark), NFR-6 (guasti iniettati), NFR-8 (suite completa), NFR-9 (prova con NVDA e VoiceOver), NFR-11, NFR-13
**Note:** SM-2…SM-5 prima del primo rilascio pubblico; il benchmark di base nasce nella storia 1.11, sulle misure di S-1, e qui diventa pubblico e completo. L'adattatore facoltativo RustFS dell'archivio dei payload (AD-17) chiude l'epica.

### Dove si chiudono le Open Questions e i rinvii dell'architettura

| AR | Casa |
| --- | --- |
| AR-86 | chiusa (AD-20); la parte delle connessioni nell'Epic 8 (AR-89) |
| AR-87 | Epic 9 |
| AR-88 | Epic 2 |
| AR-89 | Epic 8 |
| AR-90 | Epic 3 |
| AR-91 | Epic 12 |
| AR-92 | chiusa (FR-59): Epic 2, 3, 4, 5, 6, 8, 10, 13 |
| AR-93, AR-94, AR-95 | non chiuse da S-1 (ADR del bus, 2026-11-26): AR-93 prima della prima storia di «Esegui», AR-94 nella prima storia Postgres, AR-95 nella prima storia di aggiornamento (FR-51); scadenze nelle Open Questions della spine |
| AR-96 | Prima dello sprint 1 |
| AR-97 | Epic 1, che decide per tutti i JSON dei tenant elencati |
| AR-98 | Epic 1 |
| AR-99, AR-100 | Epic 3 |
| AR-101 | Epic 1 |
| AR-102 | chiusa (C-19) |
| AR-103 | non chiusa da S-2 (ADR della sandbox, 2026-11-26): riavvio, endpoint di salute e formato di distribuzione nella prima storia di installazione (Epic 1) |
| AR-104 | Epic 1 (SDK Rust); Epic 5 e Epic 6 (npm, validatore nativo per Node) |
| AR-105 | Epic 5 |
| AR-106 | chiusa (Stack della spine) |
| AR-107 | S-4 (Marco) |
| AR-108 | Epic 1 (tipo del proprietario, percorsi RFC 6901, microsecondi, indirizzo dei payload, parole chiave di AD-10); Epic 9 (C-3); Epic 10 (impronta del nodo); Epic 11 (C-4) |
| AR-109 | Epic 1 (canonicalizzazione, predefiniti dal motore, byte e 1 MB, autorizzazione per operazione); Epic 2 (`If-Match`); Epic 5 (`packages/sdk`); Epic 9 (`cron` in UTC) |
| AR-110 | Epic 1 (`npm ci --ignore-scripts`, `cargo audit`, «localhost», ambito del tenant, controlli delle Rule, campi di correlazione, `/metrics`); Epic 8 (rinnovo delle chiavi dei tenant, consegne a database fermo) |
| AR-111 | chiusa (Stack della spine) |
| AR-112 | Epic 1 |
| AR-113 | Epic 1 (M-06, M-07 per Rust); Epic 5 (M-07 per TypeScript) |
| AR-114 | Epic 1 (M-08); Epic 2 (M-09, compatibilità tra campi) |
| AR-115 | Epic 1 (M-10); Epic 2 (M-19); Epic 4 (M-23) |
| AR-116 | Epic 2 (M-29, interprete jq); Epic 7 (M-30, filtro di rete) |
| AR-117 | Epic 3 (M-44, nomi neutri e temi di `packages/ui`); Epic 6 (contratto pubblico del kit) |
| AR-118 | Epic 2 (T-65); Epic 3 (T-57, T-59, T-68); Epic 8 (conteggio d'uso dei segreti, T-67) |
| AR-119 | Epic 3 (ricerca tra gli elementi del fan-out, A-5); spec delle storie (AMB-1, AMB-5, AMB-6, AMB-9); versioni delle librerie alla prima storia che le usa |
| AR-120 | dopo la v1 |

Le voci UX legate alla tab Log (log dei task) entrano nelle epiche 3 e 6: il PRD le porta nel
perimetro con FR-59 (2026-11-23).

## Epic 1: Primo workflow eseguito

L'integratore installa topictoast e il bus con SQLite e la configurazione predefinita, pubblica via
API un documento OWS che chiama in sequenza le funzioni di un worker Rust, avvia l'esecuzione e ne
legge l'esito; un crash del motore a metà esecuzione non perde nulla.

Ogni storia riporta: le regole dell'architettura che rispetta (**Rispetta**), i requisiti che copre
(**Requisiti**), la prova che l'utente ripete (**Prova per l'utente**) e la pagina di `docs/user/`
che nasce nella stessa PR (**Docs utente**). I criteri usano Dato / Quando / Allora / E, come gli
scenari Gherkin in italiano che ne derivano.

### Story 1.1: Repository pronto con il gate verde

Come sviluppatore di topictoast,
voglio il workspace Cargo con i crate puri, il binario del motore, gli hook e l'ambiente di sviluppo,
così che ogni storia successiva parta da un gate verde con le regole di dipendenza già imposte.

**Rispetta:** AD-14, AD-15, AD-16; costituzione III e VIII
**Requisiti:** AR-1…AR-6, AR-78, AR-80, AR-82; AR-110 (controlli nel pre-push)
**Prova per l'utente:** da un clone pulito, `git config core.hooksPath .githooks`, poi il gate di CLAUDE.md passa e `docker compose up` avvia bus, Postgres e RustFS.
**Docs utente:** nessuna (non è una funzione per chi usa topictoast).

**Acceptance Criteria:**

**Dato** un clone pulito con gli hook attivi
**Quando** si esegue il gate di CLAUDE.md
**Allora** passano `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc` con `-D warnings` e la build di `crates/domain` per `wasm32-wasip1`
**E** il binario del motore stampa la sua versione

**Dato** il workspace con `crates/ows`, `crates/domain`, `crates/protocol` e `crates/engine`
**Quando** uno dei tre crate puri aggiunge un runtime async, un client HTTP, un driver di database o un crate del workspace vietato da AD-16
**Allora** il test `purity.rs` fallisce nominando crate e dipendenza

**Dato** `missing_docs` attivo
**Quando** un item pubblico non ha rustdoc in italiano
**Allora** `cargo doc` fallisce

**Dato** il file `docker compose` del repository
**Quando** lo sviluppatore lo avvia
**Allora** partono il bus scelto da S-1, Postgres e RustFS, senza profili né politiche di riavvio

**Dato** un push
**Quando** gira il pre-push
**Allora** esegue anche i controlli decisi per AR-110 (per esempio `cargo audit`)
**E** `packages/` e `dashboard/` non vengono creati in anticipo: nascono con la prima storia che li usa

**Dato** l'impalcatura degli scenari BDD (AD-14)
**Quando** gira lo scenario di esempio in `features/`, in italiano con `# language: it`
**Allora** cucumber-rs avvia bus e database reali con testcontainers e il binario del motore già costruito, mai il motore dentro il test
**E** lo scenario è stato visto rosso prima di diventare verde

### Story 1.2: Primo avvio sicuro con la configurazione predefinita

Come integratore,
voglio avviare topictoast e il bus con SQLite senza configurare nulla,
così che ho in pochi minuti un'installazione che funziona e non ha valori predefiniti noti.

**Rispetta:** AD-1, AD-2 (lock esclusivo), AD-5 (identificativo interno), AD-7 (chiave principale e permessi); C-9, C-10, C-12, C-14
**Requisiti:** FR-50, FR-26 (tenant predefinito), FR-52 (log), NFR-8, NFR-14; AR-96, AR-102, AR-103 (formato di distribuzione), AR-110 («localhost»)
**Prova per l'utente:** avviare il bus e il motore con il comando della guida; il log dice dove sono le credenziali del primo avvio; un secondo motore sulla stessa installazione esce con un codice.
**Docs utente:** installazione e primo avvio.

**Acceptance Criteria:**

**Dato** un host Linux senza Kubernetes, KVM né root, con il bus avviato
**Quando** l'integratore avvia il motore con la configurazione predefinita
**Allora** il motore usa SQLite incorporato e accetta connessioni solo da localhost, come definito in AR-110
**E** il tenant predefinito esiste, con un identificativo interno UUIDv7 e il nome esterno `default`

**Dato** un indirizzo d'ascolto fuori da localhost nella configurazione
**Quando** il motore parte
**Allora** ascolta solo in TLS, con i certificati dell'integratore o con certificati generati al primo avvio di cui stampa l'impronta (C-14)
**E** senza certificati validi rifiuta l'avvio con un codice

**Dato** un deposito vuoto
**Quando** il motore parte per la prima volta
**Allora** genera la chiave principale, le credenziali del primo avvio e la credenziale per il bus
**E** le scrive in file con permessi 0600, fuori dalle directory dati di topictoast e del bus, senza stamparle su stdout né nei log
**E** il log dice solo dove si trovano

**Dato** due installazioni nuove
**Quando** si confrontano chiavi e credenziali generate
**Allora** sono diverse

**Dato** una configurazione che contiene un valore presente negli esempi della documentazione
**Quando** il motore parte
**Allora** rifiuta l'avvio con un codice C-7

**Dato** la chiave principale o la cartella dati leggibili da altri utenti, oppure una chiave che non corrisponde all'impronta conservata
**Quando** il motore parte
**Allora** rifiuta l'avvio con un codice che nomina il problema

**Dato** un motore già attivo sulla stessa installazione
**Quando** se ne avvia un secondo
**Allora** il secondo esce con un codice e il primo continua

**Dato** una configurazione non valida o con un valore oltre il massimo di NFR-14
**Quando** il motore parte
**Allora** l'avvio fallisce con l'errore di validazione e il campo
**E** i valori predefiniti si applicano solo dopo la validazione (C-10)

**Dato** il motore in esecuzione
**Quando** scrive un log
**Allora** ogni evento è una riga JSON su stdout, senza segreti, token, chiavi né cookie

### Story 1.3: Worker Rust che registra funzioni con il manifest

Come integratore,
voglio scrivere in Rust un worker che registra funzioni tipizzate con il loro manifest,
così che i miei workflow possano chiamarle.

**Rispetta:** AD-6 (manifest controllato fuori dal motore), AD-10, AD-12 (protocollo dei worker), AD-16; C-6, C-11, C-14
**Requisiti:** FR-12, FR-37 (Rust), FR-55, NFR-8, NFR-14; AR-104 (SDK Rust), AR-113 (M-06, M-07 per Rust), AR-114 (M-08)
**Prova per l'utente:** avviare il worker di esempio del repository con la credenziale di worker del primo avvio; l'API elenca la funzione registrata con il suo manifest.
**Docs utente:** scrivere un worker in Rust.

**Acceptance Criteria:**

**Dato** un worker Rust con una funzione tipizzata
**Quando** si collega con una credenziale di worker valida
**Allora** registra la funzione con il manifest ricavato dai tipi: nome, descrizione, gruppo, connessioni richieste, schema di input, schema di output, campo di estensione
**E** la funzione risulta disponibile per i workflow dell'integratore

**Dato** un processo senza credenziale o con una credenziale non valida
**Quando** prova a collegarsi o a registrare una funzione
**Allora** viene rifiutato e non riceve attività né segreti

**Dato** un manifest non valido, con un riferimento a uno schema esterno o oltre i limiti di NFR-14
**Quando** il worker lo registra
**Allora** è rifiutato con l'errore del validatore, senza aprire connessioni di rete
**E** il controllo gira in un processo ausiliario, non nel motore (AD-6)

**Dato** un worker che dichiara una versione del protocollo incompatibile (C-6)
**Quando** si registra
**Allora** è rifiutato con un codice

**Dato** un worker su un altro host
**Quando** si collega in chiaro
**Allora** è rifiutato
**E** su TLS l'SDK verifica sempre il certificato del motore

### Story 1.4: Pubblicare un workflow OWS via API e SDK Rust

Come integratore,
voglio inviare un documento OWS e averlo subito eseguibile,
così che pubblico un workflow nuovo senza riavviare nulla.

**Rispetta:** AD-4, AD-5, AD-6, AD-10, AD-11, AD-12; C-7, C-10, C-16
**Requisiti:** FR-36, FR-1 (nucleo), FR-4 (nucleo), FR-56, NFR-7, NFR-14; AR-86 (M-02), AR-108 (percorsi RFC 6901, parole chiave di AD-10), AR-109 (forma canonica, predefiniti dal motore), AR-112
**Prova per l'utente:** inviare con `curl` il workflow di esempio; la risposta porta id e versione; rileggerlo restituisce il documento in forma canonica.
**Docs utente:** pubblicare un workflow (sottoinsieme OWS della v1, errori di validazione).

**Acceptance Criteria:**

**Dato** un documento OWS 1.1 che usa `do`, `set` e l'invocazione di funzioni registrate
**Quando** l'integratore lo invia via API o SDK Rust
**Allora** la risposta di successo arriva quando il workflow è già eseguibile, con identificativo UUIDv7 e versione

**Dato** un documento con un costrutto fuori dal sottoinsieme (per esempio `emit`, `listen`, `call` gRPC, `run`)
**Quando** viene inviato
**Allora** la risposta è `application/problem+json` con il codice C-7, il costrutto e il percorso JSON Pointer
**E** non viene salvato nulla

**Dato** un documento con `document.dsl` diverso da 1.1.x
**Quando** viene inviato
**Allora** l'errore indica le versioni accettate

**Dato** un task che chiama una funzione non registrata, o le passa un input incompatibile con lo schema di input del manifest
**Quando** il documento viene inviato
**Allora** l'errore nomina il task e il percorso

**Dato** uno schema di input del workflow (`input.schema`) con una parola chiave fuori da AD-10, come `$ref` o `oneOf`
**Quando** viene inviato
**Allora** è rifiutato con un errore che nomina la parola chiave
**E** uno schema valido viene conservato per il controllo all'avvio (storia 1.7)

**Dato** un documento oltre i limiti di NFR-14 (dimensione, numero di task, profondità)
**Quando** viene inviato
**Allora** l'errore indica il limite e il valore, e il motore continua a servire le altre richieste

**Dato** un documento che fa fallire il validatore
**Quando** viene inviato
**Allora** fallisce solo il processo ausiliario che lo valida e il motore resta attivo (AD-6)

**Dato** un workflow accettato
**Quando** viene riletto
**Allora** il documento è rigenerato dalle tabelle tipizzate in forma canonica, con l'ordine dei task conservato (AD-4)
**E** il repository dei workflow ha un test a due tenant per ogni metodo (AD-5), con SQLite; Postgres si aggiunge con la storia 1.10

**Dato** due worker che registrano la stessa funzione con manifest diversi, oppure una funzione il cui worker si è scollegato
**Quando** un workflow la cita
**Allora** vale la regola fissata nella spec della storia per M-08

### Story 1.5: Avviare un'esecuzione e leggerne l'esito

Come integratore,
voglio avviare un workflow con un input e leggerne stato e output,
così che posso usare topictoast dalla mia applicazione.

**Rispetta:** AD-2, AD-3, AD-5, AD-7, AD-17; C-1, C-2, C-7, C-16, C-17
**Requisiti:** FR-6 (avvio da API e SDK, origine), FR-9, FR-12 (attività in coda senza worker), FR-50 (esempio che gira senza modifiche), NFR-6; AR-97, AR-101, AR-108 (indirizzo dei payload, microsecondi)
**Prova per l'utente:** con il worker e il workflow di esempio, avviare un'esecuzione con `curl`; la risposta porta l'id; rileggendola risulta completata con l'output dell'ultimo task.
**Docs utente:** avviare un'esecuzione e leggerne l'esito; idempotenza dei worker (consegna almeno una volta).

Questa storia porta il percorso intero bus → proiettore → archivio dei payload → consegna: è grande per natura, perché AD-2 e AD-3 non si rimandano.

**Acceptance Criteria:**

**Dato** il workflow e il worker di esempio con la configurazione predefinita
**Quando** l'integratore avvia un'esecuzione via API o SDK Rust
**Allora** riceve subito l'identificativo dell'esecuzione
**E** l'esecuzione arriva a completata con l'output dell'ultimo task
**E** l'esecuzione registra l'origine (API o SDK) e il ruolo di chi l'ha avviata

**Dato** una richiesta di avvio
**Quando** il motore risponde con successo
**Allora** l'esecuzione era già scritta in modo durevole sul log del bus (AD-2)

**Dato** una funzione senza worker collegati
**Quando** un'esecuzione la chiama
**Allora** l'attività resta in coda con il motivo e parte appena un worker registra la funzione

**Dato** un'esecuzione completata con input e output sentinella
**Quando** si cercano i valori sentinella nei file del database
**Allora** non si trovano: input e output stanno nell'archivio dei payload, cifrati con la chiave del tenant, e nel database c'è solo il riferimento tipizzato (AD-3, AD-17)

### Story 1.6: Avvii ripetuti e letture subito dopo l'avvio

Come integratore,
voglio ripetere un avvio senza creare doppioni e rileggere subito un'esecuzione appena avviata,
così che un errore di rete o un client veloce non mi fanno perdere né duplicare lavoro.

**Rispetta:** AD-5, AD-18; C-7, C-18
**Requisiti:** FR-6 (idempotenza), NFR-14
**Prova per l'utente:** con lo script d'esempio lanciare 100 avvii concorrenti con la stessa `Idempotency-Key`: esiste una sola esecuzione; rileggere un'esecuzione con il token di posizione ricevuto la mostra subito.
**Docs utente:** chiave di idempotenza; letture subito dopo una scrittura.

**Acceptance Criteria:**

**Dato** 100 richieste concorrenti con la stessa `Idempotency-Key` dallo stesso tenant
**Quando** arrivano al motore
**Allora** esiste una sola esecuzione e tutte le risposte la restituiscono
**E** la stessa chiave usata in due tenant dà due esecuzioni distinte, con un test a due tenant (AD-5)

**Dato** un avvio che arriva dall'IPC (le pipeline stream, AD-13) e non da HTTP
**Quando** il motore lo decide
**Allora** il percorso di avvio è lo stesso, con un esito per avvio (accettato, già visto, rifiutato con il motivo)
**E** la finestra di idempotenza è un parametro documentato (NFR-14), la cui durata minima fissa la storia 11.7

**Dato** una risposta di avvio
**Quando** il client rilegge l'esecuzione passando il token di posizione ricevuto
**Allora** la vede subito, oppure riceve 503 con un codice C-7 e `Retry-After` dopo al massimo 1 secondo di attesa, mai 404 (AD-18)

**Dato** un token di posizione fuori dalla conservazione del bus
**Quando** il client lo usa
**Allora** riceve un errore C-7 che chiede di ricaricare

### Story 1.7: Schemi e limiti delle attività

Come integratore,
voglio che input e output siano controllati sugli schemi e sui limiti,
così che i dati sbagliati si fermano subito con un errore chiaro e non arrivano mai ai task a valle.

**Rispetta:** AD-6, AD-10; C-7, C-10
**Requisiti:** FR-11 (limite per attività), FR-55 (controllo di input e output), FR-56, NFR-7, NFR-14; AR-109 (byte ricevuti e 1 MB), AR-115 (M-10)
**Prova per l'utente:** avviare l'esempio con un input fuori schema: è rifiutato con percorso e regola; con la funzione d'esempio che restituisce 2 MB l'attività fallisce con dimensione e limite.
**Docs utente:** schemi di input e output; limite di dimensione.

**Acceptance Criteria:**

**Dato** un input che non rispetta lo schema di input del workflow
**Quando** l'integratore avvia
**Allora** l'avvio è rifiutato prima di essere accettato, con percorso e regola violata, senza il valore ricevuto

**Dato** un'attività il cui input non rispetta lo schema del manifest
**Quando** sta per essere consegnata
**Allora** il task fallisce prima della consegna, senza consumare tentativi

**Dato** un'attività che restituisce un output fuori dallo schema di output o oltre 1 MB, misurato sui byte ricevuti
**Quando** il worker lo invia
**Allora** l'attività fallisce con percorso e motivo, oppure con dimensione e limite
**E** quell'output non arriva mai ai task a valle

**Dato** campi facoltativi omessi nell'input
**Quando** il motore lo accetta
**Allora** i valori predefiniti si applicano dopo la validazione (C-10)

### Story 1.8: Ripresa dopo un crash senza perdite

Come integratore,
voglio che un crash del motore a metà esecuzione non perda nulla e che il motore torni a consegnare da solo,
così che posso fidarmi di topictoast in produzione.

**Rispetta:** AD-1, AD-2, AD-18; C-2, C-7
**Requisiti:** FR-9, NFR-6, NFR-15; AR-98 (fase operativa), AR-103 (chi riavvia il motore, deciso nella prima storia di installazione)
**Prova per l'utente:** durante l'esempio, `kill -9` del processo del motore; entro 30 secondi torna a consegnare e l'esecuzione arriva a completata.
**Docs utente:** cosa succede dopo un crash; limiti noti (backup e ripristino coerenti fuori dalla v1).

**Acceptance Criteria:**

**Dato** un'esecuzione in corso
**Quando** il processo del motore viene ucciso con `SIGKILL`
**Allora** il motore riparte da solo e torna a consegnare attività entro 30 secondi, senza intervento
**E** l'esecuzione arriva in fondo

**Dato** la ripresa dopo il crash
**Quando** il motore rilegge il bus
**Allora** nessun task con completamento registrato viene rieseguito
**E** un'attività consegnata senza esito può essere riconsegnata, e per ogni `attemptId` vale il primo esito (C-2)

**Dato** il motore in ripresa
**Quando** arriva una richiesta di lavoro nuovo
**Allora** non partono esecuzioni né task successivi finché la ripresa non è finita
**E** l'API espone la fase operativa del motore come deciso per AR-98

**Dato** worker collegati al momento del crash
**Quando** il motore torna attivo
**Allora** i worker si ricollegano da soli e finiscono le attività che avevano, senza perdere i risultati scritti nel frattempo

**Dato** il disco pieno
**Quando** arriva lavoro nuovo
**Allora** è rifiutato con un codice C-7
**E** ogni scrittura rifiutata lascia intatto lo stato precedente e nessun lavoro accettato va perso

**Dato** un evento del bus che il proiettore non riesce ad applicare
**Quando** il proiettore lo incontra
**Allora** si ferma con un errore e non lo salta (AD-2)
**E** al riavvio riprende dall'ultima posizione applicata, e riapplicare un gruppo non duplica righe

### Story 1.9: Log e metriche di base

Come operatore,
voglio log JSON e metriche in formato standard fin dal primo giorno,
così che capisco cosa fa il motore con gli strumenti che già uso.

**Rispetta:** C-9, C-14
**Requisiti:** FR-52 (base); AR-110 (campi di correlazione, `/metrics` fuori da localhost)
**Prova per l'utente:** dopo un'esecuzione di esempio, `curl` su `/metrics` mostra attività al secondo, latenza di avvio e code per tenant.
**Docs utente:** metriche e log.

**Acceptance Criteria:**

**Dato** il motore in esecuzione
**Quando** si legge `/metrics` da localhost
**Allora** la risposta è in formato OpenMetrics, con attività al secondo, latenza di avvio delle attività e code per tenant
**E** il tenant compare solo con un identificativo opaco

**Dato** un ascolto fuori da localhost
**Quando** si legge `/metrics` senza la credenziale dell'operatore
**Allora** la richiesta è rifiutata (C-14)

**Dato** un'esecuzione con input e output sentinella
**Quando** si cercano i valori nei log
**Allora** non si trovano, come non si trovano token, chiavi e nomi visualizzati
**E** ogni riga ha i campi di correlazione fissati per AR-110

**Dato** un processo ausiliario che scrive su stderr
**Quando** termina con un errore
**Allora** nei log del motore compaiono solo il codice C-7 e gli identificativi, non il suo stderr

### Story 1.10: Postgres al posto di SQLite

Come integratore,
voglio usare il mio Postgres al posto di SQLite,
così che metto lo stato di topictoast nel database che già gestisco e di cui faccio il backup.

**Rispetta:** AD-1, AD-5 (RLS come seconda barriera); C-14; costituzione V
**Requisiti:** FR-50 (database a scelta), NFR-8; AR-69 (D-2: Postgres 18 e 19), AR-94 (proiettore sotto RLS, deciso in S-1)
**Prova per l'utente:** configurare il motore con il Postgres del `docker compose` e un utente non superuser: l'esempio dell'Epic 1 gira uguale; con un utente superuser o con `BYPASSRLS` il motore rifiuta di partire.
**Docs utente:** usare Postgres.

**Acceptance Criteria:**

**Dato** Postgres 18.7 o successiva, o 19.0 o successiva, con un utente applicativo
**Quando** il motore parte
**Allora** crea o migra le tabelle e l'esempio dell'Epic 1 gira come con SQLite
**E** una versione sotto le minime è rifiutata con un codice

**Dato** un utente superuser o con `BYPASSRLS`
**Quando** il motore parte
**Allora** rifiuta l'avvio con un codice

**Dato** ogni tabella con dati dei tenant
**Quando** si ispeziona lo schema
**Allora** ha RLS con `FORCE ROW LEVEL SECURITY` e il tenant impostato solo per la transazione
**E** una query senza tenant impostato non vede righe

**Dato** i test a due tenant di ogni repository
**Quando** girano
**Allora** passano con SQLite e con Postgres, e da qui ogni nuovo metodo di repository li ha per entrambi

**Dato** un Postgres su un altro host
**Quando** il motore si collega
**Allora** usa TLS e verifica il certificato del server

**Dato** Postgres fermo
**Quando** arriva lavoro nuovo
**Allora** è rifiutato con un codice C-7 e nessun lavoro accettato va perso
**E** il motore riprende da solo quando il database torna

### Story 1.11: Benchmark di base nel workspace

Come sviluppatore di topictoast,
voglio un benchmark di base nel workspace che misuri gli obiettivi del motore a ogni storia,
così che so subito se una modifica allontana topictoast da NFR-2, NFR-3 e NFR-4.

**Rispetta:** AD-1, AD-2, AD-15 (`crates/bench`), AD-16 (`crates/bench` usa `crates/sdk`)
**Requisiti:** FR-53 (base), NFR-2, NFR-3, NFR-4, NFR-6
**Prova per l'utente:** lanciare il comando del benchmark di base: il rapporto riporta latenza di avvio p50 e p99, attività al secondo e istruzioni SQL per attività, con SQLite e con Postgres, accanto agli obiettivi.
**Docs utente:** nessuna (strumento di sviluppo; il benchmark pubblico è la storia 13.3).

**Acceptance Criteria:**

**Dato** `crates/bench` con il worker e il workflow di esempio
**Quando** lo sviluppatore lancia il benchmark di base con un solo comando
**Allora** il rapporto riporta latenza di avvio p50 e p99 a 1.000 attività al secondo, attività al secondo e istruzioni SQL per attività, con SQLite e con Postgres
**E** ogni valore sta accanto al suo obiettivo (NFR-2, NFR-3, NFR-4)

**Dato** un guasto iniettato durante la corsa
**Quando** la corsa finisce
**Allora** il rapporto conta le attività accettate e poi perse, con obiettivo zero

**Dato** le storie successive che toccano il motore
**Quando** chiudono
**Allora** rilanciano il benchmark di base e riportano nella PR i numeri prima e dopo

## Epic 2: DAG, fan-out e controllo delle esecuzioni

L'integratore esegue workflow con task `dag` e dipendenze arbitrarie, un fan-out dinamico con join
su centinaia di elementi, retry, timeout e annullamento; pubblica versioni nuove mentre le vecchie
girano; un worker lento o muto non blocca gli altri.

### Story 2.1: Espressioni jq per dati e condizioni

Come integratore,
voglio usare espressioni jq per passare dati tra i task, scegliere elenchi e scrivere condizioni,
così che i workflow lavorano sui dati reali senza codice in più nei worker.

**Rispetta:** AD-6 (jq in un processo ausiliario senza la chiave), AD-11 (stessa implementazione Rust anche nel browser); C-7
**Requisiti:** FR-1 (espressioni jq), FR-4 (jq non valido o vietato), FR-17 (limiti di jq), NFR-2, NFR-14; AR-116 (M-29, interprete jq)
**Prova per l'utente:** pubblicare il workflow di esempio che calcola con `set` il numero di indicatori ricevuti e lo usa in una condizione `if`; l'esecuzione mostra il valore calcolato. Lo stesso documento con `$ENV` è rifiutato all'invio.
**Docs utente:** espressioni jq (funzioni ammesse, limiti).

**Acceptance Criteria:**

**Dato** un'espressione jq in `set`, nell'input o nell'output di un task o in una condizione `if`
**Quando** l'esecuzione la valuta
**Allora** il risultato segue la semantica di jq e di OWS sul contesto del task

**Dato** un'espressione con sintassi non valida, oppure che usa `$ENV`, `env`, `input_filename`, `import`, `include` o una funzione fuori dall'elenco chiuso fissato per M-29
**Quando** il documento viene inviato
**Allora** è rifiutato alla validazione con il percorso e la funzione o l'errore di sintassi

**Dato** un'espressione che supera i limiti di CPU, memoria, tempo o profondità
**Quando** viene valutata
**Allora** il task fallisce con un errore che nomina il limite superato
**E** il motore resta attivo e le altre esecuzioni restano entro l'obiettivo di NFR-2

**Dato** la valutazione di un'espressione
**Quando** gira
**Allora** gira in un processo ausiliario senza la chiave (AD-6), con un'implementazione Rust che compila anche per `wasm32-wasip1` (AD-11)

### Story 2.2: Task `dag` con dipendenze arbitrarie

Come integratore,
voglio dichiarare per ogni task da quali altri task dipende,
così che i task indipendenti partono in parallelo e quelli a valle aspettano tutto ciò che serve loro.

**Rispetta:** AD-8, AD-9, AD-11; C-7
**Requisiti:** FR-2, NFR-14; AR-88 (Update della spine prima di questa storia), AR-118 (T-65, limite sul numero di task)
**Prova per l'utente:** pubblicare il workflow di esempio a diamante (A, poi B e C in parallelo, poi D); le esecuzioni mostrano B e C partiti insieme e D partito dopo entrambi; con B che fallisce, D risulta saltato con il motivo.
**Docs utente:** il task `dag` (dipendenze, condizioni con `if`, regola di propagazione).

**Acceptance Criteria:**

**Dato** un task `dag` in cui D dipende da B e da C, e B e C dipendono da A
**Quando** l'esecuzione parte
**Allora** B e C partono appena A è completato, in parallelo
**E** D parte solo quando B e C sono entrambi completati

**Dato** un documento con un ciclo nelle dipendenze (`dependsOn`)
**Quando** viene inviato
**Allora** è rifiutato con i task del ciclo nel messaggio
**E** `then` fuori dal `dag` mantiene la semantica OWS

**Dato** un task dentro un `dag` che usa `then`, `exit`, `end` o `switch`
**Quando** il documento viene inviato
**Allora** è rifiutato con il costrutto e il percorso; le condizioni si scrivono con `if`

**Dato** una dipendenza fallita dopo i retry, o saltata
**Quando** il motore valuta i task a valle
**Allora** il task a valle e tutti i suoi discendenti risultano saltati, ciascuno con il motivo
**E** i rami indipendenti arrivano in fondo
**E** l'esecuzione risulta fallita se almeno un task è fallito (AD-9)

**Dato** un documento senza `dag`
**Quando** viene accettato
**Allora** resta OWS puro
**E** un documento che usa `$secrets` o `$authorization` è rifiutato, con o senza `dag` (AD-8)

**Dato** un `dag` con più task del limite documentato (NFR-14)
**Quando** viene inviato
**Allora** l'errore indica limite e valore

### Story 2.3: Fan-out dinamico con join

Come integratore,
voglio ripetere un task per ogni elemento di un elenco prodotto a runtime e raccogliere tutti i risultati in un join,
così che elaboro centinaia di indicatori in parallelo con un solo workflow.

**Rispetta:** AD-3, AD-8, AD-9, AD-17; C-2
**Requisiti:** FR-3, FR-11 (tetto del join), NFR-14; AR-88
**Prova per l'utente:** avviare il workflow di esempio con 300 indicatori; l'esecuzione mostra 300 attività del fan-out e il join riceve 300 risultati nell'ordine dell'elenco.
**Docs utente:** fan-out e join (elenco vuoto, input singolo, parallelismo, politica d'errore).

**Acceptance Criteria:**

**Dato** un task che produce un elenco di N elementi e un fan-out che lo consuma
**Quando** l'esecuzione arriva al fan-out
**Allora** partono N attività e il join riceve N risultati nell'ordine dell'elenco

**Dato** un elenco vuoto
**Quando** l'esecuzione arriva al fan-out
**Allora** non parte nessuna attività e il join riceve un elenco vuoto

**Dato** un input singolo dove il fan-out si aspetta un elenco
**Quando** l'esecuzione arriva al fan-out
**Allora** viene trattato come un elenco di un elemento

**Dato** un parallelismo massimo dichiarato nel workflow
**Quando** il fan-out gira
**Allora** non ci sono mai più attività in corso di quel valore
**E** senza un valore dichiarato vale il limite predefinito dell'installazione (NFR-14); la quota per tenant si aggiunge nell'Epic 4

**Dato** un elemento che fallisce dopo i retry
**Quando** il join lo attende
**Allora** si applica la politica dichiarata: fallisce tutto il fan-out, oppure il join riceve l'errore al posto di quel risultato, con l'effetto sugli stati fissato dall'Update di AR-88

**Dato** un input aggregato del join oltre il tetto configurato (predefinito documentato, NFR-14)
**Quando** il join sta per partire
**Allora** il join fallisce con dimensione e limite
**E** un elenco con più elementi del limite di NFR-14 fa fallire il task con limite e valore, senza creare attività

### Story 2.4: Retry con attesa crescente

Come integratore,
voglio dichiarare per ogni task quante volte ritentare e quanto aspettare tra un tentativo e l'altro,
così che un errore temporaneo di un servizio esterno non fa fallire l'esecuzione.

**Rispetta:** AD-2, AD-9; C-2
**Requisiti:** FR-7, NFR-6, NFR-14
**Prova per l'utente:** con la funzione di esempio che fallisce le prime due volte, l'esecuzione mostra tre tentativi distinti e arriva a completata.
**Docs utente:** retry (tentativi, attesa, crescita dell'attesa).

Prima di questa storia: l'Update della spine su join e fan-out dentro `dag` (AR-88) fissa anche come si dichiara il retry di un task dentro un `dag`, per esempio un `try` come nodo del `dag`.

Il resto di `try` (`catch` con `do`, `when`, `as`) e `raise` arrivano nell'Epic 7.

**Acceptance Criteria:**

**Dato** un task dentro un `try` con `catch.retry` di OWS (numero di tentativi, attesa, crescita dell'attesa)
**Quando** un'attività fallisce
**Allora** il motore la ritenta secondo la politica, e ogni tentativo è un'attività distinta con il proprio `attemptId`, visibile nello storico

**Dato** i tentativi esauriti
**Quando** fallisce l'ultimo
**Allora** il task fallisce e conserva l'errore dell'ultimo tentativo

**Dato** un retry in attesa
**Quando** il motore viene ucciso durante l'attesa
**Allora** dopo la ripresa il tentativo parte allo scadere dell'attesa originale, né prima né perso

**Dato** un task senza politica dichiarata
**Quando** fallisce
**Allora** vale la politica predefinita documentata (NFR-14)
**E** un numero di tentativi o un'attesa oltre il massimo dell'installazione sono rifiutati all'invio

### Story 2.5: Annullare un'esecuzione

Come integratore,
voglio annullare un'esecuzione in coda o in corso,
così che fermo subito il lavoro che non serve più.

**Rispetta:** AD-2, AD-18; C-2, C-7
**Requisiti:** FR-10; S-1 (arresto delle attività in corso)
**Prova per l'utente:** avviare l'esempio con la funzione `sleep`, annullarlo con `curl`; l'esecuzione risulta annullata, con ruolo e ora, e il worker riceve la richiesta di fermarsi.
**Docs utente:** annullare un'esecuzione.

**Acceptance Criteria:**

**Dato** un'esecuzione in coda o in corso
**Quando** l'integratore la annulla via API o SDK Rust
**Allora** nessuna nuova attività parte dopo l'annullamento
**E** i worker ricevono la richiesta di fermare le attività in corso

**Dato** un'esecuzione annullata
**Quando** si legge lo storico
**Allora** risulta annullata, con il ruolo di chi l'ha annullata, il nome visualizzato se presente, e l'ora

**Dato** un'esecuzione già terminata
**Quando** viene annullata
**Allora** la risposta è un errore C-7 e lo stato non cambia
**E** due annullamenti concorrenti della stessa esecuzione producono un solo annullamento registrato

### Story 2.6: Timeout di task e di esecuzione

Come integratore,
voglio dichiarare quanto può durare un task e quanto l'intera esecuzione,
così che nessuna esecuzione resta appesa senza fine.

**Rispetta:** AD-2, AD-9; C-2, C-7
**Requisiti:** FR-8, NFR-14; AR-115 (M-19, due orologi per attività); §11 n.15 nella spec
**Prova per l'utente:** con la funzione di esempio `sleep`, un task con timeout di 2 secondi fallisce per timeout e segue il retry; un'esecuzione con timeout di 5 secondi termina come scaduta.
**Docs utente:** timeout e limite di attesa.

**Acceptance Criteria:**

**Dato** un'attività consegnata che supera il timeout del task
**Quando** scade il tempo
**Allora** è considerata fallita e segue la politica di retry

**Dato** un'esecuzione che supera il proprio timeout
**Quando** scade il tempo
**Allora** termina come scaduta e le attività in corso ricevono la richiesta di annullamento, come in un annullamento (storia 2.5)

**Dato** un'attività in attesa prima della consegna (nessun worker collegato)
**Quando** passa più del timeout del task
**Allora** non fallisce per timeout del task e non consuma tentativi
**E** l'attesa è limitata dal limite documentato (NFR-14): oltre quel limite l'attività prende lo stato fissato nella spec (§11 n.15)

**Dato** un'attività rimasta in coda più del timeout del task
**Quando** viene finalmente consegnata
**Allora** il timeout conta dalla consegna e l'attività si completa normalmente

**Dato** un documento senza timeout
**Quando** gira
**Allora** valgono i predefiniti documentati e l'esecuzione termina comunque (NFR-14)

### Story 2.7: Versioni nuove mentre le vecchie girano

Come integratore,
voglio pubblicare una versione nuova di un workflow mentre la precedente ha esecuzioni in corso,
così che aggiorno i workflow senza fermare nulla e senza sovrascrivere il lavoro di altri.

**Rispetta:** AD-4, AD-18; C-5, C-7
**Requisiti:** FR-5 (motore); AR-109 (`If-Match`), AR-114 (M-09, compatibilità tra campi)
**Prova per l'utente:** pubblicare la versione 2 dell'esempio mentre la versione 1 ha un'esecuzione lenta in corso; quella esecuzione finisce con la versione 1, le nuove partono con la 2.
**Docs utente:** versioni dei workflow.

**Acceptance Criteria:**

**Dato** un workflow con esecuzioni in corso sulla versione 1
**Quando** l'integratore pubblica la versione 2
**Allora** le esecuzioni in corso finiscono con la versione 1
**E** le nuove esecuzioni usano la versione 2, salvo una richiesta esplicita di un'altra versione

**Dato** una pubblicazione preparata sulla versione 1 quando l'ultima è già la 2
**Quando** arriva con `If-Match` della versione 1
**Allora** è rifiutata con un errore che indica la versione corrente, senza sostituire il lavoro di altri

**Dato** una pubblicazione senza `If-Match`
**Quando** arriva
**Allora** il motore si comporta come deciso per AR-109

**Dato** un cambio del manifest di una funzione dopo la pubblicazione
**Quando** partono esecuzioni nuove o riprendono quelle in coda
**Allora** vale la regola fissata nella spec per M-09

### Story 2.8: Concorrenza dichiarata dai worker

Come integratore,
voglio che ogni worker dichiari quante attività accetta insieme,
così che un worker non viene mai sovraccaricato e uno lento non rallenta gli altri.

**Rispetta:** AD-1, AD-12; C-2
**Requisiti:** FR-13, NFR-2, NFR-14
**Prova per l'utente:** due worker di esempio, uno con concorrenza 2 e una funzione lenta, l'altro veloce; con 100 attività avviate il primo non ne ha mai più di 2 in corso, e le attività del secondo partono entro gli obiettivi di NFR-2.
**Docs utente:** concorrenza e contropressione.

**Acceptance Criteria:**

**Dato** un worker che dichiara concorrenza N
**Quando** ci sono più di N attività pronte per le sue funzioni
**Allora** non riceve mai più di N attività in corso

**Dato** un worker lento o bloccato e un altro worker sano
**Quando** le attività di entrambi sono accodate insieme
**Allora** le attività consegnate al worker sano partono entro l'obiettivo di NFR-2 (p50 ≤ 5 ms, p99 ≤ 25 ms), misurato con il benchmark di base (storia 1.11)

**Dato** un worker senza concorrenza dichiarata
**Quando** si registra
**Allora** vale il predefinito documentato, con il massimo dell'installazione (NFR-14)

### Story 2.9: Worker che smette di rispondere

Come integratore,
voglio che topictoast si accorga di un worker che smette di rispondere e ne riassegni le attività,
così che un worker caduto non ferma le esecuzioni.

**Rispetta:** AD-2; C-2, C-11
**Requisiti:** FR-14, NFR-14
**Prova per l'utente:** avviare un'esecuzione lenta su due worker, uccidere quello che ha l'attività; dopo l'intervallo dei battiti l'attività riparte sull'altro e l'API mostra il primo come «non risponde».
**Docs utente:** worker che non risponde.

**Acceptance Criteria:**

**Dato** un worker che non manda segni di vita entro l'intervallo configurato
**Quando** l'intervallo scade
**Allora** le sue attività sono riassegnate ad altri worker e ognuna conta un tentativo
**E** l'API mostra il worker come «non risponde»

**Dato** il worker che torna dopo la riassegnazione
**Quando** invia il risultato di un tentativo non più corrente
**Allora** il risultato è rifiutato e registrato, e vale il primo esito per `attemptId` (C-2)

**Dato** un intervallo dei battiti non configurato o oltre il massimo
**Quando** il motore parte
**Allora** vale il predefinito documentato o l'avvio fallisce con l'errore di validazione (NFR-14)

### Story 2.10: Eliminare un workflow

Come integratore,
voglio eliminare un workflow che non serve più,
così che non parte più da nessuna parte senza perdere lo storico.

**Rispetta:** AD-4, AD-5; C-7
**Requisiti:** FR-58 (eliminazione nel motore), FR-31 (lo storico resta)
**Prova per l'utente:** eliminare via API il workflow di esempio mentre ha un'esecuzione lenta in corso: l'esecuzione finisce, un nuovo avvio risponde come per un workflow inesistente, lo storico resta leggibile.
**Docs utente:** eliminare un workflow.

Il controllo «chi usa questo workflow» è un punto unico del dominio: le epiche successive lo estendono con i propri casi (sotto-workflow, pianificazioni, pipeline stream).

**Acceptance Criteria:**

**Dato** un workflow con esecuzioni in corso
**Quando** l'integratore lo elimina via API o SDK Rust
**Allora** le esecuzioni in corso finiscono con la loro versione
**E** lo storico resta leggibile fino alla scadenza della conservazione

**Dato** un workflow eliminato
**Quando** arriva un nuovo avvio o una pubblicazione per quel workflow
**Allora** la risposta è quella di un workflow inesistente

**Dato** un'eliminazione ripetuta o concorrente dello stesso workflow
**Quando** arriva al motore
**Allora** il workflow risulta eliminato una sola volta e la seconda richiesta riceve la risposta di un workflow inesistente

### Story 2.11: Log del tentativo: eventi del motore letti via API

Come integratore,
voglio leggere via API il log di ogni tentativo con gli eventi del motore,
così che vedo cosa è successo a un task senza ricostruirlo dallo storico.

**Rispetta:** AD-2, AD-5; C-9
**Requisiti:** FR-59 (eventi del motore, lettura via API, isolamento), NFR-4, NFR-8; AR-92
**Prova per l'utente:** con la funzione di esempio che fallisce le prime due volte, leggere via API il log di ogni tentativo: i primi due con il fallimento e «ritentata» con l'attesa, il terzo con «completata»; annullando un'esecuzione lenta, il log del tentativo in corso finisce con «annullata».
**Docs utente:** leggere il log di un task.

Quale orologio dà l'ora di righe ed eventi e come si ordinano lo fissa la spec della storia. Le righe scritte dal codice arrivano con le storie 2.12 (worker Rust), 5.1 (worker TypeScript) e 10.2 (nodi); il tempo reale con le storie 3.5 (dashboard) e 6.5 (kit).

**Acceptance Criteria:**

**Dato** un task ritentato
**Quando** si legge via API il log dei suoi tentativi
**Allora** ogni tentativo fallito ha «ricevuta», «in esecuzione» e «fallita» con il motivo, seguiti da «ritentata» con l'attesa
**E** l'ultimo tentativo finisce con «completata»

**Dato** un tentativo fallito per timeout
**Quando** si legge il suo log
**Allora** l'evento «fallita» porta il timeout come motivo

**Dato** un'esecuzione annullata con un tentativo in corso
**Quando** si legge il log di quel tentativo
**Allora** l'ultimo evento è «annullata»

**Dato** il log dei tentativi
**Quando** il benchmark di base conta le istruzioni SQL
**Allora** le istruzioni SQL per attività restano entro NFR-4 con il log attivo; dove vivono gli eventi del motore lo decide S-1

**Dato** il test a due tenant (AD-5)
**Quando** copre la lettura del log
**Allora** un tenant non legge mai il log di un tentativo dell'altro

### Story 2.12: Righe di log dal worker Rust

Come integratore,
voglio che le mie funzioni Rust scrivano righe nel log del tentativo,
così che capisco perché un task è fallito senza cercare nei log dei miei processi.

**Rispetta:** AD-3 (righe fuori dal database), AD-5, AD-7 (cifrate per tenant), AD-17 (archivio dei payload); C-2, C-9, C-11
**Requisiti:** FR-59 (righe del worker Rust, tetto), NFR-2, NFR-4, NFR-7, NFR-8, NFR-14; trasporto delle righe deciso da S-1
**Prova per l'utente:** con la funzione di esempio che scrive una riga e fallisce le prime due volte, leggere via API il log di ogni tentativo: ognuno ha la riga della funzione accanto agli eventi del motore.
**Docs utente:** scrivere righe di log dal worker Rust.

**Acceptance Criteria:**

**Dato** un'attività consegnata a un worker Rust
**Quando** la funzione scrive righe con l'SDK
**Allora** ogni riga è salvata con ora al millesimo, livello (info, avviso, errore), messaggio e tentativo
**E** il log del tentativo la mostra insieme agli eventi del motore (storia 2.11)

**Dato** le righe di un tentativo
**Quando** vengono salvate
**Allora** non passano dal database e stanno, cifrate per tenant, nell'archivio dei payload
**E** il test a due tenant dell'archivio dei payload (AD-5, AD-17) prova che un tenant non legge mai le righe dell'altro

**Dato** un tentativo non più corrente, o una sessione diversa da quella che ha ricevuto il tentativo
**Quando** arriva una riga per quel tentativo
**Allora** è rifiutata e il rifiuto è registrato (C-2)

**Dato** un tentativo che supera il tetto di righe o di byte (NFR-14)
**Quando** la funzione continua a scrivere
**Allora** le righe in più sono scartate e il log dice quante; gli eventi del motore non contano nel tetto
**E** un worker che scrive fino al tetto non porta le altre esecuzioni oltre NFR-2 e lascia le istruzioni SQL per attività entro NFR-4, misurato con il benchmark di base (storia 1.11)

**Dato** una riga che contiene un payload di script o caratteri di controllo
**Quando** il motore la conserva e la restituisce
**Allora** resta testo invariato, e il motore non la scrive mai nei propri log (C-9)

## Epic 3: Dashboard dell'operatore

L'operatore entra con la credenziale del primo avvio e vede le esecuzioni (filtri, tempo in coda,
dettaglio dei task, annullamento) e i worker (funzioni, attività in corso, chi non risponde,
cronologia), da desktop e da telefono.

Prima della storia 3.1: Update di `bmad-ux` sulle 16 voci DA ALLINEARE (AR-7), T-57 e T-59
(AR-118), nomi neutri e temi di `packages/ui` (AR-117), divisione dei componenti tra `packages/ui` e
`packages/react` (il dettaglio dell'esecuzione è condiviso con il kit, AD-16). Ogni pagina è
disegnata prima in Penpot e approvata (skill `dashboard-react`). Ogni storia chiude con gli
screenshot a 1280, 1440, 1920 px e 390/360 px confrontati con le board approvate.

### Story 3.1: Design system di base

Come sviluppatore di topictoast,
voglio i token e i componenti di base del design system in `packages/ui`, controllati contro il documento di design,
così che dashboard e kit partono dagli stessi colori, caratteri e contrasti verificati.

**Rispetta:** AD-15, AD-16; skill `dashboard-react`
**Requisiti:** NFR-9; UX-DR-1…UX-DR-10, UX-DR-25, UX-DR-51, UX-DR-53, UX-DR-54, UX-DR-88…UX-DR-90, UX-DR-94, UX-DR-95; AR-7, AR-117 (nomi neutri), AR-118 (T-57, T-59)
**Prova per l'utente:** aprire Storybook: ogni componente di base compare in tema chiaro e scuro; il gate dei pacchetti passa con il test dei contrasti.
**Docs utente:** nessuna (pacchetto interno; la pagina utente arriva con la cornice della dashboard).

**Acceptance Criteria:**

**Dato** `packages/ui` con i token di DESIGN.md (colori chiari e scuri, tipografia, spaziatura, raggi, icone) nell'unità decisa per T-57
**Quando** gira il gate dei pacchetti
**Allora** un test confronta i token con il frontmatter di DESIGN.md
**E** un test generato dai token verifica la matrice dei contrasti: 4,5:1 per il testo, 3:1 per gli indicatori non testuali, nessuna coppia vietata

**Dato** i componenti di base (`button-secondary`, `skeleton`, `empty-state`, `load-error`, anello di fuoco)
**Quando** si apre Storybook
**Allora** ogni componente ha una storia per stato, in tema chiaro e scuro
**E** l'anello di fuoco è visibile e distinto dal contorno di selezione

**Dato** i nomi dei token e dei temi
**Quando** si leggono nel pacchetto
**Allora** sono neutri, senza riferimenti alla dashboard o a topictoast, come deciso per AR-117

### Story 3.2: Cornice della dashboard

Come operatore,
voglio aprire la dashboard e trovare una cornice chiara con le sezioni, in tema chiaro e scuro,
così che ho un punto fisso da cui controllare l'installazione.

**Rispetta:** AD-15, AD-16; skill `dashboard-react`
**Requisiti:** NFR-9, NFR-16; UX-DR-44, UX-DR-45, UX-DR-83, UX-DR-84, UX-DR-97…UX-DR-105; AR-100 (chi serve i file della dashboard)
**Prova per l'utente:** aprire l'indirizzo della dashboard servito come deciso per AR-100; la barra mostra il marchio e le voci «Worker» ed «Esecuzioni»; il tema segue `prefers-color-scheme`.
**Docs utente:** aprire la dashboard.

**Acceptance Criteria:**

**Dato** la dashboard aperta su desktop
**Quando** si carica
**Allora** mostra la `dashboard-bar` con il marchio, le voci di navigazione come `NavLink` con `aria-current="page"` e il titolo di pagina come intestazione principale
**E** ogni area senza dati mostra `skeleton` mentre carica, `empty-state` se è vuota e `load-error` con «Riprova» se fallisce, senza che la cornice sparisca

**Dato** la dashboard su un telefono largo 360 px
**Quando** si carica
**Allora** è usabile per intero, le voci di navigazione stanno in una fila in fondo e ogni bersaglio con `pointer: coarse` misura almeno `target-min`

**Dato** la CSP `default-src 'self'`
**Quando** la dashboard gira
**Allora** non carica risorse esterne, non ha stili in linea e non istanzia il validatore wasm (AD-15)

**Dato** l'uso da tastiera
**Quando** si attraversa la pagina con `Tab`
**Allora** l'ordine segue la lettura e ogni controllo ha un anello di fuoco visibile
**E** testi e date vengono dai dizionari italiano e inglese e da `Intl`, mai da stringhe nel JSX

**Dato** l'impalcatura di `playwright-bdd` in `dashboard/features/`
**Quando** gira lo scenario di esempio in italiano
**Allora** apre la dashboard servita dal motore reale e controlla la cornice, prima rosso e poi verde

### Story 3.3: Accesso con la credenziale del primo avvio

Come operatore,
voglio entrare nella dashboard con la credenziale generata al primo avvio,
così che solo chi ha accesso all'installazione vede i dati.

**Rispetta:** AD-12 (client API e tipi generati in `packages/core`), AD-16, AD-19 (le risorse dell'operatore non accettano origini del kit); C-7, C-14
**Requisiti:** FR-45, NFR-7, NFR-9, NFR-16; UX-DR-35, UX-DR-46, UX-DR-62, UX-DR-75, UX-DR-111 (accesso); AR-100 (sessione, CSRF), AR-117 (M-44, pagina d'accesso e limite di tentativi)
**Prova per l'utente:** aprire la dashboard senza sessione: compare la pagina d'accesso con la frase su dove è stata scritta la credenziale; incollarla e premere «Accedi»; alla scadenza della sessione si torna all'accesso senza dati.
**Docs utente:** accedere alla dashboard; credenziale smarrita.

**Acceptance Criteria:**

**Dato** nessuna sessione valida
**Quando** l'operatore apre qualunque pagina della dashboard
**Allora** vede solo la pagina d'accesso, e nessuna richiesta di dati parte prima dell'accesso

**Dato** la pagina d'accesso
**Quando** viene mostrata
**Allora** dice dove è stata scritta la credenziale del primo avvio, senza mostrarla
**E** il campo accetta l'incolla e usa `autocomplete="current-password"`

**Dato** una credenziale non valida
**Quando** l'operatore preme «Accedi»
**Allora** vede «Credenziale non valida.» in `role="alert"`
**E** i tentativi ripetuti sono limitati come deciso per M-44

**Dato** una sessione scaduta
**Quando** l'operatore è su una pagina
**Allora** la dashboard torna alla pagina d'accesso senza mostrare dati

**Dato** la credenziale dell'operatore
**Quando** si cercano i log dopo un accesso
**Allora** la credenziale non compare mai (FR-52)

**Dato** `packages/core`
**Quando** nasce con questa storia
**Allora** contiene il client API e i tipi generati una volta dallo schema OpenAPI del motore (AD-12), e la dashboard non dichiara a mano nessun tipo dell'API

### Story 3.4: Elenco delle esecuzioni con filtri e tempo in coda

Come operatore,
voglio vedere le esecuzioni di tutti i tenant, filtrarle e ordinarle per tempo in coda,
così che trovo subito il lavoro fermo.

**Rispetta:** AD-5, AD-12, AD-16; C-17
**Requisiti:** FR-46 (elenco), NFR-7, NFR-9, NFR-16; UX-DR-16, UX-DR-17, UX-DR-36, UX-DR-47, UX-DR-64 (elenco), UX-DR-67, UX-DR-68, UX-DR-86
**Prova per l'utente:** con alcune esecuzioni di esempio, una delle quali in coda perché nessun worker è collegato, aprire «Esecuzioni», filtrare per stato «In coda» e ordinare per «In coda da»: quella esecuzione compare in cima.
**Docs utente:** pagina Esecuzioni.

**Acceptance Criteria:**

**Dato** esecuzioni in vari stati
**Quando** l'operatore apre «Esecuzioni»
**Allora** vede l'elenco con workflow, stato (`state-badge` con icona e parola), origine e colonna «In coda da», a pagine con cursore (C-17)

**Dato** un'esecuzione già «in esecuzione» con un'attività in coda
**Quando** compare nell'elenco
**Allora** il tempo in coda è l'età del più vecchio tra task e attività in coda
**E** un'esecuzione senza task né attività in coda non ha tempo in coda

**Dato** i filtri «Workflow» e «Stato» (compreso «In coda»)
**Quando** l'operatore li usa
**Allora** l'elenco mostra solo le esecuzioni corrispondenti
**E** con il solo tenant predefinito il filtro «Tenant» non compare

**Dato** un nome di workflow o un errore che contiene un payload di script
**Quando** l'elenco lo mostra
**Allora** compare come testo e nessuno script gira (NFR-7)

**Dato** il telefono a 360 px
**Quando** l'elenco viene mostrato
**Allora** le righe diventano card impilate, e le intestazioni ordinabili sono `button` con `aria-sort`

### Story 3.5: Dettaglio di un'esecuzione in tempo reale e annullamento

Come operatore,
voglio aprire un'esecuzione e seguirne i task in tempo reale, con tentativi, input, output ed errori,
così che capisco dove si è fermata e posso annullarla.

**Rispetta:** AD-12, AD-16, AD-18; C-7, C-8
**Requisiti:** FR-46 (dettaglio e annullamento), NFR-7, NFR-9, NFR-16; UX-DR-12, UX-DR-16, UX-DR-23, UX-DR-27, UX-DR-28, UX-DR-29, UX-DR-30, UX-DR-31, UX-DR-41, UX-DR-49 (canale interrotto), UX-DR-69, UX-DR-92; AR-99 (eventi e battito), AR-100 (CSRF), AR-118 (T-68); FR-59 (tab Log), UX-DR-33, UX-DR-70
**Prova per l'utente:** avviare l'esempio con la funzione `sleep` e aprirlo nella dashboard: gli stati dei task cambiano entro 1 secondo; aprire un task mostra tentativi, input e output; «Annulla esecuzione» con conferma lo annulla.
**Docs utente:** dettaglio di un'esecuzione; annullare dalla dashboard.

**Acceptance Criteria:**

**Dato** un'esecuzione in corso
**Quando** l'operatore apre il dettaglio
**Allora** vede i task come `list-view` in ordine topologico, con stato, dipendenze e conteggi del fan-out
**E** ogni cambio di stato appare entro 1 secondo, misurato da capo a capo (AD-18)

**Dato** un task selezionato
**Quando** si apre il pannello
**Allora** mostra una `attempt-card` per tentativo, input e output come `code-block` in sola lettura, e gli identificativi dei worker
**E** non mostra mai il valore di un segreto

**Dato** un task selezionato
**Quando** l'operatore apre la tab «Log»
**Allora** vede il log del tentativo con `log-line` (orario al millesimo, livello, messaggio): eventi del motore e righe del worker
**E** una riga nuova compare entro 1 secondo da quando il codice la scrive, anche a tentativo in corso
**E** l'area del log ha `role="log"`, segue le righe nuove e «Pausa aggiornamenti» la ferma

**Dato** il canale degli eventi interrotto o il motore in ripresa
**Quando** l'operatore guarda il dettaglio
**Allora** valori e stati restano visibili e un avviso dice che gli aggiornamenti sono fermi, come deciso per T-68

**Dato** un'esecuzione in coda o in corso
**Quando** l'operatore preme «Annulla esecuzione» e conferma nel `confirm-dialog`
**Allora** l'esecuzione risulta annullata, con ruolo e ora (FR-10)
**E** il fuoco iniziale del dialogo è su «Chiudi» e `Esc` lo chiude

**Dato** una richiesta di annullamento che arriva da una pagina di un'altra origine
**Quando** raggiunge il motore
**Allora** è rifiutata come deciso per AR-100

### Story 3.6: Grafo dell'esecuzione

Come operatore,
voglio vedere un'esecuzione come grafo, con i fan-out raggruppati e i fallimenti in evidenza,
così che capisco a colpo d'occhio la forma del workflow e dove si è rotto.

**Rispetta:** AD-15, AD-16
**Requisiti:** FR-46 (dettaglio come FR-41), NFR-9, NFR-16; UX-DR-11, UX-DR-13, UX-DR-14, UX-DR-15, UX-DR-18, UX-DR-19, UX-DR-20, UX-DR-21, UX-DR-22, UX-DR-26, UX-DR-91, UX-DR-92; AR-90 (T-58, canvas e disposizione), AR-119 (ricerca tra gli elementi del fan-out, A-5)
**Prova per l'utente:** avviare l'esempio con 300 indicatori e aprirlo nella dashboard: il grafo mostra il fan-out come un solo gruppo «Per ogni indicatore · 300» con la barra dei conteggi; «altri N falliti» apre una tabella cercabile.
**Docs utente:** leggere il grafo di un'esecuzione.

**Acceptance Criteria:**

**Dato** il dettaglio di un'esecuzione
**Quando** l'operatore passa da «Elenco» a «Grafo»
**Allora** vede i task come `node-card` collegati da archi, disposti da sinistra a destra su desktop e dall'alto in basso sul telefono, in sola lettura
**E** il canvas è una regione con nome («Grafo del workflow, 7 task») e ha i comandi «Ingrandisci», «Rimpicciolisci», «Adatta»

**Dato** un fan-out con 300 elementi
**Quando** viene disegnato
**Allora** compare un solo `fanout-group` con nome, conteggio e `status-bar` dei conteggi per stato, mai un ramo per elemento

**Dato** elementi falliti nel fan-out
**Quando** l'operatore apre «altri N falliti»
**Allora** vede una `data-table` cercabile con filtro per stato e il conteggio in `role="status"`

**Dato** task non partiti e rami non presi
**Quando** il grafo li mostra
**Allora** gli archi sono tratteggiati e ogni stato si riconosce da icona e parola, mai solo dal colore

**Dato** un grafo con il numero massimo di task di NFR-14
**Quando** viene disegnato
**Allora** resta usabile entro i criteri di prestazione fissati per T-58

### Story 3.7: Worker, cronologia e avvisi dell'installazione

Come operatore,
voglio vedere i worker collegati, le loro funzioni e attività, chi non risponde e gli avvisi sullo stato del motore,
così che individuo subito un worker caduto o un disco pieno.

**Rispetta:** AD-2, AD-12, AD-16; C-11
**Requisiti:** FR-48, NFR-6, NFR-9, NFR-15, NFR-16; UX-DR-48, UX-DR-49, UX-DR-63, UX-DR-74, UX-DR-110 (worker); AR-98 (fase operativa)
**Prova per l'utente:** con due worker di esempio, uccidere quello che ha attività in corso: la pagina «Worker» lo mostra «Non risponde da … · N attività riassegnate»; riavviando il motore compare l'avviso di ripresa in cima a ogni pagina.
**Docs utente:** pagina Worker; avvisi di ripresa e disco pieno.

**Acceptance Criteria:**

**Dato** worker collegati
**Quando** l'operatore apre «Worker», la pagina iniziale
**Allora** vede per ciascuno le funzioni registrate, le attività in corso con la loro durata e la cronologia recente con le fasi ricevuta, in esecuzione, completata, fallita, ritentata, annullata, la più recente in alto

**Dato** un worker che non risponde
**Quando** la pagina lo mostra
**Allora** compare «Non risponde da …» con il numero di attività riassegnate

**Dato** un worker che registra una funzione già registrata con connessioni diverse
**Quando** si registra
**Allora** la pagina «Worker» lo segnala all'operatore

**Dato** il motore in ripresa dopo un crash, oppure il disco pieno con il lavoro nuovo rifiutato
**Quando** l'operatore è su qualunque pagina
**Allora** in cima compare il `recovery-banner` che distingue i due casi, con il tempo trascorso, non chiudibile a mano
**E** la regione live annuncia solo l'inizio e la fine

**Dato** nessun worker collegato
**Quando** l'operatore apre «Worker»
**Allora** vede «Nessun worker collegato.» e il pulsante «Apri la guida rapida»

## Epic 4: Tenant, identità e quote

L'integratore crea tenant e quote dall'SDK; ogni richiesta porta un JWT firmato dalla piattaforma
ospite con tenant e ruolo; i tenant usano i workflow dell'integratore nel proprio contesto; un
tenant oltre quota non rallenta gli altri; lo storico scade e un tenant eliminato diventa
illeggibile ovunque.

### Story 4.1: Creare e configurare tenant dall'SDK

Come integratore,
voglio creare, rinominare, elencare e cercare i tenant dall'API e dall'SDK Rust,
così che ogni cliente della mia piattaforma ha il suo spazio separato.

**Rispetta:** AD-5; C-1, C-7, C-16, C-17
**Requisiti:** FR-38 (tenant), FR-28 (solo l'integratore crea tenant), NFR-14; AR-108 (tipo del proprietario)
**Prova per l'utente:** con la credenziale dell'integratore, creare il tenant «acme» con l'SDK Rust ed elencare i tenant: compaiono `default` e «acme», ciascuno con il suo identificativo.
**Docs utente:** gestire i tenant.

**Acceptance Criteria:**

**Dato** la credenziale dell'integratore
**Quando** crea un tenant con un nome esterno
**Allora** il motore gli assegna un identificativo interno UUIDv7 e il nome esterno resta solo un'etichetta, mai usata in percorsi, topic o chiavi (AD-5)

**Dato** il nome `default`, un nome già usato o un nome oltre la lunghezza o fuori dai caratteri documentati
**Quando** l'integratore prova a creare il tenant
**Allora** riceve un errore C-7 con il motivo

**Dato** più tenant
**Quando** l'integratore li elenca o li cerca per nome
**Allora** ottiene il risultato a pagine con cursore
**E** la stessa richiesta con una credenziale di tenant è rifiutata

### Story 4.2: Identità garantita dalla piattaforma ospite

Come integratore,
voglio che ogni richiesta porti un JWT firmato dalla mia piattaforma con tenant, ruolo e nome visualizzato facoltativo,
così che topictoast non gestisce utenti né password e si fida solo di quello che firmo io.

**Rispetta:** AD-5, AD-19; C-15 (E-2)
**Requisiti:** FR-27 (motore), NFR-7, NFR-8, NFR-14; AR-96 (chi firma `aud = sdk`)
**Prova per l'utente:** generare una coppia di chiavi Ed25519 con il comando della guida, configurare nel motore la chiave pubblica, firmare un JWT per il tenant «acme» e chiamare l'API: risponde; con `exp` scaduto, un algoritmo diverso da EdDSA o un `aud` sbagliato risponde 401 con il codice.
**Docs utente:** firmare la credenziale per topictoast (campi, durata, chiave pubblica).

**Acceptance Criteria:**

**Dato** il motore configurato con la chiave pubblica e l'`iss` della piattaforma ospite
**Quando** arriva un JWT `EdDSA`/`Ed25519` valido con `tenant`, `role`, `exp` entro la durata massima e l'`aud` giusto per la risorsa
**Allora** la richiesta procede nel contesto di quel tenant e ruolo

**Dato** un JWT con un altro algoritmo, `iss` sbagliato, `aud` non ammesso per la risorsa, `exp` mancante, scaduto o oltre la durata massima
**Quando** arriva
**Allora** è rifiutato, e c'è un test di rifiuto per ogni campo

**Dato** un JWT di ruolo tenant con il campo `selectedTenant`, oppure con `selectedTenant` inesistente, oppure con `displayName` oltre la lunghezza massima o fuori dai caratteri documentati
**Quando** arriva
**Allora** è rifiutato; i tre casi entrano nel test di NFR-8

**Dato** un `displayName` valido
**Quando** viene conservato in un record (per esempio chi ha avviato un'esecuzione)
**Allora** è sempre trattato come testo e resta quanto il record che lo contiene

### Story 4.3: Visibilità per tenant e due ruoli

Come integratore,
voglio che ogni tenant veda e modifichi solo le proprie risorse e che solo il ruolo integratore faccia le operazioni riservate,
così che i miei clienti non si vedono mai tra loro.

**Rispetta:** AD-5, AD-18 (eventi filtrati in un solo punto); C-15
**Requisiti:** FR-26 (completo), FR-28 (motore), FR-10 (annullamento da parte del tenant), NFR-8; AR-109 (autorizzazione per operazione con un test per operazione)
**Prova per l'utente:** con due JWT, uno per «acme» e uno per «globex», avviare un'esecuzione con «acme» e cercarla con «globex»: la risposta è «non trovata», come per un identificativo inesistente.
**Docs utente:** tenant e ruoli.

**Acceptance Criteria:**

**Dato** una risorsa del tenant A
**Quando** il tenant B la legge, la modifica o la annulla
**Allora** la risposta è la stessa di una risorsa inesistente

**Dato** liste, ricerche, conteggi ed eventi in tempo reale
**Quando** li chiede il tenant B
**Allora** non contengono mai risorse di A

**Dato** una credenziale di A con l'identificativo di B in un parametro, nel percorso o nel corpo
**Quando** la richiesta arriva
**Allora** agisce su A o è rifiutata, mai su B

**Dato** un'esecuzione avviata da un tenant, o per suo conto con la credenziale dell'integratore
**Quando** si legge lo storico
**Allora** appartiene a quel tenant ed è visibile solo a lui e all'integratore
**E** il tenant può annullare le proprie esecuzioni

**Dato** una credenziale dell'integratore firmata per il kit
**Quando** chiede un'operazione fuori dal suo ambito (gestione dei tenant, risorse di altri tenant, operazioni dell'operatore)
**Allora** è rifiutata; con `selectedTenant` vale solo per le risorse di quel tenant, l'elenco e la ricerca dei tenant
**E** le storie 4.4 e 4.5 aggiungono i casi dei workflow dell'integratore e delle quote

**Dato** ogni tipo di risorsa e ogni punto di accesso (API, SDK, eventi in tempo reale, liste e conteggi)
**Quando** gira la suite di NFR-8
**Allora** c'è un test che prova che un tenant non legge né modifica le risorse di un altro, e che una credenziale di tenant è rifiutata su ogni operazione dell'integratore

### Story 4.4: Workflow dell'integratore usati dai tenant

Come integratore,
voglio pubblicare workflow disponibili a tutti i tenant e decidere quali funzioni i workflow dei tenant possono chiamare,
così che offro ai clienti automazioni pronte, eseguite nel loro contesto.

**Rispetta:** AD-5; C-7
**Requisiti:** FR-29 (senza connessioni), FR-4 (funzioni rese disponibili ai tenant), FR-26 (risoluzione tra le risorse del proprietario), FR-50 (tenant predefinito come destinazione)
**Prova per l'utente:** pubblicare un workflow come integratore e renderlo disponibile a tutti; avviarlo con il JWT di «acme»: l'esecuzione è di «acme» e «globex» non la vede.
**Docs utente:** workflow dell'integratore e funzioni disponibili ai tenant.

**Acceptance Criteria:**

**Dato** un workflow pubblicato dall'SDK con la credenziale dell'integratore senza tenant
**Quando** viene salvato
**Allora** va al tenant predefinito, e renderlo disponibile a tutti i tenant come workflow dell'integratore è una scelta esplicita
**E** con la credenziale di un tenant va sempre al tenant della credenziale

**Dato** un workflow dell'integratore avviato da un tenant
**Quando** gira
**Allora** gira nel contesto del tenant: storico e visibilità del tenant
**E** un'esecuzione dell'integratore esegue solo workflow dell'integratore

**Dato** un tenant
**Quando** prova a modificare o eliminare un workflow dell'integratore
**Allora** è rifiutato

**Dato** l'elenco delle funzioni che l'integratore rende disponibili ai tenant
**Quando** un tenant invia un documento che chiama una funzione non disponibile
**Allora** è rifiutato come se la funzione non esistesse, senza dettagli del manifest
**E** il controllo si ripete all'esecuzione

**Dato** una credenziale dell'integratore firmata per il kit
**Quando** prova a pubblicare un workflow dell'integratore
**Allora** è rifiutata: con quella credenziale i workflow dell'integratore si possono solo leggere

### Story 4.5: Quote per tenant

Come integratore,
voglio assegnare a ogni tenant quote su attività concorrenti, esecuzioni al minuto, richieste API al secondo, spazio e arretrato,
così che un tenant che esagera non rallenta gli altri e non perde lavoro accettato.

**Rispetta:** AD-2, AD-5; C-7
**Requisiti:** FR-30 (attività, esecuzioni, API, spazio, arretrato), FR-38 (quote), FR-59 (log nello spazio occupato), FR-3 (parallelismo del fan-out limitato dalla quota), NFR-2, NFR-14; AR-93 (dove si controlla lo stato più recente), AR-115 (M-23)
**Prova per l'utente:** dare ad «acme» 2 attività concorrenti e avviare 50 esecuzioni: le attività di «acme» restano in coda con il motivo «quota», mentre quelle di «globex» partono entro gli obiettivi di NFR-2.
**Docs utente:** quote per tenant.

**Acceptance Criteria:**

**Dato** un tenant oltre la quota di attività concorrenti
**Quando** arrivano nuove attività
**Allora** restano in coda con il motivo
**E** le attività degli altri tenant partono entro l'obiettivo di NFR-2, anche se accodate insieme

**Dato** un fan-out senza parallelismo dichiarato
**Quando** gira per un tenant
**Allora** il parallelismo è limitato dalla quota del tenant

**Dato** l'arretrato del tenant (attività ed esecuzioni in coda) al tetto, o la quota di spazio esaurita
**Quando** arriva una nuova richiesta
**Allora** riceve un errore di quota prima di essere accettata
**E** il lavoro già accettato non va perso
**E** le righe dei log dei task contano nello spazio occupato dal tenant (FR-59)

**Dato** un tenant oltre la quota di esecuzioni al minuto o di richieste API al secondo
**Quando** continua a chiamare
**Allora** riceve errori di quota, mentre gli altri tenant continuano entro NFR-2

**Dato** un tenant nuovo
**Quando** viene creato senza quote
**Allora** valgono le quote predefinite documentate, ciascuna con il massimo dell'installazione (NFR-14)

**Dato** un workflow dell'integratore eseguito per un tenant
**Quando** gira
**Allora** consuma le quote del tenant

**Dato** una credenziale di tenant o una credenziale dell'integratore firmata per il kit
**Quando** prova ad assegnare quote
**Allora** è rifiutata

### Story 4.6: Quote nella dashboard

Come operatore,
voglio vedere per ogni tenant l'uso delle quote e chi è vicino al limite o al limite,
così che intervengo prima che un cliente resti bloccato.

**Rispetta:** AD-12, AD-16; skill `dashboard-react`
**Requisiti:** FR-49, NFR-9, NFR-16; UX-DR-47, UX-DR-66, UX-DR-110 (quote)
**Prova per l'utente:** con «acme» oltre la soglia, aprire «Quote»: «acme» compare «Vicino al limite» o «Al limite» con la barra d'uso; gli altri tenant restano normali.
**Docs utente:** pagina Quote.

**Acceptance Criteria:**

**Dato** più tenant
**Quando** l'operatore apre «Quote»
**Allora** vede una riga per tenant con una colonna per quota, ciascuna con la barra d'uso, e gli stati «Vicino al limite» e «Al limite» con icona e parola

**Dato** la soglia di «vicino al limite»
**Quando** non è configurata
**Allora** vale il predefinito documentato (NFR-14)

**Dato** solo il tenant predefinito
**Quando** l'operatore apre «Quote»
**Allora** non compaiono né il nome del tenant né il filtro

**Dato** nessun tenant vicino al limite
**Quando** si attiva il filtro
**Allora** compare «Nessun tenant vicino al limite.»

**Dato** più tenant
**Quando** l'operatore apre «Esecuzioni»
**Allora** compare il filtro «Tenant» e filtra le esecuzioni per tenant

### Story 4.7: Conservazione dello storico

Come integratore,
voglio una durata di conservazione dello storico per ogni tenant,
così che input e output vecchi spariscono da soli.

**Rispetta:** AD-3, AD-5, AD-17
**Requisiti:** FR-31 (conservazione), FR-59 (log nella conservazione), NFR-14; AR-101 (pulizia dell'archivio dei payload insieme al database)
**Prova per l'utente:** impostare per «acme» una conservazione di 1 minuto e avviare un'esecuzione: dopo la scadenza l'esecuzione non si trova più, e i suoi payload non sono più nell'archivio.
**Docs utente:** conservazione dello storico.

**Acceptance Criteria:**

**Dato** un tenant senza durata configurata
**Quando** si legge la sua configurazione
**Allora** vale il predefinito documentato (30 giorni, assunzione del PRD), con il massimo dell'installazione

**Dato** un'esecuzione più vecchia della durata
**Quando** passa la pulizia
**Allora** l'esecuzione è eliminata con i suoi input e output e i log dei task (FR-59), sia dal database sia dall'archivio dei payload
**E** con lei spariscono i nomi visualizzati che conteneva

**Dato** una chiave di idempotenza usata da un'esecuzione ancora conservata
**Quando** viene riusata
**Allora** restituisce l'esecuzione esistente (FR-6)

### Story 4.8: Eliminare un tenant senza lasciare tracce

Come integratore,
voglio eliminare un tenant e avere la garanzia che i suoi dati diventino illeggibili ovunque,
così che rispetto gli obblighi verso un cliente che se ne va.

**Rispetta:** AD-5, AD-7; C-13
**Requisiti:** FR-31 (eliminazione), FR-59 (log del tenant eliminato), FR-27 (nome visualizzato cancellato), NFR-8; AR-96 (riuso del nome esterno)
**Prova per l'utente:** avviare per «acme» un'esecuzione con un valore sentinella, eliminare «acme» e, passato il tempo massimo documentato, cercare il valore nelle directory dati di topictoast e del bus: non si trova.
**Docs utente:** eliminare un tenant (ordine, tempo massimo, residui documentati di Postgres).

**Acceptance Criteria:**

**Dato** un tenant con esecuzioni in corso e connessioni in tempo reale aperte
**Quando** l'integratore lo elimina
**Allora** il motore ferma prima esecuzioni, connessioni in tempo reale e richieste, poi scrive l'evento lapide sul bus, poi cancella la chiave del tenant, poi le righe del database, e infine pulisce bus e archivio in modo asincrono (AD-7)

**Dato** il tempo massimo di eliminazione documentato (NFR-14)
**Quando** è passato
**Allora** la ricerca di un valore sentinella del tenant nel database (file compresi), nel bus e nell'archivio dei payload, log dei task compresi, non trova nulla
**E** con SQLite valgono `secure_delete` e il checkpoint del WAL con `TRUNCATE`; con Postgres il residuo dopo `VACUUM` è documentato

**Dato** la rilettura del bus dopo un riavvio
**Quando** incontra dati di un tenant con lapide
**Allora** li salta
**E** dati indecifrabili senza lapide sono un guasto, non vengono saltati

**Dato** un tenant eliminato
**Quando** si crea un tenant nuovo
**Allora** l'identificativo interno eliminato non viene mai riusato, e il riuso del nome esterno segue la decisione di AR-96

**Dato** un JWT con `selectedTenant` di un tenant eliminato
**Quando** arriva
**Allora** è rifiutato, e il caso entra nel test di NFR-8
**E** i nomi visualizzati del tenant sono cancellati con i suoi dati

### Story 4.9: Credenziali dei worker

Come integratore,
voglio creare una credenziale per ogni worker, limitarla a certe funzioni e tenant e revocarla da sola,
così che un worker compromesso si spegne senza toccare gli altri.

**Rispetta:** AD-5; C-11, C-14
**Requisiti:** FR-12 (solo credenziali di worker valide), NFR-8; AR-96 (prima credenziale di worker)
**Prova per l'utente:** creare dall'SDK una credenziale per un secondo worker limitata a `dns_lookup`: il worker registra `dns_lookup` ma non `censys_search`; revocata la credenziale, il worker viene scollegato e non riceve più attività, mentre l'altro continua.
**Docs utente:** credenziali dei worker.

**Acceptance Criteria:**

**Dato** la credenziale dell'integratore
**Quando** crea una credenziale di worker limitata a funzioni e tenant
**Allora** la credenziale è mostrata una sola volta e il motore ne conserva solo l'impronta
**E** è distinta dalle credenziali dei tenant e dell'integratore

**Dato** un worker con credenziale limitata
**Quando** registra una funzione fuori dal limite
**Allora** la registrazione è rifiutata
**E** non riceve mai attività di tenant fuori dal limite

**Dato** una credenziale di worker revocata
**Quando** il worker è collegato o prova a ricollegarsi
**Allora** viene scollegato e non riceve attività né segreti
**E** le sue attività in corso sono riassegnate come per un worker che non risponde, mentre gli altri worker continuano

**Dato** una credenziale di tenant
**Quando** prova a collegarsi come worker o a registrare una funzione
**Allora** è rifiutata, e il caso entra nel test di NFR-8

## Epic 5: SDK TypeScript, workflow come codice e CLI

L'integratore scrive worker e workflow in TypeScript, definisce workflow come codice anche in Rust,
amministra tutto dall'SDK TypeScript e usa la CLI.

Prima della storia 5.1: livello BDD di SDK TS e CLI (AR-105), distribuzione npm (AR-104),
dipendenze di `packages/sdk` (AR-109), derivazione del manifest dai tipi TypeScript (M-07, AR-113).

### Story 5.1: Worker TypeScript con funzioni tipizzate

Come integratore,
voglio scrivere in TypeScript un worker che registra funzioni tipizzate,
così che porto su topictoast le funzioni che oggi ho in TypeScript.

**Rispetta:** AD-12 (tipi TypeScript generati dallo schema OpenAPI), AD-15, AD-16; C-6, C-11, C-14
**Requisiti:** FR-37 (TypeScript), FR-55, FR-59 (righe dal worker TypeScript), NFR-8; AR-104, AR-105, AR-109, AR-113 (M-07 per TypeScript)
**Prova per l'utente:** avviare il worker TypeScript di esempio con `dns_lookup` e `censys_search` (UJ-1); un'esecuzione del workflow di esempio arriva a completata usando le due funzioni.
**Docs utente:** scrivere un worker in TypeScript.

**Acceptance Criteria:**

**Dato** una funzione TypeScript con tipi di input e di output
**Quando** il worker la registra con `packages/sdk`
**Allora** il manifest è ricavato dai tipi come deciso per M-07
**E** un test di parità prova che la stessa funzione scritta in Rust e in TypeScript produce lo stesso manifest

**Dato** una funzione in esecuzione, con l'SDK TypeScript o Rust
**Quando** riceve un'attività
**Allora** riceve input e contesto dell'esecuzione (proprietario, esecuzione, tentativo), e restituisce un output o un errore; i segreti dichiarati si aggiungono con l'Epic 8
**E** l'SDK espone il proprietario solo dal contesto, mai dall'input, e la documentazione spiega perché

**Dato** un worker TypeScript su un altro host
**Quando** si collega
**Allora** usa TLS e verifica sempre il certificato del motore; in chiaro è rifiutato

**Dato** un worker con concorrenza dichiarata
**Quando** riceve attività
**Allora** valgono le stesse regole di concorrenza, battiti e riconsegna del worker Rust (FR-13, FR-14)

**Dato** una funzione TypeScript in esecuzione
**Quando** scrive righe con `packages/sdk`
**Allora** finiscono nel log del tentativo come quelle del worker Rust (storia 2.12), con lo stesso formato e lo stesso tetto

### Story 5.2: Workflow come codice in TypeScript e Rust

Come integratore,
voglio definire un workflow nel codice TypeScript o Rust e pubblicarlo,
così che i workflow che mantengo io stanno nel mio repository, con revisione e test.

**Rispetta:** AD-4, AD-8, AD-11, AD-12
**Requisiti:** FR-35, FR-36, NFR-7
**Prova per l'utente:** pubblicare il workflow di esempio definito in TypeScript e lo stesso definito in Rust; l'API restituisce per entrambi lo stesso documento OWS, segnato «da codice».
**Docs utente:** workflow come codice.

**Acceptance Criteria:**

**Dato** un workflow definito nel codice TypeScript o Rust
**Quando** l'integratore lo pubblica
**Allora** il motore riceve un documento OWS equivalente, valido secondo lo schema di topictoast (sottoinsieme OWS più `dag`), leggibile ed esportabile

**Dato** lo stesso workflow scritto in TypeScript e in Rust
**Quando** entrambi producono il documento
**Allora** i due documenti sono uguali nella forma canonica (AD-4)

**Dato** un workflow pubblicato come codice
**Quando** lo si legge via API
**Allora** porta la provenienza «da codice»
**E** una modifica che non arriva dall'SDK con la provenienza «da codice» è rifiutata: si modifica solo dal codice

**Dato** un workflow nel codice che usa un costrutto fuori dal sottoinsieme
**Quando** viene pubblicato
**Allora** riceve gli stessi errori di validazione di un documento inviato a mano

### Story 5.3: Amministrazione completa dall'SDK TypeScript

Come integratore,
voglio fare dall'SDK TypeScript tutto quello che si fa dall'API,
così che automatizzo la gestione di tenant, quote e workflow dalla mia piattaforma in TypeScript.

**Rispetta:** AD-12, AD-16; C-7, C-16, C-17, C-18
**Requisiti:** FR-38 (parità dell'SDK TypeScript), NFR-8
**Prova per l'utente:** con lo script di esempio in TypeScript creare un tenant, assegnargli una quota e avviare un'esecuzione; l'elenco delle operazioni dell'API e quello dell'SDK coincidono.
**Docs utente:** riferimento dell'SDK TypeScript.

**Acceptance Criteria:**

**Dato** le operazioni pubbliche dell'API
**Quando** gira il test di parità
**Allora** ogni operazione ha il suo metodo nell'SDK TypeScript e nell'SDK Rust, e un'operazione senza metodo fa fallire il test
**E** ogni epica successiva aggiunge i propri metodi ai due SDK, perché il test lo impone

**Dato** un errore del motore
**Quando** arriva all'SDK TypeScript
**Allora** l'SDK lo espone con il codice C-7, i parametri e il percorso, mai come eccezione generica

**Dato** liste lunghe
**Quando** l'SDK le legge
**Allora** segue il cursore (C-17)
**E** gli avvii portano la chiave di idempotenza (C-18)

### Story 5.4: CLI di topictoast

Come integratore,
voglio una riga di comando per pubblicare workflow, avviare esecuzioni e gestire i tenant,
così che uso topictoast dagli script e dal terminale senza scrivere codice.

**Rispetta:** AD-16 (la CLI usa `crates/sdk`, client HTTP in un solo modulo); C-7, C-14
**Requisiti:** FR-38, NFR-8; AR-105 (livello BDD della CLI)
**Prova per l'utente:** con la CLI pubblicare il workflow di esempio, avviarlo e leggerne l'esito; con un input non valido la CLI stampa codice, percorso e regola, ed esce con un codice diverso da zero.
**Docs utente:** riferimento della CLI.

**Acceptance Criteria:**

**Dato** la CLI configurata con l'indirizzo e la credenziale del motore
**Quando** l'integratore pubblica un workflow, avvia un'esecuzione, ne legge l'esito o gestisce i tenant
**Allora** ottiene lo stesso risultato dell'SDK Rust su cui la CLI è costruita
**E** le epiche successive aggiungono i propri comandi (segreti, nodi personalizzati, pipeline stream)

**Dato** un errore del motore
**Quando** la CLI lo riceve
**Allora** stampa codice, percorso e messaggio ed esce con un codice diverso da zero

**Dato** un indirizzo fuori da localhost
**Quando** la CLI si collega
**Allora** usa TLS e verifica sempre il certificato
**E** la credenziale non compare mai nell'output né nei log

## Epic 6: Kit: seguire e lanciare i workflow

Lo sviluppatore della piattaforma ospite incorpora l'area workflow; il tenant vede l'elenco dei
workflow, li lancia con un modulo generato dallo schema di input e segue le esecuzioni in tempo
reale; colori, testi e lingua si adattano alla piattaforma ospite.

Prima della storia 6.1: ADR di S-4, skill del kit scritta (AD-15), contratto pubblico del kit
(AR-117: credenziale scaduta, eventi verso l'ospite, idempotenza dei lanci, dizionario dei codici,
cambio di tenant, nomi neutri, `peerDependencies` di React), distribuzione npm e piattaforme del
validatore nativo (AR-104). I componenti condivisi con la dashboard stanno già in `packages/ui`
(Epic 3).

### Story 6.1: Validare i workflow nel browser

Come sviluppatore della piattaforma ospite,
voglio validare un workflow nella mia pagina con il nucleo JS, senza React, e ottenere gli stessi errori del motore,
così che i miei utenti vedono gli errori prima di inviare.

**Rispetta:** AD-11 (un solo validatore, `wasm32-wasip1` senza thread nel browser), AD-15, AD-16, AD-19 (`.wasm` dall'origine dell'ospite, CSP); C-7
**Requisiti:** FR-39 (validazione), NFR-8; UX-DR-106; AR-104, AR-107, AR-117
**Prova per l'utente:** aprire la pagina d'esempio in HTML semplice che usa solo `packages/core`: un documento con un errore mostra percorso e codice, identici a quelli che restituisce il motore, su Chromium, Firefox e Safari.
**Docs utente:** validare con il nucleo JS.

**Acceptance Criteria:**

**Dato** un insieme di documenti di prova
**Quando** vengono validati dal motore e dal nucleo JS con lo stesso contesto di validazione (manifest e funzioni disponibili; i segreti entrano nel contesto con l'Epic 8)
**Allora** gli esiti sono identici, codici e percorsi compresi
**E** se il contesto cambia tra le due validazioni, vale l'esito del motore

**Dato** una pagina ospite senza isolamento cross-origin, con CSP che consente `connect-src` verso il motore e `'wasm-unsafe-eval'`
**Quando** il nucleo carica il validatore
**Allora** lo carica solo quando serve, dall'origine dell'ospite, e funziona su Chromium, Firefox e Safari, anche su telefono
**E** con `'wasm-unsafe-eval'` negato si comporta come deciso nell'ADR di S-4

### Story 6.2: Inviare e seguire le esecuzioni dal nucleo JS

Come sviluppatore della piattaforma ospite,
voglio inviare un workflow e seguirne le esecuzioni in tempo reale dal nucleo JS,
così che integro topictoast anche in un'interfaccia che non usa React.

**Rispetta:** AD-12, AD-16, AD-18, AD-19 (CORS, SSE via `fetch` con `Authorization`); C-7, C-8, C-15
**Requisiti:** FR-39 (invio ed esecuzioni), FR-27 (avvisi di scadenza verso l'ospite), NFR-8; AR-117
**Prova per l'utente:** dalla stessa pagina d'esempio, inviare il documento corretto e seguire l'esecuzione fino a «completata»; interrompendo la rete per qualche secondo, gli aggiornamenti riprendono da dove si erano fermati.
**Docs utente:** inviare e seguire con il nucleo JS.

**Acceptance Criteria:**

**Dato** il nucleo collegato al motore
**Quando** invia un workflow e segue un'esecuzione
**Allora** riceve gli eventi in tempo reale via `fetch` con `Authorization`, mai con il token nell'URL o in un cookie
**E** dopo un'interruzione riprende dalla posizione (C-8)

**Dato** la credenziale firmata che sta per scadere
**Quando** mancano almeno 60 secondi alla scadenza, e poi alla scadenza
**Allora** il nucleo manda all'ospite gli eventi previsti dal contratto del kit

**Dato** il motore configurato con le origini consentite
**Quando** il nucleo chiama da un'origine non consentita
**Allora** la richiesta è rifiutata da CORS

### Story 6.3: Area workflow ed elenco dei workflow

Come sviluppatore della piattaforma ospite,
voglio incorporare un solo componente, l'area workflow, con la sua navigazione interna e l'elenco dei workflow,
così che i miei tenant trovano i loro workflow dentro la mia app senza un secondo login.

**Rispetta:** AD-5, AD-15, AD-16, AD-19; skill del kit
**Requisiti:** FR-58, FR-27 (nessun secondo login), FR-26, FR-29, NFR-7, NFR-9, NFR-16; UX-DR-42, UX-DR-47, UX-DR-53, UX-DR-55, UX-DR-56, UX-DR-57, UX-DR-93, UX-DR-107 (UJ-1)
**Prova per l'utente:** nell'app d'esempio in React, incorporare l'area workflow con la credenziale firmata di «acme»: compare l'elenco con i workflow di «acme» e quelli dell'integratore, ciascuno con la provenienza; eliminare un workflow di «acme» chiede conferma.
**Docs utente:** incorporare l'area workflow.

**Acceptance Criteria:**

**Dato** l'area workflow incorporata con una credenziale valida
**Quando** si apre
**Allora** mostra la barra delle sezioni con etichette sempre visibili e l'elenco dei workflow del tenant e dell'integratore, con il chip della provenienza («Dell'integratore», «da codice»)
**E** non chiede un secondo login e non usa `main`; le regioni hanno un nome e c'è il link «Salta al canvas»

**Dato** l'elenco
**Quando** il tenant apre un workflow
**Allora** lo vede come grafo o elenco in sola lettura finché l'editor (Epic 12) non c'è

**Dato** un workflow del tenant
**Quando** il tenant preme «Elimina» e conferma nel `confirm-dialog`
**Allora** il kit usa l'eliminazione del motore (storia 2.10): le esecuzioni in corso finiscono e lo storico resta

**Dato** nessun workflow salvato
**Quando** l'elenco è vuoto
**Allora** mostra «Nessun workflow salvato.»; il pulsante «Nuovo workflow» compare solo con l'editor (Epic 12)

**Dato** un workflow dell'integratore
**Quando** il tenant guarda l'elenco
**Allora** non può eliminarlo; l'integratore elimina i propri dall'SDK

**Dato** ogni sezione dell'area
**Quando** lo sviluppatore dell'ospite la incorpora da sola
**Allora** funziona anche come componente singolo

**Dato** solo il tenant predefinito
**Quando** l'area si apre
**Allora** non mostra la scelta del tenant
**E** nessun componente mostra il nome o il logo di topictoast

### Story 6.4: Lanciare un workflow con il modulo generato dallo schema

Come tenant,
voglio lanciare un workflow compilando un modulo generato dal suo schema di input,
così che avvio un'analisi senza scrivere JSON.

**Rispetta:** AD-11, AD-12; C-7, C-18
**Requisiti:** FR-41 (lancio e idempotenza), FR-56, FR-30 (quota accanto a «Esegui»), NFR-7, NFR-9, NFR-16; UX-DR-23, UX-DR-35, UX-DR-39, UX-DR-40, UX-DR-71, UX-DR-73
**Prova per l'utente:** nel kit, aprire il workflow d'esempio con un fan-out sugli indicatori, pubblicato dall'SDK, e premere «Esegui»: il modulo chiede gli indicatori; incollarne 300, uno per riga, mostra «300 elementi»; il lancio porta alla vista dell'esecuzione in «In coda».
**Docs utente:** lanciare un workflow dal kit.

**Acceptance Criteria:**

**Dato** un workflow con schema di input
**Quando** il tenant preme «Esegui»
**Allora** si apre un modulo con un campo per ogni campo dello schema, con aiuti che riportano predefiniti e massimi dell'installazione
**E** «Mostra JSON» passa al modo avanzato

**Dato** un campo elenco
**Quando** il tenant incolla una riga per elemento
**Allora** il conteggio compare in `role="status"` e un elemento sbagliato dà un errore come «Riga 12: atteso un numero»
**E** «Esegui» resta bloccato finché ci sono errori

**Dato** un workflow senza schema
**Quando** il tenant preme «Esegui»
**Allora** si apre direttamente il JSON

**Dato** un lancio ripetuto dopo un errore di rete o un rinnovo della credenziale
**Quando** arriva al motore
**Allora** porta la stessa chiave di idempotenza dell'intenzione di lancio e restituisce la stessa esecuzione

**Dato** il tenant oltre quota o oltre l'arretrato
**Quando** lancia
**Allora** il rifiuto compare accanto a «Esegui» con il motivo

**Dato** un workflow dell'integratore o definito come codice
**Quando** il tenant lo apre
**Allora** è in sola lettura ma «Esegui» resta attivo

### Story 6.5: Seguire un'esecuzione in tempo reale nel kit

Come tenant,
voglio seguire nel kit un'esecuzione in tempo reale, con tentativi, input, output ed errori,
così che vedo il mio workflow prendere vita e capisco subito cosa è fallito.

**Rispetta:** AD-15, AD-16, AD-18; C-7, C-8
**Requisiti:** FR-41 (vista delle esecuzioni), NFR-7, NFR-9, NFR-16; UX-DR-50, UX-DR-59, UX-DR-69, UX-DR-74, UX-DR-86, UX-DR-108 (climax di UJ-2); FR-59 (tab Log, righe dei worker comprese), UX-DR-33, UX-DR-70
**Prova per l'utente:** lanciare il workflow d'esempio con un fan-out di 300 indicatori, pubblicato dall'SDK, e seguirlo nel kit: i conteggi del fan-out salgono entro 1 secondo da ogni cambio; un elemento fallito mostra tentativi ed errore; con il ruolo tenant non compaiono identificativi dei worker.
**Docs utente:** seguire un'esecuzione nel kit.

**Acceptance Criteria:**

**Dato** un'esecuzione in corso
**Quando** il tenant la apre nel kit
**Allora** vede lo stato di ogni task, le attività del fan-out raggruppate con i conteggi per stato, tentativi, input, output ed errori
**E** ogni cambio di stato appare entro 1 secondo

**Dato** un fan-out da migliaia di elementi
**Quando** la vista lo mostra
**Allora** resta leggibile come un gruppo con i conteggi, mai un ramo per elemento

**Dato** il ruolo tenant
**Quando** guarda un'attività
**Allora** non vede gli identificativi dei worker

**Dato** un task selezionato
**Quando** il tenant apre la tab «Log»
**Allora** vede gli eventi del motore e le righe del worker, come testo (NFR-7), e una riga nuova compare entro 1 secondo da quando il codice la scrive
**E** gli eventi non mostrano identificativi dei worker, e il tenant non vede mai il log di un tentativo di un altro tenant

**Dato** il motore in ripresa dopo un crash o il canale degli eventi interrotto
**Quando** il tenant guarda la vista
**Allora** un avviso neutro dice che gli aggiornamenti sono in pausa e riprendono da soli
**E** stati e righe restano visibili e nessun testo nomina il motore o il crash

**Dato** un contenuto del tenant con un payload di script (nome, errore, input, output)
**Quando** la vista lo mostra
**Allora** compare come testo e nessuno script gira (NFR-7)

### Story 6.6: Credenziale, scadenza e cambio di tenant nel kit

Come sviluppatore della piattaforma ospite,
voglio che il kit gestisca da solo la scadenza della credenziale e il cambio di tenant,
così che un utente non perde lavoro e non vede mai dati del tenant sbagliato.

**Rispetta:** AD-5, AD-19; C-15
**Requisiti:** FR-27 (kit), FR-28 (kit con credenziale dell'integratore), NFR-8; UX-DR-43, UX-DR-75
**Prova per l'utente:** nell'app d'esempio, con una credenziale dell'integratore e due tenant, cambiare tenant con il selettore: l'area mostra solo le risorse del nuovo tenant; lasciando scadere la credenziale, l'ospite riceve l'avviso prima della scadenza.
**Docs utente:** credenziale del kit, rinnovo e cambio di tenant.

**Acceptance Criteria:**

**Dato** una credenziale dell'integratore con `selectedTenant`
**Quando** l'area si apre
**Allora** ogni sezione mostra solo le risorse di quel tenant e i workflow dell'integratore, ed «Esegui» lancia nel suo contesto
**E** il selettore dei tenant compare solo con il ruolo integratore e più di un tenant

**Dato** un cambio di tenant
**Quando** l'ospite fornisce la credenziale nuova
**Allora** il kit scarta lo stato del tenant precedente, eventi in tempo reale compresi; il caso entra nel test di NFR-8
**E** con modifiche non salvate chiede prima conferma

**Dato** lavoro non salvato al momento della scadenza
**Quando** arriva la credenziale rinnovata
**Allora** il lavoro torna solo se la credenziale nuova ha lo stesso tenant e lo stesso ruolo, altrimenti è scartato senza mostrarlo
**E** resta solo nella memoria della pagina, mai salvato nel browser

**Dato** il motore
**Quando** la credenziale scade mentre uno stream di eventi è aperto
**Allora** il motore lo chiude e il kit lo riapre con la credenziale nuova

### Story 6.7: Aspetto personalizzabile dalla piattaforma ospite

Come sviluppatore della piattaforma ospite,
voglio adattare colori, caratteri, testi e lingua dei componenti,
così che il kit sembra parte della mia applicazione.

**Rispetta:** AD-15; skill del kit
**Requisiti:** FR-44, NFR-9; UX-DR-85, UX-DR-87, UX-DR-88, UX-DR-90; AR-117 (nomi neutri, dizionario)
**Prova per l'utente:** nell'app d'esempio passare al kit i colori e i testi della piattaforma: il kit li usa; una coppia di colori sotto il contrasto dà un errore in sviluppo e un avviso in console in produzione; nel bundle non c'è la parola «topictoast».
**Docs utente:** personalizzare il kit.

**Acceptance Criteria:**

**Dato** colori, caratteri, testi e lingua forniti dall'ospite
**Quando** il kit si monta
**Allora** li usa al posto dei predefiniti neutri

**Dato** un testo mancante o vuoto
**Quando** il kit lo mostra
**Allora** ricade sul predefinito della lingua; la v1 include italiano e inglese

**Dato** una coppia di colori dell'ospite sotto la soglia di contrasto di NFR-9
**Quando** il kit si monta
**Allora** segnala un errore in sviluppo e un avviso in console in produzione

**Dato** il bundle del kit
**Quando** gira il controllo nel gate
**Allora** la parola «topictoast» non compare in stringhe, icone, file o titoli, salvo l'opzione esplicita
**E** nessuna proprietà nasconde le icone o le parole degli stati

## Epic 7: Linguaggio OWS completo e chiamate HTTP

Integratore e tenant usano tutto il sottoinsieme OWS della v1: `switch`, `fork`, `for`, `try` con
`catch`, `wait`, `raise`, sotto-workflow e `call http` verso le sole destinazioni consentite.

### Story 7.1: Scelte e cicli con `switch` e `for`

Come integratore,
voglio scegliere un ramo con `switch` e ripetere dei task con `for`,
così che i workflow seguono la logica del mio processo.

**Rispetta:** AD-8, AD-9, AD-11; C-7
**Requisiti:** FR-1 (`switch`, `for`), FR-4 (costrutti), NFR-14
**Prova per l'utente:** pubblicare l'esempio con uno `switch` su tre rami e un `for` su tre elementi; un'esecuzione con l'input del secondo ramo mostra solo quel ramo eseguito e il ciclo ripetuto tre volte, in ordine.
**Docs utente:** `switch` e `for`.

**Acceptance Criteria:**

**Dato** uno `switch` con più casi e un caso predefinito
**Quando** l'esecuzione lo raggiunge
**Allora** esegue il primo caso la cui condizione è vera, altrimenti il predefinito, con la semantica di OWS
**E** i rami non presi risultano non eseguiti, con il motivo

**Dato** uno `switch` dentro un `do` annidato in un task `dag`
**Quando** il documento viene inviato
**Allora** è accettato come un solo nodo del `dag`
**E** la funzione del dominio che riconosce questa forma è esposta anche da `packages/validator` (AD-8)

**Dato** un `for` su un elenco
**Quando** l'esecuzione lo raggiunge
**Allora** ripete i task per ogni elemento in ordine, con l'elemento e l'indice disponibili alle espressioni

**Dato** un `for` su un elenco oltre il limite di NFR-14
**Quando** l'esecuzione lo raggiunge
**Allora** il task fallisce con limite e valore, senza creare attività

### Story 7.2: Rami paralleli con `fork`

Come integratore,
voglio eseguire più rami in parallelo con `fork`, anche in gara tra loro,
così che chiamo più servizi insieme e uso il primo che risponde quando serve.

**Rispetta:** AD-9; C-2
**Requisiti:** FR-1 (`fork`), FR-4 (costrutti)
**Prova per l'utente:** l'esempio con un `fork` di due rami lenti finisce nel tempo del più lento; con `compete: true` finisce con il più veloce e l'altro ramo risulta annullato.
**Docs utente:** `fork`.

**Acceptance Criteria:**

**Dato** un `fork` con più rami e `compete: false`
**Quando** l'esecuzione lo raggiunge
**Allora** i rami partono in parallelo e il `fork` termina quando sono finiti tutti, con le uscite raccolte come dice OWS

**Dato** un `fork` con `compete: true`
**Quando** un ramo finisce per primo
**Allora** la sua uscita è l'uscita del `fork` e gli altri rami ricevono la richiesta di annullamento

**Dato** un ramo che fallisce
**Quando** il `fork` lo attende
**Allora** il `fork` fallisce con l'errore del ramo, salvo un `try` che lo gestisca

### Story 7.3: Gestire gli errori con `try` e `raise`

Come integratore,
voglio intercettare gli errori con `try` e `catch` e sollevarne di miei con `raise`,
così che decido io cosa fare quando un passo fallisce.

**Rispetta:** AD-9; C-7
**Requisiti:** FR-1 (`try` con `catch`, `raise`), FR-4 (costrutti)
**Prova per l'utente:** l'esempio con una funzione che fallisce dentro un `try`: il `catch` filtra l'errore ed esegue i suoi task, e l'esecuzione arriva a completata; un `raise` con un errore dichiarato fa fallire l'esecuzione con quell'errore.
**Docs utente:** gestire gli errori (`try`, `catch`, `raise`).

**Acceptance Criteria:**

**Dato** un `try` con un `catch` che filtra per tipo o stato dell'errore (`errors.with`, `when`, `exceptWhen`)
**Quando** un task dentro il `try` fallisce con un errore che corrisponde
**Allora** il `catch` esegue il suo `do`, con l'errore disponibile sotto il nome dichiarato in `as`

**Dato** un errore che non corrisponde al filtro
**Quando** arriva al `try`
**Allora** si propaga come se il `try` non ci fosse

**Dato** un `raise` con un errore OWS (tipo, stato, titolo, dettaglio)
**Quando** l'esecuzione lo raggiunge
**Allora** il task fallisce con quell'errore, che resta leggibile nello storico

### Story 7.4: Attese durevoli con `wait`

Come integratore,
voglio mettere in pausa un workflow per un tempo dichiarato,
così che aspetto un servizio esterno senza tenere occupato un worker.

**Rispetta:** AD-2
**Requisiti:** FR-1 (`wait`), FR-9, NFR-6, NFR-14
**Prova per l'utente:** avviare l'esempio con `wait` di 30 secondi e uccidere il motore a metà attesa; dopo la ripresa il task successivo parte allo scadere dei 30 secondi, né prima né dopo il dovuto.
**Docs utente:** `wait`.

**Acceptance Criteria:**

**Dato** un `wait` con una durata ISO 8601
**Quando** l'esecuzione lo raggiunge
**Allora** il task successivo parte allo scadere della durata e nessun worker resta occupato nel frattempo

**Dato** un crash del motore durante l'attesa
**Quando** il motore riprende
**Allora** l'attesa finisce all'istante previsto in origine e non si perde

**Dato** una durata oltre il massimo dell'installazione
**Quando** il documento viene inviato
**Allora** è rifiutato con limite e valore (NFR-14)

### Story 7.5: Sotto-workflow

Come integratore,
voglio chiamare un workflow da un altro passandogli un input,
così che riuso i workflow come mattoni.

**Rispetta:** AD-5, AD-10; C-7
**Requisiti:** FR-1 (sotto-workflow), FR-4 (input sullo schema del workflow chiamato), FR-26 (risoluzione tra le risorse del proprietario), FR-29 (nomi uguali di un tenant), FR-56, NFR-14
**Prova per l'utente:** pubblicare un workflow «arricchisci» con il suo schema di input e un workflow che lo chiama; l'esecuzione mostra il sotto-workflow come figlio, con il suo esito.
**Docs utente:** sotto-workflow.

**Acceptance Criteria:**

**Dato** un task che chiama un sotto-workflow
**Quando** il documento viene inviato
**Allora** l'input passato è controllato sullo schema di input del workflow chiamato, con lo stesso esito dell'avvio diretto

**Dato** il sotto-workflow in esecuzione
**Quando** termina
**Allora** la sua uscita diventa l'uscita del task chiamante, e un suo fallimento fa fallire il task chiamante

**Dato** un workflow dell'integratore che chiama un sotto-workflow, eseguito per un tenant che ha un proprio workflow con lo stesso nome
**Quando** gira
**Allora** chiama il sotto-workflow dell'integratore, mai quello del tenant (FR-29)
**E** i sotto-workflow citati da un tenant si risolvono solo tra le sue risorse e quelle rese disponibili dall'integratore

**Dato** un workflow usato come sotto-workflow da un altro
**Quando** si prova a eliminarlo
**Allora** è rifiutato con l'elenco dei workflow che lo usano (storia 2.10)
**E** nel kit l'eliminazione rifiutata mostra l'elenco di chi lo usa (storia 6.3)

**Dato** un cambio dello schema di input di un sotto-workflow dopo la pubblicazione del workflow che lo chiama
**Quando** partono esecuzioni nuove o riprendono quelle in coda
**Allora** vale la regola fissata per M-09 (storia 2.7)

**Dato** sotto-workflow annidati o ricorsivi oltre la profondità massima di NFR-14
**Quando** il documento viene inviato o l'esecuzione li raggiunge
**Allora** l'errore indica limite e valore, senza creare attività

### Story 7.6: `call http` verso le sole destinazioni consentite

Come integratore,
voglio che i workflow chiamino servizi HTTP esterni solo verso le destinazioni che consento a ciascun tenant,
così che un workflow non raggiunge mai la rete interna dell'installazione.

**Rispetta:** AD-6 (`call http` in un processo ausiliario, senza cookie né pool condivisi), AD-16 (client HTTP in un solo modulo di `crates/adapters`); C-7
**Requisiti:** FR-1 (`call http`), FR-16 (filtro di rete di `call http`), NFR-8, NFR-14; AR-86 (risorse esterne del documento), AR-116 (M-30, filtro di rete)
**Prova per l'utente:** consentire al tenant «acme» la destinazione `api.example.com`; un `call http` verso di essa risponde; verso `127.0.0.1` o `169.254.169.254` fallisce con l'errore di politica di rete, senza aprire la connessione.
**Docs utente:** `call http` e destinazioni consentite.

**Acceptance Criteria:**

**Dato** le destinazioni che l'integratore consente a un tenant
**Quando** un `call http` del tenant chiama una destinazione consentita
**Allora** la chiamata parte e la risposta diventa l'uscita del task

**Dato** un `call http` verso loopback, indirizzi privati, link-local o indirizzi dell'installazione (motore, bus, database), anche quando un nome consentito vi si risolve o un redirect vi porta
**Quando** parte la chiamata
**Allora** fallisce con un errore di politica di rete senza aprire la connessione
**E** i casi `127.0.0.1`, `169.254.169.254` e un nome che si risolve a un indirizzo privato entrano nel test di NFR-8

**Dato** un redirect verso un'altra origine
**Quando** la chiamata lo segue
**Allora** l'intestazione `Authorization` non viene inoltrata, e non esiste un archivio di cookie

**Dato** una risposta oltre la dimensione massima di NFR-14
**Quando** arriva
**Allora** il task fallisce con dimensione e limite

**Dato** un documento con risorse esterne (`schema.resource`, `use.catalogs`, definizioni da un URL)
**Quando** viene inviato
**Allora** è rifiutato alla validazione senza aprire connessioni di rete, come fissato da AD-20 (M-02)

## Epic 8: Segreti e connessioni

Il tenant salva segreti legati a una connessione; l'attività li riceve solo alla consegna; non
compaiono mai in log, storico o errori; l'integratore ruota la chiave senza fermare nulla.

### Story 8.1: Salvare un segreto per una connessione

Come tenant,
voglio salvare una chiave API con un nome e la connessione che soddisfa,
così che i miei workflow la usano senza che nessuno possa più leggerla.

**Rispetta:** AD-5, AD-7; C-12, C-17
**Requisiti:** FR-32, FR-38 (segreti via SDK), NFR-8, NFR-14; AR-118 (conteggio dei workflow che usano un segreto)
**Prova per l'utente:** con il JWT di «acme», salvare dall'SDK il segreto «censys-prod» per la connessione Censys; l'elenco mostra nome, connessione e data, mai il valore; sostituirne il valore non richiede di toccare i workflow.
**Docs utente:** salvare e gestire i segreti.

**Acceptance Criteria:**

**Dato** un tenant
**Quando** salva un segreto con nome, connessione e valore via API o SDK
**Allora** il valore è cifrato nella busta con la chiave del tenant (AD-7)
**E** da quel momento nessuna API, SDK o dashboard lo restituisce: si vedono solo nome, connessione, data e ultimo uso

**Dato** l'elenco delle connessioni tra cui il tenant sceglie
**Quando** lo chiede
**Allora** contiene solo le connessioni dei manifest a lui disponibili, e l'elenco entra nel test di NFR-8

**Dato** un segreto salvato
**Quando** il tenant ne sostituisce il valore o lo elimina via API o SDK
**Allora** la sostituzione non richiede di modificare i workflow, e un segreto eliminato non compare più negli elenchi

**Dato** un tenant
**Quando** elenca o legge i segreti di un altro tenant
**Allora** la risposta è la stessa di un segreto inesistente

**Dato** un nome già usato, troppo lungo o un valore oltre la dimensione massima
**Quando** il tenant salva
**Allora** riceve un errore C-7 (NFR-14)

### Story 8.2: Collegare i segreti ai task e controllarli all'invio

Come tenant,
voglio scegliere per ogni connessione di un task il segreto da usare, e sapere all'invio se manca qualcosa,
così che un workflow non parte mai senza le credenziali che servono.

**Rispetta:** AD-4, AD-8 (`$secrets` vietato nei documenti), AD-11; C-7
**Requisiti:** FR-4 (connessioni e segreti all'invio), FR-32 (segreto per un task senza manifest), FR-33 (connessioni fissate nella versione), NFR-8; AR-86 (forma nel documento), AR-89 (M-37)
**Prova per l'utente:** pubblicare con il JWT di «acme» un workflow che chiama `censys_search` senza scegliere il segreto: l'errore nomina task e connessione; scegliendo «censys-prod» il workflow viene accettato.
**Docs utente:** collegare i segreti ai task.

**Acceptance Criteria:**

**Dato** il workflow di un tenant con una connessione richiesta dal manifest senza segreto scelto
**Quando** viene inviato
**Allora** è rifiutato con il task e la connessione nel messaggio

**Dato** un segreto legato a una connessione che non soddisfa quella del manifest, o in più rispetto alle connessioni del manifest
**Quando** il workflow viene inviato
**Allora** è rifiutato con task, segreto e connessione

**Dato** un task senza manifest, come `call http`
**Quando** il tenant gli assegna un segreto
**Allora** il segreto è legato a quel task nella versione pubblicata

**Dato** una versione pubblicata
**Quando** viene salvata
**Allora** le connessioni di ogni task sono fissate in quella versione, nella forma decisa da AD-20 e dalla spec di M-37, e controllate sui manifest

### Story 8.3: Consegna dei segreti al momento dell'esecuzione

Come integratore,
voglio che worker e sandbox ricevano i segreti solo insieme all'attività che li dichiara,
così che un segreto non resta mai in giro e non finisce in log o storico.

**Rispetta:** AD-2 (segreti mai sul bus), AD-6, AD-7; C-9, C-12
**Requisiti:** FR-33, FR-59 (segreti nelle righe di log), NFR-6, NFR-8; AR-110 (consegne con segreti a database fermo), AR-118 (T-67, segreti nei log del codice del tenant)
**Prova per l'utente:** eseguire il workflow di «acme» con il segreto sentinella; il worker di esempio lo riceve; cercando il valore sentinella in log, storico, output, database, bus e archivio dei payload non si trova.
**Docs utente:** come i worker ricevono i segreti.

**Acceptance Criteria:**

**Dato** un'attività il cui task dichiara dei segreti
**Quando** viene consegnata
**Allora** il worker riceve solo i segreti dichiarati che appartengono al proprietario dell'esecuzione, risolti al momento della consegna e mai scritti sul bus

**Dato** un'esecuzione completata con un segreto sentinella
**Quando** si cercano i valori in log, storico, output, errori, database, bus e archivio dei payload
**Allora** non si trovano

**Dato** una funzione che scrive in una riga di log il valore esatto di un segreto consegnato al tentativo
**Quando** la riga viene salvata o mostrata
**Allora** al posto del valore c'è un segnaposto, e la ricerca del valore sentinella non lo trova nemmeno nei log dei task
**E** la documentazione avverte che un valore trasformato, per esempio codificato, non viene riconosciuto

**Dato** un manifest registrato dopo la pubblicazione che chiede una connessione in più
**Quando** l'attività sta per essere consegnata
**Allora** il task fallisce con l'errore e non riceve altri segreti; il caso entra nel test di NFR-8

**Dato** un avvio con una connessione senza segreto
**Quando** arriva
**Allora** è rifiutato prima di essere accettato, con la connessione nel messaggio

**Dato** un segreto sostituito
**Quando** parte la consegna successiva
**Allora** l'attività riceve il valore nuovo, senza modifiche ai workflow

**Dato** un task senza manifest con un segreto assegnato
**Quando** l'attività viene consegnata
**Allora** riceve solo quel segreto

**Dato** un segreto eliminato durante un'esecuzione
**Quando** l'attività che lo usa sta per essere consegnata
**Allora** fallisce alla consegna con il motivo

**Dato** il database fermo mentre un'attività con segreti aspetta la consegna
**Quando** il database torna
**Allora** l'attività viene consegnata come deciso per AR-110, senza consumare tentativi

### Story 8.4: Connessioni dei workflow dell'integratore risolte all'avvio

Come integratore,
voglio che un mio workflow usato da un tenant prenda i segreti di chi lo avvia,
così che offro un workflow unico e ogni cliente ci mette le proprie credenziali.

**Rispetta:** AD-5, AD-7; C-7
**Requisiti:** FR-29 (connessioni risolte all'avvio), FR-33; AR-89 (M-37, §11 n.14)
**Prova per l'utente:** avviare il workflow dell'integratore che usa Censys con il JWT di «acme», che ha «censys-prod»: l'attività riceve il segreto di «acme»; con il JWT di «globex», senza segreti Censys, l'avvio è rifiutato con la connessione mancante.
**Docs utente:** connessioni nei workflow dell'integratore.

**Acceptance Criteria:**

**Dato** un workflow dell'integratore con una connessione richiesta
**Quando** un tenant lo avvia
**Allora** la connessione si risolve tra i segreti di quel tenant

**Dato** nessun segreto del tenant che soddisfa la connessione
**Quando** il tenant avvia
**Allora** l'avvio è rifiutato prima di essere accettato, con la connessione mancante nel messaggio

**Dato** più segreti del tenant che soddisfano la stessa connessione
**Quando** il tenant avvia
**Allora** la scelta segue la regola fissata per M-37 e non è mai implicita

**Dato** un task senza manifest (per esempio `call http`) in un workflow dell'integratore
**Quando** il documento dichiara la sua connessione
**Allora** si risolve allo stesso modo
**E** senza dichiarazione il task non riceve segreti

### Story 8.5: Protezione a riposo e rotazione della chiave principale

Come integratore,
voglio che nessun supporto contenga un segreto in chiaro e poter ruotare la chiave principale senza fermare il motore,
così che una copia rubata dei dati non rivela nulla.

**Rispetta:** AD-6, AD-7; C-12
**Requisiti:** FR-34, NFR-8; AR-110 (rinnovo delle chiavi dei tenant e dati associati)
**Prova per l'utente:** ruotare la chiave principale con il comando documentato mentre gira un'esecuzione: l'esecuzione finisce, i segreti restano utilizzabili e la chiave vecchia non decifra più nulla.
**Docs utente:** protezione dei segreti; ruotare la chiave; cosa succede se si perde la chiave.

**Acceptance Criteria:**

**Dato** un'installazione con segreti salvati
**Quando** si copiano database, bus e archivio senza la chiave principale
**Allora** nessun segreto è leggibile, e la chiave non sta né nel database né nelle sue copie

**Dato** l'integratore che ruota la chiave principale
**Quando** la rotazione finisce
**Allora** il motore non si è fermato, nessun segreto è perso e le chiavi dei tenant sono cifrate di nuovo con la chiave nuova (C-12)
**E** la chiave vecchia non decifra più nulla di ciò che è salvato

**Dato** la documentazione per l'integratore
**Quando** la si legge
**Allora** spiega cosa succede se la chiave principale si perde

## Epic 9: Pianificazioni

Il proprietario di un workflow lo fa partire da solo con un cron o un intervallo, dal `schedule` del
documento o con la pianificazione propria del tenant via API e SDK; una sola esecuzione per scadenza,
rifiuti registrati con il motivo.

Prima della storia 9.1: Update della spine su dove la pianificazione del documento porta il proprio
input (M-04, AR-87).

### Story 9.1: Pianificazione nel documento OWS

Come integratore,
voglio scrivere nel documento un `schedule` cron o a intervallo,
così che il workflow parte da solo, una volta per ogni scadenza, anche se il motore si ferma.

**Rispetta:** AD-2, AD-10; C-1, C-3, C-7
**Requisiti:** FR-1 (`schedule`), FR-6 (pianificazione del documento), FR-26 (proprietario fissato alla creazione), FR-29 (esecuzioni dell'integratore), FR-56, NFR-14; AR-87 (M-04), AR-108 (C-3), AR-109 (`cron` in UTC)
**Prova per l'utente:** pubblicare l'esempio con `schedule` ogni minuto; dopo tre minuti ci sono tre esecuzioni con origine «pianificazione»; fermando il motore per due minuti, al riavvio le scadenze perse e recuperate sono visibili.
**Docs utente:** pianificare un workflow nel documento.

**Acceptance Criteria:**

**Dato** un workflow con `schedule` cron o a intervallo
**Quando** scatta una scadenza
**Allora** parte un'esecuzione con origine «pianificazione», che appartiene al proprietario del workflow
**E** per un workflow dell'integratore nessun tenant la vede

**Dato** il motore che si ferma o riparte mentre scatta una scadenza
**Quando** torna attivo
**Allora** per quella scadenza esiste una sola esecuzione, garantita dalla chiave pianificazione + versione + istante UTC con un vincolo di unicità nella proiezione (C-3)
**E** le scadenze perse durante il fermo sono recuperate secondo il predefinito documentato (NFR-14) e visibili al proprietario

**Dato** l'input che la pianificazione porta, nella forma decisa per M-04
**Quando** il documento viene salvato e a ogni scadenza
**Allora** l'input è controllato sullo schema di input del workflow

**Dato** una scadenza che non diventa un'esecuzione accettata (input fuori schema, connessione senza segreto, quota)
**Quando** scatta
**Allora** il rifiuto è registrato sulla pianificazione con il motivo, visibile al proprietario e all'operatore, mai in silenzio

**Dato** un'espressione cron
**Quando** si calcola la prossima scadenza
**Allora** è in UTC ed è calcolata solo dal dominio
**E** un'espressione non valida o un intervallo sotto il minimo dell'installazione sono rifiutati all'invio

### Story 9.2: Pianificazione del tenant via API e SDK

Come tenant,
voglio aggiungere a un workflow una mia pianificazione con il mio input, senza modificare il workflow,
così che faccio girare da solo anche un workflow dell'integratore o definito come codice.

**Rispetta:** AD-5; C-3, C-7, C-17
**Requisiti:** FR-57 (API e SDK), FR-6 (pianificazione del tenant), FR-26 (il trigger appartiene al tenant), FR-29, FR-35
**Prova per l'utente:** con il JWT di «acme», creare dall'SDK una pianificazione ogni minuto del workflow dell'integratore con un input proprio; le esecuzioni partono e sono di «acme»; il workflow non cambia versione.
**Docs utente:** pianificazioni del tenant.

**Acceptance Criteria:**

**Dato** un workflow del tenant, dell'integratore o definito come codice
**Quando** il tenant crea la propria pianificazione con input e ripetizione
**Allora** la pianificazione è un trigger del tenant, non parte del workflow: il workflow non cambia
**E** la risposta mostra la prossima scadenza

**Dato** un tenant che ha già una pianificazione per quel workflow
**Quando** ne crea una seconda
**Allora** è rifiutato: al massimo una pianificazione per workflow per tenant

**Dato** la pianificazione del documento dello stesso workflow
**Quando** il tenant la legge
**Allora** la vede in sola lettura, distinta dalla propria

**Dato** il tenant
**Quando** modifica o elimina la propria pianificazione
**Allora** le scadenze successive seguono la modifica, e una pianificazione eliminata non avvia più nulla

**Dato** le esecuzioni avviate dalla pianificazione del tenant
**Quando** partono
**Allora** appartengono al tenant e usano i suoi segreti e le sue quote

**Dato** un workflow con pianificazioni, del documento o dei tenant
**Quando** viene eliminato
**Allora** le sue pianificazioni sono eliminate e non avviano più nulla (storia 2.10)

### Story 9.3: Sospendere, riprendere e seguire le scadenze

Come proprietario di una pianificazione,
voglio sospenderla e riprenderla senza toccare il workflow e vedere le scadenze rifiutate,
così che fermo un lavoro periodico per un po' e capisco perché una scadenza non è partita.

**Rispetta:** AD-5; C-3
**Requisiti:** FR-6 (sospensione e scadenze rifiutate), FR-57 (sospendere e riprendere); §11 n.17 e n.18 nella spec
**Prova per l'utente:** sospendere la pianificazione di «acme» per due minuti e riprenderla: nessuna esecuzione per le scadenze cadute nella sospensione; una scadenza con input fuori schema compare tra le rifiutate con il motivo.
**Docs utente:** sospendere una pianificazione; scadenze rifiutate.

**Acceptance Criteria:**

**Dato** una pianificazione attiva
**Quando** il proprietario la sospende
**Allora** non partono esecuzioni e il workflow non cambia
**E** alla ripresa le scadenze cadute durante la sospensione non si recuperano

**Dato** una pianificazione del documento di un workflow condiviso
**Quando** qualcuno prova a sospenderla
**Allora** vale la regola fissata nella spec per §11 n.18 su chi può farlo e da dove

**Dato** scadenze rifiutate
**Quando** il proprietario o l'operatore le cercano
**Allora** le trovano con il motivo, nel punto fissato dalla spec per §11 n.17
**E** il comportamento dopo più rifiuti di fila (nuovo tentativo, sospensione automatica) segue la stessa spec

## Epic 10: Nodi personalizzati in sandbox

Il tenant carica dalla CLI o dall'SDK un nodo compilato TypeScript o Rust e lo usa nei suoi workflow;
la sandbox leggera, o la microVM dove c'è KVM, lo isola, lo limita e lo ferma senza toccare gli
altri tenant.

Le tecnologie dei due livelli, il formato del modulo compilato e il confinamento dei processi
ausiliari vengono dall'ADR di S-2.

### Story 10.1: Caricare un nodo personalizzato compilato

Come sviluppatore di un tenant,
voglio caricare dalla CLI o dall'SDK un nodo già compilato in TypeScript o Rust con il suo manifest,
così che i workflow del mio tenant usano codice nostro senza un worker sempre attivo.

**Rispetta:** AD-6 (validazione e compilazione fuori dal motore), AD-16; C-5 (versione = SHA-256 del contenuto), C-7
**Requisiti:** FR-15 (caricamento), FR-55 (manifest dei nodi), FR-16 (trattamento dei byte fuori dal processo della chiave), NFR-8, NFR-14; AR-96 (credenziale per caricare un nodo), AR-108 (impronta `sha256:`)
**Prova per l'utente:** con la CLI e la credenziale di «acme», caricare il nodo TypeScript di esempio; l'elenco dei nodi di «acme» lo mostra con versione e impronta; «globex» non lo vede.
**Docs utente:** scrivere e caricare un nodo personalizzato.

**Acceptance Criteria:**

**Dato** un nodo TypeScript o Rust già compilato nel formato fissato da S-2, con il manifest
**Quando** il tenant lo carica via SDK o CLI
**Allora** il nodo è disponibile solo ai workflow di quel tenant
**E** il server non richiede compilatori installati

**Dato** un modulo o un manifest non conforme
**Quando** viene caricato
**Allora** è rifiutato con l'errore del validatore

**Dato** un modulo che fa crashare il validatore o il compilatore
**Quando** viene caricato
**Allora** fallisce solo il processo ausiliario, e motore ed esecuzioni in corso continuano

**Dato** un nodo caricato con un nome già usato
**Quando** viene salvato
**Allora** diventa una versione nuova, identificata dall'impronta SHA-256 del contenuto
**E** le esecuzioni in corso finiscono con la versione con cui sono partite, e ogni attività registra la versione che ha eseguito

**Dato** un nodo oltre la dimensione massima o la quota di spazio del tenant
**Quando** viene caricato
**Allora** è rifiutato con limite e valore

### Story 10.2: Eseguire un nodo nella sandbox leggera

Come tenant,
voglio che i miei workflow chiamino i miei nodi personalizzati,
così che uso la mia logica dentro un workflow con la garanzia che resti isolata.

**Rispetta:** AD-5, AD-6, AD-7; C-2
**Requisiti:** FR-1 (invocazione di nodi), FR-16 (isolamento della sandbox), FR-59 (righe dei nodi), FR-29 (nodo del tenant con lo stesso nome), NFR-8, NFR-10, NFR-12
**Prova per l'utente:** pubblicare con «acme» un workflow che chiama il nodo di esempio e avviarlo: l'attività gira nella sandbox leggera e l'esecuzione arriva a completata; il nodo ostile di prova che legge file e rete fallisce senza ottenere nulla.
**Docs utente:** usare un nodo personalizzato in un workflow; cosa vede un nodo.

**Acceptance Criteria:**

**Dato** un workflow del tenant che chiama un suo nodo
**Quando** l'attività parte
**Allora** il nodo gira nella sandbox leggera, senza KVM né root, e vede solo il proprio input e i segreti dichiarati dal suo task

**Dato** un nodo che prova a raggiungere la rete interna dell'installazione, il file system dell'host, dati o segreti di altri tenant o la chiave della cassaforte
**Quando** gira
**Allora** non ci riesce, e i casi con codice ostile noto (fuga, esaurimento, rete interna, lettura della chiave) entrano nel test di NFR-8

**Dato** le destinazioni di rete consentite al tenant
**Quando** il nodo apre una connessione
**Allora** raggiunge solo quelle, e la lista dichiarata dal nodo può restringerle, mai allargarle

**Dato** un nodo del tenant B in esecuzione con un segreto sentinella
**Quando** si ispeziona il processo del nodo del tenant A
**Allora** non contiene il valore: un'istanza della sandbox non contiene mai dati o segreti di due tenant, né insieme né in sequenza

**Dato** un workflow dell'integratore eseguito per un tenant che ha un nodo con lo stesso nome di uno usato dal workflow
**Quando** gira
**Allora** esegue il nodo previsto dal workflow dell'integratore, mai quello del tenant

**Dato** un nodo che scrive righe
**Quando** gira nella sandbox
**Allora** le righe finiscono nel log del tentativo con lo stesso formato e lo stesso tetto di quelle dei worker (storia 2.12), con il segnaposto al posto dei segreti consegnati (storia 8.3), e mai nei log del motore
**E** compaiono nella tab «Log» della dashboard e del kit (storie 3.5 e 6.5)

**Dato** il benchmark di base
**Quando** misura la sandbox leggera
**Allora** riporta avvio e memoria per istanza contro gli obiettivi confermati da S-2 per NFR-12

### Story 10.3: Limiti di risorse e quote della sandbox

Come integratore,
voglio che ogni nodo abbia limiti di CPU, memoria e tempo e che ogni tenant abbia una quota della sandbox,
così che un nodo in loop non danneggia nessun altro.

**Rispetta:** AD-6; C-7
**Requisiti:** FR-17 (limiti dei nodi), FR-30 (quote della sandbox), NFR-2, NFR-14; AR-14 (M-27, limite del nodo e timeout del task)
**Prova per l'utente:** avviare con «acme» il nodo di esempio che va in loop: la sandbox lo ferma al limite e il task fallisce con un messaggio che nomina il limite; nel frattempo le attività di «globex» restano entro NFR-2.
**Docs utente:** limiti dei nodi e quote della sandbox.

**Acceptance Criteria:**

**Dato** un nodo che supera il limite di CPU, memoria o tempo
**Quando** il limite scatta
**Allora** l'attività fallisce con un errore che nomina il limite superato
**E** il processo del motore non si ferma e le attività degli altri tenant restano entro l'obiettivo di NFR-2

**Dato** un nodo fermato
**Quando** si controllano i processi
**Allora** non restano processi né memoria occupata

**Dato** un tenant che ha esaurito la propria quota della sandbox
**Quando** arrivano nuove attività di nodi
**Allora** restano in coda con il motivo, entro il limite di attesa di FR-8

**Dato** limiti non dichiarati dal nodo
**Quando** gira
**Allora** valgono i predefiniti documentati, mai oltre il massimo dell'installazione (NFR-14)

### Story 10.4: Sandbox forte con microVM

Come integratore,
voglio attivare, dove l'host ha KVM, la sandbox forte con un pool di microVM già avviate per tutti o alcuni tenant,
così che i tenant che lo richiedono hanno un isolamento più forte senza avvio a freddo.

**Rispetta:** AD-1, AD-6
**Requisiti:** FR-54, FR-30 (parte del pool per tenant), FR-31 (sandbox riservate nella prova sentinella), NFR-8, NFR-12; UX-DR-63 (riga «Sandbox forte non attiva…» della pagina Worker)
**Prova per l'utente:** su un host con KVM, attivare la sandbox forte per «acme» con un pool di 2; le attività dei nodi di «acme» partono senza avvio a freddo; su un host senza KVM la dashboard mostra che la sandbox forte non è attiva.
**Docs utente:** attivare la sandbox forte.

**Acceptance Criteria:**

**Dato** un host con KVM e la sandbox forte attiva per un tenant
**Quando** arriva un'attività di un suo nodo con una sandbox libera nel pool
**Allora** parte senza avvio a freddo, entro l'obiettivo confermato per NFR-12

**Dato** il pool vuoto
**Quando** arriva un'attività
**Allora** aspetta una sandbox libera entro il limite di attesa di FR-8

**Dato** una sandbox forte che ha eseguito codice di un tenant
**Quando** il lavoro finisce
**Allora** non passa mai a un altro tenant: è monouso o riservata a quel tenant

**Dato** un host senza KVM
**Quando** l'integratore prova ad attivare la sandbox forte
**Allora** non si attiva, i nodi girano nella sandbox leggera e la pagina «Worker» della dashboard mostra l'avviso all'operatore
**E** la RAM del pool si misura a parte, fuori da NFR-1

### Story 10.5: Processi ausiliari fermati quando non servono

Come integratore,
voglio che i processi ausiliari inattivi si fermino da soli e ripartano alla prima richiesta,
così che a riposo topictoast occupa poca memoria.

**Rispetta:** AD-1, AD-6
**Requisiti:** FR-50 (processi ausiliari fermati e riavviati), NFR-1, NFR-2, NFR-12, NFR-14; AR-14 (M-28)
**Prova per l'utente:** dopo il tempo di inattività configurato, l'elenco dei processi mostra solo motore e bus; un'esecuzione successiva riparte normalmente e il benchmark riporta il tempo di riavvio a freddo.
**Docs utente:** processi ausiliari e memoria a riposo.

**Acceptance Criteria:**

**Dato** un processo ausiliario inattivo oltre il tempo configurato (predefinito documentato, NFR-14)
**Quando** il tempo scade
**Allora** il motore lo ferma

**Dato** un processo ausiliario fermato
**Quando** arriva la prima richiesta che lo usa
**Allora** il motore lo riavvia a freddo e la richiesta riesce
**E** il benchmark riporta il tempo di riavvio a freddo, mentre NFR-2 e NFR-12 si misurano a regime

**Dato** un tenant con pipeline stream attive
**Quando** passa il tempo di inattività
**Allora** il suo processo delle pipeline non viene fermato, anche senza eventi (finestre aperte, checkpoint), e si ferma solo con l'ultima pipeline o con l'eliminazione del tenant

**Dato** il ruolo della validazione a secco inattivo
**Quando** passa il tempo di inattività
**Allora** si ferma e riparte a freddo alla prima validazione

**Dato** l'installazione a riposo
**Quando** si misura la RAM come dice NFR-1
**Allora** si contano il motore, il bus e ogni processo ausiliario ancora attivo
**E** ogni tenant con pipeline attive aggiunge la RAM del suo processo (circa 64 MB nello spike, domanda aperta) e si conta anche il ruolo della validazione a secco quando valida

## Epic 11: Pipeline stream

Integratore e tenant scrivono eventi sugli stream e definiscono pipeline in SQL con finestre,
watermark e checkpoint; le righe emesse vanno su uno stream o avviano un workflow, una volta per
riga; l'operatore vede lo stato delle pipeline.

Il dialetto SQL chiuso, la costruzione di stato, finestre, watermark e checkpoint sopra DataFusion e
la deduplicazione dei blocchi nel processo delle pipeline vengono dall'ADR di S-3. Il caso guida è UJ-3, un'assunzione finché
non arriva il caso reale di Marco (§11 n.1).

### Story 11.1: Scrivere eventi sugli stream di un tenant

Come integratore,
voglio che la mia piattaforma, i worker e i workflow scrivano eventi sugli stream di un tenant,
così che le pipeline stream di quel tenant hanno i dati da elaborare.

**Rispetta:** AD-5, AD-7, AD-12 (eventi a blocchi), AD-13 (topic propri del tenant con l'identificativo interno); C-1, C-7, C-12, C-18
**Requisiti:** FR-18, FR-26 (scrittura solo sugli stream del proprietario), FR-30 (eventi al secondo), FR-38 (stream via SDK), NFR-8, NFR-14; AR-86 (come un task o un worker scrive su uno stream)
**Prova per l'utente:** con l'SDK scrivere 1.000 eventi di esempio sullo stream «indicatori» di «acme»; un tentativo di scrivere con il contesto di «globex» su quello stream riceve lo stesso errore di uno stream inesistente.
**Docs utente:** stream ed eventi.

**Acceptance Criteria:**

**Dato** un tenant
**Quando** l'integratore, un worker o un workflow scrive eventi a blocchi su un suo stream via API o SDK
**Allora** gli eventi sono accettati solo dopo la scrittura durevole e cifrati con la chiave del tenant
**E** un evento senza tempo dell'evento riceve il tempo di arrivo, scritto all'ingresso e mai alla lettura

**Dato** un blocco di 1.000 eventi
**Quando** viene scritto
**Allora** è accettato
**E** un blocco di 1.001 eventi o di oltre 1 MiB è rifiutato con un codice (C-7) e nessun evento è scritto; il motore non divide mai un blocco

**Dato** lo stesso blocco con la stessa `Idempotency-Key` (C-18) inviato due volte
**Quando** il motore lo riceve
**Allora** risponde 200 entrambe le volte e la pipeline elabora gli eventi una volta sola (questa seconda parte si prova nelle storie 11.4 e 11.5)

**Dato** un evento con un valore di tipo sbagliato rispetto alle colonne dello stream
**Quando** viene scritto
**Allora** è rifiutato con un errore di validazione
**E** un decimale `1.10` resta `1.10`, mai `f64`

**Dato** più eventi di quelli che stanno in un blocco
**Quando** il client li invia
**Allora** la forma del corpo e chi divide gli invii oltre il tetto (SDK) sono quelli fissati nella spec, dopo la domanda aperta sulla forma del corpo, decisa prima di questa storia

**Dato** un workflow o un worker
**Quando** scrive su uno stream
**Allora** può scrivere solo sugli stream del proprietario dell'esecuzione o dell'attività servita
**E** su uno stream di un altro tenant riceve lo stesso errore di uno stream inesistente e nessun evento arriva

**Dato** uno stream
**Quando** si configura la conservazione per durata o dimensione
**Allora** vale il predefinito documentato (NFR-14), breve: copre la finestra più lunga delle pipeline che leggono lo stream più ritardo tollerato e intervallo dei checkpoint

**Dato** un tenant che scrive oltre la quota di eventi al secondo
**Quando** continua a scrivere
**Allora** riceve errori di quota, mentre gli altri tenant continuano a scrivere e ad avviare esecuzioni entro NFR-2

### Story 11.2: Validazione a secco del SQL delle pipeline stream

Come tenant,
voglio che il SQL di una pipeline sia controllato prima di partire senza mettere a rischio le mie pipeline in corsa,
così che un errore o una query ostile non fermano il lavoro già attivo.

**Rispetta:** AD-6, AD-11, AD-13; C-7
**Requisiti:** FR-17 (limiti delle query), FR-19, FR-25, NFR-8; AR-19
**Prova per l'utente:** con una pipeline attiva di «acme», salvare una query con una voce fuori dal dialetto: il rifiuto arriva con un errore e la pipeline attiva continua; poi salvare una query che porta il ruolo oltre il suo limite di memoria: il salvataggio riceve un codice di errore e le pipeline attive restano in corsa.
**Docs utente:** validazione del SQL delle pipeline stream.

**Acceptance Criteria:**

**Dato** un salvataggio di una pipeline mentre il tenant ha pipeline attive in corsa
**Quando** il SQL è validato
**Allora** lo valida un ruolo ausiliario proprio, uno per tenant, a richiesta, con DataFusion, mai il processo delle pipeline
**E** le pipeline in corsa non si fermano

**Dato** il ruolo della validazione a secco
**Quando** si ispeziona
**Allora** riceve dal motore sull'IPC solo le definizioni delle colonne degli stream del suo tenant, non ha accesso al bus ed è confinato come gli altri ausiliari (Landlock senza cartelle né porte, `RLIMIT_DATA`)

**Dato** un SQL che porta il ruolo oltre il suo limite di memoria
**Quando** il ruolo cade
**Allora** il motore risponde con un codice (C-7) e l'editor o l'API lo mostrano
**E** il motore e le pipeline attive restano in corsa

**Dato** un SQL salvato
**Quando** il ruolo lo controlla
**Allora** il controllo del dialetto chiuso sull'albero sintattico gira nel ruolo alla pubblicazione, con la stessa implementazione che il processo delle pipeline usa al caricamento

**Dato** la lista delle query ostili dello spike (DDL su `/etc/passwd`, `COPY` verso un file, connettore verso l'indirizzo del bus, funzione dell'utente, funzioni di tabella, join e prodotto cartesiano, `repeat` enorme, costante enorme, annidamento profondo, stream di un altro tenant, errore di sintassi, finestra oltre la conservazione, ciclo tra pipeline)
**Quando** gira
**Allora** ogni voce è rifiutata ed è un test di regressione

**Dato** il ruolo inattivo
**Quando** passa il tempo di inattività
**Allora** si ferma, e riparte a freddo alla prima validazione

### Story 11.3: Pipeline stream definita in SQL

Come tenant,
voglio definire una pipeline stream con una query SQL sui miei stream e mandare le righe emesse su un mio stream,
così che trasformo eventi in risultati senza scrivere codice.

**Rispetta:** AD-6, AD-11 (SQL validato solo da un processo ausiliario), AD-13 (dialetto chiuso, quattro barriere, un processo per tenant); C-4, C-7
**Requisiti:** FR-19, FR-24 (destinazione stream), FR-38 (pipeline via SDK), NFR-8, NFR-14; AR-19
**Prova per l'utente:** salvare dall'SDK una pipeline che filtra gli eventi di «indicatori» e scrive su «sospetti»; le ultime righe emesse si leggono via API. Una query con un errore di sintassi è rifiutata con riga e colonna.
**Docs utente:** dialetto SQL delle pipeline stream.

**Acceptance Criteria:**

**Dato** una query sugli stream del tenant con le sole funzioni del dialetto chiuso
**Quando** il tenant la salva
**Allora** viene validata a secco dal ruolo ausiliario della validazione a secco (storia 11.2) e la pipeline parte nel processo delle pipeline del tenant
**E** le righe emesse arrivano sullo stream di destinazione e le ultime si leggono via API

**Dato** un riavvio dal checkpoint
**Quando** la pipeline rifà righe già emesse verso lo stream di destinazione
**Allora** le righe rifatte arrivano una volta sola (stessa chiave C-4)
**E** come raggruppare le righe in blocchi verso lo stream di destinazione è quanto fissato nella spec, dopo la domanda aperta su questo punto, decisa prima di questa storia

**Dato** un errore di sintassi o uno stream inesistente
**Quando** la query viene salvata
**Allora** l'errore indica riga e colonna
**E** uno stream di un altro tenant è trattato come inesistente

**Dato** una query con DDL, DML, ogni altra istruzione, tabelle esterne, connettori, lettura o scrittura di file, accesso alla rete, funzioni definite dall'utente, join, sottoquery, CTE, `ORDER BY`, `LIMIT`, `OVER`, funzioni di tabella o funzioni fuori dall'elenco
**Quando** viene salvata
**Allora** è rifiutata con un errore di validazione
**E** le query ostili della lista dello spike (tabella esterna su `/etc/passwd`, `COPY` verso un file, connettore verso l'indirizzo del bus, UDF) sono un test di regressione del controllo sull'albero e di NFR-8

**Dato** il testo di una query di 16 KiB, e uno di 16 KiB + 1 byte; un letterale di 1 KiB, e uno di 1 KiB + 1 byte; una profondità di 64, e una di 65
**Quando** vengono salvati
**Allora** il valore al limite è accettato e quello oltre è rifiutato con un errore di validazione

**Dato** una funzione o una clausola nuova del dialetto
**Quando** entra nell'elenco chiuso
**Allora** passa dalla lista delle query ostili prima di essere ammessa

**Dato** una query che il controllo sull'albero non fermasse
**Quando** arriva al processo delle pipeline
**Allora** la fermano le altre barriere, in quest'ordine: `SQLOptions` senza DDL, DML né istruzioni, contesto con solo gli stream del tenant, limiti di memoria

**Dato** il ritardo tollerato e l'intervallo dei checkpoint
**Quando** il tenant li dichiara
**Allora** sono campi della pipeline, fuori dal SQL, e un tipo fuori da testo, intero a 64 bit, decimale esatto, booleano e istante UTC è rifiutato

**Dato** due pipeline dello stesso tenant che formerebbero un ciclo (una scrive sullo stream da cui legge l'altra, fino a tornare alla prima)
**Quando** si salva la seconda
**Allora** è rifiutata con le pipeline coinvolte

### Story 11.4: Finestre, aggregazioni e watermark

Come tenant,
voglio aggregare gli eventi su finestre di tempo dell'evento e dichiarare quanto ritardo tollero,
così che conto gli indicatori per ASN ogni 5 minuti anche con eventi in ritardo.

**Rispetta:** AD-13
**Requisiti:** FR-20, FR-21, FR-18 (finestra oltre la conservazione), NFR-5, NFR-14
**Prova per l'utente:** con la pipeline di UJ-3 (conteggio per ASN su finestre fisse di 5 minuti, righe con più di 50 indicatori), scrivere un'ondata di eventi per un ASN: alla chiusura della finestra compare una riga per quell'ASN; un evento arrivato dopo la chiusura è scartato e contato.
**Docs utente:** finestre, watermark ed eventi in ritardo.

**Acceptance Criteria:**

**Dato** un conteggio su finestre fisse di 5 minuti raggruppato per chiave
**Quando** la finestra si chiude
**Allora** esce una riga per gruppo
**E** le finestre scorrevoli funzionano allo stesso modo; le finestre di sessione sono fuori dalla v1

**Dato** una deduplicazione su chiave
**Quando** la stessa chiave arriva più volte nella finestra
**Allora** è emessa una sola volta per finestra

**Dato** una finestra con la somma di una colonna decimale
**Quando** la finestra si chiude
**Allora** la somma è un decimale esatto, mai `f64`

**Dato** un ritardo tollerato dichiarato (predefinito 10 secondi)
**Quando** il watermark supera la fine della finestra più il ritardo tollerato
**Allora** la finestra si chiude
**E** gli eventi che arrivano dopo la chiusura della loro finestra sono scartati e contati in una metrica

**Dato** un evento con un tempo oltre 60 secondi nel futuro rispetto al suo arrivo (per esempio un anno avanti)
**Quando** arriva
**Allora** è scartato e contato in una metrica, non sposta il watermark, non fa scartare gli eventi successivi e non chiude finestre
**E** un evento a 60 secondi nel futuro è accettato, uno a 61 secondi è scartato

**Dato** uno stream letto da una pipeline attiva che smette di ricevere eventi
**Quando** passano 30 secondi di quiete dall'ultimo blocco (con il segnale durevole scritto dal motore dopo 1 secondo senza blocchi)
**Allora** le finestre aperte si chiudono comunque e l'ultima finestra dell'ondata emette le sue righe entro ritardo tollerato più quiete dopo la sua fine, senza garanzia di meno
**E** su uno stream senza pipeline attive il motore non scrive alcun segnale

**Dato** uno stream rielaborato dallo stesso log, nello stesso ordine
**Quando** la pipeline lo rilegge
**Allora** chiude le stesse finestre ed emette le stesse righe

**Dato** una pipeline con più stream (`UNION ALL`) o senza finestra
**Quando** si calcola il watermark
**Allora** vale la regola fissata nella spec, dopo la domanda aperta sul watermark con più stream e sulle pipeline senza finestra, decisa prima di questa storia

**Dato** il benchmark di base
**Quando** misura la pipeline di filtro e conteggio su finestra fissa
**Allora** riporta gli eventi al secondo contro l'obiettivo di NFR-5 confermato da S-3

**Dato** una pipeline con una finestra più lunga della conservazione dello stream
**Quando** viene salvata
**Allora** è rifiutata con i due valori

### Story 11.5: Stato e checkpoint che sopravvivono ai riavvii

Come tenant,
voglio che lo stato della pipeline sopravviva a un riavvio,
così che un crash a metà finestra non perde il conteggio.

**Rispetta:** AD-7, AD-13, AD-17 (checkpoint nell'archivio dei payload)
**Requisiti:** FR-22, FR-19 (stesse righe sugli stessi eventi), FR-31 (checkpoint nella prova sentinella), NFR-6
**Prova per l'utente:** durante una finestra di UJ-3, uccidere il processo della pipeline: riparte dall'ultimo checkpoint e alla chiusura della finestra il conteggio è lo stesso che senza il crash.
**Docs utente:** checkpoint delle pipeline stream.

**Acceptance Criteria:**

**Dato** una pipeline attiva
**Quando** passa l'intervallo dei checkpoint (predefinito 10 secondi, campo della pipeline), tra due lotti
**Allora** lo stato è salvato tramite la porta dell'archivio dei payload, cifrato con la chiave del tenant
**E** il checkpoint contiene la posizione letta, il watermark, le chiavi dei blocchi viste (solo entro la conservazione), lo stato delle finestre aperte e le righe in attesa di consegna, mai lo stato di sospensione

**Dato** un riavvio della pipeline o del motore
**Quando** la pipeline riparte
**Allora** riparte dall'ultimo checkpoint, rielabora gli eventi successivi e nessuna finestra perde eventi già accettati
**E** rielaborata sugli stessi eventi, emette le stesse righe

**Dato** un blocco già visto prima del crash
**Quando** la pipeline riparte
**Allora** il processo lo scarta usando le chiavi dei blocchi nel checkpoint

**Dato** un crash a metà scrittura del checkpoint (per esempio un file temporaneo senza la rinomina)
**Quando** la pipeline riparte
**Allora** vale l'ultimo checkpoint intero

**Dato** l'archivio dei payload su RustFS
**Quando** il processo scrive un checkpoint
**Allora** il percorso è quello fissato nella spec, dopo la domanda aperta su come il checkpoint passa al motore, decisa prima di questa storia

**Dato** un tenant eliminato
**Quando** passa il tempo massimo documentato
**Allora** la ricerca di un valore sentinella non lo trova nemmeno nei checkpoint

### Story 11.6: Attivare, sospendere, modificare ed eliminare una pipeline

Come tenant,
voglio gestire il ciclo di vita delle mie pipeline stream,
così che le fermo quando non servono senza perdere eventi in silenzio.

**Rispetta:** AD-2, AD-6, AD-13
**Requisiti:** FR-23, FR-38 (pipeline via SDK)
**Prova per l'utente:** sospendere la pipeline di UJ-3 per un minuto mentre arrivano eventi e riprenderla: elabora gli eventi arrivati nel frattempo; modificarne la query crea una versione nuova con stato vuoto.
**Docs utente:** gestire le pipeline stream.

**Acceptance Criteria:**

**Dato** una pipeline attiva
**Quando** il tenant la sospende
**Allora** non consuma CPU

**Dato** il processo delle pipeline che segnala sull'IPC un codice per una pipeline
**Quando** il motore riceve la segnalazione
**Allora** è il motore a sospenderla e a scrivere l'evento di stato (AD-2); il processo non cambia mai lo stato da solo

**Dato** il riavvio del processo o del motore
**Quando** il processo riparte
**Allora** il motore gli ripassa le pipeline attive con la loro versione, e lo stato di sospensione non sta nel checkpoint

**Dato** una pipeline sospesa
**Quando** il tenant la riprende
**Allora** elabora gli eventi arrivati nel frattempo e ancora conservati
**E** se una parte è uscita dalla conservazione, la ripresa fallisce con un codice del catalogo (C-7) visibile al tenant e all'operatore, la posizione resta e nessun evento si perde in silenzio

**Dato** una pipeline attiva
**Quando** il tenant ne modifica la query
**Allora** nasce una nuova versione con stato vuoto
**E** da quale posizione dello stream parte la nuova versione è quanto fissato nella spec, dopo la domanda aperta su questo punto, decisa prima di questa storia

**Dato** una pipeline
**Quando** il tenant la elimina
**Allora** il suo processo smette di elaborarla e il suo stato viene rimosso

### Story 11.7: Righe che avviano un workflow

Come tenant,
voglio che ogni riga emessa da una pipeline avvii un mio workflow con la riga come input,
così che un'ondata sospetta fa partire da sola l'analisi, una volta per ASN.

**Rispetta:** AD-5, AD-6, AD-13 (consegna sull'IPC a gruppi, decisione riga per riga), AD-18 (righe e avvii via IPC con identificativo deterministico); C-4
**Requisiti:** FR-24 (destinazione workflow), FR-6 (avvio da riga di pipeline stream), FR-26 (proprietario del trigger), FR-30 (righe trattenute nell'arretrato); AR-108 (C-4)
**Prova per l'utente:** con la pipeline di UJ-3 e la destinazione «avvia il workflow di arricchimento», un'ondata per un ASN produce una sola esecuzione con origine «pipeline stream»; uccidendo il processo prima del checkpoint, dopo la ripresa le esecuzioni non raddoppiano.
**Docs utente:** pipeline stream che avviano workflow.

**Acceptance Criteria:**

**Dato** una pipeline con destinazione workflow
**Quando** emette una riga
**Allora** la riga dà esattamente un avvio, anche dopo un riavvio dal checkpoint: un'esecuzione accettata o un rifiuto registrato con il motivo
**E** la chiave è pipeline + versione + fine finestra + chiave di gruppo, o la posizione nello stream senza finestra (C-4)

**Dato** un gruppo di 1.000 righe con esiti misti (accettate, già viste, oltre quota)
**Quando** il processo lo consegna sull'IPC
**Allora** il motore decide ogni riga sul percorso di avvio normale (idempotenza C-4, quote FR-30, concorrenza FR-13) e risponde riga per riga
**E** ne accetta una parte senza annullare le altre: un gruppo non è mai tutto o niente

**Dato** più di 1.000 righe da consegnare
**Quando** il processo le consegna
**Allora** le consegna in più gruppi di al più 1.000 righe

**Dato** la durata delle chiavi C-4 e le righe in attesa di consegna
**Quando** si fissano nella spec
**Allora** la finestra di idempotenza copre almeno la conservazione dello stream più l'intervallo dei checkpoint, e la spec decide se le righe in attesa contano nella quota di stato (domande aperte)

**Dato** due righe distinte con lo stesso contenuto, per esempio lo stesso conteggio in due finestre
**Quando** vengono emesse
**Allora** producono due avvii

**Dato** una riga che non diventa un'esecuzione accettata (input fuori schema, connessione senza segreto)
**Quando** viene emessa
**Allora** il rifiuto è registrato sulla pipeline con il motivo, visibile al tenant e all'operatore

**Dato** il tenant oltre la quota di esecuzioni
**Quando** la pipeline emette righe
**Allora** le righe rifiutate per quota restano nel processo e si riprovano entro il tetto dell'arretrato, non perse
**E** oltre il tetto vale quanto fissato nella spec, dopo la domanda aperta sulle righe in attesa sotto un rifiuto per quota prolungato, decisa prima di questa storia

**Dato** le esecuzioni avviate dalla pipeline
**Quando** partono
**Allora** hanno origine «pipeline stream» e appartengono al proprietario della pipeline, fissato alla creazione

**Dato** un workflow destinazione di una pipeline stream
**Quando** si prova a eliminarlo
**Allora** è rifiutato con l'elenco delle pipeline che lo usano (storia 2.10)

### Story 11.8: Isolamento e quote delle pipeline stream

Come integratore,
voglio quote per tenant sulle pipeline attive, sulla memoria di stato e sulla CPU,
così che la pipeline pesante di un tenant non rallenta quelle degli altri.

**Rispetta:** AD-6, AD-13
**Requisiti:** FR-25, FR-17 (limiti delle query), FR-30 (quote delle pipeline), NFR-2, NFR-8; AR-19 (limite di memoria di sistema, spill)
**Prova per l'utente:** dare ad «acme» una quota di memoria di stato piccola e avviare la pipeline con troppi gruppi: la pipeline di «acme» viene sospesa con un codice visibile, mentre quella di «globex» elabora al ritmo di arrivo.
**Docs utente:** quote delle pipeline stream.

**Acceptance Criteria:**

**Dato** una pipeline oltre la sua quota di memoria di stato (predefinita 64 MiB per pipeline, NFR-14)
**Quando** la pipeline cresce
**Allora** il motore la sospende con un codice del catalogo (C-7) visibile al tenant e all'operatore
**E** le altre pipeline dello stesso processo continuano
**E** la quota è superabile di poco: la sospensione avviene entro il margine dichiarato al tenant, fissato nella spec dopo la domanda aperta su questo punto, decisa prima di questa storia

**Dato** una pipeline oltre il budget di CPU per micro-lotto
**Quando** la pipeline gira
**Allora** il motore la sospende con un codice del catalogo (C-7) visibile al tenant e all'operatore, e le altre pipeline del processo continuano
**E** il valore del budget è quello riprovato in questa storia (domanda aperta sul budget di CPU)

**Dato** il processo delle pipeline di un tenant
**Quando** si ispeziona
**Allora** ha Landlock con la sola porta del bus (e la sola cartella dei checkpoint del tenant quando li scrive su disco), `RLIMIT_NOFILE` 512 e i filtri seccomp attivi prima che tocchi il contenuto del tenant

**Dato** una pipeline di un tenant al limite
**Quando** girano le pipeline degli altri tenant
**Allora** elaborano al ritmo di arrivo senza ritardo crescente sul watermark, e le attività degli altri restano entro NFR-2

**Dato** query ostili (un `repeat` enorme, un prodotto cartesiano, costanti enormi)
**Quando** girano
**Allora** sono rifiutate dal controllo sull'albero al salvataggio e, se arrivassero al processo, dal controllo al caricamento
**E** i limiti di CPU e di memoria (pool di DataFusion, quota di stato, `RLIMIT_DATA`) sono la rete di sicurezza, e il motore non si ferma
**E** ogni funzione o clausola nuova del dialetto passa dalla lista delle query ostili

**Dato** un tenant oltre il numero massimo di pipeline attive
**Quando** ne attiva un'altra
**Allora** riceve un errore di quota

### Story 11.9: Pipeline stream nella dashboard

Come operatore,
voglio vedere le pipeline stream attive, rallentate e sospese con la causa e le metriche,
così che capisco subito quale pipeline è in difficoltà e perché.

**Rispetta:** AD-12, AD-16; skill `dashboard-react`
**Requisiti:** FR-47, NFR-9, NFR-16; UX-DR-47, UX-DR-65, UX-DR-110 (pipeline stream)
**Prova per l'utente:** con la pipeline di «acme» sospesa per la quota di memoria, aprire «Pipeline stream»: compare sospesa con la causa, il ritardo sul watermark, gli eventi al secondo, la memoria di stato, gli eventi in ritardo scartati e l'ultimo checkpoint.
**Docs utente:** pagina Pipeline stream.

**Acceptance Criteria:**

**Dato** pipeline in vari stati
**Quando** l'operatore apre «Pipeline stream»
**Allora** vede per ciascuna lo stato (attiva, rallentata, sospesa) con icona e parola, la causa con il suo codice (quota di stato, budget di CPU, righe rifiutate oltre il tetto, conservazione superata, stream mancante), il ritardo sul watermark, gli eventi al secondo, la memoria di stato, gli eventi in ritardo scartati e l'ultimo checkpoint
**E** una pipeline per ciascuna causa mostra il codice corrispondente

**Dato** righe rifiutate da una pipeline con destinazione workflow
**Quando** l'operatore apre la pipeline
**Allora** vede le righe rifiutate con il motivo

**Dato** il telefono a 360 px
**Quando** la pagina viene mostrata
**Allora** le righe diventano card impilate e la pagina resta usabile per intero

## Epic 12: Kit: comporre i workflow

Il tenant compone un workflow come grafo, sceglie i segreti, gestisce pianificazioni e pipeline
stream dal kit, anche da telefono.

Prima della storia 12.1: posizioni dei nodi nella versione e nell'export, moduli dei costrutti
generati dal dominio o scritti a mano con un test di parità (AR-91).

### Story 12.1: Comporre e salvare un workflow sul canvas

Come tenant,
voglio aggiungere task dalla palette, collegarli sul canvas e salvare,
così che costruisco da solo un workflow senza scrivere un documento.

**Rispetta:** AD-4, AD-8, AD-11 (validazione nel browser con lo stesso validatore), AD-15, AD-16; C-5, C-7
**Requisiti:** FR-40 (comporre), FR-5 (salvare = pubblicare, conflitto di versione), FR-15 (nodi del tenant nella palette), FR-4 (stessi esiti dell'invio), NFR-7, NFR-9; UX-DR-11, UX-DR-13, UX-DR-21, UX-DR-23, UX-DR-24, UX-DR-28, UX-DR-34, UX-DR-40, UX-DR-58, UX-DR-72, UX-DR-76, UX-DR-77, UX-DR-78, UX-DR-81, UX-DR-91, UX-DR-92; AR-91
**Prova per l'utente:** nel kit premere «Nuovo workflow», aggiungere dalla palette `dns_lookup` e `censys_search`, collegarli e premere «Salva»: nasce la versione 1. Collegare a ciclo due task mostra l'errore con i task coinvolti e non salva. Tutto si fa anche solo da tastiera.
**Docs utente:** comporre un workflow nell'editor.

**Acceptance Criteria:**

**Dato** l'editor di un workflow nuovo
**Quando** il tenant apre la palette
**Allora** vede solo le funzioni rese disponibili dall'integratore, i suoi nodi personalizzati e i costrutti del sottoinsieme, raggruppati, con la descrizione del manifest mai troncata

**Dato** due task sul canvas
**Quando** il tenant trascina dalla porta d'uscita di uno all'ingresso dell'altro, o usa «Collega a…» da tastiera
**Allora** nasce la dipendenza
**E** un collegamento che crea un ciclo è segnalato con i task coinvolti

**Dato** un grafo con errori (tipi, campi obbligatori, ciclo)
**Quando** il tenant preme «Salva»
**Allora** non salva, mostra ogni errore sul task che lo causa e un riepilogo in `role="alert"` («3 errori in 2 task») con i link
**E** gli esiti sono gli stessi dell'invio al motore (FR-4), e il lavoro resta solo nel kit

**Dato** un grafo valido
**Quando** il tenant preme «Salva»
**Allora** salvare è pubblicare: nasce una versione nuova, valida come documento OWS secondo lo schema di topictoast, e le posizioni dei nodi sono salvate come deciso per AR-91

**Dato** una versione pubblicata nel frattempo da un altro
**Quando** il tenant salva
**Allora** riceve il conflitto con «Ricarica ultima versione» e «Scarica le modifiche», mai una sostituzione silenziosa

**Dato** modifiche non salvate quando la credenziale scade
**Quando** arriva la credenziale rinnovata
**Allora** valgono le regole di FR-27 già usate dal kit (storia 6.6): il lavoro torna solo con lo stesso tenant e ruolo e resta solo in memoria

**Dato** il canvas con il fuoco
**Quando** il tenant usa la tastiera
**Allora** frecce, `Invio`, `Esc`, `Canc`, `C`, `A`, `+`, `−` e `0` fanno quello che dice la UX, con una sola fermata di `Tab` sul canvas
**E** la `list-view` resta l'alternativa in sola lettura per chi non usa il grafo

### Story 12.2: Configurare i task: dati, fan-out, retry e timeout

Come tenant,
voglio dire da dove prende i dati ogni campo di un task e configurare fan-out, retry e timeout dal pannello,
così che compongo il workflow di UJ-2 senza scrivere jq.

**Rispetta:** AD-8, AD-9, AD-10, AD-11
**Requisiti:** FR-40 (fonti dei dati, schema di input, fan-out e join), FR-56, FR-3, FR-7, FR-8, NFR-9; UX-DR-35, UX-DR-37, UX-DR-58, UX-DR-79, UX-DR-108 (UJ-2)
**Prova per l'utente:** comporre il workflow di UJ-2 (elenco di indicatori, `dns_lookup` e `censys_search` in parallelo per ogni indicatore, arricchimento, inserimento) senza scrivere jq, salvarlo e lanciarlo con 300 indicatori.
**Docs utente:** configurare i task e il fan-out.

**Acceptance Criteria:**

**Dato** un campo dello schema di input di un task
**Quando** il tenant ne sceglie la fonte nella sezione «Dati»
**Allora** può usare un valore fisso, un campo compatibile dell'input del workflow, dell'output di un task a monte o dell'elemento corrente di un fan-out, oppure un'espressione jq nel modo avanzato
**E** un campo obbligatorio senza fonte, un tipo incompatibile o un'espressione jq vietata sono errori sul campo

**Dato** un task che chiama un sotto-workflow
**Quando** il tenant ne configura i dati
**Allora** i campi sono quelli dello schema di input del workflow chiamato

**Dato** il pannello del workflow
**Quando** il tenant definisce lo schema di input
**Allora** lo schema usa il linguaggio di AD-10 e diventa l'`input.schema` del documento

**Dato** un salvataggio che cambia lo schema di input
**Quando** il tenant preme «Salva»
**Allora** un `confirm-dialog` elenca prima le pianificazioni e le pipeline stream che avviano il workflow
**E** dopo il salvataggio quelle pianificazioni e pipeline avviano subito la versione nuova (FR-5)

**Dato** un fan-out
**Quando** il tenant lo configura
**Allora** sceglie l'elenco, il parallelismo massimo e la politica d'errore, e il join mostra cosa riceve

**Dato** un task
**Quando** il tenant ne configura retry e timeout
**Allora** gli aiuti mostrano predefiniti e massimi dell'installazione, e valori oltre il massimo sono errori

**Dato** i costrutti `switch`, `for`, `fork`, `wait`, `raise`, `set`, `call http` e sotto-workflow
**Quando** il tenant li aggiunge dalla palette
**Allora** ciascuno ha il suo modulo nel pannello, e uno `switch` dentro un `dag` diventa un `do` annidato (AD-8)

### Story 12.3: Segreti nell'editor e gestione dei segreti

Come tenant,
voglio scegliere per ogni connessione un segreto tra quelli che la soddisfano e gestire i miei segreti dal kit,
così che collego le mie credenziali senza mai vederle in chiaro.

**Rispetta:** AD-7, AD-15
**Requisiti:** FR-40 (segreto per ogni connessione), FR-32 (selettore), FR-42, NFR-7, NFR-9; UX-DR-27, UX-DR-35, UX-DR-36, UX-DR-60
**Prova per l'utente:** nella sezione «Segreti» aggiungere «censys-prod» per Censys; nell'editor il selettore della connessione Censys di `censys_search` mostra solo «censys-prod»; il valore non compare mai, nemmeno dopo «Sostituisci».
**Docs utente:** segreti nel kit.

**Acceptance Criteria:**

**Dato** un task con una connessione richiesta dal manifest
**Quando** il tenant apre «Connessioni»
**Allora** il selettore mostra solo i segreti che soddisfano quella connessione
**E** se non ce ne sono, dice «Nessun segreto per Censys.» (o il nome della piattaforma)

**Dato** un task senza manifest, come `call http`
**Quando** il tenant apre il selettore
**Allora** vede tutti i suoi segreti, e l'attività riceverà solo quello scelto

**Dato** la sezione «Segreti»
**Quando** il tenant aggiunge, sostituisce o elimina un segreto
**Allora** l'elenco mostra nome, connessione, data e ultimo uso, mai il valore
**E** l'eliminazione chiede conferma nel `confirm-dialog`

### Story 12.4: Comporre dal telefono

Come tenant,
voglio comporre e modificare un workflow anche dal telefono,
così che posso intervenire quando non sono alla scrivania.

**Rispetta:** AD-15
**Requisiti:** NFR-16 (composizione nell'editor del grafo da 360 px), FR-40, NFR-9; UX-DR-52, UX-DR-82, UX-DR-101, UX-DR-103, UX-DR-104, UX-DR-105
**Prova per l'utente:** su un telefono largo 390 px comporre il workflow di UJ-2 e salvarlo: il grafo si dispone dall'alto in basso da solo, il pannello è un foglio dal basso e ogni bersaglio si tocca senza errori.
**Docs utente:** usare l'editor dal telefono.

**Acceptance Criteria:**

**Dato** un telefono da 360 px
**Quando** il tenant apre l'editor
**Allora** il canvas occupa tutta la larghezza, il grafo si dispone da solo dall'alto in basso e il pannello del task è un `bottom-sheet`

**Dato** `pointer: coarse`
**Quando** il tenant tocca porte, archi e controlli
**Allora** ogni bersaglio misura almeno `target-min`, con un'area di tocco più larga per porte e archi

**Dato** un campo con il fuoco
**Quando** si apre la tastiera virtuale
**Allora** il campo resta visibile sopra la tastiera e il foglio va a tutta altezza

**Dato** la pagina dell'ospite
**Quando** il kit gira sul telefono
**Allora** non usa mai `user-scalable=no`

### Story 12.5: Pianificazioni nel kit

Come tenant,
voglio creare e gestire dal kit la mia pianificazione di un workflow, con input e prossima scadenza,
così che faccio partire da solo un workflow senza usare l'SDK.

**Rispetta:** AD-5; C-3
**Requisiti:** FR-57 (kit), FR-6, NFR-9; UX-DR-38
**Prova per l'utente:** nel pannello del workflow di UJ-2 impostare «Ripeti ogni 1 ora» con un input: compare la prossima scadenza; «Sospendi» la mette in pausa; la pianificazione del documento, se c'è, si vede in sola lettura.
**Docs utente:** pianificare dal kit.

**Acceptance Criteria:**

**Dato** un workflow del tenant, dell'integratore o definito come codice
**Quando** il tenant imposta «Ripeti» (ogni intervallo o cron) con un input
**Allora** nasce la sua pianificazione e il pannello mostra la prossima scadenza
**E** il workflow non cambia

**Dato** una pianificazione esistente
**Quando** il tenant la modifica, la sospende, la riprende o la elimina
**Allora** il pannello lo riflette, con il badge «Sospesa» quando è in pausa

**Dato** scadenze perse, recuperate o rifiutate
**Quando** il tenant apre la pianificazione
**Allora** le vede, quelle rifiutate con il motivo

**Dato** la pianificazione del documento
**Quando** il tenant guarda il workflow
**Allora** la vede in sola lettura, distinta dalla propria

### Story 12.6: Editor delle pipeline stream nel kit

Come tenant,
voglio scrivere, validare, attivare e sospendere le mie pipeline stream dal kit e vedere le ultime righe emesse,
così che creo la pipeline di UJ-3 senza usare l'SDK.

**Rispetta:** AD-11 (SQL validato a secco dal motore, non dal nucleo JS), AD-13, AD-15; C-7
**Requisiti:** FR-43, FR-24 (scelta della destinazione), NFR-7, NFR-9, NFR-16; UX-DR-32, UX-DR-61, UX-DR-80, UX-DR-109 (UJ-3)
**Prova per l'utente:** nella sezione «Pipeline stream» premere «Nuova pipeline stream», scrivere la query di UJ-3 con un errore: compare con riga e colonna prima di salvare; corretta, «Salva e attiva» la avvia e le ultime righe emesse compaiono nella tabella.
**Docs utente:** pipeline stream nel kit.

**Acceptance Criteria:**

**Dato** una query con errori
**Quando** il tenant la scrive nel `code-editor`
**Allora** gli errori di validazione compaiono con riga e colonna prima del salvataggio, dalla validazione a secco del motore
**E** «Salva e attiva» resta bloccato finché ci sono errori

**Dato** una validazione a secco che fallisce per limite di risorse
**Quando** il tenant salva
**Allora** il kit mostra l'errore con il suo codice e l'editor resta usabile
**E** nessun testo nomina il processo o il motore

**Dato** una query valida
**Quando** il tenant sceglie la destinazione («Stream del tenant» o «Workflow») e preme «Salva e attiva»
**Allora** la pipeline parte e il kit mostra stato e ultime righe emesse

**Dato** una pipeline attiva
**Quando** il tenant ne modifica la query e salva
**Allora** un `confirm-dialog` avvisa che la modifica azzera lo stato della pipeline stream

**Dato** il motore in ripresa dopo un crash
**Quando** il tenant guarda l'editor
**Allora** un avviso neutro dice che gli aggiornamenti sono in pausa e riprendono da soli; stato e ultime righe restano visibili e nessun testo nomina il motore o il crash

**Dato** il telefono
**Quando** il tenant scrive la query
**Allora** il `code-editor` va a tutto schermo, con correzione automatica e maiuscole automatiche spente

## Epic 13: Pronto per il rilascio

L'integratore aggiorna topictoast senza perdite, una persona nuova arriva al primo workflow in 15
minuti con la guida rapida, e il benchmark pubblico dimostra gli obiettivi.

### Story 13.1: Aggiornare topictoast senza perdere stato

Come integratore,
voglio aggiornare topictoast, bus compreso, a una versione nuova senza perdere nulla,
così che seguo i rilasci senza fermare i miei clienti più del necessario.

**Rispetta:** AD-2, AD-4 (ogni cambio della spec OWS è una migrazione), AD-17
**Requisiti:** FR-51, NFR-6; AR-95 (versione del formato di messaggi del bus, payload, checkpoint ed export), AR-72
**Prova per l'utente:** con esecuzioni in corso e una pipeline stream attiva, aggiornare il motore e il bus alla versione successiva seguendo la guida: dopo il riavvio le esecuzioni riprendono, la pipeline riparte dal checkpoint, come deciso dalla spec della prima storia di aggiornamento, e il benchmark conta zero perdite.
**Docs utente:** aggiornare topictoast.

**Acceptance Criteria:**

**Dato** un'installazione con esecuzioni in corso
**Quando** l'integratore avvia la versione nuova del motore
**Allora** lo stato viene migrato da solo e le esecuzioni in corso riprendono dopo il riavvio

**Dato** l'aggiornamento di un qualunque componente, bus compreso
**Quando** finisce
**Allora** nessuna attività accettata e nessuna esecuzione in corso va persa
**E** il motore nuovo rilegge payload e checkpoint scritti dalla versione precedente, nel formato versionato deciso per AR-95

**Dato** una pipeline stream attiva
**Quando** il bus è aggiornato
**Allora** il comportamento sugli eventi non ancora letti e sulle chiavi C-4 senza finestra è quello deciso dalla spec della prima storia di aggiornamento (domanda aperta sull'aggiornamento)

**Dato** un workflow accettato da una versione
**Quando** gira su una versione successiva della stessa major
**Allora** resta eseguibile (PRD §6)

**Dato** una rottura di compatibilità in un rilascio
**Quando** esce
**Allora** le note di rilascio la annunciano con la via di migrazione, e fino alla 1.0 alza la minor

### Story 13.2: Metriche complete

Come operatore,
voglio che le metriche coprano anche code per tenant e pipeline stream,
così che porto nei miei strumenti tutto quello che vedo nella dashboard.

**Rispetta:** C-9, C-14
**Requisiti:** FR-52 (metriche complete)
**Prova per l'utente:** con la pipeline di UJ-3 attiva, `curl` su `/metrics` mostra per la pipeline ritardo sul watermark, eventi al secondo, memoria di stato, eventi in ritardo scartati e ultimo checkpoint.
**Docs utente:** riferimento delle metriche.

**Acceptance Criteria:**

**Dato** `/metrics`
**Quando** lo si legge
**Allora** copre attività al secondo, latenza di avvio delle attività, code per tenant e, per ogni pipeline stream, le stesse grandezze della pagina della dashboard (FR-47)
**E** tenant e pipeline compaiono solo con identificativi opachi

**Dato** un'installazione che ha usato tutte le funzioni (segreti, nomi visualizzati, input e output)
**Quando** si cercano i valori sentinella nei log
**Allora** non si trovano

### Story 13.3: Benchmark pubblico riproducibile

Come integratore che valuta topictoast,
voglio un benchmark che chiunque esegue con un solo comando sulla macchina di riferimento,
così che verifico da solo i numeri promessi.

**Rispetta:** AD-1, AD-2; `crates/bench` (AD-15)
**Requisiti:** FR-53, NFR-1, NFR-2, NFR-3, NFR-4, NFR-5, NFR-6, NFR-15; SM-2, SM-3
**Prova per l'utente:** sulla macchina di riferimento lanciare il comando del benchmark: il rapporto riporta RAM a riposo, latenze p50 e p99, attività al secondo, istruzioni SQL per attività e al giorno, eventi al secondo, perdite dopo il guasto e tempo di ripresa, con SQLite e con Postgres.
**Docs utente:** eseguire il benchmark.

**Acceptance Criteria:**

**Dato** la macchina di riferimento
**Quando** si lancia il benchmark con un solo comando
**Allora** il rapporto riporta, con SQLite e con Postgres: RAM a riposo (NFR-1), latenza di avvio p50 e p99 a 1.000 attività al secondo (NFR-2), attività al secondo sostenute per 10 minuti (NFR-3), istruzioni SQL per attività (NFR-4), istruzioni SQL al giorno con il carico di hatchet-dev/hatchet#5153 (SM-2) ed eventi al secondo di una pipeline (NFR-5)
**E** ogni valore è confrontato con il suo obiettivo

**Dato** un guasto iniettato durante le corse di NFR-3 e NFR-5
**Quando** finisce la corsa
**Allora** il rapporto conta attività, esecuzioni ed eventi accettati e poi persi, con obiettivo zero (NFR-6)
**E** misura il tempo di ripresa dopo un crash iniettato del motore (NFR-15)

**Dato** la corsa a carico misto
**Quando** gira
**Allora** misura la latenza di NFR-2 mentre una pipeline stream gira al ritmo dichiarato, un tenant è oltre quota, un tenant avvia a ripetizione con gli schemi più complessi consentiti e un'attività scrive righe di log fino al tetto (FR-59)
**E** le corse di NFR-2 e NFR-3 comprendono il controllo di input e output sugli schemi dei manifest

**Dato** il rapporto
**Quando** viene prodotto
**Allora** dichiara forma del carico, dimensione dei payload, insieme di dati e modalità di scrittura, e riporta senza obiettivo la RAM sotto carico, la RAM per pipeline attiva e il tempo di riavvio a freddo dei processi ausiliari, incluso il ruolo della validazione a secco

**Dato** la corsa a carico misto
**Quando** una validazione a secco con una query ostile gira mentre le pipeline elaborano
**Allora** non le ferma

### Story 13.4: Guida rapida: primo workflow in 15 minuti

Come persona che non conosce topictoast,
voglio una guida rapida che mi porta dal download a un workflow di esempio completato,
così che capisco in un quarto d'ora se topictoast fa per me.

**Rispetta:** AD-1
**Requisiti:** NFR-13, FR-50 (formato di distribuzione), FR-45; SM-5; UX-DR-111 (UJ-5); AR-103
**Prova per l'utente:** su una macchina pulita seguire la guida alla lettera: installazione, avvio, worker ed esempio, accesso alla dashboard con la credenziale del primo avvio, esecuzione «Completata» nella pagina Esecuzioni.
**Docs utente:** guida rapida.

**Acceptance Criteria:**

**Dato** una macchina pulita e il formato di distribuzione deciso per AR-103
**Quando** una persona che non conosce topictoast segue la guida
**Allora** arriva al workflow di esempio completato, visto nella dashboard, entro 15 minuti

**Dato** la credenziale del primo avvio smarrita
**Quando** la persona apre la dashboard
**Allora** la pagina d'accesso dice dove è stata scritta

**Dato** almeno tre persone che non conoscono topictoast
**Quando** seguono la guida prima del primo rilascio pubblico
**Allora** tutte arrivano in fondo entro l'obiettivo di NFR-13, e i punti dove si sono fermate correggono la guida

### Story 13.5: Suite di sicurezza completa prima del rilascio

Come integratore,
voglio che prima del primo rilascio pubblico giri la suite completa di sicurezza,
così che rilascio sapendo che tenant, sandbox e segreti reggono i casi noti.

**Rispetta:** AD-5, AD-6, AD-7, AD-13, AD-19
**Requisiti:** NFR-7, NFR-8
**Prova per l'utente:** lanciare la suite di sicurezza con il comando documentato: il rapporto elenca per ogni tipo di risorsa e punto di accesso il test tra tenant, i test di codice ostile ai due livelli della sandbox e il test dei payload di script, tutti verdi.
**Docs utente:** modello di sicurezza di topictoast.

**Acceptance Criteria:**

**Dato** ogni tipo di risorsa e ogni punto di accesso (API, SDK, eventi in tempo reale, liste e conteggi)
**Quando** gira la suite
**Allora** c'è un test che prova che un tenant non legge né modifica le risorse di un altro, che una credenziale di tenant è rifiutata su ogni operazione dell'integratore e che una credenziale dell'integratore firmata per il kit è rifiutata fuori dal suo ambito

**Dato** la sandbox ai due livelli
**Quando** gira il codice ostile noto (fuga, esaurimento delle risorse, rete interna, lettura della chiave)
**Allora** nessun caso riesce

**Dato** la lista delle query ostili (SQL)
**Quando** gira
**Allora** il controllo sull'albero le rifiuta tutte e ogni voce nuova del dialetto passa da lì

**Dato** un processo ausiliario per ruolo (validazione a secco, pipeline stream)
**Quando** la suite lo ispeziona
**Allora** la validazione a secco non ha accesso al bus e le pipeline non hanno accesso a stream di altri tenant
**E** una destinazione stream o uno stream di un altro tenant dà lo stesso errore di uno inesistente

**Dato** un payload di script salvato in ogni campo testuale di un tenant
**Quando** si apre ogni pagina della dashboard e ogni componente del kit, con entrambi i ruoli del kit
**Allora** nessuno script gira

**Dato** ogni connessione di rete che porta segreti, credenziali o dati dei tenant
**Quando** la suite la prova
**Allora** è cifrata e autentica il server, e in chiaro è accettata solo da localhost

### Story 13.6: Accessibilità verificata e prove con persone

Come tenant e come operatore,
voglio che kit e dashboard siano verificati con lettori di schermo e provati da persone reali,
così che li uso davvero, anche senza mouse e senza documentazione.

**Rispetta:** AD-15; skill `dashboard-react` e skill del kit
**Requisiti:** NFR-9, NFR-16; SM-4, SM-6; UX-DR-89…UX-DR-99, UX-DR-105
**Prova per l'utente:** il rapporto della verifica elenca il controllo automatico di ogni componente, le prove con NVDA e VoiceOver su UJ-2 e UJ-4 e l'esito delle prove con tre persone del profilo di Giulia.
**Docs utente:** accessibilità di kit e dashboard.

**Acceptance Criteria:**

**Dato** ogni componente del kit e della dashboard
**Quando** gira il controllo automatico di accessibilità
**Allora** rispetta WCAG 2.1 AA con i colori predefiniti

**Dato** i percorsi UJ-2 (costruzione del workflow compresa) e UJ-4
**Quando** si provano con NVDA e VoiceOver
**Allora** si completano senza vedere lo schermo

**Dato** tre persone del profilo di Giulia, da sole e senza documentazione, su desktop
**Quando** compongono, salvano e lanciano il workflow di UJ-2 (almeno 5 task, fan-out e join)
**Allora** ci riescono tutte e tre entro 20 minuti (SM-6)

**Dato** l'app di Marco
**Quando** si eseguono UJ-1, UJ-2, UJ-3 e UJ-4 da capo a capo
**Allora** si completano prima del primo rilascio pubblico (SM-4)

### Story 13.7: Piattaforme e licenze

Come integratore,
voglio binari per Linux x86_64 e arm64 e la garanzia che ogni dipendenza abbia una licenza compatibile,
così che installo topictoast sui miei server e lo uso senza vincoli.

**Rispetta:** AD-1
**Requisiti:** NFR-10, NFR-11; AR-106 (licenze ammesse)
**Prova per l'utente:** scaricare i binari di un rilascio per x86_64 e arm64 e avviarli con la guida rapida; il controllo delle licenze del rilascio non riporta dipendenze fuori dall'elenco ammesso.
**Docs utente:** piattaforme supportate e licenza.

**Acceptance Criteria:**

**Dato** un rilascio
**Quando** viene costruito
**Allora** produce motore e processi ausiliari per Linux x86_64 e arm64, e la guida rapida funziona su entrambi

**Dato** le dipendenze dei lockfile di Cargo e npm
**Quando** gira il controllo delle licenze
**Allora** ogni licenza è nell'elenco deciso per AR-106, e una dipendenza fuori elenco fa fallire il controllo
**E** tutto il codice di topictoast è Apache-2.0, senza parti a pagamento

### Story 13.8: Archivio dei payload su RustFS

Come integratore,
voglio tenere payload e checkpoint su RustFS invece che su disco,
così che lo storico sta su uno storage a oggetti che già gestisco.

**Rispetta:** AD-16 (client HTTP di RustFS in un solo modulo di `crates/adapters`), AD-17; C-14
**Requisiti:** FR-50 (i servizi obbligatori restano due più il database), FR-31 (pulizia e prova sentinella anche su RustFS), NFR-6, NFR-8; AR-38
**Prova per l'utente:** configurare il motore con il RustFS del `docker compose`: l'esempio dell'Epic 1 e la pipeline di UJ-3 girano uguali e i payload stanno nel bucket, cifrati.
**Docs utente:** archivio dei payload su RustFS.

**Acceptance Criteria:**

**Dato** RustFS configurato come archivio dei payload
**Quando** girano un'esecuzione e una pipeline stream
**Allora** payload e checkpoint vanno su RustFS nei due spazi sotto l'identificativo interno del tenant (immutabile per impronta, con nome per checkpoint ed export), cifrati con la chiave del tenant

**Dato** la configurazione predefinita
**Quando** il motore parte
**Allora** l'archivio resta su disco: RustFS è facoltativo

**Dato** un'esecuzione scaduta o un tenant eliminato
**Quando** passa la pulizia
**Allora** i suoi oggetti spariscono anche da RustFS e la prova sentinella non trova nulla

**Dato** RustFS irraggiungibile
**Quando** il proiettore deve scrivere un payload
**Allora** il motore si comporta come con il database fermo: lavoro nuovo rifiutato con un codice, nessun lavoro accettato perso (AD-2)
**E** fuori da localhost la connessione usa TLS e verifica il certificato
