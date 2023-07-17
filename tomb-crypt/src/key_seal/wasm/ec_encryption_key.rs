use async_trait::async_trait;
use web_sys::CryptoKey;

use crate::key_seal::KeySealError;
use crate::key_seal::common::*;
use crate::key_seal::wasm::*;

pub struct EcEncryptionKey {
    pub(crate) private_key: CryptoKey,
    pub(crate) public_key: Option<CryptoKey>
}

#[async_trait(?Send)]
impl WrappingPrivateKey for EcEncryptionKey {
    type Error = KeySealError;
    type PublicKey = EcPublicEncryptionKey;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_key_pem("pkcs8", &self.private_key).await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))
    }
    
    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_key_der("pkcs8", &self.private_key).await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        self.public_key()?.fingerprint().await
    }

    async fn generate() -> Result<Self, KeySealError> {
        // Ok(Self(internal::generate_ec_encryption_key_pair().await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))?))
        let key_pair = internal::generate_ec_encryption_key_pair().await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))?;
        let private_key = internal::private_key(&key_pair);
        let public_key = internal::public_key(&key_pair);
        Ok(Self { private_key, public_key: Some(public_key) })
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        let private_key = internal::import_ec_key_pem("pkcs8", pem_bytes).await.map_err(|err| KeySealError::bad_format(err.into()))?;
        Ok(Self { private_key, public_key: None })
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let private_key = internal::import_ec_key_der("pkcs8", der_bytes).await.map_err(|err| KeySealError::bad_format(err.into()))?;
        Ok(Self { private_key, public_key: None })
    }

    fn public_key(&self) -> Result<EcPublicEncryptionKey, KeySealError> {
        let public_key = self.public_key.as_ref().ok_or(KeySealError::public_key_unavailable())?; 
        Ok(EcPublicEncryptionKey(public_key.clone()))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn generate_export_import() -> Result<(), KeySealError> {
        let key = EcEncryptionKey::generate().await?;
        let public_key = key.public_key()?;
        let finger_print = public_key.fingerprint().await?;
        assert_eq!(finger_print.len(), FINGERPRINT_SIZE);

        // dirty comparisons but works for now
        let raw_key_bytes = key.export_bytes().await?;
        let imported_key = EcEncryptionKey::import_bytes(&raw_key_bytes).await?;
        let reexported_key_bytes = imported_key.export_bytes().await?;
        assert_eq!(raw_key_bytes, reexported_key_bytes);

        let raw_public_key_bytes = public_key.export_bytes().await?;
        let imported_public_key = EcPublicEncryptionKey::import_bytes(&raw_public_key_bytes).await?;
        let reexported_public_key_bytes = imported_public_key.export_bytes().await?;
        assert_eq!(raw_public_key_bytes, reexported_public_key_bytes);

        // TODO: Uncomment when pem import / export is implemented
        // let raw_key_pem = key.export()?;
        // let imported_key = EcEncryptionKey::import(&raw_key_pem)?;
        // let reexported_key_pem = imported_key.export()?;
        // assert_eq!(raw_key_pem, reexported_key_pem);

        // let raw_public_key_pem = public_key.export()?;
        // let imported_public_key = EcPublicEncryptionKey::import(&raw_public_key_pem)?;
        // let reexported_public_key_pem = imported_public_key.export()?;
        // assert_eq!(raw_public_key_pem, reexported_public_key_pem);

        Ok(())
    }
}
