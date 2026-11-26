# Spike S-2, fase 4: processi ausiliari confinati, risultati

Data: 2026-11-26. Misure solo su minisheep (x86_64, Intel Core Ultra 9 185H, 22 core, kernel 7.2.4-arch1-2, LSM
`capability,landlock,lockdown,yama,bpf`, Yama `ptrace_scope` = 1, governor `powersave`). Regole di lettura:
`risultati/sandbox-fase4-regola-ms.txt`. Commit delle regole **finale** `6e0f563`, 2026-11-26 18:06:29 CET (orologi di portatile e
minisheep uguali al secondo); il primo `[stato prima]` delle corse che contano è **18:06:35** (A1 r1). Codice: commit `fb80d92`,
binario `sbxspike4` sha256 `647b6bc83e000621d26746ab260e90698f62a78ebaf96134748cd6bb53d0ccfa`, in ogni file grezzo valido.
`landlock` 0.4.7, `seccompiler` 0.5.0, wasmtime e wasmtime-wasi 50.0.3, toolchain 1.99.1.

**Cosa è stato misurato e cosa no.** Un sottocomando per ruolo di AD-6 (`validation`, `jq`, `callhttp`, `light`, `stream`).
`light` è la sandbox leggera vera (wasmtime, stesso `serve` della fase 2). Gli altri quattro sono **stub**: applicano il
profilo di confinamento del loro ruolo prima di leggere una riga e poi eseguono le syscall che il ruolo vero userebbe (file,
socket) e le prove ostili. **Gli stub misurano il confinamento, non il lavoro del ruolo**: il loro RSS è un minimo, non una stima
di jaq, di un client HTTP o di DataFusion. Il motore è un processo finto (il pilota) con segreti finti generati a ogni corsa e
mai scritti qui; la parte del motore di A5 è una **stima etichettata** (26,5 MB, spike del bus, altro binario); A7 gira sul
percorso della sandbox, non su quello completo. `call http`: il filtro degli indirizzi (M-30) non è misurato.

## Corse, validità, incidente

- File grezzi `risultati/sandbox-fase4-<riga>-ms-r<n>.txt`: A1, A2, A3, A4, A5, A6, A7, A9, 3 corse ciascuna (r1-r3), 24 file, tutte
  valide: `[uscita] 0`, nessun `panicked`, nessun `[macchina non ferma]`, binario e moduli identici, core 1-4 occupati al massimo il
  10,6 % all'avvio (A1 r1; soglia 30 %), `MemAvailable` ≥ 26,7 GB, controlli positivi di A2 presenti (nella variante `weak` con
  modalità `none` le sei prove di controllo risultano `LEAK` per tutti i 5 ruoli in tutte e 3 le corse) e di A7 (controllo con 0
  riavvii, fasi `idle` con tutti i 226/122/20 riavvii richiesti, 0 errori, 20.000 attività).
- **Incidente.** Le prime corse (binario `61877162…`, commit `2d0002b`: A1, A3, A4 r1 valide; A2 r1 e A2a r1 invalide con `[uscita] 124`)
  hanno mostrato un difetto del pilota: ogni connessione riuscita al socket UNIX astratto del motore finto lasciava una voce nella
  coda (8) e la nona `connect` si bloccava. Corretto in `fb80d92` (code da 4096, `PTRACE_SEIZE` invece di `PTRACE_ATTACH`), binario
  cambiato, regole emendate prima di ogni corsa nuova, tutte le righe rifatte da zero. I file vecchi restano come
  `sandbox-fase4-*-primafix-*` e **non contano**. A2 con la divisione per ruoli (`A2a`) era un tentativo fallito, non una riga.
