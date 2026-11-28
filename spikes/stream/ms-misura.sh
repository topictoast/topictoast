#!/bin/bash
# Corse misurate dello spike S-3 su minisheep, nell'ordine delle regole di lettura (emendamento del 2026-11-27).
# Si lancia staccato dalla sessione principale (Marco, memlog 182): ./ms-misura.sh parte1 | parte2
# parte1: V1 V2 V3, A2 e A1 (3 corse), C1-C7 e C5REAL (3 corse ciascuna). Poi la sessione principale legge C1 e C4 e
# applica la condizione di arresto prima di lanciare parte2: I1, T1 (3) e T1 senza tetto, T2-1 S1-1 T2-2 S1-2, T3, M1.
# Nessuna variabile di prova (RUNTAG, PHASE, durate): valgono i valori delle corse misurate di ms-row.sh.
cd "$(dirname "$0")" || exit 1
unset RUNTAG PHASE PHASE_S T1_DUR T2_DUR T2_K1 T2_K2 T3_DUR I1_PAIRS I1_DUR A1_N A1_VAL M1_REST M1_DUR C4_EVENTS
case $1 in
  parte1)
    ./ms-driver.sh 1 V1 V2 V3
    for r in A2 A1 C1 C2 C3 C4 C5 C5REAL C6 C7; do for n in 1 2 3; do ./ms-driver.sh "$n" "$r"; done; done ;;
  parte2)
    ./ms-driver.sh 1 I1
    for n in 1 2 3; do ./ms-driver.sh "$n" T1; done
    ./ms-driver.sh 1 T1FREE
    ./ms-driver.sh 1 T2; ./ms-driver.sh 1 S1; ./ms-driver.sh 2 T2; ./ms-driver.sh 2 S1
    ./ms-driver.sh 1 T3 M1 ;;
  *) echo "uso: $0 parte1|parte2"; exit 1 ;;
esac
echo "$(date +%T) MISURA $1 FINITA" >> risultati/driver.log
