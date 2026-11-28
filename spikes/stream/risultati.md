# Spike S-3: pipeline stream su DataFusion, risultati

Data: 2026-11-27/28. Misure solo su minisheep (x86_64, Intel Core Ultra 9 185H, 22 core, kernel 7.2.4-arch1-2, NVMe
Crucial P3 con btrfs su LUKS, governor `powersave`). Regole di lettura: `risultati/stream-regola-ms.txt` (8f03d27) con
l'emendamento datato del 2026-11-27 committato in **`32c446f`** prima della prima corsa misurata; la prima corsa che
conta (V1, seme 4101) parte alle **20:23:47** del 2026-11-27 (`risultati/stream-driver-log-minisheep.txt`). Sorgenti
`501ca0d`, binario `streamspike` sha256 `9fa3cd496fbd7affc442ecea4ac6f228498ae16a8a4c687caf110b1ba339cc34` (128.305.568
byte), identico in ogni file grezzo; `idle_nodf` (M1) sha256 `ac9246bd…588261`. datafusion =55.2.0 senza funzioni
predefinite, iggy 0.12.0 contro Iggy server 0.10.0, toolchain 1.99.1.

**Cosa è stato misurato e cosa no.** Il motore è un motore di spike: API a blocchi di FR-18, cifratura C-12, scrittura
durevole su Iggy prima del 200, avvii con la chiave C-4 in SQLite, supervisore dei processi per tenant. Il processo per
tenant (ruolo `stream`) è quello vero dell'architettura: DataFusion, dialetto sull'albero, quattro barriere, stato con
quota, checkpoint cifrati, confinamento del profilo di prodotto. Il caso è l'esempio di UJ-3 del PRD (assunzione finché
non arriva un caso reale). Non c'è il motore vero: la sua RAM in M1 è la stima di 26,5 MB dello spike del bus.

## Corse e validità

- **49 corse misurate**, tutte valide secondo le regole: cancello di quiete passato (nessun `[macchina non ferma]`), lo
  stesso sha256 prima e dopo ogni corsa, `runner_github=12` prima e dopo in ogni file, pavimento di `fdatasync`
  registrato (2,0-9,0 ms per scrittura, prima e dopo). Dati vuoti all'inizio: `reset.sh` cancella dati di Iggy,
  checkpoint e database degli avvii prima di ogni corsa (letto nello script); nei file la testa del log è pari ai
  blocchi accettati più i segnali. Nessuna corsa scartata, nessuna rifatta.
- Regole committate in `32c446f` alle 20:23:34; prima riga `[stato prima]` delle corse che contano alle 20:23:47.
- File grezzi `risultati/stream-<riga>-<n>-minisheep.txt` (e `-ctrl-` per i controlli positivi di V2, V3, A2). Le corse
  di prova (`-smoke-`) e i file di diagnosi (`diag-*`) non contano e restano nei file.
- Script delle corse, committati prima: `ms-misura.sh` (ordine), `ms-driver.sh`, `ms-row.sh` (semi), `smoke2.sh`,
  `rows2b.sh`, `gate.sh`, `env.sh`.
- Condizione di arresto: C1 e C4 superano in 3 corse su 3, quindi lo spike è proseguito con I1, T1, T2.

## Verdetti

