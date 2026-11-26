#!/bin/sh
# Corse di prova (smoke) su minisheep: ognuna con parametri ridotti, non conta per i verdetti.
# ./smoke.sh <riga>   (riga = warm|cold|mem|idle|limits|residue|sentinel|probe|compile|cache|l9)
cd "$(dirname "$0")" || exit 1
case $1 in
warm) ./run.sh L1 smoke warm n=100 warmup=10 cfgs=epoch,pool+epoch ;;
cold) ./run.sh L2 smoke cold n=10 ;;
mem) ./run.sh L3 smoke mem ns=1,10 ;;
idle) ./run.sh L4 smoke idle ns=1,5 ;;
limits) ./run.sh L5L6 smoke limits n=3 bgsecs=3 ;;
residue) ./run.sh L7 smoke residue ;;
sentinel) ./run.sh L8 smoke sentinel ;;
probe) ./run.sh L12 smoke probe ;;
compile) ./run.sh L10 smoke compile reps=1 ;;
cache) ./run.sh L11 smoke cache ;;
l9) ./run.sh L9 smoke l9 ;;
esac
