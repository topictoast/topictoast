#!/bin/sh
# Blocco A, conservazione e aggiornamento: ./a23-chain.sh. Una corsa alla volta; ogni file ha il pavimento di fsync prima e dopo.
. ~/ttspikebus/env.sh
# stesso ritmo di byte e stesse dimensioni di richiesta del motore (197 eventi per richiesta, 103 richieste/s)
CFG=b204 ./a1-run.sh bench 1 batch=204
GATE60=0 ./ms-run.sh risultati/iggy-compat-4-ms-r1.txt ./compat.sh > /dev/null
echo "FINITO compat"
for r in 1 2 3; do
  GATE60=0 ./ms-run.sh risultati/iggy-sqlite-retproof-4-ms-r$r.txt ./ret-proof.sh > /dev/null
  echo "FINITO retproof r$r"
done
for r in 1 2 3; do
  IGGY_SEGMENT_SIZE=1048576 GATE60=0 ./ms-run.sh risultati/iggy-sqlite-ret-4-ms-r$r.txt ./scen.sh iggy sqlite ret RSECS=90 > /dev/null
  echo "FINITO ret r$r"
done
for r in 1 2 3; do
  CAP=4096 GATE60=0 ./ms-run.sh risultati/iggy-sqlite-upgrade-4-ms-r$r.txt ./up-run.sh > /dev/null
  echo "FINITO upgrade r$r"
done

echo CHAIN23-FINITO
