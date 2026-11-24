#!/bin/sh
# Blocco B, prova di fattibilità del disco pieno di Iggy: ./cap-probe.sh
# Iggy su un volume tmpfs di docker da 64 MB (mai il filesystem vero) con 40 MB di zavorra nello stesso volume
# (creata con `docker exec` nel container di Iggy: un volume tmpfs si monta di nuovo per ogni container, quindi
# solo l'exec vede lo stesso mount); `bus-probe` scrive finché lo spazio finisce, poi la zavorra si toglie.
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
BIN=${SPIKE_BIN:?SPIKE_BIN}
export IGGY_DATA=iggycap IGGY_MNT=/app/local_data IGGY_SUP="while true; do iggy-server; sleep 1; done"
V=ttspikebus-iggycap
C=ttspikebus-iggy-1
docker compose -p ttspikebus down >/dev/null 2>&1
docker volume rm $V >/dev/null 2>&1
docker volume create --driver local --opt type=tmpfs --opt device=tmpfs --opt o=size=64m $V >/dev/null
docker compose -p ttspikebus up -d iggy >/dev/null 2>&1
sleep 4
docker exec $C sh -c 'df -h /app/local_data | tail -1; dd if=/dev/zero of=/app/local_data/ballast bs=1M count=40 2>&1 | tail -1; df -h /app/local_data | tail -1'
"$BIN" bus-probe size=65536 batch=8 period_ms=100 secs=45 > out/probe1.txt 2>&1 &
PP=$!
sleep 16
docker exec $C sh -c 'df -h /app/local_data | tail -1; rm -f /app/local_data/ballast; df -h /app/local_data | tail -1'
echo "ballast tolto"
wait $PP
docker compose -p ttspikebus ps --format "{{.Status}}"
docker compose -p ttspikebus logs iggy 2>&1 | grep -iE "space|ENOSPC|error|panic|fail" | cut -c1-250 | tail -8
awk '$3=="ERR"{e++; if(!f)f=$0} $3=="ok"{o++} END{print "ok="o, "err="e, "primo errore: " f}' out/probe1.txt
cut -d' ' -f3 out/probe1.txt | uniq -c | head
docker compose -p ttspikebus down >/dev/null 2>&1
docker volume rm $V >/dev/null
