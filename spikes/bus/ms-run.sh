#!/bin/sh
# Fase 2b-2d su minisheep: ./ms-run.sh <file di uscita> <comando...>
# Attende la macchina ferma (carico 1 min < 1,5 e pressione I/O some avg10 < 1 %; con GATE60=1 anche avg60 < 2 %),
# fino a 15 minuti; poi parte comunque e segna «macchina non ferma». Registra lo stato prima e dopo, compreso il
# pavimento di fdatasync, che su questo disco cambia nel tempo. Nessuna corsa si scarta dopo averla vista.
cd "$(dirname "$0")" || exit 1
OUT=$1; shift
mkdir -p "$(dirname "$OUT")"
state() {
  echo "[stato $1 $(date +%T)] load: $(cut -d' ' -f1-3 /proc/loadavg)"
  echo "[stato $1] io: $(head -1 /proc/pressure/io)"
  echo "[stato $1] cpu: $(head -1 /proc/pressure/cpu)"
  echo "[stato $1] container non nostri non in esecuzione stabile: $(docker ps -a --format '{{.Names}} {{.Status}}' | grep -v '^ttspikebus' | grep -v ' Up ' | wc -l); in esecuzione: $(docker ps --format '{{.Names}}' | grep -v '^ttspikebus' | wc -l)"
  [ -n "$SPIKE_BIN" ] && echo "[stato $1] pavimento fdatasync: $("$SPIKE_BIN" fsync-floor dir="$HOME/.cache" n=300)"
  # CPU dei container dei runner: oltre il 20 % di un core = job attivo
  echo "[stato $1] cpu runner max: $(docker stats --no-stream --format '{{.Name}} {{.CPUPerc}}' 2>/dev/null | grep -v '^ttspikebus' | tr -d '%' | sort -k2 -n -r | head -1)"
}
i=0
while :; do
  l=$(cut -d' ' -f1 /proc/loadavg)
  set -- $(sed -n 's/^some avg10=\([0-9.]*\) avg60=\([0-9.]*\).*/\1 \2/p' /proc/pressure/io) "$@"
  io=$1; io60=$2; shift 2
  [ "$(awk -v l="$l" -v a="$io" -v b="$io60" -v g="${GATE60:-0}" 'BEGIN{print (l<1.5 && a<1.0 && (g==0 || b<2.0))}')" = 1 ] && break
  i=$((i+1)); [ $i -gt 180 ] && { NONFERMA="macchina non ferma dopo 15 minuti (load=$l io=$io io60=$io60)"; break; }
  sleep 5
done
{ state prima; [ -n "$NONFERMA" ] && echo "[macchina non ferma] $NONFERMA"; echo "[attesa] $((i*5)) s"; "$@"; state dopo; } > "$OUT" 2>&1
grep -E "^(\[stato|\[macchina|###|offered|accept_rtt|accept_engine|start_ready |start_join|start_ready_tenant_b|activities_per_s|append|archive_read|bench-append|fsync-floor|MIN )" "$OUT"
