# Spike S-1, fase 2c: correttivi comuni e misura finale a 4 CPU (minisheep)

> **Aggiornamento 2d (2026-11-24):** la §2 (scritture parallele) conteneva una causa sbagliata, corretta in `fase2d-risultati.md`, dove il motore tiene più gruppi in volo e le cifre di throughput della 2c cambiano.

Data: 2026-11-23. Segue PIANO.md, sezione «Fase 2c». Per decisione di Marco tutte le corse misurate sono su
minisheep; sul desktop, che Marco usa, ho solo compilato (v. §9 per una deviazione). Numeri grezzi:
`risultati/*-c-ms-r*.txt`; ogni file riporta la riga `cpu:` con i processori usati.

## 1. Sintesi e verdetto

1. **Per la regola di decisione della 2c nessuno dei due bus vince: si torna da Marco.** Con i correttivi e a 4 CPU
   entrambi falliscono la riga del PRD «10.000 attività/s per 10 minuti»: Iggy sostiene 8.205 e 8.966 att./s con
   SQLite (7.125 con Postgres), Valkey 1.245 e 1.252 (1.240 con Postgres). Valkey fallisce anche il tenant lento.
2. **Le code sono rientrate.** A 4 CPU e con l'archivio a segmenti le righe di ritardo a regime basso passano per
   tutti e due: accettazione Iggy p99 18,5-19,2 ms (5 corse su 5, contro 27,8-99,4 ms della 2b), Valkey 8,0-8,7 ms; avvio a
   1.000/s Iggy 11,3-12,0 ms, Valkey 5,0-5,1 ms. La coda larga della 2b veniva dalla macchina libera e dall'archivio,
   non da un difetto di un bus.
3. **Il limite è il throughput a 10.000/s su 4 CPU, e pesa in modo molto diverso:** Iggy arriva all'82-90 % del ritmo,
   Valkey al 12 %. Valkey: ogni gruppo di scrittura da circa 500 eventi costa p50 98 ms nelle corse da 10 minuti, contro 3 ms a basso
   ritmo; non so ancora dire se sia la CPU (un solo thread di Valkey su 4 thread condivisi con motore, worker e SQLite)
   o l'`fsync` per richiesta; non l'ho isolato.
4. **RAM a riposo (SQLite):** Valkey 21,2 + motore 17,1 = 38,3 MB; Iggy 73,5 + 19,1 = 92,6 MB (deroga di Marco).
5. **`kill -9`:** passa per tutti e due, 2 corse ciascuno, con la verifica che ogni payload applicato si decifra.

## 2. I due correttivi

**Correttivo 1, archivio a segmenti.** Un segmento aperto per tenant (`archive/<tenant>/<id>.seg`, `id` = microsecondi
dell'apertura), payload accodati, ognuno sigillato da solo con la stessa busta e la stessa chiave derivata per oggetto
di prima (C-12); un solo `fdatasync` del segmento per gruppo del proiettore e una `fsync` della directory quando il
segmento si apre; il segmento si chiude oltre i 32 MB o dopo 1 s (la chiusura è controllata all'arrivo del gruppo
successivo: un segmento fermo resta aperto finché non arriva altro, ma non si riscrive mai). L'impronta e (segmento,
scostamento, lunghezza) stanno nelle righe che già portavano l'impronta (`execution.input_*`, `activity.out_*`):
nessuna istruzione SQL in più (0,016 istruzioni per attività con Iggy a 10.000/s, come prima; Valkey 0,037 perché con le
corse lente i gruppi sono più piccoli). Le letture delle uscite a monte usano lo scostamento (`pread` del segmento,
poi decifratura): p50 0,02-0,04 ms, p99 0,09-0,23 ms, come con un file per payload. Il motore riceve le posizioni dal
proiettore con un messaggio per gruppo (e da una mappa condivisa finché non lo ha elaborato). Dopo `kill -9`: i
payload scritti ma non ancora referenziati restano come byte in coda al segmento e non servono a nulla, quelli
referenziati sono già sincronizzati (la posizione si scrive dopo `fdatasync`); `busspike verify` rilegge e decifra ogni
payload di uscita applicato.
- Effetto sui numeri (stessa macchina, 10.000 att./s, 60 s, SQLite, **senza** pinning, una corsa per bus): Iggy 7.295 att./s
  (2b: 4.298-4.417), Valkey 1.174 (2b: 1.937-2.014). Quindi il correttivo aiuta Iggy; per Valkey la corsa singola non
  mostra un guadagno (v. §6).

