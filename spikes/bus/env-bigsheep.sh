cd ~/ttspikebus && export SPIKE_BIN=${ARM_BIN:-$HOME/ttspikebus/bin/busspike} SPIKE_DATA=$HOME/.cache/ttspikebus-data SPIKE_CPUS=1-4 SPIKE_CPUSET=1-4 SPIKE_LOADCPUS=5,6,7
# bigsheep (Ryzen 7 9700X, 8 core / 16 thread; CPU n e n+8 sono fratelli SMT): motore, worker e Iggy sui core fisici 1-4 (CPU 1-4, mai i fratelli 9-12);
# generatore di carico e campionatore sui core fisici 5-7 (CPU 5,6,7, fratelli 13-15 lasciati liberi)
