---
title: topictoast — riconciliazione addendum del brief con il PRD
created: 2026-11-19
---

# Riconciliazione: addendum del brief contro il PRD

Input: `briefs/brief-topictoast-2026-11-18/addendum.md`. Confrontato con `prd.md` e `addendum.md` del PRD.
Sono riportati solo i contenuti a livello di requisito che il PRD omette o contraddice. Le differenze
dichiarate nel PRD §1.2 non sono lacune. Le citazioni sono dall'addendum del brief, con la sezione.

## Già coperto (nessuna azione)

- Isolamento tra clienti su dati, esecuzioni, storico e segreti: FR-26, FR-31.
- Quote per cliente su task, concorrenza e sandbox: FR-30, FR-25.
- Sandbox che non raggiunge la rete interna né i dati degli altri clienti: FR-16, FR-17, NFR-8.
- Segreti cifrati a riposo, consegnati solo all'attività che li usa, mai nei log: FR-33, FR-34, FR-52.
- La sandbox la esegue topictoast: §4.4.
- Database al minimo: NFR-4, SM-2. Iggy come bus: §4.11 (ma vedi G-6).
- Log append-only di Iggy e conseguenze (ack, retry, timeout a carico di topictoast): rischio 3.
- CTK come SHOULD: FR-1 «Fuori perimetro».

## Lacune e contraddizioni

### G-1. FR-50 decide di fatto la sandbox, e FR-15 chiede la compilazione di Rust (severità: alta)

- **Addendum, §6 «Compromessi che guidano la scelta»:** «Solo wasmtime non chiede né KVM né root, quindi
  è l'unico che rispetta "binario unico, facile da installare"». Inoltre: «Wasm limita ai linguaggi
  compilabili in Wasm e condivide il processo: un bug del runtime è una fuga». E, tra i candidati,
  nsjail «servono user namespace, a volte root»; Firecracker e Hyperlight «sì (KVM)».
- **PRD:** FR-50 impone «non richiede Kubernetes, KVM né privilegi di root» (assunzione). §4.4 e il rischio 1
  dicono che la tecnologia la sceglie uno spike. FR-15 dice che un nodo Rust «che non compila» viene
  rifiutato «con l'errore del compilatore».
- **Problema:** insieme, FR-50 e il criterio di installazione escludono Firecracker, Hyperlight e (spesso) nsjail,
  quindi lo spike ha già un solo vincitore e il PRD non lo dice. Due conseguenze restano senza requisito:
  (a) l'isolamento in-process di Wasm contro il vincolo di NFR-8 («codice ostile accanto ai segreti di tutti»):
  serve dire se la sandbox deve stare fuori dal processo che tiene la chiave della cassaforte;
  (b) compilare Rust (o TypeScript) sul server richiede una toolchain installata, che è un requisito di
  installazione non scritto e che pesa su NFR-1 e FR-50.
- **Dove:** §4.4 (descrizione e FR-15, FR-16), FR-50, §10 rischio 1.
- **Fix in una riga:** dichiarare FR-50 «nessun KVM/root» come criterio di selezione della sandbox e dire
  esplicitamente se la sandbox gira in un processo separato e se il server compila i nodi o li riceve già compilati.

### G-2. Iggy: formato su disco instabile contro aggiornamenti e durabilità; head-of-line contro isolamento (severità: alta)

- **Addendum, §2 «Apache Iggy»:** «La 0.10.0 rifiuta i client precedenti e cambia il formato su disco senza
  migrazione dalla 0.9.x». Inoltre «con un task lento in una partizione, quelli dopo aspettano» e «Nessun ack
  per singolo messaggio, riconsegna, timeout di visibilità o messaggi ritardati».
- **PRD:** FR-51 «L'aggiornamento migra lo stato da solo; le esecuzioni in corso riprendono dopo il riavvio»;
  §6 «un workflow accettato da una versione resta eseguibile»; FR-9 e NFR-6 (nessuna perdita). FR-13 «Un worker lento
  non rallenta le attività consegnate ad altri worker»; FR-30 «gli altri clienti non rallentano». Il rischio 3
  nomina il problema ma nessun requisito lo traduce.
- **Problema:** se il bus è pre-1.0 e perde i dati su aggiornamento, FR-51, FR-9 e NFR-6 non sono garantiti per
  l'aggiornamento del bus. FR-13 e FR-30 dipendono da come le attività si mappano sulle partizioni.
- **Dove:** FR-51, FR-13, FR-30, rischio 3.
- **Fix in una riga:** aggiungere a FR-51 che l'aggiornamento del bus non perde attività accettate (il motore possiede
  la fonte di verità o una procedura di drenaggio) e annotare in FR-13/FR-30 che le garanzie di isolamento valgono anche
  a livello di partizione.

