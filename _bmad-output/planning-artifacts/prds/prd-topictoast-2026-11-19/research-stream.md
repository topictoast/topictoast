# Ricerca: stream processor SQL integrato in un workflow engine

Data: 2026-11-19. Ogni fatto ha fonte; senza fonte primaria: "non verificato". Fonti secondarie segnalate.

## 1. DataFusion: stato dello streaming oggi
- Le sorgenti non limitate (unbounded) sono supportate: si definiscono tabelle in modalita'
unbounded e i risultati si consumano in modo incrementale con `execute_stream()`. Operatori
come `SymmetricHashJoinExec` e `AggregationExec` girano gia' in modalita' unbounded, **senza
watermark**. Fonte: https://github.com/apache/datafusion/issues/11404 (discussione aperta
2024-07-11, ultimo commento 2026-08-11).
- Mancano, upstream: watermark, checkpoint dello stato, backend di stato collegabile,
`StreamingWindowExec` per aggregazioni su dati non limitati (4 sotto-issue collegate). Stessa
fonte, stato "aperto".
- Posizione dei maintainer: le funzioni generali vanno upstream, watermark e checkpoint (specifici
dello stream processing) restano a valle nei progetti derivati. Fonte: stessa issue #11404.
- Novita' recente: epic #24265 "Better support / documentation / tests for Streaming SQL
Engines", aperta da Andrew Lamb il 2026-08-11, senza assegnatari ne' milestone. Contenuto:
documentare l'uso streaming, test di bassa latenza, impostazioni di flush (dimensione o scadenza,
eventualmente watermark/checkpoint barrier), operatori streaming separati, un possibile
crate `datafusion-streaming`. Finestre e watermark NON sono tra le voci pianificate. Fonte:
https://github.com/apache/datafusion/issues/24265.
- Ultima release vista: DataFusion 54.0.0 (fonte secondaria, ricerca
web; numero e data esatta non verificati); 52.0.0 del 2026-01-12
(https://datafusion.apache.org/blog/2026/01/12/datafusion-52.0.0/).
- Finestre temporali (TUMBLE/HOP/SESSION) non sono nel core: arrivano da crate
esterni (es. `laminar-sql`, fonte secondaria su docs.rs) o dai motori derivati (Arroyo,
Denormalized). Verifica diretta del changelog di `StreamingTable` nella doc ufficiale: non
verificato (la pagina docs.rs provata ha dato 404).
- Sintesi: DataFusion e' un ottimo motore di query embeddabile con esecuzione push/incrementale,
ma NON un stream processor completo. Stato, finestre e watermark vanno costruiti sopra.

## 2. Stream processor SQL in Rust
- **Arroyo.** Licenza Apache-2.0 e MIT (duale). "Distributed stream processing engine
in Rust", un solo binario avviabile come cluster, Docker, manifest Kubernetes. Fonte:
https://github.com/ArroyoSystems/arroyo (letta 2026-11-19). Acquisito da Cloudflare,
annunciato 2025-04-10: confluisce in Cloudflare Pipelines (beta, ingestione in R2, ~100.000
record/s per pipeline); l'annuncio non dice nulla sul futuro dell'open source. Fonte:
https://blog.cloudflare.com/cloudflare-acquires-arroyo-pipelines-streaming-ingestion-beta/.
Attivita': ultimo push 2026-11-12, 5.077 stelle (fonte secondaria:
https://www.pistack.xyz/posts/2026-11-13-streaming-sql-engines-arroyo-feldera-risingwave-2026/).
Ultima release taggata: v0.15.0, 2025-12-01 (fonte secondaria dev.co, non verificato su
GitHub). Embeddabile come libreria: non indicato, appare solo server/cluster. RAM: nessun dato
pubblicato; confronto terzo dice "comodo in una VM da 4 GB" (pistack, stessa fonte, nessuna misura).
- **Denormalized.** Apache-2.0. "Embeddable stream processing engine based on Apache DataFusion",
singolo nodo, API Rust e Python, finestre e join, solo Kafka come sorgente/sink, checkpoint
con SlateDB; README: "work-in-progress", cerca design partner. 380 stelle, 172 commit, nessuna
release su GitHub. Fonte: https://github.com/probably-nothing-labs/denormalized. Lancio Show HN
2024-08. Dato recente sulla manutenzione: non verificato. E' l'unico caso trovato di embeddabile
+ DataFusion + finestre, ma immaturo.
- **RisingWave.** Apache-2.0, "event streaming platform", database di streaming con wire protocol
PostgreSQL, stato su object storage (Hummock); si installa come server (Docker, Kubernetes,
cloud). Fonte: https://github.com/risingwavelabs/risingwave. v3.0.0 citata da fonte secondaria
(pistack). 9.411 stelle. Embeddabile: no, solo server (la pagina non descrive un uso come
libreria). RAM: nessuna cifra pubblicata verificata.
- **Feldera.** Fonti discordanti sulla licenza: la doc ufficiale dice MIT, la pistack
dice Apache-2.0 (controllare il file LICENSE). Motore incrementale DBSP in Rust, SQL
compilato in Rust via Calcite, storage che va oltre la RAM (dichiarato 25-75% meno memoria
su query pesanti, claim del vendor). Fonti: https://www.feldera.com/blog/feldera-storage,
https://docs.feldera.com/sql/intro/. Uso come libreria: DBSP e' una libreria Rust, ma un percorso
supportato "SQL embedded" non e' verificato. Consigliato 8 GB (pistack).
- **Materialize.** Business Source License (non open source): Community Edition gratuita con
chiave obbligatoria da v26.0.0 (novembre 2025), limite 24 GiB RAM e 48 GiB disco. Fonte:
https://materialize.com/changelog/2025-11-18-v26-release/. Solo server; incompatibile con
l'obiettivo Apache-2.0 embeddabile.
- **Fluvio / SDF (InfinyOn).** Fluvio e' Apache; SDF (Stateful DataFlow) e' in beta e la
licenza community e' "single use" (un solo worker SDF su cluster Fluvio self-hosted); tier a
pagamento da 100 $/mese. Fonti secondarie/mirror: https://infinyon-website.pages.dev/pricing,
https://fluvio-docs.pages.dev/sdf. Stato 2026: non verificato. Server, non embeddabile.
- **Streamling (Goldsky).** Open source annunciato 2026-06-08; runtime Rust su Arrow
e DataFusion, pipeline in YAML, plugin Rust/WASM. Niente SQL stateful (niente join
in streaming), finestre non citate; benchmark Kafka-to-ClickHouse: CPU da oltre 10
core a 0,3 core rispetto a Flink (claim del vendor), avvio sotto 5 s. Licenza non
indicata nel post. Fonte: https://goldsky.com/blog/open-sourcing-streamling; repo
https://github.com/goldsky-io/streamling. Embeddabile come libreria: non verificato.
- **Altri nominati nei risultati, non approfonditi:** LaminarDB (`laminar-sql`), StreamHouse
(https://streamhouse.vercel.app/blog/sql-on-streams-datafusion). Maturita' non verificata.
- Memoria: nessun progetto pubblica una cifra misurata di RAM minima. Il solo dato e' il
confronto terzo "circa 2 GB per una pipeline piccola" (pistack).

## 3. Apache Iggy
- Connectors runtime: plugin come librerie condivise (`.so`/`.dylib`/`.dll`)
che implementano i trait `Sink`/`Source`, caricati all'avvio, un runtime Tokio
per plugin. Fonti: https://iggy.apache.org/blogs/2025/06/06/connectors-runtime,
https://iggy.apache.org/docs/connectors/introduction.
- Catalogo (docs, 2026): 6 sorgenti (Elasticsearch, HTTP, InfluxDB, PostgreSQL, Random, MQTT),
17 sink (BigQuery, ClickHouse, Delta Lake, Doris, Elasticsearch, HTTP, Iceberg, InfluxDB,
Meilisearch, MongoDB, PostgreSQL, Quickwit, RabbitMQ, Redshift, S3, Stdout, SurrealDB), 8 transform
(add/delete/filter/update fields, proto/flatbuffer/avro convert, unwrap_envelope). Fonte:
https://iggy.apache.org/docs/connectors/introduction. Immagine `apache/iggy-connect:0.6.0`,
server 0.10.0 nella quick start.
- Stream processing: release 0.6.0 (2025-12-09) elenca un "Flink processor", cioe' integrazione
con Apache Flink. Fonte: https://iggy.apache.org/blogs/2025/12/09/release-0.6.0. Una proposta
su GitHub (mirror dev list) discute Flink/Spark come processor, solo proposta. Fonte:
https://mail-archive.com/dev@iggy.apache.org/msg00239.html.
- SQL o DataFusion in Iggy: **nessuna menzione** nelle pagine connectors ne' nelle note
0.6.0. Niente SQL nativo. I transform sono per-messaggio, senza stato ne' finestre.
- Stato del progetto: top-level project ASF dal 2026-08-19 (fonte secondaria,
https://www.thedroptimes.com/node/72445 per 0.9.0 con clustering VSR; data TLP da hpcwire
2026-08-27). Verifica sul sito ASF: non verificato.
- Conseguenza: un processore SQL su Iggy sarebbe un componente nuovo, non esiste un'integrazione
da riusare.

## 4. Orchestratori con stream o trigger realtime
- **Kestra.** Realtime Trigger: thread sempre attivo, ogni evento avvia un'esecuzione;
sorgenti Kafka, Pulsar, SQS, GCP Pub/Sub, Azure Event Hubs. Nessuna menzione di
aggregazione, finestre o SQL sugli stream, ne' di batching e limiti di throughput. Fonte:
https://kestra.io/docs/how-to-guides/realtime-triggers. Disponibile dalla 0.17.0
(https://kestra.io/plugins/plugin-kafka/io.kestra.plugin.kafka.RealtimeTrigger).
- **Windmill.** Trigger Kafka (generale) e NATS (Enterprise self-hosted) che avviano script o
flow; preprocessor per rimodellare il messaggio; anche SQS, Pub/Sub, MQTT, WebSocket, Postgres
changes. Fonte: https://www.windmill.dev/docs/triggers/kafka_triggers. Niente SQL continuo.
- **n8n.** Kafka Trigger nativo; un evento = un'esecuzione. Il
batching citato viene solo da pagine di terzi (non verificato). Fonte:
https://docs.n8n.io/integrations/builtin/trigger-nodes/n8n-nodes-base.kafkatrigger.
- **Temporal.** Nessuna integrazione Kafka ufficiale trovata; schema comune: un servizio
consuma Kafka e invia signal ai workflow. Discussioni della community segnalano il limite
di pochi signal al secondo per singolo workflow e il problema dell'allineamento degli
offset. Fonti (forum, non ufficiali): https://community.temporal.io/t/kafka-orchestration/3570,
https://community.temporal.io/t/thoughts-on-implementing-kafka-exactly-once-messaging-integration/13495.
- **Hatchet.** Trigger da eventi con chiave e filtri (wildcard dalla 0.65.0); lo "streaming"
di Hatchet e' l'invio dello stato dei passi al client, non stream processing di dati. Fonti:
https://docs.hatchet.run/v1/events, https://docs.hatchet.run/home/features/streaming. Se un
evento arriva prima della registrazione del task, non parte nulla.
- **Prodotti che uniscono le due cose** (tutti orientati ai dati, non ai
workflow generici): TDengine (stream engine SQL con trigger integrati), Databend
Cloud Task Flow (pipeline SQL con trigger da CDC), Databricks Lakeflow. Fonti
secondarie: https://tdengine.com/stream-processing-engine-sql-triggers-and-ha/,
https://daily.dev/posts/your-sql-deserves-a-better-scheduler-introducing-task-flow-in-databend-cloud-vgrzbqvdq,
https://www.databricks.com/en/blog/introducing-databricks-lakeflow.
- **Risposta alla domanda chiave:** tra gli orchestratori generalisti cercati (Kestra, Windmill,
n8n, Temporal, Hatchet) nessuno offre DAG + SQL continuo su stream in un solo prodotto: tutti
lavorano "un evento, un'esecuzione". Non ho trovato controesempi, ma la ricerca non e' esaustiva
(Airflow, Prefect, Dagster, Argo, Conductor, Tray, Workato non verificati).
- Un fornitore (Tinybird) sostiene che gli orchestratori coordinano job ma
non elaborano stream e vanno affiancati a un motore: opinione di parte. Fonte:
https://www.tinybird.co/blog/big-data-workflow-automation.

## 5. Casi d'uso per un workflow engine con SQL sugli stream
- Filtrare prima di avviare: evitare un'esecuzione per ogni evento scartabile; l'esecuzione
per evento di Kestra/n8n/Windmill e' costosa a volumi alti (inferenza dal modello, non misurata).
- Deduplicare indicatori (stesso IOC da piu' feed) con chiave e finestra temporale; un esempio
di dedup per hash del contenuto compare in articoli su pipeline di feed (fonte secondaria,
https://cloudstreet-dev.github.io/The-Open-Source-Intelligence-Bible/chapters/chapter-23).
- Aggregare e soglie: "piu' di N osservazioni in T minuti" prima di aprire un workflow di risposta.
- Arricchire e correlare in volo prima di SIEM/SOAR: pratica descritta per Kafka + Flink
(https://www.helpnetsecurity.com/2026/03/12/soc-primes-detectflow-enterprise-moves-threat-detection-to-the-data-ingestion-layer/,
2026-03-12; SOC Prime usa Flink per regole Sigma su Kafka, "decine di migliaia" di
regole, nessuna cifra eventi/s). Serie di Kai Waehner su Kafka per cybersecurity:
https://www.kai-waehner.de/blog/2021/07/15/kafka-cybersecurity-siem-soar-part-3-of-6-cyber-threat-intelligence/amp.
- Non ho trovato un pattern documentato per far partire playbook SOAR da un'uscita di stream
processor (topic, soglie): non verificato.

## Note per la decisione di prodotto
- Differenziante: apparentemente si, nessun orchestratore generalista noto combina DAG e SQL
continuo; il rischio e' che il mercato non lo chieda (da validare).
- Rischio di perimetro: DataFusion non fornisce stato, finestre e watermark; costruirli e'
un progetto da solo. Cloudflare e Denormalized mostrano che e' lavoro da team dedicato.
- Opzioni embeddabili: DataFusion (libreria, Apache-2.0) + stato proprio; Denormalized (Apache-2.0
ma immaturo, solo Kafka); DBSP/Feldera come libreria (non verificato). Server-only: Arroyo,
RisingWave, Materialize (BSL), SDF.
