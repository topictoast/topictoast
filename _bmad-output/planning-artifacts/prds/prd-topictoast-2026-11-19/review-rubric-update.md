# PRD Quality Review — topictoast (aggiornamento del 2026-11-20)

Ambito: solo le parti cambiate rispetto alla copia di prima dell'aggiornamento (`prd.pre-update.md`)
e la loro interazione con il resto del PRD. I rilievi delle revisioni del 2026-11-19
(`review-rubric.md`, `review-edge-cases.md`, `triage.md`) tornano solo dove l'aggiornamento li
peggiora, e lo dico. Righe citate da `prd.md` attuale.

## Overall verdict

Verdetto solo sull'aggiornamento. L'aggiornamento regge: manifest, schema di input, pianificazioni
e area workflow servono tutti la prima differenza del prodotto (il kit con editor no-code); l'elenco
chiuso degli stati e i termini Manifest e Connessione danno all'architettura un vocabolario stabile;
SM-6 è la prima metrica sul kit. A rischio soprattutto un punto: gli avvii rifiutati da
pianificazioni e pipeline stream (input fuori schema, connessione senza segreto) sono rinviati alle
spec delle storie, mentre la risposta decide se FR-6 e FR-24 restano veri e se una pianificazione
porta un input. Sotto, alcuni medi da chiudere prima delle epiche: le fonti dei dati di FR-40
restano indietro rispetto al mock approvato, FR-1 e FR-57 si contraddicono sulla pianificazione,
l'identità delle connessioni non è definita. Chiuso il primo punto, il PRD è pronto per
l'architettura.

## Decision-readiness — adequate

Le decisioni nuove sono scritte come decisioni:
- «Dall'editor, salvare è pubblicare» (FR-5);
- workflow definito come codice in sola lettura per tutti (FR-35);
- workflow dell'integratore creati solo dall'SDK (FR-28);
- al massimo una pianificazione per workflow (FR-57);
- AA garantito con i colori predefiniti e controllo sui colori dell'ospite (NFR-9, FR-44);
- il segreto dichiara la connessione che soddisfa (FR-32), che chiude il lato di prodotto di §11.6.

Le sei domande nuove di §11 sono davvero aperte e hanno un destinatario.

Il PRD cede in due punti. Alcune decisioni non dicono cosa costano: salvare è pubblicare,
l'allargamento del kit. Due domande rinviate alle spec delle storie (§11.12, §11.13) decidono se
restano vere conseguenze già scritte in FR-6 e FR-24: con una risposta quelle conseguenze reggono,
con un'altra diventano false. Ho fatto questa prova su tutte le voci 10–15. Le voci 10, 11 e 14 non
rendono falso alcun testo già scritto e possono restare dove sono. La 15 può aggiungere uno stato
all'elenco chiuso delle attività (§3): la spec che la chiude deve aggiornare anche il glossario.

### Findings

- **high** Avvii rifiutati dai trigger automatici rinviati alle spec delle storie (§11 punti 12–13,
  L1403–1406; FR-6 L423; FR-24 L724–725; FR-29 L815–817; FR-56 L401–402; FR-57 L1026–1037) — Con
  l'aggiornamento un avvio può essere rifiutato «prima di essere accettato» per due motivi nuovi:
  - input non conforme allo schema (FR-56);
  - connessione senza segreto in un workflow dell'integratore (FR-29).

  Per una chiamata API il chiamante riceve l'errore. Per una pianificazione o una pipeline stream
  non c'è chiamante. Due conseguenze già scritte possono diventare false: «Una pianificazione cron o
  a intervallo avvia un'esecuzione per ogni scadenza» (FR-6) e, con destinazione workflow, «ogni
  riga avvia esattamente un'esecuzione» (FR-24). La quota invece è già regolata per le pipeline
  stream (righe trattenute, FR-24 e FR-30); per le scadenze di una pianificazione oltre quota la
  lacuna c'era già ieri.

  Non è un caso marginale. Il workflow di UJ-2 «riceve un elenco di» indicatori: senza
  quell'elenco non ha niente da fare, e per pianificarlo bisogna sapere da dove arriva l'elenco a
  ogni scadenza. È la domanda §11.12, che decide anche il modello dati di FR-57: se la
  pianificazione porta un input, con «Il cliente ha al massimo una pianificazione per workflow.»
  (L1034) ogni workflow ha un solo input pianificato. Il rifiuto per connessione mancante su un
  trigger automatico non è in nessuna domanda di §11. Una spec di storia non è il posto per
  decidere tutto questo.

  *Fix:* decidere nel PRD una regola unica per i trigger automatici.
  1. La pianificazione porta un input, validato sullo schema alla creazione e a ogni nuova versione
     del workflow.
  2. Un avvio rifiutato da una pianificazione o da una pipeline stream (schema, connessione) lascia
     una traccia con il motivo, visibile al cliente e all'operatore: scadenza rifiutata, riga
     trattenuta con errore. FR-24 dice cosa vuol dire «esattamente un'esecuzione» in quel caso.
  3. §11.12 e §11.13 passano tra le decisioni da chiudere prima delle epiche.
