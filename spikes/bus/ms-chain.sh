#!/bin/sh
# ./ms-chain.sh <scenario> <db> <rep da> <rep a> [chiave=valore]: alterna iggy e valkey corsa per corsa
. ~/ttspikebus/env.sh
S=$1; D=$2; A=$3; Z=$4; shift 4
r=$A
while [ $r -le $Z ]; do
  for b in iggy valkey; do
    ./ms-run.sh risultati/$b-$D-$S$TAG-ms-r$r.txt ./scen.sh $b $D $S "$@" > /dev/null
    echo "$b $D $S r$r: $(grep -E '^(accept_engine|start_ready |start_ready_tenant_b)' risultati/$b-$D-$S$TAG-ms-r$r.txt | head -1)"
  done
  r=$((r+1))
done
