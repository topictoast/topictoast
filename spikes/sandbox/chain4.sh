#!/bin/sh
# Corse misurate della fase 4: ./chain4.sh <n>   (n = numero della ripetizione: 1, 2, 3, ...)
# Ogni riga è una corsa a sé, con il suo file grezzo (run4.sh -> ms-run.sh): nessuna corsa si scarta.
cd "$(dirname "$0")" || exit 1
N=$1
export SHOW=1
./run4.sh A1 "$N" a1
./run4.sh A3 "$N" a3
./run4.sh A2 "$N" a2
./run4.sh A4 "$N" a4
./run4.sh A9 "$N" a9
./run4.sh A6 "$N" a6
./run4.sh A7 "$N" a7
./run4.sh A5 "$N" a5
