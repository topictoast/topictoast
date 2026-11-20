# Riconciliazione degli import visivi con le spine

Input: `imports/ref-1-pinterest.jpg`, `ref-2-pinterest.jpg`, `ref-3-pinterest.jpg`, `ref-4-dribbble.png`, `ref-4b-dribbble.png`; voci del `.memlog.md` su riferimenti e scelte di Marco.
Confronto con: `DESIGN.md`, `EXPERIENCE.md` (nessun file esistente modificato).
Data: 2026-11-19.

Cosa mostra ciascun riferimento (visto direttamente):

- **ref-1**: editor scuro quasi nero, canvas a punti, card arrotondate con bordo sottile, porte come punti colorati per tipo (giallo, verde, rosso) allineati alle righe dei campi, archi curvi sottili grigi, cursori di collaboratori con nome, barra strumenti flottante in basso, glow sul nodo attivo.
- **ref-2**: editor antracite, nodi circolari con etichetta a destra, archi curvi con percorso attivo azzurro, palette a destra (ricerca, gruppi Inputs/Transform/Database, voci a riga con icona), menu contestuale con scorciatoie, due pannelli in basso affiancati (Code, Debug running con log colorati), chip «Condition» sull'arco, barra con Play, impostazioni e pulsante primario.
- **ref-3 / ref-4** (stessa immagine, ref-4 più grande): editor chiaro, barra icone a sinistra, palette a sinistra con titolo, pulsante comprimi, ricerca, gruppi Input/Logic/Output/Utilities (liste con «+» oppure coppie di bottoni), card bianche con etichetta di categoria colorata sopra, tessera icona, chip di stato accanto al nome, riga di descrizione, campi dentro il nodo, archi ortogonali con porte sul centro del bordo, chip `if` con condizioni.
- **ref-4b**: stesso editor con pannello di configurazione a destra (titolo, sottotitolo, X, sezioni con filetti, select), barra in alto con avatar, Play verde (icona), Share, Deploy (primario scuro), nodo selezionato con bordo magenta, ramo `else` rosso.

## Riepilogo

| Gravità | Voci |
|---|---|
| alta | 2 |
| media | 10 |
| bassa | 7 |
| Totale | 19 |

---

## (a) Tratti tenuti mancanti o indeboliti nelle spine

