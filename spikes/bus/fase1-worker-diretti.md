# S-1, fase 1: worker diretti sul bus e condizioni di sicurezza

- Data: 2026-11-23
- Fase: 1 di S-1, verifica a tavolino senza codice (memlog, voce 92)
- Candidati: Apache Iggy server-0.10.0 con il crate Rust `iggy` 0.12.0; Valkey 9.1.4 come livello
  caldo (stream) con il crate `redis` 1.8.1
- Fonti primarie: i sorgenti ai tag `server-0.10.0` di apache/iggy e `9.1.4` di valkey-io/valkey,
  più la documentazione ufficiale. I riferimenti `file:riga` sono a quei tag; l'elenco completo è
  in «Fonti».

## Domanda

La variante (1) di S-1 (worker collegati direttamente ai topic del bus, con ack, retry e timeout
fatti dal motore tramite topic) soddisfa tutte e tre le condizioni di sicurezza di D-1, con Iggy e
con Valkey? Se ne manca una, la variante è esclusa qualunque siano i numeri.

## Condizioni

### Le tre condizioni e la lettura usata

1. **Nessuna chiave dei tenant né segreto ai worker.** Questa analisi la legge così:
   - «Chiave dei tenant» è la chiave per tenant di C-12, quella cifrata dalla chiave principale. La
     chiave che C-12 deriva con HKDF per un singolo oggetto (`info` = indirizzo completo
     dell'oggetto) non è la chiave del tenant: apre solo quell'oggetto e non permette di ricavarne
     altre. Darla al worker equivale a dargli il contenuto in chiaro di quell'oggetto, cosa che le
     varianti (2) e (3) fanno comunque.
   - «Né segreto» non può essere letterale. I segreti dichiarati nelle connessioni del manifest
     arrivano al worker con ogni variante: S-1 parla di «segreti dichiarati risolti alla
     consegna», e vedi anche AD-20 e FR-55. La condizione chiede che non passino dal bus (C-12,
     AD-2, AD-7).

   Con una lettura più stretta, dove nessuna chiave derivata da quella del tenant arriva al worker,
   l'esito cambia: vedi «Verdetto».
2. **C-11 applicabile dal bus.** Una credenziale del bus per ogni worker. Deve essere revocabile
   una per una, con effetto immediato anche sulle connessioni aperte, limitabile a funzioni e
   tenant, e distinta dalle credenziali dei tenant e dell'integratore.
3. **Eventi di stato scritti solo dal motore (AD-2).** Il bus impedisce alla credenziale di un
   worker di scrivere sui topic di stato, e le lascia scrivere le richieste (risultati, battiti,
   ack) che il motore giudica.

Le condizioni 2 e 3 dipendono dal bus. La condizione 1 no: nasce da C-13 (input, output ed eventi
dei tenant cifrati con la chiave del tenant sul bus) e vale allo stesso modo per entrambi i
candidati. La tratto per prima.

### Condizione 1: come un worker diretto legge una consegna cifrata

Sul bus la consegna è cifrata nel formato di C-12. Un worker che la legge dal bus deve
decifrarla. Le strade possibili sono sei.

| Strada | Chiave del tenant al worker | Segreti sul bus | Cancellazione crittografica (C-13) | Esito |
|---|---|---|---|---|
| A. Il worker riceve la chiave del tenant | sì | no | regge | fallisce la condizione 1 |
| B. Il motore scrive una copia in chiaro nel topic del worker | no | no | non si applica | viola C-13: sul bus vanno solo dati cifrati con la chiave del tenant |
| C. Il motore scrive una copia ricifrata con una chiave del worker, simmetrica o pubblica | no | no | non regge: cancellata la chiave del tenant, la copia resta leggibile con la chiave del worker fino alla pulizia asincrona | viola C-13 e il formato unico di AD-7 e C-12 |
| D. La chiave derivata dell'oggetto, avvolta per il worker, viaggia sul bus | no (con la lettura usata) | no | non regge, come in C: la chiave avvolta resta sul bus, apribile dal worker, dopo la cancellazione della chiave del tenant | viola C-13 |
| E. Il motore consegna fuori dal bus la chiave derivata dell'oggetto e i segreti; il payload si legge dal bus | no (con la lettura usata) | no | regge: la chiave derivata vive solo in memoria, nel motore e nel worker | condizione 1 soddisfatta |
| F. Il bus cifra lato server: chiave per stream, decifrata al poll (esiste solo in Iggy 0.10) | no ai worker, ma la chiave passa al bus | no | dipende dal bus: Iggy tiene una propria copia avvolta sotto la sua chiave | viola AD-7, C-12 e C-13; servirebbe un ADR che le sostituisce |

Resta solo la strada E, e regge a un patto. La chiave derivata deve aprire una sola consegna, cioè
ogni consegna sul bus deve essere un oggetto con la propria chiave derivata. C-12 e AD-7
ammettono una chiave «per oggetto o segmento». Se S-1 scegliesse una chiave per segmento del
topic, la chiave data al worker aprirebbe anche le altre consegne dello stesso segmento, comprese
quelle di altri worker o tenant, e la strada E non reggerebbe più.

Ecco cosa costa la strada E:

- **Un giro in più per ogni tentativo.** Prima di lavorare, il worker chiede al motore, fuori dal
  bus (gRPC o HTTP con TLS, C-14), la chiave derivata dell'input, quella dell'output e i segreti
  dichiarati. Il giro serve sempre, anche per una funzione senza segreti, perché l'input sul bus è
  sempre cifrato. È il «giro in più misurato dentro NFR-2» che S-1 già prevede tra i criteri non
  numerici.
- **Lavoro del motore.** Per ogni tentativo il motore deriva le chiavi con HKDF, risolve i segreti
  e controlla la sessione, ma non decifra né trasmette il payload. È l'unico risparmio rispetto
  alla variante (2), e conta solo per i payload grandi (fino a 1 MB).
- **Due collegamenti per worker.** Ogni worker ha un collegamento al bus e uno al motore, con due
  punti di controllo della credenziale, e la revoca va fatta in entrambi. Già la revoca nel motore
  toglie l'accesso al contenuto, perché senza chiave derivata il payload sul bus è illeggibile. La
  revoca sul bus toglie il testo cifrato e le scritture. Iggy 0.10 può chiamare un servizio
  esterno al login (`core/server/src/external_auth.rs:18-27`), che potrebbe essere il motore. Così
  la credenziale resterebbe una sola, ma il servizio esterno vale solo al login.
- **Diretti restano solo i byte del payload.** Il controllo di ogni tentativo passa comunque dal
  motore.

### Righe di log, messaggi d'errore, output e scritture sugli stream

Questi punti non sono condizioni di D-1 ma criteri non numerici di S-1. Li riporto perché la
strada E li cambia.

- **Righe di log e messaggi d'errore.** L'ADR del 2026-11-23 sui log dei task ha scartato il
  «segnaposto solo all'origine», perché un SDK sbagliato porterebbe il segreto sul bus. Il motore
  ricontrolla le righe e i messaggi d'errore dove passano da lui. Se il worker scrive
  direttamente sul bus, il ricontrollo arriva dopo il bus. Quindi righe e messaggi d'errore
  devono passare dal collegamento con il motore. AD-17 però vuole che le righe viaggino «come i
  risultati del tentativo: stesso collegamento». Le due regole insieme hanno due uscite: anche i
  risultati passano dal motore, e allora di diretto resta solo la lettura delle consegne; oppure
  si cambia AD-17.
- **Output e sessione (C-2), proposta da verificare in S-1.** Con la strada E il worker potrebbe
  cifrare l'output con la chiave derivata dall'indirizzo dell'output. Il motore darebbe quella
  chiave solo alla sessione a cui ha consegnato il tentativo, e il tag di AES-GCM proverebbe che
  chi scrive la possiede. Così C-2 la applicherebbe il motore; il bus da solo non sa farlo,
  perché non lega un messaggio a una sessione.
- **Scritture sugli stream (AD-20, tenant ricavato dal tentativo).** Il bus non sa legare una
  scrittura a un tentativo: dovrebbe cambiare l'ACL a ogni tentativo, migliaia di volte al
  secondo con NFR-3. Un worker abilitato per i tenant T1 e T2 potrebbe scrivere sul topic stream
  di T2 mentre lavora per T1. Quindi le scritture sugli stream vanno sul topic delle richieste del
  worker, e il motore le riscrive nel topic del tenant: non sono dirette.

### Il layout che il bus sa difendere

Con entrambi i candidati, il layout che il bus difende con meno regole è questo:

- **Consegne.** Un topic (Iggy) o una chiave stream (Valkey) per worker: il worker lo legge e
  basta, solo il motore ci scrive.
- **Richieste.** Un topic o una chiave per worker: il worker ci scrive e basta, solo il motore lo
  legge.
- **Stato e stream dei tenant.** Nessun permesso ai worker sui topic di stato e su quelli degli
  stream dei tenant.

Il motore sceglie il worker, ammesso dalla credenziale per funzioni e tenant, e applica
concorrenza e quote (FR-12, FR-13, FR-30). Poi scrive la consegna nel topic di quel worker.
Nessuno dei due bus registra nel messaggio chi l'ha scritto, quindi il motore riconosce il
mittente di una richiesta dal proprietario del topic.

Con questo layout, però, il limite a funzioni e tenant di C-11 lo applica il motore quando
sceglie il worker: il bus isola soltanto i topic del worker. Se «C-11 applicabile dal bus» vuol
dire che anche quel limite deve imporlo il bus, serve il layout alternativo: un topic per tenant ×
funzione, con un permesso di lettura per ogni coppia ammessa dalla credenziale. Entrambi i
candidati sanno imporlo (vedi le loro sezioni). Il costo è triplice:

- la scelta del worker passa al bus, e il motore la scopre solo da una richiesta del worker che
  prende in carico il tentativo, mentre C-2 parla del worker «a cui il motore lo ha consegnato»;
- ogni worker vede tutte le consegne delle sue coppie, anche quelle prese da altri worker;
- con Iggy, i permessi vanno aggiornati per ogni tenant nuovo.

Si possono anche sommare i due layout: il motore scrive nel topic del worker, e il bus lo ammette
solo se la coppia tenant × funzione rientra nella credenziale. Questo però vuol dire un topic per
worker × tenant × funzione. L'esito delle condizioni non cambia con nessuno dei layout.

## Iggy (server-0.10.0, crate `iggy` 0.12.0)

Il crate 0.12.0 è quello del tag `server-0.10.0` (`core/sdk/Cargo.toml:20`;
`core/server/Cargo.toml:20` dice 0.10.0).

### Autenticazione e permessi

- **Permessi su tre livelli.** Gli utenti hanno permessi globali, per stream e per topic
  (`core/common/src/types/permissions/permissions_global.rs:29-150`). Per un topic sono
  `manage_topic`, `read_topic`, `poll_messages` e `send_messages`. Non esistono permessi per
  partizione, e le mappe usano id numerici, senza pattern.
- **Token personali (PAT).** Sono legati all'utente. Ogni utente crea e cancella da sé i propri
  PAT e cambia da sé la propria password, senza `manage_users`
  (`core/metadata/src/stm/authz.rs:411-428`, `:443-447`; `core/server/src/dispatch/authz.rs:303-305`).

### C-11 dal bus

- **Una credenziale per worker.** È un utente Iggy per worker, non un PAT (vedi la revoca). È
  distinta dalle credenziali dei tenant, che non hanno utenti sul bus: fa eccezione solo il
  processo delle pipeline stream, AD-13. Le credenziali del kit e dell'SDK dell'integratore sono
  JWT di C-15, verificati dal motore e non dal bus.
- **Revoca una per una, immediata.** Cancellare o disattivare l'utente chiude tutte le sue
  sessioni con un'operazione replicata (`core/metadata/src/stm/user.rs:708`, `:715-758`;
  `core/metadata/src/impls/metadata.rs:1086`, `:1120-1158`). Send, poll e offset si autorizzano
  a ogni richiesta contro i permessi correnti. Una lettura in attesa ricontrolla sessione e
  permessi prima di rispondere (`core/server/src/dispatch/authz.rs:18-26`, `:54-58`,
  `:214-220`).
  - **HTTP.** Il logout non revoca il bearer (`core/server/src/http/handlers.rs:252-260`), ma ogni
    richiesta controlla che l'utente esista e sia attivo (`core/server/src/http/state.rs:461-488`,
    `core/server/src/http/extractor.rs:422`).
  - **PAT e password.** Cancellare un PAT o cambiare la password non chiude le sessioni aperte
    (`core/metadata/src/stm/user.rs:761-800`, `:1046-1066`), quindi la revoca si fa sull'utente.
    I PAT che un worker si crea da sé non la aggirano: la cancellazione dell'utente toglie i suoi
    PAT (`stm/user.rs:748`), e il login di un utente non attivo è rifiutato
    (`core/server/src/dispatch/session_ops.rs:134`, `:208-209`).
