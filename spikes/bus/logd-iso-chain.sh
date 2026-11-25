#!/bin/sh
# Blocco D, isolamento del worker rumoroso: le 18 corse misurate di iggy-sqlite-logd-iso-regola-4-ms.txt nell'ordine fisso del quadrato latino,
# staccato dalla shell: ./logd-iso-chain.sh rows > /dev/null 2>&1 &   (stato in chain-iso.log; ./logd-iso-chain.sh wait [secondi] attende)
# Con `./logd-iso-chain.sh smoke` le due corse di prova (non misurate, nome `-smoke-iso-`).
cd "$(dirname "$0")" || exit 1
L=chain-iso.log
if [ "$1" = wait ]; then
  i=0; until grep -q ALLDONE $L 2>/dev/null; do i=$((i+1)); [ $i -gt $(( ${2:-500} / 5 )) ] && { echo "ANCORA IN CORSA"; tail -3 $L; exit 0; }; sleep 5; done
  echo FINITA; tail -3 $L; exit 0
fi
export ARM_BIN=$HOME/ttspikebus/bin/busspike-logd-7c1dfcd
. ./env.sh
R() { # file braccio "VARIABILI"
  f=$1; arm=$2; e=$3
  echo "$(date +%T) INIZIO $f" >> $L
  env ARM=LOG-iso-$arm $e ./ms-run.sh risultati/$f ./scen.sh iggy sqlite lognoisy > /dev/null 2>&1
  echo "$(date +%T) FINE $f: $(grep -E '^(start_ready_tenant_b|verify |logcheck [OFA])' risultati/$f | tr '\n' ' ' | cut -c1-200)" >> $L
}
A="NOISY=1 LOGA=0x0 LOGB=3x80"; S3="LOGB=3x80"; S0="X=0"
run() { case $2 in Anolines) R $1 Anolines "$A";; Bsolo3) R $1 Bsolo3 "$S3";; Bsolo0) R $1 Bsolo0 "$S0";; esac; }
: > $L
if [ "$1" = smoke ]; then
  run iggy-sqlite-logd-smoke-iso-Bsolo0-4-ms-r1.txt Bsolo0
  run iggy-sqlite-logd-smoke-iso-Bsolo3-4-ms-r1.txt Bsolo3
  echo "$(date +%T) ALLDONE" >> $L; exit 0
fi
n=0
for giro in "Anolines Bsolo3 Bsolo0" "Bsolo3 Bsolo0 Anolines" "Bsolo0 Anolines Bsolo3" "Anolines Bsolo0 Bsolo3" "Bsolo3 Anolines Bsolo0" "Bsolo0 Bsolo3 Anolines"; do
  n=$((n+1))
  for a in $giro; do run iggy-sqlite-logd-iso-$a-4-ms-r$n.txt $a; done
done
echo "$(date +%T) ALLDONE" >> $L
