# ADR: le pipeline stream su DataFusion (spike S-3)

- Data: 2026-11-28
- Stato: proposta
- Registra: l'esito di S-3; aggiorna AD-13 e lo Stack della
  [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md) (Update a parte,
  id stabili); chiude NFR-5 e la domanda aperta n. 2 del PRD
- Fonti: spine, sezione «S-3 — Pipeline stream»; `spikes/stream/PIANO.md`,
  `spikes/stream/risultati.md`, le regole di lettura `spikes/stream/risultati/stream-regola-ms.txt` (con
  l'emendamento del 2026-11-27) e i file grezzi in `spikes/stream/risultati/`;
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 177-187

## Contesto

S-3 doveva dire come topictoast costruisce stato, finestre, watermark e checkpoint sopra DataFusion
(M-31), quale dialetto SQL chiuso accetta dai tenant (M-33, T-56), come chiude le finestre di uno
stream fermo (M-34), come identifica e deduplica gli eventi all'ingresso (M-35), e misurare NFR-5,
FR-18…FR-25, NFR-1 e la latenza aggiunta dai lotti. Il caso è l'esempio di UJ-3 del PRD (conteggio
per ASN su finestre fisse di 5 minuti, righe con più di 50 indicatori, un avvio per riga): resta
un'assunzione finché non arriva un caso reale. Marco ha deciso lo spike in un blocco unico, con le
soglie approvate una volta prima delle corse (2026-11-26).

Come leggere i numeri: tutte le misure che contano sono su **minisheep** (x86_64, Linux 7.2.4, NVMe
con btrfs cifrato), senza sudo, con regole di lettura committate prima della prima corsa e 3 corse
per riga (2 per T2 e S1). 49 corse misurate, tutte valide. Il motore è quello dello spike: la sua
parte nella memoria del motore vero resta la stima di 26,5 MB dello spike del bus. Quattro cose sono
state cambiate dopo prove fallite e prima della prima corsa misurata, senza toccare le soglie (punto
«Cambi prima delle corse» qui sotto).

## Decisione

1. **DataFusion 55.2.0** con `default-features = false` e le sole funzioni `sql`,
   `datetime_expressions`, `string_expressions`, `unicode_expressions`, `recursive_protection`.
   Nessun formato di file nel binario; i codec lz4 e zstd di arrow-ipc entrano comunque con
   DataFusion e non si usano (Arrow IPC senza compressione). topictoast costruisce stato, finestre,
   watermark e checkpoint (M-31): DataFusion pianifica la query una volta e valuta filtro,
   proiezione, chiavi e argomenti degli aggregati su ogni lotto, con i suoi accumulatori per
   (finestra, gruppo); alla chiusura topictoast valuta il valore finale, `HAVING` e la proiezione.
2. **Ingresso a blocchi (FR-18).** Un blocco ha al più 1.000 eventi o 1 MiB, con colonne tipizzate
   dichiarate dallo stream e validate al confine. Il motore mette l'istante di arrivo nel blocco, lo
   scrive in Arrow IPC, lo cifra con la chiave del tenant (C-12) e lo appende al topic dello stream
   del tenant, nel gruppo di accettazione dell'ADR del bus (una connessione, 4 gruppi in volo da al
   più 1.024 messaggi): **risponde 200 solo dopo la scrittura durevole del gruppo**. Un messaggio per
   chiamata non regge NFR-5 (misurato: 70.000-89.000 eventi/s).
3. **Identità e deduplicazione (M-35).** L'identità di un evento è la posizione del suo blocco più
   l'indice nel blocco. Il client manda ogni blocco con `Idempotency-Key` (C-18); il processo delle
   pipeline scarta un blocco già visto entro la conservazione dello stream, e l'insieme delle chiavi
   viste sta nel checkpoint. Nessun identificativo per evento nella v1: la deduplicazione per evento
   si scrive in SQL (`DISTINCT`, FR-20).
