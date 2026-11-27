#!/bin/sh
# Ferma il progetto ttspikestream, svuota i dati e riavvia Iggy pulito. Solo il progetto ttspikestream.
# I dati stanno su /home, mai su /tmp. Ogni corsa parte da dati vuoti.
# Parte 2b: lo svuotamento usa l'immagine di Iggy (presente su tutte e due le macchine; busybox su minisheep non c'e').
cd "$(dirname "$0")" || exit 1
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikestream-data}
set -a; . "$D/.env"; set +a
docker compose -p ttspikestream --env-file "$D/.env" down >/dev/null 2>&1
mkdir -p "$D"
# il server scrive come root, quindi si cancella da un container
docker run --rm -v "$D:/d" --entrypoint sh apache/iggy:0.10.0 -c 'rm -rf /d/iggy /d/engine /d/run; mkdir -p /d/iggy'
docker compose -p ttspikestream --env-file "$D/.env" up -d iggy >/dev/null 2>&1
sleep 4
