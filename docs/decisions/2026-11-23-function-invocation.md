# ADR: funzioni e nodi chiamati dal catalogo predefinito di OWS

- Data: 2026-11-23
- Stato: accettata
- Registra: AD-20 della
  [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md); chiude la
  proposta su M-02 (PRD §11 n. 7), da fare prima di S-1
- Fonti: PRD FR-1, FR-15, FR-18, FR-29, FR-55; spine AD-8, AD-10, AD-11, AD-17, C-2;
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 80, 81 e 82;
  [schema `workflow.yaml` v1.1.0](https://raw.githubusercontent.com/open-workflow-specification/specification/v1.1.0/schema/workflow.yaml)
  e [`dsl.md` v1.1.0](https://raw.githubusercontent.com/open-workflow-specification/specification/v1.1.0/dsl.md)
  (sezioni «Default Catalog» e «Using Cataloged Functions»)

## Contesto

Un task di un workflow chiama le funzioni registrate dai worker (FR-12, FR-55) e i nodi
personalizzati caricati dai tenant (FR-15). Editor, SDK e motore devono scrivere e leggere la
chiamata nello stesso modo, ed è un contratto pubblico (PRD §6).

Cosa dice OWS 1.1, verificato sullo schema e sul testo:
- `call` di una funzione personalizzata è una stringa qualunque, diversa dai nomi riservati
  (`http`, `grpc`, `openapi`, `asyncapi`, `a2a`, `mcp`). Lo schema non la lega a
  `use.functions`, e la spec non dice cosa succede con un nome che il documento non dichiara.
- Un runtime può offrire un catalogo `default`, implicito per tutti i workflow, chiamato con
  `{functionName}:{functionVersion}@default`. OWS definisce `latest` solo per la versione di
  `run.workflow`: per le funzioni di un catalogo è una convenzione di topictoast.
- OWS ammette anche di ridefinire `default` con `use.catalogs` e di chiamare una funzione remota
  con un URL (`call: https://…@v1.0.0`): rifiutarli è una deviazione voluta, coerente con FR-1.
- Le politiche di autenticazione (basic, bearer, digest, oauth2, oidc) hanno una forma `use`
  basata sui segreti; un header con una chiave API non ha una forma senza `$secrets`.

Il PRD ammette omonimi: un nodo di un tenant può avere lo stesso nome di una funzione, e la
palette li distingue con il chip di provenienza (FR-29, FR-55).

## Decisione

- **Forma della chiamata (Marco, memlog voce 80):** funzione di un worker
  `call: <nome>:latest@default`, nodo personalizzato `call: node.<nome>:latest@default`; `with`
  porta solo l'input. Il prefisso `node.` è riservato. Nella v1 la versione è sempre `latest`,
  perché il nodo si lega alla sua versione all'avvio (FR-15).
- **Conseguenze delle regole esistenti (memlog voce 81):**
  - il validatore rifiuta senza rete `use.catalogs`, `use.functions`, le chiamate verso un URL e
    `schema.resource`;
  - il nome si risolve nel contesto del proprietario del workflow;
  - una chiamata a funzione o nodo non nomina segreti: le connessioni vengono dal manifest;
  - ogni esecuzione risolve all'avvio ogni chiamata in un riferimento fisso (impronta del
    manifest o del nodo) e lo conserva;
  - un worker o un nodo scrive su uno stream solo dall'SDK o dalla sandbox, con il tenant del
    tentativo.
- **Dalla revisione (`reviews/review-ad20.md`):** una sola grammatica dei nomi (ASCII,
  snake_case, al massimo 64 caratteri, niente nomi riservati di OWS né `node`) letta da una sola
  funzione del dominio; i nomi di un documento si risolvono nel contesto del suo proprietario, i
  segreti in quello di chi avvia; un nome inesistente e uno non disponibile danno lo stesso errore.
- **Assegnato:** la forma della connessione e dei segreti di un task senza manifest (`call http`)
  la fissa la spec della storia delle connessioni, con un solo proprietario (il validatore del
  dominio). Cosa porta una consegna e come un worker scrive su uno stream entrano nei criteri di
  S-1.

## Bilancio

- **Funzioni:** un documento senza `dag` resta OWS sensato anche per chi non conosce topictoast:
  le funzioni sono quelle del runtime, nella forma che la spec prevede. Chi legge vede subito se
  un task esegue una funzione dell'integratore o un nodo del tenant.
- **Costi:** nel YAML scritto a mano la chiamata è più lunga (`:latest@default`); editor e SDK la
  scrivono da soli. Il prefisso `node.` toglie un nome ai worker.
- **Manutenzione:** una sola forma, controllata da un solo validatore (AD-11), anche nel browser.
  Versioni diverse da `latest` restano possibili dopo la v1, senza cambiare la forma.

## Alternative scartate

- **Nome semplice (`call: dns_lookup`):** è più corto, ma per la spec un nome semplice andrebbe
  dichiarato in `use.functions`, e un altro strumento OWS non saprebbe cosa chiamare.
- **Omonimi vietati:** le chiamate restano più corte, ma contraddice FR-29 e la palette, e una
  funzione registrata dopo un nodo omonimo creerebbe un conflitto da gestire.
- **Provenienza in un campo di `with`:** mescola i dati passati alla funzione con l'instradamento.
- **Cataloghi dichiarati (`use.catalogs`):** chiedono un endpoint esterno da caricare, che
  FR-55 vieta durante la validazione.
