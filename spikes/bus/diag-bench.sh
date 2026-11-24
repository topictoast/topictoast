#!/bin/sh
# Diagnosi del solo bus (fase 2d): stesse richieste con connessioni e topic diversi. ./diag-bench.sh <bus>...
cd "$(dirname "$0")" || exit 1
. ./env.sh
set -a; . ./.env; set +a
for b in "$@"; do
  ./reset.sh "$b"
  for cfg in "batch=256 conc=1" "batch=64 conc=4 own=1" "batch=64 conc=4 own=1 same=1" "batch=32 conc=8 own=1 same=1"; do
    # shellcheck disable=SC2086
    taskset -c 8-11 "$SPIKE_BIN" bench-append bus=$b size=1024 $cfg secs=8 warmup=2 2>&1 | cut -c1-170
  done
done
