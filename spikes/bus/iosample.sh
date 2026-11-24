#!/bin/sh
# Campionatore dell'I/O, SOLA LETTURA (fase 4, blocco A): ./iosample.sh <file> [secondi]
# Ogni N secondi scrive: pressione I/O del sistema, contatori del disco (nvme0n1), byte scritti da ogni cgroup di
# container (io.stat, cumulativi) e memoria disponibile; ogni 30 s anche `docker stats --no-stream`.
# Non scrive nulla fuori da <file>, non tocca container né cgroup. Si ferma con il PID salvato dal chiamante.
OUT=$1; N=${2:-5}
docker ps --no-trunc --format '{{.ID}} {{.Names}}' > "$OUT.names" 2>/dev/null
i=0
while :; do
  t=$(date +%s)
  # i container nostri nascono dopo l'avvio del campionatore (reset.sh): la mappa id -> nome si rinnova a ogni giro
  docker ps --no-trunc --format '{{.ID}} {{.Names}}' >> "$OUT.names" 2>/dev/null
  echo "T $t io_some=$(sed -n 1p /proc/pressure/io | cut -d' ' -f2) io_full=$(sed -n 2p /proc/pressure/io | cut -d' ' -f2) memavail_kb=$(awk '/MemAvailable/{print $2}' /proc/meminfo)"
  # wr_ios wr_sectors wr_ms inflight io_ticks_ms rd_sectors
  awk '$3=="nvme0n1"{print "DEV", $8, $10, $11, $12, $13, $6}' /proc/diskstats
  # temperatura del disco (milligradi, sensore Composite): per l'ipotesi del rallentamento termico
  for h in /sys/class/hwmon/hwmon*; do [ "$(cat "$h/name" 2>/dev/null)" = nvme ] && echo "TEMP $(cat "$h/temp1_input")"; done
  for d in /sys/fs/cgroup/system.slice/docker-*.scope; do
    id=${d##*/docker-}; id=${id%.scope}
    awk -v id="$id" '$1=="259:0"{for(i=2;i<=NF;i++){split($i,a,"="); v[a[1]]=a[2]}; print "CG", id, v["wbytes"], v["rbytes"], v["wios"]; exit}' "$d/io.stat" 2>/dev/null
  done
  for s in system.slice user.slice; do
    awk -v s="$s" '$1=="259:0"{for(i=2;i<=NF;i++){split($i,a,"="); v[a[1]]=a[2]}; print "SL", s, v["wbytes"], v["rbytes"], v["wios"]; exit}' "/sys/fs/cgroup/$s/io.stat" 2>/dev/null
  done
  if [ $((i % 6)) = 0 ]; then docker stats --no-stream --format 'DS {{.Name}} cpu={{.CPUPerc}} blk={{.BlockIO}}' 2>/dev/null | sed "s/^/$t /"; fi
  i=$((i+1))
  sleep "$N"
done
