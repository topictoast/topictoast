#!/bin/sh
# Blocco D, log dei task: tutte le corse misurate delle righe 1-5 delle regole (iggy-sqlite-logd-regola-4-ms.txt), una alla volta e in
# questo ordine, staccato dalla shell: ./logd-chain.sh rows12345 > /dev/null 2>&1 &   (stato in ~/ttspikebus/chain-logd.log; ./logd-chain.sh wait [secondi] attende)
cd "$(dirname "$0")" || exit 1
L=chain-logd.log
if [ "$1" = wait ]; then
  i=0; until grep -q ALLDONE $L 2>/dev/null; do i=$((i+1)); [ $i -gt $(( ${2:-500} / 5 )) ] && { echo "ANCORA IN CORSA"; tail -3 $L; exit 0; }; sleep 5; done
  echo FINITA; tail -3 $L; exit 0
fi
export ARM_BIN=$HOME/ttspikebus/bin/busspike-logd-7c1dfcd
. ./env.sh
R() { # file braccio "VARIABILI" argomenti-di-scen...
  f=$1; arm=$2; e=$3; shift 3
  echo "$(date +%T) INIZIO $f" >> $L
  env ARM=$arm $e ./ms-run.sh risultati/$f ./scen.sh iggy sqlite "$@" > /dev/null 2>&1
  echo "$(date +%T) FINE $f: $(grep -E '^(log_visibility|start_ready_tenant_b|verify |logcheck [OFA])' risultati/$f | tr '\n' ' ' | cut -c1-300)" >> $L
}
: > $L
# riga 1: visibilita'
for n in 1 2 3; do R iggy-sqlite-logd-vis-LOG-vis-4-ms-r$n.txt LOG-vis "LOGA=5x80 TAIL=1" logrun; done
# riga 2: worker rumoroso, 6 coppie, coppie dispari con withA per primo
W="NOISY=1 LOGA=100x80 LOGB=3x80"; N="NOISY=1 LOGA=0x0 LOGB=3x80"
for n in 1 2 3 4 5 6; do
  if [ $((n % 2)) = 1 ]; then a1=withA; e1=$W; a2=Anolines; e2=$N; else a1=Anolines; e1=$N; a2=withA; e2=$W; fi
  R iggy-sqlite-logd-noisy-$a1-4-ms-r$n.txt LOG-noisy-$a1 "$e1" lognoisy
  R iggy-sqlite-logd-noisy-$a2-4-ms-r$n.txt LOG-noisy-$a2 "$e2" lognoisy
done
# riga 3: segreto (2 corse con la sostituzione, 1 di controllo con la sostituzione spenta)
S="tenants=ab rate=40 n=5 secs=10 warm=2"
for n in 1 2; do R iggy-sqlite-logd-secret-on-4-ms-r$n.txt LOG-secret-on "LOGMARK=1 LOGA=20x80 LOGB=20x80" logrun $S; done
R iggy-sqlite-logd-secret-off-4-ms-r1.txt LOG-secret-off "LOGMARK=1 REDACT=0 LOGA=20x80 LOGB=20x80" logrun $S
# riga 4: crash del motore con righe in volo
for n in 1 2 3; do R iggy-sqlite-logd-crash-LOG-crash-4-ms-r$n.txt LOG-crash "LOGMARK=1 LOGA=20x80 LOGB=20x80 LOGSPLIT=1000 ACKHOLD=1500 ERATE=334 CAP=4096" ekill; done
# riga 5: tetto
R iggy-sqlite-logd-cap-LOG-cap-4-ms-r1.txt LOG-cap "LOGA=50x400 LOGB=150x80" logrun tenants=ab rate=20 n=5 secs=6 warm=2
echo "$(date +%T) ALLDONE" >> $L
