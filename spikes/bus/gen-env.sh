#!/bin/sh
# Genera credenziali casuali in .env (una volta sola, mai nel repository).
set -e
[ -f .env ] && { echo ".env esiste già"; exit 0; }
r() { head -c 24 /dev/urandom | base64 | tr -d '/+=\n'; }
umask 077
printf 'IGGY_ROOT_USERNAME=root%s\nIGGY_ROOT_PASSWORD=%s\nVALKEY_PASSWORD=%s\nPOSTGRES_PASSWORD=%s\nRUSTFS_ACCESS_KEY=ak%s\nRUSTFS_SECRET_KEY=%s\n' "$(r | head -c 6)" "$(r)" "$(r)" "$(r)" "$(r | head -c 14)" "$(r)" > .env