4. **Dialetto chiuso (M-33, T-56), controllato sull'albero sintattico prima della pianificazione.**
   Una sola `SELECT` su uno stream del tenant, o `UNION ALL` di `SELECT` con le stesse colonne;
   `WHERE`, `GROUP BY`, `HAVING`, `DISTINCT`, `CASE`, `CAST` verso i tipi dell'elenco; finestre
   `tumble` e `hop` nel `GROUP BY`; elenco chiuso di funzioni e aggregati. Rifiutati: DDL, DML,
   `COPY`, `SET`, `EXPLAIN`, join, sottoquery, CTE, `ORDER BY`, `LIMIT`, `OVER`, funzioni di tabella,
   funzioni fuori elenco e dell'utente. Limiti: testo ≤ 16 KiB, letterale ≤ 1 KiB, profondità ≤ 64.
   Ritardo tollerato e intervallo dei checkpoint sono campi della pipeline, fuori dal SQL. Lo stream
   di un altro tenant dà lo stesso errore di uno inesistente; un errore di sintassi ha riga e colonna.
5. **Quattro barriere, il controllo sull'albero obbligatorio** (Marco, 2026-11-28). (1) Controllo
   sull'albero; (2) `SQLOptions` con DDL, DML e istruzioni spenti; (3) contesto senza nulla
   registrato (solo gli stream del tenant, nessun archivio di oggetti né funzione di tabella,
   versamento su disco spento); (4) memoria: quota di stato per pipeline (64 MiB), pool di DataFusion
   e `RLIMIT_DATA` sul processo. **Mancato registrato:** senza (1) tre query ostili non le ferma
   nessun altro strato (letterale enorme, tabella unita con sé stessa, `repeat` enorme che fa cadere
   il processo). La lista delle query ostili di V1 e V2 è un test di regressione della storia che
   implementa il controllo. Il rischio rimasto è un tenant che fa cadere solo il proprio processo.
6. **Watermark, ritardo e futuro (FR-21).** Watermark = il tempo dell'evento più alto visto. Una
   finestra si chiude quando il watermark supera la sua fine più il ritardo tollerato (predefinito
   10 s). Un evento in ritardo (finestra già chiusa) è scartato e contato; un evento oltre 60 s nel
   futuro rispetto al suo arrivo è scartato e contato e non sposta il watermark.
7. **Stream fermo (M-34), deterministico.** Su uno stream senza blocchi il motore scrive ogni
   secondo un segnale durevole con l'istante di arrivo; dopo 30 s di quiete il watermark avanza con
   l'orologio dei segnali. Il tempo entra nel log, quindi una rielaborazione chiude le stesse
   finestre allo stesso punto. Tempo documentato: l'ultima finestra si chiude entro ritardo + quiete
   dalla sua fine (40 s con i predefiniti; misurato al più 39,9 s).
8. **Checkpoint (FR-22, AD-17).** Ogni 10 s, tra due lotti: posizione letta, watermark, chiavi dei
   blocchi viste, stato delle finestre aperte, righe in attesa di consegna; cifrato con la chiave
   del tenant (C-12), scritto con file temporaneo, `fdatasync`, rinomina e `fsync` della cartella.
   Al riavvio si carica l'ultimo e si rilegge da Iggy dalla posizione salvata.
9. **Destinazioni (FR-24, C-4).** Ogni riga di una finestra chiusa diventa un avvio con la chiave
   C-4 (pipeline, versione, fine della finestra, chiave del gruppo); le righe rifatte dopo un
   riavvio hanno la stessa chiave e il motore le assorbe. Il processo consegna le righe a gruppi
   (fino a 1.000 per richiesta), che il motore accetta in una transazione: una riga per richiesta
   regge circa 300 avvii/s su minisheep. Un rifiuto per quota trattiene le righe e le riprova. La
   destinazione stream scrive un blocco per riga con la chiave C-4 come `Idempotency-Key`.
10. **Ciclo di vita (FR-23).** Sospensione e ripresa esatta; una pipeline sospesa oltre la
    conservazione non riprende: errore `retention_exceeded`, posizione tenuta, nessuna perdita
    silenziosa; una nuova versione della query parte con lo stato vuoto.
11. **Isolamento (FR-25).** Quota di stato per pipeline (64 MiB, `state_quota_exceeded`) e budget di
    CPU per micro-lotto (proposta: 30 ms, `cpu_budget_exceeded`): la pipeline che li supera si
    sospende con il codice, le altre del processo continuano.
