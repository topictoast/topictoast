---
topic: rifinitura dell'aggiornamento del PRD dopo la UX (bmad-prd, Finalize)
date: 2026-11-20
target: prd.md, addendum.md (solo il testo cambiato il 2026-11-20)
lenses: structure, prose
---

# Rifinitura dell'aggiornamento

Lettura del documento: invariata rispetto a ieri (`polish-prd.md`); modello Strategic/Context.
Perimetro: le parti cambiate oggi (48 blocchi in `prd.md`, le due sezioni nuove dell'addendum).
Il testo rifinito ieri non è stato toccato. Conteggio parole con `wc -w`, stimato: prd.md da
14.125 a 14.148, addendum.md da 1.447 a 1.451.

## Struttura (5 modifiche, testo invariato)

1. FR-27: l'ultima conseguenza (scadenza, lavoro non salvato, cambio di cliente) divisa in un punto
   e due sottopunti. "Il caso entra nel test di NFR-8" resta con il cambio di cliente.
2. FR-55: la seconda conseguenza divisa in un punto (linguaggio chiuso, regola comune, §11) e due
   sottopunti (nessun riferimento esterno; manifest ostili nel test di NFR-8).
3. FR-33: "Un segreto eliminato durante un'esecuzione…" diventa una conseguenza a sé: è un caso
   diverso dall'avvio rifiutato.
4. FR-46: la definizione di "tempo in coda" passa dal requisito alle conseguenze; nel requisito
   resta "l'ordinamento per tempo in coda". È l'unico spostamento dal requisito alle conseguenze.
5. Addendum, voce "Connessioni": tre sottopunti (architettura, spec della storia, scelta mai
   implicita), stesso testo.

## Prosa (14 modifiche)

1. §3 Connessione: "Due manifest che scrivono" diventa "che dichiarano", il verbo della voce.
2. FR-4: "senza dettagli del suo manifest, che il nucleo JS non riceve" diventa "(il nucleo JS non
   lo riceve)".
3. FR-4: frase lunga divisa in due; "tipi, campi obbligatori … sono errori" diventa "Tipi sbagliati,
   campi obbligatori mancanti … sono errori". "Nel workflow di un cliente" vale ancora solo per le
   connessioni.
4. FR-4: "o oltre le connessioni del manifest" diventa "o in più rispetto alle connessioni".
5. FR-5: "un salvataggio … avvisa prima della pianificazione" era ambiguo ("prima della" = prima
   di). Diventa "Prima di un salvataggio che cambia lo schema di input (FR-56), l'editor avvisa
   della pianificazione e delle pipeline stream…". Lettura scelta: l'avviso arriva prima di
   salvare, come la conferma decisa con Marco nella UX.
6. FR-16: "(validazione, compilazione, istanziazione)" torna accanto a "trattamento dei byte";
   prima sembrava riferito al manifest.
7. FR-29: "Un task senza manifest, come `call http`, di un workflow dell'integratore" diventa "In
   un workflow dell'integratore, un task senza manifest, come `call http`, …".
8. FR-40: "dal nodo di un task" diventa "dal nodo personalizzato di un task", come nel glossario.
9. FR-55: "porta lo stesso manifest" (sembrava lo stesso documento) diventa "Anche un nodo
   personalizzato (FR-15) porta un manifest, con gli stessi campi".
10. FR-55: "gli stessi campi dal manifest … le voci nell'editor" diventa "gli stessi campi, presi
    dal manifest … le voci" ("nell'editor" era ripetuto).
11. FR-46: "l'età del task o dell'attività in coda più vecchi" diventa "l'età del più vecchio tra i
    task e le attività in coda".
12. Addendum: «topictoast» diventa "topictoast" (virgolette dritte, come nel PRD).
13. Addendum: "nel contratto del kit, architettura" diventa "nel contratto del kit: architettura",
    come le altre voci.
14. Addendum: "e l'ultimo uso di un segreto con workflow e task" diventa "e se l'ultimo uso di un
    segreto indica workflow e task", che è il senso della nota di sicurezza da cui viene.

## Non applicato

- FR-41: il modulo di lancio e la chiave di idempotenza stanno nella vista delle esecuzioni, dove
  li ha messi il piano di modifica. Spostarli cambia quale FR (e quale storia) li porta. Decide
  Marco.
- FR-48: l'avviso di disco pieno sembra fuori tema in "Worker", ma la UX lo mette come riga fissa
  nella pagina dei worker. Lasciato.
- FR-41 e FR-43: la frase sulla ripresa dopo un crash è quasi uguale nei due FR. Lasciata, così
  ogni componente si legge da solo.
- FR-28: la seconda e la terza conseguenza si sovrappongono (cosa mostra il kit, cosa permette la
  credenziale). Livelli diversi; lasciate.
- FR-33 e FR-29: l'avvio rifiutato per una connessione senza segreto compare in tutti e due. FR-33
  vale per ogni workflow, FR-29 per quelli dell'integratore: toglierne uno restringe la portata.
- FR-5: "Il costo è accettato" introduce due costi. "I due costi" aggiungerebbe un conteggio;
  lasciato.
- FR-6: otto conseguenze, ma quelle sulla pianificazione sono già vicine. Raggrupparle sotto un
  punto richiede testo nuovo; lasciate.
- §3 Manifest: ripete i campi di FR-55. È la definizione; lasciata.
- §3 Stati: le regole vengono prima dell'elenco. Mettere prima l'elenco richiede una riga vuota
  che cambia la resa di tutto il glossario; lasciato.
- §3 Connessione: il senso "di rete" è attribuito a FR-16, FR-50 e NFR-8, ma anche FR-55 dice
  "connessioni di rete" (in modo esplicito, quindi senza ambiguità). Aggiungere FR-55 è un
  riferimento nuovo: decide Marco.
- "Oltre quota" in FR-8, FR-30 e FR-53 (testo non cambiato oggi), contro gli stati "vicino al
  limite, al limite": è descrittivo, non uno stato. Lasciato.
- FR-27 e FR-28: "cliente scelto" e "cliente indicato nella credenziale" sono la stessa cosa e si
  capiscono tutti e due; lasciati.
- Addendum: l'ultima voce di "Dopo la UX" ripete §11 n. 15. Lasciata, così l'addendum si legge da
  solo.
- Righe oltre 100 caratteri: solo la tabella del §5 e addendum.md riga 37, testo già presente;
  fuori perimetro.

## Verifica

Confronto con le copie prese prima della rifinitura (`prd.pre-polish-update.md`,
`addendum.pre-polish-update.md`), in entrambi i file:

- FR-n, NFR-n, UJ-n, SM-n e §n: stesso insieme e stesso numero di occorrenze di ciascuno.
- Numeri: stesso numero di occorrenze per ogni sequenza di cifre.
- Tag `[ASSUMPTION…]`, intestazioni, frontmatter (`status: draft`) e righe del §12: identici.
- Dopo la passata di struttura, l'elenco ordinato delle parole differiva solo per i 9 trattini
  nuovi, l'etichetta di FR-46, "e non" diventato "Non" e "(§11)," diventato "(§11)." in FR-55,
  "quando" diventato "Quando" nell'addendum. Le parole cambiate dalla passata di prosa sono quelle
  elencate sopra.
- FR-4, il segreto "in più rispetto alle connessioni" e il nucleo JS che non riceve il manifest:
  letture verificate sulla nota di sicurezza da cui vengono le due regole.
- Nessuna riga nuova oltre 100 caratteri. Nessun identificativo di lavoro interno nel testo
  aggiunto.
