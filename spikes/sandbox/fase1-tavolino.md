# Spike S-2, fase 1: a tavolino

Data: 2026-11-26. Solo lettura di fonti ufficiali: nessun prototipo, nessuna compilazione, nessuna
installazione, nessuna misura. Il piano è `PIANO.md`; i criteri vengono dalla spine (sezione «S-2 —
Sandbox», AD-6, AD-16, AD-20) e dal PRD (NFR-1, NFR-2, NFR-8, NFR-10, NFR-12, FR-15, FR-16, FR-17,
FR-50, FR-54, FR-59, SM-C2). Le «proposte» sono proposte, non decisioni: le decide Marco, e l'ADR
le fissa dopo le misure.

## Come si legge

- **verificato**: letto nella fonte indicata (documentazione ufficiale, sorgente al tag o al
  rilascio, crates.io, npm, PyPI, OSV, advisory GitHub dei repository). Il sorgente di wasmtime e
  wasmtime-wasi è quello dei crate `50.0.1` scaricati da `static.crates.io`; i numeri di riga sono
  di quei file.
- **da provare**: serve una misura (fasi 2-4) o una prova sulla macchina di prova. Ogni affermazione
  su prestazioni, memoria o avvio che non viene da una misura nostra è «da provare», anche quando un
  progetto la dichiara: i numeri dei progetti sono dichiarazioni, non risultati.
- Soglia delle due settimane: al 2026-11-26 si può fissare solo ciò che è uscito **entro il
  2026-11-12**. La verifica di `research-versions.md` è del 2026-11-20 (soglia 11-06): alcune righe
  sono cambiate (sotto, «Correzioni al quadro delle versioni»).
- Avvisi di sicurezza: OSV (`https://api.osv.dev/v1/query`) per crates.io, npm; per i programmi
  senza pacchetto (Firecracker, cloud-hypervisor) gli advisory pubblicati sul repository
  (`https://api.github.com/repos/<org>/<repo>/security-advisories`). Interrogati il 2026-11-26.

## Correzioni al quadro delle versioni

| Voce | `research-versions.md` / spine | Al 2026-11-26 |
| --- | --- | --- |
| Firecracker | v1.17.0 fissabile, v1.18.0 troppo nuova | **v1.18.0 (2026-11-09) è fissabile** (17 giorni). v1.17.0 (2026-09-10) resta il ripiego |
| cloud-hypervisor | v54.1 «solo con l'eccezione di sicurezza» | **v54.1 (2026-11-11) è fissabile senza eccezione** (15 giorni) e corregge GHSA-3r3x-6hgg-2m38. v55.0 (2026-11-19) no |
| wasmtime | 50.0.1 fissabile; 50.0.2 e 50.0.3 con eccezione | invariato: 50.0.2 (11-16) e 50.0.3 (11-19) restano dopo la soglia, e servono per gli advisory (sezione 1) |
| StarlingMonkey | v0.4.0 fissabile | v0.5.1 è del 2026-11-13, un giorno dopo la soglia: ancora no |
| ComponentizeJS | 0.23.0 fissabile | 0.24.0 è del 2026-11-25 (verificato su npm): no; 0.23.0 sì |
| Rust 1.100.0 | del 2026-11-12 | sul bordo della soglia; non serve: la toolchain fissata è 1.99.1 |

Fonti: crates.io `/api/v1/crates/<crate>/versions`; npm `registry.npmjs.org/<pacchetto>`; release
GitHub (`api.github.com/repos/<org>/<repo>/releases`).

---

## 1. Livello leggero: runtime

### Opzioni

| Opzione | Versione fissabile | Licenza | Avvisi OSV | Perché |
| --- | --- | --- | --- | --- |
| **wasmtime** | vedi sotto | Apache-2.0 WITH LLVM-exception (verificato, crates.io) | vedi sotto | punto di partenza dell'addendum del brief |
| wasmi (interprete) | 2.0.0 (2026-09-01); 2.0.1 è del 11-17, no | MIT/Apache-2.0 (crates.io) | nessuno per 2.0.0 (OSV, 2026-11-26) | unico motivo per considerarlo: niente generatore di codice nativo, quindi niente pagine eseguibili da scrivere, avvio e memoria più piccoli, e il confinamento seccomp può vietare `mprotect` con PROT_EXEC. Costo atteso: esecuzione molto più lenta (da provare). Lo propongo solo come riga di confronto in fase 2, non come candidato primario |

Non considero wasmer (MIT, 7.5.0 del 2026-10-01): nessun vantaggio verificato rispetto a wasmtime
per il nostro vincolo, e il modello di sicurezza e i limiti di risorse sono meno documentati nelle
fonti lette.

### Versione di wasmtime

Fonte: crates.io `/crates/wasmtime/versions`; OSV.

| Versione | Data | Avvisi OSV |
| --- | --- | --- |
| 50.0.1 | 2026-10-29 | 9 avvisi (sotto) |
| 50.0.2 | 2026-11-16 | nessuno |
| 50.0.3 | 2026-11-19 | nessuno |
| 51.0.0 | 2026-11-20 | nessuno |
| 49.0.5 / 48.0.8 (rami precedenti) | 2026-10-29 | stessi avvisi; correzioni in 49.0.6/49.0.7 e 48.0.9/48.0.10 |

La versione fissabile per la regola delle due settimane è la **50.0.1**, ma **gli avvisi
impongono 50.0.2 o più nuova**. Avvisi che toccano 50.0.1 (OSV `RUSTSEC-2026-0566` … `0573` e
`0587`; tutti corretti in 50.0.2, l'ultimo in 50.0.3):

- `RUSTSEC-2026-0568` (GHSA-43j2-gcp4-m22g): puntatore di base dello heap GC obsoleto, un guest Wasm
  può far abortire il processo dell'host. Rilevante per NFR-8 (esaurimento/abort dal guest).
- `RUSTSEC-2026-0566` (GHSA-5m8g-xhq4-4h93): wasmtime-wasi, crescita senza limite della memoria
  dell'host con scritture vuote su `AsyncWriteStream`. Rilevante: lo stdout/stderr del nodo è il
  canale delle righe di log (FR-59).
- `RUSTSEC-2026-0567`, `0570`, `0572`, `0573` (abort o panic in build senza alcune feature, nel
  GC heap o con flussi di componenti), `0569` (panic in `Caller::get_module_export`),
  `0571` (wasmtime-wasi-http: non lo useremo).
- `RUSTSEC-2026-0587` (GHSA-f2pq-268w-886c, high): fuga dal filesystem WASI **solo su Windows**,
  corretta in 50.0.3. Non tocca Linux.

**Proposta fase 2:** fissare **wasmtime e wasmtime-wasi 50.0.3** con l'eccezione di sicurezza
della regola delle due settimane (nel commit si citano gli RUSTSEC sopra; è l'eccezione che lo
Stack già prevede). 51.0.0 (6 giorni) no. Le due ragioni contano: lo stesso processo ausiliario che
esegue il nodo ostile è quello che gli avvisi 0568/0566 possono far abortire o gonfiare.

### Limiti di risorse (FR-17)

Tutto verificato nel sorgente `wasmtime-50.0.1`:

- **CPU deterministica, il combustibile («fuel»):** `Config::consume_fuel` (`src/config.rs:643`),
  `Store::set_fuel` (`src/runtime/store.rs:1016`), `get_fuel` (`:989`). Lo `Store` parte senza
  combustibile: va versato prima di eseguire. Esaurito, c'è una trappola. Il costo per operatore si
  regola con `operator_cost` (`config.rs:~655`).
- **Tempo, le epoche («epoch interruption»):** `Config::epoch_interruption` (`config.rs:788`),
  `Store::set_epoch_deadline` (`store.rs:1084`), `Engine::increment_epoch` (`src/engine.rs:921`).
  Un thread (o un timer) fa avanzare l'epoca; alla scadenza il guest va in trappola o cede.
- **Quale usare, dal commento di wasmtime (`config.rs:~762-780`):** le epoche sono più veloci
  («fino a 2-3×» secondo il commento; numero **da provare** da noi), il combustibile è
  l'unico deterministico. Le epoche sono pensate per guest malevoli («the deadline check cannot be
  avoided by WebAssembly code»), **ma** le istruzioni di copia in blocco (`memory.copy`) controllano
  l'epoca solo all'inizio: il tempo massimo di una fetta è il massimo tra l'intervallo e la copia più
  grande, e il commento raccomanda di **limitare la memoria dello store** per avere limiti stretti.
  Le epoche **non sono compatibili con il compilatore Winch** (`config.rs:780`).
