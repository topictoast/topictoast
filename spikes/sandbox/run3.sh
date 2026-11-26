#!/bin/sh
# Una corsa della fase 3 su minisheep: ./run3.sh <riga> <n|smoke> <scenario> [k=v ...]
# Scrive risultati/sandbox-fase3-<riga>-ms-r<n>.txt (oppure sandbox-fase3-smoke-<riga>-ms.txt: la corsa di
# prova non conta). Rifiuta di sovrascrivere un file esistente. Il binario è sbxspike3 (statico, lo stesso che
# gira come agente nel guest).
cd "$(dirname "$0")" || exit 1
. ./env.sh
export SPIKE_BIN=$HOME/ttspikesandbox/bin/sbxspike3
ROW=$1
REP=$2
SCEN=$3
shift 3
case $REP in
smoke) F=risultati/sandbox-fase3-smoke-$ROW-ms.txt ;;
*) F=risultati/sandbox-fase3-$ROW-ms-r$REP.txt ;;
esac
[ -e "$F" ] && {
  echo "esiste già: $F"
  exit 1
}
ARM=$ROW-$REP ./ms-run.sh "$F" timeout 590 "$SPIKE_BIN" fc "$SCEN" $SBX_ARGS "$@"