12. **Processo per tenant (AD-6, AD-13).** Ruolo `stream`, un processo per tenant, attivo solo se il
    tenant ha pipeline attive. Riceve solo la chiave del suo tenant e legge Iggy con la credenziale
    di sola lettura. Confinamento prima del primo byte del tenant: descrittori e limiti
    (`RLIMIT_CORE` 0, `RLIMIT_NOFILE` 512, `PR_SET_NO_NEW_PRIVS`), runtime caldo (DataFusion e
    client di Iggy), Landlock su tutti i thread (sola cartella dei checkpoint del tenant, sole porte
    del bus e del motore), `PR_SET_DUMPABLE` 0, seccomp con una lista di 38 syscall riscoperta su
    DataFusion vero e sul client di Iggy. Senza Landlock il ruolo usa il ripiego dell'ADR della
    sandbox (punto 7).
13. **Validazione a secco in un processo separato** (Marco, 2026-11-28, contro la raccomandazione
    della sessione). La validazione a secco del SQL non gira nel processo delle pipeline ma in un
    processo per tenant a parte, confinato come gli altri ausiliari e senza accesso al bus: una validazione ostile non fa mai
    cadere le pipeline in corsa. Cambia l'ultima frase di AD-13.

## Esito dei criteri di S-3

| Criterio della spine | Numeri | Esito |
| --- | --- | --- |
| NFR-5: ≥ 100.000 eventi/s dall'API di FR-18, scrittura durevole, cifratura, confinamento, 10 minuti | media dei minuti 2-10: 105.004, 104.997, 104.997 eventi/s; ritardo del consumatore al più 33.000 eventi (limite 200.000); righe uguali all'oracolo; senza tetto 242.647 eventi/s | rispettato in 3 corse su 3 |
| Guasto iniettato senza eventi accettati persi (NFR-6) | T2 a 105.000 eventi/s con kill del processo per tenant e del motore: 0 persi, 0 doppi, un avvio per riga, 2 su 2; ripresa in 318 ms e 1.215 ms | rispettato |
| FR-19…FR-25 sul caso | C1-C7 in 3 corse su 3 ciascuna: 0 differenze dall'oracolo; ritardatari e futuri contati esatti; 5 kill per corsa con righe identiche alla corsa senza guasti; ciclo di vita; rifiuti per quota senza perdite; destinazione stream una volta per riga | rispettato |
| Riavvio dal checkpoint senza perdere eventi (FR-22) e un solo avvio per riga (FR-24, C-4) | C4 (3 coppie, 137.142-137.941 righe identiche) e T2 | rispettato |
| Il SQL dei tenant non raggiunge tabelle esterne né funzioni di file o di rete | V1 63 su 63; con lo strato 3 da solo 0 file e 0 connessioni; V2 non supera (punto 5) | rispettato con il controllo sull'albero; **mancato** senza |
| FR-25 con un secondo tenant ostile | I1: ritmo di A 0,9999-1,0002 del controllo, ritardo senza crescita; B sospesa con un codice | rispettato |
| NFR-1 con DataFusion nel binario e nessuna pipeline attiva | motore con DataFusion e nessun tenant: 24,6 MB RSS (10,3 MB senza DataFusion) | misurato sul motore dello spike |
| RAM di ogni processo per tenant | a riposo con una pipeline 63,7 MB RSS (53,6 MB PSS); a 100.000 eventi/s 70-75 MB; stato di UJ-3 0,6-1,3 MB | misurato, contro NFR-1 vedi «Punti aperti» |
| Latenza aggiunta dai lotti | a 100.000 eventi/s: lotto p99 1,9 ms; da accettato a contato p50 88 ms, p99 160 ms; da finestra chiudibile ad avvio accettato p50 56 ms, p99 117 ms | misurata |
| Avvio a freddo del ruolo (ADR della sandbox: p50 ≤ 100 ms, p99 ≤ 250 ms) | p50 28,6-30,2 ms, p99 33,1-38,3 ms con DataFusion vero e confinamento, 200 riavvii per corsa | rispettato |
| Confinamento del ruolo `stream` | 0 fughe su 18 prove ostili in 3 corse; il controllo senza confinamento ne trova 15 | rispettato |
| Sentinella nei dati (C-13) | 0 occorrenze in chiaro nei dati di Iggy e nei checkpoint, 2 su 2; il controllo positivo la trova | rispettato |

