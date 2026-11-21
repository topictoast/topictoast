---
title: topictoast — secondo blocco di decisioni dai revisori
created: 2026-11-21
stato: approvato da Marco il 2026-11-21 (memlog voce 63)
fonti: reviews/review-rubric.md, review-adversarial.md, review-security.md, review-versions.md
---

# Secondo blocco di decisioni dai revisori

Queste correzioni cambiano qualcosa che Marco ha già approvato, oppure hanno un costo. Per questo
non entrano nella spine senza il suo sì. Le altre correzioni dei revisori rendono applicabile una
decisione già presa e sono già nella spine, oppure sono diventate criteri degli spike.

## Proposte

- **E-1 Identità del tenant** (adversarial R10, rubrica 5). Ogni tenant ha un identificativo
  interno UUIDv7. Il campo `tenant` del JWT è un nome esterno che il motore traduce nel suo
  identificativo; `default` è riservato al tenant predefinito. Né i nomi né i valori del JWT
  finiscono mai in percorsi, nomi di topic o chiavi. L'integratore è un proprietario con un
  identificativo interno riservato e una propria chiave, e segue lo stesso filtro di AD-5.
- **E-2 Controlli del JWT, estensione di C-15** (sicurezza A8, versioni 5). Sono ammessi solo gli
  algoritmi `EdDSA` e `Ed25519`, con chiave Ed25519 (RFC 9864). `iss` è configurato e verificato;
  `aud` è verificato per risorsa (risorse del kit contro risorse dell'SDK); `exp` è obbligatorio e il
  motore impone una durata massima configurabile, con un valore predefinito breve. Nella v1 non c'è
  revoca.
- **E-3 Schema di cifratura dei dati, estensione di C-12 e C-13** (sicurezza A6, rubrica 3). Ogni
  oggetto cifrato ha una busta versionata con l'id della chiave. Per ogni oggetto o segmento si
  deriva una chiave dalla chiave del tenant con HKDF, e si cifra con AES-256-GCM. Così non si
  raggiunge il limite dei nonce di AES-GCM, e la chiave del tenant può ruotare tramite l'id. Il
  formato è uno solo, nel dominio, e lo usano motore, proiettore, processo delle pipeline stream e
  archivio. S-1 ne misura il costo.
- **E-4 Payload nell'archivio appena applicati, cambio di AD-3** (adversarial R6). Il proiettore
  scrive ogni payload nell'archivio quando lo applica, non solo per lo storico. Le uscite a monte
  si leggono da lì, quindi il bus può scartare le posizioni già applicate anche mentre un'esecuzione
  lunga è viva.
- **E-5 Crate `crates/protocol`, cambio di AD-15** (adversarial R11). Un crate puro, senza I/O, con
  i tipi dell'API e del protocollo dei worker. Lo usano il motore e l'SDK Rust, ed è la sorgente
  dello schema OpenAPI. Così l'SDK non ridichiara i tipi a mano.
- **E-6 Convenzioni dell'API** (rubrica 7):
  - campi JSON in camelCase;
  - percorsi `/v1/…`;
  - paginazione a cursore con `cursor` e `limit`;
  - filtri come parametri di query;
  - chiave di idempotenza nell'header `Idempotency-Key`;
  - identità delle risorse con UUIDv7 (C-1);
  - posizione del bus come token opaco, nel campo `position` delle risposte e nell'header
    `Last-Event-ID`.
- **E-7 Costituzione IV** (rubrica 1). «Il client HTTP vive in un solo modulo per artefatto»
  (motore, SDK Rust), al posto di un solo modulo in tutto il workspace. È una PATCH.
- **E-8 Costituzione V riformulata** (riconciliazione R29). La PATCH 1.0.1 non vale solo per
  SQLite: con ogni database un solo adattatore del repository applica il filtro del tenant ed è
  coperto dal test con due tenant; con Postgres l'RLS è una seconda barriera. Va oltre la frase su
  SQLite che Marco aveva scelto.
- **E-9 Eccezione di sicurezza più larga** (versioni 2). Vale anche per i CVE dei progetti senza
  avviso RustSec o GHSA, e per le dipendenze transitive nei lockfile.

## Domande chiuse

- **D-1** (Marco, 2026-11-21): la variante 1 di S-1 è esclusa, qualunque siano i numeri, se manca anche una delle tre condizioni.
- **D-2** (Marco, 2026-11-21): Postgres 18 e 19, minime 18.7 e 19.0; immagine di test 18.7 fino al 2026-11-30, poi 19.x. Correzione: la 19.0 è stabile dal 2026-11-16, non in RC.
