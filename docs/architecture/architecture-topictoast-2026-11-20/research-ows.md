# Ricerca: stato dell'Open Workflow Specification (ex CNCF Serverless Workflow)

Digest fattuale per la run di architettura del 2026-11-20. Tutte le fonti sono state consultate il
**2026-11-20** (una sola sessione: API GitHub via `gh`, file raw al tag `v1.1.0`, API crates.io,
pagine web). I link alla specifica puntano al tag `v1.1.0`, così restano stabili.
Legenda: **[NV]** = non verificato o solo inferito, il motivo è indicato accanto.

## 0. Nome, organizzazione, repository

- Il rinomina c'è stato: issue [#1127][i1127] "Rename project to Open Workflow Specification"
  (aperta il 2025-11-10). Il primo nome scelto era "CNCF Workflow Specification", scartato "because
  of legal issues". La PR [#1155][p1155] è stata mergiata il 2026-06-30. Visto 2026-11-20.
- Org GitHub nuova: `open-workflow-specification`, creata il 2026-06-11, 17 repo pubblici
  ([org][org-new]). `github.com/serverlessworkflow/specification` e `.../sdk-rust` fanno redirect
  alla nuova org (API `gh`, `full_name` = `open-workflow-specification/...`). Visto 2026-11-20.
- Il rinomina è **parziale**. La vecchia org `serverlessworkflow` esiste ancora, con `synapse`,
  `sdk-net`, `specification-docs` e il sito vecchio ([org vecchia][org-old]). Il README della spec
  linka `open-workflow-specification/synapse`, che restituisce 404; il repo vivo è
  [serverlessworkflow/synapse][synapse]. Visto 2026-11-20.
- Sito: [open-workflow-specification.org][site]. Lo `$id` dello schema è
  `https://open-workflow-specification.org/schemas/1.1.0/workflow.yaml` ([schema L1][schema-root]).
- CNCF: progetto "Open Workflow Specification", livello **Sandbox**, accettato il July 14, 2020
  ([cncf.io][cncf]). Visto 2026-11-20.
- Identificatori vecchi deprecati, rimossi in 2.0.0: namespace eventi `io.serverlessworkflow.*`
  sostituito da `org.open-workflow-specification.*` (i runtime **MUST** supportarli entrambi per tutta
  la 1.x); host degli error type `serverlessworkflow.io` sostituito da
  `open-workflow-specification.org` ([upgrade guide, Deprecations][upg]).
- Nome della specifica: "Open Workflow Specification"; nome del linguaggio: "Open Workflow DSL"
  ([dsl.md][dsl]).

## 1. Versione del DSL, testo normativo, stabilità, estensioni, DAG

### Versione e date (visto 2026-11-20, [releases][rel])

- Versione corrente: **v1.1.0**. Commit del tag e release del **2026-11-12**
  ([release v1.1.0][rel110]).
- Storia recente. Date dei commit dei tag (`gh api repos/…/commits/<tag>`):
  - v1.0.0 il 2025-01-27;
  - v1.0.1 il 2025-04-01;
  - v1.0.2 il 2025-10-30;
  - v1.0.3 il 2026-01-12.
- Le release GitHub di v1.0.1, v1.0.2 e v1.0.3 risultano (ri)pubblicate tutte il 2026-07-31.
  v1.0.2 e v1.0.3 sono marcate *pre-release* ([releases][rel]). La guida di upgrade dice che v1.0.1
  è "the last non-pre-release" prima della 1.1.0 ([upg][upg]).
