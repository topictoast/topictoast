# Spike S-1, fase 3: gRPC contro iroh per il collegamento dei worker (Iggy, minisheep)

Data: 2026-11-24. Segue PIANO.md, sezione «Fase 3». Solo Iggy (Valkey non è stato toccato), solo minisheep (4 CPU `1-4`, carico su `8-11`),
motore della 2d (4 gruppi in volo, 1.024 messaggi, soglia 64), un solo motore e un solo worker con 4 sessioni per collegamento. Numeri grezzi:
`risultati/iggy-sqlite-<riga>-<collegamento>-3-ms-r<n>.txt` (ogni file porta il pavimento di `fdatasync` prima e dopo la corsa, le CPU usate, il carico).
Ogni causa è segnata **misurata** (con il file grezzo) o **ipotesi**.

## 1. Sintesi

1. **Resta gRPC.** Per la regola della fase 3 iroh sostituisce gRPC solo se migliora almeno del 20 % il p99 di `start_sent` o la CPU per 1.000 attività: li peggiora entrambi
   in ogni riga misurata (p99 di `start_sent` a 1.000 att./s: gRPC 0,31 ms, iroh 0,50 ms, +61 %; CPU totale di motore e worker per 1.000 attività: 0,120 s contro 0,161 s, +34 %),
   e nei 10 minuti una corsa di iroh su due resta sotto il 99 % del ritmo offerto (9.643 att./s, 96,4 %). Il ritmo massimo è uguale (25.000 offerte, 25.000 sostenute da tutti e due).
2. **Impostazione del motore (passo 0): resta la 2d per tutte le righe.** La 2c a un gruppo in volo non ha il p99 mediano più basso del 20 % in nessuna delle due righe (accettazione +1,5 %, avvio +2,2 % a favore della 2d).
3. **Versione di iroh: 1.4.1** (34 giorni, MIT OR Apache-2.0); la 1.4.2 e la 1.5.0 sono del 2026-11-19 e nessun avviso pubblicato le giustifica.
4. **Sicurezza:** le condizioni sono soddisfatte da tutti e due i collegamenti, ognuna con una prova di rifiuto riuscita (§4). Una prova ha trovato un difetto nel mio endpoint iroh (prese UDP su tutte le interfacce), corretto prima delle misure.
5. **I 10 minuti non discriminano il collegamento**: in tutte le quattro corse ci sono episodi in cui l'append su Iggy passa da 80 a 400-470 ms con pressione I/O 30-37 %, con gRPC e con iroh (§6).

## 2. Passo 0: impostazione del motore con Iggy

3 coppie alternate (ordine a/b invertito nella ripetizione 2), SQLite, `accept` (200 es./s, p99 dal lato motore) e `start1k` (1.000 att./s, `start_ready`); a = 2c (`KGROUPS=1 MAXB=256`), b = 2d (4 / 1.024 / 64).
Valori p50 / p99 ms (pavimento prima > dopo la corsa in ms):

| Riga | Impostazione | r1 | r2 | r3 | p99 mediano |
| --- | --- | --- | --- | --- | --- |
| Accettazione | a (2c) | 11,75 / 18,96 (2,75 > 1,93) | 11,73 / 19,09 (2,86 > 2,71) | 11,46 / 19,26 (6,46 > 2,81) | 19,09 |
| Accettazione | b (2d) | 11,53 / 18,54 (2,72 > 1,94) | 11,65 / 18,80 (6,84 > 2,94) | 11,70 / 18,82 (2,76 > 2,85) | 18,80 |
| Avvio a 1.000/s | a (2c) | 8,29 / 11,52 (4,90 > 2,92) | 8,39 / 11,61 (2,87 > 2,80) | 8,29 / 11,61 (6,93 > 2,79) | 11,61 |
| Avvio a 1.000/s | b (2d) | 8,45 / 11,36 (6,82 > 2,27) | 8,37 / 11,56 (7,11 > 2,79) | 8,41 / 11,21 (2,85 > 2,84) | 11,36 |

