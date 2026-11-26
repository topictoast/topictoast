#!/bin/sh
# Costruisce senza root il disco ext4 con l'agente (`mkfs.ext4 -d` popola l'immagine da una cartella, senza
# montarla): ./mkdrive.sh <binario sbxspike3> <immagine di uscita>
set -e
BIN=$1
IMG=$2
D=$(mktemp -d "${TMPDIR:-/tmp}/sbxdrive.XXXXXX")
cp "$BIN" "$D/sbxspike"
rm -f "$IMG"
truncate -s 64M "$IMG"
# ^has_journal: nessun giornale (il disco è montato in sola lettura); -E root_owner: file di root nel guest
mkfs.ext4 -q -F -O ^has_journal -E root_owner=0:0 -d "$D" "$IMG"
rm -r "$D"
sha256sum "$IMG"
