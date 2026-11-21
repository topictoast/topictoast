# ADR: pipeline stream su DataFusion, un processo ausiliario per tenant

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-13 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-18…FR-25, NFR-1, NFR-5; addendum del PRD («Modello dei worker e dello stream»);
  `extract-prd.md` (M-31); `research-versions.md`;
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 10, 13, 14, 15,
  17, 18, 21, 23, 37, 48, 50, 51, 56; `review-decisions.md` (E-1, E-3); revisioni
  `reconcile-prd.md`, `reconcile-brief-rules.md` (R3), `review-rubric.md`

## Contesto

Le pipeline stream sono query SQL continue con stato, finestre, watermark e checkpoint, scritte
anche dai tenant e soggette a quote (FR-18…FR-25). L'obiettivo di NFR-5 è ≥ 100.000 eventi/s, ma
è un'[ASSUMPTION] da confermare dopo lo spike.

L'addendum del PRD parla di «modello Arroyo» e cita DataFusion come riferimento di Marco.
DataFusion da solo non ha finestre, watermark né checkpoint. L'addendum e `extract-prd.md` (M-31)
elencavano tre strade: riuso di Arroyo, DataFusion con lo stato costruito in topictoast, altro.

Marco ha poi chiarito che Arroyo era solo un esempio del modello e non un software da portarsi
(memlog, voce 13). Il memlog prevale: Arroyo non è mai stato un candidato.

## Decisione

Registra [AD-13](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
(memlog, voce 14).

- Il SQL delle pipeline stream gira su DataFusion, versione 55.2.0 (voce 23).
- Stato, finestre, watermark e checkpoint li costruisce topictoast.
- DBSP è escluso.
- Lo spike S-3 misura il costo di quelle parti sopra DataFusion; non sceglie il motore.
- Le pipeline stream di un tenant girano in un processo ausiliario dedicato a quel tenant, attivo
  solo se il tenant ha pipeline attive (AD-6). Il motore gli passa solo la chiave di quel tenant,
  mai la chiave principale.
- Gli eventi stream di ogni tenant stanno su topic propri del tenant, nominati con il suo
  identificativo interno e mai con un nome del JWT (AD-5, E-1); il processo legge direttamente
  solo quelli, qualunque collegamento scelgano i worker (voce 37).

Regole di altri AD che toccano il processo:

- eventi e checkpoint sono cifrati con la chiave del tenant nel formato unico di C-12: busta
  versionata con l'id della chiave, chiave per oggetto o segmento derivata con HKDF (AD-7, E-3);
- le righe e gli avvii che scrive passano dall'IPC del motore con un id deterministico (C-4,
  AD-18);
- i checkpoint stanno nello spazio con nome sovrascrivibile dell'archivio dei payload (AD-17);
- nessuna sua istanza contiene dati di due tenant, né insieme né in sequenza (AD-6).

Caso dello spike (voce 18, decisione di Marco che corregge la voce 15 con le voci 17-18):
l'esempio di UJ-3 del PRD. Conteggio per ASN su finestre di 5 minuti, soglia oltre 50, un avvio
del workflow per riga, riavvio a metà finestra dall'ultimo checkpoint. È un'[ASSUMPTION] finché
non arriva un caso reale.

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:** SQL sugli stream con finestre, watermark e checkpoint. Un solo motore SQL,
  usato anche per lo storico (AD-17). Un processo per tenant: chiavi ed eventi di due tenant non
  stanno mai nello stesso processo (FR-25). Un solo avvio per riga anche dopo un riavvio dal
  checkpoint, perché gli avvii passano dal motore con un id deterministico.