### G-3. Rinnovo OAuth tolto dai requisiti di sicurezza senza essere in §1.2 (severità: media)

- **Addendum, §7:** tra i requisiti «Ne derivano almeno»: «rinnovo dei token OAuth fatto dal motore». §5, conclusione:
  «nessuno offre le quattro cose della tabella insieme» (separazione clienti, segreti, OAuth, sandbox).
- **PRD:** §8.2 rimanda OAuth a dopo la v1 («la v1 parte con le chiavi API»); non compare tra le sei differenze di §1.2.
  §1.1 punto 4 dice che sandbox, separazione e cassaforte non esistono «insieme» altrove.
- **Problema:** la differenza è reale ma non dichiarata. Con tre su quattro, Windmill e Activepieces (che hanno già
  sandbox, segreti per cliente e rinnovo OAuth, §5) sono più vicini: il punto 4 di §1.1 si indebolisce.
- **Dove:** §1.2 (nuovo punto 7), §1.1 punto 4, §8.2.
- **Fix in una riga:** aggiungere in §1.2 «OAuth dopo la v1» con motivo e riformulare §1.1 punto 4 senza lasciare
  intendere la copertura delle quattro cose.

### G-4. RustFS e Iggy declassati da vincolo a preferenza senza essere in §1.2 (severità: media)

- **Addendum, §8:** «Vincoli tecnici indicati dall'utente»: Iggy per passare le informazioni tra i nodi; «RustFS
  (storage a oggetti compatibile S3) per i messaggi troppo grandi e per i file».
- **PRD:** §4.11 «La preferenza di Marco, da confermare in architettura, è topictoast più il bus Iggy ... RustFS non serve
  nella v1»; §8.2 sposta lo storage a dopo la v1; FR-11 fissa 1 MB. Non è in §1.2. FR-50 («al massimo due processi più
  il database») non lascia posto a RustFS come terzo processo.
- **Problema:** il brief li presenta come vincoli, il PRD come preferenza. Quando arriverà RustFS, FR-50 andrà
  in contraddizione. Il PRD non ha nessun requisito sui file (solo payload).
- **Dove:** §1.2, §4.11, FR-11, FR-50, §8.2.
- **Fix in una riga:** elencare in §1.2 «Iggy resta vincolo, RustFS dopo la v1», e in FR-50 precisare che il limite di due
  processi vale per la v1.

### G-5. Obiettivi della sandbox mancanti: avvio, memoria, linguaggi (severità: media)

- **Addendum, §6:** criteri «tempo di avvio per task, linguaggi in cui i clienti scrivono i nodi ... memoria per
  sandbox». «Linguaggi dei nodi personalizzati voluti dall'utente: Python, TypeScript, Rust, Go». Avvio: Firecracker «fino
  a 125 ms», wasmtime «microsecondi», nsjail «numeri non documentati».
- **PRD:** NFR-2 (p50 5 ms, p99 25 ms) misura solo attività verso worker sulla stessa macchina, quindi non copre i nodi
  personalizzati. Il tempo di avvio della sandbox è solo la domanda aperta 3; nessun limite di memoria per sandbox
  (FR-17 lo cita senza valore). Python e Go sono rimandati in §8.2, senza essere in §1.2. Il limite delle librerie native
  in Wasm («da verificare per linguaggio») non è citato.
- **Problema:** UJ-2 usa un nodo personalizzato in un fan-out da 300 rami. Senza un obiettivo di avvio e di memoria per
  sandbox non si può dire se la scelta regge, né dimensionare le quote.
- **Dove:** §5 (nuovo NFR sandbox), FR-17, §1.2, §11 domanda 3.
- **Fix in una riga:** aggiungere un NFR su avvio e memoria di una sandbox (anche come `[ASSUMPTION]`) e annotare in
  §1.2 il rinvio di Python e Go e il limite sulle librerie native.

### G-6. Pretese di conformità OWS e mappatura dei task (severità: media)

- **Addendum, §1 «Task obbligatori per un runtime conforme»:** «`run` (container, script, shell, workflow) e `call`
  (HTTP, gRPC, OpenAPI, AsyncAPI, A2A)»; «espressioni jq obbligatorie per ogni runtime; meccanismo dei segreti
  obbligatorio; schedule `every`/`cron`/`after`/`on`; eventi CloudEvents (`emit`/`listen`)».
- **PRD:** FR-1 accetta un sottoinsieme senza `run` container/script/shell, senza gRPC/OpenAPI/AsyncAPI/A2A, senza
  `emit`/`listen`, senza schedule `after`/`on`. Il PRD dice ovunque «documento OWS 1.1» e «motore OWS». «Invocazione di
  funzioni e nodi personalizzati» in FR-1 non corrisponde a un task OWS standard. Non è detto come i segreti dichiarati
  da un task (FR-33) si mappano al meccanismo dei segreti di OWS.
