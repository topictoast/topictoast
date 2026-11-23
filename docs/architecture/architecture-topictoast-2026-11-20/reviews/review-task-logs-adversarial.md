# Revisione avversaria: log dei task (FR-59) nella spine

Oggetto: diff di `main` su `ARCHITECTURE-SPINE.md` (AD-17, S-1, tabella di mappa, T-67), `conventions.md` (C-9) e ADR `2026-11-23-task-logs.md`.
Lente: due unità un livello sotto che rispettano ogni AD alla lettera e costruiscono in modo incompatibile.

Verdetto: la modifica non contraddice frontalmente nessun AD, ma lascia scoperte quasi tutte le cose su cui due unità divergono (chiave dei pezzi, scrittore, ordine, tappo, regola del segnaposto), e la condizione della variante (1) di S-1 è probabilmente insoddisfacibile così come scritta.

## Contraddizioni e tensioni con altri AD

### 1. «Mai sovrascritto» in uno spazio dichiarato «sovrascrivibile», con proiezione idempotente (AD-17, AD-2)
- Dove: AD-17, «spazio con nome sovrascrivibile» + «mai sovrascritto»; AD-2 «La proiezione è idempotente».
- Problema: la porta oggi non ha una semantica «crea solo se assente». Il proiettore riprende da una posizione già applicata (NFR-15) e riscrive lo stesso pezzo. Unità A lo sovrascrive (ok per la porta, viola «mai sovrascritto»), unità B fallisce sul pezzo esistente (ferma il proiettore: per AD-2 un evento non applicabile è un guasto). Entrambe obbediscono alla lettera.
- Fix: in AD-17 aggiungere alla porta un'operazione di scrittura «crea se assente» che restituisce «già presente con lo stesso contenuto» come successo; il nome del pezzo deve essere deterministico (vedi 2), così la riapplicazione è idempotente. Dichiarare i log un terzo modo dello spazio con nome (solo accodamento), non una sottocategoria di «sovrascrivibile».

### 2. Nome, numerazione e confini dei pezzi non fissati (AD-17, C-12)
- Dove: «pezzi successivi per tentativo».
- Problema: nessuna chiave di indirizzo. Unità A: `<tenant>/<attemptId>/<n>` con `n` contatore locale dello scrittore; unità B: `n` = posizione del bus. Dopo un riavvio dello scrittore A ripartono da 0 e (con «mai sovrascritto») collidono o duplicano. Inoltre per C-12 la chiave del pezzo si deriva con HKDF «per oggetto o segmento»: se il parametro HKDF non è il nome del pezzo, due unità cifrano diversamente lo stesso pezzo e uno non decifra l'altro. Il tenant eliminato (C-13) resta coerente, ma la rotazione della chiave del tenant no.
- Fix: fissare in AD-17 il nome come `(tenantId interno, executionId, attemptId, seq)` con `seq` derivato da una posizione monotona e deterministica (preferibilmente dalla posizione del bus del primo evento del pezzo), HKDF con quel nome come `info`, e il formato del pezzo (versione, intervallo di `seq`, righe) nel dominio, uno solo, come C-12.

### 3. Il nome deve contenere l'esecuzione, altrimenti conservazione e FR-30 non lavorano (FR-31, AD-7, AD-3)
- Dove: «seguono conservazione ed eliminazione dell'esecuzione e del tenant».
- Problema: se il nome ha solo `(tenant, attemptId)`, la pulizia per conservazione non sa enumerare i pezzi di un'esecuzione. Se il database cancella la riga dell'esecuzione prima (come AD-7 ordina per il tenant), i pezzi restano orfani per sempre e il test sentinella (storia 4.8, che dice «log dei task compresi») passa solo per il tenant (chiave cancellata) ma non per la scadenza di FR-31 (la chiave del tenant esiste ancora, il testo è leggibile). Inoltre il conteggio nello spazio del tenant (FR-30): chi somma, il database (proiezione) o l'archivio (listing)? Due scelte, due numeri.
- Fix: in AD-17 fissare prefisso `tenant/execution/attempt`, ordine di cancellazione per scadenza (archivio prima, poi riga del database, ripetibile) e dire che l'uso di spazio dei log si proietta dagli eventi di scrittura del pezzo (dimensione nel riferimento tipizzato come per i payload, AD-3), non da un listing.

