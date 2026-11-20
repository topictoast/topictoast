# Riconciliazione PRD ↔ spine UX (2026-11-20)

Sola lettura. **PRD** = `prd.md`; **AD** = `addendum.md`; **EX** e **DS** = `EXPERIENCE.md` e
`DESIGN.md` in `../../ux-designs/ux-topictoast-2026-11-19/`. Righe dei file attuali. Sul
perimetro vince il PRD, su aspetto e dettaglio del comportamento le spine. UJ-3 escluso.

**Allineati, nessuna contraddizione:** elenchi degli stati (PRD L289–295 = EX L287–292), workflow
come codice in sola lettura (FR-35 = EX L362–365), nome visualizzato o ruolo (FR-6, FR-10, FR-27
= EX L131–134), cliente nella credenziale (FR-27, FR-28 = EX L262), manifest e connessione (§3,
FR-55, FR-32 = EX L40–51), schema e modulo di input (FR-56, FR-40, FR-41 = EX L258, L408–413),
disco pieno e «Annulla esecuzione» (FR-48, FR-46 = EX L268, L335), pausa (FR-41, FR-43 = EX
L184, L269), telefono (NFR-16 = EX L52–53), vista in sola lettura (NFR-9 = EX L231, L509). Ogni
id FR/NFR/UJ/SM citato nelle spine esiste nel PRD; §12 coerente con i tag; nessuna voce di §11
rimanda a qualcosa ormai deciso. Sotto: lacune e incoerenze nate oggi, puntatori rimasti indietro.

Conteggi: alta 0 · media 9 · bassa 16.

## Media

**R-1 · media · «In coda» per un'esecuzione e «tempo in coda» non definiti** — PRD §3 L284–285,
L289; FR-46 L1047–1048; UJ-4 L197–198 · EX L266, L335, L746. «In coda» = pronto ma non
consegnato vale per task e attività; per un'esecuzione non si sa quando lo è. Se passa a «in
esecuzione» al primo task pronto, un'esecuzione con un task fermo senza worker sfugge al filtro
«in coda», proprio il caso di UJ-4. **Fix (PRD):** §3 definisce l'esecuzione «in coda» (accettata,
nessuna attività consegnata) e FR-46 misura «tempo in coda» come età dell'attività più vecchia in
coda dell'esecuzione (o aggiunge il filtro «con attività in coda»); EX L266, L746 seguono.

**R-2 · media · Pipeline stream: cause di sospensione e vista dell'operatore** — PRD §3 L293–294;
FR-17 L591–598; FR-25 L732–733; FR-23 L706–708; FR-30 L822–823; FR-47 L1057 · EX L291, L301–304,
L336, L747. Le cause chiuse (memoria di stato, arretrato, dati scaduti) mancano CPU e memoria di
esecuzione (FR-17, FR-25); «dati scaduti» in FR-23 è un errore alla ripresa, non una sospensione.
FR-47 mostra solo le pipeline «attive», ora nome di stato: rallentate e sospese, che FR-17/25/30
vogliono visibili all'operatore, restano fuori. **Fix (PRD):** cause = CPU, memoria (di stato o
di esecuzione), arretrato; «dati scaduti» resta un errore; FR-47 e UJ-4 «attive, rallentate e
sospese, con la causa». EX L301–304, L336, L747 seguono.

**R-3 · media · Segreti dei workflow dell'integratore** — PRD FR-4 L373–375 (nuovo), FR-40
L955–956, FR-32 L854; FR-26 L759, FR-29 L803–804 · EX L358–361, L401–402. «Connessione senza
segreto = errore all'invio» non regge per un workflow dell'integratore: all'invio non c'è cliente,
i segreti arrivano dal cliente dell'esecuzione. Non si dice cosa succede se il cliente non ha un
segreto per la connessione, o ne ha due. Rinvio già aperto (PRD `triage.md` rinvii gruppo 8,
review-adversarial M4; review-edge-cases M4); il testo di oggi lo rende esplicito. **Fix (PRD):**
per un workflow dell'integratore la connessione si risolve all'avvio tra i segreti del cliente;
senza segreto adatto l'avvio è rifiutato prima dell'accettazione, con la connessione; regola per
più segreti adatti. EX: testo e stato di «Esegui» per questo rifiuto.

