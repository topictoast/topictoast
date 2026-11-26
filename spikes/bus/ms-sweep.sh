#!/bin/sh
# Fase 2d, verifica di N: ./ms-sweep.sh <rep>. thrmax da 30 s, N in 1 2 4 8, Iggy e Valkey alternati.
# L'ordine di N ruota con la ripetizione (1 2 4 8, 8 4 2 1, 2 8 1 4): il disco di minisheep rallenta nel tempo
# e un ordine fisso confonderebbe N con la deriva (è quello che è successo nella 2c).
. ~/ttspikebus/env.sh
case $1 in 1) O="1 2 4 8" ;; 2) O="8 4 2 1" ;; *) O="2 8 1 4" ;; esac
for n in $O; do
  for b in iggy valkey; do
    f=risultati/$b-sqlite-thrmax-n$n-d-ms-r$1.txt
    WCONN=$n ./ms-run.sh $f ./scen.sh $b sqlite thrmax secs=30 > /dev/null
    echo "$b N=$n r$1: $(grep -E '^activities_per_s' $f | cut -c1-24) $(grep -E '^append ' $f | cut -c1-60)"
  done
done
