---
title: topictoast — estratto di contesto per l'architettura (brief, ricerche, UX, regole)
created: 2026-11-20
sources:
  - _bmad-output/planning-artifacts/briefs/brief-topictoast-2026-11-18/brief.md
  - _bmad-output/planning-artifacts/briefs/brief-topictoast-2026-11-18/addendum.md
  - _bmad-output/planning-artifacts/briefs/brief-topictoast-2026-11-18/.memlog.md
  - _bmad-output/planning-artifacts/prds/prd-topictoast-2026-11-19/research-footprint.md
  - _bmad-output/planning-artifacts/prds/prd-topictoast-2026-11-19/research-stream.md
  - _bmad-output/planning-artifacts/ux-designs/ux-topictoast-2026-11-19/EXPERIENCE.md
  - _bmad-output/planning-artifacts/ux-designs/ux-topictoast-2026-11-19/DESIGN.md
  - _bmad-output/planning-artifacts/ux-designs/ux-topictoast-2026-11-19/.memlog.md
  - _bmad-output/planning-artifacts/ux-designs/ux-topictoast-2026-11-19/triage.md (solo T-55…T-70)
  - CLAUDE.md
  - .specify/memory/constitution.md
  - .claude/skills/dashboard-react/SKILL.md
---

# Estratto di contesto per l'architettura

Legenda. Nei numeri di research-footprint: **[V]** benchmark del vendor, **[T]** terza parte,
**[D]** documentazione ufficiale, **n.v.** non verificato. «da verificare» = marcato così nella
fonte. *Implicazione* = deduzione di questo estratto, non scritta nella fonte. **[C-n]** = rimando
a una contraddizione o tensione tra fonti (§6). Il PRD non è una fonte di questo estratto: è citato
solo dove supera il brief (PRD §1.2, «Differenze dal brief», punto n).

## 1. Posizionamento e confronti

### 1.1 Prodotto (brief)

- Motore di workflow open source, **Apache-2.0** per tutto il nucleo (nessuna funzione a
  pagamento), in Rust, **da incorporare** in altre piattaforme. Due livelli di utenti: chi integra
  (primo: l'autore, app di threat intelligence) e i tenant della piattaforma ospite (editor
  no-code). Chi usa la dashboard «tipo n8n» non è il pubblico.
- Caso concreto: indicatore → verifica DNS e Censys in parallelo → arricchimento → inserimento;
  l'esecuzione riceve un indicatore o un elenco (fan-out dinamico parallelo = requisito).
- Cinque pezzi v1: motore OWS + estensione DAG (dipendenze tra task, parallelo su elenchi, retry,
  timeout); SDK TypeScript e Rust (workflow come codice o invio di un documento OWS a sistema
  avviato); libreria JS in due strati (nucleo senza interfaccia che gestisce il DSL e parla col
  motore, usabile senza React; componenti React sopra); dashboard di chi gestisce; cassaforte dei
  segreti (v1 solo chiavi API, OAuth dopo).
- Installazione v1: **topictoast + Iggy + database a scelta (SQLite incorporato o Postgres)**;
  RustFS solo per payload grandi o file, **fuori dalla v1**.
- Separazione tra tenant, cassaforte, **sandbox eseguita da topictoast stesso, che deve reggere
  codice ostile**; nodi personalizzati in TS e Rust (Python e Go dopo).
- Fuori v1: nodi di base pronti (cicli, connettori, LLM), RustFS, OAuth gestito, utenti tipo n8n,
  nodi in Python e Go.
- Successo a 6 mesi: **≥500 stelle GitHub** (Workflow Builder 448, Zigflow 242) e **benchmark
  pubblico del carico sul database**, istruzioni SQL per task «molto sotto» le ~7.700 di Hatchet
  con Postgres come coda. PRD §1.2 punto 9 riscrive il criterio [C-2].
- Superato dal PRD (§1.2): stream processor SQL in v1 (punto 1) [C-3]; nodi personalizzati
  precompilati, caricati via SDK o CLI, server senza compilatori (punto 7); sandbox su due livelli
  (punto 8) [C-8].

### 1.2 Differenze su cui il brief scommette (in ordine)

1. **Kit React incorporabile, nativo del motore.** «Vantaggio di esecuzione, non una barriera: un
   kit si può copiare.» Concorrente più vicino: **Workflow Builder** (Synergycodes),
   `@workflowbuilder/sdk` 3.0.0 su React Flow, Apache-2.0, 448 stelle, attivo (verificato); esegue
   solo su Temporal (`@workflowbuilder/temporal` 0.3.1, vista live); un solo nodo di partenza,
   niente cicli. Competere o integrarsi (suo adattatore verso topictoast): aperto.
   Altri (addendum §4): Trigger.dev `@trigger.dev/react-hooks` 4.8.6 solo hook; Inngest
   `useRealtime` solo hook; Hatchet e Temporal UI chiuse nella propria app; Windmill iframe o
   licenza commerciale (AGPLv3); Kestra `@kestra-io/topology` 0.2.0 grafo Vue legato al suo design
   system; n8n solo con licenza Embed a pagamento; Conductor `orkes-workflow-visualizer` 1.1.1,
   React, ultimo rilascio 2025-09, licenza da verificare. Formula difendibile: «kit incorporabile
   nativo del motore, licenza permissiva, DAG al centro»; «non esiste niente» non regge. Librerie
   riusabili: `@xyflow/react` (MIT), dagre (MIT).
2. **Database al minimo**: attività su Iggy, DB solo stato. Riferimento: hatchet-dev/hatchet#5153,
   con Postgres come coda ~7.700 istruzioni SQL e ~600 KB di WAL per task, 5,2 M istruzioni/giorno
   per ~680 task; il maintainer non lo considera un bug e consiglia RabbitMQ.
   hatchet-dev/hatchet#5501: RabbitMQ 4.4 rifiuta le code di Hatchet, autore bloccato a 4.2.9.
   Addendum: la critica «Hatchet dipende da RabbitMQ» è **superata in parte**
   (`SERVER_MSGQUEUE_KIND=postgres`, «This is optional»; default RabbitMQ; hatchet-lite «for
   development and low-volume use-cases») [C-9].
3. **Velocità, poca memoria, alta scalabilità**: vanno dimostrate con numeri; il brief non fissa
   obiettivi (rinviati al PRD). Riferimenti in §2.1.
4. **Separazione tra tenant con sandbox e cassaforte nel nucleo open permissivo.** Nessuno offre
   insieme isolamento forte, segreti cifrati per tenant, OAuth rinnovato dal motore e sandbox
   gratis con licenza permissiva; i pezzi singoli esistono (addendum §5):

| Strumento (licenza) | Separazione | Segreti | OAuth | Sandbox |
|---|---|---|---|---|
| Hatchet (MIT) | tenant in OSS; SSO/RBAC a pagamento | per tenant da verificare | no | no |
| Temporal (MIT) | namespace; autorizzazione da scrivere | no | no | no |
| Windmill (AGPL + EE) | workspace (3 in Community) | cifrati per workspace; vault esterni EE | rinnovo automatico; personalizzato EE | nsjail, spento di default |
| Kestra (Apache-2.0) | tenant solo EE/Cloud | OSS variabili d'ambiente non cifrate | no | no |
| Activepieces (MIT + `ee`) | progetti dal piano Team | gestori di segreti solo piano alto | connessioni nel nucleo | isolated-vm in OSS; codice privato a pagamento |
| n8n (Sustainable Use) | progetti, non isolamento forte | vault esterni EE | OAuth2; per utente EE | task runner separati |
| Argo (Apache-2.0) | namespace K8s | Secret K8s | no | no; pod root di default |

- **OWS è una scelta di progetto, non una differenza**: standard pronto per chi costruisce, non
  motivo di scelta per chi integra. Con l'estensione DAG i documenti non girano su altri runtime.
- **Cosa non lo rende diverso**: DAG + YAML + DB (Kestra, Windmill, Argo); binario unico + DB
  (Hatchet, Windmill, Restate). Con SQLite topictoast ha 2 processi come Hatchet con solo Postgres:
  la leggerezza si argomenta con memoria e carico, non col numero di componenti.

### 1.3 Concorrenti: fatti verificati e comparabili

- Argo: solo Kubernetes (vale, è una CRD); un pod per worker vale in parte (template HTTP e plugin
  in un pod «Agent» per workflow; ContainerSet «tens rather than hundreds»); limite 1 MB di etcd.
- Temporal: 4 servizi (Frontend, History, Matching, Worker) + DB; Elasticsearch non più necessario
  (visibility su Postgres, MySQL o SQLite); dev server «not intended for production use»; External
  Storage su S3/GCS/Azure per payload grandi già presente.
- Tabella comparabili (addendum §3; in gran parte conoscenza del modello, **da verificare**, salvo
  le voci «verificato: Completamento»): Restate binario unico, **BSL 1.1**; Inngest binario unico
  con SQLite + Redis incorporato, server **SSPL** con passaggio ritardato ad Apache-2.0; Windmill
  Postgres + server + worker, **AGPLv3**; Kestra Apache-2.0, un DB JDBC, Kafka/ES solo enterprise.
- Rust: nessun orchestratore maturo, autonomo, DAG-first (dagrs, apalis-workflow, IronFlow,
  Operon, treadle). Nessuno degli strumenti confrontati supporta OWS.
- Runtime OWS esistenti: SonataFlow, Quarkus Flow, SDK Java (JVM); Lemline, Synapse, EventMesh
  (linguaggio da verificare); **Zigflow** (Go su Temporal, Apache-2.0, 242 stelle, v0.18.1
  2026-11-11): documenti non OWS puri (`document.taskQueue`, `document.workflowType`
  obbligatori), stessi limiti DAG (`for` sequenziale, `fork` statico, `then` nello stesso blocco),
  `run.workflow` con `await: false`, `listen` su query/signal/update Temporal, niente `emit`.
