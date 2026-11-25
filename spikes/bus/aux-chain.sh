#!/bin/sh
# Blocco E2, processo ausiliario: corse di prova (`smoke`, non misurate, nome `-smoke-aux-`) e, dopo la revisione, le corse delle
# regole (iggy-sqlite-aux-regola-4-ms.txt). Staccato: nohup ./aux-chain.sh smoke > /dev/null 2>&1 &  (stato in chain-aux.log; ./aux-chain.sh wait [secondi])
cd "$(dirname "$0")" || exit 1
L=chain-aux.log
if [ "$1" = wait ]; then
  i=0; until grep -q ALLDONE $L 2>/dev/null; do i=$((i+1)); [ $i -gt $(( ${2:-500} / 5 )) ] && { echo "ANCORA IN CORSA"; tail -3 $L; exit 0; }; sleep 5; done
  echo FINITA; tail -3 $L; exit 0
fi
export ARM_BIN=${ARM_BIN:-$HOME/ttspikebus/bin/busspike-aux-440cc63}
. ./env.sh
R() { # file braccio "VARIABILI" scena [argomenti]
  f=$1; arm=$2; e=$3; sc=$4; shift 4
  echo "$(date +%T) INIZIO $f" >> $L
  env ARM=$arm $e ./ms-run.sh risultati/$f ./scen.sh iggy sqlite $sc "$@" > /dev/null 2>&1
  echo "$(date +%T) FINE $f: $(grep -E '^(verify OK|start_ready )' risultati/$f | tr '\n' ' ' | cut -c1-200)" >> $L
}
: > $L
if [ "$1" = smoke ]; then
  R iggy-sqlite-aux-smoke-aux1k-OFF-4-ms-r1.txt AUX-OFF-smoke "PAYLOAD=json" aux1k secs=10 warm=2
  R iggy-sqlite-aux-smoke-aux1k-ON-4-ms-r1.txt AUX-ON-smoke "PAYLOAD=json AUX=1" aux1k secs=10 warm=2
  R iggy-sqlite-aux-smoke-auxneg-schema-4-ms-r1.txt AUX-neg-schema-smoke "PAYLOAD=json AUX=1 INBAD=10" auxneg rate=50 n=1 secs=10 warm=2
  R iggy-sqlite-aux-smoke-auxneg-if-4-ms-r1.txt AUX-neg-if-smoke "PAYLOAD=json AUX=1 INFLAG=0" auxneg rate=50 n=1 secs=10 warm=2
  R iggy-sqlite-aux-smoke-auxrss-4-ms-r1.txt AUX-rss-smoke "PAYLOAD=json AUX=1" auxrss
fi
echo "$(date +%T) ALLDONE" >> $L
