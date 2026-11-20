# Spine Pair Review — topictoast

## Overall verdict

La coppia è un contratto utilizzabile: frontmatter di DESIGN.md valido (YAML parsato: 104 colori tutti `#rrggbb`, 13 tipografie, 7 raggi, 17 spaziature, 31 componenti), nessun riferimento `{token}` non risolto in nessuno dei due file, 31 componenti con riga sia in DESIGN.md.Components sia in EXPERIENCE.md.Component Patterns, cinque UJ con nome identico al PRD e un Key Flow ciascuno, ref-1…ref-4b tutti collegati e risolvibili. Nessun finding critico. I punti da chiudere prima del codice sono tre: i bordi dei controlli a 1,25:1 non dichiarati tra le deroghe sotto 3:1 (WCAG 1.4.11), l'assenza di una resa visiva dell'errore sul nodo nell'editor (FR-40), e una trentina di misure in pixel che stanno solo in prosa e un consumatore vanilla-extract non può leggere dal frontmatter. DESIGN.md non è gonfio nelle decisioni ma nella forma: ripete in tre posti (frontmatter, prosa Components, tabelle tipografia/contrasti) le stesse associazioni token-valore, e porta storia di round e difetti del mock C che non servono a chi implementa; si può scendere da 807 a circa 650 righe e da 104 a circa 88 colori senza perdere una decisione.

Conteggio: 0 critical, 2 high, 16 medium, 33 low.

## 1. Flow coverage — adequate

Fonti: UJ-1…UJ-5 dal PRD §2.3. Tutti e cinque i nomi sono verbatim (EXPERIENCE.md, Key Flows). Ognuno ha protagonista (Marco, Giulia, Giulia, Marco, Luca), passi numerati, momento di valore (UJ-5: «Esito», perché il PRD non ne dà uno) e caso limite. Le assunzioni sono dichiarate (UJ-3 dedotto dal PRD, UJ-4 e UJ-5 senza contesto nel PRD).

