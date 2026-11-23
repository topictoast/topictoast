#!/bin/sh
# Una corsa di misura: ./scen.sh <iggy|valkey> <sqlite|pg> <scenario> [chiave=valore ...]
# Parte da dati vuoti, avvia motore e worker (PID salvati), esegue lo scenario, spegne i suoi processi.
# Scenari: accept start1k thr10k thrmax size64k size1m join iso0 iso kill dbdown rss
cd "$(dirname "$0")" || exit 1
BUS=$1; DB=$2; SC=$3; shift 3
BIN=/home/white_sheep/.cache/cargo-targets/topictoast-spike-bus/release/busspike
set -a; . ./.env; set +a
mkdir -p out
case $DB in
  sqlite) DBURL="sqlite://$PWD/out/spike.db?mode=rwc" ;;
  pg) DBURL="postgres://spike:$POSTGRES_PASSWORD@127.0.0.1:47003/spike" ;;
esac
CAP=${CAP:-512}; TRIM=1; PB=${PB:-1000}
EP=; WP=
case $SC in kill|dbdown) TRIM=0 ;; size1m) PB=16 ;; size64k) PB=200 ;; esac

start_engine() {
  : > out/engine.log
  "$BIN" engine bus=$BUS db="$DBURL" dir=out cap=$CAP trim=$TRIM proj_batch=$PB >> out/engine.log 2>&1 &
  EP=$!
  i=0; until grep -q READY out/engine.log; do i=$((i+1)); [ $i -gt 300 ] && { echo "motore non partito"; cat out/engine.log; stop_all; exit 1; }; sleep 0.2; done
  echo "engine pid=$EP $(grep READY out/engine.log)"
}
stop_all() { [ -n "$WP" ] && kill $WP 2>/dev/null; [ -n "$EP" ] && kill $EP 2>/dev/null; wait 2>/dev/null; }

SERV="$BUS"; [ $DB = pg ] && SERV="$SERV postgres"
./reset.sh $SERV
if [ $DB = pg ]; then i=0; until docker compose -p ttspikebus exec -T postgres pg_isready -U spike >/dev/null 2>&1; do i=$((i+1)); [ $i -gt 60 ] && exit 1; sleep 1; done; sleep 2; fi
rm -rf out/archive out/spike.db* out/master.key out/ledger.txt
[ -f out/tls/cert.pem ] || "$BIN" gen-cert dir=out
echo "### $BUS + $DB: $SC $*"
start_engine
"$BIN" worker dir=out sessions=8 > out/worker.log 2>&1 &
WP=$!
sleep 1
L="$BIN load dir=out"

case $SC in
  accept) $L rate=200 kind=0 n=1 size=1024 secs=30 warm=3 "$@" ;;
  start1k) $L rate=100 kind=0 n=10 size=1024 secs=40 warm=5 "$@" ;;
  thr10k) $L rate=1000 kind=0 n=10 size=1024 secs=60 warm=5 tenants=ab "$@" ;;
  thrmax) $L rate=2500 kind=0 n=10 size=1024 secs=30 warm=5 tenants=ab "$@" ;;
  size64k) $L rate=100 kind=0 n=5 size=65536 secs=30 warm=5 tenants=ab "$@" ;;
  size1m) $L rate=20 kind=0 n=5 size=1048576 secs=30 warm=5 tenants=ab "$@" ;;
  join) $L rate=50 kind=1 n=4 size=1024 work_ms=1500 secs=30 warm=5 tenants=ab "$@" ;;
  iso0) $L rate=100 kind=0 n=10 size=1024 tenants=b secs=30 warm=5 "$@" ;;
  iso)
    $L rate=400 kind=0 n=1 size=1024 work_ms=3000 tenants=a secs=40 warm=2 stats=0 > out/iso-a.txt &
    LA=$!
    sleep 3
    $L rate=100 kind=0 n=10 size=1024 tenants=b secs=30 warm=5 "$@"
    wait $LA; echo "-- tenant A (lento)"; cat out/iso-a.txt ;;
  rss)
    sleep 60
    ./rss.sh $EP $WP ;;
  kill)
    $L rate=300 kind=0 n=10 size=1024 secs=40 warm=1 stats=0 ledger=out/ledger.txt > out/load.txt &
    LP=$!
    for t in 10 7 8; do
      sleep $t
      echo "prima del kill: $("$BIN" stats dir=out | head -2 | tr '\n' ' ')"
      kill -9 $EP; wait $EP 2>/dev/null
      T0=$(date +%s.%N)
      sleep 1
      start_engine
      T1=$(date +%s.%N)
      echo "kill -9 -> motore di nuovo in ascolto: $(echo "$T1 - $T0" | bc) s (1 s di attesa voluta inclusa)"
      # prima riconsegna/conclusione dopo la ripresa
      i=0; until "$BIN" stats dir=out | grep -q "done=[1-9]"; do i=$((i+1)); [ $i -gt 300 ] && break; sleep 0.1; done
      T2=$(date +%s.%N)
      echo "kill -9 -> prima attività conclusa dopo la ripresa: $(echo "$T2 - $T0" | bc) s"
    done
    wait $LP; cat out/load.txt
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done ;;
  dbdown)
    $L rate=300 kind=0 n=10 size=1024 secs=40 warm=1 stats=0 ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 10
    if [ $DB = pg ]; then
      docker compose -p ttspikebus stop postgres >/dev/null 2>&1; echo "postgres fermato"
      sleep 10
      docker compose -p ttspikebus start postgres >/dev/null 2>&1; echo "postgres riavviato"
    else
      "$BIN" lock-db db="$DBURL" secs=10 &
      sleep 11
    fi
    wait $LP; cat out/load.txt; grep -E "proiezione" out/engine.log | head -4
    for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && break; sleep 3; done ;;
  *) echo "scenario sconosciuto" ;;
esac
echo "-- RSS dopo la corsa (motore, worker, bus)"
./rss.sh $EP $WP
echo "-- conteggio file archivio: $(find out/archive -type f 2>/dev/null | wc -l)"
stop_all
rm -rf out/archive
