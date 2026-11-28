# Seconda revisione dell'Update S-3 della spine (riscrittura di AD-13 e AD-6, commit 010189f e 00e8ba2)

**Verdetto:** la riscrittura chiude i buchi di proprietà del primo giro (stato della pipeline, canale IPC, decisione per riga, formula del watermark), ma introduce o lascia 2 punti alti di contraddizione con AD-18, C-18 e FR-6 (blocco ripetuto con corpo diverso scartato in silenzio; destinazione stream senza deduplicazione per evento) e 3 punti alti di divergenza tra storie (chiave dei checkpoint e insieme delle chiavi, righe rifiutate per un motivo diverso dalla quota, segnali per ogni stream): non è pronta per le storie del processo stream senza altre righe di Open Questions o 4-5 frasi in AD-13.

Metodo: letti il diff 00e8ba2 contro il padre di 010189f, AD-2, AD-6, AD-11, AD-13, AD-17, AD-18, le convenzioni C-1/C-4/C-7/C-18, il blocco S-3, le Open Questions, l'ADR del 2026-11-28, `spikes/stream/risultati.md`, il codice di `spikes/stream/src/pipeline.rs` e `engine.rs` (per la formula del watermark e i segnali) e i tre file del primo giro. Le sette regole elencate in «Dove la spine va oltre l'ADR» non sono contestate come non provate: sono segnalate solo dove contraddicono un'altra AD.

---

## 1. Stato dei rilievi del primo giro

Chiusi da una regola (verificati sul testo):

- Rubrica 2 e avversaria 2: canale unico = IPC di AD-6, «mai su una porta del motore»; la «transazione» è sostituita dalla decisione per riga con esito per riga.
- Avversaria 1: lo stato della pipeline è un evento di stato scritto solo dal motore; il processo segnala, il motore sospende; il checkpoint non contiene la sospensione. Restano due effetti collaterali, vedi punti 7 e 8.
- Rubrica 5 e avversaria 3 (formula): la formula di AD-13 coincide con `pipeline.rs` (`wm = max(wm, ultimo_wm + (S - ultimo_arrivo))` se `S - ultimo_arrivo >= quiete`) ed è la sola scritta; il tempo vale con tempo dell'evento vicino all'arrivo.
- Rubrica 6: il «Prevents» ora dice «nello stesso ordine»; l'ordine tra gruppi in volo resta in Open Questions.
- Rubrica 9 (parte): l'elenco chiuso e il controllo sull'albero sono una sola implementazione pura, usata alla pubblicazione e al caricamento; tipi dichiarabili elencati, decimale esatto.
- Rubrica 3 (parte) e avversaria 4: un blocco oltre il tetto è rifiutato e mai diviso; la deduplicazione ha un solo proprietario (ma vedi punti 1 e 3).
- Rubrica 13 (parte): il blocco S-3 porta ora la lista delle scorciatoie «da non portare».

Spostati in una riga di Open Questions (accettabili se la scadenza regge, vedi punto 13): checkpoint con RustFS (rubrica 1, avversaria 5); forma del corpo e tipi della busta e dei messaggi IPC (rubrica 3, avversaria 3); aggiornamento del bus e chiavi C-4 senza finestra (rubrica 10, avversaria 6); nuova versione della query (avversaria 7); coda di consegna e quota (rubrica 11, avversaria 9); watermark con `UNION ALL` e senza finestra (avversaria 11); durata delle chiavi C-4 (avversaria 12); budget di CPU (rubrica 7); luogo dell'implementazione del dialetto (rubrica 9, avversaria 8); RAM del ruolo di validazione (rubrica 8, parte).

**Ancora aperti, senza regola né riga** (tutti ripresi sotto):

