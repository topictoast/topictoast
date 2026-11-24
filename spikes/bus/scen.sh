#!/bin/sh
# Una corsa di misura: ./scen.sh <iggy|valkey> <sqlite|pg> <scenario> [chiave=valore ...]
# Parte da dati vuoti, avvia motore e worker (PID salvati), esegue lo scenario, spegne i suoi processi.
# Scenari: accept start1k thr10k thrmax size64k size1m join iso0 iso kill dbdown rss
cd "$(dirname "$0")" || exit 1
BUS=$1; DB=$2; SC=$3; shift 3
BIN=${SPIKE_BIN:-/home/white_sheep/.cache/cargo-targets/topictoast-spike-bus/release/busspike}
set -a; . ./.env; set +a
mkdir -p out
case $DB in
  sqlite) DBURL="sqlite://$PWD/out/spike.db?mode=rwc" ;;
  pg) DBURL="postgres://spike:$POSTGRES_PASSWORD@127.0.0.1:47003/spike" ;;
esac
CAP=${CAP:-512}; TRIM=1; PB=${PB:-1000}
# pinning a 4 CPU (2c): SPIKE_CPUS per motore e worker (taskset), SPIKE_CPUSET per i container, SPIKE_LOADCPUS per il carico
TS_ENG=${SPIKE_CPUS:+taskset -c $SPIKE_CPUS}; TS_LOAD=${SPIKE_LOADCPUS:+taskset -c $SPIKE_LOADCPUS}
EP=; WP=
# N scelto dalla verifica della fase 2d (mediana più alta di thrmax): Valkey 4, Iggy 1
WDEF=1; [ "$BUS" = valkey ] && WDEF=4
WL=; ENGPFX=
case $SC in kill|dbdown) TRIM=0 ;; size1m) PB=16 ;; size64k) PB=200 ;; ekill|bkill|wkill|dfull-iggy|dfull-db) TRIM=0; WL="worklog=out/work.log" ;; esac
us() { echo $(( $(date +%s%N) / 1000 )); }
# millisecondi tra due istanti in microsecondi
ms() { echo $(( ($2 - $1) / 1000 )); }
# valore di una riga del log del motore: <ETICHETTA> <us> ..., la prima oppure l'ultima
logus() { grep -E "^$1 " out/engine.log | $2 -1 | cut -d' ' -f2; }
# attende che compaia una riga nel log del motore (al più $2 secondi)
waitlog() { i=0; until grep -qE "$1" out/engine.log; do i=$((i+1)); [ $i -gt $(( $2 * 10 )) ] && return 1; sleep 0.1; done; }
# attende almeno $2 righe che corrispondono a $1 nel log del motore (al più $3 secondi)
waitn() { i=0; until [ "$(grep -cE "$1" out/engine.log)" -ge $2 ]; do i=$((i+1)); [ $i -gt $(( $3 * 10 )) ] && return 1; sleep 0.1; done; }
CAPSIZE=${CAPSIZE:-128m}; CAPFREE=${CAPFREE:-10}
# disco pieno di Iggy: volume tmpfs di docker con limite, montato dove Iggy scrive davvero (/app/local_data: il montaggio
# /local_data del compose non è quello dei dati), con un ciclo che rilancia il server quando esce
if [ "$SC" = dfull-iggy ]; then
  export IGGY_DATA=iggycap IGGY_MNT=/app/local_data IGGY_SUP="while true; do iggy-server; sleep 1; done"
  docker compose -p ttspikebus down -t 1 >/dev/null 2>&1; docker volume rm ttspikebus-iggycap >/dev/null 2>&1
  docker volume create --driver local --opt type=tmpfs --opt device=tmpfs --opt o=size=$CAPSIZE ttspikebus-iggycap >/dev/null
fi
# disco pieno di database e archivio: il motore gira in un namespace di mount proprio (senza sudo) con un tmpfs a
# dimensione limitata su out/capmnt, dove stanno spike.db e l'archivio; le prove si fanno con nsenter sul suo PID
if [ "$SC" = dfull-db ]; then
  mkdir -p out/capmnt
  [ $DB = sqlite ] && DBURL="sqlite://$PWD/out/capmnt/spike.db?mode=rwc"
  ENGPFX="unshare -Urm ./capns.sh $PWD/out/capmnt $CAPSIZE"
fi
NSE() { nsenter -U --preserve-credentials -m -t $EP "$@"; }

