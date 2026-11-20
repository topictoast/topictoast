---
title: topictoast — triage del Reviewer Gate UX
created: 2026-11-19
oggetto: reconcile-prd.md, reconcile-brief.md, reconcile-imports.md, review-rubric.md, review-accessibility.md; contesto DESIGN.md, EXPERIENCE.md, .memlog.md
---

# Triage del Reviewer Gate UX

159 finding (PRD 38 più le 15 decisioni della sezione (d), brief 10, imports 19, rubrica 51,
accessibilità 41) uniti in temi senza doppioni. Ogni tema ha una sola classe.

| Classe | Temi | Id |
|---|---|---|
| **A — autofix** (nessuna decisione di prodotto) | 38 | T-1…T-38 |
| **B — decisione di Marco** | 16, di cui 7 bloccanti per la fase | T-39…T-54 |
| **C — rinvio** (spec delle storie o architettura) | 16 | T-55…T-70 |
| **D — candidati per l'aggiornamento del PRD** (elenco, senza giudizio) | 12 | T-71…T-82 |

**Citazioni.** `PRD A-03` = reconcile-prd.md; `PRD d-3` = decisione n. 3 della sua sezione (d);
`brief G2`; `imports C3`; `rubrica 2.1` e `a11y 3.1` = sezione 2 (o 3), primo finding della
sezione nell'ordine del file (rubrica e accessibilità non hanno id propri). DS = DESIGN.md,
EX = EXPERIENCE.md. «Assunzione n» = elenco numerato in EX › Elementi aperti.

**Regole seguite.** Gravità del tema = la più alta tra le fonti; in ogni classe dalla più grave,
a parità prima i bloccanti. Nessun tema rovescia in silenzio una decisione del memlog: chi la
tocca sta in B e la nomina. In A va il requisito; il meccanismo (libreria, API, ruolo ARIA) va in
C all'architettura. «WCAG» = 2.1 AA (NFR-9); dove un fix va oltre, è giustificato da una regola già
scritta nelle spine o nel memlog. Testi nuovi in voce impersonale (memlog); nel kit nessuna parola
su motore o crash.

---

## A — Autofix

### T-1 · critica · Collegamenti e task si tolgono solo col mouse
**Fonti:** a11y 3.1, a11y 3.2, a11y 2.1 (bersagli), imports C7 (Elimina e Scollega sul telefono).
**Fix** (EX «Collegare», «Tastiera», `panel`): nel pannello del task una sezione «Collegamenti» con predecessori e successori, ognuno con «Rimuovi collegamento a <task>», e in fondo «Elimina task» (2.1.1). Azione «Collega a…» nel pannello: elenco dei soli task validi.
Sul canvas, oltre al trascinamento, clic sulla porta d'uscita e poi sul task (come già sul telefono). Annuncio «Collegamento da <task>: scegliere la destinazione», `Esc` annulla, un ciclo si annuncia con i task coinvolti.
Bersaglio valido = anello tratteggiato più etichetta, mai solo tinta (1.4.1). Annulla/ripristina e menu del nodo: T-62.

### T-2 · alta · Editor delle pipeline stream: stream, destinazione, messaggi mancanti
**Fonti:** PRD A-01 (parte).
**Fix** (EX Interaction Primitives, nuova voce «Pipeline stream»; Voice and Tone; State Patterns): accanto al `code-editor` un pannello «Parametri» con gli stream del cliente in sola lettura e la loro conservazione (FR-18), e la destinazione a scelta tra «Stream del cliente» e «Workflow» (FR-24, FR-43; mai «Stream» da solo, regola di Voice and Tone).
Testi nuovi: «Finestra di 2 h oltre la conservazione dello stream (1 h)» (FR-18); «Riga 3, colonna 1: `<costrutto>` non consentito» (FR-19); «Ciclo tra pipeline stream: <a> → <b> → <a>» (FR-19); «Righe in attesa oltre la quota: 1.200 su 1.000» (FR-24). Uno stream di un altro cliente risulta inesistente (FR-26).
Lo stato mostra le righe in attesa (FR-24). Vuoto: «Nessuno stream disponibile.». Finestra, ritardo tollerato e checkpoint: T-56.

### T-3 · alta · Nomi di persona che il motore non conosce; origine dell'esecuzione
**Fonti:** PRD B-01, PRD A-02 (parte), brief G4 (parte).
**Fix** (EX Voice and Tone, tabella e Do/Don't; DS `workflow-bar`): il «chi» di FR-10 è il ruolo della credenziale (FR-27, FR-28). «Esecuzione annullata da Giulia alle 14:31» → «Esecuzione annullata dal cliente alle 14:31» (o «dall'integratore»).
La barra dice l'origine del trigger (FR-6): «In esecuzione · Avviata alle 14:22 dal cliente · durata 1 min 48 s»; varianti «dall'integratore», «da pianificazione», «da pipeline stream «<nome>»». Stessa sostituzione nell'esempio di DS `workflow-bar` e nella colonna Do. Nome della persona: T-73.

### T-4 · alta · Il nodo con errore nell'editor non ha aspetto
**Fonti:** rubrica 3.1, a11y 2.1.
**Fix** (DS frontmatter e prosa di `node-card`; EX `node-card`): nuove chiavi `border-error: '{colors.state-failed}'` e `icon-error: '{colors.state-failed}'`, come in `form-field`.
Nel nodo, sotto il nome, icona croce più la parola «Errore» o «N errori» in `{colors.text}`; il bordo è solo un rinforzo (1.4.1). Il nome accessibile del nodo include gli errori (T-9).

### T-5 · alta · Confini di campi e porte sotto 3:1
**Fonti:** rubrica 2.1, a11y 1.1, a11y 1.2 (porte).
**Fix** (DS colors, Contrasti, `form-field`, `code-editor`, `edge`): nuovo `border-control` `#8a8991` (3,46:1 su `{colors.surface}`) / `border-control-dark` `#706f77` (3,42:1), solo per campi, `code-editor`, select, checkbox, radio e switch.
Nuovo `port-border` `#8e8d95` (3,04:1 sul canvas, 3,28:1 sulla superficie) / `port-border-dark` `#74737b` (3,89:1): le porte, bersagli interattivi, smettono di usare `{colors.edge}`.
Il gusto «bordi sottili e morbidi» del round 3 (memlog) resta: `{colors.border}` `#e6e6e8` non cambia per card, pannelli, pulsanti e voci di palette, che hanno testo; il bordo più scuro vale solo dove il confine è l'unico segnale del controllo. Le due coppie entrano nella tabella dei contrasti.

### T-6 · alta · Workflow dell'integratore: sola lettura e provenienza
**Fonti:** PRD A-04 (parte).
**Fix** (EX State Patterns, `chip`, `workflow-bar`, `palette-item`): `chip` «Dell'integratore» nell'elenco Workflow e nella `workflow-bar`. Con ruolo cliente l'editor di quel workflow è in sola lettura (FR-29), «Esegui» resta attivo, messaggio «Workflow dell'integratore: non modificabile.».
Nella palette voci omonime si distinguono con il `chip` di provenienza; un nodo omonimo del cliente non cambia ciò che il workflow dell'integratore esegue (FR-29). Selettore del cliente e workflow da codice: T-43.