- **Limitabile a funzioni e tenant.** Sì, con il layout per worker: il filtro lo applica il motore
  quando sceglie il worker. Con un topic per tenant × funzione, invece, i permessi vanno elencati
  topic per topic, perché non ci sono pattern. Ogni tenant nuovo vuol dire aggiornare i permessi
  di ogni worker che lo serve, e un permesso globale di poll aprirebbe tutti gli stream, compresi
  quelli di stato (`core/metadata/src/permissioner/permissioner_rules/messages.rs:24-80`).
- **Topic condivisi.** Su un topic, `poll_messages` basta per salvare e cancellare gli offset di
  qualunque consumatore (`permissioner_rules/consumer_offsets.rs:23-47`). Per entrare in un gruppo
  di consumatori serve invece `read_topic` o `manage_topic` (`permissioner_rules/consumer_groups.rs:59-65`,
  `permissioner_rules/topics.rs:24-54`), e `read_topic` comprende il poll. Il permesso non nomina
  la partizione, e il poll parte da un offset, da un istante, dal primo o dall'ultimo messaggio
  (`core/common/src/types/message/polling_kind.rs:26-38`). Ne deduco, senza averlo provato, che
  su un topic condiviso un worker può spostare gli offset degli altri e rileggere tutte le
  consegne conservate. Con il layout per worker il problema non c'è.

