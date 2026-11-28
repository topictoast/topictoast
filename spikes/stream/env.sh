#!/bin/bash
# Ambiente delle corse su minisheep (spike S-3 parte 2b). Si usa con `. ./env.sh` da ~/ttspikestream.
# Core come nello spike del bus (env.sh di spikes/bus): servizi (Iggy) e processi del sistema sotto prova (motore e
# processi per tenant, che ereditano l'affinita') sui core 1-4, generatore e oracolo sui core 8-11. Cambiarli e'
# cambiare le condizioni di misura: i valori stanno nel file grezzo (riga «core assegnati» dello stato).
cd ~/ttspikestream || exit 1
export BIN=$HOME/ttspikestream/bin/streamspike
export SPIKE_DATA=$HOME/.cache/ttspikestream-data
export SPIKE_CPUSET=1-4 SPIKE_CPUS=1-4 SPIKE_LOADCPUS=8-11
export MACH=minisheep GATE=1 FINAL_DOWN=1
