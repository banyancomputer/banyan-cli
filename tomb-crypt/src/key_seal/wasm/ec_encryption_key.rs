use async_trait::async_trait;
use js_sys::{
    ArrayBuffer, Uint8Array
};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CryptoKeyPair, SubtleCrypto
};

use crate::key_seal::KeySealError;
use crate::key_seal::common::*;
use crate::key_seal::wasm::*;

pub struct EcEncryptionKey(pub(crate) CryptoKeyPair);

#[async_trait(?Send)]
impl WrappingPrivateKey for EcEncryptionKey {
    type Error = KeySealError;
    type PublicKey = EcPublicEncryptionKey;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        todo!()
        // // Export the private key
        // let export_promise = internal::subtle_crypto().export_key(
        //     "pkcs8",
        //     &self.0.private_key,
        // );
        // let export_result = JsFuture::from(export_promise).await?;
        // let export_result = export_result.dyn_into::<ArrayBuffer>()?;
        // let export_result = Uint8Array::new(&export_result).to_vec();
        // Ok(export_result)
    }
    
    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        todo!()
        // self.0
        //     .private_key_to_der()
        //     .map_err(KeySealError::export_failed)
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        todo!()
        // self.public_key()?.fingerprint()
    }

    async fn generate() -> Result<Self, KeySealError> {
        Ok(Self(internal::generate_ec_key().await.map_err(| err | {
            KeySealError::subtle_crypto_error(err)
        })?))
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> { todo!()
        // let raw_private =
        //     PKey::private_key_from_pkcs8(pem_bytes).map_err(KeySealError::bad_format)?;

        // Ok(Self(raw_private))
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> { todo!()
        // let raw_private =
        //     PKey::private_key_from_der(der_bytes).expect("parsing a valid der private key");
        // Ok(Self(raw_private))
    }

    async fn public_key(&self) -> Result<EcPublicEncryptionKey, KeySealError> { todo!()
        
    }
}
