#!/bin/sh
# Una corsa di misura: ./scen.sh <iggy|valkey> <sqlite|pg> <scenario> [chiave=valore ...]
# Parte da dati vuoti, avvia motore e worker (PID salvati), esegue lo scenario, spegne i suoi processi.
# Scenari: accept start1k thr10k thrmax size64k size1m join iso0 iso kill dbdown rss
cd "$(dirname "$0")" || exit 1
BUS=$1; DB=$2; SC=$3; shift 3
BIN=${SPIKE_BIN:-/home/white_sheep/.cache/cargo-targets/topictoast-spike-bus/release/busspike}
set -a; . ./.env; set +a
mkdir -p out
case $DB in
  sqlite) DBURL="sqlite://$PWD/out/spike.db?mode=rwc" ;;
  pg) DBURL="postgres://spike:$POSTGRES_PASSWORD@127.0.0.1:47003/spike" ;;
esac
CAP=${CAP:-512}; TRIM=1; PB=${PB:-1000}
# pinning a 4 CPU (2c): SPIKE_CPUS per motore e worker (taskset), SPIKE_CPUSET per i container, SPIKE_LOADCPUS per il carico
TS_ENG=${SPIKE_CPUS:+taskset -c $SPIKE_CPUS}; TS_LOAD=${SPIKE_LOADCPUS:+taskset -c $SPIKE_LOADCPUS}
EP=; WP=
# N scelto dalla verifica della fase 2d (mediana più alta di thrmax): Valkey 4, Iggy 1
WDEF=1; [ "$BUS" = valkey ] && WDEF=4
WL=; ENGPFX=
case $SC in kill|dbdown) TRIM=0 ;; size1m) PB=16 ;; size64k) PB=200 ;; ekill|bkill|wkill|dfull-iggy|dfull-db|tretry|tdead|theart|retry) TRIM=0; WL="worklog=out/work.log" ;; t100k) TRIM=0 ;; logrun|aux1k|auxneg|auxrss) TRIM=0 ;; esac
case $SC in cancel|cancel-crash|exectimeout) TRIM=0; WL="worklog=out/work.log stoplog=out/stop.log" ;; esac
[ "$SC" = cancel-load ] && WL="worklog=out/work.log stoplog=out/stop.log"
[ "$SC" = cancel ] && { NOWORKER=1; CAP=4; FANSLOW=1; }
[ "$SC" = cancel-crash ] && { CAP=4096; STOP_DELAY=${STOP_DELAY:-4000}; }
[ "$SC" = exectimeout ] && { EXEC_TIMEOUT=${EXEC_TIMEOUT:-3000}; CAP=4096; }
case $SC in slowdb|slowdb-guard|slowdb-crash) PB=${SLOWPB:-50}; PROJ_DELAY=${PROJ_DELAY:-20}; PROJ_FROM=${PROJ_FROM:-15}; PROJ_SECS=${PROJ_SECS:-25} ;; esac
[ "$SC" = slowdb-guard ] && LAG_MAX=${LAG_MAX:-1000}
# blocco D (log dei task): profilo delle righe dei worker (LOGA, LOGB = NxL: N righe da L byte per tentativo, tenant A e B),
# pausa tra le due metà (LOGSPLIT), tetto (LOGLINES, LOGBYTES), ritardo della conferma (ACKHOLD), sostituzione del segreto
# (REDACT=0 la spegne: corsa di controllo), LOGMARK=1 genera un segreto sintetico per corsa (mai da .env, mai stampato)
WL="$WL payload=${PAYLOAD:-bytes} loga=${LOGA:-0x0} logb=${LOGB:-0x0} logsplit_ms=${LOGSPLIT:-0} logw=out/log-written.txt"
[ -n "$LOGMARK" ] && { SPIKE_SECRET=ttmark-$(od -An -N12 -tx1 /dev/urandom | tr -d ' \n'); export SPIKE_SECRET; echo "segreto sintetico generato per questa corsa (lunghezza ${#SPIKE_SECRET}), mai stampato"; }
# blocco C: nelle corse con pochi timer il motore scrive una riga per ogni timer programmato (TSET) e scattato (TFIRE)
case $SC in tretry|tdead|theart) export TIMER_LOG=1 ;; esac
case $SC in theart) LEASE=${LEASE:-1500}; GRACE=${GRACE:-500} ;; esac
us() { echo $(( $(date +%s%N) / 1000 )); }
# millisecondi tra due istanti in microsecondi
ms() { echo $(( ($2 - $1) / 1000 )); }
# valore di una riga del log del motore: <ETICHETTA> <us> ..., la prima oppure l'ultima
logus() { grep -E "^$1 " out/engine.log | $2 -1 | cut -d' ' -f2; }
# attende che compaia una riga nel log del motore (al più $2 secondi)
waitlog() { i=0; until grep -qE "$1" out/engine.log; do i=$((i+1)); [ $i -gt $(( $2 * 10 )) ] && return 1; sleep 0.1; done; }
# attende almeno $2 righe che corrispondono a $1 nel log del motore (al più $3 secondi)
waitn() { i=0; until [ "$(grep -cE "$1" out/engine.log)" -ge $2 ]; do i=$((i+1)); [ $i -gt $(( $3 * 10 )) ] && return 1; sleep 0.1; done; }
CAPSIZE=${CAPSIZE:-128m}; CAPFREE=${CAPFREE:-10}
# disco pieno di Iggy: volume tmpfs di docker con limite, montato dove Iggy scrive davvero (/app/local_data: il montaggio
# /local_data del compose non è quello dei dati), con un ciclo che rilancia il server quando esce
if [ "$SC" = dfull-iggy ]; then
  export IGGY_DATA=iggycap IGGY_MNT=/app/local_data IGGY_SUP="while true; do iggy-server; sleep 1; done"
  docker compose -p ttspikebus down -t 1 >/dev/null 2>&1; docker volume rm ttspikebus-iggycap >/dev/null 2>&1
  docker volume create --driver local --opt type=tmpfs --opt device=tmpfs --opt o=size=$CAPSIZE ttspikebus-iggycap >/dev/null
fi
# disco pieno di database e archivio: il motore gira in un namespace di mount proprio (senza sudo) con un tmpfs a
# dimensione limitata su out/capmnt, dove stanno spike.db e l'archivio; le prove si fanno con nsenter sul suo PID
if [ "$SC" = dfull-db ]; then
  mkdir -p out/capmnt
  [ $DB = sqlite ] && DBURL="sqlite://$PWD/out/capmnt/spike.db?mode=rwc"
  ENGPFX="unshare -Urm ./capns.sh $PWD/out/capmnt $CAPSIZE"
fi
NSE() { nsenter -U --preserve-credentials -m -t $EP "$@"; }

start_engine() {
  : > out/engine.log
  $ENGPFX $TS_ENG "$BIN" engine bus=$BUS db="$DBURL" dir=out cap=$CAP trim=$TRIM link=${LINK:-grpc} proj_batch=$PB wconn=${WCONN:-$WDEF} kgroups=${KGROUPS:-4} maxb=${MAXB:-1024} minb=${MINB:-64} grace_ms=${GRACE:-2000} lease_ms=${LEASE:-0} rec_delay_ms=${REC_DELAY:-0} log_lines=${LOGLINES:-100} log_bytes=${LOGBYTES:-16384} log_close_ms=${LOGCLOSE:-500} log_ack_hold_ms=${ACKHOLD:-0} redact=${REDACT:-1} meta=${META:-0} aux=${AUX:-0} exec_timeout_ms=${EXEC_TIMEOUT:-0} stop_delay_ms=${STOP_DELAY:-0} fan_slow=${FANSLOW:-0} lag_max_ms=${LAG_MAX:-0} proj_delay_ms=${PROJ_DELAY:-0} proj_delay_from_s=${PROJ_FROM:-15} proj_delay_secs=${PROJ_SECS:-20} >> out/engine.log 2>&1 &
  EP=$!
  i=0; until grep -q READY out/engine.log; do i=$((i+1)); [ $i -gt 300 ] && { echo "motore non partito"; cat out/engine.log; stop_all; exit 1; }; sleep 0.2; done
  echo "engine pid=$EP $(grep READY out/engine.log)"
}
stop_all() { [ -n "$SP" ] && kill $SP 2>/dev/null; [ -n "$WP" ] && kill $WP 2>/dev/null; [ -n "$EP" ] && kill $EP 2>/dev/null; wait 2>/dev/null; }

