#!/bin/bash
# Campionatore di diagnosi (non misura nessuna riga): ogni 5 s scrive una riga con l'istante, i contatori di
# /proc/diskstats del disco dei dati (scritture, settori scritti, ms di scrittura, flush e ms di flush) e i tick di CPU
# (utente + sistema) di Iggy, del motore e del processo per tenant. Solo lettura di /proc, nessun privilegio.
# Uso: ./sampler.sh <file di uscita> <secondi> [disco, predefinito nvme0n1]
set -u
OUT=$1; DUR=$2; DEV=${3:-nvme0n1}
D=${SPIKE_DATA:-$HOME/.cache/ttspikestream-data}
ticks() { [ -n "$1" ] && [ -r "/proc/$1/stat" ] && awk '{print $14 + $15}' "/proc/$1/stat" || echo -; }
echo "# t_ms disco=$DEV scritture settori_scritti ms_scrittura flush ms_flush | tick_cpu iggy motore stream" > "$OUT"
end=$(( $(date +%s) + DUR ))
while [ "$(date +%s)" -lt "$end" ]; do
  # campi di /proc/diskstats: 8 scritture, 10 settori scritti, 11 ms di scrittura, 19 flush, 20 ms di flush
  ds=$(awk -v d="$DEV" '$3 == d {print $8, $10, $11, $19, $20}' /proc/diskstats)
  ig=$(pidof iggy-server | awk '{print $1}')
  en=$(cat "$D/run/t1/engine.pid" 2>/dev/null)
  st=$(cat "$D/engine/pids/stream-acme.pid" 2>/dev/null)
  echo "$(date +%s%3N) $ds | $(ticks "$ig") $(ticks "$en") $(ticks "$st")" >> "$OUT"
  sleep 5
done
