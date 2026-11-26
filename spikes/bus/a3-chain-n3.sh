#!/bin/sh
# Blocco A3, riversamento con esecuzioni a metà catena: tre passi da 3 s per esecuzione (334 esecuzioni/s = 1.002 attività/s),
# così lo stato vivo comprende anche eventi `Done` da riscrivere (con n=1 erano 0). Una corsa alla volta.
. ~/ttspikebus/env.sh
for r in 1 2 3; do
  CAP=4096 GATE60=0 ./ms-run.sh risultati/iggy-sqlite-upgrade-n3-4-ms-r$r.txt ./up-run.sh n=3 rate=334 > /dev/null
  echo "FINITO upgrade-n3 r$r"
done
echo CHAINN3-FINITO
