#!/bin/sh
# Blocco A, aggiornamento senza perdite Iggy 0.9.0 -> 0.10.0 (fase 4): ./up-run.sh [chiave=valore ...]
# Motore vecchio (client iggy 0.11.0) su Iggy 0.9.0 con carico a 1.000 attività/s da 3 s; poi:
#   1. SIGUSR1: il motore rifiuta le accettazioni (E_UPGRADE_DRAIN) e si svuota (posizione applicata = testa su ogni topic), poi esce;
#   2. Iggy 0.10.0 su una directory dati vuota;
#   3. motore nuovo (client 0.12.0) con busgen=2 upgrade=1: riscrive lo stato vivo dal database e dall'archivio, porta le
#      posizioni salvate alla nuova generazione;
#   4. il motore riprende (i worker si ricollegano da soli).
# I tempi sono in microsecondi dall'epoca (stesso orologio del motore, del worker e del generatore di carico).
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
BIN=${SPIKE_BIN:?SPIKE_BIN}
OLD=${SPIKE_OLD:-$(dirname "$BIN")/busspike-old}
DBURL="sqlite://$PWD/out/spike.db?mode=rwc"
TS_ENG=${SPIKE_CPUS:+taskset -c $SPIKE_CPUS}; TS_LOAD=${SPIKE_LOADCPUS:+taskset -c $SPIKE_LOADCPUS}
CAP=${CAP:-4096}
PRE=${PRE:-20}          # secondi di carico prima dell'aggiornamento
SECS=${SECS:-70}        # durata del generatore di carico
us() { echo $(( $(date +%s%N) / 1000 )); }
mkdir -p out
EP=; WP=; LP=
start_engine() { # <binario> <log> [argomenti]
  B=$1; L=$2; shift 2
  : > "$L"
  $TS_ENG "$B" engine bus=iggy db="$DBURL" dir=out cap=$CAP trim=0 link=grpc proj_batch=1000 wconn=1 kgroups=4 maxb=1024 minb=64 "$@" >> "$L" 2>&1 &
  EP=$!
}
wait_ready() { # <log> <pid>
  i=0; until grep -q READY "$1"; do i=$((i+1)); [ $i -gt 600 ] && { echo "motore non partito"; tail -5 "$1"; return 1; }; kill -0 "$2" 2>/dev/null || return 2; sleep 0.1; done
}
stop_all() { [ -n "$WP" ] && kill $WP 2>/dev/null; [ -n "$EP" ] && kill $EP 2>/dev/null; wait 2>/dev/null; }

IGGY_IMAGE=0.9.0 ./reset.sh iggy
docker compose -p ttspikebus ps --format "iggy: {{.Image}} {{.Status}}"
rm -rf out/archive out/spike.db* out/master.key out/ledger.txt out/ledger-w.txt out/old.log out/new.log out/work.log
[ -f out/tls/cert.pem ] || "$BIN" gen-cert dir=out
[ -f out/worker.cred ] || "$BIN" gen-cred dir=out
echo "### aggiornamento Iggy 0.9.0 -> 0.10.0: carico $*"

start_engine "$OLD" out/old.log busgen=1
wait_ready out/old.log $EP || { stop_all; exit 1; }
OLDPID=$EP
echo "motore vecchio pid=$OLDPID $(grep READY out/old.log)"
$TS_ENG "$BIN" worker dir=out link=grpc sessions=4 worklog=out/work.log > out/worker.log 2>&1 &
WP=$!
sleep 1
# blocco C, PRE100K=1: prima 100.000 esecuzioni `wait` (kind 2) con timer di ${TWAIT:-150000} ms, tutti in sospeso durante l'aggiornamento
if [ -n "$PRE100K" ]; then
  $TS_LOAD "$BIN" load dir=out rate=${TRATE:-5000} kind=2 n=1 size=64 work_ms=${TWAIT:-150000} tenants=ab secs=${TSECS:-20} warm=0 stats=0 ledger=out/ledger-w.txt > out/load-w.txt
  echo "-- timer di attesa: $(grep -E '^offered' out/load-w.txt); in sospeso nel motore vecchio: $("$BIN" stats dir=out | grep -E timers_pending | tr '\n' ' ')"
fi
$TS_LOAD "$BIN" load dir=out rate=1000 kind=0 n=1 size=1024 work_ms=3000 tenants=ab secs=$SECS warm=2 stats=0 ledger=out/ledger.txt "$@" > out/load.txt &
LP=$!
sleep $PRE
echo "prima: $("$BIN" stats dir=out | head -2 | tr '\n' ' ')"

# --- passo 1: rifiuto con codice e svuotamento
T0=$(us)
kill -USR1 $OLDPID
i=0; while kill -0 $OLDPID 2>/dev/null; do i=$((i+1)); [ $i -gt 1200 ] && { echo "il motore vecchio non si è svuotato in 60 s"; stop_all; exit 1; }; sleep 0.05; done
wait $OLDPID 2>/dev/null; RC=$?
T1=$(us)
echo "T0 segnale=$T0  T1 motore vecchio uscito=$T1 (rc=$RC)"; grep -E "DRAIN_START|DRAINED" out/old.log

