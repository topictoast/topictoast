#!/bin/sh
# Fase 3, passo 0: impostazione del motore con Iggy. 3 coppie alternate di accettazione e avvio a 1.000 att./s (SQLite):
# a = come la 2c (KGROUPS=1 MAXB=256), b = come la 2d (4 / 1024 / 64). L'ordine a/b si inverte a ogni ripetizione.
. ~/ttspikebus/env.sh
for rep in 1 2 3; do
  case $rep in 2) O="b a" ;; *) O="a b" ;; esac
  for row in accept start1k; do
    for s in $O; do
      case $s in a) K=1; M=256 ;; b) K=4; M=1024 ;; esac
      f=risultati/iggy-sqlite-$row-s0$s-3-ms-r$rep.txt
      extra=""; [ $row = start1k ] && extra="secs=25"
      KGROUPS=$K MAXB=$M ./ms-run.sh $f ./scen.sh iggy sqlite $row $extra > /dev/null
      echo "$row $s r$rep: $(grep -E '^(accept_engine|start_ready )' $f | head -1 | cut -c1-90)"
    done
  done
done
