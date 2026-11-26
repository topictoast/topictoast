#!/bin/sh
# Fase 2d, corse misurate con il codice finale (raggruppamento minb): Iggy e Valkey alternati corsa per corsa.
# Righe da 10 minuti con io some avg60 < 2 % (GATE60=1); N per bus fissato in scen.sh (Valkey 4, Iggy 1), K=4, maxb=1024, minb=64.
# Le righe di controllo hanno il numero di corse della 2c perché la corsa di controllo della prima versione aveva il p99 oltre 25 ms.
. ~/ttspikebus/env.sh
export TAG=-d GATE60=1
./ms-chain.sh thr10k sqlite 1 2 secs=600
./ms-chain.sh thr10k pg 1 1 secs=600
export GATE60=0
./ms-chain.sh accept sqlite 1 5
./ms-chain.sh accept pg 1 2
./ms-chain.sh start1k sqlite 1 5 secs=25
./ms-chain.sh start1k pg 1 2 secs=25
./ms-chain.sh join sqlite 1 3
./ms-chain.sh iso sqlite 1 3
./ms-chain.sh rss sqlite 1 1
./ms-chain.sh kill sqlite 1 1
echo FINITO
