# Spike S-1, fase 2: Iggy contro Valkey, risultati

Data: 2026-11-23. Codice da buttare in `spikes/bus/` (crate `busspike`, fuori dai workspace e dal
gate). Numeri grezzi di ogni corsa in `spikes/bus/risultati/<bus>-<db>-<scenario>-r<n>.txt`.

## 1. Sintesi

1. **Nessuno dei due bus rispetta tutti gli obiettivi di PIANO.md nella pipeline completa.** Per la
   regola di decisione («se nessuno li rispetta, la fase si ferma e si torna da Marco con i numeri»)
   questa fase si ferma qui. Sotto, la lettura dei numeri per aiutare la scelta.
2. **Valkey** (AOF `appendfsync always`): accettazione durevole p50 0,57 ms e p99 ≤ 5,4 ms; avvio di
   un'attività a 1.000/s p50 0,56 ms e p99 ≤ 2,1 ms (SQLite) e ≤ 4,8 ms (Postgres); RAM a riposo
   37,6 MB con il motore. Fallisce **solo** quello che dipende dall'archivio dei payload: 7.600-8.600
   attività/s sostenute (contro 10.000) quando l'archivio (un file cifrato per payload più `syncfs`
   per gruppo) sta sullo stesso disco del bus. Con l'archivio spento (diagnostica) sostiene 10.000
   attività/s per 60 s con avvio p50 1,3 ms e p99 7,4 ms (SQLite) o 10,7 ms (Postgres).
3. **Iggy** (`server-0.10.0`, topic `persisted`): accettazione durevole p50 3,6-4,7 ms ma p99 17-72 ms
   (fallisce in 2 corse su 3 con SQLite, e con Postgres); avvio a 1.000/s p99 22-32 ms con SQLite;
   sostiene 10.000 attività/s ma con avvio p99 51-185 ms; RAM a riposo 91,3 MB con il motore (fallisce
   gli 80 MB di NFR-1 anche con 1 shard e pool da 512 MiB); stalli sporadici di circa 1 s sul log.
   Non è la forma dell'archivio a far fallire Iggy: con l'archivio spento l'avvio resta a p99 68 ms.
4. **Lettura mia, da confermare con Marco:** la differenza tra i due è di natura. Iggy fallisce su
   latenza di coda e RAM, cioè su proprietà del bus. Valkey fallisce su una scelta dell'archivio
   (un file per payload, `syncfs` per gruppo), che AD-17 non impone e che si può cambiare
   (segmenti accodati invece di file singoli, o disco separato dal bus). **Raccomandazione:
   proseguire con Valkey, a condizione di rimisurare NFR-3 con un archivio a segmenti prima della
   fase 3**; se Marco vuole la regola alla lettera, la fase è chiusa senza vincitore.
5. Tutti e quattro i casi (2 bus × 2 database) superano la ripresa dopo `kill -9` (tre kill per
   corsa, 0 accettate perse, 0 eventi `Done` duplicati sul bus, 0 attività con più di un esito
   applicato) e il database fermo (lavoro nuovo rifiutato con `E_PROJECTION_DOWN`, 0 perdite).

## 2. Macchina e versioni

| Cosa | Valore |
| --- | --- |
| CPU | 12th Gen Intel Core i5-12600K, 16 thread |
| RAM | 64 GB (62,5 GiB) |
| Disco dei dati | WD_BLACK SN850X 1 TB NVMe, ext4 su LVM (`/home`), `noatime`, `data=ordered`; costo fisico di una scrittura da 4 KB + `fdatasync`: p50 0,42 ms, p99 0,58 ms (`fsync-floor`) |
| Kernel | Linux 7.3.5-arch1-1 |
| Docker | 29.10.0 |
| Rust | 1.99.1 (`rust-toolchain.toml`) |
| Iggy | immagine `apache/iggy:0.10.0` (`iggy-server 0.10.0`), crate `iggy` 0.12.0 |
| Valkey | `valkey/valkey:9.1.4`, crate `redis` 1.8.1 |
| Postgres | `postgres:18.7` |
| gRPC | tonic 0.14.9 con TLS (rustls, certificato autofirmato generato da `gen-cert`) |
| SQL | sqlx 0.9.1, libsqlite3-sys 0.39.1 nel lockfile (eccezione RUSTSEC-2026-0459) |
| Altre dipendenze fissate | tokio 1.54.1, prost 0.14.4, hdrhistogram 7.6.0, aes-gcm 0.11.1, hkdf 0.13.0, sha2 0.11.1, rcgen 0.14.10, uuid 1.29.1, libc 0.2.191, bytes 1.12.1, rustls 0.23.46 (tutte con almeno due settimane; le dipendenze transitive non sono verificate per data) |

