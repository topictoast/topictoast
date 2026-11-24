//! Porta del bus con due adattatori (Iggy e Valkey), come enum: nessun dyn, nessuna astrazione in più.
//!
//! Contratto: `append` torna solo dopo la scrittura durevole (Iggy: argomento `persisted`;
//! Valkey: `appendfsync always`); `read` legge da una posizione, `trim` libera le posizioni
//! già applicate dal proiettore (Valkey: `XTRIM`; Iggy dalla fase 4: soglia del consumatore più
//! `delete_segments`, a granularità di segmento).
//! Posizione = u64 uguale per i due: offset Iggy, oppure `ms * 1_000_000 + seq` dell'id di stream Valkey.
//!
//! Con `--features old` l'adattatore Iggy usa il client 0.11.0 (server 0.9.0) al posto della 0.12.0
//! (server 0.10.0): serve al motore «vecchio» della prova di aggiornamento della fase 4.

use bytes::Bytes;
#[cfg(not(feature = "old"))]
use iggy::prelude::*;
#[cfg(feature = "old")]
use iggy_old::prelude::*;
use redis::aio::MultiplexedConnection;

pub type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
const STREAM: &str = "tt";
/// Topic con la generazione del bus (un solo messaggio, offset 0).
const META: &str = "meta";

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