- SDK ufficiali dei concorrenti: Hatchet Py/TS/Go/Ruby; Temporal Go, Java, .NET, PHP, Python,
  Ruby, Rust, TS; Restate TS, Java/Kotlin, Python, Go, Rust; Inngest TS, Python, Go, Java/Kotlin.
- No-code open: n8n Sustainable Use License (non OSI), 211 mila stelle, embedding solo con licenza
  Embed; Activepieces MIT fuori da `ee`, 25,6 mila stelle: un terzo utente «tipo n8n» mette
  topictoast contro Activepieces.

## 2. Esiti delle ricerche

### 2.1 Footprint dei concorrenti (research-footprint, 2026-11-19)

Quasi tutti [V], su hardware e carichi diversi: **non confrontabili tra loro**.

**RAM**

- Temporal dev server ~150 MB (server 112 + worker 35) [T, 2023]; Temporal a carico medio-alto
  ~13 GiB (Frontend 4, History 6+, Matching 2, Worker 1) [T, 2026-02].
- Restate: minimo ~1,5 GiB (RocksDB 1 GiB, query engine 256 MiB, cache 128 MiB) [D].
- Windmill: compose 8 container 633 MiB a riposo, 819 MiB di picco (n.v.); worker 1–2 GB per vCPU
  [D].
- Kestra standalone: minimo 4 GiB, 2 vCPU [D].
- n8n: 300–350 MB + Postgres 45–50 MB [T, n.v.]; 860 MB senza workflow su VPS [T].
- Activepieces: worker 0,5 vCPU/1 GB, app 1/1, Postgres 2/4, Redis 1/1 [D].
- NATS ~6 MiB (blog, n.v.). Hatchet, Inngest, Argo: nessun valore. **Iggy: nessun valore
  pubblicato** [C-1].

**Latenza di avvio e overhead**

- Restate 1.2: p50 15 ms, p90 42 ms (3 step, carico basso); 1 step ad alto carico p50 16, p90 25
  [V].
- Windmill: ~50 ms per job (worker standard), ~12 ms (dedicato) [V].
- Hatchet v0.55.26: ~40 ms a 100 run/s, ~48 a 500, ~220 a 2.000 [V].
- Kestra EE (RabbitMQ + PG): avg 50–69 ms, p99 60–107 ms da 1.000 a 7.000 exec/min; OSS (PG) avg
  111–150, p99 151–370 fino a 4.000 exec/min, a 5.000 avg 10,8 s (saturazione) [V].
- Temporal: p95 StartWorkflowExecution, obiettivo < 150 ms, ottenuto < 50 ms [V].
- Inngest: tick di poll dell'executor 150 ms di default [D].
- Confronto Windmill (non neutrale, m4.large, 10 worker, 400 task): primo task Temporal 0,108 s,
  Hatchet 0,723 s, Windmill 1,663 s; attesa media Temporal 2,1 s, Windmill 4,2 s, Hatchet 19,2 s.
- Iggy (messaggi, non task): p99 produttore 2,59 ms, consumatore 1,46 ms (i3en.3xlarge) [V].

**Throughput**

- Hatchet su Postgres: 2.000 run/s con DB CPU 83 %, 800 IOPS (RDS m7g.2xlarge); 10k/s su
  m7g.8xlarge senza dettagli [V]; Postgres-only istanza singola «centinaia di task/s» (n.v.).
- Kestra OSS: 4.000 exec/min = ~133 task/s su n2-standard-4 + PG 4 vCPU/16 GB; EE ~233 task/s,
  harness non saturato [V].
- Temporal: 150 → 1.350 state transition/s dopo tuning [V].
- Windmill: 981 job/s con 100 worker [V]. Restate 1.2 (3 nodi): 23.131 rps (1 step), 16.844 (3),
  8.571 (9) [V]. n8n: fino a 220 exec/s [V]. Iggy: 2,4 GB/s scrittura, 4,0 GB/s lettura [V].

**Carico sul database**

- Hatchet: DB CPU 15/60/83 % e IOPS 400/600/800 a 100/500/2.000 run/s (~4/1,2/0,4 IOPS per run).
- Kestra OSS: PG CPU 68,1 % a 4.000 exec/min; JDBC adatto fino a ~1.000 task run/min, Kafka oltre
  ~2.000; sopra 1.000 task/min Postgres cresce di terabyte/anno.
- Temporal: ~60 st/s per vCPU Cassandra, ~150 per vCPU di cluster (post community).
- Activepieces: ~2,5 millicore PG per req/s. Inngest: una riga per evento/step, nessuna pulizia.
- «Nessun dato su WAL, query per task o scritture per task per nessun sistema» [C-2].

**Processi minimi**: Hatchet lite 2 container (3 con RabbitMQ); Windmill 3; Kestra 1 + PG;
Temporal 4 + DB; Activepieces app + worker + PG + Redis + S3; n8n 1 + DB (queue mode + Redis);
Inngest 1; Restate 1 (n.v.); NATS e Iggy 1 binario ciascuno (n.v.).

**Esito per le scelte**: valori credibili più bassi tra i concorrenti: RAM ~150 MB (Temporal dev),
~323 MB (n8n); scheduling 12 ms (Windmill dedicato), p50 15 ms (Restate); 2.000 run/s su Postgres
(Hatchet, DB CPU 83 %). **Da misurare prima di fissare i target**: RAM a riposo di topictoast,
latenza di avvio p50/p99 e task/s su una macchina di riferimento (es. 4 vCPU/16 GB come Kestra),
scritture DB per task.

### 2.2 Stream processor SQL (research-stream, 2026-11-19)

- **DataFusion** (Apache-2.0, libreria): sorgenti unbounded sì (`execute_stream()`,
  `SymmetricHashJoinExec`, `AggregationExec` senza watermark). Mancano upstream watermark,
  checkpoint dello stato, backend di stato collegabile, `StreamingWindowExec` (issue #11404). I
  maintainer lasciano watermark e checkpoint ai progetti a valle. Epic #24265 (2026-08-11, senza
  assegnatari): documentazione, test, flush, possibile crate `datafusion-streaming`; **finestre e
  watermark non pianificati**. TUMBLE/HOP/SESSION non nel core (`laminar-sql`, Arroyo,
  Denormalized). Release: 52.0.0 del 2026-01-12; 54.0.0 da fonte secondaria. Sintesi: motore di
  query embeddabile, **non uno stream processor completo**.
- **Arroyo**: Apache-2.0/MIT, server o cluster, embedding come libreria non indicato; acquisito da
  Cloudflare (2025-04-10, futuro dell'open source non detto); 5.077 stelle, ultimo push
  2026-11-12; v0.15.0 2025-12-01 (n.v.).
- **Denormalized**: Apache-2.0, embeddabile, DataFusion, finestre e join, solo Kafka, checkpoint
  con SlateDB; «work-in-progress», 380 stelle, nessuna release. Unico caso embeddabile + DataFusion +
  finestre, ma immaturo.
- **RisingWave** (Apache-2.0): solo server, wire protocol PostgreSQL, 9.411 stelle. **Feldera**:
  licenza discordante (MIT o Apache-2.0), DBSP è libreria Rust, «SQL embedded» non verificato,
  8 GB consigliati. **Materialize**: BSL, chiave obbligatoria da v26.0.0, limite 24 GiB RAM /
  48 GiB disco: incompatibile. **Fluvio/SDF**: SDF beta, licenza community «single use», server.
  **Streamling** (Goldsky, 2026-06-08): Arrow + DataFusion, YAML, plugin Rust/WASM, niente SQL
  stateful, licenza non indicata.
- Memoria: nessuna cifra misurata; «circa 2 GB per una pipeline piccola» (terzi).
- **Iggy e stream**: connectors runtime con plugin `.so`/`.dylib`/`.dll` (trait `Sink`/`Source`,
  un runtime Tokio per plugin); 6 sorgenti, 17 sink, 8 transform **per messaggio, senza stato né
  finestre**; «Flink processor» nella 0.6.0 (2025-12-09); **nessun SQL né DataFusion**: un
  processore SQL su Iggy è un componente nuovo.
- Orchestratori (Kestra, Windmill, n8n, Temporal, Hatchet): tutti «un evento, un'esecuzione»;
  nessuno combina DAG e SQL continuo (ricerca non esaustiva). Temporal via signal: pochi signal/s
  per workflow (forum). Hatchet: un evento arrivato prima della registrazione non avvia nulla.
- Casi d'uso: filtrare prima di avviare; deduplicare indicatori con chiave e finestra; soglie
  «più di N in T minuti»; arricchire e correlare in volo prima di SIEM/SOAR.
- **Rischio di perimetro**: stato, finestre e watermark sopra DataFusion sono «un progetto da
  solo», «lavoro da team dedicato» (Cloudflare, Denormalized). Opzioni embeddabili: DataFusion +
  stato proprio; Denormalized (immaturo, solo Kafka); DBSP/Feldera (n.v.). Solo server: Arroyo,
  RisingWave, Materialize, SDF [C-3].

### 2.3 OWS e DAG (addendum §1)

La sezione «Ricerca» è la sintesi di un subagent: **citazioni da ricontrollare sul testo
originale prima di un'ADR**. La sezione «Verifica» è letta sul testo della spec (2026-11-18).

- Versioni: **v1.1.0 stabile dal 2026-11-12**, v1.0.0 dal 2025-01-27; esempi del sito ancora
  `dsl: '1.0.3'`; compatibilità «per tutto il ciclo 1.x»; Apache-2.0.
- Obbligatori per un runtime conforme: task `run` (container, script, shell, workflow) e `call`
  (HTTP, gRPC, OpenAPI, AsyncAPI, A2A); **espressioni jq**; meccanismo dei segreti; timeout;
  schedule `every`/`cron`/`after`/`on`; eventi CloudEvents `emit`/`listen`. MCP descritto, non
  obbligatorio.
- CTK: scenari Gherkin in `ctk/features/` senza runner né step (il crate `cucumber` va bene);
  superarlo è SHOULD.
- **SDK Rust**: 3 crate (core, builders, IO YAML/JSON), `serverless_workflow_core`
  1.0.0-alpha6.3 (2025-01-27), ultimo commit 2025-12-12, **non aggiornato alla 1.1.0**, nessun
  motore. Crate comunitario `swf-runtime` alpha, manutenzione da verificare [C-4].
- DAG in OWS 1.1.0 (verificato): serie-parallelo fisso sì (`do`, `fork` con join implicito);
  dipendenze tra rami **no** (`then` solo nello stesso blocco); fan-out dinamico parallelo **solo
  l'avvio** (`for` sequenziale + `run.workflow` con `await: false`, manca il join; issue #1143
  aperta dal 2026-01-19, stale); `emit`/`listen` non garantito come dipendenza. Discussione #1133:
  `for` sequenziale (maintainer), `ForEach` parallelo della 0.x tolto, schema a eventi proposto
  (evento per elemento, sotto-workflow, ascolto delle risposte) che richiede un bus di eventi
  interno al runtime; secondo il maintainer non è lo strumento adatto ai big data.
