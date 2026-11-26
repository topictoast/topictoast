#!/bin/sh
# Estrae da risultati/*-ms-r*.txt una riga markdown per corsa: ./ms-tabella.sh <scenario> <db> [-noarc]
# p50, p99, max, campioni sopra 100 ms, carico e pressione I/O prima, CPU massima dei runner, attività/s.
cd "$(dirname "$0")/risultati" || exit 1
sc=$1; db=$2; tag=$3
case $sc in
  accept) re='^accept_engine' ;;
  join) re='^start_join' ;;
  iso0|iso) re='^start_ready_tenant_b' ;;
  bench) re='batch=1 conc=1' ;;
  *) re='^start_ready ' ;;
esac
for b in iggy valkey; do
  if [ "$sc" = bench ]; then pat="bench-$b-ms-r"; else pat="$b-$db-$sc$tag-ms-r"; fi
  for f in $(ls ${pat}*.txt 2>/dev/null | sort -V); do
    if [ "$sc" = bench ]; then l=$(grep -E "bus=$b size=1024 batch=1 conc=1" $f | head -1); else l=$(grep -E "$re" $f | head -1); fi
    p50=$(echo "$l" | sed -n 's/.*p50=\([0-9.]*\)ms.*/\1/p')
    p99=$(echo "$l" | sed -n 's/.*p99=\([0-9.]*\)ms.*/\1/p')
    mx=$(echo "$l" | sed -n 's/.* max=\([0-9.]*\)ms.*/\1/p')
    ov=$(echo "$l" | sed -n 's/.*over100ms=\([0-9]*\).*/\1/p')
    load=$(grep -m1 "stato prima.*load" $f | sed 's/.*load: \([0-9.]*\) .*/\1/')
    io=$(grep -m1 "stato prima\] io" $f | sed 's/.*avg10=\([0-9.]*\).*/\1/')
    rc=$(grep "cpu runner max" $f | sed 's/.* \([0-9.]*\)$/\1/' | sort -n | tail -1)
    rate=$(grep -m1 "^activities_per_s" $f | sed 's/activities_per_s=\([0-9]*\).*/\1/')
    echo "| $b | $(basename $f .txt) | $p50 | $p99 | $mx | $ov | $load | $io | $rc | $rate |"
  done
done
