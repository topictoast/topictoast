---
title: topictoast — addendum al product brief
created: 2026-11-18
updated: 2026-11-19
---

# Addendum

Materiale che serve ai documenti successivi (PRD, architettura) e non entra nel brief. Le date nei
titoli dicono quando i fatti sono stati raccolti o verificati.

I passaggi tolti dal brief nella revisione editoriale del 2026-11-19 sono riportati così com'erano,
tra «», nella sezione pertinente, con la sezione d'origine del brief («Dal brief (…)»). I fatti già
presenti nell'addendum (per esempio wasmtime che non chiede KVM né root) non sono ripetuti.

## 1. OWS e DAG

### Ricerca: Open Workflow Specification (2026-11-18)

Fonti: open-workflow-specification.org, github.com/open-workflow-specification/specification
(`dsl.md`, `dsl-reference.md`), github.com/open-workflow-specification/sdk-rust, crates.io.
Sintesi di un subagent: le citazioni vanno ricontrollate sul testo originale prima di finire in
un'ADR.

- **Versione.** v1.1.0 stabile dal 2026-11-12; v1.0.0 dal 2025-01-27. Gli esempi del sito dichiarano
  ancora `dsl: '1.0.3'`. Politica di versione non trovata; il DSL parla di compatibilità "per tutto il
  ciclo 1.x". Licenza Apache-2.0.
- **Flusso.** Ordine di dichiarazione, salti con `then` solo dentro lo stesso blocco `do`; `fork`
  (rami concorrenti, `compete` per la gara), `switch`, `for`, `try`/`catch` con `retry`.
- **Niente DAG (verificato sul testo, 2026-11-18).** Vedi la sezione «Verifica: OWS e DAG».
- **Dati.** `input`/`output`/`export`; espressioni jq obbligatorie per ogni runtime; meccanismo dei
  segreti obbligatorio; schedule `every`/`cron`/`after`/`on`; eventi CloudEvents (`emit`/`listen`);
  timeout obbligatori.
- **Task obbligatori per un runtime conforme.** `run` (container, script, shell, workflow) e `call`
  (HTTP, gRPC, OpenAPI, AsyncAPI, A2A); MCP descritto ma non obbligatorio.
- **Conformità.** Il CTK è una suite di scenari Gherkin (`ctk/features/`) senza runner né step:
  si scrivono i passi propri (il crate `cucumber` va bene). Superarlo è SHOULD, non MUST.
- **SDK Rust.** Tre crate (core con i tipi, builders, IO YAML/JSON), Apache-2.0, alpha
  (`serverless_workflow_core` 1.0.0-alpha6.3, 2025-01-27; ultimo commit 2025-12-12), non risulta
  aggiornato alla 1.1.0, nessun motore di esecuzione.
- **Runtime esistenti.** SonataFlow, Quarkus Flow, SDK Java di riferimento (Java); Lemline, Synapse,
  EventMesh (linguaggio da verificare); Zigflow (Go, sopra Temporal: verificato, vedi la sezione
  «Zigflow»). Nessun runtime Rust ufficiale; esiste il crate comunitario `swf-runtime` (alpha,
  manutenzione da verificare).

### Verifica: OWS e DAG (2026-11-18, sul testo della spec)

Letti direttamente `dsl.md`, `dsl-reference.md` (main, DSL 1.1.0) e le issue del repo della spec.

| Capacità tipica di un DAG | OWS 1.1.0 | Fonte |
|---|---|---|
| Sequenza e rami paralleli fissi con join (serie-parallelo, diamante) | sì: `do` sequenziale, `fork` concorrente con join implicito a fine rami | `dsl-reference.md` Do e Fork; issue #1012 (maintainer) |
| Task che dipende da task di rami diversi (forma a «N», dipendenze arbitrarie) | no: i rami di un `fork` non si aspettano, `then` salta solo nello stesso blocco | `dsl-reference.md` Flow Directive: «may only redirect to tasks declared within their own scope»; #1012 |
| Fan-out dinamico in parallelo su una lista calcolata a runtime | **solo l'avvio**: `for` (sequenziale) con `run.workflow` e `await: false` lancia N sotto-workflow senza aspettarli; manca il join, che richiede eventi di ritorno. `fork.branches` è una mappa statica | issue #1143 (aperta dal 2026-01-19, stale), discussione #1133, Run (`await`) |
| Condizioni e salti di ramo | sì: `if` su ogni task, `switch` | Task, Switch |
| Cicli | `for` con `while`; `then` verso un task precedente dello stesso blocco non è vietato (deduzione: il testo non lo dice) | For, Flow Directive |
| Dipendenze via eventi (`emit` in un ramo, `listen` in un altro) | non garantito: la spec non dice che un evento dell'istanza torni a un `listen` della stessa istanza né che venga trattenuto se emesso prima | Emit, Listen, Events |