- Schema (`schema/workflow.yaml`): task chiusi (`unevaluatedProperties: false`, anche in `for`):
  `dependsOn` o `for.parallel` rendono il documento invalido per qualunque validatore OWS.
  `metadata` è mappa libera su task e documento: `metadata: { topictoast: { dependsOn: [a, b] } }`
  resta valido. Ipotesi da verificare: un altro runtime ignora `metadata` ed esegue `do` in
  sequenza, con lo stesso risultato solo se i task sono in ordine topologico e i dati passano da
  `$context` (`export.as`). `extensions` non cambia il flusso.
- Decisione aperta (memlog del brief): metadata, chiavi nuove o proposta upstream → architettura.

### 2.4 Bus e storage (addendum §2)

- **Apache Iggy**: progetto top-level Apache dal 2026-08-19 [C-10]; **0.10.0 del 2026-11-07**,
  Apache-2.0, binario unico, ~20 MB di RAM all'avvio (FAQ) [C-1]. Pre-1.0: la 0.10.0 **rifiuta i
  client precedenti e cambia il formato su disco senza migrazione dalla 0.9.x** [C-4]. Cluster VSR
  dalla 0.9.0, spento di default, «maturing toward production readiness»; una replica che non torna
  blocca la partizione senza override.
- **Iggy è un log append-only, non un broker** (verificato su concepts.md): una partizione va a un
  solo membro del consumer group; lettura per offset; at-least-once solo confermando l'offset dopo
  l'elaborazione; **nessun ack per messaggio, riconsegna, timeout di visibilità o messaggio
  ritardato**. Conseguenza: ack, retry, timeout e ritardi dei task stanno in topictoast; **un task
  lento blocca quelli dopo nella stessa partizione**.
- SDK Iggy: Rust, C#, Java, Python, Node.js, Go. Nessun orchestratore lo usa come trasporto (solo
  un plugin Kestra).
- Alternative che un revisore citerà: **NATS JetStream** (consumer pull con ack esplicito,
  Apache-2.0); per S3: MinIO archiviato («no longer maintained»), SeaweedFS, Garage.
- **RustFS** (fuori v1): 1.0 GA 2026-09-16, **1.0.6 del 2026-11-15**, Apache-2.0, 36 mila stelle;
  le 1.0.x correggono soprattutto la riparazione dati (heal, MRF, scanner); compatibilità S3 «for
  supported features».

### 2.5 Sandbox (addendum §6, §7)

| Tecnologia | Isolamento | Avvio | Linguaggi | KVM | Da Rust | Maturità |
|---|---|---|---|---|---|---|
| nsjail | namespace, cgroup, seccomp, kernel condiviso | non documentato | qualsiasi binario Linux | no; user namespace, a volte root | processo C++ separato | 3.6, Apache-2.0; usato da Windmill |
| Hyperlight | VM senza kernel guest | ~1 ms a VM calda | `no_std` Rust o C; Wasm sperimentale | sì (KVM o MSHV) | `hyperlight-host` | pre-1.0, CNCF Sandbox |
| wasmtime | software, nel processo | µs con pooling allocator | Rust, JS, Python, Go compilati in Wasm | no, niente root | `wasmtime` | Tier 1 x86_64 Linux; Fastly |
| Firecracker | VM KVM con kernel guest | fino a 125 ms; < 5 MiB overhead | tutto il guest Linux | sì; nested limitata sui cloud | demone separato, API REST | v1.18, Apache-2.0; AWS Lambda |
| Nanos | unikernel in VM | non verificato | binari Linux, un processo | sì | strumento `ops` | «technical preview»; licenza commerciale oltre 50 dipendenti |

- Compromessi: solo wasmtime non chiede KVM né root («binario unico, facile da installare»);
  nsjail e Firecracker eseguono qualsiasi linguaggio, ma nsjail condivide il kernel con la
  macchina che tiene i segreti di tutti; **Wasm condivide il processo: un bug del runtime è una
  fuga**; wasmtime è l'unica libreria Rust in produzione.
- Punto di partenza suggerito: **wasmtime predefinito, Firecracker opzione rinforzata**. Da
  verificare: storico delle advisory di wasmtime, memoria dei guest Python e JavaScript.
- Criteri dello spike (prima dello sprint 1): forza dell'isolamento, avvio per task, linguaggi,
  requisiti d'installazione (KVM, permessi), memoria per sandbox.
- PRD §1.2 punto 8 precisa: leggera predefinita senza KVM né root, **in un processo separato da
  quello che custodisce la chiave della cassaforte**; forte (microVM) opzionale con KVM e pool di
  sandbox già avviate [C-8].
- **Requisiti di sicurezza** (addendum §7): isolamento tra tenant su dati, esecuzioni, storico e
  segreti; quote per tenant (task, concorrenza, risorse della sandbox); la sandbox non raggiunge
  segreti, rete interna, dati altrui; segreti cifrati a riposo, consegnati al worker solo per
  l'esecuzione che li usa, mai nei log; rinnovo OAuth dal motore (dopo v1).

## 3. Vincoli dalla UX

Citazioni per sezione: «EXP › …» = EXPERIENCE.md, «DES › …» = DESIGN.md.

### 3.1 Superfici e modello di incorporamento

- Due superfici, **un solo design system**: kit React senza marchio, incorporato nella piattaforma
  ospite; dashboard dell'operatore distribuita con topictoast (FR-45…FR-49). [EXP › Foundation]
- Kit: componente **«area workflow»** pronto con navigazione interna a cinque voci (Workflow,
  Editor, Esecuzioni, Segreti, Pipeline stream; FR-58), **più gli stessi pezzi esportati singoli**.
  Nei pezzi singoli niente selettore: vale il tenant della credenziale passata al componente.
  [EXP › Information Architecture › Kit; Component Patterns › tenant-selector]
- Il kit non ha superficie per caricare nodi personalizzati: solo SDK o CLI (FR-15).
  [EXP › Information Architecture › Kit]
- Il kit non aggiunge un'intestazione, **non rende mai `main`** (è dell'ospite), il livello delle
  intestazioni è una proprietà. [EXP › Accessibility Floor › Kit incorporato]
- **Il kit non scrive nella cronologia dell'ospite** (gesto indietro solo se l'ospite lo abilita).
  [EXP › Responsive & Platform] *Implicazione*: navigazione interna senza URL dell'ospite [C-6].
- **Senza marchio**: nessuna frase nomina topictoast, il motore o un crash (FR-44); il gate cerca
  «topictoast» nel bundle del kit, «stringhe, icone, nomi di file e titoli»; unico opt-in la
  proprietà `branding` (T-63). [EXP › Voice and Tone › Kit senza marchio]
- **Personalizzazione** (FR-44): colori, caratteri, testi, lingua; meccanismo (proprietà, variabili
  CSS) all'architettura; **proprietà ed eventi dei componenti React sono contratto pubblico** (PRD
  §6); il kit verifica le coppie di contrasto dei colori dell'ospite (errore visibile in
  sviluppo, avviso in console in produzione; T-45, meccanismo T-63); un testo dell'ospite mancante
  ricade sul predefinito; tema scelto dall'ospite, altrimenti `prefers-color-scheme` (A-23); la
  dashboard non si personalizza. [EXP › Personalizzazione dell'ospite]
- Librerie di canvas, layout del DAG ed editor di codice: **all'architettura (T-58)**; candidate
  `@xyflow/react`, dagre, CodeMirror 6; criteri: un solo arresto di `Tab`, roving tabindex o
  `aria-activedescendant`, ruolo che non spegne il lettore di schermo (mai `role="application"`),
  uscita dall'editor di codice con `Tab`, direzioni sinistra-destra e alto-basso, layout
  automatico. [EXP › Foundation; triage T-58]
- Font di sistema, **nessun file di font nel bundle**; icone SVG in linea; motivo: CSP
  `default-src 'self'`. [DES › Typography, Shapes]

### 3.2 Autenticazione e passaggio dall'ospite

- Kit: **credenziale firmata dall'ospite**, collegata all'identità dei suoi tenant; nessun secondo
  login, nessun nome di topictoast (FR-27). [EXP › Key Flows › UJ-1 passo 1]
- Contenuto della credenziale: **tenant** (FR-26, FR-27), **ruolo** tenant o integratore (FR-28),
  **«nome visualizzato» facoltativo**, mostrato come testo (FR-27, NFR-7). [EXP › Voice and Tone ›
  Chi ha fatto cosa; Component Patterns › tenant-selector]
- Ruolo integratore con più tenant: `tenant-selector`; **per cambiare tenant il kit chiede
  all'ospite una credenziale nuova** (meccanismo: architettura); dopo il cambio non resta nulla del
  tenant precedente, **eventi in tempo reale compresi** (FR-27). Con il solo tenant predefinito il
  selettore non esiste (FR-58, PRD §4.6). [EXP › Component Patterns › tenant-selector]
- **Scadenza della sessione**: nessun secondo login; il kit **avvisa l'ospite con un evento almeno
  60 s prima** (WCAG 2.2.1) e alla scadenza; grafo, SQL e JSON non salvati restano in memoria e
  tornano dopo il rinnovo **solo se la credenziale nuova porta lo stesso tenant e lo stesso
  ruolo**, altrimenti si scartano (FR-27). Nome dell'evento: T-63. [EXP › State Patterns › Stati
  trasversali › Sessione scaduta]
