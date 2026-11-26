#!/bin/sh
# Corsa misurata su minisheep: ./ms-run.sh <file di uscita> <comando...>
# Come spikes/bus/ms-run.sh: attende la macchina ferma (carico 1 min < 1,5 e pressione I/O some avg10 < 1 %)
# fino a 15 minuti; poi parte comunque e segna «macchina non ferma». Registra lo stato prima e dopo, compreso
# l'impronta sha256 del binario e l'occupazione dei core 1-4 e 8-11 in un secondo. Nessuna corsa si scarta
# dopo averla vista. I container dei runner si leggono soltanto.
cd "$(dirname "$0")" || exit 1
OUT=$1
shift
mkdir -p "$(dirname "$OUT")"
busy() {
  # occupazione media dei core elencati su un secondo, da /proc/stat
  a=$(awk -v L="$1" 'BEGIN{n=split(L,c," ");for(i=1;i<=n;i++)w["cpu" c[i]]=1} $1 in w{t=0;for(i=2;i<=NF;i++)t+=$i;idle+=$5+$6;tot+=t} END{print idle, tot}' /proc/stat)
  sleep 1
  b=$(awk -v L="$1" 'BEGIN{n=split(L,c," ");for(i=1;i<=n;i++)w["cpu" c[i]]=1} $1 in w{t=0;for(i=2;i<=NF;i++)t+=$i;idle+=$5+$6;tot+=t} END{print idle, tot}' /proc/stat)
  echo "$a $b" | awk '{di=$3-$1; dt=$4-$2; if (dt>0) printf "%.1f%%", 100*(1-di/dt); else print "?"}'
}
state() {
  echo "[stato $1 $(date +%T)] load: $(cut -d' ' -f1-3 /proc/loadavg)"
  echo "[stato $1] io: $(head -1 /proc/pressure/io)"
  echo "[stato $1] cpu: $(head -1 /proc/pressure/cpu)"
  echo "[stato $1] container in esecuzione (non nostri: non ne abbiamo): $(docker ps --format '{{.Names}}' | wc -l)"
  echo "[stato $1] braccio: ${ARM:-non dichiarato}"
  echo "[stato $1] binario sha256: $(sha256sum "$SPIKE_BIN" | cut -d' ' -f1)"
  echo "[stato $1] moduli sha256 (insieme): $(cat wasm/*.wasm | sha256sum | cut -d' ' -f1)"
  echo "[stato $1] core 1-4 occupati: $(busy '1 2 3 4'); core 8-11 occupati: $(busy '8 9 10 11')"
  st=$(docker stats --no-stream --format '{{.Name}} {{.CPUPerc}}' 2>/dev/null)
  echo "[stato $1] cpu runner max: $(echo "$st" | tr -d '%' | sort -k2 -n -r | head -1)"
  echo "[stato $1] governor cpu1: $(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_governor 2>/dev/null)"
  echo "[stato $1] memoria: MemAvailable_kb=$(awk "/^MemAvailable/{print \$2}" /proc/meminfo) swap_usato_kb=$(awk "/^SwapTotal/{t=\$2} /^SwapFree/{f=\$2} END{print t-f}" /proc/meminfo) $(head -1 /proc/pressure/memory)"
  echo "[stato $1] orologio minisheep UTC: $(date -u +%FT%T)"
  echo "[stato $1] processi firecracker visibili (non nostri prima della corsa): $(pgrep -c firecracker 2>/dev/null)"
  echo "[stato $1] ptrace_scope: $(cat /proc/sys/kernel/yama/ptrace_scope 2>/dev/null || echo assente); kernel: $(uname -r)"
}
i=0
while :; do
  l=$(cut -d' ' -f1 /proc/loadavg)
  set -- $(sed -n 's/^some avg10=\([0-9.]*\) avg60=\([0-9.]*\).*/\1 \2/p' /proc/pressure/io) "$@"
  io=$1
  shift 2
  [ "$(awk -v l="$l" -v a="$io" 'BEGIN{print (l<1.5 && a<1.0)}')" = 1 ] && break
  i=$((i + 1))
  [ $i -gt 180 ] && {
    NONFERMA="macchina non ferma dopo 15 minuti (load=$l io=$io)"
    break
  }
  sleep 5
done
{
  state prima
  [ -n "$NONFERMA" ] && echo "[macchina non ferma] $NONFERMA"
  echo "[attesa] $((i * 5)) s"
  "$@"
  echo "[uscita] $?"
  state dopo
} >"$OUT" 2>&1
grep -E "^(\[stato prima|\[macchina|\[uscita|L[0-9]|S[0-9])" "$OUT" | cut -c1-200 | head -${SHOW:-25}