# blocco D: rilegge tutti i pezzi dell'archivio e li confronta con la verità di base del worker (si ritenta finché i pezzi
# ancora in volo non sono scritti); solo se la corsa ha righe di log
logcheck_loop() {
  [ -n "$LOGA$LOGB" ] || return 0
  for i in 1 2 3 4 5 6 7 8 9 10; do
    "$BIN" logcheck dir=out bus=$BUS logw=out/log-written.txt log_lines=${LOGLINES:-100} log_bytes=${LOGBYTES:-16384} > out/logcheck.txt 2>&1 && break
    sleep 2
  done
  cat out/logcheck.txt
  # le righe non entrano nei log del motore (FR-52): occorrenze del segreto sintetico nel registro del motore (solo il conteggio)
  [ -n "$SPIKE_SECRET" ] && echo "marker_in_engine_log: $(cat out/engine.log out/engine-pre.log 2>/dev/null | grep -cF "$SPIKE_SECRET")"
  echo "-- log dei task a fine corsa (contatori del motore, SQL per attività dei due tenant): $("$BIN" stats dir=out | grep -E '^(tasklog|sql_per_activity_tenant)' | tr '\n' ' ')"
}
# blocco E4c: rapporto del database lento ma attivo, dai registri del motore (LAG10, LAG_OVER1S, PROJ_DELAY_*, PROJECTION_LAG_*, ACC), dal carico e da SQLite.
# Gli istanti sono microsecondi della stessa macchina.
slowdb_report() {
  ELOG=$(cat out/engine-pre.log out/engine.log 2>/dev/null)
  J() { echo "JUDGE $1 valore=$2 atteso=$3 $([ "$2" = "$3" ] && echo PASS || echo FAIL)"; }
  echo "-- curva del ritardo a tick di 100 ms, finestre da 10 s (tick_lag_max_ms, tempo sopra 1 s, RSS, pending_locs, busy, rifiuti per ritardo, guardiano):"
  echo "$ELOG" | grep '^LAG10' | cut -c1-200
  ON=$(echo "$ELOG" | grep '^PROJ_DELAY_ON' | head -1 | cut -d' ' -f2); OFF=$(echo "$ELOG" | grep '^PROJ_DELAY_OFF' | head -1 | cut -d' ' -f2)
  O1=$(echo "$ELOG" | grep '^LAG_OVER1S' | awk -v o="${ON:-0}" '$2>=o{print $2; exit}')
  U1=$(echo "$ELOG" | grep '^LAG_UNDER1S' | awk -v o="${OFF:-0}" '$2>=o{print $2; exit}')
  echo "-- ritardo acceso (PROJ_DELAY_ON) $ON; spento (PROJ_DELAY_OFF) ${OFF:-non nel registro}; ritardo sopra 1 s (LAG_OVER1S) ${O1:-mai}: $([ -n "$O1" ] && [ -n "$ON" ] && ms $ON $O1 || echo n.a.) ms dopo l'accensione; sotto 1 s di nuovo (LAG_UNDER1S) ${U1:-mai}: $([ -n "$U1" ] && [ -n "$OFF" ] && ms $OFF $U1 || echo n.a.) ms dopo lo spegnimento (recupero)"
  echo "-- contatori finali: $("$BIN" stats dir=out | grep -E '^lag ' )"
  echo "-- start_ready e proj_lag: $("$BIN" stats dir=out | grep -E '^(start_ready |proj_lag )' | cut -c1-110 | tr '\n' ';')"
  echo "-- errori di database «locked/busy» arrivati al proiettore e righe 'proiezione ferma': $(echo "$ELOG" | grep -ciE 'locked|busy') ; $(echo "$ELOG" | grep -c '^proiezione ferma')"
  # conservazione: nessuna posizione tagliata oltre quella applicata (il proiettore esce con E_POSIZIONE_PERSA se la lettura non parte da dove ha applicato)
  J "posizioni del bus tagliate oltre la posizione applicata (righe E_POSIZIONE_PERSA)" "$(echo "$ELOG" | grep -c E_POSIZIONE_PERSA)" 0
  sqlite3 out/spike.db "SELECT topic, pos FROM projection_pos" | tr '|' ' ' > out/pos.txt
  "$BIN" bus-info | sed -E 's/^bus-info ([^:]+):.*primo_offset_leggibile=(Some\(([0-9]+)\)|None).*/\1 \3/' > out/first.txt
  echo "-- primo offset leggibile del bus per topic (trim non vuoto se > 0): $(tr '\n' ';' < out/first.txt) posizione applicata: $(tr '\n' ';' < out/pos.txt)"
  J "primo offset leggibile oltre la posizione applicata (conteggio topic)" "$(awk 'NR==FNR{p[$1]=$2; next} ($2!="") && ($2>p[$1]){n++} END{print n+0}' out/pos.txt out/first.txt)" 0
  if [ -n "$LAG_MAX" ] && [ "$LAG_MAX" != 0 ]; then
    # accettazioni riuscite con istante di ricezione tra inserimento e rilascio del guardiano (l'inserimento scrive il segno prima dell'istante, il rilascio l'istante prima del segno)
    R=$(echo "$ELOG" | grep -E '^(PROJECTION_LAG_ENGAGE|PROJECTION_LAG_RELEASE|ACC )' | awk '$1=="PROJECTION_LAG_ENGAGE"{ne++; s[ne]=$NF; e[ne]=1e19} $1=="PROJECTION_LAG_RELEASE"{e[ne]=$NF} $1=="ACC"{a[++na]=$2} END{for(i=1;i<=na;i++) for(w=1;w<=ne;w++) if (a[i]>=s[w] && a[i]<e[w]) bad++; print (bad+0), ne+0, na+0}')
    set -- $R
    J "accettazioni riuscite tra inserimento e rilascio del guardiano" "$1" 0
    echo "-- inserimenti del guardiano: $2; accettazioni riuscite in tutto: $3; righe di inserimento/rilascio:"; echo "$ELOG" | grep -E '^PROJECTION_LAG_(ENGAGE|RELEASE)'
    echo "-- (controllo del conteggio) rifiuti del motore con E_PROJECTION_LAG: $("$BIN" stats dir=out | grep -oE 'refused_lag=[0-9]+')"
  fi
}

# Gli istanti sono microsecondi della stessa macchina. `durable.txt` = esecuzione e istante dell'evento `Cancelled` durevole.