### Eventi di stato solo dal motore

- **Permesso di scrittura per topic.** `send_messages` si dà per topic
  (`permissioner_rules/messages.rs:83`). Il worker ha `send_messages` solo sul proprio topic delle
  richieste, non su quelli di stato, e lo impone il bus.
- **Nessun mittente nel messaggio.** L'intestazione non porta l'identità del mittente: contiene
  checksum, id scelto dal client, offset, timestamp, timestamp d'origine, lunghezze e un campo
  riservato (`core/common/src/types/message/message_header.rs:23-33`). Il motore riconosce il
  mittente solo dal topic.

### Cifratura

- **Cifratura lato server.** Iggy 0.10 cifra lato server con una chiave per stream
  (AES-256-GCM-SIV), avvolta sotto una chiave del server presa dalla configurazione. Cifra prima
  di salvare e decifra al poll, e il client può fornire la chiave quando crea lo stream
  (`core/server/config.toml:424-436`, `core/common/src/utils/stream_key.rs:67-72`, pagina
  «Security»).
- **È la strada F, che non aiuta la condizione 1.**
  - Il formato non è quello di C-12.
  - La chiave del tenant finirebbe nel bus.
  - La cancellazione crittografica dipenderebbe da Iggy. La pagina «Security» dice che journal e
    snapshot dei metadati non sono cifrati nel loro insieme. Non ho verificato se cancellare uno
    stream toglie dal journal la sua chiave avvolta.
