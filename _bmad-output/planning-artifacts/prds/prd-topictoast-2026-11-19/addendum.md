---
title: topictoast — addendum al PRD
created: 2026-11-19
updated: 2026-11-20
---

# Addendum

Materiale emerso durante il PRD che serve ai documenti successivi (architettura, UX) e non entra nel
PRD.

## Librerie candidate (2026-11-19)

- Marco ha già alcune librerie candidate; vanno raccolte all'inizio dell'architettura.

## Modello dei worker e dello stream (2026-11-19)

- Worker: Marco li immagina come funzioni serverless sempre attive, in attesa di lavoro. Si
  registrano presso topictoast e ricevono i task; nessun avvio a freddo per task.
- Stream: modello Arroyo. Pipeline SQL continue, sempre attive, con stato, finestre temporali,
  watermark e checkpoint. Arroyo è costruito su DataFusion e Arrow.
- Marco cita DataFusion come riferimento: SQL eseguito su uno stream, per i carichi ad alta
  intensità. La tecnologia (DataFusion o altro) la sceglie l'architettura.
- Dalla ricerca (`research-stream.md`): Arroyo è Apache-2.0/MIT, acquisito da Cloudflare il
  2025-04-10, repo attivo, distribuito come server; un uso come libreria incorporata non è
  verificato. DataFusion da solo non ha finestre, watermark né checkpoint. La scelta tra riuso di
  Arroyo, DataFusion più stato costruito in topictoast, o altro spetta all'architettura, con uno
  spike.
- Domanda per l'architettura: dove vanno i checkpoint dello stato stream nella v1, visto che
  RustFS (object storage) è opzionale e arriva dopo.

## Meccanismi suggeriti dalla revisione (2026-11-19)

- Le voci «Per l'addendum» di `triage.md` (T-1…T-20) raccolgono i meccanismi proposti dai
  revisori per i requisiti di sicurezza del PRD: credenziale per worker revocabile, TLS con
  certificati generati al primo avvio, controllo dell'IP risolto a ogni connessione e redirect
  contro il DNS rebinding, jq valutato senza accesso ad ambiente e file, oppure fuori dal processo della chiave, lista bianca
  di funzioni SQL, segreti risolti alla consegna e mai scritti sul bus, cancellazione crittografica
  per tenant, controllo ottimistico delle versioni. Sono input per l'architettura, non decisioni.

## Processi ausiliari a freddo (2026-11-19)

- Proposta di Marco: i processi ausiliari poco usati (sandbox leggera, eventuali processi per jq,
  SQL e `call http`) vengono fermati dopo un periodo di inattività e riavviati a freddo alla prima
  richiesta, così a riposo restano solo motore e bus (NFR-1). Da studiare in architettura: costo
  dell'avvio a freddo rispetto a NFR-2 e NFR-12, e quali processi tenere sempre caldi.

## iroh al posto di Iggy? (2026-11-19)

- Domanda di Marco: usare iroh (connessioni p2p QUIC per chiave pubblica; 1.0 del 2026-06-15,
  MIT/Apache-2.0, binding Node.js `@number0/iroh`) invece di Iggy, con connessioni p2p tra worker.
- Analisi in chat: iroh è un trasporto, non un log. Senza un log durevole restano scoperti NFR-6,
  FR-9, FR-18, FR-22 e FR-51; mettere attività ed eventi nel database rompe NFR-4 e SM-2.
- Varianti da valutare in architettura con uno spike: (A) iroh come trasporto tra motore e worker,
  a stella, con il log che resta; (B) iroh come trasporto più un log incorporato in topictoast al
  posto di Iggy, un servizio in meno e più lavoro. Il p2p diretto tra worker solo per payload
  grandi, dopo la v1, insieme allo storage per payload grandi.
- Valkey come log (domanda di Marco): gli Streams con consumer group danno ack per messaggio,
  elenco dei pendenti e riconsegna (`XACK`, `XPENDING`, `XAUTOCLAIM`), quindi semantica di coda
  pronta e niente blocco di partizione; durabilità con AOF `appendfsync always`. Limiti: tutto il
  dato sta in RAM (conservazione degli stream e arretrato pesano sulla memoria), un servizio
  separato come Iggy, costo di fsync su NFR-2 e NFR-3 da misurare. Da confrontare nello stesso
  spike con NATS JetStream (code su disco, ack e riconsegna) e con un log incorporato in Rust.
