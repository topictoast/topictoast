# Revisione avversaria dell'Update post-S-1 (spina, AD-2, AD-12, AD-17, AD-18, C-13)

**Verdetto:** non pronta per lo sprint 1 così com'è. Le regole nuove sono coerenti una per una, ma restano 3 buchi critici o alti dove due unità che le rispettano alla lettera costruiscono cose incompatibili: la cancellazione del tenant contro il proiettore unico, la corsa tra annullamento, scadenze e esiti sulla stessa esecuzione, e il token di posizione dopo un aggiornamento del bus.

Metodo: per ogni punto, due unità un livello sotto la spina (storie o moduli di persone diverse), come divergono, regola minima che chiude.

---

## H-1 (critica) Cancellazione del tenant contro il proiettore unico

- **Unità A:** storia del proiettore. Rispetta AD-2: «un evento che il proiettore non riesce ad applicare è un guasto, il proiettore si ferma su quella posizione, senza saltarla»; il bus non elimina posizioni non applicate; un solo proiettore per tutti i tenant.
- **Unità B:** storia della cancellazione del tenant. Rispetta C-13: cancella la chiave, poi le righe del database tramite l'adattatore del repository, poi la pulizia asincrona.
- **Divergenza:**
  1. Dopo la cancellazione della chiave il proiettore non decifra più gli eventi non ancora applicati del tenant. Per AD-2 è un guasto: si ferma su quella posizione e blocca la proiezione di tutti gli altri tenant, e il motore rifiuta lavoro nuovo per tutti. Un tenant che se ne va ferma l'installazione.
  2. C-13 fa cancellare righe proiettate a un'unità che non è il proiettore, contro «nessun'altra unità scrive lo stato proiettato» (AD-2). Se il proiettore applica poi un gruppo ancora in volo, ricrea righe di un tenant cancellato; con Postgres le due scritture sono in transazioni diverse, con SQLite passano dallo stesso mutex ma l'ordine non è definito.
  3. Il topic `state-<id>` non si può eliminare finché il proiettore non lo ha applicato, ma il proiettore non può applicarlo senza chiave: nessuno ha il permesso di eliminarlo.
