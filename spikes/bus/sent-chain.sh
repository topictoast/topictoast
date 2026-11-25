#!/bin/sh
# Sentinella: una corsa di prova per riga delle regole (iggy-sqlite-sent-regola-4-ms.txt), non misurate (`-smoke-sent-`):
# ./sent-chain.sh smoke  (staccato: nohup ./sent-chain.sh smoke > /dev/null 2>&1 &; stato in chain-sent.log; ./sent-chain.sh wait [secondi])
cd "$(dirname "$0")" || exit 1
L=chain-sent.log
if [ "$1" = wait ]; then
  i=0; until grep -q ALLDONE $L 2>/dev/null; do i=$((i+1)); [ $i -gt $(( ${2:-500} / 5 )) ] && { echo "ANCORA IN CORSA"; tail -3 $L; exit 0; }; sleep 5; done
  echo FINITA; tail -3 $L; exit 0
fi
export ARM_BIN=${ARM_BIN:-$HOME/ttspikebus/bin/busspike-sent-116e57c}
. ./env.sh
R() { # file braccio "VARIABILI" scena [argomenti di scen]
  f=$1; arm=$2; e=$3; sc=$4; shift 4
  echo "$(date +%T) INIZIO $f" >> $L
  env ARM=$arm $e ./ms-run.sh risultati/$f ./scen.sh iggy sqlite $sc "$@" > /dev/null 2>&1
  echo "$(date +%T) FINE $f: $(grep -E '^(sentinel allarmi|verify OK|newtenant:)' risultati/$f | tr '\n' ' ' | cut -c1-250)" >> $L
}
: > $L
if [ "$1" = smoke ]; then
  R iggy-sqlite-sent-smoke-cost-OFF-4-ms-r1.txt SENT-cost-OFF-smoke "X=0" logrun secs=10 warm=2
  R iggy-sqlite-sent-smoke-cost-ON-4-ms-r1.txt SENT-cost-ON-smoke "SENT=1 META=1" logrun secs=10 warm=2
  R iggy-sqlite-sent-smoke-ekilldown-4-ms-r1.txt SENT-ekill-down-smoke "SENT=1 META=1 DOWN=20 ERATE=334 CAP=4096" ekill
  R iggy-sqlite-sent-smoke-retry-4-ms-r1.txt SENT-retry-smoke "SENT=1 META=1" retry
  R iggy-sqlite-sent-smoke-wkill-4-ms-r1.txt SENT-wkill-smoke "SENT=1 META=1 ERATE=334 CAP=4096" wkill
  R iggy-sqlite-sent-smoke-newtenant-4-ms-r1.txt SENT-newtenant-smoke "SENT=1 META=1" newtenant
fi
echo "$(date +%T) ALLDONE" >> $L
