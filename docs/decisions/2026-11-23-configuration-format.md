# ADR: formato della configurazione, TOML facoltativo e variabili d'ambiente

- Data: 2026-11-23
- Stato: accettata
- Registra: C-19 di `conventions.md` e della [spine](../architecture/architecture-topictoast-2026-11-20/ARCHITECTURE-SPINE.md); chiude la domanda aperta sul formato
  di configurazione, da chiudere prima di S-1
- Fonti: PRD §6 (superficie pubblica), FR-50, NFR-14, NFR-15; C-7, C-10, C-12;
  [memlog](../architecture/architecture-topictoast-2026-11-20/.memlog.md), voce 75

## Contesto

Il formato della configurazione è una superficie pubblica (PRD §6). Con la configurazione
predefinita un workflow di esempio gira senza modifiche (FR-50). Ogni valore omesso ha un
predefinito documentato e limitato, applicato dopo la validazione (NFR-14, C-10). La chiave
principale viene da un file o da una variabile (C-12). Il motore riparte entro 30 s (NFR-15).

## Decisione

C-19:
- un file TOML facoltativo, indicato da `TOPICTOAST_CONFIG`, e senza file valgono i predefiniti;
- variabili `TOPICTOAST_<SEZIONE>__<CHIAVE>`, che vincono sul file; le altre `TOPICTOAST_*` sono
  dei client e il motore le ignora;
- chiavi in snake_case inglese;
- chiavi sconosciute rifiutate, predefiniti applicati dopo la validazione ed entro i massimi;
- con una configurazione non valida il motore non parte: una riga C-7 su stdout con percorso e
  regola, mai il valore, e codice d'uscita diverso da zero;
- chiavi segrete in un elenco chiuso; nel file solo come percorso di un file, e un valore nel file
  fa rifiutare l'avvio;
- agli ausiliari solo le chiavi del loro ruolo, mai segreti (AD-6);
- nessuna ricarica a caldo.

Dalle revisioni di questo aggiornamento (memlog, voce 77): variabili dei client senza doppio
trattino basso, elenco chiuso delle chiavi segrete, nessun segreto verso gli ausiliari.

## Bilancio

- **Funzioni:** un file con i commenti per chi installa; variabili per i container; un errore
  chiaro all'avvio invece di un motore che parte a metà.
- **Costi:** ogni cambio richiede un riavvio, entro i 30 s di NFR-15.
- **Manutenzione:** un solo punto che legge e valida la configurazione. Togliere o rinominare una
  chiave è una rottura di contratto, da annunciare come vuole il PRD (§6).

## Alternative scartate

- **Ricarica a caldo di alcuni valori:** più comoda per chi gestisce, ma ogni valore ricaricabile va
  pensato e provato, e il riavvio sta già entro 30 s.
- **Solo variabili d'ambiente:** adatte ai container, ma i molti limiti di NFR-14 diventano un elenco
  lungo, senza commenti.
- **YAML o JSON:** YAML ha trappole di indentazione e di tipi impliciti; JSON non ammette commenti.
