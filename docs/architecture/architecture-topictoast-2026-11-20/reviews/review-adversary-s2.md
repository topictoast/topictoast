# Revisione avversaria dell'Update post-S-2 (spine: AD-6, Stack, S-2, Open Questions; ADR della sandbox)

**Verdetto:** non pronta per la prima storia dei processi ausiliari così com'è. Ogni regola nuova regge da sola, ma restano 2 buchi critici e 5 alti dove due storie che rispettano alla lettera ogni AD e ogni convenzione costruiscono cose incompatibili: ciclo di vita del processo stream contro l'arresto per inattività, e proprietà della cache del codice nativo (con il suo percorso di archiviazione e di cancellazione).

Metodo: per ogni punto, due unità un livello sotto la spine (storie o moduli di persone diverse), come divergono, regola minima che chiude. Letti: diff dell'Update, spine intera, `conventions.md`, ADR della sandbox, FR-15, FR-48, FR-54 del PRD.

---

## H-1 (critica) Arresto per inattività contro il processo delle pipeline stream (AD-6 contro AD-13)

- **Unità A:** storia del ciclo di vita degli ausiliari. Rispetta AD-6: «avviato alla prima richiesta e fermato per inattività», dove «inattività» è l'assenza di richieste IPC dal motore.
- **Unità B:** storia delle pipeline stream. Rispetta AD-13: processo «attivo solo se il tenant ha pipeline attive», sempre acceso finché girano, legge i topic direttamente dal bus con la propria credenziale, tiene finestre e watermark in memoria e salva checkpoint nell'archivio dei payload (AD-17).
- **Divergenza:**
  1. Il processo stream non riceve richieste dal motore: a un conteggio di inattività sull'IPC è sempre «inattivo» e viene fermato con lo stato non ancora in checkpoint (finestre aperte perse fino all'ultimo checkpoint; eventi già consumati ma non confermati). Ripartirebbe «alla prima richiesta», che per una pipeline continua non arriva mai: nessuno lo riavvia.
  2. Chi decide l'avvio è ambiguo: AD-13 dice «se ha pipeline attive», AD-6 dice «alla prima richiesta». Due storie scrivono due supervisori.
  3. La validazione a secco del SQL (AD-11, FR-19) richiede DataFusion, ma il ruolo stream esiste solo con pipeline attive, e al momento di scrivere la prima pipeline non ce n'è nessuna. L'Unità C (storia della validazione a secco) mette DataFusion nel ruolo «validazione», con la sua lista seccomp e i suoi MB; l'Unità B lo tiene solo nel ruolo stream. Due liste seccomp, due RSS (il numero di 39 MB per tenant non lo include).