# blocco E4: rapporto dell'annullamento e della scadenza dell'esecuzione, dai registri (motore, worker, sentinella) e dal database.
cancel_report() {
  ELOG=$(cat out/engine-pre.log out/engine.log 2>/dev/null)
  echo "$ELOG" | grep '^CANCEL_DURABLE' | awk '{print $2, $NF}' > out/durable.txt
  J() { echo "JUDGE $1 valore=$2 atteso=$3 $([ "$2" = "$3" ] && echo PASS || echo FAIL)"; }
  echo "-- annullamenti e scadenze durevoli: $(wc -l < out/durable.txt); per stato alla richiesta: $(echo "$ELOG" | grep '^CANCEL_RECV' | sed -E 's/.*status=([0-9]).*state=([a-z]+).*/status\1:\2/' | sort | uniq -c | tr -s ' ' | tr '\n' ';')"
  echo "-- risposte del pilota: $(grep -h '^CANCELCTL' out/cancelctl.txt 2>/dev/null | grep -oE 'code=[A-Z_:a-z]+' | sort | uniq -c | tr -s ' ' | tr '\n' ';')"
  # punto 1 (emendamento A2): si giudica sull'istante di invio del motore (campo 6 del registro dei worker, stesso orologio di CANCEL_DURABLE); la ricezione si riporta accanto
  J "1 consegne partite dopo l'evento durevole (invio_us)" "$(awk 'NR==FNR{d[$1]=$2; next} ($1 in d) && $6>d[$1]{n++} END{print n+0}' out/durable.txt out/work.log)" 0
  echo "-- (riportato) consegne ricevute dal worker dopo l'evento durevole (ricezione_us): $(awk 'NR==FNR{d[$1]=$2; next} ($1 in d) && $4>d[$1]{n++} END{print n+0}' out/durable.txt out/work.log)"
  J "2 Stop spediti prima dell'evento durevole" "$(echo "$ELOG" | grep '^STOP_SENT' | awk 'NR==FNR{d[$1]=$2; next} ($2 in d) && $5<d[$2]{n++} END{print n+0}' out/durable.txt -)" 0
  # controllo positivo dei join (E4c): gli stessi join con il confronto invertito, solo riportati; devono essere > 0 (1: almeno `inflight_at_durable`, 2: = `stops_sent`)
  echo "-- (controllo join) consegne con invio_us <= evento durevole: $(awk 'NR==FNR{d[$1]=$2; next} ($1 in d) && $6<=d[$1]{n++} END{print n+0}' out/durable.txt out/work.log)"
  echo "-- (controllo join) STOP_SENT con istante >= evento durevole: $(echo "$ELOG" | grep '^STOP_SENT' | awk 'NR==FNR{d[$1]=$2; next} ($2 in d) && $5>=d[$2]{n++} END{print n+0}' out/durable.txt -)"
  [ $DB = sqlite ] && echo "-- (controllo join) attivita' rese pronte con ready_us <= evento durevole: $(sqlite3 out/spike.db "SELECT lower(hex(a.exec_id)), a.ready_us FROM activity a JOIN execution e ON e.id=a.exec_id WHERE e.status IS NOT NULL AND a.ready_us IS NOT NULL" | tr '|' ' ' | awk 'NR==FNR{d[$1]=$2; next} ($1 in d) && $2<=d[$1]{n++} END{print n+0}' out/durable.txt -)"
  if [ $DB = sqlite ]; then
    J "3 attivita' rese pronte dopo l'evento durevole" "$(sqlite3 out/spike.db "SELECT lower(hex(a.exec_id)), a.ready_us FROM activity a JOIN execution e ON e.id=a.exec_id WHERE e.status IS NOT NULL AND a.ready_us IS NOT NULL" | tr '|' ' ' | awk 'NR==FNR{d[$1]=$2; next} ($1 in d) && $2>d[$1]{n++} END{print n+0}' out/durable.txt -)" 0
    J "4 timer rimasti per esecuzioni annullate o scadute" "$(sqlite3 out/spike.db "SELECT count(*) FROM timer t JOIN execution e ON e.id=t.exec_id WHERE e.status IS NOT NULL")" 0
  fi
  if [ -f out/sentinel.log ]; then
    J "5 allarmi overdue/retry del sentinella per esecuzioni annullate dopo l'evento durevole" "$(awk 'NR==FNR{d[$1]=$2; next} $2=="ALARM" && ($3=="overdue"||$3=="retry") && ($4 in d) && $1>d[$4]{n++} END{print n+0}' out/durable.txt out/sentinel.log)" 0
    echo "-- (riportato) allarmi overdue/retry per esecuzioni annullate PRIMA dell'evento durevole (ammessi): $(awk 'NR==FNR{d[$1]=$2; next} $2=="ALARM" && ($3=="overdue"||$3=="retry") && ($4 in d) && $1<=d[$4]{n++} END{print n+0}' out/durable.txt out/sentinel.log)"
  fi
  # punto 7 (emendamento A1): annullate = risposte CANCELLED del pilota = annullamenti durevoli del motore (registro, vale anche dopo un crash)
  SQLC=$(sqlite3 out/spike.db "SELECT count(*) FROM execution WHERE status=1")
  DRVC=$(grep -h '^CANCELCTL' out/cancelctl.txt 2>/dev/null | grep -c 'code=CANCELLED')
  ENGC=$(echo "$ELOG" | grep '^CANCEL_DURABLE' | grep -c 'status=1')
  if [ $SC != exectimeout ]; then
    J "7a righe 'annullata' = risposte CANCELLED del pilota" "$SQLC" "$DRVC"
    J "7b annullamenti durevoli del motore = risposte CANCELLED del pilota" "$ENGC" "$DRVC"
  fi
  if [ $SC = exectimeout ]; then
    # scadute attese: esecuzioni del tenant A (work_ms=2000) non concluse in tempo (concluse = senza stato e con done_us entro la scadenza)
    T=$((EXEC_TIMEOUT * 1000))
    J "7c righe 'scaduta' = esecuzioni A non concluse entro la scadenza" "$(sqlite3 out/spike.db "SELECT count(*) FROM execution WHERE status=2")" "$(sqlite3 out/spike.db "SELECT count(*) FROM execution WHERE work_ms=2000 AND NOT (status IS NULL AND done_us IS NOT NULL AND done_us <= accepted_us + $T)")"
    J "7d esecuzioni 'scaduta' prima della scadenza (end_us < accepted_us + T)" "$(sqlite3 out/spike.db "SELECT count(*) FROM execution WHERE status=2 AND end_us < accepted_us + $T")" 0
    J "7e righe 'annullata' in una scena di sole scadenze" "$SQLC" 0
  fi
  if [ $SC = cancel ]; then
    for s in never held inflight fanout retrywait; do
      J "6 stato '$s' raggiunto (1 = almeno una richiesta)" "$([ $(echo "$ELOG" | grep '^CANCEL_RECV' | grep -c "state=$s") -gt 0 ] && echo 1 || echo 0)" 1
    done
    for c in E_NOT_LIVE ALREADY_CANCELLED; do
      J "6 codice $c ottenuto (1 = almeno una risposta)" "$([ $(grep -h '^CANCELCTL' out/cancelctl.txt | grep -c "code=$c") -gt 0 ] && echo 1 || echo 0)" 1
    done
  fi
  INFL=$(echo "$ELOG" | grep '^CANCEL_DURABLE' | sed -E 's/.*inflight=([0-9]+).*/\1/' | awk '{s+=$1} END{print s+0}')
  STP=$(echo "$ELOG" | grep -c '^STOP_SENT')
  if [ $SC = cancel-crash ]; then
    echo "-- punto 8 non vale in cancel-crash (Stop attesi dal ritardo, motore ucciso): Stop spediti $STP, tentativi in volo all'evento durevole $INFL"
  else
    J "8 Stop spediti = tentativi in volo all'evento durevole" "$STP" "$INFL"
  fi
  echo "-- Stop ricevuti dai worker: $(wc -l < out/stop.log 2>/dev/null || echo 0), di cui con il tentativo ancora in corso: $(awk '$5==1' out/stop.log 2>/dev/null | wc -l)"
  echo "-- dall'evento durevole allo Stop ricevuto dal worker (ms, p50/p99/max): $(awk 'NR==FNR{d[$1]=$2; next} ($1 in d){print ($4-d[$1])/1000}' out/durable.txt out/stop.log 2>/dev/null | sort -n | awk '{a[NR]=$1} END{if (NR==0) {print "nessuno"} else {i=int(NR*0.99); if (i<1) i=1; printf "n=%d p50=%.1f p99=%.1f max=%.1f", NR, a[int((NR+1)/2)], a[i], a[NR]}}')"
  if [ $DB = sqlite ]; then
    echo "-- storico: $(sqlite3 out/spike.db "SELECT status||':'||end_role||':'||end_name, count(*) FROM execution WHERE status IS NOT NULL GROUP BY 1" | tr '\n' ';')"
    echo "-- attivita' aperte nelle esecuzioni annullate (ammesse, restano ferme): $(sqlite3 out/spike.db "SELECT count(*) FROM activity a JOIN execution e ON e.id=a.exec_id WHERE e.status IS NOT NULL AND a.done_us IS NULL")"
  fi
  # latenza dal comando alla risposta vista dal pilota, sulle sole richieste del periodo misurato (dopo `measure_from_us` se la scena lo scrive)
  MF=$(grep -h '^CANCELCTL load t0_us' out/cancelctl.txt 2>/dev/null | sed -E 's/.*measure_from_us=([0-9]+).*/\1/')
  grep -h '^CANCELCTL' out/cancelctl.txt 2>/dev/null | grep 'code=CANCELLED' | sed -E 's/.*latency_us=([0-9]+) at_us=([0-9]+).*/\1 \2/' | awk -v mf="${MF:-0}" '$2>=mf{print $1/1000}' | sort -n | awk '{a[NR]=$1} END{if (NR) {i=int(NR*0.99); if (i<1) i=1; printf "-- latenza del comando CANCELLED vista dal pilota (ms, periodo misurato): n=%d p50=%.1f p99=%.1f max=%.1f\n", NR, a[int((NR+1)/2)], a[i], a[NR]}}'
  echo "-- contatori del motore (dopo un crash ripartono da zero): $("$BIN" stats dir=out | grep -E '^(cancel |cancel_durable_latency|exec_timeout_fire_delay|start_ready_tenant_b)' | tr '\n' ' ')"
}

# blocco E2: contatori finali dell'ausiliario e latenza del passo (dopo lo svuotamento: le chiamate si confrontano con le attività concluse)
aux_final() {
  echo "-- ausiliario a fine corsa: $("$BIN" stats dir=out | grep -E '^(aux|step_latency)' | tr '\n' ' ')"
  echo "AUX_INPUT_INVALID nel registro del motore: $(grep -c AUX_INPUT_INVALID out/engine.log); AUX_OUTPUT_INVALID: $(grep -c AUX_OUTPUT_INVALID out/engine.log); panic: $(grep -c panicked out/engine.log)"
  grep -m2 AUX_INPUT_INVALID out/engine.log | cut -c1-300
}

SERV="$BUS"; [ $DB = pg ] && SERV="$SERV postgres"
./reset.sh $SERV
if [ $DB = pg ]; then i=0; until docker compose -p ttspikebus exec -T postgres pg_isready -U spike >/dev/null 2>&1; do i=$((i+1)); [ $i -gt 60 ] && exit 1; sleep 1; done; sleep 2; fi
rm -rf out/archive out/logs out/spike.db* out/master.key out/ledger.txt
# archivio su un altro disco (variabile SPIKE_ARCHIVE): out/archive diventa un collegamento
[ -n "$SPIKE_ARCHIVE" ] && { rm -rf "$SPIKE_ARCHIVE"; mkdir -p "$SPIKE_ARCHIVE"; ln -s "$SPIKE_ARCHIVE" out/archive; }
# CAPDBONLY=1: solo il database nel tmpfs (l archivio resta sul disco vero), per provare il disco pieno del database da solo
[ "$SC" = dfull-db ] && [ -z "$CAPDBONLY" ] && ln -s "$PWD/out/capmnt/archive" out/archive
rm -f out/sentinel.log out/sentinel.stop out/sentinel.txt out/newtopic.txt out/work.log out/stop.log out/cancelctl.txt out/ledger-b.txt out/cancel-recv.txt out/engine-pre.log out/kind0-dump.txt out/log-written.txt out/logcheck.txt out/logtail.txt out/noisy-a.txt
[ -f out/tls/cert.pem ] || "$BIN" gen-cert dir=out
[ -f out/worker.cred ] || "$BIN" gen-cred dir=out
echo "### $BUS + $DB: $SC $*"; echo "cpu: motore/worker/bus/db=${SPIKE_CPUS:-tutte} carico=${SPIKE_LOADCPUS:-tutte} wconn=${WCONN:-$WDEF} kgroups=${KGROUPS:-4} maxb=${MAXB:-1024} minb=${MINB:-64}"
start_engine
# blocco sentinella: SENT=1 avvia il sentinella (sulle CPU del motore: fa parte della piattaforma), META=1 (sopra) fa scrivere al motore
# l'intestazione in chiaro; il sentinella si ferma con `out/sentinel.stop` e stampa le sue statistiche
SP=
if [ -n "$SENT" ]; then
  # blocco E (credenziale di sola lettura): SENTUSER=1 fa girare il vero sentinella con l'utente `ro-sent` (password generata, mai stampata)
  if [ -n "$SENTUSER" ]; then
    "$BIN" busauth case=setup-sent
    IGGY_ROOT_USERNAME=ro-sent IGGY_ROOT_PASSWORD=$(cat out/ro-sent.pw) $TS_ENG "$BIN" sentinel dir=out secs=${SENTSECS:-900} > out/sentinel.txt 2>&1 &
  else
    $TS_ENG "$BIN" sentinel dir=out secs=${SENTSECS:-900} > out/sentinel.txt 2>&1 &
  fi
  SP=$!; sleep 1
