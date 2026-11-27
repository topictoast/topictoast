#!/bin/bash
# Corse di prova (smoke) della parte 2a sul portatile: ./smoke2.sh <id> [seme]
# id: c1f c2f c3pair c4pair c5 c6 c7 v1 v2 v2ctrl v3 v3ctrl
# Non contano per i verdetti (regole di lettura): servono a vedere che il percorso funziona.
# Scrive risultati/stream-smoke-<nome>-laptop.txt (mai su un file esistente). Solo processi e container propri,
# fermati per PID salvato. I dati stanno su /home, mai su /tmp.
set -u
cd "$(dirname "$0")" || exit 1
HERE=$(pwd)
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikestream-data}
BIN=${BIN:-/home/white_sheep/.cache/cargo-targets/topictoast-spike-stream/release/streamspike}
ID=$1
SEED=${2:-21}
case $ID in
  c1f) NAME=c1final ;;
  c2f) NAME=c2final ;;
  v2ctrl) NAME=v2-ctrl ;;
  v3ctrl) NAME=v3-ctrl ;;
  a2ctrl) NAME=a2-ctrl ;;
  *) NAME=$ID ;;
esac
NAME=$NAME${RUNTAG:+-$RUNTAG}
PREFIX=${PREFIX:-stream-smoke}; MACH=${MACH:-laptop}
OUT=${OUT_FILE:-${OUTDIR:-$HERE/risultati}/$PREFIX-$NAME${NUM:+-$NUM}-$MACH.txt}
[ -e "$OUT" ] && [ -z "${OUT_APPEND:-}" ] && { echo "esiste gia': $OUT (un file grezzo non si riscrive: cambia il nome)"; exit 1; }
KEEP=$D/keep
set -a; . "$D/.env"; set +a
ADDR=${ADDR:-127.0.0.1:47111}
PIN_E=${SPIKE_CPUS:+taskset -c $SPIKE_CPUS}; PIN_L=${SPIKE_LOADCPUS:+taskset -c $SPIKE_LOADCPUS}
ENG_EXTRA=()

say() { echo "$@" | tee -a "$OUT"; }
adm() { curl -s -m 130 -w ' [http=%{http_code}]' -X "$1" -H "Authorization: Bearer $TT_ADMIN_TOKEN" ${3:+--data-binary "$3"} "http://$ADDR$2"; }
stats() { curl -s -m 10 -H "Authorization: Bearer $TT_ADMIN_TOKEN" "http://$ADDR/v1/stats"; }
header() {
  # KIND=misura per le corse misurate (script ms-row.sh); altrimenti corsa di prova
  if [ "${KIND:-smoke}" = misura ]; then
    say "MISURA $NAME corsa=${NUM:-?} data=$(date -u +%Y-%m-%dT%H:%M:%SZ) host=$(cat /proc/sys/kernel/hostname) core=$(nproc) seme=$SEED"
  else
    say "SMOKE $NAME (non conta per i verdetti) data=$(date -u +%Y-%m-%dT%H:%M:%SZ) host=$(cat /proc/sys/kernel/hostname) core=$(nproc)"
  fi
  [ -n "${PARAMS:-}" ] && say "$PARAMS"
  say "binario sha256=$(sha256sum "$BIN" | cut -d' ' -f1) seme=$SEED id=$ID"
  say "carico_1min_prima=$(cut -d' ' -f1 /proc/loadavg) pressione_io_prima=$(grep '^some' /proc/pressure/io | sed 's/.*avg10=\([0-9.]*\).*/\1/') container_in_esecuzione_prima=$(docker ps -q | wc -l)"
  [ -n "${GATE:-}" ] && state prima
}
footer() {
  # i container nostri si fermano con il nome del progetto, mai altri; l'elenco «dopo» deve essere uguale a quello «prima»
  [ -n "${FINAL_DOWN:-}" ] && docker compose -p ttspikestream --env-file "$D/.env" down >/dev/null 2>&1
  say "carico_1min_dopo=$(cut -d' ' -f1 /proc/loadavg) container_in_esecuzione_dopo=$(docker ps -q | wc -l)"
  [ -n "${GATE:-}" ] && state dopo
}

