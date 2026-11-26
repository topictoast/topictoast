# Spike S-1, fase 2b: i ritardi di coda su una macchina ferma (minisheep)

> **Aggiornamento 2c (2026-11-23):** la misura finale a 4 CPU con i due correttivi (archivio a segmenti, scritture parallele) è in `fase2c-risultati.md`.

Data: 2026-11-23. Segue il PIANO.md, sezione «Fase 2b» (corse e regola fissate prima di misurare).
Stessi binari della fase 2 (ricompilati dalla stessa sorgente più il conteggio dei campioni sopra 100 ms),
stesse immagini, stessi scenari di `scen.sh`. Numeri grezzi: `risultati/*-ms-r*.txt` (suffisso `-ms`).
La RAM a riposo di Iggy è accettata come deroga (decisione di Marco) e non si rimisura.

## 1. Sintesi

1. **La coda di Iggy non rientra del tutto neanche a macchina ferma.** Accettazione durevole con SQLite:
   p99 sotto i 25 ms in 0 corse su 5 (27,8 / 35,5 / 99,4 / 86,1 / 91,7 ms), con stalli sopra 100 ms nelle
   corse r3-r5 (59, 12 e 34 campioni). Avvio a 1.000/s invece rientra con SQLite (4 corse su 5) e con
   Postgres (2 su 2), ma al limite (25,5 ms e 24,6 ms nelle corse peggiori).
2. **Su minisheep la classifica di throughput si capovolge.** Con l'archivio acceso Iggy sostiene 4,3-4,4k
   attività/s e Valkey 1,9-2,0k; senza archivio Iggy sostiene 10.000/s (p99 di avvio 29,6 ms in 2 corse su 3, 622 ms
   nella terza) mentre Valkey si ferma a 3,3-5,1k. La macchina ha un `fdatasync` 6 volte più lento (2,7 ms contro
   0,42): Iggy lo ammortizza con il suo group commit, Valkey con una sola richiesta in volo per volta no.
3. **Per la regola della 2b, nessuno dei due vince.** Con la RAM accettata, Iggy non ha la coda sotto i 25 ms
   in accettazione, join e tenant lento, e non ha le 10.000/s con l'archivio a un file per payload; Valkey ha la
   coda (tranne il tenant lento, 1 corsa su 3) ma non le 10.000/s né con né senza archivio su questa macchina.
   Si torna da Marco (v. §5).
4. Il pavimento del disco spiega parte delle differenze: l'accettazione di Valkey costa 1,6-3 volte il pavimento
   (p50 4,3-7,9 ms), quella di Iggy 4-9 volte (p50 11,6-23,3 ms; nel bench del solo bus 2,8-5,3 volte).

## 2. Macchina, pavimento, immagini

| Cosa | minisheep | desktop (fase 2) |
| --- | --- | --- |
| CPU | Intel Core Ultra 9 185H, 22 thread | i5-12600K, 16 thread |
| RAM | 30 GB | 64 GB |
| Disco | SSD Crucial P3 NVMe, btrfs su LUKS (`~/.cache/ttspikebus-data`) | WD_BLACK SN850X, ext4 su LVM |
| Kernel / Docker | Linux 7.2.4-arch1-2, glibc 2.44, Docker 29.8.0 (Compose 5.5.1) | Linux 7.3.5-arch1-1, Docker 29.10.0 |
| Pavimento (4 KB + `fdatasync`) | p50 2,69 / 2,65 / 2,79 ms, p99 3,33 / 3,29 / 4,94 ms (3 misure prima delle corse) | p50 0,42 ms, p99 0,58 ms |

Immagini, digest uguali sui due host (confermato):
- `apache/iggy:0.10.0` `sha256:176125a0580f526b0fee8b98ccf4ed5f77e243066183243d87282abce4851bf3`
- `valkey/valkey:9.1.4` `sha256:3ab0eca823ebbf0fe352476ebac330ac9e5f14b592b9a954eab296d341e81b0e`
- `postgres:18.7` `sha256:eb63372c1e05c755f55fa30cbd07aecc443676fc6c0fa90b9002d01537841e1a`

