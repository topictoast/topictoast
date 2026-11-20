---
title: topictoast — piano delle modifiche al PRD dopo la UX
created: 2026-11-20
oggetto: candidati T-71…T-83 del triage UX (ux-topictoast-2026-11-19)
fonti: prd.md, .memlog.md, triage.md (PRD); triage.md, .memlog.md, EXPERIENCE.md, DESIGN.md, reconcile-prd.md (UX); addendum del brief
---

# Piano delle modifiche al PRD dopo la UX

Piano, non testo finale. `prd.md` non è toccato. Per ogni candidato: decisione della UX, punti del
PRD toccati, modifica minima, conflitti, stato.

**Sigle.** `prd L` = riga di `prd.md`. `P-mem L` = `.memlog.md` del PRD. `U-mem L` = `.memlog.md`
della UX. `EX L`, `DS L` = righe di `EXPERIENCE.md` e `DESIGN.md`. `U-triage` = `triage.md` della
UX; `P-triage` = quello del PRD. `brief-add L` = addendum del brief 2026-11-18. `§11 Dn` =
domanda aperta n del §11.

**Id nuovi.** L'FR più alto oggi è FR-54 (prd L538); NFR-15 (L1083); SM-5 (L1180). Proposti:
FR-55 manifest, FR-56 schema di input del workflow, FR-57 pianificazioni nel kit, FR-58 area
workflow con elenco dei workflow; NFR-16 schermi; SM-6 solo se Marco la vuole (T-76).

**Regola degli stati.** `deciso` solo se c'è una riga `(decision)` nel memlog della UX o un esito
in «Esiti del triage». Le righe `(assumption)` e `(event)` («da confermare») non bastano. Una voce
mista prende lo stato della parte che blocca; le parti applicabili subito sono indicate.

## Riepilogo

| Id | Titolo breve | Stato | FR e sezioni toccati |
|---|---|---|---|
| T-71 | Telefono, area workflow, avviso di pausa | deciso | NFR-16 (nuovo), FR-58 (nuovo), FR-41, FR-43, §8.1 |
| T-72 | Elenco chiuso degli stati, propagazione | rinvio (parziale) | §3, FR-8, FR-12, FR-14, UJ-1, §11 D8 |
| T-73 | Nome della persona nella credenziale | serve Marco | FR-10, FR-6, (FR-27) |
| T-74 | Superfici senza FR | deciso | FR-58 (nuovo), FR-46, UJ-4 |
| T-75 | UJ-4, UJ-5 incompleti; UJ-3 reale | serve Marco | UJ-3, UJ-4, UJ-5, §11 D1, §12 |
| T-76 | Criterio di facilità d'uso | serve Marco | SM-6 (nuovo, condizionato) |
| T-77 | Pianificazioni | deciso | FR-6, FR-57 (nuovo), §12, §11 |
| T-78 | Schema di input del workflow | deciso | FR-56 (nuovo), FR-40, FR-41, §11 |
| T-79 | Integratore nel kit | serve Marco | FR-5, FR-26, FR-28, FR-35, FR-41 |
| T-80 | Segnali all'operatore | serve Marco | FR-46, FR-48, FR-49, FR-54 |
| T-81 | Contratto pubblico del kit | deciso | FR-27, FR-44, FR-45, NFR-9 |
| T-82 | Scelte delle spine oltre gli FR | deciso | NFR-9 (una riga) |
| T-83 | Manifest di worker e nodi | serve Marco | FR-55 (nuovo), FR-4, FR-12, FR-15, FR-32, FR-37, FR-40, §3, §6, §11 |

**Conteggi:** deciso 6 · serve Marco 6 · rinvio 1 (totale 13).

---

## T-71 · Ampliamenti già decisi: telefono, area workflow, avviso di pausa

**Cosa ha deciso la UX.** Sul telefono (390/360 px) funziona tutto, editor del grafo ed editor SQL
compresi (U-mem L20); stesso canvas, palette e configurazione come fogli dal basso (L21).
Componente «area workflow» con navigazione interna, più i pezzi singoli (L26). Nel kit, durante la
ripresa NFR-15, avviso neutro «aggiornamenti in pausa, riprendono da soli»; il banner esplicito
solo nella dashboard (L27). Spine: EX L33–36, L52–53, L71–76, L270, L365–367; Ampliamenti 1–3
(EX L840–842).