# avvio del motore: $1 = spec, resto = opzioni; il log del motore e il file dei PID stanno in $RUN
start_engine() {
  local spec=$1; shift
  mkdir -p "$D/engine" "$RUN"
  setsid nohup $PIN_E "$BIN" engine --spec "$spec" --listen $ADDR --data "$D/engine" --label "$(basename "$OUT")" "$@" >> "$RUN/engine.log" 2>&1 &
  echo $! > "$RUN/engine.pid"
  for _ in $(seq 1 100); do grep -q "engine pronto" "$RUN/engine.log" 2>/dev/null && return 0; sleep 0.2; done
  say "ERRORE: engine non pronto"; return 1
}
stop_engine() { [ -f "$RUN/engine.pid" ] && kill -9 "$(cat "$RUN/engine.pid")" 2>/dev/null; sleep 1; }
restart_engine() {
  local spec=$1; shift
  local n; n=$(grep -c "engine pronto" "$RUN/engine.log")
  setsid nohup $PIN_E "$BIN" engine --spec "$spec" --listen $ADDR --data "$D/engine" --label "$(basename "$OUT")" "$@" >> "$RUN/engine.log" 2>&1 &
  echo $! > "$RUN/engine.pid"
  for _ in $(seq 1 150); do [ "$(grep -c "engine pronto" "$RUN/engine.log")" -gt "$n" ] && return 0; sleep 0.2; done
  say "ERRORE: engine non ripartito"; return 1
}
kill_stream() { kill -9 "$(cat "$D/engine/pids/stream-acme.pid")" 2>/dev/null; }
# attende $1 secondi dall'istante $S0
wait_until() { while [ $(( $(date +%s) - S0 )) -lt "$1" ]; do sleep 0.2; done; }
# 5 kill -9 a istanti presi dal seme dentro finestre aperte: S E S S E (3 del processo per tenant, 2 del motore)
kills5() {
  local spec=$1
  RANDOM=$SEED
  local T=(); for _ in 1 2 3 4 5; do T+=($((8 + RANDOM % 43))); done
  T=($(printf '%s\n' "${T[@]}" | sort -n)); local KINDS=(S E S S E)
  say "istanti dei kill (s dall'inizio del carico): ${T[*]} tipi: ${KINDS[*]}"
  S0=$(date +%s)
  for i in 0 1 2 3 4; do
    wait_until "${T[$i]}"
    if [ "${KINDS[$i]}" = S ]; then kill_stream; say "kill -9 stream a t=$(( $(date +%s) - S0 ))s"; else stop_engine; say "kill -9 engine a t=$(( $(date +%s) - S0 ))s"; restart_engine "$spec" "${ENG_EXTRA[@]}"; fi
  done
}
# una corsa: label spec "argomenti del carico" modo-guasti [argomenti di check]
# modi: none | kills5 | quiet | hook:<funzione>
run_one() {
  local label=$1 spec=$2 load=$3 mode=$4; shift 4
  RUN=$D/run/$label
  say "=== corsa $label spec=$spec carico='$load' guasti=$mode motore='${ENG_EXTRA[*]}'"
  say "corsa_$label carico_1min_prima=$(cut -d' ' -f1 /proc/loadavg)"
  ./reset.sh
  start_engine "$spec" "${ENG_EXTRA[@]}" || return 1
  sleep 2
  local RD=$RUN/load; rm -rf "$RD"
  $PIN_L "$BIN" load --addr $ADDR --run-dir "$RD" $load > "$RUN/load.out" 2>&1 &
  local LOAD_PID=$!
  S0=$(date +%s)
  case $mode in
    kills5) kills5 "$spec" ;;
    hook:*) "${mode#hook:}" ;;
  esac
  wait $LOAD_PID
  case $mode in
    quiet) sleep 8; kill_stream; say "kill -9 stream durante la quiete (8 s dopo la fine del carico)" ;;
  esac
  say "--- carico ($label)"
  tail -n 12 "$RUN/load.out" | tee -a "$OUT"
  say "--- check ($label)"
  local extra=""
  [ -n "${CHECK_EXTRA_FN:-}" ] && extra=$($CHECK_EXTRA_FN)
  $PIN_L "$BIN" check --spec "$CHECKSPEC" --run-dir "$RD" --data "$D/engine" --addr $ADDR --wait-s "${WAIT:-240}" $extra "$@" 2>&1 | tee -a "$OUT"
  say "--- log del motore ($label, righe di riavvio)"
  grep -E "terminato|ripreso|sospesa|set_pipeline" "$RUN/engine.log" | sed 's/[0-9]\{4,\}/N/g' | sort | uniq -c | tee -a "$OUT"
  mkdir -p "$KEEP/$label"; cp "$RD/run.json" "$RD/rows.jsonl" "$KEEP/$label/" 2>/dev/null
  say "stats_finali_$label: $(stats | jq -c '.tenants.acme | {cpu_ns, rss_kb, pending, delivered, refused, pipes: [.pipes[] | {id, state, code, state_bytes, applied}]}')"
}

