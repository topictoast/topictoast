# Riconciliazione PRD → spine UX

Input: `prds/prd-topictoast-2026-11-19/prd.md` + `addendum.md`.
Contro: `DESIGN.md`, `EXPERIENCE.md`, `.memlog.md` (stessa cartella). Nessun file esistente modificato.

Sigle: DS = DESIGN.md, EX = EXPERIENCE.md. Gravità: alta = il percorso o un FR della v1 non si può
costruire o si costruisce in modo sbagliato; media = comportamento o testo mancante che il
designer o lo sviluppatore dovrà inventare; bassa = rifinitura.

Totale: 38 voci, 5 alte, 18 medie, 15 basse. (a) 20 · (b) 12 · (c) 6. La sezione (d) elenca
decisioni, non voci di gravità.

---

## (a) Requisiti con conseguenza UI che nessuna sezione delle spine copre

### A-01 · alta · Editor delle pipeline stream: solo SQL e destinazione, mancano i parametri e gli stream
- **PRD.** FR-18 (stream del cliente, conservazione breve), FR-19 (query su uno o più stream;
  dialetto chiuso; ciclo tra pipeline stream rifiutato), FR-20 (finestre fisse e scorrevoli), FR-21
  («dichiara quanto ritardo tollera»), FR-22 (intervallo checkpoint configurabile), FR-24
  (destinazione: stream del cliente **oppure** workflow), FR-43 («scegliere la destinazione»).
- **Spine.** EX Key Flow UJ-3 e `code-editor`: solo query SQL e destinazione «avvia il workflow…».
  Nessun elenco degli stream del cliente, nessun campo per tipo di finestra, ritardo tollerato,
  checkpoint; la destinazione «stream» non è descritta; mancano i messaggi di FR-18 (finestra oltre
  la conservazione, con i due valori), FR-19 (dialetto non consentito, stream di altro cliente come
  inesistente, ciclo tra pipeline stream), FR-24 (righe in attesa oltre la quota).
- **Dove atterra.** EX Interaction Primitives (nuova voce «Pipeline stream»), Component Patterns
  (`form-field` per i parametri; un `palette-item`-like o selettore per gli stream), Voice and Tone
  (tabella messaggi), State Patterns (vuoto: «Nessuno stream disponibile»).
- **Fix.** Aggiungere un pannello «Parametri» accanto al `code-editor`: stream di ingresso
  (elenco di sola lettura con conservazione), finestra, ritardo tollerato, checkpoint, destinazione
  (stream o workflow). Aggiungere i quattro messaggi mancanti. Mostrare nello stato della pipeline
  stream anche il numero di righe in attesa (FR-24).

### A-02 · alta · Trigger e pianificazioni senza superficie
- **PRD.** FR-6 (trigger: API/SDK, pianificazione cron o a intervallo, riga di pipeline stream;
  `[ASSUMPTION]` ma nella v1), FR-26 (ogni trigger ha un cliente fissato alla creazione), FR-31
  (eliminare un cliente ferma le pianificazioni), NFR-14 («recupero delle scadenze perse di una
  pianificazione»). Glossario: **Trigger** è un termine vincolante.
- **Spine.** Nessun cenno a pianificazioni in DS/EX (la parola `cron` compare solo nel nome di un
  componente). L'unico avvio descritto è «Esegui» manuale. Nessun campo «Avviata da» per tipo di
  trigger; nessun elenco delle pianificazioni; nessun testo per «scadenza persa».
- **Dove atterra.** EX Information Architecture (voce nell'elenco Workflow o sezione nell'editor),
  Component Patterns (`form-field` cron/intervallo, `data-table` pianificazioni), State Patterns,
  barra di un'esecuzione («Avviata da pianificazione…»).
- **Fix.** Decidere con Marco se la pianificazione si crea dall'editor del cliente o solo via SDK
  (FR-38). Se dall'editor: sezione «Pianificazione» nel pannello del workflow. In ogni caso la
  barra dell'esecuzione dice il tipo di trigger (manuale, pianificazione, pipeline stream «<nome>»).