**R-4 · media · Task senza manifest: `call http` e sotto-workflow** — PRD FR-1 L311–313; FR-32
L854; FR-40 L946–948, L953–956; FR-4 L373–375 · EX L401–402, L432–435. Il selettore mostra solo
i segreti che soddisfano «la connessione richiesta», ma `call http` non ha manifest né
connessione: dal kit non si può dargli un segreto. Per un sotto-workflow i campi e il controllo
all'invio dovrebbero venire dallo schema di FR-56 del workflow chiamato; nessuno lo dice. Come
il task lo scrive nel formato è dell'architettura (FR-1 L326–327, §11 D7 L1370–1371); qui il
lato di prodotto. **Fix (PRD):** FR-32/FR-40: un task senza manifest sceglie tra tutti i segreti
del cliente (o dichiara la connessione); FR-4/FR-40: l'input di un sotto-workflow segue il suo FR-56.

**R-5 · media · Pianificazioni nel kit: operazioni, numero, workflow come codice** — PRD FR-57
L1011–1016; FR-6 L418–420; FR-35 L896–897; §11 D12 L1380–1381; AD L105–107 · EX L257, L330.
FR-57 elenca creare, vedere, sospendere, riprendere; EX cambia intervallo o cron e toglie la
pianificazione con «Ripeti: Nessuna». EX ne prevede una per workflow, FR-57 non lo dice. Un
workflow come codice è in sola lettura «per tutti»: non si sa se la sua pianificazione si gestisce
dal kit. **Fix (PRD):** FR-57 aggiunge modificare ed eliminare, dice una o più per workflow e cosa
vale sui workflow come codice (dove vive resta all'architettura, AD L105–107). **EX:** la
`schedule-section` cita FR-57 e la domanda §11 D12 (input di un'esecuzione pianificata).

**R-6 · media · Fan-out con input singolo contro schema di input** — PRD FR-3 L353–354, FR-56
L397–399 (nuovo); UJ-2 variante L172–173 · EX L258, L686–687. FR-3 accetta un valore singolo
al posto dell'elenco; FR-56 rifiuta un avvio non conforme: con lo schema «elenco», un singolo
indicatore via API è rifiutato. **Fix (PRD):** FR-56, un valore singolo per un campo elenco vale
come elenco di un elemento (FR-3).

**R-7 · media · NFR-9 non copre l'area workflow** — PRD NFR-9 L1191–1195; FR-58 L1018–1028; FR-57
· EX L504–505, L517–519. NFR-9 elenca editor, vista delle esecuzioni, segreti e dashboard; area
workflow, elenco dei workflow e `client-selector` (nuovi oggi) restano fuori dall'AA. **Fix
(PRD):** NFR-9 «tutti i componenti del kit, area workflow compresa, e la dashboard»; EX segue.

**R-8 · media · Integratore nel kit: proprietà e modifica dei workflow** — PRD FR-28 L791–792;
FR-58 L1025–1028; FR-29 L805 · EX L262, L358–361. EX tiene in sola lettura i workflow
dell'integratore solo «con ruolo cliente» e offre «Nuovo workflow» a tutti; il PRD non dice di
chi è un workflow creato con la credenziale dell'integratore (cliente scelto o integratore) né se
un workflow condiviso si modifica da sezioni limitate a «solo le risorse di quel cliente».
**Fix (PRD):** FR-28 conseguenza su proprietario del workflow creato e modificabilità dei workflow
dell'integratore nel kit; FR-58 «risorse del cliente e dell'integratore a lui disponibili».

**R-9 · media · Workflow di esempio nell'elenco del kit** — EX L329 · PRD FR-50 L1098, UJ-5
L210–211, FR-26, FR-29. EX fa comparire «subito» il workflow di esempio nell'elenco del cliente;
il PRD lo fa avviare a Luca e non lo rende visibile ai clienti. In produzione finirebbe
nell'elenco di ogni cliente. **Fix (EX):** togliere la frase, o «se l'integratore l'ha
pubblicato»; se serve davvero, prima una conseguenza in FR-50 (PRD).

## Bassa

**R-10** — EX L579 «valore ancora assunto nel PRD»: FR-41 L969 non ha più il tag. **Fix (EX):** via.

**R-11** — Puntatori superati: EX L136 (T-79 → FR-28, FR-41), L196 (T-72 → spec di FR-8, AD
L111–112), L266 (T-80 → FR-49, NFR-14; «PRD §2.1» → FR-46), L411 (T-78 → FR-56, §11 D7), L297 e
L652 («il PRD si allinea»: fatto). A-2 (EX L143, L787) è chiusa da PRD §3 L289–291. **Fix (EX):**
aggiornare i rimandi, chiudere A-2.

**R-12** — Nuovi FR non citati dove le spine li realizzano: EX L82 (FR-56, FR-57), L162 e L164
(FR-55, FR-4), L177 e L257 (FR-57), L178 e L258 (FR-56, FR-41), L231 (NFR-9), L407 (FR-15 →
FR-55). **Fix (EX):** aggiungere i rimandi.

**R-13** — EX L98: Esecuzioni (dashboard) solo UJ-5, manca UJ-4 (PRD L197–198). **Fix (EX):** +UJ-4.

**R-14** — Copertura dell'elenco chiuso: EX L285–292 senza la riga Pianificazione «sospesa» (PRD
L296); DS L718–730 senza icone per collegato, non risponde, ricevuta, attiva, rallentata, sospesa
(il `state-badge` vuole sempre l'icona); DS L106 e L578 senza «ricevuta» nella famiglia neutra
(EX L289). **Fix (EX, DS):** aggiungere la riga, le icone e «ricevuta».

**R-15** — Parole di stato fuori elenco nel PRD: L128 «worker attivi» (→ collegati), L129
«clienti oltre quota» (→ al limite), L465 FR-10 «in corso» (→ in coda o in esecuzione), L722
«righe … in attesa» (in §3 «in attesa» è dei task; anche EX L169, L304), L1064 FR-48 fasi senza
«annullata». **Fix (PRD; EX L169, L304 seguono).**

**R-16** — PRD FR-40 L953–954 «ogni campo … ha una fonte» include i facoltativi; FR-4 L374 ed EX
L256 la vogliono solo per gli obbligatori. **Fix (PRD):** «un obbligatorio senza fonte è un errore».

**R-17** — PRD FR-55 L535–536 «si presentano allo stesso modo»; EX L42–45 e DS L571–575 li
distinguono per categoria (tessera, colore, `TS`/`RS`). **Fix (PRD):** «con gli stessi campi dal
manifest; la categoria li distingue».

**R-18** — EX L40–41 «Ogni worker espone un manifest … per ogni funzione»; PRD §3 L249–250 e FR-55
L529 lo vogliono per funzione (§11 D10). **Fix (EX):** «ogni funzione registrata ha il manifest».

**R-19** — PRD NFR-16 L1225–1227 «per intero» da telefono; EX L390–392, L483–484, L493–494: sul
telefono niente posizioni manuali né `Alt+frecce`. **Fix (PRD):** «sul telefono disposizione automatica».

**R-20** — EX L338 vuoto di Quote «Nessun cliente vicino al limite.»; FR-49 L1072 mostra l'uso di
ogni cliente. **Fix (EX):** vuoto «Nessun cliente.»; l'altra frase come esito di un filtro.

**R-21** — EX L268 e DS L872–874 usano lo stesso riquadro per ripresa e disco pieno, distinti dal
testo (FR-48 L1067–1068), ma non dicono cosa si vede quando coincidono (un disco pieno può causare
il crash). **Fix (EX):** i due avvisi possono stare insieme, uno sopra l'altro.

**R-22** — Riepiloghi e residui del PRD: §4.9 L929–930 senza elenco e pianificazioni; §8.1
L1272–1275 senza pianificazioni e senza la dashboard da telefono (NFR-16); FR-15 L555–556
«(modulo compilato, schema)»; §6 L1233–1237 senza il formato della credenziale firmata, che oggi
porta anche nome visualizzato e cliente scelto (FR-27 L771–780). **Fix (PRD):** aggiungere le
voci mancanti, «schema» → «manifest», la credenziale firmata tra le superfici pubbliche.

**R-23** — AD L108–110: spine da allineare su FR-35; EX L362–365 lo è già. **Fix (AD):** «fatto».

**R-24** — Stato di un'attività oltre il limite di attesa: §3 L284 chiude l'elenco, FR-8 L446–447
non dice l'esito; il rinvio sta solo in AD L111–112, non in §11. **Fix (PRD):** voce in §11 (spec
di FR-8), o «fallito, con il motivo» in §3.

**R-25** — EX L262: il `client-selector` cerca i clienti «per nome»; il PRD non dà un nome ai
clienti (FR-28 L790, FR-38 L920). **Fix (PRD):** FR-38 «un cliente ha un nome visualizzabile».