**Correttivo 2, scritture parallele.** Il motore scrive il bus da `N` connessioni; l'esecuzione sceglie la connessione
dall'id (parte casuale dell'UUIDv7), quindi l'ordine degli eventi di un'esecuzione non cambia (gli eventi di una
iterazione dello stesso id vanno nella stessa connessione, e l'iterazione successiva parte dopo che tutte sono state
confermate). **N fissato a 1** per tutte le corse, uguale per i due bus. La corsa di verifica a 4 CPU su minisheep
(`thrmax`, 15 s, SQLite, saturazione) ha mostrato che più connessioni **peggiorano**:

| N | Valkey att./s | Iggy att./s |
| --- | --- | --- |
| 1 | 1.882 | 14.362 |
| 2 | 516 | 3.765 |
| 4 | 768 | 2.701 |
| 8 | 378 | 2.693 |

(Prime corse con N=4 e 8, prima di una correzione mia: Valkey 0 perché il motore si fermava, Iggy 5.826 e 5.605.)
Il motivo più probabile: dividere ogni gruppo in N richieste più piccole moltiplica gli `fdatasync` (2,7 ms ciascuno
su questa macchina) e le fa contendere. **Correzione (fase 2d): questa spiegazione era sbagliata, e il bench del solo bus con 4 connessioni era su minisheep, non sul desktop** (`bench-valkey-ms-r2`: 5.750 msg/s con 1 connessione, 19.667 con 4). Il peggioramento con N > 1 non veniva dal costo degli `fsync` su questo disco ma dal ciclo del motore, che aspettava la scrittura di un solo gruppo alla volta: v. `fase2d-risultati.md`.
Il codice per N > 1 resta (`wconn=`, default 1). Nel farlo ho trovato e corretto un difetto del mio motore: la
cache dei payload si svuotava in base all'istante dell'ultimo evento applicato, e con più connessioni il proiettore può
vedere eventi più nuovi prima di quelli più vecchi della stessa iterazione, quindi un payload non ancora proiettato
spariva dalla cache (panico «payload fuori cache senza posizione»). Ora tutti gli eventi di una iterazione hanno lo
stesso istante e la cache si svuota solo per istanti strettamente più vecchi.

## 3. Macchina, pinning, condizioni

- minisheep: Intel Core Ultra 9 185H, 30 GB, Crucial P3 NVMe, btrfs su LUKS, pavimento `fdatasync` p50 2,7 ms (v. 2b);
  Linux 7.2.4-arch1-2, Docker 29.8.0. Immagini con gli stessi digest della 2b (Iggy `176125a0…`, Valkey `3ab0eca8…`,
  Postgres `eb63372c…`); binario compilato sul desktop senza `target-cpu=native`.
- **4 CPU:** motore, worker, bus e database sulle CPU `1-4` (i due core P `1` e `2` con i loro gemelli SMT: thread 1, 2
  e 3, 4 di `lscpu -e`, 5,1 GHz), con `taskset` per i processi e `cpuset` di compose per i container (variabile
  `SPIKE_CPUSET`, default invariato = nessun limite). Il generatore di carico sulle CPU `8-11` (core P 4 e 5). La stessa
  riga `cpu:` è in testa a ogni file grezzo.
- Prima di ogni corsa `ms-run.sh` attende carico a 1 minuto < 1,5 e `io some avg10` < 1 %. Massimi prima delle 60
  corse: carico 1,48, `io` 0,99 %; nessuna corsa «macchina non ferma»; CPU massima di un container non nostro 0,26 %
  di un core (nessun job dei runner attivo); 12 container non nostri in esecuzione, 4-7 dei quali in riavvio ciclico.
- Iggy e Valkey sono alternati corsa per corsa. Configurazione dei bus come nelle fasi precedenti (Iggy 1 shard, pool
  512 MiB, group commit 500 µs, `Persisted`; Valkey AOF `always`, con trim dopo ogni gruppo applicato).