### Findings
- **medium** Nessun Key Flow percorre il fallimento della validazione: ciclo (FR-2), riferimento inesistente (FR-4), conflitto di versione (FR-5), «Salva» con errori. In UJ-2 passo 4 compare solo «Un errore comparirebbe sul task…», al condizionale dentro il percorso felice; il caso limite di UJ-2 è il loop del nodo, non l'errore di editing, che è invece il comportamento più costoso da indovinare per chi implementa FR-40 (EXPERIENCE.md, Key Flows UJ-2). *Fix:* aggiungere a UJ-2 una variante «ciclo tra i task» con i passi: arco che chiude il ciclo, segno sul task, messaggio nel pannello, «Salva» che porta il fuoco al primo errore.
- **medium** Il telefono è scope ampliato («funziona tutto», editor del grafo e SQL compresi) ma nessun flusso lo percorre: collegare toccando porta e destinazione, palette come foglio, SQL a tutto schermo vivono solo in assunzioni di Interaction Primitives e Responsive (EXPERIENCE.md, Key Flows). *Fix:* una variante breve di UJ-2 o UJ-3 «da telefono, 390 px» con i passi sul foglio e sul collegamento a tocco.
- **medium** FR-54 (avviso all'operatore se la sandbox forte non si attiva) è un requisito senza superficie; l'unica traccia è in IA «Copertura» e in Domande senza risposta 1 (EXPERIENCE.md, Information Architecture). Noto, ma blocca la storia. *Fix:* decidere ora la superficie (per esempio `recovery-banner` di famiglia «avviso» in Worker) o marcare FR-54 fuori dalla UX della v1 con motivo.
- **low** Azioni del PRD non raggiunte da nessun flusso: «Annulla» un'esecuzione (FR-10), «Elimina» un segreto (FR-42), sospendere una pipeline stream (FR-43); la conferma prima di annullare/eliminare è domanda aperta 2.
- **low** UJ-1 passi 1–2 (installazione, SDK) sono fuori dalla UI; i passi di UI sono 3–6.
- **low** UJ-5 non ha momento di valore né caso limite (il PRD è un paragrafo); la spina inventa solo il passo di accesso alla dashboard, già marcato come assunzione. Va bene, ma UJ-4 e UJ-5 restano senza persona dal PRD: da riportare nell'aggiornamento del PRD.
- **low** Nessun flusso per «Accesso» rifiutato né per la credenziale scaduta del kit (FR-27), coperti solo come stato.

## 2. Token completeness — adequate

Estratti tutti i token del frontmatter e tutti i `{path}` di entrambi i file. Colori: 104 con hex (52 coppie chiaro/scuro: ogni nome non `-dark` ha il suo `-dark`, verificato). Riferimenti: 0 non risolti. Le 6 deroghe sotto 3:1 (arco inattivo, segmenti, coppia 3,28:1 del banner) sono dichiarate. Tipografia: valori semantici o rem completi. Token mai referenziati da un componente: solo `typography.heading-lg` e `typography.wordmark`.

### Findings
- **high** I bordi dei controlli sono a 1,25:1 su superficie (`{colors.border}` #e6e6e8 su #ffffff) e 1,27:1 in scuro (#2f2f32 su #1d1c1e), e `form-field`, `code-editor`, `palette-item` e `button-secondary` li usano come unico contorno. WCAG 1.4.11 (NFR-9 dichiara AA) chiede 3:1 per il confine di un campo. La sezione «Sotto 3:1 per scelta» non lo cita (DESIGN.md, Colors › Contrasti; components `form-field`, `code-editor`). *Fix:* un token `border-control` ≥ 3:1 per campi e editor (i pannelli e le card restano a `border`), oppure una deroga scritta per campo con la ragione (etichetta sempre visibile, altezza 36 px).
- **medium** Circa 25 misure vivono solo in prosa, senza token, quindi un `.css.ts` non le legge dal frontmatter: righe di `log-line` 96/64/22 px, altezza `nav-item` 32 px, voce `rail-item` 44 px e icona 20 px, spazio 2 px e minimo 6 px in `status-bar`, maniglia `bottom-sheet` 32 × 4 px, etichetta di categoria 19 px sopra la card, anello di focus 1,5 px + distanza 2 px, bordo di selezione 1,5 px, alone 4 px, indicatore 3 px (`data-table`) e 2 px (`tabs`, `nav-item`), icone 20/14/12 px con tratto 1,6 (DESIGN.md, Components, Elevation & Depth, Shapes). È l'inverso del bloat: pixel in prosa dove nessun token li copre. *Fix:* promuovere a `spacing`/chiavi di componente (`focus-ring-width`, `focus-ring-offset`, `indicator-width`, `selection-width`, `halo-width`, `icon-size-*`, `log-time-column`…) e togliere il numero dalla prosa.
- **medium** Mancano token che il layout richiede: larghezza del pannello palette a sinistra (esiste solo `side-panel-width` per il destro; a 1280 il canvas dipende da rail 64 + palette + 360 + canvas), larghezza massima del nodo (assunzione 4 degli elementi aperti, ma serve un token con il valore «da fissare in Penpot»), soglia 48 rem (solo in EXPERIENCE.md, Responsive). (DESIGN.md, Layout & Spacing). *Fix:* aggiungerli con valore provvisorio e `[ASSUMPTION]`.
- **medium** Nessun token per i colori di stato delle interazioni (hover, premuto, disabilitato, testo segnaposto): «Salva e attiva non parte finché ci sono errori» e «Salva» non si disattiva richiedono almeno un disabilitato (vedi 4).
- **low** `typography.heading-lg` (titolo pagina dashboard) e `typography.wordmark` non sono referenziati da nessun componente: dashboard senza componente barra superiore né testata di pagina (vedi 3).
- **low** `colors.selection-halo*` sono marcati come mai visti da nessuno (assunzione 1): corretto, ma il contrasto dell'alone sul canvas (#d4d3d6 su #f6f6f6, circa 1,3:1) non è dichiarato; la selezione è comunque portata dal bordo 8,95:1.
- **low** Nel contrasto «accento dashboard / voce nav attiva» (3,32 chiaro) manca la coppia con `{colors.text-secondary}` sul fondo `nav-active-dashboard` per le voci inattive in hover (nessuno stato hover definito).
- **low** I valori † (calcolati in distillazione) dipendono da `.working/color-themes-3.html` come fonte: a promozione fatta il riscontro sparisce (vedi 5).

## 3. Component coverage — adequate

Estratti i 31 nomi dal frontmatter, da DESIGN.md.Components e da EXPERIENCE.md.Component Patterns: gli elenchi coincidono, con regole reali (non una parola) su entrambi i lati.

### Findings
- **high** `node-card` non ha resa dell'errore: FR-40 chiede «l'errore sul task che lo causa prima del salvataggio», EXPERIENCE.md dice «porta un segno d'errore» (Component Patterns, node-card), ma DESIGN.md.Components non ha bordo, icona o badge d'errore per il nodo (esiste `border-error` solo in `form-field`). Chi implementa deve inventare l'aspetto della funzione centrale dell'editor. *Fix:* chiavi `border-error` e `icon-error` su `node-card` e una riga nella prosa (icona `{colors.state-failed}`, bordo `{colors.state-failed-border}`; mai solo colore).
- **medium** Componenti usati ma senza riga in nessuno dei due file: selettore/combobox (segreti nell'editor, filtri «Cliente», «Workflow», «Stato» in `data-table` dichiarati come `form-field`), pulsante icona con la X di chiusura del pannello, azione distruttiva («Elimina» segreto, «Annulla» esecuzione) senza stile, tooltip (citato in Accessibility Floor: «I tooltip aggiungono al nome»), barra superiore e wordmark della dashboard (hanno token `dashboard-bar-height` e `typography.wordmark` ma nessun componente), scheda di Accesso, errore di caricamento con «Riprova» (EXPERIENCE.md, State Patterns › Stati per superficie). *Fix:* righe brevi che riusano i token esistenti (select = `form-field` con indicatore; icon-button = `button-secondary` quadrato di `control-height`).
- **medium** Componenti con comportamento ma senza aspetto in DESIGN.md per gli stati hover/premuto/disabilitato/riga selezionata: valgono per `button-primary`, `button-secondary`, `rail-item`, `nav-item`, `palette-item`, `tabs`, righe di `data-table`. Solo il focus è definito.
- **low** `chip` ha tre usi (linguaggio, versione, funzioni dei worker) con la stessa riga: va bene, ma il chip con icona di categoria davanti non ha chiave (`icon-*`) in frontmatter.
- **low** `icon-tile` nel frontmatter ripete 12 chiavi dove `category-tag` ne ha 5: le categorie sono la stessa matrice due volte (vedi 6).
- **low** `recovery-banner` e `pause-notice` hanno ruoli distinti ma colori da famiglie di stato (ritentata, in attesa): scelta motivata, ma il nome della famiglia nel banner («avviso») non esiste nei token; assunzione già marcata.

## 4. State coverage — adequate

Percorse tutte le superfici dell'IA (11 righe di «Stati per superficie»): vuoto, caricamento (`skeleton`), errore di caricamento, validazione, quota, ripresa del motore, tempo reale interrotto, credenziale scaduta, nessun dato senza credenziale. L'elenco chiuso degli stati per oggetto è completo e coerente con i token.

### Findings
- **medium** Stati di interazione senza definizione: hover, premuto, disabilitato, focus già c'è (vedi 2 e 3). Impatto: ogni sviluppatore sceglie i suoi.
- **medium** Stati delle azioni asincrone dell'editor mancanti: «Salva» in corso, errore di rete al salvataggio, «Esegui» in corso o rifiutato (quota FR-30 compare in Stati trasversali ma non dice dove si vede), JSON d'input non valido nel `code-editor`, stato «modifiche non salvate» all'uscita (EXPERIENCE.md, State Patterns).
- **medium** La dashboard non ha stato di sessione scaduta o credenziale revocata dopo l'accesso (solo il kit ha «Sessione scaduta.»), né limite di tentativi su Accesso; FR-45 chiede «nessun dato senza credenziali» (State Patterns, Accesso).
- **low** Offline / canale in tempo reale interrotto nella dashboard: aperto e dichiarato (Domande senza risposta 3).
- **low** Telefono: stati dei fogli (caricamento, tastiera che copre il foglio), limite di dimensione del grafo e comportamento del pizzico fuori scala non descritti.
- **low** Vista delle esecuzioni: aspetto di un'esecuzione annullata o scaduta sul canvas (i task non partiti diventano «annullato» in `state-skipped`?) è dichiarato per testo ma non per resa.
- **low** Esiste un solo testo per «Workflow non trovato.»; non c'è per Segreti, Pipeline stream, Esecuzioni (FR-26 vale per tutte).

## 5. Visual reference coverage — adequate

`imports/`: ref-1-pinterest.jpg, ref-2-pinterest.jpg, ref-3-pinterest.jpg, ref-4-dribbble.png, ref-4b-dribbble.png: tutti collegati in EXPERIENCE.md › Inspiration & Anti-patterns con ciò che se ne è preso; nessun orfano; i link risolvono. Mockups e wireframes non ancora promossi: non contati. «Le spine vincono» è dichiarato in testa a entrambi i file.

### Findings
- **low** Le immagini di `imports/` sono collegate in una sola sezione e mai dove si decide: DESIGN.md non ne linka nessuna (Brand & Style e Components citano solo `.working/`), mentre la descrizione dei nodi e della palette viene proprio da ref-1/ref-3/ref-4b. *Fix:* un link inline in Brand & Style e in `panel`/`palette-item`.
- **low** Tutti i riferimenti agli schermi puntano a `.working/*.html` (IA: colonna «Riferimento visivo»; DESIGN.md Brand & Style, Colors); al momento della promozione vanno riscritti verso `mockups/` o tolti. Anche la provenienza degli hex e dei contrasti («commento `<style>` del round 3») dipende da un file di lavoro.
- **low** Otto superfici su dieci riportano «solo spina» come riferimento: è onesto, ma il Layout & Spacing del kit non dice che l'editor (con palette a sinistra) non ha alcun mock; l'unica prova del layout a tre pannelli più rail è un'assunzione.
- **low** I due mock esistenti (vista esecuzioni kit, pagina Worker) non sono collegati ai Key Flow che li mostrano (UJ-2 passo 6, UJ-4 passo 5).

## 6. Bloat & overspecification — adequate (eccesso concentrato in DESIGN.md)

Misure: DESIGN.md 807 righe = frontmatter 460 (57 %: colori 114, tipografia 68, rounded+spacing 26, components 236) + prosa 342 (Colors 91 di cui Contrasti 42, Typography 33, Layout 27, Shapes 19, Components 123, Do's and Don'ts 15). EXPERIENCE.md 533 righe, di cui «Elementi aperti» 70 e Voice and Tone 63.

Cosa non serve a un consumatore. Dashboard React con vanilla-extract che legge il frontmatter: serve `colors`, `typography`, `rounded`, `spacing`, e le chiavi `components` solo come mappa di varianti; non gli serve nulla della prosa, né la tabella dei contrasti, né gli alias che duplicano un altro token. Sviluppatore di storia: serve l'anatomia e il comportamento di ogni componente, le regole «mai» (testo neutro, nome mai troncato, stato con icona e parola) e i casi limite; non gli serve la cronologia dei round, i difetti del mock C né il calcolo dei contrasti.

### Findings
- **medium** 16 dei 104 colori sono alias o quasi-duplicati (si possono togliere senza perdere una decisione): (a) 8 token `state-skipped*` hanno gli stessi esadecimali dei `state-waiting*` (DESIGN.md dice «oggi gli stessi valori ma restano distinti» per il significato; il significato vive già nei componenti `state-badge` `fill-waiting`/`fill-skipped`, che possono puntare allo stesso `state-neutral*`); (b) 6 `state-*-segment-dark` sono identici al primo piano scuro dello stesso stato (`#7df2bc`, `#86b7f3`, `#f2d77d`, `#f4928b`, `#8b8994` ×2); (c) `surface-waiting` (#fbfbfb) e `surface-group` (#fafafa) hanno contrasto reciproco 1,01:1 e in scuro lo stesso `#19181b`: un solo `surface-subtle` e i due componenti lo citano. Resto: 88 colori, di cui 44 coppie vere. Opzionale: i 4 `selection-halo*` sono accento al 20/22 % già mescolato sul canvas, calcolabili dal tema.
- **medium** `components` nel frontmatter (236 righe, 51 % del frontmatter) è una matrice che segue una regola di nome: `state-badge` 20 chiavi (6 stati × fill/border/icon), `icon-tile` 12, `category-tag` 5, `status-bar` 8, `button-*` e `form-field` ripetono `radius`/`height`. Circa 45 righe si riducono a una regola («per ogni stato s: `fill: colors.state-s-fill`, `border: …-border`, `icon: colors.state-s`») con un solo esempio; il consumatore genera la mappa. *Fix:* mantenere le chiavi dove il valore è una scelta (padding, tipografia, raggio), togliere quelle derivabili dal nome.
- **medium** La prosa di Components (123 righe) ripete le stesse associazioni del frontmatter: per `node-card`, `state-badge`, `status-bar`, `button-primary`, `button-secondary`, `tabs`, `attempt-card`, `code-block`, `data-table`, `form-field`, `skeleton` ogni riga rielenca fondo, bordo e raggio già in `components`. Due fonti per lo stesso valore divergono alla prima modifica. *Fix:* una sola sede per i valori (frontmatter); la prosa tiene anatomia, contenuto di esempio («Per ogni indicatore · 300») e le eccezioni; si risparmiano circa 50–60 righe.
- **medium** La sezione Contrasti (42 righe, 24 righe di tabella) ha 6 righe quasi uguali «testo secondario / X» (7,20, 6,66, 6,95, 6,89, 5,77, 5,95), valori calcolati a mano dal commento di un file `.working/` e tre marcati †. È una verifica, non una decisione, e va tenuta aggiornata a mano. *Fix:* in DESIGN.md restano la regola, le soglie, le 6 deroghe e una riga minima per classe; la matrice completa va in un test generato dai token (gate `npm run gate` o Storybook a11y), risparmio circa 25 righe.
- **medium** Le assunzioni sono scritte due volte: 32 blocchi `[ASSUMPTION]` inline (6 in DESIGN.md, 26 in EXPERIENCE.md) più le 70 righe di «Elementi aperti» che le rielencano una a una, con il blocco «Componenti senza mock» di DESIGN.md che ripete di nuovo l'elemento 6 della lista. Risolverne una vuol dire modificare due o tre posti. *Fix:* ID breve inline (`[A-12]`) e un solo elenco, oppure solo inline e l'elenco generato.
- **low** Il testo ripete la storia dei round: Brand & Style («Ci si è arrivati per esclusione: round 1… round 2…»), Do's and Don'ts (due righe con «round 1/2 respinto»), EXPERIENCE.md › Inspiration (stesso rifiuto), memlog (stesso rifiuto): quattro volte. *Fix:* il rifiuto resta in Inspiration (richiesto) e in una riga di Do's and Don'ts senza numero del round; tolto da Brand & Style. Circa 8 righe.
- **low** Nove citazioni di «mock C» in DESIGN.md descrivono difetti e storia del mock, non regole («il mock C colorava la parola», «il mock C ripete le quattro voci», «esempio nel mock C: Aggiorna», «il mock C troncava»). Le spine vincono sui mock, quindi diventano stantie alla promozione. *Fix:* tenere la regola (parola neutra, solo navigazione in alto, nome intero), togliere il rimando.
- **low** Typography: la tabella di prosa (13 righe) ripete dimensione e peso già nel frontmatter (stesso numero in due posti, derivato a mano in rem); basta «Token | Uso». E `fontFamily` è copiato 13 volte (10 sans, 3 mono): un consumatore vanilla-extract vuole `fonts.sans` e `fonts.mono`; usare ancore YAML o dichiararli una volta.
- **low** Shapes (19 righe) elenca quale raggio va a quale componente, cioè i campi `radius:` già presenti in `components`. Restano solo icone, porte e curve (12 righe di contenuto); risparmio circa 10 righe.
- **low** Ripetizioni di regole: «il nome del nodo non si tronca mai» compare 5 volte (Typography, Layout, Do's, `node-card` in EXPERIENCE, Inspiration); «testo sempre neutro» 3 volte (Colors, Typography/Do's, `log-line`); «cambio di stato entro 1 s» 4 volte (Stati trasversali, Tempo reale, `state-badge`, `fanout-group`). La sezione EXPERIENCE «Tempo reale» duplica Stati trasversali e le righe dei componenti.
- **low** Prosa di Colors: i paragrafi «Base neutra» e «Superfici derivate» rielencano i ruoli già nei commenti del frontmatter; si possono ridurre a una riga ciascuno. Restano le tre regole, accenti, categorie, stati (decisioni).
- **low** EXPERIENCE.md Voice and Tone: la tabella a 22 righe è un dizionario di testi (sorgente dei dizionari i18n it/en del dashboard). Non è bloat, ma potrebbe vivere in un file a parte (`copy.md`) e lasciare qui principi e cinque esempi (circa 30 righe in meno nella spina).
- **low** EXPERIENCE.md Key Flows: UJ-1 passi 1–2 (CLI, SDK) sono fuori dalla UI.

Stima: DESIGN.md da 807 a circa 650 righe e da 104 a 88 colori, senza togliere nessuna decisione; circa 100 righe di frontmatter + 60 di prosa spostate o fuse.

## 7. Inheritance discipline — strong

`sources` risolvono tutti (prd.md, addendum.md, brief.md, `.memlog.md`, `.working/*`, `../../../../.claude/skills/dashboard-react/SKILL.md`). Nomi UJ identici al PRD (UJ-2 omette solo la parentesi «Lo scenario…»). Glossario del PRD rispettato («Pipeline stream» sempre per intero, workflow, task, esecuzione, attività, worker, funzione, nodo personalizzato, segreto, quota). Nomi dei componenti identici in tutte le sezioni dei due file; i `{token}` di EXPERIENCE.md risolvono tutti.

### Findings
- **low** Il token `state-skipped` porta saltata, annullata, scaduta e pipeline stream «sospesa»: il nome non dice il significato e collide con il glossario («sospesa» in EXPERIENCE.md vs «in pausa» nel kit, due concetti diversi con parole vicine). *Fix:* con la fusione di cui al punto 6, nominare il token `state-neutral` e tenere distinte le parole mostrate.
- **low** I nomi di token di categoria (`worker-function`, `custom-node`, `flow-control`) non sono nel glossario: rimandato giustamente a quando arrivano nel codice (Domande senza risposta 4); il glossario PRD non ha «categoria del nodo».
- **low** «In attesa» (PRD, UJ-1 caso limite) vs «in coda» (spina): divergenza dichiarata in UJ-1 e nell'IA. Va portata nell'aggiornamento del PRD, che oggi dice altro.

## 8. Shape fit — strong

DESIGN.md: Brand & Style → Colors → Typography → Layout & Spacing → Elevation & Depth → Shapes → Components → Do's and Don'ts, ordine canonico rispettato, nessuna sezione omessa. EXPERIENCE.md: Foundation, Information Architecture, Voice and Tone, Component Patterns, State Patterns, Interaction Primitives, Accessibility Floor, Key Flows presenti; Responsive & Platform e Inspiration & Anti-patterns presenti dove servono (più superfici, riferimenti e rifiuti). Sezioni inventate: Personalizzazione dell'ospite (FR-44), Tempo reale, Elementi aperti.

### Findings
- **low** «Tempo reale» come sezione propria duplica Stati trasversali e i componenti (vedi 6); se resta, deve essere l'unica sede e le altre righe rimandarvi.
- **low** «Elementi aperti» è utile ma contiene anche gli ampliamenti di perimetro e domande: sono tre elenchi con tre scopi in una sezione; separare «decisioni per Marco» da «assunzioni» faciliterebbe il triage.
- **low** DESIGN.md Elevation & Depth contiene regole di selezione e focus che appartengono a Components (`node-card`, `button-primary`); non sbagliato, ma ripete.

## Mechanical notes

- Frontmatter DESIGN.md: YAML valido; `name`, `description`, `colors`, `typography`, `rounded`, `spacing`, `components` presenti. Chiavi `rounded.'2xl'` quotata correttamente. Nessun hex malformato.
- Riferimenti `{path}`: 0 non risolti (colori, tipografia, rounded, spacing) in DESIGN.md ed EXPERIENCE.md. Colori mai citati per nome: solo le 52 varianti `-dark`, per convenzione dichiarata («I componenti citano solo il nome chiaro»); nessun colore chiaro orfano.
- Coppie chiaro/scuro: 52 su 52 complete.
- Link di `imports/`: 5 su 5 risolvono; nessun file orfano. Link a `.working/`: tutti risolvono (da riscrivere alla promozione).
- `sources` in frontmatter: tutti i percorsi esistono; EXPERIENCE.md cita DESIGN.md come fonte (corretto).
- Nessun blocco Mermaid in nessuno dei due file.
- Titoli di sezione in inglese (canonici) con corpo in italiano: coerente tra i due file e con la costituzione VIII.
- Incoerenze di nome: nessuna tra componenti; due di glossario nel punto 7.
- Controlli fatti con script locale di sola lettura (YAML, contrasti con la formula WCAG); i contrasti dichiarati nelle tabelle non sono stati ricalcolati tranne bordo/superficie, bordo/canvas, surface-waiting/surface-group, arco inattivo (1,69 confermato).
