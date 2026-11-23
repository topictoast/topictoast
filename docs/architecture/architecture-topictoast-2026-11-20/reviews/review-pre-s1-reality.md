# Revisione pre-S1: verifica dei fatti contro il web (2026-11-23)

Oggetto: ADR `2026-11-23-sql-driver`, `2026-11-23-configuration-format`, `2026-11-23-dependency-licenses`.
Metodo: crates.io API, rustsec.org, changelog di sqlx, e un albero di dipendenze reale risolto con
`cargo metadata` (sqlx =0.9.1 con sqlite, postgres, migrate, tls-rustls-ring-webpki; tokio, serde,
toml, axum, tonic: 218 crate di registry), più il codice sorgente scaricato di `ring` e `aws-lc-sys`.

## Verdetto

I fatti su sqlx e sulla vulnerabilità sono corretti; il difetto vero sta nella lista delle licenze,
che non nomina `Apache-2.0 WITH LLVM-exception` (wasmtime) e `bzip2-1.0.6` (DataFusion di default) e
non dice nulla su tre licenze npm comuni.

## Fatti confermati

- sqlx 0.9.1 esiste: crates.io `created_at` 2026-11-18T01:37Z (UTC), changelog datato 2026-11-17;
  `rust-version` 1.94.0; non ritirato. https://crates.io/api/v1/crates/sqlx/0.9.1 e
  https://github.com/transact-rs/sqlx/blob/main/CHANGELOG.md
- `sqlx-sqlite 0.9.1` richiede `libsqlite3-sys >=0.30.1, <0.41.0`
  (https://crates.io/api/v1/crates/sqlx-sqlite/0.9.1/dependencies). Il changelog cita #4502 (tetto
  0.39) e #4525 (tetto 0.40).
- RUSTSEC-2026-0459 / CVE-2026-119375 / GHSA-72w2-rjgg-q729: crate `libsqlite3-sys` (non sqlx),
  emesso 2026-11-09, CVSS 4.0 6.8 medio, vettore locale. Corretto in `>=0.39.0` (SQLite 3.54.0;
  0.39.1 = 3.54.1). https://rustsec.org/advisories/RUSTSEC-2026-0459.html
- Verificato localmente: un albero nuovo con sqlx 0.9.1 risolve `libsqlite3-sys 0.40.0` con
  `SQLITE_VERSION "3.54.1"`.
- sqlx 0.9 supporta ancora le query a runtime (le funzioni `query*()` senza macro).
- Licenze presenti nell'elenco e verificate: `webpki-roots` CDLA-Permissive-2.0, `rustls-webpki` ISC,
  `ring 0.17.14` `Apache-2.0 AND ISC`, `foldhash` Zlib, icu e unicode-ident Unicode-3.0, `ryu`/`whoami`
  BSL-1.0 (come alternativa), `tiny-keccak`/`notify` CC0-1.0, `option-ext` MPL-2.0.
- Nessun problema OpenSSL: `aws-lc-sys 0.47.0` dichiara `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND
  MIT AND BSD-3-Clause AND ...` e il suo LICENSE dice che il codice derivato da OpenSSL è sotto
  Apache-2.0; `ring` 0.17.14 è `Apache-2.0 AND ISC`. Tutte le 218 crate del mio albero di prova
  (tokio, rustls, sqlx e transitive) sono soddisfatte dall'elenco, a patto di valutare le espressioni
  `OR` (casi: `Unlicense OR MIT`, `MIT OR Apache-2.0 OR LGPL-2.1-or-later` in `r-efi`).

## Risultati

### 1. [Alta] `Apache-2.0 WITH LLVM-exception` non è nell'elenco (spike Sandbox)

`wasmtime 51.0.0`, `wasmtime-wasi` e `cranelift-codegen` hanno licenza `Apache-2.0 WITH LLVM-exception`
senza alternative (https://crates.io/api/v1/crates/wasmtime/51.0.0). Il linguaggio degli SPDX tratta
l'eccezione come identificatore distinto; `cargo-deny` richiede di ammetterla in modo esplicito. Con
l'elenco letto alla lettera, il primo candidato per lo spike Sandbox (e `wasmparser`, `wit-bindgen`,
in forma `OR`) farebbe fallire il controllo.
Correzione: aggiungere "Apache-2.0 WITH LLVM-exception" tra le licenze ammesse (è più permissiva
di Apache-2.0 semplice, nessun obbligo in più).

### 2. [Alta] `bzip2-1.0.6` tramite DataFusion (spike Stream pipeline)