- **Il p50 assoluto non è giudicato su questo host** (pavimento 2,7 ms): sta nelle tabelle, accanto al p99.

## 4. Risultati, tutte le corse

Legenda: ms; «> 100 ms» = campioni oltre 100 ms. Nelle righe di `thr10k` il «p50/p99» è l'avvio di un'attività con la coda
che si accumula (il ritmo offerto supera la capacità), quindi misura soprattutto l'arretrato.

#### Accettazione durevole, SQLite (lato motore, 200 es./s)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-accept-c-ms-r1 | 11.73 | 18.51 | 44.96 | 0 | 1.39 | 0.57 | 0.02 | 200 |
| iggy | iggy-sqlite-accept-c-ms-r2 | 11.62 | 18.69 | 94.72 | 0 | 1.00 | 0.99 | 0.04 | 200 |
| iggy | iggy-sqlite-accept-c-ms-r3 | 11.54 | 18.73 | 45.25 | 0 | 0.50 | 0.09 | 0.06 | 200 |
| iggy | iggy-sqlite-accept-c-ms-r4 | 11.77 | 18.62 | 42.27 | 0 | 0.73 | 0.92 | 0.00 | 200 |
| iggy | iggy-sqlite-accept-c-ms-r5 | 11.75 | 19.18 | 94.85 | 0 | 0.79 | 0.79 | 0.00 | 200 |
| valkey | valkey-sqlite-accept-c-ms-r1 | 4.47 | 8.44 | 24.08 | 0 | 1.04 | 0.56 | 0.01 | 200 |
| valkey | valkey-sqlite-accept-c-ms-r2 | 4.49 | 8.66 | 24.73 | 0 | 0.79 | 0.68 | 0.00 | 200 |
| valkey | valkey-sqlite-accept-c-ms-r3 | 4.56 | 8.21 | 30.32 | 0 | 0.70 | 0.75 | 0.01 | 200 |
| valkey | valkey-sqlite-accept-c-ms-r4 | 4.55 | 8.01 | 12.57 | 0 | 0.59 | 0.72 | 0.04 | 200 |
| valkey | valkey-sqlite-accept-c-ms-r5 | 4.57 | 8.73 | 37.95 | 0 | 0.91 | 0.79 | 0.01 | 200 |

#### Accettazione durevole, Postgres
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-pg-accept-c-ms-r1 | 11.73 | 18.57 | 33.85 | 0 | 0.93 | 0.72 | 0.00 | 200 |
| iggy | iggy-pg-accept-c-ms-r2 | 11.73 | 18.64 | 50.11 | 0 | 0.89 | 0.84 | 0.04 | 200 |
| valkey | valkey-pg-accept-c-ms-r1 | 4.46 | 7.96 | 31.50 | 0 | 0.66 | 0.61 | 0.01 | 200 |
| valkey | valkey-pg-accept-c-ms-r2 | 4.55 | 7.85 | 46.14 | 0 | 0.93 | 0.91 | 0.14 | 200 |

#### Avvio a 1.000 att./s, SQLite (p50 riportato, non giudicato)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-start1k-c-ms-r1 | 8.41 | 11.99 | 48.70 | 0 | 0.96 | 0.68 | 0.04 | 1000 |
| iggy | iggy-sqlite-start1k-c-ms-r2 | 8.30 | 11.31 | 24.67 | 0 | 0.85 | 0.93 | 0.26 | 998 |
| iggy | iggy-sqlite-start1k-c-ms-r3 | 8.41 | 11.84 | 36.38 | 0 | 0.76 | 0.60 | 0.03 | 1000 |
| iggy | iggy-sqlite-start1k-c-ms-r4 | 8.39 | 11.64 | 27.30 | 0 | 0.84 | 0.95 | 0.00 | 999 |
| iggy | iggy-sqlite-start1k-c-ms-r5 | 8.47 | 11.78 | 45.28 | 0 | 0.74 | 0.68 | 0.02 | 1000 |
| valkey | valkey-sqlite-start1k-c-ms-r1 | 3.19 | 5.08 | 26.86 | 0 | 0.77 | 0.80 | 0.02 | 1000 |
| valkey | valkey-sqlite-start1k-c-ms-r2 | 3.21 | 5.07 | 32.58 | 0 | 1.25 | 0.58 | 0.02 | 1000 |
| valkey | valkey-sqlite-start1k-c-ms-r3 | 3.15 | 5.02 | 30.06 | 0 | 0.72 | 0.98 | 0.14 | 1000 |
| valkey | valkey-sqlite-start1k-c-ms-r4 | 3.17 | 5.05 | 37.18 | 0 | 0.70 | 0.78 | 0.21 | 1000 |
| valkey | valkey-sqlite-start1k-c-ms-r5 | 3.15 | 5.09 | 23.07 | 0 | 0.66 | 0.68 | 0.03 | 1000 |