- **Resta la strada E,** come per Valkey.

### Altri punti

- **Credenziali predefinite.** Un server nuovo chiede credenziali di root esplicite; `iggy`/`iggy`
  vale solo con l'opzione di sviluppo `--with-default-root-credentials`. Lo dice la pagina
  «Security», che segue il ramo master e non il tag. Il root non si cancella
  (`core/metadata/src/stm/user.rs:723-729`) e ha tutti i permessi, quindi il motore non lo usa
  per il lavoro corrente.
- **Limiti.** Un topic ha al più 1000 partizioni (`core/common/src/lib.rs:192`). Non ho trovato
  limiti espliciti al numero di stream o di topic, e due topic per worker sono pochi.
- **Rischio.** Il server 0.10 ha riscritto sessioni e autorizzazione sopra il consenso
  (`core/server/src/session_manager.rs`): è codice giovane. Con la variante (1) l'ACL del bus
  diventa una barriera verso i processi dell'integratore.

### Esito Iggy

Le condizioni 2 e 3 le impone il bus, con un utente per worker e il layout per worker. La
condizione 1 regge solo con la strada E.

## Valkey (9.1.4, crate `redis` 1.8.1)

### Autenticazione e permessi

- **ACL.** Gli utenti ACL hanno più password, comandi e categorie (`@stream`), pattern di chiavi
  con permessi separati di lettura e scrittura (`%R~`, `%W~`), selettori per combinare comandi e
  chiavi diverse, e permessi per database (`db=`) (`src/acl.c:1124-1160`; pagina ACL).
