#!/bin/sh
# Corse misurate di un gruppo di righe: ./chain.sh <A|B|C> <n>   (n = numero della ripetizione: 1, 2, 3, ...)
# Ogni riga è una corsa a sé, con il suo file grezzo (run.sh -> ms-run.sh): nessuna corsa si scarta.
cd "$(dirname "$0")" || exit 1
G=$1
N=$2
export SHOW=1
case $G in
A)
  ./run.sh L1 "$N" warm n=1000 warmup=100 cfgs=epoch,pool+epoch,nocow+epoch,fuel
  ./run.sh L2 "$N" cold n=200 cfgs=epoch,pool+epoch
  ./run.sh L3 "$N" mem ns=1,10,50 cfgs=epoch,pool+epoch
  ./run.sh L4 "$N" idle ns=1,5,20
  ./run.sh L9 "$N" l9
  ;;
B)
  ./run.sh L5L6 "$N" limits n=30 bgsecs=20
  ./run.sh L7 "$N" residue
  ./run.sh L8 "$N" sentinel
  ;;
C)
  ./run.sh L12 "$N" probe
  ./run.sh L10 "$N" compile reps=3
  ./run.sh L11 "$N" cache
  ;;
esac