**Dove tocca il PRD.** §4.9 L846–849: il kit ha editor, vista delle esecuzioni, segreti, editor
delle pipeline stream; nessun contenitore, nessun telefono. FR-40 L862–872 e FR-43 L890–896:
nessun requisito sugli schermi. NFR-15 L1086–1087: «è una pausa, non una perdita»; non dice cosa
vede il cliente. §8.1 L1131–1132: componenti del kit.

**Modifica proposta.**
- NFR-16 «Schermi»: kit e dashboard si usano per intero su desktop e da telefono (da 360 px, input touch), compresa la composizione nell'editor del grafo e nell'editor SQL.
- FR-58 «Area workflow» (vedi T-74).
- FR-41 e FR-43, nuova conseguenza: durante la ripresa la vista dice in modo neutro che gli aggiornamenti sono in pausa; stati e righe restano visibili; nessuna parola su motore o crash (FR-44).
- §8.1 L1131: aggiungere area workflow e uso da telefono.

**Conflitti.** Nessuno con P-mem. Nota: allarga la v1; restano veri P-mem L30 («nessun taglio, la
v1 slitta») e il rischio 5 (prd L1207–1210).

**Stato.** `deciso`.

## T-72 · Elenco chiuso degli stati e regola di propagazione

**Cosa ha deciso la UX.** Marco ha deciso le parole: «in attesa» = dipendenze non finite, «in
coda» = pronto ma non consegnato, con il motivo; «il PRD si allinea» (T-48, U-mem L51). L'elenco è
una proposta della UX [A-8] (EX L280–284). Stati per oggetto (EX L286–293):
- Esecuzione: in coda · in esecuzione · completata · fallita · annullata · scaduta.
- Task: in attesa · in coda · in esecuzione · ritentato · completato · fallito · saltato · annullato.
- Attività: in coda · ricevuta · in esecuzione · ritentata · completata · fallita · annullata.
- Worker: collegato · non risponde. Cliente (Quote): vicino al limite · al limite.
- Pipeline stream: attiva · rallentata · sospesa, con badge della causa se la sospensione è del sistema («per memoria di stato», «per arretrato», «dati scaduti»).
- Fuori dalla tabella: la pianificazione ha «Sospesa» (EX L258).

Regole (EX L295–311): il timeout è un motivo, non uno stato; «scaduta» solo per l'esecuzione;
«non risponde» copre il «non attivo» di FR-14 [A-10]; in un'esecuzione annullata o scaduta i task
non partiti diventano «annullato»; il task saltato dice perché. Propagazione: nessuna, attende
§11 D8. Stato dopo il limite di attesa: aperto (EX L197).

**Dove tocca il PRD.** §3 L203–260: il glossario non nomina gli stati. FR-12 L442 e UJ-1
L155–156: «restano in attesa» per l'attesa prima della consegna. FR-8 L390–394. FR-14 L469 «non
attivo» contro FR-48 L936 «non rispondono». FR-2 L304–305 e §11 D8 L1227–1228: propagazione
aperta.

**Modifica proposta.**
- FR-12 L442 e UJ-1 L155–156: «restano in attesa» diventa «restano in coda, con il motivo» (T-48).
- Da confermare: §3 voce «Stati» con l'elenco sopra, pianificazione compresa [A-8]; FR-14 L469 «non attivo» diventa «non risponde» [A-10].
- Propagazione: §11 D8 resta all'architettura.
- Stato dopo il limite di attesa: alla spec della storia di FR-8, con la scelta del limite (nota di P-triage T-13).

**Conflitti.** Nessuno. Il cambio di parole in FR-8, FR-12, UJ-1 è già accettato (U-mem L51).
P-mem L35 assegna la propagazione a bmad-architecture.

**Stato.** `rinvio`: propagazione all'architettura (§11 D8, P-triage T-30); stato dopo il limite
di attesa alla spec di FR-8. Si applicano subito solo FR-12 L442 e UJ-1 L155–156 (T-48); §3 [A-8]
e FR-14 [A-10] si confermano nella revisione di questo aggiornamento.

## T-73 · Il «chi» di FR-10: ruolo o nome della persona

**Cosa ha deciso la UX.** Il «chi» è il ruolo della credenziale («dal cliente»,
«dall'integratore»); un trigger dice la sua origine; mai il nome di una persona (EX L134–136,
L149–150). È l'autofix T-3 (classe A, nessuna decisione di prodotto). Sul nome nessuna
`(decision)`.

