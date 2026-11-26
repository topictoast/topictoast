#!/bin/sh
# Corse misurate della fase 3: ./chain3.sh <A|B> <n>   (n = numero della ripetizione: 1, 2, 3, ...)
# Ogni riga è una corsa a sé, con il suo file grezzo (run3.sh -> ms-run.sh): nessuna corsa si scarta.
cd "$(dirname "$0")" || exit 1
G=$1
N=$2
export SHOW=1
case $G in
A)
  ./run3.sh S8 "$N" s8
  ./run3.sh S1 "$N" s1
  ./run3.sh S2 "$N" s2
  ./run3.sh S3 "$N" s3
  ./run3.sh S4 "$N" s4
  ;;
B)
  ./run3.sh S9 "$N" s9
  ./run3.sh S7 "$N" s7
  ./run3.sh S6 "$N" s6
  ./run3.sh S5 "$N" s5
  ;;
esac
