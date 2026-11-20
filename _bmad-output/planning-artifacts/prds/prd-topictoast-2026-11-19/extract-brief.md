# Estratto dal brief per il PRD di topictoast

Fonti: brief = brief.md (final, 2026-11-19); addendum = addendum.md (sezioni 1-8); memlog = .memlog.md.
Solo estrazione, nessuna invenzione. Le sezioni "da verificare" dell'addendum restano tali.

## 1. Visione e posizionamento

topictoast e' un motore di workflow open source (Apache-2.0), in Rust, pensato per essere incorporato in altre piattaforme. Chi lo integra offre ai propri clienti un editor no-code per comporre pipeline eseguite come DAG, in parallelo anche su elenchi di dimensione variabile. I workflow sono documenti Open Workflow Specification (OWS) con un'estensione per i DAG completi.

- Visione a 2-3 anni: motore incorporato in diverse piattaforme, non solo quella dell'autore; con nodi di base (cicli, if/else, trasformazioni, connettori, LLM), OAuth gestito e nodi Python/Go diventa alternativa open a n8n e Activepieces [brief §Visione]
- Agenti: stesse pipeline per costruirli (passi LLM, chiamate a strumenti, codice cliente in sandbox, credenziali custodite); sandbox, cassaforte e DAG ci sono dalla v1, i passi LLM arrivano con i nodi di base [brief §Visione]
- L'estensione DAG, una volta provata, puo' essere proposta alla spec OWS (issue #1143 sul parallelo dinamico) [brief §Visione]
- Posizionamento: "kit incorporabile nativo del motore, licenza permissiva, DAG al centro"; "Non esiste niente" non regge [addendum §4]
- Spazio ipotizzato dal ricercatore (da mettere alla prova): binario Rust unico, documento OWS come definizione nativa, DAG YAML di Argo senza Kubernetes, CTK come prova di conformita' [addendum §3]
- Posta in gioco: progetto open source; riformulato come combinazione unica (leggero a 2 componenti, DAG su OWS, componenti React incorporabili) perche' Hatchet, Temporal, Argo, Kestra sono gia' open [memlog]
- Obiettivi dichiarati dall'utente: veloce, facile da configurare, facile da usare, buon numero di SDK [memlog]

## 2. Utenti e ruoli

- Chi integra topictoast (utente principale): sviluppatori che costruiscono una piattaforma e vogliono offrire pipeline configurabili senza scrivere motore e interfaccia [brief §Per chi]
- Primo utente reale: l'autore, sviluppatore ed esperto di sicurezza, che costruisce un'app di threat intelligence; funziona se incorpora i componenti React, collega i worker e i suoi clienti creano pipeline senza che lui scriva codice per ognuna [brief §Per chi]
- Utente finale: i clienti della piattaforma; compongono pipeline con editor no-code, le eseguono su worker personalizzati in sandbox, non devono sapere che sotto c'e' topictoast [brief §Per chi]
- Chi gestisce (operatore): usa la dashboard per vedere i worker attivi e ogni fase del loro lavoro [brief §La soluzione, punto 4]
- Non pubblico principale: chi usa direttamente la dashboard per crearsi workflow come con n8n; "n8n e' troppo avanti per inseguirlo"; nodi di base pronti nella visione [brief §Per chi; memlog]
- Due livelli di utenti: chi integra topictoast e utenti finali della piattaforma [memlog]
- Scenario reale (caso concreto): arriva un indicatore, verifica in parallelo su DNS e Censys, arricchimento, inserimento; a seconda della configurazione del cliente l'esecuzione riceve un indicatore o un elenco da elaborare in parallelo [brief §Il problema; memlog]
- Scenario "giorno in cui funziona": un cliente dell'autore apre l'editor, compone la pipeline, collega le proprie credenziali Censys, aggiunge un suo nodo personalizzato, la lancia, ne segue l'esecuzione e vede solo le proprie; l'autore dalla dashboard vede i worker [brief §La soluzione]
- Multi-tenancy: ogni cliente carica nodi propri (codice in sandbox), configura le proprie integrazioni, vede solo storico delle proprie esecuzioni e dei propri workflow [brief §Il problema; memlog]
- Nessuna persona nominata oltre all'autore; nessun ruolo RBAC definito (non stabilito nelle fonti)

## 3. Problema e alternative

Chi vuole offrire pipeline configurabili ai clienti deve assemblare tre pezzi che nessun progetto open fornisce insieme: motore, interfaccia no-code, separazione tra clienti [brief §Il problema].

- Status quo dell'autore: Hatchet + RabbitMQ bloccato alla 4.2.9, perche' con l'ultima versione Hatchet non funziona (issue hatchet-dev/hatchet#5501, aperta su RabbitMQ 4.4: "RabbitMQ 4.4 rifiuta le code dichiarate da Hatchet") [brief §Il problema; addendum §3; memlog]
- Hatchet senza RabbitMQ usa Postgres come coda: circa 7.700 istruzioni SQL e 600 KB di WAL per task, 5,2 milioni di istruzioni/giorno per circa 680 task (hatchet-dev/hatchet#5153); il maintainer non lo considera un bug e consiglia RabbitMQ [brief §Il problema; addendum §3]
- Verifica 2026-11-18: la critica "Hatchet dipende da RabbitMQ" e' SUPERATA IN PARTE: Postgres puo' fare da broker (`SERVER_MSGQUEUE_KIND=postgres`, opzionale), default RabbitMQ; hatchet-lite "for development and low-volume use-cases" [addendum §3]
- Argo: solo Kubernetes (vale); "un pod per worker" vale in parte (template HTTP e plugin in un unico pod Agent; ContainerSet "tens rather than hundreds"); limite 1 MB di etcd [addendum §3]
- Temporal: pesante, 4 servizi (Frontend, History, Matching, Worker) piu' DB; Elasticsearch non piu' necessario; dev server "not intended for production use"; ha gia' External Storage su S3/GCS/Azure [brief §Il problema; addendum §3]
- Kestra: tenant solo EE/Cloud; segreti OSS = variabili d'ambiente non cifrate [addendum §5]
- Windmill: spazi separati, segreti cifrati, OAuth, sandbox (nsjail spento di default) in Community, ma AGPLv3; 3 workspace in Community [brief §Cosa lo rende diverso; addendum §5]
- Activepieces (MIT + `ee`): sandbox e connessioni nel nucleo, ma codice privato dei clienti e gestori di segreti a pagamento [brief; addendum §5]
- n8n: Sustainable Use License (non OSI), embedding a pagamento (licenza Embed) [addendum §3, §4]
- Interfacce incorporabili: Trigger.dev e Inngest solo hook; Hatchet, Temporal, Argo UI chiuse nella propria app; Kestra grafo Vue legato al suo design system; Windmill iframe o licenza commerciale; Conductor `orkes-workflow-visualizer` [addendum §4]
- Workflow Builder (Synergycodes, `@workflowbuilder/sdk` 3.0.0, Apache-2.0, 448 stelle): editor React white-label su React Flow, ma `@workflowbuilder/temporal` lo lega a Temporal; un solo nodo di partenza, niente cicli [brief; addendum §4]
- Altri OWS: Zigflow (Go su Temporal, Apache-2.0, 242 stelle, v0.18.1) con stessi limiti DAG e campi Temporal obbligatori; runtime OWS esistenti JVM o di nicchia; nessun runtime Rust ufficiale (esiste `swf-runtime` comunitario, alpha) [addendum §1]
- Rust: nessun orchestratore maturo e autonomo con DAG al centro; solo progetti piccoli (dagrs, apalis-workflow, IronFlow, Operon, treadle) [addendum §3]
- Cosa NON differenzia: DAG, YAML, database esistono (Kestra, Windmill, Argo); binario unico + DB pure (Hatchet, Windmill, Restate); Rust da solo non convince; con SQLite topictoast ha 2 processi come Hatchet con solo Postgres [brief §Cosa non lo rende diverso; addendum §3; memlog]
- Parti difficili dove Temporal, Restate e Hatchet sono avanti: esecuzione distribuita, retry, interfaccia, connettori [addendum §3]
- Nota: gran parte della tabella comparabili viene da conoscenza del modello e e' "da verificare" [addendum §3]

## 4. Capacita'/funzionalita' citate

Authoring / DSL
- Workflow come documenti OWS (spec CNCF); OWS 1.1.0 stabile dal 2026-11-12 [brief; addendum §1]
- Estensione per DAG completi (dipendenze tra task, parallelo su elenchi): i workflow che la usano non girano su altri runtime [brief §Cosa lo rende diverso]
- Retry e timeout [brief §Prima versione]
- OWS obbliga i runtime conformi a: espressioni jq, meccanismo segreti, timeout, task `run` e `call` (HTTP, gRPC, OpenAPI, AsyncAPI, A2A), schedule every/cron/after/on, eventi CloudEvents emit/listen [addendum §1] (conformita' completa: non dichiarata come requisito v1)

DAG e parallelismo / fan-out
- Dipendenze arbitrarie tra task (forma a "N") non coperte da OWS: serve estensione [addendum §1]
- Fan-out dinamico in parallelo su elenco di dimensione variabile: REQUISITO deciso; il come resta all'architettura [memlog; brief]
- OWS 1.1.0: `for` sequenziale; `fork.branches` statico; solo l'avvio con `for` + `run.workflow` `await:false`, manca il join [addendum §1]
- Via indicata dai maintainer (#1133): eventi + sotto-workflow + listen, possibile con bus eventi interno [addendum §1]
- Modi di estensione: `metadata` (documento resta valido, degrado su altri runtime solo se task in ordine topologico e dati via `$context`: ipotesi) oppure chiavi nuove (task chiusi, `unevaluatedProperties: false`: documento non piu' OWS valido) [addendum §1; brief §Rischi]

Esecuzione e trasporto
- Attivita' sul bus Iggy; il database tiene solo lo stato [brief §In sintesi]
- RustFS (S3-compatibile) per payload grandi e file: opzionale, dopo la v1 [brief; addendum §8]
- Ack, retry, timeout e ritardi dei task costruiti in topictoast (Iggy e' un log append-only) [brief §Rischi; addendum §2]

Multi-tenancy e sicurezza
- Separazione tra clienti: dati, esecuzioni, storico, segreti; quote per cliente (task, concorrenza, risorse sandbox) [addendum §7]
- Cassaforte credenziali: segreti e token per cliente, rinnovati e consegnati al worker solo all'esecuzione, cifrati a riposo, mai nei log; v1 solo chiavi API, OAuth dopo [brief; addendum §7]
- Sandbox per nodi personalizzati, eseguita da topictoast, deve reggere codice ostile; nessun accesso a segreti, rete interna, dati di altri clienti [brief; addendum §7]
- Nodi personalizzati: TypeScript e Rust in v1; Python e Go dopo [brief; memlog]

API / SDK
- SDK TypeScript e Rust: workflow as code, oppure invio di documento OWS per creare workflow a runtime (percorso delle pipeline costruite dai clienti nell'editor) [brief §La soluzione, §Prima versione]
- SDK ufficiali dei concorrenti per confronto: elenco in addendum §3 (non requisito)

Dashboard / editor (libreria JS in due strati)
- Nucleo JS senza interfaccia: gestione DSL e client del motore, usabile senza React [brief §La soluzione]
- Componenti React: costruire workflow (editor), seguire stato delle esecuzioni, configurare segreti e accessi a piattaforme esterne [brief §La soluzione]
- Dashboard di chi gestisce: worker attivi e ogni fase del loro lavoro [brief §La soluzione]
- Librerie riusabili citate: `@xyflow/react` (MIT), dagre (MIT) [addendum §4]

Osservabilita'
- Vista esecuzioni (stato, in tempo reale); log/DAG nel kit [brief; addendum §4]; nessuna metrica/tracing specifica indicata

Persistenza e deploy
- Database a scelta: SQLite incorporato o Postgres; il database tiene solo lo stato [brief §La soluzione]
- Installazione: componenti obbligatori topictoast + Iggy (+ DB); RustFS solo se serve [brief; memlog]

## 5. Scope MVP vs dopo vs fuori scope

Dentro v1 [brief §Prima versione; memlog "Taglio approvato"]
- Motore OWS con estensione DAG (dipendenze tra task e parallelo su elenchi), retry, timeout
- Installazione topictoast + Iggy + SQLite o Postgres
- Separazione tra clienti, cassaforte dei segreti (solo chiavi API)
- Sandbox per codice clienti eseguita da topictoast
- Nucleo JS + componenti React: editor workflow e vista esecuzioni
- Dashboard di chi gestisce
- SDK TypeScript e Rust; nodi custom in TypeScript e Rust

Dopo
- Nodi di base pronti (cicli, connettori, LLM; visione cita anche if/else, trasformazioni)
- RustFS per payload grandi e file
- OAuth gestito dal motore
- Utenti "tipo n8n"
- Nodi custom in Python e Go
- Passi LLM per agenti (con i nodi di base)
- Proposta dell'estensione DAG a OWS upstream

Fuori scope esplicito
- Inseguire n8n come prodotto per utenti diretti [brief §Per chi]
- Funzioni a pagamento nel nucleo ("niente funzioni a pagamento dentro") [brief §Licenza]
- Nessun elenco di "non-goal" ulteriore nelle fonti

## 6. Vincoli e requisiti non funzionali

- Licenza Apache-2.0 per tutto il nucleo; condizione per la differenza sulla separazione tra clienti; e' la licenza di Iggy, RustFS e OWS [brief §Licenza]
- Dipendenze: Apache Iggy (0.10.0 del 2026-11-07, Apache-2.0, ~20 MB RAM all'avvio, pre-1.0, formato su disco cambia fra versioni senza migrazione dalla 0.9.x, cluster VSR non maturo) [addendum §2]
- RustFS 1.0.6 GA (2026-11-15), Apache-2.0; riparazione dati ancora in assestamento [addendum §2]
- Database al minimo: il carico non deve stare sul database [addendum §8]
- Performance/memoria/scalabilita': "differenza dichiarata ma senza obiettivo numerico: i numeri vanno fissati nel PRD" [brief §Rischi]; "Rust da solo non convince" [brief]
- Riferimento numerico: Hatchet con Postgres come coda ~7.700 istruzioni SQL e 600 KB di WAL per task [brief; addendum §3]; il target di topictoast e' "molto sotto" (nessun numero)
- Memoria a riposo e avvio task: erano candidati come criteri con "obiettivo nel PRD" [memlog] (poi non tenuti nei criteri finali)
- Sicurezza: segreti cifrati a riposo, consegnati solo per l'esecuzione che li usa, mai nei log; isolamento forte tra clienti; sandbox contro codice ostile accanto ai segreti di tutti [addendum §7; brief]
- Sandbox: criteri per scegliere = forza isolamento, tempo di avvio per task, linguaggi, requisiti di installazione (KVM/hypervisor/permessi), memoria per sandbox [addendum §6]
- Installazione: "binario unico, facile da installare" (criterio citato dalla ricerca sandbox) [addendum §6]
- Conformita' OWS: CTK (Gherkin, senza runner) superarlo e' SHOULD; SDK Rust OWS alpha non allineato alla 1.1.0, nessun motore [addendum §1]
- Deploy: 2 processi con SQLite (topictoast + Iggy) [memlog; brief]; niente Kubernetes
- Dipendenza da OWS: documenti con estensione non portabili su altri runtime [brief]

## 7. Metriche di successo e contro-metriche

A sei mesi dal primo rilascio pubblico [brief §Criteri di successo]
- Almeno 500 stelle su GitHub (riferimenti: Workflow Builder 448, Zigflow 242)
- Benchmark pubblico del carico sul database: istruzioni SQL per task "molto sotto le circa 7.700 di Hatchet con Postgres come coda" (soglia numerica non fissata)
- Scartati: 3 progetti esterni (troppo difficile in 6 mesi); app dell'autore in produzione; memoria a riposo/avvio task come criteri [memlog]
- Contro-metriche: nessuna dichiarata nelle fonti
- Criterio qualitativo "giorno in cui funziona" (scenario sezione 2) [brief §La soluzione]

## 8. Decisioni gia' prese

- Licenza Apache-2.0 - motivo: la differenza sulla separazione clienti regge solo con licenza permissiva e tutto nel nucleo open [brief; memlog]
- Formato workflow = OWS - motivo: standard CNCF pronto, niente DSL da progettare; non e' motivo di scelta per chi integra [brief; memlog]
- Fan-out dinamico e' requisito, quindi serve estensione OWS; il come (metadata, chiavi nuove, proposta upstream) lo decide l'architettura [memlog; brief]
- Preferenze tecniche utente (da confermare in architettura): Iggy come bus, RustFS per payload grandi/file, database minimo [memlog; addendum §8]
- Componenti: topictoast + Iggy obbligatori, DB SQLite o Postgres, RustFS opzionale - motivo: risposta all'obiezione "carrozzone" [memlog]
- Libreria JS in due strati (nucleo + React) [memlog]
- Credenziali conservate e rinnovate da topictoast; v1 solo chiavi API [brief; memlog]
- Sandbox eseguita da topictoast (non da chi integra) [brief; memlog; addendum §7]
- Nodi custom v1: TypeScript e Rust; SDK v1: TypeScript e Rust [memlog]
- Utenti tipo n8n non target principale - motivo: n8n troppo avanti; Activepieces e' gia' l'alternativa permissiva [memlog; addendum §3]
- Differenze in ordine di importanza: kit React, database al minimo, velocita'/memoria/scalabilita', separazione clienti (OWS spostato a "scelta di progetto") [brief; memlog]
- Criteri successo finali: 500 stelle + benchmark pubblico database (override utente, poi tolti i 3 progetti esterni) [memlog]
- Suggerimento della ricerca (non decisione): wasmtime come sandbox predefinita, Firecracker opzionale [addendum §6; memlog]
- Status quo: Hatchet + RabbitMQ 4.2.9 [memlog]
- Brief mantiene le 4 parti (Cosa non lo rende diverso, giorno in cui funziona, nota OWS, caso concreto) [memlog]

## 9. Domande aperte / assunzioni / rischi

- Sandbox: tecnologia da scegliere con spike prima dello sprint 1 (candidata principale Wasm/wasmtime; alternative nsjail, Hyperlight, Firecracker, Nanos); rischio piu' grande; owner: architettura/spike [brief §Rischi; addendum §6]
- Da verificare: storico advisory wasmtime e memoria dei guest Python/JS; limiti delle librerie native per linguaggio con Wasm [addendum §6]
- Iggy e' log, non coda: ack, retry, timeout, ritardi da costruire; task lento blocca la partizione; Iggy 0.x con cambi di formato; owner: architettura [brief; addendum §2]
- Estensione OWS: dentro `metadata` o chiavi nuove (con chiavi nuove i documenti non sono piu' OWS validi); owner: architettura [brief; memlog]
- Ipotesi da verificare: altri runtime ignorano `metadata` e degradano in sequenza (solo con ordine topologico e dati via `$context`) [addendum §1]
- Emit/listen come meccanismo di dipendenza non garantito dalla spec [addendum §1]
- Velocita'/memoria/scalabilita' senza obiettivo numerico: da fissare nel PRD [brief §Rischi]
- Workflow Builder: competere o integrarsi (adattatore verso topictoast al posto di Temporal) [brief; addendum §4]
- Quali linguaggi dei nodi custom e quanti entrano oltre TS/Rust: v1 deciso TS+Rust; Python/Go dopo [memlog]
- Assunzione: con SQLite 2 processi come Hatchet+Postgres, la leggerezza va argomentata su memoria e carico [memlog]
- Da verificare: sandbox Windmill per edizione, rinnovo OAuth di n8n e Activepieces, separazione clienti in Trigger.dev, segreti per tenant in Hatchet [addendum §5]
- Citazioni OWS da ricontrollare sul testo originale prima di finire in un'ADR [addendum §1]
- Cluster Iggy non maturo (replica che non torna blocca la partizione) [addendum §2]
- Differenza kit React: "un kit si puo' copiare" (vantaggio di esecuzione, non barriera) [brief]
- CONFLITTO (lieve): il memlog elenca memoria a riposo e avvio task come criteri con obiettivo nel PRD, ma i criteri finali del brief li omettono; resta solo la richiesta di fissare i numeri nel PRD [memlog vs brief §Criteri di successo]
- CONFLITTO (lieve): il brief §Il problema cita solo "circa 7.700 istruzioni SQL per task"; addendum e memlog aggiungono 600 KB di WAL (non contraddittorio, solo piu' completo)
- CONFLITTO (lieve): l'addendum §1 dice SDK OWS Rust alpha "non risulta aggiornato alla 1.1.0", mentre §1 e §3 indicano OWS 1.1.0 come stabile: implica lavoro di allineamento, non risolto
- Differenza "Hatchet dipende da RabbitMQ": brief la presenta come scelta tra DB sovraccarico e RabbitMQ bloccato, addendum §3 la dichiara superata in parte (Postgres broker opzionale) [CONFLITTO di enfasi]

## 10. Materiale tecnico che appartiene all'architettura

- addendum §1: ricerca OWS 1.1.0, tabella capacita' DAG, discussione #1133, Zigflow, dove si puo' divergere (metadata vs chiavi nuove, schema chiuso)
- addendum §2: Iggy (log append-only, 0.10, cluster) e RustFS (1.0.6); alternative NATS JetStream, SeaweedFS, Garage, MinIO archiviato
- addendum §3: tabella comparabili, verifica delle critiche ai concorrenti, SDK ufficiali dei concorrenti, Activepieces/n8n
- addendum §4: tabella interfacce incorporabili, Workflow Builder, librerie `@xyflow/react`, dagre
- addendum §5: matrice separazione clienti/segreti/OAuth/sandbox dei concorrenti
- addendum §6: candidati sandbox (nsjail, Hyperlight, wasmtime, Firecracker, Nanos) con tabella di confronto
- addendum §7: requisiti di sicurezza (isolamento, quote, segreti, OAuth) da portare in PRD e architettura
- addendum §8: vincoli tecnici dell'utente (Iggy, RustFS, DB minimo) e motivazione
