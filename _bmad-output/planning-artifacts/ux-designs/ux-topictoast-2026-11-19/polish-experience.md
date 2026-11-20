# Rifinitura di EXPERIENCE.md (structure, poi prose)

Standard: `skill:bmad-review lenses=structure,prose`. Data: 2026-11-20. Solo `EXPERIENCE.md`
modificato; frontmatter, `status` e `updated` intatti (la chiusura spetta a Finalize).

Lettura del documento: spina di riferimento per chi scrive storie, architettura e codice del kit e
della dashboard. Modello di struttura: Reference/Database (accesso casuale, schema costante).

Misure: 840 → 850 righe (`wc -l`), 11.612 → 11.539 parole (−73, −0,6 %). Le righe crescono per
gli otto nuovi titoli `###` e per l'elenco dei limiti superati.

## Lente structure — 10 applicate, 6 saltate

Applicate:

1. **MOVE** — `## Inspiration & Anti-patterns` spostata subito prima di Key Flows (testo identico):
   le sezioni di comportamento (Responsive, Personalizzazione, Tempo reale) restano contigue, come
   negli esempi di bmad-ux, dove Inspiration precede Key Flows.
2. **Scaffolding** — in Interaction Primitives le otto etichette in grassetto diventano titoli
   `###` con lo stesso testo (Canvas, Selezione e configurazione, Collegare, Aggiungere dalla
   palette, Salvare ed eseguire, Pipeline stream, Tastiera, Telefono). La premessa di Tastiera, con
   [ASSUMPTION A-17], diventa il primo paragrafo. I rimandi «Interaction Primitives, Canvas» ecc.
   restano validi.
3. **MOVE** — il ciclo di vita del banner di ripresa [ASSUMPTION A-4] passa da Voice and Tone alla
   riga `recovery-banner`, che prima rimandava a Voice and Tone. Riga A-4 di Elementi aperti
   aggiornata: «(`recovery-banner`)».
4. **MERGE** — «Salvare ed eseguire» compariva in due blocchi: tolti da Stati trasversali i fatti già
   presenti in `input-form` (input non valido) e in `confirm-dialog` (uscita senza salvare).
   «con la navigazione del kit» passa nella riga `confirm-dialog`. Rimandi qualificati in
   `button-primary` e `workflow-bar`.
5. **CONDENSE** — riga `edge`: creazione e rimozione rimandano a Interaction Primitives, Collegare,
   che contiene già ogni clausola.
6. **CONDENSE** — riga `fanout-group`: tolto `Invio` sul gruppo, già in Tastiera; resta che conteggi
   e «altri N falliti» sono `button`.
7. **CONDENSE** — riga `canvas`: tolta la direzione del grafo, identica a Interaction Primitives,
   Canvas; al suo posto un rimando.
8. **CONDENSE** — Stati trasversali: «gli stati restano fermi» (già in Tempo reale, Pausa) e
   la forma degli errori in documento e SQL (già in `validation-message`) diventano rimandi.
9. **CONDENSE** — Information Architecture: tolto l'elenco dei mock approvati, già nella colonna
   «Riferimento visivo»; resta chi non ha mock.
10. **Scaffolding** — Voice and Tone, «Limiti superati»: il paragrafo con dieci esempi diventa un
    elenco puntato, stringhe invariate. Elementi aperti: l'introduzione dice «quattro blocchi» (prima
    «due elenchi») e i rinvii hanno l'etichetta **Rinvii** come gli altri blocchi.

Saltate:

- Riordino della tabella Component Patterns o gruppi interni: troppi spostamenti per poco
  guadagno; i nomi coincidono con `DESIGN.md`.
- «Testo puro» in Accessibility Floor, doppione di Voice and Tone (NFR-7): lasciato come voce di
  controllo per chi rivede l'accessibilità.
- «ID dei worker» in Voice and Tone (regola di visibilità più che di tono): spostarla richiede di
  riscrivere il rimando in `attempt-card`; guadagno basso.
- Le varianti di UJ-2 come elenco: la forma narrativa dei percorsi resta.
- `Tempo reale` come `###` dentro State Patterns: avrebbe cambiato il livello di un titolo; basta lo
  spostamento al punto 1.
- I rimandi a `DESIGN.md` all'inizio di ogni sezione: servono all'accesso casuale (PRESERVE).

## Lente prose — 8 applicate, 2 saltate

Stile tenuto: impersonale, frasi nominali, termini tecnici e stringhe «» invariati.

Applicate:

1. `panel`: «sta sotto il canvas, non nell'editor» → «assente nell'editor, sta sotto il canvas»;
   «entrambi diventano `bottom-sheet`» → «i pannelli dell'area» (la riga `bottom-sheet` e la
   tabella Responsive ne elencano tre).
2. `palette-item`: «Raggruppate per il gruppo» (soggetto mancante) → «Voci raggruppate secondo il
   gruppo».
3. Telefono: «la pagina dell'ospite si scorre fuori» → «si scorre solo fuori dal canvas».
4. Responsive & Platform: frase unica con quattro punti e virgola divisa in tre frasi; «solo se
   l'ospite lo abilita» spostato accanto al gesto indietro, a cui si riferisce.
5. Variante «da telefono» di UJ-2: frase unica divisa in tre, con il soggetto Giulia.
6. Selezione e configurazione: «(titolo aperto: Elementi aperti)» → «(titolo della sezione ancora
   aperto: Elementi aperti)».
7. Information Architecture: «nella v1» portato in testa alla frase.
8. A capo spezzati riallineati a circa 100 colonne (Sessione scaduta, «Salva», Pipeline stream,
   caso limite di UJ-1, Telefono, introduzione di Elementi aperti). Resa invariata.

Saltate:

- «Se un altro ha pubblicato prima» (Salvare ed eseguire) stride con «non esiste un passo
  «pubblica»» (T-52), ma «pubblicato» può includere la pubblicazione da codice: decide l'autore.
- «Pulsanti: un verbo» mentre «Nuova pipeline stream» non è un verbo: è una questione di contenuto,
  non di forma.

## Controlli (Ruby, contro la copia di partenza)

- Frontmatter identico byte per byte.
- Titoli: tutti i 23 originali presenti, UJ-1…UJ-5 verbatim; 8 `###` aggiunti (punto 2).
- Insiemi uguali prima e dopo: link (15, tutti esistenti), `{token}` (10, tutti risolti nel
  frontmatter di `DESIGN.md`), tag `[ASSUMPTION A-n]` (22), stringhe «» (262 distinte), span in
  backtick (151), id FR/NFR/T/UJ/A/WCAG (131). Conteggi più bassi solo per i doppioni tolti.
- Elementi aperti in pari: ogni tag del corpo ha la sua riga; le 8 righe senza tag qui (A-27, A-29,
  A-30, A-32…A-36) sono esattamente i tag di `DESIGN.md`.
