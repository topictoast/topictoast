# Revisione delle versioni, aggiornamento della spine dopo S-2 (sandbox)

Data della verifica: 2026-11-26. Ambito: differenze non committate di `ARCHITECTURE-SPINE.md` (Stack, licenze, S-2, AD-6), con `docs/decisions/2026-11-26-sandbox.md` e `spikes/sandbox/fase*-risultati.md`. Sola lettura, nessun comando di costruzione. Fonti: crates.io `/api/v1/crates/<crate>/versions`, GitHub releases e advisory, OSV (`api.osv.dev`), raw dei repository alle etichette citate.

## Verdetto

Le cinque versioni dello Stack sono reali, nella finestra delle due settimane o coperte dall'eccezione di sicurezza, con licenze ammesse e senza avvisi aperti. Nessun blocco. Restano due lacune da chiudere prima dell'accettazione (kernel del guest nell'elenco di supporto scaduto; `javy-plugin-api` e compilatore assenti dallo Stack) e alcune precisazioni.

## Esito per voce

| Voce | Dichiarato | Verificato | Esito |
|---|---|---|---|
| wasmtime, wasmtime-wasi | 50.0.3 | pubblicata 2026-11-19T17:15Z (7 giorni, sotto la soglia: serve l'eccezione); non yankata; Apache-2.0 WITH LLVM-exception. OSV: 0 avvisi su 50.0.2, 50.0.3 e 51.0.0; 8 avvisi (RUSTSEC-2026-0566…0573) su 50.0.1, tutti corretti in 50.0.2 (OSV, campo `fixed`); RUSTSEC-2026-0587 corretto in 50.0.3 (pubblicato 2026-11-19) | confermato |
| javy | 9.1.0 | release GitHub v9.1.0 del 2026-07-30 (119 giorni); Cargo.toml `Apache-2.0 WITH LLVM-exception`; nessun avviso OSV; il crate `javy` è alla 8.1.0, il 9.1.0 è la CLI | confermato, con precisazione (F-3) |
| Firecracker | v1.18.0 | release 2026-11-09 (17 giorni, fissabile senza eccezione); Apache-2.0; ultima release; gli avvisi GHSA pubblicati (GHSA-48ww-rqpp-2mgm, GHSA-776c-mpj7-jm3r, GHSA-36j2-f825-qvgc) sono tutti corretti in versioni precedenti | confermato |
| Firecracker su host 7.2 | «parte sul kernel 7.2, non nell'elenco» | `kernel-policy.md` a v1.18.0: host 5.10, 6.1, 6.18, e basta. Spike S8: host 7.2.4, 3 corse su 3, avvio a freddo 52,6-54,0 ms, ripristino 6,4-8,8 ms (`fase3-risultati.md:205-212`). Misurato, non garantito dal progetto: l'ADR lo dice (`2026-11-26-sandbox.md:140`, `168`) | confermato |
| landlock | 0.4.7 | crates.io: 2026-07-27 (122 giorni), `MIT OR Apache-2.0`, nessun avviso. Il sorgente a v0.4.7 dichiara «ABI v9» (`src/lib.rs:33`); lo spike misura `effective=V9 kernel=10` (`fase4-risultati.md:36-38`). Esiste la 0.4.8 (2026-11-20, ABI v10): ha 6 giorni, non fissabile | confermato (F-6) |
| seccompiler | 0.5.0 | crates.io: 2025-03-07, `Apache-2.0 OR BSD-3-Clause`, nessun avviso | confermato |
| libseccomp «LGPL» | LGPL | `LICENSE` del repository: GNU LGPL 2.1 | confermato |
| Kernel guest GPL-2.0 | eccezione | corretto che sia GPL-2.0; vedi F-2 sul perimetro | confermato, con riserva |

## Risultati

### F-1 (media) Kernel del guest: la 6.1 usata dallo spike ha il fine supporto minimo già passato
- **Dove:** spine, AD-6 «Sandbox forte» e Stack (nessuna versione del kernel del guest); `fase3-risultati.md:8` (guest `vmlinux-6.1.155`) e `:207`.
- **Prova:** `kernel-policy.md` v1.18.0, tabella del guest: v6.1 «Min. end of support 2026-09-02» (già superata il 2026-11-26), v6.18 2028-06-01. Fonte: https://raw.githubusercontent.com/firecracker-microvm/firecracker/v1.18.0/docs/kernel-policy.md
- **Perché conta:** la spine e l'ADR dicono che topictoast distribuisce e mantiene il kernel del guest, e che le sue correzioni di sicurezza sono a carico di topictoast. Ma nessuno dei due fissa il ramo. Le misure valgono per la 6.1.155, che è il ramo con la data minima di supporto scaduta.
- **Correzione:** scrivere nell'ADR e nello Stack il ramo del kernel del guest (proposta: 6.18, con la data minima 2028-06-01), e ripetere S1-S6 e S9 sul 6.18 prima della prima storia di FR-54, oppure dichiarare nelle Open Questions che le misure sono sulla 6.1.

### F-2 (media) Eccezione di licenza senza versione, e perimetro più largo del kernel
- **Dove:** spine, sezione Licenze (riga aggiunta); ADR punto 9 (`:86-88`) e punti aperti (`:170`).
- **Prova:** la regola della spine dice «Un'eccezione vale per un pacchetto e una versione, richiede un ADR»; la nuova eccezione non nomina né pacchetto Linux né versione. Lo spike ha usato il disco `ubuntu-24.04.squashfs` della CI di Firecracker e un agente collegato in modo statico a glibc (`2026-11-26-sandbox.md:170`): l'immagine del guest contiene molto più codice GPL e LGPL del solo kernel, che l'eccezione non copre.
- **Correzione:** nominare nell'eccezione «Linux, ramo X.Y, GPL-2.0» (versione dopo F-1); aggiungere che il disco del guest di produzione deve contenere solo software con licenze ammesse o a sua volta elencato, e che l'artefatto distribuisce con il sorgente corrispondente (come l'ADR già dice per il kernel). Il punto «musl non provato» resta il blocco per la prima storia di FR-54, come già scritto.

### F-3 (media) Strumenti di compilazione di javy non fissati nello Stack
- **Dove:** spine, Stack riga javy; `fase2-risultati.md:8`, `:162`.
- **Prova:** lo Stack elenca «javy 9.1.0» (la CLI). Lo spike per far chiamare l'host a javy ha costruito un plugin con `javy-plugin-api 7.1.0` (crates.io: 2026-07-30, `Apache-2.0 WITH LLVM-exception`, nessun avviso OSV) e un `wasi-sdk 24` scaricato dallo script di costruzione di `rquickjs-sys`. Nessuno dei due è nello Stack; il secondo è un download senza versione né impronta nel perimetro dei lockfile.
- **Correzione:** aggiungere `javy-plugin-api` 7.1.0 allo Stack (o scrivere che il plugin è fuori dai lockfile del prodotto) e annotare che il `wasi-sdk` va fissato per versione e impronta quando il plugin entra nel prodotto. Aggiungere nella riga javy la precisazione «CLI; il crate `javy` è alla 8.1.0».

### F-4 (bassa) RUSTSEC-2026-0587 non tocca Linux
- **Dove:** spine, Stack riga wasmtime; ADR punto 2.
- **Prova:** OSV, RUSTSEC-2026-0587: «WASI filesystem sandbox escape on Windows». Il prodotto gira su Linux. La 50.0.3 è comunque giustificata dagli otto avvisi 0566…0573 (tra cui 0566, crescita illimitata della memoria dell'host in wasmtime-wasi, rilevante per i tetti di memoria della sandbox).
- **Correzione:** nessuna sul numero di versione; aggiungere «0587 solo Windows» nella frase dello Stack per non far credere che la 50.0.3 serva a chiudere una fuga su Linux.

### F-5 (bassa) Wasmtime 51.0.0 non è citato
- **Dove:** spine, Stack; ADR «alternative».
- **Prova:** wasmtime e wasmtime-wasi 51.0.0 pubblicati il 2026-11-20 (6 giorni; fissabile dal 2026-12-04 se non serve un avviso). Il ramo 50 riceverà le correzioni, ma resta da dire quando si passa alla 51.
- **Correzione:** una riga nell'ADR: «la 51.0.0 non è fissabile prima del 2026-12-04; la lista seccomp (26 chiamate, `fase4-risultati.md:255`) va rimisurata a ogni salto di versione maggiore», coerente con l'ADR (`:129`).

### F-6 (bassa) Landlock 0.4.8 porta la V10
- **Dove:** ADR (`:130`, `:164`), spine, Stack.
- **Prova:** il sorgente della 0.4.8 dichiara «ABI v10» (`src/lib.rs:33` a v0.4.8; release 2026-11-20). L'ADR scrive che la V10 non è applicata dal crate fissato, «ipotesi, non provata»: l'ipotesi è in realtà confermata dal sorgente della 0.4.7 (V9) e dalla misura di `effective=V9` (`fase4-risultati.md:36-38`); non è provato che la V10 chiuda UDP e a cosa serva.
- **Correzione:** riformulare in «misurato: la 0.4.7 applica fino alla V9; la 0.4.8 (2026-11-20) arriva alla V10, fissabile dal 2026-12-04, da provare per la parte UDP». Lascia aperta la scelta.

### F-7 (bassa) ADR cita una versione di ComponentizeJS diversa da `research-versions.md`
- **Dove:** ADR, alternative scartate (`:125`-`126`); `research-versions.md:131` (0.23.0, 2026-09-21).
- **Prova:** npm: 0.24.0 del 2026-11-25. L'ADR è corretto (data 2026-11-25); `research-versions.md` è fermo alla 0.23.0. Non è un errore della spine.
- **Correzione:** aggiornare la riga di `research-versions.md` quando si chiude l'aggiornamento della spine.

### F-8 (bassa) Licenze delle dipendenze transitive non verificate
- **Dove:** spine, sezione Licenze.
- **Prova:** ho verificato la licenza dichiarata solo dei cinque crate/strumenti dello Stack (e `javy-plugin-api`). L'albero transitivo di wasmtime-wasi non è stato controllato (nessuna costruzione, nessun `cargo deny`); `spikes/` è fuori dal perimetro dei lockfile per regola.
- **Correzione:** nessuna ora; il controllo spetta al gate (`cargo deny`) alla prima storia che porta wasmtime nel motore, dove una licenza fuori elenco (per esempio MPL-2.0 modificata o LGPL) richiede un ADR.

## Cosa è confermato dalla verifica

- Le date dello Stack sono corrette: wasmtime 50.0.3 (2026-11-19) rientra solo per l'eccezione di sicurezza, che nei fatti sussiste (OSV); Firecracker v1.18.0 (17 giorni), javy 9.1.0, landlock 0.4.7 e seccompiler 0.5.0 superano la soglia delle due settimane.
- La spine concorda con gli spike: kernel dell'host 7.2.4 (S8), ABI Landlock 10 del kernel contro V9 del crate, `/dev/mem` e `/proc/kcore` apribili nel kernel di CI (`fase3-risultati.md:158`, `:256`), spike del bus 26,5 MB.
- Il criterio mancato «p99 +3,1 ms» e le soglie NFR-12 sono riportati nella spine come nell'ADR.
