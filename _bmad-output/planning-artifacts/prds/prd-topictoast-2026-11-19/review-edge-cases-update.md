# Revisione dei casi limite: aggiornamento del PRD del 2026-11-20

**Ambito:** solo i requisiti cambiati oggi (diff tra `prd.pre-update.md` e `prd.md`): FR-55, FR-56,
FR-57, FR-58, NFR-16, SM-6, §3 Stati, FR-3/4/5/6/10/12/14/15/27/28/29/32/35/37/40/41/43/44/45/46/48/49/54,
NFR-9, §11 voci 10–15. Righe citate: `prd.md` attuale.

**Filtro:** si riporta un caso solo se nasce da una frase aggiunta o cambiata oggi, se il PRD non dà
risposta e se la risposta cambia il comportamento o l'accettazione. I casi già rinviati in
`triage.md` (T-15, T-16, T-32, T-34, T-35, temi 1–8), nelle voci 10–15 del §11 o risolti da
`EXPERIENCE.md` (EX) sono nella tabella di copertura in fondo, non tra i rilievi. Gli id usano il
prefisso `EU-`, per non confondersi con A/M/B della revisione del 19 novembre.

Conteggio: **alta 4 · media 11 · bassa 11**.

---

## Gravità alta

### EU-A1 · alta · FR-27 L787–788 (anche FR-28 L797–802, FR-26 L758–761)
- **Scenario:** la credenziale scade a metà di una modifica e il lavoro non salvato «resta e torna
  dopo il rinnovo». Ma la credenziale rinnovata può portare un altro cliente: l'integratore ha
  cambiato cliente in un'altra scheda dell'app ospite, oppure l'app rinnova con il cliente
  predefinito. Grafo, SQL e JSON di A (riferimenti ai segreti, valori fissi, nodi personalizzati)
  ricompaiono nel contesto di B e, con «Salva», diventano un workflow di B. Con il ruolo cliente su
  un computer condiviso, la sessione scade e entra un utente di un altro cliente: la stessa pagina
  gli mostra il lavoro dell'altro. EX L266 copre solo il cambio esplicito con il selettore (dialogo
  di conferma). EX L190 e L381–383 ripetono «le modifiche non salvate sono conservate» senza
  condizioni.
- **Il PRD dovrebbe dire:** «Il lavoro non salvato torna solo se la credenziale rinnovata porta lo
  stesso cliente e lo stesso ruolo; altrimenti si scarta senza mostrarlo. Il caso entra nel test di
  NFR-8.» Se l'app ospite firma anche un identificativo dell'utente, va confrontato anche quello.
  Quel campo della credenziale lo decide l'architettura (addendum L96–98).

### EU-A2 · alta · FR-57 L1026–1037, FR-6 L423–425, FR-29 L815–817, FR-56 L401–402, FR-30 L833–836, NFR-6 L1194–1195
- **Scenario:** una scadenza scatta e l'avvio viene rifiutato prima di essere accettato. Le cause
  sono quattro:
  - la connessione di un workflow dell'integratore è senza segreto (FR-29, nuovo; FR-57 permette di
    pianificare i workflow dell'integratore);
  - l'input non è conforme allo schema (FR-56, nuovo);
  - il cliente è oltre il tetto dell'arretrato o la quota di spazio (FR-30);
  - il disco è pieno (NFR-6).

  Nessun chiamante riceve l'errore. FR-6 L425 rende visibili solo le scadenze perse durante un
  fermo. §11.12 dice quale input riceve l'esecuzione, non cosa succede alla scadenza rifiutata.
  §3 L298 dà alla pianificazione solo lo stato «sospesa». EX L180 ha solo il testo «Scadenze perse
  durante una pausa»: ripete la lacuna.
