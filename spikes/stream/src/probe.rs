//! Sottocomando `probe`: prove ostili sul SQL (V1, V2). Codice da buttare.
//!
//! - `probe v1 --spec spec-v1.json`: lista di query ammesse e rifiutate, ognuna un caso con il suo codice atteso;
//!   100 % classificate come scritto; errori di sintassi con riga e colonna; lo stream di un altro tenant da' lo stesso
//!   errore di uno inesistente.
//! - `probe v2 --spec spec-v1.json [--off 1,3] --label <nome del file di uscita>`: le query ostili girano, una per
//!   processo figlio, con le barriere accese o spente. Le barriere si spengono solo se il nome del file di uscita porta
//!   `-ctrl-` (controllo positivo). Il processo del prodotto non ha nessun modo di spegnerle (`Barriers::ALL`).
//!
//! Osservazione di V2 senza strace, per ogni caso (nel figlio):
//! - letture di file: un osservatore `inotify` sul file civetta (e sulla cartella delle scritture) conta open, access,
//!   create, modify; in piu' un campionatore della tabella dei descrittori (`/proc/self/fd` e `fdinfo`, ogni 0,5 ms)
//!   riporta i file regolari e le prese aperte durante la query, con la posizione di lettura;
//! - connessioni: il padre tiene un ascoltatore su una porta di loopback e conta le `accept`;
//! - gli osservatori sono provati da soli con i casi `selftest_open` e `selftest_connect` (devono vedere l'evento).
//!
//! Limite noto: un `open` e `close` piu' breve del periodo di campionamento sfugge al campionatore (non a `inotify`).

use crate::dialect::{Barriers, analyse, validate_set};
use crate::pipeline::{Pipe, context};
use crate::spec::*;
use datafusion::arrow::array::{Int64Array, StringArray};
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::prelude::SQLOptions;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub async fn run() {
    match std::env::args().nth(2).as_deref() {
        Some("v1") => v1().await,
        Some("v2") => v2_parent(),
        Some("v2-case") => v2_child().await,
        Some("s1dec") => s1dec().await,
        _ => eprintln!("uso: streamspike probe v1|v2 --spec <file>"),
    }
}

fn pdef(sql: &str) -> PipelineDef {
    serde_json::from_value(json!({"id":"p","version":1,"sql":sql})).unwrap()
}

fn tenant_streams(spec: &Spec, t: &str) -> Vec<StreamDef> {
    spec.tenants
        .iter()
        .find(|x| x.id == t)
        .map(|x| x.streams.clone())
        .unwrap_or_default()
}

/// Classificazione di una query: `accepted`, o il codice dell'errore (con riga e colonna se c'e').
async fn classify(sql: &str, streams: &[StreamDef]) -> (String, Option<(u64, u64)>) {
    let d = pdef(sql);
    match Pipe::build(&d, streams, 64 << 20, &Barriers::ALL).await {
        Ok(ps) => {
            // la query ammessa deve anche reggere una prova a secco
            for p in &ps {
                let sd = streams.iter().find(|s| s.name == p.stream).unwrap();
                if let Err(e) = p.dry_run(sd, 64).await {
                    return (format!("dry_run_failed:{}", e.code), None);
                }
            }
            (format!("accepted:{}", ps.len()), None)
        }
        Err(e) => (e.code.to_string(), e.line.zip(e.column)),
    }
}

