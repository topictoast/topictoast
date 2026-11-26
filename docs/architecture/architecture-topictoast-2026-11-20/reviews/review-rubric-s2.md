# Revisione a rubrica dell'Update S-2 (sandbox)

Oggetto: diff non committato di `ARCHITECTURE-SPINE.md` e `prd.md` contro l'ADR accettata
`docs/decisions/2026-11-26-sandbox.md`. Sola lettura; nessuna modifica ai file riveduti.

## Verdetto

Da correggere prima del merge. L'Update porta nella spine quasi tutte le decisioni dell'ADR e non
lascia testo che dica «la sandbox è indecisa» dentro la spine. Restano però una frase di AD-6 che
contraddice AD-1, una delle due regole del ripiego senza Landlock che manca, e nel PRD tre punti
ancora fermi a prima dello spike, più un'affermazione più forte delle misure.

## Esito per voce della lista

1. Decisioni dell'ADR che due storie potrebbero implementare in modo incompatibile: quasi tutte
   sono in AD-6, nello Stack o nelle Open Questions. Mancano le voci dei rilievi 2, 4, 5, 6 e 7.
2. Testo residuo sulla sandbox «indecisa»: nella spine nessuno trovato (AD-1, AD-13, AD-16,
   diagrammi, Deferred e mappa delle capacità sono coerenti, salvo il rilievo 6 e la nota di
   dettaglio sul diagramma di dispiegamento). Nel PRD ne restano tre (rilievo 5).
3. Coerenza e applicabilità del nuovo AD-6: due contraddizioni (rilievi 1 e 3) e tre lacune
   (rilievi 2, 4, 7).
4. Open Question con scadenza «ADR di S-2»: la riga «chi riavvia il motore» ora ha una scadenza
   vera (spec della prima storia di installazione). Corretto.
5. Modifiche al PRD: FR-54, §11 n.9 e la nota di NFR-12 combaciano con l'ADR. FR-50 e la riga di
   NFR-12 della tabella no (rilievi 5 e 8).

## Rilievi

### 1. Alta — AD-6, «Processi»: «il motore non tiene socket in ascolto raggiungibili dagli ausiliari»

- **Dove:** AD-6, voce «Processi», ultima frase; confronto con AD-1 («ascolta solo su localhost»).
- **Cosa non va:** il motore ascolta per forza sulla porta locale dell'API (AD-1) e `call http` e le
  pipeline stream hanno la rete. Letta alla lettera, la regola è violata da ogni installazione. Nell'ADR
  (punto 7) le regole del ripiego sono due: il canale motore-ausiliario è una coppia di socket
  ereditata, **e** la porta locale dell'API chiede sempre le credenziali (C-14). Nella spine c'è
  solo la prima, riscritta in una forma che non si può rispettare, e senza la condizione del
  ripiego.
- **Correzione:** scrivere due frasi. «Il canale motore-ausiliario è una coppia di socket
  ereditata, mai un socket in ascolto.» E «la porta locale dell'API chiede sempre le credenziali
  (C-14), anche da localhost; senza Landlock `call http` e le pipeline stream partono solo così».
  Aggiungere un test: dalla modalità `call http` senza Landlock, una richiesta alla porta locale
  senza credenziali è rifiutata.

### 2. Media — AD-6, «Moduli e codice nativo»: chi calcola l'impronta e dove stanno le cartelle

- **Dove:** AD-6, voce «Moduli e codice nativo»; ADR, punto 2 e tabella «Voci da fissare».
- **Cosa non va:** il «ruolo di compilazione» non è nell'elenco dei ruoli di «Processi» (validazione,
  jq, `call http`, sandbox leggera, pipeline stream), quindi non si sa se è un sesto ruolo per
  tenant o parte della validazione (e quanti ruoli contare nel costo di 39 MB). Non è detto chi
  calcola l'impronta: se la calcola il processo che ha trattato il modulo ostile e il motore se la
  fa dire, un compilatore compromesso scrive un artefatto e ne dichiara l'impronta. Non è detto
  che la cartella del codice nativo sia per tenant, né quali permessi Landlock abbia ogni ruolo su
  di essa (l'ADR dice che i permessi non sono misurati).
- **Correzione:** nominare il ruolo di compilazione nell'elenco, dire che la cartella è per tenant,
  scrittura solo al ruolo di compilazione di quel tenant, lettura al ruolo della sandbox leggera di
  quel tenant e a nessun altro, e dire chi calcola l'impronta (proposta: il motore, sul file
  appena scritto, o un ruolo di compilazione senza accesso al modulo ostile dopo la fine della
  compilazione). Se resta aperto, metterlo in una Open Question con scadenza.

