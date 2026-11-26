# Spike S-2, fase 2: livello leggero, risultati

Data: 2026-11-26. Misure solo su minisheep (x86_64, kernel 7.2.4, 22 core, Yama `ptrace_scope` = 1, governor
`powersave`). Regole di lettura: `risultati/sandbox-fase2-regola-ms.txt`, committate prima delle corse
(ultima versione `f2fa825`, 2026-11-26 14:28:17; il primo `[stato prima]` misurato è 14:28:32). Codice:
commit `33d68a4`, binario `sbxspike` sha256
`53e7b4e7f8e472473f335049fd0b97a4e1f150ba86004a6574d1ab268c81b4cc`, in ogni file grezzo.
wasmtime e wasmtime-wasi 50.0.3 (eccezione di sicurezza), javy 9.1.0 (CLI dal rilascio ufficiale: per la CLI
non c'è un crate su crates.io), toolchain 1.99.1.

File grezzi: `risultati/sandbox-fase2-<riga>-ms-r<n>.txt`, 3 corse per riga (r1, r2, r3); per L2 r2 non
vale (all'avvio i core 1-4 erano occupati al 35,5 %, soglia 30 %) e si è aggiunta r4, quindi L2 conta r1, r3, r4.
Tutte le altre corse: nessun `[macchina non ferma]`, `[uscita] 0`, binario e moduli identici. Corse di prova:
file `sandbox-fase2-smoke-*`, non contano.

Cosa si è costruito e misurato, in una riga: un binario con l'ausiliario per tenant (un `Engine`, moduli
precompilati verificati con la loro impronta, istanze su richiesta con limiti), il compilatore che valida e scrive
la cache nativa, e il pilota. Nodo di prova: JSON in, JSON fuori, tre righe di log, una scrittura su stream tramite
funzione dell'host, in sei forme: vuoto, javy statico, javy dinamico (plugin QuickJS istanziato a ogni esecuzione),
Rust wasip1, Rust unknown, Rust wasip2 (componente).

## Riepilogo

| Riga | Valore (peggiore tra le corse valide) | Soglia | Verdetto |
| --- | --- | --- | --- |
| L1 avvio a caldo | `t_start` p50 0,024 ms, p99 0,036 ms (J; peggiore: javy dinamico); `t_total` p99 0,132 ms (javy statico) | p50 ≤ 10 ms, p99 ≤ 25 ms | passa (anche in lettura stretta) |
| L2 avvio a freddo | `cold_to_node_start` p50 4,17 ms (javy statico), p99 5,98 ms | p50 ≤ 100 ms, p99 ≤ 250 ms | passa |
| L3 memoria per istanza | 4,12 MB RSS (N = 1, javy dinamico); a N = 50 sotto 0,5 MB | ≤ 64 MB | passa |
| L4 ausiliario fermo | RSS 15,0 MB (javy statico), PSS 11,2 MB; somma PSS N=5: 17,3 MB; somma RSS N=5: 73,2 MB | ≤ 20 MB; somma ≤ 80 MB | passa (condizione necessaria; N=20 non rientra) |
| L5 limite di CPU | sforamento massimo 0,22 ms; classe `cpu_time_limit` 100 % | ≤ 50 ms | passa |
| L6 limite di memoria | `memory_limit`, `table_limit`, `stack_limit`, `memory_limit`; ausiliario vivo; altro tenant p99 `t_start` 0,079 ms | classe del limite, nessun OOM, p99 ≤ 25 ms | passa |
| L7 nessun residuo | thread 2 invariati, figli 0, crescita RSS ≤ 4 KB; kill pulito; `RLIMIT_CPU` ferma il ciclo in 2,1-2,2 s con SIGXCPU | vedi regole | passa |
| L8 un processo per tenant | 0 occorrenze del valore dell'altro tenant (due vie, due configurazioni, 3 corse); controllo positivo 10 occorrenze | 0 e controllo ≥ 1 | passa |
| L9 TS e Rust girano | 5 moduli su 5: uscita esatta, 3 log con livelli 0-1-2, 1 emit | gira | passa |
| L10 validazione e compilazione | 1 MB: 861 ms e 141 MB (sintetico), 471 ms e 74 MB (javy statico); ostili: uccisi a 10,0 s | ≤ 1 s e ≤ 256 MB; ostile entro il tetto | passa |
| L11 cache nativa | con la verifica dell'impronta: 5 artefatti alterati su 5 rifiutati | nessun codice non nostro | passa |
| L12 codice ostile | nessun sentinella nell'uscita; ogni tentativo negato o limitato | tutto negato | passa |

Nessuna riga contraddice la fase 1 nei verdetti. Differenze e sorprese nelle sezioni sotto e in fondo.

## L1 Avvio a caldo (misurata: `sandbox-fase2-L1-ms-r1..r3.txt`)

1.000 esecuzioni per modulo dopo 100 di riscaldamento, un ausiliario già avviato. `t_start` = dalla lettura della
richiesta all'ingresso del guest in `_start`/`run` (store, contesto WASI, istanziazione).

Configurazione giudicata J (su richiesta, copy-on-write, epoca), ms, valori tra le tre corse:

| Modulo | `t_start` p50 | `t_start` p99 | `t_total` p50 | `t_total` p99 |
| --- | --- | --- | --- | --- |
| vuoto | 0,004 | 0,005-0,007 | 0,004 | 0,005-0,007 |
| javy statico | 0,010 | 0,015-0,025 | 0,065-0,068 | 0,085-0,132 |
| javy dinamico | 0,023-0,024 | 0,033-0,036 | 0,079-0,082 | 0,099-0,116 |
| Rust wasip1 | 0,007-0,008 | 0,011-0,014 | 0,017-0,019 | 0,027-0,031 |
| Rust unknown | 0,007 | 0,011-0,012 | 0,026 | 0,033-0,042 |
| Rust wasip2 (componente) | 0,010 | 0,014-0,016 | 0,021-0,022 | 0,028-0,031 |

Soglia p50 ≤ 10 ms e p99 ≤ 25 ms: margine di oltre 400 volte su `t_start` e di circa 190 volte su `t_total`.
Verdetto: passa, in lettura larga (a) e stretta (b). Confronti riportati: pooling (`t_start` javy statico p50
0,006-0,007 ms), senza copy-on-write (javy statico 0,149-0,177 ms: 15 volte più lento perché la memoria del
guest si copia a ogni istanza), combustibile (`t_start` uguale a J, p99 fino a 0,073 ms in una corsa).
Non coperto: un nodo che lavora, richieste concorrenti, il percorso del motore (gRPC, bus), il tempo reale di
inizializzazione di javy che è dentro `_start`; wasmi e ComponentizeJS non costruiti (non decisi per la v1).
Etichetta: misurata. Perché sia così piccolo: ipotesi, `Engine` e moduli già nel processo, nessun I/O.

## L2 Avvio a freddo (misurata: `sandbox-fase2-L2-ms-r1.txt`, `r3`, `r4`; `r2` non valida)

200 avvii per modulo e per corsa: fork/exec, `Engine`, lettura e verifica dell'impronta, `deserialize_file`,
prima esecuzione. `cold_to_node_start` in J, ms (p50 / p99 massimi sulle tre corse valide): vuoto 0,99 / 1,72;
javy dinamico 1,40 / 2,00; Rust wasip1 1,39 / 1,89; javy statico 4,17 / 4,70 (artefatto da 3,7 MB); il
peggiore p99 di tutte le righe J è 4,70 ms; con pooling 2,0-4,9 ms p50 e fino a 5,98 ms p99 (il pooling riserva
2,3 GB di spazio virtuale per processo). Soglia p50 ≤ 100 ms, p99 ≤ 250 ms: passa, margine di oltre 40 volte.
La corsa non valida r2 dà gli stessi valori (p99 massimo 5,65 ms). Non coperto: file della cache non nella page
cache (appena compilata, quindi in memoria), Landlock e seccomp (fase 4), lancio da systemd.

## L3 Memoria per istanza (misurata: `sandbox-fase2-L3-ms-r1..r3.txt`)

N istanze tenute vive nello stesso processo dopo aver eseguito il nodo. Massimo di RSS per istanza (J): 4,12 MB
con N = 1 (javy dinamico; 3,9 MB javy statico; 1,5 MB Rust; 0,7 MB vuoto), perché il costo fisso delle pagine di
codice entra tutto nella prima istanza; con N = 10 sotto 0,7 MB, con N = 50 sotto 0,5 MB (javy statico 0,42 MB).
Soglia ≤ 64 MB: passa, margine di 15 volte nel caso peggiore. Con pooling N=50 javy statico: 1,18 MB per
istanza (RSS del processo 70 MB). Il caso peggiore vero, un guest che usa tutto il tetto da 64 MiB, è in L6/L7: il
picco (`hwm`) del processo è 61 MB. Non coperto: istanze davvero concorrenti sotto carico, tetto del processo
intero (nessun cgroup senza root).

## L4 Ausiliario fermo (misurata: `sandbox-fase2-L4-ms-r1..r3.txt`)

| Modulo | N | RSS max per ausiliario | PSS max | somma PSS | somma RSS |
| --- | --- | --- | --- | --- | --- |
| vuoto | 1 / 5 / 20 | 10,8-11,1 MB | 7,4-7,7 MB | 7,5 / 12,6 / 25,6 MB | 10,9 / 54,9 / 219 MB |
| javy statico | 1 / 5 / 20 | 14,5-15,0 MB | 11,1-11,2 MB | 11,2 / 17,3 / 33,6 MB | 14,6 / 73,2 / 294 MB |
| Rust wasip1 | 1 / 5 / 20 | 11,5-12,1 MB | 8,1-8,6 MB | 8,6 / 13,9 / 28,4 MB | 12,0 / 59,4 / 238 MB |

Solo `Engine` (nessun modulo): RSS 8,9-9,3 MB, PSS 5,6-5,9 MB. Costo marginale per tenant in PSS: 1,0-1,5 MB
(le pagine del binario si condividono tra gli ausiliari). Soglie: ognuno ≤ 20 MB passa (RSS e PSS); somma
≤ 80 MB per N = 1 e 5 passa in PSS, e anche in RSS (javy statico N=5: 73,2 MB, ma lascerebbe 7 MB al motore).
Verdetto: passa, ma la somma è solo una condizione necessaria, perché il motore non esiste e la sua parte non è
misurabile qui. N = 20 non rientra in RSS (294 MB per javy, 219-238 per gli altri): ci vuole l'arresto per
inattività (fase 4, A5-A7). Etichetta: misurata; la conclusione sull'arresto per inattività è ipotesi.

## L5 Limite di CPU (misurata: `sandbox-fase2-L5L6-ms-r1..r3.txt`)

30 esecuzioni per caso e per corsa, epoca (J). Sforamento = `t_total` meno il limite, ms:

| Caso | Sforamento p50 | max (3 corse) | Classe |
| --- | --- | --- | --- |
| ciclo infinito, limite 100 ms | da -0,20 a -0,13 | 0,08 | `cpu_time_limit` 90/90 |
| ciclo infinito, limite 1.000 ms | da -0,20 a -0,15 | -0,05 | `cpu_time_limit` 90/90 |
| copie da 16 MiB senza fine, 100 ms | da -0,18 a -0,12 | 0,22 | `cpu_time_limit` 90/90 |

Soglia ≤ 50 ms: passa (un valore negativo è la fase del tick da 1 ms). Il nome del limite è la classe
dell'errore; il testo per l'utente lo scrive il motore. Riportato: combustibile 0,134-0,144 ns per unità
(classe `cpu_fuel_limit`), epoca più combustibile: scatta l'epoca; **una sola copia in blocco da 1 GiB
sfora di 80-96 ms** (copia da ~190 ms, l'epoca si controlla solo all'inizio): buco noto di wasmtime, fuori dal
tetto di memoria da 64 MiB (la stessa copia da 64 MiB costerebbe circa 12 ms per proporzione: ipotesi, non
misurata). Non coperto: limite wasm minore del timeout del task (A9, fase 4).

## L6 Limite di memoria (misurata: stesso file di L5)

10 esecuzioni per caso e per corsa con tetto 64 MiB: `growmem` finisce con `memory_limit`, `table` con
`table_limit` (tetto 10.000 elementi), `recurse` con `stack_limit` (pila 512 KiB), `bigmem` (memoria iniziale
da 2,6 GB) con `memory_limit` all'istanziazione. In tutte le serie e corse l'ausiliario risponde ancora, thread
2 prima e dopo, RSS da 10 a 12 MB. Altro tenant B a 200 nodi/s (circa 4.000 esecuzioni per fase, 20 s):
`t_start` p99 in `control` 0,092-0,110 ms, in `hostile` 0,072-0,079 ms (soglia 25 ms): passa; il round trip
p99 resta sotto 0,45 ms. Non coperto: `RLIMIT_AS` sull'ausiliario (non provato: la riserva di memoria virtuale
di wasmtime lo renderebbe fragile), tetto di memoria del processo intero (nessun cgroup senza root, il tetto
è per istanza: N istanze concorrenti valgono N × 64 MiB), A e B sugli stessi core ma non con carico da molti
tenant.