La macchina ospitava altri container fermi (Postgres di sviluppo, ecc.) e il lavoro di altre
sessioni: la variabilità tra corse uguali è reale (vedi Iggy, §5) e non è eliminata.
Licenze: l'elenco dei pacchetti del lockfile è dentro l'allowlist (MIT, Apache-2.0, Unicode-3.0,
ISC, Zlib, BSD, CDLA-Permissive-2.0, BSL-1.0, CC0, 0BSD, o doppie licenze con una scelta ammessa;
`jiff`, `aho-corasick`, `byteorder` sono `Unlicense OR MIT`, si sceglie MIT). L'albero del client
`iggy` è molto più grande di quello di `redis` (reqwest, quinn, tungstenite, ecc.).

## 3. Configurazione e durabilità

- **Iggy:** topic creati con `durability = Persisted` (barriera di memorizzazione stabile prima della
  risposta), 1 partizione per tenant, `message_expiry` mai. Una sola configurazione per tutte le
  misure: 1 shard (`IGGY_SHARDING_CPU_ALLOCATION=1`), pool di memoria 512 MiB (il minimo ammesso),
  `wal_group_commit_delay_micros = 500`, solo TCP, HTTP/QUIC/WebSocket spenti, autenticazione con
  utente root a credenziali generate (`gen-env.sh`, mai `--with-default-root-credentials`).
  Verifica che la durabilità sia reale: lo stesso bench con `Replicated` torna in 0,04 ms (p50),
  contro 2,6 ms con `Persisted`. Le variabili d'ambiente sono state applicate (visibili nel log di
  avvio). La scelta della configurazione viene da un bench del solo bus (§6): 4 shard e 2 shard non
  cambiavano le latenze in modo distinguibile dal rumore, ma costano più RAM; il ritardo di
  group commit di 500 µs è stato scelto perché in un primo giro dava le code più corte, ma i giri successivi non lo hanno confermato in modo netto.
- **Valkey:** `appendonly yes`, `appendfsync always`, `save ""`, `requirepass` generata, AOF con
  riscrittura automatica ai valori predefiniti (64 MB, +100%): le riscritture restano nel p99. Gli
  stream si tagliano con `XTRIM MINID` dopo ogni gruppo applicato dal proiettore (al più ogni 500
  ms), come vuole AD-2; senza il taglio RAM e AOF crescerebbero. Le corse `kill` e `dbdown` non
  tagliano (`trim=0`) per poter contare gli eventi `Done` sul bus.
- **«Accettato»** = `append` tornato dal bus dopo la scrittura durevole, poi risposta (AD-2). Ogni
  decisione (`Ready`, `Done`) è scritta in modo durevole prima di consegnare al worker.
- **Database:** SQLite in WAL con `synchronous=FULL` (la posizione fa scartare il bus, quindi il
  commit deve essere durevole); Postgres 18.7 con le impostazioni predefinite. Il proiettore è
  l'unico scrittore: ogni gruppo è una transazione con righe e posizione applicata per topic. Gruppi
  da almeno 200 ms di eventi (finestra di raccolta, al più 1.000 eventi), così il ritardo di
  proiezione resta sotto 1 s e le istruzioni per attività scendono.
- **Archivio dei payload (AD-3, AD-17):** un file per payload sotto `<tenant>/<byte0 hash>/<hash>`,
  creato solo se assente (`O_EXCL`), cifrato per tenant (§3.1), poi un `syncfs` per gruppo
  PRIMA della transazione del database.
- **Cifratura (C-12, C-13):** chiave principale da file; chiave del tenant da HKDF; chiave dell'oggetto
  da HKDF con `info` = indirizzo completo (topic o hash) più id; AES-256-GCM, nonce a zero (la chiave
  è unica per oggetto). Anche gli eventi sul bus sono cifrati per tenant. Costo misurato
  (`bench-crypto`, un thread): 1 KB: sigillo 1,2 µs, apertura 1,1 µs, SHA-256 0,6 µs; 64 KB: 17,7
  / 16,4 / 29,9 µs; 1 MB: 617 / 274 / 478 µs. È trascurabile fino a 64 KB e pesa circa 1 ms per
  payload da 1 MB (sigillo più SHA-256) sul ciclo del motore.