### 3. Media — AD-6 contro AD-13: avvio alla prima richiesta e arresto per inattività, per le pipeline stream

- **Dove:** AD-6, «Processi» («avviato alla prima richiesta e fermato per inattività»); AD-13
  («attivo solo se il tenant ha pipeline attive»).
- **Cosa non va:** un ausiliario di pipeline stream non riceve richieste: legge il bus da solo
  (AD-13). «Inattività» non è definita per lui; una pipeline con finestre lunghe e poco traffico
  sarebbe fermata e il suo stato in memoria perso, oppure non si ferma mai. Le due regole non si
  incastrano e due storie le leggeranno in modo diverso.
- **Correzione:** in AD-6, «per il ruolo delle pipeline stream il processo vive quanto ha pipeline
  attive (AD-13); l'arresto per inattività vale per gli altri ruoli».

### 4. Media — AD-6, «Confinamento»: come arriva il modulo al processo che non può aprire file

- **Dove:** AD-6, «Confinamento» e «Moduli e codice nativo»; ADR, punti 2 e 7.
- **Cosa non va:** nel ripiego la sandbox leggera ha seccomp senza `open` né `socket`, ma deve
  verificare l'impronta e caricare l'artefatto nativo. Il testo non dice se lo riceve per IPC
  (descrittore o byte) né in che punto dell'ordine di confinamento. Due storie possono fare scelte
  incompatibili (caricamento prima dei filtri, o file letto dopo, che nel ripiego fallisce).
- **Correzione:** una frase: il modulo e la sua impronta arrivano all'ausiliario sull'IPC, o si
  apre prima dei filtri, e la verifica sta prima del primo byte del tenant.

### 5. Media — PRD: testo ancora fermo a prima dello spike

- **Dove:** `prd.md`: FR-50, corpo («quali e come raggrupparli lo fissa lo spike della sandbox»);
  §4.4, introduzione («le tecnologie dei due livelli le sceglie uno spike prima dello sprint 1»);
  §10 n.1 («Uno spike … sceglie le tecnologie»).
- **Cosa non va:** FR-50 dice ancora che i processi da separare li fissa lo spike, ma l'ADR li ha
  fissati (uno per tenant e per ruolo). Gli altri due sono storia, accettabili, ma FR-50 è una
  contraddizione col suo stesso bullet appena modificato.
- **Correzione:** FR-50: «un processo per tenant e per ruolo (AD-6)». §4.4 e §10 si lasciano come
  cronologia solo se la sezione dice che lo spike è chiuso; altrimenti una riga «chiuso il
  2026-11-26, ADR della sandbox».

### 6. Media — PRD, FR-50: «Lo spike l'ha verificato» è più forte delle misure

- **Dove:** `prd.md`, FR-50, bullet dell'arresto per inattività.
- **Cosa non va:** l'ADR dice che NFR-1 regge a riposo come **stima** (motore 26,5 MB da un altro
  binario, il motore vero non esiste) e che SM-C2 non è rispettato all'1 % di riavvii. Il PRD dice
  «verificato» e non nomina il mancato. Anche l'indice delle assunzioni elenca FR-50 come
  «verificata».
- **Correzione:** «Lo spike l'ha stimato: a riposo, con gli ausiliari fermati, NFR-1 regge sul
  percorso dello spike; il motore vero va misurato. All'1 % di riavvii a freddo il p99 peggiora di
  3,1 ms (soglia 2): è una domanda aperta della spine».

### 7. Media — ADR → spine: punti aperti dell'ADR senza voce nella spine

