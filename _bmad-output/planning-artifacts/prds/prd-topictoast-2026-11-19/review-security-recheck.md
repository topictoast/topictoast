# Verifica delle correzioni di sicurezza nel PRD topictoast

**Verdetto:** le sei critiche e otto delle dieci alte sono chiuse dal PRD corrente; A3 e A6 restano parziali, ciascuna chiudibile con una riga di testo da applicare prima dello status final.

Base: `prd.md` e `.memlog.md` nella versione delle 14:18 del 2026-11-19, confrontati con le correzioni proposte in `review-security.md`.

| ID | Esito | Dove nel PRD | Cosa manca e correzione minima |
|---|---|---|---|
| C1 | CHIUSA | FR-12 (credenziale di worker distinta, rifiuto di registrazione, bus con credenziale), NFR-8 | — |
| C2 | CHIUSA | FR-26 (cliente e permessi solo dalla credenziale verificata, caso A con parametro B), FR-27 (firmata, scadenza breve, cliente e ruolo, scaduta rifiutata), NFR-8 (test verticale) | — |
| C3 | CHIUSA | FR-16 (ogni connessione per conto di un cliente, `call http` compresa; lista dell'integratore; DNS e redirect; caso nel test di NFR-8), NFR-14 (dimensione della risposta) | — |
| C4 | CHIUSA | FR-17 (limiti di CPU, memoria e tempo su jq; motore non si ferma; niente ambiente, file, moduli, rete; `$ENV`, `env`, `input_filename`, `import`, `include` rifiutati alla validazione) | — |
| C5 | CHIUSA | FR-19 (dialetto chiuso, query ostili nel test di NFR-8, rielaborazione deterministica, cicli rifiutati), FR-17, FR-25 (quota di CPU con conseguenza) | — |
| C6 | CHIUSA | FR-16 (a entrambi i livelli mai dati di due clienti, insieme o in sequenza; test con segreto sentinella) | — |
| A1 | CHIUSA | FR-34 (nessun supporto persistente con segreti in chiaro; ricerca del valore sentinella) | — |
| A2 | CHIUSA | NFR-8 (connessioni cifrate con autenticazione del server, in chiaro solo da localhost, worker remoto in chiaro rifiutato), FR-50 | Facoltativo: l'elenco tra parentesi non nomina la connessione tra motore e Postgres; aggiungerla evita che l'elenco sembri esaustivo. |
| A3 | PARZIALE | FR-4 (l'integratore dichiara le funzioni disponibili; rifiuto all'invio e controllo all'esecuzione) | Chiuso il "chiunque invoca qualsiasi funzione"; resta il "con qualsiasi input": una funzione resa disponibile che prende il cliente dall'input permette ad A di agire su B. Correzione: in FR-37 una conseguenza come quella di FR-9: "la documentazione per l'integratore spiega di ricavare il cliente dal contesto dell'esecuzione, mai dall'input". |
| A4 | CHIUSA | FR-26 (risoluzione tra le risorse del proprietario, segreti dal cliente dell'esecuzione, trigger con cliente fissato alla creazione), FR-29 (test del nome omonimo) | — |
| A5 | CHIUSA | FR-26 (workflow e worker scrivono solo sul cliente dell'esecuzione o dell'attività; errore da stream inesistente, nessun evento arriva) | — |
| A6 | PARZIALE | FR-16 («Il trattamento dei byte di un nodo caricato (validazione, istanziazione) non raggiunge la chiave della cassaforte») | "Non raggiunge la chiave" è un esito non verificabile e lascia aperta la validazione nel processo della chiave con un parser "sicuro"; manca la compilazione, che è la superficie più grande. Correzione: "(validazione, compilazione, istanziazione) avviene fuori dal processo che custodisce la chiave", con la stessa formula usata due righe sopra per la sandbox. |
| A7 | CHIUSA | FR-50 (nessun valore predefinito noto, installazioni diverse, rifiuto dei valori d'esempio, solo localhost), FR-45 (credenziale generata al primo avvio) | — |
| A8 | CHIUSA | NFR-7 (contenuto dei clienti sempre come testo; test con payload di script su ogni campo, pagina e componente) | — |
| A9 | CHIUSA | FR-30 (quote su pool, eventi al secondo, richieste API, spazio; tetto dell'arretrato; errore prima dell'accettazione), NFR-6 (disco pieno) | — |
| A10 | CHIUSA | FR-34 (chiave fuori dal database e dalle copie, perdita documentata, rotazione senza fermo, chiave vecchia inutile) | La rotazione della chiave di firma delle identità (FR-27) resta fuori, come rinviato nel memlog (`review-edge-cases.md` M5); non faceva parte della correzione di A10. |

Di riflesso risultano chiuse anche M1 (FR-16: la lista del nodo restringe, mai allarga), M7 (FR-21), M10 (FR-18, FR-31) e M11 (NFR-14). M13 è chiusa in parte da FR-15 (versioni registrate per attività, ma niente blocco di un nodo). Le altre medie e basse sono rinviate come registrato nel memlog.
