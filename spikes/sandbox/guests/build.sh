#!/bin/sh
# Costruisce tutti i moduli di prova in $T/wasm (T = cartella target dello spike). Da lanciare sul portatile:
# i moduli .wasm si copiano poi su minisheep. Richiede il binario sbxspike già costruito (per i moduli WAT).
# Versioni: toolchain 1.99.1 (rust-toolchain.toml), javy 9.1.0 dal rilascio ufficiale (nessun crate per la CLI).
set -e
cd "$(dirname "$0")"
T=${SBX_TARGET:-$HOME/.cache/cargo-targets/topictoast-spike-sandbox-2}
export CARGO_TARGET_DIR=$T/guests
W=$T/wasm
mkdir -p "$W"
"$T/release/sbxspike" mkguests "$W"
(cd rustnode && cargo build --release --target wasm32-wasip1 --bin rustnode && cp "$T/guests/wasm32-wasip1/release/rustnode.wasm" "$W/rust-p1.wasm")
(cd rustnode && cargo build --release --target wasm32-wasip2 --bin rustnode && cp "$T/guests/wasm32-wasip2/release/rustnode.wasm" "$W/rust-p2.wasm")
(cd rustnode && cargo build --release --target wasm32-unknown-unknown --lib && cp "$T/guests/wasm32-unknown-unknown/release/rustnode.wasm" "$W/rust-unknown.wasm")
(cd probe && cargo build --release --target wasm32-wasip1 && cp "$T/guests/wasm32-wasip1/release/probe.wasm" "$W/probe.wasm")
(cd javy-plugin && cargo build --release --target wasm32-wasip1)
JAVY=$T/tools/javy/javy
if [ ! -x "$JAVY" ]; then
  # CLI di javy 9.1.0 dal rilascio ufficiale (non esiste un crate per la CLI), con l'impronta del rilascio
  mkdir -p "$T/tools/javy"
  curl -sL -o "$T/tools/javy/javy.gz" https://github.com/bytecodealliance/javy/releases/download/v9.1.0/javy-x86_64-linux-v9.1.0.gz
  echo "a68b122d48eb3dfc1b801d4e14c39271fde3638243d3272d206e376ac9189e39  $T/tools/javy/javy.gz" | sha256sum -c -
  gunzip -f "$T/tools/javy/javy.gz"
  chmod +x "$T/tools/javy/javy"
fi
"$JAVY" init-plugin "$T/guests/wasm32-wasip1/release/sbx_javy_plugin.wasm" -o "$W/plugin-init.wasm"
"$JAVY" build node.js -C plugin="$W/plugin-init.wasm" -o "$W/javy-static.wasm"
"$JAVY" build node.js -C dynamic=y -C plugin="$W/plugin-init.wasm" -o "$W/javy-dynamic.wasm"
ls -l "$W"
