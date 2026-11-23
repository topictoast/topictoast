#!/bin/sh
# Fase 2b su minisheep: ./ms-run.sh <file di uscita> <comando...>
# Attende la macchina ferma (carico 1 min < 1,5 e pressione I/O some avg10 < 1 %), registra lo stato
# prima e dopo, esegue il comando. Nessuna corsa si scarta: le corse con un job dei runner attivo si segnano.
cd "$(dirname "$0")" || exit 1
OUT=$1; shift
mkdir -p "$(dirname "$OUT")"
state() {
  echo "[stato $1 $(date +%T)] load: $(cut -d' ' -f1-3 /proc/loadavg)"
  echo "[stato $1] io: $(head -1 /proc/pressure/io)"
  echo "[stato $1] cpu: $(head -1 /proc/pressure/cpu)"
  echo "[stato $1] container non nostri non in esecuzione stabile: $(docker ps -a --format '{{.Names}} {{.Status}}' | grep -v '^ttspikebus' | grep -v ' Up ' | wc -l); in esecuzione: $(docker ps --format '{{.Names}}' | grep -v '^ttspikebus' | wc -l)"
  # CPU dei container dei runner: oltre il 20 % di un core = job attivo
  echo "[stato $1] cpu runner max: $(docker stats --no-stream --format '{{.Name}} {{.CPUPerc}}' 2>/dev/null | grep -v '^ttspikebus' | tr -d '%' | sort -k2 -n -r | head -1)"
}
i=0
while :; do
  l=$(cut -d' ' -f1 /proc/loadavg); io=$(sed -n 's/^some avg10=\([0-9.]*\).*/\1/p' /proc/pressure/io)
  [ "$(awk -v l="$l" -v io="$io" 'BEGIN{print (l<1.5 && io<1.0)}')" = 1 ] && break
  i=$((i+1)); [ $i -gt 120 ] && { NONFERMA="macchina non ferma dopo 10 minuti (load=$l io=$io)"; break; }
  sleep 5
done
{ state prima; [ -n "$NONFERMA" ] && echo "[macchina non ferma] $NONFERMA"; echo "[attesa] $((i*5)) s"; "$@"; state dopo; } > "$OUT" 2>&1
grep -E "^(\[stato|###|offered|accept_rtt|accept_engine|start_ready |start_join|start_ready_tenant_b|activities_per_s|append|archive_read|bench-append|fsync-floor)" "$OUT"
