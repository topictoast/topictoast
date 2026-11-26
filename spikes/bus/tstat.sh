#!/bin/sh
# Blocco C: scarto di scatto dei timer dai registri della corsa (righe TSET/TFIRE del motore con TIMER_LOG, registro dei worker).
#   ./tstat.sh <retry|deadline> <registro motore prima del crash> <registro motore dopo> <registro dei worker> <READY us>
# retry:    il timer di ritentativo (tipo 1, tentativo 1) di ogni attività, contro l'inizio del tentativo 1 nel worker.
# deadline: la scadenza (tipo 0, tentativo 0) di ogni attività, contro l'inizio del tentativo 1 (la riconsegna) nel worker;
#           contano solo le attività che hanno una riconsegna (le altre hanno concluso prima della scadenza).
# Scarto = inizio nel worker meno scadenza; per i timer già scaduti alla ripresa si riporta anche inizio meno READY.
KIND=$1; PRE=$2; POST=$3; WLOG=$4; READY=$5
if [ "$KIND" = retry ]; then TK=1; TA=1; DA=1; else TK=0; TA=1; DA=0; fi
cat "$PRE" "$POST" | awk -v tk=$TK -v da=$DA '$1=="TSET" && $2==tk && $5==da {d[$3" "$4]=$6; n++} END{print n+0 > "/dev/stderr"; for (k in d) print k, d[k]}' 2> out/tstat.n > out/tstat.due
awk -v ta=$TA -v ready=$READY '
  NR==FNR { due[$1" "$2]=$3; next }
  $5==ta { k=$1" "$2; c[k]++; if (!(k in s) || $4<s[k]) s[k]=$4 }
  END {
    for (k in c) {
      if (!(k in due)) { sans++; continue }
      n++; if (c[k]>1) dbl++
      d=s[k]-due[k]; if (d<0) early++
      v[++m]=d/1000
      if (due[k] < ready) { od++; w[++mw]=(s[k]-ready)/1000 } else { w2[++mw2]=d/1000 }
    }
    asort(v); asort(w); asort(w2)
    i50=int((m+1)/2); i99=int(m*0.99); if (i99<1) i99=1
    printf "tentativi 1 iniziati: %d (con timer noto %d, senza %d); iniziati più di una volta: %d; iniziati prima della scadenza: %d\n", length(c), n, sans+0, dbl+0, early+0
    if (m) printf "inizio nel worker - scadenza, ms (tutti): p50 %.1f p99 %.1f max %.1f\n", v[i50], v[i99], v[m]
    if (mw) { j50=int((mw+1)/2); j99=int(mw*0.99); if (j99<1) j99=1; printf "scaduti prima di READY: %d; inizio nel worker - READY, ms: p50 %.1f p99 %.1f max %.1f\n", mw, w[j50], w[j99], w[mw] }
    if (mw2) { k50=int((mw2+1)/2); k99=int(mw2*0.99); if (k99<1) k99=1; printf "scaduti dopo READY: %d; inizio nel worker - scadenza, ms: p50 %.1f p99 %.1f max %.1f\n", mw2, w2[k50], w2[k99], w2[mw2] }
  }' out/tstat.due "$WLOG"
echo "timer programmati (righe TSET di questo tipo e tentativo): $(cat out/tstat.n)"
# scarto interno del motore (passata dei timer): TFIRE di questo tipo, istante di presa in carico meno scadenza
cat "$PRE" "$POST" | awk -v tk=$TK '$1=="TFIRE" && $2==tk {v[++m]=($7-$6)/1000} END{asort(v); i50=int((m+1)/2); i99=int(m*0.99); if (i99<1) i99=1; if (m) printf "TFIRE: %d timer scattati, presa in carico - scadenza ms: p50 %.1f p99 %.1f max %.1f\n", m, v[i50], v[i99], v[m]; else print "TFIRE: 0"}'
