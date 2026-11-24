#!/bin/sh
# Blocco A, seconda parte della sequenza di eliminazione (dopo la corsa con le soglie di scrittura dei segmenti):
# seconda modifica di configurazione di Iggy = ritardo del group commit del WAL a 5.000 µs (il predefinito delle misure è 500),
# poi il motore senza archivio e il motore completo con la configurazione di base.
. ~/ttspikebus/env.sh
# aspetta la corsa in corso (PID passato come primo argomento)
[ -n "$1" ] && while kill -0 "$1" 2>/dev/null; do sleep 5; done
IGGY_GC_DELAY=5000 CFG=gc5000 ./a1-run.sh bench 1
./a1-run.sh noarc 1
./a1-run.sh full 1
echo CHAIN2-FINITO
