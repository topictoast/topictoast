---
title: topictoast — PRD
status: final
created: 2026-11-19
updated: 2026-11-21
---

# PRD: topictoast

## 0. Scopo del documento

Questo PRD dice cosa deve fare la prima versione (v1) di topictoast e come si misura se ci riesce.
Lo leggono Marco, che ne è autore e primo integratore, e i passi successivi del metodo: UX,
architettura, epiche e storie. Parte dal brief del 2026-11-18
(`_bmad-output/planning-artifacts/briefs/brief-topictoast-2026-11-18/`) e non ne ripete le ricerche:
il confronto con i concorrenti, la ricerca su OWS, Iggy e sandbox restano nell'addendum del brief.
Le ricerche fatte per questo PRD sono in `research-stream.md` e `research-footprint.md`, il
materiale tecnico emerso qui è in `addendum.md`.

Come è fatto: i termini del §3 sono vincolanti per tutto il documento e per i documenti che
seguono. Le funzionalità (§4) hanno i requisiti funzionali annidati, numerati FR-1…FR-N in modo
stabile. I requisiti non funzionali trasversali stanno nel §5. Dove ho dedotto senza conferma c'è
il tag `[ASSUMPTION]`; tutte le assunzioni sono raccolte nel §12.

## 1. Visione

topictoast è un motore di workflow open source (Apache-2.0), scritto in Rust, fatto per essere
incorporato in altre piattaforme. Chi lo integra offre ai propri tenant un editor no-code per
comporre da sé i workflow, eseguiti come DAG, in parallelo anche su elenchi di dimensione variabile,
e pipeline SQL continue sugli stream di eventi per i carichi ad alta intensità. I workflow sono
documenti Open Workflow Specification (OWS) con un'estensione per i DAG completi.