- **medium** «Salvare è pubblicare» senza il suo costo (FR-5 L384, L389; FR-27 L787–788; FR-40
  L974–976) — La decisione è chiara e chiude il rilievo basso di ieri sulla bozza. Il PRD però non
  dice cosa costa, e i costi sono due.
  - Un lavoro incompleto o non valido non si salva: FR-40 mostra l'errore prima del salvataggio e
    FR-4 rifiuta il documento. Il lavoro resta solo nel kit («non salvato resta e torna dopo il
    rinnovo», FR-27), e chiudere la scheda lo perde.
  - Ogni salvataggio dall'editor cambia subito cosa eseguono le pianificazioni (FR-57) e le pipeline
    stream con destinazione workflow (FR-24), perché «Le nuove esecuzioni usano l'ultima versione
    pubblicata». Se il salvataggio cambia lo schema di input (FR-56), le scadenze e le righe
    successive possono essere rifiutate (rilievo high sopra).

  Il rinvio di ieri sulle versioni seguite dalle destinazioni (`review-edge-cases.md`, FR-5) pesa di
  più adesso, con ogni salvataggio che pubblica e le pianificazioni nel kit. *Fix:* una frase in
  FR-5 che nomini i due costi come accettati; una conseguenza: un salvataggio che cambia lo schema
  di input avvisa delle pianificazioni e delle pipeline stream che usano quel workflow.
- **medium** Allargamento della v1 non registrato nei rischi (NFR-16 L1248–1250; FR-40 L962;
  FR-57; FR-58; §10.5 L1375–1378) — Oggi il kit cresce in quattro punti:
  - l'editor disegna «ogni altro costrutto di FR-1» (`try`, `switch`, `fork`, `for`, `wait`,
    `raise`, sotto-workflow);
  - l'area workflow con l'elenco dei workflow;
  - le pianificazioni nel kit;
  - l'uso «per intero» da telefono, «compresa la composizione nell'editor del grafo» e nell'editor
    SQL, a 360 px e con input touch.

  Il piano di modifica lo dice («allarga la v1», `change-plan-ux.md` T-71), il PRD no: §10.5
  elenca ancora i pezzi di ieri, con «Decisione: nessun taglio». Non contesto la decisione di
  Marco. Manca il costo scritto dove lo legge chi decide. Nessun percorso utente si svolge su un
  telefono, quindi NFR-16 non ha un motivo scritto nel documento. *Fix:* in §10.5 una riga che
  nomini NFR-16, FR-40 (tutti i costrutti), FR-57 e FR-58 come allargamento accettato; in NFR-16 il
  caso che lo motiva, per esempio piattaforme ospiti usate da telefono.
- **low** Lo schema di input forse c'è già in OWS (FR-56 Note L406; §11 punto 7 L1393–1394) —
  L'addendum del brief elenca `input`/`output`/`export` tra i dati di OWS. Verificare se l'`input`
  di OWS 1.1 porta già uno schema, a livello di workflow e di task. Se sì, la parte di §11.7 su
  FR-56 si riduce a «si usa quello», e lo stesso vale per lo schema di input dei task preso dal
  manifest. *Fix:* verificarlo sul DSL prima dell'architettura e scrivere l'esito nella nota di
  FR-56.

## Substance over theater — strong