### 4. Variante (1) di S-1: le righe «evitano il bus» ma i worker diretti non hanno chiavi (S-1, C-13, AD-7)
- Dove: criterio nuovo in S-1; «Condizioni di sicurezza: nessuna chiave dei tenant né segreto ai worker».
- Problema: C-13 vuole eventi dei tenant cifrati con la chiave del tenant su bus. Un worker diretto non ha la chiave, quindi o (a) manda le righe in chiaro sul bus (viola C-13/AD-7, e il segnaposto applicato solo all'origine è l'unica barriera), o (b) le manda al motore, e allora la variante (1) non è più «diretta» per le righe: il criterio nuovo chiede un'impossibilità. Il testo lascia a S-1 la scelta senza dire che (a) è esclusa.
- Fix: in S-1 stabilire ora che le righe di log passano sempre dal motore (o da un canale cifrato dal motore), in ogni variante, e che «righe in chiaro sul bus» esclude la variante come le altre condizioni di sicurezza. Poi il criterio nuovo diventa solo un costo da misurare.

### 5. Il motore non è l'unico scrittore di eventi di stato: le righe sono eventi? (AD-2, C-2)
- Dove: AD-2 «gli eventi di stato li scrive solo il motore, i messaggi dei worker sono richieste che il motore giudica»; C-2.
- Problema: le righe di log sono state? Unità A le pubblica sul bus come eventi `LogLine` scritti dal motore (carico sul topic di stato, che per AD-2 non si scarta finché non applicato: un tetto di righe per tentativo diventa un tetto del bus). Unità B le tiene fuori dal bus (memoria del motore, poi archivio direttamente): nessun'altra unità può rileggerle in ripresa e un crash perde più dell'«ultimo pezzo». Entrambe rispettano AD-2. Inoltre C-2: righe da un tentativo non più corrente o arrivate dopo il primo esito registrato, si accettano? Nulla lo dice.
- Fix: decidere in AD-17/AD-2 che le righe sono eventi durevoli sul bus su un topic proprio (non di stato, non bloccante per la proiezione) scritti dal motore dopo il giudizio di C-2, o dichiarare che non lo sono e quale perdita è accettata. Estendere C-2: righe accettate solo dalla sessione corrente e fino al primo esito; dopo, rifiutate e registrate.

## Divergenze non coperte (nessuna riga in Deferred / Open Questions)

