#!/bin/sh
# Fase 3, corse a coppie: ./ms-pair.sh <scenario> <rep da> <rep a> [chiave=valore ...]
# gRPC e iroh una dopo l'altra, Iggy, SQLite; l'ordine si inverte a ogni ripetizione (grpc-iroh, iroh-grpc, ...).
# Il pavimento di fdatasync è registrato da ms-run.sh prima e dopo ogni corsa. File: iggy-sqlite-<scenario>-<link>-3-ms-r<n>.txt
. ~/ttspikebus/env.sh
S=$1; A=$2; Z=$3; shift 3
r=$A
while [ $r -le $Z ]; do
  if [ $((r % 2)) = 1 ]; then O="grpc iroh"; else O="iroh grpc"; fi
  for l in $O; do
    f=risultati/iggy-sqlite-$S-$l-3-ms-r$r.txt
    LINK=$l ./ms-run.sh $f ./scen.sh iggy sqlite $S "$@" > /dev/null
    echo "$S $l r$r: $(grep -m1 '^activities_per_s' $f | cut -c1-30) | $(grep -m1 '^start_sent' $f | cut -c1-70) | $(grep -m1 'cpu_per_1000' $f | cut -c1-60)"
  done
  r=$((r+1))
done