- Valori e limiti del ruolo di validazione a secco in AD-6 (rubrica 8): punto 9.
- `RLIMIT_NOFILE` per ruolo in AD-6 e nota di sostituzione nell'ADR della sandbox (rubrica 12): l'ADR `2026-11-26-sandbox.md` ha ancora `Stato: accettata` e `RLIMIT_NOFILE` 64 al punto 7; AD-6 scrive «numero di file» senza valore; l'Open Question promessa dall'ADR sul 512 non c'è. Punto 12.
- Segnali di quiete solo per stream con pipeline attiva (avversaria 3): punto 5.
- Rotazione della chiave del tenant con il processo vivo (avversaria 10): punto 11.
- Messaggio di arresto con checkpoint finale e scadenza (avversaria 13): punto 8.
- Riferimenti vecchi (rubrica 13): C-1 dice ancora «l'id degli eventi degli stream lo fissa S-3, M-35» e AD-13 lo ha fissato; nome del ruolo di validazione a secco assente da `docs/glossary.md` (non verificato lì, ma AD-6 e AD-11 usano «validazione a secco» per due ruoli diversi). Bassa.

---

## 2. Findings

### 1. (alta) Blocco ripetuto con corpo diverso: 200 e scarto silenzioso, contro FR-6, C-18 e AD-18

**Quote.** AD-13 Identità: «il motore non tiene una tabella delle chiavi dei blocchi, risponde 200 anche a un blocco ripetuto, che finisce di nuovo nel log; il processo lo scarta». AD-18: «I controlli di idempotenza (FR-6), di C-3, di C-4 e di `If-Match` (C-5) li fa il motore su tutto lo stato accettato». C-18: la chiave di idempotenza (FR-6) viaggia in `Idempotency-Key`; C-4/R17 in Open Questions cita `idempotency.conflict`.

**Problema.** L'accettazione di «200 anche a un blocco ripetuto» è del product owner, ma la regola è scritta solo per lo stesso corpo. Con la stessa chiave e un corpo diverso (errore del client, o il client che riusa chiavi per contatore) il motore risponde 200 («accettato», FR-9), il blocco entra nel log e il processo lo scarta perché la chiave è già vista: eventi accettati e persi senza traccia, che AD-2 e NFR-6 dicono di impedire. FR-6 e C-18 per tutte le altre risorse danno `idempotency.conflict`. AD-18 non ha l'eccezione, quindi una storia dell'API di ingresso può applicare FR-6 (motore) e un'altra la regola di AD-13.

**Fix.** In AD-18 e in AD-13 Identità: «per i blocchi degli stream il controllo di FR-6/C-18 non si applica all'ingresso; la ripetizione è scartata dal processo; un blocco con la stessa chiave e contenuto diverso è scartato e contato (`duplicate_block_mismatch`), visibile all'operatore». Per vederlo il processo salva nel checkpoint l'impronta del blocco accanto alla chiave. Se non si vuole l'impronta, scrivere esplicitamente che il client che riusa una chiave perde il blocco e che l'SDK genera le chiavi (UUIDv7) e non lascia sceglierle.

### 2. (alta) Destinazione stream: «chiave C-4 di ogni riga» ma «nessun identificativo per evento»

**Quote.** AD-13 Identità: «Nessun identificativo per evento: la deduplicazione per evento si scrive in SQL (`DISTINCT`, FR-20)». Destinazioni: «Una destinazione stream porta la chiave C-4 di ogni riga»; «Le righe rifatte dopo un riavvio hanno la stessa chiave e il motore le assorbe». Prevents: «un evento accettato perso o contato due volte dopo un crash».

**Problema.** Per le destinazioni avvio il motore assorbe i doppioni con C-4. Per una destinazione stream i doppioni entrano nel topic (nello spike C7: 2.287 doppioni «assorbiti alla lettura»). Nella spine nessuno li assorbe: i blocchi della destinazione hanno una `Idempotency-Key` per blocco, e dopo un riavvio le stesse righe possono essere raggruppate in blocchi diversi, quindi con chiavi di blocco diverse (e lo stesso gruppo di righe non è garantito: la riga di Open Questions «come raggruppare» non lo copre). Una pipeline che legge lo stream di destinazione conta due volte quegli eventi, contro il «Prevents» e NFR-6. La chiave C-4 non è una colonna dichiarabile dello stream, quindi `DISTINCT` non la vede. Due storie (destinazione stream, lettura di uno stream che è destinazione) risolvono in modo diverso.