- **Fix:** (a) la lapide del tenant è un evento di stato sul topic del tenant, e il proiettore la applica in ordine: applicata la lapide, scarta senza errore ogni evento successivo e precedente non ancora applicato di quel tenant (è l'unico caso in cui salta una posizione, e lo fa solo perché esiste la lapide applicata), poi cancella le righe lui stesso; l'adattatore del repository non ha un metodo di cancellazione del tenant fuori dal proiettore. (b) La chiave si cancella solo dopo che il proiettore ha applicato la lapide. (c) Il topic del tenant si elimina dopo la lapide applicata, dalla pulizia asincrona.

## H-2 (alta) Intestazione in chiaro contro C-13 e AD-5

- **Unità A:** motore che scrive l'evento di stato. Rispetta AD-2: intestazione in chiaro con tipo, esecuzione, passo, tentativo, scadenza.
- **Unità B:** storia della cancellazione crittografica e del sentinella. Rispetta C-13 («eventi dei tenant cifrati») e AD-5 (nessun valore scelto dal tenant in nomi o chiavi).
- **Divergenza:** «passo» è il nome di un task del workflow, scelto dall'integratore: contenuto di tenant in chiaro sul bus, che sopravvive alla cancellazione della chiave e finisce nel sentinella. C-13 dice che gli eventi sono cifrati, AD-2 dice che l'intestazione non lo è: due letture opposte. L'identificativo interno del tenant resta nel nome del topic dopo la cancellazione. `scadenza` in chiaro rivela i ritmi di un tenant.
- **Fix:** l'intestazione porta solo identificativi opachi (UUIDv7 dell'esecuzione, dell'attività, numero del tentativo; il «passo» è l'id dell'attività, mai il nome); C-13 si riscrive «eventi cifrati, tranne l'intestazione di AD-2», con la lista chiusa dei campi in chiaro e un test che fallisce se ne compare un altro. La scadenza in chiaro si tiene solo se serve al sentinella, altrimenti si toglie.

## H-3 (alta) Annullamento, scadenza dell'esecuzione, scadenza del tentativo e esito sulla stessa esecuzione

- **Unità A:** gestore del comando di annullamento (API). Scrive l'evento `Cancel` sul bus, poi risponde, poi manda `Stop`.
- **Unità B:** modulo dei timer. Un timer scade e scrive `AttemptDeadline` o `ExecutionTimeout`.
- **Unità C:** gestore dell'esito del worker. Scrive `Completed`.
- **Divergenza:** AD-2 dice che le code di scrittura sono FIFO per esecuzione, ma non dice chi decide la validità di un evento prima di scriverlo. Tre produttori accodano su un'esecuzione, ognuno controllando «lo stato accettato» (proiezione più rilettura): se il controllo e la scrittura non sono atomici per esecuzione, A scrive `Cancel` mentre C ha già controllato e scrive `Completed`; il log contiene entrambi e la proiezione, idempotente ma applicata in ordine, produce uno stato che dipende dall'ordine di arrivo. Inoltre `ExecutionTimeout` e `Cancel` possono comparire entrambi: stato finale «scaduta» o «annullata»? Non è fissato. Il timer di scadenza «parte dall'accettazione» ma la tabella dei timer è scritta dalla proiezione, quindi in ritardo: un timer si arma in memoria o dalla tabella? Due storie diverse scelgono diversamente, e dopo un crash la ripresa dà due scadenze o nessuna.
- **Fix:** una regola sola: il motore tiene un attore per esecuzione (un solo task che serializza controllo e scrittura degli eventi di quell'esecuzione, per tutti i produttori: API, timer, esiti dei worker). Un evento terminale vince su tutti i seguenti (già `Completed`/`Cancelled`/`TimedOut` si rifiutano). Precedenza fissa tra terminali nello stesso istante: il primo scritto sul log, e la proiezione lo deriva dall'ordine del log, mai dall'orario. I timer si armano in memoria dal motore al momento della scrittura dell'evento che li causa; la tabella è solo la copia per la ripresa e il vincolo di unicità (al più un timer per attività) è sulla tabella.

## H-4 (alta) Token di posizione dopo l'aggiornamento del bus

- **Unità A:** proiettore. Salva la posizione applicata per partizione (generazione, topic, partizione, offset) nella stessa transazione del gruppo.
- **Unità B:** API che aspetta il token di scrittura (AD-18) e SSE che riprende da `Last-Event-ID`.
- **Divergenza:** dopo l'aggiornamento (bus nuovo e vuoto, riversamento dal database, generazione nuova) gli offset ripartono da zero. «Solo il dominio confronta» il token, ma l'ordine tra generazioni non è definito. Un token della generazione vecchia: l'API aspetta 1 s che la proiezione lo raggiunga, ma la proiezione è già nella generazione nuova, quindi o il confronto dà «mai raggiunto» (503 per sempre) o dà «già raggiunto» (letture non coerenti con la scrittura). `Last-Event-ID` vecchio: non è «fuori dalla conservazione» (non c'è conservazione di una generazione chiusa), quindi l'errore C-7 non scatta e un client riprende da un offset che indica altro. Il riversamento dal database riproduce gli eventi con offset nuovi: la chiave di idempotenza della proiezione deve essere l'id dell'evento, non la posizione, e non è detto.
- **Fix:** (a) il confronto tra token è definito solo nella stessa generazione; un token di una generazione diversa dalla corrente vale «fuori dalla conservazione» (errore C-7 che chiede di ricaricare l'istantanea) e, per l'attesa della scrittura, «già raggiunto» se la generazione è più vecchia (l'aggiornamento è avvenuto dopo che la proiezione ha raggiunto la pausa). (b) Prima di cambiare generazione il motore aspetta la proiezione allineata (già nella procedura di drain: dirlo nella regola). (c) La proiezione è idempotente per id dell'evento, mai per posizione; la posizione serve solo alla ripresa.

## H-5 (alta) «Accetta e avvisa» con il database lento contro AD-18 e la proiezione

- **Unità A:** gestore di accettazione. Con il database lento ma attivo continua ad accettare (AD-2) e avvisa oltre 1 s.
- **Unità B:** controlli di idempotenza, C-3, C-4, `If-Match` e attesa del token (AD-18).
- **Divergenza:** (1) i controlli leggono «proiezione fino alla posizione applicata più il bus riletto da lì»: con il ritardo che cresce, il costo di ogni controllo cresce con il ritardo (rilettura sempre più lunga), quindi più lento il database, più lenta l'accettazione, fino a rendere vana la promessa di continuare ad accettare; il tetto è una Open Question. (2) Due richieste con la stessa chiave d'idempotenza arrivate insieme controllano entrambe lo stato, non trovano nulla e scrivono due eventi: la regola dice cosa controllare, non che controllo e scrittura siano atomici per chiave. (3) L'attesa del token fino a 1 s dà 503 con `Retry-After` per ogni scrittura mentre il ritardo supera 1 s: il motore ha accettato, l'API dice 503. Un client che ritenta può duplicare l'azione.
- **Fix:** (a) il motore tiene in memoria un indice dello stato accettato non ancora proiettato (chiavi di idempotenza, versioni, scadenze, avvii) per tenant, aggiornato alla scrittura del bus, e i controlli leggono quello più la proiezione: niente rilettura del bus nel percorso caldo; la rilettura serve solo alla ripresa. (b) Il controllo e la scrittura per la stessa chiave stanno nella stessa sezione serializzata (per tenant e chiave). (c) Un 503 per attesa scaduta porta anche la risposta di accettazione già durevole: la scrittura è accettata, il 503 riguarda solo la lettura; il client usa la stessa chiave d'idempotenza e riceve l'esito originale. (d) Tetto: oltre una soglia scritta (da fissare con Marco) lo stato è «database fermo».

## H-6 (media) Righe di log: numerazione, ripresa e due topic

- **Unità A:** scrittore dei pezzi nel motore (AD-17). Numera le righe e crea i pezzi solo se assenti.
- **Unità B:** proiettore o lettore del tempo reale che segue «il topic dei log del tenant».
- **Divergenza:** (1) la numerazione e i confini dei pezzi stanno nella memoria del motore: dopo un crash il motore nuovo ricomincia a numerare, il worker (che per C-2 non ha una sessione continua) rinvia o no le righe, e un pezzo con lo stesso nome ma contenuto diverso viene rifiutato dalla regola «non si sovrascrive mai»: righe perse o errore. (2) Il token di posizione è per topic e partizione: stato e log stanno su due topic dello stesso tenant, ma l'id SSE è un solo token; a quale topic si riferisce? Due storie (stato e log) scelgono diversamente. (3) Dopo un annullamento o una scadenza le ultime righe del tentativo servono per capire cosa è successo, ma «una riga di un tentativo non più corrente è rifiutata»: il motore perde proprio quelle.
- **Fix:** (a) il motore numera ripartendo dal riferimento dell'ultimo pezzo sul topic dei log (la ripresa lo rilegge) e il worker rinvia con il numero di riga che ha già ricevuto conferma, quindi il pezzo si ricostruisce identico o ricomincia da un numero nuovo mai riusato; (b) i log hanno un token proprio, `Last-Event-ID` separato per flusso (`state` e `logs`), mai uno solo per due topic; (c) le righe di un tentativo chiuso da timeout o annullamento si accettano fino alla conferma di `Stop` o a un limite breve scritto, marcate come tardive.

## H-7 (media) Credenziale controllata per sessione contro revoca (C-11) e C-2

- **Unità A:** storia del collegamento gRPC dei worker. Controlla la credenziale nel saluto, una volta per sessione (AD-12).
- **Unità B:** storia della revoca delle credenziali (FR-30, C-11) e storia del tentativo (C-2).
- **Divergenza:** una sessione gRPC dura ore; la revoca scrive nel database ma nessuna regola dice che chiude le sessioni aperte. Il worker revocato continua a ricevere consegne e a mandare esiti (accettati per C-2, perché la sessione è quella giusta). Inoltre «sessione» di C-2 non è definita: se la connessione cade e il worker si ricollega (nuova sessione, stesso worker), l'esito dell'attività in volo viene rifiutato per «non dalla sessione a cui è stato consegnato», e l'attività viene rieseguita sempre: una caduta di rete lunga un secondo duplica il lavoro.
- **Fix:** (a) la revoca è un evento sul log; il motore tiene il registro delle sessioni per credenziale, chiude quelle della credenziale revocata e rifiuta gli esiti di quelle sessioni. (b) La sessione di C-2 è l'identità del worker (credenziale più id del worker che dichiara nel saluto), non la connessione: un worker che si ricollega entro la scadenza del tentativo può consegnare l'esito; si controlla di nuovo la credenziale al ricollegamento. (c) Test a due tenant e a revoca a metà tentativo.

## H-8 (bassa) Corsia unica di scrittura con SQLite e scritture del motore

- **Unità A:** gruppi del proiettore. Tengono il mutex per tutta la transazione di un gruppo.
- **Unità B:** scritture del motore (segreti, chiavi avvolte), che avvengono prima della risposta.
- **Divergenza:** nessun limite alla dimensione del gruppo: un gruppo grande tiene il mutex per centinaia di ms e la scrittura di un segreto (richiesta interattiva) aspetta; con Postgres non c'è mutex e due connessioni prendono i lock in ordine diverso (deadlock tra il proiettore e il repository). Le chiavi esterne «solo verso righe scritte dal motore» sono una regola di revisione, non un controllo.
- **Fix:** un tetto alla durata del gruppo (per numero di eventi e per tempo, da misurare nella prima storia del proiettore) e il mutex con una coda a due priorità (le scritture del motore passano avanti ai gruppi); un test del Cargo graph o di schema che fallisce se una chiave esterna del motore punta a una tabella della proiezione.

---

## Sintesi per priorità

1. H-1 critica, 2. H-3/H-4/H-5/H-2 alte, 3. H-6/H-7 medie, 4. H-8 bassa. Tutte si chiudono con regole di una o due righe nella spina, più un test nominato per ognuna (cancellazione con eventi in volo, annullamento contro esito, token di generazione vecchia, indice dello stato accettato, revoca a metà sessione).