#### Avvio a 1.000 att./s, Postgres
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-pg-start1k-c-ms-r1 | 8.38 | 11.18 | 32.14 | 0 | 0.86 | 0.69 | 0.04 | 1000 |
| iggy | iggy-pg-start1k-c-ms-r2 | 8.38 | 11.21 | 38.59 | 0 | 1.02 | 0.71 | 0.12 | 1000 |
| valkey | valkey-pg-start1k-c-ms-r1 | 3.13 | 5.07 | 31.97 | 0 | 1.38 | 0.53 | 0.05 | 1000 |
| valkey | valkey-pg-start1k-c-ms-r2 | 3.19 | 5.05 | 76.48 | 0 | 0.84 | 0.83 | 0.05 | 1000 |

#### Join con ramo lento, SQLite (start_join)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-join-c-ms-r1 | 12.78 | 22.29 | 29.73 | 0 | 0.88 | 0.94 | 0.03 | 250 |
| iggy | iggy-sqlite-join-c-ms-r2 | 11.54 | 21.41 | 23.49 | 0 | 0.93 | 0.94 | 0.13 | 250 |
| iggy | iggy-sqlite-join-c-ms-r3 | 17.25 | 43.20 | 50.98 | 0 | 0.65 | 0.98 | 0.13 | 250 |
| valkey | valkey-sqlite-join-c-ms-r1 | 3.71 | 9.11 | 28.66 | 0 | 0.70 | 0.71 | 0.13 | 250 |
| valkey | valkey-sqlite-join-c-ms-r2 | 3.75 | 8.95 | 12.34 | 0 | 1.00 | 0.99 | 0.01 | 250 |
| valkey | valkey-sqlite-join-c-ms-r3 | 3.74 | 9.15 | 25.17 | 0 | 0.84 | 0.94 | 0.04 | 250 |

#### Tenant B da solo (iso0), avvio di B
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-iso0-c-ms-r1 | 8.28 | 11.51 | 89.28 | 0 | 0.73 | 0.75 | 0.05 | 1000 |
| iggy | iggy-sqlite-iso0-c-ms-r2 | 8.45 | 11.58 | 37.47 | 0 | 1.31 | 0.69 | 0.03 | 1000 |
| iggy | iggy-sqlite-iso0-c-ms-r3 | 8.41 | 12.05 | 47.62 | 0 | 0.89 | 0.82 | 0.03 | 1000 |
| valkey | valkey-sqlite-iso0-c-ms-r1 | 3.17 | 5.08 | 25.17 | 0 | 0.87 | 0.93 | 0.14 | 1000 |
| valkey | valkey-sqlite-iso0-c-ms-r2 | 3.18 | 5.17 | 28.98 | 0 | 0.95 | 0.71 | 0.12 | 1000 |
| valkey | valkey-sqlite-iso0-c-ms-r3 | 3.21 | 5.05 | 19.15 | 0 | 1.16 | 0.90 | 0.00 | 1000 |