**Fix.** Una frase in AD-13 Destinazioni: o la chiave del blocco di una destinazione stream si deriva in modo deterministico dalle chiavi C-4 delle sue righe (e il raggruppamento in blocchi è fissato dalla chiave: righe ordinate, tetti, stesso taglio), così il processo lettore scarta il blocco ripetuto come qualsiasi altro; oppure la chiave C-4 diventa una colonna riservata dello stream di destinazione, e `DISTINCT` su di essa è documentato. In entrambi i casi la riga di Open Questions sul raggruppamento deve citare la chiave del blocco.

### 3. (alta) Insieme delle chiavi viste nel checkpoint: «solo le chiavi entro la conservazione» non lo limita, e la forma del checkpoint non è fissata

**Quote.** AD-13 Identità: «L'insieme delle chiavi viste sta nel checkpoint e tiene solo le chiavi entro la conservazione dello stream». Checkpoint: «Ogni 10 s ... posizione letta, watermark, chiavi dei blocchi viste, stato delle finestre aperte, righe in attesa». ADR e risultati: «un checkpoint per processo con le posizioni per pipeline» e «l'insieme delle chiavi viste salvato intero in ogni checkpoint» sono scorciatoie da non portare.

**Problema.** 1) Il tetto è la conservazione dello stream, per cui non c'è un numero: a 105.000 eventi/s con blocchi da 1.000 sono 105 chiavi/s, ma un client con blocchi da 10 eventi ne genera 10.500/s. Il codice dello spike conta circa 56 byte più la chiave per voce (`seen_bytes`): con una conservazione di un giorno e blocchi da 100 eventi, circa 1.000 chiavi/s, 86 milioni di voci, oltre 7 GB riscritti ogni 10 s, fuori dalla quota di 64 MiB (non è detto che la contino) e fuori da `RLIMIT_DATA`. 2) Non è detto se il checkpoint è uno per pipeline-versione o uno per processo, né il nome del file/oggetto nello spazio con nome. L'insieme delle chiavi è di stream, lo stato è di pipeline: con due pipeline sullo stesso stream o una che riparte dopo una sospensione ogni storia lo sceglie diversamente, e «le altre del processo continuano» (sospensione di una sola) richiede un checkpoint per pipeline. 3) Come il processo sa che una chiave è «entro la conservazione» (serve la posizione o l'arrivo del blocco accanto alla chiave) non è scritto. 4) Il limite di AD-6 «dimensione dei file» (`RLIMIT_FSIZE`) cade sul ruolo che scrive il checkpoint su disco, senza valore.

**Fix.** In AD-13 Checkpoint: un checkpoint per (pipeline, versione), nome `tenant/pipelines/<pipeline>/<versione>` nello spazio con nome; l'insieme delle chiavi di blocco è per pipeline, con voce = (chiave, posizione del blocco) e potatura per posizione sotto la prima posizione conservata; un tetto di voci (e quindi di byte) che conta nella quota di stato, oltre il quale la pipeline si sospende con un codice; il valore di `RLIMIT_FSIZE` del ruolo stream è maggiore della quota più il tetto. Se non si fissa ora, una riga di Open Questions con scadenza «prima della storia dell'ingresso a blocchi».

### 4. (alta) Righe rifiutate dal motore per un motivo diverso dalla quota: nessuna regola

**Quote.** AD-13 Destinazioni: «risponde riga per riga (accettata, già vista, rifiutata con il motivo). Le righe rifiutate per quota restano nel processo e si riprovano».

**Problema.** I rifiuti possibili sul percorso normale sono più della quota di FR-30: concorrenza di FR-13 (rifiuto transitorio), workflow eliminato o disattivato, versione non più valida, input che non supera lo schema del workflow (AD-10), tenant sospeso, motore con database fermo (AD-2: rifiuta il lavoro nuovo). Solo la quota ha una regola. Una storia ritenta tutto, un'altra scarta e conta i permanenti, una terza sospende la pipeline: una riga scartata è un evento accettato perso (NFR-6) senza che la spine lo dica. Anche i tempi di ritentativo (chi li pilota, con quale attesa) non ci sono, e la coda cresce senza tetto finché la riga di Open Questions sulla coda non si chiude.

**Fix.** In AD-13 Destinazioni: i motivi del rifiuto sono due classi del catalogo C-7, `transient` (quota, concorrenza, database fermo, bus giù: si riprova con attesa crescente e tetto) e `permanent` (workflow o versione assenti, input non valido: la riga va in un conteggio visibile e nei log dell'operatore e la pipeline si sospende dopo N rifiuti permanenti consecutivi, con un codice). Dire che la classe la decide il motore e la porta nella risposta per riga.

### 5. (alta) Segnali di quiete: scritti per ogni stream, non solo per quelli con una pipeline attiva

**Quote.** AD-13 Tempo: «Su uno stream che in un secondo non ha ricevuto blocchi il motore scrive un segnale durevole con l'istante di arrivo». AD-6: il ruolo stream «parte quando il tenant ha una pipeline attiva»; SM-2 (il sistema a riposo non scrive); S-3: «una sola scrittura su Iggy per motore, condivisa da tutti i tenant».

**Problema.** Il motore dello spike (`engine.rs`, ciclo ogni secondo su tutti gli stream di tutti i tenant) scrive un messaggio durevole per stream e per secondo anche senza pipeline: 86.400 messaggi al giorno per stream, su un gruppo di accettazione che porta anche le scritture di stato, per un motore a riposo. Una storia che legge «uno stream fermo» scrive i segnali per tutti, un'altra solo per quelli con pipeline attiva, con effetti diversi sull'avvio di una pipeline su uno stream fermo da giorni (il watermark deriva dai segnali del log) e sul disco a riposo. La regola contraddice anche NFR-1/SM-2 (a riposo nessuna scrittura) senza dirlo.

**Fix.** «Il segnale si scrive solo per gli stream con almeno una pipeline attiva e si ferma quando l'ultima si sospende; l'istante di arrivo è `max(precedente, adesso)`.» Un avviso: con il segnale scritto solo a pipeline attiva, una pipeline appena attivata su uno stream fermo parte con quiete pari a 30 s più il primo segnale; va scritto nella spec come comportamento voluto.

### 6. (media) Unità del tetto di 1 MiB e chiave obbligatoria: due letture

**Quote.** AD-13 Ingresso: «Blocchi di al più 1.000 eventi o 1 MiB ... un blocco oltre il tetto è rifiutato con un codice (C-7) e il motore non divide mai un blocco»; Open Questions: «chi divide gli invii oltre il tetto (SDK, con una chiave per blocco)». AD-6: «I corpi delle richieste dell'API hanno tetti di dimensione, profondità e decompressione, applicati prima dell'analisi».

**Problema.** 1 MiB di che cosa: del corpo ricevuto, del corpo decompresso o del blocco Arrow IPC che il motore costruisce? L'SDK che divide non può conoscere la dimensione Arrow: se divide per byte di JSON e il motore misura l'Arrow, un blocco accettato dal divisore è rifiutato (o il contrario). Righe con più di 50 indicatori (UJ-3) sono vicine ai tetti. «Ogni blocco porta la sua `Idempotency-Key`» non dice se è obbligatoria: senza chiave il blocco non è deduplicabile e un ritentativo del client dopo un 5xx raddoppia gli eventi; se è obbligatoria, un blocco senza è un errore C-7.

**Fix.** Una frase: «il tetto di 1 MiB è sul corpo decompresso; la chiave è obbligatoria, un blocco senza è rifiutato». L'SDK usa lo stesso conteggio (funzione pura del dominio).

### 7. (media) Sospensione decisa dal motore: cosa fa il processo tra la segnalazione e il comando, e dove vive lo stato

**Quote.** AD-13 Ciclo di vita: «il processo non si sospende da solo, segnala sull'IPC la pipeline e il codice e il motore la sospende; all'avvio il motore gli passa le pipeline attive con la loro versione». AD-2: gli eventi di stato hanno un'intestazione in chiaro con un elenco chiuso di campi, «un test fallisce se compare un altro campo»; Open Questions: «Ambito della verità sul bus: quali risorse, oltre alle esecuzioni, passano dal bus».

**Problema.** 1) Tra la segnalazione e l'ordine il processo continua a leggere e ad accumulare: la quota è superabile (82,8 MB contro 64 MiB), la CPU è già oltre il budget. Una storia ferma localmente la pipeline in attesa dell'ordine, un'altra no. Se il motore cade tra segnalazione ed evento durevole, al riavvio passa la pipeline come attiva e la storia si ripete in anello, senza che l'operatore sappia perché. 2) «Evento di stato (AD-2)» presuppone che lo stato delle pipeline passi dal bus, ma la riga di Open Questions sull'ambito della verità sul bus (workflow, quote, pianificazioni) è aperta e propone che le altre risorse non passino dal bus. 3) L'intestazione dei tipi di AD-2 contiene solo campi di esecuzione (identificativo dell'esecuzione, indice del passo, tentativo, scadenza): un evento di pipeline non ci sta senza una regola.

**Fix.** «Appena segnalata una pipeline, il processo smette di leggerla e di consegnare (stato `fault_pending`) e non lo ricorda nel checkpoint; il motore risponde `Suspend` dopo l'evento durevole, o `Resume` se l'evento non è stato scritto.» Aggiungere ad AD-2 il tipo di evento della pipeline con l'identificativo della pipeline solo nel corpo cifrato, e citare che lo stato delle pipeline chiude in senso positivo la riga sull'ambito per questa risorsa.

### 8. (media) Arresto del ruolo stream e checkpoint finale: nessun messaggio e nessuna scadenza

**Quote.** AD-6: «Il ruolo delle pipeline stream ... si ferma, dopo un checkpoint finale, quando si ferma l'ultima o quando il tenant si elimina»; «l'ausiliario esce quando il socket si chiude o il motore muore».

**Problema.** Non cambiato dal primo giro (avversaria 13). Se il supervisore chiude il socket per fermarlo, il processo esce senza checkpoint finale (e riparte rileggendo fino a 10 s di eventi: corretto ma non «dopo un checkpoint finale»); all'eliminazione del tenant (C-13) il checkpoint finale è inutile e ritarda l'eliminazione. Il checkpoint con RustFS passa per il motore (Open Question), che non lo riceve se il canale si è chiuso.

**Fix.** Una riga in AD-6: messaggio IPC `Drain { scadenza, con_checkpoint }`, il motore chiude il socket dopo la risposta o alla scadenza (kill).

### 9. (media) Ruolo della validazione a secco: valori e limiti non scritti

**Quote.** AD-13 Validazione a secco: «`RLIMIT_DATA` sul processo (nello spike 1 GiB, sul processo delle pipeline che faceva anche la validazione); oltre il limite il processo cade e il motore risponde con un codice». AD-6: «limiti (core 0, numero di file, `PR_SET_NO_NEW_PRIVS`)... limiti di CPU, memoria e tempo».

**Problema.** Rimane aperto il rilievo 8 della rubrica: `RLIMIT_DATA` è un valore dello spike su un altro processo, `RLIMIT_NOFILE` e il limite di tempo del ruolo non ci sono, la lista seccomp «propria» è solo detta, e il codice dell'errore (`validation_process_died` nell'ADR) non è nel catalogo né scritto nella spine: un nome in snake_case contro la regola di C-7 («codice stabile in inglese a punti»). Il test V3 misura un processo che muore per segnale 31 (SIGSYS) a 1 GiB: la tabella del ruolo separato non è misurata (e lo dice solo la riga sulla RAM). Due storie (supervisore dei ruoli, validazione SQL) scelgono valori diversi.

