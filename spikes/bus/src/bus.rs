//! Porta del bus con due adattatori (Iggy e Valkey), come enum: nessun dyn, nessuna astrazione in più.
//!
//! Contratto: `append` torna solo dopo la scrittura durevole (Iggy: argomento `persisted`;
//! Valkey: `appendfsync always`); `read` legge da una posizione, `trim` libera le posizioni
//! già applicate dal proiettore (solo Valkey: Iggy non sa tagliare per offset).
//! Posizione = u64 uguale per i due: offset Iggy, oppure `ms * 1_000_000 + seq` dell'id di stream Valkey.

use bytes::Bytes;
use iggy::prelude::*;
use redis::aio::MultiplexedConnection;

pub type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
const STREAM: &str = "tt";

#[derive(Clone)]
pub enum Bus {
    Iggy(std::sync::Arc<IggyClient>),
    Valkey(MultiplexedConnection),
}

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| panic!("manca la variabile {k} (carica .env)"))
}

fn pos_to_id(pos: u64) -> String {
    format!("{}-{}", pos / 1_000_000, pos % 1_000_000)
}
fn id_to_pos(id: &str) -> u64 {
    let (ms, seq) = id.split_once('-').unwrap();
    ms.parse::<u64>().unwrap() * 1_000_000 + seq.parse::<u64>().unwrap()
}

impl Bus {
    pub async fn connect(kind: &str) -> Res<Bus> {
        match kind {
            "iggy" => {
                let url = format!(
                    "iggy://{}:{}@127.0.0.1:47001",
                    env("IGGY_ROOT_USERNAME"),
                    env("IGGY_ROOT_PASSWORD")
                );
                let c = IggyClient::from_connection_string(&url)?;
                c.connect().await?;
                let c = std::sync::Arc::new(c);
                // ponytail: crea lo stream se manca; una sola creazione concorrente supportata.
                let id = Identifier::named(STREAM)?;
                if c.get_stream(&id).await?.is_none() {
                    let _ = c.create_stream(STREAM, &StreamCreateOptions::default()).await;
                }
                Ok(Bus::Iggy(c))
            }
            "valkey" => {
                let url = format!("redis://:{}@127.0.0.1:47002", env("VALKEY_PASSWORD"));
                let cfg = redis::AsyncConnectionConfig::new().set_response_timeout(Some(std::time::Duration::from_secs(60)));
                let c = redis::Client::open(url)?.get_multiplexed_async_connection_with_config(&cfg).await?;
                Ok(Bus::Valkey(c))
            }
            _ => Err("bus: iggy|valkey".into()),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Bus::Iggy(_) => "iggy",
            Bus::Valkey(_) => "valkey",
        }
    }

    /// Crea il topic se manca (una partizione, durabilità piena).
    pub async fn ensure_topic(&self, topic: &str) -> Res<()> {
        if let Bus::Iggy(c) = self {
            let s = Identifier::named(STREAM)?;
            if c.get_topic(&s, &Identifier::named(topic)?).await?.is_none() {
                let o = TopicCreateOptions {
                    partitions_count: Some(1),
                    message_expiry: Some(IggyExpiry::NeverExpire),
                    durability: Durability::Persisted,
                    ..TopicCreateOptions::default()
                };
                let _ = c.create_topic(&s, topic, &o).await;
            }
        }
        Ok(())
    }

    /// Scrive un gruppo di messaggi; torna dopo la scrittura durevole.
    pub async fn append(&self, topic: &str, msgs: &[Bytes]) -> Res<()> {
        match self {
            Bus::Iggy(c) => {
                let mut m: Vec<IggyMessage> = msgs
                    .iter()
                    .map(|b| IggyMessage::builder().payload(b.clone()).build())
                    .collect::<Result<_, _>>()?;
                c.send_messages(
                    &Identifier::named(STREAM)?,
                    &Identifier::named(topic)?,
                    &Partitioning::partition_id(0),
                    &mut m,
                )
                .await?;
            }
            Bus::Valkey(c) => {
                let mut p = redis::pipe();
                for b in msgs {
                    p.cmd("XADD").arg(topic).arg("*").arg("d").arg(b.as_ref()).ignore();
                }
                let mut c = c.clone();
                p.query_async::<()>(&mut c).await?;
            }
        }
        Ok(())
    }

    /// Legge fino a `max` messaggi con posizione >= `from`.
    pub async fn read(&self, topic: &str, from: u64, max: u32) -> Res<Vec<(u64, Bytes)>> {
        match self {
            Bus::Iggy(c) => {
                let r = c
                    .poll_messages(
                        &Identifier::named(STREAM)?,
                        &Identifier::named(topic)?,
                        0,
                        &Consumer::default(),
                        &PollingStrategy::offset(from),
                        PollOptions { count: max, max_bytes: 64 << 20, ..PollOptions::default() },
                    )
                    .await?;
                Ok(r.messages.into_iter().map(|m| (m.header.offset, m.payload)).collect())
            }
            Bus::Valkey(c) => {
                let mut c = c.clone();
                // XRANGE con inizio incluso
                let r: Vec<(String, Vec<(String, Vec<u8>)>)> = redis::cmd("XRANGE")
                    .arg(topic)
                    .arg(pos_to_id(from))
                    .arg("+")
                    .arg("COUNT")
                    .arg(max)
                    .query_async(&mut c)
                    .await?;
                Ok(r.into_iter()
                    .map(|(id, f)| (id_to_pos(&id), Bytes::from(f.into_iter().next().unwrap().1)))
                    .collect())
            }
        }
    }

    /// Elimina un topic (solo per le corse di prova).
    pub async fn drop_topic(&self, topic: &str) -> Res<()> {
        match self {
            Bus::Iggy(c) => {
                let _ = c.delete_topic(&Identifier::named(STREAM)?, &Identifier::named(topic)?).await;
            }
            Bus::Valkey(c) => {
                let mut c = c.clone();
                redis::cmd("DEL").arg(topic).query_async::<()>(&mut c).await?;
            }
        }
        Ok(())
    }

    /// Libera le posizioni < `upto` (Valkey; Iggy: nessuna operazione, vedi PIANO fase 4).
    pub async fn trim(&self, topic: &str, upto: u64) -> Res<()> {
        if let Bus::Valkey(c) = self {
            let mut c = c.clone();
            redis::cmd("XTRIM")
                .arg(topic)
                .arg("MINID")
                .arg(pos_to_id(upto))
                .query_async::<()>(&mut c)
                .await?;
        }
        Ok(())
    }
}
