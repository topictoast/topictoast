#!/bin/sh
# Ferma i servizi del progetto ttspikebus, svuota i dati e li riavvia puliti: ./reset.sh [servizi...]
# Ogni corsa parte da dati vuoti. Solo il progetto ttspikebus, mai altri container.
# I dati stanno fuori dal crate (cargo scandisce la directory del pacchetto) e su /home, mai su /tmp.
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikebus-data}
S="${*:-iggy valkey}"
docker compose -p ttspikebus down >/dev/null 2>&1
mkdir -p "$D"
docker run --rm -v "$D:/d" --entrypoint sh valkey/valkey:9.1.4 -c 'rm -rf /d/* /d/.[!.]*; mkdir -p /d/iggy /d/valkey /d/postgres /d/rustfs; chown 999:999 /d/valkey /d/postgres; chown 10001:10001 /d/rustfs'
docker compose -p ttspikebus up -d $S >/dev/null 2>&1
[ -n "$NOSLEEP" ] || sleep 3
