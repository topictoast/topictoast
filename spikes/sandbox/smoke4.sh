#!/bin/sh
# Corse di prova (smoke) della fase 4, con parametri ridotti: non contano per i verdetti.
# ./smoke4.sh <riga>   (riga = A1|A2|A3|A4|A5|A6|A7|A9)
cd "$(dirname "$0")" || exit 1
case $1 in
A1) ./run4.sh A1 smoke a1 ;;
A2) ./run4.sh A2 smoke a2 roles=jq,callhttp modes=none,full ;;
A3) ./run4.sh A3 smoke a3 ;;
A4) ./run4.sh A4 smoke a4 ;;
A5) ./run4.sh A5 smoke a5 tenants=3 idle_s=5 ;;
A6) ./run4.sh A6 smoke a6 n=10 ;;
A7) ./run4.sh A7 smoke a7 secs=3 phases=ctrl:0:10000,idle:1:10000 ;;
A9) ./run4.sh A9 smoke a9 reps=5 ;;
esac