- **Dove:** AD-6 e Open Questions; ADR «Punti aperti e limiti».
- **Cosa non va:** in spine non c'è traccia di tre punti che toccano il formato o il confinamento:
  il codice nativo compilato sull'host e caricato nel guest (con CPU diversa non vale, ipotesi);
  i tetti della compilazione (ADR: proposta 512 MiB e 10 s; la spine dice solo «un tetto»); il
  caricamento dei moduli nella microVM (guest compila da bytecode o riceve nativo: dal testo non
  si capisce). Anche Firecracker su un kernel host fuori elenco e Landlock versione 10 restano
  solo nell'ADR; sono bassi.
- **Correzione:** una riga nelle Open Questions «Compilazione e caricamento dei moduli» (tetti,
  nativo host contro guest, CPU diverse), scadenza spec della prima storia dei nodi in TypeScript o
  della sandbox forte; in AD-6 i tetti come valori predefiniti configurabili.

### 8. Bassa — PRD, NFR-12: la riga della tabella dice ancora «da confermare dopo lo spike»

- **Dove:** `prd.md`, tabella NFR, riga NFR-12.
- **Cosa non va:** la nota nell'indice delle assunzioni è aggiornata, la riga dell'NFR no: due
  testi sulla stessa assunzione con stato diverso.
- **Correzione:** «[ASSUMPTION: … ; quota della sandbox confermata dallo spike, percorso intero da
  misurare con il motore vero]».

### 9. Bassa — Test di enforcement della chiave per ruolo

- **Dove:** AD-6, «Chiave principale».
- **Cosa non va:** l'ADR ha le 19 prove per ruolo (0 fughe) e la voce di S-2 le chiedeva come
  regola. La spine dice come si protegge la chiave ma non chiede il test per ogni sottocomando
  ausiliario nel gate.
- **Correzione:** «un test per ogni ruolo prova che la chiave principale non si legge: file della
  chiave, `/proc` del motore, database, configurazione, cartella di un altro tenant, `ptrace`,
  segnali».

### 10. Bassa — Diagramma di dispiegamento e Rule: due rifiniture

- **Dove:** diagramma «Dispiegamento», nodo `kvm["pool microVM, solo con KVM"]`; AD-6, ripiego
  senza KVM.
- **Cosa non va:** il nodo non dice Firecracker (il diagramma dei contenitori sì). Per il rifiuto
  senza KVM AD-6 dà solo il codice; l'avviso all'operatore (PRD FR-54) sta solo nel PRD.
- **Correzione:** allineare il nome nel diagramma; in AD-6 aggiungere «con un avviso
  all'operatore».

### 11. Bassa — «oltre NFR-1» con più tenant attivi

- **Dove:** S-2 («la memoria supera NFR-1») e Open Question sulla memoria.
- **Cosa non va:** NFR-1 è definito con nessuna esecuzione né pipeline attiva; con tenant attivi
  non si giudica. Il testo dice che lo supera, il che è un'imprecisione di vocabolario, non di
  numeri. Il vero rischio è un tetto di RAM per installazione, che nessun NFR fissa.
- **Correzione:** «supera quello che l'integratore può aspettarsi da NFR-1 e da NFR-3 con più
  tenant attivi; il tetto di ausiliari attivi lo fissa la spec».

## Cosa è corretto

- AD-6 ora vincola un ausiliario per tenant e per ruolo, ordine del confinamento, ripiego che
  fallisce chiuso, `PR_SET_DUMPABLE` come protezione della chiave (la vecchia frase sull'ambiente
  è tolta), Firecracker con immagine del guest di topictoast, microVM monouso o riservata.
- Nessun residuo di M-25…M-28, S-2 «aperto» o «ripiego da decidere» nella spine.
- Stack: wasmtime 50.0.3 con RUSTSEC, javy 9.1.0, Firecracker v1.18.0, `landlock` 0.4.7,
  `seccompiler` 0.5.0 e l'eccezione GPL-2.0 per il kernel coincidono con l'ADR.
- Open Questions: SM-C2, memoria, forte senza root e licenze del disco, javy statico o dinamico
  hanno una scadenza vera.
- PRD: FR-54 e §11 n.9 coincidono con l'ADR (nessuna contraddizione con FR-16 o FR-17).
