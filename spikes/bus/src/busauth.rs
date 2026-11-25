//! Prova della credenziale di sola lettura del bus (blocco E, E1) contro il server vero: `busauth case=<caso>`.
//! Casi: `control` (root: le stesse operazioni riescono), `ro-a` e `ro-a-s` (utente con lettura del solo topic del tenant A, senza
//! e con `read_stream`), `ro-sent` (lettura di tutto lo stream, per il sentinella), `setup-sent` (crea `ro-sent` e scrive la sua
//! password in `out/ro-sent.pw`, per far girare il vero sentinella con quell'utente) e `offsets` (posizione del gruppo `sentinel`).
//! Le password degli utenti di prova si generano a ogni corsa e non si stampano mai; gli utenti si cancellano a fine prova.
//! Ogni controllo stampa `CHECK <nome> atteso=<riesce|rifiutato> ottenuto=<riesce|errore> <PASS|FAIL> <dettaglio o codice e messaggio>`.

use crate::bus::{Bus, STREAM};
use crate::engine::{TENANTS, topic_of};
use iggy::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;

const URL: &str = "127.0.0.1:47001";

fn secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

async fn login(user: &str, pw: &str) -> Arc<IggyClient> {
    let c = IggyClient::from_connection_string(&format!("iggy://{user}:{pw}@{URL}"))
        .expect("connessione");
    c.connect().await.expect("accesso rifiutato");
    Arc::new(c)
}

fn id(n: &str) -> Identifier {
    Identifier::named(n).unwrap()
}

/// Un controllo: `want_ok` dice se l'operazione deve riuscire. Il dettaglio di un errore e' il codice e il messaggio esatti.
fn check<T: std::fmt::Debug>(
    name: &str,
    want_ok: bool,
    r: Result<T, IggyError>,
    detail: impl Fn(&T) -> String,
) {
    let (got_ok, d) = match &r {
        Ok(v) => (true, detail(v)),
        Err(e) => (false, format!("codice={} {e}", e.as_code())),
    };
    println!(
        "CHECK {name} atteso={} ottenuto={} {} {d}",
        if want_ok { "riesce" } else { "rifiutato" },
        if got_ok { "riesce" } else { "errore" },
        if want_ok == got_ok { "PASS" } else { "FAIL" }
    );
}

async fn poll(c: &IggyClient, topic: &str) -> Result<usize, IggyError> {
    Ok(c.poll_messages(
        &id(STREAM),
        &id(topic),
        0,
        &Consumer::default(),
        &PollingStrategy::offset(0),
        PollOptions {
            count: 10,
            ..PollOptions::default()
        },
    )
    .await?
    .messages
    .len())
}

async fn send(c: &IggyClient, topic: &str) -> Result<(), IggyError> {
    let mut m = vec![IggyMessage::builder().payload("x".into()).build()?];
    c.send_messages(
        &id(STREAM),
        &id(topic),
        &Partitioning::partition_id(0),
        &mut m,
    )
    .await
    .map(|_| ())
}

/// Lettura da gruppo con salvataggio della posizione (come fa il sentinella).
async fn poll_group(c: &IggyClient, topic: &str) -> Result<usize, IggyError> {
    Ok(c.poll_messages(
        &id(STREAM),
        &id(topic),
        0,
        &Consumer::group(id("sentinel")),
        &PollingStrategy::next(),
        PollOptions {
            count: 10,
            commit: CommitPolicy::OnRead,
            ..PollOptions::default()
        },
    )
    .await?
    .messages
    .len())
}

async fn offset(c: &IggyClient, topic: &str) -> Result<Option<u64>, IggyError> {
    Ok(c.get_consumer_offset(
        &Consumer::group(id("sentinel")),
        &id(STREAM),
        &id(topic),
        Some(0),
    )
    .await?
    .map(|i| i.stored_offset))
}

