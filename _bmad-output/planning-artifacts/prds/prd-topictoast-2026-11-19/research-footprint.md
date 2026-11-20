# Ricerca: riferimenti pubblicati su RAM, latenza, throughput, carico DB

Data ricerca: 2026-11-19. Solo numeri letti su fonti pubbliche; nessuna stima propria.
Legenda: **[V]** = benchmark del vendor; **[T]** = terza parte/community; **[D]** = documentazione ufficiale (sizing, non benchmark); "n.v." = non verificato (nessuna fonte primaria trovata o pagina non leggibile).
Le pagine senza data esplicita riportano "senza data" e, se noto, la versione.

## 1. RAM a riposo / minima raccomandata

| Sistema | Valore | Tipo | Fonte (data) |
|---|---|---|---|
| Temporal dev server (`start-dev`) | server 112 MB + worker 35 MB = ~150 MB, senza esecuzioni | [T] misura di un utente | https://community.temporal.io/t/reduce-memory-footprint-of-temporal-process/9083 (2023-08-03) |
| Temporal server (carico medio-alto, "30/60 sts") | Frontend 4 GiB, History 6+ GiB, Matching 2 GiB, Worker 1 GiB (totale ~13 GiB) | [T] risposta staff in forum | https://community.temporal.io/t/baseline-server-resource-consumption/19267 (2026-02-16) |
| Restate | minimo "sviluppo/poco traffico" ~1,5 GiB totali (RocksDB 1 GiB, query engine 256 MiB, cache 128 MiB); default RocksDB 2 GiB; nessun minimo assoluto dichiarato | [D] | https://docs.restate.dev/server/memory (senza data) |
| Windmill (compose default, 8 container) | 633 MiB a riposo, 819 MiB picco con 66 job | n.v. (pagina Lumadock non leggibile, 403; valore dal solo snippet di ricerca) | https://lumadock.com/tutorials/self-host-windmill-docker-compose |
| Windmill worker | 1 worker per 1 vCPU e 1-2 GB RAM; "Small" 1 GB, "Standard" 2 GB | [D] | https://www.windmill.dev/docs/advanced/scaling (senza data) |
| Kestra standalone | minimo 4 GiB RAM, 2 vCPU | [D] | https://kestra.io/docs/administrator-guide/requirements (senza data) |
| Kestra per componente (fino a 1.000 task run/min) | webserver 1 CPU/2 GB; executor 2 CPU/2 GB; scheduler 1 CPU/2 GB; worker 2 CPU/4 GB | [D] | https://kestra.io/docs/performance/sizing-and-scaling-infrastructure (senza data) |
| n8n (container + Postgres, Docker) | n8n 300-350 MB a riposo (323 MB), Postgres 45-50 MB; n8n 2.34.6 | [T] | findstack.com/resources/n8n-4gb-vps-how-many-workflows (data non letta; valore da snippet, n.v. sulla pagina) |
| n8n (VPS 2 vCPU/8 GB, Docker) | 860 MB RAM senza workflow; queue mode: baseline doppia | [T] Hostinger | https://www.hostinger.com/tutorials/n8n-benchmark (2026-02-12) |
| Activepieces | worker 0,5 vCPU/1 GB (concorrenza 1); app 1 vCPU/1 GB; Postgres 2 vCPU/4 GB; Redis 1 vCPU/1 GB | [D] | https://www.activepieces.com/docs/install/configuration/overview (senza data) |
| Hatchet / hatchet-lite | nessun valore RAM/CPU pubblicato; lite "per sviluppo e basso volume" | [D] assenza | https://docs.hatchet.run/self-hosting/hatchet-lite |
| Inngest self-hosted | nessun valore ufficiale. Railway template: min 1 vCPU/1 GB, consigliati 2/2 GB | n.v. (non Inngest) | https://railway.com/deploy/inngest |
| Argo Workflows controller | nessun valore; memoria dominata dagli informer cache | [D] assenza | https://argo-workflows.readthedocs.io/en/latest/scaling/ |
| NATS JetStream | nessun valore ufficiale a riposo; ~6 MiB a riposo in Docker (blog); esempio doc `max_memory_store` 256MB | [T] / n.v. | https://dev.to/writingmuffin/message-broker-throughput-rabbitmq-vs-kafka-vs-nats-11hd ; https://docs.nats.io/running-a-nats-service/environment |
| Apache Iggy | nessun valore RAM pubblicato | n.v. | https://iggy.apache.org/docs/server/linux-tuning/ |

## 2. Latenza di avvio task / overhead di scheduling