## L7 Nessun residuo (misurata: `sandbox-fase2-L7-ms-r1..r3.txt`)

Giri di ciclo, `growmem`, `recurse`, `table` con limite 20 ms (10, poi 100, poi 100): thread 2 sempre, figli 0,
RSS 12,1-12,3 MB, crescita tra i due gruppi da 100 giri 0-4 KB (soglia 2 MB), picco 61 MB che poi rientra.
`kill` del PID salvato: `/proc/<pid>` sparito, 0 figli, 0,2 ms. Ausiliario senza epoca né combustibile con
ciclo infinito e `RLIMIT_CPU` 2 s: ucciso da SIGXCPU dopo 2,09-2,16 s (soglia 3 s). Passa. Non coperto: il
kill di un ausiliario che tiene file o socket (non ne ha), uscita per timeout del task vero.

## L8 Un processo per tenant (misurata: `sandbox-fase2-L8-ms-r1..r3.txt`)

Valori sentinella casuali da 128 bit, mascherati con XOR 0x5A sulla linea. Per J e per `pool+epoch`, in tutte e
tre le corse: valore di B in A e valore di A in B, 0 occorrenze con tutte e due le vie (processo stesso e
pilota che legge `/proc/<pid>/mem`, permesso perché genitore); controllo positivo 10 occorrenze (J: 26-27 MB
letti; pooling: 2,2 GB letti). Passa. Riportato: dopo il rilascio delle istanze il valore del tenant resta
27 volte (J) o 15 (pooling) nello stesso processo: la memoria non si azzera, quindi il confine è il processo,
non la pulizia. Riportato: l'ausiliario A che legge la memoria di B (processo non antenato) riceve
`Permission denied` da Yama (`ptrace_scope` 1): è una difesa del sistema, non di topictoast, e va giudicata in
fase 4 insieme a Landlock. Non misurati: core dump, swap.