Conclusione: OWS standard copre i grafi serie-parallelo fissi. Un DAG pieno (dipendenze arbitrarie e
fan-out dinamico) richiede un'estensione del runtime. L'issue #1143 aperta è un possibile punto di
contributo verso la spec.

### Discussione #1133 (segnalata dall'utente)

https://github.com/open-workflow-specification/specification/discussions/1133, dicembre 2025.

- Un maintainer (ricardozanini) conferma: le iterazioni di `for` sono **sequenziali**.
- La spec 0.x aveva `ForEach` con modalità parallela (predefinita); la 1.x l'ha tolta.
- `for` con un `fork` dentro parallelizza i task di un elemento, non gli elementi tra loro.
- Per il fan-out dinamico (es. map-reduce su N blocchi) il maintainer propone uno schema a eventi:
  un evento per elemento, un sotto-workflow che lo ascolta, lo elabora e risponde con un evento, e il
  workflow principale che ascolta le risposte e si sincronizza. Precisa che non è lo strumento adatto
  ai big data e che serve un'infrastruttura di eventi locale al processo. L'autore della domanda ha
  un runtime con eventi locali e `fork` asincrono.
- Quindi `emit`/`listen` non è garantito dal testo della spec come meccanismo di dipendenza, ma è la
  via indicata dai maintainer, e funziona se il runtime ha un bus di eventi interno.

### Zigflow (segnalato dall'utente, verificato 2026-11-18)

Fonti: https://zigflow.dev/, github.com/zigflow/zigflow (documentazione in `docs/docs/dsl/`).

- Go, sopra Temporal; Apache-2.0; 242 stelle; v0.18.1 (2026-11-11); repo creato ad agosto 2025.
- Dichiara di supportare OWS dalla 1.0.0 e avverte che «alcuni aspetti non saranno implementati,
  potranno divergere o aggiungere altro». Esempio: `document.taskQueue` e `document.workflowType`
  obbligatori (concetti di Temporal), quindi i suoi documenti non sono OWS puri.
- **Stessi limiti sui DAG**: `for` esegue le iterazioni «nello stesso workflow, una dopo l'altra»;
  `fork` ha rami fissi, eseguiti come child workflow di Temporal; `then` resta nello stesso blocco.
- Il ciclo grande è limitato dalla history di Temporal: la documentazione consiglia di dividere in
  lotti e lanciare un child workflow per lotto con `run.workflow`.
- `run.workflow` con `await: false` esiste (solo per i workflow): avvio di N child senza aspettarli.
- `listen` mappato su query, signal e update di Temporal, non su un broker CloudEvents; nella lista
  dei task non c'è `emit`, quindi lo schema a eventi proposto in #1133 per il join non risulta
  disponibile così com'è (da verificare nel codice).

### Divergere dalla spec: dove si può (verificato sullo schema 2026-11-18)

Fonte: `schema/workflow.yaml` sul ramo `main` di open-workflow-specification/specification.

- I task sono chiusi: `unevaluatedProperties: false` (anche dentro `for`). Una chiave nuova come
  `dependsOn` su un task, o `for.parallel`, rende il documento non valido per qualsiasi validatore
  OWS: serve uno schema nostro.
- `metadata` esiste sul task e sul documento, è una mappa libera (`additionalProperties: true`):
  `metadata: { topictoast: { dependsOn: [a, b] } }` resta un documento valido.
- La spec ammette le personalizzazioni: «the DSL prioritizes customization options for extensions
  and runtimes» (dsl.md); per `$runtime.metadata` dice che usarla segnala che il workflow
  «might not be runtime agnostic».
