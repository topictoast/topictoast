# Spike S-1, fase 4: risultati

Segue PIANO.md, sezione «Fase 4». Qui ci sono i **blocchi A** (Iggy sotto carico, conservazione, aggiornamento), **B** (guasti e ripresa) e **C** (percorso dei dati di Iggy, aggiornamento rifatto, timer durevoli); il **blocco D** è fermo dopo la riga del costo dei timer (criterio non superato, log dei task non costruito); il blocco E non è fatto.
Base: Iggy `server-0.10.0` (immagine `apache/iggy:0.10.0`), collegamento dei worker gRPC con TLS, motore della 2d (4 gruppi in volo, 1.024 messaggi, soglia 64), SQLite, minisheep con gli stessi 4 CPU delle fasi 2c-3
(motore e Iggy su `1-4`, carico su `8-11`). Numeri grezzi in `risultati/*-4-ms-*`; ogni file porta il pavimento di `fdatasync` prima e dopo la corsa (300 scritture da 4 KB, come nella fase 3).
Ogni causa citata è segnata **misurata** (con il file grezzo) o **ipotesi**.

## Blocco A

### Sintesi

1. **Stalli a 10.000 attività/s: riprodotti, la fonte è il disco, nessuna configurazione di Iggy provata li toglie.** Il motore completo ne ha avuto uno in 1 corsa su 3 (`full` r2, minuti 4-7: 4.154 e 6.139 att./s, poi recupero), le altre due e `noarc` sono piatte a 10.000
   (nella fase 3 ne stallavano 3 su 4). Iggy da solo, allo stesso ritmo di byte, **non riproduce l'episodio**: mostra uno scalino di latenza (append p50 per richiesta da 128 messaggi da 9 a 35 ms, attesa per scrittura sul disco ×3, senza recupero in 4-6 minuti) che il bench a scadenze fisse
   trasforma in un calo di throughput (4 connessioni × 128 messaggi ogni 24,4 ms: oltre quella latenza il ritmo cala a 4 × 128 / 35 ms = 14,6k messaggi/s), mentre il motore accorpa e lo tollera (17-19 ms per richiesta, 10.000 att./s). Lo scalino è comparso sempre prima
   nelle corse successive (minuto 7, 4, 2): l'ordine delle corse è confuso con le due modifiche. Nello stallo vero del motore il disco sta al 100 % con 22-33 ms per scrittura (0,25 ms normali) e solo 1.100-1.350 scritture/s e 32 MB/s in tutto, `system.slice` meno Iggy scrive 0,7-0,9 MB/s,
   la temperatura è 51 °C: il disco stesso si ferma per circa 2 minuti.
   **Il ritardo di 7-17 s è ritardo di proiezione, misurato**: nello stallo archivio e database del proiettore costano 11-12 volte di più (63 e 98 ms contro 5,5 e 8,5), la lettura dal bus no (1,0-1,3 ms). Margine sottile anche fuori dagli stalli: il disco sta all'89 % di utilizzo a 10.000 att./s.
2. **Conservazione: c'è, a granularità di segmento.** Iggy 0.10.0 cancella i segmenti chiusi fino alla posizione salvata del consumatore (`store_consumer_offset` poi `delete_segments`): provato in 3 corse del solo bus e in 3 corse con il motore sotto carico e `kill -9`
   (nessuna posizione non applicata persa, zero accettate perse). Senza posizione salvata il server cancella ogni segmento chiuso: la posizione va salvata prima, sempre.
3. **Aggiornamento 0.9.0 → 0.10.0: due build del motore, nessun client parla con tutti e due i server** (matrice §A3.1). Procedura provata in 6 corse a 1.000 att./s con attività da 3 s (3 con esecuzioni da un passo, 3 da tre passi, queste con 3.067-3.068 `Done` riscritti sul bus nuovo): zero accettate perse, zero esecuzioni perse, pausa
   (segnale → motore nuovo pronto) 2,4-6,0 s (mediana 2,8 s), accettazioni ferme per 2,3-5,9 s dal punto di vista del client; circa 3.000 attività in volo si rifanno (eseguite due volte).

### A1. Stalli di Iggy a 10.000 att./s

#### A1.1 Che cosa si confronta

10.000 att./s = 1.000 esecuzioni/s da 10 passi = 21 eventi per esecuzione (un `Accepted` e dieci `Done` da circa 1.140 byte, dieci `Ready` da circa 80 byte): 21.000 messaggi/s, circa 13,3 MB/s di payload, su due topic (i due tenant).
Il motore li scrive con 4 connessioni (2 per topic) in richieste da 197 eventi (mediana, `noarc` r1; 214 in `full` r1), circa 103 richieste/s.

| Variante | Che cosa è | File |
| --- | --- | --- |
| `bench` | Iggy da solo (`bench-append`, 4 connessioni, 2 topic, mix 11 grandi + 10 piccoli ogni 21), 21.000 messaggi/s a scadenze fisse, richieste da **128** messaggi (164 richieste/s) | `iggy-sqlite-stall-bench-4-ms-r1.txt` |
| `bench-b204` | come `bench`, richieste da **204** messaggi (come il motore, 103 richieste/s) | `…-bench-b204-4-ms-r1.txt` |
| `bench-flush16m` | `bench`, **modifica 1**: soglie di scrittura dei segmenti 16 MiB e 65.536 messaggi (predefiniti 1 MiB e 1.024) | `…-bench-flush16m-4-ms-r1.txt` |
| `bench-gc5000` | `bench`, **modifica 2**: ritardo del group commit del WAL 5.000 µs (predefinito delle misure 500) | `…-bench-gc5000-4-ms-r1.txt` |
| `noarc` | motore con `DIAG_NOARCHIVE` (nessun payload nell'archivio, resta SQLite) | `…-noarc-4-ms-r1.txt` |
| `full` | motore completo | `…-full-4-ms-r1.txt`, `…-r2`, `…-r3` |

Le modifiche 1 e 2 sono le due sole modifiche di configurazione di Iggy (limite del piano). Una terza, la riserva dei segmenti all'apertura (`preallocate_segments`), era in programma ma la sequenza è stata cambiata prima di eseguirla e non è stata provata.
Tutte le corse partono da dati vuoti, 10 minuti, con il campionamento in sola lettura ogni 5 s di: pressione I/O di sistema, contatori di `nvme0n1`, byte scritti da ogni cgroup di container (`io.stat`), byte scritti da `system.slice` e `user.slice`,
temperatura del disco (sensore `Composite`, assente nelle prime due corse del bench) e `docker stats --no-stream` ogni 30 s. Nessun container o cgroup è stato toccato.

#### A1.2 Risultati per minuto

Throughput in messaggi/s (bench) o attività/s (motore), e p50 dell'append in ms (per richiesta nel bench, per gruppo nel motore):

| Min | bench msg/s | p50 | b204 msg/s | p50 | flush16m msg/s | p50 | gc5000 msg/s | p50 | noarc att/s | p50 | full r1 att/s | p50 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 20.975 | 9,0 | 20.322 | 19,0 | 20.971 | 9,0 | 20.971 | 8,7 | 8.963 | 72,8 | 8.943 | 75,7 |
| 2 | 20.998 | 9,2 | 21.641 | 19,6 | 21.001 | 8,8 | 18.206 | 29,9 | 10.003 | 72,4 | 10.001 | 78,5 |
| 3 | 21.001 | 9,2 | 20.893 | 22,0 | 20.905 | 8,8 | 19.556 | 24,2 | 10.003 | 71,5 | 10.009 | 80,2 |
| 4 | 21.001 | 9,2 | 21.111 | 17,4 | **12.693** | **35,3** | 20.081 | 22,7 | 10.001 | 73,1 | 10.000 | 79,6 |
| 5 | 21.001 | 9,2 | 20.998 | 21,7 | **12.331** | **35,4** | 20.834 | 22,3 | 9.897 | 72,8 | 9.990 | 78,1 |
| 6 | 19.723 | 9,8 | 21.002 | 16,6 | 14.227 | 35,1 | 18.709 | 26,1 | 10.104 | 72,1 | 10.012 | 80,8 |
| 7 | **13.259** | **35,3** | 21.002 | 22,7 | 14.647 | 34,9 | 18.537 | 29,3 | 9.999 | 72,7 | 9.988 | 80,3 |
| 8 | 13.011 | 35,3 | 20.886 | 16,9 | 14.257 | 34,8 | 18.933 | 26,1 | 10.002 | 71,9 | 10.006 | 79,5 |
| 9 | 14.694 | 35,0 | 21.114 | 24,0 | 14.671 | 34,9 | 20.548 | 22,3 | 9.996 | 73,0 | 10.005 | 80,0 |
| 10 | 14.705 | 34,8 | 20.577 | 19,1 | 14.142 | 35,1 | 19.951 | 23,3 | 9.990 | 72,4 | 9.995 | 80,4 |
| Totale | 17.988 | p99 65,2 | 20.972 | p99 64,9 | 15.960 | p99 81,3 | 19.624 | p99 45,0 | 10.000 | p99 154,5 | 9.996 | p99 186,9 |

Pavimento di `fdatasync` p50 prima > dopo (ms): bench 2,74 > 4,42; b204 4,37 > 4,27; flush16m 8,29 > 4,41; gc5000 4,63 > 4,41; noarc 4,28 > 4,45; full r1 4,62 > 4,53.
Ritardo di proiezione `lag_p50` del motore: 184-234 ms in tutti i minuti di `noarc`, `full` r1 e `full` r3 (p99 al massimo 685 ms al minuto 1 di r3, 488 ms dopo); la riga al minuto ora riporta anche i tempi del proiettore, p50
(lettura dal bus 0,6-0,7 ms, archivio 8,1-9,6 ms, transazione sul database 11,3-13,7 ms nei minuti normali).
Il pavimento prima di `full` r3 era 9,12 ms (il peggiore delle corse del motore) e non ha stallato.

**`full` r2 (la corsa con lo stallo)**, per minuto, `iggy-sqlite-stall-full-4-ms-r2.txt` (pavimento prima > dopo 2,78 > 4,63 ms):

| Min | att./s | append p50 gruppo (ms) | append p50 per richiesta (ms) | `lag_p50` / `lag_p99` (ms) | proiettore: lettura / archivio / database p50 (ms) | gruppi proiettati |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 8.957 | 23,9 | 9,0 | 154 / 275 | 0,6 / 5,5 / 8,5 | 1.223 |
| 2 | 10.000 | 34,5 | 9,0 | 162 / 292 | 0,6 / 5,4 / 8,5 | 1.363 |
| 3 | 8.571 | 35,4 | 9,1 | 190 / 2.429 | 0,6 / 5,8 / 8,7 | 1.151 |
| 4 | **4.154** | **500,7** | 119,9 | **6.853 / 16.581** | 1,3 / **62,9 / 98,0** | 497 |
| 5 | **6.139** | 193,4 | 46,1 | **17.203 / 31.916** | 1,0 / 18,5 / 44,4 | 744 |
| 6 | 17.073 | 110,4 | 29,8 | 6.083 / 9.683 | 0,9 / 13,2 / 32,2 | 2.125 |
| 7 | 13.662 | 94,3 | 24,1 | 1.958 / 9.200 | 0,9 / 10,5 / 25,1 | 1.848 |
| 8-10 | 10.005-9.995 | 78,7-79,6 | 17,9-18,6 | 216-226 / 365-461 | 0,7 / 8,4-8,8 / 12,8-13,1 | 1.310-1.324 |

Nel motore lo stallo ha prodotto anche rifiuti: su 605.000 offerte, 601.608 accettate, **1.991 rifiutate con `E_PROJECTION_DOWN`** (il proiettore ha fallito una transazione e il motore ha rifiutato lavoro nuovo, AD-2) e 1.401 errori di un tipo che lo script non registra
(il `engine.log` della corsa è stato sovrascritto dalla corsa successiva). Il più lungo intervallo senza accettazioni riuscite è 1.538 ms, il p99 di accettazione sull'intera corsa 1.234 ms (massimo 2.140 ms). Nelle corse di carico a 10.000 att./s non c'è un registro delle accettate,
quindi «zero accettate perse» qui non è verificato. Prima dello stallo (minuti 2-3) il p50 dell'append del gruppo era già salito da 24 a 35 ms, lo stesso valore del gradino del bench. Lo stesso schema (1.996-2.838 rifiuti, 1.666-3.043 errori) è nei file della fase 3 con lo stallo
(`iggy-sqlite-thr10k-{grpc-r1,grpc-r2,iroh-r1}-3-ms`), non in quello senza (`iroh-r2`).

#### A1.3 I/O, per minuto (`an4.sh`, sui campioni della stessa corsa)

| Corsa | Utilizzo disco | Scritture/s sul disco | MB/s sul disco | MB/s scritti dal container di Iggy | MB/s `user.slice` | Altri container | Temperatura |
| --- | --- | --- | --- | --- | --- | --- | --- |
| bench, min 1-5 (senza stallo) | 64-79 % | 4.163-4.881 | 76-93 | 38-52 | 0 | 0,00 | n.d. |
| bench, min 7-10 (stallo) | 88-97 % | 2.420-2.883 | 46-57 | 27-32 | 0 | 0,00 | n.d. |
| b204 (nessuno stallo) | 74-93 % | 2.441-2.783 | 50-60 | 27-38 | 0 | 0,00 | 48-50 °C |
| flush16m, min 4-10 (stallo) | 86-95 % | 2.365-2.996 | 44-59 | 25-34 | 0 | 0,00 | n.d. |
| gc5000 | 84-94 % da min 2 | 2.621-3.234 | 54-62 | 32-37 | 0 | 0,00 | 46-49 °C |
| noarc | 76-97 % (media 89 %) | 2.669-3.708 | 61-83 | 27-37 | 9,5-12,5 | 0,00 | 47-52 °C |
| full r1 | 74-97 % (media 89 %) | 3.237-4.141 | 77-99 | 28-35 | 19,5-29,9 | 0,00 | 49-53 °C |
| full r2, min 1-3 | 69-83 % | 5.146-6.190 | 105-128 | 43-55 | 18-22 | 0,00 | 47-50 °C |
| **full r2, min 4-5 (stallo)** | **100 %** | **1.084-1.347** | **32-34** | **10** | 14,5-14,9 | 0,00 | 51 °C |
| full r2, min 6-7 (recupero) | 90 % | 6.024-7.831 | 129-167 | 33-42 | 53-74 | 0,00 | 52-53 °C |
| full r2, min 8-10 | 89 % | 3.577-3.829 | 86-93 | 32-33 | 25-26 | 0,00 | 50-53 °C |
| full r3 | 81-95 % | 3.098-3.801 | 79-92 | 31-34 | 21-27 | 0,00 | 49-53 °C |

Attesa media per scrittura sul disco: 0,22-0,27 ms nei minuti senza stallo del bench, 0,55-0,70 ms in quelli con stallo (3,04 ms in un minuto di flush16m), 0,45-0,71 ms in b204, 0,83-1,52 ms in `noarc` e `full`.
Prova di capacità del disco (`disk-cliff-4-ms-r1.txt`, 45 blocchi da 1 GB scritti con `O_DIRECT` sul btrfs dello spike): 1,1-3,0 GB/s per 40 blocchi e 305-429 MB/s per 5 (blocchi 29-30 e 43-45, 2,4-3,4 s ciascuno): nessun calo duraturo,
quindi il disco non è limitato in banda a 50-90 MB/s.

#### A1.4 Cause

| Affermazione | Etichetta | Evidenza |
| --- | --- | --- |
| Iggy da solo, senza motore né SQLite né archivio, allo stesso ritmo di byte con richieste da 128 messaggi mostra uno **scalino di latenza**: append p50 da 9 a 35 ms (attesa per scrittura sul disco ×3), senza recupero fino a fine corsa; il bench a scadenze fisse (periodo 24,4 ms per 4 connessioni da 128) ne fa un calo di throughput da 21.000 a 12.300-14.700 messaggi/s e un ritardo sulla scadenza fino a 85-124 s. **Non è l'episodio del motore** (nessun recupero, nessuna attesa per scrittura da 22-33 ms, nessun rifiuto): il confronto è sui file di `full` r2 | **misurata** | `…-stall-bench-4-ms-r1.txt` (min 6-7), `…-bench-flush16m-…` (min 4); `gc5000` cala meno (18.200-20.800, p50 22-30 ms) |
| L'altezza dello scalino non dipende dalla configurazione provata, ma l'ordine delle corse è confuso con le modifiche: lo scalino è arrivato al minuto 7 (base), 4 (soglie a 16 MiB), 2 (group commit a 5 ms); p50 35 ± 0,5 ms con le soglie predefinite o a 16 MiB; con il group commit a 5 ms il p50 è 22-30 ms e il throughput regge al 93 %, ma la richiesta costa più dei 9 ms iniziali | **misurata** | file sopra |
| Con richieste da 204 messaggi, 103 richieste/s (periodo 38,9 ms) Iggy da solo regge 21.000 messaggi/s per 10 minuti (20.972, p99 65 ms); ma il p50 sta a 17-24 ms dal minuto 1, probabilmente già nello stato «dopo lo scalino» (un periodo di 38,9 ms assorbe anche 35 ms): non prova che lo scalino non ci sia | **misurata** (una corsa), lettura **ipotesi** | `…-bench-b204-4-ms-r1.txt` |
| Il motore tiene 10.000 att./s per 10 minuti senza cali in 3 corse su 5 (`noarc`, `full` r1, `full` r3: 9.996-10.036 att./s, p50 dell'append del gruppo 72-81 ms, ritardo di proiezione p50 184-234 ms); `full` r2 ha uno stallo di circa 3 minuti (9.957 att./s sull'intera corsa), con rifiuti `E_PROJECTION_DOWN` | **misurata** | `…-noarc-…-r1.txt`, `…-full-…-r{1,2,3}.txt` |
| Nello stallo del motore (`full` r2, minuti 4-5) il disco è al 100 % con 22,5-33,1 ms di attesa per scrittura (normale 0,25 ms, 1,0-1,4 ms nelle corse sane del motore), 1.084-1.347 scritture/s e 32-34 MB/s; il container di Iggy scrive 10 MB/s (43-55 prima); i container degli altri 0,00; temperatura 51 °C | **misurata** | `an4.sh` su `…-full-4-ms-r2.txt` |
| Nessun altro container è la fonte: i loro cgroup non scrivono (0,00 MB/s contando solo gli incrementi positivi, in ogni minuto di tutte le corse; due container dei runner si riavviavano di continuo e i loro contatori ripartivano, per questo `an4.sh` ignora i cali); riprova indipendente: `system.slice` meno il container di Iggy scrive 0,7-0,9 MB/s nei minuti di stallo di `full` r2 (al massimo 4,3 MB/s in un minuto qualunque di qualunque corsa, servizi di sistema compresi) e il totale sul disco nello stallo (32 MB/s) è più basso, non più alto, che nei minuti sani (86-128); la CPU massima di un container dei runner in un campione da 30 s è 0,03-10 % (blip a minuto 4,8 nel bench r1 e a 7,0 in `noarc`, senza byte scritti né relazione con gli stalli) | **misurata** | campioni `CG`, `DS` dei file delle corse; `an4.sh` |
| Nessun rallentamento termico: sensore `Composite` 46-53 °C in tutte le corse con la temperatura registrata (da gc5000 in poi) | **misurata** (il bench r1 e flush16m non la registravano) | righe `TEMP` |
| Il disco non ha un limite di banda a questo livello (1,1-3,0 GB/s sequenziali) | **misurata** | `disk-cliff-4-ms-r1.txt` |
| Durante lo scalino del bench il disco è saturo per tempo di servizio, non per banda: utilizzo 88-97 %, attesa per scrittura ×3, meno scritture/s (4,4k → 2,8k) e meno MB/s | **misurata** | `an4.sh` su `bench` e `flush16m` |
| Il peso sul disco per 13,3 MB/s di payload è 2,0-3,9×: 27-52 MB/s scritti dal cgroup di Iggy (WAL più segmenti più indici), 46-99 MB/s sul disco con btrfs e dm-crypt | **misurata** (il fattore); la scomposizione WAL/segmento/metadati **ipotesi** | `an4.sh` |
| L'episodio non è proprio di Iggy: con lo stesso motore (fase 2d), lo stesso disco e lo stesso carico, Valkey ha cali al minuto fino a 1.895-2.904 att./s in 2 corse su 4 da 10 minuti (`valkey-sqlite-thr10k-d-ms-r1`: minuti 6 e 8-10; `…-d1-ms-r1`: minuto 6) ed errori in 4 su 4 (129-24.450); Iggy nelle stesse condizioni ha un calo in 1 su 4 (`iggy-sqlite-thr10k-d-ms-r2`: minuti 7-8, 5.628-5.874) ed errori in 2 su 4 (953 e 2.897). Cambiare bus non toglie l'episodio; resta da isolare se la causa è il disco da solo o il motore che scrive archivio e SQLite sullo stesso disco | **misurata** (il confronto tra bus); la causa **ipotesi** | file della fase 2d, righe `MIN` e `offered=`; aggiunto nella revisione della sessione principale |
| La causa dello scalino e dello stallo è lo stato interno dell'SSD (un modello consumer senza DRAM, con Host Memory Buffer, sotto LUKS e btrfs) quando riceve molte fsync piccole: lo stato cambia da una corsa all'altra (pavimento prima della corsa 2,7-8,3 ms) e il gradino compare dopo 3-7 minuti | **ipotesi** | compatibile con i pavimenti e con il gradino, non isolata: non ho letto lo stato interno del disco né provato un altro disco |
| La differenza di incidenza (1 su 3 con il motore completo ora, 3 su 4 nella fase 3) viene dall'ora del giorno o dallo stato del disco; il pavimento prima della corsa non la spiega (stallo con 2,78 ms, nessuno con 9,12 ms) | **ipotesi** | nessuna lettura dei cgroup nella fase 3; il pavimento prima è in `[stato prima]` di ogni file |

**Il ritardo di 7-17 s è ritardo di proiezione?** Sì, **misurato** in `full` r2 (tabella sopra): nel minuto 5 `lag_p50` è 17.203 ms con append p50 per gruppo 193 ms e per richiesta 46 ms; l'attesa nella coda di scrittura (al più 4 gruppi) non spiega più di 1-2 s.
Il tempo manca dopo la scrittura durevole: nel proiettore l'archivio (fsync del segmento) passa da 5,5 a 62,9 ms e la transazione sul database (SQLite, `synchronous=FULL`) da 8,5 a 98,0 ms (p50, ×11-12), la lettura dal bus resta a 1,0-1,3 ms, e i gruppi proiettati al minuto
scendono da 1.363 a 497. Il proiettore è sequenziale per tenant (archivio poi database): con 160 ms per gruppo regge circa 6 gruppi/s per tenant contro i 11 che servono (1.320/min per i due tenant = circa 11/s), quindi il ritardo cresce finché il disco non torna normale e poi si
recupera (17.073 att./s nel minuto 6). Il disco che rallenta colpisce insieme l'append su Iggy, l'archivio e SQLite, che stanno tutti sullo stesso btrfs. Per il criterio «database lento ma attivo, proiezione oltre 1 s»: qui il motore ha continuato ad accettare con la
proiezione a 2-32 s di ritardo (`lag_p99`) per circa 3 minuti, con 1.991 rifiuti `E_PROJECTION_DOWN` e 1.401 errori (sopra); non ho confrontato con AD-2 riga per riga (è il blocco E).

**Esito del criterio.** Gli stalli non sono eliminati da una configurazione di Iggy: nello stallo il disco è saturo per tutto ciò che ci scrive (misurato), Valkey con lo stesso motore sullo stesso disco ha gli stessi cali (misurato), perché il disco si fermi resta un'ipotesi; le due modifiche, provate solo sul bench e giudicate contro lo scalino di latenza (non contro l'episodio, che nel bench non si è riprodotto), non lo toccano; per la regola di fase mi fermo qui e riporto i numeri. A 10.000 att./s con il motore il disco sta all'89 % di utilizzo e
una volta su tre (`full`), o tre su quattro nella fase 3, si ferma per qualche minuto. Non ho aggirato Iggy nel motore. Le modifiche di configurazione non sono state provate con il motore (una sola corsa di stallo osservata, incidenza 1 su 3: una corsa sola non distinguerebbe).

#### A1.5 Corse del motore completo ripetute (incidenza)

Tre corse del motore completo da 10 minuti con il campionamento dell'I/O, dati vuoti, SQLite, 10.000 att./s:

| Corsa | Pavimento prima > dopo (ms) | att./s sull'intera corsa | Stallo | Accettate / offerte | Rifiutate `E_PROJECTION_DOWN` | Errori | p99 accettazione (ms) | `lag_p50` massimo (ms) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| r1 | 4,62 > 4,53 | 9.996 | no | 605.000 / 605.000 | 0 | 0 | 247 | 234 |
| r2 | 2,78 > 4,63 | 9.957 | **sì, minuti 4-7** | 601.608 / 605.000 | 1.991 | 1.401 | 1.234 | 17.203 |
| r3 | 9,12 > 4,48 | 10.036 | no | 605.001 / 605.001 | 0 | 0 | 273 | 228 |

Più `noarc` r1 (nessuno stallo, 605.001 / 605.001, p99 205 ms). Quindi **1 corsa su 4 con il motore in questa fase**, contro 3 su 4 nella fase 3 (`thr10k` gRPC r1, r2, iroh r1): incidenza non stimabile con così poche corse. Il confronto con la fase 3 non mostra un miglioramento: qui `noarc` e `full` girano con `trim=1`, che per Iggy ora cancella davvero i segmenti (nella fase 3 non faceva nulla). Nessuna perdita durante lo stallo **non è verificata**: a 10.000 att./s manca il registro delle accettate e il tipo dei 1.401 errori di `full` r2 non è stato registrato; entrambi passano al blocco B.

### A2. Conservazione legata alla posizione applicata

#### A2.1 Primitive di Iggy 0.10.0 (verificate sul sorgente del tag `server-0.10.0` e sul crate `iggy` 0.12.0)

| Primitiva | Che cosa fa | Dove è documentata |
| --- | --- | --- |
| `delete_segments(stream, topic, partition, segments_count)` | cancella i **primi N segmenti chiusi** della partizione; il segmento attivo non si cancella mai; **si ferma alla posizione più bassa salvata da qualunque consumatore o gruppo**, quindi può togliere meno di N. Ritorna dopo il commit dei metadati, il file sul disco sparisce al passo successivo del server | `core/common/src/traits/segment_client.rs`; scenari `core/integration/tests/server/scenarios/delete_segments_scenario.rs` («barrier blocks deletion», «segment_deletable(seg_end, barrier)») |
| `store_consumer_offset(consumer, stream, topic, partition, offset)` | salva la posizione del consumatore: è la **soglia**. Un segmento è cancellabile quando la sua ultima posizione è ≤ della soglia | stessi scenari |
| `purge_topic` | cancella tutto il topic (non serve); esiste nel crate 0.11.0, **non** nel 0.12.0 (nessuna corrispondenza nelle sorgenti del client) | `topic_client.rs` del crate 0.11.0 |
| `message_expiry` e `max_topic_size` del topic | cancellazione per età o dimensione dal pulitore di segmenti (attivo solo con scadenza o dimensione finite); nelle sorgenti lette (`segment_cleaner.rs`, `config.toml`) non c'è alcun riferimento ai consumatori e la cancellazione vera sta nel pump dello shard, non letto: **ipotesi** che non ne rispetti la soglia, quindi non adatte (cancellerebbero posizioni non applicate) e non usate | `core/server/src/segment_cleaner.rs` |
| nessuna cancellazione per offset | non esiste: la granularità è il segmento (conferma della fase 2) | |

Senza alcuna posizione salvata non c'è soglia e `delete_segments` cancella ogni segmento chiuso (controllo negativo sotto). **Granularità**: un segmento. La dimensione è per topic (`segment_size`, tra 1 MiB e 1 GiB, 1 GiB di default); è un limite morbido: un segmento si chiude con la richiesta che lo supera
(60 messaggi da 20 KB = 1,2 MB con il limite a 1 MiB).

#### A2.2 Come è fatto (`Bus::trim` per Iggy, `src/bus.rs`)

Il proiettore, dopo aver confermato la transazione sul database (posizione applicata `pos` = la prossima da leggere), ogni 500 ms chiama `trim(topic, pos)`: `store_consumer_offset(.., pos - 1)` (l'ultima applicata) e subito `delete_segments(.., u32::MAX)`.
Si cancellano così solo segmenti la cui ultima posizione è ≤ `pos - 1`: tutti interamente applicati. Con `pos = 0` non si fa nulla. La regola è provata dal motore stesso: ogni lettura del proiettore deve partire esattamente dalla sua posizione,
altrimenti esce con `E_POSIZIONE_PERSA` (una posizione non applicata cancellata o saltata).

#### A2.3 Prova sul solo bus (`retention-proof`, 3 corse, segmenti da 1 MiB)

600 messaggi da 20 KB in 11 segmenti. Ogni riga: posizione del proiettore passata a `trim`, primo offset ancora leggibile, segmenti rimasti, e se da lì alla testa gli offset sono contigui con il payload uguale all'indice scritto
(identica nelle tre corse: `iggy-sqlite-retproof-4-ms-r{1,2,3}.txt`):

| `trim(pos)` | soglia salvata | primo offset leggibile | segmenti | regola «primo ≤ pos, contigui» |
| --- | --- | --- | --- | --- |
| nessuna chiamata (pos = 0) | nessuna | 0 | 11 | ok |
| 1 | 0 | 0 | 11 | ok |
| 75 | 74 | 60 | 10 | ok |
| 153 | 152 | 120 | 9 | ok |
| 300 | 299 | 300 | 6 | ok |
| 301 | 300 | 300 | 6 | ok |
| 450 | 449 | 420 | 4 | ok |
| 599 | 598 | 540 | 2 | ok |
| 600 (tutto applicato) | 599 | 600 (niente di leggibile) | 1 | ok |

Violazioni: 0 in 3 corse su 3. **Controllo negativo** (stesso file, riga finale): su un topic senza alcuna posizione salvata, `delete_segments(u32::MAX)` porta i segmenti da 11 a 1 e il primo offset leggibile a 600 su una testa di 600:
senza soglia si perde tutto ciò che è chiuso. Per questo `trim` salva la posizione prima, e per questo un topic nuovo non va mai pulito prima del primo salvataggio.

#### A2.4 Prova con il motore sotto carico e `kill -9` (`scen.sh … ret`, 3 corse)

300 esecuzioni/s da 10 passi (6.300 eventi/s), `trim=1`, segmenti da 1 MiB, 90 s; a 40 s un `kill -9` del motore e riavvio; verifica finale come nelle fasi precedenti.

| Corsa | Accettate | Errori durante il riavvio (senza risposta) | Accettate perse | `E_POSIZIONE_PERSA` | Segmenti in coda | Pavimento prima > dopo (ms) |
| --- | --- | --- | --- | --- | --- | --- |
| r1 | 26.974 | 325 | **0** | 0 | 1 + 1 | 8,01 > 2,85 |
| r2 | 27.257 | 42 | **0** | 0 | 1 + 1 | 2,78 > 4,37 |
| r3 | 27.258 | 41 | **0** | 0 | 1 + 1 | 7,88 > 4,54 |

Prima del kill ogni topic teneva 2-3 segmenti (1.538-4.330 messaggi) contro i 100.000 e più scritti; la soglia del consumatore stava 900-3.300 posizioni sotto la testa (ritardo di proiezione); alla fine 1 segmento per topic.
Esecuzioni concluse contro accettate riconosciute: r1 27.066 / 26.974 (92 in più), r2 27.264 / 27.257 (7), r3 27.264 / 27.258 (6): una parte delle richieste rimaste senza risposta nel `kill -9` era già durevole e il motore nuovo l'ha eseguita (nessuna perdita;
esecuzioni che il client non conosce e che ritenterebbe: l'idempotenza dell'accettazione non è nello spike). Tutti i payload di uscita si decifrano (270.660, 272.640, 272.640), `done_events != 1`: 0. Il primo `verify` di ogni corsa dice «FALLITA» perché parte mentre le ultime attività sono ancora in corso; il ciclo di ripetizione dello script lo rifà fino a «OK»
(vedi i file). Non copre: crash di Iggy con la posizione del consumatore da recuperare (blocco B), più consumatori sullo stesso topic (la soglia è il minimo, per costruzione del server, non provato qui).

### A3. Aggiornamento senza perdite, Iggy 0.9.0 → 0.10.0

#### A3.1 Quale client parla con quale server (`iggy-compat-4-ms-r1.txt`)

Controllato sul registro (`https://crates.io/api/v1/crates/iggy/versions`, 2026-11-24): `iggy` **0.11.0** è del 2026-09-18, lo stesso giorno dell'immagine `apache/iggy:0.9.0`; `iggy` **0.12.0** è del 2026-11-07 (server 0.10.0). Prova a quattro combinazioni:

| Server | Client `iggy` 0.11.0 | Client `iggy` 0.12.0 |
| --- | --- | --- |
| `apache/iggy:0.9.0` | **funziona** (200 msg/s, p50 51 ms con richieste da 10) | **non si collega**: `InvalidFormat` |
| `apache/iggy:0.10.0` | **non si collega**: `IncompatibleProtocolVersion(11264, 12290, 12290)` | **funziona** (697 msg/s, p50 14 ms con richieste da 10) |

**Scelta: due build del motore** (client 0.11.0 per il motore vecchio, 0.12.0 per il nuovo), costruite dallo stesso sorgente (`--features old` aggiunge la dipendenza `iggy` 0.11.0, pubblicata da oltre due settimane); non esiste un periodo in cui
un solo motore parla con tutti e due i server, quindi l'aggiornamento di Iggy e quello del motore sono un'operazione sola, con il motore fermo in mezzo. La procedura è comunque quella del cambio verso un bus nuovo e vuoto (il motore non legge dati vecchi dal server vecchio):
è la stessa per ogni salto che cambia il formato dei dati, anche con lo stesso client.

#### A3.2 Procedura e realizzazione

1. **Rifiuto e svuotamento.** `SIGUSR1` al motore vecchio: le accettazioni si rifiutano con `E_UPGRADE_DRAIN` (`UNAVAILABLE`), non si prende più niente dalla coda, i gruppi in volo si fanno durevoli, e quando nessun gruppo è in volo e la posizione applicata
   (nel database) è uguale alla testa del bus su ogni topic il motore aspetta 300 ms (le risposte già durevoli escono dal socket) ed esce (`DRAINED`). Le consegne non si fanno più; le attività in volo restano dai worker.
2. **Nuovo Iggy** 0.10.0 su una directory dati vuota (`reset.sh`: `compose down`, svuotamento, `compose up`).
3. **Generazione del bus.** Il bus porta la sua generazione in un topic `meta` (un messaggio, scritto alla creazione del bus vuoto con `busgen=`); la tabella `projection_pos` ha ora la colonna `gen`, scritta con ogni posizione applicata.
   All'avvio il motore confronta le due: uguali = ripresa normale; database vecchio e bus più nuovo **con** `upgrade=1` = riversamento; ogni altro caso = `E_BUS_GENERATION` e il motore non parte (prove di rifiuto sotto).
4. **Riversamento dello stato vivo.** Il motore nuovo (`busgen=2 upgrade=1`) legge dal database le esecuzioni non concluse con i passi emessi e conclusi, ne riprende i payload dall'archivio, riscrive sul bus nuovo `Accepted`, `Ready`, `Done`
   (in richieste da 256), e in **una transazione** porta le posizioni salvate alla generazione 2 subito dopo gli eventi riscritti (il database ne rappresenta già lo stato: riapplicarli conterebbe due volte i `Done`). Se il riversamento si interrompe,
   al riavvio i topic si svuotano e si ricomincia (realizzato nel codice, **non provato**: nessuna corsa lo ha interrotto). Poi la ripresa normale: le attività emesse e non concluse si riconsegnano con un nuovo tentativo, le decisioni mancanti si scrivono.
5. **Ripresa.** I worker si ricollegano da soli; il generatore di carico riprende a essere accettato.

Carico: 1.000 esecuzioni/s da un'attività da 3 s (`kind=0 n=1 work_ms=3000`, tetto per tenant 4.096), 70 s, due tenant; l'aggiornamento parte a 20 s. Circa 3.000 attività sono in volo in ogni istante.