fi
sentinel_stop() {
  [ -n "$SP" ] || return 0
  touch out/sentinel.stop; wait $SP; SP=
  cat out/sentinel.txt
  echo "sentinel allarmi: overdue=$(grep -c 'ALARM overdue' out/sentinel.log) retry=$(grep -c 'ALARM retry' out/sentinel.log) engine-silent=$(grep -c 'ALARM engine-silent' out/sentinel.log) recovered=$(grep -c 'RECOVERED engine' out/sentinel.log)"
  echo "sentinel prime righe: $(grep -m1 'ALARM overdue' out/sentinel.log) | $(grep -m1 'ALARM engine-silent' out/sentinel.log) | $(grep -m1 'RECOVERED engine' out/sentinel.log) | $(grep -m1 'ALARM retry' out/sentinel.log)"
}
start_worker() { $TS_ENG "$BIN" worker dir=out link=${LINK:-grpc} sessions=${WSESS:-4} $WL > out/worker.log 2>&1 & WP=$!; }
# blocco E4: NOWORKER=1 lascia la scena avviare il worker (per avere passi in coda mai consegnati)
[ -n "$NOWORKER" ] || { start_worker; sleep 1; }
L="$TS_LOAD $BIN load dir=out ${PAYLOAD:+payload=$PAYLOAD} ${INBAD:+inbad=$INBAD} ${INFLAG:+inflag=$INFLAG}"

