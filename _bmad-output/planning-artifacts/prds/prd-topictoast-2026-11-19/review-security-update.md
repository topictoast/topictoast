# Revisione di sicurezza dell'aggiornamento del PRD (2026-11-20)

**Verdetto:** l'aggiornamento non riapre nessuna critica e lascia alla lettera la regola confermata di
FR-26 (cliente solo dalla credenziale verificata). Tre rilievi alti vanno chiusi prima di riportare
lo status a final. SU-A1: il PRD non esclude che i segreti consegnati a un'attività di un workflow
dell'integratore li decida il manifest del worker al lancio invece del documento pubblicato, e così
FR-33 si indebolisce nei fatti. SU-A2: la
credenziale dell'integratore ora vive nel browser e il PRD descrive cosa mostra il kit, non cosa
accetta il motore. SU-A3: gli schemi scritti dai clienti sono una quarta superficie di logica del
cliente valutata nel motore fuori dalla sandbox, come lo erano jq, SQL e `call http` (C3–C5).

Conteggio: **critica 0 · alta 3 · media 9 · bassa 6**.

Base: `diff -u` tra `prd.pre-update.md` e `prd.md` (versione delle 11:42 del 2026-11-20), più
`addendum.md` (§ "Dopo la UX"). I numeri di riga si riferiscono al `prd.md` corrente. Gli ID hanno il
prefisso `SU-` per non confondersi con C/A/M/B di `review-security.md` e con T-/R- del triage. Dove un
rilievo riapre una voce rinviata della revisione precedente, lo dice.

| ID | Gravità | Dove | Titolo |
|---|---|---|---|
| SU-A1 | alta | FR-29 r. 815–817, FR-33 r. 873–878, FR-55 r. 534–536 | Il manifest del worker allarga i segreti consegnati |
| SU-A2 | alta | FR-27 r. 785–786, FR-28 r. 797–802, NFR-8 r. 1211–1213 | Credenziale dell'integratore con cliente scelto: ambito nel motore non definito |
| SU-A3 | alta | FR-55 r. 534–539, FR-56 r. 397–404, FR-4 r. 375–379, NFR-14 r. 1235–1239 | Schemi dei clienti valutati nel motore senza limiti |
| SU-M1 | media | FR-27 r. 785–788 | Stato del cliente precedente dopo un cambio di cliente |
| SU-M2 | media | FR-29 r. 815–817, §11 n. 14 r. 1407–1408 | Più segreti per la stessa connessione: scelta implicita |
| SU-M3 | media | FR-32 r. 867–868, FR-29 r. 815–817 | `call http` in un workflow dell'integratore: segreti senza regola |
| SU-M4 | media | FR-32 r. 861–862, §3 r. 271–274, FR-15 r. 560 | Elenco delle connessioni con quelle di altri clienti |
| SU-M5 | media | FR-27 r. 783–784, FR-6 r. 421–422, FR-10 r. 475–476, FR-31 r. 846–851, FR-52 r. 1146 | Nome visualizzato: attore ambiguo, log, cancellazione per persona |
| SU-M6 | media | FR-46 r. 1076 | Prima azione che cambia stato nella dashboard |
| SU-M7 | media | FR-29 r. 809–810, FR-57 r. 1035–1036, FR-58 r. 1046–1049 | Disponibilità dei workflow dell'integratore e trigger già creati |
| SU-M8 | media | NFR-7 r. 1196–1201, FR-41 r. 988–989, FR-55 r. 534–536 | Nuovo testo dei clienti fuori dal test di NFR-7 |
| SU-M9 | media | FR-27 r. 776–788 | Regole di rifiuto della credenziale mancanti |
| SU-B1 | bassa | FR-4 r. 372–379 | Errori di manifest come oracolo sulle funzioni non disponibili |
| SU-B2 | bassa | FR-4 r. 375–379, FR-40 r. 972–973 | Legame segreto–connessione controllato solo nel kit |
| SU-B3 | bassa | FR-55 r. 540–542 | Nodo del cliente che si presenta come funzione dell'integratore |
| SU-B4 | bassa | FR-45 r. 1065, UJ-5 r. 219, FR-52 r. 1146 | Credenziale del primo avvio stampata e indicata |
| SU-B5 | bassa | FR-56 r. 401–402, FR-4 r. 368–369 | Errori di validazione che ripetono il valore |
| SU-B6 | bassa | §3 r. 271–274, FR-32 r. 861–862 | La connessione non lega il segreto alle sue destinazioni |

