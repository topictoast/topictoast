# Revisione dei casi limite del PRD topictoast

**Verdetto:** il PRD copre bene il percorso felice e alcuni casi limite dichiarati (elenco vuoto, input singolo, worker assente), ma lascia indefiniti i valori predefiniti, il momento da cui partono i timeout, la semantica delle ripetizioni (chiavi di idempotenza, eventi duplicati, ricaricamenti) e il comportamento nel tempo (pianificazioni durante un fermo, orologi sfasati, watermark fermo): sono 16 buchi di gravità alta da chiudere prima dell'architettura.

Conteggio: **alta 16 · media 14 · bassa 4**. Si riportano solo i casi che il PRD lascia indefiniti o contraddittori e che cambiano ciò che utenti o architettura devono fare.

---

## Gravità alta

### A1. FR-6 — Pianificazioni durante un fermo, sovrapposizione e fuso orario
- **Caso mancante:** cosa succede alle scadenze cron perse mentre topictoast è fermo o riavviato (recupero di tutte, recupero dell'ultima, scarto); cosa succede se la corsa precedente è ancora in esecuzione alla scadenza successiva; fuso orario e cambio dell'ora legale (ora saltata o ripetuta); pianificazione ogni 0 s o ogni 1 ms.
- **Perché conta:** "un'esecuzione per ogni scadenza" dopo 3 giorni di fermo può generare migliaia di esecuzioni e saturare la quota; la scelta decide se serve un registro delle scadenze durevole.
- **Conseguenza proposta:** la pianificazione dichiara `catch-up` (nessuno | ultima | tutte, con tetto) e `overlap` (salta | accoda | consenti), il fuso orario è esplicito (UTC se omesso), l'intervallo minimo è 1 s; test: fermo di 3 scadenze, al riavvio parte il numero previsto dalla politica.

### A2. FR-6 / FR-24 — Chiave di idempotenza: ambito, durata, conflitto, gara
- **Caso mancante:** durata di validità della chiave (oltre la conservazione di 30 giorni o dopo l'eliminazione dell'esecuzione?); stessa chiave con input diverso (restituire l'esistente o errore 409?); due richieste concorrenti con la stessa chiave; chiave su esecuzione già conclusa o annullata; lunghezza massima e caratteri. In FR-24 la "chiave derivata dalla riga": due righe identiche legittime (stesso conteggio ASN in due finestre) producono la stessa chiave?
- **Perché conta:** senza definizione la garanzia "esattamente un'esecuzione per riga" di FR-24 è falsa o scarta righe valide.
- **Conseguenza proposta:** la chiave è unica per (cliente, workflow) per almeno la durata di conservazione; stessa chiave con input diverso risponde 409; la chiave di FR-24 include identificativo pipeline, versione e finestra/offset; test con 100 richieste concorrenti con la stessa chiave = 1 esecuzione.

### A3. FR-7, FR-8, FR-12, FR-13, FR-14, FR-17, FR-22, FR-31 — Valori predefiniti non definiti
- **Caso mancante:** quale politica vale se il workflow omette retry (0 o N tentativi?), timeout di task e di esecuzione (nessuno = infinito?), tetto massimo di tentativi e di attesa, intervallo di battito del worker (FR-14 dice solo "intervallo configurato"), concorrenza di un worker che non la dichiara (FR-13), limiti CPU/memoria/tempo del nodo (FR-17), ritardo tollerato del watermark (FR-21), quote di un cliente nuovo (FR-30: zero = bloccato o illimitato?).
- **Perché conta:** le regole del progetto chiedono "default risolti dopo la validazione"; senza default, UJ-1 (attività in attesa del worker) e FR-12 possono attendere per sempre, e un cliente nuovo può sia essere bloccato sia aggirare le quote.
- **Conseguenza proposta:** tabella dei predefiniti nel PRD o nell'architettura (es. retry 0, timeout task 5 min, timeout esecuzione 24 h, battito 10 s con 3 mancati = morto, concorrenza 1, quote nuove cliente = valori di installazione) e test "workflow minimo senza campi opzionali" che esegue con i predefiniti documentati.

### A4. FR-8, FR-12, FR-30 — Da quando parte il timeout: contraddizione con la coda per quota
- **Caso mancante:** FR-12 conta l'attesa del worker dentro il timeout del task; FR-30 dice che un cliente oltre quota vede le attività "in coda" e che "superare una quota non perde lavoro accettato". Se il timeout parte alla creazione, le attività in coda per quota scadono e falliscono (lavoro perso); se parte alla consegna, FR-12 non vale.
- **Perché conta:** decide se esistono due orologi (attesa in coda e tempo di esecuzione) e cosa fa il bus per le attività in attesa.
- **Conseguenza proposta:** due timeout distinti, `queue_timeout` (attesa di worker o quota) e `run_timeout` (dalla consegna); l'attesa per quota non conta mai come errore del task; test: cliente con quota 1 e 10 attività da 1 s, tutte completano.

### A5. FR-3 / FR-11 — Fan-out: dimensione massima e join oltre 1 MB
- **Caso mancante:** (a) numero massimo di elementi: 100.000 elementi creano 100.000 attività, ognuna con proprio stato, contro NFR-4 e RAM; (b) il join riceve N risultati: 300 risultati da 10 KB sono 3 MB, oltre il limite di 1 MB di FR-11 per "input di un'attività"; (c) elemento singolo oltre 1 MB; (d) elenco `null`, oggetto o stringa al posto dell'elenco; (e) fan-out annidati (moltiplicazione N×M); (f) ordine del join quando i tentativi finiscono fuori ordine.
- **Perché conta:** il caso d'uso principale (UJ-2, 300 indicatori) si scontra con il limite se i risultati superano ~3 KB ciascuno; il tetto di N decide la struttura dello stato per esecuzione.
- **Conseguenza proposta:** `max_fanout` per installazione (es. 100.000) con errore chiaro oltre; il limite di 1 MB si applica per elemento, e il join consegna i risultati in modo paginato o con un tetto aggregato dichiarato (es. 64 MB); fan-out annidato oltre una profondità fissa rifiutato; test con N = 0, 1, 100.000 e N+1.

### A6. FR-11 — Limite di 1 MB: definizione esatta e punto di controllo
- **Caso mancante:** "oltre il limite" fallisce, ma un payload esattamente di 1.048.576 byte passa? Misurato su JSON serializzato, compresso, in byte o caratteri UTF-8? Il controllo vale anche per l'input (un output da 1 MB più i metadati, passato in input al task successivo, supera il limite?) e per i valori dei segreti inviati con l'attività; il fallimento viene ritentato (FR-7) pur essendo deterministico?
- **Perché conta:** il test al limite esatto è obbligatorio; un fallimento ritentato spreca N tentativi.
- **Conseguenza proposta:** limite = 1.048.576 byte di JSON serializzato, inclusi, per l'input, i metadati; esattamente al limite passa, +1 byte fallisce; l'errore di dimensione non è ritentabile; test su 1 MB, 1 MB + 1 byte, 0 byte.

### A7. FR-9 / NFR-6 — Contraddizione "nessun task rieseguito" contro "almeno una volta"; momento di accettazione
- **Caso mancante:** FR-9 promette che dopo un riavvio "nessun task completato viene rieseguito" e subito dopo che la consegna è "almeno una volta": un worker che completa e il motore che si ferma prima di registrare l'esito produce una riesecuzione. Inoltre non è definito cosa significhi "accettata": risposta all'avvio dopo `fsync`? Con NFR-4 (≤ 5 SQL/attività) e NFR-3 (10.000/s) serve raggruppare le scritture, e la finestra del gruppo è una perdita possibile contro NFR-6.
- **Perché conta:** è la garanzia centrale (SM-C1) e determina la politica di flush del bus e del database.
- **Conseguenza proposta:** "completato" = esito registrato durabilmente; un'attività consegnata senza esito registrato può essere riconsegnata (almeno una volta); un'esecuzione è accettata solo quando la risposta di avvio parte dopo la scrittura durevole; test di crash con kill -9 a carico di NFR-3 senza esecuzioni accettate perse.

### A8. FR-10 — Annullamento: gare con retry, completamento e attività in corso
- **Caso mancante:** annullare durante l'attesa tra due tentativi (il retry parte comunque?), mentre l'attività è in consegna a un worker, a esecuzione già conclusa (errore o no-op?), due annullamenti ripetuti; il worker che completa dopo l'annullamento (risultato scartato? effetti collaterali già avvenuti?); annullare con 100.000 attività di fan-out; annullare un sotto-workflow o un'esecuzione avviata da una pipeline stream; il worker non risponde alla richiesta di fermarsi.
- **Perché conta:** senza stato terminale univoco le esecuzioni risultano contemporaneamente annullate e completate.
- **Conseguenza proposta:** annullare è idempotente e l'esito "annullata" è terminale; un risultato arrivato dopo è registrato come ignorato; annullare un'esecuzione conclusa risponde 200 senza effetto; test: annullamento durante attesa del retry non produce altri tentativi.

### A9. FR-14 — Worker lento ma vivo: doppio risultato
- **Caso mancante:** il worker presunto morto viene riassegnato ma poi completa (partizione di rete): arrivano due risultati; quale vince? Il secondo è rifiutato? La riassegnazione "conta un tentativo": con retry = 0 (predefinito?) un worker caduto fa fallire l'esecuzione? Attività molto lunghe (ore) senza battito applicativo vengono riassegnate per errore?
- **Perché conta:** determina il protocollo worker (token per tentativo, battito per attività) e le istruzioni di idempotenza per l'integratore.
- **Conseguenza proposta:** ogni consegna ha un identificativo di tentativo; il risultato di un tentativo non più corrente è rifiutato e registrato; la riassegnazione per morte del worker non consuma i tentativi di retry del task (o ha un tetto separato); test con worker sospeso (SIGSTOP) e ripreso.

### A10. FR-2 — Dipendenze da task saltati o falliti
- **Caso mancante:** un task dipende da un task in un ramo `switch` non scelto, o da un task fallito con politica "il join riceve l'errore"; dipendenza da sé stesso; dipendenza da un task interno a un fan-out; dipendenza verso un task inesistente (FR-4 lo copre solo per funzioni e segreti). "Parte quando tutte le dipendenze sono completate" non dice cosa accade se non completeranno mai.
- **Perché conta:** senza regola l'esecuzione si blocca per sempre (nessun errore, nessun timeout se il predefinito è assente, vedi A3).
- **Conseguenza proposta:** stati `saltato` e `fallito` propagati: il dipendente è `saltato` se una dipendenza è saltata (salvo `run_if: all_done`); autodipendenza e riferimenti inesistenti rifiutati all'invio; test con `switch` e dipendente a valle.

### A11. FR-5 / FR-35 / FR-36 — Versioni, modifiche concorrenti e ripubblicazione identica
- **Caso mancante:** due utenti modificano lo stesso workflow in parallelo (l'ultimo vince e sovrascrive in silenzio?); ripubblicare un documento identico crea una nuova versione o è no-op?; versione "richiesta esplicitamente" che è stata eliminata; i workflow richiamati come sotto-workflow o come destinazione di una pipeline stream (FR-24) puntano all'ultima versione o a una fissa?; esecuzione di lunga durata che richiama un sotto-workflow aggiornato a metà.
- **Perché conta:** perdita silenziosa di lavoro dell'utente nell'editor; ripetibilità delle esecuzioni.
- **Conseguenza proposta:** pubblicare richiede la versione di base (controllo ottimistico, 409 se obsoleta); documento identico = stessa versione; riferimenti a sotto-workflow e destinazioni fissano la versione alla pubblicazione o dichiarano `latest`; test con due pubblicazioni concorrenti dalla stessa base.

### A12. FR-15 — Ricaricamento e versioni dei nodi personalizzati
- **Caso mancante:** stesso nome e stesso contenuto (no-op?), stesso nome con codice diverso mentre ci sono esecuzioni in corso e workflow che lo usano (le attività in coda usano il vecchio o il nuovo?), eliminazione di un nodo in uso, modifica dello schema di input/output incompatibile con i workflow salvati, dimensione massima del modulo, due upload concorrenti.
- **Perché conta:** FR-5 garantisce che le esecuzioni finiscano con la versione con cui sono partite, ma ciò non vale per il codice del nodo: l'architettura deve rendere i nodi immutabili e versionati.
- **Conseguenza proposta:** un nodo è identificato da (nome, hash del contenuto, versione); il workflow fissa la versione; sostituire non altera le esecuzioni in corso; eliminare un nodo in uso è rifiutato o differito; test: ricarica a metà esecuzione, le attività già create usano il codice vecchio.

### A13. FR-1 / FR-16 / NFR-7 — `call http`, espressioni jq e sotto-workflow fuori dalla sandbox
- **Caso mancante:** `call http` è nel sottoinsieme ed è eseguita dal motore, non dalla sandbox: FR-16 vieta la rete interna solo ai nodi personalizzati, quindi un cliente può far raggiungere al motore indirizzi interni (SSRF, metadati cloud, il bus). Le espressioni jq dei clienti girano nel motore senza limiti di tempo/memoria: una espressione costosa blocca tutti. Un sotto-workflow che richiama sé stesso (anche indirettamente) non è coperto dal divieto di cicli di FR-2. Mancano dimensione massima del documento, numero massimo di task, profondità di annidamento, dimensione massima della risposta HTTP.
- **Perché conta:** rompe l'isolamento tra clienti (NFR-8) e la promessa "mai un crash" (NFR-7).
- **Conseguenza proposta:** `call http` applica la stessa politica di rete di FR-16 (lista di destinazioni, indirizzi interni negati dopo la risoluzione DNS e dopo ogni redirect); le espressioni hanno limiti di tempo e memoria; la ricorsione tra sotto-workflow è rifiutata o limitata a profondità N; test con `http://169.254.169.254` e jq `[range(1e9)]`.

### A14. FR-18 / FR-20 / FR-21 — Tempo dell'evento: orologi sfasati, eventi dal futuro, stream fermo
- **Caso mancante:** un evento con tempo dell'evento nel futuro lontano (o anno 9999) fa avanzare il watermark e fa scartare tutto ciò che segue come "in ritardo"; stream senza eventi: il watermark non avanza e le finestre non si chiudono mai (l'ondata di UJ-3 termina e l'ultimo conteggio non esce); ritardo tollerato di 0 o enorme; formato del tempo malformato o negativo; più stream di una query con watermark diversi; fuso orario.
- **Perché conta:** un solo evento errato, scritto da un cliente, può spegnere la sua pipeline; l'ultima finestra non uscita invalida UJ-3.
- **Conseguenza proposta:** limite di anticipo massimo sul tempo di arrivo (oltre: scarto con contatore o tempo di arrivo), `idle_timeout` che fa avanzare il watermark con l'orologio di sistema, ritardo predefinito dichiarato; test con evento a +10 anni e stream silenzioso.

### A15. FR-18 / FR-22 / FR-23 — Eventi duplicati, durata di conservazione degli stream e punto di partenza
- **Caso mancante:** scrivere due volte lo stesso evento (ritentativo dell'SDK) lo conta due volte: nessun identificativo evento né deduplicazione in ingresso; per quanto tempo il bus conserva gli eventi (FR-22 richiede di rielaborare gli eventi dopo il checkpoint, FR-23 quelli arrivati durante la sospensione: una sospensione di 3 giorni oltre il periodo di conservazione perde eventi in silenzio); la nuova versione di una query (FR-23) parte dall'inizio dello stream, dall'ora corrente o dall'offset della precedente; dimensione massima di un evento e frequenza massima di scrittura per cliente (la quota di FR-30 non cita gli stream).
- **Perché conta:** conteggi errati e perdite silenziose; il bus è un log, la sua conservazione è una decisione di architettura e di costo disco.
- **Conseguenza proposta:** evento con `event_id` opzionale e deduplicazione per finestra di tempo; `retention` per stream con tetto di dimensione e quota di ingresso per cliente; se la pipeline riprende oltre la conservazione, errore visibile e non scarto silenzioso; la nuova versione di una query dichiara l'offset di partenza (`latest` predefinito); test di sospensione oltre la conservazione.

### A16. FR-19 / FR-25 — Sicurezza e determinismo del dialetto SQL
- **Caso mancante:** il SQL è scritto dai clienti: quali funzioni sono permesse (lettura di file, tabelle esterne, rete, funzioni di sistema come in DataFusion), funzioni non deterministiche (`now()`, `random()`) che rompono la riesecuzione dal checkpoint e la chiave di idempotenza di FR-24, query con stato illimitato (GROUP BY su chiave unica senza finestra), join tra stream senza finestra, cicli tra pipeline (una scrive sullo stream che l'altra legge: amplificazione infinita), limite sulla lunghezza e complessità della query.
- **Perché conta:** isolamento tra clienti e NFR-7; l'architettura deve fissare una lista di funzioni consentite.
- **Conseguenza proposta:** dialetto a lista bianca di funzioni (nessun I/O), funzioni non deterministiche rifiutate o legate al tempo dell'evento, query senza finestra su chiave illimitata rifiutate o con tetto di stato; cicli tra pipeline rilevati al salvataggio; test con `SELECT * FROM read_csv('/etc/passwd')`.

### A17. FR-30 / FR-24 / FR-31 — Quote modificate in corso, code senza tetto, eliminazione di un cliente attivo
- **Caso mancante:** abbassare una quota sotto l'uso corrente (le attività in corso proseguono o vengono fermate?); coda delle attività e delle righe in eccesso senza tetto massimo (FR-24, FR-6 con risposta immediata): una pipeline che emette 100.000 righe/s verso una quota di 10 esecuzioni al minuto riempie disco e RAM; "esecuzioni al minuto" oltre quota: coda o rifiuto? Eliminare un cliente con esecuzioni in corso, sandbox riservate, pipeline attive, segreti in uso, o con un'identità ancora valida (FR-27) che ricrea risorse; modifica della durata di conservazione con esecuzioni più lunghe di 30 giorni (attese lunghe).
- **Perché conta:** contraddice "superare una quota non perde lavoro accettato" se la coda è finita; elimina garanzie di isolamento.
- **Conseguenze proposta:** tetto di profondità della coda per cliente, oltre il quale l'avvio risponde 429 (avvio API) o la pipeline si sospende con errore (FR-24); l'abbassamento di quota non ferma il lavoro in corso; eliminare un cliente annulla prima le esecuzioni e poi elimina, in una sequenza dichiarata; test con 1.000.000 righe emesse e quota 1.

---

## Gravità media

### M1. FR-54 — Declassamento silenzioso della sandbox forte
- **Caso mancante:** un cliente configurato per la sandbox forte, su un host che perde KVM al riavvio o con pool azzerato, prosegue con la leggera: "con un avviso all'operatore" è un abbassamento di sicurezza senza consenso.
- **Conseguenza proposta:** la configurazione dichiara `forte: obbligatoria | preferita`; se obbligatoria e KVM manca, le attività falliscono in modo esplicito; test su host senza KVM.

### M2. FR-54 / FR-17 — Pool esaurito e avvio a freddo in rapporto al timeout
- **Caso mancante:** pool vuoto con un solo cliente che lo occupa tutto; dimensione del pool 0; sandbox riservata a un cliente che viene eliminato; sandbox "riservata" che accumula stato tra attività dello stesso cliente.
- **Conseguenza proposta:** quota di sandbox per cliente nel pool, ripulitura obbligatoria tra attività, test di equità con un cliente che satura il pool.

### M3. FR-4 / FR-32 / FR-33 — Segreto eliminato o sostituito durante un'esecuzione
- **Caso mancante:** FR-4 controlla l'esistenza del segreto all'invio, ma il segreto può sparire dopo; un tentativo successivo riceve il valore vecchio o nuovo?; nome duplicato, valore vuoto, valore oltre dimensione massima, caratteri nel nome.
- **Conseguenza proposta:** il segreto è risolto alla consegna di ogni tentativo; segreto mancante = errore non ritentabile con messaggio chiaro; dimensione massima e unicità del nome; test: eliminazione tra due tentativi.

### M4. FR-29 — Workflow dell'integratore eseguito per un cliente senza segreti o con collisione di nomi
- **Caso mancante:** il cliente non ha il segreto che il workflow condiviso richiede (FR-4 non può validarlo all'invio dell'integratore); un cliente crea un workflow con lo stesso nome di uno dell'integratore; l'integratore modifica o elimina un workflow condiviso mentre i clienti lo eseguono.
- **Conseguenza proposta:** i nomi dei workflow sono in spazi separati (cliente/integratore); la mancanza di segreto fa fallire l'esecuzione prima del primo task; test con cliente privo del segreto.

### M5. FR-27 — Credenziale firmata: scadenza, sfasamento di orologio, rotazione della chiave
- **Caso mancante:** durata e tolleranza (`nbf`/`exp` con orologi sfasati tra piattaforma ospite e topictoast), riuso della credenziale, rotazione della chiave dell'integratore senza interrompere le richieste in corso, revoca di un cliente.
- **Conseguenza proposta:** tolleranza di sfasamento dichiarata (es. 60 s), più chiavi valide contemporaneamente durante la rotazione; test con orologio a ±5 min.

### M6. FR-17 / FR-8 — Limiti di tempo del nodo contro timeout del task
- **Caso mancante:** quale vale se differiscono; limite a 0 o assente; output sul flusso standard senza tetto; il timeout del nodo include l'attesa del pool.
- **Conseguenza proposta:** vale il più stretto, un solo messaggio che nomina il limite superato; tetto massimo configurabile per installazione.

### M7. FR-16 — Lista di destinazioni di rete: risoluzione e redirect
- **Caso mancante:** nome che si risolve a indirizzo interno dopo l'approvazione (DNS rebinding), indirizzi IP letterali, jolly, lista vuota contro lista assente, redirect verso destinazioni fuori lista.
- **Conseguenza proposta:** il controllo avviene sull'indirizzo IP risolto a ogni connessione; indirizzi privati e link-local negati anche se in lista; test con nome che si risolve a 127.0.0.1.

### M8. FR-12 / FR-37 — Funzioni con lo stesso nome ma firme o versioni diverse durante un aggiornamento dei worker
- **Caso mancante:** due worker registrano la stessa funzione con schemi diversi (distribuzione graduale); una funzione sparisce dal worker con attività in corso; due worker con la stessa identità.
- **Conseguenza proposta:** registrazione con versione e hash dello schema; attività instradate solo a worker con firma compatibile; test di aggiornamento graduale.

### M9. FR-13 — Concorrenza del worker: 0, enorme, modificata a caldo
- **Caso mancante:** dichiarare 0 o 2^31; riduzione della concorrenza con attività in corso; riconnessione dello stesso worker con le attività non ancora confermate.
- **Conseguenza proposta:** intervallo valido 1–N con errore di validazione; riconnessione riprende le attività per identità; test con riduzione a caldo.

### M10. FR-51 — Aggiornamento con esecuzioni in corso e worker di versione diversa
- **Caso mancante:** il protocollo worker–motore tra versione vecchia e nuova (worker non ancora aggiornati); migrazione interrotta a metà; ritorno alla versione precedente; attività in volo durante il riavvio.
- **Conseguenza proposta:** negoziazione di versione di protocollo con almeno una versione minore di compatibilità, migrazioni transazionali con ripresa, test: aggiornamento con 1.000 esecuzioni in corso e worker vecchi collegati.

### M11. FR-24 — Destinazione eliminata o non valida
- **Caso mancante:** il workflow di destinazione viene eliminato, rinominato, o la riga non rispetta il suo schema di input; la pipeline sta emettendo.
- **Conseguenza proposta:** righe non consegnabili in una coda di scarti visibile, la pipeline continua; test con destinazione eliminata.

### M12. FR-22 / FR-25 — Errore del checkpoint e limiti dell'intervallo
- **Caso mancante:** checkpoint non scrivibile (disco pieno), intervallo 0 o molto grande, dimensione dello stato oltre il tetto durante il checkpoint, sospensione per memoria e relativa ripresa (stato mantenuto? ricostruito?).
- **Conseguenza proposta:** intervallo valido 1 s–1 h; fallimento ripetuto del checkpoint sospende la pipeline con errore visibile; test con disco pieno simulato.

### M13. FR-41 — Aggiornamenti in tempo reale con grandi fan-out e riconnessioni
- **Caso mancante:** 100.000 attività che cambiano stato a 10.000/s non devono arrivare al browser una per una; dopo una disconnessione la vista ha perso eventi.
- **Conseguenza proposta:** gli aggiornamenti del fan-out sono aggregati (conteggi per stato) con tetto di frequenza e ripristino dello stato alla riconnessione; test con fan-out da 100.000.

### M14. NFR-3 / NFR-4 / FR-31 — Definizione di "istruzione SQL" e crescita del disco a 10.000 attività/s
- **Caso mancante:** un INSERT che scrive 100 righe conta 1? Le scritture raggruppate rendono l'obiettivo aggirabile. A 10.000 attività/s con conservazione di 30 giorni lo storico supera 25 miliardi di attività: nessun requisito sul disco né sull'archivio a livello di attività.
- **Conseguenza proposta:** NFR-4 conta le istruzioni viste dal driver, e il benchmark dichiara anche byte su disco per attività; la conservazione può avere una soglia più corta per i dettagli delle attività.

---

## Gravità bassa

### B1. FR-1 — Versione `document.dsl`
- **Caso mancante:** `1.1.10`, `1.1`, `1.1.0-rc`, assenza del campo; confronto numerico contro testuale.
- **Conseguenza proposta:** accettato solo `1.1.<intero>`; test su `1.1.10` e `1.10.0`.

### B2. FR-52 — Cardinalità delle metriche per cliente
- **Caso mancante:** "code per cliente" con migliaia di clienti produce etichette illimitate.
- **Conseguenza proposta:** metriche per cliente solo oltre una soglia o aggregate in fasce (top N).

### B3. FR-20 — Dimensioni delle finestre
- **Caso mancante:** finestra di 0 s o di anni; scorrimento maggiore della dimensione; finestre giornaliere e fuso orario.
- **Conseguenza proposta:** dimensione 1 s–7 giorni (configurabile); scorrimento ≤ dimensione; test sui valori limite.

### B4. FR-45 — Accesso alla dashboard
- **Caso mancante:** tentativi ripetuti di accesso, sostituzione delle credenziali senza riavvio.
- **Conseguenza proposta:** limite di tentativi e sostituzione a caldo delle credenziali; test di blocco temporaneo.