- **Comandi stream.** Il permesso richiesto su una chiave dipende dai flag della sua key spec:
  `ACCESS` chiede la lettura, `INSERT`, `DELETE` e `UPDATE` chiedono la scrittura
  (`src/acl.c:1724-1728`). XREADGROUP ha `RO ACCESS`, quindi chiede solo la lettura. XACK, XADD e
  XCLAIM hanno `RW UPDATE`, XAUTOCLAIM `RW DELETE`: chiedono solo la scrittura
  (`src/commands.def:10403-10405`, `:10434-10436`, `:10498-10500`, `:10530-10532`,
  `:11039-11041`).
- **Esempio.** Prendiamo un worker con
  `-@all +xreadgroup +xack ~d:<worker> (+xadd %W~r:<worker>)`, più i comandi di connessione.
  `~` dà lettura e scrittura su `d:<worker>`, che servono a XREADGROUP e XACK; il selettore dà
  XADD solo in scrittura su `r:<worker>`. Così il worker legge e conferma solo le proprie
  consegne, e aggiunge voci solo al proprio stream delle richieste.

### C-11 dal bus

- **Una credenziale per worker.** È un utente ACL per worker.
- **Revoca.**
  - `ACL DELUSER` chiude tutte le connessioni autenticate con l'utente (`src/acl.c:492-512`,
    `:3150-3166`; pagina ACL DELUSER).
  - `ACL SETUSER` modifica l'utente sul posto (`src/acl.c:2342`, `:2351`), e ogni comando si
    controlla contro l'utente corrente (`src/acl.c:2105-2108`, `src/server.c:4413-4417`). Vale
    anche per un XREADGROUP bloccato: quando si sblocca, processCommand lo riesegue
    (`src/blocked.c:742-758`, `src/networking.c:3977-3986`).
  - `off` impedisce solo i nuovi login: le connessioni già autenticate continuano (pagina ACL).
    Quindi la revoca si fa con DELUSER.
- **Persistenza.** `ACL SETUSER` e `ACL DELUSER` cambiano solo la memoria: il file delle ACL lo
  scrive soltanto `ACL SAVE` (`src/acl.c:2695`, unico chiamante a `:3258`). Senza file delle ACL,
  gli utenti stanno nelle righe `user` di `valkey.conf`, e solo `CONFIG REWRITE` le riscrive
  (`src/config.c:1453-1460`). Senza uno dei due passi, un worker revocato torna quando Valkey
  riparte. Requisiti: `aclfile` obbligatorio, `ACL SAVE` dopo ogni revoca, e all'avvio il motore
  riconcilia gli utenti ACL con il proprio elenco.
