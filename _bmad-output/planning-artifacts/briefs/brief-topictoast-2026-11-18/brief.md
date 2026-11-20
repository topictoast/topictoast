---
title: topictoast — product brief
status: final
created: 2026-11-18
updated: 2026-11-19
---

# topictoast — product brief

## In sintesi

topictoast è un motore di workflow open source (Apache-2.0), scritto in Rust, fatto per essere
incorporato in altre piattaforme. Chi lo integra offre ai propri clienti un editor no-code per
comporre da sé le pipeline, eseguite come DAG, in parallelo anche su elenchi di dimensione
variabile. I workflow sono documenti Open Workflow Specification, con un'estensione per i DAG
completi.

Oggi chi vuole questo deve mettere insieme un motore pesante o fragile, un'interfaccia scritta da
zero e la separazione tra clienti costruita a mano. Hatchet, il riferimento dell'autore, costringe a
scegliere tra un database sovraccarico e RabbitMQ bloccato a una versione vecchia. topictoast porta
il traffico delle attività su Iggy e lascia al database solo lo stato; distribuisce un kit React
incorporabile; separa i clienti con cassaforte dei segreti e sandbox per il loro codice, tutto nella
versione open.

Il primo utente è l'autore stesso, che lo usa come motore delle pipeline della propria app di
threat intelligence; il pubblico sono gli sviluppatori che vogliono la stessa cosa nella propria
piattaforma. Successo a sei mesi: 500 stelle su GitHub e un benchmark pubblico che mostra il carico
sul database molto sotto quello di Hatchet.

## Per chi

**Chi integra topictoast (utente principale).** Sviluppatori che costruiscono una piattaforma e
vogliono offrire ai propri clienti pipeline di lavoro configurabili, senza scrivere da zero il
motore e l'interfaccia. Il primo è l'autore stesso, sviluppatore ed esperto di sicurezza, che sta
costruendo un'applicazione di threat intelligence. Per lui funziona se incorpora i componenti React
nella propria app, collega i worker e i clienti creano le pipeline senza che lui scriva codice per
ognuna. Dopo di lui, altri sviluppatori con lo stesso profilo: il pubblico dell'open source.

**Chi usa la piattaforma (utente finale).** I clienti della piattaforma che integra topictoast:
compongono le proprie pipeline con un editor no-code e le eseguono su worker personalizzati, in
sandbox. Non hanno bisogno di sapere che sotto c'è topictoast.

**Non è il pubblico principale.** Chi usa direttamente la dashboard di topictoast per crearsi i
propri workflow, come con n8n. n8n è troppo avanti per inseguirlo; i nodi di base già pronti stanno
nella visione.

## Il problema

Chi vuole offrire ai propri clienti pipeline configurabili deve oggi assemblare tre pezzi che
nessun progetto open fornisce insieme.

1. **Un motore.** Le opzioni open sono pesanti (Temporal: quattro servizi più un database), legate
   a Kubernetes (Argo) o con dipendenze fragili. L'autore partirà con Hatchet e RabbitMQ bloccato
   alla 4.2.9, perché con l'ultima versione Hatchet non funziona (issue hatchet-dev/hatchet#5501).
   Senza RabbitMQ, Hatchet usa Postgres come coda e il carico sul database diventa sproporzionato:
   un utente ha misurato circa 7.700 istruzioni SQL per task (issue hatchet-dev/hatchet#5153); il
   maintainer consiglia di aggiungere RabbitMQ.
2. **Un'interfaccia no-code da scrivere da zero.** Nessun motore distribuisce componenti da
   incorporare per costruire pipeline e seguirne le esecuzioni (vedi addendum).
3. **La separazione tra clienti (multi-tenancy).** Ogni cliente carica i propri nodi
   personalizzati, cioè codice suo eseguito in sandbox, e configura le proprie integrazioni; vede lo
   storico delle proprie esecuzioni e dei propri workflow, e nient'altro. Va costruita sopra il
   motore.

Caso concreto: arriva un indicatore, lo si verifica in parallelo su DNS e su Censys, lo si
arricchisce, lo si inserisce. A seconda di come il cliente configura la pipeline, l'esecuzione
riceve un indicatore solo o un elenco da elaborare in parallelo.

## La soluzione

Chi integra topictoast riceve cinque pezzi che funzionano insieme:

1. **Il motore.** Esegue i workflow scritti in OWS, con DAG e parallelo su elenchi di dimensione
   variabile. Si installa con due componenti, topictoast e Iggy, più il database a scelta (SQLite
   incorporato o Postgres); RustFS solo se servono payload grandi o file.
2. **Gli SDK.** Per scrivere i workflow come codice, oppure per inviare un documento OWS e creare
   nuovi workflow mentre il sistema gira: è così che arrivano al motore le pipeline costruite dai
   clienti nell'editor.
3. **La libreria JavaScript per il frontend, in due strati.** Sotto, un nucleo senza interfaccia
   che gestisce il DSL e parla con il motore, usabile anche da chi non usa React. Sopra, i
   componenti React da incorporare: costruire i workflow, seguire lo stato delle esecuzioni,
   configurare segreti e accessi a piattaforme esterne con OAuth o altri metodi.
4. **La dashboard di chi gestisce.** Mostra i worker attivi e ogni fase del loro lavoro.
5. **La cassaforte delle credenziali.** topictoast conserva i segreti e i token di ogni cliente, li
   rinnova e li consegna ai worker solo al momento dell'esecuzione. La prima versione parte con le
   chiavi API; l'OAuth arriva dopo.

Il giorno in cui funziona: l'autore incorpora la libreria nella sua app di threat intelligence. Un
suo cliente apre l'editor, compone la pipeline del caso concreto, collega le proprie credenziali
Censys, aggiunge un suo nodo personalizzato e la lancia; ne segue l'esecuzione e vede solo le
proprie. L'autore, dalla dashboard, vede i worker al lavoro.

## Cosa lo rende diverso

In ordine di importanza per l'autore.

1. **Kit React incorporabile, nativo del motore.** Nessun motore open distribuisce componenti per
   costruire workflow e seguirne le esecuzioni dentro un'altra app (dettaglio nell'addendum). Il
   più vicino, Workflow Builder, è un editor React che però gira solo su Temporal. È un vantaggio
   di esecuzione, non una barriera: un kit si può copiare.