- **Il PRD dovrebbe dire:** «Una scadenza che non diventa un'esecuzione accettata è registrata
  sulla pianificazione con il motivo, visibile al cliente nel kit e all'operatore; mai in
  silenzio.» Alla spec della storia restano due scelte: se la scadenza si ritenta e se dopo N
  rifiuti consecutivi la pianificazione si sospende da sola, con la causa. Estende T-32, che copre
  solo i fermi. È alta perché l'architettura deve dare alla pianificazione un registro delle
  scadenze. Dove vive la pianificazione è già una domanda aperta (addendum L105–107).

### EU-A3 · alta · §3 L286–287 (anche glossario L243, FR-46 L1069–1071, UJ-4 L197–198)
- **Scenario:** «in coda» vuol dire «nessuna attività consegnata». Ma le attività esistono solo
  per worker e sandbox (L243). I task che il motore esegue da sé, come `wait`, `set`, `switch` e
  `raise`, non ne creano. Letta alla lettera, la regola vale anche per `call http`, se la esegue il
  motore o un processo ausiliario (FR-50 L1110–1112).
  - Un workflow che comincia con un `wait` di un'ora resta «in coda» per un'ora. L'operatore lo
    vede tra le esecuzioni «ferme in coda» (UJ-4), ma il suo tempo in coda di FR-46 (l'età
    dell'attività più vecchia in coda) non è definito, quindi l'ordinamento non lo è.
  - Un workflow fatto solo di `set` e `switch` passa da «in coda» a «completata» senza essere
    mai stato «in esecuzione».

  EX L291–292 ripete la definizione del PRD senza risolvere il caso.
- **Il PRD dovrebbe dire:** «Un'esecuzione è in coda quando è accettata e nessun suo task è ancora
  partito; un task eseguito dal motore parte appena le sue dipendenze sono finite.» Va detto anche
  come FR-46 calcola il tempo in coda di un'esecuzione senza attività in coda. È alta perché è un
  elenco chiuso: la macchina a stati dell'architettura e gli scenari BDD di FR-46 e UJ-4 dipendono
  da questa frase.

### EU-A4 · alta · FR-4 L375–379, FR-40 L968–976, FR-55 L534–536, FR-3 L359–360
- **Scenario:** tipi e campi obbligatori si controllano solo all'invio, ma diversi valori si
  conoscono solo all'esecuzione:
  - un campo che prende il valore da jq (FR-40 L970);
  - un campo facoltativo dell'input del workflow, o dell'output a monte, collegato a un campo
    obbligatorio;
  - gli elementi di un fan-out;
  - l'output di un worker (codice dell'integratore) o di un nodo personalizzato (codice ostile del
    cliente) non conforme allo schema di output del manifest, che arriva ai task a valle.

  NFR-7 L1196–1198 chiede di validare l'input esterno e vieta il crash. Non dice quale task
  fallisce, in quale stato, né se si ritenta.
- **Il PRD dovrebbe dire:** «All'esecuzione il motore controlla l'input di ogni attività sullo
  schema di input e l'output sullo schema di output del manifest. Un input non conforme fa fallire
  il task prima della consegna, senza consumare tentativi. Un output non conforme fa fallire
  l'attività con percorso e motivo e non arriva ai task a valle.» Se l'output non conforme si
  ritenta lo decide la spec della storia. È alta perché la validazione di ogni attività pesa su
  NFR-2 e NFR-3 (10.000 attività/s): l'architettura e il benchmark devono contarla.

---

## Gravità media

### EU-M1 · media · FR-6 L423 (anche NFR-15 L1243–1247, NFR-14 L1229)
- **Scenario:** una scadenza scatta e il motore cade dopo aver creato l'esecuzione ma prima di
  registrare la scadenza. Al riavvio, o con il recupero di NFR-14, la stessa scadenza parte di
  nuovo: due esecuzioni. FR-24 L724–726 lo esclude per le righe delle pipeline stream; FR-6 non lo
  dice per le pianificazioni, ora che non è più un'assunzione.
