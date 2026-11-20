---
title: topictoast — revisione avversariale del PRD
created: 2026-11-19
oggetto: prd.md (draft 2026-11-19), addendum.md, .memlog.md
---

# Revisione avversariale del PRD

**Verdetto:** il PRD non è pronto da consegnare all'architettura: un'architettura può rispettare alla
lettera ogni FR e lasciare comunque cinque buchi di sicurezza tra clienti, e le metriche primarie e
gli NFR si possono soddisfare senza dimostrare ciò che promettono.

Non contesto le scelte del memlog: stream processor nella v1, pipeline stream scritte dai clienti,
nodi precompilati (override); sandbox su due livelli con pool e obiettivi numerici (decisioni).
Attacco le loro conseguenze e il modo in cui i numeri si misurano.

Formato di ogni rilievo: posizione, citazione testuale, perché si rompe, correzione minima (testo o
perimetro, niente funzioni nuove).

Conteggio: **critica 5 · alta 14 · media 19 · bassa 8**.

---

## Critica

### C1. Il canale dei worker non è né autenticato né cifrato, e consegna i segreti

- **Posizione:** FR-12, FR-33, NFR-8.
- **Citazione:** «Un worker si collega con l'SDK, registra le sue funzioni e riceve le attività che
  le invocano.» · «Con più worker che registrano la stessa funzione, le attività si distribuiscono
  tra loro.» · «il worker o la sandbox li ricevono insieme all'attività.»
- **Perché si rompe:** nessun FR chiede che il worker si autentichi. Chi raggiunge la porta del
  protocollo registra `censys_search` e riceve, per FR-12, una quota delle attività di tutti i
  clienti, con i loro segreti in chiaro (FR-33). NFR-8 elenca isolamento, sandbox e cifratura a
  riposo, ma non dice nulla sulla cifratura in transito tra worker, motore, bus e browser. Il
  prodotto vende proprio la cassaforte, e qui un'architettura conforme la svuota. FR-27 («Una
  richiesta senza identità valida viene rifiutata.») non basta: copre le richieste fatte per conto di
  un cliente, la connessione di un worker non lo è, e fuori da FR-45 nessuna credenziale
  dell'integratore è definita.
- **Correzione minima:** a FR-12 aggiungere: «Solo i worker con una credenziale dell'integratore
  possono collegarsi e registrare funzioni.» A NFR-8 aggiungere: «Il traffico tra worker, motore,
  bus e kit frontend è cifrato quando esce dalla macchina; i segreti non viaggiano mai in chiaro su
  una rete.»

### C2. La logica scritta dai clienti gira nel processo del motore, senza limiti

- **Posizione:** FR-1 (espressioni jq, `then`, sotto-workflow), FR-2, FR-8, FR-17, FR-19, FR-25,
  NFR-7.
- **Citazione:** «sotto-workflow, espressioni jq» · «`for` e `then` mantengono la semantica di OWS.»
  · «Il workflow dichiara timeout per task e per l'intera esecuzione.» · «Ogni esecuzione di un nodo
  personalizzato ha limiti di CPU, memoria e tempo.» · «Ogni cliente ha quote sul numero di pipeline
  attive, sulla memoria di stato e sulla CPU.»
- **Perché si rompe:** i limiti esistono solo per i nodi personalizzati (FR-17) e per la memoria di
  *stato* delle pipeline (FR-25). Tre altre strade portano codice dei clienti nel motore, accanto
  alla chiave della cassaforte (FR-16 sposta fuori solo la sandbox):
  1. **jq.** Un'espressione valida ma ostile (ricorsione, `range` enorme) satura CPU o memoria.
  2. **SQL.** La memoria di *esecuzione* di una query (join, ordinamenti) non è quota di stato.
     Nessun FR limita il dialetto: FR-19 rifiuta solo gli stream di altri clienti. I motori SQL
     incorporabili offrono DDL per dichiarare tabelle esterne o connettori (file, HTTP); per il
     motore che sceglierà l'architettura questo resta non verificato.
  3. **Controllo di flusso.** Un `then` che torna indietro o un sotto-workflow ricorsivo gira per
     sempre in passi che non sono attività, quindi le quote di FR-30 non lo toccano. FR-8 non rende
     obbligatorio il timeout di esecuzione.

  NFR-7 copre solo l'input *non valido*: l'input valido e ostile manda il processo in OOM o ne
  esaurisce la CPU, e cade ogni cliente.
- **Correzione minima:**
  - FR-17: estendere i limiti a «espressioni jq, query SQL e passi di controllo dei workflow di un
    cliente; superarli fa fallire l'attività o sospende la pipeline, mai il processo del motore».
  - FR-19: «La query legge solo stream del cliente e scrive solo verso le destinazioni di FR-24:
    niente DDL, file, rete o funzioni definite dall'utente.»
  - FR-8: «L'integratore fissa per cliente un timeout di esecuzione predefinito e uno massimo.»

### C3. La rete in uscita è decisa dal cliente

- **Posizione:** FR-1 (`call http`), FR-16.
- **Citazione:** «L'accesso alla rete esterna è negato salvo una lista di destinazioni dichiarata
  nel nodo» · «Un nodo personalizzato in esecuzione vede solo il proprio input e i segreti che il
  suo task dichiara.»
- **Perché si rompe:**
  - FR-16 vale solo per i nodi personalizzati. `call http` sta nel sottoinsieme, il cliente la
    compone dall'editor e nessun FR dice chi la esegue né verso dove. Se la esegue il motore, è una
    SSRF dal processo che custodisce la chiave: bus, Postgres, metadati del cloud, API interne,
    anche con i segreti del cliente negli header.
  - La lista di destinazioni «dichiarata nel nodo» la scrive l'autore del nodo, cioè l'attaccante:
    non è un controllo.
