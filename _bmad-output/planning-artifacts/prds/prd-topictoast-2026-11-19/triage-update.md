---
title: topictoast — triage delle revisioni dell'aggiornamento del PRD
created: 2026-11-20
oggetto: prd.md, addendum.md, EXPERIENCE.md; review-security-update.md (SU-), review-edge-cases-update.md (EU-), review-rubric-update.md (RU-)
---

# Triage delle revisioni dell'aggiornamento (2026-11-20)

Un rilievo per riga, tutte le gravità. La rubrica non ha ID: RU-1…RU-16 seguono l'ordine dei suoi
rilievi, RU-N1…RU-N6 le sue Mechanical notes. Esiti: **applicato**; **rinviato: owner**;
**saltato: motivo**. Le sette scelte di prodotto rimaste aperte le ha decise Marco (sotto).
"EX" = `EXPERIENCE.md`. I mock in `mockups/` non sono stati toccati: le spine vincono.

| ID | Gravità | Esito | Dove |
|---|---|---|---|
| SU-A1 | alta | applicato: connessioni fissate nella versione, segreto risolto all'avvio; §11 n. 10, 11, 14 restano aperte | FR-33, FR-48; EX Worker, Voice |
| SU-A2 | alta | applicato alle credenziali firmate per il kit; come il motore le distingue: rinviato, architettura (addendum) | FR-28, NFR-8 |
| SU-A3 | alta | applicato; nessuna tecnologia nel PRD, candidati nell'addendum | FR-55, FR-56 (Note), FR-16, NFR-7, NFR-14, §11 n. 16 |
| SU-M1 | media | applicato: stesso cliente e ruolo, scarto al cambio, solo nella memoria della pagina (decisione di Marco, con EU-M4) | FR-27; EX `client-selector`, Stati trasversali |
| SU-M2 | media | rinviato: spec della storia (§11 n. 14 resta aperta; "mai implicita" annotato nell'addendum) | §11 n. 14 |
| SU-M3 | media | applicato | FR-29, FR-32 |
| SU-M4 | media | applicato | FR-32 |
| SU-M5 | media | applicato: log; ruolo sempre registrato e mostrato, più il nome; nome conservato quanto il record (decisioni di Marco) | FR-52, FR-6, FR-10, FR-27; EX Voice, State Patterns |
| SU-M6 | media | applicato (origine); ruolo registrato: rinviato, spec della storia (con EU-B4); conferma: rinviata, T-62 UX | FR-46 |
| SU-M7 | media | applicato: ogni workflow dell'integratore disponibile a tutti i clienti, tolto «a lui disponibili» (decisione di Marco) | FR-28, FR-57, FR-58; EX `client-selector` |
| SU-M8 | media | applicato | NFR-7 |
| SU-M9 | media | applicato | FR-27 |
| SU-B1 | bassa | applicato | FR-4 |
| SU-B2 | bassa | applicato | FR-4 |
| SU-B3 | bassa | saltato: il divieto di omonimi contraddice FR-29 e il `chip` di provenienza di EX; la categoria è già visibile (FR-55, EX `category-tag`) | — |
| SU-B4 | bassa | applicato (mai nei log); pagina senza percorsi dell'host: rinviato, architettura (T-63 UX) | FR-45 |
| SU-B5 | bassa | applicato | FR-56 |
| SU-B6 | bassa | applicato: nella v1 bastano le destinazioni per cliente (decisione di Marco) | §8.2 |
| EU-A1 | alta | applicato | FR-27; EX Voice, Stati trasversali |
| EU-A2 | alta | applicato; §11 n. 12 e 13 chiuse; ritentare o sospendere: rinviato, spec della storia; dove l'operatore vede le scadenze rifiutate (la dashboard non ha pianificazioni): rinviato, spec della storia della dashboard | FR-6, FR-24, FR-57, §11 n. 17; EX `schedule-section`, Voice, State Patterns |
| EU-A3 | alta | applicato | §3 Stati, FR-46; EX State Patterns, `data-table`, UJ-4 passo 3 |
| EU-A4 | alta | applicato; ritentare un output non conforme: rinviato, spec della storia (addendum) | FR-55, FR-53 |
| EU-M1 | media | applicato; chiave per scadenza: rinviato, architettura (addendum) | FR-6 |
| EU-M2 | media | applicato | FR-6; EX `schedule-section` |
| EU-M3 | media | rinviato: spec della storia di FR-14 (con T-35); un nuovo stato del worker aggiorna §3 | — |
| EU-M4 | media | applicato: un ricaricamento perde il lavoro non salvato, nulla nel browser (decisione di Marco) | FR-27; EX Stati trasversali |
| EU-M5 | media | applicato per «Esegui»; «Salva» ripetuto: rinviato, spec della storia (T-16); API: architettura | FR-41 |
| EU-M6 | media | applicato in parte (linguaggio chiuso, regola unica); dettaglio rinviato: architettura | FR-55, §11 n. 16 |
| EU-M7 | media | applicato (la domanda resta aperta) | §11 n. 11 |
| EU-M8 | media | applicato; sotto-workflow dell'integratore e connessione modificabile: rinviato (addendum) | FR-33; EX Voice |
| EU-M9 | media | applicato | FR-58; EX `confirm-dialog`, Voice, Workflow (elenco) |
| EU-M10 | media | applicato (elenco dai manifest disponibili, identità); forma dei nomi, due connessioni uguali: rinviato, architettura | FR-32, §3 Connessione |
| EU-M11 | media | applicato (limiti); granularità del rifiuto, campi sconosciuti: rinviato, spec della storia | NFR-14 |
| EU-B1 | bassa | rinviato: spec della storia | — |
| EU-B2 | bassa | rinviato: spec della storia | — |
| EU-B3 | bassa | applicato in parte (pianificazione attiva, righe rifiutate); disco e checkpoint come cause, motivi multipli: rinviato, spec della storia | §3, FR-6 |
| EU-B4 | bassa | rinviato: spec della storia; lunghezza e caratteri del nome in SU-M9 | — |
| EU-B5 | bassa | applicato | NFR-6 |
| EU-B6 | bassa | rinviato: spec della storia dell'editor | — |
| EU-B7 | bassa | rinviato: architettura (T-63 UX) | — |
| EU-B8 | bassa | rinviato: spec della storia | — |
| EU-B9 | bassa | rinviato: spec della storia; lato motore coperto da SU-A2 | FR-28 |
| EU-B10 | bassa | applicato | FR-39 |
| EU-B11 | bassa | applicato (la domanda resta aperta) | §11 n. 11 |
| RU-1 | alta | Avvii rifiutati dai trigger automatici: applicato (con EU-A2) | FR-6, FR-24, §11 n. 12–13 |
| RU-2 | media | «Salvare è pubblicare» senza il costo: applicato, costi nominati e avviso sullo schema di input (decisione di Marco) | FR-5; EX `confirm-dialog`, Voice, Salvare ed eseguire |
| RU-3 | media | Allargamento della v1 non nei rischi: applicato; motivo di NFR-16 dalla variante di EX UJ-2 | §10.5, UJ-2 |
| RU-4 | bassa | Schema di input forse già in OWS: rinviato, architettura (verifica sul DSL, addendum) | §11 n. 7 |
| RU-5 | media | Fonti dei dati di FR-40: applicato; mock non toccati | FR-40; EX `data-mapping` |
| RU-6 | media | Manifest non controllato durante l'esecuzione: applicato (EU-A4, SU-A3) | FR-55, §11 n. 16 |
| RU-7 | media | «In coda» definito sulle attività: applicato (EU-A3) | §3, FR-46 |
| RU-8 | media | SM-6 senza criterio di riuscita: applicato, da sola, su desktop, entro 20 minuti, 3 su 3 (decisione di Marco) | SM-6 |
| RU-9 | bassa | «Altri metadati» in un formato pubblico: applicato; forma: architettura | FR-55, §3 Manifest; EX Foundation |
| RU-10 | bassa | NFR-9 senza metodo: applicato | NFR-9 |
| RU-11 | bassa | Manifest e schema senza limiti: applicato (SU-A3) | NFR-14 |
| RU-12 | media | Workflow non eliminabile: applicato (EU-M9) | FR-58 |
| RU-13 | media | FR-1 e FR-57 sulla pianificazione: applicato, `schedule` nel documento fuori dal sottoinsieme | FR-1; addendum |
| RU-14 | media | Identità della connessione: applicato (stessa piattaforma e tipo di credenziale) | §3 Connessione |
| RU-15 | bassa | «A lui disponibili» o «tutti i clienti»: applicato con SU-M7 | FR-28, FR-57, FR-58 |
| RU-16 | bassa | Indicazioni di layout: applicato | FR-40, FR-54, FR-55 |
| RU-N1 | bassa | Termini: applicato (NFR-14, §3, NFR-8, FR-50, FR-38); «oltre quota» saltato: è l'evento, «al limite» lo stato. Pianificazione «attiva» aggiunta alla tabella delle icone di `DESIGN.md` | vari; EX State Patterns; DESIGN |
| RU-N2 | bassa | ID e «Realizza»: applicato «Realizza»; ordine saltato: numerazione stabile (§0) | FR-55, FR-56, FR-58 |
| RU-N3 | bassa | Rimandi: applicato | FR-28, NFR-14, FR-58 |
| RU-N4 | bassa | Indice delle assunzioni: saltato, round trip corretto | — |
| RU-N5 | bassa | Rilievi di ieri chiusi: saltato, nota informativa | — |
| RU-N6 | bassa | Frontmatter: saltato, coerente (`status: draft`) | — |

## Decisioni di Marco (2026-11-20)

Opzione raccomandata confermata per tutte e sette, applicata a PRD ed EX.

1. **EU-M4 + SU-M1** — il lavoro non salvato resta solo nella memoria della pagina: un
   ricaricamento lo perde e il kit non lo salva nel browser (FR-27).
2. **SU-M5, ruolo** — lo storico registra e mostra sempre il ruolo, più il nome visualizzato se
   presente (FR-6, FR-10; EX Voice and Tone).
3. **SU-M5, nome** — il nome visualizzato segue la conservazione del record o l'eliminazione del
   cliente, e la documentazione lo dice (FR-27).
4. **SU-M7 + RU-15** — ogni workflow dell'integratore è disponibile a tutti i clienti (FR-29);
   tolto «a lui disponibili» da FR-28, FR-57, FR-58 ed EX.
5. **RU-2** — FR-5 nomina i due costi di «salvare è pubblicare»; un salvataggio che cambia lo
   schema di input avvisa della pianificazione e delle pipeline stream (EX `confirm-dialog`).
6. **RU-8** — SM-6 riesce se ciascuna delle tre ci riesce da sola, su desktop, entro 20 minuti;
   3 su 3.
7. **SU-B6** — nella v1 bastano le destinazioni per cliente (FR-16); il legame tra connessione e
   destinazioni va in §8.2, dopo la v1.
