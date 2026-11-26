#!/bin/sh
# Blocco A, aggiornamento: quale client iggy parla con quale server. Quattro combinazioni (server 0.9.0 e 0.10.0,
# client del motore «vecchio» 0.11.0 e «nuovo» 0.12.0): una scrittura breve con bench-append e l'esito.
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
D=$(dirname "${SPIKE_BIN:?SPIKE_BIN}")
for srv in 0.9.0 0.10.0; do
  IGGY_IMAGE=$srv ./reset.sh iggy
  echo "### server apache/iggy:$srv ($(docker compose -p ttspikebus ps --format '{{.Image}} {{.Status}}'))"
  for cli in old new; do
    b=$D/busspike-$cli
    [ $cli = old ] && v="iggy 0.11.0" || v="iggy 0.12.0"
    echo "client $cli ($v): $(timeout 60 "$b" bench-append bus=iggy secs=3 batch=10 2>&1 | tail -3 | tr '\n' ' ' | cut -c1-300)"
  done
done
echo "FINITO compat"
