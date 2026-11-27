//! M1: il motore «a riposo» SENZA DataFusion (seconda compilazione minima dello stesso crate).
//!
//! Fa lo stesso lavoro di avvio del sottocomando `engine` a riposo, senza il codice che usa Arrow e DataFusion: runtime
//! di tokio, connessione al bus Iggy, database SQLite degli avvii aperto, socket di ascolto aperto. Poi dorme. Serve a
//! confrontare la memoria del processo con e senza DataFusion nel binario (il linker non porta in questo eseguibile
//! nulla di DataFusion, che questo file non usa). Codice da buttare.
use iggy::prelude::*;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};

#[tokio::main]
async fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1).cloned());
    let data = arg("--data").expect("--data");
    let listen = arg("--listen").expect("--listen");
    let iggy = arg("--iggy").unwrap_or("127.0.0.1:47101".into());
    let user = std::env::var("IGGY_ROOT_USERNAME").expect("IGGY_ROOT_USERNAME");
    let pw = std::env::var("IGGY_ROOT_PASSWORD").expect("IGGY_ROOT_PASSWORD");
    let c = IggyClient::from_connection_string(&format!("iggy://{user}:{pw}@{iggy}")).unwrap();
    c.connect().await.unwrap();
    std::fs::create_dir_all(&data).unwrap();
    let db = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(format!("{data}/starts.db"))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full),
    )
    .await
    .unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS starts(key TEXT PRIMARY KEY, body TEXT NOT NULL) WITHOUT ROWID")
        .execute(&db)
        .await
        .unwrap();
    let _l = tokio::net::TcpListener::bind(&listen).await.unwrap();
    println!("idle_nodf pronto su {listen}");
    tokio::time::sleep(std::time::Duration::from_secs(86_400)).await;
}
