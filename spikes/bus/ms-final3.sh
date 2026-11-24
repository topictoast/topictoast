#!/bin/sh
# Fase 3: corse misurate a coppie (gRPC poi iroh, Iggy, SQLite, motore della 2d: 4 gruppi, 1.024 messaggi, soglia 64).
# Le righe da 10 minuti hanno anche io some avg60 < 2 % (GATE60=1). Il pavimento è registrato prima e dopo ogni corsa.
. ~/ttspikebus/env.sh
export GATE60=0
./ms-pair.sh start1k 1 5 secs=25
./ms-pair.sh size64k 1 3
./ms-pair.sh size1m 1 3
./ms-pair.sh thrmax 1 3 secs=30
./ms-pair.sh iso 1 3
export GATE60=1
./ms-pair.sh thr10k 1 2 secs=600
echo FINITO