UJ-4 e UJ-5 ora hanno stato iniziale, esito e caso limite, e puntano a FR-45 e FR-48: guidano
requisiti invece di decorare. SM-6 ha un oggetto preciso: il workflow di UJ-2, almeno 5 task, con
fan-out e join. NFR-16 ha una soglia misurabile (360 px, input touch). NFR-9 ha una divisione di
responsabilità verificabile: AA con i colori predefiniti, segnalazione dei contrasti dell'ospite
(FR-44). L'elenco chiuso degli stati è sostanza: toglie la deriva tra «in attesa» e «in coda»
segnalata nella riconciliazione con la UX. Nelle parti nuove non c'è arredo.

## Strategic coherence — strong

Le aggiunte servono tutte la prima differenza di §1.1, il kit incorporabile con un editor no-code.
- Manifest e connessioni portano nell'editor ciò che i worker sanno fare, senza codice per ogni
  cliente: è il secondo JTBD dell'integratore.
- Schema di input e modulo generato (FR-41) servono la cliente di UJ-2.
- Area workflow e pianificazioni completano il kit.
- SM-6 misura la promessa no-code, cosa che ieri mancava.

Nessuna aggiunta esce dalla tesi; il costo dell'allargamento sta in Decision-readiness.

## Done-ness clarity — adequate

Le conseguenze nuove sono quasi tutte verificabili:
- FR-46 definisce il tempo in coda come «l'età dell'attività più vecchia in coda»;
- FR-44 ha un controllo automatico sul bundle;
- FR-41 e FR-43 dicono che nessun testo nomina il motore o il crash;
- FR-29 dice cosa contiene il messaggio di rifiuto;
- FR-56 ha tre casi chiari.

Cedono quattro punti: il flusso dei dati nell'editor, il controllo del manifest durante
l'esecuzione, una definizione di stato e il criterio di riuscita di SM-6.

### Findings

- **medium** Le fonti dei dati in FR-40 restano indietro rispetto al mock approvato (FR-40
  L968–971; FR-3; FR-56; UJ-2 L163–164; SM-6 L1347–1349) — Le fonti sono un elenco chiuso: un
  valore fisso, un «campo compatibile dello schema di output di un task a monte», oppure
  un'espressione jq come modo avanzato. In UJ-2 le verifiche DNS e Censys ricevono un indicatore
  alla volta, cioè l'elemento corrente del fan-out, che non è l'output di un task a monte.
  - Il mock approvato dell'editor da desktop (`mockups/key-editor-desktop.html`) risolve il caso: il
    campo `indicatore` di `censys_search` prende «Per ogni indicatore · elemento» con la fonte «Da
    un task». Il mock da telefono mappa invece «Ricevi elenco · indicatore».
  - FR-40 e la spina (`data-mapping`, EXPERIENCE L260) non descrivono quello che il mock da desktop
    mostra.
  - Un campo dell'input del workflow (FR-56) come fonte per un task che non sia il primo non
    compare nel PRD, nella spina né nei mock dell'editor: oggi ci si arriva solo con jq.

  Chi scrive la storia dell'editor legge FR-40 e la spina. SM-6 chiede di comporre il workflow di
  UJ-2 «senza leggere documentazione», e regge solo se la fonte del mock entra nel testo. *Fix:*
  aggiungere alle fonti di FR-40 «l'elemento corrente del fan-out (FR-3)» e «un campo dell'input
  del workflow (FR-56)», con una conseguenza verificabile: il workflow di UJ-2 si compone senza
  scrivere jq. Allineare `data-mapping` nella spina e i due mock.
- **medium** Il PRD non dice cosa succede durante l'esecuzione quando un valore non rispetta il
  manifest (FR-4 L375–379; FR-40 L968–976; FR-55 L532–543) — All'invio FR-4 controlla tipi e campi
  obbligatori, e nel workflow di un cliente le «connessioni senza un segreto sono errori». Due casi
  sfuggono a quel controllo:
  - l'output reale di un worker che non rispetta lo schema di output dichiarato, su cui FR-40 basa
    il «campo compatibile»;
  - un valore prodotto da jq, il cui tipo non si conosce prima dell'esecuzione.

  Nessun FR dice se il motore valida input e output delle attività sugli schemi del manifest. È un
  costo per ogni attività, che pesa su NFR-2 e NFR-3: l'architettura ha bisogno della regola.
  Mancano anche il linguaggio degli schemi e la regola di «compatibile», da cui dipende la parità
  tra editor, nucleo JS e motore (FR-39, FR-40, FR-4). *Fix:*
  1. In FR-55 una conseguenza: un'attività il cui input non rispetta lo schema di input fallisce
     con percorso e motivo, senza essere consegnata; l'output è (o non è) validato sullo schema di
     output.
  2. In §11 una voce per l'architettura: linguaggio degli schemi e regola di compatibilità.