- Ipotesi da verificare: un altro runtime ignora `metadata` ed esegue il `do` in sequenza. Il
  risultato è lo stesso solo se i task sono in ordine topologico e si passano i dati tramite
  `$context` (`export.as`), non tramite l'output del task precedente.
- `extensions` (before/after su un task) non cambia il flusso: non serve per le dipendenze.

## 2. Iggy e RustFS (ricerca 2026-11-18)

Scelte dell'utente: sezione 8. Dettaglio con fonti: ricerca salvata fuori dal repo; qui i fatti che
decidono.

**Apache Iggy**

- Progetto top-level di Apache dal 2026-08-19; versione 0.10.0 (2026-11-07), Apache-2.0, binario
  unico, circa 20 MB di RAM all'avvio (FAQ).
- Ancora 0.x: «Iggy hasn't reached a 1.0 production release yet». La 0.10.0 rifiuta i client
  precedenti e cambia il formato su disco senza migrazione dalla 0.9.x.
- Cluster (VSR) rilasciato dalla 0.9.0, spento di default, «maturing toward production readiness»;
  il README avverte che una replica che non torna blocca la partizione senza override.
- **È un log append-only come Kafka, non un broker come RabbitMQ** (verificato su
  https://iggy.apache.org/docs/introduction/concepts.md): ogni partizione va a un solo membro del
  consumer group, si legge per offset, at-least-once solo se l'offset si conferma dopo
  l'elaborazione. Nessun ack per singolo messaggio, riconsegna, timeout di visibilità o messaggi
  ritardati nella documentazione letta.
- Conseguenza per topictoast: ack, retry, timeout e ritardi dei task li deve gestire topictoast;
  con un task lento in una partizione, quelli dopo aspettano.
- SDK ufficiali: Rust, C#, Java, Python, Node.js, Go (C++, PHP, Swift in lavorazione).
- Nessun motore di workflow costruito su Iggy trovato (ricerca non esaustiva). Dalla verifica delle
  critiche ai concorrenti: nessun orchestratore lo usa come trasporto proprio (esiste solo un plugin
  Kestra).

**RustFS**

- 1.0 GA il 2026-09-16, ultima 1.0.6 (2026-11-15), Apache-2.0, 36 mila stelle, molto attivo.
- Nessun avviso «non per la produzione» nel README; le release 1.0.x correggono soprattutto la
  riparazione dei dati (heal, MRF, scanner), quindi quella parte è ancora in assestamento.
- Nodo singolo e distribuito disponibili; compatibilità S3 «for supported features».

**Alternative che un revisore citerà**: NATS JetStream (consumer pull con ack esplicito,
Apache-2.0); MinIO archiviato su GitHub («no longer maintained»), quindi RustFS ha senso come scelta
S3; SeaweedFS e Garage come altre opzioni S3.

## 3. Concorrenti e critiche verificate

### Ricerca: orchestratori comparabili (2026-11-18)

Ricerca web povera di risultati: gran parte dei dati viene dalla conoscenza del modello ed è **da
verificare**. Verificato con una fonte solo dove c'è il link (i due problemi noti, ora nella sezione
«Verifica: le critiche ai concorrenti») o il segno «verificato: Completamento» (voce confermata dal
paragrafo «Completamento della tabella dei comparabili», nella stessa sezione).

