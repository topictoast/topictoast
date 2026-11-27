#!/bin/bash
# Cancello di quiete e registrazione dello stato (come spikes/sandbox/ms-run.sh e spikes/bus/ms-run.sh), da caricare con `. ./gate.sh`.
# Serve a smoke2.sh quando GATE=1 (su minisheep). Scrive con la funzione `say` del chiamante.
# Valido se: carico a 1 minuto < 1,5 e pressione I/O `some avg10` < 1 % (attesa fino a 15 minuti; poi parte e segna «macchina non ferma»).
# Registra prima e dopo: carico, pressione I/O e CPU, nomi di TUTTI i container in esecuzione (per mostrare i 12 dei runner
# intatti), sha256 del binario, memoria, orologio, governor, occupazione dei core assegnati (SPIKE_CPUS, SPIKE_LOADCPUS).

busy() {
  # occupazione media dei core elencati (formato 1-4,8-11) su un secondo, da /proc/stat
  local L=$1 a b
  L=$(echo "$L" | awk -F, '{for(i=1;i<=NF;i++){split($i,r,"-"); if(r[2]=="")r[2]=r[1]; for(c=r[1];c<=r[2];c++) printf "%d ", c}}')
  a=$(awk -v L="$L" 'BEGIN{n=split(L,c," ");for(i=1;i<=n;i++)w["cpu" c[i]]=1} $1 in w{t=0;for(i=2;i<=NF;i++)t+=$i;idle+=$5+$6;tot+=t} END{print idle, tot}' /proc/stat)
  sleep 1
  b=$(awk -v L="$L" 'BEGIN{n=split(L,c," ");for(i=1;i<=n;i++)w["cpu" c[i]]=1} $1 in w{t=0;for(i=2;i<=NF;i++)t+=$i;idle+=$5+$6;tot+=t} END{print idle, tot}' /proc/stat)
  echo "$a $b" | awk '{di=$3-$1; dt=$4-$2; if (dt>0) printf "%.1f%%", 100*(1-di/dt); else print "?"}'
}

state() {
  say "[stato $1 $(date +%T)] load: $(cut -d' ' -f1-3 /proc/loadavg)"
  say "[stato $1] io: $(head -1 /proc/pressure/io)"
  say "[stato $1] cpu: $(head -1 /proc/pressure/cpu)"
  # tre elenchi: i container dei runner di GitHub (nome `github-runner-*`, mai toccati), i container dello spike (progetto
  # compose ttspikestream) e gli altri, non nostri (es. i container di servizio di un job dei runner: nomi con prefisso esadecimale)
  local all own runners others
  all=$(docker ps --format '{{.Names}}' | sort)
  own=$(docker ps --filter label=com.docker.compose.project=ttspikestream --format '{{.Names}}' | sort)
  runners=$(echo "$all" | grep '^github-runner-')
  others=$(echo "$all" | grep -vxF -f <(echo "$own") | grep -v '^github-runner-')
  say "[stato $1] container in esecuzione ($(docker ps -q | wc -l)): runner_github=$(echo "$runners" | grep -c .) [$(echo $runners)] dello_spike=$(echo "$own" | grep -c .) [$(echo $own)] altri_non_nostri=$(echo "$others" | grep -c .) [$(echo $others)]"
  say "[stato $1] binario sha256: $(sha256sum "$BIN" | cut -d' ' -f1)"
  say "[stato $1] core assegnati: servizi=${SPIKE_CPUSET:-nessuno} motore_e_processi_per_tenant=${SPIKE_CPUS:-nessuno} generatore_e_oracolo=${SPIKE_LOADCPUS:-nessuno}"
  [ -n "${SPIKE_CPUSET:-}" ] && say "[stato $1] core ${SPIKE_CPUSET} occupati: $(busy "$SPIKE_CPUSET"); core ${SPIKE_LOADCPUS:-?} occupati: $(busy "${SPIKE_LOADCPUS:-0}")"
  local st; st=$(docker stats --no-stream --format '{{.Name}} {{.CPUPerc}}' 2>/dev/null | grep -v '^ttspikestream')
  say "[stato $1] cpu runner max: $(echo "$st" | tr -d '%' | sort -k2 -n -r | head -1)"
  say "[stato $1] governor cpu1: $(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_governor 2>/dev/null)"
  say "[stato $1] memoria: MemAvailable_kb=$(awk '/^MemAvailable/{print $2}' /proc/meminfo) swap_usato_kb=$(awk '/^SwapTotal/{t=$2} /^SwapFree/{f=$2} END{print t-f}' /proc/meminfo) $(head -1 /proc/pressure/memory)"
  say "[stato $1] disco: $(df -h "$D" | tail -1)"
  # pavimento di fdatasync sul disco dei dati (regole di lettura, validità punto 1: si registra, non è un cancello)
  local fp=$D/.pavimento-fsync.probe o secs
  o=$(LC_ALL=C dd if=/dev/zero of="$fp" bs=4k count=200 oflag=dsync 2>&1 | tail -1); rm -f "$fp"
  secs=$(echo "$o" | sed -n 's/.* copied, \([0-9.]*\) s.*/\1/p')
  say "[stato $1] pavimento fdatasync (dd 4 KiB x 200, oflag=dsync, su $D): $(awk -v s="${secs:-0}" 'BEGIN{printf "%.3f", s * 1000 / 200}') ms per scrittura"
  say "[stato $1] orologio UTC: $(date -u +%FT%T) kernel: $(uname -r)"
}

gate_wait() {
  local i=0 l io
  while :; do
    l=$(cut -d' ' -f1 /proc/loadavg)
    io=$(sed -n 's/^some avg10=\([0-9.]*\).*/\1/p' /proc/pressure/io)
    [ "$(awk -v l="$l" -v a="$io" 'BEGIN{print (l<1.5 && a<1.0)}')" = 1 ] && break
    i=$((i + 1))
    [ $i -gt 180 ] && { say "[macchina non ferma] dopo 15 minuti (load=$l io=$io)"; break; }
    sleep 5
  done
  say "[attesa del cancello] $((i * 5)) s"
}
