#!/bin/sh
# Fase 3, una riga per corsa: ./an3.sh <scenario>  (legge risultati/iggy-sqlite-<scenario>-<link>-3-ms-r<n>.txt)
cd "$(dirname "$0")/risultati" || exit 1
for r in 1 2 3 4 5; do for l in grpc iroh; do
  f=iggy-sqlite-$1-$l-3-ms-r$r.txt; [ -f $f ] || continue
  ss=$(grep -m1 '^start_sent' $f | sed 's/.*p50=\([0-9.]*\)ms p99=\([0-9.]*\)ms.*/\1 \2/')
  sr=$(grep -m1 '^start_ready ' $f | sed 's/.*p50=\([0-9.]*\)ms p99=\([0-9.]*\)ms.*/\1 \2/')
  sb=$(grep -m1 '^start_ready_tenant_b' $f | sed 's/.*p50=\([0-9.]*\)ms p99=\([0-9.]*\)ms.*/\1 \2/')
  ap=$(grep -m1 '^activities_per_s' $f | sed 's/activities_per_s=\([0-9]*\).*/\1/')
  cp=$(grep -m1 'cpu_per_1000' $f | sed 's/.*engine=\([0-9.]*\) worker=\([0-9.]*\)/\1 \2/')
  fb=$(grep -m1 'prima\] pav' $f | sed 's/.*p50=\([0-9.]*\)ms.*/\1/'); fa=$(grep -m1 'dopo\] pav' $f | sed 's/.*p50=\([0-9.]*\)ms.*/\1/')
  if [ -n "$MD" ]; then set -- $1 $l r$r $ss $sr $ap $cp $fb $fa; echo "| $2 | $3 | $4 | $5 | $6 | $7 | $8 | $9 | ${10} | ${11} > ${12} |"; else echo "$1 $l r$r | sent $ss | ready $sr | B $sb | act/s $ap | cpu/1000 $cp | floor $fb>$fa"; fi
done; done