## L9 TS e Rust girano (misurata: `sandbox-fase2-L9-ms-r1..r3.txt`)

Cinque moduli su cinque, tre corse su tre: esito senza errori, uscita esatta `{"greeting":"hello topictoast","sum":5}`
(per `rust-unknown` dalla funzione `output`), tre righe di log con livelli 0, 1, 2 e una scrittura
`emit events {"sum":5}`.

| Modulo | `.wasm` | cwasm |
| --- | --- | --- |
| javy statico | 1.239.385 B | 3.688.336 B |
| javy dinamico (+ plugin da 1.210.114 B condiviso) | 1.336 B | 14.648 B |
| Rust wasip1 | 91.370 B | 292.880 B |
| Rust unknown | 76.231 B | 251.600 B |
| Rust wasip2 (componente) | 110.235 B | 343.960 B |

Passa. Osservazioni misurate: per far chiamare l'host a javy serve un plugin Rust (`javy-plugin-api` 7.1.0,
costruzione con wasi-sdk 24 scaricato dal build script di `rquickjs-sys`); il javy dinamico istanzia il plugin a
ogni esecuzione e costa `t_start` 0,023 ms contro 0,010 del statico, ma pesa 1,3 KB invece di 1,2 MB per
tenant; un cambio di versione del plugin invalida i moduli dinamici (dalla documentazione, ipotesi). Il
componente wasip2 costa quanto il modulo core in avvio e memoria (L1, L3): la condizione della fase 1 per
accettare anche i componenti è soddisfatta per Rust; ComponentizeJS non costruito. Non coperto: comodità
dell'autore, TypeScript vero (il nodo è JavaScript già in forma finale: tsc o esbuild non provati).

