---
title: topictoast — smistamento dei meccanismi per l'architettura
created: 2026-11-20
fonti: extract-prd.md (M-01…M-49), extract-context.md, research-ows.md
---

# Smistamento dei meccanismi

Test della spine: due unità costruite indipendentemente potrebbero scegliere in modo
incompatibile, la scelta non è ovvia ed è un vero compromesso → AD. Altrimenti convenzione,
rinvio o eredità.

## A. Ereditati dal PRD (registrati, non si richiedono)

Motore Rust a nodo singolo, ripresa ≤ 30 s; bus come servizio separato (FR-50 «due servizi»);
database SQLite incorporato o Postgres; attività sul bus, database solo stato; worker =
processi dell'integratore sempre attivi, anche remoti su TLS (NFR-8); sottoinsieme OWS 1.1 con
estensione DAG, salvato valido secondo lo schema di topictoast (FR-40); sandbox su due livelli
fuori dal processo della chiave (FR-16); tenant opzionali con predefinito; identità firmata dalla
piattaforma ospite; cassaforte con sole chiavi API; destinazioni di rete per tenant; niente
Kubernetes, KVM né root nel predefinito; Apache-2.0.

## B. Li decide uno spike: l'architettura fissa porta e criteri

| Spike | Meccanismi | Criteri (NFR) |
|---|---|---|
| Bus | M-11, M-12, M-17 (aggiornamento del bus), M-18, M-24, T-66 | NFR-1, NFR-2, NFR-3, NFR-4, NFR-6, NFR-15, SM-2, FR-13 |
| Sandbox | M-25, M-26, M-27, M-28 (freddo) | NFR-12, NFR-1, FR-16, FR-17, test con codice ostile (NFR-8) |
| Pipeline stream | M-31, M-32, M-33, M-34, M-35, T-56 | NFR-5, FR-19…FR-25, UJ-3 reale |
| Base | M-49 | NFR-1…NFR-4 sul primo scheletro |

## C. Scelte portanti da portare a Marco

1. Verità tra bus e database: dove è "accettato" e in che ordine si riprende (FR-9, NFR-4,
   NFR-6, NFR-15).
2. Forma dell'estensione DAG: `metadata` del task o chiavi native (M-01, M-02; FR-40 lo
   permette).
3. Parità di validazione nucleo JS ↔ motore: una sola implementazione Rust compilata in Wasm, o
   due con una suite comune (FR-39, FR-4).
4. Lettura della costituzione V: cosa è tipizzato e cosa resta documento opaco; SQLite senza RLS.
5. Regola di propagazione di fallito e saltato (M-03, §11 n.8).
6. Linguaggio degli schemi (M-05, §11 n.16).
7. Trasporto dei worker: gRPC su TLS o iroh (M-13, M-14).
8. Dove girano jq e `call http` dei tenant (M-28, M-29, M-30, AMB-11).
9. Confine degli scenari BDD e layout del repo per kit, nucleo JS e SDK TS (CLAUDE.md da
   aggiornare nella PR).
10. Bloccanti: caso reale di UJ-3, librerie candidate di Marco (M-48), ordine e sede degli
    spike.

## D. Convenzioni proposte in blocco (una sola approvazione)

M-16 identificativo di tentativo; M-17 versione del protocollo; M-20 chiave di scadenza; M-21
chiave riga → avvio; M-22 controllo ottimistico e impronta del nodo; M-15 credenziali dei worker;
M-36…M-39 cassaforte (cifratura a busta, chiave per tenant, cancellazione crittografica); M-40,
M-41 TLS e chiavi generate; M-42, M-43 formato della credenziale; canale in tempo reale;
formato di metriche e log (FR-52); forma degli errori (codice + parametri, percorso).

## E. Rinviati alla spec della storia

M-06, M-07, M-08, M-09, M-10 (costo misurato nelle corse), M-19, M-23, M-44, M-45, M-46, M-47,
T-57, T-59, T-60, T-63, T-65, T-67, T-68.
