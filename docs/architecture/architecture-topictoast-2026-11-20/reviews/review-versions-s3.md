# Revisione versioni e fonti — aggiornamento della spine dopo S-3

Verdetto: le decisioni sulle versioni reggono alla verifica indipendente (nessuna critica o alta); restano due note di tracciabilità (media e bassa).

Data del controllo: 2026-11-28. Fonti: API di crates.io (User-Agent `spike-dependency-check`), API di OSV, `spikes/stream/Cargo.toml`, `spikes/stream/Cargo.lock`, `spikes/stream/risultati/stream-osv-2c.txt`, `docs/decisions/2026-11-26-bus.md`.

## Esiti verificati

1. datafusion 55.2.0 esiste, non è ritirata, licenza Apache-2.0. Pubblicata il 2026-10-07T23:12:24Z (https://crates.io/api/v1/crates/datafusion/55.2.0), cioè 52 giorni prima del 2026-11-28: regola delle due settimane rispettata.
2. Le feature `sql`, `datetime_expressions`, `string_expressions`, `unicode_expressions`, `recursive_protection` esistono tutte nella 55.2.0 (campo `features` della stessa risposta). `Cargo.toml` riga 12 le usa con `=55.2.0` e `default-features = false`; `Cargo.lock` riga 1069-1070 conferma 55.2.0.
3. I tre avvisi esistono su OSV, tutti «unmaintained», `all versions affected`, nessuna versione corretta, pubblicati il 2026-09-14: RUSTSEC-2026-0543 (chrono), RUSTSEC-2026-0555 (chrono-tz), RUSTSEC-2026-0542 (humantime, perché avvolge chrono). Versioni nel lock: chrono 0.4.45 (riga 696), chrono-tz 0.10.4 (riga 710), humantime 2.4.0 (riga 2226): coincidono con la riga Stack della spine. `stream-osv-2c.txt`: 463 pacchetti interrogati, 0 falle.
4. Nessun avviso OSV su datafusion (query per pacchetto, e per versione 55.2.0: risposta vuota). Versioni più nuove: 55.3.0 (2026-11-17, 11 giorni prima, sotto la soglia) e 56.0.0 (2026-11-24). 55.2.0 è quindi la più recente che soddisfa la regola, e non c'è falla da correggere che faccia scattare l'eccezione di sicurezza.
5. La regola «4 gruppi in volo da al più 1.024 messaggi» in AD-13 coincide con `docs/decisions/2026-11-26-bus.md` riga 43.
6. L'affermazione «i codec lz4 e zstd di arrow-ipc entrano comunque» è coerente con il lock: `lz4_flex` (riga 2840), `zstd` (5071), `zstd-sys` (5089) presenti accanto ad `arrow-ipc` (283).

## Risultati

1. **Media — nessun controllo di 55.3.0 per correzioni di sicurezza o bug nel flusso a due settimane.** La soglia esclude 55.3.0 (11 giorni al 2026-11-28), ma la spine non dice che si è guardato. OSV non ha avvisi su datafusion, quindi l'eccezione di sicurezza non si applica; il changelog 55.3.0 non era leggibile all'URL provato (404). Evidenza: elenco versioni crates.io. Fix: aggiungere nell'ADR (o nella riga Stack) «55.3.0 del 2026-11-17 sotto soglia, nessun avviso OSV su datafusion al 2026-11-28», e rivalutare il pin dopo il 2026-12-01.
2. **Bassa — la riga Stack non riporta la data di pubblicazione né quella del controllo OSV.** Gli altri pin della tabella sono verificabili solo risalendo a `stream-osv-2c.txt` (controllo del 2026-11-27, un giorno prima della data della spine). Fix: aggiungere «pubblicata 2026-10-07; OSV 2026-11-27» alla riga datafusion, o rimandare al file dei risultati.
3. **Bassa — «senza falle né versione corretta» è esatto, ma il rischio è seguito senza un'azione di uscita.** I tre crate sono non mantenuti, non vulnerabili; l'uscita (jiff o time, indicate dall'avviso) dipende da DataFusion e non è nelle mani di topictoast. Fix: una riga «rivedere a ogni aggiornamento di DataFusion se chrono è sostituito», già implicita ma non scritta.
