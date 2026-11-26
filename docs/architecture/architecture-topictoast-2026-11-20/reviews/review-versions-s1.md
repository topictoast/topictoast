# Revisione delle versioni e delle tecnologie dell'Update S-1

Data della verifica: 2026-11-26. Fonti interrogate con User-Agent generico: crates.io (`/api/v1/crates/<nome>/versions`), GitHub releases e security-advisories di apache/iggy, rustfs/rustfs, rustls/rustls, API OSV.

## Verdetto

Le versioni fissate sono reali, non ritirate e rispettano la regola delle due settimane; nessuna decisione tecnologica è contraddetta dai risultati dello spike. Restano da correggere una versione di RustFS non aggiornata all'ultima ammissibile e alcune affermazioni verificate con prove più deboli di quanto il testo lasci intendere (nessuna critica, nessuna alta).

## Verifiche riuscite

| Voce | Esito | Evidenza |
| --- | --- | --- |
| Apache Iggy `server-0.10.0` | pubblicata 2026-11-07 (19 giorni prima del 2026-11-26), ultima release stabile, non pre-release | GitHub releases di apache/iggy |
| crate `iggy` 0.12.0 | pubblicato 2026-11-07, non ritirato (yanked=false); le versioni successive sono solo `0.12.1-edge.*` (2026-11-16 e 2026-11-23) | crates.io |
| `tonic` 0.14.9 | pubblicato 2026-10-29 (28 giorni), non ritirato, ultima della serie 0.14 | crates.io |
| `rustls` 0.23.46 (Stack) | pubblicata 2026-10-14; corregge tre avvisi GHSA del 2026-10-14 (`GHSA-c8m5-c952-6cm4`, `-h2v8-394m-p7vv`, `-w44r-93j7-qv28`), quindi è la scelta giusta | advisories rustls/rustls |
| Avvisi su `iggy` e `tonic` 0.14.9 | nessuno su OSV; l'unico avviso di `tonic` (RUSTSEC-2024-0376) riguarda versioni vecchie | OSV |
| `prost` 0.14.4, `aws-lc-rs` 1.19.0, `aws-lc-sys` 0.47.0, `ring` 0.17.14 (dal `Cargo.lock` dello spike) | nessun avviso su OSV | OSV |
| RustFS, `If-None-Match: *` con 412 | misurato, non asserito: primo PUT 200, secondo 412 `PreconditionFailed`, PUT di controllo senza intestazione 200 che sovrascrive | `spikes/bus/fase4-risultati.md:1321` |
| Iggy: posizione del consumatore e `delete_segments` | misurato: 0 violazioni in 3+3 corse, controllo negativo senza soglia riuscito | `fase4-risultati.md:150-182` |
| Iggy: `durability = Persisted` | usato in tutte le serie; effetto sul costo misurato (2,6 ms contro 0,04 ms con `Replicated`) | `fase2-risultati.md:70-83` |
| Iggy: permessi per numero di topic, nessun modello di nome | misurato (41 `Unauthorized` su B) e letto nel codice (`BTreeMap<id, TopicPermissions>`) | `fase4-risultati.md:1537-1543` |
| Client 0.11.0 per server 0.9.0, 0.12.0 per 0.10.0 | confrontato su registro e sorgenti, matrice di compatibilità | `fase4-risultati.md:204-206` |

## Rilievi

### 1. RustFS fissato a 1.0.4, esiste 1.0.5 ammissibile (media)

- **Dove:** Stack della spine, riga `RustFS (adattatore facoltativo, AD-17) | 1.0.4`.
- **Evidenza:** rustfs/rustfs ha 1.0.5 (2026-11-10, 16 giorni), 1.0.6 (2026-11-15, 11 giorni, non ancora ammissibile) e 1.0.7 (2026-11-26). Le 1.0.4 e 1.0.5 sono oltre la 1.0.2 che corregge gli avvisi del 2026-10-18 (tra cui `GHSA-rhg6-jrf2-29c4`, severità alta), quindi 1.0.4 non è vulnerabile a quelli. La regola del progetto parla di «ultima versione con almeno due settimane»: oggi sarebbe 1.0.5. Lo spike ha provato solo 1.0.4, quindi cambiare versione richiede di ripetere i due PUT.
- **Rimedio:** o si dichiara che 1.0.4 è la versione provata (aggiungendo «provata nello spike; 1.0.5 ammissibile, da riprovare prima dello sprint che usa RustFS») oppure si ripetono i due PUT su 1.0.5 e si aggiorna la riga. Su GitHub risulta anche l'issue rustfs/rustfs#9511 («1.0.5 fails to start after rebuilding a failed node in a 4-node distributed cluster»): riguarda il cluster distribuito, non il nodo singolo, ma va letta prima di passare a 1.0.5.

### 2. Il 412 di RustFS è provato su un solo nodo e un solo scenario (media)

