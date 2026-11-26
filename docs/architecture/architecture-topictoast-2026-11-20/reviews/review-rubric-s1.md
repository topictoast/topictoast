# Revisione con rubrica: Update della spine dopo S-1

**Verdetto:** da correggere prima del merge. L'Update riporta bene la maggior parte della «Decisione» dell'ADR del bus e AD-2 è coerente al suo interno, ma restano un residuo letterale da rimuovere, tre decisioni dell'ADR che due storie potrebbero realizzare in modo incompatibile (credenziale in sola lettura delle pipeline stream, durabilità e autenticazione di Iggy, generazione del bus) e due Open Questions con scadenze troppo vaghe.

Letti: diff di `ARCHITECTURE-SPINE.md` e `conventions.md`, spine intera, `docs/decisions/2026-11-26-bus.md`.

## Controlli richiesti

1. **Decisioni dell'ADR atterrate:** parziale. Atterrate: bus Iggy (AD-1), gRPC con TLS (AD-12), consegna dopo il gruppo durevole, timer, annullamento, forma sul bus, corsia unica, ritardo, scrittore unico dei pezzi (AD-17), posizione (AD-18). Non atterrate o solo nell'ADR: punti 1 (durabilità e credenziali di Iggy), 4 (battiti, finestra di 2 s), 8 (meccanismo di conservazione), 9 (generazione nel topic `meta`, `E_BUS_GENERATION`), 12 (credenziale per tenant in sola lettura), 13 (utente del sentinella). Vedi i punti sotto.
2. **Residui su collegamento indeciso:** uno solo di rilievo (AD-13, riga 293). Le menzioni di iroh e Valkey rimaste (S-1 chiusa, riga 516; riga 848) sono storia, non opzioni vive.
3. **Coerenza di AD-2:** coerente (vedi sotto, con due lacune sul confine lento/fermo e sui battiti).
4. **Scadenze delle Open Questions con «S-1»:** tutte riscritte con una nuova scadenza. Due sono troppo vaghe (punti 6 e 7).

## Findings

### 1. [alta] AD-13, riga 293: residuo «qualunque collegamento scelgano i worker»
Il testo dice ancora che il processo delle pipeline stream legge «direttamente solo quelli, qualunque collegamento scelgano i worker». Il collegamento è deciso. Ancora più importante: la decisione 12 dell'ADR (credenziale Iggy per tenant con il solo permesso di lettura sul topic del tenant; i permessi di Iggy sono per numero di topic, quindi la creazione di un tenant deve aggiornarli; chi può scrivere sul topic di stato ferma il proiettore) non compare in nessuna AD. Due storie (tenant e pipeline stream) possono dare al processo `read_stream` e fargli leggere gli altri tenant.
**Fix:** in AD-13 sostituire la coda con «con una credenziale Iggy per tenant, in sola lettura sui soli topic del tenant (ADR del bus); la creazione e l'eliminazione del tenant (AD-5, C-13) aggiornano i permessi». Aggiungere ad AD-2 che sui topic di stato scrive solo la credenziale del motore.

### 2. [alta] Durabilità e autenticazione del bus non fissate in nessuna regola
AD-2 definisce «accettato» come «scrittura durevole sul log del bus», ma non dice cosa vuol dire per Iggy: `durability = Persisted`, `wal_group_commit_delay_micros = 500`, gruppi in volo (ADR punto 1). Una storia che configura il bus con un'altra durabilità rompe NFR-6 senza violare nessuna regola. Mancano anche la regola di AD-1/FR-12/FR-50 sull'autenticazione obbligatoria del bus anche da localhost, root con credenziali generate all'installazione, nessun valore predefinito, e un solo topic e una partizione per tenant già c'è ma non la sua configurazione.
**Fix:** in AD-1 (o AD-2, «Forma sul bus») aggiungere una riga: durabilità `Persisted` sui topic di stato, credenziali root generate all'installazione e mai quelle predefinite, valori di configurazione dell'ADR punto 1 come minimi.

### 3. [media] Generazione del bus e aggiornamento solo nell'involucro operativo
AD-18 usa la «generazione del bus» nel token, ma nessuna AD dice dove vive (topic `meta` e posizione applicata), che un motore con la generazione sbagliata non parte con un codice (`E_BUS_GENERATION`), né che l'aggiornamento è «drain, bus nuovo e vuoto, riversamento dal database». La procedura è solo in «Aggiornamenti» (riga 767) e nella riga di Open Questions sul formato. La storia dell'aggiornamento e quella dei token SSE possono decidere ciascuna la propria generazione.
**Fix:** in AD-2, nuova voce «Generazione del bus»: dove si scrive, chi la controlla all'avvio, che cambia a ogni aggiornamento del bus.

### 4. [media] Battiti e finestra dopo la ripresa non decisi
AD-12 dice che il worker manda i battiti; AD-2 (Timer) non dice se un battito è un evento durevole sul log né che sposta in avanti la scadenza (ADR punto 4: «i battiti spostano in avanti la scadenza»). L'ADR registra 100.000 eventi/s di battiti a 50.000 tentativi come non misurati. Anche la finestra di 2 s dopo la ripresa prima di dichiarare scaduto un tentativo (ipotesi non provata) non è né nella Rule né tra le Open Questions. Due storie (worker e ripresa) divergono sul costo in NFR-4 e sul comportamento dopo un crash.
**Fix:** decidere in AD-2 «Timer» se i battiti sono eventi sul log o stato in memoria del motore (e se il battito allunga la scadenza); aggiungere la finestra di ripresa come riga di Open Questions con scadenza (spec della storia dei timer), oppure fissarla.

