# Spike S-3, le pipeline stream: piano in un blocco

Fonte dei criteri: spine, sezione «S-3 — Pipeline stream». Approccio deciso da Marco il 2026-11-26
(memlog dell'architettura, voce 177): tutto in un blocco, con un piano unico, le soglie approvate
una volta prima delle corse e le corse di correttezza prima di quelle di throughput. Il caso è
l'esempio di UJ-3 del PRD, un'assunzione finché non arriva un caso reale. L'ADR finale è
`docs/decisions/YYYY-MM-DD-stream-pipelines.md` e aggiorna AD-13.

Stato: **soglie e predefiniti approvati da Marco il 2026-11-26** (memlog dell'architettura, voci 178
e 179). Le regole di lettura sono in `risultati/stream-regola-ms.txt` e si committano prima di ogni
corsa misurata.

## 1. Scelte a tavolino

Ogni scelta riporta l'alternativa scartata; l'ADR la ripete con i numeri misurati.

1. **Versione.** `datafusion` 55.2.0 (2026-10-07), con `default-features = false` e solo le
   funzioni `sql`, `datetime_expressions`, `string_expressions`, `unicode_expressions` e
   `recursive_protection`: nessun formato di file e nessuna compressione nel binario. La 55.3.0
   (2026-11-17) ha meno di due settimane e non corregge falle del pacchetto: il suo «fix cargo
   audit» aggiorna solo il file delle dipendenze del repository di DataFusion, che chi usa il crate
   non eredita. Dopo il primo `Cargo.lock` dello spike si controllano su OSV tutte le dipendenze
   transitive; una con un avviso si porta subito alla versione corretta, citando l'avviso.
2. **Stream con colonne dichiarate.** Ogni stream dichiara le sue colonne con un tipo da un elenco
   chiuso (testo, intero a 64 bit, decimale a 64 bit, booleano, istante in UTC), più il tempo
   dell'evento facoltativo (costituzione V: dati tipizzati). L'ingresso valida ogni evento al
   confine: un evento non valido rifiuta il blocco con l'indice dell'evento e il campo.
   Scartata: JSON libero interpretato dalla pipeline, che sposta la validazione dentro il
   processo del tenant e il costo di parsing nel percorso caldo.
3. **Blocchi sul bus.** L'API di FR-18 riceve blocchi di eventi (al massimo 1.000 eventi o 1 MiB).
   Il motore mette l'istante di arrivo nel blocco, lo scrive in Arrow IPC, lo cifra con la chiave
   del tenant nel formato di C-12 e lo appende al topic dello stream del tenant (una partizione);
   risponde 200 solo dopo la scrittura durevole su Iggy. **Accettato** vuol dire questo: risposta
   200. Un evento senza tempo dell'evento prende l'istante di arrivo del blocco, scritto
   nell'ingresso, mai alla lettura (altrimenti la rielaborazione cambierebbe le righe).
4. **Identità e deduplicazione (M-35), proposta.** L'identità di un evento è la posizione del suo
   blocco (AD-18) più l'indice nel blocco. Il client manda ogni blocco con `Idempotency-Key`
   (C-18); la chiave viaggia nel blocco e il processo delle pipeline scarta un blocco già visto
   entro la conservazione dello stream; l'insieme delle chiavi viste sta nel checkpoint. Nessun
   identificativo per evento nella v1: la deduplicazione per evento si scrive in SQL (`DISTINCT`,
   FR-20). Scartata: deduplicazione nell'ingresso, che si perde al riavvio del motore senza una
   tabella durevole in più.
5. **Esecuzione delle finestre (M-31).** Aggregazione incrementale a piccoli lotti: DataFusion
   pianifica la query una volta e valuta filtro, proiezione, chiavi e argomenti degli aggregati su
   ogni lotto; i suoi accumulatori degli aggregati ammessi tengono lo stato parziale per (finestra,
   gruppo). topictoast tiene lo stato, il watermark e la chiusura delle finestre; alla chiusura
   valuta il valore finale, `HAVING` e la proiezione. Scartata: rieseguire la query intera alla
   chiusura su tutti gli eventi della finestra, che a 100.000 eventi al secondo su 5 minuti tiene
   in memoria circa 30 milioni di eventi (ipotesi, stima a tavolino).
6. **Dialetto chiuso (M-33, T-56), proposta.**
   - Una sola `SELECT` su uno stream del tenant, o `UNION ALL` di `SELECT` su più stream con le
     stesse colonne. Ammessi `WHERE`, `GROUP BY`, `HAVING`, `DISTINCT`, `CASE`, `CAST` verso i
     tipi dell'elenco.
   - Finestre nel `GROUP BY`: `tumble(INTERVAL '5 minutes')` (fissa) o `hop(INTERVAL '5
     minutes', INTERVAL '1 minute')` (scorrevole), con le colonne `window_start` e `window_end`.
     Senza finestra la pipeline filtra e proietta evento per evento.
   - Ritardo tollerato e intervallo dei checkpoint sono campi della pipeline, fuori dal SQL.
   - Funzioni ammesse: aritmetica e confronti, `coalesce`, `lower`, `upper`, `length`,
     `substr`, `starts_with`, `LIKE`, `abs`, `round`, `floor`, `ceil`; aggregati `count`,
     `count(DISTINCT …)`, `sum`, `min`, `max`, `avg`.
   - Rifiutati al salvataggio: DDL, DML, `COPY`, `SET`, `EXPLAIN` e ogni altra istruzione; join,
     sottoquery, CTE, `ORDER BY`, `LIMIT`, funzioni finestra con `OVER`; funzioni di tabella; ogni
     funzione fuori elenco (`repeat`, `lpad`, `now`, `random` e le altre); funzioni definite
     dall'utente.
   - Limiti di dimensione: testo della query ≤ 16 KiB, letterale ≤ 1 KiB, profondità
     dell'albero ≤ 64.
7. **Barriere sul SQL, quattro strati.**
   - (1) Controllo sull'albero sintattico contro il dialetto, prima della pianificazione.
   - (2) `SQLOptions` di DataFusion con DDL, DML e istruzioni spenti.
   - (3) Contesto senza nulla registrato: solo gli stream del tenant come tabelle, nessun
     archivio di oggetti (nemmeno `file://`), nessuna funzione di tabella, versamento su disco
     spento.
   - (4) Limiti di memoria: quota dello stato per pipeline misurata da topictoast (64 MiB, Marco), pool di
     DataFusion per il processo, e un limite del sistema sul processo (`RLIMIT_DATA`, senza root).
     Scartato come mezzo principale il cgroup delegato all'utente, che non c'è su ogni host senza
     root (si registra se minisheep lo ha).
   - Il prodotto usa tutti e quattro; lo spike prova ogni strato anche da solo.
8. **Watermark e ritardo (FR-21).**
   - Watermark = il tempo dell'evento più alto visto. Una finestra si chiude quando il watermark
     supera la sua fine più il ritardo tollerato (predefinito proposto: 10 s).
   - Evento in ritardo, cioè la sua finestra è già chiusa: scartato e contato.
   - Evento oltre la tolleranza nel futuro rispetto al suo arrivo (proposta: 60 s): scartato e
     contato, non sposta il watermark.
9. **Stream fermo (M-34), proposta deterministica.**
   - Quando uno stream non riceve blocchi, il motore scrive ogni secondo sul suo topic un
     segnale durevole con l'istante di arrivo.
   - Dopo l'attesa di quiete senza blocchi, il watermark avanza con l'orologio di arrivo dei
     segnali.
   - Il tempo entra così nel log, e una rielaborazione dallo stesso log chiude le stesse finestre
     allo stesso punto. Un processo che legge l'orologio di sistema non lo garantirebbe.
   - Tempo documentato: l'ultima finestra di un'ondata si chiude entro ritardo tollerato + attesa
     di quiete dalla sua fine. Attesa di quiete: 30 s (Marco, 2026-11-26), quindi entro 40 s con il
     ritardo predefinito.
10. **Checkpoint (FR-22, AD-17).**
    - Ogni 10 s (predefinito del PRD), per ogni pipeline, il processo scrive un checkpoint tra due
      lotti. Contenuto: posizione letta, watermark, chiavi dei blocchi viste, stato delle finestre
      aperte, righe in attesa di consegna.
    - Il checkpoint è cifrato con la chiave del tenant (C-12) e sta nello spazio con nome
      dell'archivio dei payload: file temporaneo, `fdatasync`, rinomina, `fsync` della cartella.
    - Al riavvio il processo carica l'ultimo checkpoint e rilegge da Iggy dalla posizione salvata.
11. **Destinazioni (FR-24, C-4).**
    - Ogni riga di una finestra chiusa diventa un avvio con chiave di idempotenza C-4: pipeline,
      versione, fine della finestra, chiave del gruppo.
    - Le righe rifatte dopo un riavvio hanno la stessa chiave e il motore le assorbe.
    - Un rifiuto per quota trattiene la riga nello stato e la riprova.
     - La destinazione stream scrive blocchi con una chiave di idempotenza per lotto di righe; nello
      spike solo la correttezza (Marco, 2026-11-26).
12. **Processo per tenant (AD-6, AD-13, ADR della sandbox).**
    - Ruolo `stream`, un processo per tenant, sottocomando del binario. Riceve solo la chiave del
      suo tenant e legge Iggy con la credenziale di sola lettura del bus. Parla con il motore su
      una coppia di socket ereditata, e con la porta locale dell'API sempre con credenziali.
    - Confinamento nell'ordine della sandbox: descrittori e limiti, namespace utente, runtime
      caldo, Landlock con la sola cartella dei checkpoint del tenant e le sole porte del bus e del
      motore, `PR_SET_DUMPABLE` a 0, seccomp con una lista **riscoperta** su DataFusion vero e sul
      client di Iggy (nella fase 4 della sandbox il ruolo era uno stub).
    - Lo stesso processo fa la validazione a secco a richiesta.

## 2. Che cosa si costruisce (codice da buttare, fuori dai workspace e dal gate)

- **Crate e binario.** Crate `spikes/stream`, binario `streamspike`, toolchain 1.99.1 fissata nel
  crate.
- **Sottocomandi:**
  - `engine`: API di FR-18 a blocchi, avvii con idempotenza in SQLite sincrono, supervisore dei
    processi per tenant, segnali degli stream fermi;
  - `stream`: il ruolo per tenant, confinato;
  - `load`: generatore con seme e registro dei blocchi accettati;
  - `check`: oracolo che ricalcola le finestre attese dal seme e dal registro e le confronta;
  - `probe`: prove ostili.
- **Riuso dallo spike del bus:** adattatore di Iggy, utenti e permessi del bus, cifratura C-12.
- **Riuso dallo spike della sandbox:** ordine di confinamento, `discover.sh` per la lista delle
  syscall.
- **Bus.** Iggy `server-0.10.0`; l'immagine è già su minisheep, nessun download. Compose con il
  nome `ttspikestream`, porte proprie, credenziali generate in `.env` fuori dal repository.
- **Dipendenze nuove.** Versioni esatte, pubblicate da almeno due settimane, senza avvisi su OSV,
  licenze della sezione Stack. Server HTTP: il più piccolo che basta, scelto sulla stessa regola.

## 3. Prove e soglie

| Id | Che cosa | Soglia | Fonte |
| --- | --- | --- | --- |
| V1 | Dialetto: lista di query ammesse e rifiutate (DDL su `/etc/passwd`, `COPY` verso un file, connettore all'indirizzo del bus, funzione dell'utente, funzioni di tabella, join e prodotto cartesiano, `repeat` con un numero enorme, letterale enorme, annidamento profondo, stream di un altro tenant, errore di sintassi, finestra oltre la conservazione, ciclo tra pipeline) | 100 % classificate come scritto; errori di sintassi con riga e colonna; lo stream di un altro tenant dà lo stesso errore di uno inesistente | FR-18, FR-19, spine |
| V2 | Barriere senza il controllo sull'albero (controllo positivo), strato per strato | ogni query ostile fermata da almeno un altro strato; 0 byte letti da file e 0 connessioni verso porte, con lo strato 3 da solo | spine |
| V3 | Memoria ostile a runtime e nella validazione a secco, con il solo pool di DataFusion come controllo | pipeline oltre la quota di stato sospesa con un codice; validazione a secco oltre il limite del sistema: errore con un codice; motore vivo e altre pipeline del processo intatte | FR-25, spine |
| C1 | Correttezza sul caso: finestra fissa (UJ-3), scorrevole, `DISTINCT`, filtro senza finestra | 0 differenze dall'oracolo | FR-20, FR-24 |
| C2 | Ritardo e futuro | ritardatari e futuri contati esatti; l'evento a un anno nel futuro non chiude finestre | FR-21 |
| C3 | Stream fermo, con un kill durante la quiete | ultima finestra chiusa entro ritardo + attesa di quiete dalla sua fine (+ 2 s di margine di misura); stesse righe della corsa senza kill | FR-21, M-34 |
| C4 | Kill -9 del processo per tenant e del motore a metà finestra, 5 per corsa, il client riprova i blocchi | 0 eventi accettati persi, 0 contati due volte, un avvio per riga, righe identiche alla corsa senza guasti | FR-19, FR-22, FR-24, C-4 |
| C5 | Ciclo di vita: sospensione di 30 s, ripresa, sospensione oltre la conservazione, modifica della query | sospesa ≤ 10 ms di CPU in 30 s; ripresa esatta; oltre la conservazione errore con codice e nessuna perdita silenziosa; nuova versione con stato vuoto | FR-23 |
| C6 | Rifiuti per quota del motore per 20 s | 0 righe perse, consegnate dopo | FR-24 |
| C7 | Destinazione stream (solo correttezza, Marco) | ogni riga una volta sullo stream di destinazione, anche dopo i kill | FR-24 |
| A1 | Avvio a freddo del ruolo con DataFusion vero e confinamento, 200 riavvii; validazione a secco della query di UJ-3 | p50 ≤ 100 ms, p99 ≤ 250 ms; validazione a secco riportata | Marco (ADR della sandbox) |
| A2 | Confinamento vero: prove ostili della sandbox (file della chiave, `/proc` del motore, database, configurazione, cartella di un altro tenant, porte diverse da bus e motore) | 0 fughe; lista delle syscall riscoperta e nel commit | AD-6, NFR-8 |
| I1 | Isolamento: tenant A a 50.000 eventi/s, tenant B con pipeline al limite di stato e di CPU, a coppie con il controllo (A da solo) | ritmo di A ≥ 0,95 del controllo, ritardo del consumatore di A senza crescita; B sospesa o rallentata con un codice | FR-25 |
| T1 | Throughput dall'API, con scrittura durevole, cifratura e confinamento, 10 minuti, query di UJ-3 con filtro | media minuti 2-10 ≥ 100.000 eventi accettati/s; ritardo del consumatore ≤ 200.000 eventi (2 s) in ogni campione dei minuti 2-10; righe uguali all'oracolo | NFR-5 |
| T2 | Guasto a ritmo pieno: kill del processo per tenant al minuto 4 e del motore al minuto 7, con la pipeline di verifica (tutti i gruppi) | 0 persi, 0 doppi, un avvio per riga; tempo di recupero riportato | spine, NFR-6 |
| T3 | Latenza aggiunta dai lotti: elaborazione di un lotto, da accettato a contato, da finestra chiudibile ad avvio accettato, a 1.000 e a 100.000 eventi/s | riportata (p50, p99) | spine |
| M1 | RAM: binario con e senza DataFusion, a riposo 60 s dopo l'avvio, nessuna pipeline attiva; processo per tenant a riposo con una pipeline e a 100.000/s; memoria di stato | riportata; la quota del motore vero è una stima etichettata (26,5 MB, spike del bus) | NFR-1 |
| S1 | Sentinella nei campi degli eventi dopo T2 | 0 occorrenze in chiaro nei dati di Iggy e nei checkpoint; controllo positivo dopo la decifratura | C-13 |

NFR-2 per le attività degli altri tenant (FR-25) non si misura: nello spike non c'è il percorso delle
attività. Si dichiara non misurato, come nella sandbox.

## 4. Ordine e arresto

1. V1-V3, poi A2 e A1: il SQL e il ruolo prima del carico.
2. C1-C7: la correttezza prima del ritmo. **Se C1 o C4 non passano, lo spike si ferma** e si torna
   da Marco con i numeri, prima di ogni corsa di throughput.
3. I1, poi T1 (3 corse), T2 (2 corse), T3, M1, S1.

Stima: circa mezza giornata di codice se il riuso dagli spike precedenti regge, una giornata se no;
poi circa 3 ore di corse su minisheep (le corse da 10 minuti e le attese del cancello di quiete).

## 5. Regole delle corse

- Come negli spike precedenti:
  - regole di lettura committate prima delle corse misurate;
  - soglie che non cambiano dopo una corsa;
  - ogni causa «misurata» (con file grezzo) o «ipotesi»;
  - file grezzi in `spikes/stream/risultati/`.
- Misure solo su minisheep.
  - I 12 container dei runner di GitHub non si toccano.
  - Si fermano solo processi e container propri, per PID o nome salvati.
  - Cancello di quiete dello spike del bus prima di ogni corsa: carico a 1 minuto < 1,5 e
    pressione I/O `some avg10` < 1 %.
- Niente sudo né impostazioni di sistema.
- Codice dello spike fuori dai workspace Cargo e npm e fuori dal gate; niente Python.

## 6. Fuori dallo spike

- Join tra stream, finestre di sessione, arm64 (a tavolino nell'ADR, non misurato) e RustFS come
  archivio dei checkpoint (la porta è la stessa, AD-17).
- La funzione di analisi dello storico in Parquet.
- Le quote del tenant oltre la pipeline (FR-30), salvo il rifiuto simulato di C6.
