# Riconciliazione: brief e addendum contro le spine UX

Input: `briefs/brief-topictoast-2026-11-18/brief.md` e `addendum.md`.
Spine: `DESIGN.md`, `EXPERIENCE.md`, decisioni in `.memlog.md`.
Data: 2026-11-19. Nessun file esistente è stato modificato.

## Esito in breve

10 lacune: 2 alte, 4 medie, 4 basse.

Le idee qualitative del brief sono state portate bene. Il brief in realtà dice poco su tono, voce e
sensazione: le sole idee di posizionamento che toccano la UX sono "il cliente non deve sapere che
sotto c'è topictoast", "non è n8n" e "DAG e parallelo su elenchi variabili al centro". Le prime due
stanno nelle spine (kit senza marchio in Brand & Style e Voice and Tone; anti-pattern "catalogo di
nodi pronti" in Inspiration & Anti-patterns). La terza è il fan-out con 1 / 0 / 300 elementi in UJ-2.
Le lacune vere sono di contenuto dell'editor e di superfici, non di tono.

### Differenze già note, non sono lacune (vale il PRD sul brief)

- Segreti con sole chiavi API nella v1; OAuth dopo (il brief, "La soluzione" punto 3, parla ancora di "OAuth o altri metodi" nel kit).
- Editor delle pipeline stream nel kit nella v1 (il brief non ha le pipeline stream).
- "Pipeline" riservato alle pipeline stream (il brief lo usa per i workflow, "comporre da sé le pipeline").
- Sandbox eseguita da topictoast e avviso FR-54 senza superficie: già tra le domande aperte delle spine.

## Lacune

### G1. Come i dati passano da un task all'altro (mappatura input/output, jq)

- **Severità:** alta.
- **Brief/addendum.** Addendum §1: "Dati. `input`/`output`/`export`; espressioni jq obbligatorie per
  ogni runtime"; e "si passano i dati tramite `$context` (`export.as`), non tramite l'output del
  task precedente". Brief, "Il problema", caso concreto: "lo si verifica in parallelo su DNS e su
  Censys, lo si arricchisce, lo si inserisce". Il task "Arricchisci" deve ricevere i due risultati.
- **Spine.** `EXPERIENCE.md`, Interaction Primitives, "Selezione e configurazione" elenca i campi
  del pannello: nome, funzione o nodo, segreti, retry, timeout, politica di errore del fan-out,
  parallelismo. Nessun campo per l'input di un task, per cosa riceve il join, per l'output esportato.
  FR-1 del PRD cita "espressioni jq", ma né l'editor né i testi d'errore (Voice and Tone) hanno una
  superficie per scriverle o per un jq non valido. Nel Foundation non si dice se la mappatura è
  guidata o libera.
- **Dove cade.** `EXPERIENCE.md`: Interaction Primitives (campi del pannello), Component Patterns
  (`form-field`, `validation-message`), tabella testi di Voice and Tone; UJ-2 passo 2 (cosa riceve
  "Arricchisci"). `DESIGN.md`: nessun nuovo componente se si riusa `code-editor`.
- **Fix proposto.** Aggiungere al pannello destro una sezione "Dati" con input del task e, per il
  join, l'elenco dei risultati ricevuti. Decidere se è guidata (selettore del task a monte e del
  campo) o un `code-editor` jq. Per un cliente non programmatore (vedi G3) scegliere la forma
  guidata di default. Aggiungere un testo d'errore per jq non valido con percorso e motivo, e una
  riga in UJ-2 che dica cosa mostra il pannello di "Arricchisci" e del join.

### G2. Costrutti condizionali e altri costrutti di FR-1 senza interfaccia (switch, if, try, wait, raise)

- **Severità:** alta.
- **Brief/addendum.** Addendum §1, tabella: "Condizioni e salti di ramo: sì: `if` su ogni task,
  `switch`"; "Flusso... `fork`, `switch`, `for`, `try`/`catch` con `retry`". Vision del brief:
  "cicli, if/else, trasformazioni". Il PRD (FR-1) li mette in v1: "do, set, switch, fork, for, try con
  retry, wait, raise, call http, … sotto-workflow".