- **Il PRD dovrebbe dire:** «Ogni scadenza avvia al più un'esecuzione, anche se il motore si ferma
  o riparte mentre scatta; il recupero non ripete una scadenza già avviata.» Estende T-32. Il
  meccanismo (una chiave per pianificazione e scadenza) spetta all'architettura.

### EU-M2 · media · FR-6 L424, FR-57 L1028–1029, NFR-14 L1229
- **Scenario:** una pianificazione ogni 5 minuti resta sospesa una settimana. Alla ripresa, le
  scadenze della settimana contano come «perse» e la regola di recupero di NFR-14 le rilancia? EX
  L261 dice che «Sospendi» ferma le scadenze, non cosa fa «Riprendi».
- **Il PRD dovrebbe dire:** «Le scadenze cadute durante una sospensione non si recuperano: la
  prossima scadenza si calcola dalla ripresa. Il recupero di NFR-14 vale solo per i fermi del
  motore.»

### EU-M3 · media · §3 L294, FR-48 L1085–1087, FR-14 L523–530
- **Scenario:** un worker si chiude in modo ordinato durante un deploy (SIGTERM) con attività in
  corso. L'elenco chiuso ha solo «collegato» e «non risponde». Restano aperte tre domande:
  - il worker sparisce dalla dashboard, e con lui la sua cronologia recente?
  - le sue attività tornano subito in coda o aspettano l'intervallo di battito?
  - contano un tentativo? Con un retry predefinito basso, ogni deploy a rotazione farebbe fallire
    i task in volo.
- **Il PRD dovrebbe dire:** lo stato di un worker scollegato in modo ordinato. Il resto va alla
  spec di FR-14, per esempio «un worker che si chiude in modo ordinato restituisce le sue attività,
  che tornano in coda senza consumare tentativi». Estende T-35.

### EU-M4 · media · FR-27 L787–788, NFR-16 L1248–1250
- **Scenario:** «Il lavoro non salvato resta e torna dopo il rinnovo» regge solo se il rinnovo
  avviene nella stessa pagina. Due casi lo rompono:
  - l'app ospite rinnova con un redirect al login (SSO) e la pagina si ricarica;
  - il browser del telefono chiude la scheda in background.

  In entrambi la memoria è persa (EX L382: «restano in memoria»). Conservarlo oltre il
  ricaricamento vuol dire tenere dati del cliente nel browser, anche su dispositivi condivisi.
- **Il PRD dovrebbe dire:** quale perimetro vale. Le due opzioni sono «torna dopo un rinnovo nella
  stessa pagina; con un ricaricamento si perde» oppure «si conserva nel browser, legato a cliente e
  ruolo, e si cancella al cambio». È una decisione di prodotto di Marco.

### EU-M5 · media · FR-41 L988–989, FR-6 L426–429, NFR-16 L1249 (touch), FR-5 L391–393
- **Scenario:** «Esegui» viene ripetuto: doppio tocco sul telefono, nuovo tentativo dopo un
  errore di rete con la risposta persa, oppure dopo la scadenza della credenziale. Il risultato
  sono due esecuzioni: FR-6 ha la chiave di idempotenza, ma il PRD non dice che il kit la usa. Lo
  stesso vale per «Salva»: ripetuto dopo una risposta persa, si scontra con il proprio salvataggio
  e mostra «la versione non è più l'ultima» (T-16 rinvia solo la ripubblicazione identica).
- **Il PRD dovrebbe dire:** «Un lancio dal kit porta una chiave di idempotenza per ogni intenzione
  di lancio: ripeterlo restituisce la stessa esecuzione.» Per «Salva», estende T-16. Nomi e API
  spettano all'architettura (contratto del kit).