/// Operazioni comuni di sola scrittura che un utente di lettura non deve poter fare.
async fn writes(c: &IggyClient, who: &str, send_to: &str, want_ok: bool) {
    check(
        &format!("{who}: invio a {send_to}"),
        want_ok,
        send(c, send_to).await,
        |_| "ok".into(),
    );
    check(
        &format!("{who}: creare un topic"),
        want_ok,
        c.create_topic(
            &id(STREAM),
            "intruder",
            &TopicCreateOptions {
                partitions_count: Some(1),
                ..TopicCreateOptions::default()
            },
        )
        .await
        .map(|_| ()),
        |_| "ok".into(),
    );
    check(
        &format!("{who}: cancellare un topic"),
        want_ok,
        c.delete_topic(&id(STREAM), &id("scratch")).await,
        |_| "ok".into(),
    );
}

async fn user(r: &IggyClient, name: &str, perms: Option<Permissions>) -> String {
    let _ = r.delete_user(&id(name)).await;
    let pw = secret();
    r.create_user(
        name,
        &pw,
        UserStatus::Active,
        perms,
        &UserCreateOptions::default(),
    )
    .await
    .expect("creazione utente");
    pw
}

pub async fn run(case: &str) {
    let root = match Bus::connect("iggy").await.unwrap() {
        Bus::Iggy(c) => c,
        Bus::Valkey(_) => unreachable!(),
    };
    let bus = Bus::connect("iggy").await.unwrap();
    let (a, b) = (topic_of(&TENANTS[0]), topic_of(&TENANTS[1]));
    println!(
        "busauth case={case} server_iggy={}",
        root.get_stats()
            .await
            .map(|s| s.iggy_server_version)
            .unwrap_or_default()
    );
    for t in [&a, &b, &"scratch".to_string()] {
        bus.ensure_topic(t).await.unwrap();
    }
    if case == "offsets" {
        check(
            "offset del gruppo sentinel su state-A (root)",
            true,
            offset(&root, &a).await,
            |o| format!("{o:?}"),
        );
        return;
    }
    for t in [&a, &b] {
        for _ in 0..3 {
            send(&root, t).await.unwrap();
        }
    }
    let s = root
        .get_stream(&id(STREAM))
        .await
        .unwrap()
        .expect("stream")
        .id as usize;
    let ta = root
        .get_topic(&id(STREAM), &id(&a))
        .await
        .unwrap()
        .expect("topic A")
        .id as usize;
    let stream_read = |topics: Option<BTreeMap<usize, TopicPermissions>>,
                       read_stream: bool,
                       all: bool| Permissions {
        global: GlobalPermissions::default(),
        streams: Some(BTreeMap::from([(
            s,
            StreamPermissions {
                read_stream,
                read_topics: all,
                poll_messages: all,
                topics,
                ..StreamPermissions::default()
            },
        )])),
    };
    match case {
        "control" => {
            check("root: lettura di A", true, poll(&root, &a).await, |n| {
                format!("{n} messaggi")
            });
            check("root: lettura di B", true, poll(&root, &b).await, |n| {
                format!("{n} messaggi")
            });
            writes(&root, "root", &a, true).await;
            check(
                "root: elenco dei topic",
                true,
                root.get_topics(&id(STREAM)).await,
                |t| format!("{} topic", t.len()),
            );
            check(
                "root: creare il gruppo sentinel su A",
                true,
                root.create_consumer_group(&id(STREAM), &id(&a), "sentinel")
                    .await
                    .map(|_| ()),
                |_| "ok".into(),
            );
            check(
                "root: unirsi al gruppo",
                true,
                root.join_consumer_group(&id(STREAM), &id(&a), &id("sentinel"))
                    .await,
                |_| "ok".into(),
            );
            check(
                "root: lettura da gruppo con posizione salvata",
                true,
                poll_group(&root, &a).await,
                |n| format!("{n} messaggi"),
            );
            check(
                "root: posizione salvata del gruppo",
                true,
                offset(&root, &a).await,
                |o| format!("{o:?}"),
            );
        }
        "ro-a" | "ro-a-s" => {
            let tp = TopicPermissions {
                read_topic: true,
                poll_messages: true,
                ..TopicPermissions::default()
            };
            let perms = stream_read(Some(BTreeMap::from([(ta, tp)])), case == "ro-a-s", false);
            let pw = user(&root, "ro-a", Some(perms)).await;
            let c = login("ro-a", &pw).await;
            check("ro-a: lettura di A", true, poll(&c, &a).await, |n| {
                format!("{n} messaggi")
            });
            check("ro-a: lettura di B", false, poll(&c, &b).await, |n| {
                format!("{n} messaggi")
            });
            writes(&c, "ro-a", &a, false).await;
            check(
                "ro-a: elenco dei topic (informativo, atteso rifiutato)",
                false,
                c.get_topics(&id(STREAM)).await,
                |t| format!("{} topic", t.len()),
            );
            let _ = root.delete_user(&id("ro-a")).await;
        }
        "ro-sent" | "setup-sent" => {
            let perms = stream_read(None, true, true);
            let pw = user(&root, "ro-sent", Some(perms)).await;
            if case == "setup-sent" {
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                let mut f = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open("out/ro-sent.pw")
                    .unwrap();
                f.write_all(pw.as_bytes()).unwrap();
                println!(
                    "utente ro-sent creato (lettura e poll di tutto lo stream `{STREAM}`, nessun invio)"
                );
                return;
            }
            let c = login("ro-sent", &pw).await;
            check(
                "ro-sent: elenco dei topic",
                true,
                c.get_topics(&id(STREAM)).await,
                |t| {
                    format!(
                        "{} topic: {}",
                        t.len(),
                        t.iter()
                            .map(|x| x.name.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                },
            );
            // topic creato da root DOPO l'accesso dell'utente
            let n = topic_of(&uuid::Uuid::new_v4().into_bytes());
            bus.ensure_topic(&n).await.unwrap();
            for _ in 0..3 {
                send(&root, &n).await.unwrap();
            }
            check(
                "ro-sent: elenco dei topic con il topic nato dopo",
                true,
                c.get_topics(&id(STREAM)).await,
                |t| {
                    format!(
                        "{} topic, nuovo visto: {}",
                        t.len(),
                        t.iter().any(|x| x.name == n)
                    )
                },
            );
            check(
                "ro-sent: creare il gruppo sentinel sul topic nuovo",
                true,
                c.create_consumer_group(&id(STREAM), &id(&n), "sentinel")
                    .await
                    .map(|_| ()),
                |_| "ok".into(),
            );
            check(
                "ro-sent: unirsi al gruppo",
                true,
                c.join_consumer_group(&id(STREAM), &id(&n), &id("sentinel"))
                    .await,
                |_| "ok".into(),
            );
            check(
                "ro-sent: lettura da gruppo con posizione salvata",
                true,
                poll_group(&c, &n).await,
                |k| format!("{k} messaggi"),
            );
            check(
                "ro-sent: posizione salvata del gruppo",
                true,
                offset(&c, &n).await,
                |o| format!("{o:?}"),
            );
            check(
                "ro-sent: lettura semplice di A",
                true,
                poll(&c, &a).await,
                |k| format!("{k} messaggi"),
            );
            writes(&c, "ro-sent", &a, false).await;
            // gruppo creato da root su un altro topic: basta la lettura per unirsi e leggere?
            root.create_consumer_group(&id(STREAM), &id(&b), "sentinel")
                .await
                .unwrap();
            check(
                "ro-sent: unirsi a un gruppo creato da root",
                true,
                c.join_consumer_group(&id(STREAM), &id(&b), &id("sentinel"))
                    .await,
                |_| "ok".into(),
            );
            check(
                "ro-sent: lettura da quel gruppo",
                true,
                poll_group(&c, &b).await,
                |k| format!("{k} messaggi"),
            );
            let _ = root.delete_user(&id("ro-sent")).await;
        }
        _ => println!("casi: control ro-a ro-a-s ro-sent setup-sent offsets"),
    }
}