### 5. [media] Codici di errore dell'ADR fuori dal catalogo C-7
L'ADR usa `E_PROJECTION_DOWN`, `E_STORAGE_FULL`, `E_BUS_DOWN`, `E_BUS_GENERATION` (e cita `E_PROJECTION_LAG` come scartato). C-7 vuole `code` stabili in inglese a punti in un solo catalogo del dominio. AD-2 dice solo «un codice (C-7)» senza nomi, e nessuna AD elenca i quattro casi come codici da prevedere. Due storie inventeranno nomi diversi.
**Fix:** in AD-2 elencare i motivi di rifiuto (database fermo, disco pieno, bus giù, generazione errata) con i nomi nel formato a punti di C-7 (per esempio `projection.down`), e dire che l'ADR usa nomi provvisori.

### 6. [media] Open Question «Ambito della verità sul bus» con scadenza vaga
Scadenza: «Update della spine prima della prima storia che aggiunge una risorsa oltre le esecuzioni o uno di quei controlli». Idempotenza, C-3, C-4 e `If-Match` (AD-18) sono i controlli della primissima storia di «Esegui»: la scadenza di fatto è lo sprint 1, ma non è scritta così. Inoltre AD-3 e la riga dicono che il motore tiene in memoria lo stato accettato non ancora proiettato, ma AD-18 non lo prescrive: l'ADR lo ha solo argomentato, il costo SQL non è misurato.
**Fix:** scadenza «prima dello sprint 1 per le esecuzioni; Update per ogni risorsa nuova», e in AD-18 una frase che dica dove i controlli leggono (proiezione più memoria del motore) o che questo è l'oggetto della domanda.

### 7. [media] Confine tra database «lento» e «fermo» non definito
AD-2 distingue lento (si accetta, avviso oltre 1 s) da fermo (si rifiuta), ma non dice come il motore le distingue. La Open Question sul tetto della memoria ha scadenza «spec della storia dell'avviso (FR-48)», che è una storia di avviso: il comportamento di accettazione senza tetto (memoria cresce senza limite oltre 25 s) può essere costruito prima in una storia che non legge quella riga.
**Fix:** in AD-2 «Ritardo» aggiungere il criterio provvisorio («database non risponde entro N s = fermo», N da fissare) oppure alzare la scadenza della Open Question alla prima storia del proiettore (Epic 1).

### 8. [media] Conservazione: il meccanismo non è nella regola
AD-2 dice «il bus non elimina mai posizioni non applicate», ma non il vincolo che l'ADR ha scoperto (punto 8): la posizione si salva prima della cancellazione, ogni 500 ms, altrimenti Iggy cancella ogni segmento chiuso; il gruppo `sentinel` e più consumatori non sono provati e potrebbero spostare la soglia. Una storia che aggiunge il sentinella può rompere la conservazione.
**Fix:** in AD-2 una riga: la soglia di conservazione dipende solo dalla posizione del proiettore, i consumatori in sola lettura (sentinella, pipeline stream) non la spostano; ordine «salva la posizione, poi `delete_segments`».

### 9. [bassa] Sentinella e topic dei log senza casa
AD-2 cita il sentinella in Binds e nella forma sul bus, ma non è né nel layout di crate (riga 782) né nei componenti: in Deferred è un «prototipo». Chi lo costruisce, e con quale utente (`read_stream`, `read_topics`, `poll_messages`, ADR punto 13), non è fissato. Il topic `log-<tenant>` (AD-17) non è fissato in nessuna regola, mentre `state-<id>` sì.
**Fix:** una riga in AD-2 «Forma sul bus» con il nome dei topic dei log e una nota che il sentinella è fuori dal motore e fuori dalla v1 finché una storia non lo chiede.

### 10. [bassa] Doppione di H2 e identità del tentativo
Dopo un crash prima che la scadenza sia durevole, l'attività «si riconsegna». AD-2 non dice se la riconsegna è lo stesso `attemptId` o un tentativo nuovo (C-2); da questo dipende se il worker vede un doppione di un tentativo o un secondo tentativo, e se si conta nel limite di tentativi. L'ADR non l'ha esercitato.
**Fix:** una riga in AD-2: la riconsegna dopo la ripresa è lo stesso tentativo (stesso `attemptId`), il worker deve essere idempotente per `attemptId`; oppure aprire la domanda.

### 11. [bassa] Stack e librerie
Il gRPC dei worker (tonic 0.14.9, rustls) è tra le «librerie non ancora scelte» (riga 848) mentre l'ADR lo sceglie; nella tabella Stack (CLAUDE.md: solo licenze elencate nello Stack) mancano tonic e rustls. Coerente con la regola delle due settimane, ma una storia può usare un'altra versione.
**Fix:** aggiungere tonic 0.14.9 allo Stack come scelta o lasciare esplicito in AD-12 che la versione è fissata dalla prima storia.

### 12. [bassa] Formattazione di AD-1
La riga di AD-1 che termina con «mai al bus. L'installazione predefinita…» supera la larghezza degli altri paragrafi (riga 27 del diff). Nessun effetto sul contenuto.

## Cosa è a posto

- AD-2 è coerente: un solo scrittore per lo stato proiettato; le scritture del motore (segreti, chiavi) sono eccezione dichiarata e puntano solo a righe del motore; con SQLite passano dallo stesso mutex; con Postgres la corsia unica non si applica e il testo lo dice («con SQLite»). L'obiettivo di 1 s e i rifiuti (fermo, disco pieno, bus giù, evento non applicabile) non si contraddicono.
- Nessuna contraddizione con l'ADR sulla decisione di consegna, annullamento (stop al meglio) e scadenza dell'esecuzione dall'accettazione.
- Le Open Questions con scadenza «S-1» hanno tutte una nuova scadenza; i riallineamenti del PRD (NFR-1, FR-8, FR-10) sono tracciati con proprietario e termine.