Oggi chi vuole offrire workflow configurabili ai propri tenant deve mettere insieme tre pezzi che
nessun progetto open fornisce insieme: un motore, un'interfaccia no-code da incorporare e la
separazione tra tenant. Il riferimento di Marco, Hatchet, offre due strade: Postgres come coda,
con circa 7.700 istruzioni SQL e 600 KB di WAL per task secondo la misura di un utente (5,2
milioni di istruzioni al giorno con circa 680 task, hatchet-dev/hatchet#5153), oppure
RabbitMQ, il default, che per Marco è fermo alla 4.2.9 perché la 4.4 rifiuta le code di Hatchet
(#5501). topictoast porta il carico fuori dal database: le attività viaggiano su un bus di messaggi
e il database tiene solo lo stato. L'obiettivo è un motore ad alte prestazioni con il minimo
consumo di RAM, e i numeri del §5 lo rendono verificabile.

Fra due o tre anni topictoast è incorporato in più piattaforme, non solo in quella di Marco. Sopra
può nascere un prodotto separato, con nodi pronti (cicli, trasformazioni, connettori, LLM) e utenti
in stile n8n, senza snaturare il motore `[ASSUMPTION: progetto separato, nome di lavoro
flowtoast]`. Le stesse funzioni (DAG, sandbox, cassaforte) servono anche a costruire agenti.
L'estensione DAG, una volta provata, può essere proposta alla specifica OWS (issue #1143).

### 1.1 Cosa lo distingue

In ordine di importanza per Marco:

1. **Kit frontend incorporabile, nativo del motore, con licenza permissiva e DAG al centro.**
   Esistono componenti di visualizzazione legati al proprio motore (Conductor, Kestra) e un editor
   React che gira solo su Temporal (Workflow Builder); topictoast offre il kit per costruire
   workflow e seguirne le esecuzioni dentro un'altra app. È un vantaggio di esecuzione, non una
   barriera: un kit si può copiare.
2. **Database al minimo.** È la differenza più misurabile contro Hatchet con Postgres come coda, e
   va dimostrata con un benchmark pubblico (SM-2).
3. **Velocità, poca memoria, alta scalabilità**, con i numeri del §5: Rust da solo non convince chi
   sceglie. Nella v1 si scala aggiungendo worker; il motore è a nodo singolo.
4. **Separazione tra tenant con sandbox e cassaforte, nel nucleo open con licenza permissiva.** I
   pezzi esistono altrove (Windmill con AGPL, Activepieces con parti a pagamento), ma non insieme e
   non gratis. Quei due hanno anche il rinnovo dei token OAuth, che in topictoast arriva dopo la
   v1: la v1 dimostra separazione, sandbox e cassaforte con chiavi API.
5. **Workflow DAG e pipeline SQL continue nello stesso motore.** Tra i motori di workflow
   controllati (Kestra, Windmill, n8n, Temporal, Hatchet) ognuno avvia un'esecuzione per ogni
   evento; nessuno offre SQL continuo con finestre sugli stream. Non sono stati controllati
   Airflow, Prefect, Dagster, Argo e Conductor. Alcuni prodotti orientati ai dati (TDengine,
   Databend Cloud Task Flow, Databricks Lakeflow) uniscono già SQL sugli stream e avvio di flussi.

**Cosa non lo distingue.** DAG, YAML e database esistono già (Kestra, Windmill, Argo); esiste già
anche il binario unico più database (Hatchet, Windmill, Restate). Con SQLite topictoast si
installa con due servizi, come Hatchet con solo Postgres: la leggerezza si dimostra con RAM e
carico sul database (NFR-1, NFR-4), non con il numero di componenti.

**Scelta di progetto, non differenza: OWS.** Un formato documentato e già pronto, niente linguaggio
da progettare. topictoast ne implementa un sottoinsieme e non è un runtime OWS conforme (FR-1). Non
è garantito che i workflow con l'estensione DAG girino su altri runtime.

### 1.2 Differenze dal brief

Il brief del 2026-11-18 resta com'è. Questo PRD se ne discosta in nove punti:

1. **Stream processor SQL nella v1**, con stato, finestre, watermark e checkpoint, sul modello di
   Arroyo. Nel brief non c'era.
2. **Le pipeline stream le scrivono anche i tenant**, dall'editor, con isolamento e quote.
3. **Obiettivi numerici fissati** (§5); il brief li rimandava al PRD.
4. **Nodi pronti e utenti in stile n8n fuori da topictoast**, in un possibile progetto separato
   `[ASSUMPTION]`. Nel brief erano "dopo la v1".
5. **Worker come funzioni sempre attive** in attesa di lavoro: precisazione, non cambio.
6. **Critica a Hatchet nella versione verificata** dell'addendum: Postgres come broker è opzionale,
   RabbitMQ è il default.
7. **Nodi personalizzati precompilati**, caricati tramite SDK o CLI: l'editor li usa ma non ne
   ospita il codice, e il server non ha compilatori. Il brief non lo diceva.
8. **Sandbox su due livelli**: leggera, predefinita, senza KVM né root, in un processo separato da
   quello che custodisce la chiave della cassaforte; forte (microVM) opzionale dove c'è KVM, con un
   pool di sandbox già avviate. Precisa il suggerimento del brief.
9. **Criterio sul carico del database (SM-2) riscritto**: il confronto con Hatchet si fa sullo
   stesso carico di hatchet-dev/hatchet#5153, perché le ~7.700 istruzioni per task del brief sono
   soprattutto polling a basso volume.

## 2. Utenti

### 2.1 Jobs To Be Done

**Integratore** (utente principale; il primo è Marco):

- Offrire ai propri tenant workflow configurabili senza scrivere un motore né un'interfaccia.
- Non scrivere codice per ogni workflow di ogni tenant: i tenant si arrangiano nell'editor.
- Garantire ai tenant che dati, esecuzioni e credenziali restino separati, anche quando caricano
  codice proprio.
- Tenere bassi i costi di infrastruttura: pochi componenti, poca RAM, database scarico.
- Arrivare al primo workflow funzionante in pochi minuti, anche senza conoscere topictoast.
- Elaborare flussi di eventi ad alto volume (per esempio indicatori di compromissione) senza
  avviare un workflow per ogni evento.

**Tenant** (organizzazione che usa la piattaforma ospite) e i suoi utenti:

- Comporre i propri workflow e le proprie pipeline stream senza programmare; chi sa programmare
  aggiunge nodi personalizzati propri.
- Collegare le proprie credenziali alle piattaforme esterne una volta sola.
- Vedere lo stato delle proprie esecuzioni, e solo quelle.
- Non sapere che sotto c'è topictoast.

**Operatore** (chi gestisce l'installazione per conto dell'integratore):

- Vedere quali worker sono collegati e cosa stanno facendo.
- Accorgersi di esecuzioni bloccate, pipeline stream in ritardo, tenant al limite.

### 2.2 Non utenti della v1

- **Chi vuole usare topictoast direttamente come n8n**, con nodi pronti e senza una piattaforma
  propria. n8n è troppo avanti per inseguirlo; questo pubblico è del possibile progetto separato
  `[ASSUMPTION]`.
- **Chi cerca un orchestratore su Kubernetes** in stile Argo: topictoast non ne ha bisogno e non lo
  sfrutta.
- **Chi cerca una piattaforma di analisi dati** (data warehouse, lakehouse): le pipeline stream
  servono a reagire agli eventi, non a fare analisi storiche.

### 2.3 Percorsi utente

- **UJ-1. Marco incorpora topictoast nella sua app di threat intelligence.**
  - **Persona e contesto:** Marco, sviluppatore ed esperto di sicurezza, oggi usa Hatchet con
    RabbitMQ bloccato alla 4.2.9 e vuole che i suoi tenant compongano da sé i workflow.
  - **Stato iniziale:** un server Linux, la sua app già esistente con il proprio sistema di login.
  - **Percorso:** installa topictoast e il bus con la configurazione predefinita e SQLite; scrive
    un worker con l'SDK TypeScript che registra le funzioni `dns_lookup` e `censys_search`;
    incorpora i componenti React nella sua app, collegandoli all'identità dei suoi tenant;
    definisce come codice un workflow di esempio e lo pubblica per un tenant di prova.
  - **Momento di valore:** dalla sua app lancia il workflow per il tenant di prova e vede le
    attività arrivare al worker e completarsi.
  - **Esito:** i suoi tenant possono comporre workflow dall'editor; lui non scrive più codice per
    ognuno.
  - **Caso limite:** il worker non è avviato quando parte l'esecuzione; le attività restano in
    coda, con il motivo, e partono appena il worker si registra, senza errori.

- **UJ-2. Giulia, utente di un tenant di Marco, compone, lancia e segue un workflow.** (Lo
  scenario "il giorno in cui funziona" del brief.)
  - **Persona e contesto:** Giulia, analista SOC presso un tenant di Marco, riceve ogni giorno
    elenchi di indicatori da verificare.
  - **Stato iniziale:** autenticata nell'app di Marco, apre la sezione workflow.
  - **Percorso:** nell'editor compone il workflow del caso concreto: riceve un elenco di
    indicatori, per ognuno verifica in parallelo DNS e Censys, arricchisce, inserisce. Collega le
    proprie credenziali Censys; aggiunge il nodo personalizzato che calcola un punteggio, scritto
    in TypeScript e caricato in precedenza da un collega sviluppatore con la CLI; salva e lancia
    con un elenco di 300 indicatori.
  - **Momento di valore:** vede il DAG prendere vita, con 300 rami paralleli che si completano e
    il join finale; i risultati sono suoi.
  - **Esito:** il workflow è salvato e rilanciabile; lo storico mostra solo le esecuzioni del suo
    tenant.
  - **Variante:** il giorno dopo lancia lo stesso workflow con un indicatore solo; il fan-out lo
    tratta come un elenco di un elemento, senza un workflow diverso.
  - **Variante da telefono:** compone, salva e lancia lo stesso workflow dal telefono (NFR-16).
  - **Caso limite:** il nodo personalizzato va in loop infinito; la sandbox lo ferma allo
    scadere del timeout, il task risulta fallito con un messaggio leggibile, gli altri tenant non
    se ne accorgono.

- **UJ-3. Giulia crea una pipeline stream che reagisce a un'ondata di indicatori.**
  `[ASSUMPTION: esempio dedotto dalla domanda sulle finestre, da confermare con un caso reale di
  Marco]`
  - **Stato iniziale:** gli indicatori del feed del suo tenant arrivano su uno stream.
  - **Percorso:** nell'editor apre "nuova pipeline stream", scrive una query che conta gli
    indicatori per ASN su finestre di 5 minuti e tiene le righe con più di 50 indicatori; come
    destinazione sceglie "avvia il workflow di arricchimento per ogni riga"; salva e attiva.
  - **Momento di valore:** durante un'ondata la pipeline stream emette una riga per l'ASN sospetto
    e il workflow di arricchimento parte da solo, una volta per ASN invece che per ogni indicatore.
  - **Esito:** la pipeline stream resta attiva; Giulia ne vede lo stato e le ultime righe emesse.
  - **Caso limite:** topictoast si riavvia durante una finestra; la pipeline stream riparte
    dall'ultimo checkpoint e il conteggio della finestra non si perde.

- **UJ-4. Marco, da operatore, controlla l'installazione.**
  - **Persona e contesto:** Marco, nel ruolo di operatore, gestisce l'installazione di topictoast
    che serve la sua app di threat intelligence.
  - **Stato iniziale:** l'installazione è attiva, con tenant che lanciano workflow e pipeline
    stream attive.
  - **Percorso:** apre la dashboard ed entra con la credenziale generata al primo avvio; vede i
    worker collegati con le funzioni registrate e le attività in corso, le esecuzioni ferme in
    coda da più tempo, le pipeline stream attive, rallentate e sospese con il loro ritardo e la
    causa, i tenant vicini al limite.
  - **Momento di valore:** individua un worker che non risponde: le sue attività sono già state
    riassegnate a un altro worker.
  - **Esito:** Marco sa se l'installazione lavora bene e quale worker, esecuzione o tenant
    richiede un intervento.
  - **Caso limite:** un tenant supera la quota di attività concorrenti; risulta al limite, le sue
    nuove attività restano in coda, gli altri tenant non rallentano.

- **UJ-5. Luca, che non conosce topictoast, lo prova.**
  - **Persona e contesto:** Luca sviluppa una piattaforma di automazione, non conosce topictoast e
    ne trova il repository.
  - **Stato iniziale:** una macchina pulita e la guida rapida del repository.
  - **Percorso:** segue la guida rapida; avvia topictoast con la configurazione predefinita
    (FR-50), poi il worker di esempio e il workflow di esempio; entra nella dashboard con la
    credenziale generata al primo avvio. Lavora nel tenant predefinito senza incontrare la
    gestione dei tenant (§4.6).
  - **Momento di valore:** nelle esecuzioni della dashboard vede l'esecuzione del workflow di
    esempio "completata".
  - **Esito:** in meno di 15 minuti ha topictoast in esecuzione con un worker e un workflow di
    esempio completato (NFR-13).
  - **Caso limite:** non ha sottomano la credenziale del primo avvio; la pagina d'accesso della
    dashboard dice dove è stata stampata (FR-45).

## 3. Glossario

I termini seguenti si usano esattamente così in tutto il PRD e nei documenti che ne derivano.
"Pipeline" da sola non si usa: si dice "workflow" oppure "pipeline stream".

- **topictoast** — il motore descritto da questo PRD, con SDK, kit frontend e dashboard.
- **Integratore** — sviluppatore o azienda che incorpora topictoast nella propria piattaforma
  ospite. Uno per installazione.
- **Piattaforma ospite** — l'applicazione dell'integratore che incorpora topictoast; per Marco,
  l'app di threat intelligence.
- **Tenant** — organizzazione che usa la piattaforma ospite; è l'unità di separazione. Un
  integratore ha uno o più tenant: al primo avvio esiste il tenant predefinito e, finché è
  l'unico, tutto lavora lì e la gestione dei tenant non si vede (§4.6).
- **Operatore** — persona dell'integratore che gestisce l'installazione dalla dashboard.
- **Workflow** — definizione eseguibile: un documento OWS, eventualmente con l'estensione DAG.
  Appartiene a un tenant o all'integratore (FR-29).
- **Documento OWS** — un workflow scritto nel formato Open Workflow Specification 1.1, eventualmente
  con l'estensione DAG. topictoast lo valida con il proprio schema: il sottoinsieme OWS della v1
  più l'estensione.
- **Estensione DAG** — l'aggiunta di topictoast al formato OWS per esprimere dipendenze arbitrarie
  tra task e fan-out dinamico.
- **Task** — un passo di un workflow.
- **Esecuzione** — una corsa di un workflow con un input, nel contesto del suo proprietario: un
  tenant o l'integratore (FR-26).
- **Attività** — unità di lavoro consegnata a un worker o alla sandbox: un task in un'esecuzione,
  una per ogni elemento di un fan-out, una per ogni tentativo. È l'unità dei requisiti di
  prestazione.
- **Fan-out** — creazione, durante l'esecuzione, di un'attività per ogni elemento di un elenco la
  cui lunghezza si conosce solo allora. **Join** — il punto in cui si attendono tutte.
- **Worker** — processo dell'integratore, sempre attivo, che registra funzioni e riceve attività.
- **Funzione** — codice registrato da un worker con un nome, invocabile da un task.
- **Manifest** — la descrizione di una funzione o di un nodo personalizzato (FR-55): nome,
  descrizione, gruppo, connessioni richieste, schema di input e di output, un campo di
  estensione per altri metadati.
- **Nodo personalizzato** — codice di un tenant, scritto in TypeScript o Rust e caricato già
  compilato tramite SDK o CLI, che topictoast esegue nella sandbox.
- **Sandbox** — ambiente isolato in cui topictoast esegue i nodi personalizzati. **Sandbox
  leggera**: il livello predefinito, senza KVM né root. **Sandbox forte**: microVM, opzionale dove
  l'host offre KVM.
- **Pool della sandbox** — insieme di sandbox forti già avviate, pronte a ricevere attività; ogni
  sandbox è monouso o riservata a un tenant.
- **CLI** — lo strumento a riga di comando di topictoast, costruito sull'SDK.
- **Trigger** — ciò che avvia un'esecuzione: chiamata API o SDK, pianificazione, riga emessa da una
  pipeline stream.
- **Stream** — sequenza ordinata e illimitata di eventi di un tenant.
- **Evento** — un record su uno stream, con un tempo dell'evento.
- **Pipeline stream** — query SQL continua e sempre attiva su uno o più stream, con stato, che
  emette righe verso una destinazione.
- **Finestra** — intervallo di tempo dell'evento su cui una pipeline stream aggrega.
- **Watermark** — soglia di tempo dell'evento oltre cui una finestra si chiude; gli eventi più
  vecchi sono in ritardo.
- **Checkpoint** — salvataggio periodico dello stato di una pipeline stream.
- **Segreto** — credenziale di un tenant, o dell'integratore, per una piattaforma esterna; nella v1
  una chiave API.
- **Connessione** — la piattaforma esterna e il tipo di credenziale di cui una funzione o un nodo
  personalizzato ha bisogno, dichiarati nel manifest; un segreto dichiara la connessione che
  soddisfa (FR-32). Due manifest che dichiarano la stessa piattaforma e lo stesso tipo di
  credenziale chiedono la stessa connessione. Non è una connessione di rete, il senso che la
  parola ha in FR-16, FR-50 e NFR-8, né un connettore (§7).
- **Cassaforte** — la parte di topictoast che custodisce i segreti.
- **Quota** — limite di risorse per tenant.
- **Kit frontend** — la libreria JavaScript in due strati: **nucleo JS** (senza interfaccia) e
  **componenti React**.
- **Editor** — il componente React per comporre workflow e pipeline stream.
- **Dashboard** — l'interfaccia web dell'operatore, distribuita con topictoast.
- **Macchina di riferimento** — 4 vCPU, 16 GB di RAM, SSD NVMe locale, Linux; base dei requisiti
  del §5.
- **Processo ausiliario** — processo che topictoast avvia e ferma da sé (per esempio la sandbox
  leggera); non si installa né si configura a parte.
- **Stati** — elenco chiuso, per oggetto. "In attesa" vuol dire dipendenze non finite; "in coda"
  vuol dire pronto ma non ancora consegnato, sempre con il motivo. Un'esecuzione è "in coda"
  quando è accettata e nessun suo task è ancora partito (un `wait` in corso conta come
  partito). "Ritentata" è un tentativo fallito seguito da un altro. Il timeout è un motivo, non
  uno stato. "Scaduta" vale solo per un'esecuzione; in un'esecuzione annullata o scaduta i task non
  partiti diventano "annullato". Un task saltato dice perché. La regola di propagazione di
  fallimenti e salti resta una domanda aperta (§11).
  - Esecuzione: in coda, in esecuzione, completata, fallita, annullata, scaduta.
  - Task: in attesa, in coda, in esecuzione, ritentato, completato, fallito, saltato, annullato.
  - Attività: in coda, ricevuta, in esecuzione, ritentata, completata, fallita, annullata.
  - Worker: collegato, non risponde.
  - Pipeline stream: attiva, rallentata, sospesa; quando la sospende il sistema, con la causa
    (CPU, memoria di stato o di esecuzione, arretrato).
  - Tenant, per le quote: vicino al limite, al limite.
  - Pianificazione: attiva, sospesa (FR-6).

## 4. Funzionalità

### 4.1 Definizione dei workflow

**Descrizione:** un workflow è un documento OWS 1.1. topictoast ne accetta un sottoinsieme nella v1
e aggiunge l'estensione DAG per due cose che OWS non esprime: dipendenze arbitrarie tra task (la
forma a "N") e fan-out su elenchi di lunghezza variabile, con join. La forma dell'estensione
(dentro `metadata` o con chiavi nuove) la decide l'architettura. I workflow arrivano dall'SDK,
come codice o come documento, e dall'editor. Realizza UJ-1, UJ-2.

#### FR-1: Sottoinsieme OWS della v1

topictoast implementa un sottoinsieme di OWS 1.1 e non è un runtime OWS conforme. L'integratore e
il tenant possono definire workflow con il sottoinsieme della v1: do, set, switch, fork, for, try
con retry, wait, raise, call http, invocazione di funzioni e nodi personalizzati, sotto-workflow,
espressioni jq, `schedule` del documento cron o a intervallo (FR-6).

**Conseguenze (verificabili):**
- Un documento OWS 1.1 che usa solo il sottoinsieme viene accettato ed eseguito con la semantica
  della specifica.
- Un documento che usa un costrutto fuori dal sottoinsieme viene rifiutato all'invio con un errore
  che nomina il costrutto e il percorso nel documento.
- Si accettano documenti con `document.dsl` 1.1.x; gli altri valori vengono rifiutati con un errore
  che indica le versioni accettate `[ASSUMPTION]`.

**Fuori perimetro:** `call` gRPC, OpenAPI, AsyncAPI e A2A; `run` di container, script e shell;
`emit` e `listen`; conformità completa al CTK di OWS.

**Note:** come un task invoca una funzione o un nodo personalizzato, e come dichiara i segreti
dentro il formato OWS, lo fissa l'architettura (§11).

#### FR-2: Dipendenze arbitrarie tra task

L'integratore e il tenant possono dichiarare che un task dipende da uno o più task qualsiasi del
workflow. Realizza UJ-2.

**Conseguenze:**
- Un task parte quando tutte le sue dipendenze sono completate, non prima.
- Task senza dipendenze reciproche partono in parallelo.
- Un ciclo nelle dipendenze viene rifiutato all'invio, con i task coinvolti nel messaggio. Il
  divieto vale per le dipendenze dichiarate con l'estensione DAG; `for` e `then` mantengono la
  semantica di OWS.
- Un task la cui dipendenza è fallita o è stata saltata non resta in attesa: raggiunge uno stato
  terminale secondo una regola documentata (§11).

#### FR-3: Fan-out dinamico con join

L'integratore e il tenant possono dichiarare che un task (o un sotto-grafo) si ripete in parallelo
per ogni elemento di un elenco prodotto durante l'esecuzione, e che un task successivo attende tutti
i risultati. Realizza UJ-2.

**Conseguenze:**
- Con un elenco di N elementi nascono N attività; il join riceve N risultati nell'ordine
  dell'elenco.
- Un elenco vuoto non crea attività e il join riceve un elenco vuoto.
- Un input singolo al posto dell'elenco si comporta come un elenco di un elemento: lo stesso
  workflow accetta un indicatore o un elenco (UJ-2, variante).
- Il parallelismo massimo di un fan-out si può limitare nel workflow; senza limite vale la quota
  del tenant (FR-30).
- Se un elemento fallisce dopo i retry, il comportamento segue la politica dichiarata: fallisce
  tutto il fan-out oppure il join riceve l'errore al posto del risultato.

#### FR-4: Validazione all'invio

topictoast valida ogni workflow quando lo riceve, prima di salvarlo, con il proprio schema
(sottoinsieme OWS più estensione DAG).

**Conseguenze:**
- Un documento non valido restituisce un errore di validazione con percorso e motivo; mai un
  crash, mai un salvataggio parziale.
- I riferimenti a funzioni, nodi personalizzati, segreti e task inesistenti sono segnalati
  all'invio.
- L'integratore dichiara quali funzioni sono invocabili dai workflow dei tenant. Un documento di
  un tenant che invoca una funzione non disponibile viene rifiutato all'invio come se la funzione
  non esistesse, senza dettagli del suo manifest (il nucleo JS non lo riceve); il controllo si
  ripete all'esecuzione.
- L'input di ogni task è controllato sul manifest della funzione o del nodo personalizzato
  (FR-55); quello di un sotto-workflow, sullo schema di input del workflow chiamato (FR-56). Tipi
  sbagliati, campi obbligatori mancanti e, nel workflow di un tenant, connessioni senza un
  segreto sono errori all'invio, con gli stessi esiti dell'editor (FR-40) e del nucleo JS (FR-39).
  Le connessioni di un workflow dell'integratore si risolvono all'avvio (FR-29).
- Un segreto legato a una connessione che non soddisfa, o in più rispetto alle connessioni del
  manifest del suo task, è un errore all'invio.

#### FR-5: Versioni dei workflow

L'integratore e il tenant possono pubblicare una nuova versione di un workflow mentre la
precedente ha esecuzioni in corso. Dall'editor, salvare è pubblicare: non c'è un passo di
pubblicazione separato. I due costi sono accettati: un lavoro incompleto o non valido non si salva e
resta solo nel kit (FR-27), e ogni salvataggio cambia subito ciò che avviano la pianificazione e
le pipeline stream che usano il workflow (FR-57, FR-24).

**Conseguenze:**
- Prima di un salvataggio che cambia lo schema di input (FR-56), l'editor avvisa della
  pianificazione e delle pipeline stream che avviano il workflow.
- Le esecuzioni in corso finiscono con la versione con cui sono partite.
- Le nuove esecuzioni usano l'ultima versione pubblicata, salvo richiesta esplicita di una
  versione.
- Una pubblicazione preparata su una versione che nel frattempo non è più l'ultima viene rifiutata
  con un errore che indica la versione corrente; non sostituisce mai in silenzio il lavoro di un
  altro.

#### FR-56: Schema di input del workflow

L'integratore e il tenant possono dichiarare lo schema dell'input di un workflow (campi, tipi,
campi obbligatori), dall'editor, dall'SDK o nel documento. Realizza UJ-2.

**Conseguenze:**
- Un avvio con un input non conforme allo schema viene rifiutato prima di essere accettato, con
  percorso e regola violata, mai con il valore ricevuto.
- Un valore singolo per un campo elenco vale come elenco di un elemento (FR-3).
- Un workflow senza schema accetta qualunque input JSON.

**Note:** lo schema usa il linguaggio di FR-55; come si scrive dentro il documento OWS lo fissa
l'architettura (§11).

### 4.2 Esecuzione

**Descrizione:** topictoast avvia le esecuzioni, consegna le attività ai worker o alla sandbox,
applica retry e timeout, e porta l'esecuzione fino alla fine anche se si riavvia nel mezzo.
Realizza UJ-1, UJ-2, UJ-3.

#### FR-6: Trigger

Un'esecuzione parte da una chiamata API o SDK, da una pianificazione o da una riga emessa da una
pipeline stream (FR-24). Realizza UJ-1, UJ-3.

**Conseguenze:**
- Ogni avvio restituisce subito l'identificativo dell'esecuzione.
- Ogni esecuzione registra la sua origine: API o SDK, con il ruolo di chi l'ha avviata e, se
  presente, il nome visualizzato (FR-27); pianificazione; pipeline stream.
- Una pianificazione è quella scritta nel documento OWS (`schedule`, FR-1), del proprietario del
  workflow, oppure quella che un tenant aggiunge dal kit (FR-57). Le esecuzioni che avvia
  appartengono al proprietario della pianificazione (FR-26).
- Una pianificazione porta l'input delle esecuzioni che avvia, controllato sullo schema di input
  del workflow (FR-56) quando si salva la pianificazione e a ogni scadenza.
- Una pianificazione cron o a intervallo avvia un'esecuzione per ogni scadenza accettata, e una
  sola anche se il motore si ferma o riparte mentre la scadenza scatta.
- Una scadenza, o una riga di una pipeline stream (FR-24), che non diventa un'esecuzione
  accettata (per esempio input fuori schema o connessione senza segreto) è registrata sulla
  pianificazione o sulla pipeline stream con il motivo, visibile al suo proprietario e
  all'operatore; mai in silenzio.
- Una pianificazione si sospende e si riprende senza modificare il workflow; le scadenze cadute
  durante la sospensione non si recuperano.
- Dopo un fermo, le scadenze perse e quelle recuperate sono visibili al proprietario della
  pianificazione.
- Per almeno la durata di conservazione dell'esecuzione (FR-31), un avvio con una chiave di
  idempotenza già usata dallo stesso tenant restituisce l'esecuzione esistente invece di crearne
  una nuova, anche quando le richieste arrivano in contemporanea. Cento richieste concorrenti con
  la stessa chiave producono una sola esecuzione.

#### FR-7: Retry

Il workflow dichiara per ogni task una politica di retry: numero di tentativi, attesa tra i
tentativi, crescita dell'attesa.

**Conseguenze:**
- Un'attività fallita viene ritentata secondo la politica; ogni tentativo è un'attività distinta
  visibile nello storico.
- Esauriti i tentativi, il task fallisce e l'errore dell'ultimo tentativo è conservato.

#### FR-8: Timeout

Il workflow dichiara timeout per task e per l'intera esecuzione.

**Conseguenze:**
- Un'attività oltre il timeout è considerata fallita e segue la politica di retry.
- Un'esecuzione oltre il suo timeout termina come scaduta; le attività ancora in corso vengono
  annullate.
- Il timeout del task conta dalla consegna al worker o alla sandbox. L'attesa prima della consegna
  (nessun worker collegato, tenant oltre quota, pool della sandbox vuoto) non fa fallire
  l'attività per timeout del task e non consuma tentativi; resta comunque limitata da un tempo
  documentato (NFR-14). Un'attività rimasta in coda per quota più a lungo del timeout del suo
  task, una volta consegnata, si completa normalmente.

#### FR-9: Durabilità

Un'esecuzione accettata arriva alla fine anche se topictoast o il bus si riavviano nel mezzo.

**Conseguenze:**
- Un'esecuzione, un'attività o un evento risulta accettato solo se, quando parte la risposta, è
  già al sicuro dal guasto dichiarato in NFR-6. Dopo quel guasto nessuna esecuzione, attività o
  evento accettato va perso, e nessun task il cui completamento è stato registrato viene
  rieseguito; un'attività consegnata senza esito registrato può essere consegnata di nuovo
  (consegna almeno una volta).
- La consegna ai worker è almeno una volta; la documentazione per l'integratore spiega come
  rendere idempotenti le funzioni.

#### FR-10: Annullamento

L'integratore e il tenant possono annullare un'esecuzione in coda o in esecuzione.

**Conseguenze:**
- Nessuna nuova attività parte dopo l'annullamento; i worker e la sandbox ricevono la richiesta di
  fermare le attività in corso.
- L'esecuzione risulta annullata nello storico, con chi l'ha annullata (ruolo e, se presente,
  nome visualizzato) e quando.

#### FR-11: Dati tra task

L'output di un task è disponibile come input dei task che ne dipendono.

**Conseguenze:**
- Input e output di un'attività hanno un limite di dimensione
  `[ASSUMPTION: 1 MB nella v1, perché lo storage per payload grandi arriva dopo]`.
- Un output oltre il limite fa fallire l'attività con un errore che indica dimensione e limite.
- In un fan-out il limite vale per ogni elemento; l'input aggregato del join ha un tetto proprio,
  configurabile, con valore predefinito documentato (NFR-14). Oltre il tetto il join fallisce con
  dimensione e limite.

### 4.3 Worker

**Descrizione:** i worker sono processi dell'integratore, sempre attivi, che si comportano come
funzioni serverless senza avvio a freddo: si collegano a topictoast, registrano le proprie funzioni
e ricevono attività finché restano collegati. Si scalano aggiungendo processi. Realizza UJ-1,
UJ-4.

#### FR-12: Registrazione e consegna

Un worker si collega con l'SDK, registra le sue funzioni con il loro manifest (FR-55) e riceve le
attività che le invocano.

**Conseguenze:**
- Le attività per una funzione senza worker collegati restano in coda, con il motivo, entro il
  limite di attesa di FR-8, e partono quando un worker la registra.
- Con più worker che registrano la stessa funzione, le attività si distribuiscono tra loro.
- Solo un processo con una credenziale di worker valida, distinta dalle credenziali dei tenant,
  può collegarsi, registrare funzioni e ricevere attività. Un processo senza credenziale, o con una
  credenziale di tenant, che registra una funzione esistente viene rifiutato e non riceve alcuna
  attività né alcun segreto.
- Chi raggiunge il bus senza una credenziale valida non legge né scrive attività ed eventi.
- Entrambi i casi entrano nel test di NFR-8.

#### FR-13: Concorrenza e contropressione

Ogni worker dichiara quante attività accetta in parallelo.

**Conseguenze:**
- topictoast non consegna a un worker più attività di quante ne abbia dichiarate.
- Mentre un worker è lento o bloccato, le attività consegnate ad altri worker restano entro
  l'obiettivo di NFR-2, anche quando sono accodate insieme alle sue `[ASSUMPTION: la soglia di non
  interferenza è l'obiettivo di NFR-2, qui e in FR-17, FR-25, FR-30]`.

#### FR-14: Worker che non risponde

topictoast si accorge quando un worker smette di rispondere.

**Conseguenze:**
- Le attività di un worker che non dà segni di vita entro l'intervallo configurato vengono
  riassegnate ad altri worker, contando un tentativo.
- Il worker risulta "non risponde" nella dashboard (FR-48).

#### FR-55: Manifest delle funzioni

Quando registra una funzione (FR-12), il worker ne dichiara il manifest: nome, descrizione, gruppo,
connessioni richieste (piattaforma esterna e tipo di credenziale), schema di input, schema di
output e un campo di estensione per altri metadati. Anche un nodo personalizzato (FR-15) porta un
manifest, con gli stessi campi. Realizza UJ-1, UJ-2.

**Conseguenze:**
- Un manifest non valido viene rifiutato con l'errore del validatore.
- Gli schemi del manifest e lo schema di input di un workflow (FR-56) usano un linguaggio chiuso
  e documentato (campi, tipi, campi obbligatori, elenchi, oggetti annidati), con una sola regola
  di compatibilità comune a motore, nucleo JS ed editor (§11).
  - Non rimandano a nulla fuori dal manifest o dal documento: uno schema con un riferimento
    esterno viene rifiutato alla validazione senza aprire connessioni di rete.
  - Manifest e schemi ostili (riferimento esterno, oltre i limiti di NFR-14) entrano nel test di
    NFR-8.
  - Mentre un tenant invia e avvia a ripetizione workflow con gli schemi e l'input più complessi
    consentiti (NFR-14), le attività degli altri tenant restano entro l'obiettivo di NFR-2; il
    caso entra nella corsa a carico misto di FR-53.
- All'esecuzione il motore controlla l'input di ogni attività sullo schema di input del manifest
  e l'output sullo schema di output. Un input non conforme fa fallire il task prima della
  consegna, senza consumare tentativi; un output non conforme fa fallire l'attività con percorso
  e motivo e non arriva mai ai task a valle.
- Nell'editor le funzioni dei worker e i nodi personalizzati si presentano con gli stessi campi,
  presi dal manifest; la categoria (funzione del worker o nodo personalizzato) li distingue e il
  gruppo raggruppa le voci.
- Il formato del manifest, con l'elenco chiuso dei suoi campi, è una superficie pubblica (§6).

### 4.4 Nodi personalizzati e sandbox

**Descrizione:** un tenant può caricare nodi personalizzati, scritti in TypeScript o Rust e già
compilati, e usarli nei suoi workflow. topictoast li esegue nella sandbox, non l'integratore. La
sandbox deve reggere codice ostile che gira accanto ai segreti di tutti i tenant. Ha due livelli:
la sandbox leggera, predefinita, che non richiede KVM né root; la sandbox forte (microVM),
opzionale dove l'host offre KVM, con un pool di sandbox già avviate per evitare l'avvio a freddo.
Le tecnologie dei due livelli le sceglie uno spike prima dello sprint 1. Realizza UJ-2.

#### FR-15: Caricamento di un nodo personalizzato

Il tenant può caricare un nodo personalizzato già compilato tramite SDK o CLI, con il manifest
(FR-55). L'editor lo mostra tra i task disponibili ma non ne ospita il codice.

**Conseguenze:**
- Il nodo è visibile solo ai workflow del tenant che l'ha caricato.
- Un nodo che non rispetta il formato richiesto (modulo compilato, manifest) viene rifiutato con
  l'errore del validatore.
- Il server di topictoast non richiede compilatori installati.
- Caricare un nodo con un nome già usato crea una versione nuova: un'esecuzione in corso esegue
  fino alla fine la versione del nodo con cui è partita, e ogni attività registra quale versione ha
  eseguito.

#### FR-16: Isolamento della sandbox

Un nodo personalizzato in esecuzione vede solo il proprio input e i segreti che il suo task
dichiara.

**Conseguenze:**
- Non raggiunge la rete interna dell'installazione, il file system dell'host, i dati o i segreti di
  altri tenant.
- Non raggiunge la chiave della cassaforte né i segreti non dichiarati dal suo task: la sandbox, a
  entrambi i livelli, gira fuori dal processo che custodisce la chiave.
- Ogni connessione di rete aperta per conto di un tenant, da un nodo personalizzato o da una
  `call http`, raggiunge solo le destinazioni che l'integratore consente a quel tenant.
  - La lista dichiarata in un nodo può restringerle, mai allargarle.
  - Loopback, indirizzi privati, link-local e indirizzi dell'installazione (motore, bus,
    database) restano irraggiungibili anche quando un nome consentito vi si risolve o un
    redirect vi porta.
  - Una `call http` verso `127.0.0.1`, `169.254.169.254` o un nome che si risolve a un
    indirizzo privato fallisce con un errore di politica di rete senza aprire la connessione;
    il caso entra nel test di NFR-8.
