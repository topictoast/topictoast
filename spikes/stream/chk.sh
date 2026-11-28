#!/bin/bash
# Rilancia solo `check` su una corsa gia' fatta (motore ancora attivo): ./chk.sh <id> [secondi]
cd "$(dirname "$0")" || exit 1
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikestream-data}
BIN=/home/white_sheep/.cache/cargo-targets/topictoast-spike-stream/release/streamspike
set -a; . "$D/.env"; set +a
SPEC=${SPEC:-spec-c1.json}
"$BIN" check --spec "$SPEC" --run-dir "$D/run/$1/load" --data "$D/engine" --wait-s "${2:-5}" 2>&1 | grep -v '^LOG'