- **Memoria:** `Store::limiter` (`store.rs:930`) con `ResourceLimiter` (`src/runtime/limits.rs:32`)
  o il pronto `StoreLimitsBuilder` (`limits.rs:219`): `memory_size` (`:238`), `table_elements`
  (`:250`), `instances` (`:260`), `tables`, `memories`, `trap_on_grow_failure` (`:294`). Con il
  pooling allocator c'è anche un tetto per slot: `PoolingAllocationConfig::max_memory_size`
  (`config.rs:4515`). Pila: `Config::max_wasm_stack` (`:857`).
- **Tempo reale:** epoche con un thread cronometro (sopra); in modalità asincrona il commento di
  wasmtime cita `tokio::time::timeout` attorno al futuro del guest. Una rete di sicurezza fuori dal
  runtime: `setrlimit` (`RLIMIT_CPU`, senza root) e il kill del processo dal motore.
- **Da provare (fase 2):** `RLIMIT_AS` e la riserva di memoria virtuale di wasmtime. Il commento del
  pooling allocator (`config.rs:~3920-3940`) dice che ogni slot di memoria lineare può riservare 4 GiB
  di spazio virtuale e che con 32k slot lo spazio virtuale si esaurisce. Un `RLIMIT_AS` stretto può
  quindi far fallire il runtime; si regola con `Config::memory_reservation` (`:1971`) e
  `memory_guard_size` (`:2054`). Se i due non vanno d'accordo, il tetto di memoria resta quello dello
  store più la misura dell'RSS fatta dal motore.

### Costo di istanziazione e pooling allocator

- **Verificato:** `Config::allocation_strategy` (`config.rs:1789`) con `PoolingAllocationConfig`
  (`:3947`). Il commento dichiara che il pooling rende l'istanziazione «più veloce e scalabile» perché
  la memoria virtuale è già configurata e può riusare mappature copy-on-write (`memory_init_cow`,
  `:2327`). Svantaggi dichiarati dal commento (`:3920-3945`): spazio virtuale per slot, memoria che
  resta paginata in uno slot non usato finché l'`Engine` non viene rilasciato, e valori massimi da
  fissare in anticipo. Regolazioni: `max_unused_warm_slots` (`:4165`, predefinito 100),
  `linear_memory_keep_resident` (`:4217`), `total_core_instances` (`:4406`), `total_memories`
  (`:4363`).
- **Conseguenza per FR-16 (inferenza, da provare):** uno slot riusato conserva pagine di una istanza
  precedente nello stesso processo. FR-16 vieta due tenant nella stessa istanza e anche «uno dopo
  l'altro» nella stessa memoria. Quindi il pooling si usa **solo dentro un processo ausiliario
  dedicato a un tenant** (M-26), mai in un processo condiviso tra tenant.
- **Da provare:** tempo di istanziazione con e senza pooling, con e senza `memory_init_cow`,
  per un modulo vuoto, javy e Rust (nessun numero di wasmtime è citato qui).

### Compilazione in anticipo e cache nativa

- **Verificato:** `Engine::precompile_module` (`src/engine.rs:726`), `precompile_component` (`:735`),
  `Module::serialize` (`module.rs:633`), `Module::deserialize` (`module.rs:422`),
  `deserialize_file` (`:471`), `Engine::detect_precompiled` (`engine.rs:329`),
  `Module::validate` (`module.rs:597`).
- **Regola di sicurezza, dalla documentazione di wasmtime (`module.rs:~395-445`):**
  `deserialize` è `unsafe` perché i byte non sono validati: un file contraffatto «può far eseguire
  codice arbitrario»; è sicuro solo con l'output **non modificato** di `serialize` o
  `precompile_module`, ed è responsabilità dell'embedder garantirlo. `deserialize_file` usa una mmap:
  il file deve restare **immutato per tutta la vita del `Module`**. Wasmtime rifiuta in modo
  deterministico gli artefatti di un'altra versione di wasmtime.
- **Proposta (coerente con la spine, AD-6 e S-2):**
  1. il tenant carica solo il modulo `.wasm` (bytecode); nessun artefatto nativo mai accettato;
  2. un processo ausiliario «compila» (`Module::validate`, poi `precompile_module`) con limiti di
     CPU, memoria e tempo, e scrive la cache in una cartella che solo topictoast scrive;
  3. la chiave della cache è l'impronta dei byte del tenant più versione di wasmtime più impronta
     della `Config` (le due ultime cambiano al primo aggiornamento);
  4. i processi che eseguono nodi hanno la cartella in sola lettura (Landlock, sezione 6) e non
     hanno mai il permesso di scrivervi; il file non si riscrive mai sul posto (si scrive un file
     nuovo con `rename`), per la regola della mmap;
  5. un controllo d'integrità (impronta salvata nel database del motore, riverificata prima di
     `deserialize_file`) per il caso di disco manomesso: **da decidere**, costa una lettura per
     avvio a freddo.
- **Da provare:** tempo e memoria di validazione e compilazione per moduli da 100 KB, 1 MB, 10 MB e
  per un modulo malevolo costruito per rallentare il compilatore (nessun combustibile per la
  compilazione: serve il limite del processo ausiliario, FR-16 «un modulo costruito per far crashare
  il validatore non ferma il motore»).

### WASI p1, p2, component model

- **Verificato (wasmtime-wasi 50.0.1):** `wasmtime_wasi::p1` (`src/p1.rs`) esegue moduli core con
  `wasi_snapshot_preview1`; «WASIp1 does not work for components» (`p1.rs`, intestazione del
  modulo). La p1 è costruita sulla p2. Per i componenti serve `wasmtime_wasi::p2`.
- **Stabilità (wasmtime, `https://docs.wasmtime.dev/stability-tiers.html`, verificato):** in Tier 1
  ci sono `component-model`, `component-model-async`, `wasi_snapshot_preview1`, `wasi-sockets`,
  `wasi-http`, `wasi-filesystem`; Cranelift e Winch.