- **Correzione minima:** riscrivere la terza conseguenza di FR-16 così: «Ogni chiamata in uscita
  fatta per conto di un cliente (nodi personalizzati e `call http`) non raggiunge mai indirizzi
  interni, di loopback o link-local, anche dopo la risoluzione DNS; le destinazioni consentite le
  fissa o le approva l'integratore per cliente.»

### C4. Segreti in chiaro su disco fuori dal database

- **Posizione:** §1, FR-33, FR-34.
- **Citazione:** «le attività viaggiano su un bus di messaggi e il database tiene solo lo stato.» ·
  «Una copia del database senza la chiave di cifratura non rivela alcun segreto.»
- **Perché si rompe:** il disegno naturale che il PRD descrive scrive l'attività sul log del bus,
  segreti compresi (FR-33 li consegna «insieme all'attività»). Il log è append-only e resta su disco
  per tutta la sua conservazione. FR-34 protegge solo la «copia del database», e FR-33 elenca
  «nei log, nello storico, negli output né negli errori di topictoast»: quel «log» non comprende
  chiaramente il log su disco del bus. L'architettura soddisfa entrambi e lascia i segreti in chiaro nei segmenti
  del bus, nei checkpoint o nei file temporanei.
- **Correzione minima:** FR-34: «Nessun segreto è scritto in chiaro su disco da alcun componente
  (database, bus, checkpoint, log); una copia dei dischi senza la chiave non rivela segreti.»

### C5. «non perde lavoro accettato» senza un tetto a ciò che si accetta

- **Posizione:** FR-18, FR-23, FR-24, FR-30, NFR-6.
- **Citazione:** «Superare una quota non perde lavoro accettato.» · «le righe in eccesso restano in
  attesa, non vanno perse.» · «alla ripresa elabora gli eventi arrivati nel frattempo.» ·
  «Nessuna attività, esecuzione o evento accettato va perso per un riavvio o un crash di topictoast
  o del bus.»
- **Perché si rompe:** FR-30 dà quote su attività concorrenti, esecuzioni al minuto, sandbox e
  risorse delle pipeline, ma niente limita gli eventi in ingresso (FR-18), l'arretrato in coda oltre
  quota o gli eventi conservati per una pipeline sospesa da mesi (FR-23). Così un cliente riempie il
  disco condiviso, e il disco pieno non compare tra i guasti. A quel punto o si rifiuta lavoro a
  tutti i clienti, o NFR-6 salta per tutti. È l'attacco di un cliente agli altri più semplice del
  prodotto.
- **Correzione minima:**
  - FR-30: «Ogni cliente ha un tetto all'arretrato (attività ed esecuzioni in coda, eventi
    conservati, byte su disco); oltre il tetto le nuove richieste vengono rifiutate con un errore
    esplicito, prima di essere accettate.»
  - FR-18: conservazione degli eventi per durata o dimensione.
  - FR-23: «elabora gli eventi ancora conservati».
  - NFR-6: «A disco pieno il motore rifiuta lavoro nuovo e non perde quello accettato.»

---

## Alta

### A1. SM-2 e NFR-4: confronto non omogeneo e misura aggirabile

- **Posizione:** §1, NFR-4, SM-2.
- **Citazione:** «≤ 5 istruzioni SQL per attività» · «media sull'intero ciclo di vita dell'attività
  durante il test di NFR-3; riferimento: Hatchet con Postgres come coda, circa 7.700 secondo la
  misura di un utente (hatchet-dev/hatchet#5153)». Fonte del 7.700, addendum del brief: «5,2 milioni
  di istruzioni al giorno per circa 680 task».
- **Perché si rompe:**
  - 680 task al giorno sono uno ogni due minuti circa. Il 7.700 è il polling continuo di Hatchet
    diviso per pochissimi task; lo stesso addendum riporta il maintainer: «il motore deve parlare di
    continuo con il database». Il ≤ 5 di topictoast si misura a 10.000 attività/s, dove ogni costo
    fisso tende a zero. Chi conosce Hatchet smonta il confronto il giorno del lancio, e SM-2 è una
    delle due metriche primarie.
  - La misura si aggira:
    - «istruzioni SQL» non è definito: un INSERT di 1.000 righe conta uno;
    - la forma del carico non è fissata: un'esecuzione con un fan-out enorme diluisce le istruzioni
      per esecuzione;
    - le cancellazioni di FR-31 cadono fuori dai 10 minuti;
    - i checkpoint degli stream non contano;
    - il WAL (600 KB nella fonte) sparisce;
    - spostare lo stato nel bus abbassa il numero e contraddice «il database tiene solo lo stato».
- **Correzione minima:**
  - NFR-4: definire l'istruzione («ogni istruzione eseguita dal driver; si riportano anche le
    righe»), fissare la forma del carico (attività per esecuzione), includere le cancellazioni,
    aggiungere un secondo punto a basso ritmo (680 attività al giorno) con le istruzioni al minuto a
    riposo, e riportare i byte di WAL per attività.
  - SM-2: confrontare solo con Hatchet misurato sulla macchina di riferimento allo stesso ritmo,
    oppure togliere «contro circa 7.700».

### A2. Numero di processi: FR-50 contraddice FR-16, e NFR-1 non dice cosa conta

- **Posizione:** §1.2 punto 8, FR-16, FR-50, NFR-1.
- **Citazione:** «Con SQLite incorporato, topictoast e il bus sono gli unici processi.» · «la
  sandbox, a entrambi i livelli, gira fuori dal processo che custodisce la chiave.» · «RSS di
  topictoast + bus, con SQLite, 60 s dopo l'avvio, nessuna esecuzione né pipeline stream attiva;
  esclusi worker e stato delle pipeline stream»
