#!/bin/sh
# Iggy da solo (fase 4, blocco A), allo stesso ritmo di byte del motore a 10.000 att./s: dati vuoti, poi bench-append.
# 10.000 att./s = 1.000 esecuzioni/s da 10 passi = 21.000 messaggi/s (11 da 1.140 byte e 10 da 80 per esecuzione),
# due topic (i due tenant) con due connessioni ciascuno. Argomenti `chiave=valore` dopo i predefiniti li sovrascrivono.
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
BIN=${SPIKE_BIN:-/home/white_sheep/.cache/cargo-targets/topictoast-spike-bus/release/busspike}
TS_ENG=${SPIKE_CPUS:+taskset -c $SPIKE_CPUS}
./reset.sh iggy
echo "### iggy da solo: bench-append $*"
$TS_ENG "$BIN" bench-append bus=iggy own=1 topics=2 conc=4 batch=128 size=1140 mix=1 rate=21000 secs=600 warmup=5 minsecs=60 "$@"
