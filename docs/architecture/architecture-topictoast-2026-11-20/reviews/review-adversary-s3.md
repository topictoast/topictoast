# Revisione avversaria dell'Update S-3 (AD-13, AD-6)

Bersaglio: `ARCHITECTURE-SPINE.md` dopo l'ultimo commit (ADR 2026-11-28 delle pipeline stream). Lente: due unità un
livello sotto (storia del motore, storia del processo stream o della validazione a secco) che rispettano ogni AD alla
lettera e costruiscono pezzi incompatibili. Esclusi i buchi già nelle sei righe nuove delle Open Questions.

**Verdetto: la spine non regge ancora il collegamento tra motore e processo stream. Due buchi critici (chi decide lo
stato di una pipeline, forma e semantica dell'accettazione degli avvii a gruppi), cinque alti. Chiuderli con due righe
nuove in AD-13 e una Open Question prima della prima storia dell'ingresso a blocchi (FR-18).**

## Riepilogo

| # | Gravità | Buco |
| --- | --- | --- |
| 1 | critica | Nessun proprietario dello stato di una pipeline (attiva, sospesa, versione) |
| 2 | critica | Accettazione degli avvii «in una transazione»: atomicità, quote, esito per riga non fissati |
| 3 | alta | Busta del blocco e segnale di quiete senza tipo né proprietario; formula del watermark mancante |
| 4 | alta | Doppio proprietario della deduplicazione (C-18 del motore contro l'insieme nel checkpoint) |
| 5 | alta | Checkpoint e archivio dei payload: Landlock del processo contro l'adattatore RustFS |
| 6 | alta | Chiave C-4 senza finestra e aggiornamento del bus (FR-51): la posizione cambia generazione |
| 7 | alta | Nuova versione della query: da dove parte, cosa succede allo stato e alle righe in volo |
| 8 | media | Dove vive il controllo sull'albero: tre implementazioni possibili (dominio, wasm, ruolo a secco) |
| 9 | media | Memoria: `RLIMIT_DATA` per processo contro quota per pipeline; coda di consegna fuori dalla quota |
| 10 | media | Permessi Iggy per topic: stream creati dopo il tenant; rotazione della chiave con il processo vivo |
| 11 | media | Ambito del watermark con `UNION ALL`, pipeline senza finestra, finestre `tumble` e `hop` |
| 12 | media | Ritenzione dell'assorbimento di C-4 contro conservazione dello stream e ripresa |
| 13 | bassa | Arresto del ruolo stream: «checkpoint finale» contro «esce quando il socket si chiude» |

---

## 1. critica — Chi decide lo stato di una pipeline

**Quote citate.** AD-13: «la pipeline che li supera si sospende con il codice, le altre del processo continuano»;
«Ripresa esatta dopo una sospensione; oltre la conservazione la ripresa fallisce con un codice e la posizione resta».
AD-6: «Il ruolo delle pipeline stream parte quando il tenant ha una pipeline attiva e si ferma … quando si ferma
l'ultima»; AD-2: «Sul bus gli eventi di stato li scrive solo il motore»; «nessun'altra unità scrive lo stato
proiettato».

**Le due unità.**
- Storia A (processo stream): quando la quota di stato scatta, il processo segna la pipeline «sospesa» nel proprio
  checkpoint e smette di leggerla. È l'unico che sa perché.
- Storia B (motore, API delle pipeline): lo stato della pipeline (attiva, sospesa, versione, codice) è una risorsa
  dell'API (FR-23) che il motore scrive come evento di stato e il proiettore applica; al riavvio del processo il motore
  gli passa l'elenco delle pipeline attive.

Entrambe rispettano la spine. Dopo un crash del processo, A riparte con la pipeline sospesa nel checkpoint, B la manda
come attiva: la pipeline riparte, risfora la quota, si sospende di nuovo senza che l'operatore lo sappia; oppure A
ignora il comando di B. Chi legge la dashboard vede lo stato di B. Nessuna AD dice che il processo è un richiedente e
il motore il giudice, come invece AD-2 dice dei worker.

**Fix.** In AD-13 (Ciclo di vita e isolamento) aggiungere: «Lo stato di una pipeline (attiva, sospesa con il codice,
versione) è un evento di stato scritto solo dal motore (AD-2). Il processo non si sospende da solo: manda al motore un
messaggio tipizzato sull'IPC (`PipelineFault { pipeline, code }`), il motore scrive l'evento durevole e risponde con
`Suspend`. All'avvio il processo riceve l'elenco delle pipeline attive dal motore e il checkpoint non contiene mai lo
stato di sospensione.» Il tipo dei messaggi IPC va nominato (dove vive: vedi finding 3).

## 2. critica — «Il motore accetta in una transazione» un gruppo di avvii

**Quote.** AD-13: «Il processo consegna le righe a gruppi di al più 1.000, che il motore accetta in una transazione; un
rifiuto per quota trattiene le righe e le riprova.» AD-2: «accettato = scrittura durevole sul log del bus»; «nessun'altra
unità scrive lo stato proiettato». AD-18: «Le righe e gli avvii scritti dal processo delle pipeline stream passano
dall'IPC del motore con un id deterministico (C-4)»; i controlli di FR-30 vanno fatti «su tutto lo stato accettato».

**Le due unità.**
- Storia A (motore, ingresso IPC): «transazione» = una scrittura del gruppo sul bus come un solo gruppo durevole
  (AD-2); le quote di FR-30 si controllano sull'insieme: o tutto o niente.
- Storia B (motore, percorso di avvio normale): ogni riga passa dallo stesso punto di decisione di un avvio
  dell'API (AD-2, un punto per esecuzione), con idempotenza, quote e concorrenza (FR-13) riga per riga; l'esito è
  parziale, e «transazione» è letto come transazione del database.

Divergenze concrete: (a) con A, un gruppo da 1.000 e una quota residua di 300 non passa mai, e il processo lo riprova
all'infinito (livelock: il gruppo non si accorcia); (b) con B il processo deve sapere quali righe sono entrate, ma
AD-13 non dà un esito per riga; (c) un duplicato C-4 dentro il gruppo è assorbito o fa fallire il gruppo? (d) con A
l'accettazione di 1.000 avvii non è un solo «punto di decisione per esecuzione»: AD-2 non dice come si compone con
mille esecuzioni; (e) priorità tra avvii delle pipeline e avvii dell'API sotto la stessa quota.

**Fix.** Una riga in AD-13 Destinazioni (e il tipo nell'IPC): «Il gruppo non è atomico: il motore applica a ogni riga
lo stesso percorso di avvio dell'API (idempotenza C-4, quota FR-30, FR-13), scrive gli eventi accettati in un solo
gruppo durevole e risponde con un esito per riga (`accepted`, `duplicate`, `quota`, `rejected` con codice C-7). Il
processo ripropone solo le righe `quota`, con attesa crescente, senza bloccare la lettura finché la coda non è piena
(finding 9).» Se invece si vuole tutto o niente, il tetto del gruppo è min(1.000, quota residua) e lo dice il motore.

## 3. alta — Busta del blocco, segnale di quiete, formula del watermark

**Quote.** AD-13: il motore «scrive in Arrow IPC senza compressione, lo cifra … e lo appende»; «Su uno stream senza
blocchi il motore scrive ogni secondo un segnale durevole con l'istante di arrivo; dopo 30 s di quiete il watermark
avanza con l'orologio dei segnali». AD-16: `crates/protocol` contiene solo tipi dell'API e del protocollo dei worker.

**Le due unità.** Il motore (produttore) e il processo (consumatore) devono concordare: schema Arrow del blocco, dove
stanno `Idempotency-Key`, istante di arrivo e tipo (dati o segnale), se la chiave è in chiaro nell'intestazione Iggy (C-13
permette in chiaro solo l'intestazione degli eventi di stato) o dentro il cifrato, se il segnale è un blocco a zero
eventi con la sua chiave, se la sua posizione conta come «posizione del blocco» per l'identità di M-35. Nessun crate
è nominato per questi tipi: A li mette in `crates/engine`, B in un modulo del processo; il processo non può dipendere
dal motore (AD-16) e il test di compatibilità non esiste.

Sulla quiete: «dopo 30 s di quiete il watermark avanza con l'orologio dei segnali» non dice a quale valore. A: watermark
= arrivo del segnale − 30 s; B: watermark = ultimo tempo dell'evento + (arrivo del segnale − arrivo dell'ultimo blocco) −
30 s. Due rielaborazioni chiudono finestre diverse, che è proprio ciò che la regola promette di evitare. Altri punti
senza risposta: «stream senza blocchi» vale per un secondo o per i 30 s? Il segnale si scrive anche se nessuna pipeline è
attiva sullo stream (86.400 messaggi al giorno per stream, contro SM-2 «scrive solo quando ha qualcosa da fare»)? L'istante
di arrivo non è monotono se l'orologio dell'host salta indietro: i futuri a +60 s «rispetto al suo arrivo» e la quiete
dipendono da quell'orologio.

**Fix.** In AD-13 Ingresso: «La busta del blocco (versione, tipo `data|idle`, istante di arrivo, `Idempotency-Key`,
colonne) è un tipo del dominio (`crates/domain`, puro), con un test di andata e ritorno; il motore la scrive, il processo
la legge; la chiave sta dentro il cifrato, mai nell'intestazione.» In Tempo: «Il watermark di una pipeline dopo la
quiete = max(watermark, arrivo del segnale − quiete − ritardo tollerato)`; il motore scrive il segnale solo per gli stream con
una pipeline attiva e l'istante di arrivo è `max(precedente, adesso)`.» Se non si vuole fissare qui, una riga nelle Open
Questions con scadenza «prima della prima storia dell'ingresso a blocchi».

## 4. alta — Due proprietari della deduplicazione dei blocchi

**Quote.** AD-13: «Ogni blocco porta la sua `Idempotency-Key` (C-18): il processo scarta un blocco già visto entro la
conservazione dello stream, e l'insieme delle chiavi viste sta nel checkpoint.» AD-18: «I controlli di idempotenza (FR-6)
… li fa il motore su tutto lo stato accettato». AD-5: ogni chiave d'idempotenza comprende il tenant.

**Le due unità.**
- Storia A (motore, ingresso): applica FR-6 e C-18 all'`Idempotency-Key` del blocco: stessa chiave e stesso corpo = stessa
  risposta, senza riappendere; stessa chiave e corpo diverso = `idempotency.conflict`. Finestra: quella di FR-6.
- Storia B (processo): non si fida del motore, tiene l'insieme nel checkpoint e scarta, con finestra = conservazione dello
  stream.

Le due finestre differiscono; la chiave di un blocco e quella di un avvio dividono lo stesso spazio C-18 (la
canonicalizzazione di C-4 è già nelle «da valutare» ma non lo spazio condiviso con i blocchi). Peggio: l'insieme nel
checkpoint cresce senza regola di potatura (per tempo? per posizione? chi decide che una chiave esce?), ed è «salvato
intero» a ogni checkpoint (l'ADR lo dichiara debito).

**Fix.** Scegliere un solo proprietario. Proposta: il motore applica C-18 all'ingresso (risposta di replay, nessun
doppio blocco nel topic salvo ritentativi a risposta persa), il processo tiene la deduplicazione come seconda barriera con
potatura fissata («una chiave esce quando la posizione del suo blocco è sotto la prima posizione conservata del topic»),
e la chiave di un blocco è nello spazio `stream:<id>` distinto da quello degli avvii. Scriverlo in AD-13 Identità.

## 5. alta — Il checkpoint passa dall'archivio dei payload, ma il processo non può raggiungerlo

**Quote.** AD-17: «checkpoint delle pipeline stream … passano da una sola porta, l'archivio dei payload … due spazi»; adattatore
facoltativo RustFS (S3). AD-13: checkpoint «scritto con file temporaneo, `fdatasync`, rinomina e `fsync` della cartella»;
«Landlock con la sola cartella dei checkpoint del tenant e le sole porte del bus e del motore». AD-16: il client HTTP vive
in `crates/adapters`.

**Le due unità.** Storia A (archivio, adattatore RustFS): implementa lo spazio con nome via S3 con `If-None-Match`.
Storia B (processo): scrive file e rinomina su una cartella locale. Con RustFS configurato il processo non può chiamare
S3 (Landlock e seccomp senza altre porte, nessun client HTTP in un ausiliario), e la rinomina non è una operazione S3.
Il processo stream non può usare la porta AD-17 così com'è, e nessuna AD dice cosa fa al suo posto: o il checkpoint è
sempre su disco locale (allora non è «una sola porta», e il salvataggio dell'archivio insieme al database, riga aperta,
non lo copre), o passa dal motore sull'IPC (allora la cartella Landlock non serve e la latenza dei 10 s va misurata).

**Fix.** AD-13 Checkpoint: «Il checkpoint passa dall'IPC al motore, che lo scrive con la porta dell'archivio
(sovrascrittura atomica nello spazio con nome); la cartella Landlock del processo è vuota» oppure, in alternativa,
«Il checkpoint è sempre su disco locale del motore, fuori dall'adattatore, con retention e backup propri (aggiornare
AD-17)». In entrambi i casi nominare la chiave del file (`tenant/stream/<pipeline>/<versione>`): oggi due storie lo
chiamano diversamente, e uno o più checkpoint per processo non è fissato (l'ADR lo cita come debito).

## 6. alta — Chiave C-4 senza finestra e aggiornamento del bus

**Quote.** C-4: «senza finestra: posizione nello stream». AD-13: «L'identità di un evento è la posizione del suo blocco
(AD-18) più l'indice nel blocco». AD-18: la posizione è «generazione del bus, topic, partizione, offset», «token opaco che solo
il dominio confronta, e solo dentro la stessa generazione». AD-2/FR-51: «Aggiornare il bus = … bus nuovo e vuoto,
riversamento dal database»; «I messaggi del bus non si rileggono tra versioni».

**Le due unità.** Storia A (processo): per una pipeline senza finestra costruisce la chiave C-4 con il token della posizione
(compresa la generazione). Storia B (aggiornamento del bus): riversa dal database; i topic degli stream non sono nel
database, quindi gli eventi non riversati sono persi o ricreati da zero con offset ripartiti da 0.

Esiti: (a) con la generazione dentro la chiave, dopo l'aggiornamento tutti gli avvii di una pipeline senza finestra hanno
chiavi nuove, e la rielaborazione di eventi vecchi avvia di nuovo; (b) senza generazione, offset 0 del bus nuovo collide con
l'offset 0 del vecchio e l'avvio nuovo è assorbito come duplicato. Inoltre il checkpoint salva «la posizione letta» nella
vecchia generazione: dopo l'aggiornamento è invalida. Le righe di Open Questions su FR-51 parlano di formati, non di
questo.

**Fix.** AD-13 Identità: «L'identità di un evento è (stream, epoca, numero progressivo del blocco, indice) con un contatore
per stream scritto dal motore nel blocco, indipendente dal bus; la posizione AD-18 serve solo a rileggere. Un
aggiornamento del bus azzera posizione e insieme delle chiavi viste e continua il contatore.» E dire in AD-13 se i topic
degli stream sopravvivono all'aggiornamento (riversamento dal log vecchio di sola lettura) o se FR-51 li perde.

## 7. alta — Nuova versione della query: da dove parte, cosa si perde

**Quote.** AD-13: «una nuova versione della query parte con lo stato vuoto»; C-4: la chiave contiene la versione della
pipeline.

**Le due unità.** Storia A (processo): una nuova versione legge dalla fine del topic (nuovi eventi soltanto). Storia B: legge
dall'inizio della conservazione, per «ricalcolare» (FR-22/FR-23, «rielaborazione»). Con B migliaia di finestre vecchie si
chiudono subito e producono avvii mai richiesti con chiavi nuove (la versione cambia): esecuzioni a valanga, quote
consumate. Con A una finestra aperta della versione vecchia è persa, o chiusa con righe parziali? Le righe «in attesa di
consegna» della versione vecchia vanno consegnate o buttate? Per un po' esistono due versioni sulla stessa pipeline.

**Fix.** In AD-13 Ciclo di vita: «Una nuova versione parte dalla fine del topic al momento della pubblicazione (posizione
scritta dal motore nell'evento di pubblicazione), a meno di una richiesta esplicita di rielaborazione; la versione vecchia
si ferma subito e consegna prima le righe già chiuse, scarta le finestre aperte con un conteggio visibile.»

## 8. media — Il controllo sull'albero ha tre candidati implementatori

**Quote.** AD-13: «controllo sull'albero sintattico contro il dialetto, prima della pianificazione, obbligatorio» e «ogni
funzione o clausola nuova del dialetto passa da lì». AD-11: un solo validatore Rust, anche nel browser (FR-4). AD-6: le funzioni
che interpretano contenuto dei tenant si chiamano solo in modalità ausiliario; il ruolo a secco gira con DataFusion.

**Le due unità.** Storia A (validatore, `crates/domain`): implementa il dialetto chiuso come funzione pura con `sqlparser`
per essere validato nel kit (wasm, FR-4). Storia B (ruolo a secco): implementa lo stesso controllo dentro il processo, con
l'albero di DataFusion. Due implementazioni dello stesso elenco di funzioni e clausole divergono (esempio: `CAST` verso un
tipo accettato da una e non dall'altra; lo stesso errore con riga e colonna, C-7). Nel browser il kit non ha DataFusion, quindi
chi valida i tipi e le colonne là? La spine non dice quale parte del dialetto è del dominio e quale del ruolo.

**Fix.** AD-13 Dialetto chiuso: «L'elenco del dialetto e il controllo sintattico sono una funzione pura in `crates/domain`
(wasm compatibile), chiamata dal validatore (kit, napi), dal ruolo a secco e dal processo come barriera 1; la
pianificazione con i tipi e le colonne la fa solo il ruolo a secco.» Verificare che il test `purity.rs` ammetta il parser.

## 9. media — Memoria: limite di processo, quota per pipeline, coda di consegna

**Quote.** AD-13: «memoria (quota di stato per pipeline, pool di DataFusion, `RLIMIT_DATA` del processo)»; «Quota di stato per
pipeline (64 MiB)»; «un rifiuto per quota trattiene le righe e le riprova»; checkpoint con «righe in attesa di consegna».

**Le due unità.** Storia A (motore, supervisore): imposta `RLIMIT_DATA` fisso per tenant. Storia B (processo): quota di 64 MiB
per pipeline senza tetto sul numero di pipeline. Con 5 pipeline il processo è ucciso da `RLIMIT_DATA` prima che una sola
quota scatti, e le altre quattro cadono insieme (contro «le altre del processo continuano»). La coda delle righe in attesa non
è nella quota: con la quota del motore in rifiuto (finding 2) cresce senza limite, oppure conta nella quota e una pipeline
sana si sospende con `state_quota_exceeded` per colpa del motore. Anche il ruolo a secco ha limiti (`RLIMIT_DATA`, tempo,
profondità) non elencati: AD-13 ne dà due righe.

**Fix.** AD-13 Isolamento: «`RLIMIT_DATA` = pool di DataFusion + (numero massimo di pipeline attive per tenant, NFR-14) × 64 MiB
+ margine, calcolato dal motore; la coda di consegna ha un tetto proprio (righe e byte) e, piena, sospende la pipeline con il
codice `delivery_backlog`.» Elencare per il ruolo a secco limiti e tempo.

## 10. media — Permessi Iggy per topic e rotazione della chiave con il processo vivo

**Quote.** AD-13: «I permessi di Iggy sono per numero di topic: creazione ed eliminazione del tenant li aggiornano»; «Il motore
gli passa solo la chiave di quel tenant». C-12: «la chiave del tenant ruota tramite l'id».

**Le due unità.** Storia A (stream): crea un topic per ogni nuovo stream dopo la creazione del tenant. La credenziale del
processo non li vede (il permesso si aggiorna solo con il tenant) oppure il permesso è sull'insieme dei topic del tenant, e
non si sa se vale per quelli futuri (Iggy: per numero). Storia B (chiavi): ruota la chiave del tenant con il processo in
corso; il processo ha ricevuto la chiave a inizio e non sa decifrare i blocchi nuovi, né ha le vecchie per i blocchi più
vecchi della rotazione che deve ancora rileggere.

**Fix.** AD-13: «La creazione e l'eliminazione di uno stream aggiornano i permessi di lettura della credenziale del tenant
(oltre a quella del tenant); il processo riceve l'elenco degli stream dal motore e lo riceve di nuovo a ogni cambio. Il
processo riceve l'insieme di tutte le versioni della chiave del tenant e nuove versioni sull'IPC.» Nominare il tipo del
nome del topic (UUIDv7 dello stream, mai un nome del tenant, AD-5).

## 11. media — Ambito del watermark, pipeline senza finestra, finestre

**Quote.** AD-13: «Watermark = il tempo dell'evento più alto visto»; «una sola `SELECT` … o `UNION ALL`»; «finestre `tumble` e
`hop` nel `GROUP BY`»; C-4: «senza finestra: posizione nello stream».

**Le due unità.** Storia A (motore delle finestre): un watermark per pipeline (massimo su tutti gli stream dell'`UNION ALL`).
Storia B: un watermark per stream e il minimo tra i rami, come in un motore stream normale. Con A il ramo veloce chiude le
finestre e scarta come in ritardo il ramo lento. Poi: una `GROUP BY` senza finestra emette quando? (A: mai, stato senza
limite fino alla quota; B: ad ogni evento come un aggiornamento); `DISTINCT` senza finestra è stato illimitato; origine
delle finestre `tumble` (epoca UTC o inizio dello stream) e vincolo `hop` (durata multiplo del passo): due implementazioni
producono la «fine della finestra» di C-4 diversa, quindi chiavi diverse.

**Fix.** AD-13 Tempo e Dialetto: «Il watermark è per stream, quello della pipeline è il minimo tra i rami dell'`UNION ALL`
con segnali di quiete; le finestre sono allineate all'epoca UTC; `GROUP BY` e `DISTINCT` senza finestra sono rifiutati.»

## 12. media — Assorbimento di C-4 contro conservazione e ripresa

**Quote.** AD-13: «le righe rifatte dopo un riavvio hanno la stessa chiave e il motore le assorbe». AD-18: i controlli di C-4
li fa il motore sullo stato accettato. FR-31: le esecuzioni seguono la conservazione.

**Le due unità.** Storia A (proiezione delle esecuzioni): il vincolo di unicità della chiave C-4 sta sulla riga
dell'esecuzione, e sparisce con la conservazione dell'esecuzione (FR-31). Storia B (processo): riprende da un checkpoint più
vecchio (pipeline sospesa a lungo, ma entro la conservazione dello stream): le righe già eseguite e scadute dalla
conservazione dell'esecuzione non sono più assorbite, e partono due volte (NFR-6 violato in silenzio).

**Fix.** Aggiungere in AD-13 Destinazioni: «Le chiavi C-4 si conservano almeno quanto la conservazione dello stream più
l'intervallo del checkpoint, in una tabella propria del motore (non sulla riga dell'esecuzione); la conservazione delle
esecuzioni non la accorcia.»

## 13. bassa — Arresto del ruolo stream

**Quote.** AD-6: «l'ausiliario esce quando il socket si chiude … azzerando chiavi e segreti»; «Il ruolo delle pipeline stream …
si ferma, dopo un checkpoint finale, quando si ferma l'ultima o quando il tenant si elimina».

**Le due unità.** Storia A (supervisore): chiude il socket per fermare. Storia B (processo): aspetta un messaggio `Stop` per
fare il checkpoint finale. Se A chiude il socket, B esce senza checkpoint, e il riavvio rilegge fino a 10 s di eventi (corretto
ma non «dopo un checkpoint finale»); in eliminazione del tenant (C-13) il checkpoint finale è inutile e rallenta l'arresto.

**Fix.** Messaggio `Drain { deadline, checkpoint: bool }` sull'IPC, con tetto di tempo; alla scadenza il motore uccide il
processo. Una riga in AD-6.

---

## Ordine suggerito per chiuderli

1. Finding 1, 2 e 5 vanno in AD-13 prima dello sprint che tocca l'IPC del processo stream: sono i confini tra proprietari.
2. Finding 3, 4, 6 nella stessa riga di AD-13 Ingresso e Identità, insieme alla riga «Ordine nel log tra gruppi in volo»
   già aperta.
3. Finding 7, 11, 12 in AD-13 Ciclo di vita, Tempo e Destinazioni.
4. Finding 8, 9, 10, 13: Open Question con scadenza «spec della prima storia del processo stream» se non si fissano ora.