- **medium** L'esecuzione «in coda» è definita sulle attività, che non tutti i task producono (§3
  Stati L286–287; §3 Attività L243; FR-1 L313–315; FR-46 L1069–1071; UJ-4 L197–198) — Un'esecuzione
  è in coda «quando è accettata e nessuna sua attività è ancora consegnata». Un'attività è un'«unità
  di lavoro consegnata a un worker o alla sandbox». `wait`, `set` e `switch` non consegnano nulla
  a un worker o alla sandbox (`call http` neppure, se l'architettura lo esegue nel motore o in un
  processo ausiliario, FR-50). La definizione dà due risultati sbagliati:
  - un'esecuzione che comincia con un `wait` di un'ora compare per un'ora nel filtro «in coda» di
    FR-46, tra le «esecuzioni ferme in» coda che UJ-4 chiede di individuare;
  - un'esecuzione fatta solo di task di questo tipo resta «in coda» fino alla fine e passa a
    «completata» senza essere mai stata «in esecuzione».

  È un elenco chiuso che diventerà un enum. *Fix:* «in coda finché nessun suo task è partito
  (attività consegnata, oppure task eseguito dal motore e già iniziato)». Il caso di R-1, il primo
  task fermo senza worker, resta in coda.
- **medium** SM-6 senza criterio di riuscita (§9 SM-6 L1347–1349) — È la prima metrica sul kit, e
  riduce il rilievo medio di ieri sulla differenza numero 1. Così com'è non si sa quando è
  rispettata. Mancano:
  - un tempo massimo;
  - se il facilitatore può aiutare;
  - lo schermo (con NFR-16 anche il telefono è un caso legittimo);
  - cosa trovano già pronto: funzioni registrate, nodo del punteggio caricato, segreto Censys da
    creare o già presente;
  - se basta una persona su tre.

  *Fix:* per esempio «ciascuna delle tre, da sola, su desktop, entro 20 minuti, con funzioni e nodo
  già disponibili e il segreto da creare; 3 su 3».
- **low** «altri metadati» in un formato pubblico (FR-55 L536; §6 L1257) — Il manifest è una
  superficie pubblica con semver (§6), ma l'elenco dei suoi campi finisce con «altri metadati».
  *Fix:* nel PRD dire che l'elenco è chiuso, più un campo di estensione con un nome riservato; la
  forma la decide l'architettura.
- **low** NFR-9 ha un perimetro ma non un metodo (NFR-9 L1214–1218) — Ora vale per tutti i
  componenti del kit e distingue i colori predefiniti da quelli dell'ospite: chiude metà del
  rilievo basso di ieri. Manca come si verifica l'AA, e «la modifica resta sul canvas» è il punto
  più a rischio. La spina UX prevede già una prova con NVDA e VoiceOver che comprende la
  costruzione del workflow di UJ-2 (EXPERIENCE, Accessibility Floor). *Fix:* in NFR-9 «verifica:
  controllo automatico su ogni componente, più una prova con NVDA e VoiceOver sui percorsi UJ-2 e
  UJ-4».
- **low** Manifest e schema di input senza limiti (NFR-14 L1235–1239; FR-55; FR-56) — Il manifest
  di un nodo personalizzato e lo schema di input di un workflow sono grandezze nuove che il cliente
  controlla. L'elenco di NFR-14 non li comprende: dimensione, profondità, riferimenti ricorsivi.
  *Fix:* aggiungerli all'elenco.

## Scope honesty — adequate

Nessuna delle otto assunzioni tolte è sparita in silenzio.
- Sei hanno una conferma di Marco nel memlog: FR-3, FR-5, FR-10, FR-41, FR-44, NFR-9.
- FR-6 (pianificazioni nella v1) è superata dalla decisione R-5 sulle pianificazioni nel kit.
- UJ-5 perde il tag perché i 15 minuti restano un'assunzione in NFR-13, a cui UJ-5 rimanda.

Altri numeri:
- i tag inline scendono da 30 a 22;
- §11 passa da 9 a 15 voci;
- il materiale per l'architettura sta nell'addendum («Dopo la UX»).

