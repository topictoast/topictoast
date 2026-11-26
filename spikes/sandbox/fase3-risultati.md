# Spike S-2, fase 3: livello forte (microVM Firecracker con pool), risultati

Data: 2026-11-26. Misure solo su minisheep (x86_64, Intel Core Ultra 9 185H, kernel dell'host 7.2.4-arch1-2, governor
`powersave`, Yama `ptrace_scope` = 1, `/dev/kvm` apribile dall'utente). Regole di lettura:
`risultati/sandbox-fase3-regola-ms.txt`, committate prima delle corse (commit `b2f6d82`, 2026-11-26 16:25:53 CET; il primo
`[stato prima]` misurato è delle 16:26:04, `S8` r1). Codice: commit `27341f5`, binario `sbxspike3` (statico, glibc) sha256
`c8d744c80372524c2e2b74383acd1fcb6b73e4c769169db97221e3406997e4b2`, in ogni file grezzo. `chain3.sh` (solo l'ordine delle
corse) si è committato dopo le corse. Firecracker v1.18.0 (binario sha256 `b33f46cd…02a2ba`), guest `vmlinux-6.1.155` e
`ubuntu-24.04.squashfs` della CI di Firecracker (impronte nelle regole e nella riga `S0` di ogni file), wasmtime 50.0.3
nel guest. Disco dell'agente: ext4 da 64 MiB costruito senza root con `mkfs.ext4 -d` (`mkdrive.sh`), sha256
`754da5fd…dc075`.

File grezzi: `risultati/sandbox-fase3-<riga>-ms-r<n>.txt`, 3 corse per riga (r1, r2, r3), righe S1-S9 (27 file). Tutte le
corse sono valide: nessun `[macchina non ferma]`, `[uscita] 0`, nessun `ABORT` né `panicked`, binario e impronte uguali, core
1-4 occupati al massimo il 24,9 % all'avvio (S5 r2; soglia 30 %), nessun `firecracker` visibile prima, `MemAvailable` ≥ 27,0 GB.
Nessuna corsa rifatta. Corse di prova: file `sandbox-fase3-smoke-*`, non contano (alcune con una versione precedente del
codice, prima di correggere il conteggio delle risposte di `load` e il conteggio dei tentativi di S4).

Che cosa si è costruito, in una riga: un agente nel guest (stesso binario del pilota, PID 1, ascolta sul vsock e parla lo
stesso protocollo a righe dell'ausiliario leggero, più il trasferimento di byte e comandi `native` per il codice ostile
nativo che esistono solo dentro la microVM), un pilota che avvia le VMM come processi figli e parla con la loro API (HTTP
su socket Unix scritto a mano, nessuna dipendenza nuova), e le righe S1-S9.

## Riepilogo

| Riga | Valore (peggiore tra le corse) | Soglia | Verdetto |
| --- | --- | --- | --- |
| S1 avvio con microVM libera | larga: p50 1,87 ms (max 2,28 ms); stretta: p50 6,86 ms `empty`, 7,81 ms `rust-p1`, **16,80 ms `javy-static`** | p50 ≤ 10 ms | **passa con riserva** (la stretta cade per `javy-static`) |
| S2 pool vuoto | ripristino `rust-p1` p50 14,43 ms (max 21,04 ms); `javy-static` p50 25,59 ms (max 30,31 ms); avvio a freddo p50 57,06 ms | ripristino ≤ 200 ms | passa (larga e stretta) |
| S3 microVM ferma | RSS massima 31,5 MB (da snapshot), 65,0 MB (avvio a freddo); PSS 6,5-31,5 MB da snapshot | ≤ 100 MB | passa (larga e stretta) |
| S4 mai due tenant | sentinella di A: 13 nella VMM di A, **0** nella VMM di B, 0 nell'agente di B, 0 nei file dello snapshot (15 giri su 3 corse); 0 microVM passate; riciclo p50 24-31 ms | 0 e controllo ≥ 1 | passa |
| S5 codice ostile nel guest | 0 sentinelle in 6 su 6, 0 percorsi aperti, 0 connessioni, 0 scritture; host: calo massimo 366 MB; B p99 1,36 ms | tutto negato; host e B entro NFR-2 | passa |
| S6 limiti | `cpu_time_limit` 30/30, sforamento massimo 0,45 ms; classi dei limiti di memoria 10/10; kill e residui puliti; CPU 5,91 s su 6 s | ≤ 50 ms, classe del limite, nessun residuo | passa |
| S7 declassamento | rilevamento p50 ≤ 1,54 µs; ripiego: nodo in 1,66-2,00 ms; un avviso per variante | funziona e avvisa | passa |
| S8 kernel 7.2.4 | avvio a freddo fino al ping 52,6-54,0 ms; ripristino fino al ping 6,4-8,8 ms; seccomp della VMM attivo | parte e supera S1-S6 | passa |
| S9 stesso formato | 5 moduli su 5 identici byte per byte, 3 corse su 3 | stesso output | passa |

La riserva di S1 va nell'ADR (sezione S1). Le fughe dalla VMM non si misurano qui.

## Strati di difesa assenti perché non c'è root (per l'ADR)

Senza root la sandbox forte gira, ma senza questi strati, che in produzione sono la raccomandazione del progetto
(`prod-host-setup.md`): **jailer** (chroot, `pivot_root`, `mknod`, `unshare`, cambio di uid, cartelle cgroup); **cgroup** (nessun
tetto di CPU o di memoria sul processo della VMM: il tetto di RAM è la RAM del guest, quello di CPU il numero di vCPU,
l'overhead della VMM non ha tetto: in S5 la VMM è arrivata a 122,5 MB di RSS con 128 MiB di guest); **namespace di rete e
regole `nft`** (il guest non ha scheda, quindi la rete della microVM e il filtro dell'uscita di M-30 non si possono
nemmeno provare). La VMM gira con l'uid del pilota (1000) e vede tutti i suoi file: una fuga dalla VMM finirebbe lì.
Presente invece, senza root: il **filtro seccomp predefinito** di Firecracker, letto in `/proc/<pid>/task/*/status` (`Seccomp: 2`
su tutti e 4 i thread: `fc_api`, `fc_vcpu`, il principale e `kvm-nx-lpage-re`, in tutte le 3 corse di S8 e di S5). **Le fughe
dalla VMM (exploit su virtio, sulla VMM o su KVM) non sono misurabili qui**: S5 misura solo ciò che fa il codice dentro il
guest. Nemmeno SMT e mitigazioni di canale laterale, rumore di cache tra vicini, arm64 (NFR-10), snapshot letto da disco freddo.

## S1 Avvio con una microVM libera nel pool (misurata: `sandbox-fase3-S1-ms-r1..r3.txt`)

40 campioni per modulo e per lettura e per corsa; pool di 6 microVM ripristinate e ferme; una microVM per campione (poi
fermata e rimpiazzata); 100 ms di quiete prima di ogni campione. Tempo dell'orologio dell'host, «il nodo inizia» è il
marcatore `started` scritto dall'agente un istante prima di entrare nel guest wasm.

| Modulo (cwasm) | S1a larga: p50 / max (3 corse) | S1b stretta: p50 / max (3 corse) |
| --- | --- | --- |
| `empty` (13,7 KB) | 1,632-1,866 / 2,278 ms | 6,489-6,859 / 7,961 ms |
| `rust-p1` (293 KB) | 1,675-1,772 / 2,151 ms | 7,671-7,812 / 9,489 ms |
| `javy-static` (3,69 MB) | 1,334-1,465 / 1,886 ms | **16,603-16,802** / 18,545 ms |

Larga (microVM riservata, modulo già caricato, collegamento aperto: «vsock più istanza nel guest»): passa con un margine di
5 volte (peggiore p50 1,87 ms, massimo di tutte le 360 misure 2,28 ms). Stretta (microVM generica del pool: connessione,
invio dell'artefatto, verifica dell'impronta, `load`, `run`): `empty` e `rust-p1` passano (p50 6,9 e 7,8 ms; il massimo di
`rust-p1`, 9,49 ms, è vicino alla soglia), **`javy-static` non passa in nessuna corsa** (p50 16,6-16,8 ms). Per le regole il
verdetto è **passa con riserva**: NFR-12 forte regge per i moduli piccoli anche contando tutto, ma un artefatto da 3,7 MB
(javy statico) porta il tempo oltre i 10 ms. Il `t_start` interno del guest (istanza fino all'ingresso) è 1,0-1,6 ms nel primo
nodo di una microVM appena ripristinata, contro 0,044-0,055 ms (p50) in S1c con la stessa microVM già calda (300 ripetizioni
per corsa, p99 ≤ 0,083 ms): il primo nodo dopo il ripristino paga circa 1,3 ms in più (ipotesi: pagine della memoria
ripristinata che si caricano alla prima scrittura/lettura; non separato con una misura a parte).
Perché la stretta di `javy-static` costa 9 ms più di `rust-p1`: l'unica differenza è l'artefatto (3,4 MB in più da inviare,
verificare con sha256 e deserializzare nel guest). Ipotesi, non scomposta in passi misurati; la rimedio più semplice
(ipotesi) è tenere l'artefatto già dentro la microVM riservata al tenant (nella lettura larga, a regime, costa 1,4 ms).
Non coperto: più richieste insieme, il percorso del motore (gRPC, bus), la compilazione sull'host (qui già fatta), pool più
grandi di 6.

## S2 Pool vuoto: ripristino e avvio a freddo (misurata: `sandbox-fase3-S2-ms-r1..r3.txt`)

Dal fork della VMM al nodo che inizia (processo Firecracker, API, caricamento dello snapshot, ripresa, primo ping, invio,
`load`, `run`). 30 ripristini e 20 avvii a freddo per `rust-p1`, 10 e 10 per `javy-static`, per corsa, ms:

| | p50 (3 corse) | p99 = max (3 corse) |
| --- | --- | --- |
| ripristino `rust-p1` | 13,593 / 14,077 / 14,428 | 19,357 / 20,773 / 21,040 |
| ripristino `javy-static` | 25,594 / 25,194 / 24,364 | 30,306 / 29,338 / 29,727 |
| avvio a freddo `rust-p1` | 55,868 / 56,131 / 57,059 | 58,304 / 61,038 / 63,775 |
| avvio a freddo `javy-static` | 62,118 / 64,347 / 63,318 | 65,953 / 68,045 / 68,367 |

Soglia ripristino ≤ 200 ms: passa, in larga e in stretta (anche p99 e `javy-static`), con margine di circa 6,6 volte sul
caso peggiore (`javy-static`, 30,3 ms), 9,5 volte per `rust-p1`. Passi del ripristino di `rust-p1`: fork fino al primo ping p50 9,2-9,9 ms (di cui chiamata `snapshot/load` p50
5,6-6,3 ms), poi ping fino al nodo 4,5 ms. L'avvio a freddo (kernel e agente) arriva al ping in 54-55 ms p50: la riga di
comando del kernel senza console e con `i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd` toglie i 500 ms visti nella fase
1 (prova di fumo con console: 136 ms). Un ripristino con pool vuoto, per un modulo piccolo, costa 13,6-14,4 ms: già vicino
a 10 ms (NFR-12) senza pool. Non coperto: il file dello snapshot è nella page cache (il ripristino da disco freddo non si
è misurato), «attesa entro il limite di FR-8» (è del motore), carico concorrente. Creare lo snapshot: 1,63-1,64 s in S8
(contiene 1,5 s di quiete voluta).

## S3 RSS e PSS di una microVM ferma (misurata: `sandbox-fase3-S3-ms-r1..r3.txt`)

RAM del guest 64 e 128 MiB, pool di 1, 5, 20 (ping fatto, 2 s di quiete, nessun nodo eseguito):

| Riempimento | RSS max per VMM | PSS max (pool 1 / 5 / 20) | privata (`Anonymous`) | somma pool 20: RSS / PSS |
| --- | --- | --- | --- | --- |
| da snapshot (`MAP_PRIVATE` del file) | 28,7-31,5 MB | 28,7-31,5 / 10,2-10,5 / 6,5-6,7 MB | 3,2-5,2 MB | 599-630 / 124-131 MB |
| avvio a freddo | 65,0 MB | 64,9-65,0 / 62,9 / 62,5 MB | 62,4 MB | 1.165-1.175 / 1.116-1.126 MB |

Con l'avvio a freddo 64 e 128 MiB danno lo stesso valore (65,0 MB); da snapshot le differenze sono rumore (28,7-31,5 MB): i numeri non cambiano con la
RAM del guest. Soglia ≤ 100 MB per microVM ferma: passa, in larga (da snapshot, peggiore 31,5 MB) e in stretta (a freddo,
65,0 MB). Il costo vero del pool ripristinato da snapshot è la **parte privata** (3-5 MB per VMM, 104 MB per 20): le altre
pagine sono del file dello snapshot condiviso. Una VMM avviata a freddo tocca circa 62 MB di memoria anonima sia con 64 sia con
128 MiB (ipotesi: azzeramento delle pagine e cache del disco dell'agente, 32 MB di binario; non scomposto): quindi la
dichiarazione «VMM ≤ 5 MiB di overhead» del progetto (fase 1) vale per l'overhead della VMM, non per il guest che gira.
Non coperto: microVM che hanno già eseguito un nodo (monouso, non tornano nel pool), RSS di microVM di tenant diversi.

## S4 Una microVM che ha eseguito un tenant non passa a un altro (misurata: `sandbox-fase3-S4-ms-r1..r3.txt`)

5 giri per corsa, sentinella casuale da 128 bit per tenant e per giro (solo presenza o assenza). Tenant A esegue `rust-p1` con la
sentinella nell'ingresso (arriva al guest come file in `/tmp` e nella richiesta) e l'istanza tenuta viva; il pilota, genitore
della VMM, ne cerca i byte in `/proc/<pid>/mem`.

| Controllo | Risultato (15 giri su 3 corse) |
| --- | --- |
| sentinella di A nella VMM di A (controllo positivo) | 13 occorrenze in ogni giro |
| sentinella di A nella VMM di B (microVM ripristinata dopo aver fermato quella di A) | **0** in 15 giri |
| sentinella di A nella memoria dell'agente di B (`scan` del guest) | 0 in 15 giri |
| sentinella di A nei due file dello snapshot (memoria e stato) | 0 in 15 giri |
| sentinella di B nella VMM di B (controllo) | 13 in ogni giro |
| regola del pool: rifiuti di dare la microVM di A a B / microVM date a un secondo tenant dopo l'uso | 15 su 15 / 0 |
| impronta sha256 del file di memoria dello snapshot prima e dopo | uguale in 3 corse su 3 |

Passa. Riciclo (fermare la VMM per PID, ripristinarne un'altra, primo ping): p50 24,2-30,9 ms, massimo 33,2-39,1 ms: il
monouso costa meno di quanto serva a riempire il buco nel pool. La memoria scritta dal guest è copy-on-write privata: il file
dello snapshot non cambia, quindi le pagine condivise non portano dati di un tenant a un altro (era «da provare» in fase 1).
Il controllo «la stessa microVM riservata conserva la sentinella» (13 occorrenze alla seconda lettura) non dice nulla sulla
pulizia della memoria, perché l'istanza era tenuta viva (`hold`): non si usa. Non coperto: più tenant insieme su microVM
riservate, la condivisione di pagine fra VMM con KSM, la fuga dalla VMM.

## S5 Codice ostile dentro il guest (misurata: `sandbox-fase3-S5-ms-r1..r3.txt`)

Guest da 1 vCPU e 128 MiB. Risultati uguali nelle tre corse salvo dove indicato.

- **Wasm, come L12** (modulo `probe` con 20 percorsi, `sock`, `evilimport`, `logflood`, `errflood`): 6 sentinelle su 6 assenti
  dall'uscita (file della chiave, database, configurazione, cartella di un altro tenant, ambiente della VMM, ambiente del
  pilota); 36 righe negate, 0 percorsi aperti; ambiente vuoto; TCP, UDP e processi negati; `evilimport` rifiutato al
  caricamento; `logflood` con `log_limit` dopo 16 righe; `errflood` ferma stderr a 1.048.576 byte e il nodo gira finché l'epoca
  lo ferma a 3,0 s (come in L12); l'agente risponde dopo ogni modulo.
- **Nativo dentro il guest** (codice arbitrario come root del guest: l'esito peggiore di una fuga dal modulo wasm):
  - scansione di 7.092 file e dei due dischi grezzi (557.134.048 byte, non interrotta): le 6 sentinelle dell'host 0 volte, il
    controllo positivo 1 volta; `/dev/kvm` assente;
  - rete: nessuna interfaccia; TCP verso 127.0.0.1, 169.254.169.254, 10.0.0.1, 192.168.0.1 e 93.184.216.34 e UDP verso 8.8.8.8:
    tutti errno 101 (rete irraggiungibile); vsock verso l'host: si collega solo alla porta 7000, dove il pilota ascolta
    (controllo positivo: il pilota riceve la connessione), 68 porte rifiutate;
  - scritture: sul rootfs e sul disco dell'agente aperture negate (`EROFS`), sui dischi grezzi scritture negate (`EPERM`);
    le impronte di rootfs, disco e kernel sono uguali dopo la corsa;
  - riportato, non giudicato: **`/dev/mem` e `/proc/kcore` si aprono** nel guest come root (il kernel di CI li ha): vedono la
    memoria del guest, non dell'host, ma un kernel di produzione li dovrebbe togliere dalla configurazione.
- **Distruttivo, una microVM nuova per prova, con un tenant B vicino a 200 nodi/s** (sugli stessi core 1-4):

| Prova | Reazione del guest | Host (tutte le corse) | B: p99 round trip |
| --- | --- | --- | --- |
| ciclo di 6 s su tutte le vCPU | CPU della VMM 5,90-5,98 s su 6 s; agente vivo | calo massimo di `MemAvailable` 112 MB; VMM RSS 36 MB | 0,78-0,98 ms |
| bomba di fork per 6 s | 740-759 fork riusciti, 112.098-326.383 falliti, 792-809 processi al picco; agente vivo | calo 139-366 MB; VMM RSS 122,0-122,5 MB; CPU della VMM 6,4-7,9 s su 28 s | 0,98-1,04 ms |
| memoria in un figlio | il figlio ucciso da SIGKILL (OOM del guest) in 71-74 ms; agente vivo | calo 96-105 MB; RSS 120 MB (r3: 30 MB, il campionamento ogni 100 ms ha mancato il picco di 72 ms) | 0,97-1,12 ms |
| memoria nel PID 1 | il guest muore (nessuna risposta) in 2,1 s; la VMM era già uscita quando la si è fermata (0,2 ms) | calo 42-297 MB; RSS 119,5-120,2 MB | 1,01-1,36 ms |

Per B la fase di controllo (senza attacco) dà p99 1,00-1,05 ms: non c'è peggioramento. Dopo ogni prova fermare la VMM col
PID salvato lascia 0 processi e 0 cartelle (15-33 ms). `MemAvailable` dell'host resta ≥ 26.090 MB (era 26.551-26.595 prima),
swap invariato, nessun `ABORT`. Il guest che muore per panico del PID 1 (con `panic=1`, `reboot=k`) fa uscire la VMM da sola:
ipotesi dalla console della prova di fumo (init che esce), non rimisurato con la console accesa. Passa. Non misurabile
(dichiarato): fughe dalla VMM, canali laterali, rumore di cache. Non coperto: più VMM ostili insieme (qui 1 contro B), rete
interna tramite una funzione `http` dell'host (M-30, non esiste), disco dell'host pieno.

## S6 Limiti di CPU, memoria e tempo (misurata: `sandbox-fase3-S6-ms-r1..r3.txt`)

- **Wasmtime nel guest**, come L5/L6: `loop` 100 ms e 1.000 ms e `copyloop` 100 ms: `cpu_time_limit` 30/30 in ogni caso e corsa,
  sforamento massimo 0,45 ms (negativo, fino a un tick, per il limite di 1.000 ms); `growmem`, `table`, `recurse`, `bigmem`:
  `memory_limit`, `table_limit`, `stack_limit`, `memory_limit` 10/10 in tutte le corse; l'agente risponde dopo la serie. In
  fase 2 lo sforamento massimo era 0,22 ms: la microVM non lo peggiora in modo visibile (la scala è la stessa, ≪ 50 ms).
  Il valore «sforamento» delle righe `growmem`, `table`, `recurse`, `bigmem` (negativi, circa −2.000 ms) non è un tempo del limite
  e non si usa: quei casi finiscono per un limite di memoria, non di tempo.
- **Tenant B vicino** (altra microVM, 200 nodi/s, 10 s) mentre A esegue i moduli ostili nel guest per 10 s: round trip di B
  p99 0,97-0,99 ms contro 1,02-1,05 ms senza attacco (soglia 25 ms), `t_start` del guest p99 0,18 ms.
- **Limite di tempo dal motore**: ciclo infinito senza limite wasm e VMM fermata per PID alla scadenza: scadenza 500 ms,
  totale 516-518 ms; scadenza 2.000 ms, totale 2.016-2.020 ms (la parte del kill con attesa e pulizia della cartella, 15,9-20,4
  ms); CPU della VMM 0,49-0,50 s e 1,99 s; processo sparito, cartella sparita, `MemAvailable` +5/+19 MB 300 ms dopo (non cala).
- **Limite di CPU della microVM**: 1 vCPU, ciclo nativo di 6 s: CPU della VMM 5,90-5,91 s (soglia 7,5 s). Il tetto di memoria
  è la RAM del guest (S5: RSS massima 122,5 MB).

Passa. Non coperto: tetto di memoria della VMM intera (nessun cgroup), limite wasm minore del timeout del task (M-27, fase 4).

## S7 Declassamento senza KVM (misurata: `sandbox-fase3-S7-ms-r1..r3.txt`)

Metodo: il percorso del dispositivo è un parametro del pilota (`kvm=`); non si cambia mai il permesso di `/dev/kvm`. L'assenza si
simula con un percorso inesistente, un file nostro senza permessi (`0000`) e `/dev/null` (si apre, ma `KVM_GET_API_VERSION`
vale −1, non 12). Controllo con `/dev/kvm`: forte disponibile, nessun avviso. In tutte le 3 varianti e in tutte le 3 corse: forte
non disponibile, **un** avviso all'operatore («sandbox forte non disponibile (<causa>); uso la sandbox leggera»), il ripiego
parte e `rust-p1` gira senza errori. Rilevamento (1.000 chiamate per variante): p50 0,45-1,54 µs, p99 ≤ 1,90 µs, massimo 14,1 µs:
costa niente, va fatto una volta all'avvio e non per task. Ripiego (avvio dell'ausiliario leggero, `load`, `run`): 1,66-2,00 ms,
con `t_start` 0,05-0,07 ms. Passa. Non misurato: il comportamento di Firecracker stesso senza `/dev/kvm` (non si può togliere il
dispositivo senza root); la scelta fra ripiego silenzioso e «forte obbligatoria» resta una decisione di Marco (fase 1).

## S8 Firecracker v1.18.0 su un kernel 7.2.4 (misurata: `sandbox-fase3-S8-ms-r1..r3.txt`; prova di fumo per prima)

Il kernel dell'host 7.2.4-arch1-2 non è nell'elenco 5.10 / 6.1 / 6.18 di `kernel-policy.md`: Firecracker v1.18.0 parte, avvia
il guest 6.1.155 (nell'elenco dei kernel del guest), apre il vsock, esegue un nodo, crea e ripristina uno snapshot, e il nodo gira
anche dopo il ripristino: 3 corse su 3. Avvio a freddo fino al ping 52,6 / 53,5 / 54,0 ms; ripristino fino al ping 6,4 / 7,6 /
8,8 ms. Il filtro seccomp è attivo senza root (`Seccomp: 2` sui 4 thread). Con S1-S6 «passa» o «passa con riserva» (S1 con la
riserva sopra), la lettura è **passa**: cloud-hypervisor non serve e non si è provato (servirebbe un file nuovo, quindi un
permesso nuovo). Non coperto: altri kernel dell'host, arm64.

## S9 Lo stesso modulo e la stessa ABI nei due livelli (misurata: `sandbox-fase3-S9-ms-r1..r3.txt`)

Cinque moduli (`javy-static`, `javy-dynamic`, `rust-p1`, `rust-unknown`, `rust-p2`), stesso ingresso: il file di uscita del nodo
(uscita standard, uscita dell'host, 3 righe di log con livelli, 1 `emit`, 145 byte) è identico byte per byte tra il livello
leggero (ausiliario su pipe) e il forte (agente nel guest), 5 su 5 in 3 corse su 3 (impronta di `rust-unknown` diversa dagli
altri perché l'uscita passa dalla funzione `output` dell'host, uguale nei due livelli). L'artefatto lo precompila l'host (stesso
`compile` della fase 2) e il guest lo carica con `Module::deserialize` senza ricompilare: l'impronta di compatibilità di
wasmtime (flag della CPU compresi) coincide fra host e guest su questa CPU. Passa. Non misurato: wasmtime nel guest compilato con
musl (Tier 3): il bersaglio non è installato e installarlo sarebbe uno scaricamento non autorizzato; l'agente è statico con
glibc, e il binario dinamico dell'host non si è provato nel guest (la glibc del guest Ubuntu 24.04 è più vecchia: ipotesi, non
verificata).

## Cosa non è stato misurato o coperto

- Fughe dalla VMM, jailer, cgroup, namespace di rete, filtro dell'uscita, SMT e canali laterali, arm64 (vedi sopra).
- Il percorso reale del motore (gRPC, bus), carico concorrente da molti tenant, pool più grandi di 20, più VMM ostili insieme.
- Lo snapshot letto da disco freddo; più guest di tenant diversi che condividono lo stesso file di snapshot sotto pressione di memoria.
- cloud-hypervisor, musl, altri kernel dell'host o del guest.
- La rete interna di una microVM con scheda (non c'è scheda).

## Cosa contraddice o corregge le fasi 1 e 2

1. **Kernel dell'host non in elenco** (fase 1: «da provare, nessuna garanzia»): funziona (S8). Non serve cloud-hypervisor. Resta
   da dire nell'ADR che il 7.2.4 non è un kernel con garanzia del progetto.
2. **`vsock.md` e `CONFIG_VHOST_VSOCK`** (fase 1: contraddizione apparente): il vsock funziona dall'utente senza che il pilota
   tocchi `/dev/vhost-vsock` (che su minisheep esiste); non si è verificato se Firecracker lo apre da sé: non risolto, ma non
   serve a chi lo usa.
3. **Seccomp senza root** (fase 1: «nessun privilegio»): confermato e misurato (`Seccomp: 2`).
4. **`Module::deserialize` nel guest di un artefatto dell'host** (fase 1: da provare): funziona su questa CPU. Rischio
   (ipotesi): su una flotta di CPU diverse o con modelli di CPU che nascondono funzioni, l'impronta di compatibilità può non
   coincidere e il guest rifiuterebbe l'artefatto; va tenuto un controllo e un ripiego (ricompilare nel guest).
5. **Memoria condivisa dello snapshot** (fase 1: «da provare che nessuna pagina scritta sia condivisa»): provato per S4 (file
   invariato, 0 occorrenze nella VMM di B).
6. **«VMM ≤ 5 MiB di overhead»** (fase 1: dichiarazione del progetto): vera solo per l'overhead; con un guest avviato a freddo la
   VMM tiene 62 MB di memoria anonima e 65 MB di RSS, anche con 64 MiB; da snapshot la parte privata è 3-5 MB, il resto è
   condiviso (S3).
7. **Ripristino** (fase 1: proposta ≤ 200 ms): misurato 14 ms per un modulo piccolo e 25 ms per javy: la proposta era larga di
   un ordine di grandezza; con pool vuoto si è già vicini ai 10 ms di NFR-12.
8. **«≤ 10 ms con una microVM libera»** (PRD, assunzione): vale se il modulo è già nella microVM (1,3-1,9 ms), e per moduli piccoli
   anche con la consegna (6,9-7,8 ms), **non** per un artefatto da 3,7 MB (16,7 ms): il costo di consegnare l'artefatto entra
   nel conto e l'ADR deve dire dove si tiene (S1).
9. **Primo nodo dopo il ripristino** (non previsto): 1,0-1,6 ms nel guest contro 0,05 ms a caldo (S1).
10. **Kernel di CI** (fase 1: «kernel e rootfs non inclusi, si prendono dalla CI»): quello scaricato apre `/dev/mem` e `/proc/kcore`
    nel guest; un kernel di produzione per l'integratore va configurato senza (S5). Chi fornisce il kernel (integratore o noi)
    resta una decisione aperta.
11. **Fase 2 (L12, codice ostile leggero)**: lo stesso codice ostile wasm dà gli stessi risultati nel guest; la fase 2 non
    aveva la parte nativa (esaurimento e fork bomb), che qui si prova dentro il guest e resta nel guest.
12. **Nota di metodo**: l'agente non poteva essere il binario dinamico di fase 2 (glibc più vecchia nel guest: ipotesi, non
    provato); si è costruito statico con glibc, 32 MB, e il disco dell'agente è da 64 MiB.
