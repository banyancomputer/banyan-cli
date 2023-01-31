use aes_gcm::aes::cipher::crypto_common::rand_core::{OsRng, RngCore};
use aes_gcm::aes::cipher::generic_array::GenericArray;
use aes_gcm::aes::cipher::typenum::U7;
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use serde::{Deserialize, Serialize};

pub fn keygen() -> KeyAndNonce {
    let key = Aes256Gcm::generate_key(&mut OsRng);
    let mut nonce = [0u8; 7];
    OsRng.fill_bytes(&mut nonce);
    let nonce = *GenericArray::from_slice(&nonce);

    KeyAndNonce { key, nonce }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAndNonceToDisk {
    pub(crate) key: Vec<u8>,
    pub(crate) nonce: Vec<u8>,
}
// stream wants some nonce overhead room
pub type MyNonce = GenericArray<u8, U7>;
pub type MyKey = Key<Aes256Gcm>;

// TODO do types better round these parts
// TODO add zeroize here
pub struct KeyAndNonce {
    pub key: MyKey,
    pub nonce: MyNonce,
}

impl KeyAndNonceToDisk {
    // TODO claudia why is this in a box
    pub(crate) fn consume_and_prep_from_disk(self) -> Result<Box<KeyAndNonce>, anyhow::Error> {
        let KeyAndNonceToDisk { key, nonce } = self;
        let key: MyKey = *MyKey::from_slice(&key);
        let nonce: MyNonce = *MyNonce::from_slice(&nonce);
        Ok(Box::new(KeyAndNonce { key, nonce }))
    }
}

impl KeyAndNonce {
    pub(crate) fn consume_and_prep_to_disk(self) -> KeyAndNonceToDisk {
        let KeyAndNonce { key, nonce } = self;
        KeyAndNonceToDisk {
            key: key.as_slice().to_vec(),
            nonce: nonce.as_slice().to_vec(),
        }
    }
}