Per un PRD che entra in architettura questa densità va bene. Il problema è il livello a cui vanno
due domande (rilievo high di Decision-readiness), più un'omissione che l'elenco dei workflow ora
rende visibile.

### Findings

- **medium** Un workflow non si può eliminare (FR-58 L1048; FR-23 L708; FR-42 L997; FR-57 L1028)
  — Dall'elenco di FR-58 il cliente «apre un workflow o ne crea uno nuovo». Pipeline stream,
  segreti e pianificazioni si eliminano. Il workflow no: né dal kit né dall'SDK, e §8.2 non lo
  esclude. Ieri era un rilievo basso tra le omissioni silenziose; con l'elenco dei workflow nel kit
  diventa un buco visibile. L'eliminazione tocca:
  - le pianificazioni (FR-57);
  - le destinazioni delle pipeline stream (FR-24);
  - i sotto-workflow che lo citano (FR-26);
  - le esecuzioni in corso e lo storico (FR-31).

  *Fix:* un FR, o una conseguenza di FR-58, per eliminare (o ritirare) un workflow, con gli effetti
  su pianificazioni, destinazioni e riferimenti; in alternativa una voce esplicita in §8.2.

## Downstream usability — adequate

Manifest, Connessione e Stati entrano nel glossario. Chi scrive l'architettura e le epiche ha ora un
elenco chiuso degli stati per oggetto e un solo termine per ciò di cui una funzione ha bisogno. I
rimandi nuovi risolvono: FR-57 → FR-38, FR-58 → FR-26, FR-29 e FR-44, NFR-9 → FR-44, FR-41 e
FR-43 → NFR-15, FR-48 → NFR-6. Due rimandi sono deboli (Mechanical notes). Restano due
contraddizioni tra FR.

### Findings

- **medium** FR-1 e FR-57 dicono cose diverse sulla pianificazione (FR-1 L325–326; FR-57 L1032;
  addendum «Dopo la UX», voce Pianificazioni) — FR-1 mette fuori perimetro solo «pianificazioni
  `after` e `on`». Quindi `schedule` con `every` e `cron` resta nel sottoinsieme, dentro il
  documento OWS: l'addendum del brief elenca `every`/`cron`/`after`/`on`. FR-57 invece dice che una
  pianificazione è un trigger del cliente, «non una parte del workflow», una per workflow.

  Esempio: un workflow dell'integratore definito come codice, con `schedule.cron` (FR-35, FR-36).
  Per quale cliente parte, visto che FR-26 vuole un cliente per ogni trigger? L'addendum passa
  «dove vive la pianificazione» all'architettura, ma la contraddizione sta nel testo dei FR, e
  §11 non la elenca. *Fix:* in FR-1 mettere `schedule` fuori perimetro (rifiutato all'invio, con
  un rimando a FR-57), oppure dire come il motore lo traduce; in alternativa una voce di §11 per
  l'architettura.
- **medium** L'identità di una connessione non è definita, ma §11.6 dice chiuso il lato di prodotto
  (§3 L271–274; FR-55 L535; FR-32 L861–868; FR-29 L815–817; §11 punto 6 L1391–1392) — Una
  connessione è «piattaforma esterna e tipo di credenziale». Nessuno dice chi dà il nome alla
  piattaforma, né quando due connessioni sono la stessa.
  - Se ogni manifest scrive la sua stringa, due worker che usano Censys con nomi diversi chiedono
    due segreti con lo stesso valore, e il selettore di FR-32 non li fa coincidere.
  - Un nodo personalizzato del cliente può dichiarare qualunque piattaforma.

  È una scelta di prodotto: elenco delle piattaforme tenuto dall'integratore, oppure stringhe
  libere con confronto esatto. Da lei dipendono il selettore del kit, la risoluzione di FR-29 e la
  validazione di FR-4. *Fix:* una conseguenza in FR-55 su chi definisce gli identificativi delle
  connessioni e su come si confrontano; altrimenti riaprire §11.6 su questo punto.
- **low** Workflow dell'integratore «a lui disponibili» o «disponibile a tutti i clienti»? (FR-29
  L809; FR-28 L798; FR-57 L1035; FR-58 L1046, L1049) — FR-29 dice che un workflow dell'integratore
  è disponibile a tutti i clienti. I FR nuovi dicono «a lui disponibili», che fa pensare a una
  scelta per cliente. Se la scelta esiste, manca in FR-38 chi la configura; se non esiste, la
  precisazione è di troppo. *Fix:* allineare a FR-29, oppure aggiungere la configurazione.