- Dashboard: Accesso con la **credenziale generata e stampata al primo avvio**; senza credenziale
  nessun dato (FR-45); la `login-card` dice dove è stampata (testo e luogo: T-63); limite dei
  tentativi: T-63; sessione scaduta → torna ad Accesso senza dati. [EXP › Information Architecture ›
  Dashboard; Component Patterns › login-card; Key Flows › UJ-5]
- Dashboard: le mutazioni portano **la protezione CSRF decisa dall'architettura** (skill §2).
- **Payload diverso per ruolo**: con ruolo tenant il kit non mostra l'ID del worker; con ruolo
  integratore e nella dashboard sì (FR-41, FR-48). [EXP › Voice and Tone › ID dei worker]
- Risorsa di un altro tenant o già eliminata: **«come se non esistesse»** («Workflow non
  trovato.»), **mai «accesso negato»** (FR-26, FR-31); uno stream di un altro tenant risulta
  inesistente. [EXP › State Patterns › Stati per superficie; Interaction Primitives › Pipeline
  stream]
- Workflow dell'integratore (FR-29): sola lettura per tutti, «Esegui» attivo; all'avvio ogni
  connessione si risolve tra i segreti del tenant, altrimenti rifiuto **prima dell'accettazione**;
  un nodo omonimo del tenant non cambia ciò che esegue. [EXP › Stati trasversali; Component Patterns
  › palette-item]

### 3.3 Tempo reale [EXP › Tempo reale]

- **Un cambio di stato appare entro 1 s (FR-41)** in nodi, badge, conteggi del fan-out, barra di
  stato, tentativi, log, tabelle della dashboard, cronologia dei worker.
- Fan-out: **si aggiornano i conteggi aggregati, non i singoli rami** (FR-41); mai un ramo per
  elemento disegnato. [Component Patterns › fanout-group]
- **Gli eventi del kit portano solo risorse del tenant (FR-26).**
- Pausa: i valori restano fermi e non si svuotano; **alla ripresa si riallineano da soli**. Canale
  interrotto per rete: stesso `pause-notice` (A-12). Dashboard con canale interrotto: T-68, dopo la
  scelta del canale e della riconnessione.
- Log: una riga per evento con **orario al millesimo**, livello, messaggio; «Pausa aggiornamenti»
  / «Riprendi». [Component Patterns › log-line]
- Pipeline stream: stato e «Ultime righe emesse» seguono il tempo reale. [Component Patterns ›
  pause-notice]
- T-65 (architettura): aggiornamenti raggruppati, nessun ridisegno fuori schermo, budget sul
  numero di task (NFR-14) e sulla frequenza degli aggiornamenti (FR-41).
- *Implicazione*: canale push filtrato per tenant e per ruolo, con riallineamento (stato corrente +
  differenze) dopo pausa o riconnessione; il canale non è scelto dalle fonti.

### 3.4 Validazione condivisa nucleo JS ↔ motore e forma degli errori

- **«La validazione nell'editor dà gli stessi esiti del motore»** (nucleo JS, FR-39).
  [EXP › Foundation › Fuori perimetro UX] «Salva» valida con il nucleo JS; un rifiuto del motore
  dopo «Salva», o una funzione sparita all'esecuzione, si mostra come gli errori del nucleo, sul
  task (FR-4). [EXP › Interaction Primitives › Salvare ed eseguire]
- Il salvato deve essere un **documento OWS valido** (FR-40). [EXP › Stati trasversali › Errori di
  validazione]
- **La fonte lega al nucleo JS** solo il «Salva» dell'editor del grafo e il `code-editor` (SQL,
  JSON, jq): «Valida con il nucleo JS prima del salvataggio (FR-39)». Errori che l'editor del grafo
  mostra sul task prima del salvataggio (FR-40) [EXP › Stati trasversali › Errori di validazione;
  Component Patterns › edge, data-mapping, code-editor]:
  - ciclo nel grafo con i task coinvolti (FR-2); collegamenti liberi, più radici e più foglie;
  - funzione inesistente **con suggerimento di una funzione simile** (FR-4, FR-40);
  - tipi e campi obbligatori dagli schemi del manifest (FR-55); connessione senza segreto;
  - jq non valido con colonna; jq con `$ENV`, `env`, `input_filename`, `import`, `include` vietato
    (FR-17);
  - nel `code-editor`: SQL delle pipeline stream e JSON con riga e colonna (FR-43).
- **Errori che la UI mostra, validatore non dichiarato** (nucleo, motore o entrambi) [EXP › Voice
  and Tone, tabella; Component Patterns › input-form, schedule-section, form-field]: costrutto SQL
  non consentito e ciclo tra pipeline stream (FR-19); finestra oltre la conservazione dello stream
  (FR-18); elenco incollato con numero di riga; cron con posizione dell'errore; valori oltre il
  massimo dell'installazione (NFR-14); versione «`document.dsl` 1.0.0 non accettata: versioni
  accettate 1.1.x» (FR-1) [C-4]. Alcuni chiedono stato del server (conservazione degli stream, altre
  pipeline, limiti dell'installazione).
- **Forma degli errori**: **codice + parametri** da motore e nucleo, resi dal dizionario
  sostituibile (FR-44; formato: T-63). Nell'editor task e campo; per SDK e documento percorso e
  motivo (**forma del percorso decisa dall'architettura**, FR-4); nell'SQL riga e colonna;
  suggerimento quando il motore lo conosce (WCAG 3.3.3). Nessun messaggio dice solo «Errore».
  [EXP › Voice and Tone › Errori, Codici non frasi]
- **Limiti**: «`<limite>` superato: `<valore>` oltre `<massimo>`»; **i massimi veri vengono
  dall'installazione, i numeri della UX sono esempi**. Classi di limite nominate: input e output del
  task, join, CPU/memoria/tempo della sandbox, dimensione del documento, dimensione della query,
  numero di task, annidamento, elementi del fan-out, risposta di `call http`. [EXP › Voice and Tone ›
  Limiti superati]
- Default e massimi di retry, timeout e parallelismo «letti dall'installazione e mai scritti nella
  UI» (NFR-14). [EXP › Component Patterns › form-field] *Implicazione*: un'API espone limiti e
  valori predefiniti dell'installazione.
- *Implicazione*: DSL OWS + estensione DAG, jq e dialetto SQL vanno validati con esiti identici in
  Rust e nel nucleo JS; un'implementazione unica condivisa (per esempio Rust compilato in Wasm) è
  un'opzione, non una decisione delle fonti.

### 3.5 Dati che la UI chiede al server

- **Manifest per funzione** (FR-55): nome, descrizione, gruppo, connessioni (piattaforma e
  credenziale richiesta), schema di input, schema di output, campo di estensione; **stesso manifest
  per i nodi personalizzati caricati con la CLI**. Da qui palette, campi «Dati», segreti proposti,
  validazione. [EXP › Foundation › Manifest]
- **Connessione**: piattaforma esterna dichiarata dal manifest che un segreto del tenant soddisfa;
  il segreto dichiara la connessione (FR-32); non è un connettore né un account OAuth (T-69).
  [EXP › Foundation › Connessione]
- **Palette**: solo funzioni, nodi personalizzati e segreti disponibili al tenant (FR-40, FR-15),
  più i costrutti di FR-1; omonimi distinti dalla provenienza. Costrutti disegnati (T-40):
  `switch`, fan-out (`for`, `fork`), `wait`, `raise`, `set`, `call http`, sotto-workflow; `try` e
  retry come campi del task; i campi dei costrutti vengono «dallo schema del costrutto nel
  formato»; i campi di un sotto-workflow dallo schema di input del workflow chiamato (FR-56).
  `call http` senza manifest propone tutti i segreti del tenant. [EXP › Component Patterns ›
  palette-item; Interaction Primitives › Aggiungere dalla palette, Selezione e configurazione]
- **Task nell'editor**: retry (tentativi, attesa, crescita dell'attesa; FR-7), timeout (FR-8);
  per un fan-out politica d'errore («fallisce tutto il fan-out» o «il join riceve l'errore al posto
  del risultato») e parallelismo massimo (FR-3); schema di output del nodo in sola lettura (FR-55).
  [EXP › Interaction Primitives › Selezione e configurazione] Con elenco vuoto il join riceve un
  elenco vuoto (FR-3). [EXP › Component Patterns › fanout-group]