## L10 Validazione e compilazione (misurata: `sandbox-fase2-L10-ms-r1..r3.txt`)

Sottocomando in un processo a parte sui core 1-4, 3 ripetizioni per corsa (9 per file), tetto 10 s con kill per PID.

| File | Tempo reale (min-max, 9 prove) | RSS max |
| --- | --- | --- |
| javy statico, 1,24 MB (reale) | 438-471 ms | 71-74 MB |
| sintetico 1,01 MB | 816-861 ms | 138-141 MB |
| Rust wasip1, 91 KB | 34-40 ms | 22-23 MB |
| sintetico 100 KB | 85-93 ms | 27 MB |
| sintetico 10 MB (riportato) | 7,5-7,9 s | 1.136-1.141 MB |

Soglia 1 MB ≤ 1 s e ≤ 256 MB: passa su tutte e 18 le prove, ma il margine sul tempo del sintetico è solo del
14 % (861 ms su 1.000): un modulo da 1 MB più denso di funzioni potrebbe sforare. Moduli ostili per il
compilatore: `hostile-straight` (funzione da 2,2 MB con 200 variabili locali) e `hostile-ifs` (1,7 MB) non
finiscono in 10 s e il pilota li uccide a 10,01-10,07 s (9 su 9 per ognuno), con RSS 1.901-1.906 MB e 575-584
MB; `hostile-brtable` e i byte malformati (troncato, zeri, spazzatura) sono rifiutati in meno di 12 ms;
`hostile-nest` (20.000 blocchi annidati) è accettato in 23-48 ms; solo-intestazione è un modulo valido. Mai un
segnale su byte malformati. Durante i compilatori ostili il tenant B ha `t_start` p99 0,094-0,105 ms. Con
`RLIMIT_AS` 512 MiB sul processo che compila, `hostile-straight` e `hostile-ifs` abortiscono (SIGABRT, RSS
200-206 MB) in 1,2 s e 5,6-5,9 s: il tetto di memoria virtuale ferma il compilatore ostile senza toccare nessun
altro processo. Passa. Conclusione per l'ADR, ipotesi derivata dalla misura: la memoria della compilazione non
si limita da Cranelift, serve `RLIMIT_AS` (o cgroup) più kill per tempo nel processo che compila; nessun
combustibile per la compilazione. Non coperto: compilazioni di più tenant insieme.