- Il trattamento dei byte di un nodo caricato (validazione, compilazione, istanziazione), manifest
  compreso, avviene fuori dal processo che custodisce la chiave della cassaforte: un modulo
  costruito per far crashare il validatore non ferma il motore né le esecuzioni in corso.
- A entrambi i livelli, un'istanza della sandbox, il suo processo e la memoria che ha usato non
  contengono mai dati o segreti di due tenant, né insieme né uno dopo l'altro. Nel test di NFR-8,
  mentre gira un nodo del tenant B con un segreto sentinella, il processo che esegue il nodo del
  tenant A non contiene quel segreto.

#### FR-17: Limiti di risorse

Ogni esecuzione di un nodo personalizzato e ogni espressione jq di un tenant ha limiti di CPU,
memoria e tempo; ogni query di pipeline stream ha limiti di CPU e di memoria, compresa la memoria
di esecuzione oltre a quella di stato (FR-25).

**Conseguenze:**
- Superare un limite fa fallire l'attività con un errore che nomina il limite superato, oppure
  rallenta o sospende la pipeline stream con un errore visibile al tenant e all'operatore; il
  processo del motore non si ferma e le attività degli altri tenant restano entro l'obiettivo di
  NFR-2.
- Un nodo fermato non lascia processi né memoria occupata; mentre un nodo satura i propri limiti,
  le attività degli altri tenant restano entro l'obiettivo di NFR-2.