- **Workflow**: nome, versione (FR-5), origine (tenant, «Dell'integratore», «da codice»), schema
  di input (FR-56; come si scrive nel documento: domanda aperta 7 del PRD), pianificazione del
  tenant (una per workflow, FR-57) distinta da quella scritta nel documento OWS (sola lettura),
  **posizioni manuali del desktop** (T-46). [EXP › Component Patterns › workflow-bar,
  schedule-section; Interaction Primitives › Selezione e configurazione, Canvas]
- **Esecuzione**: ID e stato «In coda» **subito all'accettazione** (FR-6); versione con cui è
  partita; origine e autore («dal tenant», «dall'integratore», nome visualizzato, «da
  pianificazione», «da pipeline stream «`<nome>`»»); annullamento con chi e quando (FR-10).
  [EXP › Stati trasversali › Salvare ed eseguire; Component Patterns › workflow-bar; Voice and Tone
  › Chi ha fatto cosa, tabella]
  - Stati chiusi: esecuzione (in coda, in esecuzione, completata, fallita, annullata, scaduta);
    task (in attesa, in coda, in esecuzione, ritentato, completato, fallito, saltato, annullato);
    attività (in coda, ricevuta, in esecuzione, ritentata, completata, fallita, annullata); worker
    (collegato, non risponde); pipeline stream (attiva, rallentata, sospesa); pianificazione
    (attiva, sospesa). Propagazione di fallito e saltato: **domanda aperta 8 del PRD,
    all'architettura**. [EXP › State Patterns › Elenco chiuso degli stati]
  - «In attesa» = dipendenze non finite; «in coda» = pronto ma attività non consegnata, **con il
    motivo**: nessun worker per la funzione, quota, pool della sandbox vuoto; l'attesa in coda non
    consuma tentativi (FR-8); quarto motivo, partizioni del bus: T-66 (ripiego «In coda: in attesa
    di un worker libero»). Un'esecuzione è «in coda» finché nessun suo task è partito (un `wait`
    conta come partito). [EXP › State Patterns › Elenco chiuso degli stati; triage T-66]
  - Tentativi come attività distinte (FR-7), con **versione del nodo eseguita** (FR-15), ID del
    worker secondo il ruolo, errore; «ritentata: tentativo 2 tra 2 s». [EXP › Component Patterns ›
    attempt-card; Voice and Tone, tabella]
  - Input e output JSON del task **mai con valori di segreti** (FR-33); log per task o elemento.
    [EXP › Component Patterns › code-block, panel, log-line]
  - Fan-out: conteggi per stato; **tabella degli elementi filtrabile per stato e cercabile per
    valore** con migliaia di elementi («elemento 87 di 300»; se virtualizzata `aria-rowcount`,
    A-5). [EXP › Component Patterns › error-card]
  - Elenco esecuzioni: ultimi N giorni, **N dalla configurazione** (FR-31). [EXP › State Patterns
    › Stati per superficie]
- **Segreti**: nome, connessione, data di aggiunta, **ultimo uso**; il valore si scrive e non si
  rilegge mai (FR-32, FR-42); conteggio dei workflow che usano un segreto (T-63). [EXP › Component
  Patterns › data-table, form-field, confirm-dialog; Voice and Tone, tabella]
- **Pipeline stream (kit)**: query, stato e causa di sospensione imposta («per CPU», «per memoria»,
  «per arretrato»; FR-17, FR-25, FR-30), ultime righe emesse, righe trattenute (FR-24), righe non
  diventate esecuzione con il motivo (FR-6), stream del tenant con la loro conservazione (FR-18),
  destinazione «Stream del tenant» o «Workflow». [EXP › State Patterns › Elenco chiuso degli
  stati, Stati per superficie; Interaction Primitives › Pipeline stream]
- **Pianificazione**: prossima scadenza; input controllato sullo schema al salvataggio **e a ogni
  scadenza** (FR-6); scadenza rifiutata visibile con il motivo, mai in silenzio; scadenze perse
  durante una pausa e recuperate secondo la regola dell'installazione (NFR-14); «Riprendi» non
  recupera le scadenze della sospensione. [EXP › Component Patterns › schedule-section]
- **Dashboard** [EXP › Component Patterns › data-table, history-list; State Patterns]:
  - Worker: collegato o non risponde, funzioni registrate, attività in corso («8 in corso, la più
    vecchia da 41 s»), cronologia delle fasi (FR-48), **attività riassegnate** (FR-14), funzione
    registrata di nuovo con connessioni diverse (FR-48), host senza KVM (FR-54).
  - Esecuzioni: filtri Tenant, Workflow, Stato e «in coda»; colonna ordinabile **«In coda da»** =
    età del task o dell'attività in coda più vecchi dell'esecuzione (FR-46); «Annulla esecuzione».
  - Pipeline stream: **ritardo sul watermark, eventi al secondo, memoria di stato, eventi in
    ritardo scartati, ultimo checkpoint** (FR-47), eventi nel futuro oltre la tolleranza (FR-21).
  - Quote per tenant: attività concorrenti, esecuzioni al minuto, sandbox, pipeline stream, eventi
    al secondo, richieste API al secondo, spazio; stati «Vicino al limite» e «Al limite»
    (FR-49, NFR-14).
  - Avvisi: ripresa del motore con tempo trascorso (NFR-15); disco pieno (FR-48, NFR-6).

### 3.6 Persistenza, concorrenza, ciclo di vita

- «Salva» crea una versione; **nessun passo «pubblica»** (FR-5, T-52). [EXP › Interaction
  Primitives › Salvare ed eseguire]
- **Conflitto di versione**: se un altro ha salvato prima, il salvataggio è rifiutato con la
  versione corrente, mai sovrascritture silenziose; «Ricarica ultima versione», «Scarica le
  modifiche». [EXP › Interaction Primitives › Salvare ed eseguire] *Implicazione*: controllo di
  concorrenza ottimistico nell'API.
- Cambio dello schema di input di un workflow avviato da pianificazione o pipeline stream: conferma;
  poi gli avvii fuori schema non sono accettati (FR-5). [EXP › Voice and Tone, tabella; Interaction
  Primitives › Salvare ed eseguire]
- **Eliminare un workflow**: le esecuzioni in corso finiscono, la pianificazione è eliminata, lo
  storico resta; **rifiutato se usato come sotto-workflow o come destinazione di pipeline stream**
  (FR-58). [EXP › Voice and Tone, tabella] *Implicazione*: riferimenti tra risorse tracciati.
- **Eliminare un segreto**: i workflow che lo usano non partono; le esecuzioni in corso falliscono
  al task che lo usa (FR-42, FR-33). [EXP › Voice and Tone, tabella] *Implicazione*: risoluzione
  del segreto al momento del task.
- Modificare la query di una pipeline stream attiva **azzera lo stato** (FR-23). [EXP › Interaction
  Primitives › Pipeline stream]
- Workflow «da codice» e dell'integratore in sola lettura per tutti (FR-35, FR-29); la
  pianificazione del tenant resta modificabile (FR-57). [EXP › Stati trasversali; Component
  Patterns › schedule-section]
- Workflow di esempio solo nel tenant predefinito di un'installazione nuova (FR-50). [EXP › State
  Patterns › Stati per superficie]
- Posizioni manuali del desktop salvate col workflow: **dove (metadati del documento OWS o archivio
  separato) lo decide l'architettura** (T-46). [EXP › Interaction Primitives › Canvas]

### 3.7 Ripresa, disco pieno, ricarica

- **Ripresa del motore (NFR-15) = pausa, non perdita**. Kit: `pause-notice` neutro («Aggiornamenti
  in pausa, riprendono da soli.»). Dashboard: `recovery-banner` («Nessuna nuova esecuzione parte
  finché non è di nuovo attivo. Nessun lavoro accettato è andato perso. Tempo trascorso: 12 s.»),
  che sparisce quando il motore torna a consegnare attività. «Esegui» durante la ripresa: T-60
  (accettare e lasciare «In coda», oppure rifiutare con messaggio neutro). [EXP › Stati trasversali
  › Ripresa del motore; Component Patterns › recovery-banner]
- **Disco pieno**: il lavoro nuovo è rifiutato, quello accettato non va perso (FR-48, NFR-6).
  [EXP › Voice and Tone, tabella; Component Patterns › recovery-banner]
- Rifiuti **prima dell'accettazione**, accanto a «Esegui»: quota di arretrato o spazio (FR-30),
  segreto mancante (FR-29, FR-33), spazio esaurito (NFR-6). Quota di concorrenza: nessun rifiuto, le
  attività restano «In coda» con il motivo. [EXP › Stati trasversali › Quota raggiunta]
- Casi limite: worker non avviato → task «in coda» senza consumare tentativi, partono da soli alla
  registrazione (UJ-1); nodo in loop infinito fermato allo scadere del timeout, nessun effetto sugli
  altri tenant (UJ-2); riavvio durante una finestra → la pipeline stream riparte dall'ultimo
  checkpoint, il conteggio non si perde (UJ-3); worker che non risponde → attività riassegnate;
  tenant al limite non rallenta gli altri (UJ-4); in meno di 15 minuti topictoast in esecuzione con
  worker ed esempio completato (UJ-5, NFR-13). [EXP › Key Flows]
- Sandbox: «Rete non consentita: 169.254.169.254» (FR-16); CPU, memoria e tempo superati (FR-17).
  [EXP › Voice and Tone, tabella e Limiti superati]
- **Nessuna persistenza nel browser**: una ricarica perde il lavoro non salvato. Nessuna modalità
  offline. [EXP › Stati trasversali › Sessione scaduta]

### 3.8 Telefono, i18n, accessibilità, stili del kit

- **Telefono 390/360 px: tutto funziona, editor del grafo ed editor SQL compresi** (NFR-16). Grafo
  dall'alto in basso, **disposizione sempre automatica**; desktop da sinistra a destra con
  posizioni manuali più «Riordina». Soglia 768 px (`48rem`, A-21); orizzontale con
  `(pointer: coarse) and (max-height: 30rem)`. [EXP › Interaction Primitives › Canvas; Responsive &
  Platform] *Implicazione*: layout automatico del DAG nel client.
- i18n: italiano e inglese, dizionario sostituibile (FR-44), numeri e date con `Intl`; il
  contenitore del kit porta `lang`; il contenuto del tenant ha `lang` e `dir="auto"` propri ed è
  **sempre testo, mai HTML** (NFR-7). [EXP › Foundation; Voice and Tone]
- **WCAG 2.1 AA (NFR-9)** per kit e dashboard; editor del grafo usabile tutto da tastiera; `list-view`
  in sola lettura come alternativa al grafo (T-39); prova con NVDA e VoiceOver;
  `prefers-reduced-motion`. [EXP › Accessibility Floor]
- **Unità del kit**: «rem, base 16 px» (DES › Typography; skill §5) contro un ospite con
  `html { font-size: 62.5% }` (testo a 7 px): **T-57, architettura del kit**, prova obbligatoria a
  radice 62,5 % [C-6].