#### Tenant B con A lento (iso), avvio di B
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-iso-c-ms-r1 | 17.28 | 22.50 | 54.66 | 0 | 1.45 | 0.05 | 0.17 | 1171 |
| iggy | iggy-sqlite-iso-c-ms-r2 | 27.54 | 85.06 | 325.12 | 201 | 0.90 | 0.88 | 0.15 | 1180 |
| iggy | iggy-sqlite-iso-c-ms-r3 | 17.21 | 22.00 | 46.94 | 0 | 0.95 | 0.89 | 0.01 | 1171 |
| valkey | valkey-sqlite-iso-c-ms-r1 | 171.01 | 911.36 | 1628.16 | 19184 | 1.00 | 0.63 | 0.06 | 891 |
| valkey | valkey-sqlite-iso-c-ms-r2 | 193.02 | 1555.45 | 1660.93 | 15483 | 0.93 | 0.96 | 0.03 | 842 |
| valkey | valkey-sqlite-iso-c-ms-r3 | 6.26 | 10.49 | 43.01 | 0 | 0.95 | 0.94 | 0.00 | 1171 |

#### 10.000 att./s per 10 minuti, SQLite (start_ready; att./s sostenute nell ultima colonna)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-thr10k-c-ms-r1 | 9543.68 | 22921.22 | 23117.82 | 3449202 | 0.63 | 0.98 | 0.01 | 8205 |
| iggy | iggy-sqlite-thr10k-c-ms-r2 | 2304.00 | 12656.64 | 14827.52 | 4584989 | 1.47 | 0.77 | 0.04 | 8966 |
| valkey | valkey-sqlite-thr10k-c-ms-r1 | 65437.69 | 133038.08 | 134217.73 | 397897 | 1.42 | 0.21 | 0.04 | 1245 |
| valkey | valkey-sqlite-thr10k-c-ms-r2 | 66486.27 | 132579.33 | 134217.73 | 409054 | 1.44 | 0.45 | 0.00 | 1252 |

#### 10.000 att./s per 10 minuti, Postgres
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-pg-thr10k-c-ms-r1 | 13713.41 | 52428.80 | 52854.78 | 3942268 | 1.48 | 0.81 | 0.00 | 7125 |
| valkey | valkey-pg-thr10k-c-ms-r1 | 60981.25 | 134086.65 | 134217.73 | 410608 | 1.38 | 0.49 | 0.04 | 1240 |

## 5. Verdetto per riga, righe del PRD e righe dei criteri di S-1 in tabelle separate

### 5.1 Righe del PRD

| Riga | Soglia | Valkey | Iggy |
| --- | --- | --- | --- |
| Avvio a 1.000 att./s, SQLite | p99 ≤ 25 ms in ≥ 4 corse su 5 | 5 su 5 (5,0-5,1 ms) **P** | 5 su 5 (11,3-12,0 ms) **P** |
| Avvio a 1.000 att./s, Postgres | 2 su 2 | 2 su 2 (5,05 / 5,07) **P** | 2 su 2 (11,2 / 11,2) **P** |
| Avvio, p50 | ≤ 5 ms, riportato non giudicato | 3,13-3,21 ms | 8,30-8,47 ms |
| 10.000 att./s per 10 minuti | sostenute in tutte le corse | 1.245 / 1.252 (SQLite), 1.240 (Postgres) **F** | 8.205 / 8.966 (SQLite), 7.125 (Postgres) **F** |
| p99 di avvio nella corsa da 10 minuti (riportato) | | 133,0 / 132,6 s; 134,1 s | 22,9 / 12,7 s; 52,4 s |
| Istruzioni SQL per attività | ≤ 5 | 0,037 **P** | 0,016 **P** |
| `kill -9` a metà corsa, 0 accettate perse | 2 corse | 2 su 2 **P** | 2 su 2 **P** |
| RAM a riposo (SQLite) | ≤ 80 MB, riportata | 38,3 MB | 92,6 MB (deroga) |

### 5.2 Righe dei criteri di S-1 della spine