Configurazione dei bus come nella fase 2 (Iggy 1 shard, pool 512 MiB, group commit 500 µs, `Persisted`;
Valkey AOF `appendfsync always`, riscritture ai valori predefiniti); bus, database, motore, worker e carico
tutti su minisheep. Il binario è compilato sul desktop (`rustc 1.99.1`, senza `target-cpu=native`) e copiato.

## 3. Condizioni delle corse

- Prima di ogni corsa `ms-run.sh` attende carico a 1 minuto sotto 1,5 e `io some avg10` sotto 1 %, poi registra
  carico, pressioni I/O e CPU, i container non nostri e la CPU massima dei loro container; lo ripete dopo.
  Tutte le corse sono partite con carico ≤ 1,49 e `io some avg10` ≤ 0,98 (colonne nelle tabelle).
- Nessun job dei runner era attivo: la CPU massima di un container non nostro è sempre sotto lo 0,25 % di un core
  (prima e dopo ogni corsa). Dodici container non nostri in esecuzione, di cui 4-7 in riavvio ciclico (i runner), con valori praticamente costanti.
- Nessuna corsa è stata scartata dopo averla vista. Una corsa è invalida per un mio errore di procedura
  (due comandi sovrapposti): `bench-valkey-ms-r3`; l'ho sostituita con `bench-valkey-ms-r11` e la lascio nei file.
- Ritmo, campioni e riscaldamento come nella fase 2: accettazione 6.000 campioni; `start1k` 25 s (25.000 campioni);
  `join` 7.500 attività e 1.500 join; `iso0`/`iso` 30.000 avvii di B; `thr10k` 60 s (circa 600.000 attività).

## 4. Risultati, tutte le corse

Legenda: p50, p99 e massimo in ms; «> 100 ms» = campioni oltre 100 ms (stalli); load1 e `io some avg10` prima della
corsa; CPU runner = CPU massima di un container non nostro, in % di un core.

### Bench del solo bus, 1 messaggio da 1 KB, un task (r1 è la corsa doppia: iggy 14,72 / 17,86 / 19,50 ms, valkey 4,33 / 5,45 / 7,32 ms; valkey r3 si è sovrapposta a un'altra corsa per un mio errore, valkey r11 la sostituisce)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | bench-iggy-ms-r2 | 13.97 | 18.16 | 32.99 | 0 | 0.95 | 0.58 | 0.04 |  |
| iggy | bench-iggy-ms-r3 | 7.88 | 10.05 | 25.20 | 0 | 1.46 | 0.98 | 0.08 |  |
| iggy | bench-iggy-ms-r4 | 8.33 | 9.59 | 10.32 | 0 | 0.71 | 0.97 | 0.03 |  |
| iggy | bench-iggy-ms-r5 | 7.63 | 10.16 | 58.34 | 0 | 0.65 | 0.72 | 0.00 |  |
| iggy | bench-iggy-ms-r6 | 7.53 | 9.33 | 13.75 | 0 | 0.73 | 0.71 | 0.19 |  |
| iggy | bench-iggy-ms-r7 | 8.40 | 9.79 | 17.89 | 0 | 1.41 | 0.43 | 0.20 |  |
| iggy | bench-iggy-ms-r8 | 8.16 | 9.94 | 13.21 | 0 | 0.85 | 0.72 | 0.00 |  |
| iggy | bench-iggy-ms-r9 | 11.78 | 17.04 | 19.18 | 0 | 1.14 | 0.61 | 0.17 |  |
| iggy | bench-iggy-ms-r10 | 13.55 | 18.16 | 36.03 | 0 | 1.04 | 0.71 | 0.00 |  |
| valkey | bench-valkey-ms-r2 | 4.92 | 9.32 | 20.99 | 0 | 0.99 | 0.98 | 0.01 |  |
| valkey | bench-valkey-ms-r3 | 19.98 | 29.58 | 31.07 | 0 | 0.93 | 0.80 | 0.16 |  |
| valkey | bench-valkey-ms-r4 | 3.05 | 4.51 | 13.56 | 0 | 0.79 | 0.92 | 0.01 |  |
| valkey | bench-valkey-ms-r5 | 3.11 | 3.84 | 4.17 | 0 | 1.00 | 0.79 | 0.00 |  |
| valkey | bench-valkey-ms-r6 | 2.97 | 3.74 | 5.16 | 0 | 1.18 | 0.83 | 0.00 |  |
| valkey | bench-valkey-ms-r7 | 2.96 | 3.83 | 11.08 | 0 | 1.14 | 0.67 | 0.06 |  |
| valkey | bench-valkey-ms-r8 | 4.71 | 8.39 | 9.43 | 0 | 1.39 | 0.69 | 0.02 |  |
| valkey | bench-valkey-ms-r9 | 2.90 | 4.34 | 20.61 | 0 | 1.42 | 0.05 | 0.00 |  |
| valkey | bench-valkey-ms-r10 | 3.48 | 8.78 | 16.45 | 0 | 1.49 | 0.82 | 0.04 |  |
| valkey | bench-valkey-ms-r11 | 4.96 | 8.95 | 18.75 | 0 | 0.80 | 0.93 | 0.05 |  |

