---
title: topictoast — EXPERIENCE
status: final
created: 2026-11-19
updated: 2026-11-20
sources:
  - ../../prds/prd-topictoast-2026-11-19/prd.md
  - ../../prds/prd-topictoast-2026-11-19/addendum.md
  - ../../briefs/brief-topictoast-2026-11-18/brief.md
  - .memlog.md
  - DESIGN.md
  - .working/extract-sources.md
  - .working/directions-brief.md
  - ../../../../.claude/skills/dashboard-react/SKILL.md
  - imports/ref-1-pinterest.jpg
  - imports/ref-2-pinterest.jpg
  - imports/ref-3-pinterest.jpg
  - imports/ref-4-dribbble.png
  - imports/ref-4b-dribbble.png
  - mockups/key-editor-desktop.html
  - mockups/key-editor-phone.html
  - mockups/key-execution-view.html
  - mockups/key-stream-editor.html
name: topictoast
---

# topictoast — EXPERIENCE

> Le spine (questo file e `DESIGN.md`) vincono, in caso di conflitto, su ogni mock in `mockups/`, `imports/` o `.working/`. I mock approvati stanno in `mockups/` e illustrano le spine; `.working/` resta l'esplorazione (direzioni, palette). I `T-n` rimandano a `triage.md`.

## Foundation

- **Forma.** Interfaccia web in React, in due superfici con un solo design system:
  - **kit React** senza marchio, incorporato nella piattaforma ospite. Offre un componente
    **«area workflow»** pronto, con navigazione interna, e gli stessi pezzi esportati singoli per
    chi compone pagine proprie;
  - **dashboard dell'operatore**, distribuita con topictoast (FR-45…FR-49).
- **Utenti degli editor.** Chi usa l'editor del grafo: analista del tenant, non sviluppatore
  (PRD §2.1). L'editor SQL delle pipeline stream è per chi sa scrivere SQL (FR-43).
- **Manifest.** Ogni funzione registrata ha il suo manifest, che dichiara nome, descrizione,
  gruppo, connessioni, schema di input, schema di output e un campo di estensione per altri
  metadati (FR-55); i nodi personalizzati caricati con la CLI portano lo stesso manifest e
  nell'editor si presentano come le funzioni. Il
  gruppo è la «categoria» del manifest («DNS», «Arricchimento»): ordina la palette e non è la
  categoria del nodo (Funzione del worker, Nodo personalizzato, Controllo di flusso), che resta
  nella tessera e nell'etichetta sopra il nodo. Dal manifest l'editor ricava
  palette, campi della sezione «Dati», segreti proposti e validazione (Interaction Primitives).
- **Connessione.** Una piattaforma esterna (per esempio Censys) che il manifest dichiara per una
  funzione, con la credenziale richiesta, e che un segreto del tenant soddisfa. Non è un
  connettore (fuori dalla v1, PRD §7) né un account OAuth (T-69): il segreto resta una chiave. Un
  segreto dichiara la connessione che soddisfa: «Aggiungi segreto» chiede nome, connessione e
  valore (FR-32).
- **Schermi.** Desktop con board a 1280, 1440 e 1920 px; telefono a 390 e 360 px. Sul telefono
  funziona tutto, editor del grafo ed editor SQL compresi, non solo la consultazione.
- **Temi e lingue.** Tema chiaro e tema scuro; italiano e inglese; dimensioni in rem con base 16 px.
  Il contenitore del kit porta `lang` della lingua scelta; il contenuto del tenant ha `lang` e
  `dir="auto"` propri.
- **Sistema UI.** Componenti propri del design system; le librerie di canvas, layout del DAG ed
  editor di codice le sceglie l'architettura (T-58). Il design system nasce con la dashboard,
  documentato in Storybook; gli stili usano vanilla-extract con i token del frontmatter di
  `DESIGN.md` letti da `src/theme/tokens.ts`. Ogni pagina nuova si disegna prima in Penpot (file
  «topictoast»).
- **Riferimento visivo.** `DESIGN.md` è il riferimento per l'aspetto; questo file descrive il
  comportamento.
- **Fuori perimetro UX.** Il nucleo JS (FR-39) non ha interfaccia. Una sola conseguenza qui: la
  validazione nell'editor dà gli stessi esiti del motore.

## Information Architecture

### Kit: area workflow e pezzi singoli

L'area workflow ha una barra icone a sinistra (`rail-item`) con cinque voci: **Workflow**,
**Editor**, **Esecuzioni**, **Segreti**, **Pipeline stream**. In alto resta l'intestazione
dell'app ospite: il kit non ne aggiunge una sua. Con la credenziale dell'integratore (FR-28) e
più di un tenant, in cima all'area c'è il `tenant-selector`, e ogni sezione mostra le risorse del
tenant scelto. Ogni
sezione esiste anche come componente singolo. Nella v1 i nodi personalizzati si caricano solo da
SDK o CLI (FR-15): il kit non ha una superficie per caricarli.

| Superficie | Raggiunta da | FR | Percorsi | Riferimento visivo |
|---|---|---|---|---|
| Area workflow (contenitore) | voce di menu dell'ospite (per esempio «Workflow») | FR-58 | UJ-1, UJ-2 | [editor](mockups/key-editor-desktop.html) (barra icone) |
| Workflow (elenco) | barra icone | FR-58 | UJ-2 | solo spina |
| Editor dei workflow, con pianificazione e vista a elenco | elenco → workflow o «Nuovo workflow» | FR-40, FR-6, FR-56, FR-57 | UJ-1, UJ-2 | [desktop](mockups/key-editor-desktop.html), [telefono](mockups/key-editor-phone.html); modulo di «Esegui» in [vista delle esecuzioni](mockups/key-execution-view.html) |
| Vista delle esecuzioni (elenco e dettaglio) | barra icone; «Esegui» nell'editor | FR-41 | UJ-1, UJ-2 | [dettaglio a grafo e a elenco](mockups/key-execution-view.html) |
| Segreti | barra icone; selettore dei segreti nell'editor | FR-42 | UJ-2 | solo spina |
| Pipeline stream (elenco, editor, stato e ultime righe) | barra icone | FR-43 | UJ-3 | [editor, stato e ultime righe](mockups/key-stream-editor.html) |

### Dashboard dell'operatore

`dashboard-bar` con wordmark «topictoast» e navigazione in alto (`nav-item`): **Worker**,
**Esecuzioni**, **Pipeline stream**, **Quote**. **Accesso** precede tutto: senza credenziali valide
non si vede alcun dato (FR-45). Dopo l'accesso si apre Worker, la prima voce: UJ-4 parte da lì e
la pagina ospita gli avvisi di FR-54 e NFR-15 (T-53).

| Superficie | Raggiunta da | FR | Percorsi | Riferimento visivo |
|---|---|---|---|---|
| Accesso | apertura della dashboard | FR-45 | UJ-4, UJ-5 | solo spina |
| Worker, con l'avviso sulla sandbox forte | navigazione; pagina iniziale | FR-48, FR-54 | UJ-4 | `.working/direction-c-carta-chiara.html` (pagina Worker) |
| Esecuzioni (elenco con filtri, dettaglio del task, «Annulla esecuzione») | navigazione | FR-46, FR-10 | UJ-4, UJ-5 | dettaglio come la vista delle esecuzioni del kit |
| Pipeline stream | navigazione | FR-47 | UJ-4 | solo spina |
| Quote | navigazione | FR-49 | UJ-4 | solo spina |

**Copertura.** Ogni percorso arriva su almeno una superficie; Esecuzioni della dashboard è nei
percorsi di UJ-4 (esecuzioni ferme in coda) e UJ-5.

Elenco Workflow, Segreti e tutta la dashboard non hanno mock approvati in `mockups/` (la pagina
Worker resta un'esplorazione in `.working/`): si costruiscono da queste tabelle e si disegnano in
Penpot prima del codice.

## Voice and Tone

Microcopy dei testi predefiniti di kit e dashboard. L'identità visiva sta in
`DESIGN.md`, Brand & Style.

- **Impersonale e piana.** Niente «tu» né «Lei». Le istruzioni usano l'infinito («Selezionare un
  task…»); gli esiti usano un nome più un participio («Esecuzione annullata»).
- **Pulsanti**: un verbo, nella forma breve dei mock e del PRD: «Annulla esecuzione», «Aggiorna»,
  «Salva», «Esegui», «Salva e attiva», «Nuova pipeline stream». Dialoghi e fogli si chiudono con
  «Chiudi» o «Indietro»: «Annulla» da solo non esiste.
- **Termini del glossario del PRD** (§3), sempre quelli: workflow, task, esecuzione, attività,
  worker, funzione, nodo personalizzato, segreto, quota. «Pipeline stream» si scrive per intero,
  mai «Pipeline» né «Stream» da soli: uno stream è un'altra cosa.
- **Errori.** Dicono dove e perché: nell'editor task e campo, nel documento il percorso, nell'SQL
  riga e colonna, i task coinvolti, il limite superato; con un suggerimento quando il motore lo
  conosce (WCAG 3.3.3). Nessun messaggio dice solo «Errore».
- **Codici, non frasi.** Gli errori di validazione e di limite arrivano da motore e nucleo JS come
  codice più parametri; il kit li rende con il dizionario sostituibile (FR-44): «Riga 4, colonna
  9: …» è la resa di un codice. Formato del dizionario: T-63.
- **Kit senza marchio.** Nel kit nessuna frase nomina topictoast, il motore o un crash (FR-44). Il
  gate di revisione lo verifica con una ricerca di «topictoast» nel bundle del kit: stringhe,
  icone, nomi di file e titoli, messaggi compresi (proprietà di opt-in: T-63).
- **Chi ha fatto cosa.** Il «chi» di FR-10 e dell'origine di un'esecuzione è sempre il ruolo
  (FR-28), «dal tenant» o «dall'integratore», più il «nome visualizzato» facoltativo della
  credenziale firmata, se c'è, mostrato come testo (FR-27, NFR-7):
  «dal tenant, `<nome visualizzato>`». Un trigger dice la sua origine
  (FR-6): «da pianificazione», «da pipeline stream».
- **ID dei worker.** Con ruolo tenant il kit non mostra l'ID del worker (FR-41); con ruolo
  integratore (FR-28) e nella dashboard l'ID resta (FR-48). Cosa vede l'integratore nel kit:
  FR-28, FR-41.
- **Contenuto del tenant.** Nomi, errori, input, output e righe emesse si mostrano sempre come
  testo, mai come HTML (NFR-7).
- **Sostituibili.** L'ospite può sostituire ogni testo; la v1 porta italiano e inglese (FR-44).
  Regole sui testi mancanti in Personalizzazione dell'ospite.
- **Numeri e date** formattati con `Intl` nella lingua della pagina («1,27 s», «1 min 48 s»).
- **Accordo delle parole di stato.** L'aggettivo concorda col nome a cui si riferisce: esecuzione
  e attività «completata», task «completato», tentativo «fallito», come nell'elenco degli stati
  del PRD (§3).

