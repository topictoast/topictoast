#!/bin/sh
# Blocco E (E1), credenziale di sola lettura del bus: le 10 corse delle regole (iggy-sqlite-cred-regola-4-ms.txt), in ordine,
# staccato dalla shell: nohup ./cred-chain.sh rows > /dev/null 2>&1 &   (stato in chain-cred.log; ./cred-chain.sh wait [secondi])
# `./cred-chain.sh case <caso>` esegue un solo caso di `busauth` dopo un bus pulito (usato da `rows`).
cd "$(dirname "$0")" || exit 1
L=chain-cred.log
if [ "$1" = case ]; then
  set -a; . ./.env; set +a
  ./reset.sh iggy
  "$SPIKE_BIN" busauth case=$2
  exit 0
fi
if [ "$1" = wait ]; then
  i=0; until grep -q ALLDONE $L 2>/dev/null; do i=$((i+1)); [ $i -gt $(( ${2:-500} / 5 )) ] && { echo "ANCORA IN CORSA"; tail -3 $L; exit 0; }; sleep 5; done
  echo FINITA; tail -3 $L; exit 0
fi
# `rows`: prima serie (binario 45e4663); `redo`: ripetizione di control, ro-sent e sentinel con il binario corretto 818fa0d (v. l'aggiunta alle regole)
if [ "$1" = redo ]; then
  export ARM_BIN=$HOME/ttspikebus/bin/busspike-cred-818fa0d; CASES="control ro-sent"; SUF=-ripetizione
else
  export ARM_BIN=$HOME/ttspikebus/bin/busspike-cred-45e4663; CASES="control ro-a ro-a-s ro-sent"; SUF=
fi
. ./env.sh
: > $L
for n in 1 2; do
  for c in $CASES; do
    f=iggy-sqlite-cred-$c-4-ms-r$n$SUF.txt
    echo "$(date +%T) INIZIO $f" >> $L
    env ARM=CRED-$c ./ms-run.sh risultati/$f ./cred-chain.sh case $c > /dev/null 2>&1
    echo "$(date +%T) FINE $f: $(grep -c ' PASS ' risultati/$f) PASS, $(grep -c ' FAIL ' risultati/$f) FAIL" >> $L
  done
  f=iggy-sqlite-cred-sentinel-4-ms-r$n$SUF.txt
  echo "$(date +%T) INIZIO $f" >> $L
  env ARM=CRED-sentinel SENT=1 META=1 SENTUSER=1 ./ms-run.sh risultati/$f ./scen.sh iggy sqlite newtenant rate=100 n=10 secs=15 > /dev/null 2>&1
  echo "$(date +%T) FINE $f: $(grep -E '^(sentinel_stats|newtenant:)' risultati/$f | tr '\n' ' ' | cut -c1-250)" >> $L
done
echo "$(date +%T) ALLDONE" >> $L