| Sistema | Valore | Tipo | Fonte (data) |
|---|---|---|---|
| Kestra EE (RabbitMQ+Postgres), flusso 2 task | avg 50-69 ms, p99 60-107 ms da 1.000 a 7.000 exec/min (latenza di esecuzione, non solo start) | [V] | https://kestra.io/docs/performance/benchmark (Kestra 2.0.0, senza data) |
| Kestra OSS (Postgres), flusso 2 task | avg 111-150 ms, p99 151-370 ms fino a 4.000 exec/min; a 5.000 exec/min avg 10,8 s (saturazione) | [V] | idem |
| Windmill | overhead ~50 ms per job (worker standard), ~12 ms (worker dedicato) | [V] | https://www.windmill.dev/platform/workers (senza data) |
| Windmill vs Lambda, fib(10) | 54 ms (dedicato), 111 ms (normale), Lambda 73 ms; EC2 m4.large, 1 worker | [V] | https://windmill.dev/docs/misc/benchmarks/aws_lambda (senza data) |
| Hatchet v0.55.26 | esecuzione media ~40 ms a 100 run/s, ~48 ms a 500, ~220 ms a 2.000; scrittura media 2,5/2,6/5,7 ms | [V] | https://docs.hatchet.run/self-hosting/benchmarking (senza data) |
| Hatchet (release note) | coda media 30 ms -> 20 ms | [V] n.v. (da snippet) | newreleases.io/project/github/hatchet-dev/hatchet/release/v0.56.3 |
| Restate 1.2, workflow 3 step, carico basso | p50 15 ms, p90 42 ms; 1 step ad alto carico p50 16 ms, p90 25 ms | [V] | https://restate.dev/blog/announcing-restate-1.2 (2025-02-18) |
| Confronto Windmill (m4.large, 10 worker, 400 task leggeri) | attesa media: Temporal 2,1 s, Windmill 4,2 s, Hatchet 19,2 s; primo task: Temporal 0,108 s, Hatchet 0,723 s, Windmill 1,663 s | [V] concorrente, non neutrale | https://www.windmill.dev/docs/misc/benchmarks/competitors/results/go (2025; Windmill 1.483.1, Temporal 2.34.0, Hatchet 0.62.0, Kestra 0.22.3) |
| Temporal | target p95 StartWorkflowExecution < 150 ms; ottenuto < 50 ms; Schedule-to-Start > 150 ms prima di aggiungere poller | [V] | https://temporal.io/blog/scaling-temporal-the-basics (2023-05-23) |
| Inngest | tick di poll dell'executor 150 ms (default); nessuna latenza misurata | [D] | https://www.inngest.com/docs/self-hosting |
| n8n, Activepieces, Argo, NATS, Iggy (come task latency) | nessun p50/p99 di start task pubblicato; Iggy: p99 produttore 2,59 ms, consumatore 1,46 ms (messaggi, non task) | [V] | https://iggy.apache.org/blogs/2025/05/08/zero-copy-deserialization/ (2025-05-08, AWS i3en.3xlarge) |

## 3. Throughput e hardware

| Sistema | Valore | Hardware | Tipo | Fonte (data) |
|---|---|---|---|---|
| Hatchet (Postgres) | 2.000 run/s (DB CPU 83%, 800 IOPS); fino a 10k/s su m7g.8xlarge (senza dettagli) | RDS m7g.2xlarge 8 CPU; engine 2 repliche c7i.4xlarge | [V] | https://docs.hatchet.run/self-hosting/benchmarking (senza data) |
| Hatchet Postgres-only, istanza singola | "centinaia di task/s" | n.d. | [D] da snippet, n.v. sulla pagina | https://docs.hatchet.run/self-hosting/architecture |
| Hatchet con RabbitMQ | nessun benchmark separato pubblicato | - | n.v. | - |
| Kestra OSS (Postgres) | 4.000 exec/min = 8.000 task/min (~133 task/s), limite sostenuto; flusso da 10 task: 1.200 exec/min | n2-standard-4 (4 vCPU/16 GB) + Postgres 16 remoto 4 vCPU/16 GB | [V] | https://kestra.io/docs/performance/benchmark (Kestra 2.0.0) |
| Kestra EE | 7.000 exec/min = 14.000 task/min (~233 task/s), harness non saturato | n2-standard-4 + RabbitMQ e Postgres su VM separate | [V] | idem |
| Temporal | 150 -> 1.350 state transition/s dopo tuning | MySQL 4 core/32 GB; 4 servizi x 2 pod; History 2 core, 8 GB; 512 shard | [V] | https://temporal.io/blog/scaling-temporal-the-basics (2023-05-23) |
| Temporal (terzi) | ~1.000 st/s (512 shard), ~4.500 st/s (2.048 shard) | K8s, PostgreSQL | n.v. | markaicode.com/benchmarks/temporal-production-benchmark-latency/ |
| Windmill | 981 job/s con 100 worker su job da 100 ms; ~26M job/mese per worker | non dichiarato | [V] | https://www.windmill.dev/platform/workers |
| Restate 1.2 | 23.131 rps (1 step), 16.844 rps (3 step), 8.571 rps (9 step) | cluster 3 nodi, replica 3; CPU/RAM non dichiarati | [V] | https://restate.dev/blog/announcing-restate-1.2 (2025-02-18) |
| n8n | "fino a 220 esecuzioni/s" istanza singola (webhook + Edit Fields); multi: 7 istanze | ECS c5a.large, 4 GB, Postgres | [V] | https://docs.n8n.io/hosting/scaling/performance-benchmarking/ (senza data/versione) |
| Activepieces | nessun throughput; DB ~2,5 millicore per req/s (a 777 req/s: 2,7 core) | n.d. | [D] | pagina Activepieces sopra |
| Iggy | 2,4 GB/s scrittura, 4,0 GB/s lettura | AWS i3en.3xlarge | [V] | https://iggy.apache.org/blogs/2025/05/08/zero-copy-deserialization/ (2025-05-08) |
| NATS JetStream | nessun numero ufficiale; solo tool `nats bench` | - | n.v. | https://docs.nats.io/ |
| Argo Workflows | nessun benchmark; default client QPS 20, burst 30 | - | [D] | https://argo-workflows.readthedocs.io/en/latest/scaling/ |
| Inngest | nessun benchmark self-hosted | - | n.v. | - |

