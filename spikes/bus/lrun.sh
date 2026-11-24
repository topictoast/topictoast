#!/bin/sh
# Prova funzionale locale, non misurata (sul desktop non si misura): ./lrun.sh <comando...> con .env caricato.
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
exec "$@"