## Voci da fissare e provare

| Voce | Esito |
| --- | --- |
| Il SQL si controlla sull'albero contro il dialetto prima della pianificazione | fissato (punto 4); V1 63 su 63 |
| Niente DDL, DML né istruzioni | fissato in due strati (albero e `SQLOptions`) |
| Nessun catalogo di file né archivio di oggetti registrato | fissato (strato 3); 0 file e 0 connessioni con lo strato da solo |
| Versamento su disco spento | fissato (strato 3) |
| Limite di memoria del sistema oltre al pool di DataFusion, anche per la validazione a secco | `RLIMIT_DATA` sul processo; con il solo pool di DataFusion la validazione a secco ostile arriva a 283 MB di RSS (V3, controllo) |
| Casi ostili: `repeat` enorme, prodotto cartesiano, costanti enormi | fermati dal controllo sull'albero; senza, `repeat` fa cadere il processo e la costante enorme passa (punto 5) |

## Cambi prima delle corse

Fatti dopo prove fallite, scritti nell'emendamento datato prima della prima corsa misurata, nessuna
soglia cambiata:

1. avvii consegnati a gruppi in una transazione (prima circa 300 avvii/s su minisheep);
2. C4 con il tempo dell'evento al ritmo dell'orologio (Marco): con il tempo compresso 20 volte le
   riprove sfasate del generatore facevano scartare blocchi come in ritardo («ipotesi» sul codice);
3. scrittura del motore su Iggy in gruppi, come fissa l'ADR del bus (prima 70.000-89.000 eventi/s);
4. la regola Landlock di sola lettura su un file senza il diritto da cartella `ReadDir`.

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:**
  - SQL dei tenant con finestre fisse e scorrevoli, `DISTINCT`, filtri, a 105.000 eventi/s
    sostenuti con scrittura durevole, cifratura e confinamento;
  - risultati esatti contro un oracolo indipendente, anche con kill del processo e del motore;
  - finestre che si chiudono anche su uno stream fermo, in modo ripetibile dal log.
- **Costi:**
  - circa 64 MB di RSS per ogni tenant con pipeline attive, più il processo di validazione a secco
    mentre valida (stima, non misurato);
  - DataFusion aggiunge circa 14 MB al motore a riposo e circa 115 MB al binario dello spike (128,3 contro
    13,6 MB);
  - una sola scrittura su Iggy per motore, condivisa da tutti i tenant;
  - la latenza da accettato a contato è di decine di millisecondi (p99 160 ms a 100.000 eventi/s).
- **Facilità di manutenzione:**
  - stato, finestre e checkpoint sono codice di topictoast sopra gli accumulatori di DataFusion: ogni
    aggiornamento di DataFusion va riprovato contro l'oracolo;
  - il controllo sull'albero è l'unica barriera contro alcune query ostili: ogni nuova funzione o
    clausola ammessa passa dalla lista di regressione;
  - la lista seccomp cambia con le versioni di DataFusion e del client di Iggy;
  - chrono, chrono-tz e humantime sono «non più mantenuti» (RUSTSEC-2026-0543, -0555, -0542),
    senza versione corretta: rischio da seguire, nessuna falla oggi.

## Alternative scartate

- **Rieseguire la query intera alla chiusura** su tutti gli eventi della finestra: a 100.000
  eventi/s su 5 minuti terrebbe in memoria circa 30 milioni di eventi (stima a tavolino); lo stato
  incrementale di UJ-3 misurato è 0,6-1,3 MB.
- **Un messaggio per chiamata verso Iggy**: 70.000-89.000 eventi/s sul disco di minisheep, sotto
  NFR-5.
- **Un avvio per richiesta**: circa 300 avvii/s su minisheep, e il controllo scadeva con righe in
  attesa.
- **Deduplicazione all'ingresso** nel motore: si perde al riavvio senza una tabella durevole in più.
- **Eventi in JSON libero** interpretati dalla pipeline: sposta la validazione nel processo del
  tenant e il parsing nel percorso caldo.
