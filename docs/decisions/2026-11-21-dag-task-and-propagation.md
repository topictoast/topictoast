# ADR: task `dag` di prima classe e regola di propagazione

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-8, AD-9 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-1, FR-2, FR-3, FR-32, FR-33, FR-40, §7, §8.2, §11 n.8; `research-ows.md` (sezioni
  1 e 5); [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 4, 7, 24,
  31, 46, 48, 51, 56; `review-decisions.md`; revisioni `reconcile-prd.md`, `review-security.md`
  (SEC-A3), `review-adversarial.md` (R18)

## Contesto

Il PRD vuole dipendenze arbitrarie (FR-2) e fan-out dinamico con join (FR-3) su un sottoinsieme
di OWS 1.1 (FR-1). Il salvato deve essere valido secondo lo schema di topictoast, non secondo lo
schema ufficiale (FR-40). PRD §11 n.8 chiede una regola di propagazione di fallimenti e salti.

Fatti di OWS 1.1.0 (`research-ows.md`):

- non ha costrutti DAG e nessuna proposta upstream; le più vicine sono #1143 e #1212;
- lo schema rifiuta proprietà nuove sui task e tipi di task nuovi (`unevaluatedProperties: false`,
  `oneOf` chiuso). Passano solo `metadata`, chiavi sconosciute alla radice e `call` a funzioni
  custom;
- `do` esegue in ordine di dichiarazione e `then` è un salto con scope;
- un task con `if` falso è «skipped», ma tra le fasi non esiste `skipped`;
- tra gli argomenti delle espressioni ci sono `$secrets` (solo in `input.from`) e
  `$authorization`;
- un maintainer ha commentato (#1184) che ciò che cambia il comportamento va in task di prima
  classe, non in chiavi di estensione. È un commento, non testo della spec.

Il PRD accetta che i workflow con l'estensione DAG non girino su altri runtime OWS (§7).

## Decisione

- **[AD-8](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)** (memlog,
  voce 24): `dag` è un task di prima classe, contenitore distinto da `do`, con le dipendenze nella
  chiave nativa `dependsOn`.
  - Schema di topictoast = il sottoinsieme di FR-1 dello schema ufficiale OWS 1.1 + il tipo `dag`;
    un costrutto fuori dal sottoinsieme è un errore con costrutto e percorso (C-7). La voce 24
    diceva «schema ufficiale OWS 1.1 più il tipo `dag`»; il sottoinsieme di FR-1 è entrato dopo la
    verifica degli input (`reconcile-prd.md`, memlog voci 48 e 51).
  - Un documento senza `dag` resta OWS puro. Uno con `dag` è rifiutato dai validatori ufficiali
    con errore esplicito.
  - Dentro `dag` sono vietati `then`, `exit`, `end` e `switch`, che restano in un `do` annidato; le
    condizioni usano l'`if` del task (voce 31).
  - Lo `switch` dell'editor è un gruppo che diventa un `do` annidato con lo `switch` di OWS e i suoi
    rami. Nel `dag` conta come un solo nodo, e il task che riunisce i rami dipende dal gruppo
    (voce 46). La forma esatta del `do` che vale come gruppo `switch` la riconosce una funzione
    del dominio, esposta da `packages/validator`; ogni altro `do` resta un `do`.
  - `$secrets` e `$authorization` di OWS sono vietati nei documenti: errore C-7 con costrutto e
    percorso, anche nel browser. I valori dei segreti entrano solo al confine della chiamata
    (consegna al worker, header di `call http`), mai in input, contesto o output.
  - Join dei dati, `$context` concorrente e sintassi del fan-out dinamico: Update della spine
    prima della prima storia `dag`.
- **AD-9** (memlog, voce 31): una dipendenza fallita (dopo i retry) o saltata rende il task
  saltato, con il motivo, e così i suoi discendenti. I rami indipendenti arrivano in fondo.
  L'esecuzione è fallita se almeno un task è fallito. Nessuna eccezione per singolo task nella v1.
  La regola vive solo nel dominio.

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:**
  - una sola forma di dipendenza per editor, SDK e motore;
  - i documenti senza `dag` restano OWS portabile;
  - nessun task resta in attesa senza fine, e il lavoro indipendente arriva in fondo;
  - lo `switch` si usa nell'editor senza toccare la regola, e editor, kit e motore riconoscono il
    gruppo con la stessa funzione;
  - nessun valore di un segreto finisce in input, contesto, output, bus o archivio dei payload.
- **Costi:**
  - un documento con `dag` non è OWS valido;
  - il CTK di OWS gira solo sulle parti standard;
  - dentro `dag` niente `then`, `exit`, `end`, `switch`: serve un `do` annidato;
  - un task saltato dal suo `if` salta anche i discendenti, come uno fallito: il gruppo `switch`
    è la via per i rami alternativi;
  - un documento OWS che usa `$secrets` o `$authorization` è rifiutato; i segreti arrivano al
    task solo dal confine della chiamata, la cui forma nel formato resta aperta (M-02).
- **Facilità di manutenzione:** la regola vive solo nel dominio, e così il riconoscimento del
  gruppo `switch`. Lo schema di topictoast segue lo schema ufficiale e va allineato a ogni
  rilascio OWS: la 1.1.0 ha 8 breaking change pur essendo una minor (memlog, voce 2). OWS non ha
  ancora una semantica normativa per concorrenza e cancellazione (#1212), quindi le scelte di
  topictoast potranno divergere da quanto OWS scriverà poi.

## Alternative scartate

- **`dependsOn` in `metadata` dentro `do`** (memlog, voce 24): un altro motore eseguirebbe in
  ordine, sbagliando in silenzio, e cambia il significato di `do`.
- **`call` a una funzione custom** (memlog, voce 24): task interni opachi per OWS.
- **Chiave per task «parte sempre»** (memlog, voce 31): un campo e casi in più.
- **Fail-fast** (memlog, voce 31): perde lavoro indipendente già avviato.
- **Distinguere il salto per condizione da quello per fallimento** (memlog, voce 46): cambia la
  regola fissa.
- **`$secrets` e `$authorization` con la semantica di OWS** (`review-security.md` SEC-A3; nessuna
  alternativa registrata nel memlog): un'espressione come `input.from` mette il valore del segreto
  nell'input del task, e da lì sul bus, nell'archivio dei payload, nella vista delle esecuzioni e
  al worker come dato, contro FR-32 e FR-33; l'ausiliario jq dovrebbe ricevere segreti, contro
  AD-6.

## Conseguenze e criteri di revisione

- **Open Questions della spine** (proprietario architettura; Marco se la risposta tocca AD-9):
  - da chiudere con un Update della spine prima della prima storia `dag`: join dei dati,
    `$context` concorrente e sintassi del fan-out dinamico dentro `dag`;
  - la politica d'errore del fan-out «il join riceve l'errore» di FR-3 rispetto ad AD-9: l'elemento
    fallito conta come dipendenza fallita? rende fallita l'esecuzione?
- M-02 (come un task invoca funzioni e nodi, come dichiara segreti e connessioni nel formato, come
  scrive su uno stream) è aperta, prima della prima storia che la tocca. Il divieto di `$secrets`
  e `$authorization` è già un vincolo per la risposta.
- Si riapre se OWS pubblica un costrutto DAG o una semantica di concorrenza normativa (#1143,
  #1212). La proposta dell'estensione a OWS viene dopo la prova in produzione (PRD §8.2) e sarà un
  ADR nuovo.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63): nessuna voce E-1…E-9
  cambia AD-8 o AD-9. Restano da valutare nelle Open Questions della spine il percorso del task
  con l'escape di RFC 6901 e l'istanza con gli indici (`review-adversarial.md` R18), che toccano
  gli errori C-7 di AD-8, e M-02 chiusa prima di S-1 con un solo proprietario della forma delle
  connessioni (rubrica).