- **Perché si rompe:**
  - La sandbox leggera in un processo separato fa almeno tre processi: FR-50 è falso per
    costruzione.
  - NFR-1 conta «topictoast + bus»: tutto ciò che si sposta nel processo della sandbox esce dalla
    misura.
- **Correzione minima:**
  - FR-50: «due servizi da installare; topictoast può avviare processi figli propri».
  - NFR-1: «somma dell'RSS di tutti i processi dell'installazione, figli e sandbox leggera inattiva
    compresi; escluso solo il pool della sandbox forte (FR-54)».

### A3. «minimo consumo di RAM» misurato solo a vuoto; «alta scalabilità» senza numeri

- **Posizione:** §1, §1.1 punto 3, NFR-1, §8.2.
- **Citazione:** «L'obiettivo è un motore ad alte prestazioni con il minimo consumo di RAM, e i
  numeri del §5 lo rendono verificabile.» · «Velocità, poca memoria, alta scalabilità»
- **Perché si rompe:**
  - NFR-1 si misura senza pipeline, mentre il glossario definisce la pipeline stream «sempre
    attiva».
  - Non dice con quanti clienti, workflow e storico: un database vuoto rende il numero una vetrina.
  - Nessun NFR misura la RAM sotto carico (NFR-3) o per pipeline attiva, quindi l'obiettivo
    dichiarato resta non verificato proprio dove conta.
  - L'«alta scalabilità» non ha alcun NFR: tutti misurano una macchina sola, e l'alta disponibilità è
    fuori dalla v1.
- **Correzione minima:**
  - NFR-1: fissare l'insieme di dati (per esempio 100 clienti, 1.000 workflow, un milione di
    esecuzioni nello storico).
  - FR-53: riportare l'RSS anche durante NFR-3 e per ogni pipeline attiva (riportato, senza
    obiettivo).
  - §1.1 punto 3: «scala aggiungendo worker; il motore è a nodo singolo nella v1».

### A4. NFR-2, NFR-3 e NFR-5 si soddisfano senza dimostrare le prestazioni

- **Posizione:** NFR-2, NFR-3, NFR-5, FR-53.
- **Citazione:** «carico di 1.000 attività/s» · «attività vuote (la funzione risponde subito),
  worker sulla stessa macchina, sostenuto per 10 minuti» · «eventi scritti sullo stream e consumati
  dalla pipeline, sostenuto per 10 minuti»
- **Perché si rompe:**
  - La latenza si misura a 1.000/s e il throughput a 10.000/s senza un tetto di latenza: accumulare
    lotti per secondi passa NFR-3.
  - Forma del workflow e dimensione dei payload non sono fissate.
  - NFR-5 non dice se gli eventi entrano dall'API pubblica (FR-18, con validazione e cliente) o
    direttamente nel bus: nel secondo caso il numero non riguarda il prodotto.
- **Correzione minima:**
  - NFR-3: aggiungere «con p99 di avvio riportato a 10.000/s».
  - FR-53: dichiarare forma del carico e dimensione dei payload.
  - NFR-5: «eventi scritti tramite l'API o l'SDK di FR-18».

### A5. «non rallenta» e «non possono degradare» non sono verificabili; manca il carico misto

- **Posizione:** FR-13, FR-17, FR-25, FR-30, §1.1 punto 5.
- **Citazione:** «Un worker lento non rallenta le attività consegnate ad altri worker» · «non
  rallenta gli altri clienti» · «Le pipeline stream di un cliente non possono degradare quelle degli
  altri.» · «gli altri clienti non rallentano»
- **Perché si rompe:**
  - Su CPU condivisa ogni carico rallenta gli altri: senza una soglia questi FR o sono sempre falsi
    o non si verificano mai.
  - «Workflow DAG e pipeline SQL continue nello stesso motore» è un differenziante, ma nessun
    requisito misura i workflow mentre una pipeline consuma CPU: NFR-2 e NFR-3 girano senza
    pipeline, NFR-5 senza workflow.
- **Correzione minima:**
  - Riscrivere i quattro FR come «il p99 di NFR-2 degli altri clienti resta entro l'obiettivo
    mentre un cliente è oltre quota / ha un worker bloccato / ha una pipeline al limite».
  - FR-53: una corsa con NFR-2 misurato e una pipeline attiva a un ritmo dichiarato.

### A6. Gli FR presuppongono un bus con partizioni che l'architettura non ha ancora scelto

- **Posizione:** §4.11, FR-9, FR-13, FR-30, FR-51, NFR-1, NFR-6.
- **Citazione:** «anche quando condividono la stessa partizione del bus» · «anche quando condividono
  partizioni del bus (UJ-4)» · «Il vincolo tecnico indicato da Marco (addendum del brief, §8), da
  confermare in architettura, è topictoast più il bus Iggy»
- **Perché si rompe:** il bus è «da confermare», ma sei requisiti ne descrivono il meccanismo. Se
  l'architettura sceglie altro, quei requisiti perdono senso. Se lo conferma, il PRD ha già imposto
  un log a partizioni più uno scheduler nel motore senza averlo deciso (costituzione I).
- **Correzione minima:**
  - Riscrivere come comportamento: «anche quando le loro attività sono accodate insieme».
  - In NFR-1 e NFR-6 sostituire «il bus» con «tutti i processi dell'installazione».
  - Lasciare Iggy solo nel paragrafo dei vincoli del §4.11.

### A7. Il timeout del task conta anche l'attesa in coda?