- **Isolamento CSS del kit** (Shadow DOM o reset con `:where`): focus non spento da
  `*:focus { outline: none }` dell'ospite né tagliato da `overflow: hidden`; ID unici con più
  istanze; `z-index` dei fogli rispetto alle modali dell'ospite; `forced-colors`; `color-scheme` nel
  tema scuro: **T-59, architettura**. [triage T-59]

### 3.9 Rinvii della UX all'architettura

| Id | Cosa | Quando |
|---|---|---|
| T-46 | dove si salvano le posizioni manuali (metadati OWS o archivio separato) | architettura |
| T-56 | finestra, ritardo tollerato e checkpoint delle pipeline stream (dialetto SQL chiuso FR-19…FR-22) | dopo lo spike delle pipeline stream |
| T-57 | unità del kit (rem o em) | prima della prima storia di un componente del kit |
| T-58 | librerie di canvas, layout del DAG, editor di codice | architettura |
| T-59 | isolamento CSS del kit e colori forzati | quando si sceglie come il kit applica gli stili |
| T-60 | «Esegui» durante la ripresa del motore | quando si descrive la ripresa NFR-15 |
| T-63 | contratto pubblico del kit: codici d'errore e dizionario tipizzato, evento di sessione in scadenza, proprietà `branding`, limite tentativi su Accesso, conteggio dei workflow per segreto, dove si stampa la credenziale del primo avvio, API dei temi per coppie | sezione contratti del kit |
| T-65 | prestazioni con grafi grandi | quando si fissa il limite sul numero di task (NFR-14) |
| T-66 | quarto motivo di «In coda» (partizioni del bus) | quando si scelgono le partizioni |
| T-67 | segreti nelle righe di log scritte dal codice del tenant (oscurati o limite documentato) | consegna dei segreti e log (FR-33, FR-52) |
| T-68 | dashboard con canale in tempo reale interrotto | spec della dashboard, dopo la scelta del canale |
| — | propagazione di fallito e saltato (domanda aperta 8 del PRD); forma del percorso negli errori (FR-4); meccanismo di cambio tenant e di personalizzazione | architettura |

## 4. Regole di progetto che vincolano l'architettura

### 4.1 Costituzione (1.0.0)

- **I.** Nessuna riga di codice di prodotto prima dell'architettura accettata. Ogni decisione
  architetturale dichiara il bilancio funzioni / costi / facilità di manutenzione e l'alternativa
  scartata.
- **II.** BDD prima, TDD dentro (non negoziabile): Gherkin in italiano
  (`features/<area>/<story>.feature`, `# language: it`) eseguito con `cucumber` **al confine
  pubblico del sistema**; servizi esterni simulati; scenari visti fallire prima del codice; un bug
  parte da uno scenario che lo riproduce.
- **III. Dominio esagonale puro**: il crate di dominio **non fa I/O e non dipende da runtime async,
  client HTTP né driver di database**; lo impone il grafo di Cargo (test `purity.rs` sulle
  dipendenze vietate). Porte nel dominio, adattatori fuori, **una sola composition root**: cambiare
  adattatore = una riga lì. **Gateway** = servizio esterno one-shot; **Repository** = collezione di
  aggregati posseduti. Una porta con un solo adattatore è richiesta, non superflua.
- **IV. Confini stretti**: validazione serde al confine; input non valido = errore di validazione,
  mai panic né `unwrap()` su input esterno; errori degli adattatori tradotti in errori di dominio;
  **nessun tipo di libreria esce da un adattatore**; il client HTTP vive in un solo modulo,
  imposto da `clippy.toml`; **mai loggare token, chiavi, cookie, segreti**.
- **V. Dati tipizzati**: colonne tipizzate e tabelle figlie per i dati che il sistema interpreta;
  **niente `jsonb` né TEXT JSON per dati strutturati**; l'utente applicativo del database **non è
  superuser e non ha `BYPASSRLS`** [C-5].
- **VI. Semplicità (ponytail)**: YAGNI, diff più corto, niente astrazioni monouso; scorciatoie con
  tetto noto marcate `ponytail:`; sicurezza e principi non si semplificano. **Macro** sempre
  candidate quando snelliscono (dichiarazioni al posto di cablaggio ripetuto); la logica che conta
  (denaro, sicurezza, stato) resta in funzioni semplici e testate.
- **VII. Niente Python** nel prodotto (codice, wrapper, sottoprocessi); il tooling di sviluppo bmad
  è escluso [C-11].
- **VIII.** Identificatori in inglese (tipi, funzioni, moduli, file, tabelle e colonne, valori enum,
  **campi JSON dell'API, codici d'errore**, sottocomandi, variabili d'ambiente, feature Cargo,
  branch); commenti, rustdoc, messaggi per l'utente, Gherkin, spec, documenti di prodotto, ADR e
  doc utente in italiano. Nomi da `docs/glossary.md`, concetto nuovo aggiunto prima dell'uso.
  `//!` su ogni crate e modulo, `///` su ogni elemento pubblico; `missing_docs` attivo; `cargo doc`
  senza warning fa parte del gate.
- Processo: sprint da 1–3 storie, ognuna provabile dall'utente; bmad per il prodotto, spec-kit per
  la storia (Constitution Check nel plan); un branch per lavoro, PR, Conventional Commits, semver;
  pagina di doc utente nella stessa PR.

### 4.2 CLAUDE.md

- **Accettazione dell'architettura**: approvazione dell'utente in chat **e** PR con
  `docs/architecture/<run>/ARCHITECTURE-SPINE.md` fusa in `main`; `status: final` non basta. Gli
  **spike che decidono l'architettura stanno in questa fase, prima dello sprint 1**. Ogni storia
  cita le regole AD-n della spine che rispetta.
- **ADR** in italiano, `docs/decisions/YYYY-MM-DD-<slug>.md`, intestazione `Data`, `Stato`
  (`proposta` → `accettata` → `sostituita da <file>`), `Sostituisce` facoltativo; un'ADR accettata
  non si riscrive; bilancio e alternative scartate.
- Aperto per l'architettura (CLAUDE.md «Current state»): mappatura dei DAG come estensione del DSL
  OWS (FR-1, FR-2), regola di propagazione, altri punti del PRD §11; da verificare a fondo il DSL
  completo (`dsl.md`) e l'SDK Rust; UJ-3 resta un'assunzione fino a un caso reale, prima dello spike
  stream.
- Layout del repo: `crates/` (workspace Cargo), `dashboard/` (React), `features/<area>/`,
  `specs/<NNN-slug>/`, `docs/architecture/`, `docs/decisions/`, `docs/user/`, `docs/glossary.md`
  [C-6: nessuna cartella per kit, nucleo JS, SDK TS].
- **Test**: confine degli scenari = API, CLI o SDK, **deciso dall'architettura**, nessun harness
  prima; servizi esterni simulati con **`wiremock`**; database reale con **`testcontainers`**;
  scenari che confrontano **il JSON esatto, tipi compresi** (`3` non è `3.0`); TDD sul dominio con
  test puri e veloci; le prove manuali dell'utente girano sui servizi reali.
- Servizio esterno: costi e limiti in clarify; spike sul servizio reale nel plan, stub dei test
  derivato solo da ciò che lo spike ha osservato, smoke test reale prima della PR.
- Cambio di **contratto pubblico** (API, CLI, SDK, **formato dei workflow accettato**) → commit con
  `!` se rompe, aggiornare `docs/user/`.
- Gate: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, `(cd dashboard && npm run gate)`.
  Eseguito dagli hook (`pre-commit`, `pre-push` con test a tempo limitato); **nessuna CI fino alla
  produzione**.
- **Dipendenze fissate a una versione esatta, pubblicata da almeno due settimane**; una nuova solo
  se piattaforma e librerie presenti non bastano [C-4].
- **Sviluppo locale: un solo `docker compose`, senza profili e senza restart policy.**
- Migrazioni consegnate per la prova non si modificano più: la correzione è una migrazione nuova;
  prove SQL in transazione con `ROLLBACK` o `SELECT` equivalente.
- Build: target Cargo mai in `/tmp`, una cartella di target per worktree attivo.
- Versioni `0.y.z` fino alla produzione; tag `vX.Y.Z` a fine sprint.
- Dashboard: TypeScript, solo function component, micro componenti; design system in **Storybook**;
  Container/Presentational con hook; Compound per i composti; **Provider per tema, sessione,
  lingua**; `useReducer` per stato complesso; **vanilla-extract**, un file per componente, token
  tipizzati; Vitest + Testing Library in TDD; flussi Gherkin in italiano con **Playwright
  (`playwright-bdd`)**; pagina nuova disegnata prima in Penpot; video MP4 H.264; **Biome** nel gate
  (`npm run gate`).

### 4.3 Skill `dashboard-react`

- Navigazione con **React Router**: rotte in `src/AppRoutes.tsx`, shell con `<Outlet>`; link interni
  solo `Link`/`NavLink` [gate]; uscite verso l'esterno (login, logout, consensi) solo da
  `src/navigate.ts` [gate].
- Dati solo da hook **TanStack Query** in `src/hooks/`; niente `fetch` nei componenti né richieste
  in `useEffect`; **tipi di risposta generati dai tipi del server in `src/api/generated/` con
  `ts-rs`**, mai riscritti a mano; mutazioni con la protezione CSRF decisa dall'architettura.
- Nessun Next.js, niente SWR, nessuna regola «server-». *Implicazione*: dashboard SPA lato
  client.
- Ogni area gestisce da sé caricamento (`Skeleton`, `aria-busy`), errore («Riprova») e vuoto; la
  shell non sparisce.
- Stili: solo `vars` del tema, valori dal frontmatter di DESIGN.md via `src/theme/tokens.ts` (un
  test li confronta); niente `style={…}` [gate]; **nessuna risorsa esterna, CSP `default-src
  'self'`**; **rem a base 16 px fissa**; tema chiaro e scuro.