---

## Gravità alta

### SU-A1. Il manifest del worker allarga i segreti consegnati
- **Gravità:** alta
- **Dove:** FR-29 r. 815–817; FR-33 r. 873–878; FR-55 r. 534–536; FR-12 r. 499–500; §11 n. 10 e 11 (r. 1398–1402)
- **Scenario:** con una credenziale di worker valida (rubata o da un host compromesso) si registra di
  nuovo `enrich` con un manifest che aggiunge la connessione «AWS / chiave». Le connessioni si
  risolvono «all'avvio» (r. 815) e §11 n. 10–11 lasciano aperto quale manifest vale, quindi il PRD non
  esclude che ogni cliente che lancia il workflow consegni la chiave AWS all'attività, che arriva
  anche al worker ostile (r. 505).
- **Correzione (PRD):** in FR-29 o FR-33: le connessioni che un'attività di un workflow
  dell'integratore riceve sono fissate nella versione pubblicata del workflow (FR-5), validate sul
  manifest al momento della pubblicazione; un manifest registrato dopo non aggiunge mai segreti
  consegnati, e una connessione in più richiesta dal manifest fa fallire il task con un errore, mai
  un segreto in più. Un worker che registra una funzione esistente con connessioni diverse è
  segnalato all'operatore (FR-48). Con questa invariante le domande 10, 11 e 14 del §11 possono
  restare all'architettura e alle spec.
- **Conseguenza verificabile:** dopo che un worker registra di nuovo `enrich` con una connessione in
  più, un'esecuzione della versione già pubblicata non consegna alcun segreto oltre a quelli della
  versione; il caso entra nel test di NFR-8.
- Riapre in parte B3 (il cliente non vede quale workflow dell'integratore ha usato il suo segreto):
  con la risoluzione automatica, l'ultimo uso dovrebbe indicare workflow e task.

### SU-A2. Credenziale dell'integratore con cliente scelto: ambito nel motore non definito
- **Gravità:** alta
- **Dove:** FR-27 r. 785–786; FR-28 r. 792–793 e 797–802; FR-38 r. 938; NFR-8 r. 1211–1213
- **Scenario:** il kit ora tiene nel browser una credenziale con ruolo integratore, e le r. 797–802
  dicono cosa il kit *mostra*, non cosa il motore accetta da lei. Uno script iniettato nella
  piattaforma ospite la usa sull'API per creare clienti, cambiare quote o leggere altri clienti,
  perché il ruolo integratore «vede tutti i clienti» (r. 792).
- **Correzione (PRD):** in FR-28: una credenziale dell'integratore che porta un cliente scelto vale nel
  motore solo per le risorse di quel cliente e per la lettura dei workflow dell'integratore a lui
  disponibili, più l'elenco e la ricerca dei clienti per il selettore (FR-38 r. 938). Una credenziale
  che l'ospite firma per il kit senza cliente scelto, prima della scelta, vale solo per l'elenco e la
  ricerca dei clienti. Ogni altra operazione riservata all'integratore (clienti, quote, workflow
  dell'integratore, risorse di altri clienti, operazioni dell'operatore) viene rifiutata con le
  credenziali del kit. L'elenco dei clienti è riservato all'integratore. In NFR-8 il test verticale
  copre anche queste credenziali.
- **Conseguenza verificabile:** con una credenziale (integratore, cliente A) le richieste di creare un
  cliente, cambiare una quota, pubblicare un workflow dell'integratore o leggere una risorsa di B
  vengono rifiutate; con una credenziale di cliente, l'elenco dei clienti viene rifiutato.

### SU-A3. Schemi dei clienti valutati nel motore senza limiti
- **Gravità:** alta
- **Dove:** FR-55 r. 534–539 (manifest dei nodi, FR-15 r. 556–557); FR-56 r. 397–404; FR-4 r. 375–379; FR-16 r. 578–579 e 587–589; NFR-14 r. 1235–1239
- **Scenario:** gli schemi del manifest di un nodo e dell'input di un workflow sono scritti dal cliente
  e valutati nel motore a ogni invio e avvio, senza linguaggio chiuso né limiti in NFR-14: uno schema
  profondo, ricorsivo o con regole di testo costose rallenta gli altri clienti oltre NFR-2. Uno che
  rimanda a uno schema esterno fa aprire al motore una connessione fuori da FR-16 (r. 578).
