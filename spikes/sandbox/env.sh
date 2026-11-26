#!/bin/sh
# Ambiente delle corse misurate su minisheep (fase 2 dello spike S-2). Si usa con `. ./env.sh`.
# Ausiliari sui core 1-4, pilota di carico sui core 8-11 (come nello spike del bus).
cd ~/ttspikesandbox || exit 1
export SPIKE_BIN=${ARM_BIN:-$HOME/ttspikesandbox/bin/sbxspike}
export SBX_AUX_CPUS=1-4 SBX_DRV_CPUS=8-11
export SBX_ARGS="wasm=$HOME/ttspikesandbox/wasm cache=$HOME/ttspikesandbox/out/cache out=$HOME/ttspikesandbox/out"
