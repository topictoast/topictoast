# Brief comune per le direzioni visive (round 1)

Ogni renderer produce UNA direzione. Tutte le direzioni mostrano gli stessi schermi con gli stessi
contenuti, così Marco le confronta a parità di contenuto. Differiscono per personalità visiva:
densità, peso dei caratteri, forma dei nodi e degli archi, registro del colore.

## Prodotto in breve

topictoast è un motore di workflow incorporabile (Rust). Due superfici, un solo design system:
- **Kit React**, incorporato senza marchio nell'app ospite (qui: un'app di threat intelligence).
  Comprende l'editor dei workflow, la vista delle esecuzioni, i segreti e l'editor delle pipeline
  stream. Nessun logo né nome topictoast: in alto c'è l'intestazione generica dell'app ospite
  (quadrato grigio come logo + "Threat Intel").
- **Dashboard dell'operatore**, distribuita con topictoast. Stessi componenti e token del kit;
  cambia solo l'accento e compare il logo (wordmark testuale "topictoast").

## Decisioni già prese (vincolanti)

1. Base neutra, quasi monocroma: il colore compare solo come significato (stato, porta, percorso
   attivo, categoria del nodo), mai come decorazione.
2. I nodi sono card arrotondate con bordo sottile su un canvas a griglia di punti.
3. Nell'editor: palette per categoria a sinistra, configurazione del task selezionato a destra.
   Nell'esecuzione: pannello in basso con log, input e output del task selezionato.
4. Tema chiaro e tema scuro sono entrambi obbligatori.
5. Il colore non è mai l'unico segnale di uno stato: c'è sempre anche testo o icona (WCAG 2.1 AA).
6. Testi in italiano. Font di sistema (la CSP vieta risorse esterne): stack sans di sistema e stack
   mono di sistema. Dimensioni in rem con base 16 px.

## Schermo principale: vista delle esecuzioni (kit, FR-41), cornice browser 1440×900

Mostralo DUE volte, una in tema scuro e una in tema chiaro (affiancate o una sopra l'altra).

Contenuto:
- Intestazione dell'app ospite ("Threat Intel", voce di menu attiva "Workflow").
- Barra del workflow: nome "Arricchimento indicatori", "versione 3", stato dell'esecuzione
  "In esecuzione", avviata "alle 14:22 da Giulia", durata "1 min 48 s", pulsante "Annulla".
- Canvas a griglia di punti con il DAG:
  `Ricevi elenco` → gruppo fan-out "Per ogni indicatore · 300" che contiene in parallelo
  `dns_lookup` e `censys_search` → `Arricchisci` → `Calcola punteggio` (nodo personalizzato,
  TypeScript) → join → `Inserisci`.
  - `Ricevi elenco` completato; il gruppo fan-out in corso; i task dopo il join in attesa.
  - Il fan-out da 300 deve restare leggibile: elementi raggruppati con conteggi per stato
    ("completate 287 · in esecuzione 9 · ritentate 3 · fallite 1") e una barra segmentata per
    stato. Non disegnare 300 rami.
  - Ogni nodo mostra tipo (funzione del worker, nodo personalizzato, controllo di flusso) e stato.
- Pannello a destra: task selezionato `censys_search` per l'indicatore `203.0.113.7`. Tentativi:
  1 fallito ("timeout dopo 30 s"), 2 completato. Input e output JSON brevi.
- Pannello in basso: log con orari (14:23:01.204 …), poche righe reali, per esempio
  "attività ricevuta da worker-ti-02", "tentativo 1 scaduto dopo 30 s", "ritentata".
- Variante di stato visibile nello schermo: l'elemento fallito del fan-out con l'errore leggibile.

## Schermo secondario: dashboard, pagina Worker (FR-48), cornice browser 1440×900

Mostralo nel tema principale della direzione.

Contenuto:
- Logo "topictoast" e navigazione: Esecuzioni, Pipeline stream, Worker (attiva), Quote.
- Banner di stato (variante NFR-15): il motore è in ripresa dopo un crash; nessuna esecuzione
  nuova parte finché non torna; nessun dato perso; tempo trascorso "12 s". Il testo esatto non è
  ancora deciso: scrivilo breve e piano, sarà rivisto.
- Tabella dei worker: `worker-ti-01` (funzioni `dns_lookup`, `censys_search`; 8 attività in corso;
  la più vecchia da 41 s), `worker-ti-02` (stesse funzioni; 9 in corso), `worker-score-01`
  (nodo personalizzato "Calcola punteggio"; 2 in corso), `worker-ti-03` "non risponde" da 2 min,
  "4 attività riassegnate".
- Cronologia recente di un worker selezionato con le fasi: ricevuta, in esecuzione, completata,
  fallita, ritentata.

## Ruoli di stato (stessi nomi in tutte le direzioni)

completata · in esecuzione · in attesa · in coda · ritentata · fallita · saltata · annullata ·
scaduta. Worker: collegato · non risponde.

## Regole tecniche

- Un solo file HTML autonomo nel percorso indicato nel prompt. CSS inline, nessun JavaScript,
  nessuna risorsa di rete, nessuna immagine esterna (SVG inline ammesso per archi e icone).
- In cima alla pagina: nome della direzione, 2-3 frasi di motivazione, e i token chiave (colori con
  hex, scala tipografica, raggi, spaziatura) come chip visibili.
- Nei commenti del `<style>`: tutti i valori hex per tema, così si possono riprendere.
- Indicatori di esempio solo da intervalli di documentazione (`203.0.113.x`, `198.51.100.x`,
  `example.com`). Nessun nome reale di aziende o ASN.