- **Spine.** Solo il fan-out/join ha un componente (`fanout-group`). La palette cita `call http`,
  `set` e sotto-workflow sotto "Controllo di flusso" (assunzione 15). Nessuna parola su `switch`,
  `if`, `wait`, `raise`, `try`. Non c'è un componente per un ramo condizionale: nodo `switch` con più
  uscite, etichetta della condizione sull'arco, ramo "altrimenti", stato "saltato" disegnato sul
  ramo non scelto (lo stato esiste nell'elenco chiuso, ma nessuna regola dice dove lo si vede). Il
  riferimento ref-4b mostrava già un chip if/else con ramo else rosso e il memlog lo registra tra i
  riferimenti, ma né `DESIGN.md` né `EXPERIENCE.md` lo ripresero.
- **Dove cade.** `DESIGN.md`: Components (nuovo `condition-chip` o varianti di `edge` e
  `node-card`); `EXPERIENCE.md`: Component Patterns (`edge` con etichetta), Interaction Primitives
  ("Collegare"), State Patterns (dove compare "saltato"), UJ-2 (variante con `switch`).
- **Fix proposto.** Elencare i costrutti di FR-1 che entrano nell'editor e quelli che restano solo
  nel documento. Per ognuno dire come si compone e come si vede in esecuzione. Minimo: nodo
  `switch` con uscite etichettate, chip della condizione sull'arco (ripreso da ref-4b), ramo non
  scelto in "saltato" con arco tratteggiato. Dichiarare cosa fa l'editor con un workflow che usa un
  costrutto non disegnabile (aperto in sola lettura? mostrato come nodo generico?).

### G3. "Editor no-code" contro JSON, SQL e jq scritti a mano

- **Severità:** media.
- **Brief.** "In sintesi": "un editor no-code per comporre da sé le pipeline". "Per chi": i clienti
  "compongono le proprie pipeline con un editor no-code... Non hanno bisogno di sapere che sotto
  c'è topictoast." Giulia è un'analista SOC, non una sviluppatrice.
- **Spine.** `EXPERIENCE.md`, Interaction Primitives: "Esegui chiede l'input. [ASSUMPTION: l'input
  dell'esecuzione si scrive come JSON in un `code-editor`]". Gli errori citano "percorso nel
  documento". L'editor SQL è deciso dal PRD (FR-43), ma resta un editor per programmatori. Nessuna
  parola su come un cliente senza competenze vede o compila questi punti.
- **Dove cade.** `EXPERIENCE.md`: Interaction Primitives ("Salvare ed eseguire"), Voice and Tone
  (errori), UJ-2 passo 5. FR-15 dà ai nodi personalizzati uno schema di input e output, quindi un
  modulo generato è fattibile.
- **Fix proposto.** Per "Esegui": modulo generato dallo schema di input del workflow, con editor
  JSON come alternativa avanzata. Gli errori mostrati a un cliente nominano il task e il campo,
  non il percorso nel documento. Dichiarare in Foundation chi è il lettore dell'editor (analista, non
  sviluppatore) e che l'editor SQL è per chi sa scriverlo.

### G4. Pianificazioni (cron e a intervallo) senza superficie

- **Severità:** media.
- **Brief/addendum.** Addendum §1: "schedule `every`/`cron`/`after`/`on`". PRD FR-6: "una
  pianificazione cron o a intervallo avvia un'esecuzione per ogni scadenza" (v1, `[ASSUMPTION]`).
- **Spine.** Nessuna menzione di pianificazione in `EXPERIENCE.md` né in IA. Un workflow parte da
  "Esegui" (manuale), dalla pipeline stream, dall'SDK. Non si vede dove un cliente imposti, veda o
  sospenda una pianificazione, né come un'esecuzione partita da pianificazione si distingue da una
  manuale (la `workflow-bar` mostra "Avviata alle 14:22 da Giulia": da chi, se parte da cron?).
