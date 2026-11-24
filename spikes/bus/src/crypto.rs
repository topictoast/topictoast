//! Cifratura per tenant nel formato di C-12: chiave del tenant, chiave per oggetto derivata con HKDF
//! (`info` = indirizzo completo dell'oggetto), AES-256-GCM. Con una chiave diversa per oggetto il
//! nonce può essere fisso a zero (la coppia chiave/nonce non si ripete mai).
//!
//! ponytail: la chiave principale è un file nella directory dello spike e la chiave del tenant è
//! derivata da essa; il deposito delle chiavi avvolte (FR-34) non si modella, non è nel piano.

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead, aead::Payload};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};

pub type Uid = [u8; 16];
const VER: u8 = 1;

#[derive(Clone)]
pub struct Keys {
    master: [u8; 32],
}

pub fn sha(b: &[u8]) -> [u8; 32] {
    Sha256::digest(b).into()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Keys {
    /// Carica la chiave principale dal file, creandola se manca.
    pub fn load(path: &str) -> Keys {
        let master = match std::fs::read(path) {
            Ok(b) if b.len() == 32 => b.try_into().unwrap(),
            _ => {
                let k = *uuid::Uuid::new_v4().as_bytes();
                let k2 = *uuid::Uuid::new_v4().as_bytes();
                let m: [u8; 32] = [k, k2].concat().try_into().unwrap();
                std::fs::write(path, m).unwrap();
                m
            }
        };
        Keys { master }
    }

    fn derive(&self, tenant: &Uid, info: &[u8]) -> Aes256Gcm {
        let tk = Hkdf::<Sha256>::new(None, &self.master);
        let mut tenant_key = [0u8; 32];
        tk.expand(&[b"tenant".as_slice(), tenant].concat(), &mut tenant_key)
            .unwrap();
        let ok = Hkdf::<Sha256>::new(None, &tenant_key);
        let mut obj_key = [0u8; 32];
        ok.expand(info, &mut obj_key).unwrap();
        Aes256Gcm::new(&obj_key.into())
    }

    /// Busta: versione, id chiave, tenant, id oggetto, testo cifrato. `addr` entra in `info`.
    pub fn seal(&self, tenant: &Uid, addr: &str, id: &Uid, plain: &[u8]) -> Vec<u8> {
        let mut out = vec![VER, 1];
        out.extend_from_slice(tenant);
        out.extend_from_slice(id);
        let c = self.derive(tenant, &[addr.as_bytes(), id].concat());
        let ct = c
            .encrypt(
                &[0u8; 12].into(),
                Payload {
                    msg: plain,
                    aad: &out,
                },
            )
            .unwrap();
        out.extend_from_slice(&ct);
        out
    }

    /// Intestazione in chiaro (tenant, id) e testo in chiaro.
    pub fn open(&self, addr: &str, env: &[u8]) -> (Uid, Uid, Vec<u8>) {
        let tenant: Uid = env[2..18].try_into().unwrap();
        let id: Uid = env[18..34].try_into().unwrap();
        let c = self.derive(&tenant, &[addr.as_bytes(), &id].concat());
        let plain = c
            .decrypt(
                &[0u8; 12].into(),
                Payload {
                    msg: &env[34..],
                    aad: &env[..34],
                },
            )
            .expect("decifratura fallita");
        (tenant, id, plain)
    }
}
