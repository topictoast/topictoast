#!/bin/sh
# Una corsa con l'output salvato: ./run1.sh <bus> <db> <scenario> <rep> [chiave=valore ...]
cd "$(dirname "$0")" || exit 1
mkdir -p risultati
B=$1; D=$2; S=$3; R=$4; shift 4
./scen.sh "$B" "$D" "$S" "$@" > "risultati/$B-$D-$S-r$R.txt" 2>&1
# riga di sintesi per la console
grep -E "^(###|offered|accept_rtt|accept_engine|start_ready |start_join|start_ready_tenant_b|activities_per_s|append|proj_lag|archive_read|sql_per|sql_stmts|verify|kill|rss iggy|rss valkey|rss pid)" "risultati/$B-$D-$S-r$R.txt"
