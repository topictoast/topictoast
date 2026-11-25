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
# validita' V1-V3 delle regole: stampa il motivo e torna 1 se la corsa non e' valida
valid() { # file riga(cost-OFF|cost-ON|altre)
  f=$1; k=$2
  grep -q '^verify OK' $f || { echo "V1: verify non OK"; return 1; }
  case $k in cost-*)
    ap=$(grep -m1 -oE 'activities_per_s=[0-9]+' $f | cut -d= -f2)
    [ "${ap:-0}" -ge 950 ] || { echo "V2: activities_per_s=$ap < 950"; return 1; }
    grep -m1 -oE 'errors=[0-9]+' $f | grep -q 'errors=0$' || { echo "V2: errors del generatore non 0"; return 1; } ;;
  esac
  if [ "$k" != cost-OFF ]; then
    ev=$(grep -m1 -oE 'sentinel_stats events=[0-9]+' $f | cut -d= -f2)
    [ "${ev:-0}" -gt 0 ] || { echo "V3: sentinel_stats mancante o events=0"; return 1; }
  fi
  return 0
}
# una corsa con controllo di validita' e una sola ripetizione subito, nella stessa posizione; riprendibile (salta le corse valide)
step() { # file braccio "VARIABILI" riga scena [argomenti]
  f=$1; arm=$2; e=$3; k=$4; sc=$5; shift 5; g=${f%.txt}-ripetizione.txt
  [ -f risultati/$f ] || R $f $arm "$e" $sc "$@"
  if ! why=$(valid risultati/$f $k); then
    echo "$(date +%T) NON VALIDA $f: $why" >> $L
    [ -f risultati/$g ] || R $g $arm "$e" $sc "$@"
    why2=$(valid risultati/$g $k) || echo "$(date +%T) NON VALIDA ANCHE LA RIPETIZIONE $g: $why2" >> $L
  fi
}
[ "$1" = smoke ] && : > $L
if [ "$1" = rows ]; then
  echo "$(date +%T) RIPRESA/AVVIO" >> $L
  # riga 1: 6 coppie, coppie dispari con OFF per primo
  for n in 1 2 3 4 5 6; do
    if [ $((n % 2)) = 1 ]; then a1=OFF; a2=ON; else a1=ON; a2=OFF; fi
    for a in $a1 $a2; do
      if [ $a = ON ]; then e="SENT=1 META=1"; else e="X=0"; fi
      step iggy-sqlite-sent-cost-$a-4-ms-r$n.txt SENT-cost-$a "$e" cost-$a logrun
    done
  done
  for n in 1 2 3; do step iggy-sqlite-sent-ekilldown-4-ms-r$n.txt SENT-ekill-down "SENT=1 META=1 DOWN=20 ERATE=334 CAP=4096" ekilldown ekill; done
  for n in 1 2; do step iggy-sqlite-sent-retry-4-ms-r$n.txt SENT-retry "SENT=1 META=1" retry retry; done
  for n in 1 2; do step iggy-sqlite-sent-wkill-4-ms-r$n.txt SENT-wkill "SENT=1 META=1 ERATE=334 CAP=4096" wkill wkill; done
  for n in 1 2; do step iggy-sqlite-sent-newtenant-4-ms-r$n.txt SENT-newtenant "SENT=1 META=1" newtenant newtenant; done
fi
if [ "$1" = smoke ]; then
  R iggy-sqlite-sent-smoke-cost-OFF-4-ms-r1.txt SENT-cost-OFF-smoke "X=0" logrun secs=10 warm=2
  R iggy-sqlite-sent-smoke-cost-ON-4-ms-r1.txt SENT-cost-ON-smoke "SENT=1 META=1" logrun secs=10 warm=2
  R iggy-sqlite-sent-smoke-ekilldown-4-ms-r1.txt SENT-ekill-down-smoke "SENT=1 META=1 DOWN=20 ERATE=334 CAP=4096" ekill
  R iggy-sqlite-sent-smoke-retry-4-ms-r1.txt SENT-retry-smoke "SENT=1 META=1" retry
  R iggy-sqlite-sent-smoke-wkill-4-ms-r1.txt SENT-wkill-smoke "SENT=1 META=1 ERATE=334 CAP=4096" wkill
  R iggy-sqlite-sent-smoke-newtenant-4-ms-r1.txt SENT-newtenant-smoke "SENT=1 META=1" newtenant
fi
echo "$(date +%T) ALLDONE" >> $L
