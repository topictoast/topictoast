#!/bin/sh
# Blocco A: per minuto, dai campioni dell'I/O di una corsa (risultati/<file>.io): ./an4.sh risultati/<file>.io
# Colonne: minuto; disco: MB/s scritti, utilizzo %, attesa media di scrittura (ms), operazioni di scrittura/s;
# MB/s scritti dal container di Iggy e da tutti gli altri container (runner compresi) e da user.slice (motore, worker,
# SQLite, archivio) e system.slice; massimo di io_some avg10 e memoria disponibile minima (GB) nel minuto.
f=$1
base=${f%.txt}.io.names
awk '
FNR==NR { nm[$1]=$2; next }
$1=="T" { t=$2; split($3,a,"="); some=a[3]+0; split($5,b,"="); mem=b[2]/1048576; if(t0==""){t0=t}; m=int((t-t0)/60); if(some>smax[m])smax[m]=some; if(!(m in memin)||mem<memin[m])memin[m]=mem; if(m>mm)mm=m; cur=m; next }
$1=="TEMP" { if($2/1000>tmax[cur])tmax[cur]=$2/1000; next }
$1=="DEV" { if(pd){ d[cur,"ios"]+=$2-pw; d[cur,"sec"]+=$3-ps; d[cur,"ms"]+=$4-pms; d[cur,"tick"]+=$6-pt } pw=$2; ps=$3; pms=$4; pt=$6; pd=1; next }
$1=="CG" { n=nm[$2]; k=(n ~ /^ttspikebus-iggy/)?"iggy":(n ~ /^ttspikebus/)?"nostro":"altri"; key="CG" $2; if(key in last){ cg[cur,k]+=$3-last[key] } last[key]=$3; next }
$1=="SL" { key="SL" $2; if(key in last){ sl[cur,$2]+=$3-last[key] } last[key]=$3; next }
END { printf "min | disco MB/s util%% wait_ms wios/s | iggy MB/s | altri container MB/s | user.slice MB/s | system.slice MB/s | io_some max | mem min GB | temp max C\n";
  for(m=0;m<=mm;m++){ ios=d[m,"ios"]; printf "%3d | %6.1f %5.1f %6.2f %7.0f | %6.1f | %6.2f | %6.1f | %6.1f | %5.1f | %5.1f | %4.1f\n", m+1, d[m,"sec"]*512/1048576/60, d[m,"tick"]/600, (ios>0?d[m,"ms"]/ios:0), ios/60, cg[m,"iggy"]/1048576/60, cg[m,"altri"]/1048576/60, sl[m,"user.slice"]/1048576/60, sl[m,"system.slice"]/1048576/60, smax[m], memin[m], tmax[m] }
}' "$base" "$f"
