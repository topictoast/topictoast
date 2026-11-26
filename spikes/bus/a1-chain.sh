#!/bin/sh
# Blocco A, sequenza di eliminazione degli stalli (una corsa alla volta, nessuna in parallelo):
# prova del disco da solo, Iggy con le due modifiche di configurazione (una per corsa), motore senza archivio, motore completo.
. ~/ttspikebus/env.sh
GATE60=0 ./ms-run.sh risultati/disk-cliff-4-ms-r1.txt ./disk-cliff.sh 45 > /dev/null
echo "FINITO disk-cliff"
IGGY_SAVE_BYTES=16777216 IGGY_SAVE_MSGS=65536 CFG=flush16m ./a1-run.sh bench 1
IGGY_PREALLOC=1 CFG=prealloc ./a1-run.sh bench 1
./a1-run.sh noarc 1
./a1-run.sh full 1
echo CHAIN-FINITO