## L11 Cache nativa (misurata: `sandbox-fase2-L11-ms-r1..r3.txt`, righe uguali nelle tre corse)

Con la verifica dell'impronta registrata dal compilatore (sha256 dell'artefatto, 0,2 ms per 293 KB): artefatto
intatto caricato ed eseguito; un byte cambiato nell'intestazione, a metà, all'ultimo byte, troncato, `.wasm` al
posto del cwasm: tutti rifiutati (`sha mismatch`). Artefatto compilato con `epoch` caricato da un motore `fuel`
o `epoch+nocow`: rifiutato da wasmtime. Un cwasm caricato al posto del modulo: il compilatore lo rifiuta
(`magic header not detected`). Passa. **Senza la verifica**, wasmtime carica e esegue gli artefatti con un byte
cambiato (intestazione, centro, ultimo byte: tutti e tre eseguiti con l'uscita giusta) e rifiuta solo quello
troncato o non-cwasm: la verifica dell'impronta è quindi obbligatoria, non facoltativa (misurata). Un artefatto
di `epoch` è accettato dal motore `pool+epoch` (stessa compatibilità). Non misurato: artefatto di un'altra
versione di wasmtime (serve una seconda costruzione; la stringa di versione non si trova in chiaro nel file;
ipotesi dalla documentazione di wasmtime: rifiuto deterministico), permessi della cartella (Landlock, fase 4).

## L12 Codice ostile noto (misurata: `sandbox-fase2-L12-ms-r1..r3.txt`)

Modulo `probe` (Rust wasip1) con 16 percorsi: file della chiave, database, configurazione, cartella di un altro
tenant, `/proc/<motore>/{environ,mem,maps}`, `/proc/<ausiliario>/{environ,mem}`, `/proc/self/*`, `/etc/passwd`,
`/`, `.`: tutti negati (nessuna directory pre-aperta: errore «no such file»). Nessuno dei 6 valori sentinella
(compreso quello nell'ambiente del «motore» e quello dell'ausiliario) compare nell'uscita del guest, in 3 corse
su 3. Ambiente del guest: 0 variabili. TCP, UDP e creazione di processi: `operation not supported`. Descrittori
3-15: tutti negati (il descrittore del file della chiave ereditato dall'ausiliario non è raggiungibile). `sock`
(accept, send, path_open su descrittori inesistenti): errno 8 (EBADF) tre volte. `evilimport`: rifiutato al
caricamento («unknown import: `env::escape` has not been defined»). `logflood`: `log_limit` dopo 16 righe da 64
KB (1 ms). `errflood`: stderr fermo a 1.048.576 byte (il tetto), ma il guest **continua a girare** finché non
scatta l'epoca (3,0 s), perché la pipe tronca le scritture e non le fa fallire. L'ausiliario risponde dopo ogni
modulo. Passa. Registrazione: i rifiuti dell'host tornano come classe (`log_limit`, importazione non offerta,
`cpu_time_limit`); i rifiuti di WASI sono errno dentro il guest e l'host non li vede: lacuna da dichiarare.
Fuori portata, come deciso: nessuna funzione `http` dell'host, quindi la rete interna (127.0.0.1,
169.254.169.254, IPv6 mappato, DNS verso indirizzi privati) non si misura: il filtro degli indirizzi è M-30; si è
misurato solo che il guest non può aprire socket né importare altro. Non coperto: Landlock e seccomp (fase 4): qui
l'ausiliario ha lo stesso utente e potrebbe leggere i file della chiave; la prova per ogni sottocomando è A2.