- **Limitabile a funzioni e tenant.** Sì, con il layout per worker, dove il limite lo applica il
  motore. Lo impone anche il bus, con chiavi per tenant × funzione e un pattern per coppia
  (`~d:<tenant>:<funzione>`); per «tutti i tenant» un pattern glob (`~d:*:<funzione>`) evita gli
  aggiornamenti a ogni tenant nuovo.
  Però il nome del gruppo e quello del consumatore di XREADGROUP sono argomenti fuori dall'ACL. Su
  una chiave condivisa, un worker legge le voci in sospeso di un altro consumatore (XREADGROUP con
  id 0 e il nome dell'altro). Se ha XACK o XCLAIM, può confermarle o prenderle.

### Eventi di stato solo dal motore

- **Scritture limitate dal bus.** Il worker non ha permesso di scrittura sulle chiavi di stato, e
  ha XADD solo sulla chiave delle richieste, tramite un selettore. Lo impone il bus.
- **Nessun mittente nella voce.** Le voci non portano l'utente che le ha scritte, e l'id può
  sceglierlo il client.

### Cifratura

Valkey non cifra i dati salvati; il TLS copre solo il transito. Per la condizione 1 resta solo la
strada E, perché la F qui non esiste.

### Altri punti

- **ACL come barriera.** Le note di rilascio della 9.1 elencano diverse correzioni di aggiramenti
  dell'ACL: PFMERGE, SORT con STORE, GEORADIUS con STORE, HGETEX, MOVE e COPY tra database
  (`00-RELEASENOTES:43`, `:68`, `:151`, `:158`, `:163`, `:196`). Con la variante (1) l'ACL
  diventa una barriera verso i worker. Serve una lista di comandi minima: `-@all` più i soli
  comandi stream e di connessione necessari.
- **Il crate.** `redis` 1.8.1 esiste e ha le feature `acl`, `streams` e `tls-rustls` (crates.io).
  Non ho verificato altro.

### Esito Valkey

Le condizioni 2 e 3 le impone il bus, con un utente ACL per worker, la revoca con `ACL DELUSER`, e
`aclfile` con `ACL SAVE`. La condizione 1 regge solo con la strada E.

## Verdetto

- **Iggy server-0.10.0: condizioni soddisfatte**, a due patti: il giro per tentativo verso il
  motore (strada E), e un utente Iggy per worker, revocato cancellandolo o disattivandolo.
- **Valkey 9.1.4: condizioni soddisfatte**, a tre patti: la strada E, un utente ACL per worker
  revocato con `ACL DELUSER`, e `aclfile` con `ACL SAVE` dopo ogni revoca.
- **Ragione decisiva.** Le condizioni 2 e 3 non distinguono i candidati: entrambi impongono
  permessi per topic o per chiave e chiudono le sessioni alla revoca. La condizione 1 non dipende
  dal bus. Un worker può decifrare una consegna che legge dal bus, cifrata secondo C-13, senza la
  chiave del tenant e senza segreti sul bus, solo così: il motore gli dà, fuori dal bus e per
  ogni tentativo, la chiave derivata dell'oggetto e i segreti. Ogni altra strada viola C-13 o
  AD-7.
- **Patto comune ai due candidati.** Ogni consegna sul bus deve avere una propria chiave derivata,
  non una chiave per segmento. Altrimenti la chiave data al worker apre anche le consegne altrui,
  e la condizione 1 cade per entrambi.
- **Con la lettura stretta la variante è esclusa.** Se nessuna chiave derivata da quella del
  tenant può arrivare al worker, la variante (1) è esclusa per entrambi i candidati: nessuna
  strada lascia il worker leggere dal bus senza chiave e senza una copia in chiaro o ricifrata.
- **Seconda lettura, su C-11.** Se «applicabile dal bus» vuol dire che anche il limite a funzioni
  e tenant lo impone il bus, e non il motore che sceglie il worker, serve il layout per tenant ×
  funzione. Entrambi i candidati lo sanno imporre, con i costi descritti in «Il layout che il bus
  sa difendere». Il verdetto non cambia.
- **Chi sceglie.** L'esito è lo stesso per Iggy e per Valkey con ogni lettura, quindi questa
  verifica non separa i candidati. Quale lettura vale lo decide Marco: D-1 è sua.
- **Se la variante resta, per le fasi successive:**
  - il worker tiene comunque un collegamento con il motore, e lo usa a ogni tentativo;
  - righe di log, messaggi d'errore e scritture sugli stream passano dal motore, e AD-17 («stesso
    collegamento dei risultati») va riallineata;
  - rispetto alla variante (2) resta un solo vantaggio: il motore non decifra né trasmette il
    payload. Si misura con payload da 1 KB a 1 MB, contro il costo del giro in più dentro NFR-2.