## 4. Carico DB per task/step

| Sistema | Dato | Fonte |
|---|---|---|
| Hatchet | a 100/500/2.000 run/s: DB CPU 15/60/83%, IOPS 400/600/800 (da cui ~4/1,2/0,4 IOPS per run; calcolo diretto dei valori pubblicati) | docs.hatchet.run/self-hosting/benchmarking |
| Kestra OSS | a 4.000 exec/min PG CPU 68,1% (4 vCPU); a 1.000 exec/min 26,5% | kestra.io/docs/performance/benchmark |
| Kestra | backend JDBC/Postgres adatto fino a ~1.000 task run/min; Kafka oltre ~2.000; sopra 1.000 task/min Postgres cresce di terabyte/anno | kestra.io/docs/performance/sizing-and-scaling-infrastructure |
| Temporal | misura in "state transition" = scritture su persistenza; ~60 st/s per vCPU Cassandra e ~150 per vCPU cluster (post community, non ufficiale); MySQL al ~79% CPU a 1.350 st/s | community.temporal.io/t/temporal-throughput/2263 ; blog 2023-05-23 |
| Activepieces | ~2,5 millicore Postgres per req/s | pagina Activepieces |
| Inngest | pool default 100 connessioni; ogni evento/step è una riga, nessuna pulizia automatica | https://www.inngest.com/docs/self-hosting |
| Windmill, Restate, Argo, n8n | query/scritture/WAL per task: non pubblicati (n.v.) | - |

## 5. Processi/servizi per un deployment minimo

| Sistema | Minimo | Fonte |
|---|---|---|
| Inngest | 1 processo (`inngest start`, SQLite + Redis in memoria); produzione: + Postgres + Redis | https://www.inngest.com/docs/self-hosting |
| Hatchet lite | 2 container (hatchet-lite + Postgres); con RabbitMQ 3; HA: grpc 4, controllers 2, scheduler 2 repliche + Postgres + RabbitMQ (3 repliche) | https://docs.hatchet.run/self-hosting/hatchet-lite ; /self-hosting/high-availability |
| Windmill | 3 componenti: Postgres, server, reverse proxy (+ worker) | https://www.windmill.dev/docs/advanced/self_host |
| Kestra | standalone 1 processo + Postgres; HA: 2 nodi x 4 componenti = 8 | kestra.io/docs/performance/sizing-and-scaling-infrastructure |
| Temporal | dev server 1 processo; produzione: 4 servizi (frontend, history, matching, worker) + DB | community.temporal.io/t/baseline-server-resource-consumption/19267 |
| Activepieces | app + worker + Postgres + Redis + object storage S3 | pagina Activepieces |
| Restate | 1 binario `restate-server` (n.v. sulla pagina letta; nota dai docs memoria) | docs.restate.dev/server/memory |
| n8n | 1 processo + DB; queue mode: main + worker + webhook + Redis | docs.n8n.io/hosting/scaling/performance-benchmarking/ |
| Argo | controller + argo-server + K8s (+ Postgres opzionale) | n.v. |
| NATS, Iggy | 1 binario ciascuno | n.v. (non verificato sulle pagine) |

## Note e lacune

- Valori credibili più bassi tra i concorrenti: RAM a riposo ~150 MB (Temporal dev) e ~323 MB (n8n); minimo Restate ~1,5 GiB; overhead di scheduling 12 ms (Windmill dedicato), p50 15 ms (Restate); throughput 2.000 run/s su Postgres (Hatchet) con DB CPU 83%.
- Quasi tutti i numeri sono [V]: pubblicati da chi vende il prodotto, su hardware e carichi diversi, non confrontabili tra loro. Il confronto di Windmill è l'unico multi-engine, ma il publisher è parte in causa e non dichiara il hardware per engine (solo m4.large).
- Nessun dato su WAL, query per task o scritture per task per nessun sistema; il più vicino è Hatchet (IOPS e CPU DB per run/s). Va misurato internamente.
- Non trovati: RAM a riposo per Hatchet, Inngest, Argo, NATS (ufficiale), Iggy; benchmark Argo e Inngest; Hatchet con RabbitMQ; hardware del cluster Restate.
- Le pagine Kestra, Hatchet, Windmill e Activepieces non riportano la data di pubblicazione; si indica la versione quando nota.
- Da misurare prima di fissare i target: idle RAM di topictoast, latenza start p50/p99 e task/s su una macchina di riferimento (es. 4 vCPU/16 GB come Kestra), scritture DB per task.
