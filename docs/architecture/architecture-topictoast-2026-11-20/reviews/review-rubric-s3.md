# Revisione con la rubrica dell'Update post-S-3 (spine: AD-13, AD-6, Stack, S-3, Open Questions; PRD; epiche)

**Verdetto:** l'Update regge i numeri e le regole dell'ADR, ma lascia 2 punti alti di divergenza (checkpoint contro AD-17 e il canale verso il motore contro l'IPC di AD-6; il formato dell'API di ingresso e i suoi blocchi) e diversi medi: da correggere prima della prima storia delle pipeline stream, non bloccanti per il merge dell'Update se diventano righe di Open Questions.

Metodo: letti l'ADR del 2026-11-28 per intero, `spikes/stream/risultati.md` (numeri, soglie, scorciatoie), il diff dell'Update, e le definizioni di AD-5, AD-6, AD-7, AD-12, AD-17, AD-18, C-1, C-4, C-7, C-12, C-18 nella spine. I numeri della spine (105.000 eventi/s, 64 MiB, 82,8 MB, 38,3 ms, 39,9 s, 10 s / 60 s / 30 s / 10 s, 1.000 eventi o 1 MiB, 4 gruppi da 1.024, 512 file, 50.000 su 100.000, 16 KiB / 1 KiB / 64) tornano tutti con l'ADR. Quello che manca è sotto.

---

## 1. (alta) Checkpoint: file su disco e `fsync` contro la porta unica di AD-17 e contro i soli «porte del bus e del motore»

- **Riga:** AD-13, Checkpoint: «nello spazio con nome dell'archivio dei payload, scritto con file temporaneo, `fdatasync`, rinomina e `fsync` della cartella»; Confinamento: «Landlock con la sola cartella dei checkpoint del tenant e le sole porte del bus e del motore».
- **Problema:** AD-17 dice che i checkpoint passano da una sola porta con due adattatori (disco o RustFS). La meccanica di AD-13 (rinomina, `fsync` della cartella, Landlock su una cartella) vale solo per l'adattatore su disco. Con RustFS il processo, confinato su bus e motore, non può scrivere sull'archivio: o apre una porta in più (e Landlock cambia), o passa dal motore, o scrive su disco fuori dall'archivio. Due storie (archivio dei payload, pipeline stream) divergono. Il processo scrive direttamente o chiede al motore? La spine non lo dice.
- **Correzione:** in AD-13 scrivere che il processo scrive i checkpoint tramite la porta dell'archivio (AD-17) nel suo spazio con nome, e dire come con RustFS (via IPC/motore, oppure porta RustFS nella regola Landlock). Il testo su `fdatasync` e rinomina va riferito all'adattatore su disco, e AD-17 deve dire che il pezzo di checkpoint è sovrascrivibile in modo atomico anche su RustFS.

## 2. (alta) Il processo delle pipeline parla al motore per IPC (AD-6, AD-18) o su una porta (AD-13)? Nessun controllo di chi è