`datafusion 55.3.0` ha `compression` nelle feature di default, che attiva `bzip2 ^0.6.1`, che di
default usa `libbz2-rs-sys 0.2.7`, licenza `bzip2-1.0.6`
(https://crates.io/api/v1/crates/libbz2-rs-sys). Non è nell'elenco: la prima dipendenza dello spike 3
bloccherebbe il pre-push.
Correzione: o ammettere `bzip2-1.0.6` (licenza permissiva in stile BSD, nessun copyleft), oppure
l'ADR dello spike usa `default-features = false` e sceglie le sole feature di compressione che servono.
Raccomando la seconda (meno superficie) più la prima come rete di sicurezza in un'eccezione per ADR.

### 3. [Media] Lato npm l'elenco non copre licenze comuni

Verificato con `npm view`: `caniuse-lite` è CC-BY-4.0; `argparse` è PSF-2.0 (Python-2.0 in
versioni più vecchie); `glob`, `minimatch`, `jackspeak` sono BlueOak-1.0.0; `lightningcss` (dipendenza
diretta di vite 8) è MPL-2.0 (ammessa). `elkjs` risulta `EPL-2.0 OR GPL-3.0-or-later`, coerente con
l'esclusione. Gli strumenti di build/test (Storybook, Playwright, Vite) portano di solito queste
licenze negli alberi `devDependencies`; non so quali finiscano davvero nel grafo di `packages/` (non
l'ho risolto), ma BlueOak e CC-BY-4.0 sono le più probabili.
Correzione: definire l'ambito. Il controllo vale per ciò che viene distribuito (dipendenze di
produzione del kit e del motore, e per il dashboard ciò che finisce nel bundle); gli strumenti di
sviluppo non producono obblighi per chi incorpora il kit. In più ammettere BlueOak-1.0.0 (permissiva),
e decidere caso per caso CC-BY-4.0 (richiede attribuzione, dati non codice) e PSF-2.0.

### 4. [Media] sqlx da solo non garantisce `libsqlite3-sys >= 0.39`

Il minimo di sqlx è 0.30.1: un lockfile esistente o un altro crate che fissa una versione vecchia
tiene un SQLite vulnerabile senza che sqlx lo impedisca. Oggi un albero nuovo risolve 0.40.0, ma
non per costruzione. Inoltre 0.40.0 (2026-11-11) ha 12 giorni, meno delle due settimane richieste
dal CLAUDE.md; 0.39.1 (2026-10-25) le soddisfa.
Correzione: nell'ADR scrivere che il lockfile fissa `libsqlite3-sys` a 0.39.1 (`cargo update -p
libsqlite3-sys --precise 0.39.1`) oppure a 0.40.0 sotto la stessa eccezione di sicurezza, e che il
pre-push esegue `cargo deny check advisories` (`cargo-deny 0.22.0` esiste, `MIT OR Apache-2.0`), che
avrebbe catturato RUSTSEC-2026-0459.

### 5. [Media] L'ADR sul driver non nomina la regola `SqlSafeStr` di sqlx 0.9

Dal changelog 0.9.0 (#3723): tutte le `query*()` accettano solo `&'static str` o
`AssertSqlSafe(...)`. "Query a runtime" è quindi vero solo per testi statici; l'adattatore che
sceglie il dialetto per database con `format!()` deve avvolgere in `AssertSqlSafe`, che è proprio
lo "speed bump" voluto da sqlx. Conviene dirlo per il test a due tenant (AD-5): il filtro del tenant
non deve mai essere interpolato.
Correzione: aggiungere all'ADR "query statiche per dialetto; `AssertSqlSafe` solo nell'adattatore,
mai con valori del chiamante; il filtro del tenant è sempre un parametro legato (`$1`/`?`)".

### 6. [Bassa] Dettagli di citazione e data

- Data di sqlx 0.9.1: l'ADR scrive 2026-11-18 (UTC di crates.io), il changelog 2026-11-17. Scrivere
  "2026-11-17/18".
- L'ADR cita l'issue sqlx #4516, non l'avviso. CLAUDE.md pretende l'avviso citato nel commit:
  aggiungere https://rustsec.org/advisories/RUSTSEC-2026-0459.html.
- Il crate vulnerabile è `libsqlite3-sys`, non sqlx; l'esposizione richiede di aprire un DB più `-wal`
  scritti da una parte meno fidata. L'eccezione resta legittima (avviso pubblicato), ma il testo
  "SQLite vulnerabile" può dire "per un DB di cui non ci si fida".
- Feature `bundled-sqlcipher` di libsqlite3-sys 0.39.x è ancora vulnerabile (SQLite 3.53.4): da non
  adottare se un domani servisse la cifratura.

### 7. [Bassa] Formato della configurazione

Nessun fatto web da correggere. Nota tecnica: `TOPICTOAST_<SEZIONE>__<CHIAVE>` funziona con
chiavi che contengono `_` (snake_case) solo perché il separatore è il doppio underscore; l'ADR lo
dice già. Il test di avvio dovrebbe coprire una chiave con underscore singolo (`max_body_bytes`) e
l'errore C-7 su una chiave sconosciuta nei due canali (file e variabile).

### 8. [Bassa] Verificabilità di "MPL-2.0 per pacchetti non modificati"

Nessuno strumento controlla che il pacchetto non sia modificato. Correzione: scriverlo come regola
sulle patch (`[patch]`, `vendor/`, fork) invece che come proprietà verificata dal controllo.

## Proposta di testo per le licenze (sintesi)

Ammesse: le dodici attuali più `Apache-2.0 WITH LLVM-exception`, `BlueOak-1.0.0` e, se serve a
DataFusion, `bzip2-1.0.6`; `Unicode-DFS-2016` come sinonimo storico di Unicode-3.0 per crate non
aggiornate. Il controllo valuta le espressioni SPDX (`OR` basta una ammessa, `AND` tutte). Ambito:
dipendenze distribuite; gli strumenti di sviluppo sono fuori.
