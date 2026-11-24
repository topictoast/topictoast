#!/bin/sh
# Blocco B: una corsa misurata staccata dalla shell, con attesa in primo piano di al più 9 minuti.
# ./brun.sh <file di uscita> <comando...>   (come ms-run.sh); se la corsa dura di più: ./brun.sh wait
# Una corsa alla volta: rifiuta di partire se ce n'è già una.
cd "$(dirname "$0")" || exit 1
. ./env.sh
if [ "$1" = wait ]; then
  PID=$(cat out/brun.pid 2>/dev/null)
else
  OUT=$1
  [ -f out/brun.pid ] && kill -0 "$(cat out/brun.pid)" 2>/dev/null && { echo "una corsa è già in corso"; exit 1; }
  mkdir -p out
  nohup ./ms-run.sh "$@" > out/brun.last 2>&1 &
  echo $! > out/brun.pid
  PID=$!
fi
i=0
while kill -0 "$PID" 2>/dev/null; do
  i=$((i+1)); [ $i -gt 108 ] && { echo "ANCORA IN CORSA pid=$PID"; exit 0; }
  sleep 5
done
echo "FINITA"; cat out/brun.last | cut -c1-300