- Una sola PR aperta sulla spec al 2026-11-20: [#1219][p1219] "Clarify $secrets availability in
  expressions".

### Dove sta il testo normativo

- [`dsl.md`][dsl]: concetti (task flow, data flow, runtime expressions, errori, catalog, extension,
  eventi).
- [`dsl-reference.md`][ref]: definizioni e tabelle delle proprietà.
- [`schema/workflow.yaml`][schema]: JSON Schema draft 2020-12. Dalla 1.1.0 esiste **solo in YAML**:
  `schema/workflow.json` è stato rimosso ([upg, Breaking #3][upg]).
- Gerarchia tra i tre documenti: **[NV]**, non c'è una frase esplicita. Indizio: nella 1.1.0 le
  "Documentation corrections" allineano `dsl-reference.md` allo schema ("The schema already behaved
  this way in 1.0.1. The reference documentation was wrong"; [upg][upg]). Di fatto prevale lo schema.
  Lavoro aperto: [#1210][i1210] (2026-11-06) chiede di dire "which wins when prose and
  `schema/workflow.yaml` disagree" e di separare il testo normativo da quello informativo.
- Ombrello [#1214][i1214], aperto il 2026-11-06 da ricardozanini: allineare la spec alle pratiche
  delle specifiche industriali. "This is 1.x: **no behavior changes**". Tracce aperte:
  - [#1207][i1207] RFC 2119;
  - [#1208][i1208] classi di conformità, funzionalità core e opzionali;
  - #1209 sicurezza;
  - [#1210][i1210] terminologia e parti normative;
  - [#1211][i1211] policy di versioning e compatibilità;
  - [#1212][i1212] semantica di esecuzione (concorrenza di `fork`, cancellazione, sospensione) e
    linguaggio di espressione fissato a una versione;
  - #1213 processo.
- Incoerenze note tra testo e schema:
  - `dsl-reference.md` tipizza `do` come `map[string, task]`, ma lo schema lo definisce come
    `taskList`, cioè un array di oggetti con una sola chiave ([schema L181-190][schema-tasklist];
    [ref#do][ref-do]).
  - La sezione "Supported Protocols" di `dsl.md` non elenca MCP, mentre `dsl-reference.md` lo
    definisce ([dsl#supported-protocols][dsl-proto]; [ref#mcp-call][ref-mcp]).

### Promesse di stabilità

- [`contributing.md`][contrib]: semver guidato dalle label delle PR. `change: breaking` incrementa la
  major, `change: feature` la minor, `change: fix` la patch.
- **Contraddizione**: la release minor 1.1.0 elenca **8 "Breaking changes"** ([upg][upg]):
  - `run.shell.arguments` passa da map a `string[]`;
  - i lifecycle event `completed` usano `outputUri` al posto di `output`;
  - rimosso `workflow.json` e cambiato lo `$id`;
  - `catch.errors.with.details` diventa `detail`;
  - OAuth2 inline richiede `authority` e `grant`;
  - `a2a` e `mcp` diventano nomi riservati di `call`;
  - rinominati alcuni titoli dello schema;
  - validazione URI conforme a RFC 3986.
- Le pre-release hanno cambiato direzione: `run.script.arguments` è diventato un array in 1.0.2/1.0.3
  ed è tornato una map in 1.1.0 (#1180, #1201; [upg][upg]).
- Le versioni accettate si dichiarano con `document.dsl` (pattern semver nello schema). La guida
  chiede ai tool di accettare `1.0.x` e `1.1.0` ([schema L13-17][schema-root]; [upg, checklist][upg]).
- Una policy scritta di compatibilità e deprecazione manca. La chiede [#1211][i1211], aperta, che
  cita anche le versioni incoerenti prima della 1.1.0.
- Un maintainer annuncia una 1.2.0 "shortly after" la 1.1.0 (ricardozanini in [#1184][i1184],
  2026-08-11). Data e contenuto **[NV]**.

### Meccanismo di estensione ufficiale

- **`use.extensions`** è un array di `extension` con le proprietà `extend`, `when`, `before: task[]`
  e `after: task[]` ([ref#extension][ref-ext]; [dsl#extensions][dsl-ext]).
  - Inietta task **prima e dopo** i task esistenti (logging, mock, controlli). Con la direttiva `exit`
    può saltare il task esteso ([ref#extension][ref-ext]).
  - **Non** aggiunge tipi di task né proprietà. L'enum di `extend` è chiuso: `call, composite, emit,
    for, listen, raise, run, set, switch, try, wait, all` ([schema L1626-1628][schema-ext]). `do` e
    `fork` sono coperti solo come `composite`.
- **Custom function**: `call: <nome>` con `with: {…}` libero. Il nome deve essere diverso da
  `asyncapi`, `grpc`, `http`, `openapi`, `a2a`, `mcp` ([schema L613-614][schema-callfn]). Si definisce
  in `use.functions`, in un file `function.yaml` remoto (`call: https://…@v1.0.0`) o in un catalog
  ([dsl#custom-and-non-standard-interactions][dsl-custom]).
- **Catalog**: `use.catalogs.<nome>.endpoint`, riferimento nel formato
  `{functionName}:{functionVersion}@{catalogName}`. Esiste un catalog `default` opzionale, definito
  dal runtime. Il layout dei file è quello del repo
  [catalog][catalog] ([dsl#catalogs][dsl-cat]). Il repo catalog è fermo dal 2025-07-02
  ([org][org-new]).
- **Campi `x-`**: nessun meccanismo vendor `x-*` nella spec. La ricerca di `x-` in `dsl.md` e
  `dsl-reference.md` trova solo `application/x-www-form-urlencoded`. Visto 2026-11-20.
- **Dove un'estensione DAG può stare restando valida per lo schema ufficiale** (verificato sul file
  [`workflow.yaml`][schema]):
  - `metadata` del task: `type: object, additionalProperties: true` ([schema L226-230][schema-taskbase]);
  - `document.metadata`: libero ([schema L46-50][schema-root]);
  - una **chiave top-level sconosciuta**: la radice ha `type/required/properties` senza
    `additionalProperties` né `unevaluatedProperties` ([schema L1-6][schema-root]), quindi è accettata;
  - `call` di una custom function, con `with` libero.
- **Dove invece fallisce**:
  - una proprietà nuova su un task standard (es. `dependsOn:`) o un tipo di task nuovo. `task` ha
    `unevaluatedProperties: false` e `oneOf` sui 12 tipi ([schema L231-247][schema-task]); lo stesso
    vale per `document`, `use`, `schedule`, `evaluate` e `fork`.
  - Lo schema è stato chiuso con la PR [#928][p928] "Restrict extra props" (merge 2024-08-04).
- **Posizione dei maintainer sulle chiavi di estensione nei task**:
  - La proposta [#1184][i1184] (aperta, 2026-08-10) vuole una chiave `agent:` opzionale sui task.
    Prevede di validare il task base "after removing the extension key" e l'estensione con uno
    schema proprio, versionato a parte.
  - Risposta di cdavernas (2026-08-10): la forma di estensione va bene "only while the additional
    data is truly optional runtime metadata". Se ignorarla cambia il comportamento, propone
    "**first-class tasks** with their own schemas and conformance requirements". Commento, non testo
    della spec.
  - Per un DAG, che cambia l'ordine di esecuzione, questa posizione indica un tipo di task di prima
    classe, non una chiave di estensione. È un'inferenza.

### Costrutti DAG esistenti o proposti

- **Nessun costrutto DAG** (dipendenze tra task, `dependsOn`/`needs`, grafo) nella 1.1.0. La
  concorrenza è solo `fork` (rami statici, join implicito) ([ref#fork][ref-fork]).
- **Nessuna issue, PR o discussion propone una semantica DAG o di "parallelo con dipendenze".**
  Ricerca del 2026-11-20 sul repo spec:
  - issue con `gh search issues`, parole chiave `DAG`, `graph`, `dependsOn`, `depends on`,
    `dependencies`, `parallel`, `fork join`, `join`, `needs`. Limite 15 risultati: `parallel`,
    `depends on` e `needs` hanno toccato il limite, quindi quelle liste sono troncate;
  - PR (aperte, chiuse, mergiate) con `gh search prs`, limite 30, parole chiave `DAG`, `graph`,
    `dependsOn`, `depends on`, `dependencies`, `parallel`, `concurrent`, `join`, `needs`,
    `fan-out`. Le PR su `parallel` risalgono tutte al vecchio DSL 0.x (Parallel State, ≤ 2022),
    tranne #882, #895 e #958;
  - **tutte le 18 issue aperte** lette per titolo (`gh issue list --state open`), senza dipendere
    dalle parole chiave;
  - discussion via GraphQL, query `DAG OR parallel OR dependencies OR "fan-out"`;
  - PR aperte con `gh pr list`: una sola, #1219.
- Le più vicine:
  - [#1143][i1143] **aperta** (2026-01-19, autore geomagilles): "Fan-out / Fan-in pattern with an
    arbitrary number of tasks". `fork` richiede rami definiti a priori e `for` è sequenziale. Un
    maintainer suggerisce un `fork` dentro un `for`. Nessuna proposta, marcata stale.
  - [#1012][i1012] chiusa (2025-01-10): "Document join behavior of `fork` task". Risposta del
    maintainer: `then` non può saltare tra rami né uscire dallo scope.
  - [#875][i875] chiusa (2024-06-11): "Remove reliance on key order in `do`". Proponeva `start` e
    `then` obbligatorio. Non adottata: l'ultimo commento sposta la discussione su #894. Oggi l'ordine
    di dichiarazione resta implicito.
  - Discussion [#889][d889] chiusa: "The future of `do` and `execute`". Proponeva
    `sequentially`/`concurrently`; risolta con la PR [#895][p895] "Make do an array" (merge
    2024-06-13).
  - [#868][i868] **aperta** dal 2024-05-30: "Context Data Scoping". Cita le scritture parallele sul
    `$context`.
  - Discussion [#1090][d1090] **aperta**, 0 risposte: "Error Management after a fork".

## 2. Task, flusso, schedule, dati, espressioni, call

### Task (visto 2026-11-20)

- 12 tipi che i runtime **must** implementare: `call`, `do`, `emit`, `for`, `fork`, `listen`,
  `raise`, `run`, `set`, `switch`, `try`, `wait` ([dsl#task][dsl-task]; [ref#task][ref-task]).
- Proprietà comuni (`taskBase`): `if`, `input`, `output`, `export`, `timeout`, `then`, `metadata`
  ([ref#task][ref-task]).
  - `if` falso: il task è "skipped" e il suo output è l'input *raw*.
- I singoli tipi ([ref][ref]):
  - `do`: sotto-task in sequenza.
  - `fork`: `branches` (taskList) più `compete` (default `false`). Con `false` l'output è un array
    degli output dei rami **nell'ordine di dichiarazione**; con `true` vince un solo ramo
    ([ref#fork][ref-fork]).
  - `for`: `each`, `in` (espressione o, dalla 1.1.0, array inline), `at`, `while`, `do`
    ([ref#for][ref-for]). Iterazioni **sequenziali**: risposta di un maintainer nella discussion
    [#1133][d1133], non testo della spec.
  - `switch`: lista di case `when` + `then` (flow directive) ([ref#switch-case][ref-switch]).
  - `try`/`catch`: `errors.with`, `as`, `when`, `exceptWhen`, `retry`, `do`, e dalla 1.1.0 `then`
    ([ref#catch][ref-catch]).
  - `run`: `container`, `script` (`js` ES2024 o `python` 3.13.x), `shell`, `workflow` (subflow con
    `namespace`/`name`/`version`), più `await` e `return` ([ref#run][ref-run]). `run.script` con
    `python` è rilevante per il principio VII della costituzione.
  - `listen`: `to` (`one`/`any`/`all`, `until`), `read`, `foreach` ([ref#listen][ref-listen]).
  - `emit`: `event.with`; dalla 1.1.0 basta `type`, `id` e `source` li genera il runtime ([upg][upg]).
  - `raise`: errore RFC 7807 ([dsl#errors][dsl-err]).
  - `set`: map o espressione.
  - `wait`: duration o ISO 8601.

### Flusso e `then` (visto 2026-11-20)

- Il workflow parte dal primo task. Dopo ogni task c'è `continue` (il prossimo task in ordine di
  dichiarazione o quello indicato da `then`), `fault` o `end` ([dsl#task-flow][dsl-flow]).
- Flow directive: `continue`, `exit` (chiude lo scope corrente), `end` (chiude il workflow), oppure
  un nome di task (salto) ([ref#flow-directive][ref-flow]; [schema L1093-1101][schema-flow]).
- **Vincolo**: "Flow directives may only redirect to tasks declared within their own scope… cannot
  target tasks at a different depth" ([ref#flow-directive][ref-flow]).
- I salti **all'indietro** sono ammessi. Lo scenario CTK "Explicit Sequence Flow" salta da `setGreen`
  (terzo task) a `setBlue` (secondo) ([flow.feature][ctk-flow]). Cicli via `then`: la spec non ne
  parla **[NV]**.
- Nomi dei task: l'unicità non è scritta nella spec (ricerca di "unique" in `dsl.md` e
  `dsl-reference.md`) **[NV]**.

### Dati (visto 2026-11-20, [dsl#data-flow][dsl-data])

- Sequenza per ogni task:
  1. `if` sull'input raw;
  2. `input.schema`;
  3. `input.from`, che produce `$input`;
  4. esecuzione;
  5. `output.as`, che produce `$output`;
  6. `output.schema`;
  7. `export.as`, che produce il nuovo `$context`;
  8. `export.schema`.
- L'output trasformato di un task è l'input raw del successivo: il flusso dei dati è **lineare**.
- `$context` è unico per il workflow; `export.as` lo **sostituisce** (il default lascia il contesto
  invariato).
- Workflow: `input.schema`/`input.from` all'inizio, `output.as`/`output.schema` alla fine.
- Errori di validazione: type `…/errors/validation`.

### Espressioni (visto 2026-11-20, [dsl#runtime-expressions][dsl-expr])

- **jq** è il linguaggio di default e obbligatorio ("All runtimes **must** support"). Altri linguaggi
  sono opzionali, via `evaluate.language` ([ref#evaluate][ref-eval]). Modalità `strict` (default,
  `${ }`) o `loose`.
- JavaScript come linguaggio di espressione: lo cita solo [`comparison.md`][cmp] ("jq, JavaScript").
  La spec non lo definisce in modo normativo **[NV]**.
- Dalla 1.1.0 l'output stream di jq si mappa così: un valore resta tale, più valori diventano un
  array, nessun valore diventa `null`, un errore fa fallire l'espressione; vietato `first(...)`
  implicito.
- I runtime **must not** valutare espressioni presenti nei dati di input (#1162).
- Argomenti delle espressioni: `$context`, `$input`, `$output`, `$secrets` (solo in `input.from`),
  `$authorization`, `$task`, `$workflow`, `$runtime`.
- Un'espressione fallita produce l'errore `…/errors/expression` con status 400.

### Schedule (visto 2026-11-20, [ref#schedule][ref-sched]; [dsl#scheduling][dsl-sched])

- `every` (duration, parte anche se l'esecuzione precedente è ancora in corso), `cron`, `after`
  (attesa dopo il completamento), `on` (eventi; l'input è l'array degli eventi), più `read`
  (`data`/`envelope`/`raw`, nuovo in 1.1.0, richiede `on`).
- Dialetto cron: la spec mostra solo un esempio a 5 campi e non lo definisce **[NV]**.

### Call (visto 2026-11-20, [ref#call][ref-call])

- `http`: `method`, `endpoint`, `headers`, `body`, `query`, `output` (`content`/`response`/`raw`),
  `redirect`.
- `grpc`, `openapi`, `asyncapi` (publish/subscribe).
- `a2a`: nuovo in 1.1.0, metodi JSON-RPC A2A, `agentCard` o `server`.
- `mcp`: nuovo in 1.1.0, `protocolVersion` di default `2025-06-18`; metodi `tools/*`, `prompts/*`,
  `resources/*`; transport `http`/`stdio`; obbligo di `initialize` ([ref#mcp-call][ref-mcp]; [upg][upg]).
- Custom function (sezione 1).
- Gli errori di chiamata hanno type `…/errors/communication`.

### Altro (visto 2026-11-20)

- Lifecycle event CloudEvents (`org.open-workflow-specification.workflow.*` e `task.*`, `.v1`).
  Dalla 1.1.0 i `completed` portano `outputUri` (claim-check), perché gli eventi oltre 64 KiB non
  sono garantiti ([dsl#lifecycle-events][dsl-life]; [upg][upg]).
- Status phase: `pending`, `running`, `waiting`, `suspended`, `cancelled`, `faulted`, `completed`.
  **Non esiste una fase `skipped`** ([dsl#status-phases][dsl-phases]).

## 3. SDK Rust e altri SDK/runtime

### SDK Rust ufficiale (visto 2026-11-20)

- Repo: [open-workflow-specification/sdk-rust][rs], licenza Apache-2.0, 10 stelle, creato il
  2025-01-12.
- Crate su crates.io: [`serverless_workflow_core`][crate-core] e
  [`serverless_workflow_builders`][crate-builders]. Ultima versione **`1.0.0-alpha6.3`**, del
  **2025-01-27**. Nessuna versione stabile (`max_stable_version` = null). Nomi non cambiati dopo il
  rinomina.
- DSL tracciato: `LATEST_DSL_VERSION = "1.0.0"` ([workflow.rs L25][rs-dslver]). Il numero
  `1.0.0-alpha6.x` è una numerazione propria dell'SDK. Non supporta la 1.1.0.
- Contenuto: due crate, `core` (modelli serde, enum `TaskDefinition` `untagged` con deserializer
  custom) e `builders` (builder fluenti) ([albero][rs]).
  - Il README annuncia anche un crate **IO** e la "validation", ma nell'albero non esistono: niente
    validazione né runtime.
  - `call` è generico (`call: String`, `with: HashMap<String, Value>`); nessun modello tipizzato per
    http, grpc, openapi, a2a o mcp ([task.rs][rs-task]).
  - Nessun `deny_unknown_fields`: i campi sconosciuti verrebbero ignorati in silenzio. **[NV]**:
    inferito dal codice, non eseguito.
  - Ha un campo `metadata` top-level che la spec non prevede ([workflow.rs][rs-wf]).
- Attività:
  - ultimo commit su `main` il **2025-12-12** (PR #8, allineamento container/extension); commit nel
    2025-11 e 2025-12 non pubblicati su crates.io ([commits][rs-commits]);
  - PR [#9][rs-p9] "bring the implementation to 1.0.3 spec" **APPROVED ma non mergiata** dal
    2026-04-08;
  - PR #10 (scansione licenze) aperta dal 2026-09-11; 1 sola issue nella storia del repo.

### Altre implementazioni Rust, non ufficiali (visto 2026-11-20)

- [ejfkdev/ServerlessWorkflow-rs][swf] (crate `swf-core`, `swf-builders`, `swf-runtime`, `swf-cli`):
  - `1.0.0-alpha14` del 2026-08-12; il README dichiara DSL 1.0.3;
  - modelli, validazione, builder, runtime, grafo e Mermaid, "spec conformance tests";
  - 0 stelle, ultimo push il 2026-08-12.
  - Attenzione: `max_version` su crates.io riporta alpha9, perché le pre-release non si ordinano
    numericamente ([crates.io swf-core][crate-swf]).
- [arminhammer/jackdaw][jackdaw] (crate `jackdaw-wf` 0.2.5, del 2026-11-11):
  - motore durable in Rust che esegue anche YAML Open Workflow;
  - vendorizza i modelli di `sdk-rust` (`jackdaw/src/dsl`) e usa la spec come submodule "for the
    conformance tests";
  - ultimo push il 2026-11-18. L'autore contribuisce anche alla spec e a sdk-rust.

### Altri SDK (visto 2026-11-20)

- Go: [sdk-go][go] v4.1.0 del 2026-11-20, "align with specification 1.1.0" (PR #280). Include una
  reference implementation **parziale**: secondo la roadmap del README, `call`, `emit`, `listen`,
  `run`, `try`, `wait` e `schedule` sono ❌.
- Java: [sdk-java][java] 7.35.4.Final del 2026-11-17. La tabella del README indica 7.x conforme a
  v1.0.0. Contiene una reference implementation in `impl/` ("Full compliant" secondo il README della
  spec). Allineamento alla 1.1.0 **[NV]**.
- TypeScript: [sdk-typescript][ts] `v1.0.3-alpha8` del 2026-08-20, pre-release (DSL 1.0.3 dal tag)
  **[NV]**.
- .NET: [serverlessworkflow/sdk-net][net] v1.0.2 del 2026-04-15, ancora nella vecchia org.
- Python: [sdk-python][py], ultima release v1.0.0 del 2022-04-12 (DSL vecchio). PHP:
  [sdk-php][php] 1.0.0 del 2023-11-25.

### Runtime (visto 2026-11-20, [README spec, Runtimes][readme])

- Elencati: Apache KIE SonataFlow, Java SDK reference implementation, Quarkus Flow, Lemline e Synapse.
- Synapse ([serverlessworkflow/synapse][synapse]): ultima release `v1.0.0-alpha5.20` del 2025-08-26,
  ultimo push il 2026-10-19; vive ancora nella vecchia org.
- Versione DSL supportata da SonataFlow, Lemline e Quarkus Flow: **[NV]**.
- "Reference runtime": la spec non ne nomina uno. Il più vicino è la "Java SDK reference
  implementation", che il README della spec definisce "Full compliant" (affermazione del progetto,
  non verificata). Il Go SDK ha una reference implementation dichiarata parziale.

## 4. Conformance Test Kit (CTK)

- Esiste: [`ctk/`][ctk] nel repo della spec, versionato con la spec (tag `v1.1.0`). È composto da
  feature **Gherkin in inglese**, da usare "for both conformance testing and Behavior Driven Design"
  ([README spec][readme]; [ctk/README][ctk-readme]). Visto 2026-11-20.
- `dsl.md`: i runtime "**should** pass all the feature conformance test scenarios" (should, non
  must) ([dsl#task][dsl-task]).
- Contenuto al tag v1.1.0:
  - 12 file `.feature`: `branch`, `call`, `data-flow`, `do`, `emit`, `expression`, `flow`, `for`,
    `raise`, `set`, `switch`, `try`;
  - **27 scenari** in tutto, tutti con `dsl: '1.1.0'`;
  - `branch.feature` copre solo `fork` con `compete: true` ([features][ctk-features]).
- Step definiti nel README:
  - arrange: `Given a workflow with definition:`, `the workflow input is:`;
  - act: `When the workflow is executed`;
  - assert: `should complete/fault/cancel [with output|error]`, proprietà dell'output, e ordine dei
    task `should run first/last/before/after`, `<task> should complete/fault/cancel`
    ([ctk/README][ctk-readme]).
- Lacune riconosciute in [#1206][i1206], aperta il 2026-11-06:
  - nessuno scenario per `listen`, `run`, `wait`;
  - mancano `fork` con `compete: false`, `retry`, timeout e schedule;
  - **nessun "adapter contract"**, cioè come il runtime si collega agli step e il formato della
    traccia dei task;
  - nessun runner di riferimento;
  - dipendenze di rete (es. `petstore.swagger.io`);
  - nessun processo di claim di conformità.
  - La frase di #1206 "All scenarios hardcode `dsl: '1.0.3'`" è superata dal tag v1.1.0, dove gli
    scenari usano `1.1.0`.
- Classi di conformità (core e opzionali) e claim di conformità: da definire, [#1208][i1208] aperta.
- Esiste una richiesta di revisione della conformità di un runtime terzo: #1185 (aperta), da cui è
  nata #1206.
- Le feature sono in inglese; il loro uso con `cucumber` (Rust) richiede step binding propri **[NV]**:
  nessun binding Rust pubblico trovato, salvo l'uso interno dichiarato da jackdaw.

## 5. Dove un'estensione DAG entra in conflitto con la spec

Fatti dalla spec, ciascuno con il punto in cui un modello DAG dovrà prendere una decisione. Visto
2026-11-20.

1. **Ordine implicito.** `do` esegue in ordine di dichiarazione, perché è un array
   ([dsl#task-flow][dsl-flow]; [#875][i875]). Mettere `dependsOn` dentro un `do` cambierebbe il
   significato di un costrutto esistente. Un DAG richiede un contenitore distinguibile.
2. **`then` è un goto con scope.** Accetta un nome di task, salti in avanti e all'indietro
   ([flow.feature][ctk-flow]), `exit` ed `end` ([ref#flow-directive][ref-flow]). In un nodo con più
   predecessori, o in rami concorrenti, la semantica di `then: <task>` ed `exit` non è definita;
   `end` da un nodo richiede la cancellazione dei nodi concorrenti, anche questa non definita.
3. **Dati lineari.** L'input raw di un task è l'output del precedente ([dsl#data-flow][dsl-data]).
   Non c'è una regola per un nodo con più predecessori (join dei dati). Unico precedente: `fork` non
   competitivo restituisce un array nell'ordine di dichiarazione ([ref#fork][ref-fork]).
4. **`$context` unico e sostituito per intero** da `export.as` ([dsl#data-flow][dsl-data]). Con nodi
   concorrenti, le scritture si sovrascrivono. Per `fork` l'unica indicazione è il commento di un
   maintainer (non normativo): contesto condiviso, vince l'ultimo che scrive ([#1146][i1146]).
   Scoping del contesto: issue aperta ([#868][i868]).
5. **`fork` limitato.** Rami statici: niente fan-out dinamico ([#1143][i1143], aperta). `for`
   sequenziale ([#1133][d1133]). Con `compete: true` gli altri rami vengono cancellati e il fork resta
   `completed` (commento non normativo in [#1146][i1146]). L'errore in un ramo non è definito
   ([#1090][d1090], aperta).
6. **Skip senza fase.** Con `if` falso il task è "skipped" e l'input raw passa come output
   ([ref#task][ref-task]), ma tra le status phase non esiste `skipped` ([dsl#status-phases][dsl-phases]).
   La regola di propagazione di skip e failure ai discendenti (PRD §11) non ha appigli nella spec.
7. **Schema chiuso.** `dependsOn:` su un task standard, o un nuovo tipo di task, non passa lo schema
   ufficiale (`unevaluatedProperties: false`, `oneOf` chiuso; [schema L231-247][schema-task]).
   Passano solo `metadata`, chiavi top-level o custom `call` (sezione 1). Le `extensions` non possono
   aggiungere grammatica ([schema L1626-1628][schema-ext]).
8. **Scope dei riferimenti.** `then` vede solo i task dello stesso scope ([ref#flow-directive][ref-flow];
   [#1012][i1012]). Gli errori indicano il task con un JSON Pointer `instance` (es. `/do/getPetById`;
   [dsl#errors][dsl-err]). Le dipendenze per nome richiedono nomi univoci, e l'unicità non è scritta
   nella spec **[NV]**.
9. **Il CTK verifica l'ordine.** Gli step `should run first/last/before/after` ([ctk/README][ctk-readme])
   presuppongono una traccia di esecuzione dei task. Un runtime DAG deve produrla per far girare il
   CTK sulle parti standard.
10. **Semantica di esecuzione ancora da scrivere.** Concorrenza di `fork`, cancellazione e
    sospensione sono il perimetro di [#1212][i1212] (aperta, "document existing behavior only; no
    new requirements in 1.x"). Le scelte di topictoast su questi punti potranno divergere da quanto
    il progetto scriverà poi.
11. **Linea dei maintainer.** Ciò che cambia il comportamento va in un task di prima classe, non in
    una chiave di estensione (commento in [#1184][i1184]; vedi sezione 1).

<!-- Riferimenti (tutti consultati il 2026-11-20) -->
[org-new]: https://github.com/open-workflow-specification
[org-old]: https://github.com/serverlessworkflow
[site]: https://open-workflow-specification.org/
[cncf]: https://www.cncf.io/projects/open-workflow-specification/
[readme]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/README.md
[rel]: https://github.com/open-workflow-specification/specification/releases
[rel110]: https://github.com/open-workflow-specification/specification/releases/tag/v1.1.0
[upg]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/docs/upgrade-to-1.1.0.md
[contrib]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/contributing.md#versioning
[cmp]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/comparison.md
[dsl]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md
[dsl-task]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#task
[dsl-phases]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#status-phases
[dsl-life]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#lifecycle-events
[dsl-sched]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#scheduling
[dsl-flow]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#task-flow
[dsl-data]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#data-flow
[dsl-expr]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#runtime-expressions
[dsl-err]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#errors
[dsl-cat]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#catalogs
[dsl-proto]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#supported-protocols
[dsl-custom]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#custom-and-non-standard-interactions
[dsl-ext]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl.md#extensions
[ref]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md
[ref-task]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#task
[ref-call]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#call
[ref-mcp]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#mcp-call
[ref-do]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#do
[ref-for]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#for
[ref-fork]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#fork
[ref-listen]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#listen
[ref-run]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#run
[ref-switch]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#switch-case
[ref-catch]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#catch
[ref-flow]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#flow-directive
[ref-ext]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#extension
[ref-sched]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#schedule
[ref-eval]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/dsl-reference.md#evaluate
[schema]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml
[schema-root]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml#L1-L50
[schema-tasklist]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml#L181-L190
[schema-taskbase]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml#L192-L230
[schema-task]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml#L231-L247
[schema-callfn]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml#L606-L620
[schema-flow]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml#L1093-L1101
[schema-ext]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/schema/workflow.yaml#L1620-L1643
[ctk]: https://github.com/open-workflow-specification/specification/tree/v1.1.0/ctk
[ctk-readme]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/ctk/README.md
[ctk-features]: https://github.com/open-workflow-specification/specification/tree/v1.1.0/ctk/features
[ctk-flow]: https://github.com/open-workflow-specification/specification/blob/v1.1.0/ctk/features/flow.feature
[catalog]: https://github.com/open-workflow-specification/catalog
[i1127]: https://github.com/open-workflow-specification/specification/issues/1127
[p1155]: https://github.com/open-workflow-specification/specification/pull/1155
[p1219]: https://github.com/open-workflow-specification/specification/pull/1219
[i1143]: https://github.com/open-workflow-specification/specification/issues/1143
[i1012]: https://github.com/open-workflow-specification/specification/issues/1012
[i875]: https://github.com/open-workflow-specification/specification/issues/875
[p895]: https://github.com/open-workflow-specification/specification/pull/895
[i868]: https://github.com/open-workflow-specification/specification/issues/868
[i1146]: https://github.com/open-workflow-specification/specification/issues/1146
[i1206]: https://github.com/open-workflow-specification/specification/issues/1206
[i1184]: https://github.com/open-workflow-specification/specification/issues/1184
[i1207]: https://github.com/open-workflow-specification/specification/issues/1207
[i1208]: https://github.com/open-workflow-specification/specification/issues/1208
[i1210]: https://github.com/open-workflow-specification/specification/issues/1210
[i1211]: https://github.com/open-workflow-specification/specification/issues/1211
[i1212]: https://github.com/open-workflow-specification/specification/issues/1212
[i1214]: https://github.com/open-workflow-specification/specification/issues/1214
[p928]: https://github.com/open-workflow-specification/specification/pull/928
[d889]: https://github.com/open-workflow-specification/specification/discussions/889
[d1090]: https://github.com/open-workflow-specification/specification/discussions/1090
[d1133]: https://github.com/open-workflow-specification/specification/discussions/1133
[rs]: https://github.com/open-workflow-specification/sdk-rust
[rs-commits]: https://github.com/open-workflow-specification/sdk-rust/commits/main
[rs-p9]: https://github.com/open-workflow-specification/sdk-rust/pull/9
[rs-dslver]: https://github.com/open-workflow-specification/sdk-rust/blob/main/core/src/models/workflow.rs#L25
[rs-wf]: https://github.com/open-workflow-specification/sdk-rust/blob/main/core/src/models/workflow.rs
[rs-task]: https://github.com/open-workflow-specification/sdk-rust/blob/main/core/src/models/task.rs
[crate-core]: https://crates.io/crates/serverless_workflow_core
[crate-builders]: https://crates.io/crates/serverless_workflow_builders
[swf]: https://github.com/ejfkdev/ServerlessWorkflow-rs
[crate-swf]: https://crates.io/crates/swf-core
[jackdaw]: https://github.com/arminhammer/jackdaw
[go]: https://github.com/open-workflow-specification/sdk-go
[java]: https://github.com/open-workflow-specification/sdk-java
[ts]: https://github.com/open-workflow-specification/sdk-typescript/releases
[net]: https://github.com/serverlessworkflow/sdk-net
[py]: https://github.com/open-workflow-specification/sdk-python
[php]: https://github.com/open-workflow-specification/sdk-php
[synapse]: https://github.com/serverlessworkflow/synapse
