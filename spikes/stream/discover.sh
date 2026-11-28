#!/bin/bash
# Scoperta delle syscall del ruolo `stream` (A2): ./discover.sh [carico-secondi]
# Come spikes/sandbox/discover.sh (non c'e' strace su minisheep: gdb `catch syscall`), ma sul lavoro vero del ruolo:
# DataFusion vero, client di Iggy, checkpoint, validazione a secco, nuova versione, sospensione e ripresa, kill del
# processo per tenant, kill del motore e riavvio di Iggy (la riconnessione deve stare nella lista).
# Il processo `stream --discover` gira sotto gdb (con ptrace_scope 1 si puo' solo lanciare, non agganciare) ma senza
# catchpoint: al posto di Landlock e seccomp si ferma da solo (SIGSTOP), lo script di gdb imposta allora `catch syscall`
# e registra ogni syscall DOPO quel punto. Con il catchpoint dall'avvio gdb non finiva (provato: 1,2 milioni di `clone3`
# in 6 minuti, contro 17 thread stabili senza gdb). I `commands` dei catchpoint non servono (con gdb 18 lo script
# finisce al primo stop): un ciclo `while` con `continue` esplicito.
# Scrive allow-out/stream.txt (numero e nome) da copiare in allow/stream.txt. Solo processi e container propri, fermati
# per PID salvato.
cd "$(dirname "$0")" || exit 1
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikestream-data}
BIN=${BIN:-/home/white_sheep/.cache/cargo-targets/topictoast-spike-stream/release/streamspike}
PIN_E=${SPIKE_CPUS:+taskset -c $SPIKE_CPUS}; PIN_L=${SPIKE_LOADCPUS:+taskset -c $SPIKE_LOADCPUS}
OUTD=${OUTD:-$(pwd)/allow-out}
SECS=${1:-40}
ADDR=127.0.0.1:47111
set -a; . "$D/.env"; set +a
rm -rf "$OUTD"; mkdir -p "$OUTD"
RUN=$D/run/discover
./reset.sh; mkdir -p "$RUN"; : > "$RUN/engine.log"
LOG=$OUTD/gdb-stream.log
cat > "$OUTD/gdb.cmd" <<EOF
set pagination off
set confirm off
set startup-with-shell off
handle SIGSTOP stop print nopass
handle SIGSEGV SIGBUS SIGILL SIGFPE nostop noprint pass
set logging file $LOG
set logging redirect on
set logging enabled on
run
catch syscall
while 1
continue
printf "SYS %d\n", \$orig_rax
end
EOF
say() { echo "$@"; }
adm() { curl -s -m 130 -X "$1" -H "Authorization: Bearer $TT_ADMIN_TOKEN" ${3:+--data-binary "$3"} "http://$ADDR$2"; echo; }
start_engine() {
  local n; n=$(grep -c "engine pronto" "$RUN/engine.log" 2>/dev/null); n=${n:-0}
  setsid nohup $PIN_E "$BIN" engine --spec spec-c1.json --listen $ADDR --data "$D/engine" --discover "$OUTD/gdb.cmd" >> "$RUN/engine.log" 2>&1 &
  echo $! > "$RUN/engine.pid"
  for _ in $(seq 1 150); do [ "$(grep -c "engine pronto" "$RUN/engine.log")" -gt "$n" ] && return 0; sleep 0.2; done
  say "ERRORE: engine non pronto"; return 1
}
wait_ready() { # attende un nuovo «pronto» del processo per tenant ($1 = numero gia' visto)
  for _ in $(seq 1 600); do [ "$(grep -c 'stream acme pronto' "$RUN/engine.log")" -gt "$1" ] && return 0; sleep 0.2; done
  say "ERRORE: processo per tenant non pronto"; return 1
}
start_engine || exit 1
wait_ready 0
sleep 2
$PIN_L "$BIN" load --addr $ADDR --run-dir "$RUN/load" --seed 7 --events 100000000 --rate 3000 --evrate 3000 --past-s 600 --duration-s "$SECS" --conns 2 > "$RUN/load.out" 2>&1 &
LP=$!
t() { sleep "$1"; }
t 6
say "validazione a secco: $(adm POST /v1/admin/acme/validate '{"id":"uj3x","version":1,"sql":"SELECT window_start, window_end, asn, count(*) AS n FROM indicators GROUP BY tumble(INTERVAL '"'5 minutes'"'), asn HAVING count(*) > 50"}')"
say "validazione ostile: $(adm POST /v1/admin/acme/validate '{"id":"h","version":1,"sql":"SELECT repeat(kind, 8000000) AS r FROM indicators"}')"
say "sospendo: $(adm POST /v1/admin/acme/pipelines/hop/suspend)"
t 3
say "riprendo: $(adm POST /v1/admin/acme/pipelines/hop/resume)"
say "nuova versione: $(adm PUT /v1/admin/acme/pipelines/dist '{"id":"dist","version":2,"sql":"SELECT DISTINCT window_start, window_end, kind FROM indicators WHERE asn < 1100 GROUP BY tumble(INTERVAL '"'5 minutes'"'), kind, asn"}')"
t 12   # un checkpoint (10 s) con lo stato
N=$(grep -c 'stream acme pronto' "$RUN/engine.log")
kill -9 "$(cat "$D/engine/pids/stream-acme.pid")" 2>/dev/null; say "kill -9 del processo per tenant (gdb) a $(date +%T)"
wait_ready "$N"
t 6
N=$(grep -c 'stream acme pronto' "$RUN/engine.log")
docker compose -p ttspikestream --env-file "$D/.env" restart iggy >/dev/null 2>&1; say "riavvio di Iggy a $(date +%T)"
t 12
kill -9 "$(cat "$RUN/engine.pid")" 2>/dev/null; say "kill -9 del motore a $(date +%T)"
sleep 1
start_engine && N=$(grep -c 'stream acme pronto' "$RUN/engine.log") && wait_ready $((N - 1))
t 8
wait $LP
tail -n 2 "$RUN/load.out"
kill -9 "$(cat "$RUN/engine.pid")" 2>/dev/null; sleep 1
[ -s "$LOG" ] || { say "ERRORE: nessun registro di gdb"; exit 1; }
awk '/^SYS / && $2 ~ /^[0-9]+$/ { print $2 }' "$LOG" | sort -n -u |
  while read -r n; do printf "%s %s\n" "$n" "$(awk -v n="$n" '$1=="#define" && $3==n {sub("__NR_","",$2); print $2}' /usr/include/asm/unistd_64.h)"; done > "$OUTD/stream.txt"
say "stream: $(wc -l < "$OUTD/stream.txt") syscall (registro gdb: $(grep -c '^SYS ' "$LOG") righe, sessioni di gdb: $(grep -c '^Program received signal SIGSTOP' "$LOG"))"
say "avvii in modalita' di scoperta (pronto): $(grep -c 'level=discover' "$RUN/engine.log")"
docker compose -p ttspikestream --env-file "$D/.env" down >/dev/null 2>&1
