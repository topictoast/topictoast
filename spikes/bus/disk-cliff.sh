#!/bin/sh
# Blocco A, ipotesi «il disco rallenta dopo molte scritture continue»: scrive N GB a blocchi da 1 GB in un file nostro
# (directory dati dello spike, btrfs come i dati di Iggy), con O_DIRECT, e stampa la velocità di ogni blocco.
# ./disk-cliff.sh [N=45]. Il file si cancella alla fine. Nessun altro file, nessun container toccato.
N=${1:-45}
D=${SPIKE_DATA:-$HOME/.cache/ttspikebus-data}
mkdir -p "$D"
F=$D/cliff.tmp
rm -f "$F"
echo "disk-cliff: $N blocchi da 1 GB su $F"
echo "io prima: $(head -1 /proc/pressure/io)"
i=0
while [ $i -lt $N ]; do
  r=$(dd if=/dev/zero of="$F" bs=1M count=1000 seek=$((i*1000)) conv=notrunc oflag=direct 2>&1 | tail -1)
  echo "blocco $((i+1)) alle $(date +%T): $r | util disco ticks=$(awk '$3=="nvme0n1"{print $13}' /proc/diskstats)"
  i=$((i+1))
done
echo "io dopo: $(head -1 /proc/pressure/io)"
rm -f "$F"
echo "FINITO disk-cliff"