- **Interfacce spente di default (verificato, `wasmtime-wasi-50.0.1/src/ctx.rs` e `sockets/mod.rs`):**
  `allow_tcp`, `allow_udp`, `allow_ip_name_lookup` sono `false` di default (`ctx.rs:411-430`:
  «By default this is disabled»); `socket_addr_check` di default è il rifiuto; `inherit_network`
  apre tutto (`ctx.rs:389`). Il filesystem è vuoto salvo `preopened_dir` (`ctx.rs:296`); l'ambiente
  è vuoto salvo `inherit_env`/`env` (`ctx.rs:206`, `:218`); stdout salvo `inherit_stdout`/`stdout`
  (`ctx.rs:89`, `:115`).
- **La p1 non ha `connect` (verificato, `p1.rs:2642-2680`):** esistono solo `sock_accept`, `sock_recv`,
  `sock_send`, `sock_shutdown`. Un modulo p1 non può aprire una connessione in uscita neanche con
  l'host sbagliato.
- **Proposta:** il nodo non riceve nessuna interfaccia WASI di rete e nessun filesystem. Ottiene input
  e output e le righe di log dall'host (sezione 4) e fa HTTP solo chiamando una funzione dell'host
  che passa dal filtro unico di M-30 (FR-16, «la rete della microVM passa dallo stesso filtro»).
  La WASI serve al massimo per `fd_read`/`fd_write` su tre descrittori.

### Alternative per il confinamento: nessuna root

Il livello leggero non chiede root (nessuna delle API sopra è privilegiata; Landlock, seccomp,
`setrlimit`, `PR_SET_DUMPABLE` sono senza privilegi, sezione 6). I cgroup v2 in cartella delegata
richiedono systemd o root: **non si contano** nel livello leggero.

### Proposta per la fase 2

wasmtime 50.0.3 (eccezione di sicurezza); `Config` con `consume_fuel` **e** `epoch_interruption`
confrontati; pooling allocator confrontato con l'allocazione su richiesta; moduli core (wasip1 e
senza WASI); limite di memoria con `StoreLimits`; compilazione in un processo ausiliario con la cache
sola lettura; wasmi 2.0.0 come riga di confronto del costo di avvio e della memoria (non come
candidato). Righe misurate: sezione 8.

---

## 2. TypeScript verso wasm

Il server non richiede compilatori (FR-15): la compilazione avviene dalla parte dell'integratore
(SDK o CLI, lavoro del Rust SDK/CLI e del pacchetto `sdk`). Qui ci interessa che cosa produce il
compilatore e come il nodo parla con l'host.

| | **javy** (QuickJS) | **ComponentizeJS** (StarlingMonkey, SpiderMonkey) | AssemblyScript |
| --- | --- | --- | --- |
| Versione fissabile | **v9.1.0** (2026-07-30) | **0.23.0** (2026-09-21); 0.24.0 è del 2026-11-25: no | 0.28.20 (2026-07-22) |
| Licenza | Apache-2.0 WITH LLVM-exception (repo `LICENSE.md`; research-versions) | Apache-2.0 WITH LLVM-exception (`LICENSE` nel repo; npm non la dichiara) | Apache-2.0 (npm) |
| Avvisi | nessuno (OSV: crate `javy` 8.1.0; advisory del repo: nessuno) | nessuno (OSV npm 0.23.0; advisory del repo: nessuno) | nessuno (OSV npm 0.28.20) |
| Stato dichiarato | nessun avviso di instabilità nel README v9.1.0 | «experimental project, no guarantees … breaking changes may be made» (README 0.23.0) | compilatore di un sottoinsieme di TypeScript |
| Ingresso | JavaScript (il TypeScript si compila prima in JS: tsc/esbuild, versioni **da verificare** in fase 2) | JavaScript ESM + un mondo WIT | TypeScript ristretto, senza motore JS |
| Uscita | **modulo core** `wasm32-wasip1` | **componente** (WASI 0.2) | modulo core |
| Dimensione | 1-16 KB con collegamento dinamico, almeno 869 KB con collegamento statico (README v9.1.0, verificato); il plugin `plugin.wasm.gz` pesa 508.726 byte (release v9.1.0) | motore incorporato «circa 8 MB» (README 0.23.0, verificato) | pochi KB (**da provare**) |
| Avvio e memoria | non pubblicati; **da provare** | «wizer pre-initialization»: il README dice che a runtime si esegue solo il bytecode «senza costi di inizializzazione» (dichiarazione, **da provare**); AOT con weval facoltativo | **da provare** |
| Binario dello strumento | rilascio per `x86_64-linux` e `arm-linux` (asset `javy-x86_64-linux-v9.1.0.gz`, `javy-arm-linux-v9.1.0.gz`); serve glibc ≥ 2.35 (docs `docs-using-runtime-requirements.md`) | pacchetto npm (Node.js); `weval` facoltativo | pacchetto npm |

Fonti: release v9.1.0 (`api.github.com/repos/bytecodealliance/javy/releases/tags/v9.1.0`), README e
`docs/` del tag v9.1.0; README di ComponentizeJS al tag 0.23.0; npm.

### Come il nodo ottiene input e output e chiama l'host

- **javy (verificato, docs al tag v9.1.0):** ES2023 più API parziali (`TextEncoder`/`TextDecoder`,
  `console.log` e `console.error`, `JSON`); le API di Node non ci sono
  (`docs-using-js-api-support.md`). Input e output passano da `Javy.IO.readSync`/`writeSync` su
  stdin, stdout e stderr (README e docs). Il collegamento dinamico (`-C dynamic`) chiede all'host di
  fornire le importazioni del plugin (`cabi_realloc`, `invoke`, e uno spazio di nomi che dipende
  dalla versione del plugin) e lega il modulo al bytecode di QuickJS
  (`docs-using-dynamic-linking.md`): **un cambio di versione di javy invalida i moduli dinamici**.
  Per aggiungere funzioni dell'host (per esempio `log(livello, messaggio)` per FR-59, o la scrittura
  su uno stream di AD-20/FR-18) si scrive **un plugin Rust** (`docs-using-extending.md`): il plugin
  può essere WASI p1 oppure p2 (convertito poi a `wasm32-unknown-unknown` con importazioni
  proprie). **Da provare:** quale delle due strade dà il minimo di importazioni; se `console.log`
  va su stdout e `console.error` su stderr; quanti byte e quanto tempo costa il plugin.
- **ComponentizeJS (verificato, README 0.23.0):** l'host è descritto da un mondo WIT; le funzioni
  importate (`import { log } from 'local:hello/logger'`) sono funzioni normali del mondo. È il modo
  più pulito per una `log(livello, messaggio)` tipizzata, ma richiede componenti e `wasmtime::component`
  nel processo ausiliario (sezione 4). Nel README la lista delle API della piattaforma comprende
  `fetch`, `crypto`, timer: sono presenti nel motore, ma senza le interfacce WASI di rete collegate
  non possono aprire connessioni (**da provare** che l'importazione assente fallisca all'istanziazione
  e non a metà esecuzione).
- **Segnaposto dei segreti (FR-59):** la sostituzione del segreto con il segnaposto è del **motore**,
  sulle righe che arrivano dall'ausiliario, prima di salvarle; non dipende dal compilatore. Il nodo
  non riceve mai segreti non dichiarati (FR-16).

### Altre opzioni serie

Nessuna altra con un motore JavaScript verificata: AssemblyScript è un sottoinsieme di TypeScript
senza motore JS, quindi **non** esegue «TypeScript» vero e non rispetta la promessa «TS compilato».
Lo tengo solo come riga di confronto per il pavimento di avvio e di memoria (modulo quasi vuoto),
non come candidato. Porffor, Extism JS-PDK e simili non li ho verificati: non li propongo.

### Proposta per la fase 2