[ -z "${OUT_APPEND:-}" ] && : > "$OUT"
. ./gate.sh
[ -n "${GATE:-}" ] && gate_wait
# parametri di T1 in cima al file grezzo (decisi da Marco): ritmo offerto fisso, tempo dell'evento = ritmo offerto, origine alla partenza
case $ID in
  t1) PARAMS="T1 PARAMETRI: ritmo_offerto=105000 eventi/s FISSO (non senza tetto); ritmo_del_tempo_dell_evento=105000 eventi/s (uguale all'offerto); origine_del_tempo_dell_evento=alla partenza del carico (il generatore aspetta il confine della griglia da 300 s, fase 0, e parte li': la regola del futuro e la guardia del generatore non frenano e non scartano); connessioni=16; blocco=1000 eventi; durata=${T1_DUR:-600} s; query=UJ-3 con filtro (spec-t1.json); nessuna pipeline di verifica; ritardo del consumatore ogni 5 s; media sui minuti 2-10; chiusura delle finestre con la regola della quiete (nessun evento di chiusura: sarebbe oltre la tolleranza nel futuro di 60 s); seme=$SEED" ;;
  t1free) PARAMS="T1-FREE PARAMETRI (solo riportata): ritmo_offerto=senza tetto (--rate 0); ritmo_del_tempo_dell_evento=300000 eventi/s (sopra ogni ritmo plausibile: la guardia del generatore non frena; il tempo dell'evento resta indietro rispetto all'arrivo); origine=alla partenza (griglia da 300 s, fase 0); connessioni=16; blocco=1000 eventi; durata=${T1_DUR:-600} s; query=UJ-3 con filtro (spec-t1.json); ritardo del consumatore ogni 5 s; seme=$SEED" ;;
esac
header