- **Posizione:** FR-8, FR-12, FR-30, FR-54, UJ-1.
- **Citazione:** «Le attività per una funzione senza worker collegati restano in attesa, entro il
  timeout del task» · «quando il pool è vuoto, le attività attendono una sandbox libera entro il
  timeout del task» · «Un cliente oltre quota vede le sue nuove attività in coda» · «le attività
  restano in attesa e partono appena il worker si registra, senza errori»
- **Perché si rompe:**
  - Se il timeout comprende l'attesa (FR-12, FR-54), le attività in coda per quota scadono e,
    finiti i retry, falliscono: è lavoro accettato perso, contro FR-30. Anche il caso limite di UJ-1
    fallisce se il worker parte dopo il timeout.
  - Se non la comprende, FR-12 e FR-54 sono sbagliati.
- **Correzione minima:** FR-8: «Il timeout del task conta dalla consegna al worker o alla sandbox;
  l'attesa in coda è limitata solo dal timeout dell'esecuzione.»

### A8. Il join supera il limite di 1 MB; il fan-out non ha un massimo

- **Posizione:** FR-3, FR-11, FR-41, UJ-2.
- **Citazione:** «Con un elenco di N elementi nascono N attività; il join riceve N risultati
  nell'ordine dell'elenco.» · «Input e output di un'attività hanno un limite di dimensione» · «Un
  fan-out da migliaia di elementi resta leggibile»
- **Perché si rompe:**
  - Il task dopo il join riceve N risultati come input: 300 risultati da 4 KB superano 1 MB, e UJ-2
    fallisce a seconda di quanto è grande l'arricchimento.
  - «migliaia» di elementi vuol dire risultati sotto 1 KB circa.
  - L'elenco che genera il fan-out è a sua volta un output da 1 MB al massimo.
  - Nessun FR fissa il numero massimo di elementi, che è il valore estremo da coprire.
- **Correzione minima:** FR-3: «Il limite di FR-11 vale anche per l'input aggregato del join;
  superarlo fa fallire il join con dimensione e limite. Il numero massimo di elementi per fan-out è
  una quota del cliente (FR-30).»

### A9. Una dipendenza fallita o saltata blocca l'esecuzione: la semantica del DAG manca

- **Posizione:** FR-1 (`switch`, `try`, `raise`), FR-2, FR-3.
- **Citazione:** «Un task parte quando tutte le sue dipendenze sono completate, non prima.»
- **Perché si rompe:**
  - Se una dipendenza fallisce dopo i retry, o se `switch` la salta, i dipendenti non partono mai e
    l'esecuzione resta appesa fino al timeout, che è facoltativo.
  - FR-3 dà una politica di errore solo al fan-out.
  - L'incrocio tra dipendenze DAG e `then` o `switch` non è definito.
  - È il cuore dell'estensione: senza, l'architettura non disegna lo scheduler e le storie non
    hanno scenari.
- **Correzione minima:** a FR-2 aggiungere tre conseguenze (anche come `[ASSUMPTION]`):
  - «una dipendenza fallita fa fallire i dipendenti e l'esecuzione, salvo `try`»;
  - «una dipendenza saltata da `switch` fa saltare i dipendenti»;
  - «un task con dipendenze DAG non è destinazione di `then`».

  In alternativa, spostare la questione nel §11 come domanda da chiudere prima dell'architettura.

### A10. Il cliente usa SDK e CLI, ma l'identità passa solo dalla piattaforma ospite

- **Posizione:** §4.8, FR-15, FR-27, FR-32, UJ-2.
- **Citazione:** «gli SDK TypeScript e Rust servono all'integratore» · «Il cliente può caricare un
  nodo personalizzato già compilato tramite SDK o CLI» · «La piattaforma ospite autentica i propri
  utenti e, per ogni richiesta al motore, garantisce l'identità del cliente.» · «caricato in
  precedenza da un collega sviluppatore con la CLI»
- **Perché si rompe:**
  - La CLI dello sviluppatore del cliente non passa dalla piattaforma ospite: nessun FR dice chi
    emette credenziali per cliente, con che durata, né come si revocano.
  - Anche il token del browser (FR-27) non ha scadenza, ruolo né revoca: se trapela, vale per
    sempre.
- **Correzione minima:** FR-27: «La credenziale dice cliente e ruolo, ha una scadenza e si può
  revocare; l'integratore la emette anche per la CLI di un cliente.»

### A11. NFR-6 non definisce il guasto e il benchmark non lo misura

- **Posizione:** NFR-6, FR-53, SM-C1, §8.2.
- **Citazione:** «Vale anche durante i test di NFR-3 e NFR-5.» · «Il benchmark produce RAM a riposo,
  latenza p50 e p99 di avvio delle attività, attività al secondo, istruzioni SQL per attività,
  eventi al secondo delle pipeline stream.»
- **Perché si rompe:**
  - Il termine «crash» non è definito. Con `kill -9` di un processo la cache del sistema
    operativo sopravvive; con lo spegnimento improvviso dell'host servono scritture sincrone su
    disco. La differenza decide se NFR-2 e NFR-3 sono raggiungibili.
  - FR-53 non produce alcun conteggio di attività perse, quindi SM-C1 non si verifica.
  - Con un nodo solo e senza backup, la perdita del disco cancella stato e segreti di tutti i
    clienti, e il PRD non lo dice.