| Riga | Soglia | Valkey | Iggy |
| --- | --- | --- | --- |
| Accettazione durevole, SQLite | p99 ≤ 25 ms in ≥ 4 su 5 | 5 su 5 (8,0-8,7 ms) **P** | 5 su 5 (18,5-19,2 ms) **P** |
| Accettazione durevole, Postgres | 2 su 2 | 2 su 2 (7,96 / 7,85) **P** | 2 su 2 (18,6 / 18,6) **P** |
| Join con ramo lento, SQLite | p99 ≤ 25 ms in ≥ 2 su 3 | 3 su 3 (9,1 / 9,0 / 9,2) **P** | 2 su 3 (22,3 / 21,4 / 43,2) **P** |
| Tenant lento (B con A lento), SQLite | p99 ≤ 25 ms in ≥ 2 su 3 | 1 su 3 (911 / 1.555 / 10,5) **F** | 2 su 3 (22,5 / 85,1 / 22,0) **P** |
| Lettura delle uscite dall'archivio (join) | dentro NFR-2 | p50 0,04, p99 0,23 ms **P** | p50 0,04, p99 0,22 ms **P** |

Il tenant lento (400 es./s con attività da 3 s sul tenant A, più i 1.000 att./s di B) satura Valkey a 4 CPU in due corse
su tre; il caso senza A (`iso0`) passa per tutti e due (Valkey 5,0-5,2 ms, Iggy 11,5-12,1 ms).

### 5.3 Regola di decisione della 2c

«Vince il bus che supera tutte le righe su minisheep. Se le superano entrambi, quello con il p99 di avvio più basso su
minisheep; a parità, quello con meno parti. Se nessuno le supera, si torna da Marco.» Nessuno supera la riga di
10.000 att./s per 10 minuti; Valkey fallisce anche il tenant lento. **Si torna da Marco.**
Il ritardo di coda non è più il problema: dove il carico resta sotto la capacità i p99 stanno entro 25 ms (tranne le
due corse di Iggy sul tenant lento e sul join oltre i 40 ms, ammesse dalla regola «2 su 3»).

## 6. Cosa hanno cambiato i correttivi, a confronto con la 2 e la 2b

| Misura | 2 (desktop, 16 thread, un file per payload) | 2b (minisheep, 22 thread, un file per payload) | 2c (minisheep, 4 CPU, segmenti, N=1) |
| --- | --- | --- | --- |
| Iggy accettazione p99 (SQLite) | 16,9-72,1 ms | 27,8-99,4 ms | 18,5-19,2 ms |
| Valkey accettazione p99 (SQLite) | 1,8-5,4 ms | 11,6-16,4 ms | 8,0-8,7 ms |
| Iggy att./s a 10.000 offerte, archivio acceso | 9.992 (60 s) | 4.298-4.417 (60 s) | 8.205-8.966 (10 min) |
| Valkey att./s a 10.000 offerte, archivio acceso | 8.099-8.574 (60 s) | 1.937-2.014 (60 s) | 1.245-1.252 (10 min) |

- **L'archivio a segmenti** (isolato con la corsa da 60 s senza pinning): Iggy da 4,3k a 7,3k att./s. Per Valkey la
  corsa da 60 s senza pinning dà 1.174 (2b: 1,9-2,0k): non c'è un guadagno e la causa non è l'archivio; non l'ho indagata.
- **Il pinning a 4 CPU** (corse da 10 minuti contro quella da 60 s senza pinning): per Iggy 8,2-9,0k contro 7,3k; per
  Valkey 1,25k contro 1,17k. Le code a basso ritmo sono invece molto più corte con i 4 CPU: la 2b libera aveva il
  tenant lento di Iggy a p99 36-41 ms e la 2c a 22 ms nelle 2 corse su 3 che passano.