- **Nomi sul bus:** topic `state-<id interno del tenant>`, mai nomi dal JWT (AD-5). Due tenant fissi.
- **Un solo motore** (AD-2): `flock` esclusivo su `out/engine.lock`, rilasciato da `kill -9`.

## 4. Forma del carico

Un'esecuzione è una **catena** di `n` passi, oppure un **fan-out** di `n` rami seguito da un passo di
**join**. L'attività è identificata da (esecuzione, passo). Eventi sul bus (cifrati): `Accepted`
(porta l'input), `Ready` (decisione, senza payload), `Done` (porta l'uscita). Il motore è un
solo ciclo che raccoglie fino a 256 messaggi, decide, scrive in modo durevole (un gruppo per tenant
in parallelo, al più 8 MB per richiesta), poi risponde e consegna ai worker (8 sessioni gRPC con TLS
in un processo separato, tetto di 512 attività in corso per tenant, consegna a turno tra i tenant).
Il proiettore (stesso processo) legge, decifra, scrive i payload nell'archivio, fa `syncfs`, applica
il gruppo e taglia il bus. Le uscite a monte per una consegna si prendono dalla cache dei payload
**non ancora proiettati** (limitata dal ritardo di proiezione) e, una volta proiettati, **dall'archivio**
(lettura più decifratura, misurata come `archive_read`); mai dalla memoria spacciata per archivio.

