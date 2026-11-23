# ADR: driver SQL, sqlx per SQLite e Postgres

- Data: 2026-11-23
- Stato: accettata
- Registra: riga dello Stack della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md); chiude la domanda aperta su driver e versione
  di SQLite, da chiudere prima di S-1
- Fonti: costituzione V; AD-1, AD-5; [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 73, 74 e 77;
  [transact-rs/sqlx#4516](https://github.com/transact-rs/sqlx/issues/4516) (RUSTSEC-2026-0459)

## Contesto

Il motore usa SQLite incorporato o Postgres, a scelta dell'integratore (AD-1). Un solo adattatore
del repository applica sempre il filtro del tenant, con entrambi i database (AD-5, costituzione V).
SQLite non aveva una versione nello Stack: la porta il driver. Lo spike del bus usa già SQLite.

RUSTSEC-2026-0459 (CVE-2026-119375) riguarda SQLite prima della 3.54.0: con un database e un WAL
di dimensioni di pagina diverse, SQLite può restituire memoria non inizializzata. sqlx 0.9.0 limita
libsqlite3-sys sotto la 0.38, quindi resta su un SQLite vulnerabile. sqlx 0.9.1 (2026-11-18)
ammette libsqlite3-sys fino alla 0.40, con SQLite 3.54.

## Decisione

- sqlx è il driver per SQLite e per Postgres: un solo driver asincrono, quindi un solo adattatore.
- Versione 0.9.1, fissata subito con l'eccezione per le correzioni di sicurezza, con la feature
  `sqlite` (SQLite incluso nel binario), mai `sqlite-unbundled`. sqlx 0.9.1 ammette
  libsqlite3-sys fino alla 0.40 ma non impone la versione corretta: il lockfile fissa
  libsqlite3-sys 0.39.1 (SQLite 3.54; la 0.40.0 ha meno di due settimane), con
  [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html) citato nel commit, e
  il controllo degli avvisi nel pre-push impedisce di tornare indietro. sqlx 0.9.1 richiede Rust
  1.94; lo Stack ha 1.99.1.
- Le query sono testo SQL controllato a runtime: niente controllo a compile-time contro un database
  attivo, che legherebbe la build a un database o a file generati. In sqlx 0.9 una query che non è
  una stringa costante passa da `AssertSqlSafe`: si usa solo dentro l'adattatore, e il filtro del
  tenant è sempre un parametro legato, mai testo composto.
- Migrazioni e linea del dialetto (generico sul database di sqlx, `Any` o SQL per dialetto) sono
  rinviate alla spec della prima storia che crea lo schema (Deferred della spine).

## Bilancio

- **Funzioni:** stesse query e stesso adattatore per i due database; SQLite corretto fin dal primo
  spike.
- **Costi:** una versione del driver più giovane delle due settimane, accettata per la sicurezza. Le
  differenze di dialetto tra SQLite e Postgres restano dentro l'adattatore e sotto il test a due
  tenant su entrambi.
- **Manutenzione:** una sola libreria da seguire per gli avvisi; gli aggiornamenti di SQLite passano
  da libsqlite3-sys.

## Alternative scartate

- **rusqlite più tokio-postgres:** due driver, uno sincrono e uno asincrono. L'adattatore unico
  dovrebbe nascondere la differenza, con due strade per ogni query.
- **sqlx 0.9.0 con l'avviso ignorato nell'audit:** versione più vecchia, ma lo spike del bus
  girerebbe già su un SQLite vulnerabile.
