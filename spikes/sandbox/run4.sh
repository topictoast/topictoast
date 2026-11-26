#!/bin/sh
# Una corsa della fase 4 su minisheep: ./run4.sh <riga> <n|smoke> <scenario> [k=v ...]
# Scrive risultati/sandbox-fase4-<riga>-ms-r<n>.txt (oppure sandbox-fase4-smoke-<riga>-ms.txt: la corsa di
# prova non conta). Rifiuta di sovrascrivere un file esistente. Il binario è sbxspike4.
cd "$(dirname "$0")" || exit 1
. ./env.sh
export SPIKE_BIN=$HOME/ttspikesandbox/bin/sbxspike4
ROW=$1
REP=$2
SCEN=$3
shift 3
case $REP in
smoke) F=risultati/sandbox-fase4-smoke-$ROW-ms.txt ;;
*) F=risultati/sandbox-fase4-$ROW-ms-r$REP.txt ;;
esac
[ -e "$F" ] && {
  echo "esiste già: $F"
  exit 1
}
ARM=$ROW-$REP ./ms-run.sh "$F" timeout 590 "$SPIKE_BIN" drive4 "$SCEN" $SBX_ARGS "$@"
