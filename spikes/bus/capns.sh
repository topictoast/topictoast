#!/bin/sh
# Blocco B, disco pieno di database e archivio: ./capns.sh <punto di mount> <dimensione> <comando...>
# Da lanciare con `unshare -Urm` (namespace utente e di mount propri, senza sudo): monta un tmpfs a dimensione limitata
# sul punto dato, vede solo questo processo e i suoi figli, e lancia il comando (che prende il posto di questo processo,
# stesso PID, quindi `nsenter -U -m -t <PID>` entra nello stesso mount). Rifiuta di partire se il mount non è un tmpfs
# della dimensione chiesta: il filesystem vero non si riempie mai.
M=$1; S=$2; shift 2
mount -t tmpfs -o size=$S tmpfs "$M" || exit 1
[ "$(stat -f -c %T "$M")" = tmpfs ] || { echo "capns: $M non è un tmpfs"; exit 1; }
mkdir -p "$M/archive"
echo "capns: $M tmpfs $S, $(df -m "$M" | tail -1 | tr -s ' ')"
exec "$@"