#### A3.3 Risultati (3 corse, `iggy-sqlite-upgrade-4-ms-r{1,2,3}.txt`)

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 6,38 > 2,67 | 6,90 > 2,60 | 4,86 > 2,81 |
| Accettate / offerte | 67.523 / 72.000 | 69.634 / 72.000 | 69.265 / 72.000 |
| Rifiutate con `E_UPGRADE_DRAIN` | 531 | 581 | 537 |
| Errori di trasporto (motore giù, **non** accettate) | 3.946 | 1.785 | 2.198 |
| **Accettate perse** | **0** | **0** | **0** |
| Esecuzioni concluse = accettate | 67.523 | 69.634 | 69.265 |
| `done_events != 1`, duplicati sul bus nuovo, payload illeggibili | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 |
| Esecuzioni vive riversate | 3.149 | 3.143 | 3.141 |
| Attività in volo rifatte (eseguite due volte) | 3.003 | 3.008 | 2.958 |
| Svuotamento (segnale → motore vecchio uscito) | 565 ms | 616 ms | 567 ms |
| Nuovo Iggy avviato (da fine svuotamento) | 1.130 ms | 1.068 ms | 1.127 ms |
| Motore nuovo fino a `READY` (di cui riversamento 266 / 285 / 299 ms) | 2.869 ms | 723 ms | 1.027 ms |
| **Pausa: segnale → `READY` del motore nuovo** | **4.563 ms** | **2.407 ms** | **2.721 ms** |
| Segnale → prima attività conclusa dopo la ripresa | 7.634 ms | 5.455 ms | 5.819 ms |
| Il più lungo intervallo senza accettazioni riuscite (client) | 4.327 ms | 2.283 ms | 2.577 ms |

Criterio **superato**: zero accettate perse, zero esecuzioni in corso perse (ogni accettata è conclusa una sola volta nella proiezione, con il payload che si decifra). Il client vede le accettazioni ferme 2,3-4,3 s; la prima attività conclusa dopo la ripresa
arriva 3,05-3,10 s dopo `READY`, che è la durata dell'attività (le 3.000 in volo si rifanno da capo). La soglia della pausa la decide Marco; i numeri per decidere sono, con esecuzioni da un passo, **2,4 / 2,7 / 4,6 s** (con tre passi 2,6 / 2,8 / 6,0 s), dominati da nuovo Iggy (1,1 s, comprende il `compose`)
e dall'avvio del motore (0,7-2,9 s; la causa della differenza non è misurata, **ipotesi**: la creazione dei topic sul server nuovo, che in una corsa del blocco A2 ha preso 7,8 s al primo avvio).

**Con esecuzioni a metà catena** (le tre corse sopra, con `n=1`, riscrivono 0 `Done`: lo stato vivo aveva solo `Ready`). Tre corse in più, `iggy-sqlite-upgrade-n3-4-ms-r{1,2,3}.txt`: 334 esecuzioni/s da **tre** passi da 3 s (1.002 attività/s, 9 s per esecuzione, circa 3.000 esecuzioni vive),
così lo stato vivo comprende passi già conclusi:

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,85 > 4,55 | 4,61 > 4,51 | 4,53 > 4,97 |
| Accettate / offerte; rifiutate `E_UPGRADE_DRAIN`; errori di trasporto | 22.057 / 24.048; 152; 1.839 | 23.118 / 24.048; 195; 735 | 23.182 / 24.048; 165; 701 |
| **Accettate perse** | **0** | **0** | **0** |
| Esecuzioni concluse = accettate | 22.057 | 23.118 | 23.182 |
| `done_events != 1`, duplicati sul bus nuovo, payload illeggibili | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 |
| Esecuzioni vive riversate / `Ready` / **`Done` riscritti** | 3.079 / 6.147 / **3.068** | 3.072 / 6.139 / **3.067** | 3.071 / 6.139 / **3.068** |
| Riversamento (database e archivio → bus nuovo) | 864 ms | 512 ms | 495 ms |
| Attività in volo rifatte (eseguite due volte) | 3.079 | 3.072 | 3.071 |
| Svuotamento / nuovo Iggy / motore nuovo fino a `READY` | 463 / 1.941 / 3.585 ms | 616 / 1.145 / 1.030 ms | 514 / 1.149 / 924 ms |
| **Pausa: segnale → `READY`** | **5.989 ms** | **2.791 ms** | **2.587 ms** |
| Segnale → prima attività conclusa | 9.170 ms | 5.857 ms | 5.742 ms |
| Il più lungo intervallo senza accettazioni riuscite (client) | 5.928 ms | 2.738 ms | 2.554 ms |

Criterio **superato anche qui**: i `Done` riscritti (3.067-3.068, uno per ogni esecuzione a metà catena) sono sul bus nuovo e tutte le esecuzioni si concludono con i payload che si decifrano. Il primo `verify` di ogni corsa dice «FALLITA» finché le ultime esecuzioni da 9 s non finiscono
(il ciclo lo ripete fino a «OK»). Le corse lente sono le r1 di entrambe le serie (4,6 e 6,0 s): nuovo Iggy + avvio del motore 1,1 + 2,9 s (n=1) e 1,9 + 3,6 s (n=3), contro 1,1 + 0,7-1,0 s nelle altre quattro; la causa (primo avvio di Iggy dopo una pausa del disco, creazione dei topic) è **ipotesi**, non isolata.

**Prove di rifiuto della generazione** (stesso file, fine corsa, 3 su 3 in entrambe le serie): bus nuovo vuoto di generazione 3 contro posizioni di generazione 2 senza `upgrade=1`: `E_BUS_GENERATION`, codice di uscita 5, il motore non parte; bus vuoto di generazione 1 con `upgrade=1`
(più vecchio del database): stesso esito.

**Non coperto o scelto così.** (a) Timer in sospeso: non esistono ancora (blocco C): il riversamento copre esecuzioni e passi, un timer andrà riscritto allo stesso modo come evento. (b) Il motore non aspetta le attività in volo: si rifanno (3.000 eseguite due volte a questo carico);
aspettarle costerebbe una pausa in più pari alla durata delle attività (3 s qui), non misurata. (c) Il vecchio dato di Iggy 0.9.0 non si conserva: la directory si svuota (in produzione si terrebbe da parte per il ritorno indietro). (d) Una richiesta di accettazione in corso nel momento
dell'uscita non riceve risposta (errore di trasporto, il client non la considera accettata); nessuna esecuzione senza riconoscimento è stata eseguita (concluse = accettate in tutte le corse).

## Deviazioni dal piano

- Il ramo di lavoro partiva da `main`: l'ho riportato a `cd89dcd` (`spike/bus`, usato da un altro worktree) prima di cominciare.
- **A1, ordine ed elenco delle corse.** Il piano chiede tre corse (Iggy da solo, `noarc`, completo); ne ho fatte otto più la prova del disco: `bench`, `bench-flush16m`, `bench-gc5000`, `bench-b204` (aggiunta dopo aver visto che le richieste del bench erano più piccole e più frequenti di quelle del motore: 164/s da 128 contro 103/s da 197-214),
  `noarc`, `full` r1-r3, più la prova del disco `disk-cliff`. Ogni variante del bench una sola volta: non so quanto spesso stallino. La corsa con la riserva dei segmenti non è stata fatta (sostituita dalla modifica del group commit prima di partire).
- **Le due modifiche di configurazione sono provate solo sul bench**, non sul motore: lo stallo del motore è comparso una volta su tre corse, una corsa per variante non distingue. Nessuna modifica è adottata.
- **Il tipo dei 1.401 errori** di `full` r2 non è noto (lo script non conserva il `engine.log` e la corsa a 10.000 att./s non ha un registro delle accettate): è un difetto della strumentazione, non rifatto per tempo.
- **Campioni dell'I/O nello stesso file della corsa** (righe `T`, `DEV`, `TEMP`, `CG`, `SL`, `DS`) e la mappa id-nome dei container in `<file>.io.names`; nella prima corsa l'id del container di Iggy non era nella mappa e l'ho aggiunto a mano dopo la corsa (poi la mappa si rinnova a ogni giro). Il campionamento della temperatura manca nelle prime due corse del bench.
- **A2:** «segmenti interamente sotto la posizione applicata più bassa» è garantito dal server (soglia del consumatore) e verificato con il proiettore del motore che esce se la lettura non parte dalla sua posizione; con un solo consumatore (il proiettore) la posizione più bassa è la sua. Più consumatori non sono provati.
- **A3:** la directory dati «nuova e vuota» è la stessa svuotata da `reset.sh`; la pausa comprende il `compose down/up`. Il motore vecchio è lo stesso sorgente compilato con il client 0.11.0, non una versione vecchia del codice del motore.
- Prove funzionali sul desktop (non misurate, non conservate): `retention-proof`, la scena `ret` e l'aggiornamento con tempi ridotti, per mettere a punto gli script mentre minisheep era occupato.
- Le righe `verify` che dicono «FALLITA» in A2 e A3 sono il primo tentativo dello script, che parte mentre le ultime attività sono in corso; il ciclo lo ripete fino a «OK».
- **`trim=1` nelle corse del motore** (`noarc`, `full`, come `scen.sh` fa sempre fuori da `kill` e `dbdown`): con Iggy il `trim` ora non è più un'operazione vuota (salva la posizione del consumatore ogni 500 ms per topic e chiede `delete_segments`, quasi mai efficace con segmenti da 1 GiB);
  la fase 3 non aveva questo carico. Confonde il confronto «1 su 4 ora, 3 su 4 nella fase 3». L'aggiornamento (A3) gira con `trim=0`.
- **Dipendenze:** `Cargo.lock` guadagna solo `iggy`, `iggy_binary_protocol`, `iggy_common` 0.11.0 (Apache-2.0, pubblicate il 2026-09-18, oltre due settimane prima), versioni esatte nel `Cargo.toml`.
- **Letture della configurazione:** le soglie di scrittura dei segmenti sono rilette dal topic (`BUS_PRINT_TOPIC`: `messages_required_to_save` 65.536, `size_of_messages_required_to_save` 16 MiB, `explicit: true`, `segment_size` 1 GiB di default); il ritardo del group commit è letto
  dall'ambiente del container (`IGGY_PARTITION_WAL_GROUP_COMMIT_DELAY_MICROS=5000`), il valore effettivo nel server non è stato riletto. Le letture sono state fatte sul desktop con le stesse variabili.
- **Esecuzione:** le sequenze sono partite in background con `nohup` mentre sul desktop compilavo e provavo gli script; i binari sono stati sostituiti su minisheep tra una corsa e l'altra (17:26 e 17:42, ora di minisheep: il secondo aggiunge i tempi del proiettore e non cambia il
  comportamento), quindi `bench` e `bench-flush16m` girano con il primo e le altre con il secondo.
- Tempo: A1 circa 2 ore e mezza tra corse e attese, dentro le 3 ore; A2 e A3 circa 1 ora di codice e prove sul desktop, 20 minuti di corse su minisheep.

## Comandi per rifare