### Accettazione durevole, SQLite (lato motore, 200 es./s)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-accept-ms-r1 | 12.34 | 27.77 | 37.28 | 0 | 0.66 | 0.85 | 0.02 | 200 |
| iggy | iggy-sqlite-accept-ms-r2 | 19.25 | 35.52 | 43.81 | 0 | 1.13 | 0.65 | 0.03 | 200 |
| iggy | iggy-sqlite-accept-ms-r3 | 23.25 | 99.45 | 174.97 | 59 | 0.81 | 0.64 | 0.00 | 200 |
| iggy | iggy-sqlite-accept-ms-r4 | 23.26 | 86.14 | 128.32 | 12 | 1.38 | 0.84 | 0.03 | 200 |
| iggy | iggy-sqlite-accept-ms-r5 | 23.28 | 91.65 | 123.20 | 34 | 1.41 | 0.61 | 0.00 | 200 |
| valkey | valkey-sqlite-accept-ms-r1 | 4.69 | 11.57 | 20.73 | 0 | 0.73 | 0.85 | 0.14 | 200 |
| valkey | valkey-sqlite-accept-ms-r2 | 7.89 | 16.36 | 31.18 | 0 | 1.03 | 0.80 | 0.00 | 200 |
| valkey | valkey-sqlite-accept-ms-r3 | 6.50 | 16.15 | 30.37 | 0 | 0.92 | 0.83 | 0.03 | 200 |
| valkey | valkey-sqlite-accept-ms-r4 | 6.77 | 16.20 | 30.05 | 0 | 0.99 | 0.69 | 0.02 | 200 |
| valkey | valkey-sqlite-accept-ms-r5 | 6.49 | 16.18 | 25.38 | 0 | 1.06 | 0.79 | 0.03 | 200 |

### Accettazione durevole, Postgres
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-pg-accept-ms-r1 | 11.82 | 21.95 | 37.44 | 0 | 1.07 | 0.88 | 0.24 | 200 |
| iggy | iggy-pg-accept-ms-r2 | 11.55 | 18.18 | 31.20 | 0 | 1.05 | 0.94 | 0.00 | 200 |
| valkey | valkey-pg-accept-ms-r1 | 4.41 | 7.74 | 12.32 | 0 | 1.07 | 0.81 | 0.01 | 200 |
| valkey | valkey-pg-accept-ms-r2 | 4.33 | 7.76 | 13.92 | 0 | 0.72 | 0.63 | 0.13 | 200 |

