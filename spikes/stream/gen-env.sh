#!/bin/sh
# Genera credenziali casuali in $SPIKE_DATA/.env (una volta sola, fuori dal repository, mai stampate).
set -e
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikestream-data}
mkdir -p "$D"
[ -f "$D/.env" ] && { echo ".env esiste gia'"; exit 0; }
r() { head -c 24 /dev/urandom | base64 | tr -d '/+=\n'; }
umask 077
printf 'IGGY_ROOT_USERNAME=root%s\nIGGY_ROOT_PASSWORD=%s\nTT_TOKEN_ACME=%s\nTT_ADMIN_TOKEN=%s\n' "$(r | head -c 6)" "$(r)" "$(r)" "$(r)" > "$D/.env"
echo ".env creato (4 valori)"
