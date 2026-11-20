---
title: topictoast — riconciliazione brief → PRD
created: 2026-11-19
input: brief-topictoast-2026-11-18 (brief.md + .memlog.md)
contro: prd.md + addendum.md (prd-topictoast-2026-11-19)
---

# Riconciliazione: cosa il brief ha e il PRD ha perso

Esclusi per istruzione: le sei differenze dichiarate nel PRD §1.2 e la ricerca tecnica rimasta
nell'addendum del brief. Verificato come coperto, quindi non riportato: ordine delle differenze 1-4,
"vantaggio di esecuzione, non barriera", "Rust da solo non convince", il giorno in cui funziona
(UJ-2 e UJ-4), "non sapere che sotto c'è topictoast" (FR-44), cassaforte e sandbox, taglio v1,
criteri di successo, rischi del brief, licenza, OAuth dopo la v1.

Sintesi: 9 lacune, 0 alta, 5 media, 4 bassa. Nessuna contraddizione frontale; il PRD perde soprattutto
la parte qualitativa (posizionamento onesto, obiettivi di esperienza d'uso, ampiezza degli SDK) e un
requisito del caso concreto (indicatore singolo o elenco).

---

## G1. "Cosa non lo rende diverso": la leggerezza non si argomenta con il numero di componenti

- **Gravità:** media
- **Il brief dice** (§"Cosa lo rende diverso", ultimo blocco): «DAG, YAML e database esistono già
  (Kestra, Windmill, Argo); binario unico più database pure (Hatchet, Windmill, Restate). Con SQLite
  topictoast ha due processi, come Hatchet con solo Postgres: la leggerezza si argomenta con memoria
  e carico, non con il numero di componenti.» Nel memlog è un'assunzione esplicita da mettere alla
  prova, poi mantenuta dall'utente come una delle «quattro parti» da non tagliare.
- **Il PRD:** §1.1 elenca solo le differenze. FR-50 ("al massimo due processi più il database") e
  UJ-1 fanno invece del numero di processi un requisito di prodotto, senza dire che non è la
  differenza. Il posizionamento onesto sparisce, e con lui il motivo per cui NFR-1 e NFR-4 contano.
- **Dove atterra:** §1.1, paragrafo dopo l'elenco ("Cosa non lo distingue"); nota in FR-50 e §10 (rischio).
- **Correzione:** aggiungere in §1.1 il paragrafo "Cosa non lo distingue" (DAG, YAML, binario unico
  esistono già; la leggerezza si dimostra con RAM e carico del database, non con il numero di processi)
  e un rimando da FR-50.

## G2. Obiettivi di esperienza d'uso senza requisito: "facile da configurare, facile da usare"

- **Gravità:** media
- **Il brief dice** (memlog, brain dump utente): «Obiettivi: veloce, facile da configurare, facile da
  usare, buon numero di SDK». Il brief li traduce in "Velocità, poca memoria, alta scalabilità"
  (differenza 3) e nella sezione "Il problema" (motori pesanti o fragili: Temporal quattro servizi
  più database).
- **Il PRD:** "veloce" ha NFR-1…NFR-5. "Facile da configurare" resta in FR-50 ("valori predefiniti
  che funzionano", workflow di esempio senza modifiche) e UJ-1. "Facile da usare" non ha né un
  requisito né una misura (tempo al primo workflow, passi per arrivare al momento di valore).
  Nessuna metrica per lo sviluppatore sconosciuto che trova il repository (il secondo pubblico del
  brief, "il pubblico dell'open source"); UJ-1 è solo Marco, che conosce già il motore.
- **Dove atterra:** §5 (nuovo NFR), §2.3 (UJ-1 variante), §9 (metrica secondaria).
- **Correzione:** aggiungere un NFR "tempo al primo workflow" (per esempio dal clone al primo
  workflow eseguito in N minuti, misurato su una macchina pulita) e una riga in UJ-1 o un UJ-5 per un
  integratore che non è Marco.

## G3. "Buon numero di SDK": nessuna direzione oltre TypeScript e Rust

- **Gravità:** media
- **Il brief dice** (memlog, obiettivi): «buon numero di SDK». Nodi personalizzati in Python e Go
  «dopo» (§"Fuori, per dopo"); sandbox «deve reggere quattro linguaggi» (memlog).
- **Il PRD:** §4.8 e §8.1 fissano due SDK. §8.2 rimanda Python e Go solo per i nodi personalizzati;
  gli SDK per worker e workflow in altri linguaggi non compaiono né in v1 né in "dopo la v1". Nessun
  requisito che il protocollo di worker e API sia documentato e indipendente dal linguaggio, cioè la
  condizione perché altri SDK nascano (anche dalla comunità).
- **Dove atterra:** §6 (superfici pubbliche), §8.2, FR-37, §11 (domanda aperta).
- **Correzione:** in §8.2 aggiungere "SDK Python e Go per worker e workflow, dopo la v1"; in §6 una
  riga "il protocollo di worker e l'API sono documentati in modo da poter scrivere SDK in altri
  linguaggi".

## G4. Il caso concreto: la stessa esecuzione riceve un indicatore solo o un elenco

- **Gravità:** media
- **Il brief dice** (§"Il problema", caso concreto): «A seconda di come il cliente configura la
  pipeline, l'esecuzione riceve un indicatore solo o un elenco da elaborare in parallelo.» Memlog:
  «Un'esecuzione può ricevere un indicatore o un elenco a seconda di come l'utente finale configura la
  pipeline: il fan-out dinamico in parallelo è un requisito».
- **Il PRD:** UJ-2 mostra solo l'elenco da 300. FR-3 copre elenco vuoto e N elementi, ma non dice che
  lo stesso workflow deve funzionare con un elemento solo (valore scalare al posto di elenco) e che
  la scelta spetta al cliente nell'editor. È la ragione d'essere del fan-out dinamico, e il caso
  limite "input singolo" non è verificabile così com'è.
- **Dove atterra:** FR-3 (conseguenze), UJ-2 (caso limite), FR-40.
- **Correzione:** in FR-3 aggiungere "un fan-out configurato su un input singolo si comporta come un
  elenco di un elemento" e in FR-40 che l'editor permette di scegliere tra input singolo ed elenco.

## G5. "Isolamento forte" tra clienti senza prova pari a quella della sandbox

- **Gravità:** media
- **Il brief dice** (§differenza 4): «isolamento forte di workflow e storico, segreti cifrati per
  cliente»; nel problema: il cliente «vede lo storico delle proprie esecuzioni e dei propri workflow,
  e nient'altro». Memlog: separazione tra clienti come requisito di primo piano, topictoast «cassaforte
  multi-tenant».
- **Il PRD:** FR-26 è ben formulato, ma NFR-8 prevede il test con codice ostile solo per la sandbox.
  Non c'è un criterio di rilascio per la separazione dei dati (test sistematico di accesso tra clienti
  su ogni risorsa, anche tramite flussi in tempo reale, filtri, conteggi). Un'unica falla qui
  annulla la differenza 4.
- **Dove atterra:** NFR-8, §9 (SM-4 o nuova metrica secondaria).
- **Correzione:** in NFR-8 aggiungere "prima del primo rilascio un test automatico prova, per ogni
  tipo di risorsa e per ogni endpoint, che un cliente non legge né modifica risorse di un altro".

## G6. Differenza 4 dichiara "OAuth rinnovato dal motore", la v1 no

- **Gravità:** bassa
- **Il brief dice** (§differenza 4): «segreti cifrati per cliente, OAuth rinnovato dal motore e
  sandbox per il codice dei clienti, tutto nella versione open»; §soluzione 5: topictoast «li rinnova».
- **Il PRD:** §1.1 punto 4 omette OAuth senza dirlo; §4.7 e §8.2 lo rimandano (corretto per il taglio
  v1). Resta senza traccia la distinzione tra ciò che è differenza a regime e ciò che la v1 dimostra.
  Rischio di promettere in comunicazione ciò che la v1 non ha.
- **Dove atterra:** §1.1 punto 4.
- **Correzione:** aggiungere "(OAuth gestito dal motore dopo la v1)" al punto 4.

## G7. Dashboard: "ogni fase del loro lavoro"

- **Gravità:** bassa
- **Il brief dice** (§soluzione 4): «Mostra i worker attivi e ogni fase del loro lavoro.»
- **Il PRD:** FR-48 mostra worker, funzioni registrate, attività in corso e da quanto tempo, worker che
  non rispondono. Mancano le fasi (ricevuta, in esecuzione, completata, fallita, ritentata) e la
  cronologia recente per worker; "il giorno in cui funziona" dice che l'autore «vede i worker al
  lavoro».
- **Dove atterra:** FR-48.
- **Correzione:** aggiungere in FR-48 la cronologia recente delle attività per worker con lo stato di
  ogni fase.

## G8. Integrazioni per cliente ridotte a segreti

- **Gravità:** bassa
- **Il brief dice** (§problema 3 e memlog): il cliente «configura le proprie integrazioni»; memlog:
  «nodi custom e integrazioni per tenant»; kit React per «configurare segreti e accessi a piattaforme
  esterne».
- **Il PRD:** esiste solo il segreto (nome, valore). Manca il concetto di connessione a una piattaforma
  esterna (per esempio "Censys del cliente X") che un task referenzia e che in futuro porta l'OAuth;
  nel glossario "Segreto" è già "credenziale per una piattaforma esterna", ma il flusso utente è per
  singola chiave.
- **Dove atterra:** §3 glossario, FR-32, FR-42.
- **Correzione:** decidere se "connessione" è un termine del PRD ora (segreto con tipo e piattaforma) o
  annotare in §11 che l'architettura la introdurrà con l'OAuth.

## G9. FR-50 anticipa una scelta che il brief lascia allo spike della sandbox

- **Gravità:** bassa
- **Il brief dice** (§rischi, memlog): sandbox «da scegliere con uno spike»; memlog: wasmtime
  predefinito, «Firecracker opzionale» (richiede KVM e demone).
- **Il PRD:** FR-50 impone "non richiede Kubernetes, KVM né privilegi di root", segnato come
  `[ASSUMPTION]`. Esclude in partenza Firecracker come opzione per chi ha bisogno di isolamento più
  forte; coerente con "leggero", ma in tensione con "deve reggere codice ostile".
- **Dove atterra:** FR-50, §4.4.
- **Correzione:** riformulare in "l'installazione predefinita non richiede KVM né root; un isolamento
  più forte può richiederli come opzione" e rimandare allo spike.

---

## Riepilogo per gravità

| ID | Gravità | Tema | Dove nel PRD |
|---|---|---|---|
| G1 | media | "Cosa non lo rende diverso" | §1.1, FR-50 |
| G2 | media | Facile da configurare e da usare, senza misura | §5, UJ-1, §9 |
| G3 | media | Buon numero di SDK: nessuna direzione | §6, §8.2, FR-37 |
| G4 | media | Indicatore singolo o elenco | FR-3, UJ-2, FR-40 |
| G5 | media | Isolamento tra clienti senza test di rilascio | NFR-8 |
| G6 | bassa | OAuth nella differenza 4 | §1.1 |
| G7 | bassa | "Ogni fase" del lavoro dei worker | FR-48 |
| G8 | bassa | Integrazioni per cliente | §3, FR-32 |
| G9 | bassa | KVM escluso prima dello spike | FR-50 |