- **Correzione minima:**
  - NFR-6: dichiarare quale guasto vale (`kill -9`, spegnimento dell'host o entrambi).
  - FR-53: aggiungere l'uscita «attività ed eventi persi dopo un guasto iniettato durante NFR-3 e
    NFR-5».
  - §8.2: «backup e ripristino coerenti di database e bus: fuori dalla v1, documentati come
    limite».

### A12. «gira senza modificare nulla» contro chiavi e credenziali

- **Posizione:** FR-27, FR-34, FR-45, FR-50, NFR-13.
- **Citazione:** «Con la configurazione predefinita, un workflow di esempio gira senza modificare
  nulla.» · «L'operatore accede alla dashboard con credenziali configurate all'installazione.»
- **Perché si rompe:** far girare l'esempio senza toccare nulla richiede che esistano già la chiave
  di firma (FR-27), la chiave della cassaforte (FR-34) e le credenziali della dashboard. Con 15
  minuti come obiettivo (NFR-13), la strada più breve è metterle fisse nel repository o
  nell'immagine: tutte le installazioni condividono gli stessi segreti, e la dashboard vede input e
  output di tutti i clienti (FR-46).
- **Correzione minima:** FR-50: «Al primo avvio topictoast genera a caso chiave della cassaforte,
  chiave di firma e credenziali della dashboard e le mostra una volta; nessun segreto fisso nel
  repository o nelle immagini; per default ascolta solo su localhost.»

### A13. Eliminare un cliente da un log append-only condiviso

- **Posizione:** FR-31, FR-18, §1.
- **Citazione:** «Eliminare un cliente elimina tutte le sue risorse.» · «Le esecuzioni più vecchie
  della durata vengono eliminate con i loro input e output.»
- **Perché si rompe:**
  - Input, output ed eventi che passano per un log append-only condiviso non si cancellano uno per
    uno: o la promessa è falsa fino alla scadenza del segmento, o si impone un topic per cliente.
    Nel secondo caso le risorse crescono con i clienti, mentre NFR-1 si misura a vuoto.
  - Nessun termine di tempo; checkpoint, moduli caricati e sandbox riservate non sono nominati.
- **Correzione minima:** FR-31: «entro <durata> dalla richiesta, in tutti i componenti (database,
  bus, checkpoint, moduli caricati, sandbox riservate)». Il meccanismo lo sceglie l'architettura.

### A14. Perimetro della v1 senza ordine di taglio

- **Posizione:** §8.1, §10 punto 5.
- **Citazione:** «sono molto lavoro per un primo rilascio; due dei pezzi hanno bisogno di uno
  spike.»
- **Perché si rompe:**
  - Il §10 nomina il rischio ma non ha mitigazione. Override e decisioni portano nella v1 uno
    stream processor (la ricerca: «lavoro da team dedicato»), due livelli di sandbox, due SDK, una
    CLI, quattro componenti React con WCAG AA da tastiera sul grafo, due lingue e due architetture
    di CPU.
  - Se un pezzo slitta, slitta il lancio, e con lui SM-1 e SM-4.
- **Correzione minima (senza togliere nulla di deciso):** nel §8 aggiungere un ordine di taglio:
  «se la v1 tarda, escono come sperimentali, in quest'ordine: FR-54, arm64 (NFR-10), …». In
  alternativa, un cancello: «il rilascio pubblico avviene quando il §8.1 è completo».

---

## Media

### M1. Versioni 0.y.z contro «stessa major»; aggiornamento del bus non controllabile

- **Posizione:** §6, FR-51.
- **Citazione:** «Fino al primo rilascio stabile (0.y.z) una rottura di contratto alza la minor;
  dopo, solo la major.» · «un workflow accettato da una versione resta eseguibile dalle versioni
  successive della stessa major.» · «Anche l'aggiornamento del bus, che oggi cambia formato su disco
  tra versioni senza migrazione»
- **Perché si rompe:**
  - In 0.y.z la major è sempre 0: una minor può rompere il formato, ma i documenti devono restare
    eseguibili «della stessa major». Le due frasi si contraddicono.
  - topictoast non controlla il formato su disco del bus: senza una procedura di svuotamento FR-51
    non è realizzabile.
- **Correzione minima:**
  - §6: «finché 0.y.z, la garanzia vale nella stessa minor; le esecuzioni in corso finiscono anche
    dopo un aggiornamento di minor».
  - FR-51: «con una procedura documentata che svuota il bus prima di cambiarne la versione».

### M2. FR-9 promette ciò che la consegna almeno una volta esclude

- **Posizione:** FR-9.
- **Citazione:** «Dopo un riavvio, nessun task completato viene rieseguito e nessuna attività
  accettata va persa.» · «La consegna ai worker è almeno una volta»
- **Perché si rompe:** un task completato dal worker ma non ancora registrato quando arriva il crash
  viene rieseguito. Letta alla lettera, la frase non si può testare.
- **Correzione minima:** «nessun task il cui completamento è stato registrato viene rieseguito».

### M3. La riassegnazione consuma un tentativo; manca la politica di retry predefinita

- **Posizione:** FR-7, FR-14, UJ-4.
- **Citazione:** «vengono riassegnate ad altri worker, contando un tentativo.» · «Il workflow
  dichiara per ogni task una politica di retry»
- **Perché si rompe:** un task con un solo tentativo, o senza politica dichiarata, fallisce invece
  di essere riassegnato, mentre UJ-4 promette «già state riassegnate». Il valore predefinito non c'è.
- **Correzione minima:** FR-7: politica predefinita esplicita. FR-14: «la perdita del worker non
  consuma tentativi», oppure correggere UJ-4.

### M4. Riferimenti «segnalati all'invio» contro funzioni e segreti che arrivano dopo

- **Posizione:** FR-4, FR-12, FR-29, UJ-1.
- **Citazione:** «I riferimenti a funzioni, nodi personalizzati e segreti inesistenti sono segnalati
  all'invio.»
- **Perché si rompe:**
  - Un workflow dell'integratore usa i segreti di ciascun cliente (FR-29): all'invio non esistono.
  - In UJ-1 il workflow può precedere la registrazione del worker (FR-12).
  - «segnalati» non dice se è un rifiuto o un avviso.
- **Correzione minima:** «segnalati come avviso; all'avvio, un segreto o una funzione mancante per
  quel cliente fa fallire l'avvio con un errore che lo nomina».

### M5. «chi l'ha annullata» con un'identità che è solo l'organizzazione

- **Posizione:** FR-10, FR-27.
- **Citazione:** «L'esecuzione risulta annullata nello storico, con chi l'ha annullata e quando.»
- **Perché si rompe:** FR-27 garantisce l'identità del *cliente*, non della persona.
- **Correzione minima:** FR-27: la credenziale può portare un identificativo opaco dell'utente; FR-10
  lo registra quando c'è.

### M6. Editor e vista delle esecuzioni sui documenti che l'editor non ha creato

- **Posizione:** FR-29, FR-35, FR-36, FR-40, FR-41.
- **Citazione:** «Il workflow salvato dall'editor è un documento OWS valido secondo lo schema di
  topictoast» · «Il cliente può seguire un'esecuzione in tempo reale: stato di ogni task»
- **Perché si rompe:** la vista deve disegnare anche workflow scritti come codice o inviati come
  documento, con `then` all'indietro (cicli), `switch`, `try`, `do` annidati e sotto-workflow.
  Nessun FR dice se l'editor li apre, li modifica o dove salva la disposizione del grafo. È lavoro
  nascosto nel pezzo che il PRD mette al primo posto tra i differenzianti.
- **Correzione minima:**
  - FR-41: «la vista disegna ogni documento accettato da FR-4».
  - FR-40: «l'editor modifica i documenti che ha creato; gli altri li apre in sola lettura», oppure
    l'opposto, ma detto.

### M7. Perimetro nascosto: distribuzione, errori senza marchio, canale in tempo reale

- **Posizione:** NFR-10, NFR-13, FR-41, FR-44.
- **Citazione:** «da download o clone a un workflow di esempio completato» · «Nessun componente
  mostra il nome o il logo di topictoast se non lo si chiede.» · «I testi sono tutti sostituibili»
- **Perché si rompe:**
  1. Quindici minuti da un clone richiedono binari e immagini precompilati per due architetture e i
     pacchetti di SDK e kit pubblicati sui registri; compilare il workspace da sorgente
     probabilmente non ci sta (non verificato).
  2. Gli errori generati dal motore (FR-1, FR-4, FR-17, FR-19) arrivano al cliente attraverso il kit:
     devono essere senza marchio e traducibili.
  3. FR-41 richiede un canale push verso il browser filtrato per cliente.

  Nessuno dei tre è nominato.
- **Correzione minima:**
  - §8.1: «binari e immagini per le piattaforme di NFR-10, SDK e kit pubblicati sui registri».
  - FR-44: «gli errori del motore mostrati ai clienti hanno un codice stabile e parametri; il testo
    lo fornisce il kit».

### M8. Chiave di idempotenza senza durata; destinazione stream senza semantica

- **Posizione:** FR-6, FR-24, FR-31.
- **Citazione:** «Un avvio con una chiave di idempotenza già usata dallo stesso cliente restituisce
  l'esecuzione esistente» · «chiave di idempotenza derivata dalla riga, FR-6»
- **Perché si rompe:**
  - Se la chiave vive per sempre, cresce senza limite; se muore con lo storico (30 giorni), una
    rielaborazione successiva rilancia.
  - Due righe distinte con lo stesso contenuto collassano in una.
  - Non è detto cosa succede a due avvii concorrenti con la stessa chiave.
  - Con destinazione stream, dopo un riavvio i duplicati non sono trattati.
- **Correzione minima:**
  - FR-6: «la chiave vale per la durata di conservazione, anche tra richieste concorrenti».
  - FR-24: «la chiave comprende pipeline, versione e finestra; con destinazione stream la consegna è
    almeno una volta».

### M9. Pianificazioni perse durante un fermo

- **Posizione:** FR-6.
- **Citazione:** «Una pianificazione cron o a intervallo avvia un'esecuzione per ogni scadenza»
- **Perché si rompe:** letto alla lettera, dopo un'ora di fermo con un cron al minuto partono 60
  esecuzioni insieme. Non è detto nemmeno cosa succede se l'esecuzione precedente è ancora in corso.
- **Correzione minima:** «le scadenze perse durante un fermo producono una sola esecuzione di
  recupero (o nessuna, a scelta per pianificazione); la sovrapposizione si dichiara».

### M10. Un evento datato nel futuro chiude tutte le finestre

- **Posizione:** FR-18, FR-21.
- **Citazione:** «Un evento senza tempo dell'evento riceve il tempo di arrivo.» · «Una finestra si
  chiude quando il watermark supera la sua fine più il ritardo tollerato.»
- **Perché si rompe:** un evento con data 2099, per errore o per malizia, fa avanzare il watermark:
  tutte le finestre si chiudono e gli eventi successivi vengono scartati come in ritardo. La
  pipeline si rompe in silenzio. Manca il valore estremo.
- **Correzione minima:** FR-18: «un tempo dell'evento oltre <soglia> nel futuro viene rifiutato con
  un errore di validazione».

### M11. I numeri della sandbox non stanno sulla macchina di riferimento

- **Posizione:** NFR-12, FR-54, UJ-2.
- **Citazione:** «memoria ≤ 64 MB per istanza della sandbox leggera; ≤ 10 ms per la sandbox forte
  quando il pool ha una sandbox libera» · «è monouso o riservata a quel cliente.» · «La RAM del pool
  si misura a parte, fuori da NFR-1.»
- **Perché si rompe:**
  - 300 nodi in parallelo (UJ-2) × 64 MB fanno 19,2 GB, oltre i 16 GB della macchina di
    riferimento, e non c'è un obiettivo di istanze concorrenti.
  - La sandbox forte si misura solo «quando il pool ha una sandbox libera»: un pool enorme supera il
    test.
  - Con sandbox monouso il ritmo sostenuto dipende dal riavvio delle microVM (fino a 125 ms per
    Firecracker, addendum del brief), che non si misura.
  - La RAM del pool non ha obiettivo e cresce con i clienti a cui si riserva una sandbox.
- **Correzione minima:** NFR-12: «istanze leggere concorrenti sostenute sulla macchina di
  riferimento; attività/s sostenute dalla sandbox forte monouso con un pool di dimensione
  dichiarata; RAM per sandbox del pool riportata».

### M12. Formato del nodo compilato: TypeScript ambiguo, livelli forse incompatibili

- **Posizione:** §3 (Nodo personalizzato), FR-15, FR-54, §6.
- **Citazione:** «Un nodo che non rispetta il formato richiesto (modulo compilato, schema)» ·
  «l'integratore può attivare la sandbox forte per tutti i clienti o per alcuni.»
- **Perché si rompe:**
  - Un nodo TypeScript «già compilato» non dice verso quale formato: il JavaScript ha bisogno
    di un motore JavaScript.
  - Se i due livelli accettano formati diversi, spostare un cliente sulla sandbox forte rompe i suoi
    nodi, e il formato è un contratto pubblico (§6).
- **Correzione minima:** FR-15: «lo stesso modulo compilato gira su entrambi i livelli; il formato
  lo fissa lo spike del §10 punto 1».

### M13. Ciclo di vita della chiave della cassaforte

- **Posizione:** FR-34.
- **Citazione:** «I segreti sono cifrati a riposo.»
- **Perché si rompe:** il PRD non dice cosa succede se la chiave si perde: i segreti di tutti i
  clienti diventano irrecuperabili, cioè perdita di dati, e nemmeno la rotazione è collocata.
- **Correzione minima:** FR-34: «la perdita della chiave rende irrecuperabili i segreti ed è
  documentata; la rotazione è fuori dalla v1».

### M14. Segreti negli output: promessa non mantenibile; segreti in transito nella sandbox condivisa

- **Posizione:** FR-33, NFR-8.
- **Citazione:** «I segreti non compaiono nei log, nello storico, negli output né negli errori di
  topictoast.»
- **Perché si rompe:**
  - Il codice del cliente o la funzione del worker può restituire il segreto, anche in base64:
    topictoast non lo impedisce.
  - Se la sandbox leggera ospita più clienti nello stesso processo, contiene i segreti decifrati di
    altri clienti in transito. Tenere la sandbox lontana dalla chiave non basta, e NFR-8 non lo
    prova.
- **Correzione minima:**
  - FR-33: «topictoast non scrive segreti in ciò che produce (log, storico, errori propri); ciò che
    restituisce il codice dei clienti o dei worker non è garantito, ed è documentato».
  - NFR-8: aggiungere il test «un nodo ostile non legge i segreti di un'attività di un altro cliente
    in esecuzione nello stesso momento».

### M15. FR-1 non ha un oracolo, e il sottoinsieme è tutto un'assunzione

- **Posizione:** FR-1, FR-4, FR-39, FR-40, §6.
- **Citazione:** «Un documento OWS 1.1 che usa solo il sottoinsieme viene accettato ed eseguito con
  la semantica della specifica.» · «conformità completa al CTK di OWS»
- **Perché si rompe:** senza il CTK, «la semantica della specifica» non si verifica. Il sottoinsieme
  è interamente `[ASSUMPTION]` ma regge FR-4, FR-39, FR-40 e un contratto pubblico.
- **Correzione minima:** «verificato con gli scenari del CTK di OWS relativi ai costrutti del
  sottoinsieme»; confermare il sottoinsieme prima dell'architettura.

### M16. Metriche di successo aggirabili

- **Posizione:** §1, SM-1, SM-4, SM-5.
- **Citazione:** «Almeno 500 a sei mesi dal primo rilascio pubblico» · «Fra due o tre anni
  topictoast è incorporato in più piattaforme, non solo in quella di Marco.» · «eseguiti
  dall'inizio alla fine prima del primo rilascio pubblico, sull'app di Marco» · «rispettato da
  almeno tre persone che non conoscono topictoast»
- **Perché si rompe:**
  - Le stelle misurano la promozione e si comprano; nessuna metrica segue la visione, cioè gli
    integratori esterni.
  - Il «primo rilascio pubblico» non è definito: un'alpha fa partire il cronometro, un rinvio lo
    ferma.
  - SM-4 lo valuta l'autore, e comprende UJ-3, che è ancora un'assunzione.
  - SM-5 non fissa né chi si recluta né la macchina.
- **Correzione minima:**
  - Definire il primo rilascio pubblico (§8.1 completo e NFR-8 superato).
  - Affiancare a SM-1 «almeno un integratore diverso da Marco con topictoast in uso entro 12 mesi».
  - SM-4 con corse registrate e UJ-3 confermato.
  - SM-5 con prerequisiti della macchina e profilo dei partecipanti.

### M17. Formulazione dei differenzianti e domanda non validata

- **Posizione:** §1, §1.1 punti 4 e 5, UJ-3, §11 punto 1.
- **Citazione:** «tre pezzi che nessun progetto open fornisce insieme» · «I pezzi esistono altrove
  (Windmill con AGPL, Activepieces con parti a pagamento), ma non insieme e non gratis.»
- **Perché si rompe:**
  - Il §1 usa il criterio «open», il §1.1 punto 4 quello della licenza permissiva e gratuita:
    Windmill (AGPL) è open, quindi la frase del §1, letta alla lettera, è più forte di quanto il
    §1.1 sostiene.
  - Il differenziante 5, il pezzo più costoso, non ha una domanda validata: UJ-3 è un'assunzione e
    `research-stream.md` scrive «il rischio e' che il mercato non lo chieda (da validare)».
- **Correzione minima:**
  - §1: «nessun progetto con licenza permissiva e senza parti a pagamento».
  - §11 punto 1: da chiudere prima che l'architettura fissi FR-18–FR-25 (l'override resta, cambia
    solo l'ordine).

### M18. Il carico sul database delle pipeline stream non è misurato

- **Posizione:** §1.1 punto 2, FR-22, NFR-4, §11 punto 4.
- **Citazione:** «L'intervallo tra checkpoint è configurabile, con un valore predefinito» · «Dove
  salvare i checkpoint delle pipeline stream nella v1, visto che RustFS arriva dopo: architettura.»
- **Perché si rompe:** «Database al minimo» si misura solo per attività. Un checkpoint ogni 10
  secondi, con lo stato fino alla quota, per ogni pipeline di ogni cliente, finisce probabilmente
  nel database (RustFS manca) senza alcun limite.
- **Correzione minima:** FR-53: riportare i byte scritti al secondo su database e disco con N
  pipeline attive.

### M19. FR-38: parità totale non verificabile, raddoppiata su due SDK

- **Posizione:** FR-38.
- **Citazione:** «Tutto ciò che si fa da editor e dashboard si può fare anche tramite SDK.»
- **Perché si rompe:** l'elenco è aperto, comprende azioni dei clienti (l'editor) in SDK che il §4.8
  riserva all'integratore, e va duplicato in TypeScript e Rust.
