---
title: topictoast — estratto del PRD per l'architettura
created: 2026-11-20
fonti:
  - _bmad-output/planning-artifacts/prds/prd-topictoast-2026-11-19/prd.md (final, 2026-11-20)
  - _bmad-output/planning-artifacts/prds/prd-topictoast-2026-11-19/addendum.md
  - _bmad-output/planning-artifacts/prds/prd-topictoast-2026-11-19/.memlog.md
  - per i meccanismi suggeriti: triage.md (T-1…T-36) e triage-update.md, citati dall'addendum
---

# Estratto del PRD per l'architettura

Precedenza tra le fonti: testo finale del PRD > voce più recente del memlog > voci precedenti >
triage. Voci superate segnalate in §7.3. Termini del glossario PRD §3: **task** (passo del
workflow) ≠ **attività** (unità consegnata a worker o sandbox, una per elemento di fan-out e per
tentativo; unità degli NFR di prestazione); "pipeline stream", mai "pipeline" da sola.

## 1. Vincoli duri numerici

Macchina di riferimento (§3): 4 vCPU, 16 GB RAM, SSD NVMe locale, Linux. Tutti gli NFR misurati
con il benchmark di FR-53, **con SQLite e con Postgres** sulla stessa macchina, con forma del
carico, dimensione dei payload, insieme di dati e modalità di scrittura dichiarati (§5).

### 1.1 Prestazioni e footprint (§5)

| Id | Vincolo | Cifra | Stato | Condizioni di misura |
|---|---|---|---|---|
| NFR-1 | RAM a riposo | ≤ 80 MB | confermato (perimetro: vedi FR-50 `[ASSUMPTION]`) | somma RSS di motore + ogni processo ausiliario attivo + bus, **con SQLite**, 60 s dopo l'avvio, insieme di dati dichiarato (tenant, workflow, storico), nessuna esecuzione né pipeline stream attiva; esclusi worker, stato delle pipeline stream, pool della sandbox forte |
| NFR-2 | Latenza di avvio attività | p50 ≤ 5 ms, p99 ≤ 25 ms | confermato | da attività pronta (dipendenze soddisfatte) a ricezione nel worker; worker sulla stessa macchina; carico 1.000 attività/s; a regime (FR-50) |
| NFR-3 | Throughput | ≥ 10.000 attività/s | confermato | attività vuote, worker sulla stessa macchina, sostenuto 10 min; p99 riportato anche qui |
| NFR-4 | Carico sul database | ≤ 5 istruzioni SQL per attività | confermato | media sull'intero ciclo di vita dell'attività durante NFR-3; con Valkey + scarico conta anche lo scarico, con righe e WAL per attività (addendum) |
| NFR-5 | Pipeline stream | ≥ 100.000 eventi/s | `[ASSUMPTION]`, da confermare dopo lo spike (§11 n.2) | una pipeline stream con filtro e conteggio su finestra fissa; eventi via API/SDK di FR-18; 10 min |
| NFR-12 | Sandbox | leggera: avvio p50 ≤ 10 ms, ≤ 64 MB/istanza; forte: ≤ 10 ms con sandbox libera nel pool | `[ASSUMPTION]`, dopo lo spike (§11 n.3) | da attività pronta a inizio esecuzione del nodo; memoria con nodo vuoto |
| NFR-13 | Tempo al primo workflow | ≤ 15 min | `[ASSUMPTION]` | da download/clone a workflow di esempio completato, guida rapida, macchina pulita, persona nuova (UJ-5, SM-5) |
| NFR-15 | Ripresa dopo crash del motore | torna a consegnare entro 30 s, senza intervento | confermato (memlog 48) | crash iniettato (FR-53); durante la ripresa nessuna nuova esecuzione né task successivo |
| SM-2 | Carico DB, regime basso | ≤ 5.000 istruzioni SQL/giorno | confermato (memlog 49) | stesso carico di hatchet-dev/hatchet#5153 (~680 task/giorno; Hatchet ~5,2 M/giorno, ~7.700 istruzioni e 600 KB di WAL per task); regime alto = NFR-4 |

Riportati **senza obiettivo** (FR-53): RAM sotto carico, RAM per pipeline stream attiva, tempo di
riavvio a freddo dei processi ausiliari (obiettivo: §11 n.9), tempo di ripresa NFR-15.

Contro-metriche: SM-C1 durabilità prima del carico DB (perdite = 0); SM-C2 niente RAM a riposo
sotto 80 MB a costo del p99 sotto carico.

### 1.2 Durabilità, perdita di dati, ripresa

- NFR-6: **zero** attività, esecuzioni, eventi accettati persi per riavvio, crash di qualunque
  processo dell'installazione, spegnimento improvviso dell'host; vale anche durante NFR-3 e NFR-5.
  Disco pieno: rifiuto esplicito di lavoro nuovo, nessuna perdita, ogni scrittura rifiutata lascia
  intatto lo stato precedente. Backup/ripristino coerenti DB + bus fuori dalla v1.
- FR-9: "accettato" = già al sicuro dal guasto di NFR-6 quando parte la risposta.
- FR-53: il benchmark riporta accettati-poi-persi dopo guasto iniettato in NFR-3 e NFR-5 (obiettivo 0).
- FR-51: aggiornamento di qualunque componente (bus compreso) senza perdere attività accettate né
  esecuzioni in corso; migrazione dello stato automatica.

### 1.3 Limiti, conservazione, tempi

| Id | Grandezza | Cifra | Stato |
|---|---|---|---|
| FR-11 | Input e output di un'attività | 1 MB (per elemento nel fan-out) | `[ASSUMPTION]`; definizione esatta: spec (T-33) |
| FR-11 | Input aggregato del join | tetto configurabile, predefinito documentato | valore dalla spec (NFR-14) |
| FR-22 | Intervallo dei checkpoint | 10 s predefinito, configurabile | `[ASSUMPTION]` |
| FR-31 | Conservazione dello storico | 30 giorni predefinito, configurabile per tenant | `[ASSUMPTION]` |
| FR-31 | Eliminazione di un tenant | dati illeggibili ovunque entro un tempo massimo documentato | valore dalla spec |
| FR-6 | Chiave di idempotenza | valida almeno per la conservazione dell'esecuzione (FR-31); 100 richieste concorrenti con la stessa chiave = 1 esecuzione | confermato |
| FR-18 | Conservazione degli stream | pochi minuti: ≥ finestra più lunga + ritardo tollerato + intervallo dei checkpoint; pipeline stream con finestra oltre la conservazione rifiutata | confermato (memlog 43) |
| FR-21 | Evento dal futuro | oltre una tolleranza non sposta il watermark; un evento a +1 anno non chiude finestre | tolleranza dalla spec |
| FR-41 | Cambio di stato nella vista delle esecuzioni | ≤ 1 s | confermato (memlog 58) |
| FR-41 | Fan-out da migliaia di elementi | resta leggibile (raggruppato per stato) | confermato |
| FR-57 | Pianificazioni del kit | ≤ 1 per workflow per tenant | confermato |
| FR-13 | Concorrenza del worker | mai più attività di quelle dichiarate | confermato |
| FR-3 | Fan-out | N elementi → N attività; 0 → join riceve `[]`; input singolo = elenco di 1 | confermato |
| FR-1 | `document.dsl` | 1.1.x | `[ASSUMPTION]` |
| NFR-16 | Schermi | da 360 px, touch, editor del grafo e SQL compresi | confermato |
| NFR-9 | Accessibilità | WCAG 2.1 AA | confermato |
| UJ-2 / UJ-3 | Casi d'uso guida | 300 rami paralleli + join; finestre di 5 min, soglia > 50 per ASN | UJ-3 `[ASSUMPTION]` |
| addendum | Avviso di scadenza della credenziale nel kit | evento ≥ 60 s prima e alla scadenza | proposta UX, non requisito |

NFR-14 — limiti **senza cifra** (valore dalla spec della storia), tutti con predefinito
documentato e limitato applicato dopo la validazione, e massimo fissato dall'installazione per
ogni tempo/limite dichiarabile dal tenant:

