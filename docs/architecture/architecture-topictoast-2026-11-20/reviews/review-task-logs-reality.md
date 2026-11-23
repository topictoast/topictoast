# Revisione di realtà: log dei task (FR-59), diff su AD-17, S-1, C-9, T-67, ADR 2026-11-23

Lente: ogni affermazione portante è verificata o solo asserita? Data: 2026-11-23.

## Verdetto

La struttura regge, perché "pezzi successivi mai sovrascritti" non dipende da nessuna capacità fragile dell'archivio. Restano però alcune affermazioni asserite e non misurate, e la citazione di C-12 è ambigua sui nonce.

## Cosa è verificato

- **Append su S3/RustFS.** Il documento NON si appoggia all'append: sceglie oggetti separati, uno per pezzo. È la scelta giusta. S3 classico non ha append. L'append nativo esiste solo su S3 Express One Zone, o in modalità Express di MinIO AIStor, tramite PutObject con offset. La documentazione RustFS non nomina AppendObject e dichiara solo "un sottoinsieme testato" dell'API S3 ([RustFS S3 compatibility](https://docs.rustfs.com/features/s3-compatibility)). Quindi non va mai assunto. Nessun difetto nel diff.
- **Elenco per prefisso su RustFS.** La pagina di compatibilità marca come testati prefisso, delimitatore, marker e max-keys per ListObjects/V2 ([fonte](https://docs.rustfs.com/features/s3-compatibility)). Non dice nulla sull'ordine delle chiavi.
- **Mascheramento a valore esatto.** Il limite è noto e documentato. GitHub Actions maschera il valore esatto più alcune codifiche comuni come Base64, e dichiara che non è garantito ([GitHub, security hardening](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions)). JSON/XML, valori spezzati o derivati sfuggono ([safeguard.sh](https://safeguard.sh/resources/blog/ci-secret-masking-matches-the-exact-string-only); [CloudBees](https://docs.cloudbees.com/docs/cloudbees-ci/latest/secure/cloudbees-mask-secrets), che non maschera i segreti dentro output Base64). T-67 elenca già forma del segnaposto, segreti corti e valore spezzato su più righe.

## Finding

1. **Ordine e completezza dei pezzi non specificati (media, portante).** Il documento dice "pezzi successivi" ma non dice come si nominano né come si leggono in ordine. Con S3 l'unico indice è l'elenco per prefisso, e la pagina RustFS non garantisce nulla sull'ordine. Serve una chiave con numero di sequenza a larghezza fissa e una regola per i buchi dopo un crash. Prova da aggiungere a S-1: elenco e rilettura in ordine su RustFS reale, con un pezzo mancante.

2. **Nonce e derivazione per pezzo non dichiarati (media, portante).** C-12 deriva con HKDF la chiave "per ogni oggetto o segmento" e dice che così si evita il limite dei nonce. Per i log un pezzo è un oggetto, quindi la derivazione è per pezzo. Il diff non dice quale `info` HKDF si usa (tenant, esecuzione, tentativo, numero del pezzo) né dove sta il nonce nella busta. Se due pezzi condividono la stessa chiave derivata e un nonce prevedibile, si riusa la coppia (chiave, nonce) e AES-GCM si rompe. Va scritto nella spec, o come criterio di S-1, che l'`info` include il numero del pezzo. Garanzia asserita, non provata.

3. **Il ricontrollo del motore non cattura valori trasformati (media, portante per la promessa di FR-59).** L'ADR presenta la doppia applicazione (origine + motore) come difesa in profondità. Entrambe condividono il limite noto dei sistemi analoghi: Base64, URL-encoding, valore spezzato su due righe, JSON serializzato. Il motore non ripara un SDK di terzi che stampa il segreto codificato. Va dichiarato nell'ADR come limite accettato. Il costo della ricerca di ogni segreto in ogni riga (righe × segreti) va misurato, come l'ADR già dice per NFR-2.

4. **"Un crash perde al massimo l'ultimo pezzo" dipende dalla dimensione del pezzo (bassa-media).** L'ADR ammette che la dimensione è da fissare. Manca inoltre da dove legge la dashboard le righe del pezzo ancora aperto: l'alternativa scartata "oggetto unico" viene bocciata perché "il tempo reale dovrebbe leggerle da un'altra parte", ma anche la scelta adottata non dice da dove le legge. Lacuna, non errore.

5. **Spazio sovrascrivibile per dati write-once (bassa).** "Mai sovrascritto" si regge solo per convenzione. Con S3 lo si impone con `If-None-Match: *` su PutObject; il supporto di RustFS non è verificato qui. Sul disco locale serve apertura esclusiva. Una riga nella spec basta.

## Non verificato (da misurare in S-1 o nella spec)

- Supporto di RustFS a PutObject condizionale (`If-None-Match`).
- Ordine di ListObjectsV2 su RustFS con molti pezzi per tentativo.
- Costo del ricontrollo dei segreti per riga e di HKDF per pezzo dentro NFR-2.
