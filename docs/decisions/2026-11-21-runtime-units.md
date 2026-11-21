# ADR: unità di runtime della v1

- Data: 2026-11-21
- Stato: accettata
- Registra: AD-1 della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md)
- Fonti: PRD FR-12, FR-50, FR-54, NFR-1, NFR-8, NFR-15, §1.1, §7, §8.2; brief e addendum del brief
  (§2, §6, §8); [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voci 6,
  52, 54, 55, 59; `review-decisions.md` (D-2); revisione `review-versions.md`

## Contesto

Le unità di runtime arrivano dal PRD e dal brief, non da una scelta dell'architettura: la domanda
sul dispiegamento è stata ritirata perché già decisa (memlog, voce 6).

- Due servizi da installare, topictoast e il bus, più il database a scelta (FR-50).
- Motore a nodo singolo che riparte da solo entro 30 s (NFR-15).
- RAM a riposo ≤ 80 MB, sommando motore, ogni processo ausiliario attivo e bus, con SQLite (NFR-1).
- Worker = processi dell'integratore sempre attivi, anche remoti su TLS (FR-12, NFR-8).
- Installazione predefinita senza Kubernetes, KVM né root, in ascolto solo su localhost (FR-50);
  la sandbox forte con KVM è opzionale (FR-54).
- Motivazione del brief: Temporal chiede quattro servizi più un database, Argo gira solo su
  Kubernetes con un pod per worker, Hatchet dipende da RabbitMQ (addendum del brief, §8).

## Decisione

Registra [AD-1](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md): un
motore Rust a nodo singolo, un bus come servizio separato, SQLite incorporato o Postgres; ogni
altro processo di topictoast lo avvia e lo ferma il motore. La tecnologia del bus non è decisa:
Apache Iggy (vincolo di partenza del PRD) e Valkey come livello caldo sono i candidati di S-1.

- **Versioni di Postgres** (D-2, decisione di Marco, memlog voce 55): major 18 e 19, minime 18.7 e
  19.0. Immagine di test 18.7 fino al 2026-11-30, poi 19.x. La 19.0 è stabile dal 2026-11-16 e si
  può fissare dal 2026-11-30 con la regola delle due settimane (memlog, voce 54).
- **Un solo motore per installazione:** il lock esclusivo all'avvio, che fa uscire con un codice
  un secondo motore, sta in AD-2 (ADR `2026-11-21-bus-source-of-truth.md`). Tiene vero il «nodo
  singolo» anche con un doppio avvio.
- **Scala:** i worker scalano in orizzontale; motore e bus in verticale fino a NFR-3, frase
  aggiunta dopo la verifica degli input (memlog, voce 59). Il PRD dice solo che nella v1 si scala
  aggiungendo worker e che il motore è a nodo singolo (§1.1).

Approvata da Marco in chat il 2026-11-21 (memlog voce 62).

## Bilancio di funzionalità, costi e facilità di manutenzione

- **Funzionalità:** installazione con due servizi più il database; i worker sono già in alta
  disponibilità (FR-12, FR-14); il motore torna a consegnare attività entro 30 s senza
  intervento; la sandbox forte c'è dove c'è KVM.
- **Costi:** niente alta disponibilità del centro nella v1. Durante la ripresa non partono nuove
  esecuzioni né task successivi: è una pausa, non una perdita (NFR-15). Motore e bus sono limitati
  a un host (obiettivo NFR-3: ≥ 10.000 attività/s). NFR-1 conta ogni processo ausiliario attivo.
  Backup e ripristino coerenti di database e bus restano fuori dalla v1 (NFR-6). Con Postgres
  sono supportate due major, 18 e 19.
- **Facilità di manutenzione:** un solo `docker compose` per lo sviluppo (CLAUDE.md); i processi
  ausiliari li avvia e li ferma il motore, senza un componente in più da installare (FR-50).

## Alternative scartate

Le prime quattro sono le strade che il PRD esclude; l'architettura le eredita.

- **Alta disponibilità del centro e cluster del bus nella v1.** Rinviati dopo la v1 (PRD §8.2):
  il cluster del bus «oggi non è ancora maturo». Per Iggy il cluster (VSR) è spento di default e
  «maturing toward production readiness»; una replica che non torna blocca la partizione
  (addendum del brief, §2).
- **Kubernetes.** Non-obiettivo: topictoast non lo richiede né lo sfrutta (PRD §7). È il limite
  di Argo che il brief vuole evitare.
- **KVM o root nell'installazione predefinita.** Esclusi da FR-50. Solo wasmtime non chiede né
  KVM né root, quindi è l'unico che rispetta «binario unico, facile da installare» (addendum del
  brief, §6). KVM resta per la sandbox forte opzionale (FR-54).
- **Più servizi obbligatori.** FR-50 ne ammette due più il database. La leggerezza si dimostra con
  RAM e carico sul database (NFR-1, NFR-4), non con il numero di componenti (PRD §1.1). Per questo
  RustFS resta facoltativo (AD-17).
- **Solo Postgres 19** (D-2, memlog voce 55): non si può fissare prima del 2026-11-30.
- **Postgres dalla 17 alla 19** (D-2, memlog voce 55): più combinazioni da provare.

## Conseguenze e criteri di revisione

- L'ADR dello spike del bus (S-1) aggiorna AD-1 con la tecnologia scelta. Criteri di S-1 che
  toccano AD-1: NFR-1 (≤ 80 MB con SQLite, bus e ausiliari attivi compresi); bus con
  autenticazione obbligatoria anche da localhost, credenziali generate all'installazione e nessun
  valore predefinito del candidato accettato (FR-12, FR-50).
- L'ADR della sandbox (S-2) decide quali processi ausiliari separare, chi riavvia il motore dopo
  un crash e l'endpoint di salute e di prontezza (Open Questions della spine, con M-28). Criterio:
  nessun componente da installare oltre topictoast, il bus e il database scelto. S-2 misura anche
  NFR-1 con gli ausiliari attivi e prova x86_64 e arm64 (NFR-10).
- Si riapre se il benchmark degli spike mostra che un nodo singolo non regge NFR-1 o NFR-3, o
  quando l'alta disponibilità del centro entra nel perimetro (dopo la v1): sarà un ADR nuovo.
- **Secondo blocco di `review-decisions.md`** (approvato, memlog voce 63): nessuna voce E-1…E-9
  cambia AD-1. Resta da valutare nelle Open Questions della spine cosa vuol dire «localhost»
  (socket Unix o loopback, deciso dall'indirizzo del socket; revisione di sicurezza M12), che
  tocca l'ascolto predefinito di AD-1.