## Cosa non è stato misurato o coperto

- ComponentizeJS e wasmi (non costruiti: non decisi per la v1); TypeScript compilato da tsc/esbuild (il nodo di
  prova è JavaScript); arm64 (dichiarato non misurato, NFR-10).
- Rete interna tramite `http` dell'host (M-30), Landlock e seccomp (fase 4), il percorso reale del motore
  (gRPC, bus), carico concorrente da molti tenant, cgroup (non senza root), `RLIMIT_AS` sull'ausiliario che
  esegue (provato solo sul compilatore), file di cache non in memoria, artefatto di un'altra versione di
  wasmtime, core dump, swap.
- I numeri di tempo sono di un processo ausiliario già caldo che riceve righe su una pipe: non sono il tempo
  del motore completo.

## Cosa contraddice o corregge la fase 1

1. **Yama**: la fase 1 lasciava aperto se `ptrace_scope` bloccasse la lettura della memoria altrui. Su
   minisheep vale 1: il padre legge i figli, i fratelli no (misurato). Il metodo di L8 usa il processo stesso e
   il padre, e riporta il fratello come difesa del sistema.
2. **Epoca contro combustibile**: la fase 1 citava «fino a 2-3 volte più veloce» dal commento di wasmtime. Qui
   `t_start` e `t_total` con combustibile sono uguali a quelli con epoca entro il rumore (L1), e il combustibile
   dà 0,14 ns per unità con una velocità stabile; l'epoca resta più comoda per il tempo (sforamento < 0,3 ms), il
   combustibile non misura il tempo. Il vantaggio di velocità non si vede nei nostri nodi brevi (ipotesi: si
   vedrebbe su cicli lunghi, non misurato).
