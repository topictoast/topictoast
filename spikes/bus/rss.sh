#!/bin/sh
# RSS (VmRSS di /proc, non cgroup) dei servizi del bus e di eventuali PID passati: ./rss.sh [pid...]
cd "$(dirname "$0")" || exit 1
[ -n "$WAIT" ] && sleep "$WAIT"
for s in iggy valkey postgres; do
  id=$(docker compose -p ttspikebus ps -q $s 2>/dev/null)
  [ -z "$id" ] && continue
  pid=$(docker inspect -f '{{.State.Pid}}' "$id")
  # somma di tutti i processi del container (valkey può avere un figlio durante il rewrite AOF)
  tot=0
  for p in $(pgrep -P "$pid") "$pid"; do
    kb=$(awk '/VmRSS/{print $2}' /proc/"$p"/status 2>/dev/null); tot=$((tot + ${kb:-0}))
  done
  echo "rss $s ${tot} kB"
done
for p in "$@"; do echo "rss pid=$p $(awk '/VmRSS/{print $2}' /proc/"$p"/status) kB"; done
