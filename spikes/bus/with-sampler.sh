#!/bin/sh
# ./with-sampler.sh <file campioni> <comando...>: avvia iosample.sh, esegue il comando, ferma solo il campionatore (PID salvato).
cd "$(dirname "$0")" || exit 1
F=$1; shift
./iosample.sh "$F" 5 &
SP=$!
"$@"
RC=$?
kill $SP 2>/dev/null; wait $SP 2>/dev/null
exit $RC