- **Validazione a secco nello stesso processo delle pipeline** (raccomandata dalla sessione,
  scartata da Marco): nessun processo in più e circa 1 ms, ma una validazione ostile con un difetto
  del controllo sull'albero ferma per circa 0,3 s le pipeline del tenant.
- **Una quinta barriera** (limiti sul piano e sul tempo di pianificazione): scartata da Marco; il
  mancato resta registrato.
- **Il cgroup delegato all'utente** come limite principale di memoria: non c'è su ogni host senza
  root; si usa `RLIMIT_DATA`.
- DBSP e Arroyo restano esclusi dalla spine (AD-13).

## Punti aperti e limiti

- **Il budget di CPU per micro-lotto non è mai scattato nelle corse misurate**: in I1 la pipeline di
  CPU di B è stata fermata prima dalla quota di stato. È visto in una sola prova; va riprovato da
  solo nella storia dell'isolamento.
- **Quota di stato superabile**: arrivata a 82,8 MB contro 64 MiB prima della sospensione («ipotesi»:
  si controlla a fine micro-lotto). Una pipeline degli IP distinti costa circa 11,7 byte per evento
  (stima da una prova): con 64 MiB regge circa 5,7 milioni di eventi per finestra.
- **Capacità di scrittura condivisa**: in I1 B ha avuto circa 50.000 dei 100.000 eventi/s offerti
  mentre A teneva i suoi 50.000. Un tenant che offre più della sua parte non è stato misurato; come
  si divide la scrittura tra tenant è da decidere.
- **RAM per tenant contro NFR-1**: NFR-1 si misura a riposo, senza pipeline attive (motore con
  DataFusion 24,6 MB); ogni tenant con pipeline attive aggiunge circa 64 MB, più il processo di
  validazione quando valida. Il totale con pipeline attive va scritto accanto a NFR-1.
- **`RLIMIT_NOFILE` 512** per il ruolo `stream` contro i 64 dell'ADR della sandbox (connessioni di
  Iggy e del motore, thread): da confermare nell'Update di AD-6.
- **Ordine tra gruppi in volo**: con 4 gruppi in volo l'ordine nel log tra gruppi dipende dal client
  di Iggy e non è stato provato. I risultati seguono l'ordine del log; che due corse con gli stessi
  blocchi possano differire per eventi vicini al limite del ritardo tollerato è «ipotesi».
- **Vicine alla soglia**: pipeline sospesa 7,7-9,2 ms di CPU in 30 s contro 10; ultima finestra di
  uno stream fermo chiusa in 39,9 s contro 42.
- **Scorciatoie dello spike** da non portare nel prodotto: HTTP senza TLS, decimali come `f64`, corpo
  degli avvii in JSON in chiaro in SQLite (S1 lo ha visto), un checkpoint per processo, l'insieme
  delle chiavi viste salvato intero, coda di consegna senza limite, versioni delle pipeline in
  memoria, budget di CPU sulla CPU di tutto il processo.
- **Non misurati**: il motore vero; un caso reale al posto di UJ-3; arm64; un kernel senza Landlock
  (ripiego non costruito); la memoria del processo di validazione separato; più processi per tenant
  insieme.

## Conseguenze

- **Spine (Update a parte, id stabili):**
  - AD-13 riscritto con le regole dei punti 2-13, compresa la validazione a secco in un processo
    separato;
  - Stack: `datafusion` 55.2.0 con le funzioni del punto 1; i tre avvisi «non più mantenuto»;
  - S-3 chiuso, con il rimando a questo ADR;
  - Open Questions: budget di CPU da riprovare, divisione della scrittura su Iggy tra tenant, quota
    superabile, `RLIMIT_NOFILE` del ruolo `stream`, RAM per tenant contro NFR-1.
- **PRD:** NFR-5 confermato a ≥ 100.000 eventi/s, misurato su minisheep; domanda aperta n. 2
  chiusa (obiettivo numerico e memoria di stato: 0,6-1,3 MB per la pipeline di UJ-3); FR-18…FR-25
  con i predefiniti fissati qui (blocchi, ritardo 10 s, futuro 60 s, quiete 30 s, checkpoint 10 s,
  quota 64 MiB).
- **Storie:** la storia del controllo sull'albero porta la lista delle query ostili come test di
  regressione; la storia dell'isolamento riprova il budget di CPU da solo.