- Testi in `src/i18n/<lingua>.ts` con le stesse chiavi, mai stringhe nel JSX; `Intl`.
- Accessibilità: elementi nativi (`button`, `Link`, `nav`, `main`), `aria-expanded`/`aria-controls`,
  focus visibile, `prefers-reduced-motion`.
- Storybook con storie chiare e scure per ogni stato; screenshot confrontati con Penpot a 1280,
  1440, 1920 e 390/360.
- La skill rimanda a una «dashboard ADR in `docs/decisions/`» che **non esiste ancora** [C-6].

## 5. Decisioni già prese nei memlog

Testo delle voci riportato com'è, con il tipo. Dove una catena di voci si è evoluta, è citata solo
la voce valida, con quella che sostituisce.

### 5.1 Memlog del brief

- «(decision) Estensione OWS per i DAG (metadata, chiavi nuove, proposta upstream) lasciata aperta
  dall'utente: nel brief come domanda aperta, la decide l'architettura»
- «(decision) Un'esecuzione può ricevere un indicatore o un elenco a seconda di come l'utente finale
  configura la pipeline: il fan-out dinamico in parallelo è un requisito, quindi l'estensione di OWS
  serve; il come resta all'architettura»
- «(decision) Preferenze tecniche dell'utente (da confermare in architettura): Apache Iggy come
  message bus tra i nodi, RustFS per messaggi troppo grandi e file, uso minimo del database»
- «(decision) Componenti obbligatori: topictoast e Iggy; RustFS opzionale (payload grandi e file).
  Risposta all'obiezione carrozzone»
- «(decision) Database necessario per lo stato: l'utente sceglie tra SQLite e Postgres. Componenti:
  topictoast + Iggy + database (SQLite incorporato o Postgres); RustFS opzionale»
- «(assumption) Da mettere alla prova nella sezione differenze: con SQLite topictoast ha 2 processi
  (topictoast + Iggy), come Hatchet con solo Postgres (motore + Postgres); la leggerezza va
  argomentata su memoria e velocità, non sul numero di componenti»
- «(event) Ricerca Iggy/RustFS: Iggy 0.10 pre-1.0, cluster non ancora maturo, log append-only senza
  ack per messaggio né riconsegna (verificato su concepts), quindi semantica di coda dei task a
  carico di topictoast; RustFS 1.0.6 GA, riparazione dati in assestamento; MinIO archiviato»
- «(decision) Punti di forza secondo l'utente: velocità, poca memoria, alta scalabilità; in più
  componenti React per integrare topictoast nella propria piattaforma»
- «(decision) Multi-tenancy: ogni cliente carica codice proprio eseguito in sandbox e vede solo lo
  storico delle proprie esecuzioni e dei propri workflow»
- «(decision) Libreria JavaScript in due strati: nucleo senza interfaccia (DSL, client del motore) e
  componenti React sopra»
- «(decision) Credenziali dei clienti (segreti, token OAuth) conservate e rinnovate da topictoast,
  consegnate ai worker solo all'esecuzione: topictoast è una cassaforte multi-tenant che esegue
  anche codice dei clienti in sandbox, la sicurezza è requisito di primo piano»
- «(decision) OWS spostato fuori dalle differenze, come scelta di progetto. Sezione 4 approvata: kit
  React, database al minimo, memoria e velocità, separazione tra clienti» — sostituisce l'ordine
  della voce «Differenze in ordine di importanza…», che metteva OWS al secondo posto.
- «(decision) Tolto il criterio dei 3 progetti esterni: in 6 mesi troppo difficile. Criteri: 500
  stelle e benchmark pubblico del carico sul database» — ultima di una catena di quattro voci
  (candidati → tolti benchmark e progetti esterni → override dell'utente che li ripristina → tolti
  i progetti esterni).
- «(decision) Taglio della prima versione approvato: dentro motore OWS con estensione DAG, retry,
  timeout, topictoast + Iggy + SQLite o Postgres, separazione tra clienti, cassaforte, sandbox,
  nucleo JS + React (editor e vista esecuzioni), dashboard; fuori nodi base, RustFS, OAuth gestito
  (solo chiavi API), utenti tipo n8n»
- «(decision) SDK della prima versione: TypeScript e Rust. Sandbox eseguita da topictoast stesso
  (deve reggere codice ostile)»
- «(event) Strade per la sandbox indicate dall'utente: nsjail, Hyperlight, Wasm,
  Firecracker/NanoVMs/unikernel; scelta rimandata a uno spike di architettura»
- «(event) Ricerca sandbox: solo wasmtime non chiede KVM né root ed è libreria Rust di produzione;
  Firecracker isolamento più forte ma KVM e demone separato; nsjail kernel condiviso; Hyperlight
  pre-1.0 con guest no_std; Nanos technical preview. Suggerito wasmtime predefinito, Firecracker
  opzionale»
- «(decision) Nodi personalizzati nella prima versione: TypeScript e Rust; Python e Go dopo» —
  restringe per la v1 la voce precedente (quattro linguaggi: Python, TypeScript, Rust, Go).
- «(decision) Licenza Apache-2.0»
- «(change) Audit memlog: aggiunta alta scalabilità alla differenza 3 (detta dall'utente, mancava) e
  quote per cliente ai requisiti di sicurezza dell'addendum»

### 5.2 Memlog della UX

- «(event) Vincoli ereditati dalla skill dashboard-react: tema chiaro e scuro, i18n it/en, rem base
  16 px, pagine disegnate prima in Penpot (file topictoast), schermi 1280/1440/1920 e telefono
  390/360, vanilla-extract con token dal frontmatter di DESIGN.md, CSP default-src self (niente
  risorse esterne, quindi niente font da CDN).»
- «(decision) Identità: un solo design system per kit React e dashboard. Il kit ha valori
  predefiniti neutri che l'ospite sovrascrive (FR-44); la dashboard usa gli stessi componenti e
  token e cambia solo accento e logo.»
- «(decision) Telefono 390/360 px: deve funzionare tutto, editor del grafo ed editor SQL delle
  pipeline stream compresi, non solo consultazione. Serve un adattamento touch del canvas e dei
  pannelli (palette, configurazione, log).»
- «(decision) Editor del grafo sul telefono: stesso canvas del desktop con zoom e trascinamento
  touch; palette e configurazione del task salgono dal basso come fogli (bottom sheet), non un
  editor a elenco separato.»
- «(decision) Navigazione del kit: un componente 'area workflow' pronto con navigazione interna
  (elenco workflow, editor, esecuzioni, segreti, pipeline stream) per chiudere UJ-1 in fretta, più
  gli stessi pezzi esportati singoli per chi vuole comporre pagine proprie.»
- «(decision) Ripresa del motore (NFR-15) nel kit: avviso neutro 'aggiornamenti in pausa,
  riprendono da soli', senza parole su crash o motore; gli stati restano fermi ma non persi. Il
  banner esplicito sulla ripresa resta solo nella dashboard dell'operatore.»
- «(decision) Direzione del grafo adattiva: da sinistra a destra su desktop, dall'alto in basso sul
  telefono; lo stesso workflow appare disposto in due modi.»
- «(decision) T-39: alternativa non visiva al grafo = vista a elenco in sola lettura (task con
  categoria, nome, stato, dipendenze); la modifica resta sul canvas, resa operabile da tastiera e
  lettore di schermo con le azioni del pannello (T-1: Collegamenti, Collega a..., Elimina task).
  Rischio noto: costruire da zero senza canvas resta più difficile.»
- «(decision) T-46: layout del grafo su desktop sia manuale (trascinamento) sia automatico (comando
  di riordino); sul telefono solo automatico. Sostituisce l'assunzione 'layout sempre automatico'.
  Le posizioni manuali del desktop vanno salvate; dove (metadata del documento OWS o archivio
  separato) lo decide l'architettura; il telefono le ignora e dispone dall'alto in basso.»
- «(decision) T-40: tutti i costrutti del sottoinsieme FR-1 si disegnano nell'editor v1: switch come
  nodo con uscite etichettate a parole e chip della condizione sull'arco (mai rosso per
  'altrimenti'), ramo non preso 'saltato' e tratteggiato; fork/for come gruppo fan-out; try/retry
  come campi del task; wait, raise, set, call http e sotto-workflow come nodi di Controllo di
  flusso.»
- «(decision) T-41: dati tra i task con una sezione 'Dati' guidata nel pannello del task (task a
  monte e campo scelti da un elenco), jq come modo avanzato; il join mostra i risultati ricevuti;
  jq non valido o con $ENV, env, input_filename, import, include dà errore con percorso e motivo
  (FR-17).»
- «(decision) T-42: pianificazioni nel kit: sezione 'Pianificazione' nel pannello del workflow con
  cron o intervallo, prossima scadenza, 'Sospendi' e testo per le scadenze recuperate dopo una
  pausa (NFR-14).»
- «(decision) T-43: (a) con la credenziale dell'integratore il kit mostra un selettore del cliente
  (solo per quel ruolo; UJ-1 lancia per il cliente di prova); (b) i workflow pubblicati come codice
  si modificano nell'editor: ogni salvataggio è una nuova versione (FR-5), con chip 'da codice' e
  avviso che la prossima pubblicazione da codice ne crea un'altra.» — parte (b) ribaltata da:
  «(override) Aggiornamento del PRD 2026-11-20, Marco: un workflow creato da codice è in sola
  lettura nell'editor; ribalta T-43 parte (b) (modificabile con chip 'da codice' e avviso). Spine
  da allineare: testo di Voice and Tone, stato vuoto/avvisi, Ampliamenti»
- «(decision) T-47: input di 'Esegui' con un modulo generato dallo schema di input del workflow
  (campo elenco che accetta testo incollato, una riga per elemento) e 'Mostra JSON' come modo
  avanzato. Richiede uno schema di input nel documento del workflow (candidato aggiornamento PRD,
  T-78).»
