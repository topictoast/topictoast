#!/bin/sh
# Blocco E2, processo ausiliario: corse di prova (`smoke`, non misurate, `-smoke-aux-...-r2`) e corse misurate (`rows`) delle regole
# (iggy-sqlite-aux-regola-4-ms.txt). Staccato: nohup ./aux-chain.sh rows > /dev/null 2>&1 &  (stato in chain-aux.log; ./aux-chain.sh wait [secondi])
# `rows` applica le validita' V1-V3 e rifa' una volta, subito, una corsa non valida (`...-ripetizione.txt`); riprendibile.
cd "$(dirname "$0")" || exit 1
L=chain-aux.log
if [ "$1" = wait ]; then
  i=0; until grep -q ALLDONE $L 2>/dev/null; do i=$((i+1)); [ $i -gt $(( ${2:-500} / 5 )) ] && { echo "ANCORA IN CORSA"; tail -3 $L; exit 0; }; sleep 5; done
  echo FINITA; tail -3 $L; exit 0
fi
export ARM_BIN=${ARM_BIN:-$HOME/ttspikebus/bin/busspike-aux-af23e79}
. ./env.sh
R() { # file braccio "VARIABILI" scena [argomenti]
  f=$1; arm=$2; e=$3; sc=$4; shift 4
  echo "$(date +%T) INIZIO $f" >> $L
  env ARM=$arm $e ./ms-run.sh risultati/$f ./scen.sh iggy sqlite $sc "$@" > /dev/null 2>&1
  echo "$(date +%T) FINE $f: $(grep -E '^(verify OK|start_ready |rss somma a riposo)' risultati/$f | tr '\n' ' ' | cut -c1-220)" >> $L
}
# validita': stampa il motivo e torna 1 se la corsa non e' valida. Tipi: OFF, ON, neg-schema, neg-if, rss
valid() { # file tipo
  f=$1; k=$2
  grep -q '^\[stato dopo' $f || { echo "corsa incompleta"; return 1; }
  case $k in
    OFF|ON)
      grep -q '^verify OK' $f || { echo "V1: verify non OK"; return 1; }
      ap=$(grep -m1 -oE 'activities_per_s=[0-9]+' $f | cut -d= -f2)
      [ "${ap:-0}" -ge 950 ] || { echo "V2: activities_per_s=$ap < 950"; return 1; }
      grep -m1 -oE 'errors=[0-9]+' $f | grep -q 'errors=0$' || { echo "V2: errors del generatore non 0"; return 1; } ;;
  esac
  case $k in
    ON)
      fin=$(grep '^-- ausiliario a fine corsa' $f | tail -1)
      cp=$(echo "$fin" | grep -oE 'calls_prepare=[0-9]+' | cut -d= -f2); cc=$(echo "$fin" | grep -oE 'calls_check=[0-9]+' | cut -d= -f2)
      ac=$(grep '^verify:' $f | tail -1 | grep -oE 'attivita_concluse=[0-9]+' | cut -d= -f2)
      [ -n "$cp" ] && [ "$cp" = "$ac" ] && [ "$cc" = "$ac" ] || { echo "V3: calls_prepare=$cp calls_check=$cc attivita_concluse=$ac"; return 1; }
      echo "$fin" | grep -q 'skipped=0 input_invalid=0 output_invalid=0 errors=0' || { echo "V3: errori, non validi o saltati nell'ausiliario"; return 1; } ;;
    OFF) grep -q '^aux calls_prepare' $f && { echo "V3: righe aux in OFF"; return 1; } ;;
    neg-schema) grep -q '^-- ausiliario a fine corsa' $f || { echo "manca la riga finale dell'ausiliario"; return 1; } ;;
    neg-if) grep -q '^verify OK' $f || { echo "V1: verify non OK"; return 1; } ;;
    rss) grep -q '^verify OK' $f && grep -q '^rss somma a riposo' $f || { echo "verify non OK o somma a riposo mancante"; return 1; } ;;
  esac
  return 0
}
step() { # file braccio "VARIABILI" tipo scena [argomenti]
  f=$1; arm=$2; e=$3; k=$4; sc=$5; shift 5; g=${f%.txt}-ripetizione.txt
  [ -f risultati/$f ] || R $f $arm "$e" $sc "$@"
  if ! why=$(valid risultati/$f $k); then
    echo "$(date +%T) NON VALIDA $f: $why" >> $L
    [ -f risultati/$g ] || R $g $arm "$e" $sc "$@"
    why2=$(valid risultati/$g $k) || echo "$(date +%T) NON VALIDA ANCHE LA RIPETIZIONE $g: $why2" >> $L
  fi
}
[ "$1" = smoke ] && : > $L
if [ "$1" = smoke ]; then
  R iggy-sqlite-aux-smoke-aux1k-OFF-4-ms-r2.txt AUX-OFF-smoke "PAYLOAD=json" aux1k secs=10 warm=2
  R iggy-sqlite-aux-smoke-aux1k-ON-4-ms-r2.txt AUX-ON-smoke "PAYLOAD=json AUX=1" aux1k secs=10 warm=2
  R iggy-sqlite-aux-smoke-auxneg-schema-4-ms-r2.txt AUX-neg-schema-smoke "PAYLOAD=json AUX=1 INBAD=10" auxneg rate=50 n=1 secs=10 warm=2
  R iggy-sqlite-aux-smoke-auxneg-if-4-ms-r2.txt AUX-neg-if-smoke "PAYLOAD=json AUX=1 INFLAG=0" auxneg rate=50 n=1 secs=10 warm=2
fi
if [ "$1" = rows ]; then
  echo "$(date +%T) RIPRESA/AVVIO" >> $L
  # riga 1: 6 coppie, coppie dispari con OFF per primo
  for n in 1 2 3 4 5 6; do
    if [ $((n % 2)) = 1 ]; then a1=OFF; a2=ON; else a1=ON; a2=OFF; fi
    for a in $a1 $a2; do
      if [ $a = ON ]; then e="PAYLOAD=json AUX=1"; else e="PAYLOAD=json"; fi
      step iggy-sqlite-aux-aux1k-$a-4-ms-r$n.txt AUX-$a "$e" $a aux1k
    done
  done
  step iggy-sqlite-aux-auxneg-schema-4-ms-r1.txt AUX-neg-schema "PAYLOAD=json AUX=1 INBAD=10" neg-schema auxneg rate=50 n=1 secs=10 warm=2
  step iggy-sqlite-aux-auxneg-if-4-ms-r1.txt AUX-neg-if "PAYLOAD=json AUX=1 INFLAG=0" neg-if auxneg rate=50 n=1 secs=10 warm=2
  step iggy-sqlite-aux-auxrss-4-ms-r1.txt AUX-rss "PAYLOAD=json AUX=1" rss auxrss
fi
echo "$(date +%T) ALLDONE" >> $L
