#!/bin/sh
# Una ripetizione del bench del solo bus, bus alternati, ognuno su dati vuoti: ./bench-rep.sh <etichetta> [bus...]
cd "$(dirname "$0")" || exit 1
B=/home/white_sheep/.cache/cargo-targets/topictoast-spike-bus/release/busspike
L=$1; shift
for bus in ${*:-iggy valkey}; do
  ./reset.sh iggy valkey
  set -a; . ./.env; set +a
  echo "== rep=$L bus=$bus"
  for cfg in "batch=1 conc=1" "batch=50 conc=1" "batch=100 conc=4 own=1"; do
    # shellcheck disable=SC2086
    $B bench-append bus=$bus size=1024 $cfg secs=6 warmup=2
  done
  [ "$bus" = valkey ] && docker compose -p ttspikebus exec valkey valkey-cli -a "$VALKEY_PASSWORD" info stats 2>/dev/null | grep -E "total_forks"
done