**Fix.** In AD-13 o AD-6 una riga per il ruolo: `RLIMIT_DATA` = 1 GiB «valore dello spike, da riprovare sul ruolo separato», `RLIMIT_NOFILE` 64, limite di tempo (scadenza della richiesta, per esempio 5 s) con uccisione, richiesta mai ripetuta, codice `stream.sql.validation_aborted` (da fissare nel catalogo C-7). Nei codici della spine dare il nome a punti a `state_quota_exceeded`, `cpu_budget_exceeded`, `retention_exceeded` (oggi senza nome in AD-13, solo «un codice del catalogo»), altrimenti ognuno usa i nomi dell'ADR in snake_case.

### 10. (media) Decimale esatto: nessuna precisione, nessuna forma verso l'input del workflow

**Quote.** AD-13 Dialetto: «Tipi dichiarabili delle colonne: testo, intero a 64 bit, decimale esatto (mai `f64`), booleano, istante in UTC». Destinazioni: ogni riga diventa un avvio. C-1: «tempi UTC, RFC 3339»; AD-10 (JSON Schema) e BDD («`3` non è `3.0`»).

**Problema.** Nessuna precisione e scala: DataFusion/Arrow offre decimali a 128 bit con precisione fissa; `SUM` e `AVG` su un decimale cambiano precisione, e l'overflow del decimale o dell'intero a 64 bit in un aggregato (`SUM`) non ha regola (errore? riga scartata? pipeline sospesa?). La riga diventa input di un workflow (JSON): decimale come stringa o come numero? istante in RFC 3339 con quanti decimali (Open Questions R24 sui microsecondi è «da valutare»)? Due storie producono input diversi, e C-4 usa la «chiave del gruppo» che include questi valori.