case $SC in
  accept) $L rate=200 kind=0 n=1 size=1024 secs=30 warm=3 "$@" ;;
  start1k) $L rate=100 kind=0 n=10 size=1024 secs=40 warm=5 "$@" ;;
  thr10k)
    # Dal blocco B ogni corsa a 10.000 att./s ha il registro delle accettate (ledger) e la verifica «zero accettate perse»
    # (verify light: ledger contro database, niente scansione del bus, payload a campione), e il tipo di ogni errore.
    # Criterio di stallo, fissato prima delle corse: un minuto (riga MIN) con meno di 9.000 att./s, oppure un intervallo
    # senza accettazioni riuscite di oltre 5 s (accept_max_gap_ms), oppure ritardo di proiezione p99 oltre 1.000 ms.
    echo "criterio di stallo: minuto con act_s < 9000, oppure accept_max_gap_ms > 5000, oppure lag_p99_ms > 1000"
    $L rate=1000 kind=0 n=10 size=1024 secs=60 warm=5 tenants=ab ledger=out/ledger.txt "$@"
    echo "-- righe di errore del motore"; grep -vE "^(MIN|READY|ripresa|capns|FIRST_DONE)" out/engine.log | head -40
    # blocco D: VTRIES (predefinito 12 = 36 s) allunga l'attesa quando il motore ha una coda di lavoro da smaltire a fine carico
    for i in $(seq 1 ${VTRIES:-12}); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    aux_final ;;
  thrmax) $L rate=2500 kind=0 n=10 size=1024 secs=30 warm=5 tenants=ab "$@" ;;
  size64k) $L rate=100 kind=0 n=5 size=65536 secs=30 warm=5 tenants=ab "$@" ;;
  size1m) $L rate=${RATE1M:-10} kind=0 n=5 size=1048576 secs=30 warm=5 tenants=ab "$@" ;;
  join) $L rate=50 kind=1 n=4 size=1024 work_ms=1500 secs=30 warm=5 tenants=ab "$@" ;;
  iso0) $L rate=100 kind=0 n=10 size=1024 tenants=b secs=30 warm=5 "$@" ;;
  iso)
    $L rate=400 kind=0 n=1 size=1024 work_ms=3000 tenants=a secs=40 warm=2 stats=0 > out/iso-a.txt &
    LA=$!
    sleep 3
    $L rate=100 kind=0 n=10 size=1024 tenants=b secs=30 warm=5 "$@"
    wait $LA; echo "-- tenant A (lento)"; cat out/iso-a.txt ;;
  logrun|aux1k|auxneg)
    # Blocco D, log dei task: una corsa con righe (profilo LOGA/LOGB). TAIL=1 misura la visibilità (logtail su CPU del carico);
    # in ogni caso rilegge tutti i pezzi, li confronta con la verità di base del worker e cerca il segreto (logcheck).
    [ -n "$TAIL" ] && { $TS_LOAD "$BIN" logtail dir=out secs=${TAILSECS:-52} > out/logtail.txt 2>&1 & TP=$!; sleep 1; }
    $L rate=100 kind=0 n=10 size=1024 secs=40 warm=5 ledger=out/ledger.txt "$@"
    [ -n "$TAIL" ] && { wait $TP; cat out/logtail.txt; }
    for i in $(seq 1 12); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    aux_final
    logcheck_loop ;;
  lognoisy)
    # Blocco D, worker rumoroso: B a 1.000 att./s (si misura start_ready_tenant_b); con NOISY=1 anche A, che scrive fino al tetto
    # a ogni attività (NRATE esecuzioni/s da 10 passi, 50 = 500 att./s), parte 3 s prima e finisce dopo la finestra di B.
    [ -n "$NOISY" ] && { $L rate=${NRATE:-50} kind=0 n=10 size=1024 tenants=a secs=50 warm=0 stats=0 > out/noisy-a.txt 2>&1 & LA=$!; sleep 3; }
    $L rate=100 kind=0 n=10 size=1024 tenants=b secs=40 warm=5 ledger=out/ledger.txt "$@"
    [ -n "$NOISY" ] && { wait $LA; echo "-- tenant A (rumoroso)"; grep -E "^(load|offered|accept_rtt|activities)" out/noisy-a.txt; }
    for i in $(seq 1 12); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    logcheck_loop ;;
  cancel)
    # Blocco E4, annullamento (FR-10): engine con `cap=4` e rami del fan-out tutti lenti, worker NON ancora collegato. Fase `never`: passi in coda
    # mai consegnati; poi il worker e la fase `rest`: in volo, trattenuti dal tetto, ritentativo in attesa, fan-out, già concluse, ripetuti.
    "$BIN" cancelctl dir=out phase=never | tee out/cancelctl.txt
    start_worker; sleep 1
    "$BIN" cancelctl dir=out phase=rest | tee -a out/cancelctl.txt
    for i in $(seq 1 ${VTRIES:-12}); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    sleep 2; sentinel_stop
    cancel_report ;;
  slowdb|slowdb-guard)
    # Blocco E4c, database lento ma attivo (AD-2): 1.000 att./s (100 esec./s da 10 passi, due tenant), tetto 512, trim acceso. Il proiettore tiene
    # il lock di scrittura di SQLite per `proj_delay_ms` dentro ogni transazione, prima del COMMIT, da `proj_delay_from_s` (dalla prima applicazione)
    # per `proj_delay_secs`; poi si rilascia e il ritardo rientra. `slowdb-guard`: `lag_max_ms=1000` (il secondo di AD-2).
    $L rate=100 kind=0 n=10 size=1024 tenants=ab secs=${LSECS:-60} warm=5 ledger=out/ledger.txt "$@" > out/load.txt
    cat out/load.txt | grep -E '^(load|offered|errors_by_type|accept_rtt|accept_engine|start_ready |activities_per_s)'
    ok=0; for i in $(seq 1 ${VTRIES:-20}); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && ok=1 && break; sleep 3; done
    J() { echo "JUDGE $1 valore=$2 atteso=$3 $([ "$2" = "$3" ] && echo PASS || echo FAIL)"; }
    J "verify OK (0 accettate perse)" "$ok" 1
    slowdb_report ;;
  slowdb-crash)
    # Blocco E4c: come `slowdb` (guardiano spento) ma `kill -9` del motore quando il ritardo del tick supera KILLLAG ms (3.000 se non detto), durante la
    # tenuta; riavvio SENZA ritardo. Si riportano kill -> READY e gli eventi riletti accanto al limite di 30 s di NFR-15.
    $L rate=100 kind=0 n=10 size=1024 tenants=ab secs=${LSECS:-60} warm=5 stats=0 ledger=out/ledger.txt "$@" > out/load.txt &
    LP=$!
    LNOW=0
    for i in $(seq 1 120); do
      sleep 1
      LNOW=$("$BIN" stats dir=out | sed -n 's/.*tick_lag_now_ms=\([0-9]*\).*/\1/p' | head -1)
      [ "${LNOW:-0}" -ge ${KILLLAG:-3000} ] && break
    done
    echo "prima del kill: ritardo del tick $LNOW ms; $("$BIN" stats dir=out | grep -E '^lag ' | tr '\n' ' ')"
    cp out/engine.log out/engine-pre.log
    T0=$(us); kill -9 $EP; wait $EP 2>/dev/null; echo "kill T0_us=$T0"
    PROJ_DELAY=0
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill -> READY: $(ms $T0 $RU) ms (limite NFR-15: 30000 ms); READY_us=$RU"; grep -E "ripresa" out/engine.log
    wait $LP; grep -E '^(load|offered|errors_by_type|accept_rtt)' out/load.txt
    ok=0; for i in $(seq 1 ${VTRIES:-20}); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && ok=1 && break; sleep 3; done
    J() { echo "JUDGE $1 valore=$2 atteso=$3 $([ "$2" = "$3" ] && echo PASS || echo FAIL)"; }
    J "verify OK dopo il riavvio (0 accettate perse)" "$ok" 1
    slowdb_report ;;
  cancel-load)
    # Blocco E4, annullamenti con il motore carico (emendamento A5): tenant B a 1.000 att./s (100 esec./s da 10 passi), tetto 512; tenant A a 20 esec./s da
    # 10 passi da 400 ms (circa 4 s l'una), ognuna annullata in un istante casuale tra l'accettazione e la fine attesa, il 5 % due volte insieme.
    # 5 s di riscaldamento e 60 s misurati (circa 1.200 annullamenti misurati).
    $L rate=100 kind=0 n=10 size=1024 tenants=b secs=60 warm=5 ledger=out/ledger-b.txt > out/load-b.txt &
    LB=$!
    "$BIN" cancelctl dir=out phase=load rate=20 n=10 work_ms=400 secs=60 warm=5 > out/cancelctl.txt
    wait $LB; grep -E '^(load|offered|activities_per_s|start_ready_tenant_b)' out/load-b.txt
    cat out/ledger-b.txt >> out/ledger.txt
    for i in $(seq 1 ${VTRIES:-12}); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    sleep 2; sentinel_stop
    grep -h '^CANCELCTL load' out/cancelctl.txt
    cancel_report ;;
  cancel-crash)
    # Blocco E4: 100 esecuzioni lunghe (5 s) annullate tutte insieme, poi altre 100; `stop_delay_ms=4000` ritarda gli Stop dopo l'evento durevole:
    # `kill -9` del motore dopo gli annullamenti durevoli e prima degli Stop, riavvio. Si registra se il worker riceve uno Stop o solo un esito rifiutato.
    "$BIN" cancelctl dir=out phase=crash | tee out/cancelctl.txt
    echo "prima del kill: $("$BIN" stats dir=out | grep -E '^(cancel |undurable_deliveries)' | tr '\n' ' ')"
    cp out/engine.log out/engine-pre.log
    T0=$(us); kill -9 $EP; wait $EP 2>/dev/null; echo "kill T0_us=$T0 (Stop spediti prima del kill: $(grep -c '^STOP_SENT' out/engine-pre.log))"
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill -> READY: $(ms $T0 $RU) ms; READY_us=$RU"; grep -E "ripresa" out/engine.log
    sleep 7
    for i in $(seq 1 ${VTRIES:-12}); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    echo "Stop ricevuti dai worker dopo la ripartenza (righe di stop.log): $(wc -l < out/stop.log 2>/dev/null || echo 0); esiti rifiutati e contati come tardivi: $("$BIN" stats dir=out | grep -oE 'late_after_cancel=[0-9]+')"
    sleep 1; sentinel_stop
    cancel_report ;;
  exectimeout)
    # Blocco E4, scadenza dell'esecuzione (FR-8): `exec_timeout_ms=3000` dall'accettazione. Tenant A: esecuzioni da 3 passi da 2 s (scadono a metà),
    # tenant B: esecuzioni da un passo da 100 ms (concludono prima). Con TCRASH=1 `kill -9` del motore a 1,5 s, prima che la prima scadenza sia dovuta.
    $L rate=20 kind=0 n=3 size=64 work_ms=2000 tenants=a secs=5 warm=0 stats=0 ledger=out/ledger-a.txt > out/load-a.txt &
    LA=$!
    $L rate=20 kind=0 n=1 size=64 work_ms=100 tenants=b secs=5 warm=0 stats=0 ledger=out/ledger-b.txt > out/load-b.txt &
    LB=$!
    if [ -n "$TCRASH" ]; then
      sleep 1.5; cp out/engine.log out/engine-pre.log
      echo "prima del kill: $("$BIN" stats dir=out | grep -E '^(timers_pending|secs)' | tr '\n' ' ')"
      T0=$(us); kill -9 $EP; wait $EP 2>/dev/null; echo "kill T0_us=$T0"
      start_engine; grep -E "ripresa" out/engine.log
    fi
    wait $LA $LB; cat out/load-a.txt out/load-b.txt | grep -E '^(load|offered)'
    cat out/ledger-a.txt out/ledger-b.txt > out/ledger.txt
    sleep 4
    for i in $(seq 1 ${VTRIES:-12}); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    sleep 2; sentinel_stop
    cancel_report ;;
  retry)
    # Blocco sentinella, riga 3: errori senza kill. kind 3: il primo tentativo di ogni passo fallisce, attesa di ${RETRY_MS:-300} ms,
    # secondo tentativo. 100 esecuzioni/s per 5 s; i tentativi falliti sono i primi tentativi iniziati (registro dei worker).
    $L rate=100 kind=3 n=1 size=1024 work_ms=100 retry_ms=${RETRY_MS:-300} secs=5 warm=0 stats=0 tenants=ab ledger=out/ledger.txt "$@"
    for i in $(seq 1 12); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    echo "tentativi falliti (primi tentativi iniziati, registro dei worker): $(awk '$5==0' out/work.log | wc -l); ritentativi iniziati: $(awk '$5==1' out/work.log | wc -l)" ;;
  newtenant)
    # Blocco sentinella, riga 5: durante un carico normale, a 4 s, nasce un topic `state-*` nuovo (un tenant che il sentinella non conosceva);
    # si confronta l'istante della sua creazione con la riga `FIRST` del sentinella.
    $L rate=20 kind=0 n=1 size=1024 secs=8 warm=1 stats=0 ledger=out/ledger.txt "$@" > out/load.txt &
    LP=$!
    sleep 4
    "$BIN" sentinel-newtopic > out/newtopic.txt
    wait $LP; cat out/load.txt out/newtopic.txt
    for i in $(seq 1 12); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    sleep 2; sentinel_stop
    NT=$(sed -n 's/^NEWTOPIC \([^ ]*\) .*/\1/p' out/newtopic.txt); CU=$(sed -n 's/.*create_us=\([0-9]*\) .*/\1/p' out/newtopic.txt)
    FR=$(grep -m1 "FIRST $NT" out/sentinel.log | cut -d' ' -f1)
    echo "newtenant: topic $NT, primo evento letto $(( (${FR:-0} - CU) / 1000 )) ms dopo la creazione (create_us=$CU first_read_us=${FR:-nessuno})" ;;
  sec)
    # prove di rifiuto (fase 3): ogni condizione di sicurezza ha un collegamento che deve essere rifiutato
    W="$TS_ENG $BIN worker dir=out link=${LINK:-grpc} once=1"
    "$BIN" gen-cert dir=out/other
    echo "buono (credenziale, identita e certificato giusti): $($W 2>&1 | tail -3 | tr "\n" " ")"
    echo "credenziale errata: $($W cred=00ff 2>&1 | tail -1)"
    if [ "${LINK:-grpc}" = grpc ]; then
      echo "il worker non verifica il server (certificato di un altro motore): $($W tlsdir=out/other 2>&1 | tail -1)"
    else
      echo "il worker non verifica il server (identita attesa di un altro nodo): $($W engine_id=$("$BIN" gen-nodeid) 2>&1 | tail -1)"
      echo "identita del nodo non derivata dalla credenziale: $($W badkey=1 2>&1 | tail -1)"
      echo "-- connessioni di rete di motore e worker (solo localhost ammesso):"
      ss -Htunap 2>/dev/null | grep -E "pid=($EP|$WP)," | awk "{print \$1, \$5, \$6}"
    fi ;;
  rss)
    sleep 60
    ./rss.sh $EP $WP ;;
  auxrss)
    # Blocco E2, riga 3: RSS a riposo (60 s dopo l'avvio, come NFR-1) e sotto carico (a 1.000 att./s per 30 s, campione a 25 s) di motore, ausiliario
    # (figlio del motore), worker e Iggy; somma motore + ausiliario + Iggy (il worker non conta nel conto della piattaforma).
    sleep ${IDLE:-60}
    AP=$(pgrep -P $EP | head -1)
    echo "== a riposo"; ./rss.sh $EP $AP $WP | tee out/rss-idle.txt
    echo "rss somma a riposo (motore + ausiliario + iggy; pid motore=$EP ausiliario=$AP worker=$WP): $(awk '/^rss iggy/{s+=$3} /^rss pid='$EP' /{s+=$3} /^rss pid='${AP:-0}' /{s+=$3} END{print s}' out/rss-idle.txt) kB (80 MB = 81920 kB)"
    $L rate=100 kind=0 n=10 size=1024 secs=30 warm=2 stats=0 ledger=out/ledger.txt "$@" > out/load.txt &
    LP=$!
    sleep 27
    echo "== sotto carico"; ./rss.sh $EP $AP $WP | tee out/rss-load.txt
    wait $LP; grep -E "^(offered|activities)" out/load.txt
    echo "rss somma sotto carico (motore + ausiliario + iggy): $(awk '/^rss iggy/{s+=$3} /^rss pid='$EP' /{s+=$3} /^rss pid='${AP:-0}' /{s+=$3} END{print s}' out/rss-load.txt) kB (80 MB = 81920 kB)"
    for i in $(seq 1 12); do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt light=1 && break; sleep 3; done
    aux_final ;;
  kill)
    $L rate=300 kind=0 n=10 size=1024 work_ms=100 secs=28 warm=1 stats=0 ledger=out/ledger.txt > out/load.txt &
    LP=$!
    for t in 8 5 6; do
      sleep $t
      echo "prima del kill: $("$BIN" stats dir=out | head -2 | tr '\n' ' ')"
      kill -9 $EP; wait $EP 2>/dev/null
      T0=$(date +%s.%N)
      sleep 1
      start_engine
      T1=$(date +%s.%N)
      D1=$(awk -v a="$T1" -v b="$T0" 'BEGIN{printf "%.3f", a-b}'); echo "kill -9 -> motore di nuovo in ascolto: $D1 s (1 s di attesa voluta inclusa)"
      # prima riconsegna/conclusione dopo la ripresa
      i=0; until "$BIN" stats dir=out | grep -q "done=[1-9]"; do i=$((i+1)); [ $i -gt 300 ] && break; sleep 0.1; done
      T2=$(date +%s.%N)
      D2=$(awk -v a="$T2" -v b="$T0" 'BEGIN{printf "%.3f", a-b}'); echo "kill -9 -> prima attività conclusa dopo la ripresa: $D2 s"
    done
    wait $LP; cat out/load.txt
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done ;;
  ret)
    # conservazione (fase 4): trim=1 con segmenti da 1 MiB (IGGY_SEGMENT_SIZE nell'ambiente), un kill -9 a metà.
    # Il proiettore esce con E_POSIZIONE_PERSA se la prima lettura non parte dalla posizione applicata.
    $L rate=${RRATE:-300} kind=0 n=10 size=1024 work_ms=100 secs=${RSECS:-90} warm=1 stats=0 tenants=ab ledger=out/ledger.txt "$@" > out/load.txt &
    LP=$!
    sleep 40
    echo "prima del kill: $("$BIN" bus-info | tr '\n' ' ')"
    cp out/engine.log out/engine-pre.log
    echo "E_POSIZIONE_PERSA prima del kill: $(grep -c E_POSIZIONE out/engine-pre.log)"
    kill -9 $EP; wait $EP 2>/dev/null
    start_engine
    echo "dopo la ripresa: $(grep -E 'ripresa' out/engine.log | tr '\n' ' ')"
    wait $LP; cat out/load.txt
    for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && break; sleep 3; done
    echo "E_POSIZIONE_PERSA dopo la ripresa: $(grep -c E_POSIZIONE out/engine.log)"
    echo "fine: $("$BIN" bus-info | tr '\n' ' ')" ;;
  dbdown)
    $L rate=300 kind=0 n=10 size=1024 work_ms=100 secs=28 warm=1 stats=0 ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 8
    if [ $DB = pg ]; then
      docker compose -p ttspikebus stop postgres >/dev/null 2>&1; echo "postgres fermato"
      sleep 8
      docker compose -p ttspikebus start postgres >/dev/null 2>&1; echo "postgres riavviato"
    else
      "$BIN" lock-db db="$DBURL" secs=10 &
      sleep 11
    fi
    wait $LP; cat out/load.txt; grep -E "proiezione" out/engine.log | head -4
    for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && break; sleep 3; done ;;
  ekill)
    # Blocco B, caso 1: crash del motore con attività lunghe in volo (300 att./s da 3 s: circa 900 in volo), riavvio subito
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 work_ms=3000 secs=30 warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 12
    echo "prima del kill: $("$BIN" stats dir=out | head -1)"
    cp out/engine.log out/engine-pre.log
    # blocco D, parte seconda: consegne inviate il cui record/Timer non è ancora durevole (ora e massimo della corsa).
    # Terza parte: il campione e il timer in memoria (attività in volo) si prendono subito prima del kill.
    echo "prima del kill: $("$BIN" stats dir=out | grep -E '^undurable_deliveries|timers_pending|^tasklog' | tr '\n' ' ')"
    T0=$(us); kill -9 $EP; wait $EP 2>/dev/null; echo "kill T0_us=$T0"
    # blocco sentinella: DOWN=<secondi> lascia il motore fermo prima di riavviarlo (scena `ekill-down` delle regole)
    [ -n "$DOWN" ] && sleep $DOWN
    # blocco D, parte terza (SQLite): righe `timer` di tipo 0 dopo il kill e PRIMA della ripartenza (esecuzione passo tentativo)
    if [ $DB = sqlite ]; then
      sqlite3 out/spike.db "SELECT lower(hex(exec_id)), step, attempt FROM timer WHERE kind=0" > out/kind0-dump.txt
      echo "scarico righe timer tipo 0 dopo il kill: $(wc -l < out/kind0-dump.txt) righe; per attività non concluse: $(sqlite3 out/spike.db "SELECT count(*) FROM timer t LEFT JOIN activity a ON a.exec_id=t.exec_id AND a.step=t.step WHERE t.kind=0 AND a.done_us IS NULL")"
    fi
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill -9 -> READY: $(ms $T0 $RU) ms ($(grep -E '^READY' out/engine.log)); READY_us=$RU"
    grep -E "ripresa" out/engine.log
    waitlog "^FIRST_DONE" 60; FD=$(logus FIRST_DONE head)
    echo "kill -9 -> prima attività conclusa: $(ms $T0 $FD) ms"
    wait $LP; cat out/load.txt
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "tentativi iniziati: $(wc -l < out/work.log), attività distinte: $(cut -d' ' -f1,2 out/work.log | sort -u | wc -l), eseguite più di una volta: $(cut -d' ' -f1,2 out/work.log | sort | uniq -d | wc -l)"
    # blocco D, parte terza: inclusione di insiemi. Tentativi (esecuzione passo tentativo) iniziati due volte, e quanti hanno una riga nello scarico
    if [ -f out/kind0-dump.txt ]; then
      awk '{print $1" "$2" "$5}' out/work.log | sort | uniq -d > out/started-twice.txt
      echo "iniziati due volte: $(wc -l < out/started-twice.txt); di cui con riga nello scarico (devono essere 0): $(awk 'NR==FNR{d[$1" "$2" "$3]=1; next} ($1" "$2" "$3) in d' out/kind0-dump.txt out/started-twice.txt | wc -l)"
      echo "-- scarico (esecuzione passo tentativo):"; cat out/kind0-dump.txt
    fi
    # blocco D, parte terza: esecuzioni nuove nei 2 s dopo READY, da istanti già registrati (accepted_us nel database, registro dei worker)
    echo "sql: SELECT min(accepted_us) - $RU FROM execution WHERE accepted_us > $RU"
    FA=$(sqlite3 out/spike.db "SELECT min(accepted_us) - $RU FROM execution WHERE accepted_us > $RU")
    sqlite3 out/spike.db "SELECT lower(hex(id)) FROM execution WHERE accepted_us > $RU" | sort > out/new-ids.txt
    FDV=$(awk -v ru=$RU 'NR==FNR{n[$1]=1; next} ($1 in n) && $4>ru {if (m==""||$4<m) m=$4} END{print m}' out/new-ids.txt out/work.log)
    echo "dopo READY: prima accettazione nuova $(( ${FA:-0} / 1000 )) ms (us grezzi: ${FA:-nessuna}); prima consegna di un'esecuzione nuova $(( (${FDV:-$RU} - RU) / 1000 )) ms (vuoto = nessuna: $([ -z "$FDV" ] && echo sì || echo no))"
    echo "dopo READY, entro 2 s: accettate nuove $(sqlite3 out/spike.db "SELECT count(*) FROM execution WHERE accepted_us BETWEEN $RU AND $RU + 2000000"); consegne di esecuzioni nuove $(awk -v ru=$RU 'NR==FNR{n[$1]=1; next} ($1 in n) && $4>ru && $4<=ru+2000000' out/new-ids.txt out/work.log | wc -l)"
    logcheck_loop ;;
  bkill)
    # Blocco B, caso 2: crash di Iggy (docker kill = SIGKILL del processo; non è uno spegnimento dell'host), riavvio dopo 5 s
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 work_ms=500 secs=40 warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 10
    echo "prima del kill: $("$BIN" stats dir=out | head -1)"
    T0=$(us); docker kill ttspikebus-iggy-1 >/dev/null; echo "T0 kill di Iggy=$T0"
    sleep 5
    T1=$(us); docker start ttspikebus-iggy-1 >/dev/null; echo "T1 avvio di Iggy=$T1"
    waitlog "^BUS_UP" 120; BU=$(logus BUS_UP tail)
    waitn "^FIRST_DONE" 2 60; FD=$(logus FIRST_DONE tail)
    echo "kill -> BUS_DOWN (motore): $(ms $T0 $(logus BUS_DOWN head)) ms; avvio -> BUS_UP: $(ms $T1 $BU) ms; kill -> BUS_UP: $(ms $T0 $BU) ms; BUS_UP -> prima attività conclusa: $(ms $BU $FD) ms; kill -> prima attività conclusa dopo il ritorno: $(ms $T0 $FD) ms"
    wait $LP; cat out/load.txt
    echo "-- eventi del motore sul bus"; grep -E "^(BUS_|APPEND_)" out/engine.log
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done ;;
  wkill)
    # Blocco B, caso 3: crash di un worker a metà attività (due worker; il secondo prende i tentativi che scadono)
    $TS_ENG "$BIN" worker dir=out link=grpc sessions=${WSESS:-4} $WL > out/worker2.log 2>&1 &
    W2=$!
    sleep 1
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 work_ms=3000 secs=30 warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 12
    echo "prima del kill: $("$BIN" stats dir=out | head -1)"
    T0=$(us); kill -9 $WP; wait $WP 2>/dev/null; echo "T0 kill del worker 1=$T0"; WP=$W2
    wait $LP; cat out/load.txt
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "tentativi iniziati: $(wc -l < out/work.log), attività distinte: $(cut -d' ' -f1,2 out/work.log | sort -u | wc -l), eseguite più di una volta: $(cut -d' ' -f1,2 out/work.log | sort | uniq -d | wc -l)"
    # tempo dal kill al secondo inizio di ogni attività eseguita due volte (minimo, mediana, massimo, in ms)
    sort -k4 -n out/work.log | awk -v t0=$T0 '{k=$1" "$2; if (k in s) {d[++n]=($4-t0)/1000}; s[k]=1} END{asort(d); if (n) printf "riesecuzioni: %d; kill -> nuovo inizio: min %.0f, mediana %.0f, max %.0f ms\n", n, d[1], d[int((n+1)/2)], d[n]; else print "riesecuzioni: 0"}' ;;
  tretry)
    # Blocco C, retry con attesa attraverso un crash. kind 3: il primo tentativo di ogni attività fallisce, attesa di
    # ${RETRY_MS:-5000} ms, secondo tentativo. 100 esecuzioni/s per 3 s; a ${KILL_AT:-4.5} s kill -9 del motore (tutti i
    # fallimenti sono già durevoli, nessun ritentativo è ancora scaduto), il motore resta giù ${PAUSE:-2} s, poi riparte.
    $L rate=${ERATE:-100} kind=3 n=1 size=1024 work_ms=100 retry_ms=${RETRY_MS:-5000} secs=3 warm=0 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep ${KILL_AT:-4.5}
    echo "prima del kill: $("$BIN" stats dir=out | grep -E '^secs|timers_pending' | tr '\n' ' ')"
    T0=$(us); kill -9 $EP; wait $EP 2>/dev/null
    cp out/engine.log out/engine-pre.log
    sleep ${PAUSE:-2}
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill -> READY: $(ms $T0 $RU) ms (giù per ${PAUSE:-2} s voluti); READY_us=$RU"
    grep -E "ripresa" out/engine.log
    wait $LP; cat out/load.txt
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "motore dopo la corsa: $("$BIN" stats dir=out | grep -E '^secs|timer_fire_delay' | tr '\n' ' ')"
    echo "tentativi iniziati: $(wc -l < out/work.log), tentativo 0 distinti: $(awk '$5==0{print $1,$2}' out/work.log | sort -u | wc -l), tentativo 0 eseguiti più di una volta: $(awk '$5==0{print $1,$2}' out/work.log | sort | uniq -d | wc -l)"
    ./tstat.sh retry out/engine-pre.log out/engine.log out/work.log $RU ;;
  tdead)
    # Blocco C, scadenza del tentativo attraverso un crash. Due worker; esecuzioni da un passo di ${DWORK:-3000} ms (scadenza = consegna
    # + durata + 2 s di margine), 100/s per 3 s. A 2 s si uccide il worker 1 (i suoi tentativi in corso non torneranno mai), a
    # 2,5 s kill -9 del motore e riavvio subito: le scadenze (5-7 s con 3 s di lavoro) cadono dopo la ripresa e oltre la sua finestra di attesa dei worker (grace_ms). Nel blocco B ripartivano da zero.
    $TS_ENG "$BIN" worker dir=out link=grpc sessions=${WSESS:-4} $WL > out/worker2.log 2>&1 &
    W2=$!
    sleep 1
    $L rate=${ERATE:-100} kind=0 n=1 size=1024 work_ms=${DWORK:-3000} secs=3 warm=0 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 2
    T0=$(us); kill -9 $WP; wait $WP 2>/dev/null; WP=$W2; echo "T0 kill del worker 1=$T0"
    sleep 0.5
    echo "prima del kill del motore: $("$BIN" stats dir=out | grep -E '^secs|timers_pending' | tr '\n' ' ')"
    T1=$(us); kill -9 $EP; wait $EP 2>/dev/null
    cp out/engine.log out/engine-pre.log
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill del motore -> READY: $(ms $T1 $RU) ms; READY_us=$RU"
    grep -E "ripresa" out/engine.log
    wait $LP; cat out/load.txt
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "motore dopo la corsa: $("$BIN" stats dir=out | grep -E '^secs|timer_fire_delay' | tr '\n' ' ')"
    echo "tentativi iniziati: $(wc -l < out/work.log), tentativo 0 distinti: $(awk '$5==0{print $1,$2}' out/work.log | sort -u | wc -l), tentativo 0 eseguiti più di una volta: $(awk '$5==0{print $1,$2}' out/work.log | sort | uniq -d | wc -l), tentativo 1 iniziati: $(awk '$5==1' out/work.log | wc -l)"
    ./tstat.sh deadline out/engine-pre.log out/engine.log out/work.log $RU ;;
  theart)
    # Blocco C, battiti del worker. Il motore parte con lease_ms=${LEASE:-1500} (la scadenza di ogni tentativo è 1,5 s dall'ultimo
    # battito, che il worker manda ogni lease_ms/3) e grace_ms=${GRACE:-500} (finestra di attesa dei worker alla ripresa).
    # Attività da ${HWORK:-5000} ms, più lunghe del contratto: senza battiti scadrebbero tutte. Due worker; a 3 s si uccide il
    # worker 1 (i suoi tentativi smettono di battere), a 3,5 s kill -9 del motore e riavvio subito. Attesi: i tentativi del worker 2
    # non scadono (battono, anche attraverso il crash); quelli del worker 1 scadono 1,5 s dopo il loro ultimo battito e si rifanno.
    $TS_ENG "$BIN" worker dir=out link=grpc sessions=${WSESS:-4} $WL > out/worker2.log 2>&1 &
    W2=$!
    sleep 1
    $L rate=${ERATE:-100} kind=0 n=1 size=1024 work_ms=${HWORK:-5000} secs=2 warm=0 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 3
    T0=$(us); kill -9 $WP; wait $WP 2>/dev/null; WP=$W2; echo "T0 kill del worker 1=$T0"
    sleep 0.5
    echo "prima del kill del motore: $("$BIN" stats dir=out | grep -E '^secs|timers_pending' | tr '\n' ' ')"
    T1=$(us); kill -9 $EP; wait $EP 2>/dev/null
    cp out/engine.log out/engine-pre.log
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill del motore -> READY: $(ms $T1 $RU) ms; READY_us=$RU"
    grep -E "ripresa" out/engine.log
    wait $LP; cat out/load.txt
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "motore dopo la corsa: $("$BIN" stats dir=out | grep -E '^secs|timer_fire_delay' | tr '\n' ' ')"
    echo "tentativi iniziati: $(wc -l < out/work.log), tentativo 0 distinti: $(awk '$5==0{print $1,$2}' out/work.log | sort -u | wc -l), tentativo 0 eseguiti più di una volta: $(awk '$5==0{print $1,$2}' out/work.log | sort | uniq -d | wc -l), tentativo 1 iniziati: $(awk '$5==1' out/work.log | wc -l)"
    # inizio del tentativo 1 meno la scadenza dell'ultimo battito noto (TSET più recente del tentativo 0)
    ./tstat.sh deadline out/engine-pre.log out/engine.log out/work.log $RU ;;
  t100k)
    # Blocco C, 100.000 timer in sospeso. kind 2: un passo `wait` di ${TWAIT:-150000} ms per esecuzione, 5.000 esecuzioni/s per 20 s.
    # A ${KILL_AT:-35} s kill -9 con i timer ancora tutti in sospeso, riavvio subito: tempo fino a READY e RSS. Poi si aspetta che
    # tutti scattino (ognuno conclude il suo passo): nessuno perso, nessuno doppio, scarto di scatto p50/p99.
    $L rate=${TRATE:-5000} kind=2 n=1 size=64 work_ms=${TWAIT:-150000} secs=20 warm=0 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    wait $LP; cat out/load.txt
    sleep $(( ${KILL_AT:-35} - 20 ))
    echo "prima del kill: $("$BIN" stats dir=out | grep -E '^secs|timers_pending' | tr '\n' ' ')"
    ./rss.sh $EP $WP
    T0=$(us); kill -9 $EP; wait $EP 2>/dev/null
    start_engine
    RU=$(sed -n 's/^READY .*now_us=\([0-9]*\).*/\1/p' out/engine.log)
    echo "kill -> READY: $(ms $T0 $RU) ms ($(grep -E '^READY' out/engine.log))"
    grep -E "ripresa" out/engine.log
    echo "motore subito dopo READY: $("$BIN" stats dir=out | grep -E '^secs|timers_pending' | tr '\n' ' ')"
    ./rss.sh $EP $WP
    # attesa dello scatto di tutti: i timer partono tra 150 s e 170 s dopo l'accettazione
    i=0; until "$BIN" stats dir=out | grep -q "timers_pending=0"; do i=$((i+1)); [ $i -gt 120 ] && break; sleep 5; done
    echo "timer in sospeso dopo l'attesa: $("$BIN" stats dir=out | grep -E 'timers_pending' | tr '\n' ' ') (dopo $((i*5)) s)"
    ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    echo "motore dopo la corsa: $("$BIN" stats dir=out | grep -E '^secs|timer_fire_delay' | tr '\n' ' ')"
    ./rss.sh $EP $WP ;;
  dfull-iggy|dfull-db)
    # Blocco B, caso 4: disco pieno. 333 esecuzioni/s da 3 passi; zavorra che lascia ${CAPFREE} MB, attesa del rifiuto,
    # 8 s di osservazione, zavorra tolta, ripresa. iggy: tmpfs di docker (zavorra con docker exec); db: tmpfs nel namespace
    $L rate=${ERATE:-300} kind=0 n=3 size=1024 secs=${DSECS:-30} warm=1 stats=0 tenants=ab ledger=out/ledger.txt > out/load.txt &
    LP=$!
    sleep 3
    # la zavorra riempie tutto lo spazio libero meno CAPFREE MB (lo spazio già usato dall'avvio non si conta due volte)
    if [ $SC = dfull-iggy ]; then AV=$(docker exec ttspikebus-iggy-1 df -m /app/local_data | tail -1 | tr -s " " | cut -d" " -f4); else AV=$(NSE df -m $PWD/out/capmnt | tail -1 | tr -s " " | cut -d" " -f4); fi
    MB=$(( AV - CAPFREE ))
    if [ $SC = dfull-iggy ]; then
      docker exec ttspikebus-iggy-1 sh -c "dd if=/dev/zero of=/app/local_data/ballast bs=1M count=$MB 2>&1 | tail -1; df -m /app/local_data | tail -1"
      FULLRE="^BUS_DOWN"; UPRE="^BUS_UP"
    else
      NSE sh -c "dd if=/dev/zero of=$PWD/out/capmnt/ballast bs=1M count=$MB 2>&1 | tail -1; df -m $PWD/out/capmnt | tail -1"
      FULLRE="^STORAGE_FULL"; UPRE="^proiezione ripresa"
    fi
    T0=$(us); echo "T0 zavorra messa=$T0 (restano ${CAPFREE} MB)"
    waitlog "$FULLRE" 90 || echo "il disco non si è riempito in 90 s"
    TF=$(logus $(echo $FULLRE | tr -d '^') head); echo "T_pieno=$TF (zavorra -> pieno $(ms $T0 ${TF:-$T0}) ms)"
    sleep 8
    if [ $SC = dfull-iggy ]; then docker exec ttspikebus-iggy-1 sh -c "df -m /app/local_data | tail -1; rm -f /app/local_data/ballast; df -m /app/local_data | tail -1"
    else NSE sh -c "df -m $PWD/out/capmnt | tail -1; rm -f $PWD/out/capmnt/ballast; df -m $PWD/out/capmnt | tail -1"; fi
    T1=$(us); echo "T1 zavorra tolta=$T1"
    # ripresa = primo evento di ritorno DOPO la zavorra tolta (con Iggy il ciclo lo rilancia anche a disco pieno, e crolla alla prima scrittura)
    after() { grep -E "$1" out/engine.log | awk -v t=$T1 '$NF > t {print $NF; exit}'; }
    i=0; TU=; while [ $i -lt 1200 ]; do TU=$(after "$UPRE"); [ -n "$TU" ] && break; i=$((i+1)); sleep 0.1; done
    echo "T_ripresa=$TU (zavorra tolta -> ripresa $(ms $T1 ${TU:-$T1}) ms)"
    i=0; FD=; while [ $i -lt 600 ]; do FD=$(grep -E "^FIRST_DONE" out/engine.log | awk -v t=$T1 '$2 > t {print $2; exit}'); [ -n "$FD" ] && break; i=$((i+1)); sleep 0.1; done
    echo "zavorra tolta -> prima attività conclusa dopo la ripresa: $(ms $T1 ${FD:-$T1}) ms"
    wait $LP; cat out/load.txt
    echo "-- eventi di disco e bus del motore"; grep -E "^(BUS_|APPEND_|STORAGE_FULL|proiezione)" out/engine.log | cut -c1-220 | head -20
    echo "motore dopo la corsa: $("$BIN" stats dir=out | head -1)"
    if [ $SC = dfull-db ]; then
      ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do NSE "$BIN" verify db="$DBURL" bus=$BUS dir=$PWD/out ledger=$PWD/out/ledger.txt && ok=1 && break; sleep 3; done
    else
      ok=0; for i in 1 2 3 4 5 6 7 8 9 10 11 12; do "$BIN" verify db="$DBURL" bus=$BUS dir=out ledger=out/ledger.txt && ok=1 && break; sleep 3; done
    fi ;;
  *) echo "scenario sconosciuto" ;;