### Avvio a 1.000 att./s, SQLite
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-start1k-ms-r1 | 8.33 | 13.60 | 22.54 | 0 | 1.04 | 0.94 | 0.17 | 1000 |
| iggy | iggy-sqlite-start1k-ms-r2 | 8.57 | 21.54 | 35.07 | 0 | 0.77 | 0.81 | 0.00 | 997 |
| iggy | iggy-sqlite-start1k-ms-r3 | 8.59 | 23.66 | 32.53 | 0 | 1.25 | 0.87 | 0.02 | 1003 |
| iggy | iggy-sqlite-start1k-ms-r4 | 8.43 | 17.95 | 26.85 | 0 | 0.79 | 0.78 | 0.18 | 1003 |
| iggy | iggy-sqlite-start1k-ms-r5 | 8.90 | 25.49 | 32.90 | 0 | 1.00 | 0.09 | 0.03 | 999 |
| valkey | valkey-sqlite-start1k-ms-r1 | 3.35 | 6.54 | 14.69 | 0 | 0.80 | 0.73 | 0.00 | 1000 |
| valkey | valkey-sqlite-start1k-ms-r2 | 3.35 | 6.50 | 16.14 | 0 | 0.87 | 0.90 | 0.00 | 1000 |
| valkey | valkey-sqlite-start1k-ms-r3 | 3.42 | 8.22 | 17.97 | 0 | 1.08 | 0.89 | 0.01 | 998 |
| valkey | valkey-sqlite-start1k-ms-r4 | 3.33 | 6.65 | 10.71 | 0 | 0.53 | 0.84 | 0.00 | 1000 |
| valkey | valkey-sqlite-start1k-ms-r5 | 3.34 | 6.61 | 13.21 | 0 | 0.76 | 0.85 | 0.04 | 1000 |

### Avvio a 1.000 att./s, Postgres
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-pg-start1k-ms-r1 | 8.59 | 21.41 | 29.63 | 0 | 0.94 | 0.93 | 0.06 | 997 |
| iggy | iggy-pg-start1k-ms-r2 | 8.70 | 24.61 | 31.33 | 0 | 1.12 | 0.84 | 0.03 | 998 |
| valkey | valkey-pg-start1k-ms-r1 | 3.46 | 9.97 | 21.76 | 0 | 0.93 | 0.63 | 0.00 | 998 |
| valkey | valkey-pg-start1k-ms-r2 | 3.53 | 11.62 | 20.99 | 0 | 1.43 | 0.07 | 0.00 | 999 |

### Join con ramo lento, SQLite (start_join)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-join-ms-r1 | 19.77 | 90.05 | 110.08 | 9 | 0.97 | 0.79 | 0.04 | 250 |
| iggy | iggy-sqlite-join-ms-r2 | 14.56 | 32.61 | 41.41 | 0 | 0.81 | 0.60 | 0.04 | 250 |
| iggy | iggy-sqlite-join-ms-r3 | 11.07 | 29.74 | 37.15 | 0 | 0.86 | 0.73 | 0.15 | 250 |
| valkey | valkey-sqlite-join-ms-r1 | 4.47 | 10.39 | 24.61 | 0 | 0.85 | 0.57 | 0.04 | 250 |
| valkey | valkey-sqlite-join-ms-r2 | 4.62 | 16.53 | 27.15 | 0 | 0.93 | 0.93 | 0.03 | 250 |
| valkey | valkey-sqlite-join-ms-r3 | 4.37 | 9.62 | 14.82 | 0 | 1.41 | 0.74 | 0.02 | 250 |