**Fix.** In AD-13: decimale = `Decimal128` con precisione 38 e scala dichiarata nello stream; overflow di un aggregato = pipeline sospesa con un codice; nell'input del workflow il decimale è una stringa decimale canonica e l'istante è RFC 3339 al microsecondo, UTC.

### 11. (media) Rotazione della chiave del tenant e nuovi stream con il processo vivo

**Quote.** AD-13: «Il motore gli passa solo la chiave di quel tenant»; «I permessi di Iggy sono per numero di topic: creazione ed eliminazione del tenant o di uno stream li aggiornano (ADR del bus)». C-12: la chiave del tenant ruota tramite l'identificativo.

**Problema.** I permessi per numero di topic sono ora detti (chiude metà del rilievo 10), ma non cosa succede alla credenziale del processo già connesso: un cambio di permessi in Iggy richiede probabilmente una nuova connessione, e il processo che non vede il topic di uno stream appena creato non lo legge mai. La rotazione della chiave con il processo vivo non è toccata: il processo ha una chiave sola, ma i blocchi più vecchi della rotazione e i checkpoint sono cifrati con la vecchia. (Il confine con AD-6 «Unica eccezione» è rispettato; manca solo la versione.)

**Fix.** «Il processo riceve sull'IPC tutte le versioni della chiave del tenant (con l'identificativo) e ogni nuova versione; riceve di nuovo l'elenco degli stream a ogni creazione o eliminazione e riapre la connessione al bus quando i permessi cambiano.»

