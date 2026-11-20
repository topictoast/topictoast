---
title: topictoast — polish di DESIGN.md
created: 2026-11-20
oggetto: DESIGN.md (doc_standards `skill:bmad-review lenses=structure,prose`)
---

# Polish di DESIGN.md

Lettura: il documento serve a chi costruisce il kit e la dashboard per trovare token, regole e
anatomia di ogni componente. Modello di struttura: Reference/Database, nell'ordine canonico di
design.md, che resta invariato.

Righe: 882 prima, 899 dopo. Parole: 4885 prima, 4968 dopo. L'aumento viene dalle tabelle e da due
chiarimenti: il polish ha puntato alla consultazione, non alla lunghezza.

## Lente structure: 6 applicati, 7 saltati

| # | Dove | Azione | Esito |
|---|---|---|---|
| S1 | Components, elenco dei mock | CONDENSE: tabella «Mock / Componenti» al posto di un elenco a virgole che mescolava link e nomi | applicato |
| S2 | Shapes, Icone | CONDENSE: tabella «Gruppo / Significato / Icona» al posto di 11 abbinamenti in prosa; il paragrafo che segue è riallineato | applicato |
| S3 | Layout, Area workflow del kit | CONDENSE: paragrafo di otto fatti diviso in sette punti | applicato |
| S4 | Layout, Telefono | MOVE: la condizione del layout compatto in testa, il mock in coda come nelle altre aree | applicato |
| S6 | Shapes e `edge` | CUT: la regola del tratteggio compariva due volte; resta in `edge` (con `dash-pending`), Shapes rimanda lì («il tratteggio è descritto in `edge`») | applicato |
| S7 | Components, `panel` | CONDENSE: sotto-punti sinistra, destra, in basso, forma | applicato |
| S5 | Colors, Testo sempre neutro | MOVE dei contrasti 4,27 / 4,31 / 3,87 in Contrasti: provato e annullato, perché sotto «Coppie vietate» diventava una regola nuova e restringeva «di categoria e di stato» a «di stato» | saltato |
| S8 | Layout (pannelli agganciati, T-51) e `panel` | PRESERVE: Layout lo dice per tutta l'area del kit, `panel` solo per l'editor; unirli imporrebbe una scelta | saltato, vedi Domanda |
| S9 | Colors, regola Accento per superficie e paragrafo Accenti | PRESERVE: regola contro descrizione | saltato |
| S10 | Elevation & Depth | PRESERVE: sezione canonica, contiene A-33; ripete Base neutra solo come rinforzo | saltato |
| S11 | Do's and Don'ts | PRESERVE: riepilogo utile; `EXPERIENCE.md` cita la regola del nome intero qui | saltato |
| S12 | Components, 44 voci senza gruppi | QUESTION: i sottotitoli aiuterebbero, ma l'ordine ricalca 1:1 le chiavi `components:` | saltato |
| S13 | Layout, frasi «Mock:» per area | PRESERVE: fatti di layout, diversi dalla tabella dei componenti | saltato |

## Lente prose: 13 applicati, 4 saltati

Stile mantenuto: italiano tecnico e asciutto, frammenti nelle voci dei componenti, stringhe UI tra
«», virgola decimale, identificatori in mono.

| # | Prima | Dopo |
|---|---|---|
| P1 | sono qualificati a 3:1 per gli indicatori non testuali | sono scelti sulla soglia di 3:1 degli indicatori non testuali |
| P2 | più spesso (`edge`) | più spesso (larghezze in `edge`) |
| P3 | Ogni categoria ha un primo piano (`{…}`) per l'icona… | token di esempio introdotti da «per esempio», come in Stati |
| P4 | Una riga per classe, con la coppia peggiore, ricalcolata… | Ogni riga riporta la coppia peggiore della classe, ricalcolata… |
| P5 | l'arco attivo passa a 3,35:1 e 8,75:1 | arriva a 3,35:1 in chiaro e 8,75:1 in scuro (verificato sugli hex) |
| P6 | la selezione è portata dal contorno | a segnalare la selezione è il contorno |
| P7 | nessun peso nel bundle | nessun file di font nel bundle («peso» si confondeva con i pesi 400 e 500) |
| P8 | sta sotto il solo canvas, con il pannello destro accanto a entrambi | sta sotto il canvas soltanto, e il pannello destro li affianca entrambi |
| P9 | `status-bar`: a capo spezzato a metà frase | riga riallineata |
| P10 | nome (`name`, o `name-function`) | nome (`name`, o `name-function` per un nome di funzione), come in `node-card` |
| P11 | testa del `bottom-sheet` | pulsanti in testa al `bottom-sheet` |
| P12 | `form-field` con indicatore a destra: segreti… | `form-field` con indicatore a destra. Usi: segreti… (come in `chip`) |
| P13 | titolo e corpo tutti in | titolo e corpo entrambi in |

Saltati:

- Shapes, «cambiano da un mock all'altro e, con le nuove, si disegnano in Penpot»: il referente di
  «le nuove» non è confermato da `.memlog.md`, `triage.md` o `EXPERIENCE.md`. Da valutare:
  «si disegnano in Penpot insieme alle icone nuove»?
- Accenti, «si adatta a ogni ospite che lo sostituisce»: suona strano, ma ogni riscrittura sceglie
  un significato. Da valutare: «così sta bene con ogni ospite, che può comunque sostituirlo»?
- Brand & Style, la frase sui riferimenti di Marco: lunga ma ordinata come un elenco; voce
  dell'autore.
- Avvertenza iniziale su una riga di 331 caratteri: paragrafo non toccato, quindi non riallineato.

## Controlli

- Frontmatter (righe 1–506) identico byte per byte; YAML valido (Ruby Psych).
- 342 riferimenti `{token}`, 98 distinti, tutti risolti.
- Tag `[ASSUMPTION A-n]`, link, codici T-, FR-, SM-, PRD § e numeri: stessi conteggi di prima.
  Le stringhe «» cambiano solo per il riallineamento delle righe. `edge` tra apici inversi compare
  una volta in più (il rimando di S6).
- 44 voci in Components, nello stesso ordine delle chiavi `components:`; l'elenco resta compatto
  (nessuna riga vuota dentro una voce). Titoli canonici intatti.
- `diff -u` sul corpo letto riga per riga: ogni blocco corrisponde a una riga S o P.

## Domanda per l'autore

S8: i pannelli sono agganciati in tutta l'area del kit (Layout) o solo nell'editor (`panel`)? Le due
frasi dicono cose diverse sulla vista delle esecuzioni.
