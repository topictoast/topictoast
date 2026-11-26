#!/bin/sh
# Fase 4: scoperta delle syscall consentite per ruolo (non c e strace su minisheep: si usa gdb `catch syscall`).
# Si lancia su minisheep: ./discover.sh   -> scrive allow-out/<ruolo>.txt (numero e nome), da copiare in allow/.
# Il marcatore e la syscall sysinfo (99), emessa dal ruolo al posto dei filtri (discover=1): lo script di gdb registra solo cio che la segue.
cd "$(dirname "$0")" || exit 1
. ./env.sh
export SPIKE_BIN=$HOME/ttspikesandbox/bin/sbxspike4
D=$HOME/ttspikesandbox/allow-out
rm -rf "$D" && mkdir -p "$D"
timeout 590 "$SPIKE_BIN" drive4 discover $SBX_ARGS dest="$D" || exit 1
for f in "$D"/discover-*.txt; do
  r=$(basename "$f" .txt); r=${r#discover-}
  awk '/^SYS / && $2 >= 0 && $2 != 99 { print $2 }' "$f" | sort -n -u |
    while read -r n; do printf "%s %s\n" "$n" "$(awk -v n="$n" '$1=="#define" && $3==n {sub("__NR_","",$2); print $2}' /usr/include/asm/unistd_64.h)"; done > "$D/$r.txt"
  echo "$r: $(wc -l < "$D/$r.txt") syscall"
done