### Tenant B da solo (iso0), avvio di B
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-iso0-ms-r1 | 8.30 | 14.86 | 34.37 | 0 | 0.97 | 0.71 | 0.05 | 998 |
| iggy | iggy-sqlite-iso0-ms-r2 | 8.44 | 17.07 | 29.41 | 0 | 1.03 | 0.79 | 0.01 | 998 |
| iggy | iggy-sqlite-iso0-ms-r3 | 8.31 | 13.14 | 24.37 | 0 | 1.08 | 0.87 | 0.06 | 1000 |
| valkey | valkey-sqlite-iso0-ms-r1 | 3.42 | 8.99 | 19.10 | 0 | 1.31 | 0.67 | 0.03 | 1000 |
| valkey | valkey-sqlite-iso0-ms-r2 | 3.37 | 8.25 | 22.30 | 0 | 1.19 | 0.58 | 0.00 | 1001 |
| valkey | valkey-sqlite-iso0-ms-r3 | 3.42 | 8.62 | 16.43 | 0 | 1.20 | 0.72 | 0.01 | 999 |

### Tenant B con A lento (iso), avvio di B
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-iso-ms-r1 | 11.05 | 36.06 | 52.86 | 0 | 1.08 | 0.84 | 0.03 | 1167 |
| iggy | iggy-sqlite-iso-ms-r2 | 14.04 | 41.18 | 55.94 | 0 | 1.41 | 0.89 | 0.04 | 1168 |
| iggy | iggy-sqlite-iso-ms-r3 | 12.37 | 38.17 | 69.31 | 0 | 1.00 | 0.72 | 0.04 | 1170 |
| valkey | valkey-sqlite-iso-ms-r1 | 5.10 | 17.45 | 30.99 | 0 | 1.38 | 0.35 | 0.00 | 1169 |
| valkey | valkey-sqlite-iso-ms-r2 | 5.91 | 25.93 | 47.17 | 0 | 1.40 | 0.17 | 0.03 | 1169 |
| valkey | valkey-sqlite-iso-ms-r3 | 9.38 | 28.91 | 44.80 | 0 | 1.43 | 0.75 | 0.04 | 1169 |

### 10.000 att./s per 60 s, pipeline completa, SQLite (start_ready; att./s sostenute nell ultima colonna)
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-thr10k-ms-r1 | 3844.09 | 10928.13 | 11051.01 | 225082 | 1.39 | 0.15 | 0.06 | 4298 |
| iggy | iggy-sqlite-thr10k-ms-r2 | 3246.08 | 10608.64 | 10747.90 | 226615 | 1.48 | 0.44 | 0.03 | 4391 |
| iggy | iggy-sqlite-thr10k-ms-r3 | 3252.22 | 10747.90 | 10936.32 | 216571 | 1.32 | 0.72 | 0.03 | 4417 |
| valkey | valkey-sqlite-thr10k-ms-r1 | 12181.50 | 19873.79 | 20135.94 | 116212 | 1.44 | 0.26 | 0.01 | 1937 |
| valkey | valkey-sqlite-thr10k-ms-r2 | 7839.74 | 20758.53 | 21053.44 | 120457 | 1.41 | 0.37 | 0.01 | 2008 |
| valkey | valkey-sqlite-thr10k-ms-r3 | 8278.01 | 19972.10 | 20201.47 | 120848 | 1.42 | 0.38 | 0.01 | 2014 |

### 10.000 att./s per 60 s, senza archivio, SQLite
| bus | corsa | p50 ms | p99 ms | max ms | campioni > 100 ms | load1 prima | io some avg10 prima | CPU runner max % | att./s |
|---|---|---|---|---|---|---|---|---|---|
| iggy | iggy-sqlite-thr10k-noarc-ms-r1 | 16.53 | 621.57 | 725.50 | 127006 | 1.38 | 0.52 | 0.10 | 10389 |
| iggy | iggy-sqlite-thr10k-noarc-ms-r2 | 16.46 | 29.58 | 100.09 | 7 | 1.35 | 0.86 | 0.00 | 10004 |
| iggy | iggy-sqlite-thr10k-noarc-ms-r3 | 15.83 | 29.76 | 79.49 | 0 | 1.18 | 0.74 | 0.01 | 9987 |
| valkey | valkey-sqlite-thr10k-noarc-ms-r1 | 5300.22 | 13295.61 | 15564.80 | 197983 | 1.49 | 0.34 | 0.17 | 3300 |
| valkey | valkey-sqlite-thr10k-noarc-ms-r2 | 2703.36 | 10518.53 | 10821.63 | 286514 | 1.49 | 0.43 | 0.00 | 4775 |
| valkey | valkey-sqlite-thr10k-noarc-ms-r3 | 2908.16 | 10272.77 | 10403.84 | 306129 | 1.38 | 0.21 | 0.00 | 5102 |