| Strumento | Linguaggio | Definizione | DAG | Persistenza | Deploy | Licenza |
|---|---|---|---|---|---|---|
| Hatchet | Go | SDK (Py/TS/Go) | sì, accanto ai task durevoli | Postgres | self-host o cloud | MIT |
| Argo Workflows | Go | YAML (CRD) | sì | etcd, SQL opzionale | solo Kubernetes | Apache-2.0 |
| Temporal | Go | SDK, workflow come codice | no | Postgres/MySQL/Cassandra | self-host o cloud | MIT (da verificare) |
| Restate | Rust | SDK | no | motore di log proprio | binario unico o cloud | BSL (verificato: Completamento) |
| Inngest | Go (da verificare) | SDK a eventi | no | verificato: Completamento | cloud prima di tutto | source-available (verificato: Completamento) |
| Dagster | Python | codice (asset) | grafo di asset | Postgres | self-host o cloud | Apache-2.0 |
| Prefect | Python | codice | implicito | Postgres/SQLite | self-host o cloud | Apache-2.0 |
| Airflow | Python | file DAG Python | sì, è il modello | Postgres/MySQL | self-host o cloud | Apache-2.0 |
| Windmill | Rust | script + flow YAML/JSON | flow con rami e cicli | Postgres | binario unico o Docker | AGPLv3 (verificato: Completamento) + enterprise (da verificare; EE: vedi sezione 5) |
| Kestra | Java | YAML | sì | Postgres/MySQL (+ Kafka/ES enterprise) | self-host o cloud | Apache-2.0 + enterprise (verificato: Completamento) |
| Conductor | Java | JSON | grafi di task | pluggable | self-host o Orkes | Apache-2.0 |

Rust: nessun orchestratore maturo, autonomo e con i DAG al centro; solo progetti piccoli (dagrs,
apalis-workflow, IronFlow, Operon, treadle).

Nessuno degli strumenti della tabella supporta OWS; i runtime OWS elencati nella sezione «Ricerca:
Open Workflow Specification» sono JVM o di nicchia (Lemline, Synapse).

**Spazio possibile** (ipotesi del ricercatore, da mettere alla prova nel brief): un binario Rust
unico con Postgres in cui il documento OWS è la definizione nativa, senza SDK in un linguaggio;
il fascino dei DAG YAML di Argo senza Kubernetes, quello di Airflow senza Python; il CTK come prova
verificabile di conformità.

**Cosa non lo renderebbe diverso** (stessa fonte): DAG + YAML + Postgres esistono già (Kestra,
Windmill, Argo); binario unico + Postgres pure (Hatchet, Windmill); Rust da solo non interessa agli
utenti; il livello DAG sarebbe un'estensione non standard di OWS, quindi «nativo OWS» vale meno di
quanto sembra; l'ecosistema OWS è piccolo. Le parti difficili (esecuzione distribuita, retry,
interfaccia, connettori) sono quelle dove Temporal, Restate e Hatchet sono avanti.

### Verifica: le critiche ai concorrenti (2026-11-18)

Fonti nella ricerca salvata fuori dal repo; i punti decisivi verificati sulla fonte primaria. Le
critiche sono la motivazione dei vincoli dell'utente (sezione 8).

