# Spike S-1, fase 4: risultati

Segue PIANO.md, sezione «Fase 4». Qui c'è solo il **blocco A** (Iggy sotto carico, conservazione, aggiornamento); i blocchi B-E non sono fatti.
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