| Situazione | Testo predefinito (it) | Fonte |
|---|---|---|
| Barra di un'esecuzione | «In esecuzione · Avviata alle 14:22 dal tenant · durata 1 min 48 s»; varianti «dal tenant, `<nome visualizzato>`», «dall'integratore», «dall'integratore, `<nome visualizzato>`», «da pianificazione», «da pipeline stream «`<nome>`»» | mock C, FR-6 |
| Esecuzione annullata | «Esecuzione annullata dal tenant alle 14:31» (o «dal tenant, `<nome visualizzato>`», «dall'integratore», «dall'integratore, `<nome visualizzato>`») | FR-10 |
| Esecuzione scaduta | «Esecuzione scaduta: superato il tempo massimo; attività in corso annullate» | FR-8 |
| Task fallito per timeout | «Fallito: timeout dopo 30 s» | FR-8 |
| Task di un ramo non preso | «Saltato: ramo «Punteggio alto» non preso» | FR-1, FR-2 |
| Conteggi del fan-out | «completate 287 · in esecuzione 9 · ritentate 3 · fallite 1» | mock C |
| Elemento del fan-out | «elemento 87 di 300» | mock C |
| Tentativo | «Tentativo 1 · fallito», sotto «timeout dopo 30 s» (con l'ID del worker: «timeout dopo 30 s · worker-ti-02») | mock C |
| Log | «attività ricevuta» (con l'ID: «attività ricevuta da worker-ti-02»), «tentativo 1 fallito: timeout dopo 30 s», «ritentata: tentativo 2 tra 2 s» | mock C |
| Task in coda senza worker | «In coda: nessun worker collegato per `dns_lookup`» | UJ-1, FR-8 |
| Task in coda per quota | «In coda: quota di attività concorrenti raggiunta» | FR-30 |
| Worker che non risponde | «Non risponde da 2 min · 4 attività riassegnate» | mock C, UJ-4 |
| Ciclo nel grafo | «Ciclo tra i task: Arricchisci → Calcola punteggio → Arricchisci» | FR-2 |
| Funzione inesistente (editor) | «Arricchisci: funzione inesistente `dns_lookp`. Funzione simile: `dns_lookup`.» | FR-4, FR-40 |
| Funzione inesistente (SDK e documento) | «`<percorso>`: riferimento a una funzione inesistente `dns_lookp`» | FR-4 (forma del percorso decisa dall'architettura) |
| Campo d'ingresso non valido | «Arricchisci · `indicatore`: atteso un testo, `dns_lookup · record` è un elenco»; «Arricchisci: campo obbligatorio `indicatore` senza valore» | FR-4, FR-40, FR-55 |
| jq non valido o non consentito | «Arricchisci · `indicatore`: espressione jq non valida, colonna 7»; «Arricchisci · `indicatore`: `$ENV` non consentito in jq» | FR-17, FR-40 |
| Connessione senza segreto | «`censys_search`: connessione Censys senza segreto»; selettore vuoto: «Nessun segreto per Censys.» | FR-4, FR-40, FR-55 |
| Connessione senza segreto all'avvio | «Richiesta non accettata: nessun segreto per Censys.» | FR-29, FR-33 |
| Query SQL non valida | «Riga 4, colonna 9: colonna sconosciuta `asn_nome`» | FR-19, FR-43 |
| Costrutto SQL non consentito | «Riga 3, colonna 1: `<costrutto>` non consentito» | FR-19 |
| Ciclo tra pipeline stream | «Ciclo tra pipeline stream: `<a>` → `<b>` → `<a>`» | FR-19 |
| Finestra oltre la conservazione | «Finestra di 15 min oltre la conservazione dello stream (10 min)» | FR-18 |
| Righe trattenute oltre la quota | «Righe trattenute oltre la quota: 1.200 su 1.000» | FR-24 |
| Modifica di una query | «La modifica azzera lo stato della pipeline stream.» | FR-23 |
| Versione non più ultima | «La versione 3 non è più l'ultima: versione corrente 4» | FR-5 |
| Versione del formato | «`document.dsl` 1.0.0 non accettata: versioni accettate 1.1.x» | FR-1 |
| Nodo fermato | «Calcola punteggio: fermato allo scadere del timeout» | UJ-2, FR-17 |
| Rete non consentita | «Destinazione di rete non consentita: 169.254.169.254» | FR-16 |
| Workflow dell'integratore | «Workflow dell'integratore: non modificabile.» | FR-29 |
| Workflow definito come codice | «Definito come codice: si modifica dal codice.» | FR-35 |
| Pianificazione | «Prossima scadenza: 21/11/2026 alle 02:00»; «Scadenze perse durante una pausa: 3 · recuperate: 1»; «Scadenza delle 02:00 non accettata: nessun segreto per Censys.» | FR-6, FR-57, NFR-14 |
| Elenco incollato nell'input | «300 elementi»; «Riga 12: atteso un numero» | FR-41, FR-56, T-47 |
| Sandbox forte non attiva | «Sandbox forte non attiva: KVM non disponibile; in uso la sandbox leggera» | FR-54 |
| Richiesta rifiutata | «Richiesta non accettata: quota di `<quota>` raggiunta.»; «Richiesta non accettata: spazio esaurito.» | FR-30, NFR-6 |
| Uscita con modifiche non salvate | «Uscire senza salvare? Le modifiche andranno perse.» | FR-40 |
| Segreto salvato | «Valore salvato, non visibile» e sotto «aggiunto il 18/11/2026 · ultimo uso alle 14:23» | FR-32 |
| Eliminare un segreto | «Eliminare il segreto `censys-api`? I workflow che lo usano non si potranno avviare; le esecuzioni in corso falliranno al task che lo usa.» | FR-42, FR-33 |
| Schema di input cambiato al salvataggio | «Salvare il nuovo schema di input? La pianificazione e 2 pipeline stream avviano questo workflow: i loro avvii fuori schema non saranno accettati.» | FR-5 |
| Eliminare un workflow | «Eliminare il workflow `<nome>`? Le esecuzioni in corso finiscono, la pianificazione viene eliminata, lo storico resta.» | FR-58 |
| Eliminazione di un workflow rifiutata | «Workflow non eliminato: usato da `<workflow>` come sotto-workflow e da `<pipeline stream>` come destinazione.» | FR-58 |
| Funzione registrata con connessioni diverse (dashboard) | «`censys_search` registrata di nuovo da worker-ti-04 con connessioni diverse» | FR-48 |
| Pausa nel kit | «Aggiornamenti in pausa, riprendono da soli.» | FR-41, FR-43 |
| Ripresa nella dashboard | «Il motore sta tornando operativo dopo un arresto imprevisto. Nessuna nuova esecuzione parte finché non è di nuovo attivo. Nessun lavoro accettato è andato perso. Tempo trascorso: 12 s.» | mock C, NFR-15 |
| Disco pieno nella dashboard | «Spazio su disco esaurito: il lavoro nuovo è rifiutato finché non si libera spazio. Il lavoro accettato non va perso.» | FR-48, NFR-6 |
| Sessione scaduta (kit) | «Sessione scaduta. Accedere di nuovo nell'app; le modifiche non salvate restano in attesa del rinnovo.» | FR-27 |
| Sessione scaduta (dashboard) | «Sessione scaduta: accedere di nuovo.» | FR-45 |
| Accesso alla dashboard rifiutato | «Credenziale non valida.» | FR-45 |

Nelle righe con fonte FR, e nelle etichette «Nuovo workflow» e «Aggiungi segreto», la
formulazione è della UX; l'FR fissa solo il contenuto [ASSUMPTION A-3].

**Limiti superati** (NFR-14). Schema «`<limite>` superato: `<valore>` oltre `<massimo>`», con
l'accordo delle parole di stato. I numeri sono d'esempio: i massimi veri vengono
dall'installazione. Task in coda oltre l'attesa massima: spec della storia di FR-8 (domanda
aperta 15 del PRD). Esempi:

- «Output di 1,4 MB oltre il limite di 1 MB», già in uso; così anche l'input (FR-11).
- «Join superato: 12 MB oltre 10 MB» (FR-11).
- «CPU superata: 12 s oltre 10 s», «Memoria superata: 300 MB oltre 256 MB», «Tempo superato: 45 s
  oltre 30 s» (FR-17).
- «Dimensione del documento superata: 1,2 MB oltre 1 MB».
- «Dimensione della query superata: 80 KB oltre 64 KB».
- «Numero di task superato: 520 oltre 500».
- «Annidamento superato: 12 livelli oltre 10».
- «Elementi del fan-out superati: 12.000 oltre 10.000».
- «Risposta di `call http` superata: 6 MB oltre 5 MB».

| Do | Don't |
|---|---|
| «Nessun worker collegato.» | «Non hai ancora collegato nessun worker!» |
| «Esecuzione annullata dal tenant alle 14:31» | «L'esecuzione è stata annullata con successo ✓» |
| «Riga 4, colonna 9: colonna sconosciuta `asn_nome`» | «Errore nella query» |
| «Aggiornamenti in pausa, riprendono da soli.» (kit) | «Il motore topictoast è andato in crash» (kit) |
| «Pipeline stream» | «Pipeline», «Stream» |
| «Annulla esecuzione»; «Chiudi» in un dialogo | «Annulla» per l'una e per l'altra cosa |

## Component Patterns

Comportamento. Gli aspetti visivi stanno in `DESIGN.md`, Components; i nomi sono gli stessi.

Mock: [editor](mockups/key-editor-desktop.html), selezione, errore sul task, `data-mapping` e
«Collegamenti»; [editor sul telefono](mockups/key-editor-phone.html), `bottom-sheet` di palette e
configurazione; [vista delle esecuzioni](mockups/key-execution-view.html), `input-form`,
`fanout-group`, `error-card`, `tabs` del log e `list-view`;
[editor delle pipeline stream](mockups/key-stream-editor.html), `code-editor` SQL con errore.

| Componente | Dove | Regole di comportamento |
|---|---|---|
| canvas | editor, vista delle esecuzioni, dettaglio nella dashboard | Trascinamento e zoom (vedi Interaction Primitives), più tre `icon-button` «Ingrandisci», «Rimpicciolisci», «Adatta» per chi non ha rotella o pizzico (posizione: T-55). È una regione con nome («Grafo del workflow, 7 task»). Nella vista delle esecuzioni è in sola lettura: si seleziona, non si modifica. Nell'editor su desktop c'è anche «Riordina». Direzione del grafo e disposizione: Interaction Primitives, Canvas. |
| list-view | al posto del `canvas` nell'editor, nella vista delle esecuzioni e nel dettaglio della dashboard, con «Elenco» nella `workflow-bar` | Alternativa non visiva al grafo, per tutti (NFR-9, T-39), in sola lettura: nessuna azione di modifica sulle righe. Una riga si seleziona (clic, `Invio`) e apre il `panel` del task in sola lettura, come dal canvas: nella vista delle esecuzioni tentativi, input, output ed errore, e i `tabs` Log/Input/Output sotto l'elenco; nell'editor la configurazione del task da leggere. Senza selezione l'elenco occupa l'area del canvas. Elenco annidato nativo: task in ordine topologico, fratelli nell'ordine del documento, i task di un `fanout-group` sotto il gruppo e quelli di uno `switch` sotto l'etichetta del ramo. Ogni riga dice categoria, nome, «dipende da», «precede», conteggi del fan-out e, nell'esecuzione, lo stato; nell'editor gli errori del task al posto dello stato. Segue il tempo reale con la regione live dell'area, senza una propria. «Grafo» riporta al canvas con il fuoco sul nodo che lo aveva. |
| node-card | canvas | Un clic o un tocco seleziona il task e apre il `panel` destro (configurazione nell'editor, dettaglio nell'esecuzione). Nell'editor un task con errori mostra icona e parola d'errore e il suo `validation-message` nel pannello (FR-40). Nella vista delle esecuzioni mostra lo stato con `state-badge`. Nome accessibile: categoria, nome, stato, errori, numero di predecessori e successori, conteggio del fan-out. |
| category-tag | sopra ogni `node-card` | Statico. Dice la categoria a parole; il lettore di schermo legge «Funzione del worker», «Nodo personalizzato» o «Controllo di flusso» prima del nome. |
| icon-tile | `node-card`, `palette-item`, intestazione di `fanout-group` | Decorativa (`aria-hidden`): la categoria è già detta da `category-tag` o dal titolo del gruppo. |
| state-badge | nodi, barre, tabelle, tentativi, cronologia | Icona e parola sempre insieme. Si aggiorna in tempo reale (vedi Tempo reale), senza regione live propria. Non è interattivo. |
| chip | nodi (`TS`, `RS`), barra del workflow («versione 3», «Dell'integratore», «da codice»), elenco Workflow e `palette-item` (provenienza), tabella dei worker (funzioni) | Statico, sola informazione. |
| fanout-group | canvas | Raggruppa le attività di un fan-out: non disegna mai un ramo per elemento (FR-41). Con un elemento solo mostra «· 1» e funziona uguale (UJ-2, variante); con un elenco vuoto mostra «· 0», nessuna attività, e il join riceve un elenco vuoto (FR-3). I conteggi e la `status-bar` seguono il tempo reale. I conteggi e «altri N falliti» sono `button` e aprono la tabella degli elementi (vedi `error-card`). |
| status-bar | `fanout-group` | Segmenti proporzionali ai conteggi. Espone `role="img"` con l'`aria-label` dei conteggi («Completate 287, in esecuzione 9, ritentate 3, fallite 1»); il cambio non si annuncia, il valore corrente sta anche nel nome accessibile del `fanout-group`. |
| error-card | sotto il `fanout-group` | Compare quando un elemento del fan-out fallisce e porta il messaggio leggibile. Un clic seleziona quell'elemento e apre tentativi, input e output. Si vede la scheda del primo elemento fallito più «altri N falliti»; «altri N falliti», o un conteggio del `fanout-group`, apre nel `panel` una `data-table` degli elementi in quello stato, cercabile per valore [ASSUMPTION A-5]. La tabella ha il filtro per stato e il numero dei risultati in un `role="status"` («12 elementi falliti su 300»); se virtualizzata, `aria-rowcount` e `aria-rowindex`. |
| edge | canvas | Si crea e si toglie come in Interaction Primitives, Collegare. Un arco che chiude un ciclo produce l'errore che nomina i task coinvolti (FR-2) e il workflow non si salva (FR-40). Nell'esecuzione: grigio a riposo, colorato sul percorso in esecuzione, tratteggiato verso i task non partiti, annullati compresi. Da uno `switch` ogni arco è un ramo con il suo `condition-chip`; nell'esecuzione l'arco di un ramo non preso è tratteggiato e i suoi task risultano «saltato». |
| condition-chip | a metà di ogni arco che esce da uno `switch` | Statico, non riceve il fuoco. Mostra l'etichetta del ramo scritta nella sezione «Rami» del pannello dello `switch`; il ramo predefinito si chiama «altrimenti». La condizione intera sta nel pannello; il nome accessibile del task a valle dice il ramo («ramo Punteggio alto di Valuta»). Nella vista delle esecuzioni resta anche sul ramo non preso. |
| workflow-bar | testa dell'editor e della vista delle esecuzioni | Nome, versione, stato, origine, metadati e azioni secondo il contesto: nell'editor «Salva» (`button-primary`) ed «Esegui» (`button-secondary`) (T-52); «Annulla esecuzione» con un'esecuzione in corso; «Elenco» e «Grafo» per passare alla `list-view` e tornare. In un'esecuzione la versione è quella con cui è partita (FR-5). Un workflow definito come codice ha il `chip` «da codice» e, nell'editor, il messaggio di sola lettura di Voice and Tone nei metadati. Stati di «Salva» ed «Esegui» in State Patterns, Stati trasversali. Ospita il `pause-notice`. |
| button-primary | barre, pannelli, `confirm-dialog`, `login-card` | Al massimo uno per area. L'etichetta è un verbo. «Salva» resta attivo anche con errori: non salva e mostra il riepilogo degli errori (vedi Interaction Primitives, Salvare ed eseguire). |
| button-secondary | barre, pannelli, `empty-state`, `load-error`, `confirm-dialog` | Azioni di contorno: «Aggiorna», «Sostituisci», «Riprova», l'azione di uno stato vuoto. |
| icon-button | X del `panel`, zoom e «Riordina» del `canvas`, «Comprimi palette» e «Apri palette», testa del `bottom-sheet` | Nome accessibile sempre presente («Chiudi», «Ingrandisci»). |
| confirm-dialog | «Elimina» un segreto o un workflow, «Salva e attiva» una query modificata, «Salva» che cambia lo schema di input di un workflow avviato da una pianificazione o da pipeline stream, uscita dall'editor con modifiche non salvate (con la navigazione del kit o cambiando tenant) | Dialogo modale con la domanda e la conseguenza (testi in Voice and Tone). Il fuoco parte dal pulsante meno distruttivo, «Chiudi»; `Esc` chiude senza agire; alla chiusura il fuoco torna al controllo che l'ha aperto. Numero dei workflow che usano il segreto: T-63; conferma di «Annulla esecuzione»: T-62. |
| panel | sinistra, destra e basso dell'area; riquadri della dashboard | Nell'editor il pannello sinistro contiene la palette (`palette-item`) e si comprime con «Comprimi palette» (T-51). Il pannello destro segue la selezione: nell'editor senza selezione mostra il pannello del workflow («Input del workflow», «Pianificazione»), nella vista delle esecuzioni un `empty-state`; si chiude con la X o con `Esc`. Contenuto del pannello del task e del workflow nell'editor: Interaction Primitives, Selezione e configurazione. Nell'esecuzione mostra la versione del nodo eseguita (FR-15); per un task saltato dice perché, con un link alla dipendenza o allo `switch` (FR-2). Il pannello in basso, assente nell'editor, sta sotto il canvas e ha le tab Log, Input e Output del task o dell'elemento selezionato. Sul telefono i pannelli dell'area diventano `bottom-sheet`. |
| tabs | pannello in basso | Schema a tab WAI-ARIA: le frecce cambiano tab, `Tab` entra nel contenuto. |
| attempt-card | pannello destro, sezione «Tentativi» | Una card per tentativo, in ordine di tempo (FR-7: ogni tentativo è un'attività distinta). L'ultimo tentativo fallito conserva il suo errore. Mostra la versione del nodo eseguita (FR-15); l'ID del worker solo dove la regola di Voice and Tone lo ammette. |
| code-block | pannello destro (Input, Output), tab Input e Output | JSON formattato in sola lettura, sempre come testo (NFR-7). Non contiene mai valori di segreti (FR-33). Un contenuto lungo scorre dentro il blocco. |
| code-editor | editor delle pipeline stream (SQL), «Mostra JSON» dell'`input-form` (JSON), modo jq del `data-mapping` | Valida con il nucleo JS prima del salvataggio (FR-39): gli errori compaiono con riga e colonna (FR-43). «Salva e attiva» non parte finché ci sono errori. Ha un modo in cui `Tab` sposta il fuoco, attivabile da tastiera e indicato a vista («Esc, poi Tab per uscire») (WCAG 2.1.2). Sul campo `autocapitalize="off"`, `autocorrect="off"`, `spellcheck="false"` e virgolette dritte. Libreria: T-58. |
| log-line | tab Log | Una riga per evento, con orario al millesimo, livello e messaggio. Area del log e pausa: vedi Tempo reale. |
| palette-item | palette dell'editor | Voci raggruppate secondo il gruppo del manifest (Foundation); i costrutti di FR-1 stanno nel gruppo «Controllo di flusso». Ogni voce mostra nome e descrizione dal manifest. Mostra solo funzioni, nodi personalizzati e segreti disponibili al tenant (FR-40, FR-15), più i costrutti; le voci omonime si distinguono con il `chip` di provenienza, e un nodo omonimo del tenant non cambia ciò che esegue un workflow dell'integratore (FR-29). Un clic o `Invio` aggiunge il task; si può anche trascinare sul canvas. Nessun catalogo di nodi pronti (PRD §7). Inserimento: su desktop il task trascinato resta dove è lasciato, con clic o `Invio` va accanto al task selezionato, a valle, o in fondo se nessuno è selezionato; trascinato su un arco si inserisce tra i due task; sul telefono va dopo il selezionato, nella disposizione automatica [ASSUMPTION A-7]. |
| form-field | pannello destro, `data-mapping`, `input-form`, Segreti, Accesso | Etichetta sempre visibile. Un campo generato da uno schema (manifest o input del workflow) prende etichetta e aiuto dalla descrizione del campo, dice «obbligatorio» a parole e usa il controllo del suo tipo: testo, numero, sì/no, scelta, elenco. L'errore sta sotto il campo, con `validation-message`; il campo ha `aria-invalid` e `aria-describedby`. Retry, timeout e parallelismo mostrano l'aiuto «predefinito: `<valore>`, massimo: `<valore>`», letti dall'installazione e mai scritti nella UI; un valore sopra il massimo è un errore del campo (NFR-14). Il valore di un segreto si scrive e non si rilegge più: dopo il salvataggio si vede «Valore salvato, non visibile» con «Sostituisci» (FR-32, FR-42). In Accesso: `autocomplete="current-password"`, incolla consentito, «Credenziale non valida.» legato al campo con `aria-describedby` e `role="alert"`. |
| select | selettore dei segreti di una connessione nel pannello, filtri «Tenant», «Workflow», «Stato», «Ripeti» della `schedule-section`, `tenant-selector` | Come `form-field`, con un elenco di scelte. Il selettore dei segreti elenca solo i nomi (FR-32) dei segreti che soddisfano quella connessione; vuoto, dice «Nessun segreto per Censys.». |
| data-mapping | pannello del task, sezione «Dati» | Una riga per campo dello schema di input del manifest, nell'ordine dello schema. Ogni riga sceglie la fonte: «Valore» (un `form-field` del tipo), «Da un task» (un `select` con i soli campi compatibili degli schemi di output dei task a monte, dell'elemento corrente del fan-out e dell'input del workflow, scritti `task · campo`, «`<fan-out>` · elemento» e «Input del workflow · `<campo>`») o «jq», il modo avanzato con un `code-editor`. Nel task di join la sezione mostra cosa riceve: un risultato per elemento del fan-out, o l'errore al posto del risultato secondo la politica di FR-3. Prima del salvataggio (FR-40) un campo obbligatorio senza fonte, un tipo incompatibile, jq non valido o con `$ENV`, `env`, `input_filename`, `import`, `include` (FR-17) danno l'errore sul campo (testi in Voice and Tone). |
| schedule-section | pannello del workflow, sezione «Pianificazione» | «Ripeti» sceglie tra «Nessuna», «Ogni» (intervallo: numero e unità) e «Cron» (espressione; l'errore dice la posizione). Sotto, la prossima scadenza. La sezione porta l'input delle esecuzioni pianificate, con gli stessi campi dell'`input-form`, controllato sullo schema di input al salvataggio e a ogni scadenza (FR-6). «Sospendi» ferma le scadenze senza toccare il workflow e mostra «Sospesa»; «Riprendi» le riattiva dalla prossima scadenza, senza recuperare quelle cadute durante la sospensione (FR-6). Dopo una pausa del motore dice quante scadenze sono andate perse e quante sono state recuperate, secondo la regola dell'installazione (NFR-14). Una scadenza non accettata compare con il motivo (Voice and Tone), mai in silenzio (FR-6). Le esecuzioni che avvia dicono «da pianificazione» (Voice and Tone). Il tenant ha una pianificazione per workflow, sua e non del workflow (FR-57): si crea, si cambia, si sospende e si toglie anche su un workflow dell'integratore o definito come codice, che restano in sola lettura. La pianificazione scritta nel documento OWS del workflow (FR-1, FR-6), se c'è, compare nella sezione in sola lettura, distinta da quella del tenant. |
| input-form | pannello destro dopo «Esegui», foglio sul telefono | Nell'editor prende il posto del contenuto del pannello destro, con il canvas accanto. Generato dallo schema di input del workflow (FR-41, FR-56): un `form-field` per campo. Il campo elenco accetta testo incollato, una riga per elemento; le righe vuote si ignorano, il conteggio sotto il campo («300 elementi») sta in un `role="status"` e un elemento del tipo sbagliato dà l'errore con il numero di riga. «Mostra JSON» passa a un `code-editor` con lo stesso valore; «Mostra modulo» torna al modulo quando il JSON rispetta lo schema. Nel JSON gli errori hanno riga e colonna. Senza schema di input si apre direttamente il JSON. «Esegui» non parte finché ci sono errori. |
| validation-message | pannello destro, sotto i campi, `code-editor` | Nell'editor nomina task e campo (FR-40), con un suggerimento quando il motore lo conosce; per SDK e documento dice percorso e motivo (FR-4); nell'SQL riga e colonna (FR-19); per un ciclo i task coinvolti (FR-2). Sta accanto al task o al campo che lo causa (FR-40). |
| tooltip | `icon-button`, icone delle barre dense | Regole in Accessibility Floor. |
| rail-item | barra icone dell'area workflow | Cinque voci con etichetta sempre visibile. La voce attiva ha `aria-current="page"`. |
| tenant-selector | fascia in cima all'area workflow | Solo con la credenziale dell'integratore e più di un tenant; con ruolo tenant non esiste (FR-28), e con il solo tenant predefinito non c'è: il kit lavora lì (FR-58, PRD §4.6). Il `select` «Tenant» elenca i tenant dell'integratore, con ricerca per nome. Il tenant scelto vale per tutte le sezioni: elenchi, editor, esecuzioni, segreti e pipeline stream mostrano solo le sue risorse e i workflow dell'integratore, «Nuovo workflow» crea un workflow suo e «Esegui» lancia per lui (FR-28). Cambiare tenant con modifiche non salvate apre il `confirm-dialog` di uscita; dopo il cambio non resta nulla del tenant precedente, eventi in tempo reale compresi (FR-27). Il tenant scelto è un campo della credenziale firmata dall'ospite (FR-26, FR-27): per cambiarlo il kit chiede all'ospite una credenziale nuova (meccanismo: architettura). Nei pezzi singoli il selettore non c'è: vale il tenant della credenziale che l'ospite passa al componente. |
| nav-item | `dashboard-bar` | `NavLink`, mai `<a href>`. La voce attiva ha `aria-current="page"`. |
| dashboard-bar | testa di ogni pagina della dashboard | Wordmark, navigazione e operatore; il titolo di pagina è l'intestazione principale della pagina. |
| login-card | Accesso | Un `form-field` per la credenziale e «Accedi». Dice il punto esatto in cui la credenziale del primo avvio è stampata, senza mostrare dati (FR-45); il testo esatto arriva dall'architettura (T-63). |
| data-table | elenchi del kit (workflow, esecuzioni, segreti, pipeline stream, ultime righe emesse) e tabelle della dashboard | La prima cella di ogni riga è un link o `button` col nome, che apre il dettaglio (niente clic sulla `tr`); la riga corrente ha `aria-current` o `aria-selected`; le intestazioni ordinabili sono `button` con `aria-sort`; stessa regola per le card impilate del telefono. Numeri con cifre tabulari. Esecuzioni della dashboard: filtri «Tenant», «Workflow», «Stato» come `select` (FR-46), «Tenant» solo con più di un tenant (PRD §4.10), filtro «in coda» e colonna ordinabile «In coda da», l'età del task o dell'attività in coda più vecchi dell'esecuzione, vuota se non c'è nulla in coda (FR-46). Segreti: nome, connessione, data, ultimo uso, mai il valore (FR-32). Quote: una colonna per ogni quota di FR-30 (attività concorrenti, esecuzioni al minuto, sandbox, pipeline stream, eventi al secondo, richieste API al secondo, spazio), ognuna con la barra d'uso, e gli stati «Vicino al limite» e «Al limite» (FR-49, UJ-4; soglia di «vicino»: FR-49, NFR-14). Pipeline stream della dashboard: «Ritardo sul watermark», «Eventi al secondo», «Memoria di stato», «Eventi in ritardo scartati», «Ultimo checkpoint» (FR-47), più gli eventi nel futuro oltre la tolleranza (FR-21). |
| history-list | Worker, dettaglio di un worker | Cronologia recente con le fasi ricevuta, in esecuzione, completata, fallita, ritentata, annullata (FR-48); la più recente in alto. |
| recovery-banner | in cima a ogni pagina della dashboard (il mock lo mostra solo in Worker) | Compare quando inizia la ripresa del motore (NFR-15), con il tempo trascorso; non si chiude a mano e sparisce quando il motore torna a consegnare attività. Il testo (Voice and Tone) viene dal mock C, scritto «breve e piano, sarà rivisto» [ASSUMPTION A-4]. La regione live annuncia solo l'inizio e la fine; il contatore sta fuori. Lo stesso riquadro, con il suo testo (Voice and Tone), avvisa quando il motore rifiuta lavoro nuovo per disco pieno (FR-48, NFR-6): è un avviso distinto dalla ripresa, con le stesse regole di chiusura e di annuncio, e sparisce quando lo spazio torna. Se ripresa e disco pieno coincidono, i due avvisi stanno insieme, uno sopra l'altro. |
| pause-notice | `workflow-bar` della vista delle esecuzioni e dell'editor delle pipeline stream | Compare quando gli aggiornamenti si fermano: stati e righe restano fermi, non spariscono. Ha `role="status"` e sparisce da solo quando gli aggiornamenti riprendono. Non parla di motore né di crash. |
| load-error | ogni area che non si carica | Messaggio nell'area più «Riprova»; la cornice resta. Una risorsa di un altro tenant non è un errore di caricamento (vedi State Patterns). |
| bottom-sheet | telefono: palette, configurazione, log/input/output | Sale dal basso sopra lo stesso canvas del desktop, invece di un editor a elenco separato (memlog). In testa «Chiudi» ed «Espandi»/«Comprimi»; la maniglia è un'aggiunta; alla chiusura il fuoco torna al nodo. Altezze e tastiera virtuale in Responsive & Platform. |
| empty-state | ogni area senza contenuto | Una frase che dice cosa fare e al massimo un'azione. |
| skeleton | ogni area in caricamento | Ha la forma del contenuto atteso e `aria-busy="true"` sull'area; «Caricamento di …» sta in un `role="status"` fuori dall'area occupata. La cornice resta al suo posto. |

## State Patterns

### Elenco chiuso degli stati

Elenco proposto dalla UX e confermato nell'aggiornamento del PRD (2026-11-20); la propagazione di
fallito e saltato resta la domanda aperta 8 del PRD, affidata all'architettura. La base sono i
«Ruoli di stato» del brief delle direzioni e i termini sparsi nel PRD. Le famiglie di colore di
ricevuta (neutra), collegato (completata) e non risponde (fallita) vengono dal mock C; quelle della
pipeline stream, di «Vicino al limite» e di «Al limite» sono proposte dalla UX [ASSUMPTION A-9].

| Oggetto | Stati | Famiglia (token del primo piano) |
|---|---|---|
| Esecuzione | in coda · in esecuzione · completata · fallita · annullata · scaduta | `{colors.state-neutral}` · `{colors.state-running}` · `{colors.state-completed}` · `{colors.state-failed}` · `{colors.state-neutral}` · `{colors.state-neutral}` |
| Task | in attesa (dipendenze) · in coda (pronto, attività non consegnata) · in esecuzione · ritentato · completato · fallito · saltato · annullato | attesa, coda, saltato, annullato `{colors.state-neutral}`; ritentato `{colors.state-retried}`; gli altri come sopra |
| Attività | in coda · ricevuta · in esecuzione · ritentata · completata · fallita · annullata | ricevuta e annullata `{colors.state-neutral}`; le altre come sopra |
| Worker | collegato · non risponde | `{colors.state-completed}` · `{colors.state-failed}` |
| Pipeline stream | attiva · rallentata · sospesa | `{colors.state-running}` · `{colors.state-retried}` · `{colors.state-neutral}`; causa di una sospensione imposta dal sistema in `{colors.state-failed}` |
| Tenant (Quote) | vicino al limite · al limite | `{colors.state-neutral}` · `{colors.state-retried}` |
| Pianificazione | attiva · sospesa | `{colors.state-running}` · `{colors.state-neutral}` |

- «In attesa» e «in coda» sono diversi. In attesa: le dipendenze non sono finite. In coda: il
  task è pronto ma l'attività non è consegnata, perché manca un worker, c'è una quota o il pool
  della sandbox è vuoto. L'attesa in coda non consuma tentativi (FR-8). Un'esecuzione è «in
  coda» finché nessun suo task è partito: un `wait` in corso conta come partito. Sono le parole
  del PRD (§3).
- Il timeout è il motivo di un fallimento, non uno stato: «Fallito: timeout dopo 30 s» (FR-8).
  «Scaduta» vale solo per l'esecuzione.
- FR-14 e FR-48 usano la stessa parola per il worker: «non risponde».
- Una pipeline stream è «sospesa» per scelta del tenant con «Sospendi» (FR-23) o per il sistema.
  La sospensione imposta dal sistema aggiunge un `state-badge` della causa («per CPU», FR-17,
  FR-25; «per memoria», di stato o di esecuzione, FR-17, FR-25; «per arretrato», FR-30) e il
  messaggio del caso. Gli eventi già usciti dalla conservazione alla ripresa sono un errore, non
  una causa (FR-23). Lo stato mostra anche le righe trattenute (FR-24) e le righe che non sono
  diventate un'esecuzione, con il motivo (FR-6).
- Un'esecuzione annullata dice chi (il ruolo e, se c'è, il nome visualizzato) e quando (FR-10). In
  un'esecuzione annullata o scaduta i task non partiti diventano «annullato», con badge grigio e
  arco tratteggiato come gli altri task non partiti.
- Il pannello di un task saltato dice «Saltato: dipendenza `<task>` fallita» con un link al task
  (FR-2), o «Saltato: ramo «`<etichetta>`» non preso» con un link allo `switch`; la regola si
  rivede quando il PRD chiude la domanda aperta 8.

### Stati per superficie

Caricamento in ogni area: `skeleton`. Errore di caricamento in un'area: `load-error`. Una risorsa
di un altro tenant, o già eliminata, risponde «come se non esistesse» (FR-26, FR-31): «Workflow
non trovato.», «Esecuzione non trovata.», «Segreto non trovato.», «Pipeline stream non trovata.»,
mai «accesso negato». Testi degli stati vuoti e di «non trovato» proposti dalla UX; il PRD non ne
descrive [ASSUMPTION A-11].

Mock: [vista delle esecuzioni](mockups/key-execution-view.html), task in esecuzione e in attesa,
elemento fallito, errore di riga nell'`input-form`; [editor](mockups/key-editor-desktop.html),
errore di validazione sul task prima del salvataggio;
[editor delle pipeline stream](mockups/key-stream-editor.html), «Attiva», «Sospesa», «Rallentata»
ed errore SQL con riga e colonna.

| Superficie | Vuoto | Altri stati |
|---|---|---|
| Area workflow | — | sessione scaduta: vedi Stati trasversali |
| Workflow (elenco) | «Nessun workflow salvato.» + «Nuovo workflow»; il workflow di esempio (FR-50) compare solo nel tenant predefinito di un'installazione nuova, non nell'elenco di ogni tenant | workflow non trovato; workflow dell'integratore con `chip` «Dell'integratore»; workflow definito come codice con `chip` «da codice»; «Elimina» su un workflow del tenant, con `confirm-dialog`, ed eliminazione rifiutata con l'elenco di chi lo usa (FR-58) |
| Editor dei workflow | «Canvas vuoto: aggiungere un task dalla palette.»; palette senza funzioni: «Nessuna funzione disponibile.» | errori di validazione sul task (FR-40); conflitto di versione (FR-5); salvataggio in corso o non riuscito; in sola lettura per tutti, con il suo messaggio, un workflow dell'integratore e un workflow definito come codice, con la «Pianificazione» che resta modificabile (FR-57); senza posizioni salvate, disposto in automatico |
| Vista delle esecuzioni | elenco: «Nessuna esecuzione negli ultimi `<N>` giorni.», N dalla configurazione (FR-31); pannello senza selezione: «Selezionare un task per vedere log, input e output.» | in tempo reale; pausa; fan-out con migliaia di elementi; elemento fallito; esecuzione annullata o scaduta; esecuzione non trovata |
| Segreti | «Nessun segreto salvato.» + «Aggiungi segreto» | valore mai mostrato (FR-32); eliminazione con `confirm-dialog`; segreto non trovato |
| Pipeline stream (kit) | «Nessuna pipeline stream.» + «Nuova pipeline stream»; ultime righe: «Nessuna riga emessa finora.»; stream: «Nessuno stream disponibile.» | errori SQL con riga e colonna; attiva, rallentata, sospesa con la causa; righe trattenute; righe rifiutate con il motivo (FR-6); pausa; pipeline stream non trovata |
| Accesso (dashboard) | — | «Credenziale non valida.»; sessione scaduta; nessun dato senza credenziale (FR-45) |
| Esecuzioni (dashboard) | «Nessuna esecuzione con questi filtri.»; senza alcuna esecuzione anche `button-secondary` «Apri la guida rapida» | dettaglio del task senza valori dei segreti (FR-46); esecuzioni ferme in coda: filtro e colonna «In coda da»; nel dettaglio di un'esecuzione in corso «Annulla esecuzione», con la stessa conferma del kit (FR-10) |
| Pipeline stream (dashboard) | «Nessuna pipeline stream.» | attive, rallentate e sospese, con la causa (FR-47); errori visibili anche all'operatore (FR-17, FR-23, FR-25, FR-30); righe rifiutate con il motivo (FR-6) |
| Worker | «Nessun worker collegato.» + `button-secondary` «Apri la guida rapida» | non risponde, con attività riassegnate (FR-14); funzione registrata di nuovo con connessioni diverse, con il testo di Voice and Tone (FR-48); host senza KVM: riga fissa in cima alla pagina, con `state-badge` della famiglia ritentata e il testo di Voice and Tone (FR-54, T-49) |
| Quote | — (dal primo avvio c'è il tenant predefinito) | tenant vicini al limite o al limite (FR-49); un filtro senza risultati: «Nessun tenant vicino al limite.»; con il solo tenant predefinito, le sue quote senza nome del tenant né filtro (PRD §4.10) |

### Stati trasversali

- **Tempo reale.** Regole nella sezione Tempo reale.
- **Quota raggiunta.** Nel kit le nuove attività restano «In coda» con il motivo («quota di
  attività concorrenti raggiunta», FR-30). Una richiesta oltre il tetto dell'arretrato o dello
  spazio riceve un errore di quota prima di essere accettata (FR-30), accanto a «Esegui». Nella
  dashboard il tenant risulta «Al limite» in Quote.
- **Errori di validazione.** Nell'editor del grafo stanno sul task che li causa, prima del
  salvataggio (FR-40), compresi tipi e campi obbligatori degli schemi del manifest e connessioni
  senza segreto. Un workflow non valido non si salva: il salvato deve essere un documento OWS
  valido (FR-40). Forma del messaggio fuori dall'editor (documento, SQL): `validation-message`.
- **Salvare ed eseguire.** «Salva» in corso: il pulsante ha `aria-busy` e dice «Salvataggio…».
  Errore di rete: «Salvataggio non riuscito.» più `button-secondary` «Riprova»; il grafo resta
  nell'editor. «Esegui» apre subito la vista con l'identificativo e lo stato «In coda» (FR-6); un
  rifiuto prima dell'accettazione compare accanto al pulsante (FR-29, FR-30, NFR-6). Disco
  pieno nella dashboard: avviso nel riquadro del `recovery-banner`. Input non valido: vedi
  `input-form`; uscita
  con modifiche non salvate: vedi `confirm-dialog`; validazione e conflitti di versione:
  Interaction Primitives, Salvare ed eseguire.
- **Workflow dell'integratore (FR-29).** Ha il `chip` «Dell'integratore» nell'elenco Workflow e
  nella `workflow-bar`. Nasce dal codice (FR-28, FR-35): l'editor di quel workflow è in sola
  lettura per tutti, integratore compreso, «Esegui» resta attivo e un messaggio dice che non è
  modificabile (Voice and Tone). All'avvio ogni connessione si risolve tra i segreti del tenant:
  se nessuno la soddisfa, il rifiuto compare accanto a «Esegui», prima dell'accettazione, con la
  connessione mancante (FR-29). Con la credenziale dell'integratore il `tenant-selector` sceglie
  il tenant (T-43) e «Nuovo workflow» crea un workflow di quel tenant (FR-28).
- **Workflow definito come codice (FR-35).** Si modifica solo dal codice: nell'editor è in sola
  lettura per tutti, integratore compreso, come un workflow dell'integratore.
  «Esegui» resta attivo; il `chip` «da codice» dice la provenienza e il messaggio di Voice and
  Tone dice che si modifica dal codice.
- **Ripresa del motore (NFR-15).** È una pausa, non una perdita. Il kit mostra il `pause-notice`
  («Aggiornamenti in pausa, riprendono da soli.»), con i valori fermi (Tempo reale, Pausa). La
  dashboard mostra il `recovery-banner` esplicito, con il tempo trascorso.
- **Canale in tempo reale interrotto.** Se cade il collegamento degli aggiornamenti (rete, non
  motore), il kit mostra lo stesso `pause-notice` [ASSUMPTION A-12]; per la dashboard: T-68.
- **Sessione scaduta (FR-27, FR-45).** Il kit non chiede un secondo login: avvisa l'app ospite con
  un evento almeno 60 s prima della scadenza (WCAG 2.2.1) e di nuovo alla scadenza. Grafo, SQL e
  JSON non salvati restano in memoria e tornano dopo il rinnovo solo se la credenziale nuova porta
  lo stesso tenant e lo stesso ruolo; altrimenti si scartano senza mostrarli (FR-27). Un
  ricaricamento della pagina li perde: il kit non li salva nel browser. Alla scadenza il messaggio
  di Voice and Tone compare nell'area con `role="alert"`. La dashboard torna ad Accesso con il suo
  messaggio, senza dati (FR-45). Nome dell'evento e limite dei tentativi su Accesso: T-63.

## Interaction Primitives

### Canvas

- Trascinamento sul vuoto per spostarsi; rotella o pizzico per lo zoom; i tre pulsanti di zoom
  del `canvas` per chi non ha né l'una né l'altro (WCAG 2.5.1).
- Direzione adattiva: lo stesso workflow scorre da sinistra a destra su desktop e dall'alto in
  basso sul telefono.
- Disposizione su desktop, manuale e automatica (T-46): i nodi si spostano col trascinamento, e
  «Riordina» li dispone in automatico da sinistra a destra, al posto delle posizioni manuali
  (annulla e ripristina: T-62). Le posizioni del desktop si salvano con il workflow; dove
  (metadati del documento OWS o archivio separato) lo decide l'architettura. Un workflow senza
  posizioni salvate, per esempio definito come codice, si apre disposto in automatico.
- Disposizione sul telefono: sempre automatica, dall'alto in basso, ignorando le posizioni del
  desktop. Le posizioni manuali valgono dove vale la direzione da sinistra a destra (soglia in
  Responsive & Platform).
- Più radici e più foglie sono ammesse (FR-2): la disposizione automatica le mette su colonne
  separate.

### Selezione e configurazione

- Un clic su un nodo lo seleziona: contorno e alone, e il pannello destro si apre.
- Nell'editor il pannello del task contiene, in quest'ordine: nome; funzione o nodo con la
  descrizione del manifest; «Dati» (`data-mapping`), i cui campi nascono dallo schema di input del
  manifest (T-41); «Connessioni», un `select` per ogni connessione dichiarata dal manifest con i
  soli segreti che la soddisfano (per `call http`, senza manifest, tutti i segreti del tenant:
  FR-32); retry (tentativi, attesa, crescita dell'attesa: FR-7) e cattura
  dell'errore (`try` di FR-1), campi del task e non nodi; timeout (FR-8), nella stessa sezione del
  retry (titolo della sezione ancora aperto: Elementi aperti); per un fan-out la politica di errore
  («fallisce tutto il fan-out» oppure «il join riceve l'errore al posto del risultato») e il
  parallelismo massimo (FR-3); «Collegamenti» e «Elimina task» (vedi Collegare);
  lo schema di output del nodo in sola lettura (FR-55).
- Nell'editor, senza selezione, il pannello del workflow contiene «Input del workflow» e
  «Pianificazione» (`schedule-section`, T-42); su desktop e sul telefono si apre anche con il nome
  del workflow nella `workflow-bar`, che è un `button`. «Input del workflow» è lo schema di input,
  un campo per riga con nome, tipo e «obbligatorio» (FR-56; come si scrive nel documento:
  domanda aperta 7 del PRD), e
  genera l'`input-form` di «Esegui»; in un workflow nuovo parte dallo schema di input del primo
  task [ASSUMPTION A-41].
- Nell'esecuzione il pannello contiene tentativi, input e output.

### Collegare

- Si trascina da una porta d'uscita a una porta d'ingresso, oppure si fa clic sulla porta d'uscita
  e poi sul task, come sul telefono. Durante la scelta si annuncia «Collegamento da `<task>`:
  scegliere la destinazione»; `Esc` annulla.
- «Collega a…» nel pannello del task apre l'elenco dei soli task validi.
- Bersaglio valido: anello tratteggiato più etichetta, mai solo tinta (WCAG 1.4.1). Colori: T-61.
- I collegamenti sono liberi (FR-2): un ciclo si segnala, e si annuncia, con i task coinvolti.
- Nel pannello del task la sezione «Collegamenti» elenca predecessori e successori, ognuno con
  «Rimuovi collegamento a `<task>`»; in fondo al pannello «Elimina task» (WCAG 2.1.1). Annulla e
  ripristina, menu del nodo: T-62.

### Aggiungere dalla palette

- Palette per gruppo nel `panel` sinistro su desktop, solo nell'editor.
- Clic, `Invio` o trascinamento (regola d'inserimento in `palette-item`).
- Tutti i costrutti di FR-1 si disegnano (T-40). Nel gruppo «Controllo di flusso»: `switch`,
  fan-out (`for` e `fork`, disegnati come `fanout-group`), `wait`, `raise`, `set`, `call http` e
  sotto-workflow; i loro campi vengono dallo schema del costrutto nel formato, come quelli di una
  funzione vengono dal manifest. I campi di «Dati» di un sotto-workflow vengono dallo schema di
  input del workflow chiamato (FR-56). `try` e retry sono campi del task (Selezione e
  configurazione).
- `switch`: nodo con un'uscita per ramo, ogni arco con il suo `condition-chip`. Nel pannello la
  sezione «Rami» elenca i rami in ordine, ognuno con etichetta e condizione; l'ultimo può essere
  «altrimenti», senza condizione. La condizione usa la scelta guidata della sezione «Dati», con jq
  come modo avanzato [ASSUMPTION A-40].

### Salvare ed eseguire

- «Salva» valida con il nucleo JS e crea una versione (FR-5); non esiste un passo «pubblica»
  (T-52). Se il salvataggio cambia lo schema di input di un workflow avviato dalla pianificazione
  o da pipeline stream, prima chiede conferma con il `confirm-dialog` (FR-5). Con errori non
  salva: accanto al pulsante compare un riepilogo con `role="alert"` («3 errori in 2 task») con i
  link ai task, e il fuoco va lì, non sul canvas.
- Un rifiuto del motore dopo «Salva», o una funzione sparita all'esecuzione, si mostra come gli
  errori del nucleo JS, sul task (FR-4).
- Se un altro ha pubblicato prima, il salvataggio è rifiutato con la versione corrente: niente
  sovrascritture silenziose. Oltre al messaggio, «Ricarica ultima versione» e «Scarica le
  modifiche», così il lavoro non si perde. Scelta della versione da eseguire e cronologia: T-62.
- «Esegui» chiede l'input con l'`input-form` nel pannello destro, generato dallo schema di input
  del workflow (T-47): per esempio il campo elenco con i 300 indicatori incollati. «Mostra JSON» è
  il modo avanzato.
- «Annulla esecuzione» ferma un'esecuzione in corso (FR-10).

### Pipeline stream

- Accanto al `code-editor` SQL, come pannello destro, il pannello «Parametri»: gli stream del
  tenant in sola lettura, con la loro conservazione (FR-18), e la destinazione, a scelta tra
  «Stream del tenant» e «Workflow» (FR-24, FR-43). Uno stream di un altro tenant risulta
  inesistente (FR-26). Finestra, ritardo tollerato e checkpoint: T-56.
- «Salva e attiva» su una query già attiva e modificata chiede conferma con `confirm-dialog`:
  la modifica azzera lo stato (FR-23).

### Tastiera

Il fuoco sul canvas è una sola fermata di `Tab`; i tasti valgono solo con il fuoco nel canvas, per
non scontrarsi con le scorciatoie dell'app ospite [ASSUMPTION A-17].

- Fuoco e selezione sono distinti. Le frecce spostano il fuoco da un nodo all'altro seguendo gli
  archi (sinistra e destra su desktop, su e giù sul telefono); con più figli seguono l'ordine del
  documento e `Shift+frecce` passa tra fratelli. Si parte dalla prima radice del documento.
- `Spazio`: seleziona. `Invio`: apre il pannello e vi porta il fuoco; `Esc` riporta il fuoco al
  nodo. `Esc` chiude pannello o foglio solo se il fuoco non è in un editor di codice; poi toglie
  la selezione.
- `Invio` sul `fanout-group` entra nei conteggi e in «altri N falliti».
- Dopo un'aggiunta il fuoco va al nuovo nodo, dopo un'eliminazione al predecessore; il canvas
  scorre per tenere visibile il nodo col fuoco.
- `Canc`: elimina il task selezionato (editor); i collegamenti si tolgono dal pannello.
- `C`: inizia un collegamento dal nodo selezionato; le frecce scelgono la destinazione, `Invio`
  conferma, `Esc` annulla.
- `A`: porta il fuoco sul primo `palette-item`.
- `Alt+frecce`: sposta il nodo col fuoco di un passo della griglia dei punti, su desktop
  nell'editor; è l'alternativa al trascinamento (WCAG 2.1.1) [ASSUMPTION A-44].
- `+` e `−`: zoom. `0`: adatta.
- Meccanismo (roving tabindex o `aria-activedescendant`): T-58.

### Telefono

- Stesso canvas, un dito per spostarsi, pizzico per lo zoom, tocco per selezionare; i pulsanti di
  zoom restano. Il gesto a un dito resta nel canvas: la pagina dell'ospite si scorre solo fuori
  dal canvas; il kit non chiede mai `user-scalable=no`.
- Palette e configurazione salgono come `bottom-sheet`: niente editor a elenco separato. I nodi
  non si spostano a mano: la disposizione è automatica (Canvas).
- Per collegare si tocca la porta d'uscita e poi il task di destinazione. Con `pointer: coarse`
  ogni bersaglio misura almeno `{spacing.target-min}`, porte comprese tramite un'area di tocco più
  larga del cerchio, e l'arco ha un'area di tocco invisibile della stessa misura. Le aree delle
  porte si verificano contro la distanza minima tra nodi a 360 px.

## Accessibility Floor

Comportamento. I contrasti stanno in `DESIGN.md`, Colors.

- **Livello.** WCAG 2.1 AA, confermato dal PRD in NFR-9, per tutti i componenti del kit, area
  workflow compresa, e per la dashboard.
- **Tastiera.** L'editor del grafo si usa tutto da tastiera (NFR-9), con lo schema di Interaction
  Primitives. L'ordine di `Tab` segue l'ordine di lettura (WCAG 2.4.3); filtri e `tabs` della
  dashboard compresi.
- **Alternativa al grafo.** La `list-view`, in sola lettura, per seguire e leggere un workflow
  (T-39). La modifica resta sul canvas, operabile da tastiera (Interaction Primitives, Tastiera) e
  con le azioni del pannello: «Collegamenti», «Collega a…», «Elimina task». Rischio noto: con un
  lettore di schermo costruire un workflow da zero senza vedere il canvas resta più difficile che
  seguirlo; la prova con NVDA e VoiceOver comprende la costruzione del workflow di UJ-2.
- **Grafo al lettore di schermo.** Il `canvas` è una regione con nome; il nome accessibile del nodo
  sta in `node-card`. Il ruolo non spegne la navigazione del lettore di schermo: niente
  `role="application"` (ruolo esatto con la libreria: T-58).
- **Kit incorporato.** Il kit non rende mai `main`, che è dell'ospite. Regioni e `nav` hanno un
  `aria-label` («Sezioni workflow»); il livello delle intestazioni è una proprietà dei componenti;
  un link «Salta al canvas» segue la barra icone (WCAG 2.4.1).
- **Mai solo colore.** Ogni stato ha icona e parola; la `status-bar` ha l'`aria-label` dei
  conteggi; nessuna icona di stato senza testo vicino o `aria-label`, anche nelle barre dense.
- **Focus visibile.** Anello a doppio filo (`DESIGN.md`, Components, Selezione e focus); nella
  dashboard il filo esterno è `{colors.accent-dashboard}`.
- **Movimento.** Con `prefers-reduced-motion` nessuna transizione su zoom, adattamento, apertura
  dei fogli e cambio dei badge; l'icona «in esecuzione», le righe del segmento in esecuzione e lo
  `skeleton` restano fermi. Mai oltre 3 lampeggi al secondo (WCAG 2.3.1).
- **Elementi nativi.** `button`, `nav` e link del router; ogni controllo ha un nome accessibile;
  `aria-expanded` e `aria-controls` su pannelli e fogli.
- **Tooltip.** Aggiungono al nome, non lo sostituiscono. Si chiudono con `Esc`, si possono
  sorvolare e restano finché servono (WCAG 1.4.13).
- **Testo puro.** Il contenuto dei tenant non è mai interpretato come HTML (NFR-7).

## Responsive & Platform

| Larghezza | Comportamento |
|---|---|
| Desktop (board 1280, 1440, 1920) | Kit: barra icone, palette nell'editor, canvas, pannello destro fisso, pannello in basso con tab nella vista delle esecuzioni. Dashboard: barra in alto e pagine con pannelli affiancati. Grafo da sinistra a destra. |
| Telefono (board 390 e 360) | Canvas a tutta larghezza, grafo dall'alto in basso. Palette, configurazione e log/input/output come `bottom-sheet`. La `workflow-bar` va su due righe: nome e versione sopra, azioni sotto. |
| Soglia tra i due | Il layout da telefono e la direzione dall'alto in basso valgono sotto `{spacing.breakpoint-compact}` (768 px); tra 768 e 1279 px vale il layout desktop con il pannello destro sopra il canvas [ASSUMPTION A-21]. |
| Telefono in orizzontale | Il layout compatto vale anche con `(pointer: coarse) and (max-height: 30rem)` (`{spacing.breakpoint-compact-height}`): sotto quell'altezza il pannello in basso diventa foglio. La direzione del grafo segue sempre la regola della larghezza. |

Sul telefono il campo col fuoco resta sopra la tastiera virtuale e il foglio va a tutta altezza
quando si scrive. Le voci `rail-item` del kit e `nav-item` della dashboard passano in una fila in
fondo allo schermo. I `bottom-sheet` si fermano a metà altezza, che lascia usare il canvas, o a
tutta altezza; si chiudono con «Chiudi», con la maniglia, con `Esc` o, solo se l'ospite lo abilita,
con il gesto indietro del sistema (il kit non scrive nella cronologia dell'ospite). Il
`code-editor` SQL apre a tutto schermo; le righe delle `data-table` diventano card impilate
[ASSUMPTION A-22].

Prove: le board di Foundation, più 844 × 390 e 740 × 360 in orizzontale.

Mock: [editor sul telefono](mockups/key-editor-phone.html), 390 px in chiaro e scuro, canvas a
riposo, palette a mezza altezza, configurazione a tutta altezza, barra icone in fondo; gli altri tre
mock di `mockups/` mostrano il desktop a 1440 px.

## Personalizzazione dell'ospite

- **Cosa si cambia.** Lo sviluppatore della piattaforma ospite adatta colori, caratteri, testi e
  lingua dei componenti del kit (FR-44). Il meccanismo (proprietà, variabili CSS) lo decide
  l'architettura.
- **Valori predefiniti.** Neutri: accento `{colors.accent-kit}` grafite. Nessun nome né logo di
  topictoast, salvo richiesta.
- **Contratto.** Proprietà ed eventi dei componenti React sono contratto pubblico (PRD §6).
- **Regole che restano.** Anche con i colori dell'ospite lo stato ha icona e parola, quindi un
  tema dell'ospite con contrasti bassi non toglie l'informazione. Il kit verifica le coppie dei
  colori dell'ospite (testo e superficie, accento e canvas, accento e superficie, accento e alone,
  `border-control` e superficie, primo piano e riempimento): in sviluppo una coppia sotto soglia è
  un errore visibile, in produzione vince il colore dell'ospite con un avviso in console (T-45;
  meccanismo: T-63).
  Nomi accessibili, parole di stato e messaggi live non sono mai vuoti: un testo dell'ospite
  mancante o vuoto ricade sul predefinito della lingua (WCAG 4.1.2, 1.4.1); nessuna proprietà
  nasconde icone o parole di stato.
- **Tema.** L'ospite sceglie tema chiaro o scuro; senza una scelta il kit segue
  `prefers-color-scheme` [ASSUMPTION A-23].
- **Dashboard.** Non si personalizza: accento `{colors.accent-dashboard}` e wordmark «topictoast».

## Tempo reale

- **Latenza.** Un cambio di stato appare entro 1 s (FR-41) in nodi,
  badge, conteggi del fan-out, barra di stato, tentativi, log, tabelle della dashboard e
  cronologia dei worker.
- **Fan-out.** Si aggiornano i conteggi aggregati, non i singoli rami (FR-41).
- **Isolamento.** Gli eventi in tempo reale del kit portano solo risorse del tenant (FR-26).
- **Pausa.** Quando gli aggiornamenti si fermano, i valori restano fermi e non vengono svuotati;
  alla ripresa si riallineano da soli.
- **Annunci.** Una regione live per area, nessuna su nodi e badge. Lo stato finale
  dell'esecuzione si annuncia subito; i fallimenti si raccolgono in un messaggio ogni pochi
  secondi («3 attività fallite in Per ogni indicatore»); niente annunci al montaggio.
- **Log.** `role="log"` con `aria-live="off"`, area focalizzabile con un nome. Il log segue le
  righe nuove; il pulsante sempre visibile «Pausa aggiornamenti» lo ferma, come lo scorrimento
  verso l'alto, e «Riprendi» torna alle ultime righe (WCAG 2.2.2).

## Inspiration & Anti-patterns

Dai riferimenti di Marco si tengono quattro tratti: base neutra quasi monocroma con il colore solo
come significato; nodi come card arrotondate con bordo sottile su un canvas a griglia di punti;
palette per categoria a sinistra e configurazione del task selezionato a destra; pannelli in
basso con input, output e log del task durante l'esecuzione.

- **Preso da [ref-1](imports/ref-1-pinterest.jpg)**: canvas a griglia di punti, card arrotondate
  con bordo sottile, archi curvi sottili grigi. Non preso: cursori e collaboratori.
- **Preso da [ref-2](imports/ref-2-pinterest.jpg)**: il percorso attivo colorato sugli archi, la
  palette raggruppata per categoria, i pannelli in basso con il log dell'esecuzione. Non preso:
  nodi circolari, pannello Code, palette a destra.
- **Preso da [ref-3](imports/ref-3-pinterest.jpg)** e dalla sua versione grande
  [ref-4](imports/ref-4-dribbble.png): tema chiaro, card bianche, barra icone a sinistra,
  etichetta di categoria sopra ogni nodo. Non preso: campi dentro il nodo, «Generate Workflow».
- **Preso da [ref-4b](imports/ref-4b-dribbble.png)**: il pannello di configurazione del nodo
  selezionato a destra, la barra in alto con l'azione primaria scura, il contorno sul nodo
  selezionato. Non preso: avatar e Share.
- **Riferimento: Workflow Builder** (PRD §1.1, §4.9): editor React che gira solo su Temporal. Si
  supera il nodo di partenza unico (qui più radici, FR-2) e la mancanza di fan-out dinamico (qui
  `fanout-group`, FR-3).
- **Riferimento: Kestra e Conductor** (PRD §1.1): componenti di visualizzazione legati al proprio
  motore. Il kit invece costruisce workflow e ne segue le esecuzioni dentro un'altra app.
- **Respinto: direzioni A, B, D**: [A notte morbida](.working/direction-a-notte-morbida.html),
  [B antracite tecnico](.working/direction-b-antracite-tecnico.html),
  [D console densa](.working/direction-d-console-densa.html). Scelta: C, con gli archi curvi di
  A e B.
- **Respinto: palette accese** ([round 1](.working/color-themes-1.html)): riempimenti vivaci e
  bordi accesi.
- **Respinto: palette spente** ([round 2](.working/color-themes-2.html)): quasi solo grigi, il
  colore non si legge più come significato.
- **Evitato dalla UX: nomi troncati** («dns_look…», «Calcola p…»): regola in `DESIGN.md`, Do's and
  Don'ts.
- **Fuori dalla v1 (PRD §7): catalogo di nodi pronti e connettori** nella palette: sposterebbe il
  prodotto verso n8n.

## Key Flows

### UJ-1. Marco incorpora topictoast nella sua app di threat intelligence.

Marco, sviluppatore ed esperto di sicurezza, ha un server Linux e la sua app con il proprio login.

Prima: installazione e worker, fuori dalla UI (topictoast e il bus con la configurazione
predefinita e SQLite; un worker con l'SDK TypeScript che registra `dns_lookup` e `censys_search`).

1. Mette l'area workflow nella sezione «Workflow» della sua app, collegata all'identità dei suoi
   tenant con la credenziale firmata (FR-27). Nessun secondo login, nessun nome di topictoast.
2. Con la credenziale dell'integratore l'area mostra il `tenant-selector`: sceglie «Tenant di
   prova».
3. Definisce come codice un workflow di esempio e lo pubblica per il tenant di prova: compare
   nell'elenco Workflow del tenant di prova con il `chip` «da codice», disposto in automatico perché non ha posizioni
   salvate. Nell'editor si apre in sola lettura: si modifica dal codice.
4. Apre il workflow e preme «Esegui»: compila l'`input-form` e lancia l'esecuzione per il tenant
   di prova. Si apre la vista delle esecuzioni e i task passano da «In coda» a «In esecuzione».
5. **Momento di valore:** dalla sua app vede le attività arrivare al worker e completarsi. Con il
   ruolo integratore il log scrive «attività ricevuta da worker-ti-01» e i badge diventano
   «Completato» uno dopo l'altro.

Caso limite: il worker non è avviato quando parte l'esecuzione. I task restano «in coda» (le
parole di State Patterns e del PRD, §3): «In coda: nessun worker collegato per
`dns_lookup`», senza errori e senza consumare tentativi. Appena il worker si registra, i task
passano a «In esecuzione» senza che Marco faccia nulla.

### UJ-2. Giulia, utente di un tenant di Marco, compone, lancia e segue un workflow.

Giulia, analista SOC presso un tenant di Marco, è autenticata nell'app di Marco e apre la
sezione workflow.

1. In Workflow sceglie «Nuovo workflow». L'editor si apre con «Canvas vuoto: aggiungere un task
   dalla palette.».
2. Dalla palette, dove ogni voce mostra nome e descrizione dal manifest, trascina sul canvas
   «Ricevi elenco», il fan-out «Per ogni indicatore», dentro il fan-out `dns_lookup` e
   `censys_search` in parallelo, poi «Arricchisci» e il nodo personalizzato «Calcola punteggio»
   (chip `TS`, caricato da un collega con la CLI), infine il join e «Inserisci». «Riordina» li
   dispone da sinistra a destra.
3. Seleziona «Arricchisci»: nella sezione «Dati» ogni campo d'ingresso del manifest ha la sua
   fonte, e lei sceglie «Da un task» `dns_lookup · record` e `censys_search · host` dall'elenco
   dei campi a monte, senza scrivere jq.
4. Seleziona `censys_search`: la sezione «Connessioni» chiede Censys e il selettore propone solo
   i segreti per Censys. Sceglie quello aggiunto prima in Segreti con «Aggiungi segreto»; del
   segreto vede solo il nome. Mock dei passi 2–4: [editor](mockups/key-editor-desktop.html), con
   `censys_search` ancora senza segreto.
5. «Salva»: la validazione gira e il workflow viene salvato come nuova versione.
6. «Esegui»: l'`input-form` mostra il campo elenco «Indicatori», preso dallo schema di input del
   primo task (A-41). Giulia incolla 300 indicatori, uno per riga, e sotto il campo compare «300
   elementi». «Esegui» nel pannello: la vista si apre subito con lo stato «In coda». Mock:
   [modulo di «Esegui»](mockups/key-execution-view.html), schermata 1.
7. **Momento di valore:** vede il DAG prendere vita. Il gruppo «Per ogni indicatore · 300» riempie
   la barra di stato, i conteggi scorrono («completate 287 · in esecuzione 9 · ritentate 3 ·
   fallite 1»), il join si chiude e «Inserisci» si completa. I risultati sono suoi: l'elenco delle
   esecuzioni mostra solo quelle del suo tenant. Mock:
   [vista delle esecuzioni](mockups/key-execution-view.html), schermate 2 e 3 (grafo e `list-view`).

Variante: il giorno dopo lancia lo stesso workflow con un indicatore solo. Il gruppo mostra «Per
ogni indicatore · 1»; il workflow è lo stesso.

Variante «ciclo tra i task»: Giulia collega per errore «Calcola punteggio» ad «Arricchisci».
L'arco compare, i due task portano il segno d'errore e il pannello dice «Ciclo tra i task:
Arricchisci → Calcola punteggio → Arricchisci». «Salva» non salva: accanto al pulsante compare il
riepilogo con il link ai task e il fuoco va lì. Giulia toglie l'arco dalla sezione «Collegamenti»
e salva.

Variante «da telefono, 390 px»: lo stesso editor, con il grafo dall'alto in basso, disposto in
automatico senza le posizioni del desktop. La palette sale come foglio e un tocco su una voce
aggiunge il task dopo il selezionato. Per collegare, Giulia tocca la porta d'uscita e poi il task.
Un tocco su un nodo porta la configurazione nel foglio, che mostra lo `skeleton` mentre carica e va
a tutta altezza quando lei scrive. Mock:
[editor sul telefono](mockups/key-editor-phone.html).

Variante: in Segreti «Elimina» sul segreto `censys-api` apre il `confirm-dialog` con il nome del
segreto e la conseguenza (Voice and Tone); il fuoco parte da «Chiudi».

Caso limite: il nodo personalizzato va in loop infinito. La sandbox lo ferma allo scadere del
timeout e «Calcola punteggio» risulta «Fallito», con il messaggio leggibile nell'`error-card` e
nell'ultimo `attempt-card` («Calcola punteggio: fermato allo scadere del timeout»). Gli altri
tenant non vedono nulla di diverso.

### UJ-3. Giulia crea una pipeline stream che reagisce a un'ondata di indicatori.

Il PRD stesso marca UJ-3 come esempio dedotto dalla domanda sulle finestre, da confermare con un
caso reale di Marco [ASSUMPTION A-25].

Gli indicatori del feed del suo tenant arrivano su uno stream.

1. In Pipeline stream sceglie «Nuova pipeline stream».
2. Nel `code-editor` scrive una query che conta gli indicatori per ASN su finestre di 5 minuti e
   tiene le righe con più di 50 indicatori. Gli errori compaiono prima del salvataggio, con riga
   e colonna.
3. Nel pannello «Parametri» vede gli stream del suo tenant con la loro conservazione e sceglie
   come destinazione «Workflow»: il workflow di arricchimento, avviato per ogni riga.
4. «Salva e attiva»: lo stato diventa «Attiva».
5. **Momento di valore:** durante un'ondata la pipeline stream emette una riga per l'ASN sospetto,
   che compare in «Ultime righe emesse», e il workflow di arricchimento parte da solo, una volta
   per ASN invece che per ogni indicatore; la nuova esecuzione compare in Esecuzioni.

Mock dei passi 2–5: [editor delle pipeline stream](mockups/key-stream-editor.html), con un errore
SQL alla riga 4.

Variante: «Sospendi» porta la pipeline stream a «sospesa»; non emette righe né avvia il workflow
finché Giulia non la riattiva.

Caso limite: topictoast si riavvia durante una finestra. Il kit mostra «Aggiornamenti in pausa,
riprendono da soli.»; stato e ultime righe restano visibili. La pipeline stream riparte
dall'ultimo checkpoint e il conteggio della finestra non si perde.

### UJ-4. Marco, da operatore, controlla l'installazione.

Stato iniziale (PRD): installazione attiva, con tenant che lanciano workflow e pipeline stream
attive. Esito: Marco sa quale worker, esecuzione o tenant richiede un intervento.

1. Apre la dashboard ed entra con la credenziale generata al primo avvio.
2. In Worker vede i worker collegati con le funzioni registrate e le attività in corso («8 in
   corso, la più vecchia da 41 s»).
3. In Esecuzioni ordina per «In coda da»: vede le esecuzioni con il task o l'attività in coda da
   più tempo, anche quelle già in esecuzione.
4. In Pipeline stream vede le pipeline stream attive, rallentate e sospese, con la causa e il
   ritardo sul watermark.
5. In Quote vede i tenant «Vicino al limite».
6. **Momento di valore:** in Worker individua `worker-ti-03`, «Non risponde da 2 min · 4 attività
   riassegnate»: le sue attività sono già state riassegnate a un altro worker. Mock:
   [pagina Worker](.working/direction-c-carta-chiara.html).

Caso limite: un tenant supera la quota di attività concorrenti. In Quote risulta «Al limite» e le
sue nuove attività restano «In coda»; le righe degli altri tenant non cambiano e le loro attività
non rallentano.

### UJ-5. Luca, che non conosce topictoast, lo prova.

Il PRD lo descrive con il momento di valore in Esecuzioni della dashboard e il caso limite della
credenziale del primo avvio.

1. Luca, che sviluppa una piattaforma di automazione, trova il repository e segue la guida rapida.
2. Avvia topictoast con la configurazione predefinita: il workflow di esempio gira senza modifiche
   (FR-50).
3. Avvia il worker di esempio e il workflow di esempio.
4. **Esito:** in meno di 15 minuti ha topictoast in esecuzione con un worker e un workflow di
   esempio completato (NFR-13).

Luca vede l'esito in Dashboard → Esecuzioni con «Completata», dopo l'accesso con la credenziale
del primo avvio. Se non ha la credenziale sottomano, la `login-card` dice dove è stampata; senza
worker o esecuzioni, i vuoti offrono «Apri la guida rapida».

## Elementi aperti

Quattro blocchi: assunzioni, rinvii, scelte dei mock non promosse, ampliamenti di perimetro. Ogni
assunzione ha un tag ASSUMPTION con il suo id nel punto delle spine a cui si riferisce. Gli id
sono stabili: A-1…A-26 erano le assunzioni di questo file e A-27…A-32 quelle di `DESIGN.md`, nei
numeri che `triage.md` cita; A-33…A-39 vengono dagli autofix T-1…T-38,
A-40…A-44 dall'applicazione delle decisioni su T-39…T-54. Gli id che mancano sono chiusi, dagli
autofix o da quelle decisioni (memlog). L'aggiornamento del PRD del 2026-11-20 ha chiuso A-2
(accordo delle parole di stato), A-8 (elenco degli stati), A-10 (FR-14), A-19 (NFR-9), A-26
(UJ-5), A-42 (FR-55) e A-43
(FR-32), e di A-41 la parte su chi definisce lo schema di input. Non restano decisioni aperte per
Marco.

**Assunzioni**

- A-3 — Formulazione della UX per le frasi con fonte FR e per «Nuovo workflow», «Aggiungi segreto» (Voice and Tone).
- A-4 — Testo del banner di ripresa dal mock C, da rivedere; banner su ogni pagina della dashboard, non chiudibile a mano (`recovery-banner`).
- A-5 — Fan-out: scheda del primo elemento fallito con «altri N»; elementi in una `data-table` nel pannello (`error-card`).
- A-7 — Inserimento dalla palette: su desktop dove il task è lasciato, con clic o `Invio` accanto al selezionato, su un arco tra i due task; sul telefono dopo il selezionato (`palette-item`).
- A-9 — Famiglie di colore di ricevuta, collegato e non risponde dal mock C; della pipeline stream e delle quote proposte (State Patterns).
- A-11 — Testi degli stati vuoti e di «non trovato» (State Patterns).
- A-12 — Collegamento degli aggiornamenti interrotto: stesso `pause-notice` nel kit (State Patterns).
- A-17 — Tasti del canvas attivi solo con il fuoco nel canvas (Interaction Primitives).
- A-21 — Soglia di larghezza a 48 rem; pannello destro sopra il canvas tra 768 e 1279 px (Responsive & Platform).
- A-22 — Telefono: navigazione in fondo, fogli a metà o a tutta altezza, editor SQL a tutto schermo, righe di tabella come card (Responsive & Platform).
- A-23 — Tema scelto dall'ospite, altrimenti `prefers-color-scheme` (Personalizzazione dell'ospite).
- A-25 — UJ-3 è un esempio dedotto nel PRD (Key Flows).
- A-27 — Aloni di selezione ricalcolati dall'accento grafite e dal viola (20 % e 22 %, sul canvas); il round 3 li aveva calcolati sull'indaco poi sostituito e nessuno li ha visti (DESIGN Colors).
- A-29 — Navigazione della dashboard solo in alto; il mock C la ripeteva nella barra icone (DESIGN Layout & Spacing).
- A-30 — Larghezza massima della card del nodo provvisoria, da fissare in Penpot (DESIGN Layout & Spacing).
- A-32 — Aspetto solo dai token per pause-notice, tooltip, confirm-dialog, dashboard-bar, login-card, load-error, empty-state, skeleton, schedule-section, tenant-selector; nav-item, history-list e recovery-banner visti solo nel mock C e ricolorati; gli altri nei mock di `mockups/` (DESIGN Components).
- A-33 — Proposta della UX: nessuna ombra su nodi e pannelli; i riferimenti ne hanno una leggera (DESIGN Elevation & Depth).
- A-34 — Proposta della UX: etichetta sempre visibile sulla barra icone, per accessibilità e telefono (DESIGN `rail-item`).
- A-35 — Proposta della UX: voci di palette a riquadro con bordo (DESIGN `palette-item`).
- A-36 — Larghezza della palette provvisoria, da fissare in Penpot (DESIGN Layout & Spacing).
- A-40 — Condizione dei rami di `switch` con la scelta guidata della sezione «Dati», jq come modo avanzato (Interaction Primitives).
- A-41 — In un workflow nuovo lo schema di input parte dallo schema di input del primo task; che il tenant lo definisca in «Input del workflow» è confermato dal PRD (Interaction Primitives).
- A-44 — `Alt+frecce` sposta un nodo da tastiera, alternativa al trascinamento manuale (Interaction Primitives).

**Rinvii**

Alle spec delle storie e all'architettura: T-55…T-70 di `triage.md`; tra questi la conferma
di «Annulla esecuzione» (T-62), il canale in tempo reale interrotto nella dashboard (T-68) e i nomi
dei token in `docs/glossary.md` (T-70). L'architettura decide anche dove si salvano le posizioni
manuali del desktop (T-46) e la propagazione di fallito e saltato (domanda aperta 8 del PRD).

**Scelte dei mock non promosse** (in `mockups/`; restano esempi o rinvii, non regole)

- Estensione della `workflow-bar`: sopra canvas e pannello destro nei mock del grafo; sopra il solo
  centro in quello delle pipeline stream, con «Parametri» a tutta altezza (T-62).
- Schermata 1 di `key-execution-view.html`: editor senza palette, nemmeno compressa, e senza
  «Elenco»; valgono le spine, come in `key-editor-desktop.html`.
- Controlli di zoom e «Riordina»: lato, fila o colonna e posto di «Riordina» cambiano da un mock
  all'altro (T-55).
- Titoli: sezione di retry e timeout («Ripetizione» su desktop, «Tentativi e timeout» sul telefono);
  palette («Palette», «Aggiungi task» nel foglio) e sua ricerca; pannello di «Esegui» («Input del
  workflow», lo stesso nome della sezione dello schema) (T-62).
- Esempi: nodi «Valuta» e «Interrompi»; gruppi «DNS», «Arricchimento», «Punteggio» (dal
  manifest); valori di retry e timeout (dall'installazione, NFR-14); icone della barra icone,
  diverse in ogni mock (Penpot).
- Pipeline stream: elenco a sinistra dell'editor, selettore del workflow di destinazione, colonne di
  «Ultime righe emesse» e `chip` di versione (clarify della storia); sintassi della finestra
  (`TUMBLE`, T-56); «Salva e attiva» disabilitato con errori (T-61).

**Ampliamenti di perimetro rispetto al PRD** (tutti accettati nell'aggiornamento del PRD del
2026-11-20)

1. Editor completo sul telefono: grafo ed editor SQL, non solo consultazione. Accettato nel PRD:
   NFR-16.
2. Componente «area workflow» pronto con navigazione interna, oltre ai pezzi singoli. Accettato
   nel PRD: FR-58.
3. Avviso neutro di pausa per il tenant nel kit durante la ripresa (NFR-15). Accettato nel PRD:
   FR-41, FR-43.
4. Pianificazioni nel kit: `schedule-section` nel pannello del workflow (T-42). Accettato
   nel PRD: FR-57.
5. Ruolo integratore nel kit: `tenant-selector` (T-43; T-79). La modifica nell'editor dei workflow
   definiti come codice è caduta: sono in sola lettura per tutti (FR-35). Accettato nel PRD: FR-27,
   FR-28.
6. Schema di input dichiarato dal workflow e `input-form` di «Esegui» (T-47). Accettato nel PRD: FR-56.
7. Manifest dei worker e dei nodi personalizzati caricati con la CLI: per ogni funzione
   connessioni (piattaforma e credenziale richiesta), schema di input, schema di output e altri
   metadati. Accettato nel PRD: FR-55, con la connessione dichiarata dal segreto (FR-32) e la
   validazione di tipi e campi obbligatori, nell'editor e nel motore (FR-40, FR-4).
