#!/bin/sh
# Blocco D, isolamento del worker rumoroso: le 18 corse misurate di iggy-sqlite-logd-iso-regola-4-ms.txt nell'ordine fisso del quadrato latino,
# staccato dalla shell: ./logd-iso-chain.sh rows > /dev/null 2>&1 &   (stato in chain-iso.log; ./logd-iso-chain.sh wait [secondi] attende)
# Riprendibile: salta le corse il cui file grezzo esiste ed e' valido (V1-V3 delle regole); una corsa non valida si rifa' una volta, subito, nel file `...-ripetizione.txt`.
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
# validita' V1-V3: stampa il motivo e torna 1 se la corsa non e' valida
valid() { # file braccio
  f=$1; a=$2
  grep -q '^verify OK' $f || { echo "V1: verify non OK"; return 1; }
  case $a in
    Anolines|Bsolo3)
      lw=$(grep -m1 -oE 'lines_written=[0-9]+' $f | cut -d= -f2); at=$(grep -m1 -oE 'attempts_written=[0-9]+' $f | cut -d= -f2)
      grep -q '^logcheck OK' $f && [ -n "$lw" ] && [ "$lw" = $((3 * at)) ] || { echo "V1: logcheck o lines_written != 3 * attempts_written ($lw, $at)"; return 1; } ;;
    Bsolo0) grep -q '^logcheck' $f && { echo "V1: riga logcheck presente"; return 1; } ;;
  esac
  db=$(grep -m1 -oE 'done_b=[0-9]+' $f | cut -d= -f2)
  [ "${db:-0}" -ge 38000 ] || { echo "V2: done_b=$db < 38000"; return 1; }
  grep -m1 -oE 'errors=[0-9]+' $f | grep -q 'errors=0$' || { echo "V2: errors del generatore di B non 0"; return 1; }
  if [ "$a" = Anolines ]; then
    da=$(grep -m1 -oE 'done_a=[0-9]+' $f | cut -d= -f2)
    [ "${da:-0}" -ge 19000 ] || { echo "V3: done_a=$da < 19000"; return 1; }
    sed -n '/tenant A (rumoroso)/,$p' $f | grep -m1 -oE 'errors=[0-9]+' | grep -q 'errors=0$' || { echo "V3: errors del generatore di A non 0"; return 1; }
    set -- $(sed -n '/tenant A (rumoroso)/,$p' $f | grep -m1 -E '^offered=' | sed -E 's/offered=([0-9]+) accepted=([0-9]+).*/\1 \2/')
    [ $(( $2 * 100 )) -ge $(( $1 * 99 )) ] || { echo "V3: accettate di A sotto il 99 % ($2 su $1)"; return 1; }
  fi
  return 0
}
step() { # file braccio
  f=$1; a=$2; g=${f%.txt}-ripetizione.txt
  [ -f risultati/$f ] || run $f $a
  if ! why=$(valid risultati/$f $a); then
    echo "$(date +%T) NON VALIDA $f: $why" >> $L
    [ -f risultati/$g ] || run $g $a
    why2=$(valid risultati/$g $a) || echo "$(date +%T) NON VALIDA ANCHE LA RIPETIZIONE $g: $why2" >> $L
  fi
}
if [ "$1" = smoke ]; then
  : > $L
  run iggy-sqlite-logd-smoke-iso-Bsolo0-4-ms-r1.txt Bsolo0
  run iggy-sqlite-logd-smoke-iso-Bsolo3-4-ms-r1.txt Bsolo3
  echo "$(date +%T) ALLDONE" >> $L; exit 0
fi
echo "$(date +%T) RIPRESA/AVVIO" >> $L
n=0
for giro in "Anolines Bsolo3 Bsolo0" "Bsolo3 Bsolo0 Anolines" "Bsolo0 Anolines Bsolo3" "Anolines Bsolo0 Bsolo3" "Bsolo3 Anolines Bsolo0" "Bsolo0 Bsolo3 Anolines"; do
  n=$((n+1))
  for a in $giro; do step iggy-sqlite-logd-iso-$a-4-ms-r$n.txt $a; done
done
echo "$(date +%T) ALLDONE" >> $L