esac
[ $BUS = valkey ] && { echo "-- valkey: $(docker compose -p ttspikebus exec -T valkey valkey-cli -a "$VALKEY_PASSWORD" info stats 2>/dev/null | grep -E "total_forks" | tr -d "\r") $(docker compose -p ttspikebus exec -T valkey valkey-cli -a "$VALKEY_PASSWORD" info persistence 2>/dev/null | grep -E "aof_rewrites:|aof_last_rewrite_time_sec" | tr -d "\r" | tr "\n" " ")"; }
grep "^MIN" out/engine.log
echo "-- RSS dopo la corsa (motore, worker, bus)"
./rss.sh $EP $WP
echo "-- conteggio file archivio: $(find out/archive -type f 2>/dev/null | wc -l)"
# blocco D, parte seconda: righe `timer` rimaste per attività concluse (attese 0: un Timer dopo il suo Done sarebbe reinserito dal proiettore)
if [ $DB = sqlite ] && [ -f out/spike.db ]; then
  TSQL="SELECT count(*) FROM timer t JOIN activity a ON a.exec_id=t.exec_id AND a.step=t.step WHERE a.done_us IS NOT NULL"
  echo "sql: $TSQL"
  echo "timer_rows_for_finished_activities: $(sqlite3 out/spike.db "$TSQL" 2>&1); timer_rows_total: $(sqlite3 out/spike.db 'SELECT count(*) FROM timer' 2>&1)"
