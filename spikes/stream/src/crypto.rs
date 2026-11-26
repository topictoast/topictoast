//! Cifratura per tenant nel formato di C-12 (copiata dallo spike del bus): chiave del tenant, chiave per
//! oggetto derivata con HKDF (`info` = indirizzo dell'oggetto + id), AES-256-GCM, nonce fisso a zero perche'
//! la chiave cambia a ogni oggetto.
//!
//! Differenza dal bus: il motore tiene la chiave principale (`Master`), il processo per tenant riceve solo
//! la chiave del suo tenant (`TenantKey`), mai la principale.
//!
//! ponytail: la chiave principale e' un file; il deposito delle chiavi avvolte (FR-34) non si modella.

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead, aead::Payload};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};

pub type Uid = [u8; 16];
const VER: u8 = 1;

pub fn sha(b: &[u8]) -> [u8; 32] {
    Sha256::digest(b).into()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// Identificatore a 16 byte di un tenant, derivato dal nome.
pub fn tenant_uid(name: &str) -> Uid {
    sha(name.as_bytes())[..16].try_into().unwrap()
}

/// Chiave principale (solo nel motore).
pub struct Master([u8; 32]);

impl Master {
    /// Carica la chiave principale dal file, creandola se manca.
    pub fn load(path: &str) -> Master {
        let m = match std::fs::read(path) {
            Ok(b) if b.len() == 32 => b.try_into().unwrap(),
            _ => {
                let m: [u8; 32] = [
                    *uuid::Uuid::new_v4().as_bytes(),
                    *uuid::Uuid::new_v4().as_bytes(),
                ]
                .concat()
                .try_into()
                .unwrap();
                std::fs::write(path, m).unwrap();
                m
            }
        };
        Master(m)
    }

    pub fn tenant_key(&self, tenant: &Uid) -> TenantKey {
        let mut k = [0u8; 32];
        Hkdf::<Sha256>::new(None, &self.0)
            .expand(&[b"tenant".as_slice(), tenant].concat(), &mut k)
            .unwrap();
        TenantKey(k)
    }
}

/// Chiave di un tenant: e' tutto quello che il processo per tenant conosce.
#[derive(Clone)]
pub struct TenantKey(pub [u8; 32]);

impl TenantKey {
    fn cipher(&self, info: &[u8]) -> Aes256Gcm {
        let mut k = [0u8; 32];
        Hkdf::<Sha256>::new(None, &self.0)
            .expand(info, &mut k)
            .unwrap();
        Aes256Gcm::new(&k.into())
    }

    /// Busta: versione, id chiave, tenant, id oggetto, testo cifrato. `addr` entra in `info`.
    pub fn seal(&self, tenant: &Uid, addr: &str, id: &Uid, plain: &[u8]) -> Vec<u8> {
        let mut out = vec![VER, 1];
        out.extend_from_slice(tenant);
        out.extend_from_slice(id);
        let ct = self
            .cipher(&[addr.as_bytes(), id].concat())
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

    /// Testo in chiaro, o `None` se la busta e' corrotta o di un altro tenant/oggetto.
    pub fn open(&self, addr: &str, env: &[u8]) -> Option<Vec<u8>> {
        if env.len() < 34 {
            return None;
        }
        let id = &env[18..34];
        self.cipher(&[addr.as_bytes(), id].concat())
            .decrypt(
                &[0u8; 12].into(),
                Payload {
                    msg: &env[34..],
                    aad: &env[..34],
                },
            )
            .ok()
    }
}

/// Id oggetto casuale.
pub fn new_id() -> Uid {
    *uuid::Uuid::new_v4().as_bytes()
}