2. **Database al minimo.** Le attività viaggiano su Iggy, il database tiene solo lo stato. Contro
   Hatchet con Postgres come coda (#5153) è la differenza più misurabile; va dimostrata con un
   benchmark.
3. **Velocità, poca memoria, alta scalabilità.** Vanno dimostrate con numeri: Rust da solo non
   convince chi sceglie.
4. **Separazione tra clienti, con sandbox e cassaforte incluse, nel nucleo open con licenza
   permissiva.** I pezzi esistono altrove, ma non insieme e non gratis. Windmill ha spazi separati,
   segreti cifrati, OAuth e sandbox nella versione Community, ma è AGPL. Activepieces (MIT) ha
   sandbox e connessioni nel nucleo, ma il codice privato dei clienti e i gestori di segreti sono a
   pagamento (confronto completo nell'addendum). La differenza è la combinazione:
   isolamento forte di workflow e storico, segreti cifrati per cliente, OAuth rinnovato dal motore
   e sandbox per il codice dei clienti, tutto nella versione open.

**Scelta di progetto, non differenza: OWS.** Il formato dei workflow è una specifica CNCF
documentata, uno standard già pronto: niente linguaggio da progettare e documentare da zero. Per
chi integra non è un motivo di scelta. I DAG completi richiedono un'estensione (il parallelo su
elenchi non esiste in OWS 1.1), quindi i workflow che la usano non girano su altri runtime.

**Cosa non lo rende diverso.** DAG, YAML e database esistono già (Kestra, Windmill, Argo); binario
unico più database pure (Hatchet, Windmill, Restate). Con SQLite topictoast ha due processi, come
Hatchet con solo Postgres: la leggerezza si argomenta con memoria e carico, non con il numero di
componenti.

## Criteri di successo

A sei mesi dal primo rilascio pubblico:

- **Almeno 500 stelle su GitHub.** Per confronto, oggi Workflow Builder ne ha 448 e Zigflow 242.
- **Carico sul database misurato** con un benchmark pubblico: istruzioni SQL per task molto sotto
  le circa 7.700 di Hatchet con Postgres come coda.

## Prima versione

**Dentro**

- Motore OWS con l'estensione per i DAG (dipendenze tra task e parallelo su elenchi), retry e
  timeout.
- Installazione con topictoast e Iggy, più SQLite o Postgres.
- Separazione tra clienti, cassaforte dei segreti; sandbox per il codice dei clienti eseguita da
  topictoast stesso, che deve reggere codice ostile.
- Nucleo JavaScript e componenti React: editor dei workflow e vista delle esecuzioni.
- Dashboard di chi gestisce.
- SDK in TypeScript e Rust; nodi personalizzati dei clienti negli stessi due linguaggi.

**Fuori, per dopo**

- Nodi di base pronti (cicli, connettori, LLM).
- RustFS per payload grandi e file.
- OAuth gestito dal motore: all'inizio solo chiavi API.
- Utenti «tipo n8n».
- Nodi personalizzati in Python e Go.

**Licenza**: Apache-2.0, per tutto il nucleo: niente funzioni a pagamento dentro. È la condizione
per la differenza sulla separazione tra clienti, ed è la licenza di Iggy, RustFS e OWS.

## Visione

Fra due o tre anni topictoast è il motore di workflow incorporato in diverse piattaforme, non solo
in quella dell'autore. Con i nodi di base pronti (cicli, if/else, trasformazioni, connettori, LLM),
l'OAuth gestito dal motore e i nodi in Python e Go, diventa anche un'alternativa open a n8n e
Activepieces per chi usa direttamente la dashboard.

Con la diffusione degli agenti, le stesse pipeline servono a costruirli: passi con LLM, chiamate a
strumenti, codice dei clienti in sandbox, credenziali custodite dal motore. Sandbox, cassaforte e
DAG ci sono già nella prima versione; i passi con LLM arrivano con i nodi di base.

L'estensione per i DAG, una volta provata, può essere proposta alla specifica OWS (issue #1143 sul
parallelo dinamico).

## Rischi e domande aperte

- **Sandbox.** topictoast esegue codice ostile accanto ai segreti di tutti i clienti: è il rischio
  più grande. Tecnologia da scegliere con uno spike (candidata principale: Wasm con wasmtime).
- **Iggy è un log, non una coda.** Conferme, retry, timeout e ritardi dei task vanno costruiti in
  topictoast; Iggy è ancora 0.x, con cambi di formato tra versioni.
- **Estensione di OWS per i DAG.** Come scriverla (dentro `metadata` o con chiavi nuove) lo decide
  l'architettura; con chiavi nuove i documenti non sono più OWS validi.
- **Velocità, memoria e scalabilità.** Sono una differenza dichiarata ma senza obiettivo numerico:
  i numeri vanno fissati nel PRD.
- **Workflow Builder.** Competere o integrarsi (per esempio un suo adattatore verso topictoast al
  posto di Temporal) resta da decidere.