### EU-M6 · media · FR-55 L534–536, FR-56 L397–398, FR-40 L968–971, FR-41 L988–989, FR-3 L359–360
- **Scenario:** il dialetto degli schemi non è definito. Restano aperti:
  - i tipi: intero o decimale (gli scenari BDD distinguono `3` da `3.0`), oggetti, elenchi di
    oggetti, enumerazioni, unioni;
  - il valore predefinito di un campo facoltativo e quando si applica;
  - i campi in più: rifiutati o ignorati;
  - la differenza tra `null` e un campo assente;
  - la regola di «campo compatibile» (L969): da intero a decimale, da facoltativo a obbligatorio,
    da valore singolo a elenco;
  - il tipo che riceve il join con la politica «il join riceve l'errore al posto del risultato»,
    rispetto al manifest della funzione di join;
  - cosa fa il modulo di FR-41 con uno schema che non sa disegnare.

  §11.7 copre solo dove lo schema si scrive nel documento.
- **Il PRD dovrebbe dire:** «Manifest e input del workflow usano lo stesso dialetto di schema e
  un'unica regola di compatibilità, comune a motore, nucleo JS ed editor (FR-39).» Il dettaglio va
  rinviato all'architettura, perché il formato del manifest è una superficie pubblica (§6
  L1257–1258).

### EU-M7 · media · FR-4 L375–377, FR-56, §11.11 L1401–1402
- **Scenario:** il workflow chiamato come sotto-workflow pubblica una versione con un campo
  obbligatorio nuovo o un tipo cambiato. Il chiamante salvato prima non è più valido. Il PRD non
  dice se se ne accorge all'avvio o all'avvio del figlio, a metà esecuzione, dopo gli effetti dei
  task a monte. §11.11 copre solo i manifest di funzioni e nodi. La risposta dipende dalla versione
  del sotto-workflow che si chiama (T-16, spec della storia).
- **Il PRD dovrebbe dire:** estendere §11.11 allo «schema di input di un sotto-workflow chiamato».

### EU-M8 · media · FR-4 L377–379, FR-29 L815–817, FR-32 L861–869
- **Scenario:** la connessione si controlla in modo diverso per i due tipi di workflow.
  - Un workflow del cliente controlla la connessione solo all'invio. Se poi il segreto scelto viene
    eliminato, o la sua connessione cambia, il PRD non dice se l'avvio è rifiutato prima
    dell'accettazione, come per un workflow dell'integratore, o se il task fallisce alla consegna.
    EX L186 risponde nel secondo modo («Le esecuzioni che lo usano falliranno»), opposto a FR-29:
    il PRD dovrebbe sceglierne uno.
  - Un sotto-workflow dell'integratore chiamato da un workflow del cliente: le sue connessioni si
    risolvono all'avvio del padre o del figlio, dopo gli effetti dei task a monte?
  - FR-32 non dice se la connessione di un segreto si può cambiare dopo il salvataggio; L869 parla
    solo del valore.
- **Il PRD dovrebbe dire:** «Prima di accettare un avvio, il motore risolve tutte le connessioni
  dell'esecuzione, compresi i sotto-workflow dell'integratore; un segreto mancante rifiuta l'avvio
  con la connessione nel messaggio, per ogni workflow.» Va detto anche se la connessione di un
  segreto è modificabile. Il segreto eliminato durante un'esecuzione resta al tema 2 di
  `triage.md`.

### EU-M9 · media · FR-58 L1039–1050 (anche FR-38 L934–938)
- **Scenario:** il PRD non dice da nessuna parte se un workflow si elimina o si archivia. FR-58
  elenca solo «apre» e «crea». Se non si può, i clienti accumulano workflow senza via d'uscita. Se
  si può, servono risposte su quattro punti:
  - la pianificazione di FR-57;
  - le esecuzioni in corso (FR-5);
  - i workflow che lo chiamano come sotto-workflow;
  - le pipeline stream che lo hanno come destinazione (tema 5, M11 del 19 novembre).

  EX L321 dà per scontato che esista («già eliminata» → «Workflow non trovato.»). Nessun `triage.md`
  ne ha la responsabilità: PRD T-16 L409 parla solo dell'eliminazione di un nodo.
