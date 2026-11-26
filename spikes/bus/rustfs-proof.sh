#!/bin/sh
# Blocco D, riga 6: creazione solo se assente su RustFS. Due PUT firmati con curl (`--aws-sigv4`) sulla stessa chiave con
# `If-None-Match: *`: il primo deve riuscire, il secondo essere rifiutato (412), e l'oggetto salvato deve essere il primo.
# Controllo: lo stesso PUT senza l'intestazione sovrascrive (prova che è l'intestazione a fare il lavoro). Stampa solo
# codici di stato, byte e impronte, mai le credenziali. Uso: ./rustfs-proof.sh (dopo ./reset.sh rustfs)
cd "$(dirname "$0")" || exit 1
set -a; . ./.env; set +a
U=http://127.0.0.1:47004; W=out/rustfs-proof; B=spike-logs; K=log/tenant/exec-0-0-0000000001
rm -rf $W; mkdir -p $W
i=0; until [ "$(curl -s -o /dev/null -w '%{http_code}' $U/health)" = 200 ] || [ $i -gt 60 ]; do i=$((i+1)); sleep 1; done
echo "rustfs pronto dopo $i s"
# curl firma da solo host, data e impronta del contenuto (`x-amz-content-sha256` si passa a mano per l'S3)
s3() { # <metodo> <percorso> [file] [intestazione...]
  m=$1; p=$2; f=$3; shift 3
  if [ -n "$f" ]; then h=$(sha256sum "$f" | cut -d' ' -f1); set -- "$@" -H "x-amz-content-sha256: $h" --data-binary "@$f"
  else set -- "$@" -H "x-amz-content-sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"; fi
  curl -sS -o $W/resp.txt -w '%{http_code}' -X "$m" --aws-sigv4 "aws:amz:us-east-1:s3" --user "$RUSTFS_ACCESS_KEY:$RUSTFS_SECRET_KEY" "$@" "$U/$p"
}
head -c 4096 /dev/urandom > $W/first.bin; head -c 4096 /dev/urandom > $W/second.bin
echo "crea bucket: $(s3 PUT $B '')"
echo "PUT 1 (If-None-Match: *, contenuto 1): $(s3 PUT $B/$K $W/first.bin -H 'If-None-Match: *')"
echo "PUT 2 (If-None-Match: *, contenuto 2, stessa chiave): $(s3 PUT $B/$K $W/second.bin -H 'If-None-Match: *')"
cat $W/resp.txt | cut -c1-300; echo
echo "GET: $(s3 GET $B/$K '' )"; cp $W/resp.txt $W/got.bin
echo "impronte: primo=$(sha256sum $W/first.bin | cut -c1-16) secondo=$(sha256sum $W/second.bin | cut -c1-16) salvato=$(sha256sum $W/got.bin | cut -c1-16)"
[ "$(sha256sum < $W/got.bin)" = "$(sha256sum < $W/first.bin)" ] && echo "l'oggetto salvato è il primo" || echo "L'OGGETTO SALVATO NON È IL PRIMO"
echo "controllo, PUT 3 senza intestazione (deve sovrascrivere): $(s3 PUT $B/$K $W/second.bin)"
echo "GET dopo il controllo: $(s3 GET $B/$K '' )"; echo "salvato=$(sha256sum $W/resp.txt | cut -c1-16)"
rm -rf $W
