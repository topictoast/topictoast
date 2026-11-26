#!/bin/sh
# Una corsa su minisheep: ./run.sh <riga> <n|smoke> <scenario> [k=v ...]
# Scrive risultati/sandbox-fase2-<riga>-ms-r<n>.txt (oppure sandbox-fase2-smoke-<riga>-ms.txt: la corsa di
# prova non conta). Rifiuta di sovrascrivere un file esistente.
cd "$(dirname "$0")" || exit 1
. ./env.sh
ROW=$1
REP=$2
SCEN=$3
shift 3
case $REP in
smoke) F=risultati/sandbox-fase2-smoke-$ROW-ms.txt ;;
*) F=risultati/sandbox-fase2-$ROW-ms-r$REP.txt ;;
esac
[ -e "$F" ] && {
  echo "esiste già: $F"
  exit 1
}
ARM=$ROW-$REP ./ms-run.sh "$F" timeout 590 "$SPIKE_BIN" drive "$SCEN" $SBX_ARGS "$@"