- «(decision) Non bloccanti accettati come consigliati: T-44 arco inattivo #8e8d95 chiaro / #74737b
  scuro (3:1), arco attivo 2 px, segmenti della barra con il primo piano dello stato (supera la
  scelta 'sotto 3:1'); T-45 il kit verifica le coppie di contrasto dei colori dell'ospite: errore
  visibile in sviluppo, avviso in console in produzione dove vince l'ospite; T-48 'in attesa' =
  dipendenze non finite, 'in coda' = pronto ma non consegnato con il motivo scritto, il PRD si
  allinea; T-49 avviso 'sandbox forte non attiva' come riga fissa nella pagina Worker; T-50 porte
  neutre (le porte escono dal tratto 'colore come significato'); T-51 pannelli agganciati con filo,
  tab Log/Input/Output in basso, palette comprimibile; T-52 'Salva' primario, 'Esegui' secondario,
  nessun passo 'pubblica' (salvare crea la versione, FR-5); T-53 la dashboard apre su Worker; T-54
  font di sistema sans e mono (chiude l'assunzione tipografia).»
- Rinvii all'architettura, ciascuno così com'è:
  - «(decision) Rinvio T-56 finestra, ritardo tollerato e checkpoint delle pipeline stream:
    architettura (dialetto SQL chiuso FR-19..22), dopo lo spike delle pipeline stream.»
  - «(decision) Rinvio T-57 unità di misura del kit incorporato (rem o em, ospite con html
    font-size 62,5%): architettura del kit, prima della prima storia di un componente, con prova a
    radice 62,5%.»
  - «(decision) Rinvio T-58 librerie di canvas, layout del DAG ed editor di codice (candidate
    @xyflow/react, dagre, CodeMirror 6): architettura, dopo T-39, T-40, T-46.»
  - «(decision) Rinvio T-59 isolamento CSS del kit e colori forzati: architettura del kit, quando
    sceglie come applica gli stili.»
  - «(decision) Rinvio T-60 'Esegui' durante la ripresa del motore: architettura, quando descrive la
    ripresa NFR-15.»
  - «(decision) Rinvio T-63 meccanismi del contratto pubblico del kit (codici errore, evento
    sessione in scadenza, proprietà branding, API temi per coppie): architettura, sezione contratti
    del kit.»
  - «(decision) Rinvio T-65 prestazioni con grafi grandi: architettura, quando si fissa il limite
    sul numero di task (NFR-14).»
  - «(decision) Rinvio T-66 quarto motivo di 'In coda' (partizioni del bus): architettura, quando
    sceglie le partizioni.»
  - «(decision) Rinvio T-67 segreti nelle righe di log scritte dal codice del cliente:
    architettura, consegna dei segreti e log (FR-33, FR-52).»
  - «(decision) Rinvio T-68 dashboard con il canale in tempo reale interrotto: spec della storia
    della dashboard, dopo la scelta del canale e della riconnessione.»
- «(decision) Requisito nuovo da Marco: ogni worker espone un manifest che dichiara, per ogni
  funzione, le connessioni esterne (piattaforme e credenziali richieste), lo schema di input e lo
  schema di output, e altri metadati ('etc etc'). È un requisito di prodotto oltre il PRD finale
  (FR-12 registrazione): candidato aggiornamento del PRD (T-83).» — accettato nel PRD come FR-55.
- «(decision) Il manifest vale anche per i nodi personalizzati caricati con la CLI (TS e Rust
  precompilati): funzioni dei worker e nodi personalizzati si presentano allo stesso modo
  nell'editor (palette, campi generati dallo schema, sezione Dati, segreti filtrati per
  connessione).»
- «(change) Spine allineate ai giri di riconciliazione e revisione dell aggiornamento del PRD
  (2026-11-20): eliminazione dei workflow con conferma e rifiuto se usati; pianificazioni del
  tenant anche su workflow dell integratore, una per workflow, con input validato e scadenze
  rifiutate visibili; chi ha agito = ruolo sempre più nome visualizzato se presente (supera
  l override precedente); avviso al salvataggio se cambia lo schema di input; lavoro non salvato
  perso alla ricarica della pagina; in coda = nessun task partito; stato attiva della pianificazione
  in DESIGN. In corso: cliente diventa tenant, tenant predefinito (selettore e colonne nascosti con
  un solo tenant), schedule nel documento accettato come pianificazione del workflow» — supera
  l'override sul «nome visualizzato».
- «(change) Spine: cliente diventa tenant (tenant-selector), tenant predefinito con selettore e
  colonne nascosti quando c e un solo tenant, pianificazione scritta nel documento mostrata in sola
  lettura nella schedule-section; i mock dicono ancora cliente: valgono le spine»

## 6. Contraddizioni e tensioni tra le fonti

- **C-1 RAM di Iggy.** Addendum §2: «circa 20 MB di RAM all'avvio (FAQ)». research-footprint §1:
  «Apache Iggy | nessun valore RAM pubblicato | n.v.». Da rimisurare nello spike.
- **C-2 Carico DB di Hatchet.** Brief e addendum: ~7.700 istruzioni SQL e ~600 KB di WAL per task
  (#5153, misura di un utente). research-footprint: «Nessun dato su WAL, query per task o scritture
  per task per nessun sistema» (non cita #5153). PRD §1.2 punto 9 riscrive il criterio: confronto
  sullo stesso carico di #5153, perché le ~7.700 istruzioni sono soprattutto polling a basso
  volume.
- **C-3 Stream processor.** Il brief non ha pipeline stream; PRD §1.2 punto 1 le porta nella v1
  (stato, finestre, watermark, checkpoint) e la UX le richiede (editor SQL, metriche di watermark e
  checkpoint, ripresa dal checkpoint). research-stream: DataFusion non fornisce stato, finestre né
  watermark, costruirli è «un progetto da solo», «lavoro da team dedicato»; Iggy non ha SQL. UJ-3 è
  un'assunzione (A-25) fino a un caso reale.
- **C-4 Versioni e regola delle due settimane.** Il server Iggy 0.10.0 è del 2026-11-07: oggi
  (2026-11-20) ha 13 giorni. *Implicazione*: fissabile dal 2026-11-21, se il crate client ha la
  stessa data (non nelle fonti). La 0.10.0 rifiuta i client precedenti e cambia il formato su disco
  senza migrazione dalla 0.9.x: scegliere la 0.9.x oggi vuol dire una migrazione dopo. RustFS 1.0.6
  è del 2026-11-15 (5 giorni; fuori v1).
  OWS 1.1.0 è del 2026-11-12 e la UX accetta solo `document.dsl` 1.1.x, ma l'SDK Rust è alpha
  1.0.0-alpha6.3 e non è allineato alla 1.1.0; gli esempi del sito dichiarano ancora `1.0.3`.
- **C-5 Costituzione V contro le fonti di prodotto.** «Niente `jsonb` né TEXT JSON per dati
  strutturati» e «utente del DB senza superuser né `BYPASSRLS`» contro: (a) l'opzione **SQLite
  incorporato** del brief (*implicazione*: SQLite non ha utenti né RLS, la regola su `BYPASSRLS`
  vale solo per Postgres); (b) dati di forma JSON nella UX: documento
  OWS, schemi di input e output del manifest con campo di estensione, posizioni dei nodi, input e
  output JSON dei task. L'architettura deve dire cosa il sistema interpreta (colonne tipizzate) e
  cosa resta opaco.
- **C-6 Regole della dashboard contro il kit incorporato.** Skill: rem a base 16 px fissa, mentre
  T-57 ammette `em` per l'ospite al 62,5 %; React Router con `Link`/`NavLink`, mentre il kit non
  scrive nella cronologia dell'ospite; elementi nativi con `main`, mentre il kit non rende mai
  `main`; CSP `default-src 'self'`, che nel kit è quella dell'ospite (*implicazione*). Il layout del
  repo in CLAUDE.md ha solo `dashboard/`: nessuna cartella per il kit, il nucleo JS e l'SDK TS. La
  skill rimanda a un'ADR della dashboard in `docs/decisions/`, che non esiste.
- **C-7 Perimetro del kit.** Brief: kit = editor dei workflow e vista delle esecuzioni (più
  configurazione di segreti e accessi OAuth in «La soluzione»; OAuth fuori v1). UX: cinque sezioni
  (anche Segreti e Pipeline stream), pianificazioni, selettore del tenant, input generato dallo
  schema, manifest. Tutto accettato nell'aggiornamento del PRD del 2026-11-20 (EXP › Elementi aperti,
  Ampliamenti 1–7).
- **C-8 Sandbox e codice ostile.** Brief: la sandbox «deve reggere codice ostile», candidata
  principale wasmtime. UX (FR-54): «Sandbox forte non attiva: KVM non disponibile; in uso la sandbox
  leggera»: senza KVM il codice dei tenant gira nella sandbox leggera. Addendum: con Wasm «un bug del
  runtime è una fuga». PRD §1.2 punto 8 tiene la leggera in un processo separato da quello con la
  chiave della cassaforte.
- **C-9 Critica a Hatchet.** Brief: «costringe a scegliere tra un database sovraccarico e RabbitMQ
  bloccato a una versione vecchia». Addendum: «superata in parte», perché Postgres come broker è
  opzionale e RabbitMQ è il default (PRD §1.2 punto 6).
- **C-10 Data di Iggy come progetto top-level.** Addendum: 2026-08-19, presentata come fatto.
  research-stream: data da fonte secondaria (hpcwire), «Verifica sul sito ASF: non verificato».
- **C-11 Costituzione VII contro la visione.** Il brief mette nella visione i nodi personalizzati in
  Python (dopo la v1). Un runtime Python spedito dentro la sandbox di topictoast andrebbe pesato
  contro «niente Python nel prodotto». Non riguarda la v1.
- **C-12 Caricamento dei nodi personalizzati (ambiguo, non contraddizione).** Brief, «il giorno in
  cui funziona»: il cliente «aggiunge un suo nodo personalizzato», senza dire dove. UX: nessuna
  superficie del kit per caricarli, solo SDK o CLI (FR-15), con nodi precompilati (PRD §1.2
  punto 7).