start_engine() {
  : > out/engine.log
  $ENGPFX $TS_ENG "$BIN" engine bus=$BUS db="$DBURL" dir=out cap=$CAP trim=$TRIM link=${LINK:-grpc} proj_batch=$PB wconn=${WCONN:-$WDEF} kgroups=${KGROUPS:-4} maxb=${MAXB:-1024} minb=${MINB:-64} grace_ms=${GRACE:-2000} >> out/engine.log 2>&1 &
  EP=$!
  i=0; until grep -q READY out/engine.log; do i=$((i+1)); [ $i -gt 300 ] && { echo "motore non partito"; cat out/engine.log; stop_all; exit 1; }; sleep 0.2; done
  echo "engine pid=$EP $(grep READY out/engine.log)"
}
stop_all() { [ -n "$WP" ] && kill $WP 2>/dev/null; [ -n "$EP" ] && kill $EP 2>/dev/null; wait 2>/dev/null; }

SERV="$BUS"; [ $DB = pg ] && SERV="$SERV postgres"
./reset.sh $SERV
if [ $DB = pg ]; then i=0; until docker compose -p ttspikebus exec -T postgres pg_isready -U spike >/dev/null 2>&1; do i=$((i+1)); [ $i -gt 60 ] && exit 1; sleep 1; done; sleep 2; fi
rm -rf out/archive out/spike.db* out/master.key out/ledger.txt
# archivio su un altro disco (variabile SPIKE_ARCHIVE): out/archive diventa un collegamento
[ -n "$SPIKE_ARCHIVE" ] && { rm -rf "$SPIKE_ARCHIVE"; mkdir -p "$SPIKE_ARCHIVE"; ln -s "$SPIKE_ARCHIVE" out/archive; }
# CAPDBONLY=1: solo il database nel tmpfs (l archivio resta sul disco vero), per provare il disco pieno del database da solo
[ "$SC" = dfull-db ] && [ -z "$CAPDBONLY" ] && ln -s "$PWD/out/capmnt/archive" out/archive
rm -f out/work.log
[ -f out/tls/cert.pem ] || "$BIN" gen-cert dir=out
[ -f out/worker.cred ] || "$BIN" gen-cred dir=out
echo "### $BUS + $DB: $SC $*"; echo "cpu: motore/worker/bus/db=${SPIKE_CPUS:-tutte} carico=${SPIKE_LOADCPUS:-tutte} wconn=${WCONN:-$WDEF} kgroups=${KGROUPS:-4} maxb=${MAXB:-1024} minb=${MINB:-64}"
start_engine
$TS_ENG "$BIN" worker dir=out link=${LINK:-grpc} sessions=${WSESS:-4} $WL > out/worker.log 2>&1 &
WP=$!
sleep 1
L="$TS_LOAD $BIN load dir=out"