javy v9.1.0 come candidato primario (modulo core, stessa forma del modulo Rust). ComponentizeJS
0.23.0 come secondo giro **solo** se il formato scelto in sezione 4 è il componente; il suo
«experimental» e gli 8 MB di motore lo rendono il candidato più rischioso per NFR-12 (64 MB per
istanza, p50 ≤ 10 ms): **da provare**. Misurare: dimensione, avvio, memoria, righe di log, input e
output di 1 KB, 100 KB, 1 MB.

---

## 3. Rust verso wasm

- **Verificato (`https://doc.rust-lang.org/rustc/platform-support.html` e le pagine dei target, lette
  il 2026-11-26 sulla documentazione corrente, quindi della 1.100; per la 1.99.1 vale la stessa
  tabella salvo prova contraria da fare con `rustc --print target-list` in fase 2):**
  `wasm32-wasip1`, `wasm32-wasip2` sono **Tier 2** (senza strumenti dell'host); `wasm32-wasip3`
  esiste. `wasm32-wasip1` produce un **modulo core** (WASI snapshot preview 1); `wasm32-wasip2`
  produce un **componente** («this target outputs a component as opposed to a core wasm module»;
  richiede un runtime con componenti).
- **Terzo target:** `wasm32-unknown-unknown` (modulo core senza WASI): le importazioni sono solo
  quelle che dichiariamo noi, superficie minima. **Da verificare** che sia Tier 2 sul 1.99.1 e come
  si dichiarano le importazioni dell'host (`extern "C"` con `#[link(wasm_import_module = "topictoast")]`).
- Il progetto già compila il dominio per `wasm32-wasip1` (gate della costituzione): la toolchain 1.99.1
  e il target sono già nel flusso.
- **Proposta fase 2:** costruire lo stesso nodo di prova (input JSON in, output JSON fuori, tre righe
  di log) in tre forme: `wasm32-wasip1` (modulo), `wasm32-unknown-unknown` (modulo, importazioni
  proprie) e `wasm32-wasip2` (componente); confrontare dimensione, avvio, memoria e comodità per
  l'autore del nodo (l'SDK Rust nasconde le importazioni).

---

## 4. Formato del modulo che il tenant carica

### Opzioni

| | **Modulo core** (`.wasm` classico) | **Componente** (component model) |
| --- | --- | --- |
| Produttori | javy, Rust `wasip1` e `unknown-unknown`, AssemblyScript, Go `wasip1`, Python (tramite il motore JS, non nativo) | ComponentizeJS, Rust `wasip2`, `componentize-py` |
| Interfaccia con l'host | funzioni importate/esportate con tipi numerici; JSON passato in memoria o su stdin/stdout | tipi WIT, ma serve il crate `wasmtime::component` (feature `component-model`, Tier 1) |
| Validazione prima della compilazione | `Module::validate`, `Engine::precompile_module` | `Engine::precompile_component` (`engine.rs:735`), `detect_precompiled` |
| WASI | p1 (modulo) | p2 |
| Strumenti per ispezionare importazioni ed esportazioni | `wasmparser` (non verificato in questa fase) | `wasm-tools` (non verificato) |

### Cosa tiene aperti Python e Go dopo la v1

- **Go (verificato, `https://go.dev/blog/wasi`):** dalla 1.21 il porting `GOOS=wasip1 GOARCH=wasm`
  produce un **modulo core** WASI p1. Il formato «modulo core con WASI p1 o senza WASI» accetta Go
  senza lavoro in più. (TinyGo non verificato.)
- **Python (verificato in parte):** `componentize-py` 0.25.1 (PyPI, 2026-09-12) produce
  **componenti**; la licenza **non è dichiarata** nei metadati PyPI (`license: null`, file
  `LICENSE` presente): **da verificare** prima di qualunque scelta. Quindi Python nel formato
  «modulo core» non esiste in modo nativo; esiste via un interprete compilato (CPython o
  MicroPython per wasi, non verificati) in un modulo.
- **La costituzione VII** (niente Python nel prodotto) si riesamina dopo la v1, come dice la spine;
  `componentize-py` è uno strumento dell'autore del nodo, come `javy`, non del server. Il server non
  esegue Python.
- **Proposta (per la fase 2, da confermare):** il formato di caricamento v1 è il **modulo core**
  validato dal server (accetta javy, Rust, Go, AssemblyScript; Python solo tramite interprete in
  modulo). Il server **accetta anche i componenti** solo se la fase 2 mostra che costano poco in
  memoria e avvio; in quel caso i tenant potranno usare ComponentizeJS, Python nativo, wasip2. Un
  campo `format` nel manifest del nodo (modulo/componente) lascia la porta aperta senza costruire
  due percorsi subito: si decide dopo la fase 2. La stessa forma dei byte vale per i due livelli
  (sezione 5).
- **Interfaccia dell'host da fissare (AD-20, FR-59):** funzioni minime `input` (lettura), `output`
  (scrittura), `log(livello, messaggio)` (tre livelli, FR-59), `emit(stream, evento)` per le scritture
  su uno stream (AD-20: «un workflow scrive su uno stream tramite una funzione o un nodo che usa
  l'SDK o l'host della sandbox»; il tenant viene dal tentativo), `http(richiesta)` per M-30. Tetti di
  righe e byte per tentativo sono del motore (FR-59, NFR-14).

---

## 5. Livello forte

### Candidati e versioni

