# Spike S-1, il bus: piano delle fasi

Fonte dei criteri: spine, sezione «S-1 — Bus». Approccio a fasi deciso da Marco il 2026-11-23
(memlog dell'architettura, voci 85 e 86).

| Fase | Cosa | Stato |
| --- | --- | --- |
| 1 | Verifica a tavolino della variante (1), worker diretti sul bus | chiusa: la variante esce (`fase1-worker-diretti.md`) |
| 2 | Iggy contro Valkey, con un motore minimo e il collegamento gRPC | da fare |
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