case $SC in
  accept) $L rate=200 kind=0 n=1 size=1024 secs=30 warm=3 "$@" ;;
  start1k) $L rate=100 kind=0 n=10 size=1024 secs=40 warm=5 "$@" ;;
  thr10k)
    # Dal blocco B ogni corsa a 10.000 att./s ha il registro delle accettate (ledger) e la verifica «zero accettate perse»
    # (verify light: ledger contro database, niente scansione del bus, payload a campione), e il tipo di ogni errore.
    # Criterio di stallo, fissato prima delle corse: un minuto (riga MIN) con meno di 9.000 att./s, oppure un intervallo
    # senza accettazioni riuscite di oltre 5 s (accept_max_gap_ms), oppure ritardo di proiezione p99 oltre 1.000 ms.
    echo "criterio di stallo: minuto con act_s < 9000, oppure accept_max_gap_ms > 5000, oppure lag_p99_ms > 1000"
    $L rate=1000 kind=0 n=10 size=1024 secs=60 warm=5 tenants=ab ledger=out/ledger.txt "$@"
    echo "-- righe di errore del motore"; grep -vE "^(MIN|READY|ripresa|capns|FIRST_DONE)" out/engine.log | head -40
    for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done ;;
  thrmax) $L rate=2500 kind=0 n=10 size=1024 secs=30 warm=5 tenants=ab "$@" ;;
  size64k) $L rate=100 kind=0 n=5 size=65536 secs=30 warm=5 tenants=ab "$@" ;;
  size1m) $L rate=${RATE1M:-10} kind=0 n=5 size=1048576 secs=30 warm=5 tenants=ab "$@" ;;
  join) $L rate=50 kind=1 n=4 size=1024 work_ms=1500 secs=30 warm=5 tenants=ab "$@" ;;
  iso0) $L rate=100 kind=0 n=10 size=1024 tenants=b secs=30 warm=5 "$@" ;;
  iso)
    $L rate=400 kind=0 n=1 size=1024 work_ms=3000 tenants=a secs=40 warm=2 stats=0 > out/iso-a.txt &
    LA=$!
    sleep 3
    $L rate=100 kind=0 n=10 size=1024 tenants=b secs=30 warm=5 "$@"
    wait $LA; echo "-- tenant A (lento)"; cat out/iso-a.txt ;;
  sec)
    # prove di rifiuto (fase 3): ogni condizione di sicurezza ha un collegamento che deve essere rifiutato
    W="$TS_ENG $BIN worker dir=out link=${LINK:-grpc} once=1"
    "$BIN" gen-cert dir=out/other
    echo "buono (credenziale, identita e certificato giusti): $($W 2>&1 | tail -3 | tr "\n" " ")"
    echo "credenziale errata: $($W cred=00ff 2>&1 | tail -1)"
    if [ "${LINK:-grpc}" = grpc ]; then
      echo "il worker non verifica il server (certificato di un altro motore): $($W tlsdir=out/other 2>&1 | tail -1)"
    else
      echo "il worker non verifica il server (identita attesa di un altro nodo): $($W engine_id=$("$BIN" gen-nodeid) 2>&1 | tail -1)"
      echo "identita del nodo non derivata dalla credenziale: $($W badkey=1 2>&1 | tail -1)"
      echo "-- connessioni di rete di motore e worker (solo localhost ammesso):"
      ss -Htunap 2>/dev/null | grep -E "pid=($EP|$WP)," | awk "{print \$1, \$5, \$6}"
    fi ;;
  rss)
    sleep 60
    ./rss.sh $EP $WP ;;
  kill)
    $L rate=300 kind=0 n=10 size=1024 work_ms=100 secs=28 warm=1 stats=0 ledger=out/ledger.txt > out/load.txt &
    LP=$!
    for t in 8 5 6; do
      sleep $t
      echo "prima del kill: $("$BIN" stats dir=out | head -2 | tr '\n' ' ')"
      kill -9 $EP; wait $EP 2>/dev/null
      T0=$(date +%s.%N)
      sleep 1
      start_engine
      T1=$(date +%s.%N)
      D1=$(awk -v a="$T1" -v b="$T0" 'BEGIN{printf "%.3f", a-b}'); echo "kill -9 -> motore di nuovo in ascolto: $D1 s (1 s di attesa voluta inclusa)"
      # prima riconsegna/conclusione dopo la ripresa
      i=0; until "$BIN" stats dir=out | grep -q "done=[1-9]"; do i=$((i+1)); [ $i -gt 300 ] && break; sleep 0.1; done
      T2=$(date +%s.%N)
      D2=$(awk -v a="$T2" -v b="$T0" 'BEGIN{printf "%.3f", a-b}'); echo "kill -9 -> prima attività conclusa dopo la ripresa: $D2 s"
    done
    wait $LP; cat out/load.txt
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done ;;
  ret)
    # conservazione (fase 4): trim=1 con segmenti da 1 MiB (IGGY_SEGMENT_SIZE nell'ambiente), un kill -9 a metà.
    # Il proiettore esce con E_POSIZIONE_PERSA se la prima lettura non parte dalla posizione applicata.
    $L rate=${RRATE:-300} kind=0 n=10 size=1024 work_ms=100 secs=${RSECS:-90} warm=1 stats=0 tenants=ab ledger=out/ledger.txt "$@" > out/load.txt &
    LP=$!
    sleep 40
    echo "prima del kill: $("$BIN" bus-info | tr '\n' ' ')"
    cp out/engine.log out/engine-pre.log
    echo "E_POSIZIONE_PERSA prima del kill: $(grep -c E_POSIZIONE out/engine-pre.log)"
    kill -9 $EP; wait $EP 2>/dev/null
    start_engine
    echo "dopo la ripresa: $(grep -E 'ripresa' out/engine.log | tr '\n' ' ')"
    wait $LP; cat out/load.txt
    for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && break; sleep 3; done
    echo "E_POSIZIONE_PERSA dopo la ripresa: $(grep -c E_POSIZIONE out/engine.log)"
    echo "fine: $("$BIN" bus-info | tr '\n' ' ')" ;;
  dbdown)
    $L rate=300 kind=0 n=10 size=1024 work_ms=100 secs=28 warm=1 stats=0 ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 8
    if [ $DB = pg ]; then
      docker compose -p ttspikebus stop postgres >/dev/null 2>&1; echo "postgres fermato"
      sleep 8
      docker compose -p ttspikebus start postgres >/dev/null 2>&1; echo "postgres riavviato"
    else
      "$BIN" lock-db db="$DBURL" secs=10 &
      sleep 11
    fi
    wait $LP; cat out/load.txt; grep -E "proiezione" out/engine.log | head -4
    for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && break; sleep 3; done ;;
  ekill)
    # Blocco B, caso 1: crash del motore con attività lunghe in volo (300 att./s da 3 s: circa 900 in volo), riavvio subito
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 work_ms=3000 secs=30 warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 12
    echo "prima del kill: $("$BIN" stats dir=out | head -1)"
    cp out/engine.log out/engine-pre.log
    T0=$(us); kill -9 $EP; wait $EP 2>/dev/null
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill -9 -> READY: $(ms $T0 $RU) ms ($(grep -E '^READY' out/engine.log))"
    grep -E "ripresa" out/engine.log
    waitlog "^FIRST_DONE" 60; FD=$(logus FIRST_DONE head)
    echo "kill -9 -> prima attività conclusa: $(ms $T0 $FD) ms"
    wait $LP; cat out/load.txt
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "tentativi iniziati: $(wc -l < out/work.log), attività distinte: $(cut -d' ' -f1,2 out/work.log | sort -u | wc -l), eseguite più di una volta: $(cut -d' ' -f1,2 out/work.log | sort | uniq -d | wc -l)" ;;
  bkill)
    # Blocco B, caso 2: crash di Iggy (docker kill = SIGKILL del processo; non è uno spegnimento dell'host), riavvio dopo 5 s
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 work_ms=500 secs=40 warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 10
    echo "prima del kill: $("$BIN" stats dir=out | head -1)"
    T0=$(us); docker kill ttspikebus-iggy-1 >/dev/null; echo "T0 kill di Iggy=$T0"
    sleep 5
    T1=$(us); docker start ttspikebus-iggy-1 >/dev/null; echo "T1 avvio di Iggy=$T1"
    waitlog "^BUS_UP" 120; BU=$(logus BUS_UP tail)
    waitn "^FIRST_DONE" 2 60; FD=$(logus FIRST_DONE tail)
    echo "kill -> BUS_DOWN (motore): $(ms $T0 $(logus BUS_DOWN head)) ms; avvio -> BUS_UP: $(ms $T1 $BU) ms; kill -> BUS_UP: $(ms $T0 $BU) ms; BUS_UP -> prima attività conclusa: $(ms $BU $FD) ms; kill -> prima attività conclusa dopo il ritorno: $(ms $T0 $FD) ms"
    wait $LP; cat out/load.txt
    echo "-- eventi del motore sul bus"; grep -E "^(BUS_|APPEND_)" out/engine.log
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done ;;
  wkill)
    # Blocco B, caso 3: crash di un worker a metà attività (due worker; il secondo prende i tentativi che scadono)
    $TS_ENG "$BIN" worker dir=out link=grpc sessions=${WSESS:-4} $WL > out/worker2.log 2>&1 &
    W2=$!
    sleep 1
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 work_ms=3000 secs=30 warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 12
    echo "prima del kill: $("$BIN" stats dir=out | head -1)"
    T0=$(us); kill -9 $WP; wait $WP 2>/dev/null; echo "T0 kill del worker 1=$T0"; WP=$W2
    wait $LP; cat out/load.txt
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "tentativi iniziati: $(wc -l < out/work.log), attività distinte: $(cut -d' ' -f1,2 out/work.log | sort -u | wc -l), eseguite più di una volta: $(cut -d' ' -f1,2 out/work.log | sort | uniq -d | wc -l)"
    # tempo dal kill al secondo inizio di ogni attività eseguita due volte (minimo, mediana, massimo, in ms)
    sort -k4 -n out/work.log | awk -v t0=$T0 '{k=$1" "$2; if (k in s) {d[++n]=($4-t0)/1000}; s[k]=1} END{asort(d); if (n) printf "riesecuzioni: %d; kill -> nuovo inizio: min %.0f, mediana %.0f, max %.0f ms\n", n, d[1], d[int((n+1)/2)], d[n]; else print "riesecuzioni: 0"}' ;;
  dfull-iggy|dfull-db)
    # Blocco B, caso 4: disco pieno. 333 esecuzioni/s da 3 passi; zavorra che lascia ${CAPFREE} MB, attesa del rifiuto,
    # 8 s di osservazione, zavorra tolta, ripresa. iggy: tmpfs di docker (zavorra con docker exec); db: tmpfs nel namespace
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 secs=${DSECS:-30} warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 3
    # la zavorra riempie tutto lo spazio libero meno CAPFREE MB (lo spazio già usato dall'avvio non si conta due volte)
    if [ $SC = dfull-iggy ]; then AV=$(docker exec ttspikebus-iggy-1 df -m /app/local_data | tail -1 | tr -s " " | cut -d" " -f4); else AV=$(NSE df -m $PWD/out/capmnt | tail -1 | tr -s " " | cut -d" " -f4); fi
    MB=$(( AV - CAPFREE ))
    if [ $SC = dfull-iggy ]; then
      docker exec ttspikebus-iggy-1 sh -c "dd if=/dev/zero of=/app/local_data/ballast bs=1M count=$MB 2>&1 | tail -1; df -m /app/local_data | tail -1"
      FULLRE="^BUS_DOWN"; UPRE="^BUS_UP"
    else
      NSE sh -c "dd if=/dev/zero of=$PWD/out/capmnt/ballast bs=1M count=$MB 2>&1 | tail -1; df -m $PWD/out/capmnt | tail -1"
      FULLRE="^STORAGE_FULL"; UPRE="^proiezione ripresa"
    fi
    T0=$(us); echo "T0 zavorra messa=$T0 (restano ${CAPFREE} MB)"
    waitlog "$FULLRE" 90 || echo "il disco non si è riempito in 90 s"
    TF=$(logus $(echo $FULLRE | tr -d '^') head); echo "T_pieno=$TF (zavorra -> pieno $(ms $T0 ${TF:-$T0}) ms)"
    sleep 8
    if [ $SC = dfull-iggy ]; then docker exec ttspikebus-iggy-1 sh -c "df -m /app/local_data | tail -1; rm -f /app/local_data/ballast; df -m /app/local_data | tail -1"
    else NSE sh -c "df -m $PWD/out/capmnt | tail -1; rm -f $PWD/out/capmnt/ballast; df -m $PWD/out/capmnt | tail -1"; fi
    T1=$(us); echo "T1 zavorra tolta=$T1"
    # ripresa = primo evento di ritorno DOPO la zavorra tolta (con Iggy il ciclo lo rilancia anche a disco pieno, e crolla alla prima scrittura)
    after() { grep -E "$1" out/engine.log | awk -v t=$T1 '$NF > t {print $NF; exit}'; }
    i=0; TU=; while [ $i -lt 1200 ]; do TU=$(after "$UPRE"); [ -n "$TU" ] && break; i=$((i+1)); sleep 0.1; done
    echo "T_ripresa=$TU (zavorra tolta -> ripresa $(ms $T1 ${TU:-$T1}) ms)"
    i=0; FD=; while [ $i -lt 600 ]; do FD=$(grep -E "^FIRST_DONE" out/engine.log | awk -v t=$T1 '$2 > t {print $2; exit}'); [ -n "$FD" ] && break; i=$((i+1)); sleep 0.1; done
    echo "zavorra tolta -> prima attività conclusa dopo la ripresa: $(ms $T1 ${FD:-$T1}) ms"
    wait $LP; cat out/load.txt
    echo "-- eventi di disco e bus del motore"; grep -E "^(BUS_|APPEND_|STORAGE_FULL|proiezione)" out/engine.log | cut -c1-220 | head -20
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    if [ $SC = dfull-db ]; then
      ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do NSE "$BIN" verify db="$DBURL" bus=$BUS dir=$PWD/out ledger=$PWD/out/ledger.txt && ok=1 && break; sleep 3; done
    else
      ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    fi ;;
  *) echo "scenario sconosciuto" ;;
esac
[ $BUS = valkey ] && { echo "-- valkey: $(docker compose -p ttspikebus exec -T valkey valkey-cli -a "$VALKEY_PASSWORD" info stats 2>/dev/null | grep -E "total_forks" | tr -d "\r") $(docker compose -p ttspikebus exec -T valkey valkey-cli -a "$VALKEY_PASSWORD" info persistence 2>/dev/null | grep -E "aof_rewrites:|aof_last_rewrite_time_sec" | tr -d "\r" | tr "\n" " ")"; }
grep "^MIN" out/engine.log
echo "-- RSS dopo la corsa (motore, worker, bus)"
./rss.sh $EP $WP
echo "-- conteggio file archivio: $(find out/archive -type f 2>/dev/null | wc -l)"
stop_all
rm -rf out/archive; [ -n "$SPIKE_ARCHIVE" ] && rm -rf "$SPIKE_ARCHIVE"
if [ "$SC" = dfull-iggy ]; then docker compose -p ttspikebus down -t 1 >/dev/null 2>&1; docker volume rm ttspikebus-iggycap >/dev/null; fi
