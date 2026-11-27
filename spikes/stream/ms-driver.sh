#!/bin/bash
# Esegue di seguito piu' righe con ms-row.sh, in fondo (si lancia staccato su minisheep, perche' una sola corsa supera i 10
# minuti di un comando ssh in primo piano): ./ms-driver.sh <n|smoke> RIGA [RIGA...]
# Il PID e l'avanzamento stanno in risultati/driver.pid e risultati/driver.log; si ferma con `kill $(cat risultati/driver.pid)`
# (ferma il driver, non la riga in corso: la riga in corso la ferma il suo `stop_engine` o si lascia finire).
cd "$(dirname "$0")" || exit 1
. ./env.sh
N=$1; shift
echo $$ > risultati/driver.pid
export WAIT=${WAIT:-1500}
for r in "$@"; do
  echo "$(date +%T) inizio $r $N" >> risultati/driver.log
  ./ms-row.sh "$r" "$N" > "risultati/driver-$r.log" 2>&1
  echo "$(date +%T) fine $r $N exit=$?" >> risultati/driver.log
done
echo "$(date +%T) DRIVER FINITO" >> risultati/driver.log