### 12. (media) `RLIMIT_NOFILE` e ADR della sandbox: ancora in contraddizione

**Quote.** AD-13 Confinamento: «`RLIMIT_NOFILE` 512». ADR sandbox (`2026-11-26-sandbox.md`, punto 7): `RLIMIT_NOFILE` 64, `Stato: accettata`. AD-6: «numero di file».

**Problema.** Identico al rilievo 12 della rubrica: AD-6 resta senza valori per ruolo; l'ADR della sandbox non porta la nota «sostituita su questo punto» (le regole del progetto lasciano cambiare solo `Stato`); la riga di Open Questions sul 512 promessa dall'ADR («provato solo con il client di Iggy e 4 gruppi in volo») non c'è.

**Fix.** AD-6: valori per ruolo (64 per gli altri, 512 per `stream`, 64 per la validazione a secco); nell'ADR della sandbox solo `Stato: accettata, sostituita su questo punto da 2026-11-28-stream-pipelines.md`.

### 13. (media) Le righe di Open Questions nuove non sono nelle epiche e alcune scadenze non reggono

**Problema.** Le epiche (`epics.md`) elencano «le decisioni che bloccano» ogni epica; le righe nuove (ingresso, destinazione stream, quota, checkpoint con RustFS, nuova versione, durata delle chiavi C-4, luogo del dialetto) non sono riportate (lo stesso rilievo 14 della rubrica). Due scadenze sono incoerenti con l'ordine delle storie: «Dove vive l'implementazione unica del dialetto ... prima che S-4 misuri il peso del validatore» (una decisione di S-4, che è già uno spike prima dello sprint 1) e la riga sulle chiavi C-4 («prima della storia delle destinazioni») che dipende da una scelta del checkpoint (punto 3) presa in una storia prima.

