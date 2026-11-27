#!/bin/bash
# Righe della parte 2b: a1 a2 a2ctrl i1 t1 t1free t2 t3 m1 s1. Si carica da smoke2.sh (`. ./rows2b.sh`) dentro il suo `case`,
# con le funzioni e le variabili di quello (say, adm, stats, start_engine, restart_engine, stop_engine, kill_stream,
# run_one, $BIN, $D, $RUN, $ADDR, $PIN_E, $PIN_L, $SEED, $OUT, $ID, ENG_EXTRA).
# Durate: le variabili `*_DUR`, `A1_N`... portano i valori della corsa breve (smoke); senza, valgono quelli del piano.

# percentile (rango piu' vicino) di una colonna di numeri su stdin
pctl() { sort -n | awk -v p="$1" '{a[NR]=$1} END{if(NR==0){print "n/a";exit} k=p/100*NR; i=int(k); if(i<k)i++; if(i<1)i=1; printf "%.3f", a[i]}'; }
# attesa del prossimo «pronto» del processo del tenant $2 (conteggio gia' visto $1)
wait_ready_n() { for _ in $(seq 1 600); do [ "$(grep -c "stream ${2:-acme} pronto" "$RUN/engine.log")" -ge "$1" ] && return 0; sleep 0.1; done; return 1; }
# istante assoluto (ms) di inizio del carico, dalla riga «fase: attesa fino a <ms>» del generatore (con `--phase-s`)
phase_target() { for _ in $(seq 1 600); do t=$(sed -n 's/^fase: attesa fino a \([0-9]*\) .*/\1/p' "$1" | head -1); [ -n "$t" ] && { echo "$t"; return; }; sleep 0.5; done; }
wait_wall_ms() { while [ "$(date +%s%3N)" -lt "$1" ]; do sleep 0.2; done; }
# analisi delle righe LAG di un file del generatore: $1=file $2=da (s) $3=a (s)
lag_stats() {
  awk -v lo="$2" -v hi="$3" '
    /^LAG / { for (i = 1; i <= NF; i++) { split($i, kv, "="); v[kv[1]] = kv[2] }
              t = v["t"] + 0; a = v["accettati"] + 0; l = v["ritardo"] + 0
              if (t >= lo && t <= hi) { if (!n) { t1 = t; a1 = a }; t2 = t; a2 = a; n++; if (l > mx) mx = l; if (l > 200000) over++; s += l; ls[n] = l } }
    END { if (n < 2) { print "campioni=" n " (troppo pochi)"; exit }
          th = int(n / 3); for (i = 1; i <= th; i++) f += ls[i]; for (i = n - th + 1; i <= n; i++) g += ls[i]
          printf "campioni=%d da_t=%.2f a_t=%.2f media_eventi_s=%.0f ritardo_max=%d ritardo_medio=%.0f campioni_oltre_200000=%d ritardo_medio_primo_terzo=%.0f ritardo_medio_ultimo_terzo=%.0f\n",
                 n, t1, t2, (a2 - a1) / (t2 - t1), mx, s / n, over + 0, f / (th ? th : 1), g / (th ? th : 1) }' "$1"
}
b2_delivered() { stats | jq -c '.tenants.acme | {delivered, pending, refused, cpu_ns, rss_kb, pipes: [.pipes[] | {id, state, code, state_bytes, applied, late, future}]}'; }
UJ3SQL="SELECT window_start, window_end, asn, count(*) AS n FROM indicators GROUP BY tumble(INTERVAL '5 minutes'), asn HAVING count(*) > 50"