- **Correzione (PRD):** in FR-55 e FR-56: gli schemi usano un linguaggio chiuso e documentato (campi,
  tipi, campi obbligatori, elenchi, oggetti annidati) e non rimandano a nulla fuori dal manifest o dal
  documento. In NFR-14: dimensione del manifest e dello schema, profondità e numero di campi hanno un
  limite con valore predefinito. In FR-16 r. 587: il trattamento dei byte di un nodo comprende il suo
  manifest. In NFR-8: manifest e schemi ostili entrano tra gli input ostili del test, come le query di
  FR-19. Vale anche per
  i manifest dei worker, che sono input esterno (NFR-7).
- **Conseguenza verificabile:** uno schema con un riferimento esterno o oltre il limite di profondità
  viene rifiutato alla validazione con limite e valore, senza aprire connessioni; mentre un cliente
  lancia a ripetizione con lo schema più complesso consentito, le attività degli altri restano entro
  NFR-2.

---

## Gravità media

### SU-M1. Stato del cliente precedente dopo un cambio di cliente
- **Gravità:** media
- **Dove:** FR-27 r. 785–788
- **Scenario:** l'integratore lavora nel kit per il cliente A e passa a B; l'ospite firma una
  credenziale nuova. La regola «il lavoro non salvato resta e torna dopo il rinnovo» (r. 787–788) non
  distingue un rinnovo da un cambio di cliente. La bozza di A, le liste in memoria e gli eventi in
  tempo reale di A restano o finiscono salvati nel contesto di B. Lo stesso vale per un altro utente
  che entra dopo la scadenza su un dispositivo condiviso.
- **Correzione (PRD):** in FR-27: il lavoro non salvato torna solo se la credenziale nuova porta lo
  stesso cliente e lo stesso ruolo; resta solo in memoria, mai nella memoria persistente del browser.
  A un cambio di cliente il kit scarta tutto lo stato del cliente precedente: bozze, dati in memoria
  e connessioni in tempo reale.
- **Conseguenza verificabile:** dopo il passaggio da A a B nessuna bozza, lista o evento di A è
  visibile o salvabile in B.
- Riapre M6 (connessioni in tempo reale oltre la credenziale).

### SU-M2. Più segreti per la stessa connessione: scelta implicita
- **Gravità:** media
- **Dove:** FR-29 r. 815–817; §11 n. 14 r. 1407–1408
- **Scenario:** il cliente ha due chiavi per la stessa piattaforma, una di prova e una di produzione
  con permessi più ampi. Il PRD rinvia la scelta alla spec; una scelta implicita (la più recente, la
  prima) consegna a un workflow dell'integratore la chiave sbagliata, anche diversa da un lancio
  all'altro, senza che il cliente lo sappia.
- **Correzione (PRD):** in FR-29: nessuna scelta implicita. Con più segreti che soddisfano la stessa
  connessione l'avvio viene rifiutato prima di essere accettato, salvo che il cliente abbia indicato
  quale segreto usa quel workflow dell'integratore (o quale vale come predefinito per la
  connessione). La spec decide solo dove si indica.
- **Conseguenza verificabile:** con due segreti adatti e nessuna indicazione l'avvio viene rifiutato
  con la connessione ambigua nel messaggio; con l'indicazione l'attività riceve solo quel segreto.

### SU-M3. `call http` in un workflow dell'integratore: segreti senza regola
- **Gravità:** media
- **Dove:** FR-32 r. 867–868; FR-29 r. 815–817; FR-33 r. 873–878
- **Scenario:** un task senza manifest, come `call http`, non dichiara connessioni. Per il cliente il
  kit mostra «tutti i segreti del cliente» (r. 868), ma per un workflow dell'integratore, condiviso,
  il PRD non dice da dove arriva il segreto di una `call http`. Copiare la regola del kit
  consegnerebbe a una `call http` di un workflow condiviso qualunque segreto del cliente che lo lancia.
- **Correzione (PRD):** in FR-29: un task senza manifest in un workflow dell'integratore dichiara nel
  documento la connessione che gli serve, risolta con la stessa regola; senza dichiarazione non riceve
  segreti del cliente. In FR-32: «tutti i segreti» vale solo per la scelta nel kit; l'attività riceve
  solo il segreto scelto (FR-33).