- **Il PRD dovrebbe dire:** se l'eliminazione è nella v1. Se è fuori, va nel §8.2. Se è dentro,
  servono le conseguenze sui quattro punti. È una decisione di prodotto.

### EU-M10 · media · FR-32 L861–862, FR-55 L535, FR-40 L972–973
- **Scenario:** «Aggiungi segreto» chiede la connessione (EX L50), ma il PRD non dice da dove
  viene l'elenco. Le possibilità sono l'unione delle connessioni dichiarate dai manifest
  disponibili al cliente o un testo libero. Mancano anche la regola di corrispondenza (maiuscole,
  versioni del tipo di credenziale) e cosa succede se un manifest rinomina la piattaforma: tutti i
  segreti smettono di soddisfarla. Infine, un manifest può chiedere due connessioni della stessa
  piattaforma (due account Censys). Allora ogni connessione ha bisogno di un nome proprio perché
  «ogni connessione ha un segreto» (L972) sia verificabile. §11.14 copre solo più segreti per una
  connessione.
- **Il PRD dovrebbe dire:** che l'elenco delle connessioni viene dai manifest disponibili al
  cliente. Il resto va all'architettura (formato del manifest; §11.6 copre solo come si salvano).

### EU-M11 · media · FR-55 L539, FR-15 L561–562, NFR-14 L1235–1239
- **Scenario:** il manifest di un nodo personalizzato lo scrive il cliente. Uno schema enorme,
  profondo o con riferimenti ricorsivi può bloccare la generazione del modulo nell'editor, nel
  browser di ogni utente di quel cliente, e il validatore. L'elenco di NFR-14 delle grandezze
  controllate dal cliente non nomina manifest e schemi. Due domande restano aperte:
  - un worker registra 20 funzioni e una ha il manifest non valido: è rifiutata tutta la
    registrazione o solo quella funzione?
  - i campi sconosciuti di un SDK più nuovo sono rifiutati? Allora un worker aggiornato prima del
    motore non si registra (FR-51; la versione del protocollo resta al tema 4).
- **Il PRD dovrebbe dire:** aggiungere a NFR-14 «manifest e schemi (dimensione, profondità,
  riferimenti ricorsivi)». La granularità del rifiuto e i campi sconosciuti vanno alla spec della
  storia.

---

## Gravità bassa

### EU-B1 · bassa · FR-56 L401–404
- **Scenario:** restano scoperti quattro valori estremi:
  - un campo elenco obbligatorio con `[]` o `null`;
  - un valore singolo che è già un elenco, in un campo elenco di elenchi (ambiguo con L403);
  - la dimensione massima dell'input di avvio, perché FR-11 limita le attività, non l'avvio;
  - i campi in più (vedi EU-M6).
- **Rinvio:** spec della storia, con un rifiuto prima dell'accettazione.

### EU-B2 · bassa · FR-57 L1034
- **Scenario:** due creazioni contemporanee sullo stesso workflow, con il limite di una per
  workflow: la seconda è rifiutata o diventa una modifica? Restano aperti anche altri tre casi:
  - due modifiche contemporanee: serve un controllo ottimistico come FR-5?
  - una modifica mentre scatta una scadenza;
  - da quando si ricalcola la prossima scadenza dopo una modifica.
- **Rinvio:** spec della storia.

### EU-B3 · bassa · §3 L286, L295–298
- **Scenario:** gli elenchi chiusi sono incompleti in tre punti.
  - Tra le cause di sospensione di una pipeline stream mancano il disco pieno o il checkpoint che
    fallisce (NFR-6) e la destinazione che rifiuta le righe (§11.13). I dati usciti dalla
    conservazione (FR-23) sono già risolti da EX L309: un errore, non una causa.
  - Non c'è una regola per il motivo «in coda» quando ne valgono due insieme (nessun worker e
    quota raggiunta). Gli scenari BDD confrontano il JSON esatto.
  - La pianificazione ha solo «sospesa» (vedi EU-A2).
