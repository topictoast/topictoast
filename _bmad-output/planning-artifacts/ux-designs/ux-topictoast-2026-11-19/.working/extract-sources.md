# Estratto UX dalle fonti (solo estrazione, nessun design)

Sigle fonti: **PRD** = prds/prd-topictoast-2026-11-19/prd.md; **ADD** = .../addendum.md; **BRIEF** = briefs/brief-topictoast-2026-11-18/brief.md.
Termine vincolante (PRD §3): "Pipeline" da sola non si usa: si dice "workflow" oppure "pipeline stream" (PRD §3).

## 1. Superfici

Kit frontend = due strati: **nucleo JS** (senza interfaccia, usabile senza React, FR-39) + **componenti React** (PRD §4.9). "I clienti non devono accorgersi che sotto c'è topictoast" (PRD §4.9). Il Workflow Builder (Temporal, un solo nodo di partenza) è solo "un riferimento" (PRD §4.9).
Superfici nel kit (PRD §4.9, §8.1): editor dei workflow, vista delle esecuzioni, gestione dei segreti, editor delle pipeline stream. Più la dashboard operatore. Totale: 5 superfici UI (4 nel kit + dashboard).

### S1. Editor dei workflow (componente React, FR-40) — PRD
- Ruolo: cliente (UJ-2); incorporato nella piattaforma ospite.
- Deve permettere: "comporre un workflow come grafo: aggiungere task (funzioni dei worker, nodi personalizzati, controlli di flusso), collegarli, configurare fan-out e join, scegliere i segreti".
- Conseguenze: mostra solo funzioni, nodi personalizzati e segreti disponibili al cliente; "un grafo non valido mostra l'errore sul task che lo causa prima del salvataggio"; il salvato è documento OWS valido (sottoinsieme + estensione DAG).
- FR correlati: FR-1 (costrutti: do, set, switch, fork, for, try con retry, wait, raise, call http, funzioni/nodi personalizzati, sotto-workflow, espressioni jq); FR-2 (dipendenze arbitrarie; ciclo rifiutato "con i task coinvolti nel messaggio"); FR-3 (fan-out/join; parallelismo massimo limitabile; due politiche di errore: "fallisce tutto il fan-out" oppure "il join riceve l'errore al posto del risultato"); FR-4 (errori con percorso e motivo; riferimenti inesistenti segnalati); FR-5 (versioni; pubblicazione su versione non più ultima rifiutata con "un errore che indica la versione corrente"); FR-7 retry (numero tentativi, attesa, crescita attesa); FR-8 timeout (task ed esecuzione); FR-15 (nodi personalizzati mostrati "tra i task disponibili", codice non ospitato); FR-6 trigger (API/SDK, pianificazione cron/intervallo, riga da pipeline stream); FR-11 (limite 1 MB input/output); FR-1 errore "nomina il costrutto e il percorso nel documento".
- Accessibilità: "uso da tastiera dell'editor del grafo" (NFR-9).
- Validazione: nucleo JS dà gli stessi esiti del motore (FR-39).

### S2. Vista delle esecuzioni (componente React, FR-41) — PRD
- Ruolo: cliente (solo proprie esecuzioni, FR-26); UJ-2 "vede il DAG prendere vita".
- Deve permettere: "seguire un'esecuzione in tempo reale: stato di ogni task, attività del fan-out, tentativi, input, output ed errori".
- Conseguenze: "Un cambio di stato appare nella vista entro 1 secondo [ASSUMPTION]"; "Un fan-out da migliaia di elementi resta leggibile (attività raggruppate con conteggi per stato)".
- Altri vincoli: ogni tentativo è "un'attività distinta visibile nello storico" (FR-7); errore dell'ultimo tentativo conservato (FR-7); annullamento mostra "chi l'ha annullata e quando" (FR-10); ogni attività registra "quale versione" del nodo ha eseguito (FR-15); lo storico mostra solo esecuzioni del cliente (UJ-2); errori leggibili (nodo in loop: "il task risulta fallito con un messaggio leggibile", UJ-2); errori di limiti "nomina il limite superato" (FR-17); output oltre limite: errore "dimensione e limite" (FR-11); join oltre tetto: "dimensione e limite"; conservazione 30 giorni default (FR-31, ASSUMPTION); contenuti mostrati sempre come testo (NFR-7).
- Azioni: annullare esecuzione (FR-10 [ASSUMPTION]); lanciare con input (UJ-2: elenco di 300 indicatori; variante: 1 indicatore); rilancio ("salvato e rilanciabile").