### A-03 · alta · Costrutti OWS di FR-1 e nodi di partenza: la palette ha solo «Controllo di flusso» generico
- **PRD.** FR-1 (do, set, switch, fork, for, try con retry, wait, raise, call http, sotto-workflow,
  espressioni jq), FR-2 (dipendenze arbitrarie, «forma a N»), FR-40 (aggiungere task… controlli di
  flusso; il salvato è un documento OWS valido), §4.9 (Workflow Builder «con un solo nodo di
  partenza» come riferimento da superare), domande aperte 7 e 8 (invocazione funzioni, propagazione
  fallito/saltato con `then` e `switch`).
- **Spine.** EX Interaction Primitives: «Controllo di flusso raccoglie i costrutti di FR-1 che non
  sono funzioni né nodi» [ASSUMPTION] senza dire come si disegnano `switch` (rami condizionali),
  `for`, `try/retry`, `wait`, `raise`, jq. Nessuna regola sui nodi di partenza multipli. L'esempio
  UJ-2 inventa «Ricevi elenco».
- **Dove atterra.** EX Component Patterns (nuovi `node-card` per ramo condizionale e per
  attesa/errore; campo espressione jq in `form-field` con regole FR-17), DS Components, EX
  Interaction Primitives.
- **Fix.** Elenco chiuso dei task di «Controllo di flusso» con il loro aspetto nel grafo e i campi
  del pannello; dichiarare che il grafo può avere più nodi di partenza; segnalare nelle assunzioni
  l'incrocio con la domanda aperta 8. Campo jq: errore di validazione per `$ENV`, `env`,
  `input_filename`, `import`, `include` (FR-17).

### A-04 · alta · Workflow dell'integratore e ruolo «integratore» nel kit
- **PRD.** FR-28 (due ruoli: l'integratore vede tutti i clienti), FR-29 (workflow dell'integratore
  disponibile a tutti i clienti; il cliente non lo modifica; un nodo omonimo del cliente non cambia
  ciò che esegue), FR-26 (risoluzione dei riferimenti).
- **Spine.** Nessuna distinzione di proprietà nell'elenco Workflow né nell'editor: niente stato
  «sola lettura», niente etichetta di provenienza, nessun selettore di cliente quando il kit è usato
  con la credenziale di ruolo integratore. Nella palette un nodo del cliente e uno
  dell'integratore con lo stesso nome non sono distinguibili.
- **Dove atterra.** EX State Patterns (editor in sola lettura), Component Patterns (`chip`
  «Dell'integratore» nell'elenco e nella `workflow-bar`; `palette-item` con origine), IA (selettore
  cliente per il ruolo integratore).
- **Fix.** Aggiungere: chip di provenienza, editor in sola lettura con «Esegui» ancora attivo e il
  messaggio «Workflow dell'integratore: non modificabile», regola per i nomi omonimi in palette.
  Decidere se il kit con ruolo integratore mostra un selettore di cliente.

### A-05 · media · Versioni dei workflow e dei nodi
- **PRD.** FR-5 (nuova versione mentre ne girano altre; «salvo richiesta esplicita di una
  versione»; conflitto → errore con versione corrente), FR-15 (nodo omonimo = nuova versione; ogni
  attività registra la versione del nodo).
- **Spine.** Solo il `chip` «versione 3» e il messaggio di conflitto. Non c'è cronologia delle
  versioni, scelta della versione da eseguire, indicazione della versione con cui è partita ogni
  esecuzione, versione del nodo personalizzato nel pannello/`attempt-card`. Il conflitto non dice
  cosa può fare l'utente: ricaricare, copiare il proprio lavoro, rinominare.
- **Dove atterra.** EX Interaction Primitives «Salvare ed eseguire», Component Patterns
  (`workflow-bar`, `attempt-card`, pannello destro), State Patterns (conflitto).
- **Fix.** «Esegui» con scelta versione (predefinita: ultima); barra esecuzione con «versione N»
  fissa; pannello del task con versione del nodo; sul conflitto, due azioni («Ricarica ultima
  versione», «Scarica la mia») per non perdere il lavoro.

### A-06 · media · Limiti, valori predefiniti e messaggi di superamento
- **PRD.** FR-11 (input/output 1 MB; tetto del join con dimensione e limite), FR-17 (errore che
  nomina il limite superato: CPU, memoria, tempo; jq), NFR-14 (valori predefiniti applicati dopo la
  validazione; massimo fissato dall'installazione per ogni tempo o limite dichiarato dal cliente;
  limiti su elementi di fan-out, dimensione documento e query, numero di task, annidamento,
  risposta `call http`), FR-8 (attesa prima della consegna limitata da un tempo documentato).
