# ADR: archivio dei payload su disco o RustFS, storico in Parquet

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-17 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-11, FR-22, FR-27, FR-31, FR-50, §4.11, §8.2, §11 n.4; addendum del PRD; addendum
  del brief (§2, §8); [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md),
  voci 28, 43, 44, 45, 49, 50, 56, 59; `review-decisions.md` (E-1, E-3, E-4); revisioni `reconcile-prd.md`,
  `reconcile-brief-rules.md` (R5, R20), `review-adversarial.md` (R19)

## Contesto

Tre cose hanno bisogno di un posto fuori dal database:

- i payload dei task per lo storico (AD-3, memlog voce 28);
- i checkpoint delle pipeline stream (FR-22). L'addendum del PRD chiedeva dove vanno nella v1,
  visto che RustFS era facoltativo e arrivava dopo (PRD §11 n.4);
- lo storico chiuso, per un'eventuale analisi.

Altri vincoli:

- FR-50 ammette due servizi obbligatori, topictoast e il bus, più il database.
- Nel brief RustFS era un vincolo di Marco per i messaggi grandi e i file (addendum del brief,
  §8).
- Il PRD lo metteva fuori dalla v1 (§8.2); in questa PR il PRD ha la nota che lo porta dentro
  come adattatore facoltativo.

## Decisione

Registra [AD-17](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
(memlog, voci 43 e 44).

- Payload dei task, checkpoint delle pipeline stream ed esportazioni dello storico passano da una
  sola porta, l'archivio dei payload.
- La porta ha fin dalla prima storia due spazi sotto l'identificativo interno del tenant (E-1):
  uno immutabile indirizzato dall'impronta, dove il proiettore scrive ogni payload dei task quando
  lo applica (AD-3, E-4), e uno con nome sovrascrivibile (checkpoint, esportazioni). Gli oggetti
  dell'archivio sono cifrati nel formato unico di C-12 (AD-7, E-3).
- Adattatore predefinito: file su disco locale. Adattatore facoltativo: RustFS (S3, Apache-2.0,
  versione 1.0.4 del 2026-10-25). I servizi obbligatori restano due.
- Lo storico chiuso si esporta in Parquet nell'archivio, cifrato per tenant (AD-7), e si
  interroga con DataFusion, già nel binario (AD-13). Nessuna esportazione in chiaro, nemmeno
  provvisoria. I Parquet seguono la conservazione e le cancellazioni di FR-31 e FR-27.
- DuckDB non entra finché non serve qualcosa che DataFusion non copre.
- Il PRD non ha una funzione di analisi: la spine fissa solo formato e motore.

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:**
  - una porta sola per payload, checkpoint ed esportazioni, con i due spazi che servono a tutti e
    tre fin dalla prima storia;
  - le uscite a monte si leggono dallo spazio immutabile anche dopo che il bus ha scartato le
    posizioni applicate (E-4);
  - funziona senza un terzo servizio, su disco;
  - RustFS per chi vuole S3;
  - storico interrogabile con il motore SQL già presente;
  - chiude PRD §11 n.4: i checkpoint stanno sulla porta, su disco per impostazione predefinita.
- **Costi:**
  - un adattatore in più da provare (RustFS), con entrambi gli spazi;
  - il proiettore scrive nello spazio immutabile ogni payload appena applicato, non solo a
    esecuzione chiusa (E-4);
  - l'esportazione in Parquet, quando arriva con la funzione di analisi, è una seconda copia dello
    storico che deve seguire conservazione e cancellazioni, e la sua cifratura per tenant
    (`parquet_encryption` di datafusion 55.2.0) va provata allora;
  - salvataggio e pulizia dell'archivio insieme al database sono aperti.
- **Facilità di manutenzione:** una porta con due adattatori. RustFS passa dal modulo del client
  HTTP del motore (AD-16). Nessun secondo motore SQL.

## Alternative scartate

- **RustFS obbligatorio:** cambia FR-50 e NFR-1, e c'è più da installare (memlog, voce 50). Il
  Prevents di AD-17 lo esclude, e il memlog lo fissa: «Restano due servizi obbligatori» (voce 43).
- **RustFS dopo la v1**, la posizione iniziale del PRD (§8.2): nessun contro tecnico; Marco
  preferisce averlo facoltativo già nella v1 (memlog, voci 43 e 50).
- **DuckDB per lo storico** (memlog, voce 44): DataFusion è già nel binario. DuckDB entra solo se
  serve qualcosa che DataFusion non copre. Il Prevents di AD-17 parla di un secondo motore SQL per
  l'analisi.
- **Porta con il solo spazio indirizzato dall'impronta** (`review-adversarial.md` R19; nessuna
  alternativa registrata nel memlog): i checkpoint si sovrascrivono e si elencano, le esportazioni
  sono per periodo, quindi la seconda storia dovrebbe cambiare la porta e i due adattatori.

## Conseguenze e criteri di revisione

- **S-1** aggiorna AD-17. Con Valkey, lo scarico porta i payload nell'archivio dei payload. S-1
  misura anche il costo della cifratura nel formato di C-12 (E-3).
- **S-3** mette i checkpoint nello spazio sovrascrivibile.
- **Esportazione in Parquet:** Marco ha scelto di costruirla insieme alla funzione di analisi dello
  storico, quando il PRD la definisce (memlog). Allora si prova la cifratura per tenant dei
  Parquet; se non regge, AD-17 si riapre.
- **Open Questions della spine:**
  - salvataggio e pulizia dell'archivio insieme al database, prima della storia che scrive il
    primo file;
- Il limite di 1 MB di FR-11 non cambia. Lo storage per payload grandi e file resta dopo la v1
  (PRD §8.2).
- La funzione di analisi dello storico entra quando il PRD la definisce; se richiede ciò che
  DataFusion non copre, DuckDB si valuta con un ADR nuovo.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63), applicato sopra:
  - E-4 nello spazio immutabile: il proiettore vi scrive ogni payload quando lo applica, non solo
    per lo storico, e le uscite a monte si leggono da lì;
  - E-3 nella cifratura degli oggetti dell'archivio: busta versionata con l'id della chiave,
    chiave per oggetto derivata con HKDF (C-12);
  - E-1 nei due spazi, che stanno sotto l'identificativo interno del tenant.

  Resta da valutare nelle Open Questions della spine l'indirizzo dei payload come HMAC o impronta
  del cifrato (rubrica, AD-3).
