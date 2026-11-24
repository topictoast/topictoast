//! Eventi di stato sul bus e stato in memoria di un'esecuzione (funzione pura degli eventi).
//!
//! Forme del carico: catena di `n` passi, oppure `n` rami in parallelo seguiti da un passo di join.
//! L'identità di un'attività è (esecuzione, passo).

use crate::crypto::Uid;
use bytes::Bytes;

pub type Hash = [u8; 32];
/// Posizione di un payload sigillato nell'archivio: (segmento, scostamento, lunghezza).
pub type Loc = (u64, u64, u32);

#[derive(Clone, Debug)]
pub enum Ev {
    /// Un'esecuzione accettata, con il suo input.
    Accepted {
        exec: Uid,
        tenant: Uid,
        kind: u8,
        n: u32,
        work_ms: u32,
        out_size: u32,
        /// Solo `kind` 3: attesa tra il tentativo fallito e il successivo.
        retry_ms: u32,
        ts: u64,
        input_hash: Hash,
        input: Bytes,
    },
    /// Timer durevole (blocco C): al più uno per attività, la riga più recente vince. `kind`: 0 scadenza del tentativo
    /// (`attempt` è in corso presso un worker), 1 attesa prima di ritentare (`attempt` è il prossimo), 2 attesa del
    /// passo `wait`. Lo cancella l'esito `Done` del passo; un nuovo timer dello stesso passo lo sostituisce.
    Timer {
        exec: Uid,
        tenant: Uid,
        step: u32,
        kind: u8,
        attempt: u32,
        due_us: u64,
        ts: u64,
    },
    /// Decisione del motore: l'attività è pronta; l'input è l'insieme di payload dedotto dallo stato.
    Ready {
        exec: Uid,
        tenant: Uid,
        step: u32,
        last: bool,
        ts: u64,
    },
    /// Esito giudicato dal motore, con il payload di uscita.
    Done {
        exec: Uid,
        tenant: Uid,
        step: u32,
        last: bool,
        ts: u64,
        out_hash: Hash,
        output: Bytes,
    },
}

impl Ev {
    /// Parte dell'id (casuale in UUIDv7) per scegliere la connessione di scrittura.
    pub fn exec_hash(&self) -> u64 {
        let (Ev::Accepted { exec, .. }
        | Ev::Ready { exec, .. }
        | Ev::Done { exec, .. }
        | Ev::Timer { exec, .. }) = self;
        u64::from_le_bytes(exec[8..16].try_into().unwrap())
    }