### 6. Lettura: API del log contro tempo reale (AD-18, C-8, C-16)
- Problema: AD-18 dice che gli eventi SSE escono dopo la scrittura sul bus e mai dalla proiezione, e che il bus scarta le posizioni applicate (AD-3). Le righe in corso stanno solo nel pezzo non ancora scritto. Unità A (API) legge i pezzi chiusi dall'archivio e basta (lag fino a un pezzo, viola 1 s di FR-59); unità B (SSE) emette le righe live ma con `id` = posizione del bus. Un client che prende la pagina dell'archivio e poi apre SSE da `Last-Event-ID` non ha un punto di giunzione: o perde righe (il bus le ha scartate, errore C-7 «ricarica») o le duplica. Gli eventi del motore (nel database, proiettati) e le righe (nell'archivio) hanno due sorgenti e due paginazioni.
- Fix: nuova voce o estensione di AD-18: un solo endpoint di lettura del log per tentativo che restituisce pezzi più coda e il token `position` della coda (C-16), riprendibile da `Last-Event-ID`; ogni riga e ogni evento hanno la stessa chiave d'ordine (posizione del bus del motore); la riga porta `seq` per deduplicare. Aggiungere all'Open Question «Ambito della verità sul bus» la lettura del log di un'esecuzione viva.

### 7. Ordine e orologio, e dove stanno gli eventi del motore
- Problema: FR-59 delega l'ordine alla spec, ma la spine deve fissare almeno la chiave di giunzione tra due luoghi. Unità A tiene gli eventi del motore nella proiezione (tabella tipizzata), unità B li scrive come righe nei pezzi. La dashboard (FR-46) deve fondere due sorgenti con orologi diversi (ora della riga del worker contro posizione del bus). Con la variante (1) la riga potrebbe non avere posizione affatto.
- Fix: in AD-17 o in una convenzione: ordine = posizione assegnata dal motore all'arrivo; l'ora del worker è solo un dato mostrato; gli eventi del motore sono nello stesso flusso dei pezzi (una sola sorgente) oppure la giunzione è esplicita.

### 8. Tetto per tentativo (FR-59, NFR-14)
- Problema: non c'è alcun AD. Chi conta e scarta? L'SDK (due implementazioni, TS e Rust, più l'host della sandbox) o il motore? Se tutti tre contano, il «log dice quante» si somma o si ripete; se solo i terzi (SDK scritto dalla comunità) lo fanno, il tetto non protegge il bus (S-1 lo cita come criterio di prova, ma senza dire dove si applica).
- Fix: applicare il tetto nel motore, autorevole; SDK e sandbox possono tagliare prima come ottimizzazione. Il contatore degli scarti è un campo tipizzato del tentativo e finisce nel pezzo di chiusura.

### 9. Chiusura del log e perdita dell'ultimo pezzo
- Problema: «un crash perde al massimo l'ultimo pezzo» contraddice FR-59 («righe visibili entro 1 s»): dimensione del pezzo contro latenza non è un parametro della spec ma una condizione di correttezza. Nessun segnale di fine log: un lettore non distingue un tentativo concluso da uno in cui manca un pezzo. Retry sullo stesso `attemptId`? Con C-2 il numero cambia, ma un pezzo scritto da un worker zombie dopo il timeout?
- Fix: pezzo di chiusura con `seq` finale e conteggio, scritto dal motore all'esito; flush a tempo (< 1 s) oltre che a dimensione; i dati cadono in S-1 come misura.

### 10. Regola del segnaposto: tre o quattro implementazioni (C-12, T-67)
- Dove: ADR «condividono la stessa regola: la sua forma la fissa la spec».
- Problema: ci sono SDK TS, SDK Rust, host della sandbox, motore. Divergenze costruibili restando conformi: segreti sovrapposti (uno è sottostringa dell'altro, ordine di sostituzione), segreti corti (un solo carattere cancella tutto il log), maiuscole, valore spezzato tra due righe, segnaposto con forma ricercabile come segreto (il ricontrollo del motore lo ri-sostituisce, non è idempotente), cosa fa il motore se trova un segreto che l'origine ha lasciato (sostituisce in silenzio, scarta la riga, fallisce il tentativo, avvisa l'operatore?). Un SDK TS non può chiamare il dominio Rust se non via wasm (AD-11).
- Fix: in C-12 (o nuova convenzione) fissare: una funzione del dominio, usata dal motore e dall'SDK Rust, esposta a TS con lo stesso binario napi-rs/wasm (AD-11) e dalla sandbox host; ordine = valore più lungo per primo; segnaposto fisso e non riconoscibile come segreto; soglia minima di lunghezza (sotto, la consegna del segreto è rifiutata o la riga scartata); azione del motore sul ritrovamento = sostituire e contare l'anomalia; test di parità fra le implementazioni. Portare «un valore spezzato su più righe» da T-67 a una decisione, non lasciarlo.

### 11. Ricontrollo nel motore contro AD-6 e contro l'ambiente
- Problema: AD-6 dice che il motore non interpreta contenuto dei tenant; una ricerca di sottostringhe non lo interpreta, ma per fare il ricontrollo il motore deve tenere in memoria per ogni tentativo vivo i valori dei segreti consegnati. AD-6/C-12 ammettono solo «il segreto della singola chiamata» passato all'ausiliario, e in memoria con tipi che si azzerano: va detto. Per i worker diretti (variante 1) ci sono i segreti ai worker (condizione di esclusione). Il passaggio «dove la riga passa da lui» è anche indefinito per la sandbox: la riga passa dall'IPC del motore o scrive direttamente l'ausiliario (che non ha chiavi)?
- Fix: dire che le righe della sandbox passano sempre dall'IPC del motore (le risposte degli ausiliari sono già input esterno con tetti, AD-6); il motore tiene i valori in tipi azzerabili per tentativo e li scarta all'esito.

### 12. Chi cifra e scrive i pezzi con i processi ausiliari (AD-6, AD-7)
- Problema: «chi scrive i pezzi, proiettore o motore, dipende da S-1». Ma il motore o il proiettore devono avere la chiave del tenant; la sandbox e i worker no. Se lo scrittore è il proiettore, con l'RLS (Open Question esistente) e il «solo quando ha qualcosa da applicare» (SM-2), il proiettore diventa scrittore di un flusso a latenza di 1 s (limite di AD-2: indietro al massimo 1 s) che è per sua natura più lento; FR-59 chiede 1 s da un capo all'altro. Se è il motore, scrive l'archivio in modo sincrono nel percorso caldo: nuovo vincolo NFR-2, non menzionato.
- Fix: scegliere ora: il motore accumula e scrive i pezzi in asincrono dal suo buffer (la coda viva si legge dal buffer, vedi 6); il proiettore non entra. Evita di legare il formato alla scelta del bus.

## Minori

13. Lo ADR è già «accettata» ma lascia aperti sia lo scrittore sia la forma del segnaposto: o lo stato è `proposta` finché S-1 non chiude, o le due cose vanno fuori dall'ADR come Open Question con scadenza.
14. T-67 è stato riscritto in modo da mescolare «chiuso» e «restano dettagli»: la riga dell'elenco dei differiti non ha più un trigger netto. Separare in una riga chiusa e una riga residua con scadenza.
15. La riga «Log dei task» è stata tolta dalle Open Questions, ma tre voci nuove (formato e nome del pezzo, tetto, lettura) non sono state aggiunte. Aggiungerle con proprietario architettura e scadenza «S-1».
16. AD-5: la porta dell'archivio ha il filtro di tenant per costruzione dell'indirizzo; con il prefisso proposto in 3 serve un test a due tenant per ogni metodo nuovo (lista dei pezzi, crea-se-assente, cancella per prefisso), come la costituzione V richiede per l'adattatore del repository.
17. Motivo di fallimento e messaggi di errore del tentativo (non righe) possono contenere un segreto e vanno nel database via proiezione (FR-52, NFR-4): il segnaposto di FR-59 non li copre. Stessa funzione di 10 sui campi `reason` degli eventi.

## Sintesi delle modifiche proposte alla spine

1. AD-17: terzo modo «solo accodamento» con scrittura crea-se-assente; nome `tenant/execution/attempt/seq`, HKDF con il nome; pezzo di chiusura; cancellazione per prefisso.
2. AD-2/C-2: dire se le righe sono eventi del bus e chi li scrive (motore, dopo il giudizio di C-2); regola per righe dopo l'esito.
3. AD-18/C-16: un endpoint di lettura (pezzi + coda) con token `position`; chiave d'ordine comune.
4. C-12: una regola e una funzione del segnaposto (lunghezza minima, ordine, non idempotenza evitata, azione al ritrovamento), distribuita via AD-11.
5. S-1: righe sempre dal motore in ogni variante, righe in chiaro sul bus come causa di esclusione.
6. Tetto per tentativo applicato dal motore; flush a tempo come condizione di correttezza (1 s).