- **Il PRD dovrebbe dire:** le cause mancanti vanno aggiunte. La regola per i motivi multipli va
  alla spec della storia.

### EU-B4 · bassa · FR-10 L475–476, FR-46 L1076, FR-31 L847
- **Scenario:** quando l'operatore annulla dalla dashboard non c'è un nome visualizzato. Il PRD
  non dice quale ruolo si registra né cosa vede il cliente. Lo stesso vale quando il sistema ferma
  un'esecuzione (eliminazione del cliente, FR-31). Un nome visualizzato vuoto rende ambiguo «se
  presente». Lunghezza, caratteri di controllo e bidirezionali permettono di imitare un altro nome
  («annullata da admin»); NFR-7 copre solo la resa come testo.
- **Rinvio:** spec della storia.

### EU-B5 · bassa · NFR-6 L1194–1195, FR-4 L368–369
- **Scenario:** con il disco pieno, «mai un salvataggio parziale» vale solo per i workflow. Non
  vale per la sostituzione di un segreto (il valore vecchio deve restare), per una pianificazione
  né per il caricamento di un nodo.
- **Il PRD dovrebbe dire:** «ogni scrittura rifiutata per disco pieno lascia intatto lo stato
  precedente».

### EU-B6 · bassa · NFR-16 L1250 (EX L397–401)
- **Scenario:** sul telefono la disposizione ignora le posizioni manuali del desktop. Un
  salvataggio dal telefono deve conservarle e collocare i nodi aggiunti dal telefono, altrimenti
  chi lavora su desktop perde il suo layout.
- **Rinvio:** spec della storia dell'editor.

### EU-B7 · bassa · FR-27 L787 (addendum L99–101)
- **Scenario:** la credenziale dura meno dell'anticipo dell'avviso (almeno 60 s): l'avviso parte
  subito o in ciclo.
- **Rinvio:** architettura (UX T-63): una durata minima, oppure un anticipo proporzionale alla
  durata.

### EU-B8 · bassa · FR-41 L991–993, FR-43 L1009–1011, NFR-15 L1243–1246
- **Scenario:** «riprendono da soli» è vero solo entro l'obiettivo di NFR-15. Se il motore cade in
  ciclo, o la ripresa supera 30 s, il testo continua a promettere la ripresa. Il PRD non dice
  nemmeno cosa fanno «Esegui» e «Salva» durante la ripresa: richiesta accettata e messa in coda,
  oppure rifiutata. NFR-15 dice solo che non partono nuove esecuzioni.
- **Rinvio:** spec della storia.

### EU-B9 · bassa · FR-28 L797–799, FR-27 L785–786
- **Scenario:** la credenziale dell'integratore arriva senza il campo del cliente, per esempio per
  un integratore con zero clienti al primo uso (UJ-1). Il PRD non dice cosa fa il kit: mostra
  solo il selettore vuoto o rifiuta?
- **Rinvio:** spec della storia.

### EU-B10 · bassa · FR-4 L378, FR-39 L956–957
- **Scenario:** la parità tra nucleo JS e motore dipende dal contesto: manifest dei worker
  collegati, segreti, funzioni disponibili. Quel contesto cambia tra la validazione nell'editor e
  il salvataggio, e allora il test di parità fallisce senza un bug. EX L460–461 mostra già il
  rifiuto del motore sul task.
- **Il PRD dovrebbe dire:** «stessi esiti a parità di contesto; in caso di differenza vale il
  motore».

### EU-B11 · bassa · FR-12 L503–504, FR-55, §11.10–11
- **Scenario:** un'attività in coda è stata validata sul manifest vecchio e viene consegnata a un
  worker che si è registrato con un manifest nuovo, non compatibile (deploy a rotazione).
- **Il PRD dovrebbe dire:** estendere il testo di §11.11 alle esecuzioni in corso e alle attività
  in coda. Nella sostanza il caso è già coperto da §11.10 e dal tema 4 di `triage.md` (M8 del 19
  novembre).

