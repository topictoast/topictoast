#!/bin/bash
# Corse di prova (smoke) sul portatile: ./smoke.sh c1a|c1b|c1c|c2|c3|c3k|c4|t1 [seme]
# Non contano per i verdetti (regole di lettura): servono a vedere che il percorso principale funziona.
# Scrive risultati/stream-smoke-<id>-laptop.txt. Solo processi e container propri, fermati per PID salvato.
set -u
cd "$(dirname "$0")" || exit 1
HERE=$(pwd)
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikestream-data}
BIN=/home/white_sheep/.cache/cargo-targets/topictoast-spike-stream/release/streamspike
ID=$1
SEED=${2:-1}
OUT=$HERE/risultati/stream-smoke-$ID-laptop.txt
RUN=$D/run/$ID
set -a; . "$D/.env"; set +a
ADDR=127.0.0.1:47111
ENG_PID=

say() { echo "$@" | tee -a "$OUT"; }

start_engine() {
  mkdir -p "$D/engine" "$RUN"
  setsid nohup "$BIN" engine --spec "$SPEC" --listen $ADDR --data "$D/engine" >> "$RUN/engine.log" 2>&1 &
  ENG_PID=$!
  echo $ENG_PID > "$RUN/engine.pid"
  for _ in $(seq 1 100); do grep -q "engine pronto" "$RUN/engine.log" 2>/dev/null && return 0; sleep 0.2; done
  say "ERRORE: engine non pronto"; return 1
}
stop_engine() { [ -f "$RUN/engine.pid" ] && kill -9 "$(cat "$RUN/engine.pid")" 2>/dev/null; sleep 1; }
# riavvia l'engine dopo un kill: il log si riempie di nuovo "engine pronto"
restart_engine() {
  local n; n=$(grep -c "engine pronto" "$RUN/engine.log")
  setsid nohup "$BIN" engine --spec "$SPEC" --listen $ADDR --data "$D/engine" >> "$RUN/engine.log" 2>&1 &
  ENG_PID=$!; echo $ENG_PID > "$RUN/engine.pid"
  for _ in $(seq 1 150); do [ "$(grep -c "engine pronto" "$RUN/engine.log")" -gt "$n" ] && return 0; sleep 0.2; done
  say "ERRORE: engine non ripartito"; return 1
}
kill_stream() { kill -9 "$(cat "$D/engine/pids/stream-acme.pid")" 2>/dev/null; }

: > "$OUT"
case $ID in
  c1a) SPEC=spec-c1.json; LOAD="--seed $SEED --events 600000 --rate 20000 --evrate 500 --past-s 1500 --flush-s 315"; KILL=none ;;
  c1b) SPEC=spec-c1.json; LOAD="--seed $SEED --events 600000 --rate 20000 --evrate 500 --past-s 1500 --flush-s 315"; KILL=none ;;
  c2)  SPEC=spec-c1.json; LOAD="--seed $SEED --events 600000 --rate 20000 --evrate 500 --past-s 1500 --flush-s 315 --late-ppm 2000 --late-s 400 --fut-ppm 500 --fut-s 120 --far-n 12345"; KILL=none ;;
  c3|c3k) SPEC=spec-c3.json; LOAD="--seed $SEED --events 300000 --rate 5000 --evrate 5000 --past-s 0"; KILL=none; [ "$ID" = c3k ] && KILL=quiet ;;
  c4)  SPEC=spec-c1.json; LOAD="--seed $SEED --events 1200000 --rate 20000 --evrate 1000 --past-s 1530 --flush-s 315"; KILL=c4 ;;
  t1)  SPEC=spec-t1.json; LOAD="--seed $SEED --events 40000000 --rate 0 --evrate 50000 --past-s 100 --duration-s 120 --conns 16"; KILL=none ;;
  *) echo "id sconosciuto"; exit 1 ;;
esac
# c1b e c1c si distinguono solo dal seme (terzo argomento di chiamata)

say "SMOKE $ID (non conta per i verdetti) data=$(date -u +%Y-%m-%dT%H:%M:%SZ) host=$(hostname) core=$(nproc)"
say "binario sha256=$(sha256sum "$BIN" | cut -d' ' -f1) spec=$SPEC seme=$SEED carico='$LOAD'"
say "carico_1min_prima=$(cut -d' ' -f1 /proc/loadavg) pressione_io_prima=$(grep '^some' /proc/pressure/io | sed 's/.*avg10=\([0-9.]*\).*/\1/')"
./reset.sh
start_engine || exit 1
sleep 2
mkdir -p "$RUN"
RD=$RUN/load
rm -rf "$RD"

"$BIN" load --addr $ADDR --run-dir "$RD" $LOAD > "$RUN/load.out" 2>&1 &
LOAD_PID=$!

case $KILL in
  c4)
    # 5 kill -9 a istanti presi dal seme dentro finestre aperte: S E S S E (3 del processo per tenant, 2 del motore)
    RANDOM=$SEED
    T=(); for _ in 1 2 3 4 5; do T+=($((8 + RANDOM % 43))); done
    T=($(printf '%s\n' "${T[@]}" | sort -n)); KINDS=(S E S S E)
    say "istanti dei kill (s dall'inizio del carico): ${T[*]} tipi: ${KINDS[*]}"
    S0=$(date +%s)
    for i in 0 1 2 3 4; do
      while [ $(( $(date +%s) - S0 )) -lt "${T[$i]}" ]; do sleep 0.2; done
      if [ "${KINDS[$i]}" = S ]; then kill_stream; say "kill -9 stream a t=$(( $(date +%s) - S0 ))s"; else stop_engine; say "kill -9 engine a t=$(( $(date +%s) - S0 ))s"; restart_engine; fi
    done ;;
esac
wait $LOAD_PID
case $KILL in
  quiet) sleep 8; kill_stream; say "kill -9 stream durante la quiete (8 s dopo la fine del carico)" ;;
esac
say "--- carico"
tail -n 30 "$RUN/load.out" | tee -a "$OUT"
say "--- check"
"$BIN" check --spec "$SPEC" --run-dir "$RD" --data "$D/engine" --addr $ADDR --wait-s ${WAIT:-240} 2>&1 | tee -a "$OUT"
say "--- log del motore (righe di riavvio)"
grep -E "terminato|ripreso" "$RUN/engine.log" | sort | uniq -c | tee -a "$OUT"
say "carico_1min_dopo=$(cut -d' ' -f1 /proc/loadavg)"
stop_engine