**Fix.** Riportare le righe nelle epiche coinvolte e mettere la scadenza del dialetto prima di S-4 senza «o la prima storia».

### 14. (bassa) Checkpoint con adattatore su disco e porta unica: la Rule dice «tramite la porta», il confinamento dice «su disco», l'Open Question dice «IPC al motore»

**Quote.** AD-13 Checkpoint: «scritto tramite la porta dell'archivio dei payload ... con l'adattatore su disco: file temporaneo, `fdatasync`, rinomina e `fsync` della cartella». AD-17: nello spazio con nome i pezzi dei log si creano solo se assenti (collegamento fisso, `If-None-Match: *`), non si sovrascrivono mai.

**Problema.** Il processo confinato dovrebbe contenere l'adattatore su disco di AD-17 (dipendenza dal crate degli adattatori, AD-16) solo per il caso su disco, mentre con RustFS usa l'IPC: due percorsi per la stessa porta. Inoltre la porta di AD-17 ha oggi un'operazione «crea se assente» per i pezzi, e una di sovrascrittura atomica serve ai checkpoint: la porta deve nominarle, altrimenti l'adattatore RustFS non ha l'equivalente della rinomina (una `PUT` su S3 è già atomica sull'oggetto, ma va scritto).

**Fix.** In AD-17: lo spazio con nome ha due operazioni, `put_if_absent` (pezzi) e `put_replace` (checkpoint, esportazioni, sovrascrittura atomica anche con RustFS). In AD-13: il processo scrive i checkpoint sempre sull'IPC e il motore li scrive con la porta; la cartella Landlock del processo è vuota. Costo: un messaggio IPC da 64 MiB ogni 10 s per pipeline con quota piena; da misurare nella storia, ma è un solo percorso. Se invece si resta con «su disco dal processo», dirlo in AD-17.

### 15. (bassa) Righe verso il motore: validazione dell'input e dimensione del gruppo

**Quote.** AD-13 Destinazioni: «il motore decide ogni riga sul percorso di avvio normale (idempotenza C-4, quote di FR-30, concorrenza di FR-13)». AD-6: il motore non interpreta contenuto dei tenant; il validatore gira solo come ausiliario.

**Problema.** Il percorso di avvio normale include la validazione dell'input contro lo schema del workflow (AD-10, AD-11), che è un ruolo ausiliario, non il motore. Un gruppo di 1.000 righe di UJ-3 (più di 50 indicatori) è una chiamata di validazione per riga o per gruppo? Il tetto in byte di un messaggio IPC (AD-6: «tetti di dimensione e profondità») non c'è: 1.000 righe di 50 colonne superano facilmente 1 MiB.