- Proposta di Marco: dati caldi in Valkey finché servono, poi scaricati su Postgres (o SQLite) per
  la persistenza. Condizione: lo scarico serve a capacità e conservazione, non alla durabilità; il
  lavoro accettato resta protetto dall'AOF di Valkey con fsync, altrimenti uno spegnimento perde
  la finestra non ancora scaricata (NFR-6). Regole: scarico a gruppi e idempotente (chiave
  naturale per evento ed esecuzione), eliminazione da Valkey solo dopo il commit, letture dello
  storico che vedono entrambi i livelli, tabelle tipizzate (costituzione V). Per NFR-4 e SM-2 il
  benchmark conta anche le istruzioni dello scarico e riporta righe e WAL per attività, così il
  raggruppamento non abbellisce la misura.
- Valkey o Iggy, quale più veloce (stima, da misurare): sulla coda delle attività Valkey è pari o
  migliore, soprattutto sul p99 (nessun blocco di partizione, ack nativi); sugli stream di eventi ad
  alto volume Iggy è migliore (log sequenziale su disco, nessuna crescita della RAM, nessuno scarico
  su Postgres). Con fsync a ogni scrittura entrambi sono limitati dal disco. Domanda chiave per lo
  spike: quanto a lungo vanno conservati gli eventi degli stream; con conservazione breve Valkey
  regge anche gli stream e su Postgres scendono solo checkpoint e storico.
- Decisione di Marco: conservazione degli eventi degli stream di pochi minuti (finestra più
  ritardo tollerato più checkpoint). Rende Valkey candidato anche per gli stream, con un solo
  broker; su Postgres o SQLite scendono solo checkpoint e storico delle esecuzioni.
- Valkey più iroh o solo Valkey: in entrambi i casi Valkey resta privato, dietro al motore, mai
  raggiunto dai worker (consegna dei segreti alla consegna, quote, contropressione restano al
  motore). La scelta vera è il trasporto tra motore e worker: protocollo standard su TLS (gRPC o
  HTTP/2) oppure iroh. Raccomandazione per la v1: protocollo standard (SDK di terzi semplici,
  strumenti comuni, nessun problema con reti che bloccano UDP, nessun relay da gestire); iroh se i
  worker o il motore devono stare dietro NAT o presso i tenant, o con il p2p per payload grandi.

## Dopo la UX (2026-11-20)

Note emerse dall'aggiornamento del PRD dopo la UX (`change-plan-ux.md`): meccanismi da decidere in
architettura o nelle spec delle storie, più una decisione che le spine UX hanno recepito.

- Manifest delle funzioni tipizzate (FR-37, FR-55): l'SDK ricava dai tipi della funzione gli
  schemi di input e di output del manifest. Come li ricava in TypeScript e in Rust, e come si
  dichiarano gli altri campi: architettura.
- Credenziale firmata (FR-27): porta tenant, ruolo e un nome visualizzato facoltativo; con la
  credenziale dell'integratore anche il tenant scelto nel kit. Forma e nomi dei campi, e come la
  credenziale nomina il tenant predefinito (§4.6): architettura.
- Scadenza della credenziale nel kit (FR-27): la UX propone un evento verso la piattaforma ospite
  almeno 60 s prima della scadenza e uno alla scadenza, con il lavoro non salvato tenuto in
  memoria. Valori, nomi degli eventi e API del kit: architettura.
- Controlli del kit (FR-44): coppie di contrasto con errore visibile in sviluppo e avviso in
  console in produzione, dove vince la piattaforma ospite; marchio verificato con una ricerca di
  "topictoast" nel bundle e una proprietà di opt-in. API: architettura.
