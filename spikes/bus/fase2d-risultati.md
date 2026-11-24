# Spike S-1, fase 2d: ultimo giro sulle 10.000 attività/s (minisheep)

Data: 2026-11-24. Segue PIANO.md, sezione «Fase 2d». Solo minisheep, 4 CPU (`1-4`, carico su `8-11`), Iggy e Valkey alternati
corsa per corsa; sul desktop solo compilazione. Numeri grezzi: `risultati/*-d-ms-r*.txt` (codice finale), `*-d1-ms-*` (prima
versione del codice, v. §3), `*-thrmax-*-d-ms-*` (verifica di N), `*-diag-*` (diagnosi di Iggy).

## 1. Sintesi

1. **Per la regola della 2d nessuno dei due bus vince: si torna da Marco con i numeri.** Con il motore corretto **tutti e due reggono 10.000
   attività/s per 10 minuti** (Iggy 9.972-9.993 su tre corse, Valkey 9.982 su una corsa e 6.357 su un'altra, 9.592 con Postgres), ma
   le righe di ritardo a 1.000 attività/s, a regime basso e del tenant lento, che nella 2c passavano a pavimento di 2,7 ms, ora falliscono per tutti
   e due: il pavimento di `fdatasync` è salito a 4,4-4,8 ms (fino a 9,8 ms in qualche corsa) e il codice nuovo non è stato provato con il pavimento di prima.
2. **Causa del peggioramento con N > 1 nella 2c (difetto del nostro motore):** il ciclo del motore scriveva un solo gruppo di eventi alla volta e
   non faceva altro finché non era durevole; il gruppo era limitato a 256 messaggi. Con N connessioni il gruppo si spezzava in N richieste più
   piccole, ognuna con i suoi `fsync`, senza più profondità di pipeline. Misurato: il ciclo era bloccato in attesa del bus per il 96 % del tempo con Valkey e il 71 %
   con Iggy (§2).
3. **Correttivo:** fino a K=4 gruppi in volo, una coda FIFO per connessione (stessa connessione per lo stesso id di esecuzione), risposte e consegne
   solo quando il gruppo è durevole, gruppi fino a 1.024 messaggi. Con questo, `thrmax` sale da 300-7.000 a 15.000-21.000 att./s (Iggy N=1) e da 300-450 a 7.800-16.600 (Valkey N=4-8).
4. **Valkey era al tetto della connessione unica**, come ipotizzato: nelle corse `iso` della 2c che fallivano il ciclo era in attesa del bus per il 98 % e l'82 % del
   tempo (append p50 160 e 103 ms), in quella che passava l'append costava 5,9 ms (§4).
5. **Iggy nei 10 minuti** non perdeva per il disco né per il tempo: la diagnosi a 10 minuti ha una serie piatta (8.0-9.1k att./s al minuto) con append bloccante per il 66 %; con
   il correttivo la serie è piatta a 10.000 (§5).
6. **RAM a riposo (SQLite, riportata):** Valkey 18,9 + 17,6 = 36,5 MB; Iggy 76,1 + 20,1 = 96,2 MB (deroga di Marco).
7. **`kill -9` e payload:** passano per tutti e due con il motore a più gruppi in volo (§8).

## 2. Causa del peggioramento con N > 1 (verifica dell'ipotesi, prima di cambiare il codice)

Ipotesi del coordinatore, letta nel codice: `Engine::run` prende al più 256 messaggi, costruisce un gruppo e aspetta `append` prima di rispondere, consegnare o
leggere altro: un solo gruppo in volo. Con N connessioni `append` spezza il gruppo in N richieste e le aspetta tutte.

Prova prima di cambiare, con le statistiche aggiunte al motore (durata dell'`append` del gruppo, di ogni richiesta, eventi per richiesta), su `thrmax` da 30 s,
N=1, codice della 2c (K=1, 256 messaggi; file `*-thrmax-vecchio-n*-d-ms-r1.txt`):

| Bus | Gruppi al secondo × durata mediana | Tempo del ciclo in attesa del bus | Eventi per richiesta (mediana) |
| --- | --- | --- | --- |
| Valkey | 301 × 95,7 ms | 96 % | 256 (il tetto) |
| Iggy | 1.115 × 19,0 ms | 71 % | 254 |

Con N=2, 4, 8 gli eventi per richiesta scendono a 198/128, 86/72, 38/42 (Valkey/Iggy) e la durata dell'append del gruppo non scende in proporzione (Iggy 19 → 35, 45, 44 ms;
Valkey 96 → 91, 87, 83 ms): si pagano più richieste durevoli senza più profondità. **Ipotesi confermata.**
Nella stessa prova a N crescente (ordine 1, 2, 4, 8 in una sola corsa per cella) Iggy cala (7.006, 3.700, 2.184, 2.695 att./s) e Valkey sale (300, 356, 397, 450):
due bus, due andamenti, nessuno dei due dà un guadagno utile con il vecchio ciclo.

**Correttivo (codice comune ai due bus):** il ciclo non aspetta più la scrittura. Gli eventi di un gruppo vanno a code di scrittura, una per tenant e connessione
(l'esecuzione usa sempre la connessione scelta dal suo id, ogni coda è FIFO, quindi l'ordine degli eventi di un'esecuzione non cambia anche con più gruppi in volo),
e le risposte alle accettazioni e le consegne dei passi pronti partono solo quando tutte le richieste del gruppo sono durevoli (AD-2). Fino a K=4 gruppi in volo, fino a
1.024 messaggi per gruppo. Altre due conseguenze nel codice: la cache dei payload non ancora proiettati si svuota ora con i payload applicati (prima per istante, che con
più gruppi in volo non segue l'ordine sul bus) e il motore stampa una riga al minuto per la diagnosi. Non ho toccato il lato di lettura del proiettore: non è stato il collo di bottiglia
tranne che in una corsa Valkey da 10 minuti (v. §5).

## 3. Verifica di N, con file grezzi

`thrmax` da 30 s, SQLite, 4 CPU, 3 corse per ogni N, bus alternati, ordine di N ruotato tra le ripetizioni (1 2 4 8, 8 4 2 1, 2 8 1 4) perché il disco rallenta nel tempo
(nella 2c l'ordine fisso confondeva N con la deriva). Pavimento di `fdatasync` prima di ogni corsa fra parentesi. Codice con K=4, 1.024 messaggi (senza la regola di raggruppamento descritta sotto).

| N | Iggy att./s (pavimento ms) | mediana | Valkey att./s (pavimento ms) | mediana |
| --- | --- | --- | --- | --- |
| 1 | 15.595 (9,65) / 18.399 (4,43) / 21.445 (4,34) | **15.595** | 2.567 (9,67) / 1.997 (4,92) / 5.094 (4,60) | 1.997 |
| 2 | 6.828 (8,40) / 14.087 (4,71) / 15.840 (4,52) | 6.828 | 3.508 (7,72) / 5.974 (2,63) / 9.189 (4,66) | 3.508 |
| 4 | 14.420 (4,54) / 16.761 (5,08) / 15.508 (4,62) | 14.420 | 11.073 (2,74) / 9.504 (4,13) / 13.690 (4,48) | **9.504** |
| 8 | 18.189 (4,36) / 4.196 (9,65) / 14.290 (4,55) | 4.196 | 7.826 (8,21) / 16.433 (8,54) / 16.609 (4,33) | 7.826 |

**N scelto per bus (mediana più alta): Iggy N=1, Valkey N=4**, fissati prima delle corse misurate (`scen.sh`). Il pavimento che varia da 2,6 a 9,7 ms dentro lo stesso blocco di
corse domina i numeri (le corse con pavimento oltre 8 ms sono quasi tutte le peggiori della cella): le mediane sono quelle che la regola chiede, non una misura pulita di N.

**Regola di raggruppamento (aggiunta dopo la verifica, deviazione):** le corse di controllo della prima versione (`*-d1-ms-*`) avevano il p99 oltre 25 ms (accettazione 83 ms per
tutti e due, p50 25-61 ms a 200 es./s): con K gruppi in volo il ciclo apriva tanti gruppi minuscoli, la coda FIFO della connessione si riempiva e la latenza del gruppo cresceva fino a K volte (append del gruppo 54 ms contro 13 ms
della richiesta). Ho aggiunto: con un gruppo già in volo se ne apre un altro solo con almeno `minb`=64 messaggi in coda, altrimenti si aspetta (raggruppamento naturale). Le corse misurate di §6-§7
sono con questo codice finale; le prime corse da 10 minuti con il codice senza regola sono tenute come `*-d1-ms-*` (Iggy 9.995 / 10.000 / 10.001 att./s, Valkey 9.844 / 10.025 / 9.997 att./s, p99 di avvio 257-864 ms per quasi tutte).
Il codice finale non ha rifatto la verifica di N.

## 4. Valkey era al tetto della connessione unica?

Sì, con questi numeri (file della 2c, `*-iso-c-ms-r*.txt`, motore a un gruppo in volo, N=1):

| Corsa 2c | att./s | Append del gruppo: iterazioni × p50 | Ciclo in attesa del bus |
| --- | --- | --- | --- |
| `iso` r1 (fallita) | 891 | 184 × 160 ms | 98 % |
| `iso` r2 (fallita) | 842 | 240 × 103 ms | 82 % |
| `iso` r3 (passata) | 1.171 | 5.369 × 5,9 ms | 100 % ma con append 17-27 volte più veloce |
| `thr10k` r1 10 minuti | 1.245 | 5.283 × 97,7 ms | 86 % |

Stesso limite in due righe: il ciclo era sempre in attesa del bus, e il throughput dipendeva solo dal costo dell'append, che varia da 6 a 160 ms con lo stato del disco
(a 2b l'append p50 era 70 ms, a 2c 97 ms, a gruppi simili). Il costo per messaggio si spiega con il comportamento di Valkey `appendfsync always`: legge circa 16 KB per giro e fa un
`fsync` per giro, quindi una pipeline lunga costa molti `fsync` e più connessioni li condividono (bench del solo bus su minisheep, stesso topic: 1×256 → 3.584 msg/s, 4×64 → 10.400, 8×32 → 17.100;
Iggy 12.100, 10.900, 11.500).

## 5. Dove perde Iggy nei 10 minuti (diagnosi, fuori dal verdetto)

Riga al minuto stampata dal motore, 10 minuti, SQLite, stesse condizioni (pavimento 4,6-4,7 ms):

| Minuto | Codice 2c (K=1, 256 messaggi): att./s | append p50 del gruppo | Codice nuovo (K=4, 1.024): att./s | append p50 del gruppo |
| --- | --- | --- | --- | --- |
| 1 | 8.175 | 17,7 ms | 8.812 | 63,6 ms |
| 2 | 9.070 | 17,7 | 10.120 | 80,6 |
| 3 | 8.823 | 17,6 | 9.998 | 80,5 |
| 4 | 8.794 | 17,6 | 9.946 | 80,3 |
| 5 | 8.669 | 17,5 | 10.063 | 81,2 |
| 6 | 8.398 | 17,8 | 10.001 | 80,3 |
| 7 | 8.317 | 17,8 | 10.000 | 80,0 |
| 8 | 8.742 | 17,2 | 9.932 | 79,7 |
| 9 | 8.030 | 17,8 | 10.002 | 79,9 |
| 10 | 8.151 | 17,2 | 9.993 | 80,0 |


Serie piatta, nessun degrado nel tempo (pressione I/O costante 20-35 %, ritardo del proiettore p50 163-170 ms a sinistra e 193-232 ms a destra): **la causa è strutturale, non il disco che si consuma.**
Con il codice 2c il ciclo era in attesa dell'append per il 66 % del tempo (22.637 gruppi × 17,6 ms in 600 s) e ogni gruppo era limitato a 256 messaggi:
8.582 att./s medi (le 8.205-8.966 della 2c); con più gruppi in volo la serie è piatta a 10.000. Causa nel codice comune, corretta per tutti e due. Nessuna modifica alla configurazione del bus.

## 6. Righe di ritardo a regime basso e tenant lento (codice finale, una tabella per riga)

Pavimento di `fdatasync` e carico prima di ogni corsa nelle ultime colonne. Corse `iso`: avvio del tenant B con il tenant A lento (p50 B: Iggy 12,9-14,4 ms, Valkey 10,0-10,3 ms; nella 2c 22 e 6 ms).

#### accept sqlite
| bus | corsa | p50 ms | p99 ms | max ms | > 100 ms | pavimento prima ms | load1 prima |
|---|---|---|---|---|---|---|---|
| iggy | r1 | 43.94 | 487.94 | 751.62 | 369 | 4.37 | 1.46 |
| iggy | r2 | 21.04 | 37.53 | 83.58 | 0 | 4.63 | 0.93 |
| iggy | r3 | 26.09 | 169.98 | 379.90 | 90 | 4.69 | 0.97 |
| iggy | r4 | 23.07 | 71.17 | 274.94 | 50 | 4.41 | 1.06 |
| iggy | r5 | 26.83 | 64.96 | 98.24 | 0 | 4.39 | 0.83 |
| valkey | r1 | 7.54 | 23.47 | 32.51 | 0 | 4.44 | 0.97 |
| valkey | r2 | 16.53 | 154.11 | 392.96 | 73 | 4.60 | 0.87 |
| valkey | r3 | 12.14 | 29.31 | 62.43 | 0 | 4.54 | 0.86 |
| valkey | r4 | 12.24 | 27.58 | 45.57 | 0 | 4.68 | 0.93 |
| valkey | r5 | 10.29 | 24.34 | 47.07 | 0 | 4.78 | 0.93 |

#### accept pg
| bus | corsa | p50 ms | p99 ms | max ms | > 100 ms | pavimento prima ms | load1 prima |
|---|---|---|---|---|---|---|---|
| iggy | r1 | 26.48 | 88.13 | 159.23 | 46 | 4.55 | 1.05 |
| iggy | r2 | 22.91 | 56.19 | 71.23 | 0 | 3.64 | 0.99 |
| valkey | r1 | 12.18 | 26.08 | 67.58 | 0 | 2.66 | 0.96 |
| valkey | r2 | 12.01 | 25.73 | 43.49 | 0 | 4.64 | 0.87 |

#### start1k sqlite
| bus | corsa | p50 ms | p99 ms | max ms | > 100 ms | pavimento prima ms | load1 prima |
|---|---|---|---|---|---|---|---|
| iggy | r1 | 14.23 | 32.16 | 66.88 | 0 | 4.59 | 1.02 |
| iggy | r2 | 14.85 | 37.18 | 48.73 | 0 | 4.67 | 0.76 |
| iggy | r3 | 14.94 | 57.18 | 265.47 | 142 | 8.71 | 1.26 |
| iggy | r4 | 8.46 | 18.02 | 41.50 | 0 | 4.54 | 0.76 |
| iggy | r5 | 14.29 | 31.18 | 47.68 | 0 | 4.62 | 0.74 |
| valkey | r1 | 9.34 | 17.52 | 33.70 | 0 | 4.60 | 0.80 |
| valkey | r2 | 21.41 | 80.89 | 379.13 | 155 | 8.56 | 0.60 |
| valkey | r3 | 10.21 | 28.91 | 48.35 | 0 | 4.76 | 1.00 |
| valkey | r4 | 9.49 | 18.19 | 30.61 | 0 | 4.38 | 0.63 |
| valkey | r5 | 9.59 | 18.02 | 51.30 | 0 | 4.69 | 1.34 |

#### start1k pg
| bus | corsa | p50 ms | p99 ms | max ms | > 100 ms | pavimento prima ms | load1 prima |
|---|---|---|---|---|---|---|---|
| iggy | r1 | 25.81 | 82.30 | 451.58 | 227 | 4.23 | 0.95 |
| iggy | r2 | 18.67 | 46.14 | 89.73 | 0 | 4.69 | 1.29 |
| valkey | r1 | 9.28 | 17.25 | 27.26 | 0 | 2.73 | 0.93 |
| valkey | r2 | 7.84 | 17.36 | 31.44 | 0 | 4.55 | 0.98 |

#### join sqlite
| bus | corsa | p50 ms | p99 ms | max ms | > 100 ms | pavimento prima ms | load1 prima |
|---|---|---|---|---|---|---|---|
| iggy | r1 | 18.06 | 49.82 | 150.40 | 4 | 4.28 | 1.47 |
| iggy | r2 | 15.82 | 31.92 | 84.73 | 0 | 4.60 | 1.05 |
| iggy | r3 | 15.38 | 29.39 | 61.89 | 0 | 2.86 | 0.98 |
| valkey | r1 | 5.22 | 14.10 | 21.30 | 0 | 4.53 | 0.96 |
| valkey | r2 | 5.69 | 37.98 | 65.92 | 0 | 9.09 | 0.88 |
| valkey | r3 | 8.99 | 31.23 | 57.31 | 0 | 4.32 | 1.41 |

#### iso sqlite
| bus | corsa | p50 ms | p99 ms | max ms | > 100 ms | pavimento prima ms | load1 prima |
|---|---|---|---|---|---|---|---|
| iggy | r1 | 12.85 | 27.73 | 76.42 | 0 | 4.64 | 0.82 |
| iggy | r2 | 14.25 | 29.23 | 48.64 | 0 | 4.52 | 0.95 |
| iggy | r3 | 14.41 | 29.39 | 47.36 | 0 | 4.63 | 1.18 |
| valkey | r1 | 10.02 | 33.34 | 48.51 | 0 | 3.86 | 1.06 |
| valkey | r2 | 10.06 | 35.90 | 59.58 | 0 | 4.85 | 1.34 |
| valkey | r3 | 10.26 | 36.73 | 101.06 | 18 | 4.27 | 1.45 |



## 7. Corse da 10 minuti (codice finale, ritmo offerto 10.000 att./s, pipeline completa)

Attesa del cancello (carico < 1,5, `io some avg10` < 1 %, `avg60` < 2 %) fino a 15 minuti: nessuna corsa partita «macchina non ferma». Pavimento di `fdatasync` prima della corsa.

| Bus | Database | Corsa | att./s | Avvio p50 / p99 | Pavimento ms | Attesa |
| --- | --- | --- | --- | --- | --- | --- |
| Iggy | SQLite | r1 | 9.983 | 86,5 ms / 2,61 s | 4,99 | 105 s |
| Iggy | SQLite | r2 | 9.972 | 100,0 ms / 16,26 s | 8,17 | 250 s |
| Iggy | Postgres | r1 | 9.993 | 93,0 ms / 2,89 s | 4,61 | 100 s |
| Valkey | SQLite | r1 | **6.357** | 2,63 s / 95,0 s | 4,38 | 180 s |
| Valkey | SQLite | r2 | non valida: il motore non è partito (`PoolTimedOut` aprendo SQLite, pavimento 9,76 ms) | | 9,76 | 205 s |
| Valkey | SQLite | r3 (al posto di r2) | 9.982 | 149,9 ms / 779 ms | 4,69 | 55 s |
| Valkey | Postgres | r1 | **9.592** | 5,17 s / 16,47 s | 4,69 | 175 s |

Il ritmo «sostenuto» lo leggo come almeno il 99 % dell'offerto (9.900 att./s): il generatore offre 10.000 e la finestra di misura taglia gli estremi, per cui nessuna corsa dà
esattamente 10.000. Istruzioni SQL per attività: 0,0155 (Iggy) e 0,0141-0,0155 (Valkey).

Il p99 di avvio nelle corse da 10 minuti non è una latenza: sono i secondi di arretrato quando il ritmo offerto supera per un po' la capacità (Iggy r2: 16 s dopo una fase lenta del disco, pavimento 8,17 ms).

**Valkey r1 (6.357):** per cinque minuti 7.900-11.100 att./s, poi il ritardo del proiettore cresce da 0,9 s a 44 s (p50) e la richiesta di scrittura passa da 26 ms a oltre 200 ms (p50 del gruppo 1 s).
Il proiettore con SQLite, l'archivio a segmenti e l'AOF di Valkey sullo stesso disco non regge nella fase di I/O alto (pressione 25-30 %); l'altra corsa SQLite (r3) con lo stesso codice ha ritardo di proiezione stabile (p50 246-303 ms) a 9.982 att./s.
Non ho isolato la causa; è un'ipotesi coerente con le serie al minuto, non una prova.

## 8. `kill -9` e RAM (codice finale, SQLite)

- **Kill** (3 kill, 1.600-1.800 attività in volo al primo): Iggy accettate 7.681, concluse 7.703, **perse 0**, duplicati di `Done` sul bus 0, payload decifrati 77.030 e illeggibili 0; ripresa del motore 81-96 ms
  (prima partenza 317 ms), prima attività conclusa 1,435 s dal kill (1 s di attesa voluta inclusa, misurato con `awk`); Valkey accettate 7.710, concluse 7.753, **perse 0**, duplicati 0, payload decifrati 77.530,
  illeggibili 0; ripresa 20-37 ms, prima attività conclusa 1,327-1,429 s dal kill. Il motore ora ha più gruppi in volo e l'ordine per esecuzione regge. 2c e 2d: 0 perse.
- **RAM a riposo**, 60 s dopo l'avvio, database vuoto: Valkey 18,9 MB + motore 17,6 MB = 36,5 MB; Iggy 76,1 + 20,1 = 96,2 MB (deroga).

## 9. Verdetto per riga, con la 2c e la 2d separate

Soglie: p99 ≤ 25 ms in ≥ 4 corse su 5 (SQLite) e 2 su 2 (Postgres); ≥ 2 su 3 per join e tenant lento; 10 minuti a ≥ 99 % del ritmo. «2c» = risultato della fase 2c (codice precedente, pavimento 2,7 ms);
«2d» = risultato con il codice finale della 2d (pavimento 4,4-4,8 ms). Come da regola della 2d, dove la corsa di controllo ha il p99 oltre 25 ms la riga si rifà con le corse della 2c sul codice
nuovo e vale il nuovo risultato (colonna 2d).

### 9.1 Righe del PRD

| Riga | Valkey 2c | Valkey 2d | Iggy 2c | Iggy 2d |
| --- | --- | --- | --- | --- |
| Avvio a 1.000 att./s, SQLite | P (5 su 5) | **F** (3 su 5: 17,5 / 80,9 / 28,9 / 18,2 / 18,0) | P (5 su 5) | **F** (1 su 5: 32,2 / 37,2 / 57,2 / 18,0 / 31,2) |
| Avvio a 1.000 att./s, Postgres | P | P (17,3 / 17,4) | P | **F** (82,3 / 46,1) |
| 10.000 att./s per 10 minuti | F (1,24k) | **F** (6.357 e 9.982 su SQLite, 9.592 su Postgres) | F (8,2-9,0k) | **P** (9.983, 9.972, 9.993) |
| Istruzioni SQL per attività | P | P (0,014-0,016) | P | P (0,0155) |
| `kill -9`, 0 perse | P | P | P | P |
| Avvio p50 (riportato, non giudicato; pavimento 4,4-4,8 ms) | 3,1-3,2 ms | 7,8-21,4 ms | 8,3-8,5 ms | 8,5-25,8 ms |

### 9.2 Righe dei criteri di S-1 della spine

| Riga | Valkey 2c | Valkey 2d | Iggy 2c | Iggy 2d |
| --- | --- | --- | --- | --- |
| Accettazione, SQLite | P | **F** (2 su 5: 23,5 / 154 / 29,3 / 27,6 / 24,3) | P | **F** (0 su 5: 488 / 37,5 / 170 / 71 / 65) |
| Accettazione, Postgres | P | **F** (26,1 / 25,7) | P | **F** (88,1 / 56,2) |
| Join con ramo lento | P | **F** (1 su 3: 14,1 / 38,0 / 31,2) | P (2 su 3) | **F** (0 su 3: 49,8 / 31,9 / 29,4) |
| Tenant lento (B con A lento) | F (1 su 3) | **F** (0 su 3: 33,3 / 35,9 / 36,7) | P (2 su 3) | **F** (0 su 3: 27,7 / 29,2 / 29,4) |
| RAM a riposo (riportata) | 38,3 MB | 36,5 MB | 92,6 MB | 96,2 MB (deroga) |

Per capire le righe di controllo: nella 2c le stesse righe passavano a pavimento 2,7 ms (Iggy accettazione p99 18,5-19,2 ms, Valkey 8,0-8,7 ms), nella 2d il pavimento prima di ogni corsa è 4,4-4,8 ms (1,7 volte).
L'append di una richiesta di Iggy di 8 eventi passa da circa 8 ms a 13-14 ms e quello di Valkey da 3 a 3,4-5,3 ms: l'aumento si spiega in gran parte con il pavimento. Aggiungono: Valkey a N=4 (un gruppo
piccolo aspetta la richiesta più lenta tra 4 `fsync`), i gruppi in volo fino a 4 (la regola `minb` limita ma non annulla la coda FIFO). Non ho rifatto le righe di controllo con il codice della 2c all'attuale pavimento,
quindi non separo questi effetti.

### 9.3 Regola di decisione della 2d

«Vince il bus che supera tutte le righe su minisheep.» **Nessuno le supera:** Iggy passa i 10 minuti e fallisce tutte le righe di ritardo della 2d; Valkey fallisce i 10 minuti, le righe di ritardo e il tenant lento.
Si torna da Marco con questi numeri. Quello che i numeri dicono:
- Il tetto dei 10.000 att./s era il motore dello spike (ciclo a un gruppo in volo, non i bus): corretto, Iggy lo regge in tutte e tre le corse, Valkey in una corsa valida su due con SQLite e non con Postgres, ma entrambi a pavimento 4,4-4,8 ms.
- Le righe di ritardo a regime basso dipendono dal pavimento del disco: con 2,7 ms (2c) passavano per tutti e due; con 4,4-4,8 ms (2d) nessuno.
- Con il codice della 2d Valkey regge meglio i ritardi bassi (accettazione p50 7,5-16,5 ms, Iggy 21-44 ms) e Iggy regge meglio i 10 minuti.

## 10. Comandi per rifare

Da `spikes/bus/` sul desktop: `cargo build --release`, copia di binario e script in `~/ttspikebus` su minisheep (e `.env` da `./gen-env.sh`); su minisheep (`env.sh` fissa `SPIKE_BIN`, `SPIKE_DATA`, `SPIKE_CPUS=1-4`, `SPIKE_CPUSET=1-4`, `SPIKE_LOADCPUS=8-11`):

| Misura | Comando |
| --- | --- |
| Verifica di N (3 ripetizioni) | `./ms-sweep.sh 1`, `./ms-sweep.sh 2`, `./ms-sweep.sh 3` (ordine di N ruotato; `WCONN=<n> KGROUPS=<k> MAXB=<m> MINB=<b>` per le prove) |
| Diagnosi Iggy a 10 minuti | `GATE60=1 ./ms-run.sh risultati/iggy-sqlite-thr10k-diag-d-ms-r1.txt ./scen.sh iggy sqlite thr10k secs=600` (con `KGROUPS=1 MAXB=256` per il codice della 2c) |
| Corse misurate | `./ms-final.sh` (10 minuti con `GATE60=1`, poi accettazione, avvio, join, tenant lento, RAM, kill) |
| Bench del solo bus con connessioni e topic diversi | `./diag-bench.sh valkey iggy` |
| Tabelle | `./ms-tabella.sh <scenario> <db> -d` |

## 11. Deviazioni

- **Tempo:** il limite di 3 ore tra diagnosi e codice è stato superato (la verifica di N e le corse di diagnosi sono caduti dentro); la regola `minb` è stata aggiunta dopo le prime corse misurate e
  ha richiesto di rifare tutte le corse (le prime restano come `*-d1-ms-*`).
- **La verifica di N è stata fatta con il codice senza `minb`** (K=4, 1.024 messaggi) e non è stata ripetuta con il codice finale; **K non è stato variato** (K=4), la regola del piano prescrive solo N.
- **Il codice finale per le righe di controllo:** le corse di controllo (1 ciascuna) della prima versione avevano il p99 oltre 25 ms, quindi le righe sono state rifatte con il numero di corse della 2c (5/2 accettazione e avvio, 3 join, 3 tenant lento), come da piano.
- **Valkey SQLite r2 non è valida** (il motore non è partito, `PoolTimedOut` su SQLite con il disco a 9,8 ms di pavimento); resta nei file, e r3 la sostituisce: le corse SQLite valide sono r1 e r3.
- **Il pavimento di `fdatasync` è cambiato** da 2,7 ms (2b, 2c) a 4,4-4,8 ms, con punte a 9,8 ms (misurato prima di ogni corsa e riportato): le fasi non sono confrontabili in assoluto. Nessuna corsa è partita
  «macchina non ferma» (attese di 55-250 s); il carico del disco sotto le nostre corse da 10 minuti è alto (pressione 18-35 %).
- **Comandi lunghi:** il sistema ha spostato in background i comandi sopra i 180 s senza interromperli; non ho mai avviato un secondo comando mentre uno girava.
- **Prove di sviluppo** (non misurate, non nel verdetto): `thrmax` da 15 s su minisheep con più valori di K e N, bench del solo bus con connessioni diverse (`diag-bench.sh`).
- **Il tempo fino alla prima attività conclusa dopo il kill** ora si misura con `awk`; il valore include 1 s di attesa voluta.
- **Cleanup:** compose giù e directory dati vuote su minisheep e sul desktop, `.env` e `out` di minisheep cancellati. Controllo delle password generate nei file committati:
  `grep -rlF -- "<valore>" risultati *.md *.sh *.yaml *.toml src proto | wc -l` per le tre password di `.env`: 0, 0, 0.
