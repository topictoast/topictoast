#!/bin/sh
# Blocco A, incidenza degli stalli con il motore completo: altre due corse da 10 minuti (r2, r3), dopo la sequenza precedente
# (PID passato come primo argomento). Con il campionamento dell'I/O di ogni cgroup, per attribuire un eventuale stallo.
. ~/ttspikebus/env.sh
[ -n "$1" ] && while kill -0 "$1" 2>/dev/null; do sleep 5; done
./a1-run.sh full 2
./a1-run.sh full 3
echo CHAIN3-FINITO