fi
# blocco D, parte terza: lo stesso controllo con Postgres (psql nel contenitore, socket locale)
if [ $DB = pg ]; then
  TSQL="SELECT count(*) FROM timer t JOIN activity a ON a.exec_id=t.exec_id AND a.step=t.step WHERE a.done_us IS NOT NULL"
  echo "sql: $TSQL"
  echo "timer_rows_for_finished_activities: $(docker compose -p ttspikebus exec -T postgres psql -U spike -d spike -tAc "$TSQL" 2>&1); timer_rows_total: $(docker compose -p ttspikebus exec -T postgres psql -U spike -d spike -tAc 'SELECT count(*) FROM timer' 2>&1)"
fi
# blocco D, parte terza: occorrenze di database bloccato nei registri del motore (prima e dopo un eventuale crash)
echo "busy_hits_engine_log: $(cat out/engine.log out/engine-pre.log 2>/dev/null | grep -ciE 'database is locked|sqlite_busy')"
sentinel_stop
[ -n "$SENTUSER" ] && { echo "sentinel (utente ro-sent) errori del processo: $(grep -c SENTINEL_ERR out/sentinel.txt)"; grep -m3 SENTINEL_ERR out/sentinel.txt | cut -c1-300; "$BIN" busauth case=offsets; }
stop_all
rm -f out/ro-sent.pw
rm -rf out/archive out/logs; [ -n "$SPIKE_ARCHIVE" ] && rm -rf "$SPIKE_ARCHIVE"
if [ "$SC" = dfull-iggy ]; then docker compose -p ttspikebus down -t 1 >/dev/null 2>&1; docker volume rm ttspikebus-iggycap >/dev/null; fi