- Corse di prova: `sandbox-fase4-smoke-*` (compreso `smoke-A2full` col binario nuovo), non contano.
- Script usati e committati prima delle corse: `run4.sh`, `chain4.sh` (l'ordine; in pratica le righe si sono lanciate a gruppi con
  `run4.sh` perché l'intera catena supera il tempo di un comando), `smoke4.sh`, `discover.sh`, `env.sh`, `ms-run.sh`.

## ABI Landlock di minisheep (A0, intestazione di ogni file)

Il kernel risponde **ABI 10** a `landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)` (misurata, A0 in ogni file).
Il crate `landlock` 0.4.7 conosce le ABI fino alla **V9** (`effective=V9 kernel=10` nel rapporto di confinamento): l'ABI 10, che la
fase 1 descrive come «rete UDP», non è nel crate e quindi **non viene applicata** (UDP non è limitato da Landlock con questo
crate: ipotesi dalla documentazione, non provato; i ruoli senza `socket` nella lista seccomp non possono creare socket comunque).
Con ABI 9 sono applicati: filesystem (tutti i diritti, `Execute` e `ResolveUnix` compresi), TCP `bind`/`connect` per porta,
scope dei segnali e dei socket UNIX astratti, applicazione a tutti i thread (`all_threads`, ABI ≥ 8). Stato riportato:
`FullyEnforced` per tutti e 5 i ruoli. Namespace utente senza privilegi: `unprivileged_userns_clone=1`,
`max_user_namespaces=126131`, nessuna sysctl AppArmor; un `unshare(CLONE_NEWUSER|CLONE_NEWNS)` in un figlio usa-e-getta riesce
(letti e provati senza cambiare nulla). `core_pattern` è una pipe a `systemd-coredump`.

## Riepilogo

| Riga | Valore (peggiore tra le 3 corse) | Soglia | Verdetto |
| --- | --- | --- | --- |
| A1 confinamento prima dell'input | 5 ruoli su 5, 3 corse su 3: `Seccomp: 2`, 1 filtro, `NoNewPrivs: 1`, dumpable 0, `Max core` 0, `nofile` 64, Landlock `FullyEnforced`, lavoro legittimo senza `KILLED`; `light`: 8 nodi con le stesse classi della fase 2 | applicato prima della prima lettura | passa (anche in lettura stretta) |
| A2 lettura della chiave | `full`: **0 fughe su 19** in 5 ruoli × 2 varianti del motore × 3 corse; controlli positivi presenti | ogni lettura negata | passa |
| A3 ambiente, IPC, core | `env_vars=0`, `environ` di proprietà di root, `core` 0, nessun core dump nelle condizioni dichiarate, `ipc=999` esce con 3 in ≤ 2,4 ms | verificato | passa (core: vedi sotto) |
| A4 ripiego senza Landlock | 19 prove su 19 negate per `validation`, `jq`, `light`; per `callhttp` e `stream` 17 su 19 (restano 2 prove di rete); rifiuto con codice per `callhttp` e `stream` senza namespace utente | A2 passa anche nel ripiego, o rifiuto | **passa con riserva** (lettura b) |
| A5 NFR-1 | dopo l'arresto: 26,5 MB (stima del motore) ; N=1 attivo: 71,1 MB; N=5: 249,3 MB; N=20: 913,5 MB | ≤ 80 MB | **passa con riserva** (a sì, b no) |
| A6 riavvio a freddo | p50 ≤ 5,56 ms, p99 ≤ 6,17 ms (caso peggiore `light` + `javy-static`); confinamento 0,2-0,3 ms | p50 ≤ 100 ms, p99 ≤ 250 ms | passa (margine > 40 volte) |
| A7 p99 a 1.000 attività/s | 1 % di riavvii a freddo: p99 3,45 ms, controllo 0,35 ms, **differenza 3,1 ms**; 0,5 %: +0,03 ms; 0,1 %: +0,04 ms | p99 ≤ 25 ms **e** ≤ controllo + 2 ms | **non passa** al 1 % (regge il 25 ms, non il +2 ms) |
| A8 un ausiliario per tenant | per tenant 38,7 MB RSS (4 ruoli) o 45,4 MB (5); cold start come A6; con N=5 e 20 fuori da A5 | dentro A5 e A7 | **non passa** per N ≥ 5 in RSS; avvio dentro A6 |
| A9 limite del nodo < timeout | 30/30 fermi prima di T e con la classe che nomina il limite, T = 10, 100, 1.000, 5.000 ms; T = 1 ms rifiutato | verificato | passa |

## A1 Confinamento applicato prima dell'input (misurata: `sandbox-fase4-A1-ms-r1..r3.txt`)

Ordine nel ruolo: chiusura dei descrittori ereditati e limiti (core 0, nofile 64, cpu 3600 s, no_new_privs); per `light` motore
wasmtime, thread dell'epoca e una esecuzione di riscaldamento; `RLIMIT_FSIZE=0` (non per `stream`); Landlock su tutti i thread,
`PR_SET_DUMPABLE=0`; seccomp su tutti i thread (`KILL_PROCESS`, TSYNC); poi la riga `confined`, `ready`; il motore manda `hello` solo
dopo. Prima di mandare qualunque riga il motore legge lo stato del kernel di ogni ausiliario: `Seccomp: 2`, `Seccomp_filters: 1`,
`NoNewPrivs: 1`, `/proc/<pid>/environ` di proprietà di root (= dumpable 0), `Max core file size` 0, `Max open files` 64, `Max file
size` 0 (tranne `stream`: illimitato). Landlock: `FullyEnforced`, `all_threads=1`, scope `signal+abstract_unix`; regole: nessuna per
`validation`, `jq`, `light` (tutto il filesystem e tutta la rete negati); `ro` (CA) e una porta TCP per `callhttp`; `rw` (cartella del
tenant) e una porta TCP (il bus) per `stream`. Syscall consentite (base di 23 scelte a mano più quelle scoperte con gdb sul
lavoro legittimo): 23 per `validation` e `jq`, 29 `callhttp`, 26 `light`, 30 `stream`; nessuna ammette `clone`, `fork`, `execve`,
`kill`, `ptrace`, `process_vm_readv`; `open`/`statx` solo per `callhttp` e `stream`, socket solo per loro. Il lavoro del `light`
confinato dà le stesse classi della fase 2: `rust-p1` e `javy-static` senza errore, `loop` `cpu_time_limit`, `growmem` e
`bigmem` `memory_limit`, `recurse` `stack_limit`, `table` `table_limit`, `probe` (guest ostile della fase 2) senza errore.
Tempo di confinamento (`t_confine_us`): 0,20-0,27 ms p50. Non coperto: syscall argomento per argomento (la lista filtra solo il
numero: `mmap` con `PROT_EXEC` e `ioctl` sono aperti per `light`), arm64 (i numeri sono x86_64).
Verdetto: passa. Etichetta: misurata. Scoperte di ordine, da riportare: (1) wasmtime-wasi crea alla prima scrittura un runtime
tokio (10 thread, `epoll`, lettura dei cgroup via `openat`/`statx`): senza un'esecuzione di riscaldamento prima dei filtri la lista
del `light` avrebbe richiesto `clone3`, `openat`, `eventfd2`, `epoll_*`; (2) i descrittori ereditati vanno chiusi **prima** di
creare quel runtime (altrimenti se ne chiude l'`epoll`); (3) `RLIMIT_FSIZE=0` va messo dopo la creazione del motore wasmtime (usa
`memfd` e `ftruncate`: ucciso da SIGXFSZ); (4) `PR_SET_DUMPABLE=0` va messo dopo il ripiego con namespace (con dumpable 0 i file
`/proc/self/uid_map` sono di root).

## A2 Lettura della chiave per ogni sottocomando (misurata: `sandbox-fase4-A2-ms-r1..r3.txt`)

19 prove giudicate × 5 ruoli × 5 modalità × 2 varianti del motore, ognuna in un ausiliario nuovo, 3 corse con segreti rigenerati:
file della chiave, database, configurazione, file e cartella di un altro tenant; `/proc/<motore>/{environ,maps,cmdline}`; lettura
della memoria del motore (`/proc/<pid>/mem` e `process_vm_readv`); `ptrace`; `SIGUSR1` al motore; descrittori ereditati; socket UNIX
con percorso e astratto del motore; TCP alla porta interna del motore; `exec`; scrittura nella cartella della cache; `fork`.
Varianti del motore: `strong` (come AD-6: dumpable 0, Yama attivo) e `weak` (dumpable 1 e `PR_SET_PTRACER_ANY`: spegne Yama e il
dumpable, per isolare ciò che fa il confinamento da solo).

| Modalità | Fughe su 19 (identiche nelle 3 corse) |
| --- | --- |
| `none` (nessun confinamento) | `strong`: 13-14; `weak`: 18 (controllo positivo: key, db, config, tenant B, environ, maps, cmdline, mem, process_vm_readv, ptrace, segnale, socket, exec, scrittura, fork) |
| `ll` (solo Landlock) | 1, per tutti i ruoli: `fork` (Landlock non lo limita); `ptrace`, segnale, socket astratto, TCP, `exec` e ogni file negati |
| `sc` (solo seccomp) | 0 per `validation`, `jq`, `light`; **9 (`strong`) e 11 (`weak`) per `callhttp` e `stream`**: con `openat` e `socket` consentiti seccomp non protegge file, `/proc` e socket |
| **`full`** | **0 per tutti i 5 ruoli, entrambe le varianti, 3 corse su 3** |
| `fb` (ripiego) | vedi A4 |

Verdetto: **passa** (lettura larga: `full`, tutti i ruoli, entrambe le varianti, 3 corse). Riportato, non giudicato: `stat` del file
della chiave riesce per `callhttp`, `stream` (e `light` in `sc`/`weak`): Landlock non limita `stat`, quindi un ruolo che ha `statx`
nella lista vede esistenza e dimensione del file della chiave (metadati, non contenuto). Controllo di appartenenza: la lettura di
un file del proprio tenant riesce solo per `stream` (diritto `rw` concesso), per gli altri è negata.
**Chi nega cosa** (misurata, `A2yama`, ausiliario non confinato, 4 varianti del motore):

| Prova | dump0 + Yama | dump1 + Yama | dump0 senza Yama | dump1 senza Yama |
| --- | --- | --- | --- | --- |
| `/proc/<motore>/environ`, `maps` | negata | **riesce** | negata | riesce |
| `/proc/<motore>/mem`, `process_vm_readv`, `ptrace` | negata | negata | negata | riesce |
| `/proc/<motore>/cmdline` | **riesce** | riesce | **riesce** | riesce |

Quindi: Yama protegge `mem`, `process_vm_readv` e `ptrace` ma **non** `environ` e `maps` (accessi in sola lettura); `environ` e `maps`
li protegge soltanto `PR_SET_DUMPABLE=0` del motore; `cmdline` è sempre leggibile da un processo dello stesso utente se non lo nega il
confinamento (Landlock: nessun permesso su `/proc`; seccomp: niente `openat`). Nel motore finto con dumpable 1 la chiave di
avvio **resta in `/proc/<motore>/environ` anche dopo `remove_var`** (`A2environ`: 807 byte, 1 occorrenza), perché l'ambiente originale
sta nella pagina della pila. Dentro la variante `strong` (la configurazione di AD-6) tutte le negazioni di `/proc` e di memoria
vengono da dumpable 0 e da Yama; con `full` vengono anche, in modo indipendente, da Landlock (nessun permesso su `/proc`, scope
dei segnali, restrizione di `ptrace`) e da seccomp (nessun `openat`, `ptrace`, `kill`, `process_vm_readv` nella lista: `KILLED(SIGSYS)`).
Limite della prova sui descrittori: il descrittore dimenticato del motore è condiviso (stessa descrizione di file), quindi solo la
prima lettura in tutto il giro lo vede; il controllo positivo c'è una volta (`validation`, `strong`, `none`, 14 fughe invece di 13) e
l'evidenza della chiusura è nel rapporto di confinamento (`inherited_fds=2 closed_fds=2` per ogni ruolo senza descrittori da tenere).
Non coperto: fughe dal kernel, canali laterali, `call http` verso indirizzi interni (M-30), lettura da un processo non figlio
(Yama la nega al sistema, non a topictoast).

## A3 Ambiente vuoto, IPC, core dump (misurata: `sandbox-fase4-A3-ms-r1..r3.txt`)

Per ognuno dei 5 ruoli e in tutte le corse: `env_vars=0` dentro; `/proc/<pid>/environ` di root (dumpable 0, letto da fuori; letto anche
da dentro in modalità `ll`: `dumpable=0`; controllo `none`: 1 e proprietà dell'utente); `Max core file size` 0; un SIGSEGV con solo il
rafforzamento (modalità `ll`) esce con segnale 11 **senza il bit di core e senza file `core*`** nella cartella di lavoro e in quella
della corsa; `hello ipc=999` fa uscire l'ausiliario con codice 3 in 0,9-2,4 ms con un messaggio, prima di qualunque altra richiesta
(versione 1: parte). Verdetto: passa. Il core dump **non ha un controllo positivo**: `core_pattern` è una pipe a `systemd-coredump` e
non si è voluto attivarla su una macchina condivisa; «nessun core nelle condizioni dichiarate» è misurato, «per effetto di
`RLIMIT_CORE` 0 e dumpable 0» è ipotesi dalla documentazione del kernel.

## A4 Ripiego senza Landlock (misurata: `sandbox-fase4-A4-ms-r1..r3.txt`)

Percorso forzato (`fb`, Landlock finto assente) per i 5 ruoli e entrambe le varianti del motore; `fb-nouserns` (anche namespace utente
finti assenti). Livello raggiunto: `validation`, `jq` e `light` **seccomp senza `open`** (confinamento pieno anche senza Landlock, 19 prove
su 19 negate); `callhttp` e `stream` **namespace utente + mount con radice vuota + seccomp** (`userns+seccomp`, 0,6-0,85 ms): filesystem,
`/proc` e segnali negati (file ENOENT, `cmdline` ENOENT, `ptrace`/`kill` uccisi da seccomp), ma **2 prove su 19 riescono**, in tutte e 6 le
combinazioni (2 ruoli × 2 varianti × 3 corse): il socket UNIX **astratto** e la **porta TCP interna** del motore. Il namespace utente senza
`CLONE_NEWNET` lascia la rete dell'host (i socket astratti appartengono al namespace di rete), e i due ruoli la usano. Con
`fb-nouserns` `callhttp` e `stream` escono con codice 78 e il rifiuto `E_AUX_CONFINEMENT_UNAVAILABLE`, il pilota scrive l'errore del
registro del motore e l'avviso all'operatore (testo nei file), e gli altri tre ruoli partono col livello `seccomp-senza-open`.
Verdetto: lettura **b, passa con riserva**: il ripiego regge tutto salvo le prove di rete dei ruoli che hanno bisogno della rete; per questi
due ruoli il ripiego non è equivalente a Landlock (che nega la porta TCP per numero e lo scope dei socket astratti). Rimedio da decidere
nell'ADR: il motore non deve ascoltare su socket astratti né su porte TCP a loopback senza autenticazione, oppure `callhttp` e `stream`
rifiutano di avviarsi senza Landlock come anche senza namespace. Non coperto: il rilevamento vero (la syscall che risponde `ENOSYS` o
`EOPNOTSUPP`) su un kernel che la nega, il ripiego con `CLONE_NEWNET`, AppArmor che vieta i namespace utente.

## A5 NFR-1: somma degli RSS (misurata: `sandbox-fase4-A5-ms-r1..r3.txt`)

100 ausiliari confinati e caldi (20 tenant × 5 ruoli), 60 s senza esecuzioni, poi arresto di tutti (20-23 ms per fermarli tutti, 0
rimasti). RSS per ruolo (stabile tra le 3 corse): `validation` 6,6 MB, `jq` 6,6, `callhttp` 6,7, `stream` 6,7, **`light` 18,8 MB** (10
thread); il RSS non cala in 60 s di inattività (nessun rilascio di memoria: serve fermare il processo). Stand-in del motore: 7,0-7,3 MB
(non è il motore). Somme, con la **stima etichettata** del motore (26,5 MB, spike del bus):

| Situazione | Somma (r1 / r2 / r3) | ≤ 80 MB |
| --- | --- | --- |
| dopo l'arresto per inattività (solo il motore) | 26,5 MB | sì |
| N = 1 tenant, 5 ruoli attivi | 71,1 / 70,6 / 70,6 MB | sì |
| N = 5 | 249,3 / 248,1 / 247,7 MB | **no** |
| N = 20 | 913,5 / 913,4 / 911,3 MB | no (riportato, non giudicato) |

Con l'RSS del pilota al posto della stima: 51,7 / 50,9 / 50,8 MB per N = 1. Verdetto: **passa con riserva** (lettura a: dopo l'arresto e con
N = 1; la stretta, N = 5, non regge). Il risultato è un **minimo**: gli stub non hanno il lavoro dei ruoli veri. Osservazione misurata
(controllo senza confinamento, l'unico che lascia leggere `smaps_rollup`): i processi condividono il binario, quindi il PSS è molto
sotto l'RSS (stub 0,7 MB, `light` 2,1 MB); NFR-1 è definito sulla somma degli RSS, che conta ogni pagina condivisa una volta per
processo. Non coperto: il motore vero, i ruoli veri (jaq, client HTTP, validatore, DataFusion: ordini di grandezza più grandi), la
parte dei dati dei nodi in esecuzione.

## A6 Riavvio a freddo (misurata: `sandbox-fase4-A6-ms-r1..r3.txt`)

200 avvii per caso dopo 10 di riscaldamento; ogni avvio comprende fork/exec, (per `light` motore wasmtime, riscaldamento e caricamento),
confinamento, `hello` e primo lavoro; l'ausiliario precedente si ferma prima del successivo. Tempo da fork a primo lavoro, modalità `full`,
massimi tra le 3 corse:

| Caso | p50 (ms) | p99 (ms) | max (ms) |
| --- | --- | --- | --- |
| `validation` | 0,92 | 1,22 | 1,95 |
| `jq` | 0,92 | 1,19 | 1,36 |
| `callhttp` (primo lavoro: file e roundtrip TCP) | 0,92 | 1,22 | 1,60 |
| `stream` (primo lavoro: scrittura, `fsync`, roundtrip TCP) | 3,83 | 5,72 | 6,56 |
| `light` + `empty` (fino a «il nodo inizia») | 2,35 | 2,97 | 5,13 |
| `light` + `rust-p1` | 2,69 | 3,06 | 4,00 |
| `light` + `javy-static` (artefatto da 3,7 MB) | 5,56 | 6,17 | 7,47 |

Soglia p50 ≤ 100 ms e p99 ≤ 250 ms: passa in ogni caso e ogni corsa, margine di oltre 40 volte. Il confinamento costa (r1, `none` contro
`full`): `validation` 0,67 contro 0,92 ms, `light`+`rust-p1` 2,40 contro 2,69 ms; `t_confine_us` 0,20-0,27 ms p50, p99 ≤ 0,45 ms. Il
caso con più Landlock (rete e filesystem) non cambia il tempo in modo misurabile. Etichetta: misurata; le cause (riscaldamento di
wasmtime-wasi ≈ 1,4 ms nel `light`; `fsync` nello `stream`) sono ipotesi lette dalle differenze tra casi, non da un profilo. Non
coperto: page cache fredda, systemd, il binario del motore vero (più grande), un avvio mentre il resto della macchina è carico.

## A7 Avvio a 1.000 attività/s con arresto per inattività (misurata: `sandbox-fase4-A7-ms-r1..r3.txt`)

Carico aperto, 20 s a fase, 20.000 attività, istante di riferimento = quello pianificato; 20 tenant caldi più un insieme rotante di 64
freddi; ogni attività è `rust-p1` in un ausiliario `light` confinato; arresto per inattività dopo 1 s (accelerato: 60 s in produzione).
Questo è il **percorso della sandbox con il pilota come motore**, non il percorso completo. Ritardo massimo del pianificatore ≤ 3,1 ms.

| Fase | p99 (ms), 3 corse | differenza dal controllo | p50 avvio a freddo |
| --- | --- | --- | --- |
| controllo, ausiliari sempre accesi | 0,352 / 0,351 / 0,342 | | |
| 1 % di riavvii a freddo (226 su 20.000 = 1,13 %) | **3,414 / 3,348 / 3,453** | **+3,06 / +3,00 / +3,11** | 4,56 / 4,47 / 4,65 |
| 0,5 % (122) | 0,373 / 0,383 / 0,380 | +0,02 / +0,03 / +0,04 | 4,90 / 4,70 / 4,75 |
| 0,1 % (20) | 0,386 / 0,370 / 0,350 | +0,03 / +0,02 / +0,01 | 4,94 / 5,10 / 5,05 |

p99.9 al 1 %: 5,2 / 5,0 / 5,3 ms; massimo 6,4 ms. Verdetto: **non passa** (la soglia di Marco richiede p99 ≤ 25 ms **e** al massimo 2 ms
sopra il controllo): il p99 assoluto regge con un margine di sette volte, la differenza dal controllo è di circa 3 ms al 1 % di riavvii a
freddo, perché con l'1 % di richieste a freddo il 99º percentile cade nel costo di un riavvio (4,5-6 ms). Alla metà dell'1 % (0,5 %) e sotto
la differenza è ≤ 0,04 ms. Il riavvio a freddo in mezzo al carico (p50 4,5-5,1 ms) costa circa il doppio che da solo (A6: 2,7 ms): ipotesi,
contesa su 4 core con 84 ausiliari. Non coperto: percorso del motore (gRPC, bus), 60 s di inattività vera, tenant realmente concorrenti su
molti core; la frazione di riavvii a freddo è una scelta del carico, non un dato della produzione: il verdetto dipende da essa.

## A8 Un ausiliario per tenant (derivata da A5, A6, A7)

Per tenant, RSS di `validation` + `jq` + `callhttp` + `light` = 38,7 MB (con `stream` 45,4 MB; PSS 4,2-4,9 MB). Con N = 1 la somma con la stima del
motore è 71 MB; con N = 5 è 249 MB e con N = 20 913 MB: in RSS **non rientra** per N ≥ 5, e l'ADR deve contare sull'arresto per inattività
o su un ausiliario condiviso. Avvio a freddo per ruolo: A6 (≤ 6,2 ms p99). Con 20 tenant caldi (A7) il riavvio costa 4,5-6 ms. Verdetto: non
passa in RSS per N ≥ 5 (per costruzione di A5), l'avvio è dentro A6 e A7 (0,5 % e meno).

## A9 Limite del nodo contro timeout del task (misurata: `sandbox-fase4-A9-ms-r1..r3.txt`)

Regola del motore finto: limite wasm = min(richiesto, T − T/10); un timeout T < 10 ms è rifiutato (`E_TASK_TIMEOUT_TOO_SMALL`), perché con
un tick di epoca da 1 ms non si garantisce un limite sotto T. Con il limite richiesto «senza limite» e 30 esecuzioni per caso, T = 10, 100,
1.000, 5.000 ms: `loop` si ferma a 9,0 / 90,0 / 900,0 / 4.500,0 ms (p50; massimo 9,19 / 90,19 / 900,23 / 4.500,54) con classe
`cpu_time_limit` 30 su 30, prima di T; `growmem` finisce in 0,01-0,18 ms con `memory_limit` 30 su 30. Controllo senza la regola: scatta il timer
del task (10,3 / 100,7 / 1.001,6 / 5.005,7 ms) e l'errore non nomina il limite del nodo. T = 1 ms rifiutato. Verdetto: passa, in tutte le corse.
Il «nome del limite» è la classe dell'errore (il testo per l'utente lo scrive il motore).

## Cosa contraddice o corregge le fasi 1-3

1. **Fase 1, ripiego con namespace utente («dà una rete vuota»).** Per i ruoli che hanno bisogno della rete (`call http`, pipeline stream) non
   si può usare `CLONE_NEWNET`: il namespace utente con radice vuota lascia socket UNIX astratti e porte a loopback del motore raggiungibili
   (A4: 2 prove su 19). La fase 1 lo prevedeva solo per `call http` e solo per Landlock; vale anche per lo stream, e vale per il ripiego
   intero.
2. **Fase 1, `PR_SET_DUMPABLE=0` e Yama.** Misurato (A2yama): Yama non protegge `environ` e `maps` del motore (accessi in sola lettura), le
   protegge solo il dumpable 0; `cmdline` è sempre leggibile da un processo dello stesso utente senza confinamento.
3. **AD-6, «il motore toglie dal proprio ambiente la variabile della chiave principale».** Non basta: dopo `remove_var` la chiave resta
   in `/proc/<motore>/environ` (807 byte, 1 occorrenza con dumpable 1). Va letto: la protezione di `environ` è il dumpable 0 e il
   confinamento degli ausiliari, non la rimozione della variabile. Meglio non passare mai la chiave per l'ambiente (file della chiave).
4. **Fase 1, sezione 6: «seccomp senza `open`/`socket`/`connect` dà un confinamento senza Landlock».** Vero per `validation`, `jq` e `light`
   (A4: 19 su 19); falso per i ruoli che aprono file o socket (A2 `sc`: 9-11 fughe su 19).
5. **Fase 1: «da verificare fino a quale ABI arriva la 0.4.7».** V9; il kernel di minisheep è ABI 10: la parte nuova dell'ABI 10 (UDP) non è
   coperta dal crate; Landlock non limita `stat` né `fork` (misurati).
6. **Fase 2, L12 e fine della sezione «cosa non è coperto»: «l'ausiliario ha lo stesso utente e potrebbe leggere i file della chiave».**
   Confermato (A2, `none`: chiave, database, configurazione, cartella di un altro tenant, `cmdline`, segnale, socket, `exec`, `fork` riescono)
   e chiuso da `full` per tutti e 5 i ruoli.
7. **Fase 2, L4 e Revisione («NFR-1 vale a riposo solo se gli ausiliari si fermano per inattività»).** Misurato col confinamento e con
   `light` che ora costa **18,8 MB** di RSS (fase 2: 15,0 MB; +3,8 MB e 10 thread per il riscaldamento di wasmtime-wasi, che serve a tenere
   corta la lista seccomp): un solo tenant con tutti i suoi 5 ruoli già porta la somma a 71 MB con il motore stimato. Il RSS non scende in 60 s
   di inattività: serve fermare i processi. Il PSS è circa 9 volte più piccolo dell'RSS (condivisione del binario): NFR-1 sull'RSS è
   molto pessimista.
8. **Fase 2, L2/L1 (avvio a freddo 4,2 ms, `javy-static`) e «Landlock e seccomp: fase 4».** Con confinamento e `hello` il caso peggiore sale a
   5,56 ms p50 (6,2 ms p99): il confinamento in sé pesa 0,2-0,3 ms; il resto è il riscaldamento e il binario più grande.
9. **Fase 2, sezione «seccomp: da provare quali syscall servono a wasmtime a regime».** Risposta: 26 (23 di base più `pread64`, `epoll_wait`, `statx`)
   se il runtime tokio di wasmtime-wasi e il motore sono creati prima dei filtri e i descrittori sono già chiusi; altrimenti anche `clone3`,
   `openat`, `eventfd2`, `epoll_*`. L'ordine dei passi conta (A1, scoperte 1-4).
10. **Fase 3.** Nessuna contraddizione: la microVM non è stata toccata. Una nota di coerenza con S5: la lista seccomp dell'ausiliario leggero
    che gira *dentro* il guest (fuori portata qui) non è stata provata.
11. **SM-C2 / fase 2, «l'arresto per inattività non peggiora il p99 di NFR-2»:** vero per il p99 assoluto (3,4 ms ≤ 25 ms), ma con l'1 % di
    riavvii a freddo il p99 sale di 3 ms rispetto agli ausiliari sempre accesi (soglia +2 ms): dipende dalla frazione di riavvii, che il
    prodotto dovrà tenere sotto lo 0,5 % (misurato: +0,03 ms) o ridurre con un avvio anticipato.

## Cosa non è stato misurato

Il motore vero e i ruoli veri (RSS, lavoro, avvio con un binario più grande); il filtro degli indirizzi di `call http` (M-30) e UDP;
cgroup e namespace di rete (senza root); arm64 (NFR-10, numeri di syscall x86_64); la rilevazione vera di un kernel senza Landlock (il
percorso è forzato dal programma) e AppArmor che vieta i namespace utente; i core dump (nessun controllo positivo); le fughe dal kernel e i
canali laterali; il percorso completo di A7 (gRPC, bus) e i 60 s di inattività veri; page cache fredda; lista seccomp argomento per argomento.

## Revisione (2026-11-26, coordinatore)

- **Valori riletti sui file grezzi:** A2 (0 fughe su 19 prove giudicate in modalità `full`, per ruolo e variante del motore finto), A5 (somme di RSS con 1, 5, 20 tenant e stima del motore), A7 (fase `idle` all'1 %: p99 3,41 ms contro 0,35 ms del controllo; 226 riavvii a freddo su 20.000 richieste, cioè l'1,13 %; i riavvii a freddo da soli p50 4,6 ms e p99 6,0 ms): uguali al testo. Hash del binario `647b6bc83e000621…` in tutti i file che contano; `61877162…` solo nei file `-primafix-`.
- **Corse prima della correzione:** un difetto del pilota (coda del socket del motore finto) ha bloccato A2. La modifica delle regole del 18:06 cambia solo l'hash del binario e aggiunge una nota datata, senza toccare le soglie; tutte le righe sono state rifatte da zero dopo. Le corse vecchie valide (A1, A3, A4) danno gli stessi esiti di quelle che contano, quindi scartarle non sposta i verdetti.
- **A7 non passa per la regola fissata da Marco:** con l'1 % di riavvii a freddo il p99 cade dentro la coda dei riavvii (4,6-6,0 ms), quindi supera di 3 ms il controllo pur restando molto sotto i 25 ms. A 0,5 % e 0,1 % la differenza è al massimo 0,04 ms. Lettura per l'ADR: il costo di un riavvio a freddo è 4,6 ms al p50, e il p99 lo mostra appena i riavvii superano l'1 % delle richieste (misurata, sul percorso della sandbox e non del motore vero).
- **A5 e NFR-1:** a riposo, con gli ausiliari fermati per inattività, resta solo il motore (26,5 MB, stima dallo spike del bus). Con tenant attivi la somma cresce di circa 38,7 MB per tenant sui quattro ruoli simulati: NFR-1 regge solo se gli ausiliari si fermano prima dei 60 s della misura (ipotesi sul motore vero).