- **Fix:** il motore ha un solo supervisore per coppia (tenant, ruolo) con due classi di ciclo di vita, scritte in AD-6: «a richiesta» (validazione, jq, `call http`, sandbox leggera: avvio alla prima richiesta, arresto per inattività deciso dal solo motore) e «fissato» (stream: avviato quando il tenant ha una pipeline attiva, fermato solo alla disattivazione dell'ultima, dopo un checkpoint finale, o su cancellazione del tenant). La validazione a secco del SQL gira nel ruolo stream avviato «a richiesta» quando non ci sono pipeline attive (stesso ruolo, stessa lista seccomp); non esiste DataFusion in altri ruoli. Il contatore di inattività conta solo le richieste di ruoli «a richiesta».

## H-2 (critica) Chi possiede la cache del codice nativo e la sua impronta di riferimento

- **Unità A:** storia della compilazione dei moduli. Rispetta AD-6 e l'ADR: solo il ruolo di compilazione produce codice nativo, «in una cartella dove le sandbox non scrivono»; al caricamento se ne verifica l'impronta, che «il motore tiene fuori da quella cartella».
- **Unità B:** storia del salvataggio dei dati dei tenant. Rispetta la Open Question «Altri JSON dei tenant» (proposta: moduli compilati nell'archivio dei payload, riferimento tipizzato, cifratura per tenant), AD-5 (nessuna deduplicazione tra tenant) e C-13 (sentinella su «nodi»).
- **Divergenza:**
  1. Due posti per lo stesso artefatto: cartella locale in chiaro (wasmtime fa `mmap` del file, non può leggere un oggetto cifrato né un RustFS remoto) contro archivio dei payload cifrato per tenant. La Open Question e AD-6 non si riconciliano.
  2. «L'impronta di riferimento la tiene il motore» ma il motore non interpreta contenuto dei tenant (AD-6) e l'artefatto lo produce l'ausiliario: se il motore prende l'impronta dalla risposta dell'ausiliario, un compilatore compromesso attesta se stesso e la verifica non verifica nulla; se la calcola il motore dalla cartella, la cartella è già scritta dal compilatore. Non è detto da dove parte la fiducia.
  3. La cache sta in una cartella condivisa? Se la chiave è l'impronta del `.wasm` (C-5), due tenant che caricano lo stesso modulo condividono il file nativo: viola AD-5 («nessun supporto deduplica tra tenant») e il test a due tenant con lo stesso modulo. Se è per tenant, nessuno l'ha detto.
  4. C-13 e il test sentinella elencano «nodi» ma non la cache nativa: dopo la cancellazione del tenant restano file nativi derivati dal suo codice.
  5. La chiave della cache non comprende versione di wasmtime, configurazione del motore (epoche, limiti) né funzionalità della CPU: dopo un aggiornamento di wasmtime (le correzioni di sicurezza escono spesso, ADR) la stessa impronta del `.wasm` punta a un artefatto incompatibile o, peggio, deserializzato da una versione diversa (`unsafe`).
- **Fix:** una sola regola in AD-6. (a) La sorgente di verità è il `.wasm` nell'archivio dei payload (spazio immutabile, impronta C-5, cifrato per tenant); la cache nativa è derivata, a perdere, per tenant (`<dati>/native/<id interno del tenant>/<impronta del wasm>-<id della build del motore>-<id CPU>.cwasm`, 0700/0600), mai condivisa. (b) Il compilatore restituisce i byte sull'IPC con un tetto di dimensione; li scrive il motore, che ne calcola l'impronta da quei byte (hash, non interpretazione) e la salva come riga tipizzata scritta dal motore (stesso meccanismo dei segreti, AD-2); il compilatore non tocca mai la cartella e la sandbox ha Landlock in sola lettura sulla sola cartella del suo tenant. (c) Un artefatto con impronta diversa si scarta e si ricompila, mai si esegue. (d) La cartella della cache entra in C-13 e nel test sentinella. (e) L'id della build del motore e l'id della CPU stanno nella chiave: un aggiornamento invalida la cache, che si ricompila.

## H-3 (alta) Caricamento del modulo (FR-15) contro compilazione e versioni (C-5)

- **Unità A:** storia di pubblicazione del nodo (FR-15, C-5). Risponde alla pubblicazione con `If-Match`, versione = SHA-256 del contenuto, salvataggio durevole subito.
- **Unità B:** storia del ruolo di compilazione. Compila con un tetto di 512 MiB e 10 s (ADR), con 0,86 s per MB; può fallire.
- **Divergenza:**
  1. La versione esiste (C-5) prima che il codice nativo esista. Un workflow che la richiama parte, la sandbox non trova l'artefatto e la compilazione (fino a 10 s) cade dentro il timeout del task e dentro NFR-12. Oppure l'Unità B compila al primo uso e l'Unità A crede la versione pronta.
  2. Lo stesso modulo, stessi byte, stessa impronta, rifiutato per memoria: ricaricarlo dà la stessa versione. Un secondo rifiuto è idempotente o un tentativo nuovo? Senza stato sulla riga della versione le due storie decidono diversamente.
  3. Il «ruolo di compilazione» non è tra i cinque ruoli elencati in AD-6 («Processi») né tra i «5 ruoli su 5» provati dallo spike: nessuna lista seccomp, nessun conto di RSS, nessun limite di concorrenza. Con 512 MiB di spazio d'indirizzamento ciascuno e un processo per tenant, N tenant che caricano insieme moltiplicano la memoria.
  4. Dopo un aggiornamento del motore (FR-51) tutte le cache nativi sono invalide: ricompilazione di tutti i moduli di tutti i tenant al primo avvio, o pigra nel percorso del task.
- **Fix:** la riga della versione del nodo ha uno stato tipizzato `compiling | ready | rejected(codice C-7)`, scritto solo dal motore; la pubblicazione risponde dopo l'evento durevole con lo stato `compiling`; il contesto di validazione (AD-11) e l'avvio di un'esecuzione ammettono solo versioni `ready`; la ricompilazione dopo un aggiornamento è una fase dell'avvio del motore (in coda, un tenant alla volta, fuori dal percorso del task), mai nel timeout di un task. Il ruolo di compilazione entra nell'elenco dei ruoli di AD-6, con propria lista seccomp, classe «a richiesta», concorrenza massima configurabile (predefinito 1) e la prova della chiave principale come gli altri.

## H-4 (alta) microVM riservata, snapshot dei moduli contro cancellazione (C-13) e aggiornamento dei moduli

- **Unità A:** storia del pool della sandbox forte. Rispetta AD-6 e ADR punto 4: microVM riservata a un tenant, moduli «nella microVM o nel suo snapshot prima della prima attività», riciclo 24-31 ms.
- **Unità B:** storia di eliminazione del tenant (FR-31, C-13, AD-7) e storia delle versioni del nodo (C-5).
- **Divergenza:**
  1. Lo snapshot di una microVM contiene la memoria del guest: codice dei moduli del tenant, e dopo un'attività anche suoi dati in chiaro e segreti risolti. Sta su disco in file che Firecracker mappa, quindi non cifrabili con la busta di C-12. L'elenco del test sentinella di C-13 (database, bus, archivio, checkpoint, nodi) non li contiene: dopo l'eliminazione restano file di snapshot e dischi di sovrapposizione.
  2. Il riciclo dopo un'attività riporta la microVM allo snapshot: ma lo snapshot è quello creato con i moduli di allora. Un modulo nuovo (versione 2) o più versioni vive nello stesso tempo (esecuzioni già partite fissate alla versione 1, C-5) non stanno nello stesso snapshot. Due storie: il pool rifà lo snapshot a ogni pubblicazione (riciclo costoso, lo snapshot per ogni insieme di versioni) o consegna il modulo su vsock a ogni attività (16,6 ms, rompe NFR-12 per javy).
  3. «Una microVM è monouso o riservata a un tenant»: nel passaggio tra stato `riservata` e arresto del tenant manca chi la ferma. C-13 dice «sandbox riservate» ma non quale componente le possiede.
- **Fix:** (a) lo snapshot contiene solo immagine di base e runtime, mai dati, segreti né output dei task; è scritto in `<dati>/vm/<id interno del tenant>/` (0700) e non può contenere segreti perché questi si consegnano per chiamata (già AD-6). (b) I moduli del tenant si consegnano alla microVM su vsock alla prima attività di una versione e restano in memoria fino al riciclo; l'ottimizzazione «modulo nello snapshot» è ammessa solo per versioni `ready` e si ricrea in background, mai nel percorso del task; più versioni coesistono nella stessa microVM (stesso tenant). (c) La cartella delle microVM e dei loro dischi entra in C-13 e nel test sentinella e si cancella nel passo «fermare sandbox riservate»; il residuo fisico dei file cancellati si documenta come in AD-7 per Postgres.

## H-5 (alta) Chi parla vsock con la microVM

- **Unità A:** storia della sandbox forte. Rispetta ADR punto 4: «l'host le parla solo su vsock».
- **Unità B:** storia del motore. Rispetta AD-6: il motore non interpreta contenuto dei tenant, le risposte degli ausiliari sono input esterno con tetti e validazione.
- **Divergenza:** l'uscita del guest è contenuto di tenant (output del nodo, righe di log, errori). Il lato host del vsock è un processo: se è il motore, B è violata (un guest ostile parla direttamente con il processo che tiene la chiave); se è l'ausiliario «sandbox leggera», la lista seccomp del ruolo ora comprende vsock e il processo Firecracker. Nessuna regola dice quale. Chi avvia Firecracker (utente, `/dev/kvm`, jailer assente) e chi lo ferma non è nel ruolo «sandbox forte» né in AD-1.
- **Fix:** il lato host del vsock e il processo Firecracker di un tenant vivono in un ruolo proprio, «sandbox forte», tra i ruoli di AD-6, un processo per tenant, con la sua lista seccomp e il tetto di risposte come ogni altro ausiliario; il motore parla solo con quel ruolo sull'IPC. Il test «lettura della chiave principale» lo include.

## H-6 (alta) Ripiego senza Landlock contro il filtro di rete M-30

- **Unità A:** storia di `call http` (M-30). Rispetta il vincolo: un solo filtro, esclude gli indirizzi dell'installazione, risoluzione con il resolver del motore, controllo a ogni connessione e redirect.
- **Unità B:** storia del confinamento degli ausiliari. Rispetta AD-6 e ADR punto 7: senza Landlock `call http` e stream hanno seccomp (senza `open`, con `socket`) e un namespace utente; l'ADR ammette che raggiungono ancora il socket astratto e la porta locale del motore.
- **Divergenza:**
  1. Un filtro seccomp non legge l'indirizzo passato a `connect`, e Landlock (ABI 4) filtra porte, non indirizzi: in nessuno dei due modi il kernel applica M-30. Il filtro è solo codice in processo. L'Unità A lo scrive come difesa principale; l'Unità B crede che il confinamento sia il secondo strato. Nessuna delle due dice che con il ripiego resta un solo strato.
  2. «Dove Landlock manca» non è definito: il crate `landlock` per impostazione predefinita applica quel che può (`PartiallyEnforced`) e tace. Su un kernel con Landlock ABI 1 (senza regole di rete) due storie risolvono «presente» in due modi: una applica il file system e considera tutto coperto, l'altra va nel ripiego.
  3. Le due regole del ripiego («socket ereditato» e «la porta locale chiede sempre le credenziali») contraddicono C-9 e C-14: `/metrics` su localhost non chiede credenziali («fuori da localhost segue C-14»), e un endpoint di salute senza credenziali è nell'Open Question. Un `call http` verso `http://127.0.0.1:<porta>/metrics` è raggiungibile in ripiego.
  4. La sandbox leggera e la sandbox forte: «la rete della microVM passa dallo stesso filtro» (M-30), ma la leggera è seccomp senza `socket`: la rete dei nodi passa tutta da `call http`? Non detto.
- **Fix:** (a) una sola funzione del dominio, `HostCapabilities` (KVM, ABI di Landlock richiesta con livello `HardRequirement`, namespace utente), calcolata una volta dal motore all'avvio, scritta nel registro e passata agli ausiliari: «Landlock presente» significa ABI ≥ quella fissata dalla spine (da decidere; se serve la rete, ABI 4), altrimenti ripiego. (b) Il filtro M-30 è in processo e obbligatorio in tutte le modalità, con una prova SSRF (loopback, indirizzi dell'installazione, IPv6 mappato, redirect, DNS che cambia) eseguita sia con Landlock sia nel ripiego; il confinamento non è mai citato come sua copertura. (c) Nessuna rotta senza credenziali sulla porta TCP del motore quando un ausiliario di rete gira in ripiego: `/metrics` e salute si servono su un socket Unix o su una porta separata. (d) La sandbox leggera non ha rete propria: ogni richiesta di rete di un nodo passa dal ruolo `call http` dello stesso tenant.

## H-7 (alta) Coppia di socket ereditata contro il riavvio dopo l'arresto per inattività

- **Unità A:** storia del supervisore degli ausiliari. Rispetta AD-6: arresto per inattività, una nuova coppia di socket a ogni avvio, `RLIMIT_NOFILE` 64, nessun socket in ascolto.
- **Unità B:** storia di `call http` (e di jq con cache) e storia della sandbox forte, che tengono stato dentro l'ausiliario: pool di connessioni per tenant (AD-6 lo ammette dentro il tenant), cache di moduli compilati caricati, microVM calde.
- **Divergenza:**
  1. Corsa: il motore scrive una richiesta mentre l'ausiliario ha deciso di uscire per inattività (o viceversa). La richiesta si perde o l'EOF arriva dopo. Se il motore la ritenta su un processo nuovo, un POST di `call http` già partito dall'ausiliario vecchio si ripete: un doppione non provocato da C-2.
  2. L'arresto lo decide l'ausiliario (timer interno) o il motore? AD-6 non lo dice: due storie decidono in due modi.
  3. Il crash del motore lascia gli ausiliari: con la coppia di socket nessuno li ha ereditati dal motore nuovo; senza `PR_SET_PDEATHSIG` o la lettura di EOF restano vivi con la chiave del tenant (stream) e senza che il motore nuovo li conosca, e il motore nuovo ne avvia un secondo per lo stesso tenant e ruolo (FR-16 non violata, NFR-1 sì).
  4. Il motore tiene tutte le estremità: un ausiliario avviato dopo eredita le estremità degli altri se non sono `CLOEXEC`. La chiusura dei descrittori è fatta dall'ausiliario dopo `exec`; se il motore le dimentica aperte, un ausiliario riceve per un istante socket di un altro tenant.
- **Fix:** in AD-6: (a) l'arresto per inattività lo decide solo il motore, con una macchina a stati per (tenant, ruolo) `Avviato → Drenaggio → Fermo` sotto un lock, con nessuna richiesta nuova accettata in `Drenaggio`; (b) il motore non ritenta mai su un ausiliario nuovo una richiesta già scritta su un socket, se l'effetto non è idempotente: l'esito è «sconosciuto» e passa dai tentativi del task con la stessa `attemptId` (C-2); (c) l'ausiliario esce sull'EOF del socket e con `PR_SET_PDEATHSIG`, azzerando le chiavi; (d) il motore crea ogni coppia con `SOCK_CLOEXEC` e passa a ogni figlio solo la propria estremità (`close_range` dalla parte del motore, prova nel test di FR-16).

## H-8 (media) `PR_SET_DUMPABLE` a 0 contro operazione e debug

- **Unità A:** storia dei processi ausiliari. Rispetta AD-6: `DUMPABLE` a 0, core 0, ordine fissato (userns assente dall'ordine).
- **Unità B:** storia dell'operatività (metriche per ausiliario in NFR-1, FR-48, registro C-9, riavvio).
- **Divergenza:**
  1. Con `DUMPABLE` 0 i file di `/proc/<pid>/` come `io`, `smaps_rollup`, `fd` diventano illeggibili anche al motore; l'RSS che NFR-1 somma si legge da `statm`, che resta leggibile, ma `smaps_rollup` (PSS) no: la storia di NFR-1 sceglie la fonte e nessuno ha detto quale è ammessa.
  2. Il passo del namespace utente non sta nell'ordine del confinamento. Se lo scrive il motore (`uid_map` del figlio), con `DUMPABLE` 0 il file è del root e la scrittura fallisce; se è l'ausiliario a farlo, deve stare prima di `NO_NEW_PRIVS`/seccomp. Due storie, due posti.
  3. Un ausiliario ucciso da seccomp (`SIGSYS`) esce senza core e senza motivo. Il motore vede solo il segnale; il tenant che provoca un `SIGSYS` ripetuto causa un riavvio per ogni richiesta (4,6 ms l'uno, e CPU di chi non ha mai pagato): una tempesta di riavvii. Dopo un aggiornamento di wasmtime una lista seccomp non aggiornata dà lo stesso segnale e sembra un attacco.
  4. Non esiste un interruttore di debug, e non deve esistere (indebolirebbe la chiave principale), ma nessuno lo dice: una storia aggiungerà `TOPICTOAST_DEBUG_DUMPABLE`, che C-19 rifiuterebbe solo se sconosciuto.
- **Fix:** (a) l'ordine del confinamento in AD-6 include il passo `unshare(CLONE_NEWUSER)` con `uid_map` scritta dall'ausiliario stesso, prima di Landlock; (b) l'RSS di NFR-1 si legge da `/proc/<pid>/statm`, l'unica fonte ammessa; (c) tabella di uscite in C-7: `helper.confinement_violation` (`SIGSYS`), `helper.limit_exceeded`, `helper.crashed`, con ruolo e identificativo interno del tenant (mai stderr, C-9), e un tetto di riavvii per (tenant, ruolo) con attesa crescente; (d) regola scritta: nessuna opzione, variabile o sottocomando riattiva `DUMPABLE` o i core dump; la diagnosi passa dai codici e dal registro.

## H-9 (media) Sandbox forte attivata senza KVM: dove sta l'attivazione, quando si rifiuta, dove sta l'avviso (FR-48, FR-54)

- **Unità A:** storia della configurazione dell'installazione (C-19). Le chiavi del TOML, senza ricarica a caldo, con nomi di tenant esterni e predefiniti applicati dopo la validazione.
- **Unità B:** storia della dashboard dell'operatore e del contesto di validazione. FR-54 dice «per tutti i tenant o per alcuni» e «avviso all'operatore nella dashboard»; FR-48 elenca solo avvisi di worker e di disco.
- **Divergenza:**
  1. Due proprietari dell'attivazione per tenant: elenco nel file TOML (`sandbox.strong.tenants`, nomi esterni, riavvio per cambiarlo) oppure colonna tipizzata sulla riga del tenant (AD-4) modificata dalla dashboard.
  2. Quando si rifiuta: all'avvio (il motore non parte, come per configurazione non valida in C-19), alla pubblicazione del nodo (il validatore lo sa) o al task (`failed` con codice). L'ADR dice «i nodi si rifiutano»; il PRD dice «con un avviso nella dashboard». Un rifiuto al task attraversa i tentativi (AD-9) e li consuma.
  3. L'avviso non è un evento del bus né una riga della proiezione: la capacità dell'host non è stato dell'esecuzione (AD-2). Nessuno possiede il dato; il motore può scriverlo solo come segreti/chiavi (AD-2, «Scritture del motore»).
  4. Lo stesso problema, con un altro codice, per «ausiliario che non parte per mancanza di userns» (AD-6): due codici, due avvisi, nessuna pagina che li ospiti (FR-48 parla di worker).
- **Fix:** (a) una sola fonte, la colonna del tenant (`sandbox_level: light | strong`), cambiata dall'API dell'integratore (FR-54), mai nel TOML; il TOML porta solo i parametri del pool. (b) `HostCapabilities` (H-6) è calcolata all'avvio ed esposta da una risorsa dell'operatore `GET /v1/operator/capabilities`, letta dalla dashboard (FR-48) per l'avviso; nessun evento. (c) Il rifiuto sta al primo punto utile: il contesto di validazione del tenant (AD-11) porta `strongSandboxAvailable`, la pubblicazione di un nodo del tenant con `strong` senza KVM è rifiutata con un codice C-7, e un'attività che arriva comunque fallisce con codice di classe non ritentabile (nessun tentativo consumato). (d) Un solo prefisso nel catalogo C-7 per «funzione dell'host non disponibile» (`host.capability_missing` con il nome della capacità nei `params`).

## H-10 (bassa) Chi fissa il tetto degli ausiliari attivi

- **Unità A:** storia del supervisore (H-1, H-7). **Unità B:** storia dell'installazione e delle quote (NFR-1, FR-30, NFR-14).
- **Divergenza:** l'Open Question rimanda il tetto alla spec della prima storia; ma `RLIMIT_NPROC` dell'utente del motore, il limite dei descrittori e l'arresto per inattività dipendono dal tetto: due storie lo fissano in due punti (config contro limite del processo), e un tenant che apre molti ruoli in sequenza (ruoli = 6 con compilazione e sandbox forte) può sforare il numero di processi prima che l'inattività li chiuda.
- **Fix:** un solo parametro di installazione in C-19 (`helpers.max_active`), con coda in attesa e codice di rifiuto (stesso prefisso di H-9) quando è raggiunto, e `RLIMIT_NPROC` derivato da quel valore, non fissato a mano.

---

## Ordine di chiusura consigliato

1. H-1 e H-7 (ciclo di vita unico e macchina a stati del supervisore): sbloccano la prima storia degli ausiliari.
2. H-2, H-3 (cache nativa e stato della versione): prima della prima storia dei nodi (FR-15).
3. H-6, H-9 (`HostCapabilities` e rifiuti): prima della storia del confinamento e di `call http`.
4. H-4, H-5 (sandbox forte): prima della storia della sandbox forte.
5. H-8, H-10: nelle spec delle storie relative.
