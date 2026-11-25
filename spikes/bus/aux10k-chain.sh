#!/bin/sh
# Blocco E2c, processo ausiliario a 10.000 att./s su bigsheep: ./aux10k-chain.sh floor | smoke | rows (staccato: nohup ./aux10k-chain.sh rows > /dev/null 2>&1 &),
# stato in chain-aux10k.log, ./aux10k-chain.sh wait [secondi]. Regole: risultati/bigsheep-iggy-sqlite-aux10k-regola-4-ms.txt.
# Tutto gira con `taskset -c 5,6,7` (campionatore e script sui core fisici 5-7; motore, worker e Iggy sono fissati da scen.sh sui core 1-4).
cd "$(dirname "$0")" || exit 1
L=chain-aux10k.log
if [ "$1" = wait ]; then
  i=0; until tail -1 $L 2>/dev/null | grep -q ALLDONE; do i=$((i+1)); [ $i -gt $(( ${2:-500} / 5 )) ] && { echo "ANCORA IN CORSA"; tail -3 $L; exit 0; }; sleep 5; done
  echo FINITA; tail -3 $L; exit 0
fi
export ARM_BIN=$HOME/ttspikebus/bin/busspike-aux-af23e79
. ./env.sh
floor() { "$SPIKE_BIN" fsync-floor dir="$HOME/.cache" n=300; }
if [ "$1" = floor ]; then
  for i in 1 2 3; do echo "$(date +%T) $(floor)"; done > risultati/bigsheep-aux10k-floor-4-ms.txt
  echo "governor cpu1: $(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_governor)" >> risultati/bigsheep-aux10k-floor-4-ms.txt
  exit 0
fi
# cancello del disco tranquillo prima di ogni corsa: p50 <= 4,0 ms e p99.9 <= 100 ms; se fuori si aspettano 5 minuti e si rimisura, al piu' 3 volte, poi si parte comunque
gate() { # etichetta
  w=0
  while :; do
    r=$(floor); echo "$(date +%T) gate $1 (attesa $w): $r" >> $L
    p50=$(echo "$r" | sed -E 's/.*p50=([0-9.]+)ms.*/\1/'); p999=$(echo "$r" | sed -E 's/.*p99.9=([0-9.]+)ms.*/\1/')
    [ "$(awk -v a="$p50" -v b="$p999" 'BEGIN{print (a<=4.0 && b<=100)}')" = 1 ] && { echo "$(date +%T) gate $1: disco tranquillo" >> $L; return; }
    [ $w -ge 3 ] && { echo "$(date +%T) gate $1: ancora fuori soglia dopo 3 attese, si parte comunque" >> $L; return; }
    w=$((w+1)); sleep 300
  done
}
R() { # file braccio "VARIABILI" secondi
  f=$1; arm=$2; e=$3; s=$4
  echo "$(date +%T) INIZIO $f" >> $L
  env ARM=$arm GATE60=1 VTRIES=100 PAYLOAD=json $e taskset -c 5,6,7 ./ms-run.sh risultati/$f ./with-sampler.sh risultati/${f%.txt}.io ./scen.sh iggy sqlite thr10k secs=$s > /dev/null 2>&1
  echo "$(date +%T) FINE $f: $(grep -E '^(activities_per_s|verify OK)' risultati/$f | tr '\n' ' ' | cut -c1-120)" >> $L
}
: > $L
if [ "$1" = smoke ]; then
  R bigsheep-iggy-sqlite-aux10k-smoke-OFF-4-ms-r1.txt AUX10K-OFF-smoke "X=0" 180
fi
if [ "$1" = rows ]; then
  # due coppie in ordine OFF, ON, ON, OFF
  gate p1-OFF; R bigsheep-iggy-sqlite-aux10k-OFF-4-ms-r1.txt AUX10K-OFF "X=0" 600
  gate p1-ON;  R bigsheep-iggy-sqlite-aux10k-ON-4-ms-r1.txt AUX10K-ON "AUX=1" 600
  gate p2-ON;  R bigsheep-iggy-sqlite-aux10k-ON-4-ms-r2.txt AUX10K-ON "AUX=1" 600
  gate p2-OFF; R bigsheep-iggy-sqlite-aux10k-OFF-4-ms-r2.txt AUX10K-OFF "X=0" 600
fi
echo "$(date +%T) ALLDONE" >> $L