- **Spine.** Tabella messaggi: solo «Output di 1,4 MB oltre il limite di 1 MB» e il timeout del
  nodo. Mancano: join oltre il tetto, CPU/memoria, dimensione documento/query, annidamento, elementi
  del fan-out, risposta `call http`, e cosa mostra un task rimasto in coda oltre il tempo massimo di
  attesa. Il pannello del task non mostra il valore predefinito di retry/timeout/parallelismo né
  rifiuta un valore sopra il massimo.
- **Dove atterra.** EX Voice and Tone (tabella), Component Patterns (`form-field`:
  segnaposto = valore predefinito, errore sul massimo), State Patterns (stato di un task scaduto
  in coda).
- **Fix.** Un messaggio generico con schema «<limite> superato: <valore> oltre <massimo>» più
  esempi per ciascun limite; `form-field` mostra «predefinito: 30 s, massimo: 10 min» (valori letti
  dall'installazione, non scritti nella UI); specificare lo stato terminale per l'attesa massima.

### A-07 · media · Task saltati: nessuna spiegazione del perché
- **PRD.** FR-2 («un task la cui dipendenza è fallita o saltata… stato terminale secondo una regola
  documentata»), domanda aperta 8.
- **Spine.** Stato «saltato» definito, ma il pannello del task non dice quale dipendenza ha causato
  il salto; l'arco verso un task saltato non ha trattamento.
- **Dove atterra.** EX State Patterns (Task), `node-card`, pannello destro.
- **Fix.** Nel pannello di un task saltato: «Saltato: dipendenza <task> fallita» con link al task.
  Rivedere quando la domanda 8 è chiusa.

### A-08 · media · Avvio di esecuzioni durante la ripresa del motore
- **PRD.** NFR-15 («durante la ripresa non partono nuove esecuzioni né task successivi: è una
  pausa»), FR-6 («ogni avvio restituisce subito l'identificativo»).
- **Spine.** Pausa descritta solo come aggiornamenti fermi. Non dice cosa fa «Esegui» (attivo?
  disattivato? accodato?) né cosa vede Giulia dopo averlo premuto.
- **Dove atterra.** EX State Patterns «Ripresa del motore», `workflow-bar`, `pause-notice`.
- **Fix.** Decidere: «Esegui» resta attivo e mostra «In coda» (coerente con FR-6/FR-9); in
  alternativa disattivato con motivo. Una riga nella tabella messaggi.

### A-09 · media · Disco pieno e rifiuto di lavoro nuovo
- **PRD.** NFR-6 («a disco pieno topictoast rifiuta lavoro nuovo con un errore esplicito»), FR-30
  (errore di quota prima dell'accettazione; spazio occupato come quota).
- **Spine.** Solo «richiesta oltre il tetto riceve un errore di quota». Nessun testo per l'utente
  del kit quando «Esegui» o «Salva» è rifiutato; nessun segnale all'operatore per disco pieno.
- **Dove atterra.** EX Voice and Tone (tabella), State Patterns «Quota raggiunta»; Dashboard
  (banner o riga in Quote/Worker).
- **Fix.** Due messaggi neutri nel kit («Richiesta non accettata: spazio esaurito»); un avviso
  nella dashboard distinto dal `recovery-banner`.

### A-10 · media · Operatore: esecuzioni bloccate e clienti «vicini» alla quota
- **PRD.** §2.1 Operatore («accorgersi di esecuzioni bloccate, pipeline stream in ritardo, clienti
  oltre quota»), UJ-4 («clienti vicini alla quota»), FR-30 (elenco delle quote), FR-49.
- **Spine.** Esecuzioni della dashboard ha solo filtri Cliente/Workflow/Stato; nessun indicatore
  «ferma da molto» né filtro per attesa lunga. Quote: «uso per cliente» e «Al limite», senza soglia
  «vicino», senza colonne per le sette quote di FR-30 (attività concorrenti, esecuzioni/min,
  sandbox, pipeline stream, eventi/s, richieste API/s, spazio).
- **Dove atterra.** EX Component Patterns `data-table`, State Patterns (Cliente), Information
  Architecture (Dashboard).
- **Fix.** Colonna «In coda da» e filtro stato «in coda» con ordinamento; Quote con una colonna per
  quota, barra di uso e due soglie («Vicino al limite», «Al limite»).

### A-11 · media · Azioni distruttive e con effetti nascosti (pipeline stream, segreti)
- **PRD.** FR-23 (modificare la query crea una nuova versione con stato **vuoto**; sospesa/ripresa
  con eventi già usciti → errore visibile), FR-32/42 (eliminare un segreto), FR-4 (riferimento a
  segreto inesistente segnalato).
- **Spine.** Conferme di «Annulla» ed «Elimina» non decise (EX domande 2). Nessun avviso prima di
  salvare una modifica che azzera lo stato di una pipeline stream; nessuna indicazione dei workflow
  che usano un segreto prima di eliminarlo.
- **Dove atterra.** EX Interaction Primitives, Component Patterns (`button-primary`/dialogo di
  conferma, assente dal sistema), Voice and Tone.
- **Fix.** Aggiungere un componente di conferma e tre testi: «Annullare l'esecuzione?», «La
  modifica azzera lo stato della pipeline stream», «Il segreto è usato da N workflow».

### A-12 · media · Percorso UJ-5 / NFR-13: nessun primo avvio guidato
- **PRD.** UJ-5, NFR-13 (≤ 15 minuti), SM-5 (tre persone esterne), FR-50 (workflow di esempio senza
  modifiche), FR-45 (credenziale generata al primo avvio).
- **Spine.** UJ-5 è l'unico flusso senza superficie propria: l'esito in Dashboard → Esecuzioni è
  un'assunzione. Nessun stato vuoto che rimandi alla guida («Nessun worker collegato» non dice come
  collegarne uno); l'accesso dice «dove viene scritta» la credenziale solo per assunzione.
- **Dove atterra.** EX Key Flows UJ-5, State Patterns (vuoti di Worker, Esecuzioni), Accesso.
- **Fix.** Uno stato vuoto di Worker/Esecuzioni con un'azione «Apri la guida rapida» (testo
  sostituibile) e il workflow di esempio visibile subito nell'elenco; la pagina Accesso nomina il
  punto esatto in cui la credenziale è stampata.

### A-13 · media · Messaggi generati dal motore: lingua e sostituzione (FR-44)
- **PRD.** FR-44 («i testi sono tutti sostituibili; italiano e inglese»), FR-4/FR-19 (errori con
  percorso, riga e colonna), FR-17, FR-11.
- **Spine.** La tabella messaggi è del kit, ma gli errori di validazione e di limite nascono nel
  motore (e nel nucleo JS). Non è detto chi li traduce né come l'ospite li sostituisce.
- **Dove atterra.** EX Voice and Tone, Personalizzazione dell'ospite.
- **Fix.** Specificare che il motore restituisce codici di errore e parametri strutturati, e che il
  kit li traduce con un dizionario sostituibile; l'esempio «Riga 4, colonna 9: …» è il rendering
  della coppia codice + parametri. Il meccanismo resta all'architettura.

### A-14 · media · Avviso «sandbox forte non attivabile» (FR-54)
- **PRD.** FR-54 («un avviso all'operatore» su host senza KVM).
- **Spine.** Già nota: EX Elementi aperti, domande 1. Confermata come lacuna.
- **Dove atterra.** Dashboard, Worker o testata di ogni pagina.
- **Fix.** Riga persistente in Worker («Sandbox forte non attiva: KVM non disponibile; in uso la
  sandbox leggera») con `state-badge` di avviso; non un banner modale.

### A-15 · bassa · Esportazione e lettura del documento OWS
- **PRD.** FR-35 («leggibile ed esportabile»), FR-36, §4.1 (workflow da codice e da editor).
- **Spine.** Nessuna azione «Esporta»/«Importa» né vista del documento. Il layout automatico evita
  il problema delle posizioni, ma non lo dice.
- **Dove atterra.** `workflow-bar`, menu del workflow.
- **Fix.** Azione secondaria «Esporta documento» (sola lettura). Importare resta via SDK. Annotare
  che le posizioni non fanno parte del documento.

### A-16 · bassa · Conservazione dello storico e cliente eliminato
- **PRD.** FR-31 (durata configurabile, 30 giorni predefinito; esecuzioni più vecchie eliminate).
- **Spine.** Il testo del vuoto dice «Nessuna esecuzione negli ultimi 30 giorni» con il numero
  scritto a mano; nessuno stato per un'esecuzione aperta da un collegamento ma già eliminata.
- **Dove atterra.** EX State Patterns (Vista delle esecuzioni).
- **Fix.** Il numero viene dalla configurazione; il collegamento a un'esecuzione eliminata mostra
  lo stesso «non trovata» di FR-26.

### A-17 · bassa · Annullamento: stato dell'attività e azione dell'operatore
- **PRD.** FR-8/FR-10 (attività in corso annullate), FR-28 (integratore con l'operatore).
- **Spine.** Elenco degli stati: l'Attività non ha «annullata»; la dashboard non dice se
  l'operatore può annullare dall'esecuzione.
- **Dove atterra.** EX State Patterns, `workflow-bar` della dashboard.
- **Fix.** Aggiungere «annullata» alle attività; dichiarare l'azione «Annulla» anche nel dettaglio
  della dashboard (stesso componente del kit).

### A-18 · bassa · Rifiuti dal server dopo «Salva» e ricontrollo a runtime
- **PRD.** FR-4 (riferimenti a funzioni/segreti inesistenti segnalati all'invio; il controllo si
  ripete all'esecuzione), FR-39 (stesso esito del nucleo JS), §4.1.
- **Spine.** «Salva valida con il nucleo JS». Un rifiuto del motore (funzione tolta dall'integratore
  dopo la modifica, versione) non ha flusso; né l'esecuzione che fallisce all'avvio perché una
  funzione non c'è più.
- **Dove atterra.** EX Interaction Primitives «Salvare ed eseguire», State Patterns.
- **Fix.** Descrivere un solo percorso: errori dal motore compaiono come quelli del nucleo JS,
  sul task; una funzione scomparsa fa fallire il task con lo stesso messaggio di «funzione
  inesistente».

### A-19 · bassa · Tastiera sulle tabelle e sulla dashboard (NFR-9)
- **PRD.** NFR-9 (editor, vista esecuzioni, segreti **e dashboard**).
- **Spine.** Tastiera dettagliata solo per il canvas. `data-table`: «Un clic su una riga apre il
  dettaglio» senza equivalente da tastiera; nessuna regola per i filtri e per i `tabs` della
  dashboard.
- **Dove atterra.** EX Component Patterns `data-table`, Accessibility Floor.
- **Fix.** Riga = link/button focalizzabile con `Invio`; ordinamento in intestazioni
  `button` con `aria-sort`.

### A-20 · bassa · Prestazioni dell'interfaccia con grafi grandi
- **PRD.** FR-41 (fan-out da migliaia leggibile), NFR-14 (limite sul numero di task), §1 vision
  («velocità, poca memoria»).
- **Spine.** Raggruppamento del fan-out coperto; nessun budget per il canvas con molti task né per
  gli aggiornamenti a 1 s (frequenza massima, aggregazione).
- **Dove atterra.** EX Tempo reale.
- **Fix.** Una riga: aggiornamenti raggruppati, il canvas non ridisegna i nodi non visibili.

---

## (b) Contraddizioni tra spine e PRD

### B-01 · alta · «Avviata da Giulia» / «annullata da Giulia» contro l'identità di FR-27
- **PRD.** FR-10 («con chi l'ha annullata e quando»); FR-26/27/28: topictoast conosce solo cliente e
  ruolo dalla credenziale firmata, «non autentica gli utenti della piattaforma ospite» (§4.6).
- **Spine.** EX Voice and Tone («Avviata alle 14:22 da Giulia», «Esecuzione annullata da Giulia
  alle 14:31»), `workflow-bar`.
- **Problema.** Il nome di una persona non è un dato del motore. Il mock mostra un dato che non
  esiste nella v1 (o presume claim aggiuntivi nella credenziale).
- **Fix.** «Chi» = cliente o integratore; se serve il nome dell'utente, chiedere a Marco di
  estendere la credenziale (claim facoltativo «nome visualizzato») e annotarlo come modifica
  del PRD. Fino ad allora: «annullata dal cliente alle 14:31».

### B-02 · media · «In attesa» e «in coda» scambiati rispetto al PRD
- **PRD.** FR-8/FR-12 usano «restano in attesa» per l'attesa prima della consegna; FR-30 usa «in
  coda» per la quota. UJ-1: «le attività restano in attesa».
- **Spine.** EX State Patterns: «in attesa» = dipendenze non finite; «in coda» = pronto ma non
  consegnato (worker, quota, pool). EX UJ-1 lo riconosce.
- **Problema.** Il vocabolario della UX ridefinisce un termine che il PRD usa con l'altro
  significato; il glossario (§3) non fissa i nomi degli stati.
- **Fix.** Chiedere a Marco di aggiungere gli stati al glossario del PRD con le definizioni delle
  spine, oppure adottare «in attesa di dipendenze» / «in attesa di consegna» con la riga del motivo.

### B-03 · media · Stati della pipeline stream: «in errore» assorbe «sospesa con errore»
- **PRD.** FR-25 («viene sospesa con un errore visibile»), FR-23 (sospesa è una scelta del cliente),
  FR-30 («smette di emettere con un errore»).
- **Spine.** Tabella degli stati: «attiva · rallentata · sospesa · in errore»; i tre casi di errore
  (memoria, arretrato, conservazione) sono «in errore» e non «sospesa».
- **Problema.** Una sospensione imposta dal sistema e una scelta dal cliente sono la stessa parola
  nel PRD e due stati diversi nelle spine; «rallentata» (FR-25, CPU) esiste ma «smette di emettere»
  no.
- **Fix.** Definire un solo asse: stato (attiva/sospesa) più un `state-badge` secondario con la
  causa («per memoria», «per arretrato», «dati scaduti»), oppure aggiungere lo stato «ferma per
  limite».

### B-04 · media · «Scaduto» per task e attività non esiste nel PRD
- **PRD.** FR-8: un'attività oltre il timeout «è considerata fallita e segue la politica di retry»;
  solo l'**esecuzione** termina «come scaduta».
- **Spine.** EX elenco chiuso: Task «scaduto» e «saltato/annullato/scaduto» con la stessa famiglia
  grigia; messaggio «Calcola punteggio: fermato allo scadere del timeout» con stato «Fallito».
- **Problema.** Incoerenza interna e col PRD: lo stesso evento è «fallito» (UJ-2) e «scaduto» (tabella).
- **Fix.** Eliminare «scaduto» da Task e Attività; il timeout è un motivo della fallita («fallito:
  timeout dopo 30 s»); «scaduta» solo per l'Esecuzione.

### B-05 · media · Input dell'esecuzione come JSON contro «senza programmare»
- **PRD.** §2.1 Cliente («comporre… senza programmare»), UJ-2 (Giulia, analista SOC, lancia «con un
  elenco di 300 indicatori»).
- **Spine.** EX Interaction Primitives: «Esegui» apre un `code-editor` JSON.
- **Problema.** Incollare 300 indicatori in un editor JSON è lavoro da sviluppatore.
- **Fix.** Forma dell'input guidata dallo schema del workflow: campo elenco con incolla di testo (una
  riga per elemento) e «Mostra JSON» come modo avanzato. Annotare come assunzione.

### B-06 · media · Esenzioni dal contrasto non testuale contro WCAG 2.1 AA (NFR-9)
- **PRD.** NFR-9 (WCAG 2.1 AA per editor, vista esecuzioni, segreti, dashboard).
- **Spine.** DS Contrasti: archi inattivi a 1,69:1 (1,78 scuro) e segmenti della `status-bar` in
  chiaro a 1,50–2,61:1, «sotto 3:1 per scelta».
- **Problema.** 1.4.11 richiede 3:1 per gli oggetti grafici necessari a capire il contenuto. Gli
  archi sono la struttura del DAG (FR-2) e per un lettore ipovedente non sono ridondanti; i segmenti
  sono ridondanti col testo, gli archi non lo sono del tutto (porte neutre).
- **Fix.** Alzare gli archi inattivi a ≥ 3:1 o aggiungere un'alternativa non grafica (la vista a
  elenco di EX A11y, oggi non decisa); lasciare l'esenzione ai soli segmenti.

### B-07 · bassa · «Al limite» contro «vicini alla quota» / «oltre quota»
- **PRD.** UJ-4 («clienti vicini alla quota»), §2.1 («oltre quota»), FR-49 («al limite»), FR-30.
- **Spine.** Un solo stato «Al limite». Vedi A-10.
- **Fix.** Due stati («Vicino al limite», «Al limite»).

### B-08 · bassa · «30 giorni» scritto nel testo del vuoto
- **PRD.** FR-31: valore predefinito configurabile. **Spine.** EX tabella vuoti. Vedi A-16.

### B-09 · bassa · Nomi dei worker nei dettagli visti dal cliente
- **PRD.** FR-48 mostra i worker **all'operatore**; FR-41 elenca per il cliente stato, tentativi,
  input, output, errori, non il worker.
- **Spine.** `attempt-card` e log: «timeout dopo 30 s · worker-ti-02», «attività ricevuta da
  worker-ti-02».
- **Problema.** Espone al cliente un dettaglio dell'infrastruttura dell'integratore, contro
  «i clienti non devono accorgersi» e contro la separazione operatore/cliente.
- **Fix.** Nel kit mostrare «worker» solo se l'ospite lo abilita; nella dashboard sì.

### B-10 · bassa · Pagina iniziale della dashboard
- **PRD.** UJ-4 parte dai worker. **Spine.** Apre Esecuzioni [ASSUMPTION], già segnalata.
- **Fix.** Decidere con Marco; il PRD suggerisce Worker come prima voce per l'operatore.

### B-11 · bassa · Testo del banner di ripresa promette più del PRD
- **PRD.** NFR-6/NFR-15 («è una pausa, non una perdita»; nessun dato accettato perso).
- **Spine.** «Nessun dato è andato perso. Tempo trascorso: 12 s.» Il testo è del mock C, «sarà
  rivisto». Va bene per i dati accettati; non copre un guasto dichiarato fuori da NFR-6.
- **Fix.** «Nessun lavoro accettato è andato perso» e niente promessa sul tempo.

### B-12 · bassa · «Annulla» ha due sensi
- **PRD.** «Annullamento» di un'esecuzione (FR-10). **Spine.** Pulsanti «Annulla» (ferma
  l'esecuzione) e, nel vocabolario comune, «Annulla» per chiudere un dialogo.
- **Fix.** «Annulla esecuzione» per il primo; «Chiudi»/«Indietro» per gli altri.

---

## (c) Idee qualitative che le spine perdono o attenuano

### C-01 · media · «Facile da usare» / «senza programmare» non è un criterio UX
- **PRD.** §2.1 (Cliente: «senza programmare»; Integratore: «primo workflow in pochi minuti, anche
  senza conoscere topictoast»), UJ-5, NFR-13, SM-5.
- **Spine.** Nessuna sezione con un obiettivo di facilità (tempo al primo workflow per Giulia, errori
  per salvataggio, passaggi), nessuna prova di usabilità prevista; la parola non compare.
- **Dove atterra.** EX Foundation (o nuova sezione «Principi»), Accessibility Floor.
- **Fix.** Due criteri: Giulia compone e lancia un workflow di 5 task senza leggere documentazione;
  Luca rispetta NFR-13 anche per la parte UI (guida rapida, dashboard). Collegare a SM-5.

### C-02 · media · Workflow Builder, Kestra, Conductor come riferimento: mai analizzati
- **PRD.** §1.1 (1: componenti legati al proprio motore; Workflow Builder gira solo su Temporal),
  §4.9 («resta un riferimento»).
- **Spine.** `.memlog.md` e Inspiration & Anti-patterns citano solo Pinterest/Dribbble. Il vantaggio
  «DAG al centro, kit nativo» (distinzione n. 1) non è tradotto in una scelta di UI verificata
  contro Workflow Builder (nodi di partenza multipli, fan-out, join, forma a N).
- **Dove atterra.** EX Inspiration & Anti-patterns.
- **Fix.** Aggiungere una voce «Riferimento: Workflow Builder» con cosa si tiene e cosa si supera
  (un solo nodo di partenza; niente fan-out dinamico), e una per i componenti di Kestra/Conductor.

### C-03 · media · Vocabolario di chi non è sviluppatore
- **PRD.** Giulia è analista SOC (§2.3), non sviluppatrice; «Non sapere che sotto c'è topictoast».
- **Spine.** Voce «impersonale e piana», ma tutta la terminologia del glossario viene portata nel
  kit senza distinguere il lessico del cliente da quello dell'integratore: «worker», «attività»,
  «funzione», «tentativo», «sandbox» (in `Funzione del worker`, `Nodo personalizzato`).
- **Dove atterra.** EX Voice and Tone, Personalizzazione dell'ospite.
- **Fix.** Una colonna «parola per il cliente» nel dizionario (es. «Funzione del worker» →
  «Funzione»), con la parola del glossario come predefinita ma sostituibile (FR-44).

### C-04 · bassa · Posizionamento rispetto a n8n come guardia di progetto
- **PRD.** §2.2, §7, SM-C3 («non si aggiungono funzioni di richiamo»).
- **Spine.** Una sola frase (palette senza catalogo di nodi pronti). Non è una regola di
  progettazione: niente lista «cosa l'editor non fa» (marketplace, template, onboarding da utente
  diretto, account, billing).
- **Fix.** Aggiungere ai Don't di DS: «Catalogo, template e account propri nel kit».

### C-05 · bassa · «Non accorgersi che sotto c'è topictoast»: manca il meccanismo di verifica
- **PRD.** §2.1, §4.9, FR-44.
- **Spine.** Principio presente (DS Brand & Style, EX Voice); manca un test (nessuna stringa, icona,
  nome di file o titolo di scheda con «topictoast» nel kit; messaggi del motore compresi) e il modo
  in cui l'ospite lo «chiede» (FR-44: «se non lo si chiede»).
- **Fix.** Voce nell'Accessibility/Review gate: ricerca automatica di «topictoast» nei bundle del
  kit; proprietà `branding` come unico punto di opt-in.

### C-06 · bassa · Velocità e leggerezza percepite
- **PRD.** §1.1 punto 3, NFR-2, FR-41 (1 s).
- **Spine.** Solo il vincolo dei 1 s; nessun principio su feedback immediato dopo «Esegui» (anche
  prima dell'identificativo, FR-6 «subito»).
- **Fix.** Principio: dopo «Esegui» la vista si apre subito con l'identificativo e lo stato «In
  coda».

---

## (d) Decisioni delle spine oltre il perimetro del PRD (non giudicate)

1. Componente «area workflow» pronto con navigazione interna (rail a 5 voci), oltre ai pezzi singoli.
2. Editor completo sul telefono (grafo e SQL), non solo consultazione; layout del grafo adattivo
   (sinistra→destra / alto→basso); soglie 48 rem e fascia 768–1279 px; bottom-sheet.
3. Layout automatico del DAG (nessuna posizione manuale dei nodi); inserimento dalla palette dopo il
   task selezionato o tra due task.
4. Tema chiaro e scuro (il PRD non lo chiede), tema scelto dall'ospite o `prefers-color-scheme`.
5. Avviso neutro «Aggiornamenti in pausa» per il cliente nel kit durante NFR-15; `recovery-banner`
   su ogni pagina della dashboard, non chiudibile.
6. Elenco chiuso di stati (esecuzione, task, attività, worker, pipeline stream, cliente), inclusi
   «rallentata», «ricevuta», «annullato», «scaduto».
7. Elenco workflow come superficie (senza FR), «Workflow non trovato.», «Sessione scaduta.» con
   evento all'ospite, stati vuoti e skeleton per ogni area, «Riprova».
8. Tastiera del canvas (frecce, `C`, `A`, `+`, `−`, `0`, `Canc`); vista a elenco alternativa al
   grafo; target di tocco 44 × 44 px; `aria-live` limitato agli esiti.
9. Tab «Log» per task con livelli (info, avviso, errore, ok); i log di FR-52 non devono contenere
   input/output, la scheda mostra eventi, non payload.
10. Tabella degli elementi del fan-out cercabile per valore; scheda del primo elemento fallito con
    «altri N».
11. Input dell'esecuzione in `code-editor` JSON; «Salva» con errori sposta il fuoco sul primo task.
12. Voce dei testi impersonale (niente tu/Lei), formati `Intl`, concordanza dei participi.
13. Dashboard non personalizzabile (accento viola, wordmark), pagina iniziale Esecuzioni.
14. Palette «Gelato pieno», icone, tipografia di sistema, token e componenti (31) per
    vanilla-extract e Storybook; Penpot come passo obbligato (regole di progetto, non del PRD).
15. Nome del worker negli `attempt-card` e nelle righe del log visti dal cliente (vedi B-09).