| Riga | Verdetto | Valori letti sui file grezzi |
|---|---|---|
| V1 | **supera** | 63 su 63 classificate come scritto; errore di sintassi con riga e colonna (riga 3, colonna 14); lo stream di un altro tenant dà `unknown_stream` come uno inesistente (`stream-v1-1`) |
| V2 | **non supera** | con il controllo sull'albero spento, `literal_huge` e `join_self` arrivano in fondo, `repeat_huge` fa cadere il processo (segnale 6), `copy_to_file` si ferma solo per «nessun formato di file registrato» e `deep_parens` per il limite di ricorsione del parser; con lo strato 3 da solo: 0 file letti, 0 connessioni (`stream-v2-ctrl-1`) |
| V3 | **da leggere con Marco** | a runtime: pipeline `hog` sospesa con `state_quota_exceeded` a 67.316.744 byte, le altre esatte (`stream-v3-1`). Validazione a secco ostile: con il dialetto acceso `function_not_allowed` in 0,25 ms; oltre il limite del sistema (controllo sull'albero spento, `RLIMIT_DATA` 1 GiB) errore con codice `validation_process_died`, motore vivo, ma il processo per tenant cade (segnale 31) e le sue pipeline ripartono dal checkpoint: «intatte» non vale alla lettera (`stream-v3-ctrl-1`); con il solo pool di DataFusion la stessa validazione arriva a 283 MB di RSS |
| C1 | **supera** | 3 su 3: 0 differenze dall'oracolo su 88.610, 87.231, 85.444 righe (finestra fissa, scorrevole, `DISTINCT`, filtro) |
| C2 | **supera** | 3 su 3: ritardatari 1.166, 1.159, 1.255 e futuri 291, 294, 294, contati uguali dall'oracolo e dalla pipeline; l'evento a un anno nel futuro (il n. 12345 del generatore) cade tra i futuri e le righe restano uguali all'oracolo |
| C3 | **supera** | 3 su 3: ultima finestra chiusa al massimo in 39,94, 39,87, 39,87 s contro 42 s; coppie con e senza kill identiche |
| C4 | **supera** | 3 su 3: 0 persi, 0 doppi, un avvio per riga; coppie identiche (137.142, 137.941, 137.607 righe, 0 diverse); 25, 27, 24 blocchi ripresi dopo i kill |
| C5 | **supera** | 3 su 3: pipeline sospesa 9,00, 7,69, 9,19 ms di CPU in 30 s contro 10; ripresa esatta; nuova versione con stato vuoto; oltre la conservazione (`C5REAL`, cancellazione vera di Iggy) ripresa rifiutata con `retention_exceeded` (409), posizione tenuta, altra pipeline in corsa, 3 su 3 |
| C6 | **supera** | 3 su 3: 99 consegne rifiutate per quota, 0 righe perse, 151.762, 152.678, 153.328 avvii consegnati dopo |
| C7 | **supera** | 3 su 3 con 5 kill: ogni riga una volta sullo stream di destinazione; 2.287, 2.033, 2.182 doppioni assorbiti alla lettura, 0 conflitti |
| A1 | **supera** | 3 su 3: p50 28,6, 30,2, 30,2 ms; p99 33,1, 38,3, 37,7 ms (contro 100 e 250); 200 riavvii su 200 confinati con Landlock e seccomp; validazione a secco di UJ-3 p50 0,99-1,14 ms, p99 1,36-1,82 ms |
| A2 | **supera** | 3 su 3: 0 fughe su 18 prove da negare (22 in tutto); il controllo senza confinamento trova 15 fughe su 18 in 3 corse su 3; da fuori `Seccomp: 2`, `NoNewPrivs: 1`, `environ` del processo e del motore non ispezionabili |
| I1 | **supera** | 3 coppie su 3: rapporto del ritmo di A 0,9999, 1,0002, 1,0001; ritardo di A senza crescita (ultimo terzo 4.375, 7.875, 7.375 eventi contro primo terzo + 50.000); B fermata con `state_quota_exceeded` |
| T1 | **supera** | 3 su 3: media dei minuti 2-10 105.004, 104.997, 104.997 eventi/s; ritardo massimo 32.000, 33.000, 33.000 eventi (0 campioni oltre 200.000); righe uguali all'oracolo |
| T1 senza tetto | riportato | 242.647 eventi/s per 10 minuti, righe uguali all'oracolo (`stream-t1-free-1`) |
| T2 | **supera** | 2 su 2: 0 persi, 0 doppi, 12.288 e 12.225 avvii per altrettante righe, a 104.993 e 104.991 eventi/s; processo per tenant di nuovo pronto in 318 ms, motore in 1.215 ms |
| S1 | **supera** | 2 su 2: 0 occorrenze in chiaro nei dati di Iggy e nei checkpoint (e nelle 41 copie a metà corsa); dopo la decifratura 375.000 e 377.000 nel log e 4.981-5.000 nelle copie di checkpoint con stato |
| T3 | riportato | vedi sotto |
| M1 | riportato | vedi sotto |

## Righe riportate

**T3** (`stream-t3-1`, 180 s per ritmo, microsecondi, secchi con errore ≤ 4,4 %):

| Ritmo | Lotto p50 / p99 | Da accettato a contato p50 / p99 | Da finestra chiudibile ad avvio accettato p50 / p99 |
|---|---|---|---|
| 1.000 eventi/s | 912 / 2.496 | 75.776 / 83.968 | 9.984 / 11.000 |
| 100.000 eventi/s | 1.248 / 1.888 | 88.064 / 159.744 | 56.320 / 116.736 |

**M1** (`stream-m1-1`, VmRSS e PSS in kB, riposo 60 s):

| Processo | VmRSS | PSS |
|---|---|---|
| Binario senza DataFusion a riposo (`idle_nodf`) | 10.296 | 8.062 |
| Motore con DataFusion, nessun tenant | 24.648 | 22.338 |
| Motore con un tenant a riposo | 25.896 | 15.745 |
| Processo per tenant a riposo, una pipeline | 63.672 | 53.595 |
| Processo per tenant a 100.000 eventi/s (120 s) | 70.360-75.420 | 57.272-62.331 |

Stato della pipeline di UJ-3 a 100.000 eventi/s: da 0,59 a 1,32 MB con una finestra aperta. La quota del motore vero è la
stima di 26,5 MB dello spike del bus (stima, altro binario).

## Cambi fatti dopo prove fallite, tutti prima della prima corsa misurata

Ognuno è nell'emendamento datato; nessuna soglia è cambiata.

1. **Avvii a lotti** (parte 2c). Il motore scriveva un avvio per richiesta con un fsync ciascuno: circa 260 avvii/s su
   minisheep (stima: 124.119 consegnati in circa 8 minuti, `stream-smoke-c6-c-minisheep.txt`), e i controlli con 150.000 righe scadevano con righe in attesa. Ora fino a 1.000 righe per transazione
   (svuotamento in 1-3 s). «misurata»: `stream-smoke-c4-minisheep.txt` (prima), `stream-smoke-c1-2c-minisheep.txt` (dopo).
2. **C4 a tempo reale** (Marco, memlog 184). Con il tempo dell'evento compresso 20 volte le riprove sfasate del
   generatore dopo un kill del motore facevano scartare blocchi come in ritardo (8 e 4 righe diverse, 0 persi, 0 doppi,
   `stream-smoke-c4pair-2c-*-laptop.txt`); la causa è «ipotesi» letta sul codice del generatore. Ora 2.000 eventi/s al
   ritmo dell'orologio per 10 minuti: a 20.000/s la pipeline `ipd` superava la quota di stato.
3. **Scrittura del motore su Iggy in gruppi** (parte 2f), come fissa l'ADR del bus (una connessione, 4 gruppi in volo da
   al più 1.024 messaggi). Prima un messaggio per chiamata: 70.000-89.000 eventi/s a T1 per 10 minuti con la CPU al 5-8 %
   e circa 2,9 flush del disco per blocco; dopo 104.992 eventi/s con circa 2,2 flush per blocco («misurata»,
   `diag-campioni-fase150-minisheep.txt`, `diag-campioni-2g-t1-minisheep.txt`). Che il limite fosse una scrittura alla
   volta sull'unica connessione è «ipotesi».
