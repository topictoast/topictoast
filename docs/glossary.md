# Code glossary (Italian → English)

Rule (constitution VIII): **identifiers in English**, comments in Italian. A new concept gets its
row here before it is used.

## Domain concepts

| Italian | English |
|---|---|
| motore | engine |
| bus | bus |
| proiezione | projection |
| proiettore | projector |
| posizione nel bus | bus position |
| processo ausiliario | helper process |
| archivio dei payload | payload store |
| impronta (del contenuto) | digest |
| workflow | workflow |
| versione (di un workflow o di un nodo) | version |
| esecuzione | run |
| task | task |
| attività | activity |
| tentativo | attempt |
| identificativo di tentativo | attemptId |
| dipendenza | dependency (`dependsOn` in the format) |
| task DAG | `dag` task |
| fan-out | fan-out |
| join | join |
| worker | worker |
| protocollo dei worker | worker protocol (`crates/protocol`) |
| modello OWS | OWS model (`crates/ows`) |
| funzione (registrata da un worker) | function |
| nodo personalizzato | custom node |
| manifest | manifest |
| schema di input | input schema |
| contesto di validazione | validation context |
| connessione | connection |
| segreto | secret |
| cassaforte | vault |
| chiave principale | master key |
| chiave del tenant | tenant key |
| tenant | tenant |
| tenant predefinito | default tenant |
| identificativo interno (del tenant) | internal id |
| nome esterno (del tenant, nel JWT) | external name |
| proprietario (di workflow, esecuzioni, segreti) | owner |
| busta (cifratura) | envelope |
| cursore (paginazione) | cursor |
| integratore | integrator |
| operatore | operator |
| credenziale | credential |
| tenant scelto | selectedTenant |
| nome visualizzato | displayName |
| ruolo | role |
| quota | quota |
| pianificazione | schedule |
| scadenza (di una pianificazione) | schedule tick |
| pipeline stream | stream pipeline |
| finestra | window |
| watermark | watermark |
| checkpoint | checkpoint |
| sandbox leggera | light sandbox |
| sandbox forte | strong sandbox |
| nucleo JS | JS core (`packages/core`) |
| kit | kit |
| dashboard | dashboard |
| codice d'errore | error code (`code`) |
| motivo (di attesa o di salto) | reason |

## States (closed lists, PRD §3)

| Italian | English |
|---|---|
| in attesa | waiting |
| in coda | queued |
| ricevuta | received |
| in esecuzione | running |
| ritentato / ritentata | retried |
| completato / completata | completed |
| fallito / fallita | failed |
| saltato | skipped |
| annullato / annullata | cancelled |
| scaduta | expired |