- **Dove:** punto 10 dell'ADR e riga 406 della spine («creazione solo se assente su RustFS con `If-None-Match: *`»).
- **Evidenza:** `fase4-risultati.md:1321`: due corse, due PUT in sequenza, nodo singolo su disco. Nessuna prova di due PUT concorrenti sulla stessa chiave né dopo un riavvio. Il blocco D aveva lasciato il test fuori (`fase4-risultati.md:878`, `:1039`) e lo ha recuperato dopo; l'ADR non dice che la concorrenza non è provata. RustFS ha inoltre una pull request aperta sul supporto delle chiavi di condizione `s3:if-none-match` nelle policy (rustfs/rustfs#8614, #8615): il comportamento dell'intestazione sul PUT funziona, ma la parità con S3 non è completa.
- **Rimedio:** aggiungere ai «Punti aperti» dell'ADR «`If-None-Match: *` su RustFS non provato con due PUT concorrenti sulla stessa chiave», oppure farlo (due `curl` in parallelo bastano).

### 3. `Persisted` provato per costruzione, non con un'interruzione dell'alimentazione (media)

- **Dove:** punto 7 dell'ADR («lo spegnimento improvviso dell'host non è misurabile … lo argomentano la risposta solo dopo l'fsync di Iggy (`Persisted`)»).
- **Evidenza:** `fase2-risultati.md:319` dice «La durabilità è verificata solo per costruzione (Iggy `Persisted`, confronto con `Replicated`…)». L'ADR lo dichiara già come argomentato, quindi è coerente; ma il criterio NFR-6 nella tabella dice «rispettato (spegnimento dell'host argomentato, non misurato)» e il bilancio dice «zero perdite in ogni guasto provato»: è corretto, ma il lettore della spine non vede il limite.
- **Rimedio:** nella spine, accanto ad AD-2, ripetere in una riga che la durabilità all'interruzione dell'alimentazione non è provata né per Iggy né per SQLite `synchronous=FULL`.

### 4. Valore effettivo del group commit non riletto dal server (bassa)

- **Dove:** punto 1 dell'ADR (`wal_group_commit_delay_micros = 500`).
- **Evidenza:** `compose.yaml:18` imposta 500 come predefinito (`${IGGY_GC_DELAY:-500}`); `fase4-risultati.md:296-297` dichiara che per le corse del blocco A il valore (nel caso della variante `bench-gc5000`, 5000) è stato letto dall'ambiente del container e «il valore effettivo nel server non è stato riletto». Per le serie H2 il valore 500 non è mai stato confermato dal server.
- **Rimedio:** nella documentazione del deploy, far stampare al motore all'avvio la configurazione effettiva letta da Iggy, oppure annotare nell'ADR che il valore è quello passato, non riletto.

### 5. `tonic` e `prost` non compaiono nello Stack con versione fissata (bassa)

- **Dove:** Stack della spine; riga delle «Versioni delle librerie non ancora scelte» (riga 367 del diff) cita «tonic; S-1 ha misurato 0.14.9».
- **Evidenza:** lo spike usa `tonic =0.14.9`, `tonic-prost =0.14.9`, `prost =0.14.4` (`spikes/bus/Cargo.toml`). `prost` 0.14.5 (2026-11-10, 16 giorni) e 0.14.6 (2026-11-12, 14 giorni) esistono e sono ammissibili, ma non c'è motivo di aggiornarle: la riga della spine non le fissa, quindi non c'è contraddizione. Il rischio è solo che la prima storia fissi `tonic` 0.14.9 con `prost` diverso da quello misurato.
- **Rimedio:** nella riga, scrivere «`tonic` 0.14.9, `tonic-prost` 0.14.9, `prost` 0.14.4, `rustls` 0.23.46 con `tls-ring` misurati; la prima storia riparte da questi».

### 6. Dipendenze rimaste nello spike ma tolte dalla spine (bassa)

- **Dove:** `spikes/bus/Cargo.toml` (`redis =1.8.1`, `iroh =1.4.1`); ADR, «Stack: … Valkey, `redis` e iroh tolti».
- **Evidenza:** coerente, lo spike è fuori dal gate. Il commento in `Cargo.toml` dice «iroh 1.4.1: ultima versione con almeno due settimane», mentre il vecchio testo della spine citava `iroh` 1.4.2 «solo con l'eccezione di sicurezza»: nessun problema, iroh è escluso.
- **Rimedio:** nessuno obbligatorio.

## Nessuna contraddizione con lo spike

Ho controllato per ciascuna decisione tecnologica dell'Update se i file `spikes/bus/fase*-risultati.md` dicono il contrario: non ho trovato contraddizioni sulle versioni, su `delete_segments`, sull'offset del consumatore, sui permessi per numero di topic né sul 412 di RustFS. Le limitazioni di prova (rilievi 2, 3, 4) sono già in gran parte dichiarate nell'ADR.