case $ID in
  # ------------------------------------------------------------------------------------------------ A1
  a1)
    N=${A1_N:-200}
    RUN=$D/run/a1; ./reset.sh
    say "=== A1: $N riavvii a freddo del ruolo con DataFusion vero e confinamento di prodotto (kill -9 del processo per tenant, il motore lo rilancia). Misura: dall'avvio del processo (fork compreso) a «pronto» (runtime caldo e confinamento applicato), presa dal motore"
    start_engine spec-t1.json || exit 1
    wait_ready_n 1 || { say "ERRORE: nessun avvio"; exit 1; }
    sleep 2
    for i in $(seq 1 "$N"); do
      kill_stream
      wait_ready_n $((i + 1)) || { say "ERRORE: riavvio $i non avvenuto"; break; }
      sleep 0.3
    done
    grep 'stream acme pronto' "$RUN/engine.log" | tail -n +2 > "$RUN/a1.lines"
    ms=$(sed 's/.*pronto in \([0-9.]*\) ms.*/\1/' "$RUN/a1.lines")
    init=$(sed 's/.*init_ms=\([0-9.]*\)).*/\1/' "$RUN/a1.lines")
    conf=$(grep -o 't_confine_us=[0-9]*' "$RUN/a1.lines" | cut -d= -f2)
    say "A1 riavvii=$(wc -l < "$RUN/a1.lines") confinati_landlock+seccomp=$(grep -c 'level=landlock+seccomp' "$RUN/a1.lines") rifiutati=$(grep -c 'refused' "$RUN/engine.log")"
    say "A1 avvio_a_pronto_ms: p50=$(echo "$ms" | pctl 50) p99=$(echo "$ms" | pctl 99) min=$(echo "$ms" | pctl 0.001) max=$(echo "$ms" | pctl 100) (soglie: p50 <= 100, p99 <= 250)"
    say "A1 dentro il processo (init_ms, dal messaggio iniziale a pronto): p50=$(echo "$init" | pctl 50) p99=$(echo "$init" | pctl 99); confinamento (t_confine_us): p50=$(echo "$conf" | pctl 50) p99=$(echo "$conf" | pctl 99)"
    say "A1 rapporto di confinamento di un avvio: $(tail -1 "$RUN/a1.lines" | sed 's/.*init_ms=[0-9.]*) //')"
    DEF="{\"id\":\"uj3\",\"version\":1,\"sql\":\"$UJ3SQL\"}"
    : > "$RUN/val.ms"
    for i in $(seq 1 "${A1_VAL:-30}"); do adm POST /v1/admin/acme/validate "$DEF" | grep -o '"ms":[0-9.]*' | cut -d: -f2 >> "$RUN/val.ms"; done
    say "A1 validazione a secco della query di UJ-3 (processo caldo e confinato, analisi + pianificazione + lotto sintetico), ms: n=$(wc -l < "$RUN/val.ms") prima=$(head -1 "$RUN/val.ms") p50=$(pctl 50 < "$RUN/val.ms") p99=$(pctl 99 < "$RUN/val.ms") max=$(pctl 100 < "$RUN/val.ms")"
    stop_engine ;;

  # ------------------------------------------------------------------------------------------------ A2
  a2|a2ctrl)
    RUN=$D/run/$ID; ./reset.sh
    if [ "$ID" = a2ctrl ]; then ENG_EXTRA=(--allow-probe --probe-port 47121 --ctrl-no-confine); else ENG_EXTRA=(--allow-probe --probe-port 47121); fi
    say "=== A2 ($ID): prove ostili dentro il processo per tenant vero (comando di prova del motore, solo con --allow-probe); motore: ${ENG_EXTRA[*]}"
    start_engine spec-a2.json "${ENG_EXTRA[@]}" || exit 1
    wait_ready_n 1 || exit 1
    wait_ready_n 1 beta || exit 1
    sleep 2
    EP=$(cat "$RUN/engine.pid")
    mkdir -p "$D/engine/payload/acme" "$D/engine/payload/beta"
    echo "dato del tenant acme" > "$D/engine/payload/acme/probe-own"
    echo "dato del tenant beta" > "$D/engine/payload/beta/probe-other"
    say "A2 percorsi: chiave=$D/engine/master.key database=$D/engine/starts.db configurazione=$HERE/spec-a2.json,$D/.env motore_pid=$EP"
    LEAKS=0; BAD=0; N=0; MUSTDENY=0; CTRLLEAKS=0
    pr() { # nome atteso(deny|allow) corpo-json
      local name=$1 exp=$2 body=$3 n0 out res cls
      n0=$(grep -c 'stream acme pronto' "$RUN/engine.log")
      out=$(adm POST /v1/admin/acme/probe "$body"); out=${out% \[http=*}
      res=$(echo "$out" | jq -r 'if .result then .result else (.error.code + ": " + .error.message) end' 2>/dev/null)
      if echo "$res" | grep -q 'leak=1'; then cls=leak
      elif echo "$res" | grep -q 'leak=0'; then cls=denied
      elif [ "$name" = exec ] && echo "$res" | grep -q 'segnale None, codice Some(0)'; then cls=leak; res="leak=1 (il processo e' stato sostituito da /bin/true: $res)"
      elif echo "$res" | grep -q 'segnale Some(31)'; then cls=denied_by_seccomp_SIGSYS
      elif echo "$res" | grep -q 'validation_process_died'; then cls=denied_process_died_OTHER
      else cls=unknown; fi
      N=$((N + 1)); local verdict
      if [ "$exp" = deny ]; then
        MUSTDENY=$((MUSTDENY + 1))
        case $cls in leak) LEAKS=$((LEAKS + 1)); CTRLLEAKS=$((CTRLLEAKS + 1)); verdict=FUGA ;; unknown) BAD=$((BAD + 1)); verdict=NON_CLASSIFICATA ;; *) verdict=OK ;; esac
      else
        case $cls in leak) verdict=OK ;; *) BAD=$((BAD + 1)); verdict=ROTTA ;; esac
      fi
      say "A2 prova=$name atteso=$exp esito=$cls => $verdict | $res"
      case $cls in denied_*) wait_ready_n $((n0 + 1)) || say "A2 ATTENZIONE: il processo per tenant non e' ripartito dopo $name" ;; esac
    }
    pr chiave_principale deny "{\"id\":\"file\",\"path\":\"$D/engine/master.key\"}"
    pr database_avvii deny "{\"id\":\"file\",\"path\":\"$D/engine/starts.db\"}"
    pr configurazione_spec deny "{\"id\":\"file\",\"path\":\"$HERE/spec-a2.json\"}"
    pr configurazione_env deny "{\"id\":\"file\",\"path\":\"$D/.env\"}"
    pr cartella_altro_tenant deny "{\"id\":\"dir\",\"path\":\"$D/engine/payload/beta\"}"
    pr file_altro_tenant deny "{\"id\":\"file\",\"path\":\"$D/engine/payload/beta/probe-other\"}"
    pr etc_passwd deny '{"id":"file","path":"/etc/passwd"}'
    pr proc_motore_environ deny "{\"id\":\"file\",\"path\":\"/proc/$EP/environ\"}"
    pr proc_motore_maps deny "{\"id\":\"file\",\"path\":\"/proc/$EP/maps\"}"
    pr proc_motore_cmdline deny "{\"id\":\"file\",\"path\":\"/proc/$EP/cmdline\"}"
    pr proc_motore_mem deny "{\"id\":\"file\",\"path\":\"/proc/$EP/mem\"}"
    pr ptrace_motore deny "{\"id\":\"ptrace\",\"pid\":$EP}"
    pr process_vm_readv_motore deny "{\"id\":\"pvm\",\"pid\":$EP}"
    pr segnale_al_motore deny "{\"id\":\"kill\",\"pid\":$EP}"
    pr porta_diversa_da_bus_e_motore deny '{"id":"tcp","port":47121}'
    pr exec deny '{"id":"exec"}'
    pr fork deny '{"id":"fork"}'
    pr scrittura_fuori_dalla_cartella deny "{\"id\":\"write\",\"path\":\"$D/engine/evil\"}"
    pr lettura_cartella_propria allow "{\"id\":\"file\",\"path\":\"$D/engine/payload/acme/probe-own\"}"
    pr scrittura_cartella_propria allow "{\"id\":\"write\",\"path\":\"$D/engine/payload/acme/probe-write\"}"
    pr porta_del_bus allow '{"id":"tcp","port":47101}'
    pr porta_del_motore allow '{"id":"tcp","port":47111}'
    out=$(adm POST /v1/admin/acme/probe '{"id":"env"}'); say "A2 informazione env: ${out% \[http=*}"
    # lo stato del confinamento visto da fuori (le prove dumpable e fds dall'interno uccidono il processo: prctl e fcntl non sono in lista)
    SP=$(cat "$D/engine/pids/stream-acme.pid")
    say "A2 stato del processo per tenant visto da /proc/$SP: $(grep -E '^(Seccomp|Seccomp_filters|NoNewPrivs|Threads):' /proc/$SP/status | tr '\n\t' '  ') limiti: $(grep -E 'Max (open files|core file size|file size)' /proc/$SP/limits | tr -s ' ' | tr '\n' ';')"
    say "A2 PR_SET_DUMPABLE visto da fuori (proprietario di /proc/<pid>/environ: root = non ispezionabile): processo_per_tenant=$(stat -c %U /proc/$SP/environ) motore=$(stat -c %U /proc/$EP/environ)"
    say "A2 riepilogo ($ID): prove=$N da_negare=$MUSTDENY fughe=$LEAKS rotte_o_non_classificate=$BAD file_creato_fuori=$([ -e "$D/engine/evil" ] && echo SI || echo no)"
    if [ "$ID" = a2 ]; then
      say "A2 verdetto: $([ "$LEAKS" = 0 ] && [ "$BAD" = 0 ] && echo PASS || echo FAIL) (0 fughe e prove ammesse funzionanti)"
    else
      say "A2 controllo positivo senza confinamento: fughe trovate=$CTRLLEAKS su $MUSTDENY prove da negare; $([ "$CTRLLEAKS" -ge 9 ] && echo 'il controllo vede le fughe' || echo 'IL CONTROLLO NON VEDE ABBASTANZA FUGHE')"
    fi
    stop_engine ;;

  # ------------------------------------------------------------------------------------------------ RC (riconnessione a Iggy sotto confinamento)
  rc)
    CHECKSPEC=spec-t1.json
    RUN=$D/run/rc; ./reset.sh; mkdir -p "$RUN"
    start_engine spec-t1.json || exit 1
    wait_ready_n 1 || exit 1
    sleep 2
    SP0=$(cat "$D/engine/pids/stream-acme.pid"); EP=$(cat "$RUN/engine.pid")
    say "=== RC: riavvio di Iggy (progetto ttspikestream) a 15 s dentro un carico di 3.000 eventi/s di 60 s; il processo per tenant e il motore NON si riavviano: devono riconnettersi da soli sotto confinamento"
    $PIN_L "$BIN" load --addr $ADDR --run-dir "$RUN/load" --seed "$SEED" --events 100000000 --rate 3000 --evrate 3000 --past-s 600 --duration-s 60 --conns 2 --lag-every-s 1 > "$RUN/load.out" 2>&1 &
    LP=$!
    sleep 15
    docker compose -p ttspikestream --env-file "$D/.env" restart iggy >/dev/null 2>&1; say "riavvio di Iggy eseguito a t=15 s"
    wait $LP
    sleep 10
    say "--- carico"; grep -v '^LAG' "$RUN/load.out" | tail -4 | tee -a "$OUT"
    say "RC ritardo del consumatore (t:ritardo): $(grep '^LAG' "$RUN/load.out" | sed 's/LAG t=\([0-9.]*\)s.*ritardo=\([0-9-]*\) max.*/\1:\2/' | tr '\n' ' ')"
    say "RC il processo per tenant e' lo stesso: prima=$SP0 dopo=$(cat "$D/engine/pids/stream-acme.pid"); il motore e' lo stesso: $EP vive=$(kill -0 "$EP" 2>/dev/null && echo si || echo no)"
    say "RC righe del processo per tenant sugli errori: $(grep -c 'lettura fallita' "$RUN/engine.log") letture fallite; riavvii del processo: $(grep -c 'terminato' "$RUN/engine.log")"
    say "--- check"
    $PIN_L "$BIN" check --spec spec-t1.json --run-dir "$RUN/load" --data "$D/engine" --addr $ADDR --wait-s "${WAIT:-240}" 2>&1 | tee -a "$OUT"
    stop_engine ;;

  # ------------------------------------------------------------------------------------------------ I1
  i1)
    PAIRS=${I1_PAIRS:-3}; DUR=${I1_DUR:-300}
    RANDOM=$SEED; FIRST=$((RANDOM % 2))
    say "=== I1: tenant A a 50.000 eventi/s, tenant B (pipeline al limite di stato hog e al limite di CPU cpuhog, budget $(jq -r '.tenants[1].pipelines[] | select(.id=="cpuhog") | .cpu_budget_ms' spec-i1.json) ms per micro-lotto, ritmo offerto 100.000 eventi/s); $PAIRS coppie da ${DUR} s; limite di CPU: nessuno del sistema in modalita' senza privilegi (l'ADR della sandbox non decide un limite di CPU per questo ruolo, solo RLIMIT_CPU 3600 s di rete di sicurezza): budget di CPU per micro-lotto con codice cpu_budget_exceeded; seme=$SEED prima coppia: $([ $FIRST = 0 ] && echo 'controllo poi prova' || echo 'prova poi controllo')"
    i1_phase() { # k fase(ctrl|test)
      local k=$1 ph=$2 label="i1-$1-$2"
      RUN=$D/run/$label; ./reset.sh; mkdir -p "$RUN"
      start_engine spec-i1.json || return 1
      wait_ready_n 1 acme; wait_ready_n 1 beta
      sleep 2
      say "--- coppia $k fase $ph: A da solo/ con B, $DUR s"
      $PIN_L "$BIN" load --addr $ADDR --run-dir "$RUN/loadA" --tenant acme --seed $((SEED + k)) --events 100000000 --rate 50000 --evrate 50000 --past-s 0 --duration-s "$DUR" --conns 4 > "$RUN/A.out" 2>&1 &
      local pa=$! pb=
      if [ "$ph" = test ]; then
        $PIN_L "$BIN" load --addr $ADDR --run-dir "$RUN/loadB" --tenant beta --seed $((SEED + 100 + k)) --prefix "bb$k" --events 100000000 --rate 100000 --evrate 100000 --past-s 0 --duration-s "$DUR" --conns 4 > "$RUN/B.out" 2>&1 &
        pb=$!
      fi
      wait $pa; [ -n "$pb" ] && wait $pb
      sleep 3
      say "I1 $label A: $(grep '^DONE' "$RUN/A.out")"
      say "I1 $label A (da t=30 s): $(lag_stats "$RUN/A.out" 30 100000)"
      if [ "$ph" = test ]; then
        say "I1 $label B: $(grep '^DONE' "$RUN/B.out")"
        say "I1 $label B pipeline: $(stats | jq -c '.tenants.beta | {cpu_ns, rss_kb, pipes: [.pipes[] | {id, state, code, state_bytes, applied}]}')"
        say "I1 $label B righe del processo: $(grep -c 'sospesa' "$RUN/engine.log") sospensioni; $(grep -h 'sospesa' "$RUN/engine.log" | sed 's/[0-9]\{4,\}/N/g' | sort | uniq -c | tr '\n' ';')"
      fi
      say "I1 $label A pipeline: $(stats | jq -c '.tenants.acme | {cpu_ns, rss_kb, pipes: [.pipes[] | {id, state, code, state_bytes}]}')"
      # regola di Marco (2026-11-27): ritmo e ritardo di A letti sui minuti 2-5 (t = 60..300 s) di ogni corsa da 5 minuti
      local m25; m25=$(lag_stats "$RUN/A.out" 60 300)
      say "I1 $label A (intera corsa): $(lag_stats "$RUN/A.out" 0 100000)"
      say "I1 $label A (minuti 2-5, t=60..300 s): $m25"
      I1R25[$label]=$(echo "$m25" | sed -n 's/.*media_eventi_s=\([0-9]*\).*/\1/p')
      I1LF[$label]=$(echo "$m25" | sed -n 's/.*ritardo_medio_primo_terzo=\([0-9]*\).*/\1/p')
      I1LL[$label]=$(echo "$m25" | sed -n 's/.*ritardo_medio_ultimo_terzo=\([0-9]*\).*/\1/p')
      I1RATE[$label]=$(grep '^DONE' "$RUN/A.out" | sed 's/.*ritmo=\([0-9]*\)\/s.*/\1/')
      I1STEADY[$label]=$(lag_stats "$RUN/A.out" 30 100000 | sed -n 's/.*media_eventi_s=\([0-9]*\).*/\1/p')
      stop_engine
    }
    declare -A I1RATE I1STEADY I1R25 I1LF I1LL
    for k in $(seq 1 "$PAIRS"); do
      if [ $(((FIRST + k - 1) % 2)) = 0 ]; then ORD="ctrl test"; else ORD="test ctrl"; fi
      for ph in $ORD; do i1_phase "$k" "$ph"; done
      c=${I1RATE[i1-$k-ctrl]}; t=${I1RATE[i1-$k-test]}; cs=${I1STEADY[i1-$k-ctrl]}; ts=${I1STEADY[i1-$k-test]}
      say "I1 coppia $k (ordine: $ORD): ritmo_A_controllo=$c ritmo_A_con_B=$t rapporto=$(awk -v a="$t" -v b="$c" 'BEGIN{printf "%.4f", (b > 0 ? a / b : 0)}') (soglia >= 0,95); da_t=30s: controllo=$cs con_B=$ts rapporto=$(awk -v a="$ts" -v b="$cs" 'BEGIN{printf "%.4f", (b > 0 ? a / b : 0)}') (righe aggiuntive di riferimento)"
      # verdetto della coppia (regola di Marco, 2026-11-27): ritmo minuti 2-5 >= 0,95 del controllo e, nella corsa con B,
      # media del ritardo dell'ultimo terzo <= media del primo terzo + 50.000 eventi
      cr=${I1R25[i1-$k-ctrl]}; tr=${I1R25[i1-$k-test]}; tf=${I1LF[i1-$k-test]}; tl=${I1LL[i1-$k-test]}
      say "I1 coppia $k VERDETTO (minuti 2-5): ritmo_controllo=$cr ritmo_con_B=$tr rapporto=$(awk -v a="$tr" -v b="$cr" 'BEGIN{printf "%.4f", (b > 0 ? a / b : 0)}') (soglia >= 0,95) ritardo_medio_primo_terzo(con_B)=$tf ultimo_terzo(con_B)=$tl (soglia ultimo <= primo + 50000) [controllo: primo=${I1LF[i1-$k-ctrl]} ultimo=${I1LL[i1-$k-ctrl]}] $(awk -v a="$tr" -v b="$cr" -v f="$tf" -v l="$tl" 'BEGIN{print (b > 0 && f != "" && l != "" && a / b >= 0.95 && l <= f + 50000) ? "PASS" : "FAIL"}')"
    done ;;

  # ------------------------------------------------------------------------------------------------ T1, T1-free
  t1|t1free)
    CHECKSPEC=spec-t1.json
    DUR=${T1_DUR:-600}
    if [ "$ID" = t1 ]; then L="--seed $SEED --events 70000000 --rate 105000 --evrate 105000 --past-s 0 --phase-s 0 --phase-period-s ${PHASE:-300} --duration-s $DUR --conns 16"
    else L="--seed $SEED --events 400000000 --rate 0 --evrate 300000 --past-s 0 --phase-s 0 --phase-period-s ${PHASE:-300} --duration-s $DUR --conns 16"; fi
    RUN=$D/run/$ID; ./reset.sh; mkdir -p "$RUN"
    start_engine spec-t1.json || exit 1
    sleep 2
    RD=$RUN/load; rm -rf "$RD"
    $PIN_L "$BIN" load --addr $ADDR --run-dir "$RD" $L > "$RUN/load.out" 2>&1 &
    LP=$!
    wait $LP
    say "--- carico"; grep -v '^LAG' "$RUN/load.out" | tail -5 | tee -a "$OUT"
    say "T1 minuti 2-10 (t=60..600 s, dal secondo minuto come nello spike del bus, campioni ogni 5 s): $(lag_stats "$RUN/load.out" 60 100000)"
    say "T1 serie del ritardo (t, ritardo): $(grep '^LAG' "$RUN/load.out" | sed 's/LAG t=\([0-9.]*\)s.*ritardo=\([0-9-]*\) max.*/\1:\2/' | tr '\n' ' ')"
    say "--- check (finestre chiuse con la regola della quiete entro 40 s dalla fine; tutte le finestre con dati devono essere chiuse)"
    $PIN_L "$BIN" check --spec spec-t1.json --run-dir "$RD" --data "$D/engine" --addr $ADDR --wait-s "${WAIT:-900}" --all-closed 2>&1 | tee -a "$OUT"
    say "stats_finali: $(b2_delivered)"
    stop_engine ;;

  # ------------------------------------------------------------------------------------------------ T2, S1
  t2|s1)
    DUR=${T2_DUR:-600}; K1=${T2_K1:-240}; K2=${T2_K2:-420}
    if [ "$ID" = s1 ]; then
      SPECF=spec-s1.json; SENT="SENTINEL$(head -c 12 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 12)"; SARG="--sentinel $SENT"
      say "S1 sentinella (stringa casuale di questa corsa, non un segreto): $SENT"
    else SPECF=spec-t2.json; SARG=""; fi
    CHECKSPEC=$SPECF
    RUN=$D/run/$ID; ./reset.sh; mkdir -p "$RUN"
    start_engine "$SPECF" || exit 1
    sleep 2
    RD=$RUN/load; rm -rf "$RD"
    say "=== $ID: ${DUR} s a ritmo pieno (105.000 eventi/s offerti, tempo dell'evento = ritmo offerto, origine alla partenza), pipeline di verifica (tutti i gruppi, senza filtro e senza HAVING); kill -9 del processo per tenant a $K1 s e del motore a $K2 s dall'inizio del carico"
    $PIN_L "$BIN" load --addr $ADDR --run-dir "$RD" --seed "$SEED" --events 70000000 --rate 105000 --evrate 105000 --past-s 0 --phase-s 0 --phase-period-s ${PHASE:-300} --duration-s "$DUR" --conns 16 --lag-every-s 1 $SARG > "$RUN/load.out" 2>&1 &
    LP=$!
    SNAP=
    if [ "$ID" = s1 ]; then
      # copie del checkpoint durante la corsa (ogni 20 s), con finestre aperte: il file finale non porta stato
      mkdir -p "$RUN/ckpt-snap"
      ( i=0; while :; do sleep 20; i=$((i + 1)); cp "$D/engine/payload/acme/ckpt.bin" "$RUN/ckpt-snap/ckpt-$i.bin" 2>/dev/null; done ) &
      SNAP=$!
    fi
    T0=$(phase_target "$RUN/load.out")
    say "inizio del carico (ms epoch): $T0"
    wait_wall_ms $((T0 + K1 * 1000)); KS=$(date +%s%3N); NS=$(grep -c 'stream acme pronto' "$RUN/engine.log"); kill_stream
    say "kill -9 stream a t=$(( (KS - T0) / 1000 )) s ($KS ms)"
    wait_ready_n $((NS + 1)); say "processo per tenant di nuovo pronto dopo $(( $(date +%s%3N) - KS )) ms dal kill"
    wait_wall_ms $((T0 + K2 * 1000)); KE=$(date +%s%3N); stop_engine
    say "kill -9 engine a t=$(( (KE - T0) / 1000 )) s ($KE ms)"
    restart_engine "$SPECF" "${ENG_EXTRA[@]}"; say "motore di nuovo pronto dopo $(( $(date +%s%3N) - KE )) ms dal kill"
    wait $LP
    [ -n "$SNAP" ] && kill "$SNAP" 2>/dev/null
    say "--- carico"; grep -v '^LAG' "$RUN/load.out" | tail -5 | tee -a "$OUT"
    say "$ID ritmo e ritardo sull'intero carico: $(lag_stats "$RUN/load.out" 0 100000)"
    for k in "stream:$K1" "engine:$K2"; do
      kind=${k%%:*}; kt=${k##*:}; ke=$([ "$kind" = stream ] && echo "$K2" || echo $((DUR + 120)))
      say "$ID recupero dopo il kill di $kind a t=${kt} s (campioni da 1 s): $(awk -v kt="$kt" -v ke="$ke" '
        /^LAG / { for (i = 1; i <= NF; i++) { split($i, kv, "="); v[kv[1]] = kv[2] }
                  t = v["t"] + 0; l = v["ritardo"] + 0
                  if (t < kt - 1) next
                  if (!seen_first) { seen_first = 1; prev_t = t }
                  if (t > kt && t < ke) { n++; if (l > pk) { pk = l; pkt = t }
                      if (pk > 200000 && !back && t > pkt && l <= 200000) { back = t }
                      if (l > 200000) over++ ;
                      if (t - prev_t > 1.5 && !gap) gap = t - prev_t }
                  prev_t = t }
        END { printf "campioni=%d picco_ritardo=%d a_t=%.1f secondi_con_ritardo_oltre_200000=%d rientro_sotto_200000_a_t=%s primo_buco_oltre_1,5_s_tra_campioni=%.1f (rientro misurato dal kill: %s s)\n",
              n, pk, pkt, over + 0, (back ? back : "mai/non_superato"), gap + 0, (back ? sprintf("%.1f", back - kt) : "n/a") }' "$RUN/load.out")"
    done
    say "--- righe del motore (riavvii)"; grep -E "terminato|pronto in|ripreso" "$RUN/engine.log" | sed 's/[0-9]\{5,\}/N/g' | sort | uniq -c | tee -a "$OUT"
    say "--- check (0 persi, 0 doppi, un avvio per riga)"
    $PIN_L "$BIN" check --spec "$SPECF" --run-dir "$RD" --data "$D/engine" --addr $ADDR --wait-s "${WAIT:-900}" --all-closed 2>&1 | tee -a "$OUT"
    say "stats_finali: $(b2_delivered)"
    if [ "$ID" = s1 ]; then
      say "--- S1: conteggio delle occorrenze in chiaro della sentinella (deve essere 0)"
      say "S1 dati di Iggy (grep -a su tutta la cartella, dal container di Iggy in sola lettura): $(docker run --rm -v "$D/iggy:/d:ro" --entrypoint sh apache/iggy:0.10.0 -c "grep -raoF '$SENT' /d | wc -l")"
      say "S1 cartella dei checkpoint ($D/engine/payload): $(grep -raoF "$SENT" "$D/engine/payload" | wc -l); copie scattate durante la corsa ($(ls "$RUN/ckpt-snap" | wc -l) file di cifrati): $(grep -raoF "$SENT" "$RUN/ckpt-snap" | wc -l)"
      say "S1 file dei checkpoint presenti: $(ls -l "$D"/engine/payload/acme | tail -n +2 | awk '{print $5, $9}' | tr '\n' ';')"
      say "S1 (informazione, non richiesto) database degli avvii: $(grep -aoF "$SENT" "$D/engine/starts.db"* | wc -l) occorrenze in chiaro (le righe delle finestre portano il gruppo kind)"
      say "S1 controllo positivo dopo la decifratura:"
      "$BIN" probe s1dec --data "$D/engine" --tenant acme --stream indicators --sentinel "$SENT" --ckpt-dir "$RUN/ckpt-snap" 2>&1 | tee -a "$OUT"
    fi
    stop_engine ;;

  # ------------------------------------------------------------------------------------------------ T3
  t3)
    CHECKSPEC=spec-t3.json
    DUR=${T3_DUR:-180}
    for R in 1000 100000; do
      RUN=$D/run/t3-$R; ./reset.sh; mkdir -p "$RUN"
      start_engine spec-t3.json || exit 1
      sleep 2
      C=4; [ "$R" -ge 50000 ] && C=16
      say "=== T3 a $R eventi/s per ${DUR} s (finestre da 30 s: piu' chiusure in una corsa breve; tempo dell'evento = ritmo offerto)"
      $PIN_L "$BIN" load --addr $ADDR --run-dir "$RUN/load" --seed $((SEED + R)) --events $((R * (DUR + 60))) --rate "$R" --evrate "$R" --past-s 0 --phase-s 0 --phase-period-s 30 --duration-s "$DUR" --conns $C > "$RUN/load.out" 2>&1
      grep '^DONE' "$RUN/load.out" | tee -a "$OUT"
      sleep 75   # quiete (30 s) + ritardo (10 s) + consegna delle ultime righe
      say "T3 $R eventi/s, latenze del processo per tenant (microsecondi; secchi con errore relativo <= 4,4 %; n = campioni): $(stats | jq -c '.tenants.acme.t3')"
      say "T3 $R eventi/s, consegna: $(b2_delivered)"
      stop_engine
    done ;;

  # ------------------------------------------------------------------------------------------------ M1
  m1)
    CHECKSPEC=spec-t1.json
    BIN2=$(dirname "$BIN")/idle_nodf
    memrow() { # etichetta pid
      local p=$2
      say "M1 $1 pid=$p VmRSS_kB=$(awk '/^VmRSS/{print $2}' /proc/$p/status) RssAnon_kB=$(awk '/^RssAnon/{print $2}' /proc/$p/status) RssFile_kB=$(awk '/^RssFile/{print $2}' /proc/$p/status) Pss_kB=$(awk '/^Pss:/{print $2}' /proc/$p/smaps_rollup 2>/dev/null) thread=$(awk '/^Threads/{print $2}' /proc/$p/status)"
    }
    REST=${M1_REST:-60}
    say "M1 binari: con DataFusion = streamspike ($(stat -c %s "$BIN") byte), senza DataFusion = idle_nodf ($(stat -c %s "$BIN2") byte), seconda compilazione minima dello stesso crate: tokio + client di Iggy + SQLite + socket di ascolto, nessun codice di Arrow e DataFusion"
    # a) senza DataFusion
    RUN=$D/run/m1a; ./reset.sh; mkdir -p "$RUN"
    $PIN_E "$BIN2" --data "$D/engine" --listen $ADDR > "$RUN/nodf.log" 2>&1 &
    PN=$!; sleep "$REST"; memrow "senza_DataFusion_a_riposo_${REST}s" $PN; kill -9 $PN 2>/dev/null; sleep 1
    # b) motore con DataFusion, nessun tenant, nessuna pipeline
    RUN=$D/run/m1b; ./reset.sh; ENG_EXTRA=(--m1-keep-dumpable); start_engine spec-m0.json "${ENG_EXTRA[@]}" || exit 1
    PE=$(cat "$RUN/engine.pid"); sleep "$REST"; memrow "con_DataFusion_motore_a_riposo_${REST}s_nessun_tenant" "$PE"; stop_engine
    # c) motore con un tenant e una pipeline, a riposo; poi a 100.000 eventi/s
    RUN=$D/run/m1c; ./reset.sh; start_engine spec-t1.json "${ENG_EXTRA[@]}" || exit 1
    PE=$(cat "$RUN/engine.pid"); wait_ready_n 1 || exit 1; PS=$(cat "$D/engine/pids/stream-acme.pid")
    sleep "$REST"; memrow "motore_con_un_tenant_a_riposo_${REST}s" "$PE"; memrow "processo_per_tenant_a_riposo_${REST}s_una_pipeline" "$PS"
    say "M1 statistiche del processo per tenant a riposo: $(stats | jq -c '.tenants.acme | {rss_kb, pipes: [.pipes[] | {id, state_bytes}]}')"
    DUR=${M1_DUR:-120}
    $PIN_L "$BIN" load --addr $ADDR --run-dir "$RUN/load" --seed "$SEED" --events 100000000 --rate 100000 --evrate 100000 --past-s 0 --phase-s 0 --phase-period-s ${PHASE:-300} --duration-s "$DUR" --conns 16 > "$RUN/load.out" 2>&1 &
    LP=$!
    T0=$(phase_target "$RUN/load.out"); wait_wall_ms $((T0 + 10000))
    while kill -0 $LP 2>/dev/null; do
      memrow "processo_per_tenant_a_100000_al_secondo" "$PS"; say "M1 stato: $(stats | jq -c '.tenants.acme | {rss_kb, pipes: [.pipes[] | {id, state_bytes, open_windows}]}')"; sleep 10
    done
    wait $LP; grep '^DONE' "$RUN/load.out" | tee -a "$OUT"
    sleep 5; memrow "processo_per_tenant_dopo_il_carico" "$PS"; say "M1 stato dopo il carico: $(stats | jq -c '.tenants.acme | {rss_kb, pipes: [.pipes[] | {id, state_bytes, open_windows}]}')"
    stop_engine ;;
esac