3. **Copia in blocco**: confermato il buco dell'epoca (80-96 ms per una copia da 1 GiB), ma dentro il tetto da
   64 MiB è fuori portata pratica.
4. **Componenti**: la fase 1 chiedeva se costassero poco in memoria e avvio per accettarli: per Rust wasip2 sì
   (L1, L3); manca ComponentizeJS e Python.
5. **Memoria della compilazione**: la fase 1 non aveva numeri; un modulo da 2,2 MB può chiedere 1,9 GB e
   non finire in 10 s, quindi il tetto sul processo che compila (`RLIMIT_AS` più tempo) non è facoltativo.
6. **Verifica dell'impronta della cache**: la fase 1 la lasciava «da decidere»; L11 mostra che wasmtime esegue
   artefatti con un byte cambiato, quindi è obbligatoria.
7. **Pooling**: conferma la fase 1 (spazio virtuale per slot: 2,3 GB per processo, avvio a freddo da 1,4 a 2,5 ms
   più lento) e aggiunge che non serve: senza pooling `t_start` è già 0,01 ms. Resta fuori per FR-16.
8. **Memory_init_cow**: spenta, javy parte 15 volte più lento (0,15 ms contro 0,01): non è un parametro da toccare.

## Revisione (2026-11-26, coordinatore)

- **Valori riletti sui file grezzi:** L1 (javy dinamico, il peggiore, `t_start` p50 0,023-0,024 ms e p99 0,035-0,036 ms), L2 (javy statico p50 4,16-4,17 ms, p99 4,59-4,70 ms), L4 (somme di RSS e PSS per N = 1, 5, 20), L5 (sforamento massimo 0,22 ms; la copia unica da 1 GiB 86-96 ms), L8 (0 occorrenze nelle prove, 10 nei controlli), L10 (modulo sintetico da 1 MB 820-829 ms e 139-140 MB): uguali al testo. Hash del binario `53e7b4e7f8e47247…` in tutti i 68 punti in cui compare.
- **Ordine:** il commit finale delle regole è delle 14:28:17 e la prima corsa misurata parte alle 14:28:32; il commit del codice `33d68a4` arriva dopo la prima stesura delle regole ma prima di ogni corsa misurata, quindi è ammesso.
- **L2, corsa r2 non valida e r4 aggiunta:** le regole scritte non prevedono la corsa sostitutiva (fuori dalle regole, come nello spike del bus). Il verdetto non dipende da questo: anche r2 passa (javy statico p50 4,18 ms, p99 4,64 ms).
- **L4 contro NFR-1 (riserva):** NFR-1 è una somma di RSS. Con 5 ausiliari fermi la somma delle RSS è 54,9-73,2 MB senza il motore; con il motore misurato nello spike del bus (26,5 MB, altro binario) si arriva a circa 81-100 MB, sopra gli 80 MB. La somma regge solo in PSS. Quindi a riposo NFR-1 vale solo se gli ausiliari dei tenant si fermano per inattività (FR-50): è la fase 4 (stima da due misure diverse, ipotesi).
- **Avvio a freddo:** misurato con il binario piccolo dello spike e la cache nella page cache; l'ausiliario vero è un sottocomando del binario del motore, più grande (ipotesi: tempi più alti, da rimisurare con il binario vero).
- **Sequenze di 64 cifre esadecimali nei file grezzi:** sono nomi dei file della cache (impronte dei moduli), non segreti. I valori sentinella di L8 e L12 non compaiono.