- Pianificazioni (FR-6, FR-57): OWS mette `schedule` nel documento, mentre un workflow
  dell'integratore è condiviso tra tenant. Deciso poi nel PRD: `schedule` nel documento è
  accettato (FR-1) ed è la pianificazione del workflow, un trigger del proprietario del workflow;
  le sue esecuzioni appartengono a lui, per un workflow dell'integratore all'integratore con i suoi
  segreti (FR-6, FR-26, FR-29). Il tenant aggiunge dal kit la propria pianificazione, con il
  proprio input (FR-57). Come si salvano: architettura.
- Workflow definito come codice (FR-35): in sola lettura nell'editor per tutti, per decisione di
  Marco; le spine UX, che lo rendevano modificabile con una versione a ogni salvataggio, sono
  allineate.
- Stato di un'attività rimasta in coda oltre il limite di attesa (FR-8): alla spec della storia di
  FR-8, insieme al valore del limite.

## Dopo la revisione dell'aggiornamento (2026-11-20)

Meccanismi e dettagli rinviati dalle tre revisioni dell'aggiornamento (`triage-update.md`).

- Linguaggio degli schemi (FR-55, FR-56, §11 n. 16): candidati un sottoinsieme chiuso di JSON
  Schema senza riferimenti esterni, oppure JSON Type Definition (RFC 8927). Da verificare sul DSL
  se l'`input` di OWS 1.1 porta già uno schema e se ammette uno schema esterno, che FR-55 esclude.
- Controllo di input e output a ogni attività (FR-55): dove gira e quanto costa, misurato nelle
  corse di NFR-2 e NFR-3 (FR-53): architettura. Se un output non conforme si ritenta: spec della
  storia.
- Registro delle scadenze e delle righe rifiutate (FR-6, FR-24): dove vive, e una chiave per
  pianificazione e scadenza che garantisca una sola esecuzione per scadenza: architettura.
- Connessioni (FR-29, FR-33, §3):
  - Quando si risolvono quelle di un sotto-workflow dell'integratore chiamato da un workflow del
    tenant, forma dei nomi di piattaforma e due connessioni della stessa piattaforma in un
    manifest: architettura.
  - Se la connessione di un segreto si cambia dopo il salvataggio, e se l'ultimo uso di un
    segreto indica workflow e task: spec della storia.
  - Per più segreti sulla stessa connessione (§11 n. 14) la revisione di sicurezza chiede che la
    scelta non sia mai implicita.
- Chiave di idempotenza dei lanci dal kit (FR-41): nome e API nel contratto del kit: architettura.
- Credenziale dell'integratore firmata per il kit (FR-28): come il motore la distingue da quella
  con cui l'integratore usa l'SDK: architettura.

## Dopo l'architettura (2026-11-23)

Le scelte rimandate all'architettura nelle sezioni sopra sono decise dalla spine accettata
(`docs/architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md`) o riassegnate da
lei (tabelle Deferred e Open Questions, spike S-1…S-4); in caso di dubbio vale la spine. Qui le
decise:

- Pipeline stream: SQL su DataFusion, con stato, finestre, watermark e checkpoint costruiti in
  topictoast, un processo ausiliario per tenant (AD-13). Arroyo resta solo il modello, mai un
  candidato.
- Linguaggio degli schemi: sottoinsieme chiuso di JSON Schema 2020-12, scritto come `input.schema`
  di OWS nel documento (AD-10, C-10).
- Estensione DAG: task `dag` di prima classe con `dependsOn` (AD-8); regola di propagazione in
  AD-9.
- Bus: servizio separato (AD-1); Iggy o Valkey lo sceglie lo spike del bus. iroh resta uno dei
  collegamenti dei worker da misurare nello stesso spike.
- Log dei task (FR-59, decisione di Marco del 2026-11-23): righe mai nel database (NFR-4),
  conservate nell'archivio dei payload (AD-17) e cifrate per tenant come gli altri dati dei tenant
  (AD-7); trasporto delle righe dei worker nel protocollo dei worker, misurato nello spike del bus;
  righe dei nodi raccolte dalla sandbox, mai nei log del motore (C-9). Il segnaposto dei segreti
  (confermato da Marco il 2026-11-23) si applica prima che la riga entri nel bus o nell'archivio,
  nell'SDK o nell'host della sandbox, perché i segreti non passano mai dal bus (C-12); lo spike del
  bus sceglie il trasporto delle righe di conseguenza.