case $ID in
  c1f)
    CHECKSPEC=spec-c1.json
    run_one c1f spec-c1.json "--seed $SEED --events 600000 --rate 20000 --evrate 500 --past-s 1500 --flush-s 315" none
    stop_engine ;;
  c2f)
    CHECKSPEC=spec-c1.json
    run_one c2f spec-c1.json "--seed $SEED --events 600000 --rate 20000 --evrate 500 --past-s 1500 --flush-s 315 --late-ppm 2000 --late-s 400 --fut-ppm 500 --fut-s 120 --far-n 12345" none
    stop_engine ;;
  c3pair)
    # C3, caso peggiore: fase fissa sulla griglia da 5 minuti, ultimo blocco trattenuto fino a E + 9 s (ritardo tollerato 10 s)
    CHECKSPEC=spec-c3.json
    L="--seed $SEED --events 300000 --rate 5000 --evrate 5000 --phase-s 0 --phase-period-s 300 --hold-last-s 9 --win-s 20"
    run_one c3a spec-c3.json "$L" none --all-closed; stop_engine
    run_one c3b spec-c3.json "$L" quiet --all-closed; stop_engine
    say "--- confronto della coppia (stesse righe con e senza il kill, dopo lo scarto dell'origine)"
    $PIN_L "$BIN" check --spec spec-c3.json --compare "$KEEP/c3a" "$KEEP/c3b" --period-s 300 2>&1 | tee -a "$OUT" ;;
  c4pair)
    # origine del tempo fissa: stessi eventi e stesse righe in ogni corsa con lo stesso seme
    CHECKSPEC=spec-c1.json
    O=$(( ($(date +%s) - 2100) / 300 * 300 * 1000 ))
    L="--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --origin-ms $O --flush-s 315"
    say "origine fissa del tempo dell'evento (ms): $O"
    run_one c4a spec-c1.json "$L" none; stop_engine
    run_one c4b spec-c1.json "$L" kills5; stop_engine
    say "--- confronto della coppia (righe con i guasti = righe senza guasti)"
    $PIN_L "$BIN" check --spec spec-c1.json --compare "$KEEP/c4a" "$KEEP/c4b" --period-s 300 2>&1 | tee -a "$OUT" ;;
  c5)
    # a) sospensione di 30 s e ripresa esatta sotto carico
    CHECKSPEC=spec-c1.json
    c5a_hook() {
      wait_until 8; say "t=$(( $(date +%s) - S0 ))s sospendo uj3v e hop"; adm POST /v1/admin/acme/pipelines/uj3v/suspend | tee -a "$OUT"; echo | tee -a "$OUT"; adm POST /v1/admin/acme/pipelines/hop/suspend | tee -a "$OUT"; echo | tee -a "$OUT"
      wait_until 38; say "t=$(( $(date +%s) - S0 ))s riprendo (sospese per 30 s)"; adm POST /v1/admin/acme/pipelines/uj3v/resume | tee -a "$OUT"; echo | tee -a "$OUT"; adm POST /v1/admin/acme/pipelines/hop/resume | tee -a "$OUT"; echo | tee -a "$OUT"
    }
    run_one c5a spec-c1.json "--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315" hook:c5a_hook
    stop_engine
    # b) CPU di una pipeline sospesa: un solo processo per tenant con la sola pipeline `solo`
    say "=== c5b CPU della pipeline sospesa. Definizione: tempo di CPU del processo per tenant (CLOCK_PROCESS_CPUTIME_ID letto dal processo stesso: utente + sistema, tutti i thread, anche quelli finiti, ns) tra due righe di statistiche a 30 s l'una dall'altra, con la sola pipeline sospesa nel processo; i segnali (1 al secondo sul log, non letti), i checkpoint (10 s, saltati se nulla e' cambiato) e le righe di statistiche (1 al secondo) sono dentro la misura. Controllo incrociato: utime+stime da /proc/<pid>/stat (tick da 10 ms)."
    CHECKSPEC=spec-c5b.json
    RUN=$D/run/c5b; ./reset.sh; start_engine spec-c5b.json || exit 1; sleep 2
    RD=$RUN/load; rm -rf "$RD"
    $PIN_L "$BIN" load --addr $ADDR --run-dir "$RD" --seed $SEED --events 300000 --rate 20000 --evrate 1000 --past-s 400 --flush-s 315 > "$RUN/load.out" 2>&1
    sleep 6
    say "sospendo solo: $(adm POST /v1/admin/acme/pipelines/solo/suspend)"
    sleep 12
    PID=$(cat "$D/engine/pids/stream-acme.pid")
    s0=$(stats | jq '.tenants.acme.cpu_ns'); t0=$(awk '{print $14+$15}' /proc/$PID/stat); cs0=$(grep -E '^(voluntary|nonvoluntary)_ctxt' /proc/$PID/status | awk '{s+=$2} END{print s}')
    sleep 30
    s1=$(stats | jq '.tenants.acme.cpu_ns'); t1=$(awk '{print $14+$15}' /proc/$PID/stat); cs1=$(grep -E '^(voluntary|nonvoluntary)_ctxt' /proc/$PID/status | awk '{s+=$2} END{print s}')
    say "c5b pid=$PID stato=$(stats | jq -c '.tenants.acme.pipes[0] | {state, code}') cpu_ns_inizio=$s0 cpu_ns_fine=$s1 delta_ms=$(awk -v a=$s0 -v b=$s1 'BEGIN{printf "%.3f", (b-a)/1e6}') tick_proc_stat_delta=$(( t1 - t0 )) (10 ms ciascuno) cambi_di_contesto_delta=$(( cs1 - cs0 )) soglia_ms=10"
    say "ripresa: $(adm POST /v1/admin/acme/pipelines/solo/resume)"
    say "--- check c5b (ripresa esatta dopo la sospensione lunga)"
    $PIN_L "$BIN" check --spec spec-c5b.json --run-dir "$RD" --data "$D/engine" --addr $ADDR --wait-s 240 2>&1 | tee -a "$OUT"
    stop_engine
    # c) sospensione oltre la conservazione: Iggy con segmenti da 1 MiB e cancellazione dei segmenti chiusi piu' vecchi
    say "=== c5c oltre la conservazione: lo stream indicators ha conservazione di 40 s (finestre da 20 s, ritardo 10 s); la conservazione e' simulata dal lettore sul tempo del log (ponytail nel codice); uj3v resta sospesa piu' di 40 s di tempo del log"
    CHECKSPEC=spec-c5c.json
    c5c_hook() { wait_until 5; say "t=$(( $(date +%s) - S0 ))s sospendo uj3v"; adm POST /v1/admin/acme/pipelines/uj3v/suspend | tee -a "$OUT"; echo | tee -a "$OUT"; }
    run_one c5c spec-c5c.json "--seed $SEED --events 300000 --rate 5000 --evrate 5000 --past-s 0" hook:c5c_hook
    say "ripresa dopo la perdita: $(adm POST /v1/admin/acme/pipelines/uj3v/resume)"
    sleep 3
    say "stato dopo la ripresa rifiutata: $(stats | jq -c '.tenants.acme.pipes[] | select(.id=="uj3v") | {state, code, applied}') altre: $(stats | jq -c '[.tenants.acme.pipes[] | select(.id!="uj3v") | .state] | unique')"
    say "--- check c5c dopo la ripresa rifiutata (uj3v ancora sospesa con il codice, nessuna riga in piu', le altre esatte)"
    $PIN_L "$BIN" check --spec spec-c5c2.json --run-dir "$D/run/c5c/load" --data "$D/engine" --addr $ADDR --wait-s 120 2>&1 | tee -a "$OUT"
    stop_engine
    # d) cambio di query: nuova versione con stato vuoto dalla posizione di lettura corrente
    say "=== c5d cambio della query: la pipeline chg passa dalla v1 alla v2 (HAVING count(*) > 50) a t=15 s; la v2 parte con lo stato vuoto"
    CHECKSPEC=spec-c5d-check.json
    c5d_extra() { echo "--from chg=$(grep -o 'set_pipeline chg v2 start_pos=[0-9]*' "$RUN/engine.log" | tail -1 | grep -o '[0-9]*$')"; }
    CHECK_EXTRA_FN=c5d_extra
    c5d_hook() {
      wait_until 15; say "t=$(( $(date +%s) - S0 ))s PUT chg v2"
      adm PUT /v1/admin/acme/pipelines/chg "{\"id\":\"chg\",\"version\":2,\"sql\":\"SELECT window_start, window_end, asn, count(*) AS n FROM indicators GROUP BY tumble(INTERVAL '5 minutes'), asn HAVING count(*) > 50\"}" | tee -a "$OUT"; echo | tee -a "$OUT"
    }
    run_one c5d_load spec-c5d.json "--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315" hook:c5d_hook
    stop_engine
    ;;
  c6)
    CHECKSPEC=spec-c1.json
    c6_hook() {
      wait_until 10; say "t=$(( $(date +%s) - S0 ))s il motore rifiuta gli avvii per quota per 20 s: $(adm POST '/v1/admin/refuse-starts?seconds=20')"
      wait_until 15; say "t=15s in attesa nel processo: $(stats | jq -c '.tenants.acme | {pending, refused}')"
      wait_until 28; say "t=28s in attesa nel processo: $(stats | jq -c '.tenants.acme | {pending, refused, delivered}')"
    }
    run_one c6 spec-c1.json "--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315" hook:c6_hook
    stop_engine ;;
  c7)
    CHECKSPEC=spec-c7.json
    run_one c7 spec-c7.json "--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315" kills5
    stop_engine ;;
  v1)
    say "--- probe v1"
    "$BIN" probe v1 --spec spec-v1.json 2>&1 | tee -a "$OUT" ;;
  v2)
    say "--- probe v2, tutte le barriere accese (prodotto)"
    "$BIN" probe v2 --spec spec-v1.json --label "$(basename "$OUT")" --work "$D" 2>&1 | tee -a "$OUT" ;;
  v2ctrl)
    say "--- probe v2, matrice dei controlli positivi: ogni barriera spenta da sola, accesa da sola, tutte spente"
    "$BIN" probe v2 --spec spec-v1.json --matrix --label "$(basename "$OUT")" --work "$D" 2>&1 | tee -a "$OUT" ;;
  v3)
    # produzione: quota di stato 64 MiB e RLIMIT_DATA 1 GiB accesi; la pipeline hog supera la quota
    CHECKSPEC=spec-v3.json
    v3_hook() {
      wait_until 25; say "t=25s validazione a secco di una query ostile (dialetto acceso): $(adm POST /v1/admin/acme/validate "{\"id\":\"hostile\",\"version\":1,\"sql\":\"SELECT repeat(kind, 8000000) AS r FROM indicators\"}")"
      say "t=26s motore vivo: $(stats | jq -c '{starts, dupes}')"
    }
    run_one v3 spec-v3.json "--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315" hook:v3_hook
    stop_engine ;;
  v3ctrl)
    # controlli: (a) barriera 1 spenta con quota e RLIMIT_DATA accesi: la validazione a secco ostile arriva alla memoria;
    # (b) niente quota di stato e niente RLIMIT_DATA, solo il pool di DataFusion
    CHECKSPEC=spec-v3.json
    v3a_hook() {
      wait_until 20; say "t=20s validazione a secco ostile con RLIMIT_DATA 1024 MiB (barriera 1 spenta, controllo): $(adm POST /v1/admin/acme/validate "{\"id\":\"hostile\",\"version\":1,\"sql\":\"SELECT repeat(kind, 8000000) AS r FROM indicators\"}")"
      say "t=21s motore vivo: $(stats | jq -c '{starts, dupes}')"
    }
    ENG_EXTRA=(--ctrl-off-syntax)
    run_one v3a spec-v3.json "--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315" hook:v3a_hook
    stop_engine
    v3b_hook() {
      for t in 10 20 30 40 50; do wait_until $t; say "t=${t}s controllo (solo pool di DataFusion): $(stats | jq -c '.tenants.acme | {rss_kb, pipes: [.pipes[] | select(.id=="hog") | {state, code, state_bytes}]}')"; done
      wait_until 55; say "validazione a secco ostile senza RLIMIT_DATA: $(adm POST /v1/admin/acme/validate "{\"id\":\"hostile\",\"version\":1,\"sql\":\"SELECT repeat(kind, 8000000) AS r FROM indicators\"}")"
      say "dopo la validazione: $(stats | jq -c '.tenants.acme | {rss_kb}')"
    }
    ENG_EXTRA=(--ctrl-off-syntax --no-quota --rlimit-data-mb 0)
    CHECKSPEC=spec-v3b.json
    run_one v3b spec-v3.json "--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315" hook:v3b_hook
    stop_engine ;;
  a1|a2|a2ctrl|i1|t1|t1free|t2|t3|m1|s1) . ./rows2b.sh ;;
  *) say "id sconosciuto"; exit 1 ;;
esac
footer