| | **Firecracker** | **cloud-hypervisor** |
| --- | --- | --- |
| Versione fissabile | **v1.18.0** (2026-11-09); v1.17.0 (2026-09-10) ripiego | **v54.1** (2026-11-11); v55.0 (11-19) no |
| Licenza | Apache-2.0 (`LICENSE`, verificato) | Apache-2.0 e BSD-3-Clause (cartella `LICENSES`: Apache-2.0, BSD-3-Clause, CC-BY-4.0 per la documentazione; verificato). GitHub non rileva la licenza |
| Avvisi (advisory del repo, 2026-11-26) | GHSA-48ww-rqpp-2mgm (high, accesso del guest alla memoria del processo dopo un discard fallito) corretto in 1.16.2 e **1.17.0**; GHSA-776c-mpj7-jm3r (high, scrittura fuori limiti in virtio-pci) corretto in 1.14.4/1.15.1; GHSA-36j2-f825-qvgc (medium, jailer, sovrascrittura via symlink, gennaio 2026): versione corretta **non letta**. Nessuno aperto per 1.18.0 | GHSA-3r3x-6hgg-2m38 (high, use-after-free in virtio-pmem) corretto in v54.1 e v53.1; GHSA-f47p-p25q-83rh (high, virtio-block, maggio 2026) e GHSA-jmr4-g2hv-mjj6 (high, QCOW, febbraio 2026): versioni corrette **non lette** |
| Requisiti dell'host | lettura e scrittura su `/dev/kvm`; x86_64 o aarch64 (`getting-started.md`) | KVM o MSHV (README) |
| Kernel dell'host supportati | 5.10, 6.1, **6.18** (`kernel-policy.md`, v1.18.0). **La macchina di prova ha Linux 7.2/7.3: fuori dall'elenco** (da provare, nessuna garanzia del progetto) | non letto |
| Kernel del guest | 5.10, 6.1, 6.18; kernel e rootfs **non inclusi**: si prendono dalla CI di Firecracker (scaricamento) o si costruiscono (`docs/rootfs-and-kernel-setup.md`, `resources/guest_configs/`) | `vmlinux` con PVH o bzImage (x86_64), `Image` (aarch64) con avvio diretto, oppure firmware (README) |
| vsock | AF_VSOCK nel guest ↔ socket **AF_UNIX** sull'host, «bypassing vhost kernel code» (`vsock.md`); l'host apre il socket e scrive `CONNECT <porta>\n`. La stessa pagina elenca come prerequisito `CONFIG_VHOST_VSOCK=m` sull'host: **contraddizione apparente, da provare** quale modulo serve davvero | stessa interfaccia, «based on the Firecracker implementation» (`docs/vsock.md`) |
| Snapshot e ripristino | sì (`docs/snapshotting/snapshot-support.md`): la memoria si mappa con `MAP_PRIVATE` dal file (caricamento a richiesta, le scritture vanno in memoria anonima copy-on-write), le pagine di sola lettura si condividono tra microVM; il file di memoria deve restare per tutta la vita della microVM; snapshot differenziali **in anteprima per sviluppatori** | `docs/snapshot_restore.md` (non letto in dettaglio) |
| Unicità dopo il ripristino | ripristinare **più volte** lo stesso stato è «insicuro» se non si garantisce l'unicità di identificatori, semi casuali, token; Firecracker implementa **VMGenID** (kernel guest ≥ 5.18 con ACPI; su ARM serve il binding DeviceTree del 6.10 portato indietro sul 6.1) (`random-for-clones.md`) | non letto |
| Memoria della microVM | VMM «≤ 5 MiB» di overhead con 1 vCPU e 128 MiB (`SPECIFICATION.md:24-31`): **dichiarazione del progetto, da provare**; più la RAM del guest | «Low memory footprint» (README): da provare |
| Seccomp, confinamento | filtri seccomp **predefiniti** nel binario (`docs/seccomp.md`); jailer separato (chroot, namespace, cgroup, cambio di uid) | seccomp e **Landlock** integrati (`docs/seccomp.md`, `docs/landlock.md`); `docs/threat-model.md` |
| arm64 | sì, aarch64 (`getting-started.md`); kernel policy per l'host non letta per ARM; il VMGenID su ARM chiede un kernel guest con backport | sì, «main supported architectures x86-64 and AArch64» (README) |
| Superficie | minimale (pochi dispositivi virtio, niente PCI completo) | più ampia (PCI, ACPI, UEFI, VFIO, hotplug, ecc.) |

Motivo per considerare cloud-hypervisor: Landlock integrato, funzioni più ricche, stesso modello
vsock. Costo: una superficie più grande e una serie di advisory «high» in sei mesi (tre sopra),
contro due per Firecracker. **La spine lo cita come alternativa, non come primo candidato**: lo
propongo come secondo giro solo se Firecracker non funziona sul kernel 7.x della macchina di prova.

### Che cosa chiede root o capacità (verificato nelle docs di Firecracker v1.18.0)

| Funzione | Privilegio | Fonte |
| --- | --- | --- |
| Avviare Firecracker | accesso r/w a `/dev/kvm` (gruppo `kvm` o ACL: il primo accesso lo concede un amministratore **una volta**, poi nessun root a ogni avvio) | `getting-started.md:25-80` |
| **Jailer** | chroot, `pivot_root`, `mknod`, `unshare`, cgroup, cambio di uid/gid, namespace di rete (`--netns`): richiede privilegi (cartelle cgroup delegate o root) | `jailer.md:1-140` |
| **Rete tap** del guest | `ip tuntap add`, `ip addr`, `ip link`, `sysctl ip_forward`, regole `nft` di NAT: **root o CAP_NET_ADMIN** | `network-setup.md:57-90` |
| Rete: filtro dell'uscita | regole `nft` o `iptables-nft`: root | `prod-host-setup.md` (sezioni «Filtering Guest Egress Network Traffic») |
| Limiti di CPU/memoria della VMM con cgroup | cartella cgroup delegata o root | `prod-host-setup.md` (sezioni Jailer, Memory, vCPU) |
| vsock | nessun privilegio aggiunto (socket AF_UNIX nel percorso scelto) | `vsock.md` |
| seccomp della VMM | nessun privilegio (filtri incorporati, caricati dal processo) | `seccomp.md` |

La **raccomandazione del progetto è usare il jailer** (`prod-host-setup.md`). Senza jailer e senza
cgroup, il livello forte funziona ma con meno strati di difesa attorno alla VMM: **va scritto
nell'ADR** come «cosa servirebbe con root», come dice il piano.

### Che cosa può misurare la fase 3 senza sudo, e che cosa no

La macchina di prova ha `/dev/kvm` utilizzabile dall'utente, niente jailer, niente tap, solo vsock.

**Si può misurare:**
- avvio a freddo della microVM (kernel più rootfs minimo, nessuna rete) fino al «pronto» sul vsock;
- ripristino da snapshot fino alla prima risposta sul vsock; tempo di un «passaggio» dal pool al
  primo istante in cui il nodo inizia (NFR-12 forte, ≤ 10 ms);