### A1. Palette a sinistra assente dal layout e dai token — **alta**
- Riferimento: ref-3 / ref-4 (palette a sinistra con titolo, ricerca, pulsante comprimi, gruppi per categoria).
- Cosa dicono le spine: tratto 3 del memlog («palette a sinistra, configurazione a destra»). In `DESIGN.md`, Layout & Spacing, «Area workflow del kit, desktop» elenca barra icone, canvas, pannello in basso e pannello destro: **la palette non c'è**. Il pannello sinistro compare solo nella voce `panel` di Components. Nel frontmatter esistono `rail-width`, `side-panel-width`, `log-panel-height` ma nessuna larghezza della palette. Non è detto se la palette sia sempre aperta, comprimibile o aperta solo nell'editor, né come convive con la barra icone (nei riferimenti la palette sta accanto alla barra, non al suo posto).
- Correzione proposta: aggiungere la palette al paragrafo del layout desktop, aggiungere il token `palette-width` (da fissare in Penpot) e una regola in EXPERIENCE (visibile nell'editor, assente nella vista delle esecuzioni, comprimibile sì/no). Marco decide il comprimibile.

### A2. Colore come significato anche sulle porte — **media**
- Riferimento: ref-1 (porte colorate per tipo); memlog tratto 1: «il colore solo come significato (stati, porte, percorso attivo, categorie)».
- Cosa dicono le spine: `DESIGN.md` Brand & Style elenca «stato, categoria del nodo, percorso attivo» e lascia fuori le porte; `edge` mette una `[ASSUMPTION]` («porte neutre… uso non definito») e Elementi aperti n. 5. Il tratto scelto da Marco è quindi dichiarato aperto, non deciso.
- Correzione proposta: chiedere a Marco se le porte colorano (per stato in esecuzione, o per tipo di dato) oppure restano neutre; in ogni caso togliere «porte» dal tratto nel memlog o dalle spine, per non avere due versioni.

### A3. Pannelli in basso: uno con tab invece di più pannelli affiancati — **media**
- Riferimento: ref-2 (Code e Debug running affiancati, ognuno con la sua intestazione e azioni); memlog tratto 4: «pannelli in basso con input, output e log».
- Cosa dicono le spine: un solo pannello `log-panel-height` con `tabs` Log / Input / Output. Il tratto di Marco parla di pannelli al plurale e il riferimento li mostra affiancati; i tab sono una scelta della UX, non annotata come tale.
- Correzione proposta: segnare l'unione in tab come `[ASSUMPTION]` (risparmia larghezza, specie a 1280 px) oppure specificare affiancati con Input/Output a sinistra e Log a destra; far confermare a Marco.

### A4. Pannelli agganciati o card flottanti: testo incoerente — **media**
- Riferimento: ref-2, ref-3, ref-4b (palette, configurazione e pannelli in basso sono card arrotondate con margine staccate dal bordo e dal canvas); tratto 2 «card arrotondate».
- Cosa dicono le spine: Elevation dice «I pannelli seguono le card»; `panel` in Components li descrive come agganciati, con solo un filo sul lato verso il canvas e senza angoli arrotondati nell'editor (arrotondati solo nella dashboard, `rounded.2xl`). Le due frasi non coincidono e il riferimento mostra card flottanti.
- Correzione proposta: decidere e scrivere una sola forma per i pannelli dell'editor (card flottanti con margine `{spacing.3}` e `rounded.2xl`, oppure agganciati) e correggere Elevation o `panel`.

---

## (b) Elementi adottati senza una scelta di Marco (da confermare)

### B1. Niente ombre — **bassa**
- Riferimento: ref-3 / ref-4 (card con ombra leggera sotto campi e nodi).
- Spine: Elevation «il round 3 ha tolto l'ombra di 1–2 px del mock C»; Do's: «Ombre su nodi e card» nel Don't. Il memlog non registra che Marco l'abbia chiesto: è una decisione dell'autore.
- Correzione proposta: segnarla come scelta della UX e farla confermare con il resto (un filo al posto dell'ombra riduce il rilievo rispetto al riferimento).

### B2. Barra icone con etichetta sempre visibile e voci di palette a riquadro con bordo — **bassa**
- Riferimento: ref-3 (barra icone senza etichetta; voci palette a riga senza bordo nei gruppi con «+», a bottoni nei gruppi Logic/Utilities); ref-2 (voci a riquadro).
- Spine: `rail-item` con etichetta `label-xs` sempre visibile; `palette-item` come riquadro con bordo. Nessuno dei due è nel memlog come scelta di Marco; il secondo è un ibrido di ref-2 e ref-3.
- Correzione proposta: marcarli come proposta UX in Elementi aperti (la etichetta sempre visibile è motivata dall'accessibilità e dalla lettura sul telefono; va detto).

### B3. Nella sezione Inspiration appaiono come «Respinto» cose non respinte da Marco — **bassa**
- Spine: EXPERIENCE, Inspiration & Anti-patterns: «Respinto: nomi troncati» (difetto notato dall'autore nel mock C, memlog `(event)`) e «Respinto: catalogo di nodi pronti e connettori» (limite del PRD §7, non una scelta visiva di Marco, anche se ref-2 mostra connettori come GitHub e Notion). Accanto a «Respinto: palette accese/spente», che sono scelte di Marco, il lettore le prende per scelte sue.
- Correzione proposta: cambiare l'etichetta in «Evitato dalla UX» (nomi troncati) e «Fuori dalla v1 (PRD §7)» (catalogo); lasciare «Respinto» solo a ciò che Marco ha respinto (direzioni A, B, D e le palette dei round 1 e 2).

Verificato che non sono stati adottati, e va bene così: collaboratori e cursori (ref-1, ref-4b), pulsante «Generate Workflow» e catalogo di connettori (ref-2, ref-3), campi modificabili dentro il nodo (ref-3), nodi circolari (ref-2), pannello Code (ref-2), archi ortogonali (ref-3), bordo magenta di selezione (ref-4b; il kit usa l'accento grafite scelto da Marco).

---

## (c) Dettagli visibili nei riferimenti e non specificati (specifiche candidate, nessuna decisione presa)

### C1. Posizione delle porte e ancoraggio degli archi secondo l'orientamento — **alta**
- Riferimento: ref-3 / ref-4 (porta come punto al centro del bordo inferiore e superiore del nodo; arco ortogonale che esce da sotto); ref-1 (porte sui lati, una per riga di campo, con il nome del campo vicino); ref-2 (arco che esce dal lato del cerchio).
- Spine: DESIGN dà solo il raggio (`port-radius` 3,5 px) e colore; EXPERIENCE dice «trascina da una porta d'uscita a una d'ingresso». Non è detto dove stanno le porte. Con la direzione adattiva (sinistra-destra su desktop, alto-basso su telefono) la posizione cambia con l'orientamento, e il fan-out e il join (molti archi su una porta) non hanno regola.
- Candidata: porta di ingresso sul lato a monte e di uscita sul lato a valle, centrate sul bordo; ruotano con l'orientamento; una porta per nodo con più archi che entrano o escono; area di tocco 44 px già assunta. Da decidere in Penpot.

### C2. Anatomia della card del nodo — **media**
- Riferimento: ref-3 / ref-4 (riga di testa: tessera icona, nome, chip di stato inline; sotto un filetto e una riga di descrizione).
- Spine: `node-card` ha due righe (tessera e nome; badge di stato e chip `TS`). Non c'è una riga di descrizione o riepilogo della configurazione (per esempio funzione e tentativi) né un filetto fra testa e corpo; non è detto cosa mostra un nodo nell'editor senza stato.
- Candidata: decidere cosa porta il corpo del nodo nell'editor (nessun riepilogo, oppure una riga: funzione, timeout, segreto) tenendo i nomi interi.

### C3. Costrutti con rami: switch, condizione, ramo «else» — **media**
- Riferimento: ref-4b (chip `if` con condizione, chip `else` con «Stop Workflows» in rosso), ref-2 (chip «Condition» sull'arco).
- Spine: Controllo di flusso è una categoria, ma nessuna regola dice come si disegna un nodo con più uscite condizionate, dove sta l'etichetta della condizione (sul nodo, sull'arco), come si vede il ramo non preso in esecuzione (stato «saltata» esiste) e come si distingue il ramo predefinito. Il colore del ramo else in rosso nel riferimento confligge con «rosso = fallita» e va escluso.
- Candidata: chip della condizione sull'arco in `{colors.chip}`, ramo predefinito etichettato a parole, rami saltati con arco tratteggiato e stato «saltata».

### C4. Palette: ricerca, comprimi, aggiunta, raggruppamento — **media**
- Riferimento: ref-2 («Items search»), ref-3 (titolo del pannello, ricerca, pulsante comprimi, «+» a destra di ogni voce, gruppi con filetto e titolo, voci piccole in due colonne).
- Spine: `palette-item` e il titolo di gruppo in `label-caps`; nessun campo di ricerca, nessun titolo del pannello, nessun comprimi; l'aggiunta è clic o `Invio` o trascinamento, senza controllo visibile. Con molte funzioni dei worker la ricerca pesa.
- Candidata: campo di ricerca in cima, titolo, comprimi, gruppo per categoria (3 gruppi: Funzione del worker, Nodo personalizzato, Controllo di flusso), «+» come alternativa al trascinamento sul telefono.

### C5. Intestazione del pannello di configurazione — **bassa**
- Riferimento: ref-4b (titolo grande, sottotitolo esplicativo, X a destra, filetto, sezioni con titolo).
- Spine: `panel` descrive chip categoria, nome, sottotitolo e sezioni con `label-caps`; la X è solo in EXPERIENCE. Manca la posizione della X nella spina visiva e se l'intestazione resta fissa mentre il corpo scorre.
- Candidata: X in alto a destra a 44 px di area, intestazione fissa, sezioni separate da filetti `{colors.border}`.

### C6. Controlli Esegui / Salva / pubblica nella barra — **media**
- Riferimento: ref-4b (Play verde come icona secondaria, Share, Deploy come unico primario scuro, avatar), ref-2 (Play, impostazioni, azione primaria).
- Spine: `workflow-bar` «Salva» ed «Esegui» nell'editor, ma `button-primary` «al massimo uno per area»: non è detto quale dei due sia primario né la gerarchia con «Annulla». Non è deciso se Esegui è un pulsante con testo o un'icona di Play; il verde del Play del riferimento va contro «colore solo come significato» salvo che significhi «completata». Share e Deploy non hanno equivalente (il PRD crea una versione al salvataggio, FR-5; nessun «pubblica» a parte).
- Candidata: «Salva» secondario e «Esegui» primario scuro, oppure l'inverso; Play senza verde. Marco decide, e dice se esiste un passo «pubblica».

### C7. Menu contestuale e azioni di nodo senza tastiera — **media**
- Riferimento: ref-2 (menu con voci e scorciatoie: Example data, Copy link, Custom code, Hide, Remove).
- Spine: l'eliminazione è `Canc` sul canvas; sul telefono non c'è un modo scritto per eliminare un task o un arco, né duplicare, né scollegare (nella UX c'è «collegare toccando» ma non il resto). Nessun menu contestuale o menu del nodo.
- Candidata: un menu del nodo (clic destro su desktop, pressione lunga o pulsante nel pannello sul telefono) con Elimina, Duplica, Scollega; le scorciatoie mostrate accanto alle voci.

### C8. Intestazione dei pannelli in basso e controlli del log — **bassa**
- Riferimento: ref-2 («Debug running» con stato «in progress», «Select object(s)», «Clear», «Copy», riga con orario in colore).
- Spine: `tabs` con contesto in mono a destra; nessun controllo «copia», «svuota», «segui le righe nuove» (c'è la regola di scorrimento automatico nell'accessibilità ma non un controllo visibile), nessun indicatore «in corso».
- Candidata: copia dell'output come azione secondaria, interruttore «segui» nel log, stato in corso nell'intestazione.

### C9. Frecce e confluenze degli archi, percorso attivo sui nodi — **bassa**
- Riferimento: ref-3 (piccola freccia sull'arco, archi che confluiscono in un punto), ref-2 (percorso attivo che evidenzia anche il nodo di partenza con riempimento azzurro).
- Spine: `edge` senza punta di freccia (la direzione è «detta da posizione e porte»); come confluiscono più archi su una porta; il nodo sul percorso attivo resta con il suo stato, non cambia aspetto.
- Candidata: punta di freccia piccola in `{colors.edge}`; confluenza su un solo punto di porta; il percorso attivo colora solo l'arco.

### C10. Controlli di zoom e adattamento a schermo — **media**
- Riferimento: ref-1 (barra flottante in basso con icone e pulsante attivo bianco), ref-1 (indicatori numerici in basso a sinistra), ref-3 (nessun controllo).
- Spine: zoom con rotella, pizzico, `+`, `−`, `0`; nessun controllo a schermo (pulsanti zoom, adatta, ricentra). Chi usa il mouse senza rotella o chi sta sul telefono con una mano non ha un controllo visibile; il tratto «barra flottante» di ref-1 non è stato scelto da Marco, quindi è solo una candidata.
- Candidata: gruppo di tre pulsanti (zoom più, zoom meno, adatta) in un angolo del canvas, 44 px sul telefono.

---

## (d) Riferimenti collegati e descritti nelle spine

### D1. DESIGN.md non cita nessun import — **media**
- Spine: `DESIGN.md` ha nei `sources` i mock di `.working/` e il memlog ma nessuno dei cinque file di `imports/`; Brand & Style ricostruisce le scelte da «Carta chiara» e «Gelato pieno» senza dire che vengono dai riferimenti. Solo `EXPERIENCE.md`, Inspiration & Anti-patterns, li collega.
- Correzione proposta: aggiungere i cinque file a `sources` di DESIGN (e a quelli di EXPERIENCE) e una frase in Brand & Style con i link (ref-3 e ref-4 per Carta chiara, ref-1 e ref-2 per i punti e gli archi, ref-4b per il pannello e la barra).

### D2. I collegamenti in EXPERIENCE dicono cosa si è preso ma non cosa si è lasciato — **bassa**
- Spine: EXPERIENCE collega tutti e cinque i file (ref-4 insieme a ref-3, ref-4b a parte) e dice per ciascuno cosa si prende. Mancano: cosa ciascun riferimento mostra e non si prende (cursori e collaboratori di ref-1, nodi circolari e pannello Code di ref-2, campi nel nodo e Generate Workflow di ref-3, avatar e Share di ref-4b), e che la palette di ref-2 sta a destra mentre Marco ha scelto sinistra. Anche il collegamento a ref-2 «pannelli in basso con il log» non dice che sono due affiancati (vedi A3).
- Correzione proposta: una riga «Non preso» per riferimento, così le prossime revisioni non reintroducono quegli elementi.
