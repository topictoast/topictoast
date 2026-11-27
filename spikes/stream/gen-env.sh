#!/bin/sh
# Genera credenziali casuali in $SPIKE_DATA/.env (fuori dal repository, mai stampate).
# Parte 2b: se il file esiste ma manca il token del secondo tenant (TT_TOKEN_BETA, I1), lo aggiunge senza stamparlo.
set -e
D=${SPIKE_DATA:-/home/white_sheep/.cache/ttspikestream-data}
mkdir -p "$D"
r() { head -c 24 /dev/urandom | base64 | tr -d '/+=\n'; }
umask 077
if [ -f "$D/.env" ]; then
  grep -q '^TT_TOKEN_BETA=' "$D/.env" || printf 'TT_TOKEN_BETA=%s\n' "$(r)" >> "$D/.env"
  echo ".env esiste gia' (controllato il token del secondo tenant)"
  exit 0
fi
printf 'IGGY_ROOT_USERNAME=root%s\nIGGY_ROOT_PASSWORD=%s\nTT_TOKEN_ACME=%s\nTT_TOKEN_BETA=%s\nTT_ADMIN_TOKEN=%s\n' "$(r | head -c 6)" "$(r)" "$(r)" "$(r)" "$(r)" > "$D/.env"
echo ".env creato (5 valori)"