### S3. Gestione dei segreti (componente React, FR-42, FR-32) — PRD
- Ruolo: cliente.
- Deve permettere: "aggiungere, sostituire ed eliminare i propri segreti"; salvare con un nome (FR-32).
- Conseguenze: "Il componente non mostra mai il valore di un segreto salvato"; si vedono "solo nome, data e ultimo uso" (FR-32); sostituire valore senza modificare i workflow (FR-32). Nella v1 solo chiavi API (OAuth dopo).
- Selezione segreti dentro l'editor (FR-40); un task dichiara i segreti che gli servono (FR-33).

### S4. Editor delle pipeline stream (componente React, FR-43) — PRD
- Ruolo: cliente (e integratore); UJ-3.
- Deve permettere: "scrivere, validare, attivare e sospendere le proprie pipeline stream, scegliere la destinazione e vedere stato e ultime righe emesse".
- Conseguenze: "errori di validazione della query compaiono con riga e colonna prima del salvataggio" (FR-43; FR-19: "messaggio con riga e colonna").
- Vincoli correlati: ciclo di vita attiva/sospendi/modifica/elimina (FR-23); modifica query di pipeline attiva = nuova versione con stato vuoto [ASSUMPTION]; finestra oltre conservazione stream rifiutata "con un errore che indica i due valori" (FR-18); dialetto SQL a lista chiusa, DDL/tabelle esterne/connettori/file/rete/UDF rifiutati con errore di validazione (FR-19); ciclo tra pipeline stream rifiutato al salvataggio (FR-19); stream di altro cliente rifiutato "come se lo stream non esistesse" (FR-19); destinazioni: uno stream del cliente oppure avvia un workflow del cliente (un'esecuzione per riga) (FR-24); finestre fisse e scorrevoli (FR-20 [ASSUMPTION]); ritardo tollerato dichiarato (FR-21); errori visibili "al cliente e all'operatore" per memoria di stato superata, CPU, riprese con dati usciti dalla conservazione, arretrato oltre tetto (FR-17, FR-23, FR-25, FR-30).
- Testo UI citato in UJ-3: "nuova pipeline stream"; destinazione "avvia il workflow di arricchimento per ogni riga"; "salva e attiva".

### S5. Dashboard operatore (FR-45…FR-49, §4.10) — PRD
- Ruolo: operatore (Marco in UJ-4); "interfaccia web dell'operatore, distribuita con topictoast. Mostra worker, esecuzioni e pipeline stream di tutti i clienti". BRIEF: "Mostra i worker attivi e ogni fase del loro lavoro".
- FR-45 Accesso: "credenziale generata al primo avvio; non esiste una password predefinita"; "Senza credenziali valide la dashboard non mostra alcun dato".
- FR-46 Esecuzioni: tutte le esecuzioni di tutti i clienti, "filtri per cliente, workflow e stato"; dettaglio di ogni task come FR-41 "ma senza i valori dei segreti".
- FR-47 Pipeline stream: attive con "ritardo sul watermark, eventi al secondo, memoria di stato, eventi in ritardo scartati, ultimo checkpoint"; anche metrica eventi con tempo nel futuro oltre tolleranza (FR-21).
- FR-48 Worker: "worker collegati, le funzioni che registrano, le attività in corso per ogni worker e da quanto tempo, i worker che non rispondono, e la cronologia recente delle attività di ogni worker con le fasi: ricevuta, in esecuzione, completata, fallita, ritentata". FR-14: worker non attivo risulta tale nella dashboard.
- FR-49 Quote: "uso delle quote" per cliente e "clienti al limite".
- UJ-4 (vedi §3). Avviso all'operatore se sandbox forte non attivabile (no KVM) (FR-54). Metriche e log: FR-52 (formati standard, non UI).
- Vincoli: contenuto dei clienti sempre come testo (NFR-7, test su ogni pagina della dashboard); WCAG 2.1 AA (NFR-9).

### Conseguenze trasversali che vincolano il comportamento (riepilogo)
- Stato visibile entro 1 s (FR-41); fan-out di migliaia raggruppato con conteggi per stato (FR-41); errori di validazione con percorso e motivo (FR-4), con riga e colonna per SQL (FR-19, FR-43), sul task colpevole per il grafo (FR-40); valore del segreto mai mostrato (FR-32, FR-42); "Nessun componente mostra il nome o il logo di topictoast se non lo si chiede" (FR-44); nessun secondo login nel kit (FR-27); richieste per risorse di altri clienti rispondono "come se la risorsa non esistesse" (FR-26): liste, ricerche, conteggi ed eventi in tempo reale mai con risorse di altri clienti; segreti mai in log/storico/output/errori (FR-33).

## 2. Ruoli e tenancy

- **Integratore**: sviluppatore/azienda che incorpora topictoast, "Uno per installazione" (PRD §3). Primo: Marco. Vede tutti i clienti; solo lui "crea e configura clienti e quote" (FR-28). Dichiara quali funzioni sono invocabili dai workflow dei clienti (FR-4) e quali destinazioni di rete (FR-16, FR-30). Può avere workflow propri disponibili a tutti i clienti; un cliente non può modificarli (FR-29). Attiva sandbox forte (FR-54).
- **Cliente** (tenant): "organizzazione che usa la piattaforma ospite" (PRD §3). Utente finale: Giulia (analista SOC). Vede solo sé stesso/proprie risorse. Non deve sapere che sotto c'è topictoast (PRD §2.1).
- **Operatore**: "persona dell'integratore che gestisce l'installazione dalla dashboard" (PRD §3). FR-28 distingue solo due ruoli: "l'integratore (con l'operatore), che vede tutti i clienti, e il cliente, che vede solo sé stesso".
- Identità: la piattaforma ospite autentica gli utenti e garantisce l'identità con "una credenziale firmata, a scadenza breve, che porta il cliente e il ruolo" (FR-27). Richiesta senza identità valida o scaduta rifiutata; il kit "non chiede un secondo login" (FR-27). Il cliente si ricava solo dalla credenziale, mai da campi della richiesta (FR-26).
- Il cliente non vede mai: risorse di altri clienti (FR-26), valori dei segreti (FR-32), stream/pipeline di altri (FR-19), workflow dell'integratore modificabili (FR-29). Non vede nodi personalizzati di altri (FR-15).
- Workflow dell'integratore eseguito per un cliente gira nel contesto del cliente: suoi segreti, quote, storico (FR-29).
- Esclusi: ruoli e permessi tra utenti dello stesso cliente (li gestisce la piattaforma ospite, FR-28 "Fuori perimetro").
- Dashboard operatore: autenticazione propria con credenziale generata al primo avvio (FR-45); vede dati di tutti i clienti ma mai i valori dei segreti (FR-46).
- Errori di quota/limite delle pipeline stream "visibili al cliente e all'operatore" (FR-17, FR-23, FR-25, FR-30).
- Nota: PRD §2.1 elenca 3 persone (Integratore, Cliente, Operatore); in UJ-4 Marco è "da operatore".

## 3. Percorsi UJ-1…UJ-5 (copiati verbatim da PRD §2.3)

- **UJ-1. Marco incorpora topictoast nella sua app di threat intelligence.**
  - **Persona e contesto:** Marco, sviluppatore ed esperto di sicurezza, oggi usa Hatchet con RabbitMQ bloccato alla 4.2.9 e vuole che i suoi clienti compongano da sé i workflow.
  - **Stato iniziale:** un server Linux, la sua app già esistente con il proprio sistema di login.
  - **Percorso:** installa topictoast e il bus con la configurazione predefinita e SQLite; scrive un worker con l'SDK TypeScript che registra le funzioni `dns_lookup` e `censys_search`; incorpora i componenti React nella sua app, collegandoli all'identità dei suoi clienti; definisce come codice un workflow di esempio e lo pubblica.
  - **Momento di valore:** dalla sua app lancia il workflow per un cliente di prova e vede le attività arrivare al worker e completarsi.
  - **Esito:** i suoi clienti possono comporre workflow dall'editor; lui non scrive più codice per ognuno.
  - **Caso limite:** il worker non è avviato quando parte l'esecuzione; le attività restano in attesa e partono appena il worker si registra, senza errori.

- **UJ-2. Una cliente di Marco compone, lancia e segue un workflow.** (Lo scenario "il giorno in cui funziona" del brief.)
  - **Persona e contesto:** Giulia, analista SOC presso un cliente di Marco, riceve ogni giorno elenchi di indicatori da verificare.
  - **Stato iniziale:** autenticata nell'app di Marco, apre la sezione workflow.
  - **Percorso:** nell'editor compone il workflow del caso concreto: riceve un elenco di indicatori, per ognuno verifica in parallelo DNS e Censys, arricchisce, inserisce. Collega le proprie credenziali Censys; aggiunge il nodo personalizzato che calcola un punteggio, scritto in TypeScript e caricato in precedenza da un collega sviluppatore con la CLI; salva e lancia con un elenco di 300 indicatori.
  - **Momento di valore:** vede il DAG prendere vita, con 300 rami paralleli che si completano e il join finale; i risultati sono suoi.
  - **Esito:** il workflow è salvato e rilanciabile; lo storico mostra solo le esecuzioni del suo cliente.
  - **Variante:** il giorno dopo lancia lo stesso workflow con un indicatore solo; il fan-out lo tratta come un elenco di un elemento, senza un workflow diverso.
  - **Caso limite:** il nodo personalizzato va in loop infinito; la sandbox lo ferma allo scadere del timeout, il task risulta fallito con un messaggio leggibile, gli altri clienti non se ne accorgono.

- **UJ-3. Giulia crea una pipeline stream che reagisce a un'ondata di indicatori.** `[ASSUMPTION: esempio dedotto dalla domanda sulle finestre, da confermare con un caso reale di Marco]`
  - **Stato iniziale:** gli indicatori del feed del suo cliente arrivano su uno stream.
  - **Percorso:** nell'editor apre "nuova pipeline stream", scrive una query che conta gli indicatori per ASN su finestre di 5 minuti e tiene le righe con più di 50 indicatori; come destinazione sceglie "avvia il workflow di arricchimento per ogni riga"; salva e attiva.
  - **Momento di valore:** durante un'ondata la pipeline stream emette una riga per l'ASN sospetto e il workflow di arricchimento parte da solo, una volta per ASN invece che per ogni indicatore.
  - **Esito:** la pipeline stream resta attiva; Giulia ne vede lo stato e le ultime righe emesse.
  - **Caso limite:** topictoast si riavvia durante una finestra; la pipeline stream riparte dall'ultimo checkpoint e il conteggio della finestra non si perde.

- **UJ-4. Marco, da operatore, controlla l'installazione.**
  - **Percorso:** apre la dashboard, vede i worker attivi con le funzioni registrate e le attività in corso, le pipeline stream attive con il loro ritardo, i clienti vicini alla quota.
  - **Momento di valore:** individua un worker che non risponde: le sue attività sono già state riassegnate a un altro worker.
  - **Caso limite:** un cliente supera la quota di attività concorrenti; le sue nuove attività restano in coda, gli altri clienti non rallentano.

- **UJ-5. Luca, che non conosce topictoast, lo prova.** Luca sviluppa una piattaforma di automazione, trova il repository, segue la guida rapida e in meno di 15 minuti ha topictoast in esecuzione con un worker e un workflow di esempio completato (NFR-13) `[ASSUMPTION]`.

(UJ-4 non ha persona/stato iniziale/esito/variante; UJ-5 è un paragrafo unico: così nella fonte.)
Scenario BRIEF "Il giorno in cui funziona": "l'autore incorpora la libreria nella sua app di threat intelligence. Un suo cliente apre l'editor, compone la pipeline del caso concreto, collega le proprie credenziali Censys, aggiunge un suo nodo personalizzato e la lancia; ne segue l'esecuzione e vede solo le proprie. L'autore, dalla dashboard, vede i worker al lavoro."

## 4. Stati da rappresentare

Nomi esatti usati dal PRD (in corsivo i termini; non esiste un elenco chiuso di stati nel PRD, sotto solo quelli citati).
- **Fasi dell'attività sul worker (FR-48):** ricevuta, in esecuzione, completata, fallita, ritentata.
- **Worker:** collegato; "non attivo" (FR-14: "Il worker risulta non attivo nella dashboard"); "non risponde" (FR-48, UJ-4); attività "riassegnate ad altri worker, contando un tentativo" (FR-14); concorrenza dichiarata per worker (FR-13); da quanto tempo un'attività è in corso (FR-48).
- **Esecuzione (esiti citati):** "scaduta" (oltre timeout, attività in corso annullate, FR-8); "annullata" (con chi e quando, FR-10); fallita/completata implicite (UJ-2); "in attesa"/"in coda" (FR-12, FR-30). Filtro per "stato" nella dashboard (FR-46) senza elenco valori.
- **Task:** completato; fallito ("fallisce e l'errore dell'ultimo tentativo è conservato", FR-7); "saltato" (FR-2: "dipendenza fallita o saltata... raggiunge uno stato terminale secondo una regola documentata"; regola ancora aperta, domanda aperta 8); in attesa di dipendenze (FR-2).
- **Attività:** una per task, per elemento di fan-out, per tentativo (PRD §3); "in coda" per quota; attesa prima della consegna (nessun worker, cliente oltre quota, pool sandbox vuoto) non consuma tentativi (FR-8); attesa limitata (NFR-14).
- **Fan-out:** conteggi per stato raggruppati (FR-41); elenco vuoto = nessuna attività, join riceve elenco vuoto (FR-3); elemento singolo = elenco di 1 (FR-3).
- **Pipeline stream:** attiva, sospesa (FR-23, "non consuma CPU"), eliminata; "sospesa con un errore visibile" per memoria di stato (FR-25); "rallentata o sospesa" per CPU (FR-17, FR-25); "smette di emettere segnalandolo con un errore" per arretrato oltre tetto (FR-30); errore se alla ripresa parte degli eventi è uscita dalla conservazione (FR-23); nuova versione con stato vuoto (FR-23).
- **Metriche pipeline stream (dashboard):** ritardo sul watermark, eventi al secondo, memoria di stato, eventi in ritardo scartati, ultimo checkpoint (FR-47); metrica eventi con tempo nel futuro oltre tolleranza (FR-21); righe emesse (ultime righe, FR-43); righe in attesa entro tetto arretrato (FR-24).
- **Ripresa dopo crash del motore (NFR-15):** entro 30 s riparte; "Durante la ripresa non partono nuove esecuzioni né task successivi: è una pausa, non una perdita (NFR-6)"; worker "si ricollegano da soli". Il PRD non specifica come la UI la mostra.
- **Quote raggiunte:** "vede le sue nuove attività in coda" (FR-30); oltre tetto arretrato o spazio: "errore di quota prima di essere accettata" (FR-30); eventi oltre quota "errori di quota"; dashboard: "uso delle quote e i clienti al limite" (FR-49); cliente "vicino alla quota" (UJ-4). Quote su: attività concorrenti, esecuzioni al minuto, risorse sandbox, risorse pipeline stream, eventi/s, richieste API/s, spazio (FR-30).
- **Errori di validazione:** documento (percorso e motivo, FR-4); costrutto fuori sottoinsieme (FR-1); versione `document.dsl` non 1.1.x "con un errore che indica le versioni accettate" (FR-1); ciclo DAG con task coinvolti (FR-2); riferimenti inesistenti a funzioni/nodi/segreti/task (FR-4); funzione non disponibile al cliente "come se non esistesse" (FR-4); espressione jq con `$ENV`, `env`, `input_filename`, `import`, `include` (FR-17); SQL con riga e colonna (FR-19); nodo con formato non valido "con l'errore del validatore" (FR-15); errore di politica di rete su `call http` verso `127.0.0.1`, `169.254.169.254` (FR-16); limiti sulle grandezze del cliente (NFR-14); versione corrente in conflitto (FR-5).
- **Errori di limiti sandbox:** "errore che nomina il limite superato" (FR-17); timeout nodo "messaggio leggibile" (UJ-2).
- **Retry/timeout/annullamento:** politica per task (FR-7), ogni tentativo distinto nello storico; timeout task dalla consegna (FR-8); timeout esecuzione = "scaduta"; annullamento (FR-10).
- **Stati vuoti:** il PRD non descrive schermate vuote. Casi di contenuto vuoto citati: elenco fan-out vuoto (FR-3); stream senza eventi con finestre che si chiudono comunque (FR-21). Nessun altro contenuto.
- **Sandbox:** avviso all'operatore se sandbox forte non attivabile (no KVM) (FR-54). Pool vuoto: attività attendono (FR-54).
- **Disco pieno:** "topictoast rifiuta lavoro nuovo con un errore esplicito" (NFR-6).
- **Connessione/credenziali:** richiesta con credenziale scaduta rifiutata (FR-27).

## 5. Contenuto reale per i mock

- Funzioni worker: `dns_lookup`, `censys_search` (UJ-1). Nodo personalizzato: "calcola un punteggio" (TypeScript, caricato con la CLI) (UJ-2). Fasi del flusso: "riceve un elenco di indicatori, per ognuno verifica in parallelo DNS e Censys, arricchisce, inserisce" (UJ-2; BRIEF: "arriva un indicatore, lo si verifica in parallelo su DNS e su Censys, lo si arricchisce, lo si inserisce").
- Segreto: credenziali Censys (UJ-2, BRIEF); chiavi API.
- Persone/aziende: Marco (integratore/operatore), Giulia (analista SOC), Luca (sviluppa piattaforma di automazione), "un collega sviluppatore" (carica il nodo). Piattaforma ospite: "app di threat intelligence".
- Numeri: 300 indicatori (UJ-2); 1 indicatore (variante); finestre di 5 minuti; soglia "più di 50 indicatori" per ASN (UJ-3); fan-out "migliaia di elementi" (FR-41); 100 richieste concorrenti con stessa chiave di idempotenza = 1 esecuzione (FR-6); ~680 task al giorno (SM-2/#5153); 15 minuti al primo workflow (NFR-13); limite 1 MB (FR-11); checkpoint 10 s default (FR-22); storico 30 giorni (FR-31); crash recovery 30 s (NFR-15); 1 s aggiornamento (FR-41).
- Testi UI da UJ-3: "nuova pipeline stream"; "avvia il workflow di arricchimento per ogni riga"; query: conteggio indicatori per ASN, finestre di 5 minuti, filtro > 50. Query ostili usate nei test (non per mock): `/etc/passwd`, `COPY`.
- Indirizzi nei messaggi di errore di rete: `127.0.0.1`, `169.254.169.254` (FR-16).
- Costrutti OWS (etichette di task candidate, da FR-1): do, set, switch, fork, for, try (con retry), wait, raise, call http, sotto-workflow, espressioni jq.
- Policy del fan-out: "fallisce tutto il fan-out" / "il join riceve l'errore al posto del risultato" (FR-3).
- Metadati del segreto: nome, data, ultimo uso (FR-32).
- Metriche worker: funzioni registrate, attività in corso, "da quanto tempo", cronologia con fasi (FR-48).
- Benchmark/dimensioni (se serve per mock dashboard): RAM a riposo ≤ 80 MB; p50 ≤ 5 ms, p99 ≤ 25 ms; ≥ 10.000 attività/s; ≤ 5 istruzioni SQL per attività (NFR-1…4).
- Nessun nome di cliente/ASN concreto nelle fonti (nessun "AS…" reale): non inventare.

## 6. Requisiti trasversali UX

- **i18n:** "I testi sono tutti sostituibili; la v1 include italiano e inglese [ASSUMPTION]" (FR-44).
- **Personalizzazione dell'host (FR-44):** "adattare colori, caratteri, testi e lingua dei componenti React alla propria piattaforma"; "Nessun componente mostra il nome o il logo di topictoast se non lo si chiede". Nessun accenno al theming della dashboard.
- **Accessibilità (NFR-9):** "Editor, vista delle esecuzioni, gestione dei segreti e dashboard rispettano WCAG 2.1 AA [ASSUMPTION], compreso l'uso da tastiera dell'editor del grafo".
- **Prestazioni visibili:** cambio di stato entro 1 s (FR-41); fan-out di migliaia leggibile con raggruppamento e conteggi (FR-41); dashboard dati (nessun obiettivo di tempo di caricamento nelle fonti).
- **Sicurezza che tocca la UI:** credenziale dashboard generata al primo avvio, nessuna password predefinita, nessun dato senza credenziali (FR-45; FR-50: nessuna chiave/credenziale con valore predefinito noto; accetta connessioni solo da localhost di default); valore dei segreti mai visibile (FR-32, FR-42, FR-46); contenuto dei clienti (nomi, descrizioni, errori, input, output, righe emesse) mostrato sempre come testo, mai HTML/script (NFR-7, con test su ogni pagina e componente); nessun secondo login nel kit (FR-27); connessioni cifrate, in chiaro solo da localhost (NFR-8); segreti mai in log/errori (FR-33).
- **Parità SDK:** "Tutto ciò che si fa da editor e dashboard si può fare anche tramite SDK" (FR-38).
- **Nucleo JS indipendente da React** (FR-39); componenti React: proprietà ed eventi sono contratto pubblico (PRD §6); ogni superficie pubblica ha pagine di documentazione con esempi eseguibili (PRD §6).
- **Piattaforme:** Linux x86_64/arm64 (NFR-10) [ASSUMPTION] (server, non UI). Licenza Apache-2.0 (NFR-11).
- **Primo workflow:** NFR-13 ≤ 15 min con guida rapida; "con la configurazione predefinita, un workflow di esempio gira senza modificare nulla" (FR-50).
- **Valori predefiniti:** campi omessi hanno default documentati (NFR-14); ogni tempo/limite dichiarabile ha un massimo.
- **Versioni e conflitti:** pubblicazione concorrente non sostituisce "in silenzio il lavoro di un altro" (FR-5).

## 7. Tono e posizionamento

- Utenti (PRD §2.1): cliente "Non sapere che sotto c'è topictoast"; integratore "Arrivare al primo workflow funzionante in pochi minuti, anche senza conoscere topictoast".
- "Un'interfaccia no-code da scrivere da zero" è il problema (BRIEF); i clienti "compongono da sé"/"si arrangiano nell'editor" (PRD §2.1 JTBD). Non compare la frase "facile da usare" nelle fonti: nessun contenuto letterale; il concetto è nel JTBD "senza programmare".
- Dashboard di topictoast per utenti diretti: "Non è il pubblico principale" (BRIEF); "n8n è troppo avanti per inseguirlo; questo pubblico è del possibile progetto separato [ASSUMPTION]" (PRD §2.2). Non-obiettivo: "niente catalogo di nodi pronti né connettori nella v1" (PRD §7); SM-C3: "Non si aggiungono funzioni di richiamo (nodi pronti, connettori) per guadagnare stelle: spostano topictoast verso n8n".
- Riferimenti competitor:
  - **Workflow Builder** (editor React, "gira solo su Temporal", "un solo nodo di partenza", 448 stelle): "resta un riferimento" (PRD §4.9); "Competere o integrarsi... resta da decidere" (BRIEF). Zigflow 242 stelle (SM-1).
  - **Conductor, Kestra**: componenti di visualizzazione legati al proprio motore (PRD §1.1).
  - **n8n**: utenti diretti "in stile n8n", fuori dal prodotto; "troppo avanti" (BRIEF/PRD).
  - **Hatchet**: riferimento di Marco; due strade Postgres/RabbitMQ; critica (PRD §1, BRIEF).
  - **Temporal**: pesante ("quattro servizi più un database", BRIEF); dev server ~150 MB a riposo (PRD §5).
  - **Argo**: legato a Kubernetes; "topictoast non ne ha bisogno" (BRIEF, PRD §2.2).
  - **Windmill** (AGPL, spazi separati, sandbox), **Activepieces** (MIT, parti a pagamento): separazione tra clienti (PRD §1.1, BRIEF).
  - Anche Kestra, Windmill, n8n, Temporal, Hatchet citati: ognuno avvia un'esecuzione per ogni evento (PRD §1.1).
- Qualità attese: "motore ad alte prestazioni con il minimo consumo di RAM" (PRD §1); "Velocità, poca memoria, alta scalabilità" da dimostrare con numeri; "Rust da solo non convince".
- Il kit è "un vantaggio di esecuzione, non una barriera: un kit si può copiare" (PRD §1.1, BRIEF).
- Prodotto per sviluppatori: superfici pubbliche = contratti (PRD §6).
- Futuro (non v1): nodi pronti, LLM, agenti (PRD §1, BRIEF "Visione").

## 8. Fuori dalla v1 con impatto UI

- OAuth gestito dal motore e rinnovo token (PRD §8.2; BRIEF) — la UI segreti v1 gestisce solo chiavi API. Domanda aperta 6: segreto resta "chiave con un nome" o diventa "connessione a una piattaforma esterna (tipo e piattaforma)".
- Nodi pronti, passi LLM, utenti in stile n8n, catalogo/connettori: progetto separato "flowtoast" [ASSUMPTION] (PRD §8.2, §7).
- Nodi personalizzati Python e Go; SDK Python e Go (PRD §8.2).
- Storage per payload grandi e file (RustFS): con esso "il join che riceve i risultati a pagine o per riferimento" (PRD §8.2) — in v1 limite 1 MB e tetto del join.
- Connettori degli stream verso sistemi esterni (Kafka): UI destinazioni v1 solo stream del cliente o workflow (FR-18, FR-24).
- Finestre di sessione (FR-20) dopo la v1.
- Backup/ripristino (PRD §8.2); alta disponibilità motore/bus; rotazione chiave di firma identità.
- Costrutti OWS fuori sottoinsieme: call gRPC/OpenAPI/AsyncAPI/A2A, `run` di container/script/shell, `emit`, `listen`, pianificazioni `after`/`on` (FR-1) — l'editor non li offre.
- Ruoli/permessi interni a un cliente (FR-28).
- Utenti diretti della dashboard come n8n (PRD §2.2).
- Proposta estensione DAG a OWS (issue #1143).

## 9. Voci aperte e [ASSUMPTION] che toccano la UI

Tag con testo (PRD §12 e inline):
- FR-3: "entrambe le politiche nella v1" (fallisce tutto / join riceve errore).
- FR-5: versioni dei workflow (pubblicare con esecuzioni in corso).
- FR-6: "pianificazioni nella v1" (cron/intervallo) — impatta editor trigger.
- FR-10: annullamento delle esecuzioni (azione UI).
- FR-11: "1 MB nella v1, perché lo storage per payload grandi arriva dopo".
- FR-20: "finestre fisse e scorrevoli nella v1; finestre di sessione dopo".
- FR-22: checkpoint "10 secondi" predefinito.
- FR-23: "Modificare la query di una pipeline stream attiva crea una nuova versione che parte con stato vuoto".
- FR-31: storico 30 giorni predefinito.
- FR-41: "Un cambio di stato appare nella vista entro 1 secondo".
- FR-44: "la v1 include italiano e inglese".
- FR-1: "Si accettano documenti con `document.dsl` 1.1.x; gli altri valori vengono rifiutati con un errore che indica le versioni accettate".
- NFR-9: WCAG 2.1 AA.
- UJ-3: "esempio dedotto dalla domanda sulle finestre, da confermare con un caso reale di Marco".
- UJ-5/NFR-13/SM-5: 15 minuti; tre persone esterne.
- FR-13/17/25/30: soglia di non interferenza = obiettivo di NFR-2.
- NFR-5 (100.000 eventi/s), NFR-12 (sandbox), FR-50 (processi ausiliari a freddo) [non UI].
Domande aperte (PRD §11) rilevanti per UI: 1 caso reale di pipeline stream; 6 connessioni per cliente (chiave con nome vs connessione tipo+piattaforma); 7 come un task OWS invoca funzioni/nodi e dichiara segreti nel formato (condiziona editor); 8 "Regola di propagazione di fallimenti e salti tra dipendenze DAG, e incrocio con `then` e `switch`" (condiziona stati task e editor); 2 memoria di stato attesa per pipeline stream. Forma dell'estensione OWS (metadata vs chiavi nuove) decisa dall'architettura (PRD §4.1, §10.4). Rinvii: "T-30…T-36 e circa 90 rilievi" in `triage.md` (non letto).

## Discrepanze tra fonti (rilevate)
- BRIEF: componenti React = "editor dei workflow e vista delle esecuzioni" in v1 (§Prima versione) e segreti con "OAuth o altri metodi" (§La soluzione); PRD aggiunge gestione segreti e editor pipeline stream (§1.2 punti 1-2) e v1 solo chiavi API. Vale il PRD.
- BRIEF usa "pipeline" per workflow; PRD vieta "pipeline" da sola.
- BRIEF/PRD citano Iggy come bus; ADD discute Valkey/iroh come alternative (non UI).
- FR-28 dichiara due ruoli ma §2.1 ha tre personae (integratore, cliente, operatore): l'operatore è sotto il ruolo "integratore".
- FR-54 numerato fuori sequenza (dopo FR-17); NFR-12/13/14/15 fuori ordine nella tabella: non rilevante per UI ma gli ID vanno citati come sono.
- La dashboard (FR-46) mostra esecuzioni "come nella vista delle esecuzioni (FR-41)" ma senza segreti: stessa vista in due contesti.
