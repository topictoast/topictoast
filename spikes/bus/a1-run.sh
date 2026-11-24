#!/bin/sh
# Blocco A, stalli di Iggy a 10.000 att./s (10 minuti): ./a1-run.sh <bench|noarc|full> <rep> [chiave=valore ...]
# Un file per corsa in risultati/ (iggy-sqlite-stall-<variante>[-<CFG>]-4-ms-r<n>.txt), con pavimento di fsync prima e dopo
# (ms-run.sh); i campioni dell'I/O (righe T, DEV, CG, SL, DS) sono nello stesso file, la mappa id-nome dei container in <file>.io.names. CFG è solo un'etichetta.
. ~/ttspikebus/env.sh
V=$1; R=$2; shift 2
F=risultati/iggy-sqlite-stall-$V${CFG:+-$CFG}-4-ms-r$R.txt
export GATE60=1
case $V in
  bench) ./ms-run.sh $F ./with-sampler.sh ${F%.txt}.io ./bench-iggy.sh "$@" ;;
  noarc) DIAG_NOARCHIVE=1 ./ms-run.sh $F ./with-sampler.sh ${F%.txt}.io ./scen.sh iggy sqlite thr10k secs=600 "$@" ;;
  full) ./ms-run.sh $F ./with-sampler.sh ${F%.txt}.io ./scen.sh iggy sqlite thr10k secs=600 "$@" ;;
esac
echo "FINITO $F"
