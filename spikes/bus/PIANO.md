# Spike S-1, il bus: piano delle fasi

Fonte dei criteri: spine, sezione «S-1 — Bus». Approccio a fasi deciso da Marco il 2026-11-23
(memlog dell'architettura, voci 85 e 86).

| Fase | Cosa | Stato |
| --- | --- | --- |
| 1 | Verifica a tavolino della variante (1), worker diretti sul bus | chiusa: la variante esce (`fase1-worker-diretti.md`) |
| 2 | Iggy contro Valkey, con un motore minimo e il collegamento gRPC | chiusa senza vincitore sul desktop (`fase2-risultati.md`) |
| 2b | Ritardi di coda di Iggy e Valkey rimisurati su una macchina ferma (minisheep) | chiusa senza vincitore (`fase2b-risultati.md`) |
| 2c | Correttivi comuni ai due bus (archivio a segmenti, scritture in parallelo), misura finale a 4 CPU su minisheep | chiusa senza vincitore (`fase2c-risultati.md`) |
| 2d | Ultimo giro sulle 10.000 attività/s: scritture parallele del motore corrette, perdita di Iggy nei 10 minuti | chiusa senza vincitore (`fase2d-risultati.md`), Marco sceglie Iggy (2026-11-24) |
| 3 | gRPC contro iroh, solo sul bus vincitore | chiusa (`fase3-risultati.md`): iroh peggiore in consegna e CPU, pari nel ritmo massimo, una corsa da 10 minuti sotto il 99 %: resta gRPC con TLS |
| 4 | Ripresa, guasti, aggiornamento senza perdite, log dei task, timer durevoli, ADR e Update della spine | blocchi A-C fatti (`fase4-risultati.md`, sezioni «Blocco A», «Blocco B» e «Blocco C»); blocco D fermo dopo la riga del costo dei timer: criterio non superato (avvio p99 2,9 volte, 10.000 att./s al 45-54 % con i timer), log dei task non costruito (sezione «Blocco D»); blocco E da fare |

## Fase 2: Iggy contro Valkey

### Cosa si costruisce (codice da buttare, fuori dai workspace e dal gate)

- `spikes/bus/compose.yaml`: Iggy `server-0.10.0`, Valkey 9.1.4, Postgres 18.7, su porte proprie
  dello spike. Autenticazione del bus obbligatoria anche da localhost, credenziali generate.
- Un crate Rust autonomo in `spikes/bus/` (toolchain stabile 1.99.1 fissata nel crate), con:
  - **porta del bus** con due adattatori, Iggy e Valkey: `append` torna solo dopo la scrittura
    durevole (Iggy con fsync, Valkey con `appendfsync always`); lettura da una posizione;
  - **motore minimo:** accetta l'avvio di un'esecuzione (scrittura durevole prima della risposta,
    AD-2), sceglie le attività pronte, le consegna al worker via gRPC (tonic 0.14.9), giudica il
    risultato e scrive l'evento di stato;
  - **proiettore:** legge il bus, applica a gruppi (al massimo 1 s indietro) in SQLite e in Postgres
    con sqlx 0.9.1, con la posizione applicata nella stessa transazione; scrive i payload su disco,
    cifrati per tenant (AES-256-GCM con chiave derivata con HKDF, C-12), con l'impronta nel
    database (AD-3);
  - **worker di prova** in un processo separato, collegato via gRPC con TLS;
  - **carico:** catena di passi e fan-out con join, payload da 1 KB a 1 MB, due tenant.
- Un contatore delle istruzioni SQL per attività dentro l'adattatore del database.

### Cosa si misura, per bus, con SQLite e con Postgres

| Misura | Obiettivo |
| --- | --- |
| Scrittura durevole per accettazione | p50 ≤ 5 ms, p99 ≤ 25 ms (NFR-2) |
| Latenza di avvio di un'attività a 1.000 attività/s | p50 ≤ 5 ms, p99 ≤ 25 ms (NFR-2) |
| Lettura delle uscite a monte dall'archivio, proiezione fino a 1 s indietro | dentro NFR-2 |
| Attività al secondo sostenute | ≥ 10.000 (NFR-3) |
| Istruzioni SQL per attività, scarico compreso | ≤ 5 (NFR-4, SM-2) |
| RAM a riposo di motore, bus e SQLite | ≤ 80 MB (NFR-1) |
| Motore ucciso a metà corsa | ripresa ≤ 30 s, zero attività accettate perse (NFR-6, NFR-15) |
| Database fermo | lavoro nuovo rifiutato con un codice, nessuna perdita (AD-2) |
| Un'attività lenta di un tenant | le attività dell'altro tenant entro NFR-2 (FR-13) |

### Regola di decisione

Vince il bus che rispetta tutti gli obiettivi. Se li rispettano entrambi, vince quello con il p99
più basso alla stessa RAM; a parità, quello con meno parti da gestire. Se nessuno li rispetta, la
fase si ferma e si torna da Marco con i numeri.

### Fuori dalla fase 2

iroh, aggiornamento del bus senza perdite, disco pieno, log dei task, timer durevoli, RLS con
Postgres sotto il proiettore, formato dei messaggi tra versioni: fasi 3 e 4, solo sul vincitore.

### Uscita

`spikes/bus/fase2-risultati.md` con i numeri, la macchina, la forma del carico e i comandi per
rifare ogni corsa.

## Fase 2b: i ritardi di coda su una macchina ferma

Decisione di Marco del 2026-11-23 (memlog dell'architettura, voci 87 e 88): la RAM a riposo di Iggy
(91 MB con motore e SQLite) va bene come deroga per la scelta del bus; i ritardi di coda si
rimisurano su minisheep, una macchina ferma, perché sul desktop in uso la coda di Iggy cambiava
molto tra corse identiche.

### Macchina

minisheep: Intel Core Ultra 9 185H (22 thread), 30 GB, SSD Crucial P3 con btrfs su LUKS. Costo
fisico di una scrittura da 4 KB più `fdatasync`: p50 2,6-2,7 ms, p99 3,3-4,3 ms, contro
0,42 / 0,58 ms del desktop; `chattr +C` sulla cartella non cambia nulla. Bus, database, motore, worker e carico
girano tutti su minisheep, mai attraverso la rete. Sulla macchina girano anche runner GitHub non
nostri, che non si toccano.

### Corse, decise prima di misurare

Stessa configurazione della fase 2 (stessi binari, immagini, `scen.sh`), Iggy e Valkey alternati
corsa per corsa, dati vuoti a ogni corsa del bench.

| Riga | Corse per bus |
| --- | --- |
| Bench del solo bus (`bench-rep.sh`) | 10 |
| Accettazione durevole, SQLite / Postgres | 5 / 2 |
| Avvio a 1.000 attività/s, SQLite / Postgres | 5 / 2 |
| Join con ramo lento, SQLite | 3 |
| Tenant lento (`iso0` e `iso`), SQLite | 3 |
| 10.000 attività/s per 60 s, con e senza archivio, SQLite | 3 + 3 |

Prima e dopo ogni corsa si registrano carico medio, `/proc/pressure/io` e `cpu` e i container
non nostri attivi o in riavvio. Una corsa parte solo con carico medio a 1 minuto sotto 1,5 e
pressione I/O `some avg10` sotto 1 %; altrimenti si attende e si riprova. Nessuna corsa si scarta
dopo averla vista: le corse con un job dei runner attivo restano nella tabella, segnate.

### Cosa conta come superato

- **p99 (giudicato):** un bus supera una riga di ritardo se il p99 resta entro 25 ms in almeno 4
  corse su 5 (SQLite) e in entrambe le corse con Postgres. Il pavimento del disco di minisheep (2,6 ms)
  lascia ancora un margine di 10 volte sotto i 25 ms.
- **p50 (riportato, non giudicato):** sul desktop il pavimento era 6 volte più basso; il p50 si
  riporta accanto al pavimento.
- **Stalli:** per ogni corsa il ritardo massimo e il numero di campioni sopra 100 ms.
- **Throughput:** 10.000 attività/s sostenute come nella fase 2.

### Lettura del risultato

Con la RAM accettata, se la coda di Iggy rientra e Iggy tiene le 10.000 attività/s, Iggy rispetta
gli obiettivi della fase 2 e vince per la regola (Valkey resta sotto le 10.000 attività/s con
l'archivio a un file per payload). Se la coda di Iggy resta fuori anche a macchina ferma, è una
proprietà del bus e si torna da Marco con la scelta tra Valkey con un archivio diverso e un altro passo.

### Uscita

`spikes/bus/fase2b-risultati.md`, numeri grezzi in `spikes/bus/risultati/` con il suffisso `-ms`.

## Fase 2c: correttivi comuni e misura finale a 4 CPU

Decisione di Marco del 2026-11-23 (memlog dell'architettura, voci 89 e 90). La fase 2b ha mostrato
due limiti nel codice dello spike, non nei bus: l'archivio a un file per payload frena entrambi, e il
motore scrive sul bus da una sola connessione (Valkey, nel bench del solo bus, passa da 5.700 a 19.500
messaggi/s con 4 connessioni). Si correggono per entrambi i bus, poi si misura una volta sola, alle
condizioni del PRD.

### Correttivi, uguali per i due bus

- **Archivio a segmenti accodati:** un segmento aperto per tenant, i payload accodati, un solo
  `fdatasync` del segmento per gruppo al posto di un file per payload e `syncfs`; il segmento si
  chiude per dimensione o per tempo (entro 1 s) e non si riscrive mai. Ogni payload resta cifrato da
  solo (C-12: busta versionata, chiave derivata per oggetto), con l'impronta e la posizione (segmento,
  scostamento, lunghezza) nella stessa riga del database che già porta l'impronta (AD-3), senza
  istruzioni SQL in più. Dopo un `kill -9` un segmento troncato si legge fino all'ultimo payload
  completo, e nessun payload già confermato si perde. Solo su disco: RustFS resta fuori dalla 2c.
- **Scritture sul bus in parallelo:** il motore scrive da N connessioni (N fissato prima di misurare,
  uguale per i due bus); ogni esecuzione usa sempre la stessa connessione, scelta dall'id
  dell'esecuzione, così l'ordine degli eventi di un'esecuzione resta quello di oggi.

### Macchina di riferimento

Il PRD misura su 4 vCPU, 16 GB, SSD NVMe locale, Linux. Motore, worker, bus e database girano su 4
thread hardware fissati (2 core P con i loro thread gemelli, con
`taskset` e `cpuset` di compose); il generatore di carico, che sta al posto dei client, gira sugli
altri. Solo minisheep, per decisione di Marco del 2026-11-23: sul desktop, che Marco usa, nessuna
corsa misurata (solo la compilazione). Iggy e Valkey alternati corsa per corsa.

### Corse, per bus (su minisheep)

| Riga | Corse |
| --- | --- |
| Accettazione durevole, SQLite / Postgres | 5 / 2 |
| Avvio a 1.000 attività/s, SQLite / Postgres | 5 / 2 |
| Join con ramo lento, SQLite | 3 |
| Tenant lento (`iso0` e `iso`), SQLite | 3 |
| 10.000 attività/s per 10 minuti, pipeline completa, SQLite / Postgres | 2 / 1 |
| RAM a riposo, SQLite | 1 |
| `kill -9` a metà corsa con l'archivio a segmenti, SQLite (solo minisheep) | 2 |

Prima di ogni corsa, la stessa soglia della 2b (carico medio a 1 minuto sotto 1,5, pressione I/O
`some avg10` sotto 1 %). Se la soglia non si raggiunge entro 10 minuti di attesa, la
corsa parte comunque ed è segnata «macchina non ferma». Nessuna corsa si scarta dopo averla vista.

### Cosa conta come superato

- **Righe del PRD:** avvio a 1.000 attività/s con p99 entro 25 ms in almeno 4 corse su 5 (SQLite) e
  in 2 su 2 (Postgres), p50 entro 5 ms riportato ma non giudicato (minisheep ha il pavimento del
  disco a 2,7 ms); 10.000 attività/s sostenute per 10 minuti in tutte le corse, con il p99 di avvio
  riportato; istruzioni SQL per attività entro 5; ripresa dopo `kill -9` con zero attività accettate
  perse.
- **Righe dei criteri di S-1 della spine:** accettazione durevole con p99 entro 25 ms (stessa soglia
  delle righe da 5 e da 2 corse); join e tenant lento con p99 entro 25 ms in almeno 2 corse su 3.
- **RAM:** riportata; per Iggy vale la deroga di Marco (91 MB).
- Il report tiene separate le righe del PRD e quelle dei criteri della spine.

### Regola di decisione

Vince il bus che supera tutte le righe su minisheep. Se le superano entrambi, quello con il
p99 di avvio più basso su minisheep; a parità, quello con meno parti da gestire. Se nessuno le supera,
si torna da Marco con i numeri.

### Uscita

`spikes/bus/fase2c-risultati.md`, numeri grezzi in `spikes/bus/risultati/` con il suffisso `-c-ms`.

## Fase 2d: ultimo giro sulle 10.000 attività/s

Decisione di Marco del 2026-11-24 (memlog dell'architettura, voci 92 e 93). Nella 2c nessuno dei due
bus regge 10.000 attività/s per 10 minuti (Iggy 8.205-8.966, Valkey 1.240-1.252), e Valkey fallisce
anche il tenant lento. Per Valkey il tetto è il motore dello spike: scrive sul bus da una connessione
sola (circa 5.000 eventi/s), e sulla stessa macchina il bench del solo bus passa da 5.750 a 19.667
messaggi/s con 4 connessioni (2b, `bench-valkey-ms-r2`). Il correttivo delle scritture parallele non
è entrato nella misura della 2c, perché nel motore dello spike N > 1 peggiora entrambi i bus: è un
difetto del nostro codice. È l'ultimo giro di misura: se nessuno passa, la scelta torna a Marco sui
numeri, senza altre fasi.

### Cosa si corregge, nel codice comune ai due bus

- **Scritture parallele del motore:** si trova la causa del peggioramento con N > 1 e la si corregge
  nel codice comune. L'ordine degli eventi di un'esecuzione resta quello di oggi (stessa connessione
  per lo stesso id). Verifica a 4 CPU su minisheep, `thrmax` da 30 s, 3 corse per ogni N tra 1, 2, 4
  e 8, per bus, con i file grezzi conservati. Ogni bus usa la N con la mediana più alta (a parità, la
  più piccola), fissata prima delle corse misurate.
- **Dove Iggy perde nei 10 minuti:** la corsa da 10 minuti scrive una riga al minuto (attività/s,
  p50 della scrittura sul bus, ritardo del proiettore, pressione I/O). Una corsa di diagnosi da 10
  minuti con Iggy, fuori dal verdetto, indica la causa. Se sta nel codice comune (proiettore, SQLite,
  archivio), si corregge per tutti e due. Se sta nel bus o nel disco, si riporta con i numeri; al
  massimo una modifica di configurazione del bus, se la diagnosi la indica, scritta nel report prima
  delle corse misurate.
- **Tempo:** al massimo 3 ore tra diagnosi e codice; poi si misura con quello che c'è e lo si dice.

### Corse misurate, per bus (minisheep, stessi 4 CPU della 2c)

| Riga | Corse |
| --- | --- |
| 10.000 attività/s per 10 minuti, pipeline completa, SQLite / Postgres | 2 / 1 |
| Tenant lento (`iso`), SQLite | 3 |
| Controllo: accettazione, avvio a 1.000 attività/s, join, RAM a riposo, SQLite | 1 ciascuna |
| `kill -9` a metà corsa, SQLite | 1 |

Soglia prima di ogni corsa come nella 2c; prima delle corse da 10 minuti anche pressione I/O `some
avg60` sotto 2 %. Se la soglia non si raggiunge entro 15 minuti, la corsa parte ed è segnata «macchina
non ferma». Iggy e Valkey alternati corsa per corsa; nessuna corsa si scarta dopo averla vista.

### Cosa conta come superato

- Soglie della 2c. Le righe da 10 minuti e del tenant lento si giudicano sulle corse della 2d.
- Le altre righe tengono il risultato della 2c, tranne quando la corsa di controllo della riga ha il
  p99 oltre 25 ms: allora la riga si rifà con le corse della 2c sul codice nuovo, e vale il nuovo
  risultato.
- `kill -9`: zero attività accettate perse sia nella 2c sia nella 2d.

### Regola di decisione

Vince il bus che supera tutte le righe su minisheep. Se le superano entrambi, quello con il p99 di
avvio a 1.000 attività/s più basso su minisheep; a parità, quello con meno parti da gestire. Se nessuno
le supera, si torna da Marco con i numeri per la scelta.

### Uscita

`spikes/bus/fase2d-risultati.md`, numeri grezzi in `spikes/bus/risultati/` con il suffisso `-d-ms`. In
`fase2c-risultati.md` una nota in testa che rimanda alla 2d e la correzione della §2: il bench del
solo bus con 4 connessioni era su minisheep, non sul desktop.

## Fase 3: gRPC contro iroh, su Iggy

Decisione di Marco del 2026-11-24 (memlog dell'architettura, voce 95): il bus è Apache Iggy. La RAM
a riposo (92-96 MB) resta in deroga e il limite del PRD si riallinea con l'Update della spine. La fase
3 confronta i collegamenti (2) e (3) dei criteri di S-1, worker dal motore su gRPC con TLS e worker
dal motore su QUIC con iroh, con lo stesso motore della 2d e Iggy a una connessione.

### Passo 0: impostazione del motore (Iggy, prima del confronto)

Nella 2d i ritardi di Iggy a regime basso sono saliti rispetto alla 2c senza una causa isolata.
Prima del confronto, 3 coppie alternate di accettazione e di avvio a 1.000 attività/s con SQLite:
motore come nella 2c (un gruppo in volo, gruppi fino a 256 messaggi) contro motore della 2d (fino a
4 gruppi in volo, gruppi fino a 1.024, soglia di 64 messaggi). Se la 2c ha il p99 mediano più basso
di almeno il 20 % in entrambe le righe, la fase 3 la usa per le righe a regime basso e la 2d per le
10.000 attività/s; altrimenti la 2d per tutte. Il risultato entra nell'ADR come dato su Iggy e sul
limite dei 25 ms su questo disco.

### Cosa si costruisce

- **Collegamento iroh:** un secondo adattatore del collegamento dei worker, con la stessa forma della
  variante gRPC (una connessione per worker, 4 sessioni bidirezionali, stessi messaggi di consegna e
  di esito); il motore e il worker scelgono l'adattatore dalla configurazione, il resto del codice è
  comune.
- **Versione di iroh:** verificata sul registro dei crate prima di scrivere codice: l'ultima pubblicata
  da almeno due settimane, con licenza ammessa dalla spine; una più recente solo per una vulnerabilità
  pubblicata. Si verifica anche che quella versione funzioni con relay e scoperta esterni spenti e con
  l'identità del nodo remoto fissata. Al massimo 4 ore per l'adattatore; poi si misura con quello che
  c'è e lo si dice.
- **Condizioni di sicurezza, per tutti e due i collegamenti, ognuna con una prova di rifiuto:**
  credenziale del worker legata alla connessione e controllata a ogni collegamento (oggi il saluto del
  worker non ne porta); il worker verifica il server (gRPC: certificato; iroh: identità del nodo
  attesa, dalla configurazione); con iroh relay e scoperta esterni spenti e identità del nodo del
  worker legata alla sua credenziale (C-11). Un collegamento che non le soddisfa esce qualunque siano i
  numeri.

### Corse misurate, per collegamento (minisheep, stessi 4 CPU della 2c e della 2d)

| Riga | Corse |
| --- | --- |
| Avvio a 1.000 attività/s, SQLite | 5 |
| Payload da 64 KB (500 attività/s) e da 1 MB (50 attività/s), SQLite | 3 ciascuno |
| Ritmo massimo (`thrmax`, 30 s), SQLite | 3 |
| 10.000 attività/s per 10 minuti, SQLite | 2 |
| Tenant lento (`iso`), SQLite | 3 |

Le corse sono a coppie, gRPC e iroh una dopo l'altra, con il pavimento del disco registrato prima e
dopo ogni corsa; soglia di partenza come nella 2d. Se con gRPC il ritmo dei payload da 1 MB satura il
bus nella prova di fumo, si dimezza per tutti e due prima di misurare. Il worker registra la propria
CPU come già fa il motore.

### Cosa si confronta e come si decide

- **Giudicato, perché dipende dal collegamento:** il tratto dalla consegna inviata alla consegna
  ricevuta (`start_sent`, p50 e p99), la CPU di motore e worker per 1.000 attività, il ritmo massimo.
  Si confrontano le mediane delle coppie.
- **Riportato, non giudicato:** il tempo di avvio completo (`start_ready`), che comprende la scrittura
  durevole su Iggy, comune ai due e con una variabilità tra corse (18-57 ms di p99 nella 2d) più grande
  della differenza da misurare; le soglie assolute del PRD accanto.
- **10.000 attività/s per 10 minuti:** almeno il 99 % del ritmo offerto in entrambe le corse; un
  collegamento che non le tiene esce.
- **Regola:** iroh sostituisce gRPC solo se migliora di almeno il 20 % il p99 di `start_sent` o la CPU
  per 1.000 attività, non peggiora di oltre il 20 % nessun'altra riga giudicata e tiene le 10.000
  attività/s. Altrimenti resta gRPC, che ha meno parti e meno codice nuovo (scelta di Marco per la
  parità).

### Fuori dalla fase 3

La variante (1) è uscita nella fase 1. Ripresa, guasti, aggiornamento senza perdite, log dei task e
timer durevoli sono della fase 4, sul collegamento che vince.

### Uscita

`spikes/bus/fase3-risultati.md`, numeri grezzi in `spikes/bus/risultati/` con il suffisso `-3-ms`.
Ogni causa citata nel report è segnata «misurata» (con il file grezzo) o «ipotesi».

## Fase 4: ripresa, guasti, aggiornamento, timer, log dei task, su Iggy con gRPC

Base: Iggy `server-0.10.0`, collegamento dei worker gRPC con TLS (fase 3), motore della 2d, SQLite;
Postgres solo dove la riga lo dice. Minisheep, stessi 4 CPU delle fasi 2c-3. Qui non si confrontano
varianti: ogni criterio è superato o no.

### Copertura dei criteri di S-1

Ogni criterio della spine (sezione S-1, compreso «da fissare e provare nell'ADR») e ogni punto «non
provato» delle fasi 2-3 ha un esito: già misurato, corsa della fase 4, deciso a tavolino nell'ADR,
dichiarato non fatto.

| Criterio | Esito |
| --- | --- |
| Scrittura durevole per accettazione (NFR-2), modalità di scrittura durevole (M-18) | misurato nelle fasi 2-3: p99 circa 19 ms, p50 circa 11,5 ms contro 5 ms su questo disco; nell'ADR |
| 10.000 attività/s (NFR-3) | misurato nella 2d e nella 3, con episodi di stallo di Iggy non spiegati: blocco A |
| Istruzioni SQL per attività (NFR-4), proiezione idempotente | misurato nella fase 2; NFR-4 si ricontrolla con i timer e il log dei task attivi (blocco D) |
| RAM (NFR-1) | misurato (Iggy 92-96 MB), in deroga per decisione di Marco; il PRD si riallinea |
| Lettura dall'archivio con proiezione indietro fino a 1 s, cifratura per tenant (C-12) | misurato nelle fasi 2 e 2c |
| Attività lenta di un tenant (FR-13) | misurato nelle fasi 2-3 (`iso`) |
| Autenticazione obbligatoria del bus, credenziali generate | fatto nella fase 2 (compose dello spike) |
| Credenziale del worker, verifica del server | misurato nella fase 3, con prove di rifiuto |
| Conservazione legata alla posizione del proiettore | blocco A |
| Aggiornamento senza perdite (FR-51, M-17) | blocco A |
| Database lento ma attivo, proiezione oltre 1 s | blocco A (se gli stalli sono ritardo di proiezione) e una corsa con ritardo iniettato |
| Crash del motore, database fermo (NFR-6) | misurato nelle fasi 2 e 2c; il crash del motore si rifà come controllo nel blocco B |
| Crash di Iggy e del worker, disco pieno (NFR-6), ripresa entro 30 s con i worker che si ricollegano (NFR-15) | blocco B |
| Spegnimento improvviso dell'host (NFR-6) | non provabile su minisheep (ospita i runner): argomentato nell'ADR (risposta solo dopo fsync di Iggy, SQLite `synchronous=FULL`) e dichiarato non misurato |
| Timer durevoli, con retry e attesa attraverso un crash | blocco C (fatto); costo dei timer sul percorso caldo nel blocco D |
| Log dei task (FR-59), creazione solo se assente su disco e su RustFS | blocco D |
| Chiamata all'ausiliario sul percorso caldo (AD-6), `if`, jq e controllo di input e output (FR-53) | blocco E |
| `SQLITE_BUSY` tra scritture del motore e gruppi del proiettore | blocco E |
| Annullamento (FR-10) e arresto per timeout dell'esecuzione | blocco E |
| Credenziale del bus per tenant in sola lettura (pipeline stream) | blocco E, con prova di rifiuto |
| Posizione per partizione con la generazione del bus, token opaco, ordine tra eventi (AD-18, C-8) | a tavolino nell'ADR; la generazione si prova nel blocco A |
| Controlli di AD-18 sullo stato più vecchio della rilettura | a tavolino nell'ADR |
| Nessun effetto esterno prima della scrittura durevole | lettura del codice più i duplicati a zero dopo i kill; nell'ADR |
| Spazio per tenant sul bus, motivo di attesa dovuto alle partizioni (T-66) | a tavolino nell'ADR (oggi un topic e una partizione per tenant) |
| Contenuto di una consegna, worker che scrive su uno stream (C-2, AD-20) | a tavolino nell'ADR |

### Ordine

Prima ciò che può riaprire la scelta del bus (blocco A), poi il resto. Un blocco alla volta, con il
rapporto dopo ogni blocco.

### Blocco A: Iggy sotto carico, conservazione, aggiornamento

- **Stalli a 10.000/s** (fase 3: l'append passa da circa 80 a 400-470 ms per 1-2 minuti, con tutti e
  due i collegamenti). Per eliminazione, 10 minuti ciascuna: Iggy da solo allo stesso ritmo di byte
  (`bench-append`), poi il motore senza archivio (`DIAG_NOARCHIVE`), poi completo. Si leggono, in sola
  lettura, la pressione I/O e le statistiche I/O per cgroup, per vedere se la fonte è un altro
  container. Al più due cambi di configurazione di Iggy. Al massimo 3 ore. Ogni causa segnata
  «misurata» o «ipotesi».
- **Conservazione:** prima si verifica quali primitive di cancellazione ha Iggy 0.10.0 (la fase 2 ha
  visto che non taglia per offset). La riga prova che si cancellano solo segmenti sotto la posizione
  applicata più bassa e mai una posizione non applicata.
- **Aggiornamento senza perdite:** caso reale, Iggy 0.9.0 verso 0.10.0, che non legge i dati della
  0.9. Prima si verifica quale versione del crate `iggy` parla con il server 0.9.0. Se la 0.12.0 non ci
  parla, si sceglie tra due build del motore e un cambio verso un bus nuovo e vuoto con la stessa
  versione, e lo si dice. Procedura da provare, sotto carico a 1.000 attività/s con attività lunghe in
  volo:
  1. il motore smette di accettare (rifiuto con codice) e aspetta che la posizione applicata sia uguale
     alla testa su ogni topic;
  2. parte il nuovo Iggy su una directory vuota e il motore ci riscrive lo stato vivo (esecuzioni non
     concluse, timer in sospeso) dal database e dall'archivio;
  3. la posizione salvata porta la generazione del bus, perché gli offset ripartono da zero;
  4. il motore riprende.

  Criterio: zero accettate perse, zero esecuzioni in corso perse, timer conservati. La durata della
  pausa si misura; la soglia la decide Marco con il numero davanti (FR-51 non la fissa).

### Blocco B: guasti e ripresa (NFR-6, NFR-15)

3 corse per caso, SQLite; una con Postgres per i casi che toccano il database. Per ogni caso si misura
il tempo fino a `READY` e fino alla prima attività conclusa (mancava nella 2c).

- **Crash del motore** con attività lunghe in volo (`work_ms=3000`): i worker si ricollegano da soli e
  consegnano gli esiti finiti durante la pausa; ripresa entro 30 s; zero accettate perse; ogni
  tentativo eseguito due volte è contato e spiegato.
- **Crash di Iggy:** il motore rifiuta lavoro nuovo con codice finché il bus non torna, e si ricollega
  da solo (oggi un errore del bus ferma il processo). Zero accettate perse, nessun «accettato» senza
  scrittura durevole. Questo misura il crash del processo, non lo spegnimento dell'host: la cache del
  sistema sopravvive.
- **Crash del worker** a metà attività: l'attività torna disponibile alla scadenza del tentativo e si
  ritenta, senza esiti doppi.
- **Disco pieno**, separatamente per i dati di Iggy e per database e archivio. Solo su un mount
  piccolo con limite di dimensione ottenuto senza sudo (per esempio un tmpfs con limite montato da
  docker); prima si verifica che si possa fare, e il filesystem vero di minisheep non si riempie mai.
  Criterio: lavoro nuovo rifiutato con un codice esplicito, accettato non perso, stato precedente
  intatto (ogni payload applicato si decifra), ripresa dopo aver liberato spazio.
- **Pause del disco a 10.000 att./s** (dal blocco A, decisione di Marco del 2026-11-24): il registro
  delle accettate e il tipo di ogni errore del generatore di carico diventano parte di ogni corsa a
  10.000 att./s; 3 corse da 10 minuti con la verifica «zero accettate perse». Se una pausa del disco
  capita, la verifica la copre; se non capita, si dice.

### Blocco C: timer durevoli

Il motore scrive i timer come eventi sul log (attesa tra i tentativi, `wait`, scadenza del tentativo,
battiti del worker) e la ripresa li ricostruisce.

Prima dei timer, due righe aggiunte dopo il blocco B:

- **Percorso dei dati di Iggy:** dalla fase 2 Iggy scriveva in `/app/local_data`, nello strato del
  container, non nel montaggio del compose. Si corregge il montaggio (predefinito `/app/local_data`)
  e si fa una coppia di corse, strato contro montaggio, su accettazione e avvio a 1.000 att./s e su
  10.000 att./s per 10 minuti; se la coppia differisce di oltre il 20 % lo si dice, e i numeri delle
  fasi precedenti vanno riletti.
- **Aggiornamento rifatto col motore del blocco B:** la procedura del blocco A (3 corse, attività da
  3 s) con il nuovo ritorno dei tentativi e la nuova ripresa; pausa dal segnale a `READY` e attività
  rieseguite riportate, prima di aggiungere i timer, così la riga con i timer cambia una cosa sola.

- **Retry con attesa attraverso un crash** (il caso della spine): un'attività fallisce, attesa di 5 s,
  crash del motore a metà attesa, riavvio. Il ritentativo parte una volta sola, non prima della
  scadenza, al più 1 s dopo la fine della ripresa.
- **Molti timer:** 100.000 timer in sospeso, ripresa ancora entro 30 s, RAM riportata.
- **Aggiornamento del bus con i timer:** la procedura del blocco A con 100.000 timer in sospeso e
  attività in volo; pausa dal segnale a `READY` entro 30 s (soglia di Marco del 2026-11-24, uguale
  alla ripresa di NFR-15); nessun timer perso né doppio.

Criterio: nessun timer perso né doppio in 3 corse; ritardo di scatto p50 e p99 riportato.

### Blocco D: log dei task (FR-59)

- **Prima del log, costo dei timer sul percorso caldo (dal blocco C):** ogni consegna ora scrive un
  evento `Timer` sul bus prima dell'invio, più un `INSERT … ON CONFLICT` e un `DELETE` nel proiettore.
  Primo passo: binario costruito dal commit in testa, hash registrato, e una corsa ciascuna di retry e
  scadenza attraverso un crash (nel blocco C quelle righe non hanno corse sul codice committato).
  Corse a coppie, motore del blocco B contro motore del blocco C: avvio a 1.000 att./s (6 coppie, ogni
  braccio primo 3 volte) e 10.000 att./s per 10 minuti (2 coppie, una per ordine, almeno il 99 %), con
  istruzioni SQL per attività (NFR-4). Criterio: mediana per braccio del p99 dell'avvio non peggiore di
  oltre il 20 %, e la differenza si dichiara solo se ha lo stesso segno in tutte e due le posizioni; se
  peggiora, lo si dice con i numeri prima di costruire il log. Stessa aggregazione per le righe a
  coppie del log (worker rumoroso) e del blocco E (ausiliario).
- **Cosa si costruisce:**
  - il worker scrive righe nella sessione gRPC;
  - il motore sostituisce il valore esatto dei segreti consegnati con un segnaposto, applica il tetto
    per tentativo (righe e byte, scarto con conteggio), raggruppa le righe per tentativo in pezzi
    chiusi al più ogni 500 ms, li cifra per tenant nel formato di C-12 e li scrive nell'archivio con
    creazione solo se assente;
  - sul bus va solo il riferimento al pezzo;
  - il worker tiene le righe finché il pezzo non è confermato e dopo un ricollegamento le rimanda;
    i doppioni si scartano per tentativo e numero di pezzo.

  Unico scrittore dei pezzi: il motore (proposta per AD-17, da confermare nell'ADR). RustFS: una prova
  della creazione solo se assente (`If-None-Match`), con la versione verificata prima.
- **Righe misurate:**
  - visibilità: dalla scrittura nel worker alla riga leggibile nell'archivio, p99 entro 1 s;
  - worker rumoroso: tenant A scrive fino al tetto su ogni attività, tenant B a 1.000 attività/s. Corse
    a coppie con e senza A: il p99 di B non peggiora di oltre il 20 %, con le soglie di NFR-2 accanto;
    istruzioni SQL per attività invariate (NFR-4);
  - segreto: gli eventi del bus sono cifrati, quindi la ricerca nei dati grezzi di Iggy non prova
    niente. Si cerca dopo la decifratura, negli eventi del bus e nei pezzi, più una corsa di controllo
    con il segnaposto spento che il valore lo deve trovare;
  - crash del motore con righe in volo: nessuna riga persa né doppia dopo la ripresa;
  - tetto: oltre il tetto le righe si scartano e il pezzo dice quante.

### Blocco E: ausiliario e casi rimasti

- **Chiamata all'ausiliario (AD-6):** sottocomando dello stesso binario in un processo separato,
  chiamato dal motore per `if`, mappatura jq e controllo di input e output; librerie verificate prima
  (versione, licenza). Corse a coppie con e senza l'ausiliario: avvio a 1.000 attività/s (5 coppie,
  p99 non peggiore di oltre il 20 %) e 10.000 attività/s per 10 minuti (2 coppie, almeno il 99 %);
  costo per attività riportato.
- **Database lento ma attivo:** ritardo iniettato nell'adattatore del database fino a portare la
  proiezione oltre 1 s; si registra cosa fa il motore e lo si confronta con AD-2.
- **`SQLITE_BUSY`:** scritture del motore (segreti, chiavi avvolte) insieme ai gruppi del proiettore a
  10.000/s; criterio: zero `SQLITE_BUSY`. Il blocco B ne ha già visti due («database is locked», 331 e
  411 ms) nella corsa r3 a 10.000/s.
- **Annullamento e timeout dell'esecuzione** consegnati a un worker in corso; tempo dal comando
  all'arresto riportato.
- **Credenziale del bus per tenant in sola lettura:** utente Iggy con sola lettura sul topic di A;
  lettura del topic di B e scrittura su A rifiutate.

### Regole delle corse

- Corse a coppie dove la riga confronta con e senza; pavimento del disco registrato prima e dopo ogni
  corsa, come nelle fasi 2d-3.
- Dal blocco C: ogni confronto a coppie alterna l'ordine dei due bracci da una coppia all'altra, e
  ogni file grezzo registra il braccio (variabile o binario), non solo il nome del file. Nel blocco C
  le corse a 200 e 1.000 att./s si sono alternate lenta e veloce fino a 2×, qualunque fosse il braccio.
- Si uccidono solo container e processi propri, per nome o PID salvati; i runner di GitHub su
  minisheep non si toccano.
- **Criterio non superato:** se la causa è nel codice dello spike, si corregge, si rifà la riga e lo si
  dice; se è in Iggy e nessuna configurazione la risolve entro il limite di tempo, ci si ferma e si
  torna da Marco con i numeri, compresa la possibilità di riaprire la scelta del bus.

### Uscita

`spikes/bus/fase4-risultati.md`, una sezione per blocco; numeri grezzi in `spikes/bus/risultati/` con
il suffisso `-4-ms`. Ogni causa citata è segnata «misurata» (con il file grezzo) o «ipotesi». Poi
l'ADR `docs/decisions/YYYY-MM-DD-bus.md` e l'Update della spine, in una PR a parte.