- **Problema:** un runtime che non implementa i task obbligatori non è «conforme», quindi il PRD non deve dire «OWS
  conforme». Manca anche dove stanno le chiamate a funzioni e nodi nel documento.
- **Dove:** §1, §3 (Documento OWS), FR-1, FR-33.
- **Fix in una riga:** scrivere in FR-1 «sottoinsieme, non runtime conforme» e dire quale costrutto OWS rappresenta una funzione
  o un nodo personalizzato (per esempio `call` con funzione personalizzata) e come si mappano i segreti.

### G-7. Validità OWS dell'estensione DAG e portabilità (severità: media)

- **Addendum, §1 «Divergere dalla spec»:** «Una chiave nuova come `dependsOn` su un task ... rende il documento non
  valido per qualsiasi validatore OWS: serve uno schema nostro». Con `metadata` resta valido. «Ipotesi da verificare: un
  altro runtime ignora `metadata` ed esegue il `do` in sequenza» (identico solo se in ordine topologico e dati via
  `$context`).
- **PRD:** FR-40 «Il workflow salvato dall'editor è un documento OWS valido con l'estensione DAG»; FR-35 «documento OWS
  equivalente»; §1 «I workflow che usano l'estensione DAG non girano su altri runtime» (affermato come fatto).
- **Problema:** FR-40 è vero solo se l'architettura sceglie `metadata`; con chiavi nuove è falso. La frase su altri
  runtime è un'ipotesi non verificata, non un fatto. FR-39 (stessi esiti di validazione tra nucleo JS e motore) richiede
  inoltre uno schema nostro, non il validatore OWS.
- **Dove:** §1.1 (nota OWS), FR-35, FR-40, FR-39, §10 rischio 4.
- **Fix in una riga:** riformulare FR-40/FR-35 come «valido secondo lo schema topictoast» e passare «non girano su altri
  runtime» a «non garantito».

### G-8. Cicli: dipendenze e `then` (severità: bassa)

- **Addendum, §1 tabella:** `for` con `while`; «`then` verso un task precedente dello stesso blocco non è vietato».
- **PRD:** FR-2 rifiuta ogni ciclo nelle dipendenze; FR-1 include `for` e `switch`, non dice cosa succede a un salto `then`
  all'indietro.
- **Fix in una riga:** in FR-2 chiarire che il divieto vale per le dipendenze dichiarate, non per i cicli `for`/`then` di
  OWS, o rifiutare `then` all'indietro in FR-1.

### G-9. Versione del DSL accettata (severità: bassa)

- **Addendum, §1 «Versione»:** «v1.1.0 stabile dal 2026-11-12»; «Gli esempi del sito dichiarano ancora `dsl: '1.0.3'`»;
  politica di versione non trovata; SDK Rust «non risulta aggiornato alla 1.1.0».
- **PRD:** FR-1 dice solo «OWS 1.1».
- **Fix in una riga:** aggiungere a FR-1 quali valori di `document.dsl` si accettano (1.1.x; 1.0.x sì o no).

### G-10. Pretesa «nessun motore open distribuisce componenti» (severità: bassa)

- **Addendum, §4:** Conductor ha `orkes-workflow-visualizer` (componente React con diagramma ed esecuzione, Apache-2.0
  da verificare); Kestra `@kestra-io/topology`. «"Non esiste niente" non regge. Formulazione difendibile: "kit incorporabile
  nativo del motore, licenza permissiva, DAG al centro"».
- **PRD:** §1.1 punto 1: «Nessun motore open distribuisce componenti per costruire workflow e seguirne le esecuzioni dentro
  un'altra app».
- **Fix in una riga:** sostituire con la formulazione difendibile dell'addendum.

### G-11. Il valore di 7.700 istruzioni SQL è la misura di un singolo utente (severità: bassa)

- **Addendum, §3:** «un utente ha misurato circa 7.700 istruzioni SQL ... Il maintainer non lo considera un bug».
- **PRD:** §1, NFR-4 e SM-2 lo usano come riferimento di Hatchet senza dire che è una misura di un utente.
- **Fix in una riga:** in NFR-4 e SM-2 scrivere «misura di un utente (hatchet-dev/hatchet#5153)» per il benchmark pubblico.

### G-12. Custodia della chiave di cifratura (severità: bassa)

- **Addendum, §7:** «una sandbox da cui il codice di un cliente non raggiunge i segreti» e «segreti cifrati a riposo»;
  §6: nsjail «condivide il kernel con la macchina che tiene i segreti di tutti».
- **PRD:** FR-16 vieta solo i segreti «di altri clienti»; FR-34 parla di «chiave di cifratura» senza dire dove sta.
- **Fix in una riga:** in FR-16/FR-34 aggiungere che la sandbox non raggiunge né la chiave della cassaforte né i segreti non
  dichiarati dal task.