File: `iggy-sqlite-{accept,start1k}-s0{a,b}-3-ms-r{1,2,3}.txt`. **Scelta: 2d per tutte le righe** (la 2c dovrebbe stare almeno 20 % sotto in entrambe le righe: la differenza è +1,5 % e +2,2 % a favore della 2d, cioè nessuna).

Letture, con la loro etichetta:
- **misurata:** a pavimento di circa 2,8 ms dopo la corsa, le due impostazioni danno lo stesso p99 (18,5-19,3 ms in accettazione, 11,2-11,6 ms in avvio), uguale alla 2c (18,5-19,2 e 11,3-12,0).
  Quindi il motore della 2d non è la causa del p99 più alto visto nella 2d (accettazione 37-488 ms): a questo disco con questo stato le due impostazioni non differiscono.
- **misurata:** il pavimento misurato *prima* di una corsa varia da 2,7 a 7,1 ms nello stesso blocco ma il p99 della corsa non lo segue (es. `accept-s0a-r3`: pavimento prima 6,46 ms, p99 19,26 ms; `accept-s0b-r1`: 2,72 ms, 18,54 ms).
  Il pavimento prima della corsa non predice il p99 in queste 12 corse.
- **ipotesi:** la causa dell'aumento dei ritardi nella 2d resta non isolata (il pavimento più alto di allora e un diverso stato del disco sono compatibili, non dimostrati); questo passo la esclude solo per l'impostazione del motore.
- Dato per l'ADR: su questo disco, a pavimento 2,7-2,9 ms, Iggy a regime basso sta a p99 18,5-19,3 ms in accettazione e 11,2-11,6 ms in avvio, dentro il limite dei 25 ms con margine del 23-55 %.

## 3. iroh: versione e fatti, verificati sul registro prima di scrivere codice

Controllato il 2026-11-24 (due settimane prima = 2026-11-10):

| Fatto | Fonte controllata | Esito |
| --- | --- | --- |
| Versioni e date | `https://crates.io/api/v1/crates/iroh/versions` | 1.5.0 e 1.4.2: 2026-11-19; 1.4.1: 2026-10-21; 1.4.0: 2026-10-20; nessuna yankata |
| Ultima con almeno due settimane | stessa pagina | **1.4.1** (34 giorni al 24/11) |
| Licenza | `https://crates.io/api/v1/crates/iroh/1.4.1` e `cargo metadata` | `MIT OR Apache-2.0`, ammessa dalla spine (Stack); idem iroh-base, iroh-relay, noq. Nessuna licenza nuova nell'albero oltre quelle già presenti (`r-efi` ha una scelta ammessa) |
| Avvisi di sicurezza per 1.4.1 e 1.4.2 | `POST https://api.osv.dev/v1/query` con `{"package":{"name":"iroh","ecosystem":"crates.io"},"version":"1.4.2"}` (e 1.4.1) | risposta `{}`: nessun avviso pubblicato |
| Cosa corregge la 1.4.2 / 1.5.0 | `https://raw.githubusercontent.com/n0-computer/iroh/main/CHANGELOG.md` | correzioni dell'attore del relay e di `iroh-relay` (limiti, ping, flush): nessun avviso, e il relay è spento nella nostra configurazione |
| Documentazione della versione | `https://docs.rs/iroh/1.4.1/iroh/` | pagina presente (HTTP 200) |
| Funziona con relay e scoperta spenti e identità del nodo remoto fissata | sorgente del crate 1.4.1: `Builder::empty()` («empty builder with no address lookup services, and `RelayMode::Disabled`», `src/endpoint.rs`), `presets::Minimal`, `clear_address_lookup`, `clear_ip_transports`, `Endpoint::connect(EndpointAddr{id, indirizzo diretto})`; e la prova di §4 | sì: `relay_urls=0`, nessun servizio di scoperta, la connessione verso un'identità diversa fallisce nell'handshake |