- **Costi:**
  - finestre, watermark e checkpoint sono codice di topictoast, da costruire e mantenere;
  - DataFusion ha major frequenti;
  - NFR-1 va misurato con DataFusion nel binario, e la RAM di ogni processo per tenant pesa;
  - il SQL dei tenant non deve raggiungere tabelle esterne né funzioni di file o di rete di
    DataFusion: lo chiude il dialetto dei tenant (M-33), ancora da fissare in S-3, con un controllo
    sull'albero sintattico prima della pianificazione;
  - il buffering degli operatori di DataFusion 55.x aggiunge latenza, da misurare in S-3;
  - righe e avvii attraversano l'IPC del motore.
- **Facilità di manutenzione:** una libreria Apache-2.0 incorporata, nessun servizio in più. Le
  major frequenti chiedono aggiornamenti regolari con la regola delle due settimane.

## Alternative scartate

- **DBSP (Feldera), come libreria** (memlog, voci 13 e 14): candidato accanto a DataFusion,
  poi escluso da Marco. Motivo (memlog, voce 50): una nuova 0.x quasi ogni giorno, quindi
  instabile, e Marco aveva già indicato DataFusion. Dettagli da `research-versions.md`:
  - sette minor dal 2026-11-07;
  - il repository ha un'edizione open-source MIT più parti «Enterprise» sotto un'altra licenza;
    il crate dichiara MIT OR Apache-2.0.
- **Arroyo: mai un candidato.** Era solo l'esempio del modello (query SQL continue con stato,
  finestre, watermark, checkpoint), non un software da portarsi (memlog, voce 13). Per
  completezza, da `research-versions.md`:
  - il motore non è su crates.io: si distribuisce come binario singolo o Docker;
  - l'uso come libreria non è documentato;
  - usa un fork di DataFusion.

## Conseguenze e criteri di revisione

- **S-3** misura sul caso di UJ-3:
  - NFR-5 dall'API di FR-18 (a blocchi), con scrittura durevole, cifratura per tenant e guasto
    iniettato senza eventi accettati persi;
  - FR-19…FR-25;
  - riavvio dal checkpoint senza perdere eventi accettati, e un solo avvio per riga (C-4);
  - nessun accesso del SQL dei tenant a tabelle esterne, file o rete;
  - NFR-1 con DataFusion nel binario e nessuna pipeline attiva, e la RAM di ogni processo per
    tenant;
  - la latenza aggiunta dal buffering degli operatori di DataFusion 55.x;
  - un secondo tenant, con un proprio processo e una query ostile, prova FR-25.

  Da fissare e provare nell'ADR: SQL controllato sull'albero sintattico contro il dialetto chiuso
  prima della pianificazione; niente DDL, DML né istruzioni; nessun catalogo di file né archivio
  di oggetti registrato; versamento su disco spento, o in una cartella del tenant cifrata; limite
  di memoria imposto dal sistema al processo, oltre al pool di DataFusion, anche per la
  validazione a secco. Casi ostili: `repeat` con un numero enorme, prodotto cartesiano, costanti
  enormi.

  L'ADR delle pipeline stream aggiorna AD-13.
- **S-1** fissa per il processo delle pipeline stream una credenziale del bus per tenant, in sola
  lettura sui topic del tenant (`review-rubric.md`). Se il bus non la sa imporre, il motore
  alimenta il processo, e la lettura diretta di AD-13 cambia.
- Lo spike non riapre la scelta del motore. Se i criteri non reggono sopra DataFusion, la scelta
  torna a Marco con un ADR nuovo.
- La cifratura per tenant dei Parquet dello storico non è in S-3: l'esportazione in Parquet si
  costruisce con la funzione di analisi dello storico (scelta di Marco, memlog), e la prova si fa
  allora.
- L'[ASSUMPTION] su UJ-3 e su NFR-5 si rivede quando arriva un caso reale.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63), applicato sopra:
  - E-3 nella cifratura che il processo usa su eventi e checkpoint: un solo formato nel dominio,
    chiave per oggetto o segmento derivata con HKDF (C-12);
  - E-1 nei nomi dei topic propri del tenant: identificativo interno, mai un nome del JWT.