## 5. Verdetto con la regola della 2b

Regola: una riga di ritardo è superata se il p99 resta entro 25 ms in almeno 4 corse su 5 (SQLite) e in entrambe le
corse con Postgres. Per le righe da 3 corse (join, tenant lento) la regola non dà una soglia: applico «almeno 2 su 3».
Il p50 è riportato, non giudicato (§1, punto 4).

| Riga | Valkey | Iggy |
| --- | --- | --- |
| Bench del solo bus (informativo), p99 ≤ 25 ms | 10 su 10 (senza r3 invalida; con r3, 9 su 11) | 10 su 10 |
| Accettazione, SQLite (≥ 4 su 5) | 5 su 5 (11,6 / 16,4 / 16,2 / 16,2 / 16,2) **P** | 0 su 5 (27,8 / 35,5 / 99,4 / 86,1 / 91,7), stalli > 100 ms in 3 corse **F** |
| Accettazione, Postgres (2 su 2) | 2 su 2 (7,7 / 7,8) **P** | 2 su 2 (22,0 / 18,2) |
| Avvio a 1.000/s, SQLite (≥ 4 su 5) | 5 su 5 (6,5 / 6,5 / 8,2 / 6,7 / 6,6) **P** | 4 su 5 (13,6 / 21,5 / 23,7 / 18,0 / 25,5) **P** al limite |
| Avvio a 1.000/s, Postgres (2 su 2) | 2 su 2 (10,0 / 11,6) **P** | 2 su 2 (21,4 / 24,6) **P** al limite |
| Join con ramo lento (≥ 2 su 3) | 3 su 3 (10,4 / 16,5 / 9,6) **P** | 0 su 3 (90,1 / 32,6 / 29,7) **F** |
| Tenant lento, B (≥ 2 su 3; senza A: Valkey 8-9 ms, Iggy 13-17 ms) | 1 su 3 (17,5 / 25,9 / 28,9) **F** (al limite) | 0 su 3 (36,1 / 41,2 / 38,2) **F** |
| ≥ 10.000 att./s, pipeline completa | 1,9-2,0k **F** | 4,3-4,4k **F** |
| ≥ 10.000 att./s, senza archivio (diagnostica) | 3,3 / 4,8 / 5,1k **F** | 10,4 / 10,0 / 10,0k **P**; p99 avvio 622 / 29,6 / 29,8 ms **F** |

Altri fatti:
- Il costo di accettazione rispetto al pavimento (2,7 ms): Valkey p50 4,3-7,9 ms; Iggy p50 11,6-23,3 ms. Nel bench del solo bus un
  messaggio costa a Valkey p50 2,9-5,0 ms (1,1-1,9 pavimenti) e a Iggy 7,5-14,7 ms (2,8-5,5 pavimenti): Iggy paga più
  barriere di scrittura per richiesta. Sul desktop, 0,47 contro 2,6 ms (1,1 contro 6 pavimenti).
- Valkey con 1,2 KB per evento sostiene meno eventi/s dei suoi `append` in gruppo: su questa macchina un gruppo da circa 500
  `XADD` impiega p50 39 ms (senza archivio) e 55 ms (con archivio); Iggy 12,5 e 27 ms.
- Valkey: 4-8 riscritture AOF nelle corse da 60 s.

### Cosa dice la regola con la RAM accettata