## Fonti

Sorgenti consultati il 2026-11-23 ai tag indicati.

- Apache Iggy, tag `server-0.10.0` (commit `91e0206367aad2d30259f7d45203832b08ae84e7`):
  <https://github.com/apache/iggy/tree/server-0.10.0>
  - permessi: `core/common/src/types/permissions/permissions_global.rs`,
    `core/metadata/src/permissioner/permissioner_rules/{messages,consumer_offsets,consumer_groups,topics}.rs`,
    `core/metadata/src/stm/authz.rs`, `core/common/src/types/message/polling_kind.rs`
  - revoca e sessioni: `core/metadata/src/stm/user.rs`, `core/metadata/src/impls/metadata.rs`,
    `core/server/src/dispatch/authz.rs`, `core/server/src/dispatch/session_ops.rs`,
    `core/server/src/http/{state,extractor,handlers}.rs`, `core/server/src/session_manager.rs`
  - messaggi: `core/common/src/types/message/message_header.rs`
  - cifratura: `core/server/config.toml` (sezione `[encryption]`),
    `core/common/src/utils/stream_key.rs`, `core/server/src/stream_keys.rs`
  - login esterno: `core/server/src/external_auth.rs`
  - limiti: `core/common/src/lib.rs`
  - versioni: `core/server/Cargo.toml`, `core/sdk/Cargo.toml`
- Documentazione Iggy, pagina «Security»: <https://iggy.apache.org/docs/server/security>. Segue
  il ramo master, non il tag 0.10.0: l'ho usata solo per conferme, e le affermazioni sopra
  poggiano sui sorgenti del tag.
- Crate `iggy` 0.12.0: <https://crates.io/crates/iggy/0.12.0>
- Valkey, tag `9.1.4` (commit `8efb5bb3dd3b69f707777352a105bbea911affd3`):
  <https://github.com/valkey-io/valkey/tree/9.1.4>
  - `src/acl.c`, `src/config.c`, `src/server.c`, `src/blocked.c`, `src/networking.c`,
    `src/commands.def`, `00-RELEASENOTES`
- Documentazione Valkey: <https://valkey.io/topics/acl/>,
  <https://valkey.io/commands/acl-deluser/>, <https://valkey.io/commands/acl-save/>
- Crate `redis` 1.8.1 (elenco delle feature): <https://crates.io/crates/redis/1.8.1>
- Documenti del progetto: `ARCHITECTURE-SPINE.md` (S-1, AD-2, AD-5, AD-7, AD-17, AD-20),
  `conventions.md` (C-2, C-11, C-12, C-13), `review-decisions.md` (D-1),
  `docs/decisions/2026-11-23-task-logs.md`, `.memlog.md` (voci 60, 75, 92)

## Decisione di Marco (2026-11-23)

La variante (1) esce: nemmeno la chiave derivata di un singolo oggetto arriva a un worker. Le
consegne passano sempre dal motore, con gRPC o con iroh. Iggy e Valkey restano pari: la scelta del
bus passa alla fase 2.