La 1.4.2 (spine: «solo con l'eccezione di sicurezza») **non ha** un avviso pubblicato, quindi si usa la 1.4.1. Anche le dipendenze transitive rispettano le due settimane: ho riportato a versioni del 2026-11-09 o prima
`iroh-base`/`iroh-relay`/`iroh-dns` 1.4.1, `noq`/`noq-proto`/`noq-udp` 1.4.0, `iroh-metrics` 1.0.2, `n0-dns-resolver` 0.1.0, `simple-dns` 0.13.0, `postcard` 1.1.3, `postcard-derive` 0.2.2, `plist` 1.10.3, `mio` 1.2.4
(`Cargo.lock` committato; feature `tls-ring` senza `portmapper`/`metrics`).

## 4. Condizioni di sicurezza e prove di rifiuto (file `iggy-sqlite-sec-{grpc,iroh}-3-ms-r1.txt`)

Credenziale del worker (C-11): 32 byte casuali in esadecimale in `out/worker.cred`, generata (`gen-cred`), mai con un valore predefinito. Il saluto del worker la porta e il motore la controlla a **ogni** collegamento di sessione (prima la confrontava con nulla).
Con iroh l'identità del nodo del worker è **derivata** dalla credenziale (SHA-256 sulla credenziale con un prefisso fisso): il motore accetta una connessione solo se l'identità del nodo, provata dall'handshake, è tra quelle derivate dalle credenziali note, e ogni sessione ripete la credenziale. L'identità del motore viene da un file di configurazione (`engine.nodeid`).

| Condizione | Collegamento | Prova di rifiuto | Esito misurato |
| --- | --- | --- | --- |
| Credenziale controllata a ogni collegamento | gRPC | worker con credenziale errata | **rifiutato**: `Unauthenticated: E_CREDENZIALE` |
| Credenziale controllata a ogni collegamento | iroh | worker con credenziale errata (identità derivata dalla credenziale sbagliata) | **rifiutato**: connessione chiusa dal motore |
| Il worker verifica il server | gRPC | certificato di un altro motore | **rifiutato**: `invalid peer certificate: BadSignature` |
| Il worker verifica il server | iroh | identità del motore attesa diversa | **rifiutato**: handshake fallito (`invalid peer certificate: UnknownIssuer`) |
| Identità del nodo legata alla credenziale (solo iroh) | iroh | credenziale giusta ma chiave di nodo casuale | **rifiutato**: connessione chiusa dal motore |
| Relay e scoperta esterni spenti (solo iroh) | iroh | `relay_urls=0`, servizi di scoperta `data: []`; elenco delle prese di rete di motore e worker | solo `127.0.0.1` |
| Controllo positivo | entrambi | credenziale, identità e certificato giusti | accettato |

La prova delle prese ha trovato un difetto del mio codice: con `presets::Minimal` iroh apriva comunque prese UDP su `[::]` (due porte casuali), oltre all'indirizzo indicato. Corretto con `clear_ip_transports()`; dopo la correzione
le uniche prese sono `127.0.0.1:47011` e quella del worker su `127.0.0.1`. (Prima della correzione: `udp [::]:38518`, `udp [::]:34104`, in un'esecuzione di sviluppo non conservata.)

## 5. L'adattatore iroh

`src/iroh_link.rs`: stessa forma di gRPC (una connessione per worker, 4 sessioni bidirezionali, stessi messaggi `Delivery` e `FromWorker` codificati con `prost`, prefisso di lunghezza u32); il motore e il worker scelgono il collegamento
da configurazione (`link=grpc|iroh`), il resto del codice (motore, proiettore, lavoro del worker) è comune. Tempo: circa un'ora e mezza di codice e prove, dentro le 4 ore. Il worker scrive la propria CPU su file ogni 200 ms (`CLOCK_PROCESS_CPUTIME_ID`, come `engine_cpu_s`) e il generatore di carico ne riporta la differenza sulla finestra di misura (`cpu_per_1000_att`).
Tutte le corse sono con 4 sessioni (nella 2d i worker aprivano 8): è la forma richiesta dal piano.


## 6. Misure a coppie (gRPC poi iroh, ordine invertito a ogni ripetizione)

Pavimento di `fdatasync` registrato prima e dopo ogni corsa (ultima colonna delle tabelle); nessuna corsa è partita «macchina non ferma»; nei 10 minuti attese del cancello di 165-180 s.
Il tempo di avvio completo (`start_ready`) e le soglie assolute del PRD sono **riportati, non giudicati**, perché comprendono la scrittura durevole su Iggy, comune ai due collegamenti.

### 6.1 Righe giudicate: mediane delle coppie

| Riga (corse) | Misura | gRPC | iroh | Variazione di iroh |
| --- | --- | --- | --- | --- |
| Avvio a 1.000 att./s (5) | `start_sent` p50 | 0,15 ms | 0,20 ms | +33 % |
| | `start_sent` p99 | 0,31 ms | 0,50 ms | **+61 %** |
| | CPU motore / 1.000 att. | 0,096 s | 0,119 s | +24 % |
| | CPU worker / 1.000 att. | 0,024 s | 0,042 s | +75 % |
| | CPU totale / 1.000 att. | 0,120 s | 0,161 s | **+34 %** |
| Payload da 64 KB, 500 att./s (3) | `start_sent` p50 / p99 | 0,40 / 1,10 ms | 0,57 / 1,42 ms | +43 % / +29 % |
| | CPU totale / 1.000 att. | 0,593 s | 0,765 s | +29 % |
| Payload da 1 MB, 50 att./s (3) | `start_sent` p50 / p99 | 0,88 / 1,85 ms | 1,56 / 3,05 ms | +77 % / +65 % |
| | CPU totale / 1.000 att. | 6,554 s | 10,417 s | +59 % |
| Ritmo massimo `thrmax` 30 s, 25.000 offerti (3) | att./s sostenute | 24.998 | 25.013 | +0,06 % (pari: il ritmo offerto non satura nessuno dei due) |
| Tenant lento `iso` (3) | `start_sent` p50 / p99 | 0,13 / 0,36 ms | 0,17 / 0,53 ms | +31 % / +47 % |
| 10.000 att./s per 10 minuti, ≥ 99 % dell'offerto (2) | att./s | 9.959 e 9.951 (99,6 %, 99,5 %): tiene | 9.643 (96,4 %) e 9.999 (100,0 %): **non tiene** nella prima | |

La CPU per 1.000 attività è la CPU del processo motore (come `engine_cpu_s`) più quella del worker sulla finestra di misura, divisa per le attività concluse. Il 1 MB a 50 att./s non ha saturato il bus con gRPC nella prova di fumo (50 sostenute), quindi il ritmo non è stato dimezzato.

**Riga extra, fuori dal piano e non giudicata:** `thrmax` con 50.000 att./s offerte (3 coppie, `*-thrmax-50k-*`): gRPC 4.711 / 20.361 / 21.934, iroh 23.708 / 18.075 / 23.055 att./s; la corsa gRPC r1 cade con il pavimento a 10 ms prima e dopo. Valori dominati dal bus e dal disco, mediane 20.361 e 23.055; non confrontabili tra i collegamenti.

### 6.2 Riportato: ritardo di avvio completo

`start_ready` p50 / p99 a 1.000 att./s: gRPC 8,29-8,35 / 11,20-11,90 ms, iroh 8,29-8,46 / 11,57-12,29 ms (soglia assoluta del PRD: 25 ms al p99). Tenant lento: avvio del tenant B p99 gRPC 14,8-15,4 ms, iroh 14,7-15,8 ms. Nessuna differenza leggibile: la variabilità tra corse è più grande della differenza.

### 6.3 Tutte le corse

Legenda: ms; `start_sent` = dalla consegna inviata a quella ricevuta nel worker; CPU in secondi per 1.000 attività; pavimento di `fdatasync` prima > dopo la corsa. Nelle righe `thr10k` il p50/p99 di `start_ready` è l'arretrato (non una latenza).

#### start1k
| link | corsa | start_sent p50 ms | start_sent p99 ms | start_ready p50 ms | start_ready p99 ms | att./s | CPU motore /1000 att. | CPU worker /1000 att. | pavimento prima > dopo (ms) |
|---|---|---|---|---|---|---|---|---|---|
| grpc | r1 | 0.15 | 0.30 | 8.35 | 11.20 | 999 | 0.096 | 0.024 | 4.72 > 2.88 |
| iroh | r1 | 0.20 | 0.52 | 8.34 | 12.29 | 1000 | 0.118 | 0.043 | 7.06 > 2.92 |
| grpc | r2 | 0.15 | 0.32 | 8.29 | 11.90 | 1000 | 0.096 | 0.024 | 2.97 > 2.60 |
| iroh | r2 | 0.20 | 0.51 | 8.46 | 11.71 | 1000 | 0.119 | 0.042 | 7.33 > 2.72 |
| grpc | r3 | 0.15 | 0.31 | 8.29 | 11.39 | 1002 | 0.095 | 0.024 | 2.77 > 2.47 |
| iroh | r3 | 0.20 | 0.49 | 8.37 | 11.62 | 1000 | 0.118 | 0.042 | 6.21 > 2.65 |
| grpc | r4 | 0.15 | 0.31 | 8.29 | 11.42 | 1000 | 0.096 | 0.024 | 6.93 > 2.63 |
| iroh | r4 | 0.20 | 0.50 | 8.42 | 11.59 | 1000 | 0.120 | 0.042 | 2.23 > 2.73 |
| grpc | r5 | 0.15 | 0.32 | 8.32 | 11.86 | 1000 | 0.096 | 0.024 | 2.78 > 2.81 |
| iroh | r5 | 0.20 | 0.50 | 8.29 | 11.57 | 1000 | 0.119 | 0.042 | 3.94 > 2.57 |

#### size64k
| link | corsa | start_sent p50 ms | start_sent p99 ms | start_ready p50 ms | start_ready p99 ms | att./s | CPU motore /1000 att. | CPU worker /1000 att. | pavimento prima > dopo (ms) |
|---|---|---|---|---|---|---|---|---|---|
| grpc | r1 | 0.38 | 1.10 | 9.13 | 18.91 | 500 | 0.451 | 0.138 | 2.80 > 2.85 |
| iroh | r1 | 0.57 | 1.42 | 9.31 | 18.80 | 500 | 0.535 | 0.229 | 5.60 > 2.65 |
| grpc | r2 | 0.40 | 1.14 | 9.21 | 18.72 | 500 | 0.462 | 0.140 | 2.65 > 2.49 |
| iroh | r2 | 0.57 | 1.44 | 9.27 | 17.97 | 500 | 0.539 | 0.230 | 7.33 > 2.85 |
| grpc | r3 | 0.40 | 1.08 | 9.21 | 19.12 | 500 | 0.456 | 0.137 | 2.62 > 2.94 |
| iroh | r3 | 0.56 | 1.40 | 9.18 | 18.80 | 500 | 0.535 | 0.230 | 2.85 > 2.69 |

#### size1m
| link | corsa | start_sent p50 ms | start_sent p99 ms | start_ready p50 ms | start_ready p99 ms | att./s | CPU motore /1000 att. | CPU worker /1000 att. | pavimento prima > dopo (ms) |
|---|---|---|---|---|---|---|---|---|---|
| grpc | r1 | 0.88 | 1.76 | 10.70 | 20.43 | 50 | 5.040 | 1.514 | 6.79 > 2.87 |
| iroh | r1 | 1.56 | 3.05 | 11.08 | 20.06 | 50 | 6.573 | 3.661 | 2.85 > 2.89 |
| grpc | r2 | 0.96 | 1.85 | 10.70 | 18.56 | 50 | 4.927 | 1.683 | 2.90 > 2.83 |
| iroh | r2 | 1.58 | 3.17 | 11.12 | 19.79 | 50 | 6.640 | 3.789 | 4.96 > 2.83 |
| grpc | r3 | 0.87 | 1.92 | 10.77 | 18.50 | 50 | 4.993 | 1.514 | 2.81 > 2.78 |
| iroh | r3 | 1.56 | 2.99 | 11.22 | 21.89 | 50 | 6.647 | 3.770 | 4.51 > 2.83 |

#### thrmax
| link | corsa | start_sent p50 ms | start_sent p99 ms | start_ready p50 ms | start_ready p99 ms | att./s | CPU motore /1000 att. | CPU worker /1000 att. | pavimento prima > dopo (ms) |
|---|---|---|---|---|---|---|---|---|---|
| grpc | r1 | 0.55 | 2.79 | 49.15 | 72.19 | 24998 | 0.034 | 0.004 | 2.82 > 2.74 |
| iroh | r1 | 1.05 | 4.30 | 49.76 | 76.29 | 25000 | 0.039 | 0.007 | 2.78 > 2.79 |
| grpc | r2 | 0.55 | 2.87 | 49.02 | 80.83 | 25020 | 0.034 | 0.004 | 6.98 > 2.73 |
| iroh | r2 | 1.03 | 3.75 | 48.77 | 70.46 | 25013 | 0.037 | 0.007 | 2.76 > 2.52 |
| grpc | r3 | 0.55 | 3.00 | 48.51 | 71.94 | 24995 | 0.034 | 0.004 | 2.64 > 2.86 |
| iroh | r3 | 1.05 | 4.31 | 50.05 | 78.21 | 25041 | 0.039 | 0.007 | 6.75 > 2.60 |

#### thrmax-50k
| link | corsa | start_sent p50 ms | start_sent p99 ms | start_ready p50 ms | start_ready p99 ms | att./s | CPU motore /1000 att. | CPU worker /1000 att. | pavimento prima > dopo (ms) |
|---|---|---|---|---|---|---|---|---|---|
| grpc | r1 | 0.02 | 2.26 | 7249.92 | 19595.26 | 4711 | 0.106 | 0.019 | 9.95 > 10.38 |
| iroh | r1 | 1.22 | 6.54 | 1996.80 | 5214.21 | 23708 | 0.052 | 0.007 | 8.08 > 4.45 |
| grpc | r2 | 0.86 | 5.41 | 2887.68 | 5816.32 | 20361 | 0.051 | 0.005 | 4.47 > 8.24 |
| iroh | r2 | 0.88 | 6.51 | 2850.82 | 7344.13 | 18075 | 0.060 | 0.011 | 4.60 > 4.86 |
| grpc | r3 | 0.90 | 5.39 | 2658.30 | 5566.46 | 21934 | 0.050 | 0.005 | 4.66 > 6.48 |
| iroh | r3 | 0.05 | 2.97 | 3162.11 | 5316.61 | 23055 | 0.078 | 0.023 | 4.57 > 4.65 |

#### iso
| link | corsa | start_sent p50 ms | start_sent p99 ms | start_ready p50 ms | start_ready p99 ms | att./s | CPU motore /1000 att. | CPU worker /1000 att. | pavimento prima > dopo (ms) |
|---|---|---|---|---|---|---|---|---|---|
| grpc | r1 | 0.13 | 0.38 | 9.01 | 19087.36 | 1171 | 0.123 | 0.023 | 6.58 > 2.79 |
| iroh | r1 | 0.18 | 0.53 | 9.11 | 19087.36 | 1170 | 0.142 | 0.039 | 2.91 > 2.69 |
| grpc | r2 | 0.13 | 0.35 | 8.90 | 19087.36 | 1171 | 0.123 | 0.023 | 2.87 > 2.62 |
| iroh | r2 | 0.17 | 0.54 | 8.96 | 19087.36 | 1171 | 0.142 | 0.038 | 4.42 > 2.81 |
| grpc | r3 | 0.14 | 0.36 | 9.04 | 19087.36 | 1171 | 0.122 | 0.023 | 2.85 > 1.96 |
| iroh | r3 | 0.17 | 0.53 | 8.87 | 19087.36 | 1171 | 0.142 | 0.040 | 2.72 > 1.94 |

#### thr10k
| link | corsa | start_sent p50 ms | start_sent p99 ms | start_ready p50 ms | start_ready p99 ms | att./s | CPU motore /1000 att. | CPU worker /1000 att. | pavimento prima > dopo (ms) |
|---|---|---|---|---|---|---|---|---|---|
| grpc | r1 | 0.51 | 4.09 | 95.81 | 10493.95 | 9959 | 0.041 | 0.005 | 1.91 > 4.63 |
| iroh | r1 | 0.88 | 5.16 | 96.13 | 19234.81 | 9643 | 0.045 | 0.009 | 2.89 > 4.63 |
| grpc | r2 | 0.54 | 4.06 | 96.64 | 15171.58 | 9951 | 0.040 | 0.005 | 4.55 > 4.26 |
| iroh | r2 | 0.81 | 3.08 | 83.26 | 218.11 | 9999 | 0.039 | 0.009 | 4.62 > 4.56 |


## 7. Cause: misurate e ipotesi

| Affermazione | Etichetta | Evidenza |
| --- | --- | --- |
| iroh ha `start_sent` più alto (p50 +33 %, p99 +61 % a 1.000/s) e CPU più alta (worker +75 %, motore +24 %) di gRPC | **misurata** (l'effetto) | `iggy-sqlite-start1k-{grpc,iroh}-3-ms-r1..r5.txt`, righe `start_sent` e `cpu_per_1000_att`; coerente in tutte le 5 coppie e nelle righe 64 KB, 1 MB, `iso` |
| La causa del costo in più di iroh è QUIC in spazio utente (cifratura e cicli di pacchetto nello stesso processo) più una copia per messaggio nel mio adattatore (prefisso di lunghezza e corpo concatenati in un `Vec` per ogni messaggio) | **ipotesi** | non isolata: non ho profilato né provato una variante senza la copia; le differenze crescono con la dimensione del payload (1 MB: CPU +59 %), compatibile con costi per byte, non dimostrato |
| I 10 minuti non discriminano i collegamenti: gli episodi in cui l'append su Iggy passa da circa 80 a 400-470 ms (pressione I/O 30-37 %) compaiono con tutti e due | **misurata** | `iggy-sqlite-thr10k-grpc-3-ms-r1.txt` righe `MIN t=2` (append p50 399 ms) e `t=7`; `iggy-sqlite-thr10k-iroh-3-ms-r1.txt` righe `MIN t=8`, `t=9` (447 e 473 ms), sub-richiesta 111-117 ms |
| La corsa iroh r1 sotto il 99 % (9.643 att./s) viene da quegli episodi e non dal collegamento | **ipotesi** | la serie al minuto mostra 4.669 e 4.435 att./s nei minuti 8-9 con append p50 447-473 ms; ma la corsa gRPC r1 ha un episodio analogo al minuto 2 (6.476 att./s) e resta sopra il 99 % perché recupera nei minuti 3 e 8 (15.712 e 14.364): non si può escludere che iroh recuperi peggio |
| Il motore della 2d non è la causa dei p99 più alti visti nella 2d | **misurata** (per l'impostazione) | passo 0, §2 |
| La causa dell'aumento dei p99 della 2d | **ipotesi** (non isolata) | v. §2 |

## 8. Verdetto per la regola della fase 3

«iroh sostituisce gRPC solo se migliora di almeno il 20 % il p99 di `start_sent` o la CPU per 1.000 attività, non peggiora di oltre il 20 % nessun'altra riga giudicata e tiene le 10.000 attività/s. Altrimenti resta gRPC.»

- Miglioramento del p99 di `start_sent` o della CPU per 1.000 attività: **no**, in nessuna riga (+61 %, +29 %, +65 %, +47 % sul p99; +34 %, +29 %, +59 % sulla CPU).
- Nessuna riga peggiora meno del 20 %: **no** (quasi tutte oltre il 25 %), né iroh tiene le 10.000/s in entrambe le corse (96,4 % nella prima).
- **Resta gRPC con TLS**, come scelta di parità (meno parti, meno codice nuovo). Il collegamento iroh passa le condizioni di sicurezza (§4) e i test di rifiuto, quindi non esce per sicurezza ma per i numeri.

## 9. Comandi per rifare

Su minisheep, in `~/ttspikebus` (`env.sh` fissa `SPIKE_BIN`, `SPIKE_DATA`, `SPIKE_CPUS=1-4`, `SPIKE_CPUSET=1-4`, `SPIKE_LOADCPUS=8-11`; `.env` da `./gen-env.sh`):

| Misura | Comando |
| --- | --- |
| Passo 0 | `./ms-step0.sh` |
| Prove di rifiuto | `./ms-pair.sh sec 1 1` (esegue `scen.sh … sec` con `LINK=grpc` e `LINK=iroh`) |
| Corse a coppie | `./ms-final3.sh` (start1k 5, size64k 3, size1m 3, thrmax 3, iso 3, thr10k 2 con `GATE60=1`) |
| Riga extra a 50.000/s | `SFX=-50k ./ms-pair2.sh thrmax 1 3 secs=30 rate=5000` |
| Tabelle | `MD=1 ./an3.sh <riga>` (da `spikes/bus/`, file in `risultati/`) |
| Scelta del collegamento | `LINK=grpc|iroh` per `scen.sh`; motore e worker: `link=`, `iroh_listen=`, `cred=`, `once=1` (prove di rifiuto) |

## 10. Deviazioni

- **Ritmo massimo (`thrmax`):** il ritmo offerto del piano (25.000 att./s) non satura il motore corretto con nessuno dei due collegamenti, quindi il massimo vero non è misurato nella riga del piano; ho aggiunto la riga extra a 50.000, non giudicata.
- **Regola «tiene le 10.000/s»:** la lettura «almeno il 99 % del ritmo offerto» è quella fissata nel piano; la prima corsa iroh è al 96,4 %.
- **Prove di sviluppo non conservate** (non nel verdetto): fumo di `start1k` 10 s e `size1m` per entrambi i collegamenti, e una prima prova di rifiuto prima della correzione delle prese UDP.
- **Sessioni del worker:** 4 per tutte le corse (la 2d ne usava 8), per rispettare la forma del piano.
- **Il pavimento prima della corsa** misurato da `ms-run.sh` (300 scritture) va da 1,9 a 10,4 ms nel blocco; non ho scartato né ripetuto corse.
- **Tempo dell'adattatore iroh:** circa un'ora e mezza, sotto il limite di 4 ore. Tempo totale della fase molto più alto per le corse da 10 minuti (circa 1 ora e 50 min di attesa).
- **Comandi lunghi:** il sistema ha spostato in background i comandi sopra i 180 s senza interromperli; non ho mai avviato un secondo comando mentre uno girava.
- **Cleanup e controllo delle password:** compose giù e directory dati vuote su minisheep (e sul desktop, che non è stato caricato), `.env` di minisheep cancellato dopo il controllo `git grep -lF -- "<valore>" HEAD`: 0 file per ognuna delle tre password.