- RSS e PSS di una microVM ferma, con RAM del guest di 64 e 128 MiB, con snapshot condiviso, per N
  microVM (il costo del pool, che FR-54 esclude da NFR-1 ma l'ADR deve riportare);
- il modulo del nodo eseguito dentro il guest, per esempio con wasmtime come agente (sotto);
- codice ostile **dentro il guest**: esaurimento di memoria, CPU, descrittori, processi; scrittura
  del disco; tentativo di rete (non c'è la scheda); lettura di file (solo quelli del rootfs);
  ciò che vede il guest della macchina (niente `/proc` dell'host);
- ripristino della stessa microVM monouso (distruggere e rifare) contro il riuso per tenant;
- declassamento: avvio senza accesso a `/dev/kvm` e verifica del ripiego sul livello leggero con
  l'avviso (FR-54);
- che il kernel host 7.x senza supporto dichiarato faccia girare Firecracker (e, se serve,
  cloud-hypervisor).

**Non si può misurare:** l'isolamento del jailer (chroot, namespace, uid, cgroup); il filtro di rete
tap e le regole di uscita (la rete della microVM che passa dal filtro di M-30 si può solo progettare:
la scheda non c'è); i limiti cgroup sulla VMM; **le fughe dalla VMM** (un exploit su virtio o sulla
VMM non lo si produce qui: la prova «fuga» a questo livello si limita a ciò che fa il guest); gli
effetti di SMT e delle mitigazioni di canale laterale (`prod-host-setup.md`, sezione «Mitigating
hardware vulnerabilities»); il rumore tra vicini; l'arm64.

### Come il livello forte esegue lo stesso formato di modulo

Il guest contiene un **agente** (programma Rust statico come PID 1, o in un initrd: `docs/initrd.md`
esiste) che ascolta sul vsock, riceve il modulo (già compilato dal processo ausiliario del motore o i
byte da compilare di nuovo), lo esegue con wasmtime e restituisce l'output e le righe di log. Così il
**formato è lo stesso** e l'ABI dell'host è la stessa: nel livello leggero le funzioni dell'host
sono funzioni del processo; nel forte sono messaggi sul vsock inoltrati dall'agente. Punti aperti:

- wasmtime dentro il guest: l'agente deve essere compilato per il guest. Il target `*-linux-musl`
  (per un agente statico minuscolo) è **Tier 3** per wasmtime (stability tiers, verificato: sia
  `x86_64-unknown-linux-musl` sia `aarch64-unknown-linux-musl` sono nell'elenco del Tier 3): **da
  provare**; l'alternativa è un guest con glibc (rootfs più grande).
- `Module::deserialize` nel guest di un artefatto prodotto dall'host: stessa regola di sicurezza
  (sezione 1); l'artefatto lo produce topictoast, il guest è monouso o riservato a un tenant.
- La **memoria condivisa dello snapshot**: lo snapshot si fa su un guest avviato **senza dati di
  tenant** e non si fa mai dopo l'esecuzione di un nodo. Le pagine di sola lettura condivise tra
  microVM di tenant diversi non sono «dati di due tenant» (contengono il kernel e l'agente), ma la
  scrittura è copy-on-write privata: **da provare** che nessuna pagina scritta sia condivisa.

### Declassamento (senza KVM)

- **Verificato nel PRD (FR-54):** «Su un host senza KVM la sandbox forte non si attiva e topictoast
  funziona con la sandbox leggera, con un avviso all'operatore nella dashboard»; FR-50: l'installazione
  predefinita non chiede KVM.
- **Conseguenze, da decidere:** (1) un tenant per cui l'integratore ha attivato la forte ottiene
  **meno isolamento** senza che lui lo veda, se il ripiego è silenzioso: l'avviso è all'operatore,
  non al tenant. (2) Il formato e l'ABI dell'host devono essere identici ai due livelli (sopra), altrimenti
  un nodo funziona in uno solo. (3) Una opzione per tenant «forte obbligatoria» (il task fallisce con
  un errore chiaro invece di ripiegare) è una **proposta**, non è nel PRD: richiede una modifica a
  FR-54 e la deve decidere Marco. (4) Il tempo di ripiego si misura (rilevamento di `/dev/kvm`
  all'avvio, non per task).

### Proposta per la fase 3

Firecracker v1.18.0 come candidato unico delle misure; cloud-hypervisor v54.1 solo come secondo giro
se Firecracker non parte sul kernel 7.x o se emerge un vincolo che lo esclude. Guest kernel e
rootfs: **non scaricare binari non verificati**; costruire il guest da `resources/guest_configs/`
oppure documentare l'impronta di ciò che si scarica (il kernel del guest è GPL-2.0: non è una
dipendenza dei lockfile coperta dalla regola delle licenze, ma **è un componente distribuito
insieme al livello forte**: l'ADR deve dire chi lo fornisce, l'integratore o noi — **da decidere**,
si lega a FR-50 «nessun componente da installare»).

---

## 6. Confinamento dei processi ausiliari

I processi ausiliari (validazione, jq, `call http`, sandbox leggera, pipeline stream) girano con lo
stesso uid del motore: le minacce da chiudere sono la lettura della chiave (file, `/proc/<motore>`,
database, configurazione, cartella di un altro tenant), i segnali, `ptrace`, e le connessioni verso
servizi interni.

### Landlock

Fonte: `https://docs.kernel.org/userspace-api/landlock.html` (documentazione del kernel corrente;
verificata il 2026-11-26).

- **Versioni ABI descritte dalla pagina, fino alla 12:** 2 (`REFER`), 3 (`TRUNCATE`), 4 (rete TCP: bind
  e connect per **numero di porta**), 5 (`IOCTL_DEV`), 6 (**scope**: socket UNIX astratti e segnali),
  7 (registro di audit), 8 (`LANDLOCK_RESTRICT_SELF_TSYNC`: applica il ruleset a tutti i thread),
  9 (socket UNIX con percorso, `RESOLVE_UNIX`), 10 (rete UDP), 11 (flag `LANDLOCK_RESTRICT_SELF_NO_NEW_PRIVS`, che lega `no_new_privs` all'applicazione del ruleset), 12 (`PERMISSION_NAMESPACE_USE`,
  `PERMISSION_CAPABILITY_USE`: namespace e capacità). Il testo non mappa le ABI sui numeri del kernel: **quale ABI ha la macchina di prova** (kernel 7.2/7.3, LSM
  `capability,landlock,lockdown,yama,bpf`) si legge in fase 4 con
  `landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)`.
- **Che cosa limita (verificato):** accessi al filesystem per gerarchia (lettura, scrittura, esecuzione,
  creazione, rimozione, `REFER`, truncate, ioctl sui dispositivi); connessioni e bind TCP **per porta**
  (non per indirizzo: **non basta** a vietare gli indirizzi interni di FR-16/M-30; vieta del tutto la
  rete a un ausiliario che non ne ha bisogno); `connect` a socket UNIX astratti e con percorso e
  l'invio di segnali fuori dal dominio; `ptrace` verso processi fuori dal dominio (sezione
  «Ptrace restrictions»); namespace e capacità (ABI 12). Una restrizione si eredita da figli ed
  `exec` e non si può togliere.
- **Che cosa non limita (la pagina lo dice):** `chdir`, `stat`, `flock`, `chmod`, `chown`, `setxattr`,
  `utime`, `fcntl`, `access` su file non coperti («it is currently not possible to restrict some
  file-related actions»). L'accesso a `/proc/<pid>` altrui si nega **non concedendo** `/proc` nel
  ruleset (le regole sono sui percorsi); un `PR_SET_DUMPABLE=0` (già in AD-6) copre `/proc/<pid>/mem`
  ed `environ` per i processi con lo stesso uid; `yama` è attivo sulla macchina di prova (ptrace
  ristretto) ma **non è garantito altrove**.
- **Requisiti:** nessun privilegio, solo `no_new_privs` (verificato, `landlock.html`: «the no_new_privs attribute is required by Landlock»). Il kernel
  deve averlo abilitato (`landlock: Up and running` in `dmesg`; verifica in fase 4 con la syscall,
  senza leggere `dmesg`, che chiede root).
- **Crate `landlock`** (verificato, crates.io): 0.4.8 del 2026-11-20 (troppo nuova), **0.4.7**
  (2026-07-27) fissabile, licenza `MIT OR Apache-2.0`, nessun avviso OSV né per 0.4.7 né per 0.4.8.
  **Da verificare:** fino a quale ABI arriva la 0.4.7 (la macchina può avere un'ABI più alta di quella
  che il crate conosce).

### seccomp

- **`seccompiler` 0.5.0** (crates.io, 2025-03-07): `Apache-2.0 OR BSD-3-Clause`, nessun avviso OSV;
  è il compilatore di filtri di Firecracker (`docs/seccompiler.md`), in Rust puro: **niente
  libseccomp**. Maturo, aggiornato poco (ultimo rilascio di 20 mesi fa): il rischio è che non
  conosca le syscall più nuove (**da provare** con un kernel 7.x).
- **`libseccomp` (crate 0.4.0, `MIT OR Apache-2.0`):** si appoggia alla libreria C `libseccomp`, che
  è **LGPL-2.1**: collegarla ricade nella regola «escluse LGPL» (lo Stack) e il binario del motore
  non può dipendere da una libreria di sistema in più: **escluso**. (Licenza della libreria C
  non riletta qui; è nota, ma va confermata nell'ADR prima di citarla come ragione.)
- `extrasafe` 0.5.1 (MIT, 2024-04) e `birdcage` (GPL-3.0-or-later: escluso dalla regola delle
  licenze) non li propongo.
- **Come:** una lista **consentita** di syscall per sottocomando, caricata dopo l'avvio e prima di
  toccare contenuto dei tenant, con `SECCOMP_FILTER_FLAG_TSYNC` per tutti i thread; azione `KILL_PROCESS`.
  Un filtro «consentito» più la rinuncia a `open`/`openat`/`socket`/`connect`, dopo che il motore ha
  passato i descrittori già aperti, dà **un confinamento senza Landlock**: la sandbox leggera non ha
  bisogno di aprire file.
- **Da provare:** quali syscall servono a wasmtime a regime (mmap/mprotect con PROT_EXEC per il codice
  generato, madvise, futex, clock, signal per i trap); lo stesso elenco con wasmi (niente PROT_EXEC).

### Ripiego dove Landlock manca

Landlock può mancare per kernel troppo vecchi (versione minima non verificata), kernel compilato senza, LSM non elencato nel parametro
`lsm=`, o container che lo blocca. Il rilevamento è la stessa syscall (ENOSYS o EOPNOTSUPP).

1. **seccomp senza `open`/`socket`/`connect`** per i sottocomandi che non ne hanno bisogno (sandbox
   leggera, validazione, jq): il confinamento resta pieno anche senza Landlock.
2. **`setrlimit`** (`RLIMIT_NOFILE`, `NPROC`, `CPU`, `FSIZE`) e `PR_SET_DUMPABLE=0` (già in AD-6),
   `PR_SET_NO_NEW_PRIVS`: senza privilegi.
3. **namespace utente, mount e rete senza root** (`unshare(CLONE_NEWUSER|CLONE_NEWNS|CLONE_NEWNET)`,
   pivot_root su una radice vuota): dà una rete vuota e un filesystem vuoto, **se** l'host consente i
   namespace utente non privilegiati (sysctl o AppArmor possono vietarli: **da provare**; sulla
   macchina di prova non lo so).
4. Se nemmeno questi bastano: **rifiutare di avviare** l'ausiliario che tocca contenuto dei tenant
   (fallisce chiuso, con errore nel registro del motore e avviso all'operatore) oppure farlo girare
   senza confinamento solo con un'opzione esplicita dell'integratore. **Proposta, da decidere.**
5. Il `call http` ha bisogno della rete: lì Landlock (porte) non basta e il filtro di M-30 (in
   processo, con controllo di ogni indirizzo risolto) è la protezione principale; il confinamento
   toglie almeno il filesystem, i segnali e i socket UNIX.

### Proposta per la fase 4

Per ogni sottocomando: Landlock (ABI massima disponibile, degradando con grazia come nell'esempio
della pagina) più seccomp con lista consentita; il motore apre per l'ausiliario ciò che serve
(descrittori) e non concede percorsi della chiave, del database o della configurazione. Crate:
`landlock` 0.4.7, `seccompiler` 0.5.0. Righe misurate: sezione 8.

---

## 7. arm64 (NFR-10): stato a tavolino, **non misurato**

Marco ha deciso (2026-11-26) di dichiararlo non misurato nell'ADR. Stato di ogni candidato:

| Candidato | Stato su arm64 | Fonte | Etichetta |
| --- | --- | --- | --- |
| wasmtime | `aarch64-unknown-linux-gnu` è **Tier 2** (manca il «continuous fuzzing» per il Tier 1); Tier 1 sono solo x86_64 Linux, macOS, Windows. `aarch64-unknown-linux-musl` Tier 3 | `docs.wasmtime.dev/stability-tiers.html` | verificato |
| wasmi | interprete in Rust puro: nessun codice nativo specifico | crates.io (descrizione) | da provare |
| javy | binario di rilascio `javy-arm-linux-v9.1.0.gz` presente | release v9.1.0 | verificato (esistenza); funzionamento da provare |
| ComponentizeJS | pacchetto npm con motore incorporato in wasm: indipendente dall'architettura; `weval` (AOT) non verificato | README 0.23.0 | da provare |
| Rust verso wasm | indipendente dall'architettura dell'host | — | verificato (target wasm) |
| Firecracker | supporta **aarch64** (kernel supportati: stessa tabella; VMGenID su ARM richiede un kernel 6.1 con il binding DeviceTree del 6.10 portato indietro) | `getting-started.md`, `random-for-clones.md` | verificato |
| cloud-hypervisor | AArch64 architettura principale (GICv3 richiesto, README) | README v54.1 | verificato |
| Landlock | architettura-indipendente (ABI del kernel) | `landlock.html` | verificato |
| seccomp / `seccompiler` | tabelle di syscall per architettura; `seccompiler` supporta x86_64 e aarch64 (Firecracker), **da provare** con le syscall del nostro filtro | `docs/seccompiler.md` | da provare |
| Il modulo cwasm della cache | **non portabile** tra architetture: la cache nativa è per architettura | `module.rs` (la versione di wasmtime e il target sono nell'artefatto) | verificato in parte (target nell'artefatto; il rifiuto tra architetture va provato) |

Nell'ADR: «arm64: non misurato; le dipendenze lo dichiarano così; prima di un rilascio arm64 si
misurano le stesse righe di NFR-12 su una macchina arm64».

---

## 8. Righe da misurare nelle fasi 2-4 (bozza)

Le soglie sono **proposte per Marco, da fissare prima delle corse**. Dove la spine o il PRD ne hanno
una, la riporto con la fonte; altrimenti lo dico. Misure solo su minisheep (x86_64, KVM, Landlock),
numero di ripetizioni e percentili dichiarati nello stesso file, causa «misurata» o «ipotesi».

### Fase 2: livello leggero

| Riga | Criterio | Che cosa si misura | Soglia |
| --- | --- | --- | --- |
| L1 | NFR-12 | tempo da «attività pronta» a «il nodo inizia», istanza fresca da cwasm in un processo caldo (wasmtime con e senza pooling; con e senza `memory_init_cow`; epoch contro fuel acceso), moduli: vuoto, javy statico, javy dinamico, Rust wasip1, Rust unknown, (ComponentizeJS se in gioco), wasmi come confronto | p50 ≤ 10 ms (NFR-12, assunzione del PRD). Riportare anche p99; **proposta** p99 ≤ 25 ms |
| L2 | NFR-12, M-26, PRD §11 n.9 | come L1 ma con processo ausiliario **per tenant** avviato a freddo (fork/exec, `Engine`, deserialize) | la spine/PRD non danno la soglia dell'avvio a freddo: **proposta** p50 ≤ 100 ms e p99 ≤ 250 ms, da confermare |
| L3 | NFR-12 | memoria per istanza durante un nodo vuoto (RSS e PSS del processo prima/dopo, N=1, 10, 50 istanze nello stesso processo di un tenant) | ≤ 64 MB per istanza (NFR-12) |
| L4 | NFR-1, M-26 | RSS di un processo ausiliario per tenant fermo, per N = 1, 5, 20 tenant; costo marginale per tenant | **proposta:** ogni ausiliario fermo ≤ 20 MB; la somma con motore e altri ausiliari attivi resta ≤ 80 MB (NFR-1) |
| L5 | FR-17 | CPU: un ciclo infinito va in trappola con l'errore che nomina il limite; precisione del limite (sforamento oltre il limite di tempo) con epoca e fuel; copia in blocco da 1 GiB (la riserva di `config.rs:~790`) | errore che nomina il limite; **proposta:** sforamento ≤ 50 ms oltre il limite |
| L6 | FR-17 | memoria: `memory.grow` oltre il tetto, tabelle, pila profonda, segmenti di dati enormi: errore che nomina il limite, processo vivo, altri tenant entro NFR-2 | errore che nomina il limite; nessun OOM del processo |
| L7 | FR-17 | tempo reale e arresto: il nodo fermato non lascia processi né memoria occupata (RSS e conteggio processi prima/dopo) | nessun residuo (FR-17) |
| L8 | FR-16 | un processo per tenant e uno slot di memoria mai riusato tra tenant: scrivere un segreto sentinella nel tenant B, leggere la memoria del processo del tenant A (`/proc/<pid>/mem`, core, `strings`) | il segreto non compare (FR-16) |
| L9 | FR-15, TS e Rust compilati girano | stesso nodo (JSON in/out, tre righe di log, `emit` su uno stream) in javy statico/dinamico, Rust wasip1/unknown/wasip2, (ComponentizeJS): funziona, dimensione, avvio, memoria, righe di log con livelli | «girano» (criterio S-2); soglie di L1 e L3 |
| L10 | FR-15 / NFR-8 | validazione e compilazione di moduli da 100 KB, 1 MB, 10 MB e di un modulo costruito per rallentare il compilatore, dentro un ausiliario con limiti: tempo, RSS massimo, il motore non si ferma | **proposta:** modulo da 1 MB validato e compilato in ≤ 1 s e ≤ 256 MB; il modulo ostile ucciso entro il tetto di tempo |
| L11 | NFR-8 (cache) | cwasm manomesso (un byte), cwasm di un'altra versione, cartella scrivibile dal nodo: rifiutato o impossibile da scrivere | nessun codice nativo non prodotto da topictoast viene eseguito |
| L12 | NFR-8, hostile light | **codice ostile noto**: fuga (filesystem, ambiente, `/proc`, socket, `fd` ereditati), esaurimento (loop, memoria, tabelle, ricorsione, righe di log enormi: FR-59), rete interna (`127.0.0.1`, `169.254.169.254`, IPv6 mappato su IPv4, nome che si risolve a indirizzo privato: tramite la funzione `http` dell'host), **lettura della chiave** (file della chiave, `/proc/<motore>/{environ,mem,maps}`, file del database, configurazione, cartella di un altro tenant) | ogni tentativo negato e registrato; il motore e gli altri tenant entro NFR-2 |

### Fase 3: livello forte

| Riga | Criterio | Che cosa si misura | Soglia |
| --- | --- | --- | --- |
| S1 | NFR-12 forte | tempo da «attività pronta» a «il nodo inizia» con una microVM libera nel pool (vsock + istanza nel guest), x86_64 | ≤ 10 ms (NFR-12) |
| S2 | FR-54 | stesso tempo con pool vuoto (avvio a freddo e ripristino da snapshot); attesa entro il limite di FR-8 | nessuna soglia nella spine; **proposta:** riportare p50/p99, ripristino ≤ 200 ms |
| S3 | FR-54 (RAM del pool fuori da NFR-1) | RSS/PSS di una microVM ferma (RAM del guest 64 e 128 MiB), per pool di 1, 5, 20; con snapshot condiviso | nessuna soglia (si riporta); **proposta:** ≤ 100 MB per microVM ferma |
| S4 | FR-16 | monouso contro riservata al tenant: costo del riciclo (distruggere e ripristinare); una microVM che ha eseguito codice di un tenant non passa mai a un altro | monouso o riservata a un tenant (FR-54), verificato con segreto sentinella nel guest |
| S5 | NFR-8, hostile strong | stesso codice ostile di L12 **dentro il guest**; fuga dalla VMM **non** misurabile; il guest non vede né rete né chiave; esaurimento (OOM nel guest, fork bomb) non tocca l'host | host e altri tenant entro NFR-2; nessun segreto dell'host raggiungibile |
| S6 | FR-17 | limiti di CPU, memoria, tempo nel guest (wasmtime nel guest + limite della microVM) e kill della VMM dal motore | errore che nomina il limite; nessun residuo |
| S7 | FR-54 | declassamento: avvio senza accesso a `/dev/kvm`, ripiego sul leggero con avviso; tempo di rilevamento | funziona senza KVM (FR-50, FR-54); avviso emesso |
| S8 | kernel 7.x | Firecracker v1.18.0 su host 7.2/7.3 (non nell'elenco 5.10/6.1/6.18) | parte e supera S1-S6; altrimenti cloud-hypervisor v54.1 |
| S9 | formato | lo stesso modulo e la stessa ABI girano nei due livelli; wasmtime nel guest (gnu contro musl: Tier 3) | stesso output byte per byte nei due livelli |

Non misurabili: jailer, tap, cgroup, fughe dalla VMM, mitigazioni dell'host, arm64.

### Fase 4: processi ausiliari

| Riga | Criterio | Che cosa si misura | Soglia |
| --- | --- | --- | --- |
| A1 | S-2 / AD-6 | ABI Landlock della macchina; per ogni sottocomando (validazione, jq, `call http`, sandbox leggera, pipeline stream) il ruleset e la lista seccomp applicati prima di toccare contenuto dei tenant | confinamento applicato prima della prima lettura di input del tenant |
| A2 | NFR-8, S-2 | **lettura della chiave per ogni sottocomando** (non solo le sandbox): file della chiave, `/proc` del motore (`environ`, `mem`, `maps`, `cmdline`), file del database, configurazione, cartella di un altro tenant; più `ptrace` e segnali verso il motore; più `PR_SET_DUMPABLE` | ogni lettura negata, per ognuno dei sottocomandi |
| A3 | AD-6 | ambiente vuoto, versione dell'IPC, uscita con versione diversa, assenza di core dump | verificato |
| A4 | S-2 (ripiego) | ripiego senza Landlock (percorso forzato in prova: seccomp più `setrlimit`, più namespace se consentiti) e rifiuto di avvio dove nemmeno questi bastano | A2 passa anche nel ripiego, o l'avvio è rifiutato |
| A5 | NFR-1, FR-50 | somma dell'RSS dei processi di topictoast con gli ausiliari attivi e con gli ausiliari fermati per inattività (60 s dopo l'avvio, nessuna esecuzione); dal 2026-11-26 il bus a parte | ≤ 80 MB (NFR-1) |
| A6 | FR-50, PRD §11 n.9 | tempo di riavvio a freddo di ogni ausiliario dopo l'arresto per inattività | obiettivo da fissare (proposta in L2) |
| A7 | SM-C2, NFR-2 | p99 di avvio delle attività a 1.000 attività/s con arresto per inattività attivo e riavvii a freddo nel carico (per esempio 1% delle richieste), contro lo stesso carico con ausiliari sempre accesi | p99 ≤ 25 ms (NFR-2) e **non peggiore** del controllo; **proposta:** differenza ≤ 2 ms, margine da fissare prima |
| A8 | FR-16 | un'istanza per tenant per jq, `call http`, validazione e validazione a secco del SQL contro riavvii a freddo di NFR-2 e NFR-1: costo in RAM e avvio con 1, 5, 20 tenant attivi | dentro A5 e A7 |
| A9 | M-27 | limite del nodo rispetto al timeout del task: il limite wasm è sempre minore del timeout del task e l'errore nomina il limite superato | verificato |

### Cosa blocca o richiede una decisione di Marco prima delle corse

Vedi l'elenco in fondo al rapporto; sintesi: eccezione di sicurezza per wasmtime 50.0.3; soglie
proposte sopra; formato (modulo core contro componente); chi fornisce kernel e rootfs del guest;
ripiego silenzioso o «forte obbligatoria»; ripiego di Landlock (rifiutare o girare senza).