- **Righe:** AD-13: «Landlock con ... le sole porte del bus e del motore»; AD-13 Destinazioni: «consegna le righe a gruppi di al più 1.000, che il motore accetta in una transazione»; AD-18: «Le righe e gli avvii scritti dal processo delle pipeline stream passano dall'IPC del motore»; AD-6 IPC: «coppia di socket creata con chiusura all'esecuzione, e ogni figlio riceve solo la propria estremità».
- **Problema:** una porta del motore è un canale TCP, l'IPC di AD-6 no. Con una porta serve un'autenticazione del processo come «tenant X» (AD-5: il tenant viene solo dalla credenziale verificata); la scorciatoia dello spike «HTTP senza TLS» è nell'ADR tra quelle da non portare nel prodotto, ma la spine non lo scrive. Due storie scelgono canali e credenziali diversi.
- **Correzione:** fissare in AD-13 un solo canale (preferibile l'IPC di AD-6, con tenant implicito nella coppia di socket) e togliere «porte del motore» dalla regola Landlock, oppure dire qual è la porta e come si autentica. Il formato di «gruppo di righe» sul canale (contenuto, chiave C-4 per riga, risposta per riga o per gruppo) va nominato o messo in Open Questions.

## 3. (alta) Formato dell'API di ingresso (FR-18) e sorte di un blocco troppo grande: nessuna regola e nessuna riga aperta

- **Riga:** AD-13 Ingresso: «Blocchi di al più 1.000 eventi o 1 MiB, con le colonne tipizzate dichiarate dallo stream e validate al confine»; AD-12: «Gli eventi degli stream si inviano in blocchi».
- **Problema:** non è detto in che forma il client manda il blocco (righe JSON? colonne? Arrow?), chi suddivide un invio più grande (client, SDK o motore), né cosa fa il motore con un blocco sopra il tetto. Se il motore divide, la `Idempotency-Key` per blocco (C-18) non è più per blocco e la deduplicazione del punto sotto salta. Lo schema OpenAPI, l'SDK Rust, l'SDK TS e il kit divergono. Il tetto di 1 MiB si confronta anche con i tetti dei corpi di AD-6 («tetti di dimensione, profondità e decompressione») senza dire quale vince.
- **Correzione:** aggiungere in AD-13 (o AD-12): corpo JSON a righe con le colonne dello stream, un blocco sopra 1.000 eventi o 1 MiB è rifiutato con un codice C-7 (mai diviso dal motore), l'SDK divide e assegna una chiave per blocco. Se non lo si fissa ora, una riga di Open Questions «forma del corpo di FR-18», con scadenza «prima della storia dell'ingresso a blocchi».

## 4. (media) La deduplicazione per blocco entra in conflitto con AD-18 e non ha un ambito

- **Riga:** AD-13 Identità: «Ogni blocco porta la sua `Idempotency-Key` (C-18): il processo scarta un blocco già visto entro la conservazione dello stream, e l'insieme delle chiavi viste sta nel checkpoint».
- **Problemi:**
  1. AD-18 assegna al motore i controlli di idempotenza (FR-6) su tutto lo stato accettato; qui la scarta il processo a valle, quindi un secondo invio con la stessa chiave riceve un 200 nuovo e ripete la scrittura sul bus, mentre per le altre risorse l'API restituisce l'esito originale. La differenza di semantica verso il client non è dichiarata.
  2. L'ambito della chiave (per stream? per tenant? per pipeline?) non è detto, e AD-5 vuole il tenant nella chiave.
  3. L'insieme delle chiavi cresce con la conservazione dello stream ed entra intero in ogni checkpoint; l'ADR lo elenca tra le scorciatoie da non portare, la spine tace. A 100.000 eventi/s e blocchi da 1.000 sono 100 chiavi/s: 8,6 milioni al giorno.
- **Correzione:** dire in AD-13 che la ripetizione dello stesso blocco è silenziosa e che l'API risponde 200 con lo stesso corpo, quale ambito ha la chiave, e aggiungere una riga di Open Questions sul limite dell'insieme (finestra di chiavi, o chiavi che scadono con la conservazione) prima della storia dell'ingresso.

## 5. (media) Il tempo di chiusura su stream fermo: la spine non lo fissa, il PRD lo promette quasi

- **Righe:** AD-13 Tempo: «dopo 30 s di quiete il watermark avanza con l'orologio dei segnali»; PRD FR-21: «con questi l'ultima finestra di uno stream fermo si chiude circa 40 s dopo la sua fine (misurato al più 39,9 s)».
- **Problemi:**
  1. L'ADR dice «Non si promette 40 s» e ricorda 41,5 s in prove con binari precedenti, con soglia di 42 s; il PRD scrive «circa 40 s» con solo il 39,9 s misurato, e un lettore ne ricava un impegno (NFR-14 parla di «tempo documentato»).
  2. La formula del watermark con i segnali non è univoca: a quale valore sale (l'istante del segnale meno il ritardo? l'istante del segnale?), da quando si contano i 30 s (ultimo blocco o ultimo evento), e cosa succede se il tempo degli eventi è molto indietro rispetto all'arrivo (rielaborazione di dati storici): con la regola attuale 30 s di quiete chiudono tutto il passato. Due implementazioni chiudono finestre diverse.
- **Correzione:** AD-13: scrivere la formula (watermark = max(watermark, istante di arrivo dell'ultimo segnale − 0?) e da quale istante parte la quiete) e dichiarare che il tempo vale con tempo dell'evento vicino al tempo di arrivo. PRD FR-21: «entro circa 40 s più la lettura (misurato 39,9 s; fino a 41,5 s con binari precedenti), non è una garanzia».

## 6. (media) Il «Prevents» di AD-13 dichiara più di quello che la Rule garantisce

- **Riga:** AD-13 Prevents: «finestre che una rielaborazione chiude in un punto diverso»; Open Questions: «Ordine nel log tra gruppi in volo (4 sulla stessa connessione): dipende dalle corsie del client di Iggy e non è provato».
- **Problema:** il determinismo vale solo se l'ordine del log è ripetibile; la riga aperta dice che non è provato. Il «Prevents» è presentato come cosa che la regola impedisce.
- **Correzione:** scrivere in AD-13 «finestre ... a parità di ordine nel log (Open Question sull'ordine tra gruppi in volo)», e rendere esplicito che l'ordine nel log, non quello di arrivo al motore, è quello che conta.

## 7. (media) Il budget di CPU è una regola senza valore e senza prova; il valore, il codice e il perimetro mancano

- **Riga:** AD-13: «budget di CPU per micro-lotto: la pipeline che li supera si sospende con il codice»; ADR: «proposta: 30 ms, `cpu_budget_exceeded`»; Open Question: «Budget di CPU ... mai scattato».
- **Problema:** la Rule non è applicabile: manca il valore (nemmeno come predefinito), la CPU misurata è quella di tutto il processo (scorciatoia dell'ADR), non quella della pipeline. Il perimetro del tenant (FR-25: quote per tenant) contro «per pipeline» e se la quota di stato 64 MiB è predefinita o fissa (PRD: «predefinita») non è detto.
- **Correzione:** in AD-13 scrivere «predefinito 64 MiB e 30 ms per micro-lotto, entrambi configurabili entro i massimi di NFR-14 (C-19)», con il budget di CPU «proposta da confermare» finché la riga di Open Questions non è chiusa; elencare i codici (`state_quota_exceeded`, `cpu_budget_exceeded`, `retention_exceeded`, `validation_process_died`) come voci del catalogo C-7, oggi nessuno è nella spine.

## 8. (media) Ruolo della validazione a secco: perimetro incompleto in AD-6

- **Righe:** AD-6 «validazione a secco del SQL (con DataFusion, senza accesso al bus)», «un ruolo «a richiesta» proprio»; AD-13 barriera 4 «`RLIMIT_DATA` del processo».
- **Problemi:**
  1. Quale processo ha `RLIMIT_DATA` e con quale valore: il test V3 dell'ADR lo prova sulla validazione (1 GiB, risultati.md), ma AD-6 nell'elenco dei limiti scrive solo «core 0, numero di file, `PR_SET_NO_NEW_PRIVS`» e la barriera 4 in AD-13 non dice se vale per il ruolo `stream`, per quello di validazione o per entrambi. L'unico che serve davvero a fermare un processo ostile è quello della validazione, e quel valore (e il suo `RLIMIT_NOFILE`, e la lista seccomp) non sono scritti.
  2. La morte del processo per limite avviene con un segnale di seccomp (31, `SIGSYS`), non con un errore pulito: la Rule dice «errore `validation_process_died`» ma non che il motore lo traduce da un'uscita per segnale, né se la richiesta ripetuta è sicura (AD-6: «non si ripete su un processo nuovo»; per una validazione è idempotente, e la regola generale dice il contrario).
  3. Con Landlock presente, quale ruleset ha il ruolo: nessuna cartella e nessuna porta. Scritto solo per il caso senza Landlock.
  4. L'avvio a freddo del ruolo di validazione con DataFusion non è misurato (la misura è del ruolo `stream`), ma AD-6 estende l'obiettivo p99 ≤ 250 ms a tutti i ruoli e la spine non lo mette tra i non misurati.
  5. Da dove prende il ruolo le colonne dichiarate degli stream del tenant (serve per validare): passa dal motore sull'IPC, con solo gli stream del tenant? La frase «lo stream di un altro tenant dà lo stesso errore di uno inesistente» lo implica ma non lo dice.
- **Correzione:** una riga in AD-6 per il ruolo: limiti (`RLIMIT_DATA` 1 GiB come valore provato, `RLIMIT_NOFILE` da fissare), Landlock «nessuna cartella e nessuna porta», contesto passato dal motore, e una riga in Open Questions o nei «non misurati» per l'avvio a freddo e la RAM del ruolo (oggi la RAM è citata solo nella riga di NFR-1).

## 9. (media) Controllo sull'albero: dove gira alla pubblicazione e dove all'avvio, e chi tiene l'elenco chiuso

- **Righe:** AD-13 Dialetto: «un elenco chiuso di funzioni e aggregati», «`CAST` verso i tipi dichiarabili»; barriere: «controllo sull'albero ... **obbligatorio**».
- **Problemi:**
  1. L'elenco delle funzioni, degli aggregati e dei tipi dichiarabili è il punto di divergenza principale del dialetto, ed è né nella spine né nell'ADR: sta solo in `spikes/stream/src/dialect.rs` e in `PIANO.md`, che sono fuori dal gate (CLAUDE.md: gli spike decidono, risultati negli ADR). Due storie (la validazione e l'esecuzione) possono scrivere due elenchi.
  2. Il controllo sull'albero gira nel ruolo di validazione a secco alla pubblicazione, ma «obbligatorio» prima della pianificazione vale anche nel processo `stream` al caricamento; la spine non dice se si ripete lì (difesa in profondità, costa poco) né chi ne è il proprietario nel codice (un solo modulo del dominio compilabile per entrambi i ruoli?). Il rischio è due implementazioni.
  3. Il tipo «decimale»: lo spike usa `f64` (scorciatoia da non portare); la spine non lo dice e non fissa il tipo decimale del prodotto, mentre AD-4 chiede tipi esatti.
- **Correzione:** in AD-13 scrivere che l'elenco chiuso (funzioni, aggregati, tipi) vive in un solo posto del dominio e che il controllo si esegue sia alla pubblicazione sia al caricamento; elencare i tipi dichiarabili (testo, intero a 64 bit, decimale, booleano, istante UTC, da `PIANO.md`) e dire che il decimale non è `f64`. Il passaggio dell'elenco iniziale dallo spike all'ADR o a un file del dominio è un'azione della storia del controllo, da citare.

## 10. (media) Identità dell'evento e C-4 legati alla posizione del bus, ma l'aggiornamento del bus riparte da vuoto

- **Righe:** AD-13: «L'identità di un evento è la posizione del suo blocco (AD-18) più l'indice nel blocco»; C-4: «senza finestra: posizione nello stream»; Open Question: «I messaggi del bus non si rileggono tra versioni: l'aggiornamento parte da un bus nuovo e vuoto e riversa dal database».
- **Problema:** la posizione di AD-18 comprende la generazione del bus, che cambia a ogni aggiornamento. Gli eventi degli stream stanno solo sul bus e non nel database (nessuna proiezione): un aggiornamento li perde o li rinumera, il checkpoint con la posizione letta diventa inutilizzabile, e una pipeline senza finestra (C-4: chiave = posizione nello stream) produce chiavi nuove per le stesse righe. La riga «Versione del formato di payload, checkpoint ed export» non nomina i topic degli stream.
- **Correzione:** aggiungere alla riga esistente (FR-51) «topic degli stream e posizione nei checkpoint e in C-4», oppure scrivere in AD-13 cosa si fa degli eventi non ancora consumati all'aggiornamento.

## 11. (media) Coda di consegna e insieme di chiavi nel checkpoint: crescita non limitata, scorciatoie dello spike non portate nella spine

- **Riga:** AD-13 Checkpoint: «righe in attesa di consegna»; Destinazioni: «un rifiuto per quota trattiene le righe e le riprova».
- **Problema:** l'ADR elenca «coda di consegna senza limite» e «insieme delle chiavi viste salvato intero» come scorciatoie da non portare; la spine non porta né la lista né un limite. Con una quota piena a lungo, le righe in attesa nel checkpoint crescono senza fine, e la quota di stato di 64 MiB non le conta (non è detto). Il comportamento sotto rifiuto prolungato (sospendere la pipeline con un codice?) manca.
- **Correzione:** in AD-13: la coda di consegna conta nella quota di stato; superata, la pipeline si sospende con un codice (come per lo stato). Portare la lista delle scorciatoie dello spike da non copiare (HTTP senza TLS, decimali `f64`, corpo degli avvii in chiaro nel database, coda illimitata, insieme delle chiavi intero, budget di CPU sul processo) nel blocco S-3 come «non portare», così la storia le trova.

## 12. (media) Punto aperto non chiuso: `RLIMIT_NOFILE` e la riga dell'ADR della sandbox restano in contraddizione

- **Righe:** AD-13: «`RLIMIT_NOFILE` 512»; ADR della sandbox, punto 7: «`RLIMIT_NOFILE` 64»; ADR S-3 punto 13: «la riga corrispondente dell'ADR della sandbox, che questo ADR sostituisce su questo punto».
- **Problema:** l'ADR della sandbox ha ancora `Stato: accettata` senza «sostituita ... su questo punto», e AD-6 dice solo «numero di file» senza valore. Una storia dei processi ausiliari che legge l'ADR della sandbox mette 64 su tutti i ruoli. Inoltre il 512 è una scelta di Marco non provata (l'ADR dice «64 non è stato provato», e il 512 non è provato come sufficiente), e la riga di Open Questions che l'ADR prometteva («`RLIMIT_NOFILE` del ruolo `stream`») non c'è: le righe sono sei, ma una diversa.
- **Correzione:** in AD-6 scrivere i valori per ruolo (64 per gli altri, 512 per `stream`, valore per la validazione a secco), e annotare nell'ADR della sandbox (solo `Stato`/nota) il rimando a quello nuovo; riga di Open Questions per «512 provato solo con 4 connessioni in volo e il client di Iggy», oppure togliere la promessa dall'ADR.

## 13. (bassa) Riferimenti vecchi nella spine

- **C-1:** «l'id degli eventi degli stream lo fissa S-3, M-35» è ormai fissato da AD-13 (posizione del blocco + indice). Sostituire con «l'identità degli eventi degli stream: AD-13».
- **S-3 chiuso:** il blocco non registra il criterio NFR-1 (motore con DataFusion 24,6 MB contro 10,3 senza, +14 MB; binario 128,3 contro 13,6 MB). Aggiungere una riga, perché NFR-1 (≤ 80 MB) si legge con 40,3 MB + 14 MB = vicino al tetto, e la riga di Open Questions sulla RAM dice solo «64 MB per tenant».
- **AD-6 Binds:** non cita FR-19 (validazione del SQL a secco) e NFR-6; AD-13 invece sì. Minimo.
- **Termine «validazione a secco»:** compare per due cose (validazione a secco dell'API, AD-11; validazione a secco del SQL, AD-6/AD-13) e due ruoli diversi (`validazione` e il nuovo ruolo). Un nome fisso del ruolo, per esempio `stream_sql_check`, evita errori di glossario (`docs/glossary.md`: il nuovo concetto va aggiunto prima di usarlo, costituzione VIII).
- **Formattazione:** le righe di AD-6 «Processi e ciclo di vita» e «Confinamento» hanno interruzioni di riga irregolari dopo l'Update (righe lunghe in mezzo a righe corte); non cambia il senso.

## 14. (bassa) PRD ed epiche

- **PRD NFR-5:** corretto con l'ADR; manca il perimetro «misurato su minisheep, caso di UJ-3, assunzione finché non arriva un caso reale» (c'è la dicitura UJ-3, non «assunzione»). La domanda aperta n. 1 resta aperta, bene.
- **PRD FR-23:** «meno di 10 ms in 30 s, segnali e statistiche compresi» è una soglia misurata a 7,7-9,2 ms («vicine alla soglia» nell'ADR) e va scritta come misura, non come garanzia.
- **PRD NFR-1:** non è stata aggiornata la nota con il costo di 14 MB di DataFusion e con i 64 MB per tenant con pipeline attive (la riga di Open Questions lo rimanda, accettabile).
- **Epiche:** la riga S-3 è coerente; le nuove righe di Open Questions (ingresso, destinazione stream, quota, budget di CPU, RAM, ordine) non sono riportate nelle epiche come «decisioni che bloccano» le storie corrispondenti, come invece chiede l'impostazione di `epics.md` («ogni epica elenca le decisioni che la bloccano»).

---

**Cosa regge:** AD-13 e AD-6 non si contraddicono su dove gira il SQL (mai nel motore) e sul ruolo separato; il «Prevents» sul processo ostile è coerente con la riga «una validazione non fa cadere le pipeline»; il confine con AD-5 (topic per tenant con identificativo interno, credenziale per tenant a sola lettura), AD-7 (chiave del tenant, checkpoint cifrato), C-12 e C-13 è coerente; C-4 e AD-18 sul passaggio delle righe con id deterministico sono coerenti con la consegna a gruppi; i numeri e le soglie dell'ADR sono riportati senza invenzioni, e le soglie mancate (V2, budget di CPU) sono dichiarate.