4. **Landlock `FullyEnforced`** (parte 2d): la regola di sola lettura su un file chiedeva anche `ReadDir`; tolto, il
   ruleset è pieno («misurata», `stream-smoke-a1-2d-*`).

## Scoperte per l'ADR

- **V2 non supera** («misurata»): il controllo sull'albero è l'unica barriera contro `literal_huge` e `join_self`, e
  contro `repeat_huge`, che senza di lui fa cadere il processo per tenant. Le altre tre barriere non bastano da sole.
  Con il solo strato 3 (niente registrato) nessun file e nessuna connessione.
- **La validazione a secco nel processo del tenant può farlo cadere** («misurata», `stream-v3-ctrl-1`): con il controllo
  sull'albero spento e `RLIMIT_DATA` a 1 GiB il processo muore per segnale 31 (`SIGSYS`) e il motore lo riavvia; le
  pipeline ripartono dal checkpoint. Che il segnale venga da `abort()`, che chiede `tgkill` fuori dalla lista, è
  «ipotesi». Con il dialetto acceso la query si ferma prima, in 0,25 ms.
- **Quota di stato e cardinalità** (stima da una prova, `stream-smoke-c4-2d-minisheep.txt`): la pipeline degli IP
  distinti è arrivata a 67 MB dopo circa 5,75 milioni di eventi della stessa finestra, circa 11,7 byte per evento; con 64
  MiB regge quindi circa 5,7 milioni di eventi per finestra. La quota può essere superata (82,8 MB in I1): che si
  controlli solo a fine micro-lotto è «ipotesi».
