#!/bin/sh
# Blocco B, prova del client `iggy` 0.12.0 su un crash del server: ./kill-probe.sh
# Iggy normale (nessuna zavorra), `bus-probe` scrive ogni 100 ms; a 8 s `docker kill`, a 12 s `docker start`.
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
BIN=${SPIKE_BIN:?SPIKE_BIN}
C=ttspikebus-iggy-1
./reset.sh iggy
mkdir -p out
"$BIN" bus-probe size=1024 batch=1 period_ms=100 secs=40 > out/probe2.txt 2>&1 &
PP=$!
sleep 8
echo "kill $(date +%s.%N)"; docker kill $C >/dev/null
sleep 4
echo "start $(date +%s.%N)"; docker start $C >/dev/null
wait $PP
awk '$3=="ERR"{e++; if(!f)f=$0; l=$0} $3=="ok"{o++} END{print "ok="o, "err="e; print "primo: " f; print "ultimo: " l}' out/probe2.txt
cut -c1-200 out/probe2.txt | awk '$3=="ERR"' | cut -d' ' -f3- | sort | uniq -c | sort -rn | head -5
grep -E "^P" out/probe2.txt | awk '{ if ($3!=p) {print; p=$3} }' | head -8
tail -1 out/probe2.txt
docker compose -p ttspikebus down >/dev/null 2>&1
