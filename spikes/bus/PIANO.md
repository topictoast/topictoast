# Spike S-1, il bus: piano delle fasi

Fonte dei criteri: spine, sezione «S-1 — Bus». Approccio a fasi deciso da Marco il 2026-11-23
(memlog dell'architettura, voci 85 e 86).

| Fase | Cosa | Stato |
| --- | --- | --- |
| 1 | Verifica a tavolino della variante (1), worker diretti sul bus | chiusa: la variante esce (`fase1-worker-diretti.md`) |
| 2 | Iggy contro Valkey, con un motore minimo e il collegamento gRPC | chiusa senza vincitore sul desktop (`fase2-risultati.md`) |
| 2b | Ritardi di coda di Iggy e Valkey rimisurati su una macchina ferma (minisheep) | in corso |
| 3 | gRPC contro iroh, solo sul bus vincitore | dopo la fase 2 |
| 4 | Ripresa, guasti, aggiornamento senza perdite, log dei task, timer durevoli, ADR e Update della spine | dopo la fase 3 |

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