# --- passo 2: Iggy 0.10.0 su directory vuota
NOSLEEP=1 ./reset.sh iggy
docker compose -p ttspikebus ps --format "iggy: {{.Image}} {{.Status}}"
T2=$(us)
echo "T2 nuovo Iggy avviato=$T2"

# --- passo 3-4: motore nuovo, riscrittura dello stato vivo, ripresa
tries=0
while :; do
  start_engine "$BIN" out/new.log busgen=2 upgrade=1
  wait_ready out/new.log $EP && break
  tries=$((tries+1)); [ $tries -ge 20 ] && { echo "motore nuovo non partito"; stop_all; exit 1; }
  sleep 0.5
done
T3=$(us)
echo "timer subito dopo READY: $("$BIN" stats dir=out | grep -E timers_pending | tr '\n' ' '); RSS: $(./rss.sh $EP | tr '\n' ' ')"
echo "T3 motore nuovo READY (script)=$T3 tentativi di avvio=$((tries+1))"; grep -E "READY|RIVERSAMENTO|ripresa|bus nuovo" out/new.log
i=0; until "$BIN" stats dir=out | grep -q "done=[1-9]"; do i=$((i+1)); [ $i -gt 600 ] && break; sleep 0.05; done
T4=$(us)
echo "T4 prima attività conclusa dopo la ripresa=$T4"
awk -v t0=$T0 -v t1=$T1 -v t2=$T2 -v t3=$T3 -v t4=$T4 'BEGIN{printf "pausa: svuotamento %.0f ms; nuovo Iggy %.0f ms; motore nuovo fino a READY %.0f ms; prima attività conclusa %.0f ms dopo READY; totale segnale -> READY %.0f ms, segnale -> prima conclusa %.0f ms\n", (t1-t0)/1e3, (t2-t1)/1e3, (t3-t2)/1e3, (t4-t3)/1e3, (t3-t0)/1e3, (t4-t0)/1e3}'

wait $LP; cat out/load.txt
echo "-- tempi del generatore di carico: segnale T0=$T0 READY T3=$T3"
# PRE100K: si aspetta che tutti i timer scattino (partono 150-170 s dopo l'accettazione), poi la verifica dei loro esiti
if [ -n "$PRE100K" ]; then
  i=0; until "$BIN" stats dir=out | grep -q "timers_pending=0"; do i=$((i+1)); [ $i -gt 120 ] && break; sleep 5; done
  echo "timer in sospeso dopo l'attesa: $("$BIN" stats dir=out | grep -E timers_pending | tr '\n' ' ') (dopo $((i*5)) s dalla fine del carico)"
  for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=iggy dir=out ledger=out/ledger-w.txt && break; sleep 3; done
fi
ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=iggy dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
# attività eseguite due volte e contatori del motore nuovo (blocco C), a corsa finita: registro dei worker, una riga per tentativo
echo "tentativi iniziati: $(wc -l < out/work.log), attività distinte: $(cut -d' ' -f1,2 out/work.log | sort -u | wc -l), eseguite più di una volta: $(cut -d' ' -f1,2 out/work.log | sort | uniq -d | wc -l), tentativi con numero 1 o più: $(awk '$5>=1' out/work.log | wc -l)"
echo "contatori del motore nuovo: $("$BIN" stats dir=out | head -1)"
echo "-- bus nuovo: $("$BIN" bus-info | tr '\n' ' ')"
echo "-- posizioni: $(sqlite3 out/spike.db 'select topic,pos,gen from projection_pos' 2>/dev/null | tr '\n' ' ')"
echo "-- log del motore vecchio (eventi): $(grep -cE 'DRAIN' out/old.log) righe; nuovo:"; grep -E "ripresa|RIVERSAMENTO|E_" out/new.log
echo "-- RSS dopo la corsa (motore, worker)"
./rss.sh $EP $WP
# prove di rifiuto della generazione: un bus vuoto con una generazione diversa non si legge con le posizioni di un altro bus
kill $EP 2>/dev/null; wait $EP 2>/dev/null; EP=
NOSLEEP=1 ./reset.sh iggy; sleep 4
timeout 20 $TS_ENG "$BIN" engine bus=iggy db="$DBURL" dir=out cap=$CAP trim=0 busgen=3 > out/neg1.log 2>&1
echo "rifiuto 1 (bus nuovo generazione 3, database alla 2, senza upgrade=1): rc=$? $(grep -m1 E_BUS_GENERATION out/neg1.log | cut -c1-160)"
NOSLEEP=1 ./reset.sh iggy; sleep 4
timeout 20 $TS_ENG "$BIN" engine bus=iggy db="$DBURL" dir=out cap=$CAP trim=0 busgen=1 upgrade=1 > out/neg2.log 2>&1
echo "rifiuto 2 (bus vuoto di generazione 1 con upgrade=1, database alla 2: generazione più vecchia): rc=$? $(grep -m1 E_BUS_GENERATION out/neg2.log | cut -c1-160)"
stop_all
rm -rf out/archive