- **Il budget di CPU non è mai scattato nelle corse misurate di I1**: la pipeline di CPU di B è stata fermata prima dalla
  quota di stato. La barriera della CPU per micro-lotto è vista solo in una prova (`stream-smoke-i1-minisheep.txt`, 2b).
- **La scrittura su Iggy è unica per tutti i tenant**: in I1 B ha avuto circa 50.000 dei 100.000 eventi/s offerti. La
  soglia è su A e A ha tenuto; un tenant che offre di più non è stato misurato.
- **Database degli avvii in chiaro**: 28.966 e 30.720 occorrenze della sentinella nelle righe degli avvii (fuori da S1,
  scorciatoia dello spike: il corpo dell'avvio è JSON in chiaro). Dove stanno e come si cifrano le righe degli avvii nel
  prodotto lo decide l'ADR.
- **Vicine alla soglia**: C5 (7,7-9,2 ms contro 10) e C3 (39,9 s contro 42).
- **Codec nel binario**: arrow-ipc lz4 e zstd entrano sempre con DataFusion (`stream-codec-tree.txt`); la pipeline scrive
  IPC senza compressione.
- **Tre avvisi «non più mantenuto»** (chrono, chrono-tz, humantime), senza versione corretta; nessuna falla (OSV,
  `stream-osv-2c.txt`).

## Scorciatoie dello spike (non del prodotto)

HTTP scritto a mano senza TLS; un checkpoint per processo con le posizioni per pipeline; decimali come `f64`; corpo JSON
degli avvii in SQLite; l'insieme delle chiavi viste salvato intero in ogni checkpoint; conservazione simulata dal lettore
in c5c (quella vera in C5REAL); matrice di V2 in un processo figlio; byte di stato stimati; `UNION ALL` solo costruita e
validata a secco; versioni delle pipeline nella memoria del motore; coda delle righe in consegna senza limite;
`RLIMIT_NOFILE` 512 invece del 64 dell'ADR della sandbox; nessun ripiego con il namespace utente senza Landlock (il
processo non parte); ordine nel log tra gruppi in volo non garantito; budget di CPU sulla CPU di tutto il processo.

## Non misurato

Il motore vero; un caso reale al posto di UJ-3; arm64; un kernel senza Landlock; un tenant che supera la capacità di
scrittura su Iggy mentre un altro è sotto soglia; latenza dell'API sotto carico (solo ritmo e ritardo); la memoria di più
processi per tenant insieme.