- **Correzione minima:** elencare le operazioni (clienti, quote, segreti, pipeline stream, workflow,
  annullamento, lettura dello storico).

---

## Bassa

### B1. WCAG AA con temi della piattaforma ospite

- **Posizione:** NFR-9, FR-44.
- **Citazione:** «Lo sviluppatore della piattaforma ospite può adattare colori, caratteri, testi e
  lingua dei componenti React»
- **Perché si rompe:** colori scelti dalla piattaforma ospite possono rompere il contrasto.
- **Correzione minima:** «AA garantito con il tema predefinito; con un tema personalizzato il
  contrasto è responsabilità dell'integratore».

### B2. «Tutti misurati con il benchmark» non è vero

- **Posizione:** §5, FR-53.
- **Citazione:** «Tutti misurati con il benchmark di FR-53 sulla macchina di riferimento»
- **Perché si rompe:** NFR-13 si misura con persone, e NFR-12 non è tra le uscite di FR-53.
- **Correzione minima:** aggiungere NFR-12 alle uscite di FR-53 e dire che NFR-13 si misura a parte.

### B3. Disco della macchina di riferimento non specificato

- **Posizione:** §3 (Macchina di riferimento).
- **Citazione:** «4 vCPU, 16 GB di RAM, disco SSD, Linux»
- **Perché si rompe:** la latenza delle scritture sincrone (NVMe locale o disco di rete) decide
  NFR-2 e NFR-3; «eseguibile da chiunque» darà risultati molto diversi.
