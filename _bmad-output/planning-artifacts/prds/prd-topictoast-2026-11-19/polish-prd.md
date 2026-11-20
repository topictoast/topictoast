---
topic: rifinitura del PRD (bmad-prd, Finalize passo 5)
date: 2026-11-19
target: prd.md
lenses: structure, prose (bmad-review)
---

# Rifinitura del PRD

Lettura del documento: serve a Marco e ai passi successivi (UX, architettura, epiche) per sapere
cosa fa la v1 e come si misura. Modello di struttura: Strategic/Context (piramide). Il documento era
già compatto; nessun taglio di contenuto. Verifica dopo ogni passata: stessi FR-n, NFR-n, UJ-n,
SM-n, §n, numeri, tag `[ASSUMPTION]` e intestazioni dell'originale; nessuna riga oltre 100
caratteri fuori dalla tabella del §5; frontmatter invariato.

## Struttura (8 modifiche)

1. §10 rischio 2: "sprint / 1." andava a capo con `1.` a inizio riga e diventava una lista
   numerata annidata; riga rifatta.
2. A capo rifatti su righe oltre 100 caratteri o spezzate a metà: §1.1 "Cosa non lo distingue",
   NFR-6, NFR-8, §7.
3. NFR-14: il paragrafo unico (circa 250 parole) diviso in quattro sottopunti con il testo
   invariato; "I valori li fissano le spec delle storie" portato in testa, dopo il titolo.
4. FR-16: la conseguenza sulla rete divisa in tre sottopunti (lista del nodo, indirizzi
   irraggiungibili, caso di test), stesso testo.
5. FR-26: la conseguenza "Un'esecuzione resta nel contesto del suo cliente" divisa in due
   sottopunti (risoluzione dei riferimenti e dei segreti; scrittura sugli stream), stesso testo.
6. FR-19: la conseguenza sulla rielaborazione dal checkpoint spostata in fondo, così le regole di
   validazione al salvataggio stanno insieme.
7. §5: l'avvertenza su `research-footprint.md` spostata dall'introduzione al paragrafo dei
   riferimenti sotto la tabella, con lo stesso testo.
8. §12: ultime voci messe nell'ordine del documento (NFR-9, NFR-10, §8.2 SDK, §8.2 alta
   disponibilità, SM-5).

## Prosa (10 modifiche)

1. Glossario: "pipeline" da sola diventa "pipeline stream" dove il riferimento è certo (§2.1
   operatore, UJ-3 ×3, FR-22, FR-23, FR-25 ×2, NFR-5 colonna "Come si misura", §10 rischio 2 ×2,
   §11 domanda 2).
2. Glossario: "chi integra" diventa "l'integratore" (§4.4 descrizione, §6 Documentazione).
3. §1.2 punto 8: "leggera di default" diventa "leggera, predefinita", come nel glossario e nel §4.4.
4. §1.1 "Cosa non lo distingue": "binario unico più database pure" (si legge "database pure")
   diventa "esiste già anche il binario unico più database".
5. FR-6: "per almeno la durata di conservazione" portato a inizio frase; prima sembrava riferito
   alle richieste contemporanee.
6. FR-8: "più a lungo del suo timeout del task" diventa "più a lungo del timeout del suo task".
7. FR-9: "solo quando la risposta parte dopo che è al sicuro" (soggetto ambiguo) diventa "solo se,
   quando parte la risposta, è già al sicuro".
8. FR-19: "(condizione di FR-24)" diventa "(condizione per FR-24)".
9. FR-24: "una esecuzione per riga" diventa "un'esecuzione per riga".
10. FR-32: "si vede solo nome, data e ultimo uso" diventa "si vedono solo …".

## Non applicato

- UJ-1 "compongano da sé le pipeline": non si capisce se sono workflow o pipeline stream;
  sceglierne uno cambierebbe il significato. Decide Marco.
- §1 visione e §1.1 punto 5 "pipeline SQL continue": formula descrittiva, prima del glossario;
  lasciata.
- §1 visione "Chi lo integra": frase di presentazione, prima del glossario; lasciata.
- §1.1 punto 5 "e alcuni prodotti orientati ai dati lo fanno": "lo" può indicare l'SQL continuo
  oppure DAG più SQL continuo insieme (`research-stream.md` cita prodotti che uniscono le due
  cose); scegliere cambierebbe l'affermazione. Da chiarire con Marco.
- NFR-5, "per una pipeline" dentro il tag `[ASSUMPTION]`: il testo dei tag non si tocca.
- UJ-3 senza "Persona e contesto", UJ-4 senza "Persona e contesto", "Stato iniziale" ed "Esito",
  UJ-5 in un solo paragrafo: completarli richiede contenuto nuovo.
- FR-47, FR-48, FR-49 senza "Conseguenze": aggiungerle vorrebbe dire aggiungere requisiti.
- FR-54 dopo FR-17 e NFR-12/NFR-13 nella tabella, lontani da NFR-6…NFR-11: si sistemerebbero solo
  rinumerando.
- FR-17, la clausola su NFR-2 ripetuta nelle prime due conseguenze: le condizioni sono diverse
  (superare un limite contro saturarlo); toglierne una cambia il requisito.
- FR-9, "consegna almeno una volta" detto due volte: la prima frase vale anche per la sandbox, la
  seconda solo per i worker; toglierne una ne restringe la portata.
- Sovrapposizioni tra §2.2, §7 e §8.2 (n8n, analisi dati, Kubernetes): rinforzo tra sezioni
  diverse; spostarle vorrebbe dire uscire dalla sezione.
- §11, domande da raggruppare per responsabile: si dovrebbero rinumerare.