«Se la coda di Iggy rientra e Iggy tiene le 10.000 att./s, Iggy vince; se la coda resta fuori anche a macchina ferma,
è una proprietà del bus e si torna da Marco.» La coda di Iggy resta fuori in accettazione, join e tenant lento, e le 10.000/s
con l'archivio a un file per payload non sono raggiunte da nessuno dei due. **Si torna da Marco.** Le scelte che i numeri
lasciano aperte:
1. **Valkey con un archivio diverso e con la ragione del suo throughput basso su un disco lento chiarita** (il suo limite
   a 3,3-5,1k senza archivio è un fatto nuovo rispetto al desktop, dove senza archivio reggeva 10.000).
2. **Iggy**, che regge il throughput quando l'archivio non pesa e la coda di avvio, ma con accettazione p99 di 86-99 ms
   in 3 corse su 5 a macchina ferma e con stalli sopra 100 ms.
Il pavimento di 2,7 ms rende comunque le due macchine non confrontabili tra loro in assoluto: valgono i confronti tra i bus
sulla stessa macchina.

## 6. Comandi per rifare

Da `spikes/bus/` sul desktop: compilare con `CARGO_TARGET_DIR=… cargo build --release`, copiare binario e script con
`scp` in `~/ttspikebus` (e `.env` generato da `./gen-env.sh`), poi su minisheep:

| Misura | Comando (in `~/ttspikebus`, dopo `. ./env.sh`: `SPIKE_BIN`, `SPIKE_DATA=$HOME/.cache/ttspikebus-data`) |
| --- | --- |
| Pavimento | `$SPIKE_BIN fsync-floor dir=$HOME/.cache` |
| Bench del solo bus | `./ms-run.sh risultati/bench-iggy-ms-r2.txt ./bench-rep.sh 2 iggy` (e `valkey`) |
| Accettazione, avvio, join, tenant lento | `./ms-chain.sh <accept\|start1k\|join\|iso0\|iso> <sqlite\|pg> <rep da> <rep a> [secs=25]` |
| 10.000/s pieni | `./ms-chain.sh thr10k sqlite 1 3` |
| 10.000/s senza archivio | `DIAG_NOARCHIVE=1 TAG=-noarc ./ms-chain.sh thr10k sqlite 1 3` |
| Tabelle | `./ms-tabella.sh <scenario> <db> [-noarc]` (da `spikes/bus/` con i file in `risultati/`) |

`ms-chain.sh` alterna Iggy e Valkey corsa per corsa; `ms-run.sh` attende la macchina ferma e registra lo stato.

## 7. Deviazioni

- **Corsa doppia r1 del bench:** `bench-ms-r1.txt` contiene Iggy e Valkey in un solo comando, con il controllo di
  macchina ferma una volta sola; le altre sono una per bus.
- **`bench-valkey-ms-r3` invalida** (sovrapposta da me a un altro comando); sostituita da `r11`, quindi Valkey ha 10
  corse valide (r1, r2, r4-r11) e Iggy 10 (r1-r10).
- **I comandi lunghi sono stati spostati in background dal sistema** dopo 180 s (la durata di una coppia di corse),
  ogni volta senza interromperli; sono sempre rimasti sotto i 10 minuti e non ho mai avviato un secondo comando
  mentre uno girava, salvo la sovrapposizione di r3 sopra.
- **`ms-run.sh` attende la quiete** fino a 10 minuti; l'attesa è registrata nei file. Le corse sono partite subito o
  dopo qualche decina di secondi (la pressione I/O della corsa precedente).
- **Gli script del desktop** sono stati cambiati solo per leggere `SPIKE_BIN` da una variabile (default invariato);
  `bc` non c'è su minisheep, ho usato `awk` solo negli script nuovi.
- **Non rifatto:** RAM (deroga), bench con più configurazioni di Iggy, archivio a segmenti (non richiesto), `kill`,
  database fermo, payload da 64 KB e 1 MB.
- **Postgres** solo per accettazione e avvio (2 corse per bus, come da piano).