- Un'espressione jq non legge l'ambiente del processo, il file system, moduli esterni né la rete:
  un'espressione che usa `$ENV`, `env`, `input_filename`, `import` o `include` viene rifiutata alla
  validazione (FR-4).

#### FR-54: Sandbox forte opzionale

Dove l'host offre KVM, l'integratore può attivare la sandbox forte per tutti i tenant o per alcuni.

**Conseguenze:**
- Un pool configurabile di sandbox forti già avviate riceve le attività senza avvio a freddo;
  quando il pool è vuoto, le attività attendono una sandbox libera entro il limite di attesa di
  FR-8.
- Una sandbox che ha eseguito codice di un tenant non passa mai a un altro tenant: è monouso o
  riservata a quel tenant.
- La RAM del pool si misura a parte, fuori da NFR-1.
- Su un host senza KVM la sandbox forte non si attiva e topictoast funziona con la sandbox
  leggera, con un avviso all'operatore nella dashboard.

### 4.5 Pipeline stream

**Descrizione:** una pipeline stream è una query SQL continua, sempre attiva, su uno o più stream
di un tenant. Funziona sul modello di Arroyo: tiene uno stato, aggrega su finestre di tempo
dell'evento, gestisce gli eventi in ritardo con i watermark e salva lo stato con checkpoint
periodici. Le pipeline stream le scrivono l'integratore e i tenant, dall'editor o tramite SDK;
ogni tenant vede e consuma solo le proprie risorse. Come costruirle (riuso di Arroyo, DataFusion
con stato costruito in topictoast, altro) lo decide l'architettura con uno spike. Realizza UJ-3.

#### FR-18: Stream e ingresso degli eventi

L'integratore, i worker e i workflow possono scrivere eventi sugli stream di un tenant tramite API
o SDK. `[ASSUMPTION: nella v1 nessun connettore verso sistemi esterni come Kafka]`

**Conseguenze:**
- Un evento scritto su uno stream di un tenant è leggibile solo dalle pipeline stream di quel
  tenant.
- Un evento senza tempo dell'evento riceve il tempo di arrivo.
- Ogni stream ha una conservazione per durata o per dimensione, con valore predefinito documentato
  (NFR-14).
- La conservazione è breve, nell'ordine di pochi minuti: copre la finestra più lunga delle
  pipeline stream che leggono lo stream, più il ritardo tollerato e l'intervallo dei checkpoint.
  Una pipeline stream con una finestra che supera la conservazione dello stream viene rifiutata al
  salvataggio con un errore che indica i due valori.

#### FR-19: Definizione in SQL

L'integratore e il tenant possono definire una pipeline stream con una query SQL su uno o più
stream.

**Conseguenze:**
- La query viene validata al salvataggio; un errore di sintassi o un riferimento a uno stream
  inesistente restituisce un messaggio con riga e colonna.
- Una query che fa riferimento a stream di un altro tenant viene rifiutata come se lo stream non
  esistesse.
- Il dialetto delle pipeline stream dei tenant è una lista chiusa e documentata: interrogazioni
  sugli stream del tenant con le funzioni elencate. DDL, tabelle esterne, connettori, lettura o
  scrittura di file, accesso alla rete e funzioni definite dall'utente vengono rifiutati al
  salvataggio con un errore di validazione. Un insieme di query ostili (tabella esterna su
  `/etc/passwd`, `COPY` verso un file, connettore verso l'indirizzo del bus, funzione definita
  dall'utente) entra nel test di NFR-8.
- Un ciclo tra pipeline stream dello stesso tenant (una scrive sullo stream da cui un'altra legge,
  fino a tornare alla prima) viene rifiutato al salvataggio.
- Rielaborata dal checkpoint sugli stessi eventi, una pipeline stream emette le stesse righe
  (condizione per FR-24).

#### FR-20: Finestre e aggregazioni

Una pipeline stream può aggregare su finestre di tempo dell'evento, deduplicare e filtrare.
`[ASSUMPTION: finestre fisse e scorrevoli nella v1; finestre di sessione dopo]`

**Conseguenze:**
- Un conteggio su finestre fisse di 5 minuti emette una riga per gruppo quando la finestra si
  chiude.
- La deduplicazione su una chiave emette ogni chiave una sola volta per finestra.

#### FR-21: Watermark ed eventi in ritardo

Una pipeline stream dichiara quanto ritardo tollera.

**Conseguenze:**
- Una finestra si chiude quando il watermark supera la sua fine più il ritardo tollerato.
- Gli eventi arrivati dopo la chiusura della loro finestra vengono scartati e contati in una
  metrica visibile nella dashboard.
- Un evento con tempo dell'evento oltre una tolleranza nel futuro rispetto all'arrivo non sposta il
  watermark e non fa scartare gli eventi successivi; è contato in una metrica visibile nella
  dashboard. Un evento datato un anno nel futuro non chiude alcuna finestra.
- Su uno stream che smette di ricevere eventi, le finestre aperte si chiudono comunque entro un
  tempo documentato (NFR-14): l'ultima finestra di un'ondata emette le sue righe (UJ-3).

#### FR-22: Stato e checkpoint

Lo stato di una pipeline stream sopravvive ai riavvii.

**Conseguenze:**
- Dopo un riavvio di topictoast la pipeline stream riparte dall'ultimo checkpoint e rielabora gli
  eventi successivi; nessuna finestra perde eventi già accettati.
- L'intervallo tra checkpoint è configurabile, con un valore predefinito
  `[ASSUMPTION: 10 secondi]`.

#### FR-23: Ciclo di vita

L'integratore e il tenant possono attivare, sospendere, modificare ed eliminare una pipeline
stream.

**Conseguenze:**
- Una pipeline stream sospesa non consuma CPU; alla ripresa elabora gli eventi arrivati nel
  frattempo e ancora conservati. Se una parte è già uscita dalla conservazione, la pipeline stream
  lo segnala con un errore visibile al tenant e all'operatore, mai in silenzio.
- Modificare la query di una pipeline stream attiva crea una nuova versione che parte con stato
  vuoto `[ASSUMPTION]`.

#### FR-24: Destinazioni

Le righe emesse da una pipeline stream vanno su uno stream del tenant oppure avviano un workflow
del tenant, un avvio per riga, con la riga come input. Realizza UJ-3.

**Conseguenze:**
- Con destinazione workflow, ogni riga dà esattamente un avvio anche dopo un riavvio dal
  checkpoint: un'esecuzione accettata oppure un rifiuto registrato con il motivo (FR-6). Due
  righe distinte con lo stesso contenuto (per esempio lo stesso conteggio in due finestre) danno
  due avvii.
- Il numero di esecuzioni avviate da una pipeline stream rispetta la quota del tenant (FR-30); le
  righe in eccesso restano trattenute entro il tetto dell'arretrato e non vanno perse.

#### FR-25: Isolamento e quote delle pipeline stream

Le pipeline stream di un tenant non possono degradare quelle degli altri.

**Conseguenze:**
- Ogni tenant ha quote sul numero di pipeline stream attive, sulla memoria di stato e sulla CPU.
- Una pipeline stream che supera la memoria di stato consentita viene sospesa con un errore
  visibile al tenant e all'operatore.
- Una pipeline stream oltre la quota di CPU viene rallentata o sospesa con un errore visibile al
  tenant e all'operatore.
- Mentre una pipeline stream di un tenant è al limite delle sue quote, le pipeline stream degli
  altri tenant elaborano gli eventi al ritmo con cui arrivano, senza ritardo crescente sul
  watermark, e le attività degli altri tenant restano entro l'obiettivo di NFR-2.

### 4.6 Separazione tra tenant

**Descrizione:** ogni risorsa (workflow, esecuzione, attività, nodo personalizzato, stream,
pipeline stream, segreto) appartiene a un tenant o all'integratore. topictoast non autentica gli
utenti della piattaforma ospite: si fida dell'identità del tenant che la piattaforma ospite gli
garantisce. I tenant sono opzionali: al primo avvio esiste il tenant predefinito e, finché è
l'unico, tutto lavora lì senza configurare nulla e la gestione dei tenant non si vede. Quando
l'integratore crea un secondo tenant, la separazione si attiva. Il motore ha un solo
comportamento: segreti, quote e storico sono per tenant anche con il solo tenant predefinito.
Realizza UJ-2, UJ-4.

#### FR-26: Visibilità per tenant

Ogni richiesta fatta per conto di un tenant vede e modifica solo le risorse di quel tenant.

**Conseguenze:**
- Una richiesta per una risorsa di un altro tenant risponde come se la risorsa non esistesse.
- Liste, ricerche, conteggi ed eventi in tempo reale non contengono mai risorse di altri tenant.
- Il tenant di una richiesta, e ciò che la richiesta può fare, si ricavano solo dalla credenziale
  verificata, mai da un campo della richiesta (percorso, corpo, filtro di una lista): una
  credenziale del tenant A con l'identificativo del tenant B in un parametro agisce su A o viene
  rifiutata.
- Un'esecuzione appartiene a chi la avvia e resta nel suo contesto. Avviata da un tenant, o per
  suo conto con la credenziale dell'integratore (FR-28), a mano, con la sua pianificazione o da
  una sua pipeline stream, è di quel tenant e, tra i tenant, la vede solo lui; avviata dalla
  pianificazione del documento (FR-6) è del proprietario del workflow, che per un workflow
  dell'integratore è l'integratore (FR-29). Un workflow di un tenant gira sempre nel contesto di
  quel tenant, anche quando lo avvia l'integratore; un'esecuzione dell'integratore esegue solo
  workflow dell'integratore. Il caso entra nel test di NFR-8.
  - Funzioni, nodi personalizzati e sotto-workflow citati da un workflow si risolvono tra le
    risorse del proprietario del workflow e, se il proprietario è un tenant, tra quelle che
    l'integratore gli rende disponibili; i segreti si prendono dal proprietario
    dell'esecuzione; nessun riferimento si risolve mai tra le risorse di un altro tenant.
  - Un workflow scrive solo sugli stream del proprietario dell'esecuzione; un worker, mentre
    serve un'attività, scrive solo sugli stream del proprietario di quell'attività. Una
    scrittura verso lo stream di un altro tenant riceve lo stesso errore di uno stream
    inesistente e a quel tenant non arriva alcun evento.
- Ogni trigger (pianificazione, pipeline stream) ha un proprietario fissato alla creazione, un
  tenant o l'integratore, e avvia esecuzioni solo nel suo contesto.

#### FR-27: Identità garantita dalla piattaforma ospite

La piattaforma ospite autentica i propri utenti e, per ogni richiesta al motore, garantisce
l'identità del tenant con una credenziale firmata, a scadenza breve, che porta il tenant e il
ruolo e, se la piattaforma ospite lo vuole, un nome visualizzato.

**Conseguenze:**
- Una richiesta senza identità valida, o con una credenziale scaduta, viene rifiutata.
- Il kit frontend usato dentro la piattaforma ospite non chiede un secondo login all'utente.
- Il nome visualizzato è un dato personale: è mostrato sempre come testo (NFR-7), conservato
  quanto il record che lo contiene e cancellato con lui o con il tenant (FR-31); la
  documentazione per l'integratore lo dice.
- Con la credenziale dell'integratore, il tenant scelto nel kit è un campo della credenziale
  firmata dalla piattaforma ospite, che firma una credenziale nuova a ogni cambio di tenant.
- Viene rifiutata una credenziale di ruolo tenant che porta il campo del tenant scelto, una con
  un tenant scelto inesistente o eliminato (FR-31) e una con un nome visualizzato oltre la
  lunghezza massima o fuori dai caratteri documentati (NFR-14). I tre casi entrano nel test di
  NFR-8.
- Il kit avvisa la piattaforma ospite prima che la credenziale scada e alla scadenza.
  - Il lavoro non salvato torna dopo il rinnovo solo se la credenziale nuova porta lo stesso
    tenant e lo stesso ruolo; altrimenti si scarta senza mostrarlo. Resta solo nella memoria
    della pagina: un ricaricamento lo perde e il kit non lo salva nel browser.
  - A un cambio di tenant il kit scarta lo stato del tenant precedente, eventi in tempo reale
    compresi. Il caso entra nel test di NFR-8.

#### FR-28: Due ruoli

topictoast distingue due ruoli: l'integratore (con l'operatore), che vede tutti i tenant, e il
tenant, che vede solo sé stesso.