async fn v1() {
    let spec = Spec::load(&arg("--spec").expect("--spec"));
    let acme = tenant_streams(&spec, "acme");
    let mut cases: Vec<(&str, &str, String, &str)> = vec![]; // (nome, tenant, sql, atteso)
    let mut c = |n: &'static str, t: &'static str, sql: &str, exp: &'static str| {
        cases.push((n, t, sql.to_string(), exp))
    };
    // ammesse
    c(
        "ok_tumble_having",
        "acme",
        "SELECT window_start, window_end, asn, count(*) AS n FROM indicators GROUP BY tumble(INTERVAL '5 minutes'), asn HAVING count(*) > 50",
        "accepted:1",
    );
    c(
        "ok_hop",
        "acme",
        "SELECT window_start, window_end, kind, count(*) AS n, count(DISTINCT asn) AS asns, sum(asn) AS s, min(asn) AS lo, max(asn) AS hi, avg(asn) AS av FROM indicators WHERE ip LIKE '10.%' GROUP BY hop(INTERVAL '5 minutes', INTERVAL '1 minute'), kind",
        "accepted:1",
    );
    c(
        "ok_distinct",
        "acme",
        "SELECT DISTINCT window_start, window_end, kind FROM indicators WHERE asn < 1100 GROUP BY tumble(INTERVAL '5 minutes'), kind, asn",
        "accepted:1",
    );
    c(
        "ok_filter_functions",
        "acme",
        "SELECT coalesce(kind, 'x') AS a, lower(kind) AS b, upper(kind) AS c, length(ip) AS d, substr(ip, 1, 3) AS e, starts_with(ip, '10') AS f, abs(asn) AS g, round(asn / 3.0) AS h, floor(asn / 3.0) AS i, ceil(asn / 3.0) AS j, CASE WHEN asn > 1 THEN 1 ELSE 0 END AS k, CAST(asn AS TEXT) AS l, CAST(asn AS DOUBLE) AS m FROM indicators WHERE ip LIKE '10.%' AND asn BETWEEN 1 AND 2000 AND kind IN ('a', 'b') OR NOT asn = 3",
        "accepted:1",
    );
    c(
        "ok_union_all",
        "acme",
        "SELECT asn, kind FROM indicators WHERE asn = 1 UNION ALL SELECT asn, kind FROM indicators2 WHERE asn = 2",
        "accepted:2",
    );
    c(
        "ok_epoch_interval_units",
        "acme",
        "SELECT window_start, window_end, count(*) AS n FROM indicators GROUP BY tumble(INTERVAL '20' SECOND)",
        "accepted:1",
    );
    // rifiutate, una per caso
    c(
        "ddl_passwd",
        "acme",
        "CREATE EXTERNAL TABLE t STORED AS CSV LOCATION '/etc/passwd'",
        "statement_not_allowed",
    );
    c(
        "copy_to_file",
        "acme",
        "COPY (SELECT asn FROM indicators) TO '/tmp/out.csv'",
        "statement_not_allowed",
    );
    c(
        "connector_bus_ddl",
        "acme",
        "CREATE EXTERNAL TABLE bus STORED AS CSV LOCATION 'tcp://127.0.0.1:47101'",
        "statement_not_allowed",
    );
    c(
        "connector_bus_select",
        "acme",
        "SELECT asn FROM \"tcp://127.0.0.1:47101\"",
        "unknown_stream",
    );
    c(
        "dml_insert",
        "acme",
        "INSERT INTO indicators VALUES (1, 'a', 'b', 0)",
        "statement_not_allowed",
    );
    c(
        "statement_set",
        "acme",
        "SET datafusion.execution.batch_size = 1",
        "statement_not_allowed",
    );
    c(
        "statement_explain",
        "acme",
        "EXPLAIN SELECT asn FROM indicators",
        "statement_not_allowed",
    );
    c(
        "two_statements",
        "acme",
        "SELECT asn FROM indicators; SELECT asn FROM indicators",
        "statement_not_allowed",
    );
    c(
        "create_function",
        "acme",
        "CREATE FUNCTION f(x BIGINT) RETURNS BIGINT AS 'x'",
        "statement_not_allowed",
    );
    c(
        "user_function_call",
        "acme",
        "SELECT myudf(asn) FROM indicators",
        "function_not_allowed",
    );
    c(
        "table_fn_generate_series",
        "acme",
        "SELECT * FROM generate_series(1, 1000000000)",
        "table_function_not_allowed",
    );
    c(
        "table_fn_range",
        "acme",
        "SELECT value FROM range(10)",
        "table_function_not_allowed",
    );
    c(
        "join",
        "acme",
        "SELECT a.asn FROM indicators a JOIN indicators b ON a.asn = b.asn",
        "join_not_allowed",
    );
    c(
        "cartesian_comma",
        "acme",
        "SELECT asn FROM indicators, indicators2",
        "join_not_allowed",
    );
    c(
        "cartesian_cross",
        "acme",
        "SELECT asn FROM indicators CROSS JOIN indicators2",
        "join_not_allowed",
    );
    c(
        "repeat_huge",
        "acme",
        "SELECT repeat('x', 2000000000) AS r FROM indicators",
        "function_not_allowed",
    );
    c(
        "lpad_huge",
        "acme",
        "SELECT lpad(kind, 2000000000, 'x') AS r FROM indicators",
        "function_not_allowed",
    );
    c(
        "now",
        "acme",
        "SELECT now() AS t FROM indicators",
        "function_not_allowed",
    );
    c(
        "random",
        "acme",
        "SELECT random() AS t FROM indicators",
        "function_not_allowed",
    );
    let huge = "x".repeat(2000);
    c(
        "literal_huge",
        "acme",
        &format!("SELECT asn FROM indicators WHERE kind = '{huge}'"),
        "literal_too_large",
    );
    let big = format!("SELECT asn FROM indicators{}", " ".repeat(17 << 10));
    c("query_over_16kib", "acme", &big, "query_too_large");
    c(
        "deep_parens_100",
        "acme",
        &format!(
            "SELECT {}asn{} AS a FROM indicators",
            "(".repeat(100),
            ")".repeat(100)
        ),
        "too_deep",
    );
    c(
        "deep_parens_extreme",
        "acme",
        &format!(
            "SELECT {}asn{} AS a FROM indicators",
            "(".repeat(4000),
            ")".repeat(4000)
        ),
        "too_deep",
    );
    c(
        "deep_binary_chain",
        "acme",
        &format!("SELECT asn{} AS a FROM indicators", " + 1".repeat(3000)),
        "too_deep",
    );
    c(
        "stream_of_other_tenant",
        "acme",
        "SELECT asn FROM secret",
        "unknown_stream",
    );
    c(
        "stream_nonexistent",
        "acme",
        "SELECT asn FROM nosuchstream",
        "unknown_stream",
    );
    c(
        "syntax_error_pos",
        "acme",
        "SELECT asn\nFROM indicators\nWHERE asn >> ;",
        "syntax_error",
    );
    c(
        "syntax_error_keyword",
        "acme",
        "SELEC asn FROM indicators",
        "syntax_error",
    );
    c(
        "window_beyond_retention",
        "acme",
        "SELECT window_start, window_end, count(*) AS n FROM indicators_short GROUP BY tumble(INTERVAL '1 hour')",
        "window_exceeds_retention",
    );
    c(
        "subquery_from",
        "acme",
        "SELECT asn FROM (SELECT asn FROM indicators)",
        "subquery_not_allowed",
    );
    c(
        "subquery_where",
        "acme",
        "SELECT asn FROM indicators WHERE asn IN (SELECT asn FROM indicators)",
        "subquery_not_allowed",
    );
    c(
        "cte",
        "acme",
        "WITH c AS (SELECT asn FROM indicators) SELECT asn FROM c",
        "cte_not_allowed",
    );
    c(
        "order_by",
        "acme",
        "SELECT asn FROM indicators ORDER BY asn",
        "order_by_not_allowed",
    );
    c(
        "limit",
        "acme",
        "SELECT asn FROM indicators LIMIT 10",
        "limit_not_allowed",
    );
    c(
        "over_window_fn",
        "acme",
        "SELECT sum(asn) OVER () AS s FROM indicators",
        "window_function_not_allowed",
    );
    c(
        "wildcard",
        "acme",
        "SELECT * FROM indicators",
        "wildcard_not_allowed",
    );
    c(
        "union_distinct",
        "acme",
        "SELECT asn FROM indicators UNION SELECT asn FROM indicators2",
        "set_operation_not_allowed",
    );
    c(
        "except",
        "acme",
        "SELECT asn FROM indicators EXCEPT SELECT asn FROM indicators2",
        "set_operation_not_allowed",
    );
    c("values", "acme", "VALUES (1)", "construct_not_allowed");
    c(
        "reserved_name",
        "acme",
        "SELECT __lb FROM indicators",
        "reserved_name",
    );
    c(
        "cast_int32",
        "acme",
        "SELECT CAST(asn AS INT) AS a FROM indicators",
        "type_not_allowed",
    );
    c(
        "cast_date",
        "acme",
        "SELECT CAST(asn AS DATE) AS a FROM indicators",
        "type_not_allowed",
    );
    c(
        "interval_days",
        "acme",
        "SELECT window_start, window_end, count(*) AS n FROM indicators GROUP BY tumble(INTERVAL '5 days')",
        "window_invalid",
    );
    c(
        "hop_not_multiple",
        "acme",
        "SELECT window_start, window_end, count(*) AS n FROM indicators GROUP BY hop(INTERVAL '5 minutes', INTERVAL '2 minutes')",
        "window_invalid",
    );
    c(
        "two_windows",
        "acme",
        "SELECT window_start, window_end, count(*) AS n FROM indicators GROUP BY tumble(INTERVAL '5 minutes'), tumble(INTERVAL '1 minute')",
        "window_invalid",
    );
    c(
        "aggregate_in_where",
        "acme",
        "SELECT asn FROM indicators WHERE count(*) > 1",
        "aggregate_misplaced",
    );
    c(
        "window_in_select",
        "acme",
        "SELECT tumble(INTERVAL '5 minutes') AS w FROM indicators",
        "window_misplaced",
    );
    c(
        "qualified_column",
        "acme",
        "SELECT x.asn FROM indicators",
        "qualified_name_not_allowed",
    );
    c(
        "qualified_table",
        "acme",
        "SELECT asn FROM main.indicators",
        "qualified_name_not_allowed",
    );
    c(
        "sum_distinct",
        "acme",
        "SELECT window_start, window_end, sum(DISTINCT asn) AS s FROM indicators GROUP BY tumble(INTERVAL '5 minutes')",
        "function_not_allowed",
    );
    c(
        "concat_operator",
        "acme",
        "SELECT kind || ip AS a FROM indicators",
        "operator_not_allowed",
    );
    c(
        "exists",
        "acme",
        "SELECT asn FROM indicators WHERE EXISTS (SELECT 1)",
        "subquery_not_allowed",
    );
    c(
        "group_by_rollup",
        "acme",
        "SELECT asn FROM indicators GROUP BY ROLLUP (asn)",
        "construct_not_allowed",
    );
    let mut ok = 0;
    let n = cases.len();
    for (name, t, sql, exp) in &cases {
        let streams = tenant_streams(&spec, t);
        let (got, pos) = classify(sql, &streams).await;
        let mut good = got == *exp;
        if *exp == "syntax_error" {
            good &= pos.is_some();
        }
        ok += good as usize;
        println!(
            "CASO {name} atteso={exp} ottenuto={got}{} {}",
            pos.map(|(l, c)| format!(" riga={l} colonna={c}"))
                .unwrap_or_default(),
            if good { "OK" } else { "KO" }
        );
    }
    // cicli tra pipeline, ognuno un caso a parte (validazione dell'insieme)
    let mut graph_cases = 0;
    let mk = |id: &str, sql: &str, dest: &str| -> PipelineDef {
        serde_json::from_value(json!({"id":id,"version":1,"sql":sql,"dest_stream":dest})).unwrap()
    };
    let cyc = vec![
        mk(
            "a",
            "SELECT asn, ip, kind, ts FROM indicators",
            "indicators2",
        ),
        mk(
            "b",
            "SELECT asn, ip, kind, ts FROM indicators2",
            "indicators",
        ),
    ];
    let selfloop = vec![mk(
        "a",
        "SELECT asn, ip, kind, ts FROM indicators",
        "indicators",
    )];
    let chain = vec![mk(
        "a",
        "SELECT asn, ip, kind, ts FROM indicators",
        "indicators2",
    )];
    for (name, defs, exp) in [
        ("cycle_two_pipelines", cyc, "pipeline_cycle"),
        ("cycle_self", selfloop, "pipeline_cycle"),
        ("chain_no_cycle", chain, "accepted"),
    ] {
        graph_cases += 1;
        let got = match validate_set(&defs, &acme, true) {
            Ok(()) => "accepted".to_string(),
            Err((_, e)) => e.code.to_string(),
        };
        let good = got == exp;
        ok += good as usize;
        println!(
            "CASO {name} atteso={exp} ottenuto={got} {}",
            if good { "OK" } else { "KO" }
        );
    }
    // lo stream di un altro tenant e uno inesistente: stesso errore, byte per byte (stessa query, tenant che non ha lo stream)
    let q = "SELECT asn FROM secret";
    let e1 = Pipe::build(&pdef(q), &acme, 64 << 20, &Barriers::ALL)
        .await
        .err()
        .map(|e| serde_json::to_string(&e).unwrap());
    let e2 = Pipe::build(
        &pdef(q),
        &tenant_streams(&spec, "initech"),
        64 << 20,
        &Barriers::ALL,
    )
    .await
    .err()
    .map(|e| serde_json::to_string(&e).unwrap());
    let good = e1.is_some() && e1 == e2;
    ok += good as usize;
    graph_cases += 1;
    println!(
        "CASO other_tenant_equals_nonexistent acme={e1:?} initech={e2:?} {}",
        if good { "OK" } else { "KO" }
    );
    // UNION ALL: i due rami producono righe con le chiavi dei rami
    let br = analyse(
        &pdef("SELECT asn FROM indicators UNION ALL SELECT asn FROM indicators2"),
        &acme,
        true,
    )
    .map(|b| b.len());
    println!("UNION ALL rami={br:?}");
    println!(
        "V1 casi={} classificati_come_scritto={ok} {}",
        n + graph_cases,
        if ok == n + graph_cases {
            "PASS"
        } else {
            "FAIL"
        }
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// V2

fn off_list(s: &str) -> Barriers {
    let mut b = Barriers::ALL;
    for c in s.split(',').filter(|c| !c.is_empty()) {
        match c {
            "1" => b.syntax = false,
            "2" => b.options = false,
            "3" => b.registration = false,
            "4" => b.memory = false,
            "all" => {
                b = Barriers {
                    syntax: false,
                    options: false,
                    registration: false,
                    memory: false,
                }
            }
            _ => panic!("barriera sconosciuta {c}"),
        }
    }
    b
}

/// SQL dei casi ostili di V2. `canary`: file civetta; `dir`: cartella delle scritture; `port`: porta di loopback.
fn v2_sql(case: &str, canary: &str, dir: &str, port: u16) -> String {
    match case {
        "ddl_file" => format!("CREATE EXTERNAL TABLE t STORED AS CSV LOCATION '{canary}'"),
        "copy_to_file" => {
            format!("COPY (SELECT asn FROM indicators) TO '{dir}/out.csv' STORED AS CSV")
        }
        "connector_port" => format!(
            "CREATE EXTERNAL TABLE t STORED AS CSV LOCATION 'http://127.0.0.1:{port}/x.csv'"
        ),
        "dml_insert" => "INSERT INTO indicators VALUES (1, 'a', 'b', 0)".into(),
        "statement_set" => "SET datafusion.execution.batch_size = 1".into(),
        "create_function" => "CREATE FUNCTION f(x BIGINT) RETURNS BIGINT AS 'x'".into(),
        "table_fn_generate_series" => {
            "SELECT count(*) FROM generate_series(1, 100000000000)".into()
        }
        "cartesian" => "SELECT array_agg(a.asn) FROM indicators a, indicators b".into(),
        "repeat_huge" => "SELECT repeat(kind, 60000) AS r FROM indicators".into(),
        "repeat_over_limit" => "SELECT repeat(kind, 2000000000) AS r FROM indicators".into(),
        "literal_huge" => format!(
            "SELECT asn FROM indicators WHERE kind = '{}'",
            "x".repeat(20 << 20)
        ),
        "deep_parens" => format!(
            "SELECT {}asn{} AS a FROM indicators",
            "(".repeat(20000),
            ")".repeat(20000)
        ),
        "join_self" => "SELECT a.asn FROM indicators a JOIN indicators b ON true".into(),
        _ => panic!("caso sconosciuto {case}"),
    }
}

const V2_CASES: [&str; 13] = [
    "ddl_file",
    "copy_to_file",
    "connector_port",
    "dml_insert",
    "statement_set",
    "create_function",
    "table_fn_generate_series",
    "cartesian",
    "repeat_huge",
    "repeat_over_limit",
    "literal_huge",
    "deep_parens",
    "join_self",
];

fn v2_parent() {
    let label = arg("--label").unwrap_or_default();
    let off = arg("--off").unwrap_or_default();
    let configs: Vec<(String, String)> = if arg("--matrix").is_some() {
        // matrice dei controlli: ogni barriera da sola, ognuna spenta, tutte spente
        let mut v = vec![("all_on".to_string(), String::new())];
        for k in ["1", "2", "3", "4"] {
            v.push((format!("off_{k}"), k.to_string()));
        }
        v.push(("off_1_2_4_only3_on".into(), "1,2,4".into()));
        v.push(("off_2_3_4_only1_on".into(), "2,3,4".into()));
        v.push(("off_1_3_4_only2_on".into(), "1,3,4".into()));
        v.push(("off_1_2_3_only4_on".into(), "1,2,3".into()));
        v.push(("off_all".into(), "all".into()));
        v
    } else {
        vec![("all_on".into(), off.clone())]
    };
    if configs.iter().any(|(_, o)| !o.is_empty()) && !label.contains("-ctrl-") {
        eprintln!(
            "rifiuto: le barriere si spengono solo in un file di controllo positivo (nome con `-ctrl-`)"
        );
        std::process::exit(2);
    }
    let base = std::env::temp_dir();
    let work =
        std::path::PathBuf::from(arg("--work").unwrap_or_else(|| base.display().to_string()))
            .join(format!("v2-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    let canary = work.join("canary.csv");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let accepts = Arc::new(AtomicU64::new(0));
    {
        let a = accepts.clone();
        std::thread::spawn(move || {
            for s in listener.incoming() {
                if s.is_ok() {
                    a.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }
    let exe = std::env::current_exe().unwrap();
    let spec_path = arg("--spec").expect("--spec");
    let mut cases: Vec<&str> = V2_CASES.to_vec();
    cases.extend(["selftest_open", "selftest_connect"]);
    let mut summary: BTreeMap<String, (usize, usize, usize, u64, u64)> = BTreeMap::new(); // fermate, completate, timeout, eventi file, accept
    for (cfg, o) in &configs {
        for case in &cases {
            std::fs::write(&canary, "a,b\n1,2\n").unwrap();
            let out_dir = work.join("out");
            let _ = std::fs::remove_dir_all(&out_dir);
            std::fs::create_dir_all(&out_dir).unwrap();
            let acc0 = accepts.load(Ordering::Relaxed);
            let mut ch = std::process::Command::new(&exe)
                .args([
                    "probe",
                    "v2-case",
                    "--spec",
                    &spec_path,
                    "--case",
                    case,
                    "--off",
                    o,
                    "--canary",
                    canary.to_str().unwrap(),
                    "--dir",
                    out_dir.to_str().unwrap(),
                    "--port",
                    &port.to_string(),
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let t0 = Instant::now();
            let status = loop {
                if let Some(s) = ch.try_wait().unwrap() {
                    break Some(s);
                }
                if t0.elapsed() > Duration::from_secs(25) {
                    let _ = ch.kill();
                    let _ = ch.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            let mut out = String::new();
            if let Some(so) = ch.stdout.as_mut() {
                let _ = so.read_to_string(&mut out);
            }
            let acc = accepts.load(Ordering::Relaxed) - acc0;
            let res = out
                .lines()
                .find(|l| l.starts_with("RESULT "))
                .unwrap_or("")
                .to_string();
            use std::os::unix::process::ExitStatusExt;
            let died = status.and_then(|s| s.signal());
            let (stage, rest) = match (&status, res.strip_prefix("RESULT ")) {
                (None, _) => ("timeout".to_string(), String::new()),
                (Some(_), Some(r)) => (
                    r.split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_start_matches("stage=")
                        .to_string(),
                    r.to_string(),
                ),
                (Some(_), None) => (
                    format!("process_died_signal_{}", died.unwrap_or(0)),
                    String::new(),
                ),
            };
            let ev: u64 = rest
                .split("file_events=")
                .nth(1)
                .and_then(|x| x.split_whitespace().next())
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            let e = summary.entry(cfg.clone()).or_default();
            if stage == "completed" {
                e.1 += 1;
            } else if stage == "timeout" {
                e.2 += 1;
            } else {
                e.0 += 1;
            }
            if !case.starts_with("selftest") {
                e.3 += ev;
                e.4 += acc;
            }
            println!(
                "CFG {cfg} (barriere spente: [{o}]) CASO {case} => {} connessioni_accettate={acc} tempo_ms={}",
                if rest.is_empty() {
                    format!("stage={stage}")
                } else {
                    rest
                },
                t0.elapsed().as_millis()
            );
        }
    }
    println!(
        "RIEPILOGO (per configurazione: casi fermati, casi completati, scaduti, eventi_file_sul_civetta_o_scritture, connessioni_accettate; selftest esclusi)"
    );
    for (cfg, (s, c, t, ev, a)) in &summary {
        println!(
            "  {cfg}: fermati={s} completati={c} scaduti={t} eventi_file={ev} connessioni={a}"
        );
    }
    let _ = std::fs::remove_dir_all(&work);
}

fn sample_streams(spec: &Spec) -> Vec<StreamDef> {
    tenant_streams(spec, "acme")
}

fn sample_table(sd: &StreamDef, n: usize) -> (SchemaRef, RecordBatch) {
    let schema = sd.schema();
    let cols: Vec<Arc<dyn datafusion::arrow::array::Array>> = sd
        .columns
        .iter()
        .map(|c| -> Arc<dyn datafusion::arrow::array::Array> {
            match c.ty {
                ColType::Int => Arc::new(Int64Array::from_iter_values(0..n as i64)),
                ColType::Instant => Arc::new(
                    datafusion::arrow::array::TimestampMillisecondArray::from_iter_values(
                        (0..n as i64).map(|i| i * 1000),
                    )
                    .with_timezone("UTC"),
                ),
                _ => Arc::new(StringArray::from_iter_values(
                    (0..n).map(|i| format!("v{}", i % 50)),
                )),
            }
        })
        .collect();
    (schema.clone(), RecordBatch::try_new(schema, cols).unwrap())
}

#[derive(Default)]
struct Observed {
    files: BTreeMap<String, u64>,
    sockets: std::collections::BTreeSet<String>,
}

/// Campionatore della tabella dei descrittori: file regolari e prese comparsi dopo il punto di partenza.
fn spawn_sampler(stop: Arc<AtomicBool>) -> std::thread::JoinHandle<Observed> {
    let snap = || -> Vec<(String, String)> {
        std::fs::read_dir("/proc/self/fd")
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let fd = e.file_name().to_string_lossy().to_string();
                        std::fs::read_link(e.path())
                            .ok()
                            .map(|l| (fd, l.display().to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let base: std::collections::BTreeSet<String> = snap().into_iter().map(|(_, l)| l).collect();
    std::thread::spawn(move || {
        let mut o = Observed::default();
        while !stop.load(Ordering::Relaxed) {
            for (fd, link) in snap() {
                if base.contains(&link)
                    || link.starts_with("/proc/")
                    || link.starts_with("anon_inode")
                    || link.starts_with("pipe:")
                {
                    continue;
                }
                if link.starts_with("socket:") {
                    o.sockets.insert(link);
                } else if link.starts_with('/') {
                    let pos = std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}"))
                        .ok()
                        .and_then(|t| {
                            t.lines()
                                .find_map(|l| l.strip_prefix("pos:"))
                                .and_then(|v| v.trim().parse::<u64>().ok())
                        })
                        .unwrap_or(0);
                    let e = o.files.entry(link).or_default();
                    *e = (*e).max(pos);
                }
            }
            std::thread::sleep(Duration::from_micros(500));
        }
        o
    })
}

fn inotify_open(paths: &[&str]) -> i32 {
    // SAFETY: chiamate di libreria con argomenti validi
    unsafe {
        let fd = libc::inotify_init1(libc::IN_NONBLOCK);
        let mask = libc::IN_OPEN
            | libc::IN_ACCESS
            | libc::IN_CREATE
            | libc::IN_MODIFY
            | libc::IN_CLOSE_NOWRITE
            | libc::IN_CLOSE_WRITE;
        for p in paths {
            let c = std::ffi::CString::new(*p).unwrap();
            libc::inotify_add_watch(fd, c.as_ptr(), mask);
        }
        fd
    }
}

fn inotify_count(fd: i32) -> u64 {
    let mut buf = vec![0u8; 8192];
    let mut n = 0u64;
    loop {
        // SAFETY: `buf` e' valido per `len` byte
        let r = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if r <= 0 {
            return n;
        }
        let (mut p, r) = (0usize, r as usize);
        while p + 16 <= r {
            let len = u32::from_le_bytes(buf[p + 12..p + 16].try_into().unwrap()) as usize;
            n += 1;
            p += 16 + len;
        }
    }
}

async fn v2_child() {
    let spec = Spec::load(&arg("--spec").unwrap());
    let case = arg("--case").unwrap();
    let b = off_list(&arg("--off").unwrap_or_default());
    let canary = arg("--canary").unwrap();
    let dir = arg("--dir").unwrap();
    let port: u16 = arg("--port").unwrap().parse().unwrap();
    // barriera 4: limite del sistema sul processo (come il processo per tenant del prodotto)
    if b.memory {
        let l = libc::rlimit {
            rlim_cur: 1 << 30,
            rlim_max: 1 << 30,
        };
        // SAFETY: `l` e' valido
        unsafe { libc::setrlimit(libc::RLIMIT_DATA, &l) };
    }
    let ino = inotify_open(&[&canary, &dir]);
    let stop = Arc::new(AtomicBool::new(false));
    let sampler = spawn_sampler(stop.clone());
    let (stage, detail) = match case.as_str() {
        "selftest_open" => {
            let mut s = String::new();
            let f =
                std::fs::File::open(&canary).and_then(|mut f| f.read_to_string(&mut s).map(|_| f));
            std::thread::sleep(Duration::from_millis(30));
            drop(f);
            ("completed".to_string(), "lettura del civetta".to_string())
        }
        "selftest_connect" => {
            let r = std::net::TcpStream::connect(("127.0.0.1", port))
                .map(|_| ())
                .map_err(|e| e.to_string());
            std::thread::sleep(Duration::from_millis(5));
            ("completed".to_string(), format!("connessione {r:?}"))
        }
        _ => {
            let sql = v2_sql(&case, &canary, &dir, port);
            let streams = sample_streams(&spec);
            match tokio::time::timeout(Duration::from_secs(20), free_run(&sql, &streams, &b)).await
            {
                Ok(x) => x,
                Err(_) => ("timeout".into(), "oltre 20 s".into()),
            }
        }
    };
    stop.store(true, Ordering::Relaxed);
    let o = sampler.join().unwrap();
    let ev = inotify_count(ino);
    let files: Vec<String> = o.files.iter().map(|(k, v)| format!("{k}:{v}")).collect();
    println!(
        "RESULT stage={stage} detail={:?} file_events={ev} file_visti_dal_campionatore={files:?} prese_viste={}",
        detail.chars().take(150).collect::<String>(),
        o.sockets.len()
    );
}

/// Esegue il SQL come testo libero contro il contesto costruito con le barriere date; torna (strato che ha fermato, dettaglio).
async fn free_run(sql: &str, streams: &[StreamDef], b: &Barriers) -> (String, String) {
    let stage_of = |m: &str| -> &'static str {
        if m.contains("RecursionLimitExceeded") || m.contains("recursion limit") {
            "parser_limit"
        } else if m.contains("Resources exhausted") || m.contains("memory") {
            "memory"
        } else if m.contains("nessun archivio di oggetti")
            || m.contains("not found")
            || m.contains("Unable to find factory")
            || m.contains("No suitable object store")
            || m.contains("factory")
        {
            "registration"
        } else {
            "other_error"
        }
    };
    if b.syntax
        && let Err(e) = analyse(&pdef(sql), streams, true)
    {
        return ("syntax".into(), e.to_string());
    }
    let ctx = match context(b, 64 << 20) {
        Ok(c) => c,
        Err(e) => return ("other_error".into(), e.to_string()),
    };
    let sd = &streams[0];
    let (schema, batch) = sample_table(sd, 16384);
    let sh = crate::pipeline::Shared::new(schema);
    sh.set(batch);
    ctx.register_table(&sd.name, sh).unwrap();
    let plan = match ctx.state().create_logical_plan(sql).await {
        Ok(p) => p,
        Err(e) => {
            let m = e.strip_backtrace();
            return (stage_of(&m).into(), m);
        }
    };
    if b.options
        && let Err(e) = SQLOptions::new()
            .with_allow_ddl(false)
            .with_allow_dml(false)
            .with_allow_statements(false)
            .verify_plan(&plan)
    {
        return ("options".into(), e.strip_backtrace());
    }
    match ctx.execute_logical_plan(plan).await {
        Ok(df) => match df.collect().await {
            Ok(r) => ("completed".into(), format!("{} lotti", r.len())),
            Err(e) => {
                let m = e.strip_backtrace();
                (stage_of(&m).into(), m)
            }
        },
        Err(e) => {
            let m = e.strip_backtrace();
            (stage_of(&m).into(), m)
        }
    }
}

/// Occorrenze di `needle` in `hay` (sovrapposizioni escluse, basta per una sentinella casuale).
fn count_hits(hay: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || hay.len() < needle.len() {
        return 0;
    }
    let (mut n, mut i) = (0, 0);
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            n += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    n
}

/// S1, controllo positivo: `probe s1dec --data <cartella del motore> --tenant acme --stream indicators --sentinel S`.
/// Con la chiave principale decifra il checkpoint e un campione di messaggi del log di Iggy (i primi e gli ultimi
/// `--max`, 500 di predefinito) e conta la sentinella nel chiaro: deve trovarla, altrimenti il conteggio zero delle
/// occorrenze nei file non dimostra nulla.
async fn s1dec() {
    let data = arg("--data").expect("--data");
    let tenant = arg("--tenant").unwrap_or("acme".into());
    let stream = arg("--stream").unwrap_or("indicators".into());
    let sent = arg("--sentinel").expect("--sentinel");
    let max: u64 = arg("--max").map(|v| v.parse().unwrap()).unwrap_or(500);
    let master = crate::crypto::Master::load(&format!("{data}/master.key"));
    let key = master.tenant_key(&crate::crypto::tenant_uid(&tenant));
    let needle = sent.as_bytes();
    // checkpoint: file `ckpt.bin` nella cartella del tenant, indirizzo `ckpt/<tenant>`
    let ck = std::fs::read(format!("{data}/payload/{tenant}/ckpt.bin")).unwrap_or_default();
    let plain = key.open(&format!("ckpt/{tenant}"), &ck);
    let ck_hits = plain.as_deref().map_or(0, |p| count_hits(p, needle));
    println!(
        "S1DEC checkpoint byte_cifrati={} decifrato={} sentinella_nel_chiaro={ck_hits}",
        ck.len(),
        plain.is_some()
    );
    let bus = crate::bus::Bus::connect(
        &arg("--iggy").unwrap_or("127.0.0.1:47101".into()),
        &env("IGGY_ROOT_USERNAME"),
        &env("IGGY_ROOT_PASSWORD"),
    )
    .await
    .expect("bus");
    let topic = topic_of(&tenant, &stream);
    let head = bus.head(&topic).await.unwrap_or(0);
    let (mut seen, mut opened, mut hits) = (0u64, 0u64, 0usize);
    for from in [0, head.saturating_sub(max)] {
        for (_, p) in bus.read(&topic, from, max as u32).await.unwrap_or_default() {
            seen += 1;
            if let Some(pl) = key.open(&topic, &p) {
                opened += 1;
                hits += count_hits(&pl, needle);
            }
        }
    }
    println!(
        "S1DEC log testa={head} messaggi_letti={seen} decifrati={opened} sentinella_nel_chiaro={hits}"
    );
    let ok = ck_hits > 0 && hits > 0;
    println!("S1DEC controllo_positivo={}", if ok { "PASS" } else { "FAIL" });
}