**Dove tocca il PRD.** FR-10 L416: «con chi l'ha annullata e quando». FR-27 L702–704: la
credenziale porta cliente e ruolo. §4.6 L673–675: topictoast non autentica gli utenti dell'ospite.
FR-6 L358–370: l'origine dell'esecuzione non è registrata.

**Modifica proposta.** Subito: FR-6 registra l'origine di ogni esecuzione (API o SDK con il ruolo,
pianificazione, pipeline stream). Opzione 1: FR-10 L416 → «con il ruolo che l'ha annullata
(cliente o integratore) e quando». Opzione 2: FR-27 aggiunge un campo facoltativo «nome
visualizzato», mostrato come testo (NFR-7) e cancellato con il cliente (FR-31 L758–762).

**Conflitti.** Nessuno. Con l'opzione 2 lo storico conserva un dato personale in più.

**Stato.** `serve Marco`. Domanda: kit e storico mostrano per nome chi ha avviato o annullato?
- Opzione 1 (consigliata): solo il ruolo. Nessun cambio alla credenziale; già nelle spine.
- Opzione 2 (reconcile B-01): campo facoltativo «nome visualizzato». Storico più leggibile; un dato personale in più da cancellare.

## T-74 · Superfici senza FR

**Cosa ha deciso la UX.** L'area workflow comprende l'elenco dei workflow (U-mem L26, decision).
Lacune registrate: elenco senza FR; Esecuzioni della dashboard raggiunta solo da UJ-5 per
assunzione (U-mem L42; EX L81, L102–106). L'elenco ha vuoto con «Nuovo workflow» e `chip`
«Dell'integratore», «da codice» (EX L330). Esecuzioni della dashboard: filtro «in coda» e colonna
«In coda da» (EX L267, L336).

**Dove tocca il PRD.** §4.9 L846–849: nessun elenco. UJ-2 L170: «salvato e rilanciabile». FR-46
L920–926: filtri per cliente, workflow, stato. UJ-4 L192–193: non passa da Esecuzioni. §2.1 L129:
l'operatore deve «accorgersi di esecuzioni bloccate».

**Modifica proposta.**
- FR-58 «Area workflow ed elenco dei workflow» (§4.9): lo sviluppatore dell'ospite incorpora un componente unico con navigazione interna (elenco dei workflow, editor, esecuzioni, segreti, pipeline stream); ogni sezione esiste anche come componente singolo. Conseguenze: l'elenco mostra i workflow del cliente e quelli dell'integratore a lui disponibili, con la provenienza; da lì si apre o si crea un workflow; solo risorse del cliente (FR-26); nessun marchio (FR-44).
- FR-46: filtro «in coda» e ordinamento per tempo in coda. UJ-4 L192–193: «e le esecuzioni ferme in coda da più tempo».

**Conflitti.** Nessuno.

**Stato.** `deciso`.

## T-75 · UJ-4 e UJ-5 incompleti; UJ-3 da confermare con un caso reale

**Cosa ha deciso la UX.** Niente da decidere; ha scritto i flussi. UJ-4 (EX L737–752): accesso,
Worker, Pipeline stream, Quote; momento di valore il worker che non risponde; caso limite il
cliente al limite. UJ-5 (EX L754–767): esito visto in Dashboard → Esecuzioni [A-26]. UJ-3 resta
l'esempio dedotto del PRD [A-25] (EX L711–712).

**Dove tocca il PRD.** UJ-3 L178–189, tag L179–180. UJ-4 L191–197: senza persona, stato iniziale,
esito. UJ-5 L199–201: un paragrafo, [ASSUMPTION]. §11 D1 L1216; §12 L1238–1239.

**Modifica proposta.** Subito: UJ-4 con persona (Marco come operatore), stato iniziale
(installazione attiva con clienti), esito, e il passo di T-74. UJ-5 nel formato degli altri:
momento di valore il workflow di esempio «Completata» in Esecuzioni della dashboard [A-26, da
confermare]; caso limite
la credenziale del primo avvio non a portata, la pagina d'accesso dice dove è stampata (FR-45).
Dopo la risposta: UJ-3 sul caso reale, via i tag (L179, §12 L1238), chiusa §11 D1.

**Conflitti.** P-mem L35: «caso reale UJ-3 e nome di flowtoast (owner: Marco; si rivedono prima di
bmad-ux)». Il controllo è saltato: la UX ha disegnato su un esempio dedotto.