**Conseguenze:**
- Solo l'integratore crea e configura tenant e quote.
- Con la credenziale dell'integratore il kit agisce per il tenant indicato nella credenziale
  (FR-27): ogni sezione mostra solo le risorse di quel tenant e i workflow dell'integratore, e
  "Esegui" lancia nel suo contesto.
- Una credenziale dell'integratore firmata per il kit (FR-27) con un tenant scelto vale solo per
  le risorse di quel tenant, per la lettura dei workflow dell'integratore e per l'elenco e la
  ricerca dei tenant; senza tenant scelto vale solo per l'elenco e la ricerca dei tenant. Ogni
  altra operazione riservata all'integratore (tenant, quote, pubblicazione dei workflow
  dell'integratore, risorse di altri tenant, operazioni dell'operatore) viene rifiutata con le
  credenziali del kit (NFR-8). L'elenco dei tenant è riservato all'integratore.
- Un workflow creato nell'editor del kit con la credenziale dell'integratore appartiene al tenant
  indicato nella credenziale. Nel kit non si crea un workflow dell'integratore: quelli disponibili
  a tutti i tenant si creano dall'SDK (FR-35, FR-36) e nel kit sono in sola lettura.

**Fuori perimetro:** ruoli e permessi tra gli utenti di uno stesso tenant: li gestisce la
piattaforma ospite.

#### FR-29: Workflow dell'integratore e del tenant

Un workflow può appartenere all'integratore, ed essere disponibile a tutti i tenant, o a un
singolo tenant.

**Conseguenze:**
- Un workflow dell'integratore eseguito per un tenant gira nel contesto di quel tenant: usa i
  suoi segreti, le sue quote, il suo storico.
- Avviato dalla pianificazione del documento (FR-6), un workflow dell'integratore gira nel
  contesto dell'integratore, con i suoi segreti, e nessun tenant vede l'esecuzione (FR-26).
- Le connessioni richieste da un workflow dell'integratore si risolvono all'avvio tra i segreti
  di chi lo avvia: se nessun segreto soddisfa una connessione, l'avvio viene rifiutato
  prima di essere accettato, con la connessione mancante nel messaggio.
- In un workflow dell'integratore, un task senza manifest, come `call http`, dichiara nel
  documento la connessione che gli serve, risolta allo stesso modo; senza dichiarazione non
  riceve segreti.
- Un tenant non può modificare un workflow dell'integratore.
- Un tenant che crea un nodo personalizzato o un sotto-workflow con lo stesso nome di uno usato da
  un workflow dell'integratore non cambia ciò che quel workflow esegue quando gira per lui.

#### FR-30: Quote per tenant

L'integratore assegna a ogni tenant quote su attività concorrenti, esecuzioni al minuto, risorse
della sandbox (compresa la sua parte del pool della sandbox forte), risorse delle pipeline stream,
eventi scritti al secondo, richieste API al secondo e spazio occupato (stream, storico, code, nodi
caricati).

**Conseguenze:**
- Un tenant oltre quota vede le sue nuove attività in coda; le attività degli altri tenant
  restano entro l'obiettivo di NFR-2, anche quando sono accodate insieme alle sue (UJ-4).
- Superare una quota non perde lavoro accettato.
- L'arretrato di un tenant (attività ed esecuzioni in coda, righe trattenute di FR-24, eventi
  conservati) ha un tetto. Oltre il tetto o la quota di spazio, una nuova richiesta riceve un
  errore di quota prima di essere accettata, e una pipeline stream smette di emettere segnalandolo
  con un errore visibile al tenant e all'operatore; il lavoro già accettato non va perso.
- Un tenant che scrive eventi oltre la sua quota riceve errori di quota, mentre gli altri
  continuano a scrivere e ad avviare esecuzioni entro l'obiettivo di NFR-2.

#### FR-31: Conservazione dello storico

Lo storico delle esecuzioni di ogni tenant ha una durata di conservazione configurabile.
`[ASSUMPTION: 30 giorni come valore predefinito]`

**Conseguenze:**
- Le esecuzioni più vecchie della durata vengono eliminate con i loro input e output.
- Eliminare un tenant ferma prima le sue esecuzioni, pipeline stream e pianificazioni, poi rende
  i suoi dati illeggibili in ogni supporto (database, bus, checkpoint, nodi caricati, sandbox
  riservate) entro un tempo massimo documentato. Passato quel tempo, una ricerca di un valore
  sentinella del tenant nelle directory dati di topictoast e del bus non trova nulla; un
  identificativo di tenant eliminato non viene riusato.

### 4.7 Cassaforte dei segreti

**Descrizione:** topictoast custodisce i segreti di ogni tenant e li consegna solo al momento
dell'esecuzione, solo all'attività che li usa. Nella v1 i segreti sono chiavi API; OAuth arriva
dopo. Realizza UJ-2.

#### FR-32: Salvataggio di un segreto

Il tenant può salvare un segreto con un nome e la connessione che soddisfa (FR-55), dall'editor
(componente React) o tramite SDK.

**Conseguenze:**
- Dopo il salvataggio il valore non è più leggibile da nessuna API, componente o dashboard; si
  vedono solo nome, connessione, data e ultimo uso.
- Il selettore dei segreti del kit mostra solo i segreti che soddisfano la connessione richiesta;
  per un task senza manifest, come `call http`, mostra tutti i segreti del tenant, e l'attività
  riceve solo quello scelto (FR-33).
- Le connessioni tra cui il tenant sceglie sono solo quelle dei manifest a lui disponibili: le
  funzioni che l'integratore gli rende disponibili e i suoi nodi personalizzati. L'elenco entra
  nel test di NFR-8.
- Sostituire il valore di un segreto non richiede di modificare i workflow che lo usano.

#### FR-33: Consegna al momento dell'esecuzione

Un task dichiara i segreti che gli servono; il worker o la sandbox li ricevono insieme
all'attività.

**Conseguenze:**
- Un'attività riceve solo i segreti dichiarati dal suo task e appartenenti al proprietario
  dell'esecuzione (FR-26).
- I segreti non compaiono nei log, nello storico, negli output né negli errori di topictoast.
- Le connessioni di ogni task sono fissate nella versione pubblicata del workflow (FR-5) e
  controllate sui manifest alla pubblicazione; il segreto che le soddisfa si sceglie come dicono
  FR-29 e FR-40. Un manifest registrato dopo non aggiunge connessioni né segreti a quella
  versione: se chiede una connessione in più, il task fallisce con l'errore e non riceve altri
  segreti. Il caso entra nel test di NFR-8.
- Un avvio con una connessione senza segreto viene rifiutato prima di essere accettato, con la
  connessione nel messaggio (FR-29).
- Un segreto eliminato durante un'esecuzione fa fallire alla consegna l'attività che lo usa, con
  il motivo.

#### FR-34: Protezione a riposo

I segreti sono cifrati a riposo.

**Conseguenze:**
- Nessun supporto persistente di topictoast (database, bus, checkpoint, log, file temporanei)
  contiene un segreto in chiaro: una copia di questi supporti senza la chiave di cifratura non
  rivela alcun segreto. Dopo un'esecuzione con un segreto sentinella, una ricerca del valore nelle
  directory dati di topictoast, del bus e dei checkpoint non trova nulla.
- La chiave di cifratura non è raggiungibile dai nodi personalizzati (FR-16).
- La chiave di cifratura non sta nel database né nelle sue copie; cosa succede se si perde è
  documentato per l'integratore.
- L'integratore può ruotare la chiave di cifratura senza fermare il motore e senza perdere segreti;
  dopo la rotazione la chiave vecchia non decifra nulla di ciò che è salvato.

### 4.8 SDK

**Descrizione:** gli SDK TypeScript e Rust servono all'integratore per scrivere worker e workflow
come codice, inviare documenti OWS mentre il sistema gira, gestire tenant, segreti e pipeline
stream. Il percorso "documento OWS a runtime" è quello con cui arrivano al motore i workflow
composti dai tenant nell'editor. Realizza UJ-1.

#### FR-35: Workflow come codice

L'integratore può definire un workflow nel proprio linguaggio (TypeScript o Rust) e pubblicarlo.

**Conseguenze:**
- Un workflow definito come codice produce un documento OWS equivalente, valido secondo lo schema
  di topictoast, leggibile ed esportabile.
- Nell'editor un workflow definito come codice è in sola lettura per tutti, con la provenienza
  visibile; si modifica solo da codice.

#### FR-36: Documenti OWS a runtime

L'integratore può inviare un documento OWS, con l'estensione DAG, e ottenere un workflow eseguibile
senza riavviare nulla.

**Conseguenze:**
- Il workflow è eseguibile appena l'invio risponde con successo.

#### FR-37: Worker e funzioni

L'integratore può scrivere un worker che registra funzioni tipizzate, in TypeScript o Rust.

**Conseguenze:**
- Una funzione tipizzata dichiara il suo manifest (FR-55).
- Una funzione riceve input, segreti dichiarati e contesto dell'esecuzione (proprietario,
  esecuzione, tentativo) e restituisce un output o un errore.
- La documentazione per l'integratore spiega di ricavare il proprietario dal contesto
  dell'esecuzione, mai dall'input, e l'SDK espone il proprietario solo da lì.

#### FR-38: Amministrazione

L'integratore può creare e configurare tenant, quote, segreti e pipeline stream tramite SDK.

**Conseguenze:**
- Tutto ciò che si fa da editor e dashboard si può fare anche tramite SDK.
- Ogni tenant ha un nome, con cui il kit lo mostra e lo cerca (FR-28).

### 4.9 Kit frontend

**Descrizione:** la libreria JavaScript ha due strati. Il nucleo JS, senza interfaccia, gestisce il
modello dei workflow e parla con il motore: lo usa anche chi non usa React. Sopra, i componenti
React da incorporare nella piattaforma ospite: elenco dei workflow, editor, pianificazioni, vista
delle esecuzioni, gestione dei segreti, editor delle pipeline stream, riuniti anche nell'area
workflow (FR-57, FR-58). I tenant non devono accorgersi che sotto c'è topictoast.
L'editor è costruito nel kit: Workflow Builder, legato a Temporal e con un solo nodo di partenza,
resta un riferimento. Realizza UJ-1, UJ-2, UJ-3.

