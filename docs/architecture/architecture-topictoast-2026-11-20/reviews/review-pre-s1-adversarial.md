# Revisione avversaria: decisioni prima di S-1 (driver SQL, configurazione, licenze)

Oggetto: diff di `main` su `CLAUDE.md`, `ARCHITECTURE-SPINE.md` (Stack, riga Configurazione, Open Questions), `conventions.md` (C-19), `.memlog.md`, `docs/decisions/README.md` e i tre ADR `2026-11-23-sql-driver.md`, `2026-11-23-configuration-format.md`, `2026-11-23-dependency-licenses.md`.
Lente: due unità un livello sotto che rispettano ogni regola alla lettera e costruiscono in modo incompatibile.

Verdetto: nessuna contraddizione frontale con gli AD, ma C-19 e AD-6 insieme mandano segreti ai processi ausiliari, e restano senza proprietario le migrazioni con un utente non superuser, il prefisso condiviso con CLI e SDK, il confine di cosa è un segreto e il perimetro del controllo licenze.

## A. Configurazione (C-19)

### 1. «Configurazione risolta» sull'IPC contiene i segreti (AD-6, C-19, C-12)
- Dove: AD-6 («passa sull'IPC la configurazione risolta»); C-19 («segreti ... con una variabile o con il percorso di un file»).
- Problema: con C-19 la configurazione risolta include password del database, credenziali del bus, credenziali di RustFS, il segreto della chiave di firma se c'è. AD-6 dice solo che l'ausiliario non riceve la chiave principale. Unità A (motore) serializza l'intera struttura sull'IPC; unità B (ausiliario) la riceve intera: tutte e due rispettano la lettera di AD-6, e il processo che esegue contenuto ostile dei tenant ha la password del database e del bus. Il filtro della rete in uscita (M-30) esclude gli indirizzi, non le credenziali.
- Fix: scrivere in AD-6 o C-19 che l'ausiliario riceve una proiezione per ruolo (limiti, tetti, indirizzi che gli servono), mai segreti di installazione; tipo distinto nel dominio, test che la proiezione non contiene campi segreti.

### 2. «Toglie dal proprio ambiente la variabile della chiave principale» non basta e non nomina la variabile (AD-6, C-12, C-19)
- Dove: AD-6; C-12 («file o variabile d'ambiente»); C-19.
- Problema: tre punti non fissati. (a) Il nome: è `TOPICTOAST_<SEZIONE>__<CHIAVE>` (quale sezione?) o un nome speciale? Due unità scelgono nomi diversi e il compose di sviluppo non parte. (b) Le altre variabili segrete (password del database, bus, RustFS) restano nell'ambiente del motore: AD-6 ne toglie una sola. (c) `remove_var` toglie la variabile dall'ambiente del processo ma non dal blocco iniziale letto da `/proc/<pid>/environ`; è `unsafe` nell'edizione 2024 con più thread. `PR_SET_DUMPABLE` copre solo chi non è lo stesso utente.
- Fix: C-19 elenca i segreti per nome (tabella nel glossario), li legge una volta all'avvio prima di avviare i thread, e li dichiara «solo da file» o «variabile ammessa, rimossa»; il tipo del segreto si azzera (già in AD-6). Preferire «percorso di un file» come via normale e la variabile come ripiego documentato.

### 3. Prefisso `TOPICTOAST_` condiviso con CLI, SDK, worker, bench, e chiavi sconosciute rifiutate (C-19)
- Dove: C-19 («chiavi sconosciute ... rifiutate», variabile `TOPICTOAST_<SEZIONE>__<CHIAVE>»).
- Problema: la regola è scritta per il motore. CLI, SDK Rust e TS, worker di integratori e bench hanno bisogno di indirizzo, credenziale, certificato; l'ovvio è `TOPICTOAST_URL`, `TOPICTOAST_TOKEN`. In un container o in uno script che li espone insieme al motore, il motore rifiuta di partire per «chiave sconosciuta» (senza `__` o con sezione inesistente). Stesso problema per la variabile che indica il file di configurazione («indicato all'avvio»: flag o variabile? se è `TOPICTOAST_CONFIG` è già una chiave sconosciuta).
- Fix: C-19 dice quali variabili legge il motore (solo `TOPICTOAST_<SEZIONE>__<CHIAVE>` con doppio trattino basso; le altre `TOPICTOAST_*` ignorate o con avviso, non rifiutate), nome fisso e riservato della variabile del file, e un prefisso diverso o una sezione per i client (`TOPICTOAST_CLIENT__…`) con lo stesso parser.

### 4. Annidamento, liste e mappe non definiti; `__` come separatore
- Dove: C-19 («raggruppate in sezioni», `SEZIONE__CHIAVE`).
- Problema: C-19 dice una sola sezione e una chiave. Ma servono liste (origini CORS, issuer di C-15, indirizzi d'ascolto, certificati, algoritmi) e probabilmente sezioni annidate (`bus.iggy.…`). Unità A: `TOPICTOAST_CORS__ORIGINS=a,b`; unità B: JSON, o `ORIGINS__0=a`. Con le chiavi che contengono già `_` (`payload_store`) basta il parser per `__`, ma una sezione annidata dà `A__B__C` e un nome di sezione che finisce con `_` diventa ambiguo. Nessuna regola per variabile vuota (`=`) contro assente, né per booleani e durate (`30s`, `30000`), né per interi grandi (C-10: `3` non è `3.0`; da env tutto è testo).
- Fix: un paragrafo in C-19: profondità massima (una sezione, o N), codifica delle liste (separatore e escape, oppure solo nel file), vuoto = assente o errore, sintassi di durate e dimensioni, un solo modulo che fa il parsing (la regola «un solo punto» dell'ADR oggi non dice in quale crate sta).

### 5. Cosa conta come segreto, e cosa succede se è nel file
- Dove: C-19 («segreti mai nel file»); ADR configurazione («chiave principale, password del database, credenziali del bus»).
- Problema: l'elenco è tre esempi. Mancano: URL del database con password incorporata (`postgres://u:p@h/db`, il formato naturale per sqlx), credenziali di RustFS, chiave privata TLS (percorso: ok; contenuto incollato: no), segreti dei worker (C-11) e le credenziali generate al primo avvio (C-14). Né C-19 né gli ADR dicono l'esito: errore all'avvio (e con quale codice, il catalogo C-7 non ha codici di configurazione), avviso, o accettato. Unità A rifiuta qualsiasi campo segreto trovato nel file; unità B lo accetta con un avviso. Inoltre `toml` e serde stampano nell'errore la riga con il valore: «mai il valore» di C-19 è violabile dal crate scelto.
- Fix: elenco chiuso dei campi segreti (tipo `Secret`, fuori dalla struttura deserializzata dal file, quindi «chiave sconosciuta» per costruzione se compare nel file), codici `config.*` nel catalogo C-7, e una regola esplicita: errori di parsing senza frammento della sorgente; test che una chiave segreta nel file dà il codice e nessun valore in output.

### 6. «Segreto da un percorso di file»: come si dice e dove
- Dove: C-19; C-12.
- Problema: la convenzione per il percorso non c'è (`master_key_file` come chiave sorella, suffisso `_FILE` come Docker secrets, valore `file:/path`?). Il percorso può stare nel file TOML? Il testo «mai nel file, solo da variabile o da percorso» si legge in due modi. Permessi del file (0600), spazi e a-capo finale nel contenuto, percorso relativo (rispetto a cosa?) non detti.
- Fix: una sola forma (proposta: chiave sorella `<nome>_file`, ammessa anche nel TOML, il contenuto è letto senza a-capo finale, permessi più larghi di 0600 = errore) e un test.

### 7. Errore «C-7 con il percorso» all'avvio: C-7 è HTTP, il percorso è JSON Pointer
- Dove: C-19; C-7 (`path` JSON Pointer nel documento).
- Problema: all'avvio non c'è risposta HTTP. Dove esce l'errore: stderr, stdout (C-9 dice log JSON su stdout), codice di uscita? Il `path` può essere `/database/url` (JSON Pointer), `database.url` (TOML) o `TOPICTOAST_DATABASE__URL` (se il valore veniva dall'ambiente). Due unità producono tre formati; i test BDD sul «motore non parte» non sono scrivibili.
- Fix: stabilire: l'errore esce su stderr come JSON con `errors[]` di C-7, `path` sempre JSON Pointer di sezione/chiave più un campo `source` (`file`, `env`); codice di uscita non zero fisso.

### 8. Precedenza completa e limiti, «entro il massimo dell'installazione» (C-19, NFR-14, C-10)
- Dove: C-19.
- Problema: precedenza detta solo tra variabile e file. Argomenti di riga di comando: esistono? Il file mancante ma indicato esplicitamente è errore (deve esserlo, a differenza del file assente per default)? «Massimo dell'installazione»: i massimi sono costanti del codice o chiavi? Se sono chiavi, chi le limita? Un predefinito sopra il massimo (per un massimo abbassato nel file) è errore o clamp? C-10 dice «predefiniti dopo la validazione»: se il predefinito non è validato, un predefinito sbagliato in codice passa; se lo è, i due passaggi si invertono.
- Fix: C-19 precisa: nessun argomento per valori (solo per il percorso del file), file indicato e mancante = errore, massimi costanti del codice (documentati), un test sul tipo del predefinito (ogni predefinito sta entro il massimo, test unico che attraversa la struttura).

### 9. Chi legge l'ambiente: unità non testabili in parallelo
- Dove: ADR configurazione, Manutenzione («un solo punto che legge e valida»).
- Problema: non si dice che il lettore prende una mappa di variabili come argomento invece di leggere `std::env`. Gli scenari cucumber con motore in-process e `testcontainers` girano in parallelo: variabili globali di processo si contaminano (e `set_var` è `unsafe`). Una unità legge `std::env`, l'altra accetta una mappa: il test harness delle storie non si compone. Dominio puro (costituzione III): la struttura tipizzata della configurazione sta nel dominio o nel crate del motore? Se sta nel dominio, serve che il crate non legga l'ambiente; gli adattatori (`crates/adapters`) leggono parametri di connessione dalla stessa struttura.
- Fix: C-19: il caricamento è una funzione `(file: Option<&str>, env: &dyn Fn(&str)->Option<String>) -> Result<Config, Vec<Error>>` in `crates/engine`; i tipi di sezione che servono agli adattatori sono nel dominio o in un crate di tipi, e nessun adattatore legge l'ambiente.

## B. Driver SQL (sqlx)

### 10. Migrazioni: nessun proprietario, e l'utente dell'applicazione non può eseguirle (AD-4, AD-5, costituzione V, FR-51)
- Dove: ADR driver; AD-5 («utente dell'applicazione non superuser e senza `BYPASSRLS`», RLS); AD-4 («ogni cambio della spec OWS è una migrazione»); Aggiornamenti («migrazione automatica»).
- Problema: nulla dice dove stanno le migrazioni: una cartella con lo stesso SQL, due cartelle (`sqlite/`, `postgres/`), o codice? Né chi le lancia al boot (l'adattatore, il motore, un sottocomando) né con quale utente. `CREATE POLICY`, `ALTER TABLE … ENABLE ROW LEVEL SECURITY` e le tabelle richiedono un ruolo proprietario; l'utente di AD-5 non può. Quindi servono due credenziali (migratore e applicazione) o un passo separato: C-19 e il compose di sviluppo devono prevederlo, e la prima storia sceglie a caso. Con sqlx `migrate!` le migrazioni consegnate non si modificano (checksum): coerente con CLAUDE.md, ma la regola «una migrazione per cambio di formato» vale per i due dialetti insieme? Un test che le due cartelle producono lo stesso schema logico non è previsto.
- Fix: ADR o riga di AD-5: le migrazioni stanno in `crates/adapters` (due directory, stesso numero e nome), le esegue il motore all'avvio con una credenziale migratore distinta (sezione di configurazione `database.migrator_*`, segreto), poi apre il pool con l'utente dell'applicazione e verifica `NOT superuser / NOT bypassrls`; un test di parità dello schema tra i due dialetti.

### 11. «Le differenze di dialetto restano dentro l'adattatore»: dove passa la linea?
- Dove: ADR driver, Costi; AD-5.
- Problema: sqlx offre `Any` (poco tipi: niente UUID, niente timestamp tipati), due `Pool<Sqlite>` / `Pool<Postgres>` con codice duplicato, o generico sul tipo `Database`. Segnaposti `?` contro `$1`, `RETURNING`, `UUID` (testo o BLOB in SQLite contro `uuid` nativo), tempi (microsecondi, R24 ancora aperta), booleani, `FOR UPDATE SKIP LOCKED` solo in Postgres. Peggio: la semantica di concorrenza cambia. SQLite ha un solo scrittore (`SQLITE_BUSY`), Postgres lavora in `READ COMMITTED`: la stessa sequenza di `If-Match` o di chiave di idempotenza (C-3, C-4, AD-18) può dare conflitto in uno e riuscire nell'altro. NFR-4 («≤ 5 istruzioni SQL per attività con SQLite e con Postgres») fissa il budget ma non lo stesso piano di query. RLS richiede di impostare il tenant per transazione (`SET LOCAL`) su connessioni di un pool: una connessione non resettata filtra il tenant sbagliato; in SQLite non c'è nulla di equivalente e il filtro lo fa solo il codice.
- Fix: nell'ADR: scelta di una forma (proposta: un tipo generico con una sola serie di query in un modulo e un piccolo trait di dialetto per le differenze, non `Any`); livello di isolamento dichiarato per ogni transazione e `BEGIN IMMEDIATE` in SQLite per le scritture; `SET LOCAL` solo dentro una transazione con test che una connessione riusata non porta il tenant precedente; la suite a due tenant gira sugli stessi scenari per entrambi.

### 12. SQLite incluso: nulla impone `bundled`, né le opzioni di compilazione che AD-7 richiede
- Dove: Stack («SQLite 3.54, incluso tramite libsqlite3-sys ≥ 0.39»); AD-7 (`secure_delete`, checkpoint `TRUNCATE`).
- Problema: la feature `sqlite` di sqlx è incorporata per impostazione predefinita, `sqlite-unbundled` usa la libreria di sistema: un pacchetto di distribuzione Linux o una immagine con SQLite 3.46 compila e passa il gate ma torna vulnerabile a RUSTSEC-2026-0459. Inoltre `libsqlite3-sys` ≥ 0.39 ha una sola versione di SQLite per versione del crate, quindi «3.54 nella riga dello Stack» è un fatto da verificare, non un pin. Le opzioni di compilazione (`SQLITE_SECURE_DELETE`, `SQLITE_THREADSAFE`, estensioni) non si controllano da sqlx. Rischio di lockfile: con le feature di sqlx non ridotte (macro, migrate con più driver) il lockfile può portare crate di driver non usati (`sqlx-mysql`, `rsa`) che il futuro `cargo audit` segnala.
- Fix: in Stack e nell'ADR: `default-features = false`, elenco esatto delle feature, `sqlite` incorporata e `sqlite-unbundled` vietata dal gate (grep sul lockfile o `cargo tree`); un test all'avvio che `sqlite_version()` ≥ 3.54.0 e che `secure_delete` è attivo; `cargo audit` nel gate prima di S-1 (la riga aperta M13 oggi lo lascia «da valutare»).

### 13. Eccezione di sicurezza: sqlx 0.9.1 ha 5 giorni, e i test non coprono la regola
- Dove: ADR driver; CLAUDE.md (due settimane, eccezione per le vulnerabilità).
- Problema: la regola dell'eccezione è rispettata, ma l'ADR non dice quando l'eccezione scade né cosa succede se 0.9.1 ha un difetto (la 0.9.2 uscita la settimana dopo è «più giovane di due settimane» e va fissata?). Nessun criterio di uscita: il ritorno alla regola normale quando esiste una versione con più di due settimane.
- Fix: una riga: «alla prima 0.9.x con più di due settimane, si passa a quella»; l'eccezione non si estende ad altre dipendenze.

## C. Licenze

### 14. Il controllo «automatico nel pre-push» è aggirabile e lo strumento non c'è; chi lo chiede?
- Dove: ADR licenze, Controllo; spine, Stack; CLAUDE.md.
- Problema: `pre-push` ha `--no-verify`, `SKIP_CARGO`, `SKIP_PACKAGES` e nessuna CI fino alla produzione: il controllo è consultivo come tutto il resto. Rust (`cargo deny` o equivalente) e npm (`license-checker` o equivalente) sono due strumenti con due formati di eccezione; «lo fissa la prima storia che aggiunge una dipendenza» lascia a quella storia di scegliere, e quella storia è la stessa che ne ha bisogno. Gli strumenti sono a loro volta dipendenze di sviluppo con la regola delle due settimane. Il controllo vale a ogni cambio del lockfile (una dipendenza che cambia licenza in un aggiornamento, già successo con BUSL), non solo alla prima aggiunta.
- Fix: stabilire in una riga di CLAUDE.md: il gate (e non solo il pre-push) esegue il controllo; fallisce a ogni differenza del lockfile; la prima storia consegna il controllo con la configurazione (`deny.toml` e l'equivalente per npm) e un test che fallisce con una licenza esclusa.

### 15. Perimetro: cosa è «dipendenza» e quali albero/bersagli entrano
- Dove: ADR licenze, Decisione («Rust e npm, sia del motore sia del kit»).
- Problema: non nominati: `dashboard/` (ha un proprio gate e un albero npm separato da `packages/`; legge `packages/ui`), `spikes/` (fuori dal gate, ma l'ADR di spike controlla la licenza), dipendenze di sviluppo e di test (testcontainers, wiremock, Playwright, Storybook), pacchetti npm opzionali per piattaforma (binding napi-rs, `@emnapi/*`), font e icone di `packages/ui` (OFL, CC-BY non sono nell'elenco), immagini Docker dei servizi (Postgres sotto la licenza PostgreSQL, Valkey BSD, Iggy, RustFS Apache): servizi, non dipendenze, ma il testo non lo dice. Due unità decidono diversamente se `dashboard/` o le dipendenze di sviluppo sono sotto controllo.
- Fix: elenco dei bersagli: tutti i lockfile del repository esclusi `spikes/` (che hanno il controllo nell'ADR); dipendenze di sviluppo incluse oppure in una lista a parte con un elenco più largo, deciso adesso; asset non codice dichiarati; i servizi esterni fuori dal perimetro.

### 16. Sintassi SPDX: `OR`, `AND`, `WITH`, doppie licenze, MPL «non modificato»
- Dove: ADR licenze.
- Problema: l'elenco è di identificatori semplici. Non è deciso: `MIT OR GPL-3.0` (ammesso se una alternativa è permessa? la sintassi `OR` sì, è la norma Rust; elkjs è `EPL-2.0 OR GPL-3.0-or-later`, nessuna alternativa ammessa quindi resta escluso, ma la regola va detta); `AND` (tutte devono essere ammesse: `ring` è `ISC AND OpenSSL AND MIT`, OpenSSL non è nell'elenco, quindi `rustls` con `ring` fallirebbe; il TLS è già fissato in C-14, quindi controllare adesso); `Apache-2.0 WITH LLVM-exception` (wasmtime e simili); `Unlicense`, `BlueOak-1.0.0`, `Python-2.0`, `CC-BY-4.0` (dati di `caniuse-lite`); il testo di una licenza non SPDX. «MPL-2.0 non modificata» non è verificabile da uno strumento: patch, fork e `[patch]` o `overrides` di npm la rendono modificata senza che la licenza cambi.
- Fix: la regola dello strumento è «espressione SPDX soddisfatta se esiste una scelta in cui tutti gli identificatori sono ammessi», con `WITH LLVM-exception` ammesso; aggiungere `OpenSSL` o una eccezione a priori per `ring`, `Unlicense`, `BlueOak-1.0.0` dopo avere guardato il lockfile reale in S-1; vietare `[patch]`/`overrides` su un pacchetto MPL (grep nel gate).

### 17. Chi decide un'eccezione, e a quale granularità
- Dove: ADR licenze («eccezioni: solo con un ADR, caso per caso»); CLAUDE.md («any other license needs an ADR first»).
- Problema: un ADR accettato non si riscrive (CLAUDE.md): un'eccezione per `pacchetto@versione` si accumula a ogni aggiornamento, una per nome copre versioni future con licenza diversa. Un ADR «accettata» dipende da Marco: lo sprint si ferma finché risponde. Dove vive l'elenco machine-readable delle eccezioni (il file dello strumento) e chi lo tiene allineato agli ADR? Un'eccezione per una dipendenza indiretta di cui si è fidata una libreria ammessa?
- Fix: un'eccezione è `nome + intervallo di versione + ADR` nel file dello strumento, con un test che ogni voce cita un ADR esistente; per le indirette preferire sostituire la dipendenza diretta.

## D. Incoerenze con il resto della spine e silenzi

### 18. Residui di righe chiuse
- Dove: spine, «Aggiornamenti e distribuzione» (riga «Formato di distribuzione, formato di configurazione, supervisione del riavvio: domande aperte»); spine, tabella di mappa NFR (riga «Licenza: NFR-11 | … | Open Questions (licenze)»).
- Problema: le due Open Questions sono chiuse e rimosse, ma queste due righe le citano ancora come aperte. Unità che leggono la sezione di distribuzione concludono che la configurazione non è decisa.
- Fix: togliere «formato di configurazione» dalla prima riga e rimandare a C-19; cambiare la seconda in «Stack, ADR licenze».

### 19. NFR-1 con SQLite e sqlx, e M-02 prima di S-1 (due punti nei criteri)
- Dove: NFR-1 («≤ 80 MB con SQLite, bus e …»), S-1 (usa già SQLite).
- Problema: sqlx SQLite apre un thread di lavoro per connessione e un pool; il budget di memoria e il numero di connessioni (scritture serializzate) sono impostazioni di configurazione (NFR-14) che C-19 non ha ancora come chiavi. Lo spike S-1 gira con sqlx 0.9.1 ma la riga dello Stack lo dice «fissato», mentre la domanda aperta «un solo proprietario della forma delle connessioni» (M-02 della rubrica) resta aperta prima di S-1 e riguarda lo stesso oggetto (indirizzo del database, del bus, di RustFS).
- Fix: unire le due: una sola sezione `database` con `kind`, `url`/`path`, `pool_max` entro il massimo; chiudere M-02 insieme a C-19.

### 20. Silenzi: nessuna ricarica a caldo ma niente su file di configurazione e CORS/TLS a riavvio, `iss`, chiavi dei certificati
- Dove: C-19 («nessuna ricarica a caldo»); C-14 (certificati forniti dall'integratore, rinnovo), C-15 (`iss`, durata massima).
- Problema: i certificati TLS scadono: «nessuna ricarica a caldo» implica un riavvio per il rinnovo (entro i 30 s di NFR-15 ma con un taglio delle connessioni SSE e dei worker). Non è né deciso né rimandato. L'aggiunta di un tenant o di una credenziale di worker è stato dell'installazione (database), non configurazione: il confine «configurazione contro stato» non è scritto, e le due unità decidono dove mettere (ad esempio) il `default` tenant o gli `iss` multipli.
- Fix: una riga in C-19: la configurazione è solo ciò che serve all'avvio; tenant, credenziali dei worker, quote (FR-30) sono stato nel database e hanno API; rinnovo del certificato = riavvio, documentato in `docs/user/`.

## Cinque punti da chiudere prima di S-1

1. Proiezione della configurazione per gli ausiliari, senza segreti (n. 1).
2. Proprietario, directory e credenziale delle migrazioni, con l'utente non superuser (n. 10).
3. Confine di C-19 con i client: prefisso e variabile del file, nessun rifiuto delle variabili altrui (n. 3).
4. Elenco chiuso dei segreti, codice d'errore e niente valori negli errori (nn. 5, 6, 7).
5. Perimetro e sintassi del controllo licenze, con `OpenSSL`/`ring` verificato sul lockfile reale (nn. 14, 15, 16).