- **Dove cade.** `EXPERIENCE.md`: Information Architecture (voce o sezione nell'editor),
  Component Patterns (`workflow-bar`), Voice and Tone ("da chi" per esecuzioni non umane).
- **Fix proposto.** Decidere se la v1 ha una UI (sezione "Avvio" nel pannello del workflow, con
  cron o intervallo, stato e prossima scadenza) o solo SDK. In ogni caso fissare il testo dell'origine
  dell'esecuzione: "Avviata da pianificazione", "Avviata da pipeline stream «nome»", "Avviata da API".

### G5. Documento OWS: vederlo, esportarlo, workflow definiti come codice

- **Severità:** media.
- **Brief/addendum.** Brief, "Cosa lo rende diverso": "i workflow che la usano non girano su altri
  runtime". "La soluzione" punto 2: SDK per "scrivere i workflow come codice, oppure per inviare un
  documento OWS". PRD FR-35: il documento è "leggibile ed esportabile".
- **Spine.** UJ-1 passo 4: il workflow definito come codice "compare nell'elenco Workflow del
  cliente di prova", ma non si dice se il cliente lo apre e lo modifica nell'editor, né cosa
  succede alla versione se lo si salva (il codice lo ripubblica e sovrascrive?). Nessuna vista
  "documento" né esportazione; nessun segno di portabilità. Il conflitto di versione (FR-5) è
  coperto solo per due editori in editor.
- **Dove cade.** `EXPERIENCE.md`: Information Architecture (elenco Workflow: colonna origine),
  Component Patterns (`workflow-bar`, `chip`), Interaction Primitives ("Salvare ed eseguire"),
  Elementi aperti.
- **Fix proposto.** Nell'elenco e nella `workflow-bar` un `chip` "da codice" / "da editor"; azione
  secondaria "Esporta documento" (e vista di sola lettura del documento per l'integratore).
  Decidere la regola di modifica di un workflow da codice (sola lettura nell'editor, oppure salvataggio
  = nuova versione con avviso). Una riga di Voice and Tone sulla portabilità dell'estensione DAG,
  solo se l'integratore la chiede.

### G6. Gestione dei nodi personalizzati: nessuna superficie né versione visibile

- **Severità:** media.
- **Brief.** "Il problema" punto 3: "Ogni cliente carica i propri nodi personalizzati... configura le
  proprie integrazioni". "La soluzione": "aggiunge un suo nodo personalizzato e la lancia".
  PRD FR-15: caricare con nome già usato crea una versione nuova.
- **Spine.** UJ-2: il nodo è "caricato da un collega con la CLI" (fuori UI). Il `chip` mostra solo
  `TS`; la v1 ha anche Rust. Nessun elenco dei nodi del cliente, nessuna versione, nessun posto dove
  vedere schema di input e output o quale versione usa un task in un workflow.
- **Dove cade.** `EXPERIENCE.md`: Information Architecture (assente), Component Patterns (`chip`,
  `palette-item`), UJ-2.
- **Fix proposto.** Dichiarare esplicitamente "caricamento solo da CLI/SDK nella v1" in IA, e
  prevedere almeno: `chip` `TS` e `RS`, versione del nodo mostrata nel pannello del task, schema di
  input e output in sola lettura (serve anche a G1 e G3). Una lista dei nodi può restare fuori.

### G7. Più nodi di partenza e scelta della libreria del canvas

- **Severità:** bassa.
- **Addendum.** §4: Workflow Builder, il più vicino, "ha un solo nodo di partenza e niente cicli";
  "Librerie di base riusabili: `@xyflow/react` (MIT), dagre (MIT)".
- **Spine.** Gli esempi hanno un solo inizio ("Ricevi elenco"). Nessuna regola per un DAG con più
  radici o più foglie, né per l'ordine di lettura da tastiera in quel caso. `Foundation` dice
  "Nessuna libreria esterna" per il sistema UI; il layout automatico e il canvas da tastiera
  (Accessibility Floor) dipendono di fatto da una libreria o da molto codice proprio, e la
  scelta non è registrata.
- **Dove cade.** `EXPERIENCE.md`: Interaction Primitives (Canvas), Elementi aperti, Foundation.
- **Fix proposto.** Una riga: più radici e più foglie sono ammesse e il layout le dispone su colonne
  separate. Aggiungere agli Elementi aperti la domanda "libreria del canvas e del layout (`@xyflow/react`,
  dagre) o codice proprio", con l'impatto sull'accessibilità da tastiera. Se si usa una libreria,
  correggere "Nessuna libreria esterna".

### G8. Estensioni previste dalla visione: nuove categorie di nodi e collegamento OAuth

- **Severità:** bassa.
- **Brief.** "Visione": nodi di base pronti "(cicli, if/else, trasformazioni, connettori, LLM)",
  "l'OAuth gestito dal motore", "passi con LLM". "La soluzione" punto 3: segreti "con OAuth o altri
  metodi" (superato nella v1, ma resta la direzione).
- **Spine.** Tre categorie fisse nei token (`worker-function`, `custom-node`, `flow-control`) con
  colori propri; il vincolo "distanza minima di tinta 28 gradi" esaurisce lo spazio dei colori. Il
  `form-field` del segreto prevede solo "valore scritto e mai riletto". Non è un errore per la v1,
  ma né una riga dice come crescono.
- **Dove cade.** `DESIGN.md`: Colors (categorie), Do's and Don'ts; `EXPERIENCE.md`: Component
  Patterns (`form-field` dei segreti), Elementi aperti.
- **Fix proposto.** Una nota in Elementi aperti: le categorie future (cicli, LLM, connettori)
  avranno tessera neutra o un quarto colore, e il segreto OAuth sarà un'azione "Collega account" nello
  stesso componente. Nessun lavoro ora.

### G9. Motivi di "In coda" non coperti: attesa dietro un task lento della stessa partizione

- **Severità:** bassa.
- **Addendum.** §2: "con un task lento in una partizione, quelli dopo aspettano"; brief, "Rischi":
  "Iggy è un log, non una coda".
- **Spine.** State Patterns elenca tre motivi di "In coda": manca un worker, quota, pool della
  sandbox vuoto. Un quarto, l'attesa per accodamento dietro altre attività, può comparire per
  l'architettura scelta e il cliente vedrebbe un task "In coda" senza motivo.
- **Dove cade.** `EXPERIENCE.md`: State Patterns ("In attesa" e "in coda"), tabella testi.
- **Fix proposto.** Dopo l'architettura, aggiungere un testo generico di ripiego: «In coda: in attesa
  di un worker libero». Il testo dice sempre un motivo; mai "In coda" nudo.

### G10. Segreti nei log e portata delle quote

- **Severità:** bassa.
- **Addendum.** §7: "segreti cifrati a riposo, consegnati al worker solo per l'esecuzione che li usa,
  mai nei log"; "quote per cliente (task, concorrenza, risorse della sandbox)".
- **Spine.** La regola "mai valori di segreti" sta in `code-block` e in FR-46, ma non in `log-line`:
  l'output di un nodo personalizzato che stampa il proprio segreto finirebbe nel tab Log. La tabella
  Quote mostra "uso per cliente e Al limite" senza dire quali grandezze (concorrenza, arretrato,
  sandbox).
- **Dove cade.** `EXPERIENCE.md`: Component Patterns (`log-line`, `data-table` Quote).
- **Fix proposto.** Una riga in `log-line`: il motore oscura i valori dei segreti anche nelle righe
  scritte dal codice del cliente (se l'architettura lo garantisce); altrimenti dirlo come limite.
  Elencare le colonne di Quote secondo FR-49.

## Cosa le spine hanno già portato dal brief (per non riaprirlo)

- Il cliente non sa che sotto c'è topictoast: kit senza marchio, testi sostituibili, accento grafite neutro (Brand & Style, Voice and Tone, Personalizzazione dell'ospite).
- Non è n8n: anti-pattern "catalogo di nodi pronti e connettori" (Inspiration & Anti-patterns).
- Parallelo su elenchi di dimensione variabile: `fanout-group` con 1, 0 e 300 elementi, politica di errore, barra di stato.
- Separazione tra clienti: "come se non esistesse" (FR-26), eventi in tempo reale solo del cliente.
- Segreto "scritto e mai riletto": `form-field`, "Valore salvato, non visibile".
- Codice ostile fermato: UJ-2 caso limite, errore di rete non consentita.
- La dashboard dell'operatore mostra worker e ogni fase del loro lavoro: `history-list`, UJ-4.
- Ripresa del motore come pausa, non perdita: `pause-notice` e `recovery-banner`.