Generatore di carico a ritmo fisso (open-loop: la latenza parte dall'istante previsto), in un
processo separato. Payload di input e uscita da 1 KB salvo diversa indicazione; due tenant (A e B).

| Scenario | Forma | Ritmo offerto | Finestra misurata (dopo il riscaldamento) | Campioni |
| --- | --- | --- | --- | --- |
| `accept` | catena n=1 | 200 esecuzioni/s | 30 s (riscaldamento 3 s) | 6.000 accettazioni per corsa |
| `start1k` | catena n=10 | 100 es./s = 1.000 att./s | 40 s (r1) o 25 s (r2, r3), riscaldamento 5 s | 25.000-40.000 avvii |
| `thr10k` | catena n=10, due tenant | 1.000 es./s = 10.000 att./s | 60 s (non 10 min), riscaldamento 5 s | circa 600.000 attività |
| `thrmax` | idem | 2.500 es./s = 25.000 att./s | 30 s | satura: misura il massimo |
| `join` | fan-out n=4 + join, ramo 0 lento 1,5 s, due tenant | 50 es./s | 30 s | 7.500 attività, 1.500 join, 4.500 letture di archivio |
| `size64k` / `size1m` | catena n=5 | 100 / 20 es./s | 30 s | 15.000 / 3.000 attività |
| `iso0` / `iso` | B: catena n=10 a 100 es./s; A (solo `iso`): attività da 3 s a 400 es./s | | 30 s | 30.000 avvii di B |
| `kill` | catena n=10, `work_ms=100` | 300 es./s | 28 s, tre `kill -9` a circa 8, 14 e 21 s | 7.700-11.400 accettate |
| `dbdown` | idem | idem | 28 s, DB giù da circa 8 s per 8-10 s | 5.800-6.800 accettate |

Misure di avvio (`start_ready`): dalla decisione «pronta» del motore (prima della scrittura durevole)
alla ricezione nel worker, **compresa la scrittura durevole della decisione**; `start_sent` è solo
dall'invio alla ricezione (0,06-0,12 ms p50, 1,2 ms p99 a 10.000/s). Gli orologi sono quelli della stessa
macchina (microsecondi Unix tra processi).

## 5. Risultati, una tabella per bus e database

Legenda: **P** supera, **F** fallisce. «r1/r2/r3» = valori delle ripetizioni; una riga passa solo se
passano **tutte** le ripetizioni. Dove c'è una sola corsa lo dico. «Pieno» = pipeline completa con
archivio su file sullo stesso disco; «senza archivio» = diagnostica con `DIAG_NOARCHIVE=1` (nessuna
scrittura né `syncfs` dei payload: isola bus, motore e database).

### 5.1 Valkey 9.1.4 + SQLite

| Misura | Obiettivo | Valore | Esito |
| --- | --- | --- | --- |
| Accettazione durevole (motore) | p50 ≤ 5 ms, p99 ≤ 25 ms | p50 0,57 / 0,57 / 0,57 ms; p99 5,42 / 1,77 / 5,29 ms (RTT cliente p99 5,7 / 2,0 / 5,5) | P |
| Avvio a 1.000 att./s | p50 ≤ 5, p99 ≤ 25 ms | p50 0,57 / 0,56 / 0,56; p99 2,14 / 1,78 / 2,04 ms | P |
| Lettura uscite dall'archivio (join) | dentro NFR-2 | `archive_read` p50 0,03 ms, p99 0,11 ms (n 4.500, cache del sistema calda); avvio del join p50 0,73, p99 1,36 ms; ritardo di proiezione p99 circa 0,2 s | P |
| Attività/s sostenute (60 s) | ≥ 10.000 | pieno: 8.268 / 8.574 / 8.099 (AOF min 4 GB); massimo 6.076. Senza archivio: 10.000 (massimo 11.794) | **F** pieno; P senza archivio |
| Avvio a 10.000 att./s | p99 ≤ 25 ms | pieno: p50 11,7 / 19,6 ms, p99 2.710 / 2.302 ms (coda, non regge). Senza archivio: p50 1,27, p99 7,40 ms | F pieno; P senza archivio |
| Istruzioni SQL per attività | ≤ 5 | 0,016 a 10.000/s (circa 8.000-9.400 istruzioni per 500.000-600.000 attività); 0,17 a 200/s; 0,8-1,0 a 1 MB; righe per attività 2,2 | P |
| RAM a riposo (motore + bus + SQLite), 60 s | ≤ 80 MB | Valkey 21,3 MB + motore 16,3 MB = 37,6 MB | P |
| `kill -9` a metà corsa | ripresa ≤ 30 s, 0 perse | 2 corse × 3 kill: motore di nuovo in ascolto 0,2 s dopo l'avvio (21-32 ms di ripresa, dopo 1 s di attesa voluta tra kill e riavvio), prima attività conclusa dopo ≤ 0,32 s dal riavvio; accettate 7.730, concluse 7.749 (19 con la risposta persa nel kill), perse 0, `Done` duplicati sul bus 0, attività con esiti ≠ 1: 0; in volo al primo kill circa 1.700 attività | P |
| Database fermo | rifiuto con codice, 0 perdite | SQLite bloccato per 10 s con lock esclusivo (emulazione): 2.866 richieste rifiutate con `E_PROJECTION_DOWN`, 0 altri errori; accettate 5.834, tutte concluse dopo lo sblocco | P |
| Attività lenta di un tenant | B entro NFR-2 | B: p50 0,69, p99 24,8 ms (senza A: 0,56 / 1,11 ms) | P (al limite) |
| Payload 64 KB | informativo | avvio p50 0,89, p99 8,97 ms a 500 att./s | P |
| Payload 1 MB | informativo | sostenute 71-80 att./s su 100 offerte; avvio p50 128-323 ms, p99 1,1-2,7 s; RSS motore fino a 2,8 GB (cache) | F |

AOF: nelle corse da 60 s a 10.000/s ci sono state 14 riscritture (una ogni 5 s) con il valore
predefinito; con `auto-aof-rewrite-min-size 4gb` 0 riscritture e lo stesso risultato (8.099/s), quindi
la caduta non viene dalle riscritture.

### 5.2 Valkey 9.1.4 + Postgres 18.7

| Misura | Obiettivo | Valore | Esito |
| --- | --- | --- | --- |
| Accettazione durevole | p50 ≤ 5, p99 ≤ 25 ms | p50 0,57, p99 2,01 ms (1 corsa) | P |
| Avvio a 1.000 att./s | p50 ≤ 5, p99 ≤ 25 ms | p50 0,57, p99 4,82 ms (1 corsa) | P |
| Lettura uscite dall'archivio (join) | dentro NFR-2 | `archive_read` p50 0,03, p99 0,10 ms; avvio del join p99 1,54 ms (1 corsa) | P |
| Attività/s sostenute (60 s) | ≥ 10.000 | pieno 7.584 (1 corsa); senza archivio 9.998 | F pieno; P senza archivio |
| Avvio a 10.000 att./s | p99 ≤ 25 ms | pieno p50 1.846, p99 3.676 ms; senza archivio p50 1,39, p99 10,70 ms | F pieno; P senza archivio |
| Istruzioni SQL per attività | ≤ 5 | 0,016 (a 10.000/s), righe 2,2 e circa 800 byte di WAL per attività | P |
| RAM a riposo | ≤ 80 MB (con SQLite) | non applicabile: NFR-1 si misura con SQLite | n/a |
| `kill -9` a metà corsa | ripresa ≤ 30 s, 0 perse | 3 kill: ripresa del motore 18 ms, prima attività conclusa ≤ 0,32 s dal riavvio; accettate 7.778, concluse 7.778, perse 0, duplicati 0 | P |
| Database fermo | rifiuto con codice, 0 perdite | `docker compose stop postgres` per circa 8 s: 1.910 rifiutate con `E_PROJECTION_DOWN`, 0 altri errori; accettate 6.790, tutte concluse dopo il riavvio | P |
| Attività lenta di un tenant | B entro NFR-2 | B: p50 0,67, p99 3,38 ms (1 corsa) | P |
| Payload 64 KB | informativo | p50 0,88, p99 11,26 ms | P |
| Payload 1 MB | informativo | 100 att./s sostenute, avvio p50 16,4, p99 128 ms | F (p99) |

### 5.3 Iggy server-0.10.0 + SQLite

| Misura | Obiettivo | Valore | Esito |
| --- | --- | --- | --- |
| Accettazione durevole (motore) | p50 ≤ 5, p99 ≤ 25 ms | p50 4,68 / 4,37 / 3,58 ms; p99 72,1 / 54,7 / 16,9 ms | **F** (2 corse su 3 sopra 25 ms) |
| Avvio a 1.000 att./s | p50 ≤ 5, p99 ≤ 25 ms | p50 2,43 / 2,35 / 2,59 ms; p99 30,7 / 22,3 / 31,7 ms | **F** (2 su 3) |
| Lettura uscite dall'archivio (join) | dentro NFR-2 | `archive_read` p50 0,03, p99 0,13 ms; ma avvio del join p50 12,97, p99 87,7 ms (dominato dalla scrittura durevole) | F |
| Attività/s sostenute (60 s) | ≥ 10.000 | pieno 9.992 (massimo 11.036); senza archivio 9.995 (massimo 15.480) | P (al limite del ritmo offerto) |
| Avvio a 10.000 att./s | p99 ≤ 25 ms | pieno: p50 6,41, p99 51,1 ms; senza archivio: p50 8,60, p99 68,4 ms | **F** |
| Istruzioni SQL per attività | ≤ 5 | 0,016 a 10.000/s; 0,16 a 200/s; 1,1 a 1 MB | P |
| RAM a riposo (motore + bus + SQLite), 60 s | ≤ 80 MB | Iggy 72,4 MB + motore 18,9 MB = 91,3 MB (1 shard, pool 512 MiB); con 2 shard e pool 4 GiB il solo bus è 88,6 MB | **F** |
| `kill -9` a metà corsa | ripresa ≤ 30 s, 0 perse | 3 kill: ripresa 34-46 ms, prima attività conclusa ≤ 0,42 s dal riavvio; accettate 7.735, concluse 7.739, perse 0, duplicati 0; in volo al primo kill circa 1.800 attività | P |
| Database fermo | rifiuto con codice, 0 perdite | 2.852 rifiutate con `E_PROJECTION_DOWN`, 0 altri errori; accettate 5.847, tutte concluse | P |
| Attività lenta di un tenant | B entro NFR-2 | B: p50 10,4, p99 79,0 ms (senza A: 5,8 / 45,3 ms: già fuori) | F |
| Payload 64 KB | informativo | avvio p50 22,8, p99 100 ms | F |
| Payload 1 MB | informativo | 41-77 att./s sostenute; un invio da 8 MB impiega ~2 s | F |

### 5.4 Iggy server-0.10.0 + Postgres 18.7

| Misura | Obiettivo | Valore | Esito |
| --- | --- | --- | --- |
| Accettazione durevole | p50 ≤ 5, p99 ≤ 25 ms | p50 4,19, p99 55,7 ms (1 corsa) | F |
| Avvio a 1.000 att./s | p50 ≤ 5, p99 ≤ 25 ms | p50 2,18, p99 8,90 ms (1 corsa; con SQLite, 2 corse su 3 fuori) | P su 1 corsa, instabile |
| Lettura uscite dall'archivio (join) | dentro NFR-2 | `archive_read` p50 0,03, p99 0,11 ms; avvio del join p50 4,46, p99 72,9 ms | F |
| Attività/s sostenute (60 s) | ≥ 10.000 | pieno 9.987; senza archivio 9.694 (97 %) | P (pieno), F (senza archivio, 1 corsa) |
| Avvio a 10.000 att./s | p99 ≤ 25 ms | pieno p50 18,5, p99 184,6 ms; senza archivio p50 13,9, p99 884 ms (stallo da circa 1 s) | **F** |
| Istruzioni SQL per attività | ≤ 5 | 0,016, righe 2,2, circa 800 byte di WAL per attività | P |
| `kill -9` a metà corsa | ripresa ≤ 30 s, 0 perse | 3 kill: ripresa 32-37 ms, prima attività conclusa ≤ 0,42 s; accettate 7.759, concluse 7.762, perse 0, duplicati 0 | P |
| Database fermo | rifiuto con codice, 0 perdite | 2.600 rifiutate con `E_PROJECTION_DOWN`, 0 altri errori; accettate 6.099, tutte concluse | P |
| Attività lenta di un tenant | B entro NFR-2 | B: p50 4,61, p99 15,7 ms (1 corsa; con SQLite 79 ms) | P su 1 corsa, instabile |
| Payload 64 KB | informativo | p50 10,3, p99 79,4 ms | F |
| Payload 1 MB | informativo | 82 att./s; avvio p50 198, p99 1,7 s | F |

Righe senza un'unità di misura proprie ma richieste dalla spine, solo osservate:
- **Ritardo di proiezione** (evento scritto, evento applicato nel database): a 10.000 att./s, p50
  0,13-0,19 s e p99 0,23-0,32 s senza saturazione; con Valkey e archivio sullo stesso disco p99 0,47-0,70 s,
  massimo 0,95 s (al limite di 1 s); con l'archivio su un disco più lento 10-36 s.
- **Cosa fa il motore quando la proiezione è indietro:** serve le uscite dalla cache dei payload non
  proiettati; dopo la proiezione le legge dall'archivio (p99 0,1 ms a 1 KB, ma tutto in cache del sistema:
  non c'è una misura con disco freddo).
- **Righe e WAL per attività** con Postgres: 2,2 righe e 790-823 byte di WAL a 10.000 att./s. Con SQLite
  il WAL non è stato contato.
- **Posizione per partizione, token opaco, ordine tra eventi, retention legata al proiettore**: non
  misurati qui (v. §8).

### 5.5 Il ruolo dell'archivio

Il file per payload più `syncfs` per gruppo è il collo di bottiglia dell'intera pipeline, per entrambi i
bus. Prova in corsa Valkey + SQLite, 10.000 att./s per 20 s: archivio spento 10.001 att./s con avvio
p99 3,3 ms; archivio acceso con `rename` da `tmp` 5.456 att./s; archivio acceso con `O_EXCL` diretto
9.894 att./s e avvio p99 57 ms; sulle corse da 60 s il valore cala a 7.600-8.600. Con l'archivio su un
altro disco (il disco di sistema, più lento: fsync p50 1,2 ms, p99 7 ms) Valkey ha tenuto 9.905 att./s
con scritture sul bus p99 7,5 ms, ma il proiettore è rimasto indietro di 10-36 s: l'archivio a file
singoli non regge 10.000 payload al secondo neanche da solo su quel disco. Ipotesi non verificata: Valkey fa `fsync` nel thread principale e il giornale di ext4 (modo `ordered`) lo fa attendere i dati dei file dell'archivio. Osservato: a parità di archivio Iggy tiene il rate (9.992/s contro 8.268/s) ma con le code lunghe.

## 6. Bench del solo bus (senza motore), 1 KB, dati vuoti a ogni corsa

Comando: `./bench-rep.sh <n>` (azzera i dati, 6 s più 2 s di riscaldamento per configurazione).

| Configurazione | Iggy (r1 / r2) | Valkey (r1 / r2) |
| --- | --- | --- |
| 1 messaggio, 1 task: p50 / p99 | 2,60 / 6,32 ms ; 2,61 / 10,45 ms | 0,47 / 0,66 ms ; 0,46 / 0,63 ms |
| gruppo da 50, 1 task: p50 / p99, msg/s | 3,01 / 12,25 ms, 15.158 ; 2,93 / 11,86 ms, 16.808 | 1,55 / 2,77 ms, 31.117 ; 1,54 / 6,94 ms, 26.850 |
| gruppi da 100, 4 task con connessione propria: msg/s, p99 | 64.683, 24,2 ms ; 63.900, 24,7 ms | 74.750, 19,1 ms ; 74.933, 20,9 ms |

Primi giri con dati sporchi (topic non eliminati) hanno dato p99 di Iggy 8-47 ms e una corsa Valkey
da 500 messaggi andata in timeout: erano artefatti dei dati residui e delle riscritture AOF, tolti
dal confronto. `fsync-floor`: 0,42 ms (p50).

## 7. Comandi per rifare ogni misura

Prerequisiti, da `spikes/bus/`: `./gen-env.sh` (credenziali generate in `.env`), toolchain 1.99.1,
`export CARGO_TARGET_DIR=/home/white_sheep/.cache/cargo-targets/topictoast-spike-bus`,
`cargo build --release`. I dati dei servizi stanno in `/home/white_sheep/.cache/ttspikebus-data`
(`./reset.sh [servizi]` li azzera e riavvia solo il progetto `ttspikebus`).

| Misura | Comando (da `spikes/bus/`) |
| --- | --- |
| Costo fisico di `fsync` | `busspike fsync-floor dir=out` |
| Bench del solo bus | `./bench-rep.sh <etichetta> [iggy] [valkey]` |
| Una corsa completa | `./run1.sh <iggy\|valkey> <sqlite\|pg> <scenario> <rep> [chiave=valore]` (scrive `risultati/…`) |
| Accettazione | `./run1.sh valkey sqlite accept 1` |
| Avvio a 1.000/s | `./run1.sh valkey sqlite start1k 1` (ripetizioni: `secs=25`) |
| Throughput 60 s / massimo | `./run1.sh valkey sqlite thr10k 1`, `thrmax` |
| Senza archivio (diagnostica) | `DIAG_NOARCHIVE=1 ./run1.sh valkey sqlite thr10k 5noarc` |
| AOF senza riscritture | `VALKEY_AOF_MIN=4gb ./run1.sh valkey sqlite thr10k 3tuned` |
| Archivio su altro disco | `SPIKE_ARCHIVE=/var/tmp/ttspike-archive ./run1.sh valkey sqlite thr10k 4arcdisk` |
| Join e archivio | `./run1.sh valkey sqlite join 2` |
| Isolamento tra tenant | `iso0` (B da solo) e `iso` (A lento più B) |
| Payload | `size64k`, `size1m` |
| RAM a riposo | `./run1.sh valkey sqlite rss 1` (attende 60 s, poi `./rss.sh <pid motore>`) |
| Kill del motore | `./run1.sh valkey sqlite kill 1` (tre `kill -9`, poi `busspike verify`) |
| Database fermo | `./run1.sh valkey pg dbdown 1` (Postgres: `docker compose stop`; SQLite: `busspike lock-db`) |
| Costo della cifratura | `busspike bench-crypto dir=out` |
| Postgres | gli scenari con `pg` avviano Postgres 18.7 della compose (`ttspikebus`) |

Il ritmo, i campioni e il riscaldamento di ogni scenario sono nelle righe di `scen.sh`; ogni file in
`risultati/` riporta la riga `load …` con i parametri effettivi.

## 8. Cosa non ho misurato, e perché

- **10 minuti sostenuti** (NFR-3): ogni corsa dura 60 s di finestra, per il limite di 180 s sui comandi.
  Con Valkey le corse da 20 s sembravano passare (9.894/s) e quelle da 60 s no: 10 minuti sono più duri.
- **Postgres in NFR-1** (RAM): la misura è definita con SQLite.
- **Postgres: righe e WAL** misurati; **SQLite: WAL non contato**.
- **Tre ripetizioni** solo per accettazione e avvio a 1.000/s con SQLite. Gli altri valori sono 1 corsa
  (Valkey + SQLite `thr10k`: 4 corse con varianti). Le variazioni tra corse uguali sono grandi per Iggy.
- **Disco freddo**: le letture dall'archivio escono dalla cache del sistema.
- **Archivio a segmenti** (la variante che sblocca Valkey): non provata.
- **Una sola partizione per tenant e una sola connessione Iggy**; con più partizioni per tenant
  Iggy potrebbe migliorare il throughput, non l'accettazione singola.
- **Conservazione legata al proiettore con Iggy**: Iggy non sa tagliare per offset; la retention
  (per dimensione o per età) non sa dell'ultima posizione applicata. Non provata la politica «mai
  scartare posizioni non applicate» (AD-2), che con Iggy richiede un ritardo sul taglio o una verifica
  esterna.
- **Disco pieno**, `SQLITE_BUSY` tra le scritture del motore (segreti, chiavi avvolte) e i gruppi del
  proiettore, RLS Postgres, token opaco di posizione, aggiornamento senza perdite, log dei task,
  timer durevoli: fuori dalla fase 2 (PIANO) o rimandati.
- **Database lento ma attivo** (proiezione oltre 1 s): non provato. Il motore rifiuta lavoro nuovo
  solo con la proiezione ferma per errore, non per ritardo.
- **«Database fermo» con SQLite** è un'emulazione: un altro processo tiene `BEGIN EXCLUSIVE` per 10 s,
  il proiettore fallisce con `database is locked` (busy timeout 300 ms). Non c'è un container da fermare.
- **Il motore esce se `append` fallisce** (`ponytail`): rifiuto con codice per errore del bus e
  ripresa automatica sono della fase 4.
- **`kill -9` del solo motore**: la perdita di corrente dell'host e il `kill` del bus non sono provati.
  La durabilità è verificata solo per costruzione (Iggy `Persisted`, confronto con `Replicated`;
  Valkey `always`) e con la soglia fisica di `fdatasync`.
- **SQL con valori nel testo**: l'adattatore costruisce numeri e letterali esadecimali da tipi interni
  (`ponytail`, nel codice); in produzione servono parametri legati.

## 9. Deviazioni dal piano

- **Toolchain**: 1.99.1 come richiesto (installata con `rustup`).
- **Il progetto parte da `spike/bus`**: il mio worktree era su `main`; ho portato il ramo avanti con
  un fast-forward a `spike/bus` per avere PIANO.md.
- **Dati dei servizi fuori dalla directory del crate** (`/home/white_sheep/.cache/ttspikebus-data`):
  i file di root rompevano la scansione di cargo.
- **Il motore e il proiettore stanno nello stesso processo**; il worker è un processo separato con
  TLS, come da piano. La RAM di NFR-1 somma motore (con SQLite incorporato) e bus; il worker è escluso.
- **Corse da 60 s e non da 10 minuti** per NFR-3.
- **Il tetto di 512 attività per tenant e la consegna a turno** non sono nel piano: servono alla prova
  di isolamento (FR-13).
- **Il join legge dall'archivio solo se il ramo lento ritarda** (ramo 0 da 1,5 s): con rami uguali le
  uscite sono ancora nella cache e `archive_read` risulta 0.
- **Iggy 1 shard e pool 512 MiB**: scelti per stare più vicino a NFR-1, dopo aver visto che 2 e 4 shard
  non miglioravano le latenze in modo distinguibile dal rumore.

## 10. Raccomandazione secondo la regola di decisione

Regola: vince il bus che rispetta tutti gli obiettivi; se li rispettano entrambi, vince quello con il
p99 più basso alla stessa RAM; se nessuno li rispetta, la fase si ferma e si torna da Marco con i numeri.

**Nessuno rispetta tutto.** Quindi la fase si ferma e si torna da Marco con questi numeri. Riassunto per
decidere:

| Obiettivo | Valkey | Iggy |
| --- | --- | --- |
| Accettazione p99 ≤ 25 ms | P | F (r1, r2) |
| Avvio a 1.000/s p99 ≤ 25 ms | P | F (r1, r3 con SQLite) |
| ≥ 10.000 att./s, pipeline completa | F (7,6-8,6k) | P (9,99k) ma p99 di avvio 51-185 ms |
| ≥ 10.000 att./s, senza archivio | P (p99 7-11 ms) | P in rate, p99 68-884 ms |
| SQL ≤ 5 per attività | P | P |
| RAM ≤ 80 MB (con SQLite) | P (37,6 MB) | F (91,3 MB) |
| Ripresa dopo `kill -9`, 0 perse | P | P |
| Database fermo | P | P |
| Attività lenta di un tenant | P | F con SQLite, P su 1 corsa con Postgres |

**Mia lettura:** Valkey è il bus che sta dentro gli obiettivi di latenza e di RAM; Iggy no. Il punto
aperto di Valkey è NFR-3 con l'archivio a file singoli sullo stesso disco: è una scelta dell'archivio,
non del bus, e con l'archivio spento Valkey sostiene i 10.000/s in modo pulito. Propongo a Marco di
**scegliere Valkey a condizione** che prima della fase 3 si provi l'archivio a segmenti accodati (un
file per tenant e per gruppo, indice nel database o ricostruibile) e si rimisuri NFR-3 per 10 minuti
sullo stesso disco. Con Iggy la stessa condizione non basterebbe: restano la RAM (91 MB), le code
(p99 17-72 ms in accettazione) e gli stalli da circa 1 s.