**Fix.** «Il motore valida un gruppo con una sola richiesta al ruolo di validazione dello stesso tenant; il gruppo ha un tetto di 1.000 righe e di 1 MiB, il più piccolo dei due.»

### 16. (bassa) Riferimenti vecchi e termini

- C-1 (riga 739 della spine) dice ancora «l'id degli eventi degli stream lo fissa S-3, M-35»: sostituire con «l'identità degli eventi degli stream: AD-13».
- «avvii» nell'elenco dell'IPC di AD-13 («avvii, righe verso stream di destinazione, segnalazioni») usa la stessa parola dell'avvio di un workflow (FR-24) e dell'avvio di una pipeline: scrivere «avvii dei workflow».
- La frase «validazione a secco» designa tre cose (API dei workflow in AD-11, ruolo del SQL in AD-6, richiesta dell'API); un nome di ruolo (per esempio `stream_sql_check`) e una voce del glossario prima della prima storia.

---

## 3. Contraddizioni con altre AD (domanda 2), in sintesi

- AD-18 (idempotenza FR-6 sul motore) e C-18: punto 1. Le altre sette regole accettate sono coerenti con AD-6 (IPC, ciclo di vita), AD-2 (stato scritto solo dal motore), AD-17 (porta) e C-4, con le eccezioni dei punti 7 (intestazione degli eventi di stato, ambito della verità sul bus) e 14 (porta unica).
- AD-6 «una richiesta già scritta a un ausiliario che muore non si ripete»: coerente con la validazione a secco che cade (non si ripete) e con le righe del processo (si ripetono, ma le assorbe C-4).
- AD-2 «il proiettore non legge i topic degli stream»: coerente.

## 4. Due storie che obbediscono alle regole e costruiscono in modo incompatibile (domanda 3)

| Punto | Storia A | Storia B | Finding |
|---|---|---|---|
| Stato della pipeline | processo fermo localmente alla segnalazione | processo continua fino all'ordine | 7 |
| Decisione per riga | ritenta tutti i rifiuti | scarta i permanenti | 4 |
| Deduplicazione dei blocchi | insieme per stream, un checkpoint per processo | insieme per pipeline-versione | 3 |
| Quiete | segnali per ogni stream | solo con pipeline attiva | 5 |
| Checkpoint | il processo scrive su disco | il processo passa dall'IPC | 14 |
| Dry-validation | `RLIMIT_DATA` 1 GiB, 64 file | valori propri | 9 |
| Destinazione stream | chiave di blocco libera | chiave derivata dalle righe | 2 |

## 5. Regole presentate come fatto che l'ADR o gli spike contraddicono (domanda 4)

Fuori dalle sette regole accettate non ne ho trovate di contraddette, ma tre affermazioni vanno qualificate:

- «misurato al più 39,9 s con i predefiniti» accanto alla formula: con la formula (`wm = ultimo_wm + (S − A)`, quiete 30 s dal **blocco**, non dalla fine della finestra) la chiusura cade a circa `max(fine + 10 s, A + 30 s)` se il tempo dell'evento è vicino all'arrivo, quindi al più 30 s circa dopo la fine; il 39,9 s misurato dallo spike include anche il ritardo di lettura e il gruppo di scrittura. «Circa ritardo + quiete dopo la sua fine» è un tetto largo, non la formula; basta scrivere «al più circa 40 s dopo la fine, con tempo dell'evento vicino all'arrivo».
- Il budget di CPU «proposta 30 ms» è citato come regola con i valori; mai scattato nelle corse (Open Question): già nella spine come proposta.
- «Una lista seccomp riscoperta su DataFusion e sul client del bus» vale per il ruolo con il canale HTTP dello spike verso il motore (38 syscall); con l'IPC su socket ereditato la lista del ruolo reale cambia e non è misurata. Aggiungere alla riga di S-3 «non misurato: il ruolo con l'IPC al posto di HTTP».