### T-7 · alta · Palette assente dal layout; misure in prosa o fisse in px
**Fonti:** imports A1 (parte), rubrica 2.2, rubrica 2.3, a11y 7.1.
**Fix** (DS Layout & Spacing, `spacing`, Components): layout desktop «barra icone · palette (a sinistra per il memlog, solo nell'editor per EX `panel`) · canvas con barra e pannello in basso · pannello destro».
Token nuovi, col numero tolto dalla prosa: `palette-width`, `node-max-width` (valore da Penpot, `[ASSUMPTION]`), `breakpoint-compact: 48rem`, `focus-ring-width`/`selection-width: 0.09375rem`, `focus-ring-offset: 0.125rem`, `halo-width: 0.25rem`, `indicator-width: 0.125rem` (tabella `0.1875rem`), `icon-size-lg/md/sm: 1.25/0.875/0.75rem`, `icon-stroke: 1.6`, `nav-item-height: 2rem`, `rail-item-height: 2.75rem`, `status-bar-gap: 0.125rem`, `status-bar-min-segment: 0.375rem`, maniglia `2rem × 0.25rem`.
Testo ridimensionabile (1.4.4, 1.4.10, 1.4.12): altezze minime invece di fisse (righe di log da 22 px, barre), colonne del log in `ch`, distanza dell'etichetta di categoria in `em`, nodi che si ridimensionano col testo; prove a 320 px, al 200 % e con spaziatura del testo.

### T-8 · alta · Modello del fuoco sul canvas; più radici
**Fonti:** a11y 3.4, a11y 3.5, a11y 3.7, brief G7 (parte), PRD A-03 (parte: più nodi di partenza).
**Fix** (EX «Tastiera», Accessibility Floor): fuoco e selezione sono distinti: `Spazio` seleziona, `Invio` apre il pannello e vi porta il fuoco, `Esc` lo riporta al nodo; dopo un'aggiunta il fuoco va al nuovo nodo, dopo un'eliminazione al predecessore; il canvas scorre per tenere visibile il nodo col fuoco.
Con più figli le frecce seguono l'ordine del documento e `Shift+frecce` passa tra fratelli; `Invio` sul `fanout-group` entra nei conteggi e in «altri N falliti», che sono `button`. Più radici e più foglie sono ammesse (FR-2): il layout le mette su colonne separate, la tastiera parte dalla prima radice del documento.
Anello di fuoco a doppio filo (interno `{colors.surface}`, esterno accento), distinto dal contorno di selezione. Meccanismo (roving tabindex o `aria-activedescendant`): T-58.

### T-9 · alta · Semantica del grafo e del kit incorporato
**Fonti:** a11y 4.2, a11y 4.4.
**Fix** (EX Accessibility Floor, `node-card`, `status-bar`): il canvas è una regione con nome («Grafo del workflow, 7 task»). Nome accessibile del nodo: categoria, nome, stato, errori, numero di predecessori e successori, conteggio del fan-out. Il ruolo non spegne la navigazione del lettore di schermo (niente `role="application"`; ruolo esatto con la libreria, T-58).
Il kit non rende mai `main` (è dell'ospite): togliere `main` dalla riga «Elementi nativi»; regioni e `nav` con `aria-label` («Sezioni workflow»); livello delle intestazioni come proprietà dei componenti; link «Salta al canvas» dopo la barra icone (2.4.1). Prova con NVDA e VoiceOver sul workflow di UJ-2.

### T-10 · alta · Editor di codice: uscita da tastiera e tastiera virtuale
**Fonti:** a11y 3.3, a11y 7.6, a11y 6.5.
**Fix** (EX `code-editor`, Interaction Primitives, Responsive): gli editor SQL e JSON hanno un modo in cui `Tab` sposta il fuoco, attivabile da tastiera e indicato a vista («Esc, poi Tab per uscire»); `Esc` chiude pannello o foglio solo se il fuoco non è in un editor (2.1.2).
Sul campo `autocapitalize="off"`, `autocorrect="off"`, `spellcheck="false"`, virgolette dritte. Sul telefono il campo col fuoco resta sopra la tastiera virtuale e il foglio va a tutta altezza quando si scrive (memlog: sul telefono l'editor SQL deve funzionare). Libreria: T-58.

### T-11 · alta · Regioni live, log e movimento
**Fonti:** a11y 5.1, a11y 5.2, a11y 5.3, a11y 5.4, a11y 4.5, imports C8 (parte: «segui»), assunzione 24.
**Fix** (EX Tempo reale, `recovery-banner`, `log-line`, `skeleton`, Accessibility Floor): una regione live per area, nessuna su nodi e badge; stato finale dell'esecuzione subito, fallimenti raccolti in un messaggio ogni pochi secondi («3 attività fallite in Per ogni indicatore»), niente annunci al montaggio. `recovery-banner`: si annunciano solo inizio e fine, il contatore resta fuori dalla regione live.
Log: `role="log"` con `aria-live="off"`, area focalizzabile con nome, pulsante sempre visibile «Pausa aggiornamenti» / «Riprendi» (2.2.2). `skeleton`: «Caricamento di …» in un `role="status"` fuori dall'area occupata. Con `prefers-reduced-motion` icona «in esecuzione», righe del segmento e `skeleton` fermi; mai oltre 3 lampeggi al secondo (2.3.1).

### T-12 · alta · Telefono: gesti senza alternativa
**Fonti:** a11y 6.1, a11y 6.2, imports C10.
**Fix** (EX Telefono, `canvas`, `bottom-sheet`; DS `bottom-sheet`): sul canvas tre pulsanti «Ingrandisci», «Rimpicciolisci», «Adatta» (2.5.1; utili anche su desktop senza rotella). Il gesto a un dito resta nel canvas e la pagina dell'ospite si scorre fuori; il kit non chiede mai `user-scalable=no`.
Il `bottom-sheet` ha in testa «Chiudi» ed «Espandi»/«Comprimi», come la X del `panel`; la maniglia è un'aggiunta; il fuoco torna al nodo. Il kit non scrive nella cronologia dell'ospite: nell'assunzione 22 il «gesto indietro» vale solo se l'ospite lo abilita. Posizione dei pulsanti: T-55.

### T-13 · alta · Telefono in orizzontale, bersagli, flusso da telefono
**Fonti:** a11y 6.3, a11y 6.4, rubrica 1.2, rubrica 4.5 (parte: stati dei fogli).
**Fix** (EX Responsive & Platform, Key Flows; DS spacing): il layout compatto vale anche con `(pointer: coarse) and (max-height: 30rem)`; sotto 30 rem di altezza il pannello in basso diventa foglio; la direzione del grafo segue sempre la regola della larghezza (memlog, direzione adattiva).
Token `target-min: 2.75rem` applicato con `pointer: coarse` a tutti i controlli e alla maniglia (regola 44 × 44 già nell'assunzione 18), arco con area di tocco invisibile della stessa misura; verificare le aree delle porte contro la distanza minima tra nodi a 360 px.
Nuova variante di UJ-2 «da telefono, 390 px»: palette come foglio, collegamento con due tocchi, configurazione nel foglio, foglio in caricamento. Prove anche a 844 × 390 e 740 × 360.

### T-14 · alta · Conferme per eliminazioni e azzeramenti
**Fonti:** a11y 7.3, PRD A-11, rubrica 1.4, rubrica 3.2 (parte: azione distruttiva).
**Fix** (EX Interaction Primitives, Component Patterns, Voice and Tone; DS Components): nuovo `confirm-dialog` (fondo `{colors.surface}`, un `button-primary` e un `button-secondary`, fuoco iniziale sul pulsante meno distruttivo).
Testi: «Eliminare il segreto `censys-api`? Le esecuzioni che lo usano falliranno.» (3.3.4: il valore non si rilegge); prima di salvare una query modificata: «La modifica azzera lo stato della pipeline stream.» (FR-23). Numero dei workflow che usano il segreto: T-63.
Key Flows: una riga per «Elimina» un segreto e «Sospendi» una pipeline stream. Conferma di «Annulla esecuzione»: T-62.

### T-15 · alta · Sessione scaduta: avviso, lavoro conservato, dashboard
**Fonti:** a11y 7.4, rubrica 4.3 (parte), rubrica 1.7, assunzione 13.
**Fix** (EX Stati trasversali, State Patterns di Accesso): il kit avvisa l'ospite con un evento almeno 60 s prima della scadenza (2.2.1) e di nuovo alla scadenza. Grafo, SQL e JSON non salvati restano in memoria e tornano dopo il rinnovo.
Messaggio con `role="alert"`: «Sessione scaduta. Accedere di nuovo nell'app; le modifiche non salvate sono conservate.». Dashboard: alla scadenza torna ad Accesso con «Sessione scaduta: accedere di nuovo.», senza dati (FR-45). Nome dell'evento e limite di tentativi: T-63.

### T-16 · alta · Testi sostituibili: messaggi del motore, lingua, nomi accessibili
**Fonti:** a11y 8.2, PRD A-13, a11y 7.7.
**Fix** (EX Voice and Tone, Personalizzazione dell'ospite, Foundation): gli errori di validazione e di limite arrivano da motore e nucleo JS come codice più parametri; il kit li rende con il dizionario sostituibile (FR-44): «Riga 4, colonna 9: …» è la resa di un codice.
Il contenitore del kit porta `lang` della lingua scelta; il contenuto del cliente ha `lang` e `dir="auto"` propri. Nomi accessibili, parole di stato e messaggi live non sono mai vuoti: un testo dell'ospite mancante o vuoto ricade sul predefinito della lingua (4.1.2, 1.4.1); nessuna proprietà nasconde icone o parole di stato. Formato del dizionario: T-63.

### T-17 · alta · Tabelle: tastiera, ordinamento, elenchi lunghi
**Fonti:** a11y 4.3, a11y 3.6, PRD A-19.
**Fix** (EX `data-table`, `error-card`, Accessibility Floor): la prima cella di ogni riga è un link o `button` col nome (niente clic sulla `tr`); riga corrente con `aria-current` o `aria-selected`; intestazioni ordinabili come `button` con `aria-sort`; stessa regola per le card impilate del telefono.
Tabella degli elementi del fan-out: filtro per stato, numero dei risultati in un `role="status"` («12 elementi falliti su 300»), `aria-rowcount` e `aria-rowindex` se virtualizzata. Filtri e `tabs` della dashboard nell'ordine di lettura.

### T-18 · media · Errori di validazione: per nome del task, con suggerimento e riepilogo
**Fonti:** a11y 7.5, brief G3 (parte), rubrica 1.1, PRD A-18, assunzione 6.
**Fix** (EX Voice and Tone, `validation-message`, «Salvare ed eseguire», Key Flows): nell'editor l'errore nomina task e campo (FR-40): «`<percorso>`: riferimento a una funzione inesistente `dns_lookp`» → «Arricchisci: funzione inesistente `dns_lookp`. Funzione simile: `dns_lookup`.»; il percorso resta per SDK e documento. Suggerimento quando il motore lo conosce (3.3.3).
Campi con `aria-invalid` e `aria-describedby`. «Salva» con errori mostra accanto a sé un riepilogo `role="alert"` («3 errori in 2 task») con link ai task e il fuoco va lì, non sul canvas (sostituisce l'assunzione 6).
Un rifiuto del motore dopo «Salva» o una funzione sparita all'esecuzione si mostrano come gli errori del nucleo JS, sul task (FR-4). Nuova variante di UJ-2 «ciclo tra i task»: arco, segno sul task, messaggio, riepilogo.

### T-19 · media · Stati da allineare alle parole del PRD
**Fonti:** PRD B-04, PRD B-03, PRD A-17 (parte).
**Fix** (EX Elenco chiuso degli stati, tabella messaggi; DS Colors «Stati»): «scaduto» esce da Task e Attività: il timeout è il motivo di un fallimento («Fallito: timeout dopo 30 s», FR-8); «scaduta» resta solo all'Esecuzione. Le Attività guadagnano «annullata» (FR-10).
Pipeline stream: «attiva · rallentata · sospesa» (FR-23, FR-25); la sospensione imposta dal sistema è «sospesa» più un `state-badge` della causa in famiglia `{colors.state-failed}` («per memoria di stato», «per arretrato», «dati scaduti»; FR-25, FR-30, FR-23) con il messaggio del caso; «in errore» sparisce.

### T-20 · media · Dashboard: clienti vicini al limite ed esecuzioni ferme
**Fonti:** PRD A-10, PRD B-07, brief G10 (parte: colonne di Quote).
**Fix** (EX `data-table`, State Patterns Cliente, UJ-4): Quote con una colonna per ogni quota di FR-30 (attività concorrenti, esecuzioni al minuto, sandbox, pipeline stream, eventi al secondo, richieste API al secondo, spazio) e barra d'uso.
Due stati: «Al limite» (FR-49) resta in `{colors.state-retried}`, «Vicino al limite» (UJ-4) usa la famiglia grigia degli stati neutri, con icona e parola proprie; vuoto «Nessun cliente vicino al limite.». Esecuzioni della dashboard: filtro «in coda» e colonna ordinabile «In coda da» (§2.1). Soglia di «vicino»: T-80.

### T-21 · media · Stati di «Salva» ed «Esegui»
**Fonti:** rubrica 4.2, PRD A-09 (parte), PRD C-06.
**Fix** (EX State Patterns, `workflow-bar`, Voice and Tone): «Salva» in corso: pulsante con `aria-busy` e «Salvataggio…»; errore di rete: «Salvataggio non riuscito.» più `button-secondary` «Riprova», il grafo resta nell'editor.
«Esegui» apre subito la vista con l'identificativo e lo stato «In coda» (FR-6). Rifiuti prima dell'accettazione accanto al pulsante: «Richiesta non accettata: quota di <quota> raggiunta.» (FR-30), «Richiesta non accettata: spazio esaurito.» (NFR-6). JSON d'input non valido: errori con riga e colonna, «Esegui» non parte.
Uscendo dall'editor con modifiche non salvate (navigazione del kit): «Uscire senza salvare? Le modifiche andranno perse.». Disco pieno nella dashboard: T-80.

### T-22 · media · Limiti superati e valori predefiniti
**Fonti:** PRD A-06 (parte).
**Fix** (EX Voice and Tone, `form-field`): uno schema «<limite> superato: <valore> oltre <massimo>» con un esempio per ogni limite: input e output (FR-11, già presente), join, CPU, memoria e tempo (FR-17), dimensione del documento e della query, numero di task, annidamento, elementi del fan-out, risposta di `call http` (NFR-14).
Retry, timeout e parallelismo mostrano come aiuto «predefinito: <valore>, massimo: <valore>», letti dall'installazione e mai scritti nella UI; un valore sopra il massimo è un errore del campo (NFR-14). Task in coda oltre l'attesa massima: T-72.

### T-23 · media · Versioni e nodi personalizzati
**Fonti:** PRD A-05 (parte), brief G6.
**Fix** (EX «Salvare ed eseguire», `workflow-bar`, `attempt-card`, `chip`, IA): la barra di un'esecuzione mostra la versione con cui è partita («versione 3», FR-5); pannello del task e `attempt-card` mostrano la versione del nodo eseguita (FR-15).
Conflitto di versione: oltre al messaggio, «Ricarica ultima versione» e «Scarica le modifiche», così il lavoro non si perde. IA: «i nodi personalizzati si caricano solo da SDK o CLI nella v1» (FR-15); `chip` `TS` e `RS` (FR-15); schema di input e output del nodo in sola lettura nel pannello. Scelta della versione e cronologia: T-62.

### T-24 · media · Task saltati e annullati: perché e come si vedono
**Fonti:** PRD A-07, rubrica 4.6.
**Fix** (EX State Patterns Task, `panel`, `edge`): nel pannello di un task saltato «Saltato: dipendenza <task> fallita» con link al task (FR-2); da rivedere quando il PRD chiude la domanda aperta 8.
In un'esecuzione annullata o scaduta i task non partiti diventano «annullato», badge grigio e arco tratteggiato come gli altri task non partiti.

### T-25 · media · Componenti usati ma senza riga nel catalogo
**Fonti:** rubrica 3.2, rubrica 2.5, rubrica 3.4, rubrica 3.6.
**Fix** (una riga in DS Components e una in EX Component Patterns, solo token esistenti): `select` = `form-field` con indicatore (segreti, filtri «Cliente», «Workflow», «Stato»); `icon-button` = `button-secondary` quadrato di `{spacing.control-height}` con nome accessibile (X del pannello, zoom); `tooltip` (regole in T-38).
`dashboard-bar` con wordmark (`{typography.wordmark}`, `{spacing.dashboard-bar-height}`) e titolo di pagina in `{typography.heading-lg}`; `login-card` per Accesso; `load-error` = messaggio nell'area più «Riprova». Chiave `icon` per il `chip` delle funzioni. `recovery-banner`: «famiglia ritentata», non «avviso». Dialogo di conferma: T-14.

### T-26 · media · Testo sui riempimenti di stato; alone
**Fonti:** a11y 1.4, rubrica 2.6.
**Fix** (DS Colors «Coppia che non passa», Do's and Don'ts): regola generale «su ogni riempimento `-fill` di stato o di categoria il testo è solo `{colors.text}`» (il testo secondario scende a 3,23–4,17:1 in scuro su completata, ritentata, in esecuzione e fallita, e a 4,30:1 in chiaro su fallita) e una riga Don't.
Tabella dei contrasti: alone 1,38–1,67:1 sul canvas, decorativo; la selezione è portata dal contorno (8,95:1).

### T-27 · media · Primo avvio (UJ-5, NFR-13)
**Fonti:** PRD A-12 (parte), assunzione 26.
**Fix** (EX State Patterns Worker ed Esecuzioni della dashboard, Accesso, UJ-5): vuoto di Worker «Nessun worker collegato.» più `button-secondary` «Apri la guida rapida»; lo stesso pulsante nel vuoto di Esecuzioni; il workflow di esempio (FR-50) compare subito nell'elenco.
Accesso dice il punto esatto in cui la credenziale del primo avvio è stampata, senza mostrare dati (FR-45); il testo esatto arriva dall'architettura (T-63).

### T-28 · media · Collegamenti agli import
**Fonti:** imports D1, imports D2, rubrica 5.1, rubrica 5.3, rubrica 5.4.
**Fix** (`sources` di DS ed EX, Brand & Style, Inspiration, Key Flows): i cinque file di `imports/` entrano nei `sources`; in Brand & Style una frase con i link (ref-3 e ref-4 per «Carta chiara», ref-1 e ref-2 per punti e archi, ref-4b per pannello e barra) e link in linea in `panel` e `palette-item`.
In Inspiration una riga «Non preso» per riferimento: cursori e collaboratori (ref-1), nodi circolari, pannello Code e palette a destra (ref-2), campi nel nodo e «Generate Workflow» (ref-3), avatar e Share (ref-4b).
Layout del kit: «l'editor con palette non ha mock». I due mock esistenti collegati a UJ-2 passo 6 e UJ-4 passo 5.

### T-29 · media · Posizionamento: Workflow Builder, n8n, nessun «topictoast» nel kit
**Fonti:** PRD C-02, PRD C-04, PRD C-05 (parte).
**Fix** (EX Inspiration & Anti-patterns, DS Do's and Don'ts): voce «Riferimento: Workflow Builder» (PRD §1.1, §4.9): si supera il nodo di partenza unico e la mancanza di fan-out dinamico; una voce per Kestra e Conductor con quanto dice il PRD §1.1.
Don't: «Catalogo di nodi, template e account propri nel kit» (PRD §7, SM-C3). Nel gate di revisione: nessuna stringa, icona, nome di file o titolo con «topictoast» nel kit, messaggi compresi, verificato con una ricerca nel bundle. Proprietà di opt-in: T-63.

### T-30 · media · Bloat: una sola sede per ogni valore
**Fonti:** rubrica 6.2, rubrica 6.3, rubrica 6.8, rubrica 6.9, rubrica 6.11, rubrica 8.3, rubrica 3.5.
**Fix** (DS): `components` tiene solo le chiavi che sono scelte (padding, tipografia, raggio, misure); le matrici che seguono il nome (`state-badge` 20 chiavi, `icon-tile` 12, `category-tag` 5, `status-bar` 8) diventano una regola con un esempio («per ogni stato s: `fill: colors.state-s-fill`, `border: colors.state-s-border`, `icon: colors.state-s`»).
La prosa di Components tiene anatomia, contenuto d'esempio ed eccezioni, senza rielencare fondo, bordo e raggio. Typography: tabella «Token | Uso», `fontFamily` dichiarato una volta (`fonts.sans`, `fonts.mono`). Shapes: solo icone, porte e curve. Colors: «Base neutra» e «Superfici derivate» in una riga ciascuno. Selezione e focus da Elevation a Components. Circa 150 righe in meno, nessuna decisione persa.

### T-31 · media · Bloat: token alias
**Fonti:** rubrica 6.1, rubrica 7.1.
**Fix** (DS frontmatter, Colors, Components; riferimenti in EX): i 16 token `state-waiting*` e `state-skipped*`, con gli stessi hex, diventano 8 `state-neutral*`; attesa e saltata restano distinte per icona e parola (`state-badge` punta a `state-neutral-fill` per entrambe).
`surface-waiting` `#fbfbfb` e `surface-group` `#fafafa` (1,01:1 tra loro, uguali in scuro) diventano un solo `surface-subtle`. I 6 `state-*-segment-dark` identici al primo piano scuro seguono T-44: se i segmenti in chiaro passano al primo piano dello stato, sparisce tutta la famiglia `segment`.

### T-32 · media · Bloat: verifiche e assunzioni fuori dalla spina
**Fonti:** rubrica 6.4, rubrica 6.5, rubrica 8.2, rubrica 2.8, rubrica 5.2.
**Fix** (DS Contrasti, EX Elementi aperti): Contrasti tiene regola, soglie, deroghe e una riga per classe (testo, testo secondario, accento, icone, `border-control`, `port-border`); la matrice completa diventa un test generato dai token nel gate della dashboard (compito della prima storia del design system).
Assunzioni con id breve in linea (`[A-12]`) e un solo elenco; via il blocco ripetuto «Componenti senza mock» di DS. Elementi aperti in tre elenchi: assunzioni, decisioni per Marco, ampliamenti di perimetro. Rimandi a `.working/` e provenienza degli hex (†, commento `<style>`) riscritti verso `mockups/` alla promozione.

### T-33 · bassa · Foundation: niente «nessuna libreria esterna»; chi legge l'editor
**Fonti:** brief G7 (parte), brief G3 (parte).
**Fix** (EX Foundation): «Sistema UI. Nessuna libreria esterna.» → «Sistema UI. Componenti propri del design system; le librerie di canvas, layout del DAG ed editor di codice le sceglie l'architettura.» (T-58).
Nuova riga: «Chi usa l'editor del grafo: analista del cliente, non sviluppatore (PRD §2.1). L'editor SQL delle pipeline stream è per chi sa scrivere SQL (FR-43).».

### T-34 · bassa · Testi da correggere
**Fonti:** PRD B-11, PRD B-12, PRD B-08, PRD A-16, rubrica 4.7.
**Fix** (EX Voice and Tone, State Patterns): banner di ripresa «Nessun dato è andato perso.» → «Nessun lavoro accettato è andato perso.», senza previsioni di durata. «Annulla» per l'esecuzione → «Annulla esecuzione»; «Chiudi» o «Indietro» per dialoghi e fogli.
«Nessuna esecuzione negli ultimi 30 giorni.» → «… negli ultimi <N> giorni.», N dalla configurazione (FR-31). Esecuzione eliminata o di un altro cliente: «Esecuzione non trovata.»; così anche «Segreto non trovato.» e «Pipeline stream non trovata.» (FR-26).

### T-35 · bassa · ID dei worker visti dal cliente
**Fonti:** PRD B-09, PRD d-15.
**Fix** (EX `attempt-card`, `log-line`, Voice and Tone): con ruolo cliente il kit non mostra l'ID del worker (FR-41 non lo prevede; FR-48 lo dà all'operatore): «timeout dopo 30 s», «attività ricevuta».
Con ruolo integratore (FR-28) e nella dashboard l'ID resta («attività ricevuta da worker-ti-01», momento di valore di UJ-1). Cosa vede l'integratore nel kit: T-79.

### T-36 · bassa · Bloat: storia dei round e regole ripetute
**Fonti:** rubrica 6.6, rubrica 6.7, rubrica 6.10, rubrica 8.1, rubrica 1.5, rubrica 6.13.
**Fix** (DS Brand & Style, Do's and Don'ts, Components; EX Tempo reale, UJ-1): la storia dei round esce da Brand & Style; nei Don'ts resta la regola senza numero di round; il rifiuto resta in Inspiration. Via le nove citazioni del «mock C» che descrivono difetti: resta la regola.
Ogni regola in una sede con rimandi: nome mai troncato (Do's), testo sempre neutro (Colors), cambio entro 1 s (Tempo reale, unica sede). UJ-1 passi 1–2 in una riga «Prima: installazione e worker, fuori dalla UI».

### T-37 · bassa · Scelte della UX da marcare come tali
**Fonti:** imports B1, imports B2, imports B3.
**Fix** (EX Elementi aperti, Inspiration): tre voci «proposta della UX»: nessuna ombra su nodi e pannelli (i riferimenti ne hanno una leggera); etichetta sempre visibile sulla barra icone (accessibilità, telefono); voci di palette a riquadro con bordo.
In Inspiration: «Respinto: nomi troncati» → «Evitato dalla UX: nomi troncati»; «Respinto: catalogo…» → «Fuori dalla v1 (PRD §7): catalogo…»; «Respinto» solo per le direzioni A, B, D e le palette dei round 1 e 2 (memlog).

### T-38 · bassa · Regole native mancanti
**Fonti:** a11y 7.9, a11y 7.8, a11y 2.3.
**Fix** (EX Accessibility Floor, `form-field`): i tooltip si chiudono con `Esc`, si possono sorvolare e restano finché servono (1.4.13). Accesso: `autocomplete="current-password"`, incolla consentito, «Credenziale non valida.» legato al campo con `aria-describedby` e `role="alert"`.
Nessuna icona di stato senza testo vicino o `aria-label` (barre dense).

---

## B — Decisioni di Marco

### T-39 · critica · BLOCCANTE · Alternativa non visiva al grafo
**Fonti:** a11y 4.1, assunzione 20, PRD d-8. **Decisione nel memlog:** «Editor del grafo sul telefono: stesso canvas… non un editor a elenco separato».
**Domanda:** come costruisce e segue un workflow chi usa un lettore di schermo (NFR-9)?
1. **Consigliata.** Interruttore «Grafo / Elenco» per tutti, stessa selezione e stesso pannello: una vista, non un editor separato; righe con categoria, nome, stato, «dipende da», «precede», azioni «Aggiungi dopo», «Collega a…», «Elimina». — AA raggiunto, aiuta tastiera e telefono; una storia in più.
2. Elenco in sola lettura; la modifica resta sul canvas con le azioni del pannello (T-1). — Costa meno; costruire da zero senza canvas resta difficile.
3. Solo semantica ARIA sul canvas. — Il più economico; alto rischio di mancare AA.
**Bloccante:** le epiche non sanno se esiste la storia «vista a elenco» e l'architettura non sa se selezione e modifica vivono fuori dal canvas.

### T-40 · alta · BLOCCANTE · Costrutti di FR-1 nell'editor
**Fonti:** PRD A-03, brief G2, imports C3, assunzione 15 (domande aperte 7 e 8 del PRD).
**Domanda:** quali costrutti (`switch`, `for`, `try`/retry, `wait`, `raise`, `set`, `call http`, sotto-workflow) si disegnano nell'editor della v1?
1. **Consigliata.** Tutti: `switch` come nodo con uscite etichettate a parole e chip della condizione sull'arco (ref-4b, mai rosso per «altrimenti»), ramo non preso «saltato» e tratteggiato; `fork`/`for` come `fanout-group`; `try`/retry come campi del task; gli altri come nodi di Controllo di flusso. — FR-40 pieno; più componenti.
2. Task, fan-out/join e `switch`; il resto solo nel documento, e un workflow che lo usa si apre in sola lettura. — Meno lavoro; la cliente perde costrutti.
3. Ogni costrutto come nodo generico con configurazione JSON. — Rapido; contro «senza programmare».
**Bloccante:** l'architettura deve decidere come il DAG si esprime in OWS (domanda aperta del progetto) e quali costrutti l'editor produce.

### T-41 · alta · BLOCCANTE · Dati tra i task (input, export, jq)
**Fonti:** brief G1, PRD A-03 (parte: campo jq e FR-17).
**Domanda:** come decide la cliente cosa riceve un task, per esempio «Arricchisci» con i risultati di DNS e Censys?
1. **Consigliata.** Sezione «Dati» guidata nel pannello: task a monte e campo da un elenco, jq come modo avanzato; il join mostra i risultati ricevuti; jq non valido o con `$ENV`, `env`, `input_filename`, `import`, `include` dà errore con percorso e motivo (FR-17). — Adatta a un'analista; un componente in più.
2. Solo `code-editor` jq per input ed export. — Meno lavoro; da sviluppatore.
3. Nessuna mappatura: ogni task riceve l'output intero dei predecessori. — Il più semplice; non copre `export.as` dell'addendum.
**Bloccante:** decide come l'editor scrive `input` ed `export.as` nel documento OWS e se il kit include un editor jq.

### T-42 · alta · BLOCCANTE · Pianificazioni (cron e intervallo)
**Fonti:** PRD A-02, brief G4.
**Domanda:** nella v1 la cliente crea e vede le pianificazioni nel kit, o solo l'integratore via SDK? (L'origine nella barra è comunque scritta: T-3.)
1. **Consigliata.** Sezione «Pianificazione» nel pannello del workflow: cron o intervallo, prossima scadenza, «Sospendi», testo per le scadenze recuperate dopo una pausa (NFR-14). — Coerente con «senza programmare»; una storia in più.
2. Creazione via SDK; nel kit elenco in sola lettura con la prossima scadenza. — Meno lavoro; la cliente dipende dall'integratore.
3. Solo SDK, nessuna superficie. — Nessun costo UI; pianificazioni invisibili alla cliente.
**Bloccante:** cambia il numero di storie del kit e l'API pubblica delle pianificazioni.

### T-43 · alta · BLOCCANTE · Ruolo integratore nel kit e workflow definiti come codice
**Fonti:** PRD A-04 (parte), brief G5 (parte); FR-28, UJ-1.
**Domanda:** (a) con la credenziale dell'integratore il kit mostra un selettore del cliente? (b) un workflow pubblicato come codice si modifica nell'editor?
1. **Consigliata.** (a) Sì, selettore del cliente solo per il ruolo integratore (UJ-1 lancia per il cliente di prova); (b) sì, ogni salvataggio è una versione (FR-5), con `chip` «da codice» e avviso che la prossima pubblicazione da codice ne crea un'altra. — Un solo modello; versioni alternate possibili.
2. (a) Sì; (b) no: sola lettura nell'editor con `chip` «da codice». — Niente conflitti tra codice ed editor; la cliente non li ritocca.
3. (a) No: l'integratore usa dashboard e SDK; (b) come 1. — Kit più semplice; il momento di valore di UJ-1 si sposta nella dashboard.
**Bloccante:** cambia i claim della credenziale firmata (FR-27) e i filtri dell'API del kit.

### T-44 · alta · Archi inattivi e segmenti della barra sotto 3:1
**Fonti:** PRD B-06, a11y 1.2 (archi), a11y 1.3, a11y 2.2. **Decisione nel memlog:** «Gelato pieno»: «segmenti della barra e archi inattivi sotto 3:1 per scelta (informazione ridondante)». Le porte salgono comunque a 3:1 (T-5).
**Domanda:** archi e segmenti salgono a 3:1 o restano sotto per scelta?
1. **Consigliata.** Arco inattivo `#8e8d95` (3,04:1 sul canvas, 3,15:1 sul gruppo) / `#74737b` scuro (3,89:1), tratto 1,25 px invariato, arco attivo a 2 px; segmenti in chiaro col primo piano dello stato (3,87–5,48:1). — AA senza deroghe; linee più presenti.
2. Archi sotto 3:1 con l'alternativa a elenco (regge solo con T-39, opzione 1); segmenti esenti, conteggi già in testo. — Tiene il look leggero.
3. Tutto com'è, con deroga scritta. — Nessun cambio; probabile mancato AA (NFR-9).
**Bloccante:** no.

### T-45 · alta · Temi dell'ospite sotto le soglie di contrasto
**Fonti:** a11y 8.1.
**Domanda:** cosa fa il kit se i colori dell'ospite (FR-44) rompono 4,5:1 o 3:1 su testo, accento, focus o bordo dei campi?
1. **Consigliata.** L'ospite cambia i colori come oggi; il kit verifica le coppie (testo/superficie, accento/canvas, accento/superficie, accento/alone, `border-control`/superficie, primo piano/riempimento): in sviluppo una coppia che fallisce è un errore visibile, in produzione vale il colore dell'ospite con un avviso in console. — L'ospite resta padrone; AA dipende da lui.
2. Come 1, ma in produzione la coppia che fallisce torna al predefinito (proposta della review). — AA garantito; un colore di marca può essere ignorato.
3. Nessun controllo, solo documentazione. — Costo zero; nessuna garanzia.
**Bloccante:** no (il controllo si aggiunge all'API dei temi senza romperla; meccanismo in T-63).

### T-46 · media · BLOCCANTE · Layout automatico del DAG
**Fonti:** memlog (assunzione «da confermare al triage»), assunzione 14, PRD d-3, PRD A-15 (posizioni fuori dal documento).
**Domanda:** i nodi si dispongono solo in automatico?
1. **Consigliata.** Sì, sempre: nessuna posizione manuale e nessuna posizione nel documento OWS; inserimento dalla palette dopo il task selezionato o su un arco (assunzione 7). — Coerente con la direzione adattiva decisa; meno controllo per chi disegna.
2. Automatico più spostamenti manuali salvati in un'estensione del documento. — Più libertà; telefono e desktop non coincidono più.
3. Manuale su desktop, automatico sul telefono. — Familiare; due modelli da mantenere.
**Bloccante:** decide se le posizioni entrano nel documento OWS e quale libreria di layout serve.

### T-47 · media · BLOCCANTE · Input dell'esecuzione: modulo o JSON
**Fonti:** PRD B-05, brief G3, assunzione 16, PRD d-11.
**Domanda:** come scrive Giulia l'input di «Esegui», per esempio 300 indicatori?
1. **Consigliata.** Modulo generato dallo schema di input del workflow (campo elenco che accetta testo incollato, una riga per elemento), «Mostra JSON» come modo avanzato. — «Senza programmare»; serve uno schema di input nel workflow (T-78).
2. Solo `code-editor` JSON (assunzione attuale). — Meno lavoro; da sviluppatore.
3. Campo di testo, una riga per elemento, solo per workflow con un input elenco. — Semplicissimo; copre solo casi come UJ-2.
**Bloccante:** l'opzione 1 richiede uno schema di input nel documento del workflow, chi lo definisce nell'editor e una storia in più.

### T-48 · media · Parole per «in attesa» e «in coda»
**Fonti:** PRD B-02, rubrica 7.3, UJ-1 caso limite in EX.
**Domanda:** il PRD scrive «in attesa» per l'attività senza worker (FR-8, FR-12, UJ-1) e «in coda» per la quota (FR-30); le spine usano «in attesa» = dipendenze non finite e «in coda» = pronto ma non consegnato, con il motivo. Quali parole?
1. **Consigliata.** Quelle delle spine, motivo sempre scritto; il PRD si allinea (FR-8, FR-12, UJ-1, glossario). — Due concetti, due parole; tocca il PRD (T-72).
2. «In attesa di dipendenze» e «In attesa di consegna», senza «in coda». — Fedele a FR-8; etichette lunghe, FR-30 da allineare.
**Bloccante:** no (l'architettura ha bisogno della distinzione, non delle parole).

### T-49 · media · Dove compare l'avviso «sandbox forte non attiva» (FR-54)
**Fonti:** PRD A-14, rubrica 1.3, EX domanda 1, memlog (lacune di copertura).
**Domanda:** dove vede l'operatore che la sandbox forte non si attiva su un host senza KVM?
1. **Consigliata.** Riga fissa in Worker: «Sandbox forte non attiva: KVM non disponibile; in uso la sandbox leggera», con badge di avviso. — Dove l'operatore guarda i worker; poco codice.
2. Banner su ogni pagina della dashboard, come il `recovery-banner`. — Impossibile da perdere; rumore permanente.
3. Solo log e CLI: FR-54 fuori dalla UX della v1. — Nessuna UI; il PRD va aggiornato (T-80).
**Bloccante:** no.

### T-50 · media · Porte: colore come significato o neutre
**Fonti:** imports A2, assunzione 5 di DS (`edge`). **Decisione nel memlog:** «base neutra… con il colore solo come significato (stati, porte, percorso attivo, categorie)».
**Domanda:** le porte prendono un colore?
1. **Consigliata.** Neutre, bordo `port-border` a 3:1 (T-5); «porte» esce dal tratto nel memlog. — Meno rumore; il significato sta già in nodi e archi.
2. Nella vista delle esecuzioni la porta d'uscita prende il colore dello stato del task. — Rinforza lo stato; un segnale in più da tenere coerente.
3. Colore per tipo di dato, come ref-1. — Richiede tipi sulle porte: da verificare con l'architettura.
**Bloccante:** no.

### T-51 · media · Pannelli dell'editor: agganciati o card flottanti
**Fonti:** imports A4, imports A3, imports A1 (parte: comprimibile). **Decisione nel memlog:** «palette per categoria a sinistra e configurazione del task selezionato a destra; pannelli in basso con input, output e log».
**Domanda:** che forma hanno i pannelli dell'editor? (Oggi DS si contraddice: Elevation «seguono le card», `panel` li vuole agganciati.)
1. **Consigliata.** Agganciati con un filo, pannello in basso a tab Log/Input/Output, palette comprimibile. — Più canvas a 1280 px (barra 64 px + palette + 360 px).
2. Card flottanti con margine `{spacing.3}` e `{rounded.2xl}` come ref-2/3/4b, tab in basso, palette comprimibile. — Più vicine ai riferimenti; meno spazio.
3. Card flottanti con Input/Output e Log affiancati (ref-2). — Fedele a «pannelli» al plurale; stretto a 1280 px.
**Bloccante:** no.

### T-52 · media · Azione primaria nella barra e passo «pubblica»
**Fonti:** imports C6.
**Domanda:** nell'editor quale pulsante è primario, ed esiste un «pubblica» oltre a «Salva»?
1. **Consigliata.** «Salva» primario, «Esegui» secondario; nessun «pubblica»: salvare crea la versione (FR-5); Play senza verde. — Un solo primario per area; tiene FR-5.
2. «Esegui» primario, «Salva» secondario. — Spinge a provare; «Esegui» con modifiche non salvate va definito.
3. «Salva» come bozza più «Pubblica» separato. — Bozze sicure; concetto nuovo, non nel PRD.
**Bloccante:** no.

### T-53 · bassa · Pagina iniziale della dashboard
**Fonti:** PRD B-10, assunzione 1.
**Domanda:** dopo l'accesso la dashboard apre Worker o Esecuzioni?
1. **Consigliata.** Worker, prima voce della navigazione: UJ-4 parte da lì e la pagina ospita gli avvisi di FR-54 e NFR-15. — Segue il PRD; per UJ-5 Esecuzioni è a un clic.
2. Esecuzioni (assunzione attuale). — Esito di UJ-5 subito visibile; UJ-4 a un clic.
**Bloccante:** no.

### T-54 · bassa · Font di sistema
**Fonti:** memlog (assunzione «Tipografia… da confermare al triage»), assunzione 2 di DS.
**Domanda:** stack di font di sistema o un font ospitato in proprio?
1. **Consigliata.** Stack di sistema sans e mono. — Peso zero, nessun problema di CSP, l'ospite li sostituisce (FR-44).
2. Font sans e mono ospitati nel bundle. — Aspetto uguale ovunque; più peso, licenza da verificare.
**Bloccante:** no.

---

## C — Rinvii

### T-55 · alta · Anatomia del canvas: porte, corpo del nodo, frecce, limiti sul telefono
**Fonti:** imports C1, imports C2, imports C9, rubrica 4.5 (parte).
**Proprietario:** spec della storia dell'editor, dopo il disegno in Penpot. Candidate: porta d'ingresso a monte e d'uscita a valle, centrate, che ruotano con l'orientamento, una per nodo anche con più archi; corpo del nodo vuoto o con una riga (funzione, timeout, segreto); punta di freccia piccola e confluenza in un punto; posizione dei pulsanti di zoom (T-12); limite di dimensione del grafo e pizzico fuori scala sul telefono.
**Riprendere:** quando l'editor si disegna in Penpot, prima di `speckit-specify`.

### T-56 · alta · Finestra, ritardo tollerato e checkpoint delle pipeline stream
**Fonti:** PRD A-01 (parte).
**Proprietario:** architettura (dialetto SQL chiuso, FR-19…FR-22). Se finestre e ritardo stanno nella query, il pannello «Parametri» (T-2) aggiunge solo l'intervallo di checkpoint; se sono parametri, diventano `form-field`.
**Riprendere:** quando l'architettura fissa il dialetto, dopo lo spike delle pipeline stream.

### T-57 · alta · Unità di misura del kit incorporato (rem o em)
**Fonti:** a11y 7.2.
**Proprietario:** architettura del kit. Con `rem`, un ospite con `html { font-size: 62.5% }` porta il testo a 7 px; con `em` dalla radice del kit si eredita la preferenza dell'utente (1.4.4). Tocca la regola «rem con base 16 px» della skill dashboard-react (memlog, vincoli ereditati).
**Riprendere:** prima della prima storia di un componente del kit, con prova obbligatoria a radice 62,5 %.

### T-58 · media · Librerie di canvas, layout del DAG ed editor di codice
**Fonti:** brief G7 (parte), a11y 3.3 (parte: CodeMirror 6), a11y 3.4 e a11y 4.2 (meccanismo).
**Proprietario:** architettura. Candidate: `@xyflow/react` e dagre (MIT, addendum), CodeMirror 6. Criteri: i requisiti di T-8, T-9 e T-10 (un solo arresto di `Tab`, roving tabindex o `aria-activedescendant`, ruolo che non spegne la lettura, uscita dall'editor con `Tab`), direzione sinistra-destra e alto-basso (memlog), layout automatico (T-46).
**Riprendere:** nell'architettura, dopo le risposte a T-39, T-40 e T-46.

### T-59 · media · Isolamento CSS del kit e colori forzati
**Fonti:** a11y 8.3.
**Proprietario:** architettura del kit (Shadow DOM o reset con `:where`). Requisiti: focus del kit non spento da un `*:focus { outline: none }` dell'ospite né tagliato da `overflow: hidden`; ID unici con più istanze; `z-index` dei fogli rispetto alle modali dell'ospite; in `forced-colors` colori di sistema per contorni, archi, porte e bordi dei campi; `color-scheme` nel tema scuro.
**Riprendere:** quando l'architettura sceglie come il kit applica gli stili.

### T-60 · media · «Esegui» durante la ripresa del motore
**Fonti:** PRD A-08.
**Proprietario:** architettura (comportamento dell'API durante NFR-15). Se l'API accetta, «Esegui» resta attivo e l'esecuzione resta «In coda» (FR-6, NFR-15); se rifiuta, un messaggio neutro nel kit, senza parole su motore o crash (memlog).
**Riprendere:** quando l'architettura descrive la ripresa dopo un crash.

### T-61 · media · Token degli stati d'interazione
**Fonti:** rubrica 2.4, rubrica 3.3, rubrica 4.1, rubrica 2.7, a11y 1.5.
**Proprietario:** spec della prima storia del design system, con Penpot. Da fissare: hover, premuto, disabilitato, segnaposto (proposta: `{colors.text-secondary}`, 7,20:1), link, riga selezionata, checkbox, radio, switch, select, tooltip, bersagli validi del collegamento, voce nav in hover; ogni coppia nella tabella dei contrasti.
**Riprendere:** prima di `speckit-specify` della storia del design system.

### T-62 · media · Controlli dell'editor e della vista delle esecuzioni
**Fonti:** imports C4, imports C5, imports C7 (parte), imports C8 (parte), PRD A-15, brief G5 (parte), PRD A-05 (parte), EX domanda 2 (parte).
**Proprietario:** spec delle storie di editor e vista delle esecuzioni, al `speckit-clarify`. Candidate: ricerca, titolo, comprimi e «+» nella palette; intestazione fissa del pannello con X; annulla/ripristina (`Ctrl+Z`) e menu del nodo (Elimina, Duplica, Scollega); «Copia» sull'output; «Esporta documento» in sola lettura (FR-35); scelta della versione da eseguire e cronologia (FR-5); conferma prima di «Annulla esecuzione».
**Riprendere:** al clarify di ciascuna storia.

### T-63 · media · Meccanismi che le spine nominano e l'architettura sceglie
**Fonti:** PRD A-13 (parte), PRD C-05 (parte), a11y 7.4 (parte), a11y 8.2 (parte), rubrica 4.3 (parte), PRD A-11 (parte), PRD A-12 (parte).
**Proprietario:** architettura, contratto pubblico del kit (PRD §6). Da fissare: codici e parametri degli errori e dizionario tipizzato; nome dell'evento di sessione in scadenza; proprietà `branding`, unico opt-in del nome topictoast; limite dei tentativi su Accesso; conteggio dei workflow che usano un segreto; dove si stampa la credenziale del primo avvio; API dei temi per coppie (T-45).
**Riprendere:** nella sezione dei contratti del kit dell'architettura.

### T-64 · media · Lessico del cliente e dizionario dei testi
**Fonti:** PRD C-03, rubrica 6.12.
**Proprietario:** spec della storia i18n. Una colonna «parola per il cliente» (per esempio «Funzione del worker» → «Funzione»), con la parola del glossario come predefinita e sostituibile (FR-44); la tabella di Voice and Tone può diventare `copy.md`, sorgente dei dizionari it/en.
**Riprendere:** quando nasce il primo dizionario i18n.

### T-65 · bassa · Prestazioni con grafi grandi
**Fonti:** PRD A-20.
**Proprietario:** architettura. Aggiornamenti raggruppati, nessun ridisegno dei nodi fuori schermo, budget sul numero di task (NFR-14) e sulla frequenza degli aggiornamenti (FR-41).
**Riprendere:** quando si fissa il limite sul numero di task di NFR-14.

### T-66 · bassa · Quarto motivo di «In coda»
**Fonti:** brief G9.
**Proprietario:** architettura (partizioni del bus). Se un task può aspettare dietro un'attività lenta della stessa partizione, testo di ripiego «In coda: in attesa di un worker libero»; mai «In coda» senza motivo.
**Riprendere:** quando l'architettura sceglie le partizioni.

### T-67 · bassa · Segreti nelle righe di log scritte dal codice del cliente
**Fonti:** brief G10 (parte).
**Proprietario:** architettura. Se il motore oscura i valori dei segreti anche nelle righe dei nodi, una riga in `log-line`; altrimenti va scritto come limite nella documentazione.
**Riprendere:** quando l'architettura definisce consegna dei segreti e log (FR-33, FR-52).

### T-68 · bassa · Dashboard con il canale in tempo reale interrotto
**Fonti:** rubrica 4.4, EX domanda 3.
**Proprietario:** spec della storia della dashboard. Candidata: lo stesso avviso neutro del kit, distinto dal `recovery-banner`.
**Riprendere:** quando l'architettura sceglie il canale in tempo reale e la riconnessione.

### T-69 · bassa · Categorie future e segreti OAuth
**Fonti:** brief G8.
**Proprietario:** aggiornamento del PRD, quando entrano nel perimetro. Nota per allora: le nuove categorie avranno tessera neutra o un quarto colore (a 28° di distanza lo spazio delle tinte è quasi esaurito); OAuth come azione «Collega account» nello stesso `form-field`.
**Riprendere:** quando cicli, LLM, connettori o OAuth entrano nel perimetro.

### T-70 · bassa · Nomi dei token nel glossario
**Fonti:** rubrica 7.2, EX domanda 4.
**Proprietario:** prima storia che porta i token nel codice (costituzione VIII). `worker-function`, `custom-node`, `flow-control`, `state-*` (con `state-neutral` di T-31) entrano in `docs/glossary.md`.
**Riprendere:** alla prima storia del design system.

---

## D — Candidati per l'aggiornamento del PRD

Elenco senza giudizio: dove le revisioni mostrano che il PRD dovrebbe cambiare (aggiunte di
perimetro o lacune).

- **T-71** — Ampliamenti già decisi da Marco: editor completo sul telefono (grafo e SQL), componente «area workflow» con navigazione interna, avviso neutro di pausa nel kit (NFR-15). Fonti: memlog (ampliamenti), EX Ampliamenti 1–3, PRD d-1, d-2, d-5, rubrica 1.2.
- **T-72** — Elenco chiuso degli stati di esecuzione, task, attività, worker, pipeline stream e cliente, con regola di propagazione (domanda aperta 8) e stato del task in coda oltre l'attesa massima (FR-8). Fonti: PRD d-6, B-02, B-03, B-04, A-06 (parte), rubrica 7.3, memlog (lacune).
- **T-73** — Il «chi» di FR-10 oggi è un ruolo; un nome visualizzato richiede un claim facoltativo nella credenziale firmata (FR-27). Fonti: PRD B-01.
- **T-74** — Superfici senza FR: elenco Workflow; Esecuzioni della dashboard raggiunta solo da UJ-5. Fonti: PRD d-7, memlog (lacune IA), EX Copertura.
- **T-75** — UJ-4 e UJ-5 senza persona, stato iniziale, momento di valore e caso limite; UJ-3 da confermare con un caso reale. Fonti: rubrica 1.6, EX UJ-3, UJ-4, UJ-5, memlog (lacune).
- **T-76** — Un criterio di facilità d'uso per il cliente oltre NFR-13 e SM-5 (per esempio un workflow di 5 task composto senza documentazione). Fonti: PRD C-01.
- **T-77** — Pianificazioni: FR-6 è `[ASSUMPTION]`; un FR per la superficie se T-42 sceglie la UI; scadenze recuperate (NFR-14) visibili. Fonti: PRD A-02, brief G4.
- **T-78** — Schema di input dichiarato dal workflow, se T-47 sceglie il modulo. Fonti: PRD B-05, brief G3, PRD d-11.
- **T-79** — Ruolo integratore nel kit (selettore del cliente) e visibilità dei worker per ruolo: il PRD non dice cosa vede l'integratore nel kit. Fonti: PRD A-04, B-09, d-15.
- **T-80** — Segnali all'operatore non previsti: disco pieno nella dashboard (NFR-6), soglia di «vicino al limite», annullamento dalla dashboard, FR-54 fuori dalla UX se T-49 sceglie l'opzione 3. Fonti: PRD A-09, A-10, B-07, A-17, rubrica 1.3.
- **T-81** — Contratto pubblico del kit (PRD §6): evento di sessione in scadenza e scadenza nella dashboard, temi per coppie, testi obbligatori per i nomi accessibili, proprietà `branding`. Fonti: a11y 7.4, a11y 8.1, a11y 8.2, PRD C-05.
- **T-82** — Scelte delle spine oltre gli FR: layout automatico, tema chiaro e scuro, vista a elenco, tabella degli elementi del fan-out, tab Log con livelli, voce impersonale, dashboard non personalizzabile, palette, token e passaggio in Penpot. Fonti: PRD d-3, d-4, d-8, d-9, d-10, d-12, d-13, d-14.

---

## Esiti del triage (2026-11-20)

Fonte canonica: `.memlog.md`. Qui solo l'esito, per chi legge il triage.

| Tema | Esito |
|---|---|
| T-1…T-38 | Applicati alle spine (autofix). |
| T-39 | Vista a elenco **in sola lettura** (non l'interruttore consigliato); le righe aprono il pannello del task in sola lettura; la modifica resta sul canvas con le azioni del pannello. Rischio noto registrato. |
| T-40 | Tutti i costrutti di FR-1 disegnabili (consigliata). |
| T-41 | Sezione «Dati» guidata, jq come modo avanzato (consigliata). |
| T-42 | Sezione «Pianificazione» nel kit (consigliata). |
| T-43 | Selettore del cliente per l'integratore; workflow da codice modificabili, ogni salvataggio una versione (consigliata). |
| T-44 | Archi e segmenti a 3:1 (consigliata). |
| T-45 | Il kit verifica i contrasti dell'ospite: errore in sviluppo, avviso in produzione (consigliata). |
| T-46 | Desktop **manuale e automatico** («Riordina»), telefono solo automatico (non la consigliata). |
| T-47 | Modulo generato dallo schema di input, «Mostra JSON» avanzato (consigliata). |
| T-48…T-54 | Consigliate: «in attesa»/«in coda» delle spine, avviso FR-54 in Worker, porte neutre, pannelli agganciati, «Salva» primario, dashboard apre su Worker, font di sistema. |
| T-55…T-70 | Rinviati con proprietario e condizione (memlog). |
| T-71…T-82 | Candidati per l'aggiornamento del PRD. |

### D — aggiunta

- **T-83** — Manifest dei worker e dei nodi personalizzati: per ogni funzione connessioni esterne
  (piattaforma e credenziale richiesta), schema di input, schema di output e altri metadati.
  Tocca FR-12, FR-15, FR-32 (il segreto dichiara la connessione che soddisfa), FR-40 (validazione
  di tipi e campi obbligatori). Fonte: memlog, richiesta di Marco del 2026-11-20.