#### FR-39: Nucleo JS

Lo sviluppatore della piattaforma ospite può leggere, costruire, validare e inviare workflow e
seguire le esecuzioni dal nucleo JS, senza React.

**Conseguenze:**
- La validazione del nucleo JS dà gli stessi esiti della validazione del motore (FR-4) sugli stessi
  documenti, a parità di contesto (manifest, segreti, funzioni disponibili); se il contesto cambia
  tra le due, vale l'esito del motore.

#### FR-40: Editor dei workflow

Il tenant può comporre un workflow come grafo: aggiungere task (funzioni dei worker, nodi
personalizzati e ogni altro costrutto di FR-1), collegarli, configurare fan-out e join, scegliere
i segreti. Realizza UJ-2.

**Conseguenze:**
- L'editor mostra solo funzioni, nodi personalizzati e segreti disponibili al tenant.
- Il tenant definisce lo schema di input del workflow (FR-56) dall'editor.
- Un campo dello schema di input di un task prende il valore da una fonte: un valore fisso, un
  campo compatibile dell'input del workflow (FR-56), dell'output di un task a monte o
  dell'elemento corrente di un fan-out (FR-3), oppure un'espressione jq come modo avanzato. Un
  campo obbligatorio senza fonte è un errore. Per un sotto-workflow lo schema di input è quello
  del workflow chiamato (FR-56). Il workflow di UJ-2 si compone senza scrivere jq.
- Ogni connessione richiesta dalla funzione o dal nodo personalizzato di un task (FR-55) ha un
  segreto, scelto tra quelli che la soddisfano (FR-32).
- Un grafo non valido mostra l'errore sul task che lo causa prima del salvataggio; tipi, campi
  obbligatori e connessioni senza un segreto si controllano con gli stessi esiti dell'invio
  (FR-4).
- Il workflow salvato dall'editor è un documento OWS valido secondo lo schema di topictoast
  (sottoinsieme OWS più estensione DAG).

#### FR-41: Vista delle esecuzioni

Il tenant può seguire un'esecuzione in tempo reale: stato di ogni task, attività del fan-out,
tentativi, input, output ed errori.

**Conseguenze:**
- Un cambio di stato appare nella vista entro 1 secondo.
- Un fan-out da migliaia di elementi resta leggibile (attività raggruppate con conteggi per stato).
- Il tenant lancia un workflow dal kit con un modulo generato dallo schema di input (FR-56), con
  il JSON come modo avanzato; un workflow senza schema si lancia solo con il JSON.
- Un lancio dal kit porta una chiave di idempotenza per ogni intenzione di lancio (FR-6):
  ripeterlo, anche dopo un errore di rete o un rinnovo della credenziale, restituisce la stessa
  esecuzione.
- Con il ruolo tenant la vista non mostra identificativi dei worker.
- Durante la ripresa dopo un crash (NFR-15) la vista dice in modo neutro che gli aggiornamenti
  sono in pausa e riprendono da soli; stati e righe restano visibili; nessun testo nomina il
  motore o il crash.

#### FR-42: Gestione dei segreti

Il tenant può aggiungere, sostituire ed eliminare i propri segreti da un componente React.

**Conseguenze:**
- Il componente non mostra mai il valore di un segreto salvato (FR-32).

#### FR-43: Editor delle pipeline stream

Il tenant può scrivere, validare, attivare e sospendere le proprie pipeline stream, scegliere la
destinazione e vedere stato e ultime righe emesse. Realizza UJ-3.

**Conseguenze:**
- Gli errori di validazione della query compaiono con riga e colonna prima del salvataggio.
- Durante la ripresa dopo un crash (NFR-15) l'editor dice in modo neutro che gli aggiornamenti
  sono in pausa e riprendono da soli; stato e ultime righe emesse restano visibili; nessun testo
  nomina il motore o il crash.

#### FR-44: Aspetto personalizzabile

Lo sviluppatore della piattaforma ospite può adattare colori, caratteri, testi e lingua dei
componenti React alla propria piattaforma.

**Conseguenze:**
- Nessun componente mostra il nome o il logo di topictoast senza un'opzione esplicita; un
  controllo automatico sul bundle lo verifica.
- I testi sono tutti sostituibili; un testo mancante o vuoto ricade sul predefinito della lingua;
  la v1 include italiano e inglese.
- Il kit segnala le coppie di colori della piattaforma ospite sotto la soglia di contrasto
  (NFR-9): errore in sviluppo, avviso in produzione.

#### FR-57: Pianificazioni nel kit

Il tenant può creare, vedere, modificare, sospendere, riprendere ed eliminare dal kit la propria
pianificazione di un workflow (FR-6), con il suo input e la prossima scadenza.

**Conseguenze:**
- La pianificazione del kit è un trigger del tenant (FR-26), non una parte del workflow:
  cambiarla non modifica il workflow. È distinta dalla pianificazione del documento (FR-1,
  FR-6), che il tenant vede nel kit in sola lettura, come il resto di un workflow definito come
  codice.
- Il tenant ha al massimo una pianificazione per workflow.
- Il tenant pianifica anche i workflow dell'integratore e i workflow definiti come codice, che
  restano in sola lettura (FR-29, FR-35).
- Le stesse operazioni si fanno tramite SDK (FR-38).

#### FR-58: Area workflow ed elenco dei workflow

Lo sviluppatore della piattaforma ospite può incorporare un componente unico, l'area workflow, con
navigazione interna tra elenco dei workflow, editor con la pianificazione, esecuzioni, segreti e
pipeline stream; ogni sezione esiste anche come componente singolo. Realizza UJ-2.

**Conseguenze:**
- L'elenco mostra i workflow del tenant e quelli dell'integratore, con la provenienza.
- Dall'elenco il tenant apre un workflow, ne crea uno nuovo o elimina uno dei suoi;
  l'integratore elimina i propri tramite SDK (FR-38).
- Eliminare un workflow lascia finire le esecuzioni in corso, elimina le sue pianificazioni e
  lascia lo storico (FR-31). Se un altro workflow lo usa come sotto-workflow o una pipeline stream
  lo ha come destinazione, l'eliminazione viene rifiutata con l'elenco di chi lo usa.
- L'area mostra solo le risorse del tenant e i workflow dell'integratore (FR-26, FR-29), e
  nessun marchio di topictoast (FR-44).
- Finché esiste solo il tenant predefinito (§4.6), l'area non mostra la scelta del tenant.

### 4.10 Dashboard

**Descrizione:** la dashboard è l'interfaccia web dell'operatore, distribuita con topictoast. Mostra
worker, esecuzioni e pipeline stream di tutti i tenant; finché esiste solo il tenant predefinito
(§4.6) non mostra filtri, colonne e nomi di tenant. Realizza UJ-4.

#### FR-45: Accesso

L'operatore accede alla dashboard con una credenziale generata al primo avvio; non esiste una
password predefinita.

**Conseguenze:**
- Senza credenziali valide la dashboard non mostra alcun dato.
- Alla scadenza della sessione la dashboard torna alla pagina d'accesso senza mostrare dati.
- La pagina d'accesso dice dove è stata stampata la credenziale del primo avvio, senza mostrarla.
- La credenziale del primo avvio non compare mai nei log (FR-52).

#### FR-46: Esecuzioni

L'operatore vede le esecuzioni di tutti i tenant, con filtri per tenant, workflow e stato
(compreso "in coda") e l'ordinamento per tempo in coda.

**Conseguenze:**
- Il tempo in coda è l'età del più vecchio tra i task e le attività in coda dell'esecuzione,
  anche per un'esecuzione già "in esecuzione". Un'esecuzione senza task né attività in coda non
  ha tempo in coda.
- Da un'esecuzione l'operatore arriva al dettaglio di ogni task, come nella vista delle esecuzioni
  (FR-41), ma senza i valori dei segreti.
- Dal dettaglio di un'esecuzione l'operatore la annulla (FR-10, FR-28).
- Un'azione della dashboard che cambia stato è accettata solo se parte dalla dashboard stessa: una
  richiesta di annullamento da una pagina di un'altra origine viene rifiutata.

#### FR-47: Pipeline stream

L'operatore vede le pipeline stream attive, rallentate e sospese, con la causa, ritardo sul
watermark, eventi al secondo, memoria di stato, eventi in ritardo scartati, ultimo checkpoint.

#### FR-48: Worker

L'operatore vede i worker collegati, le funzioni che registrano, le attività in corso per ogni
worker e da quanto tempo, i worker che non rispondono, e la cronologia recente delle attività di
ogni worker con le fasi: ricevuta, in esecuzione, completata, fallita, ritentata, annullata.

**Conseguenze:**
- La dashboard segnala il disco pieno e il rifiuto di lavoro nuovo (NFR-6) con un avviso che lo
  distingue dalla ripresa dopo un crash del motore (NFR-15).
- Un worker che registra una funzione già registrata con connessioni diverse è segnalato
  all'operatore (FR-33).

#### FR-49: Quote

L'operatore vede per ogni tenant l'uso delle quote e i tenant vicini al limite e al limite.

**Conseguenze:**
- La soglia di "vicino al limite" ha un valore predefinito documentato (NFR-14).

### 4.11 Installazione e funzionamento

**Descrizione:** topictoast si installa con pochi componenti e si configura con valori predefiniti
che funzionano. Il vincolo tecnico indicato da Marco (addendum del brief, §8), da confermare in
architettura, è topictoast più il bus Iggy, con SQLite incorporato o Postgres. RustFS entra nella
v1 solo come archivio facoltativo dello storico dei payload e dei checkpoint (architettura, AD-17);
lo storage per payload grandi e file arriva dopo la v1. Realizza UJ-1, UJ-5.

#### FR-50: Installazione minima

Nella v1 l'integratore installa due servizi, topictoast e il bus, più il database a scelta.
topictoast avvia e ferma da sé i processi ausiliari di cui ha bisogno (la sandbox leggera e, se
l'architettura li separa, i processi che eseguono espressioni jq, query delle pipeline stream e
`call http`). Il numero di processi non è la differenza (§1.1): lo sono RAM e carico sul database.

**Conseguenze:**
- Con SQLite incorporato, topictoast e il bus sono gli unici servizi da installare e avviare; ogni
  altro processo lo avvia e lo ferma topictoast.
- Un processo ausiliario inattivo oltre un tempo configurabile viene fermato e riavviato a freddo
  alla prima richiesta. Il tempo di riavvio è riportato nel benchmark (FR-53); NFR-2 e NFR-12 si
  misurano a regime `[ASSUMPTION: idea di Marco da studiare in architettura, con NFR-1 che conta
  tutti i processi attivi; obiettivo del riavvio a freddo in §11]`.
- Con la configurazione predefinita, un workflow di esempio gira senza modificare nulla.
- Il tenant predefinito esiste dal primo avvio (§4.6). Con la credenziale dell'integratore, un
  workflow pubblicato dall'SDK senza indicare un tenant va al tenant predefinito; con quella di
  un tenant va sempre al tenant della credenziale (FR-26). Renderlo disponibile a tutti i
  tenant, come workflow dell'integratore (FR-29), è una scelta esplicita.
- L'installazione predefinita non richiede Kubernetes, KVM né privilegi di root; solo la sandbox
  forte opzionale (FR-54) richiede KVM.
- Nessuna chiave o credenziale che protegge identità, segreti o dashboard ha un valore predefinito
  noto: due installazioni nuove hanno valori diversi, e topictoast rifiuta di partire con un valore
  presente negli esempi della documentazione. Con la configurazione predefinita topictoast accetta
  connessioni di rete solo da localhost.

#### FR-51: Aggiornamenti

L'integratore aggiorna topictoast a una versione nuova senza perdere stato.

**Conseguenze:**
- L'aggiornamento migra lo stato da solo; le esecuzioni in corso riprendono dopo il riavvio.
- L'aggiornamento di qualunque componente dell'installazione, compreso il bus (che oggi cambia
  formato su disco tra versioni senza migrazione), non perde attività accettate né esecuzioni in
  corso.

#### FR-52: Osservabilità

topictoast espone metriche e log strutturati in formati standard.

**Conseguenze:**
- Le metriche coprono attività al secondo, latenza di avvio delle attività, code per tenant,
  pipeline stream (le stesse grandezze di FR-47).
- I log non contengono segreti, credenziali, nomi visualizzati né input e output delle attività.

#### FR-53: Benchmark riproducibile