---

## Tabella di copertura

### Scenari di ciclo di vita richiesti

| Scenario | Esito |
|---|---|
| Manifest cambiato sotto workflow salvati | coperto da §11.11 |
| Manifest cambiato sotto esecuzioni in corso e attività in coda | EU-B11 (nodi in corso: FR-15 L564–566) |
| Due worker con la stessa funzione e manifest diversi | coperto da §11.10; `triage.md` tema 4 (M8) |
| Worker che si scollega | EU-M3 (chiusura ordinata); T-35 (worker che non risponde) |
| Scadenza durante la ripresa del motore | coperto da T-32 e NFR-14 L1229 (recupero); duplicati: EU-M1 |
| Scadenza durante una sospensione | EU-M2 |
| Credenziale che scade durante una modifica | FR-27 L787–788 risponde; resta aperto: EU-A1, EU-M4, EU-B7 |
| Cambio di cliente con lavoro non salvato | risolto da EX L266 (dialogo di conferma); rinnovo con un altro cliente: EU-A1 |
| Cliente eliminato con pianificazioni | coperto da FR-31 L847 (ferma prima le pianificazioni); nessun rilievo |
| Schema di input cambiato tra il modulo e il lancio | coperto da FR-56 L401 (rifiuto con percorso e motivo) e FR-5 L388; versione lanciata da «Esegui»: UX T-62; nessun rilievo |
| Disco pieno durante un salvataggio | workflow: FR-4 L368–369, NFR-6, EX L357–358; altre scritture: EU-B5 |
| Annullamento in gara con il completamento | coperto da T-34; FR-10 aggiunge solo «in coda»; nessun rilievo nuovo |

### Requisiti cambiati

| Requisito | Esito |
|---|---|
| FR-3 | EU-M6 (tipo del join con la politica d'errore) |
| FR-4 | EU-A4, EU-M7, EU-M8, EU-B10 |
| FR-5 | EU-M5 («Salva» ripetuto); ripubblicazione identica: T-16 |
| FR-6 | EU-A2, EU-M1, EU-M2; origine dell'esecuzione: nessun rilievo |
| FR-10 | EU-B4; gare: T-34 |
| FR-12 | EU-B3 (più motivi), EU-B11; stato oltre il limite di attesa: §11.15 |
| FR-14 | EU-M3; T-35 |
| FR-15 | EU-M11; §11.11 |
| FR-27 | EU-A1, EU-M4, EU-B7; sfasamento degli orologi: `triage.md` tema 1 |
| FR-28 | EU-B9 |
| FR-29 | EU-A2, EU-M8; più segreti per connessione: §11.14 |
| FR-32 | EU-M8, EU-M10 |
| FR-35, FR-37, FR-38, FR-49, FR-54, NFR-9, SM-6, UJ-4, UJ-5, §6 | nessun rilievo |
| FR-40 | EU-A4, EU-M6 |
| FR-41 | EU-M5, EU-B8; versione da eseguire: UX T-62 |
| FR-43 | EU-B8 |
| FR-44 | nessun rilievo (nome del motore negli errori: `triage.md` tema 6) |
| FR-45 | nessun rilievo (dove si stampa la credenziale, limite dei tentativi: UX T-63, `triage.md` tema 1) |
| FR-46 | EU-A3, EU-B4 |
| FR-48 | EU-M3 |
| FR-55 | EU-A4, EU-M6, EU-M10, EU-M11; §11.10–11 |
| FR-56 | EU-A4, EU-M6, EU-M7, EU-B1; §11.12–13 |
| FR-57 | EU-A2, EU-M2, EU-B2 |
| FR-58 | EU-M9 |
| NFR-16 | EU-M4, EU-M5, EU-B6 |
| §3 Stati | EU-A3, EU-M3, EU-B3 |
| §11 voci 10–15 | ben formulate; da estendere la 11 (EU-M7, EU-B11) |