- tempi: retry, timeout del task e dell'esecuzione, limite di attesa prima della consegna (FR-8),
  intervallo di battito dei worker (FR-14), chiusura delle finestre di uno stream inattivo
  (FR-21), intervallo dei checkpoint, conservazione degli stream, recupero delle scadenze perse,
  inattività prima di fermare un processo ausiliario (FR-50);
- grandezze del tenant: elementi di un fan-out, dimensione del documento e della query, numero di
  task, profondità di annidamento e ricorsione dei sotto-workflow, dimensione della risposta di
  `call http`, dimensione/profondità/numero di campi di manifest e schemi, nome visualizzato
  (lunghezza e caratteri, FR-27);
- risorse: concorrenza di un worker, limiti dei nodi personalizzati (CPU, memoria, tempo), quote
  di un nuovo tenant, soglia "vicino al limite" (FR-49), funzioni e destinazioni di rete
  disponibili ai tenant, tetto dell'arretrato (FR-30);
- invariante: nessuna combinazione di predefiniti e valori dichiarati lascia un'esecuzione,
  un'attività o una finestra ferma senza fine; superare un limite = errore di validazione o task
  fallito con limite e valore, senza creare attività e senza fermare il motore.

Quote per tenant (FR-30): attività concorrenti, esecuzioni/minuto, risorse della sandbox (parte del
pool forte compresa), risorse delle pipeline stream (FR-25: numero attive, memoria di stato, CPU),
eventi scritti/s, richieste API/s, spazio (stream, storico, code, nodi caricati). Limiti per
esecuzione (FR-17): CPU, memoria, tempo per nodo personalizzato e per espressione jq del tenant;
CPU e memoria (stato + esecuzione) per query di pipeline stream.

## 2. Unità e confini

### 2.1 Unità di runtime implicate

| Unità | Cosa è (fonte) | Note portanti |
|---|---|---|
| Motore | servizio Rust a nodo singolo nella v1 (§1.1 p.3, §8.2, NFR-15) | API, validazione, scheduling DAG, consegna, retry/timeout, quote, cassaforte, trigger (inferenza: le fonti dicono solo "il processo che custodisce la chiave", FR-16, senza nominarlo; vedi AMB-11); scala aggiungendo worker; nessuna HA del centro nella v1 |
| Bus | servizio separato (FR-50: "due servizi, topictoast e il bus") | trasporta attività ed eventi; il DB tiene solo lo stato (§1); Iggy come vincolo di partenza (§4.11), alternative nello spike (§3 M-11) |
| Database | SQLite incorporato **oppure** Postgres, a scelta dell'integratore (FR-50, §4.11) | solo stato; ≤ 5 istruzioni/attività; Postgres = servizio di rete → cifrato (NFR-8) |
| Worker | processi dell'integratore sempre attivi, senza avvio a freddo; registrano funzioni con manifest (§4.3, FR-12, FR-55) | SDK TypeScript e Rust; distribuzione tra worker della stessa funzione; HA dei worker già data (FR-12, FR-14) |
| Sandbox leggera | processo ausiliario predefinito, senza KVM né root, **fuori dal processo che custodisce la chiave** (FR-16, FR-50) | esegue nodi personalizzati precompilati TS/Rust; mai due tenant nella stessa istanza/processo/memoria, né insieme né in sequenza |
| Sandbox forte | microVM opzionale dove c'è KVM, pool già avviato, monouso o riservata a un tenant (FR-54) | RAM del pool fuori da NFR-1; senza KVM si ripiega sulla leggera con avviso |
| Altri processi ausiliari | se l'architettura li separa: jq, query delle pipeline stream, `call http` (FR-50) | avviati/fermati dal motore; arresto per inattività e riavvio a freddo `[ASSUMPTION]`; NFR-1 conta tutti quelli attivi |
| Runtime delle pipeline stream | query SQL continue con stato, finestre, watermark, checkpoint, modello Arroyo (§4.5) | dentro il motore o processo ausiliario: da decidere; checkpoint: dove, da decidere (§11 n.4) |
| Kit frontend | libreria JS: **nucleo JS** senza interfaccia + **componenti React** (area workflow, elenco, editor, pianificazioni, esecuzioni, segreti, editor delle pipeline stream) (§3, §4.9) | gira nel browser della piattaforma ospite, parla con il motore con la credenziale firmata; editor costruito nel kit |
| Dashboard | interfaccia web dell'operatore, distribuita con topictoast (§4.10) | accesso con credenziale generata al primo avvio (FR-45); dati che il motore deve esporre: esecuzioni di tutti i tenant con tempo in coda (FR-46), pipeline stream con causa, ritardo sul watermark, eventi/s, memoria di stato, eventi in ritardo scartati, ultimo checkpoint (FR-47, stesse grandezze delle metriche FR-52), worker con funzioni, attività in corso e cronologia delle fasi (FR-48), uso delle quote (FR-49) |
| CLI | costruita sull'SDK (§3) | carica nodi personalizzati (FR-15) |
| Piattaforma ospite | app dell'integratore (esterna) | autentica gli utenti e firma credenziali a scadenza breve (FR-27) |
| Benchmark | nel repository, un comando (FR-53) | produce tutte le misure del §1 |
| Fuori v1 | RustFS (payload grandi, §8.2), HA del centro, cluster del bus | — |

### 2.2 Cosa attraversa i confini