**Stato.** `serve Marco`. Domanda: qual è il caso reale di pipeline stream (stream, query, cosa
succede dopo)?
- Opzione 1 (consigliata): Marco lo dà ora. Chiude §11 D1 prima dell'architettura e dello spike delle pipeline stream.
- Opzione 2: UJ-3 resta [ASSUMPTION] fino allo spike (come §11 D2). L'architettura progetta su un esempio dedotto.

## T-76 · Criterio di facilità d'uso per il cliente

**Cosa ha deciso la UX.** Nulla. Reconcile C-01 propone: «Giulia compone e lancia un workflow di
5 task senza leggere documentazione». Le spine hanno solo la prova con lettore di schermo sulla
costruzione di UJ-2 (EX L511–512).

**Dove tocca il PRD.** §2.1 L120–121 «senza programmare». NFR-13 L1024 e SM-5 L1180–1181
misurano solo Luca (UJ-5). SM-4 L1178–1179 misura i percorsi, non persone nuove.

**Modifica proposta (solo con l'opzione 1).** SM-6 «Composizione senza documentazione», nella
forma di SM-5: prima del primo rilascio pubblico, tre persone con il profilo della cliente di UJ-2
compongono, salvano e lanciano dall'editor il workflow di UJ-2 (almeno 5 task, con fan-out e
join) senza leggere documentazione. Valida FR-40, UJ-2.

**Conflitti.** Nessuno. SM-C3 L1189–1190 non è toccata.

**Stato.** `serve Marco`. Domanda: il PRD misura la promessa «senza programmare»?
- Opzione 1 (consigliata): SM-6 come sopra. Rende verificabile la promessa principale al cliente; una sessione di prova in più.
- Opzione 2: nessuna metrica; prova d'uso solo nella spec della storia dell'editor. Zero costo nel PRD; la promessa resta non misurata.

## T-77 · Pianificazioni

**Cosa ha deciso la UX.** T-42 (U-mem L48): sezione «Pianificazione» nel pannello del workflow
con cron o intervallo, prossima scadenza, «Sospendi» e testo per le scadenze recuperate dopo una
pausa. EX L258: «Ripeti» Nessuna, Ogni o Cron; «Sospendi» porta a «Sospesa», «Riprendi»
riattiva; scadenze perse e recuperate dopo una pausa; le esecuzioni dicono «da pianificazione»;
con ruolo cliente, su un workflow dell'integratore, sola lettura.

**Dove tocca il PRD.** FR-6 L365–366: pianificazioni con [ASSUMPTION]; §12 L1243. FR-1 L287–288:
fuori perimetro solo `after` e `on`. FR-26 L697–698: ogni trigger ha un cliente fissato alla
creazione. NFR-14 L1069: recupero delle scadenze perse. FR-38 L842: tutto ciò che si fa
dall'editor si fa anche da SDK.

**Modifica proposta.**
- FR-6 L366: via [ASSUMPTION]; via la riga di §12 L1243.
- FR-6, nuove conseguenze: una pianificazione si sospende e riprende senza modificare il workflow; dopo un fermo, scadenze perse e recuperate sono visibili al cliente (regola di recupero: NFR-14 e spec, P-triage T-32).
- FR-57 «Pianificazioni nel kit» (§4.9): il cliente crea, vede, sospende e riprende le pianificazioni dei propri workflow, con la prossima scadenza. Conseguenze: sola lettura sui workflow dell'integratore (FR-29); le stesse operazioni via SDK (FR-38).
- §11, domanda nuova (spec): quale input riceve un'esecuzione pianificata se il workflow dichiara un input obbligatorio (FR-56)?

**Conflitti.** Nessuno con P-mem. Punto per l'architettura: OWS mette `schedule` nel documento
(brief-add L32); un workflow dell'integratore è condiviso tra clienti, mentre FR-26 L697 vuole un
cliente per trigger. L'architettura decide dove vive la pianificazione.

**Stato.** `deciso`.

## T-78 · Schema di input dichiarato dal workflow

**Cosa ha deciso la UX.** T-47 (U-mem L50): «Esegui» apre un modulo generato dallo schema di
input del workflow; il campo elenco accetta testo incollato, una riga per elemento; «Mostra JSON»
è il modo avanzato; «richiede uno schema di input nel documento del workflow». Spine:
`input-form` (EX L259; senza schema si apre il JSON); «Input del workflow» nel pannello del
workflow, con nome, tipo e «obbligatorio» (EX L407–412). Che lo schema parta dal primo task è
[A-41]: resta nelle spine.