- **Correzione minima:** «SSD NVMe locale», oppure riportare la latenza delle scritture sincrone nel
  benchmark.

### B4. Validazione del nucleo JS identica a quella del motore

- **Posizione:** FR-39.
- **Citazione:** «La validazione del nucleo JS dà gli stessi esiti della validazione del motore
  (FR-4) sugli stessi documenti.»
- **Perché si rompe:** FR-4 verifica anche che i riferimenti esistano, cosa che solo il server sa;
  la frase impone un validatore condiviso.
- **Correzione minima:** «per lo schema; i riferimenti li verifica il motore».

### B5. La modifica di una pipeline perde le finestre aperte

- **Posizione:** FR-22, FR-23.
- **Citazione:** «Modificare la query di una pipeline attiva crea una nuova versione che parte con
  stato vuoto»
- **Perché si rompe:** contrasta con «nessuna finestra perde eventi già accettati».
- **Correzione minima:** dichiarare l'eccezione in FR-22.

### B6. Riferimento Temporal non confrontabile

- **Posizione:** §5.
- **Citazione:** «Temporal dev server circa 150 MB a riposo»
- **Perché si rompe:** il valore comprende il worker (112 + 35 MB, `research-footprint.md`), mentre
  NFR-1 esclude i worker.
- **Correzione minima:** usare 112 MB.

### B7. «Pipeline» da sola, vietata dal glossario

- **Posizione:** §2.1, UJ-1.
- **Citazione:** «pipeline in ritardo» · «vuole che i suoi clienti compongano da sé le pipeline.»
- **Perché si rompe:** il §3 lo vieta, e in UJ-1 il senso è ambiguo (workflow o pipeline stream).
- **Correzione minima:** «pipeline stream in ritardo»; «compongano da sé i workflow».

### B8. Mappature del §9 che non validano ciò che dichiarano

- **Posizione:** SM-2, SM-3.
- **Citazione:** «Valida FR-9, FR-53.» · «Valida FR-12, FR-13, FR-18–FR-24.»
- **Perché si rompe:** contare istruzioni SQL non valida la durabilità, e i numeri di throughput non
  validano la correttezza di checkpoint e idempotenza.
- **Correzione minima:** SM-2 valida NFR-4 e FR-53; la durabilità si valida con l'uscita di A11.