- Browser (kit) → motore: chiamate API e eventi in tempo reale con credenziale firmata (tenant,
  ruolo, nome visualizzato facoltativo, tenant scelto per l'integratore) (FR-27); documenti
  workflow, avvii con chiave di idempotenza (FR-41), segreti in sola scrittura (FR-32), query
  SQL. Cifrato, server autenticato (NFR-8).
- SDK/CLI → motore: workflow come codice o documento OWS a runtime (FR-35, FR-36), amministrazione
  di tenant/quote/segreti/pipeline stream/pianificazioni (FR-38, FR-57), nodi compilati con
  manifest (FR-15), eventi sugli stream (FR-18).
- Worker ↔ motore (protocollo pubblico, §6): registrazione funzioni + manifest + concorrenza
  dichiarata (FR-12, FR-13, FR-55); consegna attività con input, segreti dichiarati, contesto
  (proprietario, esecuzione, tentativo) (FR-37); battito (FR-14); risultati/errori; richiesta di
  arresto all'annullamento (FR-10); scritture sugli stream del proprietario dell'attività (FR-26).
  Cifrato; in chiaro solo da localhost (NFR-8).
- Motore ↔ bus: attività ed eventi; nessun segreto in chiaro su nessun supporto (FR-34).
- Motore ↔ DB: solo stato (NFR-4, SM-2).
- Motore ↔ sandbox: byte del nodo (validazione/compilazione/istanziazione fuori dal processo della
  chiave), input, solo i segreti dichiarati (FR-16, FR-33).
- Motore/sandbox → Internet: `call http` e rete dei nodi solo verso destinazioni che l'integratore
  consente a quel tenant (il nodo può solo restringerle); mai loopback/privati/link-local/installazione (FR-16).
- Pipeline stream → motore: righe verso uno stream del tenant o un avvio di workflow per riga
  (FR-24).
- Motore → esterno: metriche e log strutturati in "formati standard" (FR-52; formato non nominato).

### 2.3 Superfici pubbliche = contratti (§6)

API del motore; protocollo worker ↔ motore (documentato per SDK di terzi); SDK TypeScript e Rust;
CLI; formato dei nodi personalizzati compilati; formato del manifest (FR-55); nucleo JS; componenti
React (proprietà ed eventi); formato dei workflow (sottoinsieme OWS + estensione DAG); dialetto SQL
delle pipeline stream; formato di configurazione; formato della credenziale firmata (FR-27);
metriche. Semver: in 0.y.z una rottura alza la minor, dopo solo la major, con via di migrazione.
Un workflow accettato resta eseguibile dalle versioni successive della stessa major.

## 3. Meccanismi che l'architettura deve decidere

Una riga per decisione, voci uguali da più fonti unite. "T-n" = meccanismo suggerito dai
revisori in triage.md, richiamato dall'addendum: input, non decisione.

### 3.1 Formato e modello dei workflow

- M-01 · Forma dell'estensione DAG — §4.1, §10.4, FR-2, FR-3. Esito: dipendenze arbitrarie
  (forma a N) e fan-out dinamico con join. Candidati: dentro `metadata` (documento OWS resta
  valido) o chiavi nuove (non più valido). Prima: verifica di `dsl.md` e `sdk-rust` di OWS.
- M-02 · Mappatura nel formato OWS — FR-1 (note), FR-56 (note), FR-29, §11 n.7, RU-4. Esito: come
  un task invoca funzione o nodo personalizzato, come dichiara segreti/connessioni (anche task
  senza manifest, es. `call http`, FR-29), come il workflow dichiara lo schema di input.
  Verificare se `input` di OWS 1.1 porta già uno schema e se ammette schemi esterni (vietati da
  FR-55).
- M-03 · Propagazione di fallimenti e salti tra dipendenze DAG e incrocio con `then`, `switch`,
  `try` — FR-2, FR-3, §3 Stati, §11 n.8, T-30. Esito: nessun task in attesa senza fine; stato
  terminale per regola documentata; da chiudere prima delle storie.
- M-04 · Pianificazioni nel formato e nel salvataggio — FR-6, FR-57, §11 n.12, addendum. Esito:
  dove la pianificazione del documento porta il proprio input; come si salvano pianificazione del
  documento (trigger del proprietario del workflow) e pianificazione del kit (trigger del tenant).
- M-05 · Linguaggio degli schemi di manifest e input — FR-55, FR-56, §11 n.16, EU-M6. Esito:
  linguaggio chiuso (campi, tipi, obbligatori, elenchi, oggetti annidati), intero vs decimale,
  predefiniti, campi in più, `null` vs assente, **una sola** regola di compatibilità comune a
  motore, nucleo JS ed editor (parità FR-39); nessun riferimento esterno; limiti NFR-14.
  Candidati: sottoinsieme chiuso di JSON Schema senza riferimenti esterni; JSON Type Definition
  (RFC 8927).
- M-06 · Forma del campo di estensione del manifest (formato pubblico) — FR-55, §3 Manifest, RU-9.
- M-07 · Derivazione del manifest dai tipi della funzione in TypeScript e in Rust, e
  dichiarazione degli altri campi — FR-37, FR-55, addendum.
- M-08 · Manifest conteso: quale vale senza worker collegati (la palette lo mostra) e con due
  worker che registrano la stessa funzione con manifest diversi — §11 n.10, FR-12, FR-55, FR-48.
- M-09 · Effetti di un cambio di manifest (funzione o nodo) o dello schema di input di un
  sotto-workflow su workflow salvati, esecuzioni in corso, attività in coda — §11 n.11, FR-33
  (connessioni già fissate nella versione pubblicata).
- M-10 · Controllo di input e output a ogni attività sul manifest: dove gira e quanto costa,
  misurato nelle corse NFR-2/NFR-3 — FR-55, FR-53, addendum.

### 3.2 Bus, persistenza, esecuzione

- M-11 · Scelta del bus (spike) — §4.11, §10.3, FR-9, FR-13, FR-18, FR-22, FR-30, FR-51, NFR-6,
  T-31, addendum. Esito: log durevole (NFR-6) con ack, retry, timeout, ritardi costruiti se è un
  log e non una coda; nessun blocco di partizione che porti attività di altri tenant oltre NFR-2;
  aggiornamento senza perdite nonostante il formato su disco che cambia (Iggy pre-1.0); NFR-4 e
  SM-2; verifica della maturità del cluster (§8.2). Candidati: Iggy (vincolo di partenza),
  Valkey Streams (consumer group, `XACK`/`XPENDING`/`XAUTOCLAIM`, AOF `appendfsync always`, tutto
  in RAM), NATS JetStream, log incorporato in Rust. Stima addendum: coda attività → Valkey pari o
  meglio sul p99; stream ad alto volume → Iggy; con conservazione breve Valkey regge anche gli
  stream.
- M-12 · Livello caldo Valkey + scarico su Postgres/SQLite (proposta di Marco, addendum). Esito
  richiesto se adottato: durabilità sull'AOF con fsync, non sullo scarico; scarico a gruppi e
  idempotente con chiave naturale per evento ed esecuzione; eliminazione da Valkey solo dopo il
  commit; letture dello storico su entrambi i livelli; tabelle tipizzate; benchmark che conta le
  istruzioni dello scarico.
- M-13 · Trasporto motore ↔ worker — addendum, NFR-8, §6. Candidati: protocollo standard su TLS
  (gRPC o HTTP/2; **raccomandato** per la v1: SDK di terzi semplici, niente UDP né relay) oppure
  iroh (QUIC p2p per chiave pubblica, 1.0 del 2026-06-15, MIT/Apache-2.0, `@number0/iroh`) se
  servono NAT o worker presso i tenant. Varianti: (A) iroh come trasporto + log; (B) iroh + log
  incorporato al posto di Iggy (un servizio in meno). p2p tra worker solo per payload grandi, dopo
  la v1.
- M-14 · Accesso dei worker al bus: diretto o solo tramite il motore — FR-12, T-1. Raccomandazione
  addendum: bus privato dietro al motore, mai raggiunto dai worker (segreti, quote,
  contropressione restano al motore).
- M-15 · Credenziali dei worker — FR-12, FR-26, T-1, T-9. Esito: credenziale di worker distinta da
  quelle dei tenant. Suggeriti: una per worker, revocabile una per una, eventualmente limitata a
  funzioni o tenant; credenziale legata all'attività per le scritture dei worker sugli stream.
- M-16 · Protocollo dei worker: identificativo di tentativo; risultato di un tentativo non più
  corrente rifiutato e registrato — FR-7, FR-14, T-35 (parte architettura).
- M-17 · Versioni e aggiornamenti del protocollo — §6, FR-13, FR-51, triage tema 4. Esito: firme
  di funzione durante l'aggiornamento dei worker, concorrenza dichiarata 0 o enorme, versione del
  protocollo, 0.y.z vs "stessa major", procedura di aggiornamento del bus.
- M-18 · Modalità di scrittura (sincrona, a gruppi) che soddisfa "accettato = al sicuro quando
  parte la risposta", dichiarata nel benchmark — FR-9, NFR-4, NFR-6, T-14.
- M-19 · Due orologi per attività (attesa prima della consegna, esecuzione) se la spec di FR-8
  sceglie un timeout di attesa separato — FR-8, T-13 (condizionale).
- M-20 · Registro delle scadenze e delle righe rifiutate: dove vive; chiave per pianificazione e
  scadenza che garantisca una sola esecuzione per scadenza anche a cavallo di fermi — FR-6, FR-24,
  EU-M1, addendum.
- M-21 · Chiave di idempotenza riga → avvio per le pipeline stream (esattamente un avvio per riga
  dopo un riavvio dal checkpoint; righe uguali in finestre diverse = due avvii) — FR-24, T-15.
  Suggerita: pipeline stream + versione + finestra o posizione nello stream.
- M-22 · Concorrenza sulle pubblicazioni e identità delle versioni — FR-5, FR-15, T-16. Suggeriti:
  controllo ottimistico sulla versione di base; versione del nodo = impronta del contenuto.
- M-23 · Quote e storico delle esecuzioni dell'integratore: quale quota le limita, dove le vede
  l'operatore — §11 n.19 (architettura + spec), FR-30, FR-46.
- M-24 · Misura dello spazio occupato per tenant sul bus (quota di spazio, tetto dell'arretrato) —
  FR-30, T-12.

### 3.3 Sandbox e processi ausiliari

- M-25 · Tecnologie dei due livelli di sandbox (spike prima dello sprint 1) — §4.4, §10.1, FR-15,
  FR-16, FR-54, NFR-12, §11 n.3. Esito: leggera senza KVM né root; forte microVM con pool; TS e
  Rust compilati girano nella leggera; conferma dei numeri di NFR-12; test con codice ostile noto
  (NFR-8). Candidati: non nominati nelle fonti del PRD; rimando a
  `_bmad-output/planning-artifacts/briefs/brief-topictoast-2026-11-18/addendum.md`.
- M-26 · Nessuna convivenza tra tenant nella sandbox leggera — FR-16, T-7. Suggeriti: istanze
  monouso o un processo per tenant; costo in RAM e avvio misurato nello spike.
- M-27 · Dettagli della sandbox — triage tema 3, FR-15, FR-17, FR-54. Esito: formato del modulo
  compilato (TypeScript su entrambi i livelli; superficie pubblica §6), declassamento della
  forte, parte del pool per tenant e pulizia tra attività dello stesso tenant, limite del nodo
  rispetto al timeout del task.
- M-28 · Processi ausiliari — FR-50, §11 n.9, T-27, addendum. Esito: quali processi separare
  (sandbox leggera obbligatoria; jq, query delle pipeline stream, `call http` facoltativi);
  arresto per inattività e riavvio a freddo `[ASSUMPTION]`, quali tenere caldi; costo del freddo
  rispetto a NFR-2/NFR-12; obiettivo del riavvio a freddo dopo lo spike; NFR-1 conta ogni processo
  attivo.
- M-29 · Valutazione di jq dei tenant — FR-16, FR-17, T-5. Esito: nessun accesso ad ambiente, file,
  moduli, rete; limiti CPU/memoria/tempo. Suggeriti: interprete senza ambiente né file, oppure
  valutazione fuori dal processo della chiave; validazione e compilazione dei moduli fuori dal
  processo della chiave.
- M-30 · Filtro della rete in uscita — FR-16, T-4. Esito: solo destinazioni consentite al tenant;
  loopback, privati, link-local e indirizzi dell'installazione irraggiungibili anche via DNS o
  redirect. Suggeriti: controllo dell'IP risolto a ogni connessione e a ogni redirect (DNS
  rebinding); `call http` con lo stesso filtro dei nodi, dentro o fuori dal processo della chiave.

### 3.4 Pipeline stream

- M-31 · Tecnologia delle pipeline stream (spike prima dello sprint 1) — §4.5, §10.2, FR-19…FR-25,
  §11 n.2, addendum. Esito: stato, finestre fisse e scorrevoli, watermark, checkpoint,
  rielaborazione deterministica (FR-19), quote CPU e memoria per pipeline stream (FR-25);
  obiettivo NFR-5 e memoria di stato attesa per pipeline stream. Candidati: riuso di Arroyo
  (Apache-2.0/MIT, Cloudflare dal 2025-04-10, distribuito come server, uso come libreria non
  verificato), DataFusion + stato costruito in topictoast (DataFusion non ha finestre, watermark
  né checkpoint), altro.
- M-32 · Dove salvare i checkpoint nella v1 (RustFS arriva dopo) — FR-22, §11 n.4, addendum.
- M-33 · Dialetto SQL chiuso dei tenant — FR-19, T-6. Esito: rifiuto di DDL, tabelle esterne,
  connettori, file, rete, UDF. Suggeriti: lista bianca di funzioni sul motore scelto; funzioni non
  deterministiche rifiutate o legate al tempo dell'evento; nessun accesso a cataloghi, sorgenti e
  destinazioni del motore sottostante.
- M-34 · Chiusura delle finestre su stream inattivo — FR-21, T-17. Suggerito: avanzamento del
  watermark con l'orologio di sistema.
- M-35 · Identificativo facoltativo dell'evento e deduplicazione all'ingresso — FR-18, T-36 (parte
  architettura).

### 3.5 Segreti, identità, sicurezza

- M-36 · Come si salvano segreti e connessioni — §11 n.6, FR-32, FR-55. Lato prodotto chiuso: il
  segreto dichiara la connessione che soddisfa.
- M-37 · Risoluzione e forma delle connessioni — FR-29, FR-33, §3 Connessione, EU-M10, addendum.
  Esito: quando si risolvono le connessioni di un sotto-workflow dell'integratore chiamato da un
  workflow del tenant; forma dei nomi di piattaforma; due connessioni della stessa piattaforma in
  un manifest. Richiesta della revisione di sicurezza (addendum, non requisito PRD): con più segreti sulla stessa connessione la scelta non è mai
  implicita (§11 n.14, valore alla spec).
- M-38 · Segreti sul bus — FR-33, FR-34, T-8. Suggeriti: risolti alla consegna e mai scritti nel
  messaggio, oppure cifrati dentro il messaggio.
- M-39 · Eliminazione del tenant su ogni supporto (DB, bus, checkpoint, nodi, sandbox riservate) —
  FR-31, T-19. Suggeriti: stream separati per tenant oppure cancellazione crittografica (una
  chiave per tenant).
- M-40 · TLS su ogni connessione con dati sensibili — NFR-8, T-2. Suggerito: certificati generati
  al primo avvio o forniti dall'integratore, entro NFR-13.
- M-41 · Chiavi e credenziali senza valore predefinito noto — FR-50, FR-45, T-10. Suggeriti:
  generazione casuale al primo avvio con valori mostrati una volta, oppure rifiuto di partire
  finché non forniti; rifiuto dei valori presenti negli esempi; guida rapida entro NFR-13.
- M-42 · Formato della credenziale firmata (superficie pubblica) — FR-27, §6, addendum. Esito:
  forma e nomi dei campi (tenant, ruolo, nome visualizzato, tenant scelto), come nomina il tenant
  predefinito.
- M-43 · Distinguere la credenziale dell'integratore firmata per il kit da quella con cui
  l'integratore usa l'SDK — FR-28, NFR-8, SU-A2, addendum.
- M-44 · Pagina d'accesso della dashboard che dice dove è stata stampata la credenziale senza
  rivelare percorsi dell'host — FR-45, SU-B4, EU-B7 (T-63 UX).

### 3.6 API del kit frontend

- M-45 · Scadenza della credenziale nel kit: valori, nomi degli eventi verso la piattaforma ospite,
  API — FR-27, addendum (proposta UX: evento ≥ 60 s prima e alla scadenza; lavoro non salvato solo
  in memoria).
- M-46 · Controlli del kit: API delle coppie di contrasto (errore visibile in sviluppo, avviso in
  console in produzione) e proprietà di opt-in del marchio; verifica con ricerca di "topictoast"
  nel bundle — FR-44, NFR-9, addendum.
- M-47 · Chiave di idempotenza dei lanci dal kit: nome e API nel contratto del kit — FR-41, FR-6,
  EU-M5, addendum.

### 3.7 Trasversali

- M-48 · Librerie candidate di Marco da raccogliere all'inizio dell'architettura — addendum.
- M-49 · Benchmark di base negli spike: dice se NFR-1…NFR-4 reggono — §10.6, SM-3.

## 4. Modello multi-tenant e sicurezza

Invarianti da imporre per struttura (non per disciplina dei singoli handler).

### 4.1 Proprietà e contesto

- INV-01 · Ogni risorsa (workflow, esecuzione, attività, nodo personalizzato, stream, pipeline
  stream, segreto, trigger) ha un solo proprietario: un tenant o l'integratore (§4.6).
- INV-02 · Tenant opzionali, un solo comportamento: il tenant predefinito esiste dal primo avvio;
  segreti, quote e storico sono per tenant anche con il solo predefinito; il secondo tenant attiva
  la separazione senza cambiare il motore (§4.6, memlog 74). Identificativo di tenant eliminato mai
  riusato (FR-31).
- INV-03 · Tenant e permessi di una richiesta derivano **solo** dalla credenziale verificata, mai da
  percorso, corpo o filtro; credenziale di A con id di B → agisce su A o rifiuto (FR-26).
- INV-04 · Una risorsa di un altro tenant risponde come inesistente; liste, ricerche, conteggi ed
  eventi in tempo reale non contengono mai risorse altrui (FR-26). Stessa regola per stream
  (FR-19, FR-26) e per funzioni non rese disponibili al tenant (FR-4, nessun dettaglio del
  manifest).
- INV-05 · L'esecuzione appartiene a chi la avvia; ogni trigger ha un proprietario fissato alla
  creazione e avvia solo nel suo contesto. La pianificazione del documento → proprietario del
  workflow (integratore per un workflow dell'integratore, con i suoi segreti, invisibile ai
  tenant). Un workflow di un tenant gira sempre nel contesto del tenant, anche avviato
  dall'integratore; un'esecuzione dell'integratore esegue solo workflow dell'integratore
  (FR-6, FR-26, FR-29, memlog 80).
- INV-06 · Risoluzione dei riferimenti: funzioni, nodi, sotto-workflow tra le risorse del
  proprietario del workflow (+ quelle che l'integratore rende disponibili al tenant); segreti dal
  proprietario dell'esecuzione; mai tra le risorse di un altro tenant. Omonimi creati dal tenant
  non cambiano ciò che esegue un workflow dell'integratore (FR-26, FR-29).
- INV-07 · Un workflow scrive solo sugli stream del proprietario dell'esecuzione; un worker, mentre
  serve un'attività, solo sugli stream del proprietario dell'attività; scrittura altrui = errore di
  stream inesistente, nessun evento consegnato (FR-26). Eventi di uno stream leggibili solo dalle
  pipeline stream del suo tenant (FR-18).

### 4.2 Ruoli e credenziali

- INV-08 · Due ruoli: integratore (con l'operatore) vede tutti i tenant; tenant vede solo sé. Solo
  l'integratore crea tenant e quote (FR-28). Utenti e permessi interni al tenant: fuori
  (piattaforma ospite).
- INV-09 · Credenziale firmata dalla piattaforma ospite, a scadenza breve: tenant, ruolo, nome
  visualizzato facoltativo; con l'integratore, il tenant scelto è un campo firmato (nuova firma a
  ogni cambio). Rifiutate: senza identità valida, scaduta, ruolo tenant con campo "tenant scelto",
  tenant scelto inesistente o eliminato, nome visualizzato fuori limiti (FR-27).
- INV-10 · Credenziale dell'integratore firmata per il kit: con tenant scelto vale solo per le
  risorse di quel tenant, lettura dei workflow dell'integratore, elenco/ricerca dei tenant; senza
  tenant scelto solo elenco/ricerca. Ogni altra operazione riservata → rifiuto (FR-28). Nel kit
  non si crea un workflow dell'integratore.
- INV-11 · Worker: credenziale di worker valida, distinta da quelle dei tenant, per collegarsi,
  registrare funzioni e ricevere attività; senza, nessuna attività né segreto. Chi raggiunge il bus
  senza credenziale valida non legge né scrive attività ed eventi (FR-12). Proprietario esposto
  dall'SDK solo dal contesto dell'esecuzione, mai dall'input (FR-37).
- INV-12 · Dashboard: credenziale generata al primo avvio, nessuna password predefinita; senza
  credenziale nessun dato; alla scadenza torna all'accesso; azioni che cambiano stato accettate
  solo dalla stessa origine (FR-45, FR-46).

### 4.3 Segreti e cassaforte

- INV-13 · Valore in sola scrittura: dopo il salvataggio nessuna API, componente o dashboard lo
  legge; visibili solo nome, connessione, data, ultimo uso (FR-32, FR-42).
- INV-14 · Consegna solo all'attività, solo i segreti dichiarati dal suo task, solo del
  proprietario dell'esecuzione; connessioni fissate nella versione pubblicata; un manifest
  registrato dopo non aggiunge connessioni né segreti; segreto eliminato → attività fallita alla
  consegna (FR-33).
- INV-15 · Mai in chiaro su alcun supporto persistente (DB, bus, checkpoint, log, file
  temporanei); ricerca di un segreto sentinella nelle directory dati di topictoast, bus e
  checkpoint = nulla. Chiave di cifratura non nel DB né nelle sue copie, non raggiungibile dai nodi;
  rotazione senza fermo e senza perdite, dopo la quale la chiave vecchia non decifra nulla
  (FR-34). Rotazione della chiave di firma delle identità: dopo la v1 `[ASSUMPTION]`.
- INV-16 · Segreti, credenziali, nomi visualizzati, input e output delle attività mai nei log
  (FR-52); credenziale del primo avvio mai nei log (FR-45); segreti mai in storico, output, errori
  (FR-33).

### 4.4 Sandbox, codice e risorse dei tenant

- INV-17 · Sandbox, a entrambi i livelli, fuori dal processo che custodisce la chiave; anche il
  trattamento dei byte di un nodo (validazione, compilazione, istanziazione, manifest) avviene
  fuori da quel processo: un modulo ostile non ferma motore né esecuzioni (FR-16).
- INV-18 · Un'istanza della sandbox, il suo processo e la sua memoria non contengono mai dati o
  segreti di due tenant, né insieme né in sequenza; la sandbox forte è monouso o riservata a un
  tenant (FR-16, FR-54).
- INV-19 · Il nodo vede solo il proprio input e i segreti dichiarati; nessun accesso a rete
  interna, file system dell'host, dati altrui (FR-16).
- INV-20 · Rete in uscita per conto di un tenant (nodo o `call http`) solo verso destinazioni
  consentite dall'integratore a quel tenant; il nodo può restringere, mai allargare; loopback,
  privati, link-local, motore/bus/DB irraggiungibili anche via DNS o redirect; violazione = errore
  di politica senza aprire la connessione (FR-16).
- INV-21 · Limiti per esecuzione (FR-17); superarli fa fallire l'attività o rallenta/sospende la
  pipeline stream con errore visibile; il processo del motore non si ferma; nessun processo né
  memoria residui. jq senza `$ENV`, `env`, `input_filename`, `import`, `include` (rifiuto alla
  validazione).
- INV-22 · Dialetto SQL dei tenant a lista chiusa; query che citano stream altrui = stream
  inesistente; cicli tra pipeline stream dello stesso tenant rifiutati (FR-19).
- INV-23 · Manifest e schemi senza riferimenti esterni: rifiutati senza aprire connessioni di rete
  (FR-55).
- INV-24 · Non interferenza: con un tenant oltre quota, un worker lento, un nodo che satura i
  limiti, una pipeline stream al limite, schemi e input ostili al massimo consentito, le attività
  degli altri tenant restano entro NFR-2 e le loro pipeline stream senza ritardo crescente sul
  watermark (FR-13, FR-17, FR-25, FR-30, FR-55). Soglia = NFR-2 `[ASSUMPTION]`.
- INV-25 · Quote: oltre quota le nuove attività restano in coda; mai perso lavoro accettato;
  oltre il tetto dell'arretrato o lo spazio → errore di quota **prima** dell'accettazione, la
  pipeline stream smette di emettere con errore visibile (FR-30, FR-24).

### 4.5 Confini e trasporto

- INV-26 · Ogni input esterno validato (API, SDK, documenti, manifest, query, codice dei nodi):
  errore di validazione, mai crash (NFR-7, FR-4).
- INV-27 · Contenuto dei tenant (nomi, descrizioni, errori, input, output, righe emesse, campi e
  metadati dei manifest, nomi visualizzati) mostrato sempre come testo in dashboard e kit; test con
  payload di script su ogni campo, ogni pagina, entrambi i ruoli (NFR-7). Suggerito (T-11): CSP e
  nessun HTML grezzo, secondo la skill `dashboard-react`.
- INV-28 · Ogni connessione con segreti, credenziali o dati dei tenant (worker↔motore,
  motore↔bus, motore↔DB di rete, browser/SDK/CLI→motore) cifrata con server autenticato; in chiaro
  solo da localhost; predefinito: ascolto solo su localhost; worker in chiaro da altro host
  rifiutato (NFR-8, FR-50).
- INV-29 · Nessuna chiave o credenziale con valore predefinito noto: due installazioni hanno valori
  diversi; avvio rifiutato con valori degli esempi della documentazione (FR-50).
- INV-30 · Eliminazione del tenant: prima ferma esecuzioni, pipeline stream e pianificazioni, poi
  rende i dati illeggibili in ogni supporto entro il tempo massimo; ricerca di un valore
  sentinella nelle directory dati di topictoast e bus = nulla (FR-31). Nome visualizzato
  conservato quanto il record e cancellato con lui o col tenant (FR-27).
- INV-31 · Kit: al cambio di tenant scarta lo stato precedente, eventi in tempo reale compresi;
  lavoro non salvato solo nella memoria della pagina, mai nel browser (FR-27). Con ruolo tenant
  nessun identificativo di worker (FR-41).
- INV-32 · Verifica (NFR-8, prima del primo rilascio pubblico): test della sandbox con codice
  ostile noto a entrambi i livelli; test automatico per ogni tipo di risorsa × punto di accesso
  (API, SDK, eventi in tempo reale, liste, conteggi) + rifiuti di ruolo; i casi citati in FR-12,
  FR-16, FR-19, FR-26, FR-27, FR-28, FR-32, FR-33, FR-55 entrano nel test.

## 5. Stato ed esecuzione

### 5.1 Stati (elenco chiuso, §3)

- Esecuzione: in coda, in esecuzione, completata, fallita, annullata, scaduta. "In coda" =
  accettata e nessun task partito (un `wait` in corso conta come partito).
- Task: in attesa (dipendenze non finite), in coda (pronto, non consegnato, sempre con motivo), in
  esecuzione, ritentato, completato, fallito, saltato (con motivo), annullato.
- Attività: in coda, ricevuta, in esecuzione, ritentata, completata, fallita, annullata.
- Worker: collegato, non risponde. Pipeline stream: attiva, rallentata, sospesa (causa: CPU,
  memoria di stato o di esecuzione, arretrato). Tenant: vicino al limite, al limite.
  Pianificazione: attiva, sospesa.
- Timeout = motivo, non stato. "Scaduta" solo per l'esecuzione; in esecuzione annullata o scaduta
  i task non partiti → annullato. Propagazione di fallimenti e salti: M-03.
- Tempo in coda (FR-46) = età del più vecchio tra task e attività in coda dell'esecuzione, anche se
  già "in esecuzione".

### 5.2 Consegna, retry, timeout, durabilità

- Consegna ai worker **almeno una volta**; un task con completamento registrato non viene mai
  rieseguito; un'attività consegnata senza esito registrato può essere riconsegnata (FR-9).
  Idempotenza delle funzioni: documentata per l'integratore.
- Retry per task: tentativi, attesa, crescita; ogni tentativo è un'attività distinta nello
  storico; esauriti → task fallito con l'errore dell'ultimo tentativo (FR-7).
- Timeout per task (dalla consegna al worker o alla sandbox) e per esecuzione (→ scaduta, attività
  in corso annullate). L'attesa prima della consegna (nessun worker, oltre quota, pool vuoto) non
  consuma tentativi né fa scattare il timeout del task, ma è limitata (NFR-14); stato oltre il
  limite: §11 n.15 (FR-8).
- Contropressione: concorrenza dichiarata dal worker mai superata (FR-13); attività per funzioni
  senza worker restano in coda con il motivo e partono alla registrazione (FR-12, UJ-1).
- Worker che non risponde entro l'intervallo: attività riassegnate contando un tentativo (FR-14;
  conflitto con UJ-4, §7.4).
- Annullamento: nessuna nuova attività; richiesta di arresto a worker e sandbox; registra chi
  (ruolo + nome) e quando (FR-10). Gare con retry/completamento: spec (T-34).
- Durabilità: accettato solo se al sicuro quando parte la risposta; nessuna perdita per riavvio di
  motore o bus (FR-9, NFR-6). Ripresa dopo crash ≤ 30 s, worker si ricollegano, finiscono le
  attività, risultati intanto scritti non persi; durante la ripresa pausa, non perdita (NFR-15).
  Il kit dice la pausa in modo neutro (FR-41, FR-43); la dashboard distingue la ripresa dal disco pieno (FR-48).
- Disco pieno: rifiuto di lavoro nuovo, avviso in dashboard distinto dalla ripresa (NFR-6, FR-48).
- Aggiornamenti: migrazione automatica, esecuzioni riprendono (FR-51).

### 5.3 Dati e validazione

- Output di un task disponibile ai dipendenti (FR-11); limite 1 MB per input/output e per elemento
  del fan-out `[ASSUMPTION]`; join con tetto aggregato configurabile. Niente storage per payload
  grandi nella v1.
- Validazione all'invio con lo schema topictoast (FR-4): errore con percorso e motivo, mai
  salvataggio parziale; riferimenti inesistenti (funzioni, nodi, segreti, task); input di ogni
  task sul manifest; connessioni senza segreto nel workflow di un tenant; cicli DAG rifiutati con i
  task coinvolti (FR-2; `for` e `then` mantengono la semantica OWS).
- Validazione a runtime (FR-55): input dell'attività sullo schema di input → task fallito prima
  della consegna senza consumare tentativi; output sullo schema di output → attività fallita, mai
  consegnata a valle. Costo misurato in NFR-2/NFR-3.
- Avvio con input fuori dallo schema del workflow (FR-56) rifiutato prima dell'accettazione, con
  percorso e regola, mai il valore; senza schema accetta qualunque JSON; valore singolo per campo
  elenco = elenco di 1.
- Avvio con connessione senza segreto: rifiutato prima dell'accettazione (FR-29, FR-33).

### 5.4 Fan-out e DAG

- Dipendenze arbitrarie: un task parte quando tutte le dipendenze sono completate; task
  indipendenti in parallelo (FR-2).
- Fan-out dinamico su task o sotto-grafo: N attività, join con N risultati nell'ordine
  dell'elenco; parallelismo massimo dichiarabile, altrimenti quota del tenant; politica di errore
  dichiarata: fallisce tutto il fan-out oppure il join riceve l'errore al posto del risultato
  (FR-3).
- Sottoinsieme OWS (FR-1): do, set, switch, fork, for, try con retry, wait, raise, call http,
  invocazione di funzioni e nodi, sotto-workflow, jq, `schedule` cron o a intervallo. Fuori: gRPC,
  OpenAPI, AsyncAPI, A2A, `run`, `emit`, `listen`, CTK. Costrutto fuori sottoinsieme → rifiuto con
  costrutto e percorso.

### 5.5 Trigger, pianificazioni, idempotenza

- Trigger: API/SDK, pianificazione (del documento o del kit), riga di pipeline stream (FR-6). Ogni
  avvio restituisce subito l'id; origine registrata (con ruolo e nome visualizzato).
- Pianificazione: una esecuzione per ogni scadenza accettata, una sola anche se il motore si ferma
  o riparte mentre scatta; input proprio, validato al salvataggio e a ogni scadenza; scadenze
  rifiutate registrate con motivo, mai in silenzio; sospensione senza recupero delle scadenze
  cadute; dopo un fermo scadenze perse e recuperate visibili (FR-6). Recupero, sovrapposizione,
  fuso orario, intervallo minimo: spec (T-32).
- Idempotenza: chiave per tenant, valida almeno quanto la conservazione dell'esecuzione, sicura in
  concorrenza (FR-6); ogni lancio dal kit porta una chiave per intenzione (FR-41).

### 5.6 Sotto-workflow

- Nel sottoinsieme (FR-1); input controllato sullo schema del workflow chiamato (FR-4, FR-40);
  profondità di annidamento e ricorsione limitata (NFR-14); eliminazione di un workflow usato come
  sotto-workflow o come destinazione di una pipeline stream rifiutata con l'elenco (FR-58); effetti
  dei cambi di schema: M-09; connessioni di un sotto-workflow dell'integratore: M-37; versioni dei
  sotto-workflow: spec (T-16).

### 5.7 Stream e pipeline stream

- Eventi scritti da integratore, worker e workflow via API/SDK; senza tempo dell'evento →
  tempo di arrivo; nessun connettore esterno nella v1 `[ASSUMPTION]` (FR-18).
- Conservazione breve per durata o dimensione; gli stream non sono un archivio (FR-18, §7).
- Query SQL validata al salvataggio con riga e colonna (FR-19); finestre fisse e scorrevoli
  `[ASSUMPTION]`, deduplicazione per chiave e finestra, filtri (FR-20).
- Watermark: finestra chiusa quando il watermark supera fine + ritardo tollerato; eventi in ritardo
  scartati e contati; eventi oltre la tolleranza nel futuro non spostano il watermark, contati;
  finestre di uno stream inattivo chiuse entro un tempo documentato (FR-21).
- Checkpoint periodici; dopo un riavvio si riparte dall'ultimo checkpoint e si rielaborano gli
  eventi successivi senza perdere eventi accettati (FR-22); rielaborazione deterministica (FR-19).
- Ciclo di vita: attiva, sospendi (nessuna CPU; alla ripresa elabora gli eventi ancora conservati
  e segnala quelli usciti dalla conservazione), modifica (nuova versione con stato vuoto
  `[ASSUMPTION]`; punto di partenza: spec, T-36), elimina (FR-23).
- Destinazioni: stream del tenant oppure un avvio di workflow per riga, esattamente uno anche dopo
  il riavvio dal checkpoint; righe trattenute oltre quota entro il tetto, non perse (FR-24).

### 5.8 Versioni

- Workflow: nuova versione pubblicabile con esecuzioni in corso; le esecuzioni in corso finiscono
  sulla loro versione; le nuove usano l'ultima salvo richiesta esplicita; pubblicazione su base non
  più ultima → rifiuto con la versione corrente; dall'editor salvare = pubblicare (FR-5); salvare un
  cambio dello schema di input avvisa delle pianificazioni e pipeline stream che lo avviano.
- Workflow come codice: documento OWS equivalente, in sola lettura nell'editor per tutti (FR-35).
  Documento a runtime eseguibile appena l'invio risponde (FR-36).
- Nodo personalizzato: stesso nome = nuova versione; l'esecuzione in corso resta sulla sua;
  ogni attività registra la versione eseguita (FR-15).
- Pipeline stream: modifica = nuova versione (FR-23). Formato dei workflow: compatibilità nella
  stessa major (§6).

### 5.9 Conservazione ed eliminazione

- Storico per tenant con conservazione configurabile (30 giorni `[ASSUMPTION]`), input e output
  eliminati con l'esecuzione (FR-31).
- Eliminazione di un workflow: esecuzioni in corso finiscono, pianificazioni eliminate, storico
  resta (FR-58).

## 6. Già deciso

| Scelta | Stato | Fonte |
|---|---|---|
| Motore in Rust, open source Apache-2.0, nessuna parte a pagamento, dipendenze compatibili | deciso | §1, NFR-11, §7 |
| Formato: OWS 1.1, sottoinsieme non conforme + estensione DAG; nessuna garanzia su altri runtime | deciso (forma dell'estensione: M-01) | FR-1, §1.1, §7 |
| Attività sul bus, DB solo stato | deciso | §1 |
| Database: SQLite incorporato oppure Postgres, a scelta dell'integratore | deciso | FR-50, §4.11, §5 |
| Bus: servizio separato; Iggy come vincolo di partenza "da confermare in architettura"; Valkey, NATS JetStream, log incorporato, iroh: candidati | **non deciso**, solo default | §4.11, memlog 40, 46, addendum |
| Trasporto worker: protocollo standard su TLS (gRPC/HTTP/2); bus privato dietro al motore | **raccomandazione**, non decisione | addendum |
| Conservazione degli eventi degli stream: pochi minuti; gli stream non sono un archivio | deciso | memlog 43, FR-18, §7 |
| Motore a nodo singolo nella v1; HA del centro dopo la v1; worker già in HA; ripresa ≤ 30 s | deciso | §8.2, NFR-15, memlog 47-48 |
| Worker = processi dell'integratore sempre attivi che registrano funzioni; nessun avvio a freddo per attività | deciso | §4.3, memlog 17 |
| SDK TypeScript e Rust; Python e Go dopo la v1 `[ASSUMPTION]`; protocollo documentato per SDK di terzi | deciso | §4.8, §6, §8.2 |
| Nodi personalizzati TypeScript e Rust **precompilati**, caricati via SDK o CLI; nessun compilatore sul server; Python e Go dopo la v1 | deciso | FR-15, memlog 22 |
| Sandbox su due livelli: leggera predefinita (no KVM, no root, processo separato dalla chiave); forte microVM opzionale con pool, monouso o per tenant; tecnologie dallo spike | deciso (tecnologie: M-25) | FR-16, FR-54, memlog 23 |
| Pipeline stream nella v1, scritte anche dai tenant, modello Arroyo; tecnologia dallo spike | deciso (tecnologia: M-31) | §4.5, memlog 14, 18, 19 |
| Identità: la piattaforma ospite firma credenziali a scadenza breve; topictoast non autentica gli utenti; due ruoli | deciso (formato: M-42) | FR-27, FR-28, memlog 26 |
| Tenant opzionali con tenant predefinito; termine "tenant" | deciso | §4.6, memlog 72-76 |
| `schedule` nel documento OWS accettato, trigger del proprietario del workflow; pianificazione del kit trigger del tenant | deciso | FR-1, FR-6, FR-57, memlog 75 |
| Workflow come codice in sola lettura nell'editor | deciso | FR-35, memlog 54 |
| Workflow dell'integratore disponibili a tutti i tenant; creati solo dall'SDK | deciso | FR-28, FR-29, memlog 60, triage-update SU-M7 |
| Esecuzione di un workflow di un tenant sempre nel suo contesto; esecuzione dell'integratore solo su workflow dell'integratore | deciso | FR-26, memlog 80 |
| Cassaforte con sole chiavi API nella v1; OAuth dopo; rotazione della chiave della cassaforte nella v1; rotazione della chiave di firma dopo `[ASSUMPTION]` | deciso | §4.7, FR-34, §8.2, memlog 28 |
| Destinazioni di rete per tenant (non per connessione) nella v1 | deciso | FR-16, §8.2, memlog 67 |
| Editor costruito nel kit (Workflow Builder solo riferimento); librerie `@xyflow/react` e dagre: **candidate** | deciso / candidate | §4.9, memlog 34 |
| Kit in due strati: nucleo JS + componenti React; italiano e inglese; nessun marchio senza opt-in | deciso | §3, FR-39, FR-44 |
| Installazione: due servizi (topictoast + bus) + DB; processi ausiliari gestiti dal motore; niente Kubernetes, KVM, root nel predefinito | deciso | FR-50, §7 |
| Storage per payload grandi (RustFS) dopo la v1 | deciso | §4.11, §8.2 |
| Backup/ripristino coerenti fuori dalla v1, limite documentato | deciso | NFR-6, §8.2 |
| Piattaforme Linux x86_64 e arm64 | `[ASSUMPTION]` | NFR-10 |
| Nessun taglio della v1 se uno spike fallisce: si cambia tecnologia e il rilascio slitta | deciso | §10.5, memlog 30 |
| Formato di metriche e log: "formati standard", tecnologia non nominata | non deciso | FR-52 |

### 6.1 Costituzione: dove incrocia il PRD (vincolante, non dal PRD)

- I: nessun codice di prodotto prima dell'architettura accettata; ogni decisione con bilancio
  funzioni/costi/manutenibilità e alternativa scartata (ADR).
- III: dominio esagonale puro, senza I/O né runtime async (test `purity.rs`); una sola composition
  root; Gateway vs Repository. Incrocia bus, DB, sandbox, trasporto worker (tutti adattatori).
- IV: validazione serde ai confini; errori degli adattatori tradotti; client HTTP in un solo modulo
  (`clippy.toml`) → incrocia `call http` e filtro di rete (M-30).
- V: colonne tipizzate e tabelle figlie per i dati che il sistema interpreta, niente `jsonb`/`TEXT
  JSON` strutturato; utente DB non superuser e senza `BYPASSRLS`. **Tensione da risolvere**:
  documenti OWS, manifest e schemi, input/output delle attività (JSON fino a 1 MB), stato e
  checkpoint delle pipeline stream; SQLite non ha RLS.
- VI: macro come candidate dove snelliscono (dichiarazioni al posto del cablaggio).
- VII: nessun Python nel prodotto.
- VIII: identificatori in inglese secondo `docs/glossary.md`; rustdoc in italiano.

## 7. Assunzioni e domande aperte

### 7.1 `[ASSUMPTION]` (PRD §12) con proprietario

Proprietari da memlog 35, 62, 68.

| Assunzione | Proprietario | Tocca l'architettura |
|---|---|---|
| NFR-5 ≥ 100.000 eventi/s | spike pipeline stream (§11 n.2) | sì (M-31) |
| NFR-12 avvio ≤ 10 ms, ≤ 64 MB | spike sandbox (§11 n.3) | sì (M-25) |
| FR-50 processi ausiliari a freddo; NFR-1 conta tutti i processi attivi | architettura (§11 n.9) | sì (M-28) |
| FR-13 soglia di non interferenza = NFR-2 (anche FR-17, FR-25, FR-30) | accettata, non confermata (memlog 38) | sì (INV-24) |
| FR-11 limite 1 MB | spec della storia (T-33) | sì: dimensiona bus e DB |
| FR-22 checkpoint ogni 10 s | spec della storia | sì (M-32) |
| FR-31 storico 30 giorni | spec della storia | sì: volume del DB |
| FR-23 modifica della query = nuova versione con stato vuoto | accettata | sì (M-31) |
| FR-20 finestre fisse e scorrevoli, sessione dopo | accettata | sì (M-31) |
| FR-18 nessun connettore esterno per gli stream | accettata | marginale |
| FR-1 `document.dsl` 1.1.x | accettata | sì (M-01, M-02) |
| UJ-3 esempio di pipeline stream (ASN, 5 min) | Marco, prima dello spike delle pipeline stream (§11 n.1) | sì: input dello spike |
| NFR-10 Linux x86_64 e arm64 | accettata | sì: build, sandbox |
| §8.2 rotazione della chiave di firma delle identità dopo la v1 | accettata | sì (M-42) |
| §8.2 SDK Python e Go dopo la v1 | accettata | no |
| NFR-13 ≤ 15 minuti; SM-5 tre persone | accettata | indiretto (M-40, M-41) |
| flowtoast, progetto separato | Marco (§11 n.5) | no |

### 7.2 PRD §11: stato e proprietario

| §11 | Tema | Proprietario | Rimando |
|---|---|---|---|
| n.1 | caso reale di pipeline stream (UJ-3) | Marco | 7.1 |
| n.2 | obiettivo NFR-5, memoria di stato | spike | M-31 |
| n.3 | conferma NFR-12 | spike | M-25 |
| n.4 | dove salvare i checkpoint | architettura | M-32 |
| n.5 | nome e confini di flowtoast | Marco | — |
| n.6 | come si salvano segreti e connessioni | architettura | M-36 |
| n.7 | invocazione, segreti, schema di input nel formato OWS | architettura | M-02 |
| n.8 | propagazione di fallimenti e salti | architettura, prima delle storie | M-03 |
| n.9 | obiettivo del riavvio a freddo | spike | M-28 |
| n.10 | manifest senza worker o conteso | architettura | M-08 |
| n.11 | cambio di manifest o schema di sotto-workflow | architettura | M-09 |
| n.12 | input della pianificazione del documento nel formato | architettura | M-04 |
| n.13 | riga fuori schema | chiusa (rifiuto registrato) | M-20 |
| n.14 | più segreti sulla stessa connessione | spec; mai implicito | M-37 |
| n.15 | stato oltre il limite di attesa + valore | spec | M-19 |
| n.16 | linguaggio degli schemi | architettura | M-05 |
| n.17 | scadenza rifiutata: ritentare, sospensione automatica, dove si vede | spec | M-20 |
| n.18 | chi sospende la pianificazione del documento di un workflow condiviso | spec | — |
| n.19 | quote e storico delle esecuzioni dell'integratore | architettura + spec | M-23 |

### 7.3 Rinvii alle spec delle storie che toccano l'architettura

- T-32 pianificazioni durante un fermo (recupero, sovrapposizione, fuso orario, intervallo minimo).
- T-33 definizione esatta del limite di 1 MB (byte del JSON, metadati, errore non ritentabile).
- T-34 annullamento: stato terminale unico, idempotente, risultato tardivo ignorato e registrato.
- T-35 (parte spec) la riassegnazione consuma o no un tentativo; EU-M3 nuovi stati del worker.
- T-36 (parte spec) da quale punto dello stream parte la nuova versione di una query.
- T-16 (parte spec) ripubblicazione identica, versioni dei sotto-workflow, nodo in uso eliminato.
- Triage tema 5: destinazione eliminata, checkpoint che fallisce e suo carico sul DB, dimensioni
  delle finestre, pipeline stream dell'integratore su più tenant, modifica che perde le finestre.
- EU-A4: se un output non conforme si ritenta.

Voci superate (non usare): memlog 66 e triage-update RU-13 ("`schedule` nel documento
rifiutato") → superate da memlog 75 e FR-1; memlog 20-21 (terminologia "task/s", conteggi FR/NFR
vecchi); memlog 38 su SM-2 `[ASSUMPTION]` → confermato in memlog 49; "cliente" in memlog e triage =
"tenant".

### 7.4 Ambiguità nelle fonti

- AMB-1 · FR-14 "riassegnate contando un tentativo" contro UJ-4 ("già riassegnate"): con un solo
  tentativo il task fallisce invece di passare a un altro worker (T-35).
- AMB-2 · FR-50 "due servizi, topictoast e il bus" contro la variante B dell'addendum (log
  incorporato, un servizio in meno): adottarla cambia FR-50.
- AMB-3 · FR-12 "chi raggiunge il bus senza credenziale valida" presuppone un bus raggiungibile;
  T-1 lascia aperto l'accesso diretto, l'addendum raccomanda il bus privato (M-14).
- AMB-4 · NFR-1 misurato "con SQLite", mentre l'introduzione del §5 chiede tutte le misure con
  SQLite e con Postgres: non è detto se con Postgres valga un obiettivo di RAM né se l'RSS di
  Postgres conti.
- AMB-5 · Disponibilità delle funzioni ai tenant: FR-4 "l'integratore dichiara quali funzioni
  sono invocabili dai workflow dei tenant", FR-26/FR-32 "che l'integratore gli rende disponibili":
  granularità (globale o per tenant) non detta.
- AMB-6 · Pipeline stream dell'integratore: FR-23 dice che l'integratore le crea, FR-18 che un
  evento di uno stream di un tenant è leggibile solo dalle pipeline stream di quel tenant, il
  glossario definisce lo stream come "di un tenant": esistenza di stream dell'integratore e
  lettura tra tenant non definite (triage tema 5).
- AMB-7 · §4.11 "Iggy da confermare in architettura" e memlog 46 "Iggy resta il vincolo del PRD":
  trattato come default da battere nello spike, non come decisione.
- AMB-8 · NFR-2/NFR-3 misurati con worker sulla stessa macchina: nessun obiettivo con worker
  remoti su TLS (NFR-8).
- AMB-9 · Idempotenza "dallo stesso tenant" (FR-6): per le esecuzioni dell'integratore il campo
  della chiave non è definito (legato a §11 n.19).
- AMB-10 · FR-21 "ritardo tollerato" e "tolleranza nel futuro" e FR-18 conservazione "pochi
  minuti": i massimi installazione (NFR-14) determinano il dimensionamento del bus, ma nessuna
  cifra è data.
- AMB-11 · Quale processo custodisce la chiave della cassaforte: FR-16 e FR-34 parlano solo del
  "processo che custodisce la chiave" e chiedono che sandbox, byte dei nodi (e, se separati, jq e
  `call http`) ne stiano fuori; che sia il motore è un'inferenza, non scritto.
