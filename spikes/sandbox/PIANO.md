# Spike S-2, la sandbox: piano delle fasi

Fonte dei criteri: spine, sezione «S-2 — Sandbox». Approccio a fasi deciso da Marco il 2026-11-26
(memlog dell'architettura). L'ADR finale è `docs/decisions/YYYY-MM-DD-sandbox.md` e aggiorna AD-6.

| Fase | Cosa | Stato |
| --- | --- | --- |
| 1 | A tavolino: candidati e versioni dei due livelli, TS e Rust verso wasm, formato del modulo, cosa chiede root, Landlock e seccomp | chiusa (`fase1-tavolino.md`); Marco: modulo core e soglie della fase 2 (2026-11-26) |
| 2 | Livello leggero (senza KVM né root): avvio, memoria, limiti, TS e Rust compilati, codice ostile, un'istanza per tenant | chiusa (`fase2-risultati.md`): L1-L12 passano; riserva su NFR-1 con più ausiliari fermi |
| 3 | Livello forte (microVM con pool): avvio con una sandbox libera, codice ostile, declassamento, parte del pool per tenant | chiusa (`fase3-risultati.md`): S1-S9 passano; S1 con riserva (artefatto da 3,7 MB oltre i 10 ms nella lettura stretta) |
| 4 | Processi ausiliari: Landlock e seccomp per ognuno, lettura della chiave per ogni sottocomando, arresto per inattività contro NFR-2, NFR-1 con gli ausiliari attivi | da fare |

## Criteri (dalla spine)

- NFR-12: leggera avvio p50 ≤ 10 ms e ≤ 64 MB per istanza; forte ≤ 10 ms con una sandbox libera
  nel pool (assunzione del PRD che lo spike conferma o corregge).
- NFR-1 con gli ausiliari attivi: dal 2026-11-26 conta solo i processi di topictoast (il bus a parte).
- L'arresto per inattività degli ausiliari non peggiora il p99 di NFR-2 (SM-C2).
- FR-16: fuori dal processo della chiave, mai due tenant nella stessa istanza.
- FR-17: limiti di CPU, memoria e tempo.
- TS e Rust compilati girano nella leggera; il formato non esclude Python e Go dopo la v1.
- NFR-8: codice ostile noto a tutti e due i livelli (fuga, esaurimento di risorse, rete interna,
  lettura della chiave).
- NFR-10: x86_64 misurato; arm64 a tavolino, dichiarato non misurato (Marco, 2026-11-26).
- FR-50: l'installazione predefinita non chiede KVM né root.

Da fissare e provare nell'ADR: istanze per tenant per jq, `call http`, validazione e validazione a
secco del SQL, contro i riavvii a freddo di NFR-2 e contro NFR-1; Landlock e seccomp per ogni
ausiliario prima di toccare contenuto dei tenant, con il ripiego dove Landlock manca; la prova di
lettura della chiave principale per ogni sottocomando (file della chiave, `/proc` del motore, file
del database, configurazione, cartella di un altro tenant); il tenant carica solo bytecode, che
topictoast valida e compila nell'ausiliario; la cache del codice nativo la scrive solo topictoast.

## Regole delle corse

- Come nello spike del bus: regole di lettura committate prima delle corse misurate, soglie che non
  cambiano dopo una corsa, ogni causa «misurata» (con file grezzo) o «ipotesi», file grezzi in
  `spikes/sandbox/risultati/`.
- Misure solo su minisheep (x86_64, KVM, Landlock). I runner di GitHub su minisheep non si toccano;
  si fermano solo processi e container propri, per PID o nome salvati.
- Niente sudo né impostazioni di sistema: niente jailer di Firecracker e niente rete tap; le
  microVM parlano solo su vsock. Cosa servirebbe con root lo dice la fase 1 e lo riporta l'ADR.
- Codice dello spike fuori dai workspace Cargo e npm e fuori dal gate; niente Python.

## Uscita

Un file di risultati per fase (`fase1-tavolino.md`, `fase2-risultati.md`, …), poi l'ADR e la PR
che aggiorna la spine (intento Update, id stabili).