| Critica | Stato oggi | Fonte |
|---|---|---|
| Hatchet dipende da RabbitMQ | **superata in parte**: Postgres può fare da broker (`SERVER_MSGQUEUE_KIND=postgres`, «This is optional»), ma il default resta RabbitMQ; hatchet-lite (un'immagine) è «for development and low-volume use-cases» | docs.hatchet.run/llms/self-hosting/docker-compose.md |
| Argo gira solo su Kubernetes | **vale**: è una CRD, nessuna modalità senza Kubernetes | github.com/argoproj/argo-workflows |
| Argo: un pod per worker | **vale in parte**: template HTTP e plugin girano in un unico pod «Agent» per workflow; ContainerSet mette più container in un pod («tens rather than hundreds») | argo-workflows.readthedocs.io |
| Temporal è pesante | **vale in parte**: quattro servizi (Frontend, History, Matching, Worker) più un database; Elasticsearch non serve più (visibility su Postgres, MySQL o SQLite); il dev server a processo unico «not intended for production use» | docs.temporal.io |

**Hatchet: RabbitMQ e Postgres come coda**

- Problema noto con fonte: Hatchet sulle code Postgres ad alto carico
  (https://hn.svelte.dev/item/43572733).
- Dal brief (Il problema, punto 1, issue hatchet-dev/hatchet#5501): «aperta: RabbitMQ 4.4 rifiuta
  le code dichiarate da Hatchet».
- Dal brief (Il problema, costo del motore, issue hatchet-dev/hatchet#5153): «Il costo del motore
  non è solo un componente in più. Senza RabbitMQ, Hatchet usa Postgres come coda e il carico sul
  database diventa sproporzionato: un utente ha misurato circa 7.700 istruzioni SQL e 600 KB di WAL
  per ogni task eseguito, 5,2 milioni di istruzioni al giorno per circa 680 task (issue
  hatchet-dev/hatchet#5153). Il maintainer non lo considera un bug: il motore deve parlare di
  continuo con il database per decidere cosa eseguire, ancora di più senza RabbitMQ; il consiglio è
  aggiungere RabbitMQ. Quindi con Hatchet si sceglie tra un database sovraccarico e un broker in
  più, bloccato a una versione vecchia.»

**Argo**: problema noto con fonte, il limite di 1 MB di etcd
(https://argo-workflows.readthedocs.io/en/latest/offloading-large-workflows/).

SDK ufficiali: Hatchet Python, TypeScript, Go, Ruby; Temporal Go, Java, .NET, PHP, Python, Ruby,
Rust, TypeScript; Restate TypeScript, Java/Kotlin, Python, Go, Rust; Inngest TypeScript, Python,
Go, Java/Kotlin; Argo Go e Python (Hera) più client della comunità.

**Completamento della tabella dei comparabili.** Restate binario unico senza dipendenze, BSL 1.1
(vieta solo di offrirlo come servizio pubblico); Windmill Postgres + server + worker, AGPLv3; Kestra
Apache-2.0, un database JDBC, Kafka/Elasticsearch solo enterprise; Inngest binario unico con SQLite
e Redis incorporato, server SSPL con passaggio ritardato ad Apache-2.0.

Payload grandi: Temporal ha già l'External Storage su S3/GCS/Azure. Per Iggy vedi la sezione 2.

### Alternative no-code open (2026-11-18)

- n8n: Sustainable Use License (non OSI), 211 mila stelle; incorporarlo richiede la licenza Embed a
  pagamento.
- Activepieces: MIT fuori dalle cartelle `ee`, 25,6 mila stelle, attivo (LICENSE letto da GitHub).
  È già l'alternativa permissiva a n8n: puntare a un terzo utente «tipo n8n» mette topictoast in
  concorrenza con Activepieces.
- Dal brief (Per chi, utenti «tipo n8n»): «i nodi di base già pronti (cicli, if/else,
  trasformazioni, connettori verso altre piattaforme, LLM) stanno nella visione.»

## 4. Interfacce incorporabili (ricerca 2026-11-18)

Domanda: esiste già, open source, un kit React per mostrare esecuzioni e DAG di un orchestratore
dentro la propria piattaforma?

| Strumento | Cosa offre da incorporare | Licenza |
|---|---|---|
| Trigger.dev | `@trigger.dev/react-hooks` 4.8.6: solo hook (stato in tempo reale), nessun componente; niente DAG né vista a grafo nella documentazione | MIT (repo Apache-2.0) |
| Inngest | `useRealtime`: solo hook | server SSPL, SDK Apache-2.0 |
| Hatchet | nessun pacchetto: la dashboard è un'app privata | MIT |
| Temporal | UI Svelte separata, nessun embedding documentato | MIT |
| Windmill | iframe, oppure licenza commerciale per riesporlo ai propri utenti | AGPLv3 |
| Kestra | `@kestra-io/topology` 0.2.0: grafo Vue, legato al design system Kestra | Apache-2.0 |
| n8n | embedding solo con licenza Embed a pagamento | Sustainable Use License |
| Conductor | `orkes-workflow-visualizer` 1.1.1: componente React con diagramma ed esecuzione; ultimo rilascio 2025-09 | Apache-2.0 (da verificare) |
| **Workflow Builder** (Synergycodes) | `@workflowbuilder/sdk` 3.0.0, editor React white-label su React Flow; `@workflowbuilder/temporal` 0.3.1 esegue i diagrammi su Temporal con vista live dell'esecuzione | Apache-2.0, 448 stelle, attivo (verificato) |

- **Lo spazio che resta**: nessun motore open source distribuisce un proprio pacchetto React mantenuto
  per stato delle esecuzioni, grafo DAG e log. Dal brief (Cosa lo rende diverso, punto 1, kit
  React): «chi offre hook (Trigger.dev, Inngest) dà solo i dati, chi ha un'interfaccia (Hatchet,
  Temporal, Argo) la tiene dentro la propria.» Il più vicino è Workflow Builder, che però dipende da
  Temporal, ha un solo nodo di partenza e niente cicli.
- Formulazione difendibile: «kit incorporabile nativo del motore, licenza permissiva, DAG al
  centro». «Non esiste niente» non regge.
- Da decidere: competere con Workflow Builder o interoperare (per esempio un suo adattatore verso
  topictoast al posto di Temporal).
- Librerie di base riusabili: `@xyflow/react` (MIT), dagre (MIT).

## 5. Separazione tra clienti, segreti, OAuth, sandbox (ricerca 2026-11-18)

Requisiti da cui parte il confronto: sezione 7. Ricerca con fonti salvata fuori dal repo; Kestra
verificato sulla pagina dei tenant («Cloud & Enterprise Edition»). Versione gratuita = OSS, a
pagamento = EE.

| Strumento (licenza) | Separazione tra clienti | Segreti | Connessioni OAuth | Sandbox |
|---|---|---|---|---|
| Hatchet (MIT) | tenant in OSS; SSO/RBAC a pagamento | nessuno per tenant (da verificare) | no | no |
| Temporal (MIT) | namespace in OSS; autorizzazione da scrivere | no | no | no |
| Windmill (AGPL + EE) | workspace in OSS (3 in Community) | cifrati per workspace in OSS; vault esterni EE | rinnovo automatico; OAuth personalizzato EE | nsjail, spento di default |
| Kestra (Apache-2.0) | tenant solo EE/Cloud | OSS: variabili d'ambiente non cifrate; vault EE | no | no |
| Trigger.dev (Apache-2.0) | da verificare | non documentati | no | container con limiti CPU/RAM |
| Activepieces (MIT + `ee`) | progetti dal piano Team | gestori di segreti solo nel piano più alto | connessioni nel nucleo | isolated-vm in OSS; codice privato a pagamento |
| n8n (Sustainable Use) | progetti, non isolamento forte | vault esterni EE | OAuth2; per utente EE | task runner separati |
| Argo (Apache-2.0) | namespace Kubernetes | Secret Kubernetes | no | no; pod root di default |

Dal brief (Cosa lo rende diverso, punto 4, separazione tra clienti): «Kestra mette i tenant nella
versione Enterprise. Hatchet e Temporal separano i clienti ma non hanno né cassaforte né sandbox.»
Per Hatchet i segreti per tenant restano da verificare (tabella).

Conclusione: nessuno offre le quattro cose della tabella insieme nella versione gratuita con
licenza permissiva. Da non presentare come lacune singole: sandbox, rinnovo OAuth e segreti per
cliente esistono già in Windmill e Activepieces. Da verificare: sandbox di Windmill per edizione,
rinnovo OAuth di n8n e Activepieces, separazione tra clienti in Trigger.dev.

## 6. Sandbox

### Strade indicate dall'utente (2026-11-19, per lo spike di architettura)

topictoast esegue il codice dei clienti nella propria sandbox (decisione del brief). Tecnologie
candidate:

- **nsjail**: namespace Linux, seccomp, cgroup; esegue qualsiasi binario; kernel condiviso.
- **Hyperlight**: micro-VM per singola chiamata di funzione, sopra un hypervisor.
- **Wasm**: runtime WebAssembly incorporato; isolamento nel processo.
- **Firecracker, NanoVMs o unikernel**: micro-VM con kernel proprio.

Criteri per scegliere: forza dell'isolamento contro codice ostile, tempo di avvio per task,
linguaggi in cui i clienti scrivono i nodi, requisiti di installazione (KVM o hypervisor
disponibile, permessi), memoria per sandbox. La scelta si fa con uno spike prima dello sprint 1.

Linguaggi dei nodi personalizzati voluti dall'utente: Python, TypeScript, Rust, Go. Con Wasm tutti
e quattro sono possibili ma con limiti sulle librerie native (da verificare per linguaggio); con
nsjail o micro-VM girano i binari normali.

### Ricerca: tecnologie di sandbox (2026-11-19)

Ricerca con fonti salvata fuori dal repo; Hyperlight verificato sui README (pre-1.0, guest `no_std`
Rust o C, hyperlight-wasm «experimental... not production-grade»).

| Tecnologia | Isolamento | Avvio | Linguaggi dei nodi | Serve KVM | Da Rust | Maturità |
|---|---|---|---|---|---|---|
| nsjail | namespace, cgroup, seccomp, kernel condiviso | numeri non documentati | qualsiasi binario Linux | no; servono user namespace, a volte root | processo C++ separato | 3.6, Apache-2.0; usato da Windmill |
| Hyperlight | VM senza kernel guest | circa 1 ms per chiamata a VM calda | `no_std` Rust o C; Wasm sperimentale | sì (KVM o MSHV) | crate `hyperlight-host` | pre-1.0, CNCF Sandbox |
| wasmtime | isolamento software nel processo | microsecondi con il pooling allocator | Rust, JS, Python, Go e altri compilati in Wasm | no, niente root | crate `wasmtime` | Tier 1 su x86_64 Linux; usato da Fastly |
| Firecracker | VM KVM con kernel guest | fino a 125 ms; meno di 5 MiB di overhead | tutto ciò che gira nel guest Linux | sì; nested virtualization limitata sui cloud | demone separato con API REST | v1.18, Apache-2.0; AWS Lambda |
| Nanos | unikernel in VM, un solo processo | non verificato | binari Linux, un processo | sì | strumento `ops` | «technical preview»; licenza commerciale oltre 50 dipendenti |

Compromessi che guidano la scelta:

1. **Isolamento contro facilità di installazione.** Solo wasmtime non chiede né KVM né root, quindi
   è l'unico che rispetta «binario unico, facile da installare».
2. **Libertà di linguaggio contro superficie d'attacco.** nsjail e Firecracker eseguono qualsiasi
   linguaggio; nsjail però condivide il kernel con la macchina che tiene i segreti di tutti. Wasm
   limita ai linguaggi compilabili in Wasm e condivide il processo: un bug del runtime è una fuga.
3. **Incorporabilità e maturità.** wasmtime è l'unica libreria Rust con stato di produzione.

Punto di partenza suggerito dalla ricerca: wasmtime come sandbox predefinita, Firecracker come
opzione rinforzata per chi vuole linguaggi arbitrari. Da verificare: lo storico delle advisory di
wasmtime e la memoria dei guest Python e JavaScript.

## 7. Requisiti di sicurezza emersi (2026-11-18, per PRD e architettura)

topictoast conserva le credenziali di più clienti ed esegue codice dei clienti in sandbox, nello
stesso sistema. Ne derivano almeno:

- isolamento tra clienti su dati, esecuzioni, storico e segreti;
- quote per cliente (task, concorrenza, risorse della sandbox), perché un cliente non saturi gli
  altri;
- una sandbox da cui il codice di un cliente non raggiunge i segreti, la rete interna o i dati
  degli altri clienti;
- segreti cifrati a riposo, consegnati al worker solo per l'esecuzione che li usa, mai nei log;
- rinnovo dei token OAuth fatto dal motore;
- chi esegue la sandbox (topictoast o chi lo integra) è ancora da decidere. Aggiornamento del
  2026-11-19: decisione del brief, la esegue topictoast (sezione «Strade indicate dall'utente»).

Come li coprono gli strumenti esistenti: sezione 5; tecnologie di sandbox: sezione 6.

## 8. Vincoli tecnici indicati dall'utente (2026-11-18, per l'architettura)

- **Apache Iggy** come message bus per passare le informazioni tra un nodo e l'altro (ricerca nella
  sezione 2).
- **RustFS** (storage a oggetti compatibile S3) per i messaggi troppo grandi e per i file (ricerca
  nella sezione 2).
- **Database al minimo**: il carico non deve stare sul database.
- Motivazione: Hatchet è un buon motore ma dipende da RabbitMQ; Argo gira solo su Kubernetes con un
  pod per worker; Temporal richiede molti componenti e appesantisce l'architettura. Lo stato di ogni
  critica è nella sezione «Verifica: le critiche ai concorrenti».
- Strade per la sandbox e linguaggi dei nodi personalizzati indicati dall'utente: sezione 6.
