#!/bin/sh
# Blocco A, conservazione: prova delle primitive di Iggy 0.10.0 su dati vuoti, segmenti da 1 MiB (il minimo).
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
BIN=${SPIKE_BIN:?SPIKE_BIN}
./reset.sh iggy
echo "### retention-proof su $(docker compose -p ttspikebus ps --format '{{.Image}}')"
IGGY_SEGMENT_SIZE=1048576 "$BIN" retention-proof
echo "rc=$?"