- **Le connessioni parallele** non aiutano a 4 CPU su questa macchina (§2).
- Lo scarto fra desktop e minisheep (fino a 6 volte nel pavimento dell'`fdatasync`) rende non confrontabili i valori
  assoluti tra le fasi 2 e 2b-2c; valgono i confronti tra i bus sulla stessa macchina.

## 7. Dettaglio: 10 minuti, kill

- **10 minuti** (ritmo offerto 1.000 es./s = 10.000 att./s, tre corse per bus): sono nelle tabelle di §4. Il ritmo sostenuto
  è `att./s`; il «p50/p99» è l'arretrato accumulato. Nessuna corsa ha perso lavoro accettato, ma l'arretrato cresce
  (la coda del carico generato è aperta, non limitata).
- **`kill -9`** (3 kill per corsa a circa 8, 14 e 21 s, `work_ms=100`, 1.600-1.800 attività in volo al kill; SQLite;
  2 corse per bus): nelle 4 corse `busspike verify` conclude OK dopo lo svuotamento: accettate 7.727 / 7.723 (Iggy) e 7.759 /
  7.767 (Valkey), tutte concluse una volta sola, 0 perse, 0 duplicati di `Done` sul bus, 0 attività con più di un esito;
  **payload di uscita decifrati 77.340 / 77.310 (Iggy) e 77.670 / 77.700 (Valkey), illeggibili 0** (ogni payload applicato
  si rilegge dal suo segmento e si decifra). Ripresa del motore (`READY` dopo il riavvio): Valkey 19-37 ms, Iggy 57-69 ms
  (la prima partenza 103-439 ms). Il tempo fino alla prima attività conclusa non è stato misurato: lo script usa `bc`,
  che su minisheep non c'è.

## 8. Comandi per rifare

Da `spikes/bus/` sul desktop: `cargo build --release`, poi copia di binario e script in `~/ttspikebus` su minisheep e
di `.env` (da `./gen-env.sh`); su minisheep, in `~/ttspikebus` (`env.sh` fissa `SPIKE_BIN`, `SPIKE_DATA`,
`SPIKE_CPUS=1-4`, `SPIKE_CPUSET=1-4`, `SPIKE_LOADCPUS=8-11`):

| Misura | Comando |
| --- | --- |
| Accettazione, avvio, join, tenant lento, RAM, kill | `TAG=-c ./ms-chain.sh <accept\|start1k\|join\|iso0\|iso\|rss\|kill> <sqlite\|pg> <rep da> <rep a> [secs=25]` |
| 10.000/s per 10 minuti | `TAG=-c ./ms-chain.sh thr10k sqlite 1 2 secs=600`, `... pg 1 1 secs=600` |
| Senza pinning (diagnostica) | `./ms-run.sh risultati/<bus>-sqlite-thr10k-unpinned-c-ms-r1.txt ./scen.sh <bus> sqlite thr10k secs=60` (senza `env.sh`) |
| Connessioni parallele | `WCONN=<n> ./scen.sh <bus> sqlite thrmax secs=15` |
| Tabelle | `./ms-tabella.sh <scenario> <db> -c` (da `spikes/bus/`, file in `risultati/`) |

## 9. Deviazioni

- **Corse sul desktop:** prima della decisione «solo minisheep» avevo eseguito sul desktop (pinning 0-3, carico 8-15) la
  prova di fumo con entrambi i bus e una parte della verifica di N (interrotta con `TaskStop`); sono 20 corse brevi
  non conservate, nessuna è una misura. Dopo la decisione il desktop non è più stato caricato; i suoi servizi e i dati
  sono stati rimossi.
- **Le corse da 10 minuti** durano circa 10,2 minuti più l'avvio, cioè sopra al limite di 10 minuti per comando: il
  sistema le ha spostate in background dopo 180 s senza interromperle (come tutte le corse lunghe); non ho mai avviato
  un secondo comando mentre una girava. Tutti i comandi di attesa e delle catene corte sono sotto i 10 minuti.
- **Incidente:** una chiamata di `ms-chain.sh` senza argomenti, mia, ha lasciato `iggy---ms-r.txt` (una corsa vuota
  fra due corse misurate, senza sovrapposizioni; resta nei file, non fa parte delle tabelle).
- **N=1** invece del 4 proposto, per il risultato della verifica (§2).
- **Il tempo di ripresa fino alla prima attività conclusa** non c'è (manca `bc`); c'è il tempo di `READY`.
- **Corse di diagnostica non previste:** `thr10k` senza pinning (due corse da 60 s) e la verifica di N. Non entrano
  nel verdetto.
- Lo stesso `ms-run.sh` ora, dopo 10 minuti senza quiete, fa partire comunque la corsa e la segna «macchina non ferma»
  (non è mai successo).
- Scritture parallele: il difetto di cache (§2) è stato corretto prima delle corse misurate; le corse della 2b non
  usavano il codice nuovo.
- Cleanup: compose giù, directory dati vuote e `.env` cancellato su minisheep; sul desktop compose giù e dati vuoti.
