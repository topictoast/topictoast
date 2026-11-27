#!/bin/bash
# Una riga delle prove su minisheep: ./ms-row.sh <RIGA> <n|smoke> [seme]
#   RIGA: V1 V2 V3 A1 A2 C1..C7 C5REAL I1 T1 T1FREE T2 T3 M1 S1
#   n:    numero della corsa misurata -> risultati/stream-<riga>-<n>-minisheep.txt (con la sua coppia `-ctrl-` per V2, V3, A2)
#   smoke: corsa di prova a durata ridotta -> risultati/stream-smoke-<riga>-minisheep.txt (T1 con `t1full`: 10 minuti interi)
# Prima di ogni corsa il cancello di quiete (carico 1 min < 1,5, pressione I/O some avg10 < 1 %, fino a 15 minuti), poi lo
# stato prima e dopo (carico, pressione, nomi di TUTTI i container in esecuzione, sha256 del binario, core, memoria, disco).
# I semi delle corse misurate sono scritti QUI, prima di ogni corsa (regola di lettura: «semi diversi scritti prima»).
cd "$(dirname "$0")" || exit 1
. ./env.sh
ROW=$(echo "$1" | tr 'a-z' 'A-Z'); N=$2
declare -A SEED=(
  [V1-1]=4101 [V2-1]=4201 [V3-1]=4301
  [A1-1]=5101 [A1-2]=5102 [A1-3]=5103
  [A2-1]=5201 [A2-2]=5202 [A2-3]=5203
  [C1-1]=6101 [C1-2]=6102 [C1-3]=6103
  [C2-1]=6201 [C2-2]=6202 [C2-3]=6203
  [C3-1]=6301 [C3-2]=6302 [C3-3]=6303
  [C4-1]=6401 [C4-2]=6402 [C4-3]=6403
  [C5-1]=6501 [C5-2]=6502 [C5-3]=6503
  [C5REAL-1]=6551 [C5REAL-2]=6552 [C5REAL-3]=6553
  [C6-1]=6601 [C6-2]=6602 [C6-3]=6603
  [C7-1]=6701 [C7-2]=6702 [C7-3]=6703
  [I1-1]=7101
  [T1-1]=8101 [T1-2]=8102 [T1-3]=8103
  [T1FREE-1]=8201
  [T2-1]=8301 [T2-2]=8302
  [T3-1]=8401
  [M1-1]=8501
  [S1-1]=8601 [S1-2]=8602
)
if [ "$N" = smoke ]; then
  S=${3:-91}; export PREFIX=stream-smoke; unset NUM
  export KIND=smoke
else
  S=${3:-${SEED[$ROW-$N]}}; export PREFIX=stream; export NUM=$N; export KIND=misura
  # tetto di attesa del check (25 minuti) per lo svuotamento delle righe in consegna; gli avvii escono a gruppi da 1.000
  # righe in una transazione (un fsync per gruppo)
  export WAIT=${WAIT:-1500}
  [ -n "$S" ] || { echo "nessun seme scritto per $ROW-$N"; exit 1; }
fi
LOWER=$(echo "$ROW" | tr 'A-Z' 'a-z')
run() { # id-in-smoke2 nome  [variabili d'ambiente per la durata]
  NAME_OVERRIDE=$2 ./smoke2.sh "$1" "$S"
}
if [ "$N" = smoke ]; then
  export A1_N=${A1_N:-30} A1_VAL=${A1_VAL:-10} I1_PAIRS=${I1_PAIRS:-1} I1_DUR=${I1_DUR:-120}
  export T1_DUR=${T1_DUR:-60} T2_DUR=${T2_DUR:-120} T2_K1=${T2_K1:-40} T2_K2=${T2_K2:-80} T3_DUR=${T3_DUR:-60} M1_REST=${M1_REST:-20} M1_DUR=${M1_DUR:-40}
  export PHASE=${PHASE:-60} WAIT=${WAIT:-420}
fi
case $ROW in
  V1) run v1 "$LOWER" ;;
  V2) run v2 v2; run v2ctrl v2-ctrl ;;
  V3) run v3 v3; run v3ctrl v3-ctrl ;;
  A1) run a1 a1 ;;
  A2) run a2 a2; run a2ctrl a2-ctrl ;;
  C1) run c1f c1 ;;
  C2) run c2f c2 ;;
  C3) run c3pair c3 ;;
  C4) run c4pair c4 ;;
  C5) run c5 c5 ;;
  C5REAL) run c5real c5real ;;
  C6) run c6 c6 ;;
  C7) run c7 c7 ;;
  I1) run i1 i1 ;;
  T1) run t1 t1 ;;
  RC) run rc rc ;;
  T1FULL) unset PHASE T1_DUR WAIT; export PREFIX=stream-smoke; unset NUM; export KIND=smoke; run t1 t1full ;;
  T1FREE) run t1free t1-free ;;
  T2) run t2 t2 ;;
  T3) run t3 t3 ;;
  M1) run m1 m1 ;;
  S1) run s1 s1 ;;
  *) echo "riga sconosciuta: $ROW"; exit 1 ;;
esac
