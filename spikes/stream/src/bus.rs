//! Adattatore di Iggy (copiato dallo spike del bus, ridotto a quello che serve qui): connessione con utente e
//! password, creazione dei topic (una partizione), append durevole, lettura da una posizione, utente di sola
//! lettura per il processo `stream` (permessi come `ro-a` dello spike del bus).

use bytes::Bytes;
use iggy::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

pub type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const STREAM: &str = "tt";

#[derive(Clone)]
pub struct Bus(pub Arc<IggyClient>);

fn id(n: &str) -> Identifier {
    Identifier::named(n).unwrap()
}

impl Bus {
    pub async fn connect(addr: &str, user: &str, pw: &str) -> Res<Bus> {
        let c = IggyClient::from_connection_string(&format!("iggy://{user}:{pw}@{addr}"))?;
        c.connect().await?;
        Ok(Bus(Arc::new(c)))
    }

    /// Crea lo stream `tt` se manca (ponytail: una sola creazione concorrente supportata).
    pub async fn ensure_stream(&self) -> Res<()> {
        if self.0.get_stream(&id(STREAM)).await?.is_none() {
            let _ = self
                .0
                .create_stream(STREAM, &StreamCreateOptions::default())
                .await;
        }
        Ok(())
    }

    /// Crea il topic se manca: una partizione, scrittura durevole (`persisted`), nessuna scadenza.
    pub async fn ensure_topic(&self, topic: &str) -> Res<()> {
        if self.0.get_topic(&id(STREAM), &id(topic)).await?.is_none() {
            let o = TopicCreateOptions {
                partitions_count: Some(1),
                message_expiry: Some(IggyExpiry::NeverExpire),
                durability: Durability::Persisted,
                ..TopicCreateOptions::default()
            };
            self.0.create_topic(&id(STREAM), topic, &o).await?;
        }
        Ok(())
    }

    /// Scrive un messaggio; torna dopo la scrittura durevole. Un esito incerto si risolve rigiocando (come nel
    /// bus: l'append non si annulla mai).
    pub async fn append(&self, topic: &str, msg: Bytes) -> Res<()> {
        let mut m = vec![IggyMessage::builder().payload(msg).build()?];
        loop {
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            match self
                .0
                .send_messages(&id(STREAM), &id(topic), &Partitioning::partition_id(0), &mut m)
                .await
            {
                Ok(_) => return Ok(()),
                Err(IggyError::OperationUncertain(op)) => {
                    self.0.resolve_operation(op, deadline).await?;
                    return Ok(());
                }
                Err(IggyError::LaneBlocked(op)) => self.0.resolve_operation(op, deadline).await?,
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Legge fino a `max` messaggi con posizione >= `from`.
    pub async fn read(&self, topic: &str, from: u64, max: u32) -> Res<Vec<(u64, Bytes)>> {
        let r = self
            .0
            .poll_messages(
                &id(STREAM),
                &id(topic),
                0,
                &Consumer::default(),
                &PollingStrategy::offset(from),
                PollOptions {
                    count: max,
                    max_bytes: 64 << 20,
                    ..PollOptions::default()
                },
            )
            .await?;
        Ok(r.messages
            .into_iter()
            .map(|m| (m.header.offset, m.payload))
            .collect())
    }

    /// Prossima posizione libera del topic (testa del log).
    pub async fn head(&self, topic: &str) -> Res<u64> {
        let t = self
            .0
            .get_topic(&id(STREAM), &id(topic))
            .await?
            .ok_or("topic mancante")?;
        let p = t.partitions.first().ok_or("partizione mancante")?;
        Ok(if p.messages_count == 0 && p.current_offset == 0 {
            0
        } else {
            p.current_offset + 1
        })
    }

    /// Utente di sola lettura sui soli topic dati (lettura e poll, nessun invio); password casuale a ogni
    /// chiamata, mai stampata. Ritorna la password.
    pub async fn ro_user(&self, name: &str, topics: &[String]) -> Res<String> {
        let c = &self.0;
        let s = c.get_stream(&id(STREAM)).await?.ok_or("stream mancante")?.id as usize;
        let mut tp = BTreeMap::new();
        for t in topics {
            let tid = c.get_topic(&id(STREAM), &id(t)).await?.ok_or("topic mancante")?.id as usize;
            tp.insert(
                tid,
                TopicPermissions {
                    read_topic: true,
                    poll_messages: true,
                    ..TopicPermissions::default()
                },
            );
        }
        let perms = Permissions {
            global: GlobalPermissions::default(),
            streams: Some(BTreeMap::from([(
                s,
                StreamPermissions {
                    topics: Some(tp),
                    ..StreamPermissions::default()
                },
            )])),
        };
        let _ = c.delete_user(&id(name)).await;
        let pw = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        c.create_user(name, &pw, UserStatus::Active, Some(perms), &UserCreateOptions::default())
            .await?;
        Ok(pw)
    }
}