**Dove tocca il PRD.** §4.1 L266–270: il workflow non dichiara un input. FR-6 L358–370: input non
validato all'avvio. FR-24 L646–647: la riga emessa è l'input. FR-40 L862–866 e FR-41 L874–877:
nessun «Esegui» con input. UJ-2 L166–167: «lancia con un elenco di 300 indicatori».

**Modifica proposta.**
- FR-56 «Schema di input del workflow» (§4.1): un workflow può dichiarare lo schema del suo input (campi, tipi, obbligatori), dall'editor, dall'SDK o nel documento. Conseguenze: un avvio con input non conforme è rifiutato prima di essere accettato, con percorso e motivo; un workflow senza schema accetta qualunque input JSON, come oggi.
- FR-40, da confermare [A-41; U-triage T-47 lasciava aperto «chi lo definisce nell'editor»]: il cliente definisce lo schema di input nel pannello del workflow.
- FR-41: il cliente lancia dal kit con un modulo generato dallo schema; JSON come modo avanzato; senza schema solo JSON.
- §11, domanda nuova (spec): una riga di FR-24 che non rispetta lo schema (già in review-edge-cases L141).
- Forma nel documento: architettura (§11 D7). Le note del repo citano solo `input`/`output`/`export` (brief-add L31) e sono una sintesi da ricontrollare (L22–23); se il DSL abbia già uno schema di input si verifica su `dsl-reference.md`.

**Conflitti.** Nessuno.

**Stato.** `deciso`.

## T-79 · Ruolo integratore nel kit

**Cosa ha deciso la UX.** T-43 (U-mem L49): con la credenziale dell'integratore il kit mostra un
selettore del cliente (UJ-1 lancia per il cliente di prova); i workflow definiti come codice si
modificano nell'editor, ogni salvataggio è una versione, con `chip` «da codice» e avviso. Il
cliente scelto vale per tutte le sezioni; nei pezzi singoli è una proprietà (EX L73–75, L263).
Con ruolo cliente niente ID del worker; con ruolo integratore e in dashboard sì (EX L137–138,
autofix T-35).

**Dove tocca il PRD.** FR-26 L684–687: cliente solo dalla credenziale verificata, «mai da un campo
della richiesta». FR-27 L702–704: credenziale firmata «per ogni richiesta», con cliente e ruolo.
FR-28 L712–716: l'integratore vede tutti i clienti. FR-35 L811–817. FR-41 L876–877.

**Modifica proposta.**
- FR-28, nuova conseguenza: con la credenziale dell'integratore il kit agisce per un cliente scelto; ogni sezione mostra solo le sue risorse e «Esegui» lancia nel suo contesto.
- FR-35, nuova conseguenza: chi può modificarlo (FR-29 L729) modifica nell'editor un workflow definito come codice; ogni salvataggio è una versione (FR-5); la provenienza resta visibile; la prossima pubblicazione da codice crea un'altra versione.
- FR-41, nuova conseguenza: con ruolo cliente nessun identificativo di worker.
- FR-26: precisare da dove viene il cliente scelto (domanda sotto).

**Conflitti.** FR-26 L684–687 e P-mem L26 («topictoast ricava il cliente solo dalla credenziale
verificata»), P-mem L27 («cliente solo dalla credenziale»). Un selettore che mette il cliente
nella richiesta contraddice la lettera di FR-26. U-triage T-43 lo prevedeva: «cambia i claim
della credenziale firmata (FR-27)». FR-5 L348–350: una pubblicazione preparata su una versione non
più ultima è rifiutata, «mai in silenzio»; T-43 vuole che la pubblicazione da codice dopo un
salvataggio dall'editor crei un'altra versione. Proposta: FR-5 precisa che la pubblicazione da
codice crea sempre una versione nuova e l'editor lo avvisa; altrimenti la spec sceglie il rifiuto.

**Stato.** `serve Marco`. Domanda: con la credenziale dell'integratore, da dove viene il cliente
scelto?
- Opzione 1 (consigliata): campo del cliente scelto nella credenziale firmata dall'ospite. FR-26 resta alla lettera; l'ospite emette già credenziali a scadenza breve (FR-27 L702–704), ma a ogni cambio di cliente ne serve una nuova.
- Opzione 2: FR-26 vale per le credenziali di cliente; l'integratore indica il cliente nella richiesta, come già serve all'SDK di FR-38 L839. Più semplice per l'ospite; rilassa una regola confermata, il test di NFR-8 deve coprirla.

## T-80 · Segnali all'operatore

**Cosa ha deciso la UX.** Due stati in Quote, «Vicino al limite» e «Al limite» (EX L267, L293,
L339); la soglia di «vicino» è aperta (valore). Filtro «in coda» e «In coda da» (EX L336). FR-54:
T-49 ha scelto l'opzione 1, riga fissa in Worker (U-mem L51), quindi la clausola «FR-54 fuori
dalla UX» di T-80 decade. Nel kit: «Richiesta non accettata: spazio esaurito.» (EX L182). Disco
pieno nella dashboard: aperto (EX L355–356). «Annulla esecuzione» dalla dashboard: non deciso
(reconcile A-17).

**Dove tocca il PRD.** NFR-6 L1037–1038: a disco pieno rifiuta lavoro nuovo con un errore
esplicito. FR-49 L941 «i clienti al limite»; UJ-4 L193 «vicini alla quota». FR-46 L920–926 e
FR-48 L933–937: solo lettura («vede»). FR-10 L411 e FR-28 L712: integratore e operatore possono
annullare. FR-54 L549–550: «con un avviso all'operatore».

**Modifica proposta.** Subito: FR-49 «clienti vicini al limite e al limite», soglia di «vicino»
con valore predefinito (NFR-14, spec); FR-54 L550 «avviso all'operatore nella pagina dei worker».
Secondo la risposta: FR-48 (o §4.10) la dashboard segnala il disco pieno e il rifiuto di lavoro
nuovo, distinto dall'avviso di ripresa; FR-46 l'operatore annulla un'esecuzione dal dettaglio
(FR-10).

**Conflitti.** Nessuno. FR-10 e FR-28 permettono già all'operatore di annullare.

**Stato.** `serve Marco`. Domanda: la dashboard v1 ha l'avviso di disco pieno e «Annulla
esecuzione»?
- Opzione 1 (consigliata): entrambi. Copre «accorgersi di…» (§2.1 L129) e NFR-6; due piccole superfici (reconcile A-09, A-17).
- Opzione 2: solo disco pieno. Dashboard di sola lettura; l'annullamento resta a kit e SDK.
- Opzione 3: nessuno. Disco pieno solo da metriche e log (FR-52); l'operatore lo scopre dagli errori dei clienti.

## T-81 · Contratto pubblico del kit

**Cosa ha deciso la UX.** Temi, T-45 (U-mem L51): il kit verifica le coppie di contrasto dei
colori dell'ospite; errore visibile in sviluppo, avviso in console in produzione dove vince
l'ospite (EX L563–568). Sessione (autofix T-15): evento all'ospite almeno 60 s prima della
scadenza e alla scadenza, lavoro non salvato tenuto in memoria; la dashboard torna ad Accesso (EX
L370–374). Testi (autofix T-16): nomi accessibili e parole di stato mai vuoti, ricaduta sul
predefinito (EX L569–571). Marchio: ricerca di «topictoast» nel bundle e proprietà di opt-in (EX
L131–133). Meccanismi: architettura (T-63, U-mem L60).

**Dove tocca il PRD.** §6 L1093–1096: «componenti React (proprietà ed eventi)». FR-44 L898–905,
«se non lo si chiede» (L904). FR-27 L706–708 e FR-45 L917–918: nulla sulla scadenza. NFR-9
L1057–1058: AA senza qualifiche.

**Modifica proposta.**
- FR-27: il kit avvisa l'ospite prima della scadenza della credenziale e alla scadenza; il lavoro non salvato torna dopo il rinnovo. FR-45: alla scadenza la dashboard torna all'accesso senza dati.
- FR-44: il kit segnala le coppie di colori dell'ospite sotto soglia (errore in sviluppo, avviso in produzione); un testo mancante o vuoto ricade sul predefinito della lingua; nome e logo di topictoast solo con un'opzione esplicita, verificata in automatico sul bundle.
- NFR-9: AA garantito con i colori predefiniti; con i colori dell'ospite vale il controllo di FR-44.
- Numeri (60 s), nomi degli eventi e API: architettura (T-63).

**Conflitti.** Nessuno. La qualifica su NFR-9 serve perché T-45 fa vincere l'ospite.

**Stato.** `deciso` (T-45 di Marco; il resto deriva da NFR-9 e FR-44). Meccanismi rinviati (T-63).

## T-82 · Scelte delle spine oltre gli FR

**Cosa ha deciso la UX.** Si leggono le spine finali, non l'elenco (d) di reconcile-prd: T-46
sostituisce «layout automatico», T-47 l'input JSON, T-53 «pagina iniziale Esecuzioni».

| Scelta (spine finali) | Dove | Perché |
|---|---|---|
| Disposizione manuale e automatica su desktop, automatica sul telefono (T-46, U-mem L45) | spine; posizioni: architettura | interazione; FR-40 L871–872 resta vero |
| Tema chiaro e scuro (EX L54, L572–573) | spine | aspetto; FR-44 copre l'ospite |
| Vista a elenco in sola lettura (T-39, U-mem L44, L83) | PRD: una riga in NFR-9; dettaglio nelle spine | capacità che crea una storia (T-39 bloccava le epiche) |
| Tabella degli elementi del fan-out (EX L240) | spine | FR-41 L881 «resta leggibile» la copre |
| Tab Log con livelli (EX L253) | spine | eventi, non payload: coerente con FR-52 L989 |
| Voce impersonale (U-mem L19) | spine | testi sostituibili (FR-44) |
| Dashboard non personalizzabile (EX L574) | spine | FR-44 vale solo per il kit (L900–901) |
| Palette, token, Penpot (DS; CLAUDE.md) | spine e regole di progetto | non sono requisiti |

**Modifica proposta.** NFR-9: un workflow e un'esecuzione si leggono anche senza il grafo, in
un'alternativa in sola lettura; la modifica resta sul canvas, da tastiera.

**Conflitti.** Nessuno.

**Stato.** `deciso`.

## T-83 · Manifest dei worker e dei nodi personalizzati

**Cosa ha deciso la UX.** Due `(decision)` di Marco. U-mem L69: ogni worker espone un manifest che
dichiara, per ogni funzione, le connessioni esterne (piattaforme e credenziali richieste), lo
schema di input e di output e altri metadati («etc etc»). U-mem L71: lo stesso manifest vale per i
nodi personalizzati caricati con la CLI. Le frasi testuali («una sorta di manifest…», «Sì, stesso
manifest») vengono dalla richiesta che ha avviato questo piano; nei file non ci sono. Le
conseguenze sono `(assumption)` «da confermare» (U-mem L70) o `(event)` (L75: A-42, A-43, e
«validazione di tipi e campi obbligatori… da confermare nell'aggiornamento del PRD»).

Uso nelle spine: campi del manifest nome, descrizione, gruppo, connessioni, schemi (EX L40–46);
il gruppo ordina la palette [A-42]; la connessione non è un connettore né OAuth, il segreto
dichiara quella che soddisfa [A-43] (EX L47–51); palette con nome e descrizione (EX L254; DS
L830–833); campi generati dallo schema (EX L255); selettore con i soli segreti della connessione
(EX L256; DS L837–838); «Dati» con i soli campi compatibili degli schemi di output a monte (EX
L257); errori su tipi, obbligatori e connessioni senza segreto prima del salvataggio (EX L348–350,
L164–166); colonna connessione in Segreti (EX L267).

**OWS.** Nel repo nessuna nota su schemi o definizioni di funzione nel DSL: brief-add L31 cita solo
`input`/`output`/`export`, jq e il meccanismo dei segreti; L99–103 dice che i task sono chiusi e
`metadata` è libero; L22–23 avverte che è una sintesi da ricontrollare. Se OWS abbia già qualcosa
di simile si verifica su `dsl-reference.md`, con §11 D7 (architettura).

**Dove tocca il PRD.** FR-12 L437–439: registra le funzioni, senza schema. FR-37 L829: funzioni
«tipizzate» solo nel codice. FR-15 L482–483: già «nome e schema di input e output». FR-32
L775–776: si vedono nome, data, ultimo uso. FR-33 L781–782: il task dichiara i segreti. FR-40
L864–872: nulla su campi e dati. FR-4 L331–337 con FR-39 L859–860: stessi esiti di validazione.
§3: mancano «manifest» e «connessione». §6 L1093–1096. §11 D6 L1223–1224, D7 L1225–1226.

**Modifica proposta.**
- FR-55 «Manifest delle funzioni» (§4.3): registrando una funzione (FR-12) il worker ne dichiara il manifest: nome, descrizione, gruppo, connessioni richieste (piattaforma e tipo di credenziale), schema di input, schema di output; un nodo personalizzato porta lo stesso manifest. Conseguenze: manifest non valido rifiutato con l'errore del validatore; nell'editor funzioni e nodi si presentano allo stesso modo; il formato è contratto pubblico.
- FR-12: «registra le sue funzioni con il loro manifest (FR-55)». FR-15 L482–483: «con il manifest (FR-55)»; cambiano solo connessioni e metadati. FR-37: la funzione tipizzata dichiara il manifest (come l'SDK lo ricava dai tipi: addendum).
- FR-40, nuove conseguenze: ogni campo dello schema di input ha una fonte (valore, campo compatibile di un task a monte, jq come modo avanzato; T-41, U-mem L47); ogni connessione ha un segreto; errori sul task prima del salvataggio.
- FR-4: stessi controlli all'invio (parità di FR-39); all'esecuzione si ricontrolla se il manifest è cambiato.
- FR-32, se sì: il segreto dichiara la connessione che soddisfa; si vedono nome, connessione, data, ultimo uso.
- §3: «Manifest» e «Connessione», precisando che non è una connessione di rete né un connettore. §6: aggiungere «formato del manifest».
- §11, domande nuove (architettura): quale manifest vale se nessun worker della funzione è collegato (la palette serve comunque) o se due worker registrano la stessa funzione con manifest diversi (FR-12 L444); manifest cambiato sotto workflow già salvati.

**Conflitti.**
- §11 D6 e P-mem L35 («connessioni per cliente (owner: bmad-architecture e spike…)»): A-43 decide il lato di prodotto prima dell'architettura; a questa resta come si salva.
- «Connessione» ha già il senso di rete in FR-16 L504, FR-50 L970, NFR-8 L1046–1049: il glossario deve distinguerli.
- §7 L1108–1109 (niente connettori): evitato per definizione (EX L47–49), va scritto.
- SM-C3 L1189–1190: nessun conflitto, il manifest descrive le funzioni dell'integratore e non aggiunge nodi.

**Stato.** `serve Marco` (il manifest è deciso, L69 e L71). Domanda: il segreto dichiara la
connessione che soddisfa (A-43)?
- Opzione 1 (consigliata): sì. Selettore con i soli segreti adatti, errore «connessione senza segreto» prima del salvataggio, §11 D6 chiusa sul lato prodotto; FR-32 ha un campo in più.
- Opzione 2: no, il segreto resta un nome. Il manifest nomina la connessione solo come aiuto, il selettore mostra tutti i segreti, un segreto sbagliato emerge all'esecuzione; D6 resta all'architettura.

Conferma separata, sì o no, indipendente da A-43: validazione di tipi e campi obbligatori dal
manifest nell'editor e nel motore (U-mem L75; consigliato sì, per la parità di FR-39).

---

## Fuori elenco

Assunzioni di «Elementi aperti» (EX L780–816) di competenza del PRD, non coperte da T-71…T-83:

- **A-19, NFR-9 L1058 [ASSUMPTION] WCAG 2.1 AA** (§12 L1259; ancora aperta in EX L791). Indizi: Marco ha scelto la lente WCAG 2.1 AA per la revisione (U-mem L39) e «AA senza deroghe» in T-44 (U-mem L51). Proposta da confermare: togliere il tag, con la qualifica di T-81 e la riga di T-82.
- **A-10** («non attivo» = «non risponde»): coperta da T-72 (FR-14 L469).
- **T-69** (U-mem L66): categorie future e segreti OAuth; aggiornamento futuro del PRD, quando cicli, LLM, connettori o OAuth entrano nel perimetro. Non ora.
- Le altre (A-2…A-44) sono di spine, Penpot o architettura.

Due note fuori da «Elementi aperti»:

- **Assunzioni del §12 da rivedere ora.** P-mem L35: si rivedono «al primo aggiornamento del PRD», cioè questo. Le spine ne usano già cinque: FR-5 L342 (salvare crea la versione, T-52), FR-10 L411 («Annulla esecuzione»), FR-3 L323 (entrambe le politiche nel pannello del task, EX L403–405), FR-41 L880 (1 s, EX L578), FR-44 L905 (it/en, U-mem L8). Proposta: confermarle nella revisione di questo aggiornamento.
- **Decisioni B senza candidato D.** T-40 (U-mem L46): FR-40 L864–865 «controlli di flusso» diventa «tutti i costrutti di FR-1». T-41: in T-83 (FR-40). T-52 (U-mem L51): nessun «pubblica»; in FR-5 L341 «pubblicare» vale anche per «salvare» dall'editor. T-46: architettura. T-49: in T-80.