    pub fn ts(&self) -> u64 {
        match self {
            Ev::Accepted { ts, .. }
            | Ev::Ready { ts, .. }
            | Ev::Done { ts, .. }
            | Ev::Timer { ts, .. } => *ts,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::new();
        let u32w = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
        let u64w = |v: &mut Vec<u8>, x: u64| v.extend_from_slice(&x.to_le_bytes());
        match self {
            Ev::Accepted {
                exec,
                tenant,
                kind,
                n,
                work_ms,
                out_size,
                retry_ms,
                ts,
                input_hash,
                input,
            } => {
                v.push(1);
                v.extend_from_slice(exec);
                v.extend_from_slice(tenant);
                v.push(*kind);
                u32w(&mut v, *n);
                u32w(&mut v, *work_ms);
                u32w(&mut v, *out_size);
                u32w(&mut v, *retry_ms);
                u64w(&mut v, *ts);
                v.extend_from_slice(input_hash);
                v.extend_from_slice(input);
            }
            Ev::Timer {
                exec,
                tenant,
                step,
                kind,
                attempt,
                due_us,
                ts,
            } => {
                v.push(4);
                v.extend_from_slice(exec);
                v.extend_from_slice(tenant);
                u32w(&mut v, *step);
                v.push(*kind);
                u32w(&mut v, *attempt);
                u64w(&mut v, *due_us);
                u64w(&mut v, *ts);
            }
            Ev::Ready {
                exec,
                tenant,
                step,
                last,
                ts,
            } => {
                v.push(2);
                v.extend_from_slice(exec);
                v.extend_from_slice(tenant);
                u32w(&mut v, *step);
                v.push(*last as u8);
                u64w(&mut v, *ts);
            }
            Ev::Done {
                exec,
                tenant,
                step,
                last,
                ts,
                out_hash,
                output,
            } => {
                v.push(3);
                v.extend_from_slice(exec);
                v.extend_from_slice(tenant);
                u32w(&mut v, *step);
                v.push(*last as u8);
                u64w(&mut v, *ts);
                v.extend_from_slice(out_hash);
                v.extend_from_slice(output);
            }
        }
        v
    }

    pub fn decode(b: &[u8]) -> Ev {
        let u32r = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let u64r = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        let exec: Uid = b[1..17].try_into().unwrap();
        let tenant: Uid = b[17..33].try_into().unwrap();
        match b[0] {
            1 => Ev::Accepted {
                exec,
                tenant,
                kind: b[33],
                n: u32r(34),
                work_ms: u32r(38),
                out_size: u32r(42),
                retry_ms: u32r(46),
                ts: u64r(50),
                input_hash: b[58..90].try_into().unwrap(),
                input: Bytes::copy_from_slice(&b[90..]),
            },
            4 => Ev::Timer {
                exec,
                tenant,
                step: u32r(33),
                kind: b[37],
                attempt: u32r(38),
                due_us: u64r(42),
                ts: u64r(50),
            },
            2 => Ev::Ready {
                exec,
                tenant,
                step: u32r(33),
                last: b[37] == 1,
                ts: u64r(38),
            },
            3 => Ev::Done {
                exec,
                tenant,
                step: u32r(33),
                last: b[37] == 1,
                ts: u64r(38),
                out_hash: b[46..78].try_into().unwrap(),
                output: Bytes::copy_from_slice(&b[78..]),
            },
            t => panic!("evento sconosciuto {t}"),
        }
    }
}

/// Stato di un'esecuzione viva. Si ricostruisce dal database (fino alla posizione applicata) e dal
/// rilancio degli eventi successivi, con la stessa funzione.
#[derive(Clone)]
pub struct Exec {
    pub tenant: Uid,
    pub kind: u8,
    pub n: u32,
    pub work_ms: u32,
    pub out_size: u32,
    /// Solo `kind` 3: attesa tra il tentativo fallito e il successivo.
    pub retry_ms: u32,
    pub input_hash: Hash,
    pub issued: Vec<bool>,
    pub done: Vec<Option<Hash>>,
    /// Posizioni nei segmenti dei payload già proiettati (si riempie da `Msg::Located` e dal database).
    pub locs: std::collections::HashMap<Hash, Loc>,
}

impl Exec {
    pub fn new(
        tenant: Uid,
        kind: u8,
        n: u32,
        work_ms: u32,
        out_size: u32,
        retry_ms: u32,
        input_hash: Hash,
    ) -> Exec {
        let steps = if kind != 1 { n } else { n + 1 } as usize;
        Exec {
            tenant,
            kind,
            n,
            work_ms,
            out_size,
            retry_ms,
            input_hash,
            issued: vec![false; steps],
            done: vec![None; steps],
            locs: Default::default(),
        }
    }

    pub fn steps(&self) -> u32 {
        self.issued.len() as u32
    }

    pub fn finished(&self) -> bool {
        self.done.last().unwrap().is_some()
    }

    /// Passi pronti (dipendenze soddisfatte) non ancora emessi né conclusi.
    pub fn ready_unissued(&self) -> Vec<u32> {
        (0..self.steps())
            .filter(|&i| {
                let i = i as usize;
                let deps = if self.kind != 1 {
                    i == 0 || self.done[i - 1].is_some()
                } else if i < self.n as usize {
                    true
                } else {
                    self.done[..self.n as usize].iter().all(|d| d.is_some())
                };
                deps && !self.issued[i] && self.done[i].is_none()
            })
            .collect()
    }

    /// Hash dei payload che compongono l'input del passo, nell'ordine.
    pub fn input_refs(&self, step: u32) -> Vec<Hash> {
        let s = step as usize;
        if self.kind != 1 {
            vec![if s == 0 {
                self.input_hash
            } else {
                self.done[s - 1].unwrap()
            }]
        } else if s < self.n as usize {
            vec![self.input_hash]
        } else {
            self.done[..self.n as usize]
                .iter()
                .map(|d| d.unwrap())
                .collect()
        }
    }
}
