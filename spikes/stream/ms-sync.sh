#!/bin/bash
# Dal portatile: copia su minisheep (~/ttspikestream) il binario e gli script di questa cartella.
# Stessa glibc su tutte e due le macchine (2.44, Arch): il binario si copia cosi' com'e', come per gli spike del bus e della
# sandbox. Un solo comando ssh alla volta, ognuno in primo piano. Uso: ./ms-sync.sh
cd "$(dirname "$0")" || exit 1
T=${CARGO_TARGET_DIR:-/home/white_sheep/.cache/cargo-targets/topictoast-spike-stream}
ssh -o BatchMode=yes minisheep 'mkdir -p ~/ttspikestream/bin ~/ttspikestream/risultati ~/ttspikestream/allow' || exit 1
scp -q "$T/release/streamspike" "$T/release/idle_nodf" minisheep:ttspikestream/bin/ || exit 1
# gli script e le specifiche (non i sorgenti, non i risultati del portatile); allow/ comprende la lista scoperta
tar -C . --exclude=./src --exclude=./allow-out --exclude=./risultati --exclude=./target -cf - . | ssh -o BatchMode=yes minisheep 'tar -C ~/ttspikestream -xf -' || exit 1
ssh -o BatchMode=yes minisheep 'cd ~/ttspikestream && chmod +x *.sh bin/* && sha256sum bin/streamspike bin/idle_nodf && ls | head -50'