Il repository contiene il benchmark che misura i requisiti del §5, eseguibile da chiunque sulla
macchina di riferimento con un solo comando.

**Conseguenze:**
- Il benchmark produce RAM a riposo, latenza p50 e p99 di avvio delle attività, attività al
  secondo, istruzioni SQL per attività, istruzioni SQL al giorno con il carico di
  hatchet-dev/hatchet#5153 (SM-2), eventi al secondo delle pipeline stream.
- Il benchmark riporta attività, esecuzioni ed eventi accettati e poi persi dopo un guasto
  iniettato durante i test di NFR-3 e NFR-5 (obiettivo: zero, NFR-6).
- Il benchmark include una corsa a carico misto: la latenza di NFR-2 misurata mentre una pipeline
  stream gira a un ritmo dichiarato, un tenant è oltre quota e un tenant avvia a ripetizione con
  gli schemi più complessi consentiti (FR-55).
- Il benchmark dichiara forma del carico, dimensione dei payload, insieme di dati e modalità di
  scrittura, e riporta senza obiettivo la RAM sotto carico, la RAM per pipeline stream attiva e il
  tempo di riavvio a freddo dei processi ausiliari (FR-50).
- Il benchmark misura il tempo di ripresa dopo un crash iniettato del motore (NFR-15).
- Le corse di NFR-2 e NFR-3 comprendono il controllo di input e output delle attività sugli
  schemi del manifest (FR-55).

## 5. Requisiti non funzionali trasversali

Tutti misurati con il benchmark di FR-53 sulla macchina di riferimento, con SQLite e con Postgres
sulla stessa macchina, con forma del carico, dimensione dei payload, insieme di dati e modalità di
scrittura dichiarati nel benchmark. La RAM sotto carico e per pipeline stream attiva è riportata
senza obiettivo.

| ID | Requisito | Obiettivo v1 | Come si misura |
|---|---|---|---|
| NFR-1 | RAM a riposo | ≤ 80 MB | somma dell'RSS dei processi di topictoast in esecuzione (il motore e ogni processo ausiliario attivo) e del bus, con SQLite, 60 s dopo l'avvio, con l'insieme di dati dichiarato (tenant, workflow, storico), nessuna esecuzione né pipeline stream attiva; esclusi worker, stato delle pipeline stream e pool della sandbox forte (FR-54) |
| NFR-2 | Latenza di avvio delle attività | p50 ≤ 5 ms, p99 ≤ 25 ms | dal momento in cui l'attività è pronta (dipendenze soddisfatte) a quando il worker la riceve; worker sulla stessa macchina, carico di 1.000 attività/s |
| NFR-3 | Throughput | ≥ 10.000 attività/s | attività vuote (la funzione risponde subito), worker sulla stessa macchina, sostenuto per 10 minuti; il p99 di avvio delle attività è riportato anche a questo ritmo |
| NFR-4 | Carico sul database | ≤ 5 istruzioni SQL per attività | media sull'intero ciclo di vita dell'attività durante il test di NFR-3; il confronto con Hatchet, sullo stesso carico, è in SM-2 |
| NFR-5 | Pipeline stream | `[ASSUMPTION: ≥ 100.000 eventi/s per una pipeline con filtro e conteggio su finestra fissa; da confermare dopo lo spike]` | eventi scritti tramite l'API o l'SDK di FR-18 e consumati dalla pipeline stream, sostenuto per 10 minuti |
| NFR-12 | Sandbox | `[ASSUMPTION: avvio p50 ≤ 10 ms e memoria ≤ 64 MB per istanza della sandbox leggera; ≤ 10 ms per la sandbox forte quando il pool ha una sandbox libera; da confermare dopo lo spike]` | dal momento in cui l'attività è pronta a quando il nodo personalizzato inizia a eseguire; memoria per istanza durante un nodo vuoto |
| NFR-13 | Tempo al primo workflow | `[ASSUMPTION: ≤ 15 minuti]` | da download o clone a un workflow di esempio completato, seguendo la guida rapida, su una macchina pulita, per una persona che non conosce topictoast (UJ-5) |

Riferimenti dei concorrenti in `research-footprint.md`: sono benchmark dei vendor, su hardware
diverso, non confrontabili direttamente. Riferimenti che hanno guidato i numeri: Temporal dev
server circa 150 MB a riposo (misura di un utente, 2023); Windmill circa 12 ms di overhead con
worker dedicato e Restate p50 15 ms (vendor); Hatchet 2.000 run/s con un database da 8 CPU
(vendor).

**Altri requisiti trasversali:**

- **NFR-6. Nessuna perdita di dati.** Nessuna attività, esecuzione o evento accettato va perso per
  un riavvio, per il crash di qualunque processo dell'installazione o per lo spegnimento
  improvviso dell'host. Vale anche durante i test di NFR-3 e NFR-5. Backup e ripristino coerenti di
  database e bus sono fuori dalla v1 (§8.2) e documentati come limite. A disco pieno topictoast
  rifiuta lavoro nuovo con un errore esplicito e non perde quello accettato; ogni scrittura
  rifiutata lascia intatto lo stato precedente.
- **NFR-7. Confini stretti.** Ogni input esterno (API, SDK, documenti, manifest, query SQL, codice
  dei nodi personalizzati) è validato; un input non valido produce un errore di validazione, mai un
  crash del motore. Il contenuto fornito dai tenant (nomi, descrizioni, errori, input, output,
  righe emesse, campi e metadati dei manifest, nomi e descrizioni dei campi degli schemi, nomi
  visualizzati) è mostrato sempre come testo, mai interpretato come HTML o script, nella dashboard
  e nei componenti React. Un test salva un payload di script in ogni campo testuale di un tenant,
  apre ogni pagina della dashboard e ogni componente, con entrambi i ruoli del kit, e verifica che
  nessuno script venga eseguito.