/// Consumatore Iggy che rappresenta il proiettore: la sua posizione salvata è la soglia sotto cui
/// il server può cancellare segmenti (nessun altro consumatore è registrato sul topic).
#[cfg(not(feature = "old"))]
fn projector_consumer() -> Consumer {
    Consumer { kind: ConsumerKind::Consumer, id: Identifier::numeric(1).unwrap() }
}
#[cfg(feature = "old")]
fn projector_consumer() -> Consumer {
    Consumer { kind: ConsumerKind::Consumer, id: Identifier::numeric(1).unwrap() }
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
                    #[cfg(not(feature = "old"))]
                    let _ = c.create_stream(STREAM, &StreamCreateOptions::default()).await;
                    #[cfg(feature = "old")]
                    let _ = c.create_stream(STREAM).await;
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

    /// Crea il topic se manca (una partizione, durabilità piena). `IGGY_SEGMENT_SIZE` (byte, almeno 1 MiB)
    /// fissa la dimensione dei segmenti, `IGGY_PREALLOC=1` ne riserva i byte all'apertura.
    pub async fn ensure_topic(&self, topic: &str) -> Res<()> {
        if let Bus::Iggy(c) = self {
            let s = Identifier::named(STREAM)?;
            if c.get_topic(&s, &Identifier::named(topic)?).await?.is_none() {
                let o = TopicCreateOptions {
                    partitions_count: Some(1),
                    message_expiry: Some(IggyExpiry::NeverExpire),
                    segment_size: std::env::var("IGGY_SEGMENT_SIZE").ok().map(|v| IggyByteSize::from(v.parse::<u64>().unwrap())),
                    // IGGY_DURABILITY=replicated solo come controllo di confronto
                    durability: if std::env::var("IGGY_DURABILITY").as_deref() == Ok("replicated") {
                        Durability::Replicated
                    } else {
                        Durability::Persisted
                    },
                    // soglie di scrittura dei segmenti (messaggi e byte), per la fase 4: predefiniti 1.024 messaggi e 1 MiB
                    messages_required_to_save: std::env::var("IGGY_SAVE_MSGS").ok().map(|v| v.parse().unwrap()),
                    size_of_messages_required_to_save: std::env::var("IGGY_SAVE_BYTES").ok().map(|v| IggyByteSize::from(v.parse::<u64>().unwrap())),
                    #[cfg(not(feature = "old"))]
                    preallocate_segments: std::env::var("IGGY_PREALLOC").ok().map(|v| v == "1"),
                    ..TopicCreateOptions::default()
                };
                c.create_topic(&s, topic, &o).await?;
            }
            // rilettura: la durabilità richiesta deve essere quella applicata
            let t = c.get_topic(&s, &Identifier::named(topic)?).await?.ok_or("topic mancante")?;
            if std::env::var("BUS_PRINT_TOPIC").is_ok() {
                eprintln!("{t:?}");
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
                #[cfg(not(feature = "old"))]
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
                #[cfg(feature = "old")]
                let r = c
                    .poll_messages(&Identifier::named(STREAM)?, &Identifier::named(topic)?, Some(0), &Consumer::default(), &PollingStrategy::offset(from), max, false)
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

    /// Libera le posizioni < `upto` (la prossima da leggere del proiettore).
    /// Valkey: `XTRIM MINID`, al messaggio. Iggy: salva `upto - 1` (l'ultima applicata) come posizione del
    /// consumatore, poi chiede la cancellazione di tutti i segmenti chiusi; il server si ferma al segmento
    /// la cui ultima posizione supera quella del consumatore (il segmento attivo non si cancella mai).
    /// Granularità = un segmento (1 MiB come minimo, 1 GiB di default). L'ordine conta: senza una posizione
    /// salvata non c'è alcuna soglia e il server cancellerebbe ogni segmento chiuso.
    pub async fn trim(&self, topic: &str, upto: u64) -> Res<()> {
        match self {
            Bus::Valkey(c) => {
                let mut c = c.clone();
                redis::cmd("XTRIM").arg(topic).arg("MINID").arg(pos_to_id(upto)).query_async::<()>(&mut c).await?;
            }
            Bus::Iggy(c) => {
                if upto == 0 {
                    return Ok(());
                }
                let (s, t) = (Identifier::named(STREAM)?, Identifier::named(topic)?);
                c.store_consumer_offset(&projector_consumer(), &s, &t, Some(0), upto - 1).await?;
                c.delete_segments(&s, &t, 0, u32::MAX).await?;
            }
        }
        Ok(())
    }

    /// Prossima posizione libera del topic (testa del log). Solo Iggy.
    pub async fn head(&self, topic: &str) -> Res<u64> {
        match self {
            Bus::Iggy(c) => {
                let t = c.get_topic(&Identifier::named(STREAM)?, &Identifier::named(topic)?).await?.ok_or("topic mancante")?;
                let p = t.partitions.first().ok_or("partizione mancante")?;
                // partizione vuota: current_offset vale 0 come con un solo messaggio
                Ok(if p.messages_count == 0 && p.current_offset == 0 { 0 } else { p.current_offset + 1 })
            }
            Bus::Valkey(_) => Err("head: solo Iggy".into()),
        }
    }

    /// Dimensioni e numero di segmenti della partizione (per la prova di conservazione).
    pub async fn segments(&self, topic: &str) -> Res<(u32, u64, u64)> {
        match self {
            Bus::Iggy(c) => {
                let t = c.get_topic(&Identifier::named(STREAM)?, &Identifier::named(topic)?).await?.ok_or("topic mancante")?;
                let p = t.partitions.first().ok_or("partizione mancante")?;
                Ok((p.segments_count, p.messages_count, p.size.as_bytes_u64()))
            }
            Bus::Valkey(_) => Err("segments: solo Iggy".into()),
        }
    }

    /// Posizione salvata dal consumatore del proiettore (ultima applicata), se c'è.
    pub async fn consumer_offset(&self, topic: &str) -> Res<Option<u64>> {
        match self {
            Bus::Iggy(c) => Ok(c
                .get_consumer_offset(&projector_consumer(), &Identifier::named(STREAM)?, &Identifier::named(topic)?, Some(0))
                .await?
                .map(|i| i.stored_offset)),
            Bus::Valkey(_) => Err("consumer_offset: solo Iggy".into()),
        }
    }

    /// Generazione del bus: gli offset ripartono da zero a ogni bus nuovo (aggiornamento senza perdite).
    /// Vive nel topic `meta`, scritto una volta alla creazione del bus vuoto con il valore `default`.
    /// Valkey: non ha una generazione propria, torna `default`.
    pub async fn generation(&self, default: u64) -> Res<u64> {
        if let Bus::Valkey(_) = self {
            return Ok(default);
        }
        self.ensure_topic(META).await?;
        if let Some((_, b)) = self.read(META, 0, 1).await?.into_iter().next() {
            return Ok(u64::from_le_bytes(b[..8].try_into()?));
        }
        self.append(META, &[Bytes::copy_from_slice(&default.to_le_bytes())]).await?;
        Ok(default)
    }
}