- **Conseguenza verificabile:** un workflow dell'integratore con una `call http` senza connessione
  dichiarata, lanciato da un cliente con segreti, consegna l'attività senza alcun segreto.

### SU-M4. Elenco delle connessioni con quelle di altri clienti
- **Gravità:** media
- **Dove:** FR-32 r. 861–862; §3 r. 271–274; FR-15 r. 560; FR-26 r. 757
- **Scenario:** salvando un segreto il cliente sceglie la connessione che soddisfa. Se l'elenco delle
  connessioni nasce da tutti i manifest registrati, il cliente B vede le connessioni dichiarate dai
  nodi di A (per esempio «crm-interno-acme»), cioè i fornitori e i sistemi di un altro cliente. Il
  nodo è visibile solo ad A (FR-15), ma l'elenco derivato dai manifest non è una «risorsa» citata da
  FR-26.
- **Correzione (PRD):** in FR-32: le connessioni che un cliente vede e sceglie sono solo quelle dei
  manifest a lui disponibili (funzioni che l'integratore gli rende disponibili e i suoi nodi). In
  NFR-8: l'elenco delle connessioni entra nel test tra clienti.
- **Conseguenza verificabile:** dopo che A carica un nodo con una connessione nuova, l'elenco delle
  connessioni di B non la contiene.

### SU-M5. Nome visualizzato: attore ambiguo, log, cancellazione per persona
- **Gravità:** media
- **Dove:** FR-27 r. 783–784; FR-6 r. 421–422; FR-10 r. 475–476; FR-31 r. 846–851; FR-52 r. 1146
- **Scenario:** (1) «nome visualizzato o ruolo»: con il nome presente il ruolo non si registra, e il
  cliente A non distingue un lancio o un annullamento fatto dall'integratore per lui (FR-28) da uno
  dei suoi utenti; il nome è testo libero e non univoco scelto dall'ospite. (2) FR-31 elenca i
  supporti da cancellare senza i log, e FR-52 esclude dai log segreti e input/output, non le
  credenziali né i nomi: il dato personale sopravvive alla cancellazione del cliente. (3) Senza
  cancellare l'intero cliente non si toglie il nome di una sola persona che lo chiede.
- **Correzione (PRD):** origine (FR-6), annullamento (FR-10) e versione di un workflow salvata dal kit
  (FR-5, FR-28) registrano sempre il ruolo, più il nome se presente. In FR-52: i log non contengono la credenziale né il nome visualizzato. In FR-27: il
  nome segue la conservazione del record che lo contiene (FR-31), e l'integratore può togliere il
  nome di una persona dai record di un cliente.
- **Conseguenza verificabile:** dopo un lancio dell'integratore per A con nome «Mario», lo storico di A
  mostra ruolo integratore e nome; una ricerca di «Mario» nei log non trova nulla; dopo la richiesta
  di rimozione nessun record lo mostra.
- Riapre M5 (registro delle azioni con attore opaco).

### SU-M6. Prima azione che cambia stato nella dashboard
- **Gravità:** media
- **Dove:** FR-46 r. 1076; FR-45 r. 1059–1060
- **Scenario:** finora la dashboard era in sola lettura; ora annulla esecuzioni di qualunque cliente.
  Una pagina di un altro sito aperta dall'operatore con la sessione attiva invia richieste di
  annullamento a nome suo, anche in massa. La credenziale dell'operatore è unica (M5 rinviata), quindi
  lo storico del cliente non dice quale operatore.
- **Correzione (PRD):** in FR-46: un'azione della dashboard che cambia stato è accettata solo se parte
  dalla dashboard stessa e chiede conferma; lo storico del cliente registra «operatore» come ruolo.
- **Conseguenza verificabile:** una richiesta di annullamento partita da una pagina di un'altra origine,
  con la sessione dell'operatore aperta, viene rifiutata e l'esecuzione resta in corso.

### SU-M7. Disponibilità dei workflow dell'integratore e trigger già creati
- **Gravità:** media
- **Dove:** FR-29 r. 809–810; FR-28 r. 798–799; FR-57 r. 1035–1036; FR-58 r. 1046–1049; FR-4 r. 372–374
- **Scenario:** FR-29 dice che un workflow dell'integratore è «disponibile a tutti i clienti»; FR-28,
  FR-57 e FR-58 dicono «a lui disponibili», cioè per cliente. Se la disponibilità è per cliente, nessun
  FR dice chi la decide né che si ricontrolla al momento del lancio. Una pianificazione (FR-57) o una
  pipeline creata quando il workflow era disponibile continua a lanciarlo dopo il ritiro. FR-4
  ricontrolla all'esecuzione solo le funzioni.
- **Correzione (PRD):** scegliere. (a) Tutti i workflow dell'integratore sono disponibili a tutti i
  clienti: FR-28, FR-57 e FR-58 si allineano. (b) L'integratore li rende disponibili per cliente: la
  regola vale a ogni lancio, da qualunque trigger (API, pianificazione, pipeline, sotto-workflow). Una
  scadenza di un workflow ritirato non parte e la pianificazione si sospende con il motivo, visibile
  al cliente.
- **Conseguenza verificabile (b):** dopo il ritiro, la scadenza successiva non crea esecuzioni e la
  pianificazione risulta «sospesa» con il motivo.

### SU-M8. Nuovo testo dei clienti fuori dal test di NFR-7
- **Gravità:** media
- **Dove:** NFR-7 r. 1196–1201; FR-55 r. 534–536; FR-56 r. 397–398; FR-41 r. 988–989; FR-27 r. 783
- **Scenario:** l'aggiornamento porta nel kit nuovo testo scritto dai clienti: descrizione, gruppo e
  «altri metadati» del manifest di un nodo, nomi e descrizioni dei campi dello schema di input, usati
  come etichette del modulo di avvio, e nomi visualizzati. Il kit ora gira anche con la credenziale
  dell'integratore (FR-28): uno script nel testo di A eseguito in quella sessione prende una
  credenziale con poteri sull'installazione (vedi SU-A2).
- **Correzione (PRD):** in NFR-7: l'elenco del contenuto mostrato come testo nomina campi del manifest
  (metadati compresi: mai collegamenti, immagini o HTML interpretati), nomi e descrizioni degli schemi
  e nomi visualizzati. Il test gira con entrambi i ruoli del kit e nella dashboard.
- **Conseguenza verificabile:** un nodo con uno script in descrizione, gruppo e metadati, e un workflow
  con uno script nel nome di un campo dello schema, aperti nel kit con la credenziale
  dell'integratore, non eseguono alcuno script.

### SU-M9. Regole di rifiuto della credenziale mancanti
- **Gravità:** media
- **Dove:** FR-27 r. 776–788
- **Scenario:** la credenziale ha ora tre campi in più o in meno a seconda del ruolo, e il PRD non dice
  cosa si rifiuta. Una credenziale di ruolo cliente che porta anche un cliente scelto lascia ambiguo
  quale cliente vale. Un cliente scelto inesistente o eliminato può ricadere su un comportamento
  predefinito. Un nome visualizzato senza limite di lunghezza, o con caratteri di controllo, finisce
  in storico e viste.
- **Correzione (PRD):** in FR-27: una credenziale di ruolo cliente con il campo del cliente scelto
  viene rifiutata. Una credenziale con un cliente scelto inesistente o eliminato viene rifiutata
  (FR-31 r. 851). Il nome visualizzato ha una lunghezza massima e un insieme di caratteri documentati
  (NFR-14); fuori da quelli la credenziale viene rifiutata. I tre casi entrano nel test di NFR-8.
- **Conseguenza verificabile:** ognuna delle tre credenziali riceve un rifiuto e nessuna risorsa viene
  letta o creata.

---

## Gravità bassa

### SU-B1. Errori di manifest come oracolo sulle funzioni non disponibili
- **Dove:** FR-4 r. 372–379; FR-39 r. 956–957
- **Scenario:** il controllo dell'input sul manifest (r. 375) eseguito prima o insieme a quello di
  disponibilità restituisce «campo `tenant_id` obbligatorio» per una funzione riservata, e ne rivela
  esistenza e schema.
- **Correzione (PRD):** in FR-4: per una funzione non disponibile l'errore è identico a quello di una
  funzione inesistente, senza dettagli del manifest. Il nucleo JS non riceve manifest di funzioni non
  disponibili.

### SU-B2. Legame segreto–connessione controllato solo nel kit
- **Dove:** FR-4 r. 375–379; FR-40 r. 972–973; FR-32 r. 867
- **Scenario:** FR-4 rifiuta solo «connessioni senza un segreto». Un documento inviato con il nucleo JS
  lega a un task un segreto che soddisfa un'altra connessione, oppure più segreti di quelli dichiarati
  dal manifest. Restano dello stesso cliente, quindi il danno resta dentro il cliente.
- **Correzione (PRD):** in FR-4: all'invio un segreto legato a una connessione deve soddisfarla, e un
  task con manifest non dichiara segreti oltre le sue connessioni.

### SU-B3. Nodo del cliente che si presenta come funzione dell'integratore
- **Dove:** FR-55 r. 540–542
- **Scenario:** un utente del cliente carica un nodo con nome e gruppo uguali a una funzione
  dell'integratore («VirusTotal lookup») e la connessione della chiave VirusTotal. Un collega lo sceglie
  dalla palette e gli lega la chiave, che il nodo manda fuori.
- **Correzione (PRD):** in FR-55: la categoria è sempre visibile nella palette e nel pannello del task,
  e un nodo non usa il nome di una funzione disponibile al cliente.

### SU-B4. Credenziale del primo avvio stampata e indicata
- **Dove:** FR-45 r. 1065; UJ-5 r. 219; FR-52 r. 1146
- **Scenario:** la pagina d'accesso, pubblica, dice dove è stata stampata la credenziale. Se è stata
  stampata nel log, finisce nei sistemi di raccolta dei log, contro il principio IV della
  costituzione (mai token o chiavi nei log), che il PRD non riprende (vedi SU-M5).
- **Correzione (PRD):** in FR-45: la credenziale si mostra una sola volta, nella console del primo
  avvio o in un file leggibile solo dall'utente dell'installazione, mai nei log strutturati (FR-52).
  La pagina lo dice senza percorsi dell'host.

### SU-B5. Errori di validazione che ripetono il valore
- **Dove:** FR-56 r. 401–402; FR-4 r. 368–369
- **Scenario:** «con percorso e motivo» lasciato aperto diventa «valore `sk-live-…` non è un numero»
  se l'utente incolla una chiave nel campo sbagliato. L'errore resta in risposte, storico e log.
- **Correzione (PRD):** in FR-56: l'errore nomina percorso e regola, non il valore ricevuto.

### SU-B6. La connessione non lega il segreto alle sue destinazioni
- **Dove:** §3 r. 271–274; FR-32 r. 861–862; FR-16 r. 578–580
- **Scenario:** un segreto dichiarato per la piattaforma VirusTotal si può legare a una `call http`
  verso qualunque destinazione consentita al cliente. Il rischio c'era già; la connessione ora
  nomina la piattaforma e potrebbe chiuderlo.
- **Correzione (PRD, facoltativa):** l'integratore può legare una connessione alle destinazioni della
  sua piattaforma; un segreto di quella connessione viaggia solo verso di esse.

---

## Verificato, nessun rilievo

- FR-26 r. 758–761: la regola confermata resta alla lettera, perché il cliente scelto sta nella
  credenziale firmata (decisione T-79, opzione 1). L'ambito nel motore è SU-A2.
- FR-28 r. 800–802: un workflow creato con la credenziale dell'integratore appartiene al cliente, quindi
  FR-26 r. 763–766 lo risolve tra le risorse del cliente e FR-4 ne controlla la disponibilità come
  per il cliente. Chi l'ha creato: SU-M5.
- FR-32 r. 867–868: il selettore è solo una scelta nell'interfaccia; FR-33 e FR-16 restano intatti
  (l'attività riceve solo i segreti dichiarati dal task).
- FR-15 r. 560: i nodi restano visibili solo al cliente che li carica, quindi un manifest di nodo non
  ottiene segreti di altri clienti; dentro il cliente vale SU-B3.
- FR-57 r. 1032: la pianificazione è un trigger del cliente con il cliente fissato (FR-26 r. 771), quindi
  sui workflow dell'integratore gira con i segreti e le quote del cliente che l'ha creata. Il ritiro è
  SU-M7.
- NFR-14 r. 1229: il recupero delle scadenze perse ha già un limite; nessun burst illimitato dopo un
  fermo.
- FR-41 r. 990: niente identificativi dei worker con il ruolo cliente.
- FR-35 r. 910–911: sola lettura per tutti, quindi nessun percorso di modifica dal kit di un workflow
  condiviso.
- FR-44 r. 1019–1024 (marchio e contrasto) e FR-48 r. 1089–1091 (disco pieno, solo operatore): nessun
  effetto sulla sicurezza.