- **NFR-8. Sicurezza.** Isolamento tra tenant (FR-26), sandbox contro codice ostile (FR-16,
  FR-17), segreti cifrati e mai nei log (FR-33, FR-34). Ogni connessione di rete che porta segreti,
  credenziali o dati dei tenant (tra worker e motore, tra motore e bus, tra motore e database
  quando è un servizio di rete, dal browser, dall'SDK e dalla CLI verso il motore) è cifrata e
  autentica il server; una connessione in chiaro è accettata solo da localhost. Con la
  configurazione predefinita, un worker che si collega in chiaro da un altro host viene rifiutato.
  Prima del primo rilascio pubblico:
  - la sandbox, a entrambi i livelli, passa un test con codice ostile noto (fuga dalla sandbox,
    esaurimento di risorse, accesso alla rete interna, lettura della chiave della cassaforte);
  - un test automatico prova, per ogni tipo di risorsa e ogni punto di accesso (API, SDK, eventi
    in tempo reale, liste e conteggi), che un tenant non legge né modifica risorse di un altro,
    che una credenziale di tenant riceve un rifiuto su ogni operazione riservata all'integratore,
    e che una credenziale dell'integratore firmata per il kit lo riceve su ogni operazione fuori
    dal suo ambito (FR-28).
- **NFR-9. Accessibilità.** Tutti i componenti del kit, area workflow compresa, e la dashboard
  rispettano WCAG 2.1 AA, compreso l'uso da tastiera dell'editor del grafo. L'AA è garantito con
  i colori predefiniti; con i colori della piattaforma ospite vale il controllo di FR-44. Un
  workflow e un'esecuzione si leggono anche senza il grafo, in una vista alternativa in sola
  lettura; la modifica resta sul canvas, utilizzabile da tastiera. Verifica: un controllo
  automatico su ogni componente e una prova con NVDA e VoiceOver sui percorsi UJ-2 e UJ-4.
- **NFR-10. Piattaforme.** Linux x86_64 e arm64 `[ASSUMPTION]`.
- **NFR-11. Licenza.** Tutto il codice di topictoast è Apache-2.0, senza parti a pagamento; le
  dipendenze hanno licenze compatibili.
- **NFR-14. Valori predefiniti e limiti.** I valori li fissano le spec delle storie.
  - Ogni campo che un workflow, una pipeline stream, un worker, un nodo personalizzato o la
    configurazione di un nuovo tenant può omettere ha un valore predefinito documentato e
    limitato, applicato dopo la validazione. Vale almeno per: retry, timeout del task e
    dell'esecuzione, limite di attesa prima della consegna, intervallo di battito dei worker,
    concorrenza di un worker, limiti dei nodi personalizzati, ritardo tollerato e tolleranza sul
    tempo dell'evento, chiusura delle finestre di uno stream inattivo, intervallo dei checkpoint,
    conservazione degli stream, recupero delle scadenze perse di una pianificazione, quote di un
    nuovo tenant, soglia di "vicino al limite" (FR-49), funzioni e destinazioni di rete
    disponibili ai tenant, inattività prima di fermare un processo ausiliario.
  - Ogni tempo e ogni limite che un tenant può dichiarare ha un massimo fissato
    dall'installazione, e nessuna combinazione di valori predefiniti e dichiarati lascia
    un'esecuzione, un'attività o una finestra ferma senza fine.
  - Le grandezze che un tenant controlla (elementi di un fan-out, dimensione del documento e
    della query, numero di task, profondità di annidamento e di ricorsione dei sotto-workflow,
    dimensione della risposta di una `call http`, dimensione, profondità e numero di campi di
    manifest e schemi) hanno un limite con valore predefinito documentato: superarlo è un errore
    di validazione o fa fallire il task con limite e valore, senza creare attività e senza
    fermare il motore.
  - Un workflow senza campi facoltativi gira con i valori predefiniti documentati e termina; una
    pipeline stream senza campi facoltativi gira con i valori predefiniti documentati e chiude le
    sue finestre.
- **NFR-15. Ripresa dopo un crash.** Nella v1 il motore è a nodo singolo. Dopo un suo crash
  riparte da solo e torna a consegnare attività entro 30 s, senza intervento; i worker si
  ricollegano da soli, finiscono le attività che avevano e i risultati scritti nel frattempo non
  vanno persi. Durante la ripresa non partono nuove esecuzioni né task successivi: è una pausa,
  non una perdita (NFR-6). Il benchmark lo misura con un crash iniettato (FR-53).
- **NFR-16. Schermi.** Kit frontend e dashboard si usano per intero su desktop e da telefono, da
  360 px di larghezza e con input touch, compresa la composizione nell'editor del grafo e
  nell'editor SQL delle pipeline stream; sul telefono la disposizione del grafo è automatica.

## 6. Superfici pubbliche e compatibilità

topictoast è un prodotto per sviluppatori: le sue superfici pubbliche sono contratti.

- **Superfici pubbliche:** API del motore; protocollo tra worker e motore; SDK TypeScript e Rust;
  CLI; formato dei nodi personalizzati compilati; formato del manifest delle funzioni e dei nodi
  personalizzati (FR-55); nucleo JS; componenti React (proprietà ed eventi); formato dei workflow
  (sottoinsieme OWS ed estensione DAG); dialetto SQL delle pipeline stream; formato di
  configurazione; formato della credenziale firmata dalla piattaforma ospite (FR-27); metriche.
- **SDK di terzi:** l'API e il protocollo tra worker e motore sono documentati in modo da
  permettere SDK in altri linguaggi, anche scritti dalla comunità.
- **Versioni:** semver. Fino al primo rilascio stabile (0.y.z) una rottura di contratto alza la
  minor; dopo, solo la major. Ogni rottura è annunciata nelle note di rilascio con la via di
  migrazione.
- **Documenti salvati:** un workflow accettato da una versione resta eseguibile dalle versioni
  successive della stessa major.
- **Documentazione:** ogni superficie pubblica ha pagine per l'integratore, con esempi eseguibili.

## 7. Non-obiettivi

- topictoast non diventa un'alternativa a n8n per utenti diretti: niente catalogo di nodi pronti
  né connettori nella v1 `[ASSUMPTION: appartengono al possibile progetto separato]`.
- Non è una piattaforma di analisi dati: le pipeline stream reagiscono agli eventi, non
  sostituiscono un data warehouse.
- Gli stream non sono un archivio di eventi: li conservano per pochi minuti (FR-18). Chi vuole
  tenere gli eventi li scrive, con un workflow, verso una propria destinazione.
- Non gestisce utenti e permessi interni ai tenant: lo fa la piattaforma ospite.
- Non richiede né sfrutta Kubernetes.
- Non ha un'edizione a pagamento né funzioni riservate.
- Non garantisce che i workflow con l'estensione DAG girino su altri runtime OWS.

## 8. Perimetro della v1

### 8.1 Dentro la v1

- Motore OWS (sottoinsieme FR-1) con estensione DAG: dipendenze arbitrarie, fan-out dinamico con
  join, retry, timeout, durabilità.
- Worker come funzioni sempre attive, con SDK TypeScript e Rust.
- Nodi personalizzati dei tenant in TypeScript e Rust, già compilati, eseguiti nella sandbox
  leggera o, dove c'è KVM, in quella forte con pool.
- Pipeline stream SQL con stato, finestre, watermark e checkpoint, scritte da integratore e
  tenant, con quote.
- Separazione tra tenant, quote, cassaforte con chiavi API.
- Kit frontend: nucleo JS e componenti React (area workflow con l'elenco dei workflow, editor dei
  workflow, pianificazioni, vista delle esecuzioni, segreti, editor delle pipeline stream), che si
  usano anche da telefono (NFR-16).
- Dashboard dell'operatore, anche da telefono (NFR-16).
- Installazione con due servizi più il database (SQLite o Postgres), guida rapida per il primo
  workflow.
- Benchmark pubblico riproducibile.

### 8.2 Fuori dalla v1

- **OAuth gestito dal motore** — dopo la v1; la v1 parte con le chiavi API.
- **Storage per payload grandi e file** — dopo la v1; finché manca valgono il limite e il tetto del
  join di FR-11. Con lo storage arriva il join che riceve i risultati a pagine o per riferimento.
  RustFS è già nella v1 come archivio facoltativo dello storico dei payload e dei checkpoint
  (architettura, AD-17), senza cambiare il limite di FR-11.
- **Backup e ripristino coerenti di database e bus** — dopo la v1; limite documentato (NFR-6).
- **Rotazione della chiave di firma delle identità** (FR-27) — dopo la v1 `[ASSUMPTION]`; la
  rotazione della chiave della cassaforte invece è nella v1 (FR-34).
- **Nodi personalizzati in Python e Go** — dopo la v1.
- **SDK Python e Go per worker e workflow** — dopo la v1 `[ASSUMPTION]`; il protocollo documentato
  (§6) li rende possibili anche dalla comunità.
- **Connettori degli stream verso sistemi esterni** (Kafka e simili) — dopo la v1.
- **Destinazioni di rete legate a una connessione** (un segreto che viaggia solo verso la sua
  piattaforma) — dopo la v1; nella v1 valgono le destinazioni per tenant (FR-16).
- **Costrutti OWS fuori dal sottoinsieme** (gRPC, OpenAPI, AsyncAPI, A2A, `emit`, `listen`) e
  conformità completa al CTK — dopo la v1.
- **Alta disponibilità del centro (motore e bus)** — dopo la v1; i worker sono già in alta
  disponibilità (FR-12, FR-14) e il motore riparte da solo entro il tempo di NFR-15. Il cluster
  del bus, se serve, si configura dopo la verifica dello spike; oggi non è ancora
  maturo.
- **Nodi pronti, passi LLM, utenti in stile n8n** — nel possibile progetto separato
  `[ASSUMPTION]`.
- **Proposta dell'estensione DAG a OWS** — dopo che l'estensione è provata in produzione.

## 9. Metriche di successo

**Primarie**

- **SM-1. Stelle su GitHub.** Almeno 500 a sei mesi dal primo rilascio pubblico (oggi Workflow
  Builder ne ha 448, Zigflow 242). Valida l'interesse per l'insieme del prodotto.
- **SM-2. Benchmark pubblico del carico sul database.** Pubblicato con il primo rilascio,
  riproducibile con FR-53, in due regimi. Con lo stesso carico di hatchet-dev/hatchet#5153, circa
  680 task al giorno, mostra al massimo 5.000 istruzioni SQL al giorno, accanto ai
  circa 5,2 milioni misurati da un utente su Hatchet con Postgres come coda (le ~7.700 per task di
  quella misura sono soprattutto polling a basso volume). Sotto il carico di NFR-3, mostra al
  massimo 5 istruzioni SQL per attività (NFR-4). Valida FR-9, FR-53.

**Secondarie**

- **SM-3. Obiettivi di prestazione rispettati.** NFR-1, NFR-2, NFR-3 e NFR-5 rispettati nel
  benchmark del primo rilascio e di ogni rilascio successivo. Valida FR-12, FR-13, FR-18–FR-24.
- **SM-4. Percorsi dimostrati.** UJ-1, UJ-2, UJ-3 e UJ-4 eseguiti dall'inizio alla fine prima del
  primo rilascio pubblico, sull'app di Marco. Valida il perimetro della v1.
- **SM-5. Primo workflow in fretta.** NFR-13 rispettato da almeno tre persone che non conoscono
  topictoast, prima del primo rilascio pubblico `[ASSUMPTION]`. Valida FR-50, UJ-5.
- **SM-6. Composizione senza documentazione.** Prima del primo rilascio pubblico, tre persone con
  il profilo di Giulia (UJ-2) compongono, salvano e lanciano dall'editor il workflow di UJ-2
  (almeno 5 task, con fan-out e join) senza leggere documentazione: ciascuna da sola, su desktop,
  entro 20 minuti. Riesce con 3 persone su 3. Valida FR-40, UJ-2.

**Contro-metriche (da non ottimizzare)**

- **SM-C1. Durabilità prima del carico sul database.** Ridurre le istruzioni SQL rinunciando alla
  durabilità è vietato: le attività perse dopo un crash devono restare zero (NFR-6). Bilancia SM-2.
- **SM-C2. RAM a riposo contro latenza sotto carico.** Non si scende sotto gli 80 MB a riposo a
  costo di peggiorare il p99 sotto carico (NFR-2). Bilancia NFR-1.
- **SM-C3. Stelle contro perimetro.** Non si aggiungono funzioni di richiamo (nodi pronti,
  connettori) per guadagnare stelle: spostano topictoast verso n8n. Bilancia SM-1.

## 10. Rischi

1. **Sandbox.** È il rischio più grande: codice ostile accanto ai segreti di tutti. Uno spike prima
   dello sprint 1 sceglie le tecnologie dei due livelli (leggera senza KVM, forte con microVM e
   pool) e verifica che TypeScript e Rust compilati girino nella sandbox leggera. I candidati sono
   nell'addendum del brief.
2. **Stato delle pipeline stream.** Finestre, watermark e checkpoint non sono pronti nelle librerie
   di base e vanno costruiti o riusati da un progetto più grande. Secondo spike prima dello
   sprint 1. Con le pipeline stream scritte dai tenant si aggiungono isolamento e quote per
   pipeline stream.
3. **Il bus è un log, non una coda.** Ack, retry, timeout e ritardi vanno costruiti in topictoast;
   un'attività lenta può bloccare una partizione (FR-13, FR-30); il bus è pre-1.0 e cambia formato
   su disco tra le versioni, e FR-51 chiede che l'aggiornamento non perda attività.
4. **Forma dell'estensione OWS.** Dentro `metadata` il documento resta OWS valido; con chiavi nuove
   no. Decide l'architettura.
5. **Ampiezza della v1.** Motore, DAG, sandbox, pipeline stream, separazione tra tenant,
   cassaforte, kit frontend, dashboard e due SDK sono molto lavoro per un primo rilascio; due dei
   pezzi hanno bisogno di uno spike. Decisione: nessun taglio. Se uno spike fallisce si cerca
   un'altra tecnologia e il rilascio slitta; il perimetro della v1 non si riduce. Dopo la UX la
   v1 cresce ancora, con il costo accettato: editor con tutti i costrutti di FR-1 (FR-40),
   pianificazioni nel kit (FR-57), area ed elenco dei workflow (FR-58), uso completo da telefono
   (NFR-16).
6. **Obiettivi ambiziosi.** NFR-1…NFR-4 sono sotto ogni riferimento pubblico trovato; il primo
   benchmark di base, negli spike di architettura, dice se reggono.

## 11. Domande aperte

1. Caso reale di pipeline stream di Marco (stream, query, cosa succede dopo), per confermare UJ-3.
2. Obiettivo numerico delle pipeline stream (NFR-5) e memoria di stato attesa per pipeline
   stream: dopo lo spike.
3. Confermare NFR-12 (avvio e memoria della sandbox) dopo lo spike.
4. Dove salvare i checkpoint delle pipeline stream nella v1: chiusa dall'architettura. Stanno
   nell'archivio dei payload, su disco o su RustFS facoltativo (AD-17).
5. Nome e confini del possibile progetto separato con i nodi pronti.
6. Connessioni per tenant: il lato di prodotto è chiuso (il segreto dichiara la connessione che
   soddisfa, FR-32, FR-55); come si salvano segreti e connessioni: architettura.
7. Come un task OWS invoca funzioni e nodi personalizzati, come dichiara i segreti e come un
   workflow dichiara lo schema di input (FR-56), dentro il formato: architettura.
8. Regola di propagazione di fallimenti e salti tra dipendenze DAG, e incrocio con `then` e
   `switch`: da chiudere in architettura prima delle storie.
9. Obiettivo per il riavvio a freddo di un processo ausiliario (FR-50): dopo lo spike.
10. Quale manifest vale quando nessun worker della funzione è collegato (la palette dell'editor lo
    mostra comunque) o quando due worker registrano la stessa funzione con manifest diversi
    (FR-12, FR-55): architettura.
11. Cosa succede ai workflow già salvati, alle esecuzioni in corso e alle attività in coda quando
    cambia il manifest di una funzione o di un nodo personalizzato, o lo schema di input di un
    workflow chiamato come sotto-workflow (FR-55, FR-56): architettura.
12. Input di un'esecuzione pianificata: chiusa, la pianificazione porta il proprio input (FR-6,
    FR-57); dove lo porta, dentro il formato, la pianificazione del documento (FR-1):
    architettura.
13. Riga di una pipeline stream che non rispetta lo schema di input del workflow di destinazione:
    chiusa, il rifiuto è registrato con il motivo (FR-6, FR-24).
14. Quale segreto usa un workflow dell'integratore quando più segreti del tenant soddisfano la
    stessa connessione (FR-29, FR-32): spec della storia.
15. Stato di un'attività rimasta in coda oltre il limite di attesa, insieme al valore del limite
    (FR-8): spec della storia.
16. Linguaggio degli schemi di manifest e input (FR-55, FR-56): tipi (intero e decimale), valori
    predefiniti, campi in più, `null` e campo assente, regola di compatibilità tra campi:
    architettura.
17. Se una scadenza rifiutata si ritenta, se dopo più rifiuti la pianificazione si sospende da
    sola con la causa, e dove l'operatore e il proprietario che non usa il kit (l'integratore,
    per la pianificazione del documento) vedono le scadenze rifiutate (FR-6, FR-46): spec della
    storia.
18. Chi sospende e riprende la pianificazione scritta nel documento di un workflow condiviso, e
    da dove (SDK, dashboard): spec della storia.
19. Quote e storico delle esecuzioni che appartengono all'integratore: quale quota le limita
    (FR-30) e dove le vede l'operatore (FR-46): architettura e spec della storia.

Altri punti rinviati dalla revisione (T-30…T-36 e circa 90 rilievi medi e bassi, con il
responsabile di ciascun gruppo) sono in `triage.md`, nella stessa cartella di questo PRD.

## 12. Indice delle assunzioni

- §1, §1.2, §2.2, §7, §8.2 — Nodi pronti, passi LLM e utenti in stile n8n in un progetto separato
  (nome di lavoro flowtoast).
- §2.3 UJ-3 — Esempio di pipeline stream (conteggio per ASN su finestre di 5 minuti).
- FR-1 — `document.dsl` 1.1.x accettato.
- FR-11 — Limite di 1 MB per input e output.
- FR-13 — Soglia di non interferenza uguale all'obiettivo di NFR-2 (anche FR-17, FR-25, FR-30).
- FR-18 — Nessun connettore esterno per gli stream nella v1.
- FR-20 — Finestre fisse e scorrevoli nella v1, sessione dopo.
- FR-22 — Checkpoint ogni 10 secondi come predefinito.
- FR-23 — Modifica della query = nuova versione con stato vuoto.
- FR-31 — Conservazione dello storico 30 giorni come predefinito.
- FR-50 — Processi ausiliari fermati quando inattivi e riavviati a freddo; NFR-1 conta tutti i
  processi attivi.
- NFR-5 — 100.000 eventi/s per pipeline stream.
- NFR-12 — Sandbox: avvio ≤ 10 ms, ≤ 64 MB per istanza leggera.
- NFR-13 — Tempo al primo workflow ≤ 15 minuti.
- NFR-10 — Linux x86_64 e arm64.
- §8.2 — SDK Python e Go per worker e workflow dopo la v1.
- §8.2 — Rotazione della chiave di firma delle identità dopo la v1.
- SM-5 — Tre persone esterne per misurare NFR-13.