## Shape fit — strong

Resta un prodotto per sviluppatori in cima alla catena (UX, architettura, storie). UJ-4 e UJ-5 hanno
ora lo schema completo, NFR-16 e FR-58 sono nella forma giusta per un kit da incorporare, e nulla è
sovra-formalizzato.

### Findings

- **low** Indicazioni di layout nel PRD (FR-40 L967; FR-55 L541–542; FR-54 L624) — «nel pannello
  del workflow», «ordina la palette dell'editor» e «nella pagina dei worker» dicono dove sta una
  cosa nell'interfaccia, non cosa deve fare. Il dettaglio dell'interfaccia vive nelle spine UX: se
  la UX sposta un pannello, il PRD resta indietro. *Fix:* scrivere il risultato. Per esempio: «il
  cliente definisce lo schema di input dall'editor», «il gruppo raggruppa le voci nell'editor»,
  «un avviso all'operatore nella dashboard».

## Mechanical notes

- **Termini usati in modo diverso dopo l'elenco chiuso degli stati.**
  - NFR-14 L1234 scrive «in attesa senza fine» per esecuzioni e attività, che non hanno quello
    stato.
  - «oltre quota» (FR-8 L450, FR-30 L830, FR-53 L1160) convive con gli stati «vicino al limite» e
    «al limite». UJ-4 L204 li fa coincidere: un cliente che supera la quota «risulta al limite».
  - «Pianificazione: sospesa (FR-6).» (L298) ha un solo stato: manca «attiva».
  - L'attività «ritentata» convive con «ogni tentativo è un'attività distinta» (FR-7 L437–438).
    Va detto che «ritentata» è l'esito di un tentativo seguito da un altro.
  - «Connessione» nel senso di rete resta in NFR-8 L1203 e in FR-50 L1127. In NFR-8 «Ogni
    connessione che porta segreti» ora si legge nel senso del glossario: scrivere «connessione di
    rete».
  - «nome visualizzato» (della persona, FR-27 L778) e «nome visualizzabile» (del cliente, FR-38
    L938) non sono nel glossario e porteranno allo stesso identificativo inglese.
- **ID.** FR-55, FR-56, FR-57 e FR-58 stanno fuori dall'ordine numerico: FR-56 a L395 dopo FR-5,
  FR-55 a L532 dopo FR-14, FR-57 e FR-58 a L1026 e L1039 dopo FR-44. È coerente con la numerazione
  stabile di §0, ma chi estrae i FR nell'ordine del documento trova FR-5, poi FR-56, poi FR-6. Ieri
  il caso riguardava solo FR-54. Nessuno dei quattro dice «Realizza UJ-n»: FR-56 e FR-58 servono
  UJ-2, FR-55 serve UJ-1 e UJ-2.
- **Rimandi.** I rimandi nuovi verificati risolvono, con tre debolezze.
  - FR-38 L938 rimanda a FR-28 per il kit che mostra e «cerca» il cliente, ma FR-28 non parla di
    ricerca.
  - FR-49 L1098 rimanda a NFR-14 per la soglia di «vicino al limite», che l'elenco di NFR-14
    («Vale almeno per») non nomina.
  - §4.9 L945–946 mette le pianificazioni nell'area workflow, ma FR-58 L1042 le omette dall'elenco
    delle sezioni.
- **Indice delle assunzioni.** Il round trip torna: 22 tag inline e 18 voci, perché la voce di
  flowtoast copre cinque punti del testo. Le otto voci tolte hanno perso anche il tag inline. NFR-13
  resta prima di NFR-10, come ieri.
- **Rilievi di ieri chiusi o ridotti dall'aggiornamento.**
  - UJ-4 e UJ-5 hanno lo schema completo: chiuso il rilievo medio di Downstream usability (UJ-3
    resta, accettata).
  - FR-5 dice che salvare è pubblicare: chiuso il rilievo basso sulla bozza.
  - FR-48 e FR-49 hanno conseguenze; FR-47 ancora no.
  - NFR-9 ha un perimetro; il metodo manca ancora.
  - SM-6 riduce il rilievo medio sulla differenza numero 1, ma non misura ancora un secondo
    integratore.
- **Frontmatter.** `status: draft` e `updated: 2026-11-20` sono coerenti con un aggiornamento in
  corso.