Su minisheep, in `~/ttspikebus` (`env.sh`; `.env` da `./gen-env.sh`; i due binari `bin/busspike` e `bin/busspike-old`, quest'ultimo costruito con `cargo build --release --features old`):

| Misura | Comando |
| --- | --- |
| Stalli, Iggy da solo | `./a1-run.sh bench 1` (`CFG=b204 ./a1-run.sh bench 1 batch=204`; `IGGY_SAVE_BYTES=16777216 IGGY_SAVE_MSGS=65536 CFG=flush16m …`; `IGGY_GC_DELAY=5000 CFG=gc5000 …`) |
| Stalli, motore | `./a1-run.sh noarc 1`, `./a1-run.sh full 1` |
| Tabella per minuto dell'I/O | `./an4.sh risultati/<file>.txt` |
| Disco | `./ms-run.sh risultati/disk-cliff-4-ms-r1.txt ./disk-cliff.sh 45` |
| Conservazione (bus solo) | `./ms-run.sh risultati/iggy-sqlite-retproof-4-ms-r1.txt ./ret-proof.sh` |
| Conservazione (motore, `kill -9`) | `IGGY_SEGMENT_SIZE=1048576 ./ms-run.sh risultati/iggy-sqlite-ret-4-ms-r1.txt ./scen.sh iggy sqlite ret RSECS=90` |
| Compatibilità client-server | `./ms-run.sh risultati/iggy-compat-4-ms-r1.txt ./compat.sh` |
| Aggiornamento | `CAP=4096 ./ms-run.sh risultati/iggy-sqlite-upgrade-4-ms-r1.txt ./up-run.sh` |
| Sequenze usate | `a1-chain.sh` (interrotta dopo `flush16m`), `a1-chain2.sh`, `a1-chain3.sh`, `a23-chain.sh`, `a3-chain-n3.sh` |


## Blocco B

Guasti e ripresa (NFR-6, NFR-15). Cinque casi, 3 corse su SQLite ciascuno e una su Postgres dove il caso tocca il database. Binario unico per tutto il blocco (sha256 `d51b36de4ca9c6dc`, stampato in ogni file grezzo), nessuna sostituzione a metà serie.
Minisheep, stessi 4 CPU (motore e Iggy su `1-4`, carico su `8-11`), `ms-run.sh` con pavimento di `fdatasync` prima e dopo. File grezzi `risultati/iggy-<db>-<caso>-4-ms-r<n>.txt`. Ogni causa è **misurata** (con il file) o **ipotesi**.
Tutti i guasti qui sono crash di **processo** (`kill -9`, `docker kill`) o esaurimento di spazio: non sono spegnimenti dell'host, e la cache del sistema operativo sopravvive. Lo spegnimento improvviso dell'host non è misurato (minisheep ospita i runner).

### Sintesi

1. **Crash del motore: superato in 4 corse su 4** (3 SQLite, 1 Postgres). `kill -9` → `READY` in 94-112 ms, → prima attività conclusa in 217-222 ms (soglia NFR-15: 30 s). Zero accettate perse. I worker si ricollegano da soli e consegnano gli esiti: 2.688-2.719 attività su 2.710-2.722 in volo
   si chiudono con l'esito finito durante la pausa (`late_ok`), le altre 3-25 scadono e si rifanno. **Nessun tentativo eseguito due volte** (registro dei worker: 27.792-27.816 inizi, tutti distinti): le scadute non erano mai arrivate al worker (misurata, v. sotto).
2. **Crash di Iggy: superato in 4 corse su 4**, ma non come il piano lo immaginava. Il client `iggy` 0.12.0 **non torna con un errore** quando il server muore: la chiamata aspetta e si risolve da sola (8,4 s in una prova). Il motore rifiuta lavoro nuovo con `E_BUS_DOWN` entro 1,9-2,0 s dal kill (sonda
   periodica) e si ricollega da solo; `READY` non vale qui (il motore non riparte): kill → bus di nuovo raggiungibile 8,4-13,7 s (5 s di pausa voluta prima di `docker start`, poi 3,1-8,5 s di avvio di Iggy e ricollegamento del client). Zero accettate perse, zero duplicati sul bus.
3. **Crash di un worker: superato in 3 corse su 3.** L'attività torna disponibile alla scadenza del tentativo (consegna + `work_ms` 3 s + margine 2 s = 5 s): 2,0-5,1 s dopo il kill, mediana 3,5-3,6 s. 1.347-1.355 attività rifatte, tutte e sole le scadute, zero esiti doppi.
4. **Disco pieno: superato per database e archivio** (6 corse + 3 con il solo database): rifiuto con `E_STORAGE_FULL` entro 0,8-7,6 s dalla zavorra, ripresa 5-392 ms dopo che lo spazio è tornato, zero accettate perse, ogni payload si decifra.
   **Per i dati di Iggy: Iggy non rifiuta, si ferma.** A disco pieno il server esce (v. §B4.1); il motore lo vede come bus giù (`E_BUS_DOWN`, **non** un codice di disco pieno) e, con un ciclo che rilancia Iggy (rilancio del processo, non della macchina), riprende 1,7-10,9 s dopo che lo spazio torna. Zero accettate perse, stato precedente intatto.
   Il criterio «codice esplicito» è superato solo in senso lato (`E_BUS_DOWN` è esplicito ma non dice «disco»); la causa è in Iggy (**misurata**), nessuna configurazione provata la cambia: **da riportare a Marco**.
5. **Pause del disco a 10.000 att./s: in 3 corse da 10 minuti su 3 il criterio di stallo, scritto prima delle corse, segna almeno un minuto sotto soglia** (v. §B5); zero accettate perse in tutte e tre. Il registro delle accettate e il tipo di ogni errore sono ora permanenti.

### Cosa è cambiato nel motore (codice dello spike, per le prove)

- **Attesa dell'esito oltre la sessione.** Il worker tiene gli esiti finiti in una cassetta che sopravvive alla sessione, li rimanda a ogni ricollegamento e li toglie solo con la conferma durevole del motore (nuovo campo `ack` di `Delivery`).
  Il motore accetta l'esito da qualunque sessione se porta la consegna corrente, o se la consegna è di prima della ripresa; gli altri li scarta e li conferma.
- **Scadenza del tentativo come unico modo di tornare disponibile.** Ogni consegna ha una scadenza (`work_ms` + 2.000 ms, in memoria); un passaggio ogni 100 ms rimette in coda le scadute. Una sessione persa non rimette più in coda le sue consegne (prima sì, subito). Dopo una ripresa le attività emesse e non concluse restano «in volo» (consegna 0)
  fino a `work_ms` + 2.000 ms dalla ripresa, invece di essere riconsegnate subito (**cambia anche il comportamento dell'aggiornamento del blocco A**, che non è stato rimisurato).
- **Bus giù.** Una sonda ogni 500 ms (tempo massimo 1,5 s) e una guardia sulle append (5 s) alzano un indicatore; con l'indicatore alzato le accettazioni tornano `E_BUS_DOWN`. L'append non si annulla mai (annullarla lascia un'operazione incerta che blocca la partizione, §B2.1).
- **Disco pieno nel proiettore.** Gli errori dell'archivio (prima `unwrap`) e del database passano dallo stesso ciclo di ripetizione; se il testo dice spazio finito le accettazioni tornano `E_STORAGE_FULL`. Il segmento in scrittura si abbandona a un errore, così gli scostamenti dei payload non slittano.
- **Generatore di carico:** tipo (codice e messaggio) di ogni risposta non riuscita; `verify light=1` per le corse da 10.000/s.

### B1. Crash del motore con attività lunghe in volo

300 esecuzioni/s da 3 passi da 3 s (900 att./s, circa 2.700 in volo), due tenant, 30 s; a 12 s `kill -9` del motore e riavvio immediato (nessuna attesa voluta, a differenza della fase 2). Un solo worker (4 sessioni) sempre vivo.

| Misura | SQLite r1 | SQLite r2 | SQLite r3 | Postgres r1 |
| --- | --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 5,12 > 1,92 | 6,73 > 2,76 | 2,90 > 2,68 | 1,92 > 2,93 |
| **kill → `READY`** (ms) | **109** | **112** | **106** | **94** |
| **kill → prima attività conclusa** (ms) | **217** | **222** | **220** | **217** |
| Attività in volo alla ripresa | 2.713 | 2.722 | 2.712 | 2.710 |
| Esiti arrivati dopo il ricollegamento (`late_ok`) | 2.688 | 2.719 | 2.705 | 2.701 |
| Scadute e rifatte (`expired`) | 25 | 3 | 7 | 9 |
| Esiti scartati come doppioni (`rejected`) | 17 | 0 | 7 | 6 |
| Accettate (ledger) / concluse nella proiezione | 9.264 / 9.265 | 9.261 / 9.264 | 9.264 / 9.266 | 9.268 / 9.272 |
| **Accettate perse** | **0** | **0** | **0** | **0** |
| Tentativi iniziati / distinti / eseguiti due volte | 27.795 / 27.795 / 0 | 27.792 / 27.792 / 0 | 27.798 / 27.798 / 0 | 27.816 / 27.816 / 0 |
| `done_events != 1`, duplicati sul bus, payload illeggibili | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 |

Criterio **superato** (ripresa 30 s, zero accettate perse, nessun tentativo doppio). Il tempo di `READY` (94-112 ms) è quasi tutto la ripresa del motore (`recovered_ms` 90-103).

**Concluse > accettate** di 1-4 per corsa: sono esecuzioni accettate dal motore la cui risposta è andata persa nel `kill` (il client ha visto un errore di trasporto e non le ha messe nel registro); eseguite, non perse. **Misurata**: `errors_by_type` ha 3-8 `Unknown: transport error` per corsa.

**Perché le scadute non sono doppie (misurata, `expired + late_ok` = in volo alla ripresa, in tutte e quattro le corse):** erano consegne scritte sul socket dal motore morto e mai lette dal worker; non hanno una riga nel registro dei worker, quindi il primo tentativo è quello rifatto. **Gli scartati** (`rejected`) sono esiti rimandati dal worker per una conferma andata persa con la
sessione e già accettati. Un tentativo che girasse due volte non c'è stato in nessuna corsa; il meccanismo che lo conterebbe (registro `worklog` dei worker) è provato nel caso B3, dove i doppi sono 1.350.

### B2. Crash di Iggy (`docker kill`, riavvio dopo 5 s)

300 esecuzioni/s da 3 passi da 500 ms, 40 s; a 10 s `docker kill` del container di Iggy (SIGKILL), a +5 s `docker start` (stesso container, stessi dati). `trim=0` per il controllo sul bus.

| Misura | SQLite r1 | SQLite r2 | SQLite r3 | Postgres r1 |
| --- | --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,79 > 2,92 | 2,65 > 2,92 | 2,79 > 2,85 | 6,23 > 2,85 |
| kill → `BUS_DOWN` del motore (ms) | 2.002 | 1.889 | 1.901 | 1.889 |
| avvio di Iggy → `BUS_UP` (ms) | 7.222 | 8.464 | 7.166 | 3.108 |
| **kill → `BUS_UP`** (ms) | **12.515** | **13.682** | **12.414** | **8.396** |
| `BUS_UP` → prima attività conclusa (ms) | 429 | 578 | 582 | 59 |
| Il più lungo intervallo senza accettazioni riuscite (client, ms) | 11.881 | 8.096 | 7.434 | 7.914 |
| Accettate / rifiutate con `E_BUS_DOWN` / offerte | 8.602 / 3.698 / 12.300 | 8.256 / 4.043 / 12.299 | 8.629 / 3.670 / 12.299 | 9.836 / 2.463 / 12.299 |
| Altri errori del client | 0 | 0 | 0 | 0 |
| Tentativi scaduti durante la pausa (`expired` = `rejected`) | 1.188 | 894 | 630 | 927 |
| **Accettate perse** | **0** | **0** | **0** | **0** |
| `done_events != 1`, duplicati sul bus, payload illeggibili | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 |

Criterio **superato**: rifiuto con codice (`E_BUS_DOWN`, e `E_PROJECTION_DOWN` mai), ricollegamento da solo (nessun intervento sul motore), zero accettate perse, nessun «accettato» senza scrittura durevole (le accettazioni già nel gruppo in volo aspettano il bus e rispondono dopo: `accept_rtt` max ≈ 8 s).
La coda `APPEND_FAIL` (ricerca dell'esito nella coda del topic) **non si è mai attivata**: le append bloccate sono tornate `Ok` da sole (`APPEND_SLOW` 2 per corsa, poi `APPEND_OK`); quel percorso non è provato.

#### B2.1 Il client `iggy` 0.12.0 e il crash del server (sorgente del crate e prove sul desktop; non misurate)

- **La chiamata non torna con un errore: aspetta.** Prova `bus-probe`, una append ogni 100 ms, `docker kill` a 8 s e `docker start` a 12 s: 301 append `ok`, zero errori; una sola è durata 8.401 ms (a cavallo del crash), poi ha risposto `ok`. Il client si ricollega e rifà il login da solo (`reconnection` attivo, tentativi illimitati, nelle impostazioni predefinite
  del crate). La testa del topic era 301 = messaggi riconosciuti: nessun duplicato. Per questo la sonda del motore ragiona sul **tempo**, non sull'errore.
- **Annullare l'append è un errore.** In una prima versione ogni append aveva un tempo massimo e si annullava (`tokio::time::timeout`). Dopo il crash il client risponde `LaneBlocked(op)`: «Lane blocked by operation N with an unknown outcome». Il motore è rimasto fermo oltre 5 minuti (un'operazione incerta, quella annullata,
  blocca la partizione finché non si risolve) e le richieste dei client si sono accumulate. Il crate offre `resolve_operation(op, scadenza)`, che rigioca l'operazione trattenuta finché ne prova l'esito (una sola volta sul log); `OperationUncertain(op)` e `LaneBlocked(op)` sono gestiti in `Bus::append` e l'append non si annulla più
  (si segna solo l'indicatore «lenta»). Con questa correzione le quattro corse sopra. **Causa: osservata senza file grezzo** (log del motore sul desktop, non conservato: non vale come misura) e **letta** nel sorgente del crate (`iggy-0.12.0/src/tcp/tcp_client.rs`, `iggy_common-0.12.0/src/traits/client.rs`).
- **Perché i tentativi scadono** (`expired` = `rejected`, 630-1.188 per corsa): durante la pausa il ciclo del motore ha 4 gruppi in volo sul bus e non legge più dalla coda (`can_take`, lettura del codice), quindi gli esiti dei worker restano fermi in coda oltre la scadenza (`work_ms` 0,5 s + 2 s); alla ripresa gli esiti vecchi si scartano e le attività si rifanno.
  **Ipotesi coerente con i numeri, non isolata:** i rifacimenti non sono contati con il registro dei worker in questo caso. Costo: un'interruzione del bus più lunga della scadenza rifà le attività che erano finite; con scadenze reali (minuti) non succederebbe.

### B3. Crash di un worker a metà attività

Due worker (4 sessioni ciascuno), 300 esecuzioni/s da 3 passi da 3 s, 30 s; a 12 s `kill -9` del worker 1; il worker 2 prende i tentativi che scadono. Il worker 1 non riparte.

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,96 > 2,86 | 1,91 > 2,86 | 7,16 > 2,77 |
| Tentativi scaduti e rifatti (`expired`) | 1.355 | 1.351 | 1.347 |
| Attività eseguite più di una volta (registro dei worker) | 1.355 | 1.351 | 1.347 |
| Tentativi iniziati / attività distinte | 29.252 / 27.897 | 29.251 / 27.900 | 29.247 / 27.900 |
| **kill → nuovo inizio della stessa attività** (min / mediana / max, ms) | 2.034 / 3.535 / 5.034 | 2.081 / 3.581 / 5.082 | 2.100 / 3.602 / 5.002 |
| Accettate perse / esiti doppi (`done_events != 1`) / duplicati sul bus | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Esiti scartati (`rejected`) | 0 | 0 | 0 |

Criterio **superato**: l'attività torna disponibile alla scadenza (consegna + 5 s: un tentativo consegnato t secondi prima del kill riparte a 5 − t s dopo il kill, quindi 2-5 s) e si rifà una volta sola. Le rifatte sono **tutte e sole** le scadute (1.355 = 1.355, **misurata**). `READY` non vale (il motore non cade) e non ho misurato «prima attività conclusa»: il motore ha continuato a concludere
le attività del worker 2 senza pausa (`accept_max_gap_ms` 97 ms nella prova sul desktop). La scadenza è in memoria: una scadenza che sopravvive a un crash del motore è il blocco C.

### B4. Disco pieno

Nessun filesystem vero si riempie. Il limite di dimensione si ottiene senza sudo in due modi:

- **Iggy:** volume docker di tipo `tmpfs` con `size=128m` montato dove Iggy scrive davvero, `/app/local_data` (**il montaggio `/local_data` del compose non è usato da Iggy**, v. deviazioni). La zavorra (un file con `dd`) si crea e si toglie con `docker exec` nel container di Iggy,
  perché un volume tmpfs si monta di nuovo per ogni container.
- **Database e archivio:** il motore gira in un namespace di mount proprio (`unshare -Urm`, nessun privilegio) con un `tmpfs` da 128 MB su `out/capmnt` dove stanno `spike.db` e l'archivio dei payload (`capns.sh`, che rifiuta di partire se il mount non è un tmpfs); zavorra e verifica con `nsenter` sul PID del motore.
  Variante `dbonly`: nel tmpfs c'è solo il database (archivio sul disco vero), per provare il disco pieno del database da solo.

Carico: 300 esecuzioni/s da 3 passi, 30 s (circa 1,4 MB/s di scritture). La zavorra lascia 10 MB liberi (1 MB per `dbonly`), 8 s di osservazione a disco pieno, poi la zavorra si toglie.
**Il tmpfs sta in RAM: `fsync` non costa niente e l'errore di spazio finito è quello vero (`ENOSPC`), ma la latenza non è quella del disco.**

#### B4.1 Dati di Iggy: Iggy si ferma (`dfull-iggy`, 3 corse)

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 7,08 > 2,92 | 2,81 > 2,89 | 2,93 > 1,96 |
| zavorra → motore vede il bus giù (ms) | 3.223 | 2.890 | 3.220 |
| **zavorra tolta → bus di nuovo su** (ms) | **4.522** | **1.666** | **10.916** |
| zavorra tolta → prima attività conclusa (ms) | 4.213 | 1.686 | 7.615 |
| Il più lungo intervallo senza accettazioni (client, ms) | 10.930 | 11.917 | 16.461 |
| Accettate / rifiutate `E_BUS_DOWN` | 5.463 / 3.281 | 5.002 / 4.298 | 3.225 / 6.074 |
| Concluse nella proiezione | 6.018 | 5.002 | 3.225 |
| **Accettate perse** | **0** | **0** | **0** |
| `done_events != 1`, duplicati sul bus, payload illeggibili | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 |

- **Iggy non rifiuta con un errore, esce.** Nel log del server (prova `cap-probe.sh`, `risultati/iggy-cap-probe-4-ms-r1.txt`): `partition prepare persistence failed … No space left on device (os error 28)`, poi `message pump stopped on a partition commit fault; the server is shutting down` e uscita del processo. **Misurata.**
  Il motore lo vede come bus giù (`E_BUS_DOWN`). Senza un supervisore il bus resta giù finché qualcuno rilancia Iggy: il ciclo di rilancio (`while true; do iggy-server; sleep 1; done` come entrypoint del container di prova) è parte della prova, non del motore. Con spazio ancora pieno il server riparte e ricade alla prima scrittura (nel log del motore `BUS_UP`/`BUS_DOWN` si alternano).
- **Stato precedente intatto:** dopo il rilancio Iggy rilegge i suoi dati; nella prova sul solo bus le messaggi riconosciuti (1.008) uguagliano la testa del topic (1.008); nelle corse ogni payload applicato si decifra e ogni accettata è conclusa una volta sola.
- **r1: 555 esecuzioni concluse ma non nel registro** (concluse 6.018 contro 5.463 nel ledger): 555 risposte `ResourceExhausted: h2 protocol error` viste dal client mentre le richieste erano ferme sul bus; il motore le aveva accettate in modo durevole. Eseguite, non perse; **causa lato client: ipotesi** (limite del collegamento HTTP/2 con migliaia di richieste ferme).
- **Il tempo di ripresa dipende dal ciclo di rilancio e dal client** (1,7-10,9 s): **ipotesi**, tre corse non lo spiegano (la corsa lenta ha avuto un ciclo di crash in più prima di restare su).
- **Soglia di 2 ore per la causa in Iggy:** non usata: la prova mostra la causa subito (ENOSPC fatale nel server) e nessuna impostazione provata la cambia; **non** è stata tentata `preallocate_segments`. Riportato come previsto.

#### B4.2 Database e archivio (`dfull-db`, 3 corse SQLite + 1 Postgres con l'archivio nel tmpfs)

| Misura | SQLite r1 | SQLite r2 | SQLite r3 | Postgres r1 (archivio nel tmpfs) |
| --- | --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,81 > 6,91 | 2,84 > 2,59 | 2,79 > 2,84 | 2,87 > 2,89 |
| zavorra → `STORAGE_FULL` (ms) | 5.275 | 5.339 | 5.278 | 7.597 |
| **zavorra tolta → proiezione ripresa** (ms) | **84** | **30** | **79** | **45** |
| zavorra tolta → prima attività conclusa (ms) | 392 | 261 | 374 | 250 |
| Accettate / rifiutate `E_STORAGE_FULL` | 6.800 / 2.499 | 6.816 / 2.484 | 6.798 / 2.502 | 6.798 / 2.501 |
| **Accettate perse** | **0** | **0** | **0** | **0** |
| Payload illeggibili, `done_events != 1`, duplicati | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 | 0, 0, 0 |

Con il database e l'archivio insieme in tutte le corse il primo a finire lo spazio è **l'archivio** (8 righe `STORAGE_FULL` su 8, tutte «archivio: No space left on device (os error 28)»; **misurata**). Per provare il database da solo, variante `dbonly` (1 MB liberi, archivio sul disco vero), 3 corse:

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,64 > 5,32 | 2,81 > 2,94 | 2,83 > 2,70 |
| zavorra → `STORAGE_FULL` (ms) | 761 | 790 | 788 |
| zavorra tolta → proiezione ripresa (ms) | 108 | 145 | 5 |
| zavorra tolta → prima attività conclusa (ms) | 266 | 322 | 270 |
| Accettate / rifiutate `E_STORAGE_FULL` | 6.828 / 2.471 | 6.794 / 2.506 | 6.837 / 2.462 |
| **Accettate perse** | **0** | **0** | **0** |

L'errore del database è `database or disk is full (code: 13)` (SQLite). **Il disco pieno del solo database di Postgres non è provato:** la corsa Postgres ha il tmpfs sull'archivio, non sul volume di Postgres (che, a disco pieno, andrebbe in `PANIC` e riavvio di recupero: **ipotesi**, non misurata).

Il motore continua ad accettare i risultati dei worker (durevoli sul bus) mentre il proiettore è fermo; la proiezione, a ripresa avvenuta, recupera l'arretrato. «Prima attività conclusa» qui è un esito **durevole sul bus**, non ancora proiettato.

### B5. Pause del disco a 10.000 att./s (3 corse da 10 minuti, SQLite, motore completo)

Come `full` del blocco A (1.000 esecuzioni/s da 10 passi, due tenant, `trim=1`), ora con il registro delle accettate e `verify light` (ledger contro database, `done_events`, payload a campione di circa 1/997 perché il taglio del bus è attivo: **la scansione dei duplicati sul bus non prova niente con `trim=1` e non si fa**).
**Criterio di stallo, scritto nel file grezzo prima delle corse:** un minuto (riga `MIN`) con meno di 9.000 att./s, oppure un intervallo senza accettazioni riuscite oltre 5 s, oppure ritardo di proiezione p99 oltre 1.000 ms.

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,77 > 4,55 | 4,40 > 4,39 | 4,58 > 4,30 |
| Offerte / accettate (ledger) | 605.000 / 605.000 | 605.001 / 604.622 | 605.000 / 603.639 |
| Rifiutate `E_PROJECTION_DOWN` | 0 | 0 | 819 |
| Errori del client, per tipo | nessuno | 379 `ResourceExhausted: h2 protocol error` | 541 `ResourceExhausted: h2 protocol error`, 1 `Cancelled` |
| **Accettate perse** (ledger contro database) | **0** | **0** | **0** |
| Concluse nella proiezione, oltre il ledger | 0 | 378 | 541 |
| `done_events != 1`, payload illeggibili (campione) | 0, 0 | 0, 0 | 0, 0 |
| Minuti con meno di 9.000 att./s (escluso il minuto 1, di avvio, sotto soglia in tutte e tre: 8.770-8.964) | 3 (8.889) | 6 (8.990) | 2, 3 (5.421 e 4.913) |
| Ritardo di proiezione p99 oltre 1.000 ms | minuti 3 e 4 (1.933, 2.357 ms) | minuti 6 e 7 (3.979, 1.244 ms) | minuti 2-5 (8.749, 16.327, 20.480, 5.431 ms) |
| Il più lungo intervallo senza accettazioni (client, ms) | 492 | 1.777 | 1.184 |
| **Stallo secondo il criterio** | **sì** (minuti 3-4) | **sì** (minuti 6-7) | **sì** (minuti 2-5, il più lungo) |

**Esito:** zero accettate perse in 3 corse su 3. Il criterio scatta in tutte e tre, ma solo r3 è uno stallo di throughput; r1 e r2 sono picchi di ritardo, e il minuto 1 è stato escluso dopo le corse (nel blocco A: uno stallo di throughput in 3 corse del motore completo). I 3 episodi sono diversi: r1 e r2 sono un picco di ritardo di proiezione di 1-4 s con throughput quasi pieno; r3 è lungo 4 minuti, la proiezione resta indietro fino a 20 s e il throughput scende a 4.913 att./s, poi recupera a 14-15.000.

- **Durante lo stallo di r3 il disco è lento, non il bus** (misurata, righe `MIN` di `iggy-sqlite-stall10k-4-ms-r3.txt`): scrittura dei payload nell'archivio p50 47,7-53,7 ms contro 8-12 (minuti 6-10) e transazione del database 80,6-89,3 ms contro 13-27; la lettura dal bus resta a 0,9-1,2 ms. Stessa firma dell'episodio del blocco A. **La causa del rallentamento del disco resta ipotesi** (non isolata, come nel blocco A).
- **`database is locked` di SQLite (code 5), 2 volte in r3** (`proiezione ferma` … `ripresa` dopo 331 e 411 ms): `busy_timeout` è 300 ms e il proiettore ritenta; nel frattempo le accettazioni tornano `E_PROJECTION_DOWN` (819 rifiuti). In r1 e r2 nessun `database is locked`. È il punto del blocco E («`SQLITE_BUSY`») già comparso a 10.000/s; **la causa è ipotesi** (il file trattenuto oltre 300 ms durante il rallentamento del disco).
- **Gli errori «senza tipo» del blocco A (1.401) erano probabilmente di questo tipo** (**ipotesi**, perché quel registro non c'era): `ResourceExhausted: h2 protocol error` visto dal client, con richieste che il motore ha invece accettato e concluso (378 e 541 concluse oltre il ledger). Per «zero accettate perse» valgono come eseguite non confermate, non come perse.
- **Il minuto 1 sta sotto 9.000 in tutte e tre le corse** (8.770-8.964): è la salita del carico e dell'avvio (nessun intervallo senza accettazioni oltre 0,5 s all'inizio); l'ho escluso dal conteggio e lo dichiaro qui perché il criterio scritto prima non lo escludeva.

### Deviazioni dal piano (blocco B)

- **`/local_data` non è la directory dati di Iggy.** Il compose monta i dati in `/local_data`, ma `iggy-server` scrive in `/app/local_data` (visto con `find` nel container). In tutte le fasi precedenti i dati di Iggy erano nello strato scrivibile del container (overlay2, `/var/lib/docker`, che su minisheep sta sullo stesso
  filesystem di `/home`); la corsa di `reset.sh` li cancellava perché `compose down` elimina il container. Le misure non cambiano; la directory dati «vuota» dell'aggiornamento del blocco A era in realtà il container nuovo. Per il disco pieno il montaggio limitato va su `/app/local_data` (`IGGY_MNT`).
- **Iggy con un ciclo di rilancio** nel solo caso `dfull-iggy` (`IGGY_SUP`): senza, il container esce e il volume tmpfs si perde. Rilancio del processo, non della macchina.
- **Il tmpfs è RAM**, non disco (v. B4). Nessun altro modo senza sudo; il filesystem vero non è mai stato riempito.
- **Postgres:** una corsa per il crash del motore, una per il crash di Iggy, una per il disco pieno con l'archivio nel tmpfs. **Non** una per il disco pieno del volume di Postgres (v. B4.2). Il crash del worker non tocca il database: solo SQLite. I 10.000/s: solo SQLite (il blocco A non aveva Postgres).
- **Il codice del motore è cambiato** (elenco sopra) **rispetto al blocco A**: le corse del blocco A non sono rimisurate e l'aggiornamento (A3) ha un comportamento diverso dopo la ripresa. L'ultima corsa di A3 non è stata rifatta.
- **Scarto di corse.** Nessuna corsa scartata dopo averla vista. Prima della serie misurata sono girate sul desktop (non conservate) le prove di messa a punto: la prima versione di Iggy giù (con tempo massimo e annullamento) è rimasta ferma sul `LaneBlocked` e ho cambiato il codice (B2.1); nessuna corsa misurata usa quella versione.
- **Script cambiati tra una corsa e l'altra**, non il binario: `scen.sh` ha avuto due modifiche durante la serie, la riga `DBURL` per Postgres con `dfull-db` (prima di `diskfull-db` Postgres e `dbonly`) e la variabile `CAPDBONLY` (prima di `dbonly`); i casi già eseguiti non le usano. `ms-run.sh` ha da ora la riga `binario sha256`.
- **La prima corsa `dbonly` r1 con 10 MB liberi non ha riempito il database** (non si è scartata come «fallita»: il database cresce troppo poco in 30 s; l'ho rifatta con 1 MB liberi e la prima versione non è conservata: il file è stato sovrascritto prima di aver deciso di riportarla). È la sola corsa rifatta con parametri diversi.
- **Prove sul desktop non misurate né conservate** (messa a punto e fattibilità): `cap-probe.sh` e `kill-probe.sh` sul desktop, le corse di prova dei casi. Il file `iggy-cap-probe-4-ms-r1.txt` su minisheep rifà `cap-probe.sh`.
- **Le attività rifatte per un bus giù** (B2) sono contate dal motore (`expired`, `rejected`) e non dal registro dei worker.
- **Registro dei worker** (`worklog`) attivo solo nei casi B1-B4; a 10.000/s no (una scrittura per consegna).
- **Tempo:** circa un giorno di lavoro tra codice, prove sul desktop e corse (le tre corse da 10 minuti e le attese del cancello di quiete comprese), dentro il tempo previsto; nessun caso ha superato le 2 ore per la causa in Iggy.

### Non coperto

- Spegnimento improvviso dell'host (NFR-6) e la perdita della cache del sistema: non misurabile su minisheep, argomentato nell'ADR.
- Disco pieno del volume di Postgres; disco pieno su un filesystem vero (qui tmpfs).
- Il percorso `APPEND_FAIL` (ricerca dell'esito nella coda del topic): mai eseguito dalle corse misurate.
- Un crash del motore mentre Iggy è giù; più guasti insieme; un crash del motore a metà `rewrite_live` (già non provato nel blocco A).
- Una scadenza del tentativo che sopravvive al crash del motore (blocco C): qui dopo la ripresa la scadenza riparte da zero.
- Iggy con refusal specifico «disco pieno»: Iggy esce, il motore dice `E_BUS_DOWN`.

### Comandi per rifare (blocco B)

Su minisheep, in `~/ttspikebus` (`env.sh`, `.env` da `./gen-env.sh`, binario `bin/busspike`). Una corsa alla volta, staccata dalla shell e attesa in primo piano: `./brun.sh <file> <comando>` poi `./brun.sh wait` se serve.

| Misura | Comando |
| --- | --- |
| B1 crash del motore | `CAP=4096 ./brun.sh risultati/iggy-sqlite-crash-engine-4-ms-r1.txt ./scen.sh iggy sqlite ekill` (Postgres: `… iggy pg ekill`) |
| B2 crash di Iggy | `CAP=4096 ./brun.sh risultati/iggy-sqlite-crash-iggy-4-ms-r1.txt ./scen.sh iggy sqlite bkill` |
| B3 crash di un worker | `CAP=4096 ./brun.sh risultati/iggy-sqlite-crash-worker-4-ms-r1.txt ./scen.sh iggy sqlite wkill` |
| B4.1 disco pieno di Iggy | `CAP=4096 ./brun.sh risultati/iggy-sqlite-diskfull-iggy-4-ms-r1.txt ./scen.sh iggy sqlite dfull-iggy` |
| B4.2 disco pieno di database e archivio | `CAP=4096 ./brun.sh risultati/iggy-sqlite-diskfull-db-4-ms-r1.txt ./scen.sh iggy sqlite dfull-db` (Postgres: `… iggy pg dfull-db`; solo database: `CAPFREE=1 CAPDBONLY=1 …`) |
| B4.1 fattibilità del disco pieno di Iggy | `./ms-run.sh risultati/iggy-cap-probe-4-ms-r1.txt ./cap-probe.sh` |
| B5 stalli a 10.000/s | `GATE60=1 ./brun.sh risultati/iggy-sqlite-stall10k-4-ms-r1.txt ./with-sampler.sh risultati/iggy-sqlite-stall10k-4-ms-r1.io ./scen.sh iggy sqlite thr10k secs=600` |
| Prova del client `iggy` su un crash | `./kill-probe.sh` (desktop o minisheep) |



## Blocco C

Percorso dei dati di Iggy, aggiornamento rifatto col motore del blocco B, timer durevoli. Minisheep, stessi 4 CPU (motore e Iggy su `1-4`, carico su `8-11`), `ms-run.sh` con pavimento di `fdatasync` prima e dopo (ogni file grezzo lo porta, con la riga `binario sha256`). File grezzi `risultati/iggy-sqlite-<riga>-4-ms-r<n>.txt`.
Quattro binari, ognuno per una serie intera (nessuna sostituzione a metà serie):

| Binario (sha256, primi 16) | Righe |
| --- | --- |
| `a5fff0281600e9e7` (+ `bccc3a31ff3bd020` costruito con `--features old`) | C1 e C2: il codice del blocco B (commit `7f156ba`), ricostruito dal sorgente |
| `175e97d37560d594` (+ `0f974d28fd20d3d5`, `--features old`) | C3: retry, scadenza, 100.000 timer, aggiornamento con i timer (le serie principali) |
| `2d387cd9c10037ab` | C3: battiti (riga mia, non del piano) e controlli «b2» di retry e scadenza |
| `2d0fda73e8e7adf9` (+ `285c6c2c450a1726`, `--features old`) | il codice committato (dopo `cargo fmt`): controlli «b3» di 100.000 timer e aggiornamento con i timer |

**Il codice committato non è il sorgente di `175e97d3…`.** Le due differenze sono il codice dei battiti (spento di predefinito, `lease_ms=0`) e la guardia con cui `finalize` arma un timer solo se serve ancora (prima: l'esecuzione esiste; ora, per le scadenze, il tentativo è ancora in volo, per gli altri tipi il passo non è concluso): la guardia vale per ogni tipo di timer. Per questo ogni riga principale ha una corsa di controllo con un binario più recente (retry e scadenza «b2», 100.000 timer e aggiornamento con i timer «b3»), con gli stessi esiti.

### Sintesi

1. **Percorso dei dati: il montaggio ora è `/app/local_data` (predefinito del compose).** La prima coppia (strato prima, montaggio dopo) differisce di oltre il 20 % nelle righe `accept` (200 att./s) e `start1k` (1.000 att./s): accettazione p50 34,3 contro 14,9 ms, `start_ready` p50 14,1 contro 8,2 ms. Una seconda coppia **in ordine inverso** capovolge il segno: in tutte e quattro le coppie la **prima** corsa è la lenta, qualunque sia il percorso. A 10.000/s (una coppia sola, non rovesciata) le due corse differiscono del 14-18 % e consegnano 9.993 e 9.996 att./s, zero accettate perse. Nessun effetto del percorso si vede sopra la variabilità tra corse (in ordine di tempo le otto corse a 200 e 1.000 att./s si alternano lenta, veloce, lenta, veloce; causa ipotesi); le righe a 200 e 1.000 att./s delle fasi precedenti portano però una variabilità tra corse fino a 2× (v. C1), da rileggere con questo limite.
2. **Aggiornamento rifatto col motore del blocco B: pausa 2.622 / 2.773 / 2.664 ms, accettate perse 0, attività eseguite due volte 0.** Con la prima corsa della serie (r0, tenuta a parte solo per il contatore delle attività doppie mal posizionato) la pausa va da 2,6 a 4,4 s su quattro corse; nel blocco A da 2,6 a 6,0 s. Le attività in volo ora finiscono e consegnano l'esito al motore nuovo (`late_ok` 2.966 / 3.034 / 2.993 = quelle in volo allo svuotamento), invece di rifarsi (nel blocco A 3.079 / 3.072 / 3.071 rifatte). Le 97 / 60 / 95 pronte ma non ancora consegnate aspettano la scadenza e girano per la prima volta 5 s dopo (non sono doppie).
3. **Retry con attesa attraverso un crash: superato in 3 corse su 3.** Ogni ritentativo parte una volta sola (299 / 300 / 299 su altrettanti), nessuno prima della scadenza, quelli già scaduti a fine ripresa partono 73-92 ms dopo `READY` (soglia 1 s), gli altri 19-23 ms (p50) dopo la scadenza.
4. **Scadenza del tentativo attraverso un crash: superato in 3 corse su 3.** 100 / 100 / 99 tentativi persi (worker 1 ucciso) tornano disponibili alla scadenza originale: inizio del rifacimento 20-23 ms (p50), 33-37 ms (p99) dopo la scadenza, nessuno prima né doppio. Nel blocco B la scadenza ripartiva da zero alla ripresa.
5. **100.000 timer in sospeso: ripresa entro 1,0-1,2 s** (soglia 30 s), RSS del motore 100 MB prima del crash e 193-220 MB dopo la ripresa. Tutti e 100.000 scattano una volta sola (scarto p50 9,8-10,1 ms, p99 19,9-20,0 ms), nessuno prima della scadenza, zero perse.
6. **Aggiornamento con 100.000 timer e attività in volo: pausa 9.499 / 6.744 / 7.305 ms** (soglia 30 s). Nessun timer perso né doppio; i 3.028-3.102 tentativi in volo finiscono e consegnano (`late_ok`), `expired` 0.
7. **Battiti (aggiunti da me, non nelle righe del piano):** con `lease_ms=1500` attività da 5 s non scadono finché il worker batte, anche attraverso il crash del motore; un worker morto si scopre 1,5 s dopo l'ultimo battito, non 7 s. 99 rifatte su 99, nessuna doppia.

Criterio generale: **nessun timer perso né doppio in nessuna corsa** (verifica «accettate perse 0» in tutte, `done_events != 1` a 0, nessun tentativo iniziato due volte con lo stesso numero). Nessuna riga ha superato il tempo né ha chiesto di fermarsi per Iggy.

### Cosa è cambiato nel motore (codice dello spike, per le prove)

- **Evento `Timer`** (tag 4 sul bus): (esecuzione, passo, tipo, tentativo, scadenza in µs, istante). Tipi: 0 scadenza del tentativo in corso, 1 attesa prima di ritentare, 2 passo `wait`. **Al più un timer per attività**: la riga più recente vince (`INSERT … ON CONFLICT DO UPDATE` nel proiettore, con una sola riga per attività in ogni istruzione), l'esito `Done` del passo la cancella (`DELETE … WHERE (exec_id, step) IN (…)`). Tabella tipata `timer(exec_id, step, kind, attempt, due_us)`.
- **Scrivere prima, poi agire (AD-2).** La consegna di un tentativo scrive l'evento `Timer` di scadenza sul bus; la consegna al worker parte e il timer si arma in memoria solo a gruppo durevole (`Group::sends`, `Group::arm`). Un tentativo il cui timer è durevole ma la cui consegna non è partita (crash in mezzo) si ritrova alla ripresa «in volo» e scade, poi si rifà.
- **Ripresa.** Timer = righe del database più coda del bus dalla posizione applicata (un `Timer` sostituisce, un `Done` toglie). Ogni attività emessa e non conclusa si classifica con il suo timer: scadenza = in volo presso un worker (esito valido con qualunque consegna, scadenza originale); ritentativo o `wait` = scatta alla sua scadenza (se è già passata, alla prima passata di 20 ms dopo la ripresa); **nessun timer = mai consegnata, si consegna subito** (nel blocco B restava «in volo» per `work_ms` + margine dalla ripresa). Una scadenza di tentativo già passata o entro `grace_ms` (2 s) dalla ripresa si sposta a `grace_ms` dopo la ripresa, solo in memoria: un worker che ha finito durante la pausa deve poter ricollegarsi e consegnare l'esito prima che il tentativo si rifaccia. È una scelta di progetto presa prima delle corse; **ipotesi** che serva (senza la finestra una scadenza assoluta già passata alla ripresa rifarebbe attività che il worker ha già finito e il cui esito sta per arrivare): nessuna corsa è stata fatta senza la finestra, quindi l'effetto non è misurato.
- **Passata dei timer ogni 20 ms** (prima 100 ms, solo per le scadenze), al più 4.096 timer per passata; i `wait` scaduti diventano un messaggio interno `WaitDone` che conclude il passo (uscita vuota).
- **Carichi nuovi:** `kind` 2 (catena di `wait`, `work_ms` = durata), `kind` 3 (il primo tentativo di ogni passo fallisce, attesa `retry_ms`). Nuovi campi: `retry_ms` (accettazione, evento, tabella `execution`), tentativo e indicatore di fallimento nella consegna, `failed` nell'esito, `attempt` nel registro dei worker.
- **Battiti** (`lease_ms`, spento di predefinito): il worker manda un battito ogni `lease_ms/3` per tentativo; il motore lo traduce in un nuovo `Timer` di scadenza (`lease_ms` dall'ultimo battito), cioè un evento sul bus per battito.
- **Riversamento dell'aggiornamento** (`rewrite_live`): riscrive anche i timer come eventi `Timer` con la scadenza originale; i due topic si scrivono in parallelo su due connessioni, a richieste da 1.024 (con 100.000 esecuzioni vive la scrittura in serie da 256 sarebbe dominata dalla durata di ogni richiesta; non provata in serie).
- **Gruppi di consegna e timer non contano per il limite `k`** dei gruppi di raccolta (altrimenti rallenterebbero l'ingresso).

### C1. Percorso dei dati di Iggy

Prima: `IGGY_MNT` valeva `/local_data`, ma Iggy scrive in `/app/local_data` (visto nel blocco B): i dati stavano nello strato del container. Ora il predefinito è `/app/local_data` (`compose.yaml`); `IGGY_MNT=/local_data` riproduce il caso vecchio. Verificato su minisheep (**osservata senza file grezzo**: output di `ssh`, non conservato): i file di Iggy compaiono in `~/.cache/ttspikebus-data/iggy` (di root, 6 voci: `logs`, `metadata`, `runtime`, `state`, `storage-format`, `streams`), e dentro il container `/app/local_data` sta su `/dev/mapper/root`, lo stesso filesystem di `/home` (come lo strato di overlay2, secondo il blocco B).

Coppie strato contro montaggio, binario `a5fff0281600e9e7`, motore del blocco B. Piano: una coppia; ne ho fatte due su `accept` e `start1k` (la seconda con l'ordine invertito, perché la prima differiva di oltre il 20 %) e una su `thr10k` (10 minuti, `trim=1`, registro delle accettate, `GATE60=1`, come B5):

| Riga | Corsa (ordine) | Pavimento p50 prima > dopo (ms) | Accettazione p50 / p99 (ms) | `start_ready` p50 / p99 (ms) | Append p50 (ms) | att./s |
| --- | --- | --- | --- | --- | --- | --- |
| accept, coppia 1 | **strato (1ª)** | 7,84 > 7,74 | 34,30 / 387,58 | 22,64 / 312,06 | 21,41 | 200 |
| accept, coppia 1 | montaggio (2ª) | 2,76 > 4,61 | 14,90 / 29,12 | 8,85 / 17,52 | 8,27 | 200 |
| accept, coppia 2 | **montaggio (1ª)** | 4,99 > 4,29 | 22,11 / 63,45 | 14,12 / 37,79 | 13,78 | 200 |
| accept, coppia 2 | strato (2ª) | 4,53 > 2,52 | 12,74 / 31,23 | 8,34 / 18,35 | 8,07 | 200 |
| start1k, coppia 1 | **strato (1ª)** | 7,70 > 7,55 | 20,96 / 37,82 | 14,06 / 23,89 | 13,66 | 998 |
| start1k, coppia 1 | montaggio (2ª) | 7,37 > 2,65 | 12,06 / 29,15 | 8,15 / 16,94 | 7,74 | 1.002 |
| start1k, coppia 2 | **montaggio (1ª)** | 2,88 > 5,00 | 20,56 / 36,48 | 13,97 / 21,12 | 13,61 | 1.000 |
| start1k, coppia 2 | strato (2ª) | 8,36 > 4,58 | 12,83 / 30,56 | 8,40 / 18,38 | 7,88 | 1.000 |
| thr10k | **strato (1ª)** | 2,75 > 4,43 | 90,88 / 254,72 | 79,17 / 212,09 | 72,70 | 9.993 |
| thr10k | montaggio (2ª) | 3,96 > 4,51 | 76,99 / 226,30 | 67,84 / 182,27 | 65,86 | 9.996 |

File: `iggy-sqlite-path-{layer,mount}-{accept,start1k}-4-ms-r{1,2}.txt`, `iggy-sqlite-path-{layer,mount}-thr10k-4-ms-r1.txt`. Il braccio (strato o montaggio) è scritto solo nel nome del file: il file grezzo non registra `IGGY_MNT`. L'ordine delle corse viene dall'orario `[stato prima …]` in testa a ogni file (22:06 strato `accept` r1, poi 22:08, 22:09, 22:11; 10.000/s alle 22:15 e 22:30; seconde coppie dalle 22:41 alle 22:47).

**Lettura, da dire chiaramente.**

- **Nella prima coppia le righe `accept` (200 att./s) e `start1k` (1.000 att./s) differiscono di oltre il 20 %** (montaggio meglio dello strato: accettazione p50 −57 %, p99 −92 %; `start1k` accettazione p50 −42 %, `start_ready` p50 −42 %, p99 −29 %). A 10.000/s (una coppia sola, **non rovesciata**) la differenza, rispetto allo strato, è −15 % (accettazione p50), −14 % (`start_ready` p50 e p99), cioè 14-18 % se si prende il montaggio come base, con lo stesso ritmo.
- **Ma la differenza non è del percorso:** nella seconda coppia, a ordine invertito, la corsa lenta è quella del montaggio e la veloce quella dello strato (**misurata**, stessi file). In tutte e quattro le coppie a 200 e 1.000 att./s la **prima** corsa è la più lenta (strettamente, otto corse su otto, anche quando non c'era una pausa prima) e lo è anche nella coppia a 10.000/s (che però non ha la controprova a ordine inverso): latenza p50 1,6-2,3 volte quella della seconda a 200-1.000/s, 1,17-1,18 volte a 10.000/s. Le corse lente hanno l'append p50 a 13,6-21,4 ms, le veloci a 7,7-8,3 ms, a parità di percorso. Il pavimento di `fdatasync` non spiega l'ordine (es. `start1k` coppia 1: 7,7 e 7,4 ms prima delle due corse, la lenta e la veloce). **La causa dell'effetto d'ordine è ipotesi, non isolata** (stato del disco dopo la corsa precedente).
- **Zero accettate perse, nessuno stallo nei minuti 2-10, in entrambe le corse a 10.000/s** (il minuto 1 di avvio, 8.970 e 8.989 att./s, sta sotto la soglia di 9.000 del criterio ed è escluso come in B5) (verifica «accettate 605.000 / 605.001, perse 0», nessun errore del client, intervallo più lungo senza accettazioni 519 e 533 ms, ritardo di proiezione p99 sempre sotto 520 ms per minuto, tutti i minuti sopra 9.800 att./s tranne il minuto 1 di avvio: 8.970 e 8.989). Nel blocco B le tre corse da 10 minuti segnavano tutte un episodio sotto soglia, qui nessuna: lo strato (che era la configurazione del blocco B) non ne ha avuti. L'incidenza degli stalli varia nel tempo senza che il percorso c'entri (ipotesi: stato del disco).
- **Effetto sulle fasi precedenti:** il percorso non introduce un errore sistematico (il segno cambia con l'ordine), quindi i numeri delle fasi 2-4 non si ribaltano per questo. **Vanno però riletti** quelli a 200 e 1.000 att./s presi con una corsa per braccio: la variabilità tra corse arriva a 2× (append p50 8 contro 14 contro 21 ms), più di ogni differenza del 20 % che un confronto di quelle fasi poteva dichiarare. Le righe a 10.000/s differiscono del 14-18 % in questa coppia (una sola, non rovesciata: non basta per dire che siano stabili).
- **Rilettura fatta sui file delle fasi precedenti (aggiunta dopo il rapporto, sui file grezzi già nel repo):** in ordine di tempo, append p50 e orario `[stato prima …]`.
  - **Fase 3 (gRPC contro iroh):** ordine alternato ABBA e append p50 fermo tra 7,96 e 8,77 ms in tutte le 28 corse `start1k`, `size64k`, `size1m`, `iso`: nessuna alternanza lenta e veloce, la scelta di gRPC non ne è toccata (**misurata**).
  - **Fase 2c (Iggy contro Valkey):** append p50 fermo (Iggy 7,96-8,22 ms, Valkey 2,99-3,11 ms) in 20 corse: nessuna alternanza (**misurata**).
  - **Fase 2d:** rumorosa (Iggy 8,1-27,2 ms, quasi sempre circa 14; Valkey 3,4-20,2 ms) e Iggy sempre **primo** nella coppia: i numeri di Iggy a 200 e 1.000 att./s della 2d possono essere gonfiati dall'ordine (**ipotesi**). Lo scarto andrebbe contro Iggy, quindi non indebolisce la scelta di Iggy; il confronto pulito resta quello della 2c (8 contro 3 ms di append).
  - La pressione I/O e il carico registrati prima di ogni corsa non distinguono le corse lente dalle veloci, né nella 2d né qui.
- Da qui in poi si usa il montaggio.

### C2. Aggiornamento rifatto col motore del blocco B

Procedura del blocco A (`up-run.sh`, Iggy 0.9.0 → 0.10.0, due build del motore, `--features old`), con i binari ricostruiti dal sorgente del blocco B. 3 corse da 334 esecuzioni/s di **tre passi da 3 s** (1.002 att./s, circa 3.000 vive), 70 s, aggiornamento a 20 s, due tenant, `trim=0`. `iggy-sqlite-upgrade-rerun-4-ms-r{1,2,3}.txt`. La directory dati di Iggy è ora davvero il montaggio svuotato da `reset.sh` (nel blocco A era il container nuovo).

| Misura | Blocco A r1 | r2 | r3 | Rifatto r1 | r2 | r3 |
| --- | --- | --- | --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,85 > 4,55 | 4,61 > 4,51 | 4,53 > 4,97 | 4,50 > 4,35 | 5,07 > 4,70 | 5,93 > 2,74 |
| Accettate / offerte | 22.057 / 24.048 | 23.118 / 24.048 | 23.182 / 24.048 | 23.199 / 24.048 | 23.140 / 24.048 | 23.159 / 24.048 |
| **Accettate perse** | 0 | 0 | 0 | **0** | **0** | **0** |
| Esecuzioni vive riversate | 3.079 | 3.072 | 3.071 | 3.063 | 3.094 | 3.088 |
| In volo allo svuotamento (`DRAINED`, motore vecchio) | — | — | — | 2.966 | 3.034 | 2.993 |
| **Attività eseguite due volte** (registro dei worker, a corsa finita) | 3.079 | 3.072 | 3.071 | **0** | **0** | **0** |
| Esiti dei tentativi in volo consegnati al motore nuovo (`late_ok`) | — | — | — | 2.966 | 3.034 | 2.993 |
| Tentativi scaduti e consegnati dopo la scadenza (`expired`) | — | — | — | 97 | 60 | 95 |
| Svuotamento / nuovo Iggy / riversamento / motore nuovo fino a `READY` (ms) | 463 / 1.941 / 864 / 3.585 | 616 / 1.145 / 512 / 1.030 | 514 / 1.149 / 495 / 924 | 616 / 1.079 / 541 / 928 | 513 / 1.129 / 577 / 1.131 | 565 / 1.171 / 531 / 927 |
| **Pausa: segnale → `READY`** (ms) | 5.989 | 2.791 | 2.587 | **2.622** | **2.773** | **2.664** |
| Segnale → prima attività conclusa (ms) | 9.170 | 5.857 | 5.742 | 2.794 | 2.967 | 2.845 |
| Il più lungo intervallo senza accettazioni riuscite (client, ms) | 5.928 | 2.738 | 2.554 | 2.511 | 2.686 | 2.633 |

I valori del blocco A sono quelli della serie a tre passi di A3.3. Corsa in più, `iggy-sqlite-upgrade-rerun-4-ms-r0-conteggio-anticipato.txt` (la prima della serie, tenuta): pausa 4.401 ms, intervallo senza accettazioni 4.365 ms, 0 attività eseguite due volte, accettate perse 0; il conteggio delle attività doppie era nel punto sbagliato dello script (a corsa non ancora finita) e l'ho spostato dopo la verifica, quindi per r0 manca il contatore `late_ok`.

- **Le attività in volo finiscono e consegnano al motore nuovo invece di rifarsi** (**misurata**: `late_ok` = tentativi in volo allo svuotamento, tutte e tre le corse; 0 attività eseguite due volte contro circa 3.070 del blocco A). Il motore vecchio non consegna più dopo `SIGUSR1` ma i worker tengono l'esito finché non è confermato (blocco B), e il motore nuovo accetta l'esito di una consegna precedente. Per questo la prima attività conclusa arriva 172-194 ms dopo `READY` (nel blocco A 3,1 s dopo).
- **`expired` 97 / 60 / 95** = esecuzioni vive meno attività in volo (3.063 − 2.966, 3.094 − 3.034, 3.088 − 2.993, **misurata**): sono passi pronti con l'evento `Ready` già scritto ma non ancora consegnati allo svuotamento. Il motore del blocco B li tratta come «in volo» (consegna 0) e li consegna solo alla scadenza di `work_ms` + 2 s dalla ripresa: girano per la prima volta circa 5 s dopo `READY` (5 s dal codice, **non misurato**). Non sono attività doppie (nessun tentativo iniziato due volte). I timer durevoli del punto C3 tolgono questo ritardo (v. C3.4: «da consegnare subito»).
- **La pausa non cambia** rispetto al blocco A (2,6-2,8 s contro 2,6-2,8 s nelle due corse veloci di A; la lenta di A, 6,0 s, non si è ripetuta, ma la prima corsa di oggi, r0, è stata lenta anch'essa: 4,4 s, con il motore nuovo che ha impiegato 2,96 s a partire). Resta dominata da nuovo Iggy (1,1 s) e dall'avvio del motore (0,9-1,1 s); **la causa del primo avvio lento è ancora ipotesi** (creazione dei topic sul server nuovo dopo una pausa del disco): in entrambe le serie la prima corsa è la lenta (blocco A r1 6,0 s, oggi r0 4,4 s).

### C3. Timer durevoli

Tutte le righe: SQLite, due tenant, `kill -9` del motore e riavvio con lo stesso script del blocco B (`start_engine`); le righe retry e scadenza scrivono sul registro del motore una riga per ogni timer programmato e scattato (`TIMER_LOG`, `TSET` e `TFIRE`); l'inizio di ogni tentativo viene dal registro dei worker (`worklog`, con il numero del tentativo). Lo scarto di scatto è l'istante di inizio nel worker meno la scadenza (`tstat.sh`), quindi comprende la scrittura durevole del timer di scadenza nuovo (circa 15-20 ms) e la consegna.

#### C3.1 Retry con attesa attraverso un crash (`iggy-sqlite-timer-retry-4-ms-r{1,2,3}.txt`)

`kind` 3, `n=1`, 100 esecuzioni/s per 3 s (tutti i primi tentativi falliscono dopo 100 ms), attesa **5 s**. A 4,5 s `kill -9` del motore, quando ogni fallimento è già durevole e nessun ritentativo è scaduto (prima scadenza a 5,1 s); il motore resta giù **2 s** (scelta mia: così parte dei timer è già scaduta alla fine della ripresa e parte no, e si provano tutti e due i casi), poi riparte.

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,87 > 2,90 | 1,91 > 2,72 | 2,74 > 2,92 |
| Esecuzioni accettate, timer di ritentativo scritti | 299, 299 | 300, 300 | 299, 299 |
| Ripresa: attività in attesa di un timer / da consegnare subito | 299 / 0 | 300 / 0 | 299 / 0 |
| kill → `READY` (ms, di cui 2.000 voluti) | 2.146 | 2.175 | 2.166 |
| **Tentativo 1 iniziato: totale / più di una volta / prima della scadenza** | 299 / **0** / **0** | 300 / **0** / **0** | 299 / **0** / **0** |
| Tentativo 0 eseguito più di una volta | 0 | 0 | 0 |
| Già scaduti a `READY` (n): inizio − `READY` p50 / p99 / max (ms) | 152: 92,0 / 92,3 / 92,3 | 154: 72,8 / 73,0 / 73,0 | 152: 77,9 / 78,2 / 78,2 |
| Non scaduti a `READY` (n): inizio − scadenza p50 / p99 / max (ms) | 147: 19,5 / 55,3 / 71,9 | 146: 22,9 / 44,0 / 62,0 | 147: 23,2 / 61,1 / 72,7 |
| Accettate perse, verifica | 0, OK | 0, OK | 0, OK |

Criterio **superato**: ogni ritentativo parte una volta sola, mai prima della scadenza, e al più 92 ms dopo la fine della ripresa (soglia 1 s). Il tempo dopo `READY` per i timer già scaduti è quasi tutto l'attesa dei worker, che si ricollegano dopo 200 ms dal collegamento perso (attesa fissa del worker; **ipotesi** per la ripartizione, non isolata). Il ritardo «presa in carico meno scadenza» del solo motore ha p99 1,47 s: per i timer scaduti durante la pausa comprende i 2 s di pausa voluta, non è un ritardo di scatto. Corsa di controllo con il binario `2d387cd9c10037ab` (`…-retry-b2-4-ms-r1.txt`): 299 su 299 una volta, 0 prima, 85,5 ms dopo `READY` per i 153 già scaduti, 22,7 / 66,1 ms per gli altri 146.

#### C3.2 Scadenza del tentativo attraverso un crash (`iggy-sqlite-timer-deadline-4-ms-r{1,2,3}.txt`)

Mancava nel blocco B. Due worker (4 sessioni ciascuno), esecuzioni da un passo di **3 s** (scadenza = consegna + 3 s + 2 s), 100/s per 3 s. A 2 s si uccide il worker 1 (i suoi tentativi in corso, circa 100, non torneranno mai), a 2,5 s `kill -9` del motore e riavvio subito. Le scadenze cadono tra 5 e 7 s, quindi dopo la ripresa (a 2,6 s) e oltre la finestra di `grace_ms` (2 s).

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,83 > 2,83 | 2,62 > 2,62 | 2,83 > 2,56 |
| kill del motore → `READY` (ms) | 78 | 87 | 77 |
| Ripresa: in volo con scadenza ripristinata / da consegnare subito | 250 / 1 | 250 / 1 | 250 / 1 |
| Scadenze scattate (`expired`) / tentativo 1 iniziati / iniziati più di una volta / prima della scadenza | 100 / 100 / **0** / **0** | 100 / 100 / **0** / **0** | 99 / 99 / **0** / **0** |
| Tentativi del worker 2 consegnati al motore nuovo (`late_ok`) | 150 | 150 | 151 |
| Tentativo 1: inizio − scadenza p50 / p99 / max (ms) | 23,1 / 37,4 / 37,8 | 21,3 / 34,8 / 34,8 | 20,3 / 32,6 / 36,4 |
| Presa in carico − scadenza, solo motore, p50 / p99 (ms) | 9,6 / 19,9 | 11,1 / 18,9 | 10,2 / 20,0 |
| Tentativo 0 eseguito più di una volta | 0 | 0 | 0 |
| Accettate perse | 0, OK | 0, OK | 0, OK |

Criterio **superato**: la scadenza scatta dopo il crash, alla scadenza originale (nel blocco B sarebbe ripartita da `work_ms` + 2 s dalla ripresa, cioè dopo circa 5,6 s), una volta sola. In r2 e nella corsa di controllo «b2» un tentativo 1 non ha un timer di scadenza 0 nel registro del motore (`senza timer noto 1`; nelle due corse `deadline-grace` r1 sono 2): **ipotesi, non verificata**, è il caso «timer durevole, consegna non partita» (il motore muore tra la scrittura del timer e l'invio): alla ripresa risulta in volo, scade e si consegna come tentativo 1 senza che il tentativo 0 sia partito. Coerente con i conteggi, non provato. Corsa di controllo `…-deadline-b2-4-ms-r1.txt`: 101 su 101 una volta, inizio − scadenza 21,1 / 36,0 ms (p50 / p99).

**Corse in più, finestra di `grace_ms`** (`iggy-sqlite-timer-deadline-grace-4-ms-r{1,2}.txt`, le prime due che ho fatto, con attività da 1 s: le scadenze, 4-5 s, cadevano dentro i 2 s dopo la ripresa): le scadenze si spostano a `grace_ms` dopo `READY` (solo in memoria), quindi il rifacimento parte 109 / 113 ms (p50) e 557 / 607 ms (p99) dopo la scadenza originale, 51 e 50 su 51 e 50 una volta sola, nessuna doppia, accettate perse 0. È il prezzo voluto della finestra (v. sopra), non un ritardo del motore: la presa in carico del motore resta a 1,3 / 2,0 ms p50.

#### C3.3 100.000 timer in sospeso (`iggy-sqlite-timer-100k-4-ms-r{1,2,3}.txt`)

`kind` 2: un passo `wait` da **150 s** per esecuzione, 5.000 esecuzioni/s per 20 s (100.005 / 100.000 / 100.000 accettate), `size=64`. A 35 s dall'inizio (tutte accettate e proiettate, nessun timer ancora scaduto) `kill -9` del motore e riavvio subito; poi si aspetta che tutti scattino (150-170 s dopo l'accettazione) e si verifica. `trim=0`, quindi la verifica scandisce tutto il bus.

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,81 > 2,89 | 2,83 > 2,78 | 2,91 > 2,76 |
| Timer in sospeso prima del crash / dopo `READY` | 100.005 / 100.005 | 100.000 / 100.000 | 100.000 / 100.000 |
| **kill → `READY`** (ms) | **1.025** | **1.035** | **1.219** |
| RSS del motore prima del crash / dopo `READY` / a fine corsa (kB) | 100.552 / 197.528 / 202.676 | 101.904 / 192.964 / 197.656 | 100.280 / 219.984 / 225.288 |
| Timer scattati, di cui prima della scadenza | 100.005, **0** | 100.000, **0** | 100.000, **0** |
| Scarto di scatto (esito − scadenza, istanti delle decisioni) p50 / p99 / max (ms) | 9,9 / 20,0 / 20,9 | 10,1 / 19,9 / 20,9 | 9,8 / 20,0 / 20,8 |
| Presa in carico − scadenza, solo motore, p50 / p99 (ms) | 9,89 / 19,89 | 10,01 / 19,84 | 9,73 / 19,95 |
| Accettate perse / `done_events != 1` / eventi `Done` sul bus / duplicati sul bus | 0 / 0 / 100.005 / 0 | 0 / 0 / 100.000 / 0 | 0 / 0 / 100.000 / 0 |

Criterio **superato**: ripresa in 1,0-1,2 s (soglia 30 s), nessun timer perso né doppio. Lo scarto è lo scarto della passata da 20 ms (uniforme tra 0 e 20 ms), non c'è coda neanche con 100.000 scadenze in 20 s (5.000/s). L'esito è l'istante della decisione (`ts` dell'evento `Done`, prima della scrittura durevole): non comprende i circa 15-20 ms di scrittura. RSS: **il raddoppio dopo la ripresa (100 → 193-220 MB) è ipotesi**: il picco del caricamento dal database di 100.000 esecuzioni, attività e timer non restituito al sistema; non isolato (il motore resta a 198-225 MB anche a fine corsa). `kill -9` con 100.000 timer e nessun evento da rileggere dal bus: la ripresa qui è quasi tutta lettura del database; il percorso con la rilettura del bus è coperto dalle righe scadenza e battiti (10-56 eventi riletti), non a questa scala.

Corsa di controllo «b3» (`iggy-sqlite-timer-100k-b3-4-ms-r1.txt`, binario `2d0fda73e8e7adf9` = codice committato): 100.000 timer, kill → `READY` 1.219 ms, RSS 100.260 / 194.716 / 200.144 kB, tutti scattati una volta, nessuno prima della scadenza, scarto p50 / p99 / max 10,0 / 20,0 / 20,9 ms, accettate perse 0, `done_events != 1` 0.

#### C3.4 Aggiornamento con i timer (`iggy-sqlite-timer-upgrade-4-ms-r{1,2,3}.txt`)

Procedura di C2 (tre passi da 3 s, 334 esecuzioni/s) preceduta da **100.000 esecuzioni `wait` da 150 s** (`PRE100K=1`, 5.000/s per 20 s), tutte in sospeso durante l'aggiornamento, e **attività in volo** (circa 3.000). Motore vecchio e nuovo con il codice dei timer (`--features old` per il vecchio). Dopo il carico si aspetta che i timer scattino (80 s) e si verifica due volte (le 100.000 attese e le esecuzioni del carico).

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,77 > 2,77 | 2,27 > 2,75 | 2,59 > 2,73 |
| Timer in sospeso nel motore vecchio / riscritti sul bus nuovo | 100.005 + 3.098 / 103.103 | 100.005 + 3.028 / 103.033 | 100.005 + 3.102 / 103.107 |
| Esecuzioni vive riversate | 103.158 | 103.173 | 103.162 |
| Ripresa: in volo / in attesa di un timer / da consegnare subito | 3.098 / 100.005 / 55 | 3.028 / 100.005 / 140 | 3.102 / 100.005 / 55 |
| Svuotamento / nuovo Iggy / motore nuovo fino a `READY` (ms) | 1.286 / 1.155 / 7.058 | 558 / 1.158 / 5.027 | 821 / 1.144 / 5.340 |
| di cui riversamento (database e archivio → bus nuovo, ms) | 3.898 | 3.868 | 3.983 |
| **Pausa: segnale → `READY`** (ms) | **9.499** | **6.744** | **7.305** |
| Il più lungo intervallo senza accettazioni riuscite (client, ms) | 9.479 | 6.606 | 7.175 |
| Accettate / offerte (carico a 3 passi); errori di connessione | 20.866 / 24.048; 2.766 | 21.806 / 24.048; 2.067 | 21.635 / 24.048; 2.148 |
| Timer scattati / scarto (esito − scadenza) p50 / p99 (ms) | 100.005 / 10,2 / 20,1 | 100.005 / 10,3 / 20,2 | 100.005 / 10,1 / 20,1 |
| `late_ok` / `expired` / tentativi iniziati due volte | 3.098 / 0 / 0 | 3.028 / 0 / 0 | 3.102 / 0 / 0 |
| Accettate perse (entrambe le verifiche) / `done_events != 1` / duplicati sul bus | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| RSS del motore nuovo dopo `READY` / a fine corsa (kB) | 233.320 / 238.688 | 251.824 / 259.880 | 268.664 / 269.528 |

Criterio **superato**: pausa 6,7-9,5 s (soglia 30 s di Marco), nessun timer perso né doppio, nessuna attività rifatta. Con 100.000 timer la pausa cresce di 4-7 s rispetto a C2 (2,6-2,8 s): il riversamento dura 3,9 s (ipotesi sulla durata: 100.000 `Accepted` con payload letti dall'archivio uno alla volta e riscritti, ~310.000 eventi; non separato dal costo di scrittura) e il resto dell'avvio del motore nuovo 1,2-3,2 s (lettura di 100.000 righe per tabella dal database, non separata dal resto). Gli «da consegnare subito» (55 / 140 / 55) sono passi pronti non consegnati allo svuotamento: ora si consegnano subito (in C2 aspettavano 5 s). Gli errori di connessione dei client (2.067-2.766) sono richieste rifiutate durante la pausa (motore non raggiungibile), non accettate: nessuna delle accettate è persa.

**Dipendenza dalla finestra di `grace_ms`:** la pausa (6,7-9,5 s) supera la scadenza di 5 s (3 s di lavoro + 2 s di margine) di ogni attività in volo, quindi alla ripresa tutte le scadenze erano già passate; `expired` è 0 perché la finestra le sposta di `grace_ms` (2 s) dopo `READY`, tempo in cui i worker si ricollegano e consegnano gli esiti (`late_ok`, che arrivano 0,2 s dopo `READY`). Senza la finestra le stesse attività si sarebbero rifatte: **ipotesi**, non provata. Il «nessuna attività rifatta» di questa riga dipende quindi dalla finestra.

Corsa di controllo «b3» (`iggy-sqlite-timer-upgrade-b3-4-ms-r1.txt`, binari `2d0fda73e8e7adf9` e `285c6c2c450a1726` = codice committato; pavimento 5,60 > 2,77 ms): 103.000 timer riscritti (riversamento 3.827 ms), pausa **7.990 ms** (svuotamento 1.639, nuovo Iggy 1.219, motore nuovo fino a `READY` 5.132 ms), intervallo senza accettazioni 7.867 ms, `late_ok` 3.000, `expired` 0, 148 da consegnare subito, 100.000 timer scattati una volta (scarto p50 / p99 / max 9,8 / 19,9 / 20,3 ms), nessuna attività iniziata due volte, accettate perse 0 in entrambe le verifiche, RSS del motore nuovo 248.656 kB dopo `READY`, 257.256 a fine corsa.

#### C3.5 Battiti (`iggy-sqlite-timer-heartbeat-4-ms-r{1,2,3}.txt`, binario `2d387cd9c10037ab`)

Riga non prevista dal piano ma chiesta dal compito («battiti del worker»): con `lease_ms=1500` e `grace_ms=500` la scadenza di ogni tentativo è 1,5 s dall'ultimo battito, che il worker manda ogni 500 ms (ogni battito è un evento `Timer` sul bus). Attività da **5 s**, più lunghe del contratto (senza battiti scadrebbero tutte); 100/s per 2 s; due worker; a 3 s si uccide il worker 1, a 3,5 s `kill -9` del motore e riavvio subito.

| Misura | r1 | r2 | r3 |
| --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,76 > 2,78 | 2,75 > 2,90 | 2,83 > 1,96 |
| Esecuzioni, in volo alla ripresa | 199, 199 | 199, 199 | 199, 199 |
| Battiti scritti (prima del crash / a fine corsa) | 785 / 1.306 | 783 / 1.309 | 784 / 1.310 |
| Scadute (`expired`) / consegnate dopo il crash dal worker 2 (`late_ok`) | 99 / 100 | 99 / 100 | 99 / 100 |
| Tentativo 1 iniziati / più di una volta / prima della scadenza | 99 / **0** / **0** | 99 / **0** / **0** | 99 / **0** / **0** |
| Tentativo 1: inizio − scadenza dell'ultimo battito p50 / p99 (ms) | 21,0 / 99,0 | 22,5 / 105,4 | 24,9 / 90,3 |
| Accettate perse | 0, OK | 0, OK | 0, OK |

Esito: i 100 tentativi del worker 2, vivi e battenti, **non scadono** né attraverso il crash del motore; i 99 del worker 1, morto, scadono 1,5 s dopo l'ultimo battito e si rifanno una volta. Il p99 di circa 100 ms è **ipotesi**: sono le scadenze dentro la finestra di `grace_ms` (500 ms) dopo la ripresa, spostate in avanti. Costo dei battiti: un evento sul bus per battito per tentativo (due al secondo per tentativo in volo); con 50.000 tentativi in volo sarebbero 100.000 eventi al secondo, **non misurato** e probabilmente da non fare così (v. «Non coperto»).

### Deviazioni dal piano (blocco C)

- **Coppie del percorso dei dati:** il piano chiedeva una coppia; ne ho fatte due per `accept` e `start1k` (la seconda a ordine invertito) e una per `thr10k`, perché la prima differiva di oltre il 20 %. Ordine delle corse: strato prima e montaggio dopo nelle coppie 1, il contrario nelle coppie 2.
- **Binari ricostruiti dal sorgente:** per C1 e C2 non ho usato il binario del blocco B (`d51b36de…`) ma uno ricostruito dallo stesso codice (`a5fff028…`, hash diverso per i percorsi nel debug); quello «vecchio» del blocco A (`eb9c8838…`) era costruito dal codice del blocco A, quindi ho ricostruito anche `busspike-old` dal codice del blocco B (`bccc3a31…`). Le righe C3 usano `175e97d3…` e `0f974d28…`; battiti e controlli «b2» `2d387cd9…`. **Il codice committato non è il sorgente delle serie principali.** Le righe retry, scadenza, 100.000 timer e aggiornamento con i timer sono misurate su `175e97d3…` (tre corse ciascuna); i controlli «b2» (retry, scadenza, `2d387cd9…`) e «b3» (100.000 timer, aggiornamento con i timer, `2d0fda73…` = il codice committato, dopo `cargo fmt`) mostrano gli stessi esiti, ma nessuna riga è stata rifatta in tre corse con il codice committato. Il commit del sorgente di `175e97d3…` non esiste (il lavoro è in due commit, motore e risultati, dopo i battiti).
- **Aggiornamento rifatto, conteggio doppio:** lo script contava le attività eseguite due volte a carico finito ma non a corsa finita; la prima corsa (r0) ha quel difetto e l'ho tenuta come corsa in più; le tre del confronto sono r1-r3 con il conteggio dopo la verifica. Per l'aggiornamento con i timer il conteggio e i contatori del motore nuovo sono nello script dall'inizio.
- **Script cambiati tra una corsa e l'altra, non il binario della serie:** `scen.sh`: la copia del registro del motore prima del crash è passata da prima a dopo il kill (le tre corse retry usano la prima; tutti i timer erano già scritti prima del kill, nessun effetto), il parametro `DWORK` (prima era 1 s fisso: le due corse `deadline-grace` sono con 1 s e le tre `deadline` con 3 s), la scena `theart`. `up-run.sh`: parametri `TRATE`, `TSECS` per le 100.000 attese. `brun.sh`: `BRUN_ITERS` (attesa in primo piano di 2,5 minuti: lo strumento sposta in secondo piano un comando che dura oltre 3 minuti; una volta è successo con due corse di fila e ho aspettato la sua fine senza lanciarne altre).
- **Corse in più tenute:** `deadline-grace` r1-r2 (finestra di `grace_ms`, prima forma della riga scadenza), `…-r0-conteggio-anticipato`, le due «b2», i battiti. La prima prova dei battiti è stata sovrascritta: la scena non era ancora sul sorgente caricato su minisheep («scenario sconosciuto»), nessun dato.
- **Numero dei timer:** 100.005 / 100.000 / 100.000 (il generatore offre per tempo, non per numero).
- **Riga retry con 2 s di pausa e non 0:** scelta mia, per provare insieme i timer scaduti e non scaduti alla fine della ripresa; non ho una variante a riavvio immediato.
- **Finestra di `grace_ms` dopo la ripresa** (`due.max(READY + grace_ms)`, solo in memoria) non era nel piano: è una scelta di progetto fatta prima delle corse, per la corsa tra l'esito di un worker che ha finito durante la pausa e una scadenza assoluta già passata; **nessuna corsa è stata fatta senza la finestra**, quindi il suo bisogno è ipotesi. Il ritardo di scatto di una scadenza dentro la finestra arriva a `grace_ms` (2 s con le attività del piano).
- **Definizioni dello scarto di scatto:** per retry, scadenza e battiti è l'inizio del tentativo nel worker meno la scadenza (comprende scrittura durevole e consegna); per i `wait` è l'istante dell'evento `Done` meno la scadenza (decisione, prima della scrittura durevole); la «presa in carico» è l'istante in cui il motore prende il timer meno la scadenza.
- **Postgres:** nessuna corsa del blocco C (tutte SQLite). Il codice del proiettore per i timer (riga unica per attività in ogni `INSERT … ON CONFLICT`, `DELETE … WHERE (exec_id, step) IN (…)`) non è stato provato su Postgres.
- **Prove sul desktop non misurate né conservate:** le prove funzionali di retry, scadenza, 100.000 (ridotte a 40.000), aggiornamento con timer ridotti e battiti, per mettere a punto gli script; il desktop usa un percorso dati proprio (`~/.cache/ttspikebus-data-4c`) e docker `ttspikebus`, fermato a ogni giro.
- **Tempo:** circa un giorno di lavoro tra codice (eventi, tabella, ripresa, riversamento, battiti), prove sul desktop e corse; nessuna causa in Iggy, nessuna riga ha chiesto di fermarsi.

### Non coperto

- **Costo dei timer sul percorso caldo.** Ogni consegna ora scrive un evento `Timer` sul bus prima dell'invio e il proiettore fa un'istruzione `INSERT … ON CONFLICT` per gruppo più un `DELETE` per ogni gruppo con esiti: né la portata a 1.000 e 10.000 att./s né le istruzioni SQL per attività (NFR-4) sono rimisurate con i timer attivi (la riga dei 10.000/s del blocco B e C1 sono senza timer). Da fare prima dell'ADR.
- **Battiti a scala:** un evento per battito per tentativo; a decine di migliaia di tentativi in volo il traffico sul bus diventerebbe dominante. Una scelta (battiti solo sul log del tentativo, o aggregati per worker) non è stata provata.
- **Postgres** per il codice dei timer; **crash a metà riversamento** (già non provato nel blocco A) e con i timer; **spegnimento dell'host**.
- **Più crash insieme** (motore e worker nella stessa finestra) oltre al caso della riga scadenza; **crash tra scrittura del timer e invio** come caso provocato (sospettato in 3 delle 6 corse con scadenza, v. C3.2).
- **Orologio:** il motore confronta le scadenze con l'orologio del suo processo; un salto dell'orologio tra due incarnazioni (ora dell'host cambiata durante il riavvio) non è provato.
- **Scadenze a scala oltre 100.000** o con picchi di oltre 4.096 per passata di 20 ms (200.000/s): non provati.
- Tutte le righe a due tenant sulla stessa partizione per tenant; la scansione dei timer non è per tenant.

### Comandi per rifare (blocco C)

Su minisheep, in `~/ttspikebus` (`env.sh`, `.env` da `./gen-env.sh`, binari `bin/busspike` e `bin/busspike-old`; i due binari «vecchi» con `cargo build --release --features old`). Una corsa alla volta, in primo piano con `BRUN_ITERS=30 ./brun.sh <file> <comando>` e `./brun.sh wait` se serve.

| Misura | Comando |
| --- | --- |
| C1 percorso dei dati (strato / montaggio) | `IGGY_MNT=/local_data ./brun.sh risultati/iggy-sqlite-path-layer-accept-4-ms-r1.txt ./scen.sh iggy sqlite accept` (montaggio: senza `IGGY_MNT`); `start1k` allo stesso modo; `GATE60=1 ./brun.sh risultati/iggy-sqlite-path-mount-thr10k-4-ms-r1.txt ./with-sampler.sh risultati/iggy-sqlite-path-mount-thr10k-4-ms-r1.io ./scen.sh iggy sqlite thr10k secs=600` |
| C2 aggiornamento rifatto (binari del blocco B) | `CAP=4096 ./brun.sh risultati/iggy-sqlite-upgrade-rerun-4-ms-r1.txt ./up-run.sh n=3 rate=334` |
| C3.1 retry attraverso un crash | `./brun.sh risultati/iggy-sqlite-timer-retry-4-ms-r1.txt ./scen.sh iggy sqlite tretry` (`PAUSE=0` per il riavvio immediato) |
| C3.2 scadenza attraverso un crash | `./brun.sh risultati/iggy-sqlite-timer-deadline-4-ms-r1.txt ./scen.sh iggy sqlite tdead` (`DWORK=1000` per la finestra di `grace_ms`) |
| C3.3 100.000 timer | `./brun.sh risultati/iggy-sqlite-timer-100k-4-ms-r1.txt ./scen.sh iggy sqlite t100k` |
| C3.4 aggiornamento con i timer | `PRE100K=1 CAP=4096 ./brun.sh risultati/iggy-sqlite-timer-upgrade-4-ms-r1.txt ./up-run.sh n=3 rate=334` |
| C3.5 battiti | `./brun.sh risultati/iggy-sqlite-timer-heartbeat-4-ms-r1.txt ./scen.sh iggy sqlite theart` (`LEASE`, `GRACE`, `HWORK`) |
| Scarto di scatto da registri | `./tstat.sh <retry\|deadline> out/engine-pre.log out/engine.log out/work.log <READY us>` |

## Blocco D

Costo dei timer sul percorso caldo, prima del log dei task. **Il criterio non è superato: il log dei task (FR-59) non è stato costruito** (regola del piano e del compito: se il costo dei timer peggiora oltre il 20 %, ci si ferma con i numeri). Minisheep, stessi 4 CPU (motore e Iggy su `1-4`, carico su `8-11`), `ms-run.sh` con pavimento di `fdatasync` prima e dopo; ogni file grezzo porta le righe `braccio:` e `binario sha256:`. File grezzi `risultati/iggy-sqlite-<riga>-4-ms-r<n>.txt`; la regola (metrica, aggregazione, soglie) è stata scritta e committata prima delle corse a coppie, in `risultati/iggy-sqlite-timercost-regola-4-ms.txt` (commit `b58975e`).
Due binari, ognuno per una serie intera (nessuna sostituzione a metà serie):

| Binario (sha256, primi 16) | Braccio | Righe |
| --- | --- | --- |
| `ef60a7e23befacd0` | `d0` e `H`: sorgente del commit `5117575` (HEAD del blocco C; nessun commit sul sorgente dopo) | retry e scadenza (passo 0), braccio H del costo dei timer |
| `d730b1a73111ca79` | `B`: sorgente del commit `7f156ba` (blocco B, senza timer), da `git archive 7f156ba spikes/bus` in una cartella a parte | braccio B del costo dei timer |

Gli hash dei binari del blocco C (`2d0fda73…` per il «codice committato») non coincidono con `ef60a7e2…`. Il sorgente è lo stesso (nessun commit su `spikes/bus/src` dopo `5117575`); che la differenza venga dai percorsi della costruzione è **ipotesi**, non verificata.

### Sintesi

1. **Retry e scadenza sul codice committato: superati** (una corsa ciascuna, come chiesto). Retry: 300 ritentativi su 300 partiti una volta sola, 0 prima della scadenza, i già scaduti a fine ripresa 157 ms dopo `READY` (soglia 1 s). Scadenza: 100 su 100 rifatti una volta sola, 0 prima della scadenza, inizio 18,7 ms (p50) e 32,3 ms (p99) dopo la scadenza.
2. **Costo dei timer a 1.000 att./s: l'avvio peggiora di 2,9 volte, con segno coerente nelle due posizioni.** Mediana del p99 dell'avvio (`start_ready`) 10,5 ms (B) contro 30,3 ms (H); p50 7,8 contro 22,3 ms. Criterio (rapporto ≤ 1,20) **non superato**. Contro le soglie assolute del PRD (avvio p99 ≤ 25 ms a 1.000 att./s): B dentro (10,5 ms), **H fuori (30,3 ms)**. L'avvio p50 ≤ 5 ms è sopra in entrambi i bracci (7,8 e 22,3 ms), riportato non giudicato come nelle fasi 2c e 2d.
3. **A 10.000 att./s con i timer il motore non regge il ritmo:** 4.518 e 5.354 att./s (45 e 54 %) contro 9.982 e 9.987 (99,8 %) senza i timer; la soglia del 99 % non è raggiunta in nessuna delle due corse H. Con i timer l'avvio p50 arriva a 41-42 s (la coda del lavoro cresce), senza 67-68 ms.
4. **Istruzioni SQL per attività (NFR-4):** 0,0324 (B) contro 0,0412 (H) a 1.000 att./s; 0,0155-0,0157 contro 0,0318-0,0326 a 10.000 att./s (nelle corse H le attività concluse nella finestra sono meno della metà, quindi il rapporto per attività sale anche per questo). Restano due ordini di grandezza sotto il limite di 5; le righe toccate per attività passano da 2,2 a 4,2-4,4. **Il limite delle istruzioni SQL regge (misurata). Dove stia il costo è ipotesi:** il tempo del proiettore sul database per minuto si sovrappone tra i bracci (`proj_db_p50` nei minuti 4-10: B 12,6-21,2 ms, H 13,7-36,9 ms), mentre l'append per gruppo raddoppia (v. D1.2).

### Passo 0. Retry e scadenza sul binario committato (`iggy-sqlite-timer-{retry,deadline}-d0-4-ms-r1.txt`, binario `ef60a7e23befacd0`)

Stesse scene e stessi criteri del blocco C (`./scen.sh iggy sqlite tretry` e `tdead`, una corsa ciascuna, 2 s di pausa voluta nella riga retry).

| Misura | Retry (C3.1) | Scadenza (C3.2) |
| --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,74 > 2,85 | 6,08 > 2,79 (nel «prima» un campione a 111 ms, `over100ms=1`) |
| kill del motore → `READY` (ms) | 2.092 (di cui 2.000 voluti) | 76 |
| Ripresa: in attesa di un timer / in volo con scadenza ripristinata / da consegnare subito | 300 / 0 / 0 | 0 / 250 / 1 |
| Tentativo 1 iniziato: totale / più di una volta / prima della scadenza | 300 / **0** / **0** | 100 / **0** / **0** |
| Già scaduti a `READY`: inizio − `READY` p50 / p99 / max (ms) | 146: 157,0 / 157,3 / 157,3 | — |
| Inizio − scadenza p50 / p99 / max (ms) | non scaduti a `READY` (154): 24,6 / 134,6 / 150,0 | 99 con timer noto: 18,7 / 32,3 / 33,5 |
| Presa in carico − scadenza, solo motore, p50 / p99 (ms) | 20,0 / 1.411,6 (per i timer scaduti comprende i 2 s di pausa voluta: non è un ritardo di scatto) | 9,4 / 19,7 |
| Tentativo 0 eseguito più di una volta; accettate perse | 0; 0 (verifica OK al secondo tentativo) | 0; 0 (verifica OK al quarto tentativo) |

Criterio **superato** per entrambe. Le righe `verify FALLITA` che precedono l'`OK` sono i tentativi del ciclo di attesa dello script (le attività non sono ancora concluse), come nel blocco C. Differenze dai numeri del blocco C, riportate senza spiegazione: nei ritentativi non scaduti a `READY` il p99 è 134,6 ms (blocco C: 44-61 ms) e il max 150,0 ms (62-73 ms); i già scaduti partono 157 ms dopo `READY` (73-92 ms). **La causa non è stata indagata** (una corsa per riga, nessuna ipotesi misurata). Nella scadenza 1 tentativo su 100 non ha un timer di scadenza noto nel registro (`senza timer 1`), come in 3 delle 6 corse del blocco C (ipotesi di C3.2 invariata, non provata).

### D1. Costo dei timer sul percorso caldo

Bracci: **B** (blocco B, senza timer) e **H** (commit `5117575`: evento `Timer` scritto sul bus prima di ogni consegna, `INSERT … ON CONFLICT` e `DELETE` nel proiettore). Ogni braccio ha il suo binario per motore, worker e carico. Metrica: `start_ready` p99 (dalla decisione «pronta» alla ricezione nel worker).

#### D1.1 `start1k` a 1.000 att./s, 6 coppie (`iggy-sqlite-timercost-start1k-{B,H}-4-ms-r{1..6}.txt`)

Ordine: coppie 1, 3, 5 con B per prima; coppie 2, 4, 6 con H per prima (in ordine di tempo B1 H1, H2 B2, B3 H3, H4 B4, B5 H5, H6 B6).

| Coppia | B: pavimento prima > dopo (ms) | B: accettazione p99 / `start_ready` p50 / p99 (ms) | H: pavimento prima > dopo (ms) | H: accettazione p99 / `start_ready` p50 / p99 (ms) | H − B, p99 dell'avvio (ms) |
| --- | --- | --- | --- | --- | --- |
| 1 (B prima) | 2,75 > 2,67 | 17,92 / 7,68 / 10,01 | 3,11 > 2,79 | 35,26 / 23,73 / 30,30 | +20,29 |
| 2 (H prima) | 1,90 > 2,79 | 18,91 / 8,21 / 11,03 | 2,93 > 2,75 | 35,94 / 23,82 / 30,78 | +19,75 |
| 3 (B prima) | 7,05 > 2,74 | 17,84 / 7,64 / 9,80 | 7,17 > 1,93 | 34,72 / 22,54 / 29,95 | +20,15 |
| 4 (H prima) | 2,76 > 2,83 | 18,13 / 8,12 / 10,54 | 6,09 > 2,83 | 34,81 / 21,86 / 30,37 | +19,83 |
| 5 (B prima) | 2,90 > 2,86 | 18,83 / 8,00 / 11,28 | 6,90 > 2,88 | 34,85 / 22,05 / 30,34 | +19,06 |
| 6 (H prima) | 2,79 > 2,66 | 18,30 / 7,53 / 10,53 | 7,52 > 2,54 | 34,43 / 22,05 / 27,54 | +17,01 |
| **Mediana per braccio** | | **18,22 / 7,84 / 10,54** | | **34,83 / 22,30 / 30,32** | |

- **Rapporto H/B delle mediane del p99 dell'avvio: 2,88** (soglia 1,20). Accettazione p99: 1,91; `start_ready` p50: 2,84. Tutte le corse H sono più lente di tutte le corse B (il p99 migliore di H, 27,54 ms, è 2,4 volte il peggiore di B, 11,28 ms).
- **Segno coerente nelle due posizioni** (misurata): mediana delle differenze con B prima +20,15 ms (coppie 1, 3, 5), con H prima +19,75 ms (coppie 2, 4, 6). La differenza si dichiara. **Criterio non superato.**
- L'alternanza lenta e veloce di 200 e 1.000 att./s del blocco C non si vede qui in nessuno dei due bracci: B resta tra 7,53 e 8,21 ms di p50, H tra 21,86 e 23,82 ms. Cinque corse hanno un pavimento «prima» di 6,1-7,5 ms (una B, quattro H) e non si distinguono dalle altre dello stesso braccio. Il cancello di quiete non controlla il pavimento (solo carico e pressione I/O).
- **Istruzioni SQL per attività** (misurata): B 0,0322-0,0324, H 0,0411-0,0412; righe toccate per attività 2,20-2,22 contro 4,20-4,21, cioè +2,0 righe, coerente con una riga `timer` inserita e una cancellata per ogni consegna (la differenza è misurata, la spiegazione viene dal codice del proiettore).

#### D1.2 `thr10k` a 10.000 att./s per 10 minuti, 2 coppie (`iggy-sqlite-timercost-thr10k-{B,H}-4-ms-r{1,2}.txt`, più i campioni di I/O `.io.names`)

Come B5: 1.000 esecuzioni/s da 10 passi, due tenant, `trim=1`, registro delle accettate, `verify light`, `GATE60=1`. Ordine in tempo: B r1, H r1, H r2, B r2 (una coppia per ordine). Soglia scritta prima: almeno il 99 % (9.900 att./s) sull'intera finestra misurata, accettate perse 0.

| Misura | B r1 | H r1 | H r2 | B r2 |
| --- | --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 2,46 > 10,76 | 9,73 > 4,60 | 4,89 > 4,51 | 2,94 > 4,66 |
| Attesa di quiete (s) | 165 | 190 | 140 | 165 |
| Offerte / accettate | 605.000 / 603.932 | 605.001 / 598.898 | 605.001 / 600.582 | 605.000 / 604.135 |
| Rifiutate `E_PROJECTION_DOWN` / errori del client | 745 / 323 | 1.254 / 4.849 | 0 / 4.419 | 799 / 66 |
| **Attività concluse al secondo** (finestra misurata) | **9.982** | **4.518** | **5.354** | **9.987** |
| Minuti 2-10 sotto 9.900 att./s | 3 (9.891, 9.870, 9.788) | 9 su 9 | 9 su 9 | 0 |
| Intervallo più lungo senza accettazioni (ms) | 1.255 | 3.203 | 3.973 | 905 |
| `start_ready` p50 / p99 | 66,8 ms / 1,37 s | 42,3 s / 128 s | 41,2 s / 70,6 s | 68,0 ms / 0,89 s |
| Ritardo di proiezione p99, massimo per minuto nei 10 minuti (ms) | 3.357 | 4.112 | 1.335 | 2.249 |
| Append p50 per gruppo, per minuto (min-max, ms) | 23,9-66,3 | 124,1-414,0 | 123,8-282,9 | 65,3-66,6 |
| Istruzioni SQL per attività (finestra) | 0,0157 | 0,0326 | 0,0318 | 0,0155 |
| **Accettate perse** (registro contro database) | 0 (verifica OK) | **non determinabile**: 462.770 accettate non ancora concluse dopo 36 s | **non determinabile**: 56.110 non ancora concluse dopo 5 minuti (in calo) | 0 (verifica OK) |

Il minuto 1 di avvio è sotto 9.900 in B r1 (8.961) e B r2 (8.255) e non conta per la soglia, che è sull'intera finestra.

- **Il braccio H non supera il 99 % in nessuna delle due corse** (45 % e 54 %), nelle due posizioni (misurata). Il braccio B lo supera in entrambe (99,8 %), con zero accettate perse.
- **Il braccio H resta a 4.782-6.119 att./s anche senza carico nuovo** (misurata: righe `MIN` t=11-18 di H r2, dopo la fine del carico, mentre il motore smaltisce la coda), con la durata della singola richiesta a Iggy invariata (`sub_p50` 16,5-16,9 ms) e l'append per gruppo a 120-129 ms. Non è una pausa passeggera di Iggy (misurata); quale parte del percorso saturi è ipotesi (sotto).
- **«Accettate perse» in H non determinabile, non «perse»:** il carico finisce con il motore ancora in coda; la verifica attende 36 s (r1) e 5 minuti (r2, `VTRIES=100`, aggiunto per questo) e le accettate non concluse calano senza arrivare a zero; poi lo script ferma il motore. Nessuna perdita osservata e nessuna prova che non ce ne siano.
- **Causa del dimezzamento: ipotesi, non provata.** È misurato: +2,0 righe toccate per attività nel database (2,2 → 4,4; un evento `Timer` in più per consegna, dal codice); l'avvio a 1.000 att./s sale da 7,8 a 22,3 ms di p50, +14,5 ms. Che sia una sola scrittura durevole in più è ipotesi: l'intero percorso senza timer vale 7,8 ms di p50, quindi +14,5 ms è vicino a due cicli; a 10.000 att./s l'append p50 per gruppo sale da 66 a 124-129 ms mentre la durata della singola richiesta resta uguale nei minuti non disturbati (`sub_p50` 16,6-17,1 ms nei minuti 4-10 di tutte le corse, tranne i minuti 8 e 9 di H r1 con 28,0 e 48,3 ms). Ipotesi: con il tetto di `cap=512` consegne in volo per tenant (`scen.sh`, `CAP`) il tempo di ciclo di ogni attività raddoppia per l'attesa del timer e, per la legge di Little, il ritmo massimo si dimezza (1.024 in volo / 10.000 att./s ≈ 100 ms contro 1.024 / 5.500 att./s ≈ 186 ms). Nessuna corsa con `CAP` più alto: il piano chiede di fermarsi dopo la riga.

### Cosa è cambiato nel codice dello spike

Nessuna modifica al motore. Solo script: `env.sh` (nuovo nel repository: `ARM_BIN` sceglie il binario dopo la lettura di `env.sh`; prima `brun.sh` lo sovrascriveva sempre con `bin/busspike`), `ms-run.sh` (riga `braccio:` accanto all'hash), `scen.sh` (`VTRIES`, predefinito 12 = 36 s come prima, per l'attesa della verifica di `thr10k`).

### Deviazioni dal piano (blocco D)

- **Il blocco si ferma dopo la riga del costo dei timer**: log dei task, RustFS, `If-None-Match`, visibilità, worker rumoroso, segreto, crash con righe in volo e tetto per tentativo **non sono stati fatti** (regola del compito e del piano). Nessun codice del log scritto.
- **Braccio B** costruito da `git archive 7f156ba spikes/bus` in `~/.cache/ttspike-d4-blockB`, con la sua cartella di destinazione di `cargo`; non è il binario `d51b36de…` del blocco B (l'hash dipende dai percorsi).
- **`thr10k` H r1 con verifica di 36 s, H r2 e B r2 con `VTRIES=100`:** H r1 è stata fatta prima di avere `VTRIES`; le accettate perse di H r1 e H r2 restano non determinabili. B r2 ha usato la stessa impostazione (esce al primo `OK`).
- **`start1k` senza `GATE60`** (cancello di quiete come nel blocco C); `thr10k` con `GATE60=1` come B5.
- **Ordine a 10.000/s:** B r1, H r1, H r2, B r2 (una coppia per ordine, come chiesto).
- **Passo 0:** una sola corsa per riga, come chiesto; le tre del blocco C non sono rifatte.
- **File temporanei fuori dalla mia cartella:** due file di somme md5 degli script scritti per errore in `/tmp/claude-1000/` e tolti subito per nome; il lavoro temporaneo sta in `~/.cache/ttspike-d4-work` e `~/.cache/ttspike-d4-blockB`.
- **Il worktree** era partito da `main` e l'ho portato a `9ba2b5f` con `git reset --hard` (pulito, nessun commit mio prima).
- **Tempo:** circa un'ora e mezza di corse su minisheep (12 da 1,5 minuti, 4 da 10 minuti con attese), più la lettura del codice.

### Non coperto

- Tutto il log dei task (FR-59): v. sopra.
- **Una correzione del costo dei timer** (per esempio un tetto di consegne in volo più alto, scrivere il timer di scadenza nello stesso gruppo del `Ready`, o nessun timer durevole per le attività brevi): non provata. È la decisione per Marco: con il timer scritto prima della consegna (AD-2) il percorso caldo ha una scrittura durevole in più; o il 99 % a 10.000 att./s si ottiene in un altro modo, o la consegna senza timer durevole per le attività brevi va nell'ADR.
- Postgres per i timer (come nel blocco C); le accettate perse di H a 10.000/s; i limiti del blocco C (spegnimento dell'host, orologio, battiti a scala).

### Comandi per rifare (blocco D)

Su minisheep, in `~/ttspikebus` (`env.sh` dal repository, `.env` da `./gen-env.sh`, binari `bin/busspike-bB` e `bin/busspike-d0`). Una corsa alla volta; `ARM_BIN` e `ARM` nell'ambiente scelgono il binario e il nome del braccio.

| Misura | Comando |
| --- | --- |
| Passo 0 | `ARM_BIN=$HOME/ttspikebus/bin/busspike-d0 ARM=HEAD-5117575 BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timer-retry-d0-4-ms-r1.txt ./scen.sh iggy sqlite tretry` (`tdead` per la scadenza) |
| D1.1 | `ARM_BIN=$HOME/ttspikebus/bin/busspike-bB ARM=B-blockB-7f156ba BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost-start1k-B-4-ms-r1.txt ./scen.sh iggy sqlite start1k` (H: `busspike-d0`, `ARM=H-head-5117575`) |
| D1.2 | `GATE60=1 VTRIES=100 ARM_BIN=… ARM=… BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost-thr10k-H-4-ms-r2.txt ./with-sampler.sh risultati/iggy-sqlite-timercost-thr10k-H-4-ms-r2.io ./scen.sh iggy sqlite thr10k secs=600`, poi `./brun.sh wait` finché non finisce |

## Blocco D, seconda parte

Misura prima, il disegno non cambia finché non ci sono numeri (decisione di Marco, 2026-11-25). Due prove sul costo dei timer del blocco D: (a) tetto di consegne in volo più alto sul binario H esistente, senza codice; (b) una modifica del motore, «invia prima, registra dopo» (braccio **H2**). Minisheep, stessi 4 CPU (motore e Iggy su `1-4`, carico su `8-11`), `ms-run.sh` con pavimento di `fdatasync` prima e dopo; ogni file grezzo porta le righe `braccio:` e `binario sha256:`. File grezzi `risultati/iggy-sqlite-timercost2-<riga>-4-ms-r<n>.txt`. La regola di lettura (metrica, soglie, controlli) è stata scritta e committata **prima** della prima corsa, in `risultati/iggy-sqlite-timercost2-regola-4-ms.txt` (commit `c5df9fa`, 08:17; la prima corsa parte alle 08:20:31). Il log dei task (FR-59) non è stato costruito.

| Binario (sha256, primi 16) | Braccio | Righe |
| --- | --- | --- |
| `d730b1a73111ca79` | `B`: blocco B, commit `7f156ba` (senza timer) | coppie `start1k` e `thr10k` |
| `ef60a7e23befacd0` | `H`: commit `5117575` (blocco C, timer durevole prima della consegna) | corsa a `CAP=2048` |
| `1a1348abf3bea374` (+ `4478461c59299ba9`, `--features old`) | `H2`: commit `c2ac38a` (la modifica); il commit dopo di lui (`ef33d90`) tocca solo gli script | controlli di correttezza, coppie, aggiornamenti (il binario `--features old` è il «motore vecchio» degli aggiornamenti, dallo stesso sorgente) |

### Sintesi

1. **Tetto di consegne a 2.048 (braccio H, nessun codice): l'ipotesi «tetto per tenant e ciclo allungato» è supportata dalla regola, con riserve.** 9.953 att./s sull'intera corsa, 9.968 att./s di media dei minuti 2-10 (soglia 9.900), accettate perse 0 (verifica `OK` al secondo tentativo). Nel blocco D, con `CAP=512`, la stessa riga dava 4.518 e 5.354 att./s. Riserve (tutte misurate): tre minuti su nove sotto 9.900 (9.877, 9.433 e **5.116** nel minuto 7, con 15,8 s di ritardo di proiezione e due «database is locked»), 6.554 errori del client (`ResourceExhausted`, h2), `start_ready` p50 250,6 ms e p99 7,59 s, e **12.364 scadenze di tentativo** (`expired=12.364`, `rejected=12.364`: i tentativi che aspettano nel worker più di 2 s, il margine `grace_ms`, scadono e si rifanno). L'append per gruppo resta a 125 ms (p50, minuti 4-10) invece dei 66 ms del braccio B: la velocità si recupera con più consegne in volo, non con un percorso più corto. Una corsa sola: è la riga del piano.
2. **H2 a 1.000 att./s: il peggioramento rispetto a B scende da 2,9 a 1,83 volte, il rapporto di 1,20 resta non superato, il limite assoluto del prodotto è rispettato.** Mediana del p99 di `start_ready`: B 10,81 ms, **H2 19,83 ms** (H del blocco D: 30,32 ms); p50 7,99 contro 14,84 ms (H: 22,30). Rapporto H2/B 1,83 (soglia 1,20), segno coerente nelle due posizioni (+8,32 ms con B prima, +10,19 ms con H2 prima): la differenza si dichiara, criterio del rapporto **non superato**. Limite assoluto (p99 ≤ 25 ms): **superato** in tutte e sei le corse H2 (19,05-21,10 ms) e nella mediana. L'avvio p50 ≤ 5 ms resta sopra in tutti i bracci, riportato non giudicato.
3. **H2 a 10.000 att./s con `CAP=512`: superato.** Quattro corse su quattro sopra 9.900 att./s, accettate perse 0: H2 9.983 e 9.993 (medie dei minuti 2-10: 9.962 e 10.000), B 9.994 e 10.000. Nel blocco D il braccio H faceva 4.518 e 5.354. Le attività trattenute dal tetto (`held_max` per minuto) arrivano a 930 e 1.011: il tetto morde ancora, ma non limita più la portata (misurata). L'avvio a 10.000/s resta più lento di B (p50 136,6 / 137,2 ms contro 66,2 / 67,8 ms; p99 405,5 / 399,4 ms contro 144,5 / 153,0 ms).
4. **Invarianti di H2 (misurate).** Zero righe `timer` per attività concluse in tutte le corse `start1k` e `thr10k` di H2 e nelle tre di kill sotto carico; zero timer persi o doppi e zero attività rifatte negli aggiornamenti (pausa 3.352 ms senza timer in attesa, 7.023 ms con 100.000); retry e scadenza attraverso un crash come nel blocco C.
5. **Regola della specifica (AD-2): l'invio di H2 aspetta ancora il gruppo durevole che rende pronta l'attività, non quello del suo timer di scadenza.** `file:line` sotto, in «Regola della specifica». La modifica è compatibile con la regola se questa chiede la scrittura durevole della decisione che causa l'effetto (la prontezza); non lo è se chiede anche la scadenza durevole prima dell'invio. La scelta è di Marco.
6. **Dove stia il costo che resta (H2 contro B a 1.000/s, +6,9 ms di p50) è ipotesi.** Il gruppo di scrittura del `Timer` non si aspetta più, ma occupa ancora la coda di scrittura dell'esecuzione (una connessione, `wconn=1`): un gruppo `Done`/`Ready` successivo della stessa catena aspetta dietro di lui. Non isolato: nessuna corsa con il `Timer` fuori dalla coda o con più connessioni.

### Passo 1. Tetto alto, nessun codice (`iggy-sqlite-timercost2-thr10k-H-cap2048-4-ms-r1.txt`, binario `ef60a7e23befacd0`, `CAP=2048`)

`GATE60=1 VTRIES=100`, 10 minuti, due tenant, `trim=1`, registro delle accettate, `verify light`. Pavimento prima > dopo (p50): 2,85 > 4,37 ms; attesa di quiete 0 s (macchina già ferma).

| Misura | Valore |
| --- | --- |
| Attività concluse al secondo: statistiche dell'intera corsa / media dei minuti 2-10 | **9.953 / 9.968** |
| Minuti 2-10 sotto 9.900 att./s | 3 su 9 (t3 9.877, t6 9.433, t7 5.116) |
| Offerte / accettate / rifiutate `E_PROJECTION_DOWN` / errori del client | 605.001 / 597.542 / 905 / 6.554 |
| Intervallo più lungo senza accettazioni (ms) | 3.138 |
| `start_ready` p50 / p99 | 250,6 ms / 7,59 s |
| Append p50 per gruppo, per minuto (min-max, ms) | 63,0-644,1 (125 ms nei minuti 4-10 non disturbati) |
| Ritardo di proiezione p99 | 14,99 s (minuti 7-8 fino a 15,8 s) |
| Scadenze di tentativo / esiti scartati | 12.364 / 12.364 |
| Istruzioni SQL per attività / righe | 0,0293 / 4,205 |
| Accettate perse | **0** (la prima verifica a fine carico `FALLITA` con 4.476 non ancora concluse, la seconda `OK`) |
| Righe `timer` per attività concluse | 0 (totale 0) |

Lettura secondo la regola: ≥ 9.900 att./s con 0 accettate perse, quindi **ipotesi supportata**. La velocità è misurata; la spiegazione «tetto e ciclo» è supportata dal fatto che con quattro volte i tentativi in volo la portata torna al valore senza timer (il binario H è lo stesso, cambia solo `CAP`), non è isolata. Il costo non sparisce: stessa durata dell'append per gruppo (125 ms contro 66 ms di B), una coda di avvio di secondi (`start_ready` p99 7,59 s) e scadenze di tentativo (12.364) che rifanno attività già consegnate. La riga `MIN` di H non ha `held_max` (binario del blocco D).

### Passo 2. La modifica «invia prima, registra dopo» (commit `c2ac38a`, `spikes/bus/src/engine.rs`)

- **Cosa cambia in `dispatch`** (unico punto in cui un tentativo si consegna, quindi il primo tentativo, i ritentativi dopo l'attesa e le riconsegne dopo una scadenza): la consegna al worker parte subito, la scadenza di tipo 0 si arma in memoria nello stesso momento (`set_timer`), e l'evento `Timer` si scrive nel gruppo lanciato in fondo a `dispatch`, **senza aspettarlo** (`Group::sends` tolta; `Group::arm` resta per i tipi 1 e 2 e per i battiti). Tipi 1 e 2 non cambiati. La ripresa non cambia: un'attività senza timer durevole conta come mai consegnata e si consegna subito; ora questo copre anche una consegna il cui `Timer` non era durevole al crash: il **doppione accettato e contato**.
- **Invariante 1, ordine nel log (per costruzione e misurata).** Il gruppo con il `Timer` parte dentro `dispatch`, nello stesso ciclo del motore che non può elaborare l'esito del worker prima di averlo lanciato; le code di scrittura sono FIFO per esecuzione (stessa connessione scelta dall'`exec_id`, `submit`), quindi il `Timer` di un tentativo precede sempre il suo `Done` o il suo fallimento sul bus. Nel proiettore (`db.rs`) gli `INSERT … ON CONFLICT` dei timer precedono i `DELETE` dello stesso gruppo. Controllo dopo ogni corsa `start1k` e `thr10k` di H2: `SELECT count(*) FROM timer t JOIN activity a ON a.exec_id=t.exec_id AND a.step=t.step WHERE a.done_us IS NOT NULL` (scritta nel file grezzo con il conteggio): **0 nelle 6 corse `start1k` e nelle 2 `thr10k`**, e in tutte le altre di H2.
- **Invariante 2, consegna dell'aggiornamento (per costruzione e misurata).** I gruppi con i `Timer` stanno in `Engine::groups` (non contano per `k`); lo svuotamento del motore vecchio esce solo con `groups` vuoto e posizione applicata uguale alla testa (`run`), quindi i `Timer` in sospeso sono sul bus e nel database prima di `rewrite_live`. Misurata: timer riscritti 2.894 (= tentativi in volo allo svuotamento) e 102.921 (= 100.000 + 2.921), zero persi, zero attività rifatte.
- **Aggiunte di strumentazione** (poche righe): `held` e `held_max` (attività pronte trattenute dal tetto, `held_max` nella riga `MIN`), `undurable_deliveries` e `undurable_max` (consegne inviate il cui `Timer` non è ancora durevole, nelle statistiche), `TSET 0` scritto al momento dell'invio (con `TIMER_LOG`).
- Costruito da `c2ac38a` con `cargo build --release` (e `--features old` per il motore vecchio degli aggiornamenti); `cargo fmt` e `cargo clippy --all-targets -- -D warnings` puliti.

#### Regola della specifica (AD-2: nessun effetto esterno prima della scrittura durevole della decisione che lo causa)

- **Blocco B (commit `7f156ba`, `spikes/bus/src/engine.rs`):** `finalize` (riga 627) mette le attività nuove in `pending` solo quando il gruppo che porta il loro `Ready` è durevole (righe 651-653) e poi chiama `dispatch` (656); l'invio al worker è alla riga 432 di `dispatch`. L'invio aspetta quindi il gruppo durevole che rende pronta l'attività.
- **H2 (commit `c2ac38a`, `spikes/bus/src/engine.rs`):** stessa catena: `finalize` (891) mette in `pending` le attività nuove a gruppo durevole (916-918) e chiama `dispatch` (942); l'invio è alla riga 606 di `dispatch`. Anche le riconsegne vengono da timer già durevoli (`fire_due`, 440-490, chiama `dispatch` alla 487) o dalla ripresa (2086, 2106). **L'invio aspetta ancora il gruppo che rende pronta l'attività; NON aspetta il gruppo del `Timer` di scadenza del tentativo che si sta consegnando.** Se la regola chiede anche la scadenza durevole prima dell'invio, H2 non è conforme; se chiede la decisione che causa l'effetto (la prontezza), sì.

### Passo 3. Controlli di correttezza su H2 (una corsa ciascuno, `-h2-4-ms-r1.txt`; tutti superati)

| Controllo | Esito (misurato) |
| --- | --- |
| C3.1 retry attraverso un crash (`tretry`, attesa 5 s, giù 2 s) | 299 ritentativi su 299 iniziati una volta sola, 0 prima della scadenza; i 154 già scaduti a `READY` partono 53,8 ms (p50, p99 56,4) dopo `READY` (soglia 1 s), gli altri 10,1 ms (p50) dopo la scadenza; tentativo 0 eseguito più di una volta 0; accettate perse 0; righe `timer` per concluse 0 |
| C3.2 scadenza attraverso un crash (`tdead`) | 99 su 99 rifatti una volta sola, 0 prima della scadenza, inizio 11,0 ms (p50), 20,8 (p99) dopo la scadenza; tentativo 0 eseguito più di una volta 0; accettate perse 0; timer noto 99 su 99 |
| Kill del motore sotto carico (`ekill`, `ERATE=334 CAP=4096`, 3 s di lavoro, circa 1.000 att./s e 3.000 in volo) | r1 / r2 / r3: attività iniziate due volte **0 / 0 / 0**; riprese «da consegnare subito» 15 / 18 / 20; `undurable_deliveries` subito prima del kill 0 / 0 / 0, massimo della corsa 37 / 31 / 64; accettate perse 0; `late_ok` 3.019 in r1; righe `timer` per concluse 0 |
| C2 aggiornamento rifatto (`up-run.sh n=3 rate=334`, `CAP=4096`) | pausa **3.352 ms**; accettate perse 0; attività iniziate due volte 0; `late_ok` 2.894, `expired` 0; timer riscritti 2.894 |
| C3.4 aggiornamento con 100.000 timer (`PRE100K=1 CAP=4096`) | pausa **7.023 ms** (limite 30 s); accettate perse 0 (due verifiche); 100.000 timer scattati una volta, 0 prima della scadenza (scarto p50 10,3 / p99 20,2 ms); attività iniziate due volte 0; `late_ok` 2.921, `expired` 0; timer riscritti 102.921 |

Soglia del kill sotto carico: «iniziate due volte ≤ consegne dell'ultimo gruppo non durevole»: 0 ≤ qualunque valore, **soddisfatta, ma il percorso del doppione non è stato esercitato**: in tre corse il campione prima del kill vedeva 0 consegne non durevoli (il massimo della corsa 31-64: finestre brevi) e nessuna attività è partita due volte. Non c'è una prova che il doppione accadrà o non accadrà; c'è il limite superiore (il gruppo non durevole) e la ripresa che lo conta. Le 15-20 «da consegnare subito» alla ripresa sono attività pronte non consegnate al kill (nessuna è partita due volte), non doppioni.

### Passo 4. Coppie B contro H2, entrambi i bracci con `CAP=512`

#### D2.1 `start1k` a 1.000 att./s, 6 coppie (`iggy-sqlite-timercost2-start1k-{B,H2}-4-ms-r{1..6}.txt`)

Ordine in tempo: B1 H2-1, H2-2 B2, B3 H2-3, H2-4 B4, B5 H2-5, H2-6 B6 (coppie 1, 3, 5 con B per prima).

| Coppia | B: pavimento prima > dopo (ms) | B: accettazione p99 / `start_ready` p50 / p99 (ms) | H2: pavimento prima > dopo (ms) | H2: accettazione p99 / `start_ready` p50 / p99 (ms) | H2 − B, p99 dell'avvio (ms) |
| --- | --- | --- | --- | --- | --- |
| 1 (B prima) | 1,94 > 2,51 | 18,41 / 7,59 / 11,01 | 2,76 > 2,78 | 33,98 / 14,59 / 19,33 | +8,32 |
| 2 (H2 prima) | 7,13 > 2,88 | 17,95 / 8,06 / 10,86 | 7,13 > 2,85 | 35,68 / 14,88 / 21,10 | +10,24 |
| 3 (B prima) | 1,90 > 2,92 | 17,57 / 8,02 / 10,46 | 2,82 > 2,76 | 34,24 / 14,80 / 20,32 | +9,86 |
| 4 (H2 prima) | 2,84 > 2,74 | 17,98 / 8,05 / 10,81 | 3,35 > 2,81 | 33,92 / 14,91 / 19,20 | +8,39 |
| 5 (B prima) | 2,98 > 2,77 | 17,70 / 7,91 / 10,81 | 2,72 > 2,77 | 33,79 / 14,76 / 19,05 | +8,24 |
| 6 (H2 prima) | 6,00 > 2,78 | 18,18 / 7,95 / 10,80 | 7,18 > 2,92 | 34,46 / 14,95 / 20,99 | +10,19 |
| **Mediana per braccio** | | **17,97 / 7,99 / 10,81** | | **34,11 / 14,84 / 19,83** | |

- **Rapporto H2/B delle mediane del p99 dell'avvio: 1,83** (soglia 1,20). Segno coerente nelle due posizioni: mediana delle differenze con B prima +8,32 ms (coppie 1, 3, 5), con H2 prima +10,19 ms (coppie 2, 4, 6): la differenza si dichiara, **criterio del rapporto non superato**. Tutte le corse H2 sono più lente di tutte le corse B (il p99 migliore di H2, 19,05 ms, è 1,7 volte il peggiore di B, 11,01 ms).
- **Limite assoluto del prodotto (p99 ≤ 25 ms): superato**, mediana 19,83 ms (sei corse tra 19,05 e 21,10 ms). Blocco D, braccio H: 30,32 ms.
- p50: rapporto 1,86 (H2 14,84 ms contro B 7,99 ms; H 22,30 ms). Accettazione p99: B 17,97 contro H2 34,11 ms (H: 34,83 ms): la modifica non cambia la latenza di accettazione.
- Istruzioni SQL per attività (misurata): B 0,0322-0,0324, H2 0,0411-0,0412; righe toccate per attività 2,20-2,21 contro 4,20-4,22: come in H, le due righe `timer` per attività restano (una inserita, una cancellata).
- Righe `timer` per attività concluse dopo ogni corsa H2: **0 / 0 / 0 / 0 / 0 / 0**; righe `timer` totali a fine corsa 25 / 21 / 11 / 11 / 15 / 9 (attività ancora in volo allo spegnimento, non concluse).

#### D2.2 `thr10k` a 10.000 att./s per 10 minuti, 2 coppie (`iggy-sqlite-timercost2-thr10k-{B,H2}-4-ms-r{1,2}.txt`, più i campioni di I/O `.io.names`)

Come B5 e D1.2: 1.000 esecuzioni/s da 10 passi, due tenant, `trim=1`, registro delle accettate, `verify light`, `GATE60=1 VTRIES=100`. Ordine in tempo: B r1, H2 r1, H2 r2, B r2. Soglia scritta prima: almeno 9.900 att./s sulla finestra misurata (minuto 1 escluso), accettate perse 0.

| Misura | B r1 | H2 r1 | H2 r2 | B r2 |
| --- | --- | --- | --- | --- |
| Pavimento prima > dopo (ms, p50) | 1,90 > 7,27 | 4,46 > 4,32 | 4,39 > 4,87 | 2,69 > 3,77 |
| Attesa di quiete (s) | 165 | 175 | 165 | 170 |
| Offerte / accettate | 605.000 / 605.000 | 605.000 / 604.339 | 605.001 / 605.001 | 605.000 / 605.000 |
| Rifiutate `E_PROJECTION_DOWN` / errori del client | 0 / 0 | 661 / 0 | 0 / 0 | 0 / 0 |
| **Attività concluse al secondo** (intera corsa / media dei minuti 2-10) | **9.994 / 9.994** | **9.983 / 9.962** | **9.993 / 10.000** | **10.000 / 10.001** |
| Minuti 2-10 sotto 9.900 att./s | 0 | 2 (t3 9.877, t10 9.769) | 1 (t6 9.801) | 0 |
| Intervallo più lungo senza accettazioni (ms) | 500 | 928 | 537 | 559 |
| `start_ready` p50 / p99 (ms) | 66,2 / 144,5 | 136,6 / 405,5 | 137,2 / 399,4 | 67,8 / 153,0 |
| Ritardo di proiezione p99 (ms) | 345,6 | 537,6 | 553,0 | 357,6 |
| Append p50 per gruppo, per minuto (min-max, ms) | 23,9-66,5 | 117,6-128,8 | 125,0-130,2 | 65,5-66,3 |
| `held_max` per minuto (min-max) | — | 202-930 | 289-1.011 | — |
| Istruzioni SQL per attività | 0,0157 | 0,0293 | 0,0292 | 0,0155 |
| **Accettate perse** | 0 | 0 | 0 | 0 |
| Righe `timer` per attività concluse | — (B non ha la tabella) | **0** | **0** | — |

- **Superato in tutte e 4 le corse** (misurata): ≥ 9.900 att./s sull'intera corsa e sulla media dei minuti 2-10, accettate perse 0. Il confronto tra bracci si riporta con i numeri, senza mediana (2 coppie): H2 è sotto B di circa 0,1 % sull'intera corsa; nelle corse H2 un minuto isolato scende a 9.769-9.877 att./s (in B nessuno).
- **Le istruzioni SQL per attività restano due ordini di grandezza sotto il limite di 5** (NFR-4, misurata): 0,0292-0,0293 (H2) contro 0,0155-0,0157 (B).
- **Il costo che resta a 10.000/s è nel tempo di avvio e nell'append, non nella portata** (misurata): p50 dell'avvio 136-137 ms contro 66-68 ms, append p50 per gruppo 118-130 ms contro 66 ms (come nel blocco D, braccio H: 124-129 ms). Che dipenda dal gruppo `Timer` in più per consegna è ipotesi (stesso numero di eventi sul bus di H, v. sintesi 6).

### Cosa è cambiato nel codice dello spike

- `spikes/bus/src/engine.rs` (commit `c2ac38a`): `dispatch` invia subito e arma la scadenza in memoria; `Group::sends` e `Engine::sends` tolte; `Group::deliveries`, `Shared::{undurable, undurable_max, held, held_max}`; riga `MIN` con `held_max`; statistiche con `undurable_deliveries`, `undurable_max`, `held`; `TSET 0` al momento dell'invio.
- `spikes/bus/scen.sh` (commit `ef33d90`): a fine corsa (SQLite) conteggio delle righe `timer` per attività concluse e totali, con la SQL scritta nel file grezzo; la scena `ekill` stampa `undurable_deliveries` prima del kill.

### Deviazioni dal piano (blocco D, seconda parte)

- **Worktree:** partito da `main`, portato a `5775283` con `git reset --hard` (pulito, nessun commit mio prima).
- **Kill del motore sotto carico:** la scena è `ekill` (blocco B) con `ERATE=334 CAP=4096` (circa 1.000 att./s con attività da 3 s, quindi 3.000 in volo; con `CAP=512` il carico non sarebbe sostenibile). Una corsa richiesta; ne ho tenute tre (r2 e r3 in più, per cercare il doppione: non è comparso). Il conteggio degli iniziati due volte è già nella scena (`work.log`): nessun meccanismo nuovo, solo la riga di `undurable_deliveries`.
- **Soglia di kill sotto carico:** campione di `undurable_deliveries` preso subito prima del kill (una chiamata delle statistiche, pochi ms prima del segnale), non al suo istante, che non si può osservare; il massimo della corsa è il limite superiore riportato. La soglia è banalmente soddisfatta (0) e il percorso non è esercitato (v. sopra).
- **`TSET` di tipo 0 in H2** si scrive all'invio, non a gruppo durevole come in H: `tstat.sh` di retry e scadenza continua a funzionare, ma la scadenza del tentativo non è più «scrittura durevole + 15-20 ms».
- **Modifica a tutti i tentativi, non solo al primo:** `dispatch` è l'unico punto di consegna, quindi anche i tentativi dopo un ritentativo (tipo 1) e dopo una scadenza (tipo 0) inviano prima e registrano dopo. Effetto: un crash prima che il `Timer` del tentativo `n+1` sia durevole lascia il timer del passo precedente (ritentativo o scadenza) e il tentativo `n+1` si rifà con lo stesso numero (nessuna corsa lo provoca).
- **Chiamate parallele per errore:** in un solo messaggio ho lanciato dieci chiamate `ssh` per `start1k`; `brun.sh` rifiuta di partire con una corsa in corso, quindi le prime quattro sono andate in fila nell'ordine voluto (H2 r2, B r2, B r3, H2 r3: nessun dato perso né mescolato) e le altre sei sono state rifiutate e le ho rifatte una per volta; nessun file grezzo è stato scritto dalle rifiutate.
- **B senza tabella `timer`:** nei file grezzi B la riga `timer_rows_for_finished_activities` riporta l'errore di `sqlite3` («no such table: timer»), atteso.
- **Un'unica corsa a 10.000/s di H a 2.048** (la riga del piano), con attesa di quiete 0 s (macchina già ferma).
- **Tempo:** circa due ore (lettura del codice, modifica, costruzione, 30 corse su minisheep tra 1,5 e 13 minuti).

### Non coperto

- **Una correzione che porti il rapporto H2/B sotto 1,20:** nessuna provata (per esempio il `Timer` in un gruppo senza coda dietro i `Done`/`Ready`, più connessioni di scrittura, o nessun timer durevole per attività brevi). H2 rispetta il limite assoluto (19,83 ms ≤ 25 ms) e i 10.000/s, non il rapporto di 1,20 contro B.
- **Il doppione dopo un crash con `Timer` non durevole** come caso provocato (non è comparso in tre corse); **crash a metà aggiornamento** e con consegne non durevoli; **Postgres**.
- **Il gruppo `Timer` in coda** come causa del +6,9 ms di p50: ipotesi, non isolata.
- **Il tetto più alto su H2** (`CAP=2048` su H2) e `held_max` sul braccio H: non misurati; la corsa a 2.048 è su H, senza `held_max`.
- **Le scadenze di tentativo con il tetto alto** (12.364 su H): il margine `grace_ms` di 2 s contro l'attesa nel worker non è stato riesaminato; senza battiti un tentativo in coda scade.
- Tutto il resto del blocco D (log dei task, RustFS, `If-None-Match`, visibilità, worker rumoroso, segreto, crash con righe in volo, tetto per tentativo): non fatto, come chiesto.

### Comandi per rifare (blocco D, seconda parte)

Su minisheep, in `~/ttspikebus` (`env.sh` dal repository, `.env` da `./gen-env.sh`, binari `bin/busspike-bB`, `bin/busspike-d0`, `bin/busspike-h2`, `bin/busspike-h2-old`). Una corsa alla volta, una chiamata per volta; `ARM_BIN` e `ARM` scelgono il binario e il nome del braccio.

| Misura | Comando |
| --- | --- |
| Passo 1 | `GATE60=1 VTRIES=100 CAP=2048 ARM_BIN=$HOME/ttspikebus/bin/busspike-d0 ARM=H-cap2048-head-5117575 BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost2-thr10k-H-cap2048-4-ms-r1.txt ./with-sampler.sh risultati/iggy-sqlite-timercost2-thr10k-H-cap2048-4-ms-r1.io ./scen.sh iggy sqlite thr10k secs=600`, poi `./brun.sh wait` |
| Retry / scadenza su H2 | `ARM_BIN=$HOME/ttspikebus/bin/busspike-h2 ARM=H2-c2ac38a BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost2-timer-retry-h2-4-ms-r1.txt ./scen.sh iggy sqlite tretry` (`tdead`) |
| Kill sotto carico | `ERATE=334 CAP=4096 ARM_BIN=… ARM=H2-c2ac38a BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost2-ekill-h2-4-ms-r1.txt ./scen.sh iggy sqlite ekill` |
| C2 / C3.4 | `[PRE100K=1] SPIKE_OLD=$HOME/ttspikebus/bin/busspike-h2-old CAP=4096 ARM_BIN=$HOME/ttspikebus/bin/busspike-h2 ARM=H2-c2ac38a BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost2-upgrade-rerun-h2-4-ms-r1.txt ./up-run.sh n=3 rate=334` |
| D2.1 | `ARM_BIN=$HOME/ttspikebus/bin/busspike-bB ARM=B-blockB-7f156ba BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost2-start1k-B-4-ms-r1.txt ./scen.sh iggy sqlite start1k` (H2: `busspike-h2`, `ARM=H2-c2ac38a`) |
| D2.2 | `GATE60=1 VTRIES=100 ARM_BIN=… ARM=… BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost2-thr10k-H2-4-ms-r1.txt ./with-sampler.sh risultati/iggy-sqlite-timercost2-thr10k-H2-4-ms-r1.io ./scen.sh iggy sqlite thr10k secs=600`, poi `./brun.sh wait` |


## Blocco D, terza parte

Record di consegna asincrono nel database (braccio **H3**, decisione di Marco, 2026-11-25), al posto dell'evento `Timer` di tipo 0 sul bus. Il log dei task (FR-59) non è stato costruito. Minisheep, stessi 4 CPU (motore e Iggy su `1-4`, carico su `8-11`), `ms-run.sh` con pavimento di `fdatasync` prima e dopo; ogni file grezzo porta le righe `braccio:` e `binario sha256:`. File grezzi `risultati/iggy-{sqlite,pg}-timercost3-<riga>-4-ms-r<n>.txt`. La regola di lettura è stata scritta e committata **prima** della prima corsa, in `risultati/iggy-sqlite-timercost3-regola-4-ms.txt` (commit `944fb3a`, 11:48; prima corsa 11:55:52). Il commit `01cf3bc` ha aggiunto una riga sulla lettura della soglia del doppione; il commit `c851ef0` (12:03, prima della corsa r2) **sostituisce** la soglia del doppione provocato con un controllo di inclusione di insiemi, per decisione del coordinatore: il numero dei record in attesa a un `kill -9` non si può campionare (ogni campione precede il segnale e il conteggio è una dentatura tra un commit del lotto e l'altro). La corsa r1 resta nel rapporto come **FALLITA con la regola vecchia**, con tutti i suoi numeri.

| Binario (sha256, primi 16) | Braccio | Righe |
| --- | --- | --- |
| `d730b1a73111ca79` | `B`: blocco B, commit `7f156ba` (senza timer) | `start1k`, `thr10k`, Postgres |
| `1a1348abf3bea374` (+ `4478461c59299ba9`, `--features old`) | `H2`: commit `c2ac38a` | `start1k`, `thr10k` |
| `25c97a9e69512b9a` (+ `a1c840e200df4736`, `--features old`) | `H3`: commit `01cf3bc` (codice `b60b001` più la correzione della ripresa) | controlli, `start1k`, `thr10k`, Postgres |

### Sintesi

1. **Controlli di correttezza su H3: superati, con la regola sostituita per il doppione.** `tretry` (300/300 una volta, 0 prima, già scaduti a `READY` 65,9 ms p50), `tdead` (99/99 una volta, 0 prima, 10,3 / 20,3 ms p50/p99 dopo la scadenza), doppione provocato r2 (363 iniziati due volte > 0, nessuno con riga nello scarico, 0 perse), due `ekill` con ritardo 0 (39 e 43 iniziati due volte, nessuno con riga nello scarico, 0 perse), aggiornamento semplice (pausa 2.633 ms, 0 rifatte, 3.042 timer riscritti = in volo) e con 100.000 timer (pausa 8.890 ms, 103.032 timer riscritti, 100.000 scattati una volta, 0 rifatte), tutti con 0 accettate perse, zero righe `timer` per attività concluse e zero «database bloccato».
2. **`start1k` a tre bracci (6 triple, tutti gli ordini): H3 è molto più vicina a B che a H2; tutti e due i criteri di H3 superati.** Mediana del p99 di `start_ready`: B 10,82 ms, H2 19,72 ms, **H3 11,28 ms**. Rapporto H3/B **1,04** (soglia 1,20); limite assoluto 25 ms: 11,28 (sei corse 11,09-11,53). Differenza H3 − B: +0,78 ms mediana con B prima, +0,29 ms con H3 prima (corretto nella revisione: la relazione dell'agente diceva +0,42; le tre differenze sono +0,29, +0,42 e −24,46): stesso segno, **si dichiara** (H3 più lenta di B di circa mezzo millisecondo), ben dentro la soglia. Differenza H2 − H3: +8,40 ms con H2 prima, +8,46 ms con H3 prima: **si dichiara** (H3 più veloce di H2 di 8,4 ms). Lettura dell'ipotesi: |H3−B| = 0,46 ms contro |H3−H2| = 8,45 ms, **l'ipotesi della coda di scrittura sul bus è supportata** (misurata come confronto; la causa resta ipotesi, non isolata).
3. **`thr10k`: H3 una corsa su due ≥ 9.900 att./s (r1 9.887, r2 9.998 sulla media dei minuti 2-10); la causa della corsa sotto soglia è aperta.** Dettaglio sotto. Il disturbo del disco è misurato solo per B r1 (pavimento prima 7,79 / 207 ms). Il pavimento prima di H3 r1 (4,79 / 68,7 ms) è simile a quello di H2 r2 (4,50 / 84,5 ms), che ha superato la soglia. Per H3 r1 il disturbo del disco è quindi ipotesi, alla pari con l'ipotesi della contesa del task scrittore (4 raffiche `REC_ERR` nella stessa corsa). Il limite di 10.000 att./s del PRD per H3 non si può dichiarare rispettato. Con H3 compaiono «database bloccato» a 10.000/s: 11 nella r1 (4 raffiche di errore del task scrittore, 7 arresti del proiettore) e 0 nella r2. B ne ha 7 e 0: il file grezzo di B r1 ne contiene 6; il 7 viene dal registro del motore, cancellato con `out/` (osservato senza file grezzo). H2 ne ha 4 e 2. **Il criterio «zero» di H3 è violato a 10.000/s** (misurato), ma lo stesso errore compare nei bracci senza task scrittore.
4. **Doppioni dopo un crash (tentativo 0 iniziato due volte), da mettere accanto a H2.** `tdead`: **1** (H2 nelle sue corse: 0); `ekill` con ritardo 0: **39 e 43**, con 36-39 consegne non durevoli al campione (H2 nelle sue tre corse: 0, con 0 non durevoli al campione e massimi 31-64); `ekill` con ritardo 500 ms: **363** (r2) e 379 (r1, fallita con la regola vecchia). Il doppione provocato dalla finestra del task scrittore di **20 ms (`REC_WINDOW_MS`) è una mia scelta, non chiesta; allunga la finestra in cui un record non è durevole e quindi i doppioni possibili; non l'ho regolata a metà serie.** `tdead` ha superato la regola come scritta (copre i tentativi rifatti: 99 su 99 una volta sola), ma un tentativo 0 è partito due volte.
5. **Postgres, `start1k`, B contro H3: latenza riportata, non giudicata; criterio delle righe vecchie FALLITO.** Ordine B,H3: B p99 41,22 ms, H3 19,52 ms; ordine H3,B: H3 163,71 ms, B 10,90 ms. Rumore grande tra le corse. **Controllo delle righe `timer` per attività concluse sulle corse Postgres di H3: 13 nella r1 (0 nella r2).** La regola 4 chiede zero righe «in OGNI corsa di H3 di tutto il blocco», quindi anche con Postgres: **FALLITO** nella r1. La guardia contro le righe vecchie ha lasciato 13 righe per attività concluse (misurato). Effetto, letto nel codice e non misurato: la ripresa classifica solo le attività non concluse e ignora queste righe; `rewrite_live` all'aggiornamento le rimette sul bus come `Timer` se l'esecuzione è ancora viva, e il motore nuovo le scarta perché il tentativo non è in volo; nessuno le cancella più, quindi la tabella cresce. Nessuna consegna doppia da questa via. Che la causa sia la corsa dichiarata nel commento `ponytail:` del codice (transazione del proiettore ancora aperta a livello READ COMMITTED) è ipotesi: nessuna corsa la isola. Con SQLite il controllo è 0 in tutte le corse.
6. **Istruzioni SQL per attività** (misurata, SQLite `start1k`): B 0,0322-0,0324, H2 0,0412, **H3 0,1876-0,1881** (il task scrittore fa `BEGIN`, due istruzioni, `COMMIT` a lotti di 20 ms, circa 4 istruzioni ogni 20 consegne); righe toccate per attività: 2,20 (B), 4,21 (H2 e H3). Sempre ampiamente sotto il limite di 5 di NFR-4.

### Controlli di correttezza su H3 (una corsa ciascuno salvo dove detto; tutti misurati, raw files `iggy-sqlite-timercost3-…-h3-4-ms-r<n>.txt`)

| Controllo | Esito |
| --- | --- |
| `tretry` (r1) | 300 ritentativi su 300 iniziati una volta, 0 prima della scadenza; i 156 già scaduti a `READY` partono 65,9 ms dopo (p50, p99 68,1; soglia 1 s), gli altri 11,1 ms (p50) dopo la scadenza; 0 perse; righe `timer` per concluse 0; bloccato 0 |
| `tdead` (r1) | 99 su 99 rifatti una volta sola, 0 prima, 10,3 / 20,3 ms (p50/p99) dopo la scadenza; **tentativo 0 eseguito due volte: 1** (alla ripresa 2 «da consegnare subito»); 0 perse; bloccato 0 |
| `ekill` doppione provocato, `rec_delay_ms=500`, **r1, regola vecchia: FALLITA** | iniziati due volte 379; campione di `undurable_deliveries` subito prima del kill 371; massimo della corsa 546; 387 «da consegnare subito»; 2.633 in volo; 0 perse. 379 > 371. Kill → `READY` 108 ms |
| `ekill` doppione provocato, **r2, regola nuova: SUPERATA** | iniziati due volte **363** (> 0); **0 con una riga nello scarico** (scarico `timer` di tipo 0 preso dopo il kill e prima della ripartenza: 2.778 righe, tutte per attività non concluse); 0 perse (`verify OK` alla quarta). Campione 363 consegne non durevoli, massimo 549; alla ripresa «in volo (scadenza ripristinata)» **2.649** e 373 «da consegnare subito». **Controllo incrociato:** righe dello scarico per attività non concluse **2.778** contro «in volo» **2.649**, differenza **129**: ipotesi (non verificata) che siano righe di attività il cui `Done` è già nella coda del bus ma non ancora proiettato, che la ripresa toglie con la rilettura. kill → `READY` 125 ms |
| `ekill` ritardo 0, r1 / r2 | iniziati due volte **39 / 43**; con riga nello scarico 0 / 0; scarico 3.045 / 3.058 righe contro «in volo» 2.972 / 2.973; campione di non durevoli al kill 39 / 36; «da consegnare subito» 47 / 48; 0 perse; kill → `READY` 132 / 123 ms |
| Aggiornamento semplice (`up-run.sh n=3 rate=334`, `CAP=4096`) | pausa **2.633 ms** (svuotamento 768, nuovo Iggy 1.145, motore nuovo 720); 0 accettate perse; attività iniziate due volte 0; `late_ok` 3.042; timer riscritti 3.042 = in volo allo svuotamento; 3.088 esecuzioni vive riversate |
| Aggiornamento con 100.000 timer (`PRE100K=1 CAP=4096`) | pausa **8.890 ms** (limite 30 s; svuotamento 565, nuovo Iggy 1.156, motore nuovo 7.169 di cui riversamento 3.975); 0 perse in entrambe le verifiche; 103.032 timer riscritti (100.000 + 3.032), 100.000 scattati una volta, scarto p50 10,3 / p99 20,1 ms; attività iniziate due volte 0; `late_ok` 3.032 |
| Tutte le corse H3 dei controlli | righe `timer` per attività concluse 0; «database bloccato» 0 (registro del motore e file grezzo) |

**Numeri sul blocco (solo riportati; derivati da `accepted_us` nel database e dal registro dei worker, nessun meccanismo nuovo; kill con `rec_delay_ms=500` r2, le altre corse sono nei file grezzi).**

| Scena | Attività in volo al kill / allo svuotamento | kill (o segnale) → `READY` | Prima accettazione nuova dopo `READY` | Prima consegna di un'esecuzione nuova | Nei 2 s dopo `READY`: accettate nuove / consegne nuove |
| --- | --- | --- | --- | --- | --- |
| `ekill` ritardo 500, r1 | 3.012 | 108 ms | 3,7 ms | 106 ms | 667 / 664 |
| `ekill` ritardo 500, r2 | 3.012 | 125 ms | 2,3 ms | 86 ms | 668 / 662 |
| `ekill` ritardo 0, r1 | 3.011 | 132 ms | 1,1 ms | 76 ms | 667 / 665 |
| `ekill` ritardo 0, r2 | 3.012 | 123 ms | 3,9 ms | 87 ms | 666 / 663 |
| Aggiornamento semplice | 3.042 | 2.633 ms (dal segnale) | 3,4 ms | 36 ms | 667 / 665 |
| Aggiornamento con 100.000 timer | 3.032 | 8.890 ms (dal segnale) | 3,7 ms | 187 ms | 666 / 663 |

### `start1k` a 1.000 att./s, tre bracci, 6 triple, SQLite, `CAP=512` (`iggy-sqlite-timercost3-start1k-{B,H2,H3}-4-ms-r{1..6}.txt`)

Ordine in tempo (ogni braccio in ogni posizione due volte): r1 B,H2,H3; r2 B,H3,H2; r3 H2,B,H3; r4 H2,H3,B; r5 H3,B,H2; r6 H3,H2,B. `start_ready` p99 (p50) in ms:

| Tripla | B | H2 | H3 |
| --- | --- | --- | --- |
| 1 | 10,97 (8,26) | 20,37 (14,91) | 11,53 (7,71) |
| 2 | 10,36 (7,46) | 19,60 (14,68) | 11,14 (7,56) |
| 3 | 10,56 (7,52) | 19,84 (14,60) | 11,44 (7,73) |
| 4 | 11,01 (7,67) | 19,41 (14,53) | 11,30 (7,57) |
| 5 | 10,67 (7,62) | 20,45 (14,61) | 11,09 (7,61) |
| 6 | **35,71** (8,35) | 19,30 (14,49) | 11,25 (7,58) |
| **Mediana** | **10,82** | **19,72** | **11,28** |

- La corsa B della tripla 6 (35,71 ms) è un valore estremo (disturbo di ambiente: ipotesi, non indagato); si tiene, nessuna corsa si scarta. Le mediane lo assorbono; la sua differenza con H3 (−24,5 ms) è la sola di segno opposto, ma la mediana dei tre della sua posizione (H3 prima di B) resta positiva: +0,29 ms (differenze +0,29, +0,42, −24,46).
- Tutte le corse H3 e H2: righe `timer` per attività concluse 0; righe totali a fine corsa H3 0 / 120 / 48 / 0 / 0 / 127 (attività ancora in volo o non ancora proiettate allo spegnimento, ipotesi). «Database bloccato» nelle sei corse H3: 0.
- Il p50 di avvio: B 7,46-8,35, **H3 7,56-7,73**, H2 14,49-14,91 ms.

### `thr10k` a 10.000 att./s per 10 minuti, SQLite, `CAP=512`, `GATE60=1 VTRIES=100` (`iggy-sqlite-timercost3-thr10k-{B,H2,H3}-4-ms-r{1,2}.txt`, più i campioni di I/O)

Ordine in tempo: r1 B,H2,H3 poi r2 H3,H2,B. Soglia scritta prima: almeno 9.900 att./s sulla finestra misurata (minuto 1 escluso), 0 accettate perse.

| Misura | B r1 | H2 r1 | H3 r1 | H3 r2 | H2 r2 | B r2 |
| --- | --- | --- | --- | --- | --- | --- |
| Pavimento prima (p50 / p99.9 ms) | 7,79 / 207 | 4,67 / 83,0 | 4,79 / 68,7 | 3,85 / 77,1 | 4,50 / 84,5 | 2,83 / 67,2 |
| Offerte / accettate | 605.001 / 585.149 | 605.000 / 596.159 | 605.000 / 598.880 | 605.001 / 605.001 | 605.000 / 603.025 | 605.000 / 605.000 |
| Rifiutate `E_PROJECTION_DOWN` / errori | 10.324 / 9.528 | 3.982 / 4.859 | 6.120 / 0 | 0 / 0 | 820 / 1.155 | 0 / 0 |
| **Concluse al secondo** (intera corsa) | **7.785** | 9.928 | **9.894** | 9.994 | 9.979 | 9.994 |
| Media dei minuti 2-10 | 7.317 | 10.299 | 9.887 | 9.998 | 10.003 | 10.008 |
| Minuti 2-10 sotto 9.900 | 7 su 9 | 3 su 9 | 1 (t3 8.957) | 1 (t9 9.895) | 1 (t3 9.875) | 0 |
| Intervallo più lungo senza accettazioni (ms) | 4.067 | 3.450 | 3.226 | 606 | 1.211 | 437 |
| `start_ready` p50 / p99 | 10,7 s / 70,8 s | 175 ms / 21,9 s | 67,8 ms / 1,24 s | 67,5 / 160,5 ms | 137,9 ms / 3,04 s | 67,5 / 147,6 ms |
| «Database bloccato» (registro del motore) | 7 | 4 | **11** (4 + 7) | **0** | 2 | 0 |
| Accettate perse | 0 | 0 | 0 | 0 | 0 | 0 |
| Righe `timer` per concluse | — | 0 | 0 | 0 | 0 | — |

- **Superato (≥ 9.900 sulla media dei minuti 2-10, 0 perse):** B r2, H2 r1 (con un'ampia coda: tre minuti sotto 9.900 e 20.099 in un minuto di recupero), H2 r2, H3 r2. **Non superato:** B r1 (7.785 / 7.317), H3 r1 (9.894 intera corsa, 9.887 media dei minuti 2-10). Sull'intera corsa H3 r1 è sotto la soglia di 6 att./s, con il minuto 3 a 8.957 coincidente con arresti del proiettore per «database bloccato».
- **Lettura onesta:** la B r1 gira con il disco disturbato (pavimento prima 7,79 ms p50 e 207 ms p99.9, misurato); per H2 r1 e H3 r1 il pavimento prima (4,67 / 83,0 e 4,79 / 68,7 ms) è simile a quello di H2 r2 (4,50 / 84,5 ms), che ha superato la soglia, quindi il loro disturbo è ipotesi; che la B r1 sia sotto soglia per il disturbo e non per il braccio è ipotesi (B fa 9.994 att./s nel blocco D precedente e nella r2, misurato). Il secondo ordine (H3, H2, B) gira con un disco più tranquillo e dà H3 9.994, H2 9.979, B 9.994 (H3 con 0 «bloccato», H2 con 2). Il confronto tra bracci a 10.000/s non è dunque pulito; la causa del disturbo (carico dei runner o disco: ipotesi) non è isolata. Nessuna corsa è scartata né rifatta.
- Il `start_ready` p50 a 10.000/s: H3 67,5-67,8 ms, B 67,5 ms (r2), H2 137,9 ms (r2): la corsa H3 non ha il sovrapprezzo di avvio di H2, nei due ordini.
- Istruzioni SQL per attività a 10.000/s: nei file grezzi (`sql_per_activity`).

### Postgres, `start1k`, B contro H3 (`iggy-pg-timercost3-start1k-{B,H3}-4-ms-r{1,2}.txt`; riportato, non giudicato)

| Ordine | Corsa | `start_ready` p50 / p99 (ms) | SQL per attività / righe | Righe `timer` per concluse |
| --- | --- | --- | --- | --- |
| 1: B poi H3 | B r1 | 13,85 / 41,22 | 0,0288 / 2,20 | (B senza tabella `timer`) |
| | H3 r1 | 13,33 / 19,52 | 0,1608 / 4,21 | **13** (totale 13) |
| 2: H3 poi B | H3 r2 | 33,95 / 163,71 | 0,1047 / 4,23 | 0 (totale 180) |
| | B r2 | 8,00 / 10,90 | 0,0315 / 2,20 | (B senza tabella `timer`) |

Due coppie non bastano per una mediana. La prima corsa di ciascuna coppia non è sempre la lenta (r1: B 41,2 lenta, H3 19,5; r2: H3 163,7 lenta, B 10,9): rumore tra corse con Postgres.

### Cosa è cambiato nel codice dello spike (commit `b60b001`, `01cf3bc`, `c851ef0`)

- **`dispatch`** (`engine.rs`): invia subito, arma la scadenza in memoria e manda `(esecuzione, passo, tentativo, scadenza)` a un canale; nessun gruppo di bus per il tipo 0 (tipi 1, 2 e battiti invariati sul bus). `undurable_deliveries` = record inviati e non ancora confermati dal commit.
- **Task `rec_writer`:** un solo task asincrono, lotti ogni 20 ms al più (`REC_WINDOW_MS`, scelta mia; motivo, ipotesi senza file grezzo: il gestore di blocco di SQLite non sarebbe equo e un commit continuo potrebbe far aspettare il proiettore oltre `busy_timeout`), `Db::write_delivery_records` (stesso `Db`, istruzioni e righe contate: `BEGIN`, un `INSERT … ON CONFLICT` per 500 righe, un `DELETE` di guardia, `COMMIT`). Un errore si riprova ogni 100 ms (riga `REC_ERR` una volta per raffica).
- **Durabilità:** la stessa dei commit del proiettore, `PRAGMA journal_mode=WAL` e `PRAGMA synchronous=FULL` (in `Db::open`, per ogni connessione). Un commit confermato sopravvive al `kill -9` del processo (provato dalle corse di kill); che sopravviva anche alla perdita di corrente dell'host con `FULL` in WAL è ipotesi, non provata (uno spegnimento dell'host non è un `kill`). Il record è durevole circa 20-30 ms dopo l'invio, non prima.
- **Nessuna chiave verso righe non proiettate:** l'inserimento non richiede `activity`. La lettura della ripresa (`Db::timers`) non ha più il collegamento con `execution` (il codice precedente avrebbe nascosto i record di esecuzioni non ancora proiettate; corretto in `01cf3bc` prima di costruire i binari).
- **Guardia contro le righe vecchie:** dopo l'inserimento, nella stessa transazione, cancella le righe del lotto la cui attività ha `done_us` non nullo; se `activity` non esiste ancora, l'esito non è stato proiettato e la riga resta (la cancella il proiettore). La corsa è commentata in italiano nel codice. Esatta con SQLite (misurato: 0 in tutte le corse SQLite); con Postgres READ COMMITTED non è esatta (misurato: 13 righe in una corsa, v. sintesi 5). Due regole d'ordine nello stesso `INSERT`: un record non sostituisce una riga con un tentativo più nuovo (o lo stesso con un tipo più alto), uguale nell'upsert del proiettore e nella rilettura del bus alla ripresa.
- **Aggiornamento:** il motore vecchio esce dallo svuotamento solo con `undurable == 0` oltre a nessun gruppo in volo e posizione applicata uguale alla testa, quindi prima di `rewrite_live`. Misurato dai due aggiornamenti: timer riscritti = in volo, 0 persi.
- **Stato che ora vive solo nel database** (kind 0): numero del tentativo e scadenza di ogni consegna in corso (l'insieme `timer` di tipo 0). Sul bus restano `Accepted`, `Ready`, `Done` e i `Timer` di tipo 1, 2 e i battiti. **Ripresa:** un'attività emessa e non conclusa con una riga `timer` di tipo 0 è «in volo» (scadenza originale, spostata di `grace_ms` se già passata); senza riga, o con la sola riga di un ritentativo, è «mai consegnata» e si consegna subito: il doppione accettato (misurato: 1, 39, 43, 363, 379 in `tdead` e nelle corse `ekill`).
- **Gancio di guasto solo per lo spike:** opzione del motore `rec_delay_ms` (predefinito 0, variabile `REC_DELAY` in `scen.sh`); usata solo nelle due corse del doppione provocato.
- **Script:** `scen.sh` (campione di `undurable_deliveries` e `timers_pending` subito prima del kill, scarico delle righe `timer` di tipo 0 dopo il kill e prima della ripartenza con inclusione di insiemi, numeri del blocco dopo `READY`, controllo SQL delle righe `timer` anche per Postgres, conteggio di «database bloccato») e `up-run.sh` (gli stessi controlli).

### Deviazioni dal piano (blocco D, terza parte)

- **Soglia del doppione provocato sostituita** dopo la corsa r1 (v. sopra); r1 resta FALLITA con la regola vecchia, con tutti i numeri.
- **Correzione prima delle corse:** il codice `b60b001` leggeva i timer con il collegamento a `execution`; corretto in `01cf3bc`; i binari misurati sono costruiti da `01cf3bc`.
- **`REC_WINDOW_MS` di 20 ms** è un parametro mio, non chiesto; non regolato a metà serie.
- **Serie `thr10k` disturbata** (v. sopra): nessuna corsa scartata o rifatta; non ho aggiunto corse oltre alle due per braccio.
- **Un campione di kill sempre prima del segnale:** il campione di `undurable_deliveries` è preso nel processo `stats` subito prima del segnale; non serve più alla soglia, solo come numero riportato.
- **Postgres:** il controllo delle righe `timer` per concluse è stato eseguito (psql nel contenitore) su tutte le corse Postgres di H3.
- **Worktree:** partito da `main`, portato a `e07f79f` con `git reset --hard` (pulito, nessun commit mio prima).
- **Chiamate:** una chiamata `ssh` alla volta per le corse; le attese lunghe sono state fatte con più attese brevi di una chiamata ciascuna (il limite dello strumento è circa 3 minuti in primo piano).

### Non coperto

- **La causa dei «database bloccato» a 10.000/s** nei tre bracci (B, H2, H3) e del disturbo del disco nella serie del primo ordine: non isolata. Con H3 il task scrittore partecipa ai conflitti (4 raffiche `REC_ERR` nella r1), ma con 0 nella r2 non c'è una corsa che separi l'effetto del task dal rumore del disco.
- Postgres a 10.000/s e il controllo delle righe vecchie sotto concorrenza stretta (il caso trovato è con `start1k`).
- Un crash a metà aggiornamento con record non durevoli; una perdita di corrente dell'host (non un `kill`); una finestra di raccolta del task scrittore diversa da 20 ms (e il suo effetto sui doppioni).
- Il confronto di H3 con B a 10.000/s sulle stesse condizioni di disco: una ripetizione in ordine invertito, con la macchina più tranquilla, non è stata fatta.

### Comandi per rifare (blocco D, terza parte)

Su minisheep, in `~/ttspikebus` (`env.sh` dal repository, `.env` da `./gen-env.sh`, binari `bin/busspike-bB`, `bin/busspike-h2`, `bin/busspike-h2-old`, `bin/busspike-h3`, `bin/busspike-h3-old`). Una corsa alla volta, una chiamata per volta; `ARM_BIN` e `ARM` scelgono il binario e il nome del braccio (`B-blockB-7f156ba`, `H2-c2ac38a`, `H3-01cf3bc`).

| Misura | Comando |
| --- | --- |
| Retry / scadenza | `ARM_BIN=$HOME/ttspikebus/bin/busspike-h3 ARM=H3-01cf3bc BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost3-timer-retry-h3-4-ms-r1.txt ./scen.sh iggy sqlite tretry` (`tdead`) |
| Doppione provocato / `ekill` ritardo 0 | `[REC_DELAY=500] ERATE=334 CAP=4096 ARM_BIN=… ARM=H3-01cf3bc BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost3-ekill-delay500-h3-4-ms-r2.txt ./scen.sh iggy sqlite ekill` |
| Aggiornamenti | `[PRE100K=1] SPIKE_OLD=$HOME/ttspikebus/bin/busspike-h3-old CAP=4096 ARM_BIN=$HOME/ttspikebus/bin/busspike-h3 ARM=H3-01cf3bc BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost3-upgrade-rerun-h3-4-ms-r1.txt ./up-run.sh n=3 rate=334` |
| `start1k` | `ARM_BIN=… ARM=… BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost3-start1k-H3-4-ms-r1.txt ./scen.sh iggy sqlite start1k` (Postgres: `iggy pg start1k`) |
| `thr10k` | `GATE60=1 VTRIES=100 ARM_BIN=… ARM=… BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost3-thr10k-H3-4-ms-r1.txt ./with-sampler.sh risultati/iggy-sqlite-timercost3-thr10k-H3-4-ms-r1.io ./scen.sh iggy sqlite thr10k secs=600`, poi `./brun.sh wait` |

## Blocco D, ripetizione di `thr10k`

**Perché.** Nella terza parte H3 ha superato 9.900 att./s in una corsa su due (r1 9.887, r2 9.998 sulla media dei minuti 2-10). La causa della corsa sotto soglia era aperta: il pavimento del disco prima di H3 r1 (4,79 / 68,7 ms) era simile a quello di H2 r2 (4,50 / 84,5 ms), che era passata, e nella stessa corsa il task scrittore aveva avuto 4 raffiche `REC_ERR`. Marco ha deciso (2026-11-25) di ripetere i 10.000 att./s prima di scegliere quale registro di consegna va nell'ADR. Le regole di lettura sono in `risultati/iggy-sqlite-timercost4-regola-4-ms.txt`, committate alle 14:31:33 (commit `e955b3d`) **prima** di ogni corsa; sono state applicate alla lettera, senza cambiare soglie. Nessun codice nuovo, nessun binario nuovo, `REC_WINDOW_MS` resta 20 ms. Ordine fisso: H3 r1, B r1, H3 r2. Minisheep, stesso `ms-run.sh`, `GATE60=1 VTRIES=100 CAP=512`, 10 minuti, SQLite, campionatore di I/O. File grezzi `risultati/iggy-sqlite-timercost4-thr10k-{H3,B}-4-ms-r<n>.txt` (+ `.io.names`); ogni file grezzo porta in fondo i conteggi della regola 5, presi dal registro del motore prima che `out/` fosse cancellata.

| Binario (sha256, primi 16) | Braccio | Controllato prima di |
| --- | --- | --- |
| `25c97a9e69512b9a` | `H3`: commit `01cf3bc` (`ARM=H3-01cf3bc`) | H3 r1 (14:37), H3 r2 (15:07) |
| `d730b1a73111ca79` | `B`: blocco B, commit `7f156ba` (`ARM=B-blockB-7f156ba`) | B r1 (14:49) |

Lo stesso `sha256` è anche riga `[stato prima] binario sha256:` di ogni file grezzo.

### Pavimento del disco prima di ogni corsa (regola 2)

Soglia: p50 ≤ 4,0 ms e p99.9 ≤ 100 ms (`fsync-floor dir=$HOME/.cache n=300`). Conta la riga `[stato prima] pavimento fdatasync` del file grezzo della corsa, presa da `ms-run.sh` dopo la sua attesa di macchina ferma.

| Ora | Misura | p50 / p99.9 ms | Entro soglia |
| --- | --- | --- | --- |
| 14:32:31 | mia, prima di H3 r1 | 8,00 / 12,65 | no (attesa di 5 minuti) |
| 14:37:40 | mia, seconda per H3 r1 | 2,94 / 76,29 | sì |
| 14:37:51 | **file grezzo H3 r1** | **2,97 / 68,03** | **sì** |
| 14:49:11 | mia, prima di B r1 | 2,88 / 8,28 | sì |
| 14:51:38 | **file grezzo B r1** (dopo 140 s di attesa di `ms-run.sh`) | **7,73 / 182,14** | **no** |
| 15:02:30 | mia, prima di H3 r2 | 9,65 / 14,92 | no (attesa di 5 minuti) |
| 15:07:39 | mia, seconda per H3 r2 | 3,06 / 68,42 | sì |
| 15:07:55 | **file grezzo H3 r2** | **5,09 / 88,45** | **no** |

Dopo la corsa (riga `[stato dopo]`): H3 r1 4,80 / 6,74; B r1 4,36 / 6,92; H3 r2 4,77 / 73,53. Il pavimento cambia in pochi secondi: tra la mia misura e quella del file grezzo passano 11 s (H3 r1), 147 s (B r1) e 16 s (H3 r2), e in due casi su tre il valore è passato da dentro a fuori soglia (misurato). Una sola attesa di 5 minuti per H3 r1 e per H3 r2, nessuna per B r1; mai si è arrivati alle tre.

### `thr10k` a 10.000 att./s per 10 minuti, SQLite, `CAP=512`

| Misura | H3 r1 | B r1 | H3 r2 |
| --- | --- | --- | --- |
| Pavimento prima (p50 / p99.9 ms), dal file grezzo | 2,97 / 68,03 | 7,73 / 182,14 | 5,09 / 88,45 |
| **Pavimento prima entro soglia** | **sì** | **no** | **no** |
| Offerte / accettate | 605.001 / 585.913 | 586.068 / 511.024 | 605.000 / 591.724 |
| Rifiutate `E_PROJECTION_DOWN` / errori | 18.917 / 171 | 52.097 / 22.947 (più 18.932 offerte saltate per sovraccarico del generatore) | 12.773 / 503 |
| **Concluse al secondo** (intera corsa) | **9.675** | **8.856** | **9.788** |
| Media dei minuti 2-10 | **9.648** | **9.036** | **9.890** |
| Minuti 2-10 sotto 9.900 | 5 su 9: t3 9.744, t4 4.908, t5 7.166, t7 5.110, t9 9.849 | 3 su 9: t2 3.660, t6 7.493, t7 4.009 | 1 su 9: t6 5.435 |
| Intervallo più lungo senza accettazioni (ms) | 3.135 | 20.485 | 2.578 |
| `start_ready` p50 / p99 | 72,6 ms / 15,81 s | 107,2 ms / 52,53 s | 68,6 ms / 9,04 s |
| «Database bloccato», registro del motore / file grezzo | **77** / 27 | 4 / 4 | **39** / 29 |
| `REC_ERR` (raffiche del task scrittore, registro del motore) | **43** | 0 (B non ha il task) | **22** |
| Accettate perse (dopo le verifiche, quarta o prima riuscita) | 0 | 0 | 0 |
| Righe `timer` per attività concluse | 0 | non applicabile (B senza tabella `timer`) | 0 |
| Pressione I/O `some avg10` per minuto (min-max, righe `MIN`) | 21,8-42,5 % | 15,6-69,9 % | 25,6-41,9 % |

«Accettate perse»: in tutte e tre le corse la prima verifica è FALLITA (999, 703 e 610 accettate non ancora concluse nella proiezione) e la verifica con i tentativi di `VTRIES=100` è poi riuscita con 0 perse, come nella terza parte. Il numero finale è 0 e il criterio della regola 3 («0 accettate perse alla fine delle verifiche») è rispettato nelle tre corse (misurato).

### Esito di ogni corsa (regola 3: media dei minuti 2-10 ≥ 9.900 e 0 accettate perse)

- **H3 r1: NON SUPERATA** (media 9.648). Su disco tranquillo (pavimento prima entro soglia).
- **B r1: NON SUPERATA** (media 9.036). Su disco non tranquillo (pavimento prima fuori soglia).
- **H3 r2: NON SUPERATA** (media 9.890, 10 att./s sotto la soglia). Su disco non tranquillo (pavimento prima 5,09 ms p50, fuori soglia).

### Lettura per l'ADR (regola 4): **c, NON CONCLUSIVO**

Una sola lettura: B non è stata superata (regola 4c), quindi la serie nuova non separa H3 da B a 10.000 att./s. Per la stessa regola, nessuna corsa aggiuntiva senza il coordinatore. Misurato, da riportare accanto: nessuna delle tre corse supera 9.900; l'unica corsa H3 su disco tranquillo (r1) è sotto soglia (9.648) e, per la regola 4b, sarebbe bastata a leggere «H3 non regge» **se** B fosse stata superata, ma B non lo è. La r2 di H3 è sotto soglia di soli 10 att./s su disco non tranquillo. La r1 della terza parte (9.887) e questa r2 (9.890) sono entrambe sotto, la r2 della terza parte (9.998) è sopra: tre corse H3 su quattro sotto 9.900. La corsa sopra (r2 della terza parte) aveva un pavimento prima di 3,85 / 77,1 ms, entro la soglia di questa serie (misurato), e la r1 di questa serie, entro soglia, è sotto: due corse H3 su disco tranquillo, una sopra e una sotto, e il confronto con B manca. Non traggo dal quadro nessuna conclusione sul braccio oltre quella detta.

### Conteggi della regola 5 (tutti dal registro del motore, copiati nei file grezzi prima della cancellazione di `out/`)

«Database bloccato» / `SQLITE_BUSY` (il criterio di H3 resta zero; **violato nelle due corse H3**): H3 r1 **77** nel registro del motore (43 `REC_ERR` + 34 «proiezione ferma») e 27 nel file grezzo (che riporta solo un estratto); H3 r2 **39** (22 + 17) e 29 nel file grezzo; B r1 4 (tutte «proiezione ferma») e 4 nel file grezzo. Nei registri dei worker 0 in tutte e tre.

Righe `timer` per attività concluse: H3 r1 0, H3 r2 0 (criterio rispettato). B: la tabella non esiste.

Raffiche `REC_ERR` e «proiezione ferma» per minuto (righe del registro tra `MIN t=k-1` e `MIN t=k` attribuite al minuto k; il minuto 11 è dopo l'ultima riga `MIN`):

| Minuto | H3 r1 `REC_ERR` / ferma | H3 r2 `REC_ERR` / ferma | B r1 ferma |
| --- | --- | --- | --- |
| 1 | 0 / 0 | 3 / 1 | 0 |
| 2 | 0 / 0 | 5 / 3 | 0 |
| 3 | 0 / 0 | 0 / 0 | 2 |
| 4 | 13 / 10 | 0 / 0 | 1 |
| 5 | 11 / 8 | 0 / 0 | 0 |
| 6 | 2 / 4 | 6 / 3 | 0 |
| 7 | 9 / 6 | 6 / 10 | 1 |
| 8 | 6 / 4 | 1 / 0 | 0 |
| 9 | 1 / 2 | 0 / 0 | 0 |
| 10 | 0 / 0 | 1 / 0 | 0 |
| 11 | 1 / 0 | 0 / 0 | 0 |

**Coincidenze nello stesso minuto per i minuti 2-10 sotto 9.900** (osservate; la causa resta ipotesi, mai dedotta dalla coincidenza):

- **H3 r1.** Minuto 4 (4.908): 13 `REC_ERR`, 10 arresti del proiettore, `held_max` 38.278, `lag_p99` 20,2 s. Minuto 5 (7.166): 11 / 8, `held_max` 77.564. Minuto 7 (5.110): 9 / 6, `held_max` 32.385. Minuto 9 (9.849): 1 / 2, `held_max` 102. Minuto 3 (9.744): **nessuna** raffica, nessun arresto, 0 righe «bloccato»; `held_max` 628, `lag_p99` 786 ms. I minuti 6 (16.221) e 8 (13.851) sono sopra soglia con 2 / 4 e 6 / 4 (recupero del ritardo accumulato).
- **B r1.** Minuto 2 (3.660): 0 / 0 / 0 «bloccato»; pressione I/O `avg10` 69,9 % (il valore più alto delle tre corse), `lag_p99` 14,4 s. Minuto 6 (7.493): nessun «bloccato»; `lag_p99` 4,4 s. Minuto 7 (4.009): 1 arresto, `lag_p99` 52,8 s. Gli arresti per «bloccato» di B stanno nei minuti 3, 4 e 7; i minuti 3 (12.697) e 4 (12.916) sono sopra soglia.
- **H3 r2.** Minuto 6 (5.435): 6 `REC_ERR`, 3 arresti, `held_max` 30.370, `lag_p99` 20,1 s; il minuto 7 recupera a 13.036 con 6 `REC_ERR` e 10 arresti.

Dalle coincidenze: nelle due corse H3 i minuti con produttività più bassa coincidono con raffiche `REC_ERR` e arresti del proiettore (osservato); in B r1 due dei tre minuti sotto soglia non hanno alcun «bloccato» (osservato), per cui un blocco del database non basta a spiegare il calo di B. Il minuto 3 di H3 r1 (9.744) non ha né raffiche né arresti. La pressione I/O `avg10` in tutte le corse è tra 16 e 70 % durante la corsa: il carico stesso scrive sul disco, quindi il valore dice poco da solo. Il tempo prima del calo (H3 r1 dal minuto 4, H3 r2 dal minuto 6, B r1 dal minuto 1) è osservato, la causa (disco condiviso, contesa sul lock di SQLite, task scrittore) è **ipotesi** per tutte.

### Deviazioni

- **Il pavimento entro soglia nel file grezzo non coincide con la mia misura precedente** in B r1 e in H3 r2 (v. tabella): ho seguito la regola 2 come scritta (conta la riga del file grezzo), quindi B r1 e H3 r2 contano come «non su disco tranquillo». Nessuna corsa è stata scartata o rifatta.
- **B r1: attesa di 140 s di `ms-run.sh`** (soglia macchina ferma con `GATE60=1`), tra la mia misura delle 14:49 e lo stato prima delle 14:51:38. Le altre due: 5 s e 10 s.
- **Nessuna corsa aggiuntiva** dopo la lettura c, come chiede la regola 4.
- **Il conteggio dei «bloccato» e delle `REC_ERR` per minuto** usa un piccolo script locale (non committato) per attribuire le righe del registro ai minuti; il suo output è copiato nel file grezzo. Il file grezzo H3 riporta solo un estratto del registro del motore (14 e 17 `REC_ERR`): i numeri 43 e 22 vengono dal registro intero, copiati in fondo al file grezzo.
- **Worktree:** portato a `e955b3d` con `git reset --hard` (pulito, nessun commit mio prima). Una sola chiamata `ssh` alla volta; le attese lunghe a passi da circa 2,5 minuti.

### Non coperto

- **La causa delle corse sotto soglia** (disco condiviso con i runner, contesa sul lock di SQLite con il task scrittore, altro): non isolata; nessuna corsa la separa.
- **Il confronto tra H3 e B su due dischi tranquilli**: B r1 e H3 r2 sono partite con il pavimento fuori soglia, quindi la serie non misura il caso per cui è stata pensata; serve una nuova decisione del coordinatore.
- Il pavimento del disco *durante* la corsa (si conosce solo prima e dopo, e la pressione I/O del carico stesso).
- Postgres a 10.000 att./s; `REC_WINDOW_MS` diverso da 20 ms.

### Comandi per rifare (blocco D, ripetizione di `thr10k`)

Su minisheep, in `~/ttspikebus` (`env.sh` e gli script dal repository, `.env` da `./gen-env.sh`, binari `bin/busspike-h3` e `bin/busspike-bB`). Una corsa alla volta, una chiamata per volta. Prima di ogni corsa: `bin/busspike-h3 fsync-floor dir=$HOME/.cache n=300`; se p50 > 4,0 o p99.9 > 100 ms attendere 5 minuti e rimisurare, al più tre volte. Poi, per H3 (per B: `ARM_BIN=$HOME/ttspikebus/bin/busspike-bB ARM=B-blockB-7f156ba` e `-B-` nel nome):

| Misura | Comando |
| --- | --- |
| `thr10k` | `GATE60=1 VTRIES=100 CAP=512 ARM_BIN=$HOME/ttspikebus/bin/busspike-h3 ARM=H3-01cf3bc BRUN_ITERS=30 ./brun.sh risultati/iggy-sqlite-timercost4-thr10k-H3-4-ms-r1.txt ./with-sampler.sh risultati/iggy-sqlite-timercost4-thr10k-H3-4-ms-r1.io ./scen.sh iggy sqlite thr10k secs=600`, poi `BRUN_ITERS=32 ./brun.sh wait` finché stampa `FINITA` |
| Conteggi della regola 5 | prima che `out/` si cancelli: «database is locked» / `SQLITE_BUSY` in `out/engine.log`, righe `REC_ERR` e «proiezione ferma» per minuto (le righe fra `MIN t=k-1` e `MIN t=k` appartengono al minuto k), righe `MIN` dei minuti 2-10 sotto 9.900, da aggiungere in fondo al file grezzo |

## Blocco D, log dei task

Log dei task (FR-59) costruito sul motore H2 (decisione di Marco, 2026-11-25) e misurato su minisheep (stessi 4 CPU: motore, worker e Iggy su `1-4`, carico e lettore di visibilità su `8-11`), SQLite, `ms-run.sh` con pavimento di `fdatasync` prima e dopo; ogni file grezzo porta `braccio:` e `binario sha256:`. Regole di lettura scritte e committate **prima** della prima corsa misurata: `risultati/iggy-sqlite-logd-regola-4-ms.txt` (commit `2d05dd9`, poi la riga 2 riscritta per decisione del coordinatore nel commit `a43f308`, 16:00:03; la prima corsa misurata parte alle 16:00:40). Nessuna soglia è cambiata dopo una corsa; nessuna corsa è stata scartata o rifatta (nessuna condizione di non validità è scattata). File grezzi `risultati/iggy-sqlite-logd-<riga>-<braccio>-4-ms-r<n>.txt`; le corse di prova (`-smoke-`, non misurate) sono accanto.

| Binario (sha256, primi 16) | Cosa |
| --- | --- |
| `ab0c12d5d6a4aa81` | H2 ricostruito da `69f496f` (non ha lo stesso hash del vecchio `1a1348abf3bea374`: i controlli di H2 non si riportano); solo la corsa di prova `ekill` (accettate perse 0, iniziate due volte 0) |
| `fd04efcdeefe0425` | H2 + log dei task, commit `7c1dfcd` (`bin/busspike-logd-7c1dfcd`): tutte le corse misurate di questo blocco |

### Sintesi

1. **Riga 1, visibilità: superata.** p99 dalla riga scritta nel worker al pezzo leggibile e decifrato nell'archivio: 557,57 / 557,57 / 557,05 ms nelle tre corse (limite 1.000 ms), `over1s` 0, 45.000 pezzi e 225.000 righe per corsa, il lettore ha visto tutti i pezzi (`chunks` uguale a quelli di `logcheck`). Il p50 è 530 ms, coerente con la chiusura del pezzo a 500 ms (parametro della spike, non misurato a parte); che il resto, 30-60 ms, sia il ciclo di scrittura di 50 ms più fsync è un'ipotesi, non isolata.
2. **Riga 2, worker rumoroso: criterio del rapporto rispettato, limite assoluto di NFR-2 NON rispettato nel braccio con le righe di A, per 0,25 ms.** Mediana del p99 di B: `withA` **25,25 ms**, `Anolines` **23,70 ms**; R = 1,066 (soglia 1,20); differenza dichiarata positiva (X +0,71 ms con `withA` per primo, Y +1,20 ms con `Anolines` per primo). Il limite di 25 ms: `Anolines` lo rispetta, `withA` no (quattro corse su sei sopra: 25,49 / 25,92 / 25,68 / 25,01). Istruzioni SQL per attività invariate (B 0,0411-0,0412 nelle 12 corse; A 0,083 in tutti e due i bracci; globale 0,055, limite 5).
3. **Riga 3, segreto: superata.** Con la sostituzione: 0 occorrenze del segreto nei pezzi, 0 negli eventi del bus (stato e riferimenti), 0 nel registro del motore, in tutte e due le corse; 4.800 segnaposto nei pezzi, uguali al contatore `redacted` del motore. Controllo con la sostituzione spenta: 4.800 occorrenze trovate nei pezzi, 0 segnaposto (la ricerca sa trovare il valore). Il bus ha 0 anche nel controllo, come previsto: le righe non passano dal bus.
4. **Riga 4, crash del motore con righe in volo: superata in tutte e tre le corse, e il caso del pezzo riscritto con contenuto diverso è esercitato in tutte e tre.** Dopo la decifrazione di tutti i pezzi ogni (tentativo, numero) scritto dal worker compare una sola volta: salvate + scartate = scritte (616.980 / 616.200 / 616.320), `missing=0`, `overlap=0`, `stored_twice=0`, `refs_without_chunk=0`; accettate perse 0; tentativi iniziati due volte 0 / 0 / 0. `conflicts` (chiave già esistente) 2.993 / 3.002 / 3.003 e `conflicts_differ` (il pezzo esistente copriva meno righe di quelle da riscrivere) **1.492 / 1.509 / 1.489**.
5. **Riga 5, tetto: superata.** 800 tentativi, 80.000 righe scritte: salvate 56.000 e scartate 24.000, esattamente le attese (A: 40 salvate e 10 scartate per il tetto in byte, B: 100 e 50 per il tetto in righe); ogni pezzo dichiara le sue scartate (`header_inconsistent=0`), nessun tentativo oltre il tetto.
6. **Riga 6, RustFS `If-None-Match: *`: superata in tutte e due le corse.** `rustfs/rustfs:1.0.4` (Apache-2.0): primo PUT 200, secondo PUT sulla stessa chiave **412** (`PreconditionFailed`), l'oggetto letto ha l'impronta del primo contenuto; il PUT di controllo senza l'intestazione torna 200 e sovrascrive. Bastano due PUT firmati con `curl --aws-sigv4` (nessuna libreria nuova).
7. **Il riferimento al pezzo non è proiettato nel database**: sta nel topic `log-<tenant>`, che il proiettore non legge. NFR-4 col log attivo: B 0,0411-0,0412 istruzioni per attività con 3 righe per attività; A 0,083 sia con 100 righe per attività sia senza righe (misurata: nessuna differenza dovuta alle righe).

### Disegno e deviazioni (il codice sta nel commit `7c1dfcd`)

- **Numero del pezzo = primo numero di riga del pezzo**, mai un contatore del motore (`<esecuzione>-<passo>-<tentativo>-<primo numero>`). Se la chiave esiste si legge il pezzo esistente: le righe fino al suo ultimo numero sono già salvate e si scartano, le altre si scrivono sotto `ultimo + 1`. Così un pezzo riscritto dopo un riavvio con un raggruppamento diverso non può cadere su una chiave già presa da contenuto diverso e perdere righe; la riga 4 lo esercita davvero (`conflicts_differ`).
- **Scrittura su disco** (approvata dal coordinatore): file temporaneo con `create_new` + `fdatasync`, poi `std::fs::hard_link` sulla chiave. È creazione solo se assente (il collegamento fallisce con `AlreadyExists` se la chiave esiste), atomica, e la chiave compare solo completa: con una `create_new` diretta sulla chiave un crash a metà scrittura lascerebbe un pezzo troncato che una riscrittura troverebbe «esistente». Un `fsync` della directory per tenant a ogni ciclo, prima del riferimento e della conferma.
- **Conferma al worker** dopo pezzo durevole E riferimento durevole sul bus; cumulativa («fino al numero N», per tentativo); un ciclo di scrittura per volta, quindi N vale per tutte le righe sotto N. I doppioni di righe già durevoli si riconfermano.
- **Tetto**: valori scelti dalla spike (NFR-14 non li fissa): 100 righe e 16.384 byte per tentativo, pezzo chiuso al più dopo 500 ms. Il tetto in righe è per numero di riga (uguale dopo un riavvio); quello in byte conta in memoria e riparte da zero dopo un riavvio (`ponytail:` nel codice).
- **Segnaposto** `[REDACTED]`, sostituzione del valore esatto prima del bus e dell'archivio; il worker di prova non lo applica all'origine (lo fa solo il motore).
- **Manopola della prova di crash**: `ACKHOLD=1500` ritarda di 1,5 s la conferma dopo il pezzo durevole, per avere pezzi scritti con la conferma non ancora partita al kill (come `REC_DELAY` in H3). Senza di essa la finestra è di pochi ms.
- **Riga 2, decisione del coordinatore dopo la prima revisione**: il braccio di confronto è `Anolines` (A a 500 att./s senza righe), non «senza A», così il rapporto isola il costo del log; verificato con una corsa di prova (`-smoke-anolines-`: `lines_written` = 3 × tentativi, A a circa 20.000 attività in 40 s).
- **Nessuna modifica del binario** dopo `7c1dfcd`; gli script (`scen.sh`, `compose.yaml`, `rustfs-proof.sh`, `logd-chain.sh`) sono nei commit successivi.

### Righe, con i numeri

**Riga 1** (`iggy-sqlite-logd-vis-LOG-vis-4-ms-r{1,2,3}.txt`, `LOGA=5x80 TAIL=1`, 1.000 att./s, SQLite): pavimento prima 7,09 / 2,85 / 2,91 ms (p50); 999 / 1.001 / 1.000 att./s; accettate perse 0; `log_visibility` p50 530,43 ms, p99 557,57 / 557,57 / 557,05, p99.9 592 / 591 / 582, max 640 / 649 / 614 ms.

**Riga 2** (`iggy-sqlite-logd-noisy-{withA,Anolines}-4-ms-r{1..6}.txt`). Ordine in tempo: r1 withA, Anolines; r2 Anolines, withA; r3 withA, Anolines; r4 Anolines, withA; r5 withA, Anolines; r6 Anolines, withA. `start_ready_tenant_b` p99 (p50) in ms:

| Coppia | withA | Anolines | d = withA − Anolines |
| --- | --- | --- | --- |
| 1 (withA prima) | 25,49 (17,42) | 23,17 (17,71) | +2,32 |
| 2 (Anolines prima) | 25,92 (17,21) | 24,72 (17,45) | +1,20 |
| 3 (withA prima) | 22,93 (17,41) | 22,61 (17,63) | +0,32 |
| 4 (Anolines prima) | 25,68 (17,17) | 22,40 (17,71) | +3,28 |
| 5 (withA prima) | 25,01 (17,21) | 24,30 (17,76) | +0,71 |
| 6 (Anolines prima) | 23,20 (17,34) | 24,22 (17,57) | −1,02 |
| **Mediana** | **25,25 (17,28)** | **23,70 (17,67)** | |

- R = 25,25 / 23,70 = **1,066** (soglia 1,20). X (mediana delle d con withA prima: 2,32, 0,32, 0,71) = +0,71 ms; Y (con Anolines prima: 1,20, 3,28, −1,02) = +1,20 ms: stesso segno, la differenza si dichiara (misurata): differenza delle mediane +1,55 ms, mediana delle sei d +0,96 ms, dentro il 20 %. Criterio b **rispettato**.
- Criterio a (mediana del p99 di B ≤ 25 ms): `Anolines` **rispettato** (23,70), `withA` **non rispettato** (25,25, 0,25 ms sopra). Condizione di carico: NFR-2 scrive il limite a 1.000 att./s; qui il carico totale è 1.500 att./s (B 1.000 + A 500), il caso dei tenant vicini a uno rumoroso, dove il PRD chiede che gli altri tenant restino entro lo stesso obiettivo; il criterio resta non rispettato. Il p50 (17 ms) resta sopra i 5 ms di NFR-2 in tutti e due i bracci, riportato non giudicato. Quanto di quei 23,7 ms venga dai 500 att./s di A e quanto dalle 3 righe di B non è isolato (in questo blocco non c'è un braccio senza A; la terza parte del blocco D misurò per H2 a 1.000 att./s senza A e senza righe una mediana del p99 di 19,72 ms, con un altro binario e in un'altra ora, quindi non confrontabile alla pari): **ipotesi** che il resto sia il carico di A.
- Criterio c: B 0,0411-0,0412 istruzioni per attività nelle 12 corse (invariate, scarto 0 %); A 0,0828-0,0837 nei due bracci (le righe di A non cambiano le istruzioni); globale 0,0550-0,0554; tutte molto sotto il limite 5 (misurata).
- Validità: `errors=0` e 2.500 accettate su 2.500 nel generatore di A in tutte le 12 corse; `Anolines`: `lines_written` = 3 × `attempts_written` (134.970 = 3 × 44.990) e A tra 19.990 e 20.026 attività; `withA`: 100 righe per attività di A, 0 scartate. `verify OK` e `logcheck OK` ovunque. Pavimento di `fdatasync` prima di ogni corsa nei file (p50 1,93-7,15 ms, in sette corse con p99.9 sopra 60 ms): nessuna corsa porta la riga «macchina non ferma».

**Riga 3** (`iggy-sqlite-logd-secret-{on-4-ms-r1,on-4-ms-r2,off-4-ms-r1}.txt`, `tenants=ab rate=40 n=5 secs=10 warm=2`, 2.400 tentativi e 48.000 righe per corsa): vedi sintesi 3. Segreto sintetico di 31 caratteri per corsa, mai stampato. Il controllo `off` ha `marker_in_chunks=4.800`.

**Riga 4** (`iggy-sqlite-logd-crash-LOG-crash-4-ms-r{1,2,3}.txt`, `LOGSPLIT=1000 ACKHOLD=1500 ERATE=334 CAP=4096`): kill a 12 s con circa 3.000 tentativi in volo; ripresa «in volo (scadenza ripristinata)» 3.031 / 3.031 / 3.014 (16 «da consegnare subito» in r3). Dopo il riavvio il motore ha ricevuto `lines_in` 488.420 / 487.190 / 487.520 righe (quelle rimandate dai worker più le nuove), `dups` 130 / 0 / 0, e ha scritto 44.953 / 45.175 / 45.089 pezzi; i riferimenti sul bus (`refs`) sono più dei pezzi distinti (63.808 contro 60.815 in r1) perché nel conflitto il riferimento al pezzo esistente si riscrive: innocuo, chi legge deduplica per chiave. Ciclo di scrittura del motore ripartito: p50 24-25 ms, p99 57-62 ms, massimo 679-724 ms (osservazione dai file: la coda dopo il riavvio, causa non isolata). Pavimento prima: 2,93 / 2,85 / 7,20 ms.

**Riga 5** (`iggy-sqlite-logd-cap-LOG-cap-4-ms-r1.txt`): vedi sintesi 5.

**Riga 6** (`iggy-sqlite-logd-rustfs-cond-4-ms-r{1,2}.txt`): vedi sintesi 6. Immagine `rustfs/rustfs:1.0.4` (id `2755bcf766f7…`, creata il 2026-10-25), credenziali generate da `gen-env.sh`; il `rustfsadmin` predefinito non è usato.

### Non coperto

- Il tetto in byte dopo un riavvio (riparte da zero); il tenant ricavato dal tentativo e il rifiuto delle righe di un tentativo concluso o non più corrente (C-2, AD-17): la spike si fida del tenant dichiarato dal worker e accetta righe anche dopo l'esito, per non perdere righe non confermate; il pezzo finale che chiude il log; il tempo reale verso il client (AD-18); il motore che scrive i pezzi su RustFS (solo la prova dei due PUT); il segnaposto lato SDK; segreti molto corti o spezzati su più righe; Postgres; 10.000 att./s (nessuna riga di questo blocco, per il pavimento del disco).
- Una corsa senza A (né attività né righe) con le 3 righe di B, per dire quanto del p99 di 23,7 ms viene dal log e quanto da A.
- La conservazione del topic `log-<tenant>` (nessuno lo legge né lo taglia nella spike).

### Comandi per rifare (blocco D, log dei task)

Su minisheep, in `~/ttspikebus` (`env.sh` dal repository, `.env` da `./gen-env.sh`, binario `bin/busspike-logd-7c1dfcd`); tutte le righe 1-5 in sequenza con `./logd-chain.sh rows` (staccato) e `./logd-chain.sh wait`. Una corsa: `env ARM_BIN=$HOME/ttspikebus/bin/busspike-logd-7c1dfcd ARM=<braccio> <variabili> ./ms-run.sh risultati/<file> ./scen.sh iggy sqlite <scena> [chiave=valore]`. Variabili e scene delle righe: nella regola di lettura. RustFS: `./reset.sh rustfs` poi `./ms-run.sh risultati/<file> ./rustfs-proof.sh`.

## Blocco D, log dei task: isolamento del worker rumoroso

Isolamento del 25,25 ms della riga 2 (decisione di Marco, 2026-11-25). Regole di lettura: `risultati/iggy-sqlite-logd-iso-regola-4-ms.txt` (commit `3868163`, 17:07:57, scritte prima di ogni corsa misurata; la prima corsa misurata ha `[stato prima 17:12:20]`). Binario `bin/busspike-logd-7c1dfcd`, sha256 `fd04efcdeefe0425`, controllato prima di partire e scritto in ogni file grezzo, mai ricostruito. Il risultato della riga 2 (criterio a non rispettato) **non cambia**: qui si attribuisce, non si giudica di nuovo. Tre bracci, 6 giri da 3 corse nell'ordine fisso del quadrato latino (18 corse), `lognoisy`, SQLite, `CAP=512`, minisheep: `Anolines` (A a 500 att./s senza righe, B con 3 righe), `Bsolo3` (niente A, B con 3 righe), `Bsolo0` (niente A, B senza righe). Corse di prova non misurate (`-smoke-iso-`): Bsolo0 senza riga `logcheck` e con `verify OK`, `done_b` presente (39.990), Bsolo3 con `lines_written` = 3 × `attempts_written` (134.970 = 3 × 44.990): tutte le regole applicabili come scritte.

**Macchina.** Durante questa serie minisheep ha avuto molta attività dei runner di GitHub (carico 1 min dopo le corse tra 1,5 e 28; nel file grezzo `[stato prima]`/`[stato dopo]`). L'attesa di quiete di `ms-run.sh` ha trattenuto alcune corse fino a 15 minuti (`[attesa]` 835-905 s in tre corse); due corse hanno la riga `[macchina non ferma]` (Anolines r2 originale, Bsolo0 r2): contano e si segnalano, nessuna è esclusa.

**Corse non valide (una).** `Anolines` r2 (`iggy-sqlite-logd-iso-Anolines-4-ms-r2.txt`, `[stato prima 17:55:56]`): V2 non rispettata, `done_b` 36.547 < 38.000 (p99 1.580 ms, p50 442 ms; un runner a 1.770 % di CPU, pavimento di `fdatasync` dopo con p99.9 1.538 ms, macchina non ferma). Rifatta una volta, nella stessa posizione, appena ripresa la serie (`...-r2-ripetizione.txt`, `[stato prima 18:01:00]`, valida: `done_b` 39.774, p99 75,71 ms, p50 34,34 ms); entrambe restano nel rapporto e **nelle mediane vale la ripetizione**. La serie è stata fermata e ripresa una sola volta (alle 17:59, mentre `Bsolo0` r3 era ancora in attesa di quiete e senza file grezzo): la prima versione dello script di catena non rifaceva da sola le corse non valide; la versione ripresa le controlla da sé (V1-V3) e salta le corse già valide. Nessun'altra corsa è non valida.

### Mediane per braccio (6 corse; Anolines r2 = ripetizione)

| Braccio | p99 di avvio di B, mediana (ms) | p50, mediana (ms) | p99 delle 6 corse (r1…r6) |
| --- | --- | --- | --- |
| `Anolines` | **25,335** | 17,855 | 25,55 / 75,71 / 24,40 / 27,61 / 23,15 / 25,12 |
| `Bsolo3` | **22,255** | 15,395 | 176,00 / 83,78 / 22,37 / 22,14 / 21,15 / 22,06 |
| `Bsolo0` | **25,595** | 14,825 | 46,98 / 46,94 / 30,37 / 20,82 / 20,25 / 19,26 |

### Una tabella per giro (`start_ready_tenant_b` p99 (p50) in ms, in ordine di esecuzione)

| Giro | 1º | 2º | 3º |
| --- | --- | --- | --- |
| r1 | Anolines 25,55 (17,79) | Bsolo3 176,00 (18,50) | Bsolo0 46,98 (15,00) |
| r2 | Bsolo3 83,78 (15,06) | Bsolo0 46,94 (25,17) | Anolines 75,71 (34,34) (ripetizione; originale 1.580,03 / 442,37, non valida) |
| r3 | Bsolo0 30,37 (15,42) | Anolines 24,40 (17,60) | Bsolo3 22,37 (16,02) |
| r4 | Anolines 27,61 (17,93) | Bsolo0 20,82 (14,65) | Bsolo3 22,14 (15,26) |
| r5 | Bsolo3 21,15 (15,46) | Anolines 23,15 (17,87) | Bsolo0 20,25 (14,61) |
| r6 | Bsolo0 19,26 (14,50) | Bsolo3 22,06 (15,33) | Anolines 25,12 (17,84) |

### Differenze (dA = Anolines − Bsolo3, dL = Bsolo3 − Bsolo0, dT = Anolines − Bsolo0), in ms sul p99

| Giro | dA | dL | dT |
| --- | --- | --- | --- |
| r1 | −150,45 | +129,02 | −21,43 |
| r2 | −8,07 | +36,84 | +28,77 |
| r3 | +2,03 | −8,00 | −5,97 |
| r4 | +5,47 | +1,32 | +6,79 |
| r5 | +2,00 | +0,90 | +2,90 |
| r6 | +3,06 | +2,80 | +5,86 |

| Differenza | X (primo braccio prima) | Y (primo braccio dopo) | Verdetto (stesso segno) | Mediana delle 6 d | Differenza delle mediane dei bracci |
| --- | --- | --- | --- | --- | --- |
| dA | +2,03 (r1, r3, r4) | +2,00 (r2, r5, r6) | **dichiarata**, positiva (misurata) | +2,015 | +3,08 |
| dL | +36,84 (r1, r2, r5) | +1,32 (r3, r4, r6) | **dichiarata**, positiva (misurata), ma instabile (v. lettura b) | +2,06 | −3,34 |
| dT | +2,90 (r1, r4, r5) | +5,86 (r2, r3, r6) | **dichiarata**, positiva (misurata) | +4,38 | −0,26 |

### Lettura per l'ADR (a-e come fissate nelle regole)

- **a. Avvio di B con il log presente e senza righe (`Bsolo0`)**: mediana del p99 **25,595 ms, sopra il limite di 25 ms** (di 0,6 ms); p50 14,83 ms. Il confronto con la mediana di H2 della terza parte (19,72 ms) è **non alla pari** (altro binario, altra ora, altre condizioni della macchina). Osservazione dai file grezzi, fuori dalle regole e non usata per un verdetto: tre corse di `Bsolo0` sopra il limite (r1 46,98, r2 46,94, r3 30,37; r2 con macchina non ferma e runner a 82,8 % di CPU, in r1 e r3 pavimento di `fdatasync` p99.9 85 e 71 ms) e tre sotto (r4 20,82, r5 20,25, r6 19,26 ms, le ultime della serie). **Ipotesi** che l'attività dei runner sulla macchina abbia alzato il p99 delle prime corse: non isolata, non c'è una corsa ripetuta a macchina ferma.
- **b. Quota delle 3 righe di B (dL)**: dichiarata (X +36,84, Y +1,32 ms, stesso segno): **misurata** come differenza positiva, ma la sua misura non è stabile: X è dominato da due corse `Bsolo3` disturbate (r1 176,00 e r2 83,78 ms contro 46,98 e 46,94 di `Bsolo0`), la mediana delle 6 d è +2,06 ms e la differenza delle mediane dei bracci è −3,34 ms (segno opposto). Nei giri r3-r6 (quattro valori) dL è −8,00 / +1,32 / +0,90 / +2,80 ms. Per l'ADR: le righe di B costano al più qualche millisecondo (ordine di +1…+3 ms, **ipotesi** sul valore), non un valore preciso.
- **c. Quota del carico del tenant vicino senza righe (dA)**: dichiarata, **+2,0 ms** (X +2,03, Y +2,00; mediana delle 6 d +2,015; differenza delle mediane +3,08): **misurata**. È la parte dovuta ai 500 att./s di A senza righe.
- **d. Scomposizione dei 25,25 ms della riga 2**: Bsolo0 25,595 + dL (+2,06, mediana delle 6 d) + dA (+2,015) + righe di A (+1,55 dalla riga 2) = **31,22 ms**, accanto al 25,25 della riga 2 e non al suo posto: le quote vengono da due serie in ore diverse e non si sommano alla pari, e il Bsolo0 di questa serie (25,595) è già alzato dalle prime corse disturbate. Nel solo gruppo r4-r6 (osservazione fuori dalle regole, macchina più ferma, non usata per un verdetto): Bsolo0 20,25 (mediana), Bsolo3 22,06, Anolines 25,12 ms, cioè circa +1,8 per le righe di B, +3,1 per il carico di A senza righe, +4,9 insieme.
- **e. Cause**: restano «ipotesi» tutte quelle che non sono una differenza dichiarata qui con il suo file grezzo: dichiarate e misurate sono dA (+2,0 ms), dT (positiva, mediana +4,38 ms) e dL (positiva ma instabile). Il costo di avvio di H2 stesso (il resto del 25 ms con Bsolo0) e l'effetto dei runner sulla macchina sono ipotesi.

### Per corsa: pavimento di `fdatasync` prima, istruzioni SQL per attività

File grezzi `iggy-sqlite-logd-iso-<braccio>-4-ms-r<n>.txt`. Pavimento prima: p50 / p99.9 in ms. SQL per attività: tenant A / tenant B / globale (A è 0 nei bracci senza A). Attesa di quiete prima della corsa in secondi.

| Corsa | Pavimento prima | SQL A / B / globale | Attesa (s) |
| --- | --- | --- | --- |
| Anolines r1 | 7,32 / 100,67 | 0,0828 / 0,0412 / 0,0550 | 30 |
| Anolines r2 (non valida) | 4,40 / 9,67 | 0,0607 / 0,0361 / 0,0443 | 905 (macchina non ferma) |
| Anolines r2 ripetizione | 6,73 / 76,61 | 0,0797 / 0,0396 / 0,0530 | 120 |
| Anolines r3 | 2,74 / 5,02 | 0,0832 / 0,0412 / 0,0552 | 40 |
| Anolines r4 | 4,65 / 6,20 | 0,0835 / 0,0413 / 0,0554 | 835 |
| Anolines r5 | 2,95 / 13,10 | 0,0832 / 0,0414 / 0,0553 | 130 |
| Anolines r6 | 6,69 / 100,29 | 0,0836 / 0,0411 / 0,0553 | 50 |
| Bsolo3 r1 | 5,90 / 74,88 | 0 / 0,0392 / 0,0392 | 145 |
| Bsolo3 r2 | 2,88 / 33,66 | 0 / 0,0409 / 0,0409 | 45 |
| Bsolo3 r3 | 6,97 / 17,28 | 0 / 0,0414 / 0,0414 | 35 |
| Bsolo3 r4 | 2,85 / 40,73 | 0 / 0,0412 / 0,0412 | 45 |
| Bsolo3 r5 | 6,08 / 10,28 | 0 / 0,0414 / 0,0414 | 40 |
| Bsolo3 r6 | 5,49 / 10,91 | 0 / 0,0414 / 0,0414 | 45 |
| Bsolo0 r1 | 3,02 / 85,63 | 0 / 0,0405 / 0,0405 | 90 |
| Bsolo0 r2 | 15,56 / 26.394,62 | 0 / 0,0396 / 0,0396 | 905 (macchina non ferma) |
| Bsolo0 r3 | 4,43 / 70,59 | 0 / 0,0408 / 0,0408 | 120 |
| Bsolo0 r4 | 7,89 / 46,91 | 0 / 0,0411 / 0,0411 | 55 |
| Bsolo0 r5 | 7,05 / 75,33 | 0 / 0,0412 / 0,0412 | 70 |
| Bsolo0 r6 | 2,90 / 54,88 | 0 / 0,0412 / 0,0412 | 45 |

Tutte molto sotto il limite di 5 istruzioni per attività (NFR-4). Le istruzioni per attività di B variano tra 0,0361 (corsa non valida) e 0,0414 nei tre bracci, senza differenza legata alle righe o ad A; il pavimento di `fdatasync` di `Bsolo0` r2 ha avuto un valore massimo di 26,4 s nella misura (disco fermo, osservata nel file; la corsa vale per le regole).

### Revisione

- **Cosa risponde la serie.** Una sola delle tre quote è determinata: il carico di A senza righe, +2,0 ms (misurata). Il costo di base (`Bsolo0`) e le righe di B **non sono determinati**, e la ragione viene dalle regole stesse: la regola delle coppie dichiara B con le righe più lento di B senza righe, mentre le mediane dei bracci dicono il contrario (22,26 contro 25,60 ms). I numeri dei giri r4-r6 restano «fuori dalle regole» e non decidono nulla.
- **Runner e corse lente: legame non sostenuto dai file.** Riga `cpu runner max` di `[stato prima]` e `[stato dopo]` accanto al p99, in tutte le 19 corse: delle cinque corse valide con p99 sopra 30 ms (Bsolo3 r1 176,00, Bsolo3 r2 83,78, Anolines r2 ripetizione 75,71, Bsolo0 r1 46,98, Bsolo0 r2 46,94) tre hanno i runner a 0,52 % o meno sia prima sia dopo (Bsolo3 r1, Anolines r2 ripetizione, Bsolo0 r1); Bsolo3 r3 ha un runner all'89 % dopo la corsa e un p99 di 22,37 ms. Le due istantanee possono non vedere l'attività durante la corsa, ma i file non sostengono il legame: la causa delle corse lente dei primi giri è **aperta**, e l'«ipotesi» della lettura a non ha il sostegno dei file.
- **Stesso braccio, due serie.** `Anolines` (stesso binario, stesso comando) è 23,70 ms nella riga 2 e 25,34 ms qui: il livello assoluto si sposta di circa 1,6 ms tra un'ora e l'altra; valgono le differenze a coppie. Con un tenant vicino a 500 att./s B sta al limite dei 25 ms anche quando il vicino non scrive righe.
- **Contrasto con la terza parte, per l'ADR.** La lettura a dà 25,6 ms per il motore col log e senza righe, sopra i 25 ms; H2 fu tenuto anche per «avvio p99 19,72 ms ≤ 25». L'ADR non può citare quel limite come rispettato senza mettere accanto questo numero; il costo di base resta aperto con i due valori (19,72 e 25,6).

### Comandi per rifare

`./logd-iso-chain.sh rows` su minisheep (`./logd-iso-chain.sh wait`, `./logd-iso-chain.sh smoke`), con `bin/busspike-logd-7c1dfcd`; le tre varianti: `NOISY=1 LOGA=0x0 LOGB=3x80 ./scen.sh iggy sqlite lognoisy`, `LOGB=3x80 ./scen.sh iggy sqlite lognoisy`, `./scen.sh iggy sqlite lognoisy`, ciascuna dentro `ms-run.sh` con `ARM=LOG-iso-<braccio>`.

## Sentinella (prototipo)

Prototipo di Marco (2026-11-25): un processo di sola lettura che segue i topic `state-*` di Iggy con il gruppo di consumatori `sentinel` e, dalle sole intestazioni utente in chiaro che il motore aggiunge con `META=1` (35 byte grezzi, 46 sul filo), vede se un workflow avanza, è bloccato o ha errori e se il motore è fermo. Non ha chiavi (ingressi e uscite restano sigillati per tenant), non scrive sul bus né sul database, gira sulle CPU `1-4` del motore. Regole di lettura: `risultati/iggy-sqlite-sent-regola-4-ms.txt` (commit `4b54a74`, 19:25:19, scritte prima di ogni corsa misurata; adattamenti accettati dal coordinatore: il ritardo di lettura guida il criterio c della riga 1, la riga 2 passa con «almeno un `overdue`», la riga 4 riporta `margin_lt_1s` e lascia `engine_silent` non giudicato). Binario `bin/busspike-sent-116e57c` (commit `116e57c`), sha256 `514d262500a5f659`, scritto in ogni file grezzo. Script di catena `sent-chain.sh rows` (commit `06d55ce`, 19:28:02, con validità V1-V3 e una ripetizione immediata). La prima corsa misurata ha `[stato prima 19:28:03]`. 21 corse, **nessuna non valida**, nessuna ripetizione. File grezzi `risultati/iggy-sqlite-sent-<riga>-<braccio>-4-ms-r<n>.txt`.

**Macchina.** Minisheep tranquilla in questa serie: carico a 1 min prima delle corse 0,41-1,47 e dopo 0,97-3,38; CPU massima di un runner prima e dopo ogni corsa sotto 0,3 % (0,00-0,26); nessuna corsa con `[macchina non ferma]`; attesa di quiete 0-75 s (righe 1) e 30-55 s (righe 2-5). Pavimento di `fdatasync` prima: p50 1,92-8,45 ms; in 11 corse il p99.9 è sopra 60 ms (fino a 146,7 ms), nessuna sopra 150 ms. A differenza dell'isolamento (runner molto attivi), qui i runner sono rimasti fermi: **i numeri della riga 1 sono stabili**.

### Sintesi

1. **Riga 1, costo: superata.** Mediana del p99 di avvio: OFF **19,70 ms**, ON **19,94 ms**; R = **1,012** (soglia 1,20). Differenza **non dichiarabile (effetto d'ordine)**: X +1,02 ms (ON per primo), Y −0,59 ms (ON per secondo). Criterio a (≤ 25 ms per braccio): rispettato in tutti e due. Criterio c: in tutte le 6 corse ON ritardo di lettura p99 16,8-17,8 ms (limite 1.000), `gaps=0`, 0 allarmi di ogni tipo. Criterio d: istruzioni SQL per attività 0,0414 (OFF) contro 0,0416 (ON), scarto 0,5 % (limite 10 %). CPU del sentinella 0,49 s per corsa. (misurata)
2. **Riga 2, motore ucciso e lasciato fermo: superata in 3 corse su 3.** `engine-silent` a +2,08 / +2,09 / +2,07 s dal kill (limite 3 s); primo `overdue` a +4,10 / +4,02 / +4,00 s; `overdue` 3.025 / 3.010 / 3.033 contro 3.025 / 3.010 / 3.033 tentativi in volo (tutti); `RECOVERED engine` 197 / 65 / 217 ms dopo `READY`; `verify OK`. (misurata)
3. **Riga 3, errori: superata in 2 corse su 2.** Allarmi `retry` 500 e 483, uguali ai tentativi falliti (500 e 483, dal registro dei worker); ritardo dall'append del Timer all'allarme p99 57,5 e 86,9 ms (limite 1.000); 0 `overdue`, 0 `engine-silent`; `verify OK`. (misurata)
4. **Riga 4, worker ucciso, motore vivo: superata in 2 corse su 2.** 0 `overdue` in tutte e due (il motore ha riconsegnato 1.524 e 1.502 tentativi e il sentinella ha visto 1.532 e 1.514 scadenze risolte dopo la loro scadenza); `margin_lt_1s` 0 in tutte e due (margine mediano 1,956 e 1,958 s sui 2 s di tolleranza: il motore agisce circa 40 ms dopo la scadenza); `engine_silent` 0 e 0 (non giudicato). (misurata)
5. **Riga 5, nuovo tenant: superata in 2 corse su 2.** Primo evento del topic nuovo letto 285 e 276 ms dopo la creazione (limite 2.000), `topics=3`, `verify OK`. (misurata)

### Riga 1, per coppia (`iggy-sqlite-sent-cost-{OFF,ON}-4-ms-r{1..6}.txt`)

`start_ready` p99 (p50) in ms. Ordine in tempo: r1 OFF, ON; r2 ON, OFF; r3 OFF, ON; r4 ON, OFF; r5 OFF, ON; r6 ON, OFF.

| Coppia | OFF | ON | d = ON − OFF | SQL per attività OFF / ON | Ritardo di lettura p99 (ON, ms) |
| --- | --- | --- | --- | --- | --- |
| 1 (OFF prima) | 19,01 (14,74) | 19,66 (14,38) | +0,65 | 0,0413 / 0,0416 | 17,23 |
| 2 (ON prima) | 18,88 (14,54) | 19,90 (14,87) | +1,02 | 0,0416 / 0,0416 | 17,76 |
| 3 (OFF prima) | 20,57 (14,77) | 19,98 (14,19) | −0,59 | 0,0414 / 0,0416 | 17,60 |
| 4 (ON prima) | 20,37 (14,65) | 19,57 (14,36) | −0,80 | 0,0414 / 0,0417 | 17,04 |
| 5 (OFF prima) | 21,57 (14,59) | 20,21 (14,25) | −1,36 | 0,0414 / 0,0416 | 16,82 |
| 6 (ON prima) | 19,02 (14,71) | 20,59 (14,28) | +1,57 | 0,0416 / 0,0416 | 17,65 |
| **Mediana** | **19,70 (14,68)** | **19,94 (14,32)** | mediana delle 6 d +0,03 | **0,0414 / 0,0416** | |

- R = 19,94 / 19,70 = **1,012**; X (ON per primo: +1,02, −0,80, +1,57) = +1,02 ms; Y (ON per secondo: +0,65, −0,59, −1,36) = −0,59 ms: segni opposti, **non dichiarabile**; la differenza delle mediane (+0,25 ms) sta nel rumore tra corse (OFF varia da 18,88 a 21,57 ms). Criterio b rispettato (R ≤ 1,20). Il costo dell'intestazione e della lettura non si distingue dal rumore a 1.000 att./s. (misurata)
- Sentinella (6 corse ON): eventi letti 139.469-139.500 per corsa, `gaps=0`, lag in eventi sempre 0 (1 è il valore minimo dell'istogramma), CPU 0,49 s per corsa. Pavimento prima: OFF p50 2,73-3,02 ms (p99.9 4,7-41 ms), ON p50 2,68-7,08 ms (p99.9 7,4-99,7 ms).

### Righe 2-5, per corsa

| Corsa | `[stato prima]` | Esito | Numeri |
| --- | --- | --- | --- |
| ekilldown r1 | 19:49:27 | superata | silent +2.083 ms; primo overdue +4.104 ms; overdue 3.025 / in volo 3.025; RECOVERED +197 ms dopo READY (READY a +20.222 ms dal kill); episodi `engine-silent` 4, `RECOVERED` 4; ritardo di lettura p99 74,5 ms; `gaps=0` |
| ekilldown r2 | 19:51:10 | superata | silent +2.093 ms; primo overdue +4.015 ms; overdue 3.010 / 3.010; RECOVERED +65 ms; episodi 4 / 4; ritardo p99 54,8 ms; `gaps=0` |
| ekilldown r3 | 19:52:40 | superata | silent +2.074 ms; primo overdue +3.998 ms; overdue 3.033 / 3.033; RECOVERED +217 ms; episodi 3 / 3; ritardo p99 23,2 ms; `gaps=0` |
| retry r1 | 19:54:22 | superata | `retry` 500 = falliti 500; latenza Timer→allarme p50 44,1 / p99 57,5 ms; 0 overdue, 0 silent |
| retry r2 | 19:55:38 | superata | `retry` 483 = falliti 483; latenza p50 44,7 / p99 86,9 ms; ritardo di lettura p99 285 ms; 0 overdue, 0 silent |
| wkill r1 | 19:56:34 | superata | 0 overdue; risolte dopo la scadenza 1.532, `margin_lt_1s` 0, margine mediano 1.956 ms; silent 0 |
| wkill r2 | 19:58:29 | superata | 0 overdue; risolte 1.514, `margin_lt_1s` 0, margine mediano 1.958 ms; silent 0 |
| newtenant r1 | 20:00:15 | superata | primo evento letto 285 ms dopo la creazione; `topics=3` |
| newtenant r2 | 20:01:44 | superata | 276 ms; `topics=3` |

Stato della macchina per queste corse (dai file grezzi): carico 1 min prima/dopo ekilldown 0,92/1,64, 1,25/1,94, 1,32/1,44; retry 0,95/0,97, 0,57/1,43; wkill 0,88/2,81, 1,43/2,68; newtenant 1,42/3,38, 1,27/1,61; CPU massima di un runner prima/dopo sempre sotto 0,3 %. Pavimento di `fdatasync` prima: p50 1,92-8,45 ms, p99.9 4,1-146,7 ms.

### Note di disegno (osservazioni e opzioni, non verdetti)

- **`engine-silent` è silenzio, non «motore fermo».** Scatta quando il lavoro aspetta legittimamente, oltre che quando il motore è fermo. Osservato (misurato) nella riga 2: un solo kill dà 3-4 episodi per corsa (4, 4, 3 con altrettanti `RECOVERED`): il primo è il silenzio vero (+2,07-2,09 s dal kill), gli altri dopo il riavvio, quando le circa 3.000 attività recuperate aspettano le loro scadenze e il motore tace più di 2 s tra una raffica di eventi e l'altra. Nelle corse senza arresto (righe 1, 3, 4, 5) gli episodi sono 0. Un evento di battito del motore (scritto a intervalli fissi sul bus anche senza lavoro) separerebbe i due casi: **non è costruito**, è solo un'opzione per l'ADR.
- **Limiti del prototipo** (dal codice, non misurati come limite): `tick` scorre tutti i timer aperti a ogni passo di 100 ms, quindi costa O(timer aperti) (qui fino a circa 3.000, nessun problema visibile; con centinaia di migliaia servirebbe un indice per scadenza); l'insieme `alarmed` non si restringe mai (cresce di una voce per allarme di tipo overdue o retry, nella riga 2 a 3.000 voci, in un servizio lungo andrebbe potato alla chiusura dell'esecuzione); `due_us` viene dall'orologio del motore e `now` da quello del sentinella: sulla stessa macchina coincidono (qui), su un altro host lo scarto degli orologi si somma ai 2 s di tolleranza (e il ritardo di lettura usa l'ora di append del server di Iggy).
- **Avvertenza del prototipo, già nelle regole:** all'avvio il sentinella stampa un `SENTINEL_ERR` («rebalance in progress») per ogni topic e poi si riprende; il gruppo e la sua posizione salvata dal server funzionano (nessun salto, `gaps=0` in tutte le corse), ma la ripresa dopo un riavvio del sentinella non è provata.
- **Avvisi di `cargo check --features old`:** a `2b4bdd3` (contati qui) c'è 1 avviso (`unused import std::time::Duration`, preesistente); a `4b54a74` ce ne sono 6. Questa fase ne ha aggiunti **5**: sono codice morto del modulo `sentinel` (`GRACE_US`, `SILENT_US`, `decode`, `State` e i suoi metodi), perché con `--features old` la funzione `run` è esclusa e restano senza uso. Non toccano la compilazione di default (clippy con `-D warnings` è pulito).
- **Per il costo di base aperto dall'isolamento (nota del coordinatore).** Il braccio OFF della riga 1 è il motore con il codice del log dei task e senza righe, a 1.000 att./s, su una macchina ferma (CPU massima di un runner sotto 0,3 % prima e dopo ogni corsa). La sua mediana del p99 di avvio è 19,70 ms: vicina ai 19,72 ms di H2 della terza parte e lontana dai 25,6 ms di `Bsolo0` nell'isolamento. È un'osservazione, non un confronto alla pari: binario, scena (`logrun` con un tenant), tenant e ora sono diversi. Va nella direzione di un isolamento alzato dalle condizioni della macchina, ma non lo prova. Per le regole il costo di base resta aperto; l'ADR riporta i tre valori.

### Non coperto

Come nelle regole: conservazione che aspetta il sentinella (e se la posizione del gruppo limiti la cancellazione dei segmenti del motore: non verificato, nelle corse nessun segmento si chiude); crash e riavvio del sentinella; più membri dello stesso gruppo; Postgres; 10.000 att./s; consegna degli allarmi a una persona; esecuzioni aperte a lungo senza eventi (darebbero `engine-silent`).

### Comandi per rifare

Su minisheep, in `~/ttspikebus` (`.env` da `./gen-env.sh`, binario `bin/busspike-sent-116e57c`): `./sent-chain.sh rows` staccato e `./sent-chain.sh wait`; singole corse con `env ARM_BIN=… ARM=<braccio> <variabili> ./ms-run.sh risultati/<file> ./scen.sh iggy sqlite <scena>`, variabili e scene nella regola di lettura (`SENT=1 META=1`, `DOWN=20`).

## Blocco E, credenziale di sola lettura del bus

Credenziale Iggy di sola lettura (riga del blocco E del piano) e, dalla decisione di Marco sul sentinella, cancello del sentinella: i metadati degli eventi di stato sono in chiaro per chiunque legga il bus. Misurata contro il server vero (`apache/iggy:0.10.0`, versione scritta in ogni file grezzo), con `busspike busauth case=<caso>` (client Iggy 0.12.0 già nel crate, nessuna dipendenza nuova). Regole di lettura: `risultati/iggy-sqlite-cred-regola-4-ms.txt` (commit `339a350`, 20:17:15, scritte prima della prima corsa; la prima corsa ha `[stato prima 20:17:18]`), con un'aggiunta del 2026-11-25 (commit `8d9d753`, 20:27:49) scritta prima delle ripetizioni. Due serie, due corse per caso, bus pulito prima di ogni corsa:
- prima serie, binario `1bfe5d3f2bce205b` (commit `45e4663`): control, ro-a, ro-a-s, ro-sent, sentinel;
- ripetizione di control, ro-sent e sentinel con il binario corretto `d07c64fc7d46873e` (commit `818fa0d`), file `…-ripetizione.txt`, dopo due difetti dello strumento (sotto). ro-a e ro-a-s non erano toccati dai difetti e restano quelli della prima serie. File grezzi `risultati/iggy-sqlite-cred-<caso>-4-ms-r<n>[-ripetizione].txt` (16).
Password degli utenti di prova generate a ogni corsa (64 cifre esadecimali), mai stampate né scritte nei file grezzi o nei commit; le uniche copie vivono in `out/ro-sent.pw` (modo 600) durante la corsa del sentinella.

### Sintesi

1. **Utente di sola lettura sul solo tenant A (`ro-a`): superato, 2 corse su 2.** Permessi: nessun permesso globale, sullo stream `tt` solo `read_topic` + `poll_messages` sul topic `state-<A>`. Lettura di A riesce (3 messaggi); lettura di B rifiutata con **codice 41 `Unauthorized`**; invio a A, creazione di un topic e cancellazione di un topic rifiutati con **41 `Unauthorized`**; anche l'elenco dei topic è rifiutato (41). (misurata)
2. **Aggiungere `read_stream` apre B (`ro-a-s`): FAIL atteso, 2 corse su 2.** Con `read_stream` sullo stream l'utente legge anche B (3 messaggi) e vede l'elenco dei topic; invio, creazione e cancellazione restano rifiutati (41). Reperto per l'ADR: per il tenant A non si concede `read_stream`, solo i permessi sul topic. (misurata)
3. **Sentinella (`ro-sent`): superato, 2 corse su 2 (ripetizione, 12 PASS su 12 ciascuna).** Permessi: nessun permesso globale, sullo stream `tt` (livello di stream) `read_stream` + `read_topics` + `poll_messages`; nessun `send_messages`, nessun `manage_*`. Con quel set l'utente: elenca i topic; vede un topic `state-*` creato da root dopo il suo accesso; **crea il gruppo `sentinel`** sul topic nuovo; si unisce al gruppo; legge da gruppo salvando la posizione (3 messaggi, posizione salvata presente); legge da un gruppo creato da root. Invio a `state-<A>`, creazione e cancellazione di un topic: **41 `Unauthorized`**. (misurata)
4. **Che cosa serve al sentinella (reperto per l'ADR).** Creare e unirsi al gruppo non richiede nulla oltre la lettura: nel set provato bastano `read_topics` (che per la documentazione del server include la gestione dei gruppi) e `poll_messages`, quindi per un tenant nuovo **non serve che root crei il gruppo**; il sentinella lo crea da sé. Il set provato non è stato ridotto (non si è provato senza `read_stream`): è «sufficiente», non «minimo». (misurata per «sufficiente»; «minimo» non misurato)
5. **Il vero sentinella con l'utente `ro-sent`: superato, 2 corse su 2 (ripetizione).** `SENT=1 META=1 SENTUSER=1 … newtenant rate=100 n=10 secs=15` (circa 1.000 att./s per 15 s, a 4 s un topic nuovo creato da root): `verify OK`; `events` 49.601 in entrambe, `topics=3` (due tenant più il nuovo, scoperti con quell'utente), `gaps=0`, 0 allarmi; primo evento del topic nuovo letto **288 e 137 ms** dopo la creazione (limite 2.000); posizione salvata del gruppo `sentinel` su `state-<A>` presente (`Some(49599)`, letta con root). Righe `SENTINEL_ERR`: 3 e 2, tutte «5009 rebalance in progress» all'avvio, nessun errore di permesso. (misurata)
6. **Controllo con root: superato, 10 PASS su 10 in entrambe le corse della ripetizione.** Le stesse operazioni riescono, quindi i rifiuti 41 vengono dai permessi e non da chiamate rotte. (misurata)
7. **Non c'è un permesso per modello di nome.** Iggy chiede i permessi per numero di stream e di topic (`streams: BTreeMap<id, StreamPermissions>`, `topics: BTreeMap<id, TopicPermissions>`): un modello `state-*` non esiste. Il sentinella usa il permesso a livello di stream, che copre anche i topic nati dopo (visto: «nuovo visto: true») ma non permette di escludere un tenant; l'utente di un solo tenant (`ro-a`) invece è legato al numero del topic, quindi i permessi di un topic nuovo vanno concessi alla sua creazione. (misurata per la copertura dei topic nuovi; il resto dalla struttura dei permessi)

### Per caso

| Caso | Prima serie (r1 / r2) | Ripetizione (r1 / r2) | Note |
| --- | --- | --- | --- |
| control (root) | 9 PASS 1 FAIL / 9 PASS 1 FAIL | **10 PASS 0 FAIL / 10 PASS 0 FAIL** | il FAIL era la lettura da gruppo subito dopo l'unione (5009, difetto dello strumento) |
| ro-a | 6 PASS 0 FAIL / 6 PASS 0 FAIL | (non rifatta) | 41 `Unauthorized` su B, invio, creazione, cancellazione, elenco |
| ro-a-s | 4 PASS 2 FAIL / 4 PASS 2 FAIL | (non rifatta) | `read_stream` apre la lettura di B e l'elenco dei topic |
| ro-sent | 10 PASS 2 FAIL / 10 PASS 2 FAIL | **12 PASS 0 FAIL / 12 PASS 0 FAIL** | i 2 FAIL erano le letture da gruppo (5009) |
| sentinel (vero sentinella, utente ro-sent) | non valida (verify FALLITA, 0 attività concluse) | **superata / superata** | v. sintesi 5 |

Primi `[stato prima]`: prima serie 20:17:18 (control r1), ripetizione 20:27:58. Pavimento di `fdatasync` nei file grezzi, macchina tranquilla (carico 1 min 0,5-0,9).

Nota del coordinatore sulle ripetizioni. Le regole (commit `339a350`) dicevano di rifare una volta il solo `control` quando una sua chiamata non riesce, e di rifare le corse con `verify` fallita. La ripetizione di `ro-sent` va oltre la lettera delle regole. È sostenuta dal controllo: nella stessa serie anche root ha fallito la stessa lettura da gruppo con lo stesso codice 5009, quindi i 2 FAIL di `ro-sent` della prima serie vengono dallo strumento, non dai permessi (misurata, con i file della prima serie). Il verdetto su `ro-sent` poggia sulla ripetizione. La prima serie resta nella tabella con i suoi FAIL.

### Reperti dello strumento (due difetti, trovati dal controllo e dalla verifica)

- **5009 «rebalance in progress».** Subito dopo l'unione a un gruppo, la prima lettura da gruppo dà il codice 5009 («Consumer group member … does not own partition: 0 at the current generation (rebalance in progress)») anche a root; si risolve riprovando dopo 200 ms con una nuova unione (come fa il sentinella: da qui i `SENTINEL_ERR` di avvio). Nelle ripetizioni la lettura da gruppo riesce al secondo tentativo. Non è un errore di permessi. (misurata)
- **Un evento non sigillato in un topic di stato fa cadere il proiettore.** Nella prima serie `setup-sent` scriveva tre eventi di prova nei topic di stato: il proiettore non li decifra e non conclude nessuna attività (0 concluse, `verify FALLITA` per tutte e 12 le verifiche). Corretto (`setup-sent` e `offsets` non creano topic né scrivono). Osservazione per l'ADR, non verdetto: **chiunque possa scrivere sul topic di stato può fermare il motore**, quindi il permesso `send_messages` sui topic `state-*` va riservato al motore. (misurata come effetto; la prova era involontaria)

### Non coperto

Rotazione o scadenza delle password, token personali di accesso, TLS verso Iggy, piu' utenti di sola lettura, riduzione del set di `ro-sent` al minimo (per esempio senza `read_stream`), il permesso per un topic di tenant creato dopo l'utente `ro-a`, Valkey.

### Comandi per rifare

Su minisheep, in `~/ttspikebus` (`.env` da `./gen-env.sh`, binari `bin/busspike-cred-45e4663` e `bin/busspike-cred-818fa0d`): `./cred-chain.sh rows` (prima serie) e `./cred-chain.sh redo` (ripetizione), staccati, con `./cred-chain.sh wait`; un caso solo: `./cred-chain.sh case <control|ro-a|ro-a-s|ro-sent>` dopo `. ./env.sh`; il sentinella: `SENT=1 META=1 SENTUSER=1 ./scen.sh iggy sqlite newtenant rate=100 n=10 secs=15`.

## Blocco E, processo ausiliario (1.000 att./s)

Processo ausiliario di AD-6 (sottocomando `aux` dello stesso binario, figlio del motore con `env_clear`, IPC a frame con id su pipe, un processo, un task scrittore e uno lettore nel motore) con filtro `if`, mappatura jq e controllo degli schemi d'ingresso (sul mappato) e d'uscita. Regole: `risultati/iggy-sqlite-aux-regola-4-ms.txt` (commit `4e5e158`, 21:07:25, con l'aggiunta sul binario con `PREPARE` in pipeline, scritta prima di ogni corsa misurata; la prima corsa misurata ha `[stato prima 21:07:26]`). Binario `bin/busspike-aux-af23e79` (commit `af23e79`), sha256 `f2152947f38633cf`. Dopo la revisione il percorso `PREPARE` è stato messo in pipeline (scelta delle attività del giro, una richiesta per ciascuna senza attendere, poi attesa nell'ordine di scelta): le corse di prova del primo binario non contano. Corse di prova con il binario finale: `-smoke-aux-…-r2`. 15 corse misurate, **nessuna non valida**. File grezzi `risultati/iggy-sqlite-aux-<riga>-<braccio>-4-ms-r<n>.txt`.

Dipendenze nuove (tutte pinnate, nessun TLS né HTTP tra i 32 crate nuovi): `jaq-core 3.1.2`, `jaq-std 3.0.4`, `jaq-json 2.0.3` (MIT), `jsonschema 0.61.1` con `default-features = false` (MIT; senza `resolve-http` né `resolve-file`), `serde_json` (già presente). Licenze dei crate nuovi: MIT, MIT OR Apache-2.0, Apache-2.0, MIT-0 e `self_cell 1.3.0` con **`Apache-2.0 OR GPL-2.0-only`: si elegge Apache-2.0** (scelta approvata dal coordinatore).

**Macchina.** Tranquilla: carico a 1 min prima delle corse 0,61-1,42 e dopo 1,10-2,00; CPU massima di un runner prima e dopo sotto 0,15 % in ogni corsa (0,00-0,14); nessuna corsa con `[macchina non ferma]`. Pavimento di `fdatasync` prima: p50 1,93-8,26 ms; il p99.9 ha picchi di 68,9 / 67,3 / 182,4 / 184,5 ms in quattro corse della riga 1 (OFF r2, ON r3, OFF r5, ON r6).

### Sintesi

1. **Riga 1, costo a 1.000 att./s: superata.** Mediana del p99 di `start_ready`: OFF **19,55 ms**, ON **20,05 ms**; R = **1,026** (soglia 1,20). Differenza **non dichiarabile (effetto d'ordine)**: X (ON per primo, coppie 2/4/6: +1,55, +1,06, +34,53) = +1,55 ms, Y (ON per secondo, coppie 1/3/5: −0,53, −0,22, +0,70) = −0,22 ms; mediana delle 6 d +0,88 ms. Criterio a (≤ 25 ms): rispettato in tutti e due i bracci. Criterio c: istruzioni SQL per attività 0,0415 (OFF) contro 0,0416 (ON), scarto 0,2 % (limite 10 %). Validità: in tutte e 6 le corse ON `calls_prepare` = `calls_check` = attività concluse (esattamente 2 chiamate per attività completata: `calls_prepare` e `calls_check` uguali alle `attivita_concluse` dell'ultimo `verify`), 0 errori dell'ausiliario, `verify OK`, `activities_per_s` 1.000-1.006. (misurata)
2. **Costo dell'ausiliario (misurata, mediane delle 6 corse):** CPU dell'ausiliario **0,072 s per 1.000 attività** (0,069-0,072); CPU del motore per 1.000 attività OFF 0,098 contro ON 0,116 (+18 %); andata e ritorno del motore con l'ausiliario p50 **0,23 ms** e p99 1,88 ms (misurata dall'invio alla risposta, con la coda della pipeline: 0,29 / 3,48 ms nella corsa r6); RSS dell'ausiliario 14,1-15,0 MB.
3. **Riga 2, controlli negativi: superati.** schema: 60 ingressi su 600 senza `name`; 60 righe `AUX_INPUT_INVALID … 0 0: /name: null is not of type "string"` (nomina il campo) = `input_invalid` 60 = `accettate_perse` 60; le 540 esecuzioni valide concludono (`calls_check` 540); nessun panic, nessuna riga `AUX_CADUTO` né `AUX_EXIT`, `errors=0`, l'ausiliario resta vivo. if falso: `skipped` 600 = `calls_prepare` 600, `calls_check` 0, `verify OK` (le 600 esecuzioni si concludono senza consegna), nessuna riga di errore. (misurata)
4. **Riga 3, memoria: NON rispettata a riposo.** A riposo (60 s dopo l'avvio, nessuna esecuzione): Iggy 76.176 kB, motore 26.484 kB, ausiliario 13.836 kB, worker 15.612 kB (escluso da NFR-1); **somma motore + ausiliario + Iggy = 116.496 kB** (113,8 MB) contro 81.920 kB. Sotto carico a 1.000 att./s: Iggy 78.232 kB, motore 35.784 kB, ausiliario 14.324 kB, worker 16.896 kB; somma 128.340 kB (riportata, non giudicata). **La deroga di Marco sul requisito di RAM (Iggy 92-96 MB nella fase 2) vale ancora**: Iggy da solo è 74 MB dei 114; motore + ausiliario (senza il bus) fanno 40,3 MB a riposo; **la quota dell'ausiliario è 13,8 MB** (circa 14 MB sotto carico). Non si giudica oltre: il giudizio contro 80 MB riguarda l'intero conto di NFR-1, non l'ausiliario. (misurata)

### Riga 1, per coppia (`iggy-sqlite-aux-aux1k-{OFF,ON}-4-ms-r{1..6}.txt`)

`start_ready` p99 (p50) in ms; ordine in tempo: r1 OFF, ON; r2 ON, OFF; r3 OFF, ON; r4 ON, OFF; r5 OFF, ON; r6 ON, OFF.

| Coppia | OFF | ON | d = ON − OFF | SQL per attività OFF / ON | CPU motore per 1.000 att. OFF / ON | CPU ausiliario per 1.000 att. | `aux_rtt` p50 / p99 (ms) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 (OFF prima) | 20,18 (14,70) | 19,65 (14,79) | −0,53 | 0,0414 / 0,0416 | 0,098 / 0,116 | 0,071 | 0,22 / 1,90 |
| 2 (ON prima) | 19,17 (14,50) | 20,72 (14,69) | +1,55 | 0,0414 / 0,0416 | 0,099 / 0,115 | 0,072 | 0,23 / 1,88 |
| 3 (OFF prima) | 19,98 (14,62) | 19,76 (14,94) | −0,22 | 0,0416 / 0,0418 | 0,098 / 0,117 | 0,072 | 0,23 / 1,88 |
| 4 (ON prima) | 19,28 (14,58) | 20,34 (14,80) | +1,06 | 0,0414 / 0,0416 | 0,098 / 0,116 | 0,072 | 0,23 / 1,88 |
| 5 (OFF prima) | 19,04 (14,54) | 19,74 (14,80) | +0,70 | 0,0416 / 0,0416 | 0,097 / 0,117 | 0,072 | 0,23 / 1,85 |
| 6 (ON prima) | 19,81 (14,67) | 54,34 (15,24) | +34,53 | 0,0416 / 0,0409 | 0,099 / 0,114 | 0,069 | 0,29 / 3,48 |
| **Mediana** | **19,55 (14,60)** | **20,05 (14,80)** | +0,88 | **0,0415 / 0,0416** | **0,098 / 0,116** | **0,072** | **0,23 / 1,88** |

- R = 20,05 / 19,55 = **1,026**; X = +1,55 ms (ON per primo), Y = −0,22 ms (ON per secondo): segni opposti, **non dichiarabile**. La coppia 6 (ON 54,34 ms, p99.9 del pavimento del disco prima della corsa 184,5 ms e `activities_per_s` 1.006) pesa su X e sulla media; senza di lei X varrebbe +1,3 ms (osservazione, non usata per il verdetto). Criterio b **rispettato**. La differenza di mediana (+0,5 ms) è dello stesso ordine del rumore tra corse (OFF 19,04-20,18 ms). (misurata)
- Il costo visibile dell'ausiliario a questo ritmo sta nella CPU (motore +18 %, più 0,072 s per 1.000 attività nell'ausiliario) più che nel p99 di avvio. Non si conclude oltre sul 10.000/s, che ha le sue regole.

### Latenza del passo (`step_latency`): metrica non validata, solo riportata

Mediana delle 6 corse del p50 / p99: OFF **28,9 / 36,6 ms**, ON **17,5 / 37,0 ms**. **L'inversione vista nella prova si ripete in tutte le corse misurate**: nel braccio OFF il passo risulta più lento (p50 circa 29 ms) che nel braccio ON (p50 circa 17,5 ms), pur facendo ON più lavoro; `start_ready` p50 è invece 14,6 contro 14,8 ms. La causa è **aperta (ipotesi)**: la misura va da «pronta» al giudizio di `Done` e dipende dal momento in cui il ciclo del motore elabora il risultato, che a sua volta dipende dal limite di gruppi in volo sul bus; con l'ausiliario il ciclo attende le risposte e potrebbe raggruppare i risultati in modo diverso. Nessun criterio usa questa metrica.

### Righe 2-3, per corsa

| Corsa | `[stato prima]` | Esito | Note |
| --- | --- | --- | --- |
| auxneg schema | 21:28:38 | superata | v. sintesi 3; macchina: runner 0,01/0,05 %, pavimento p50 2,82 ms |
| auxneg if | 21:29:45 | superata | v. sintesi 3; runner 0,00/0,03 % |
| auxrss | 21:30:52 | NFR-1 a riposo non rispettato (con deroga) | v. sintesi 4; runner 0,14/0,03 % |

### Come lavora l'ausiliario (note di disegno, non verdetti)

- `PREPARE` in pipeline: il giro di `dispatch` sceglie le attività (stesso turno tra i tenant e stesso tetto), lancia una richiesta per ciascuna e poi le attende nell'ordine; con questo il costo non è più un andata e ritorno seriale nel ciclo del motore (nella prova col primo binario la CPU dell'ausiliario era 0,083 s per 1.000 attività e la mediana dell'andata e ritorno 0,09 ms; con la pipeline 0,072 e 0,23 ms, perché le richieste fanno coda nell'ausiliario).
- Un'uscita non valida segna il tentativo come fallito (si ritenta); un ingresso non valido non si consegna e non si ritenta; il messaggio di validazione può riportare il valore del campo e va ripulito prima di un uso reale.

### Non coperto

`PR_SET_DUMPABLE` e i core dump (AD-6); **un ausiliario che cade non viene riavviato: il motore tiene il lavoro (le attività tornano in testa alla coda dei pronti) ma smette di avanzare**; **l'ingresso non valido non ha un fallimento terminale: l'esecuzione resta bloccata**; più processi ausiliari; `call http`; il tenant vicino con schemi pesanti del benchmark a carico misto del PRD; Postgres; la compilazione dell'interprete per `wasm32-wasip1` (la spine la richiede, qui non si verifica); il costo a 10.000 att./s (su bigsheep, con regole proprie).

### Comandi per rifare

Su minisheep, in `~/ttspikebus` (`.env` da `./gen-env.sh`, binario `bin/busspike-aux-af23e79`): `./aux-chain.sh rows` staccato (poi `tail chain-aux.log`); singole corse: `PAYLOAD=json [AUX=1] ./scen.sh iggy sqlite aux1k`, `PAYLOAD=json AUX=1 INBAD=10 ./scen.sh iggy sqlite auxneg rate=50 n=1 secs=10 warm=2`, `PAYLOAD=json AUX=1 INFLAG=0 …`, `PAYLOAD=json AUX=1 ./scen.sh iggy sqlite auxrss`.

## Blocco E, processo ausiliario a 10.000 att./s (bigsheep)

Il processo ausiliario di AD-6 (stesso binario `bin/busspike-aux-af23e79`, commit `af23e79`, sha256 `f2152947f38633cf`, `PREPARE` in pipeline) a 10.000 att./s su **bigsheep** (Ryzen 7 9700X, 8 core / 16 thread, 60 GB, NVMe Samsung 990 PRO, Arch, kernel 7.2.4, governor `powersave` non modificato e scritto in ogni file grezzo). I numeri di bigsheep si confrontano solo tra loro, mai con le serie di minisheep. Regole: `risultati/bigsheep-iggy-sqlite-aux10k-regola-4-ms.txt` (commit `4371c8c`, 21:49:30, scritte prima di ogni corsa misurata); la prima corsa misurata ha `[stato prima 22:04:56]` (dopo tre attese del cancello del disco, v. sotto). File grezzi `risultati/bigsheep-iggy-sqlite-aux10k-<OFF|ON>-4-ms-r<n>.txt` (con i campioni di I/O; contengono 690 righe `CG <id>` di 64 cifre esadecimali = identificatori dei cgroup dei container, nessuna password), più `bigsheep-aux10k-floor-4-ms.txt`, `bigsheep-aux10k-chain-4-ms.txt` (registro del cancello e della catena) e la corsa di prova `…-smoke-OFF-…`.

**Preparazione.** Immagine copiata da minisheep con `docker save`/`docker load` (nessun pull): `apache/iggy:0.10.0` (e `valkey/valkey:9.1.4`, solo per il contenitore usa e getta che svuota i dati). Verificata uguale: stessi strati (impronta di `RootFS.Layers` uguale), stessa configurazione, stessa data di creazione; l'identificatore mostrato da `docker images` differisce (4574944d61b9 contro 78f25266c7e9) perché bigsheep usa l'archivio containerd, che mostra l'impronta del manifest. L'immagine resta caricata su bigsheep.

**Posizionamento.** Motore, worker e Iggy sui core fisici 1-4 (CPU 1-4; i fratelli SMT 9-12 senza carico nostro); generatore di carico, campionatore e script di corsa sui core fisici 5-7 (CPU 5,6,7; fratelli 13-15 liberi).

**Disco.** Pavimento di `fdatasync` prima delle regole, tre misure: p50 4,92 / 4,93 / 4,93 ms, p99.9 6,13 / 5,20 / 9,75 ms. **Il cancello «p50 ≤ 4,0 ms» non è mai rispettato su questo disco** (il pavimento è stabile a 4,9 ms): tutte e quattro le corse hanno aspettato le tre attese da 5 minuti e sono partite comunque (4 volte «ancora fuori soglia dopo 3 attese», 0 volte «disco tranquillo»; p50 nelle misure del cancello 4,92-4,96 ms, p99.9 6,5-16,1 ms), come previsto dalle regole.

**Prova non misurata (3 minuti, OFF).** 9.999 att./s sull'intera corsa, minuti 2 e 3 a 9.997 e 10.003, 0 perse, `start_ready` p50 161 ms: bigsheep raggiunge i 10.000/s.

### Sintesi

1. **Tutte e 4 le corse sono superate** (media dei minuti 2-10 ≥ 9.900 att./s e 0 accettate perse): OFF r1 **9.999,9**, ON r1 **9.999,9**, ON r2 **10.000,3**, OFF r2 **9.999,1** att./s; minuti 2-10 sotto 9.900: 0 in tutte e quattro (il minuto 1, escluso dalla regola, è a 8.884-8.992 per l'avvio). `verify OK` in tutte.
2. **Validità di ON: rispettata in tutte e due le corse.** `calls_prepare` = `calls_check` = **6.050.000** = `attivita_concluse` dell'ultima verifica; `skipped=0`, `input_invalid=0`, `output_invalid=0`, errori dell'ausiliario **0**.
3. **Lettura a: «l'ausiliario regge 10.000 att./s» (misurato, su bigsheep).** Le due coppie contano (OFF r1 e OFF r2 superate) e ON è superata in tutte e due. (misurata)

### Per corsa (ordine in tempo: OFF r1, ON r1, ON r2, OFF r2)

| Misura | OFF r1 | ON r1 | ON r2 | OFF r2 |
| --- | --- | --- | --- | --- |
| `[stato prima]` | 22:04:56 | 22:30:29 | 22:56:32 | 23:22:04 |
| att./s, intera corsa | 10.000 | 9.987 | 9.990 | 9.999 |
| media minuti 2-10 | 9.999,9 | 9.999,9 | 10.000,3 | 9.999,1 |
| minuti 2-10 sotto 9.900 | 0 | 0 | 0 | 0 |
| intervallo più lungo senza accettazioni (ms) | 164 | 149 | 128 | 170 |
| `start_ready` p50 / p99 (ms) | 162,18 / 242,43 | 156,03 / 205,95 | 155,78 / 214,40 | 165,25 / 251,78 |
| CPU del motore per 1.000 att. (s) | 0,041 | 0,043 | 0,043 | 0,041 |
| CPU dell'ausiliario per 1.000 att. (s) | — | 0,025 | 0,024 | — |
| `aux_rtt` p50 / p99 (ms); RSS dell'ausiliario | — | 0,26 / 2,05; 14.604 kB | 0,26 / 2,04; 14.380 kB | — |
| istruzioni SQL per attività | 0,0293 | 0,0294 | 0,0294 | 0,0293 |
| «database is locked» / `SQLITE_BUSY` nel registro del motore | 0 | 0 | 0 | 0 |
| attività concluse (verifica) | 6.050.000 | 6.050.000 | 6.050.000 | 6.050.010 |
| pavimento prima (p50 / p99 / p99.9, ms) | 4,93 / 5,16 / 14,36 | 6,56 / 7,24 / 11,94 | 4,92 / 5,20 / 14,34 | 4,93 / 5,37 / 9,89 |
| pavimento dopo | 4,91 / 5,22 / 6,07 | 4,92 / 5,20 / 6,00 | 4,94 / 5,44 / 6,29 | 4,92 / 5,20 / 5,87 |
| carico a 1 min prima / dopo | 0,11 / 4,19 | 0,00 / 3,99 | 0,06 / 4,53 | 0,02 / 3,91 |
| container di Marco, CPU % prima | qwen38-27b 1,82; open-webui 0,41; watchamovie 0,12; reranker 0; embedder 0 | 0,07; 0,25; 0,12; 0; 0 | 0,07; 0,25; 0,12; 0; 0 | 0,06; 0,24; 0,13; 0; 0 |
| container di Marco, CPU % dopo | 0,07; open-webui 3,39; 0,12; 0; 0 | 0,08; 0,43; 0,12; reranker 1,74; 0 | 0,07; 0,26; 0,12; 0; embedder 1,82 | 0,08; 0,23; 0,12; 0; 0 |

Nessuna corsa porta `[macchina non ferma]`; attesa di quiete 0-30 s. (misurata)

### Osservazioni (non giudicate)

- **Costo.** ON contro OFF: CPU del motore per 1.000 attività 0,043 contro 0,041 (circa +5 %) più 0,024-0,025 s per 1.000 attività nell'ausiliario; andata e ritorno p50 0,26 ms e p99 2,0 ms; RSS dell'ausiliario circa 14,4-14,6 MB; istruzioni SQL per attività uguali. Questi valori non si confrontano con quelli di minisheep (altra CPU, altro disco).
- **`start_ready` in ON è più basso che in OFF** (p50 156 contro 162-165 ms, p99 206-214 contro 242-252 ms) in entrambe le coppie. La causa è **ipotesi**, non isolata: il motore tiene il tetto di 512 attività in volo per tenant e a 10.000/s la coda di avvio dipende dal ritmo con cui le attività escono dal tetto, che con l'ausiliario cambia (la rilevazione `held_max` per minuto è 0 in tutte e due le corse ON e 276-741 in OFF). Non è un vantaggio dell'ausiliario da dichiarare: nessun criterio lo usa.
- **Cancello del disco.** Il pavimento di questo disco (4,9 ms, stabile) è sopra il limite di 4,0 ms in tutte le misure: il cancello non ha mai potuto passare e ha solo aggiunto 15 minuti di attesa prima di ogni corsa (nessun effetto sul pavimento: 4,92-4,96 ms prima e dopo). Il pavimento misurato all'inizio di ON r1 (6,56 ms) è un picco isolato, subito rientrato (4,92 ms dopo).
- **Scritture sul disco.** Il campionatore mostra una pressione I/O `some` di circa 53-62 % durante le corse, in tutti e quattro i casi (osservata; nessun confronto con minisheep, come da regole).

### Non coperto

Confronto con minisheep; il criterio «database is locked» / `SQLITE_BUSY` (solo riportato: zero in tutte le corse); `PR_SET_DUMPABLE` e i core dump; più ausiliari; il comportamento dopo un crash dell'ausiliario; schemi pesanti; Postgres; il percorso `wasm32-wasip1`.

### Comandi per rifare

Su bigsheep (ogni comando in `bash -c`, shell di login fish), in `~/ttspikebus` (script copiati dal repository, `env.sh` = `env-bigsheep.sh`, `.env` da `./gen-env.sh`, binario `bin/busspike-aux-af23e79`): `./aux10k-chain.sh floor`, `./aux10k-chain.sh smoke`, `./aux10k-chain.sh rows` (staccato, con `taskset -c 5,6,7`; stato in `chain-aux10k.log`).
