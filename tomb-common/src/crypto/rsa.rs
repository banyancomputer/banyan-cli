use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hex::ToHex;
use rsa::{
    pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding},
    rand_core,
    traits::PublicKeyParts,
    BigUint, Oaep,
};
use sha2::Sha256;
use spki::{DecodePublicKey, EncodePublicKey, SubjectPublicKeyInfoOwned};

use crate::crypto::error::RsaError;

// Re-export the ExchangeKey and PrivateKey traits
pub use wnfs::private::{ExchangeKey, PrivateKey};

//--------------------------------------------------------------------------------------------------
// Constants
//--------------------------------------------------------------------------------------------------

pub const RSA_KEY_SIZE: usize = 3072;
pub const PUBLIC_KEY_EXPONENT: u64 = 65537;

pub type PublicKeyModulus = Vec<u8>;

#[derive(Debug, Clone)]
pub struct RsaPublicKey(rsa::RsaPublicKey);

#[derive(Debug, Clone)]
pub struct RsaPrivateKey(rsa::RsaPrivateKey);

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl RsaPublicKey {
    /// Gets the public key modulus.
    pub fn get_public_key_modulus(&self) -> Result<Vec<u8>> {
        Ok(self.0.n().to_bytes_le())
    }

    /// Gets base64 string of the SHA-256 fingerprint of the public key's SPKI.
    pub fn get_fingerprint(&self) -> Result<String> {
        let document = self.0.to_public_key_der()?;
        let spki = SubjectPublicKeyInfoOwned::try_from(document.as_bytes())?;
        // let fingerprint = spki.finfingerprint_bytesse64()?;
        let fingerprint_bytes = spki.fingerprint_bytes()?;
        let fingerprint = fingerprint_bytes.encode_hex::<String>();
        Ok(fingerprint)
    }

    /// Writes the public key to a SPKI PEM file.
    /// # Arguments
    /// path - The path to the file to write to.
    pub fn to_pem_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        self.0
            .write_public_key_pem_file(path, LineEnding::LF)
            .map_err(|e| anyhow!(RsaError::ExportToPemFileFailed(anyhow!(e))))
    }

    /// Reads the public key from a SPKI PEM file.
    /// # Arguments
    /// path - The path to the file to read from.
    pub fn from_pem_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let key = rsa::RsaPublicKey::read_public_key_pem_file(path)
            .map_err(|e| anyhow!(RsaError::ImportFromPemFileFailed(anyhow!(e))))?;
        Ok(Self(key))
    }

    /// Export the public key to DER bytes.
    pub fn to_der(&self) -> Result<Vec<u8>> {
        let doc = self
            .0
            .to_public_key_der()
            .map_err(|e| anyhow!(RsaError::ExportToDerFileFailed(anyhow!(e))))?;
        let vec = doc.as_bytes().to_vec();
        Ok(vec)
    }

    /// Read the public key from DER bytes.
    /// # Arguments
    /// bytes - The DER bytes to read from.
    pub fn from_der(bytes: &[u8]) -> Result<Self> {
        let key = rsa::RsaPublicKey::from_public_key_der(bytes)
            .map_err(|e| anyhow!(RsaError::ImportFromDerFileFailed(anyhow!(e))))?;
        Ok(Self(key))
    }
}

// #[cfg(test)]
impl RsaPrivateKey {
    /// Constructs a new 2048-bit RSA private key.
    pub fn new() -> Result<Self> {
        Ok(Self(rsa::RsaPrivateKey::new(
            &mut rand_core::OsRng,
            RSA_KEY_SIZE,
        )?))
    }

    /// Writes the private key to a PKCS#8 PEM file.
    ///
    /// # Arguments
    /// path - The path to the file to write to.
    pub fn to_pem_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        self.0
            .write_pkcs8_pem_file(path, LineEnding::LF)
            .map_err(|e| anyhow!(RsaError::ExportToPemFileFailed(anyhow!(e))))
    }

    /// Reads the private key from a PKCS#8 PEM file.
    ///
    /// # Arguments
    /// path - The path to the file to read from.
    pub fn from_pem_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let key = rsa::RsaPrivateKey::read_pkcs8_pem_file(path)
            .map_err(|e| anyhow!(RsaError::ImportFromPemFileFailed(anyhow!(e))))?;
        Ok(Self(key))
    }

    /// Exports the private key to DER bytes.
    pub fn to_der(&self) -> Result<Vec<u8>> {
        let doc = self
            .0
            .to_pkcs8_der()
            .map_err(|e| anyhow!(RsaError::ExportToDerFileFailed(anyhow!(e))))?;
        let vec = doc.as_bytes().to_vec();
        Ok(vec)
    }

    /// Reads the private key from DER bytes.
    ///
    /// # Arguments
    /// bytes - The DER bytes to read from.
    pub fn from_der(bytes: &[u8]) -> Result<Self> {
        let key = rsa::RsaPrivateKey::from_pkcs8_der(bytes)
            .map_err(|e| anyhow!(RsaError::ImportFromDerFileFailed(anyhow!(e))))?;
        Ok(Self(key))
    }

    /// Gets the public key.
    pub fn get_public_key(&self) -> RsaPublicKey {
        RsaPublicKey(self.0.to_public_key())
    }
}

// #[cfg(test)]
#[async_trait(?Send)]
impl ExchangeKey for RsaPublicKey {
    async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let padding = Oaep::new::<Sha256>();
        self.0
            .encrypt(&mut rand_core::OsRng, padding, data)
            .map_err(|e| anyhow!(RsaError::EncryptionFailed(anyhow!(e))))
    }

    async fn from_modulus(modulus: &[u8]) -> Result<Self> {
        let n = BigUint::from_bytes_le(modulus);
        let e = BigUint::from(PUBLIC_KEY_EXPONENT);

        Ok(Self(
            rsa::RsaPublicKey::new(n, e).map_err(|e| RsaError::InvalidPublicKey(anyhow!(e)))?,
        ))
    }
}

// #[cfg(test)]
#[async_trait(?Send)]
impl PrivateKey for RsaPrivateKey {
    async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let padding = Oaep::new::<Sha256>();
        self.0
            .decrypt(padding, ciphertext)
            .map_err(|e| anyhow!(RsaError::DecryptionFailed(anyhow!(e))))
    }
}

impl Default for RsaPrivateKey {
    fn default() -> Self {
        let der = hex::decode("308206fe020100300d06092a864886f70d0101010500048206e8308206e40201000282018100c18c5eb3f7bf0d0b8cc6ffe4937ccc528ea78c50c47925c6d3535cb756156399dfc97828b008f2f468d7b051c342177762dba06fc0f8022cd407a044b570be5b3a7a74a9a95e5addea69d5eeef641d5c3557a3b46a711dd029292e43f144c663f0bde67da908c746906d155e9ea9ff64b0f2733b6f256e8c1e610a14a7384072af821483b0cc9810ffa047760a20e06b6515a4c7c28e398e1feec837c43f1424d927d988f540c325af28ee842628aab9ecbc52d1bad73990437ed4b894a2a40f6f1866b2e3477860a0ed4e42a59c85a7971f834dbfb47ffe9c0381f0cdf24b89975ca77cc57981b4819f787f7295bd010830a873bab03fafd11c7e1f26d0f845eef1187658fee34ddec75aabff8554dc156e03bec67874ac92b381c2e6dec8d776b5ac74cf92f2eb4a35bcf54541c5bc4a8cee4206cd7b9326588bc24684aa65ef6a4ca9ae0c28190544056f84bbbbfbdb8542d6c8bd7a02d268beecf2d5ce30bd2ffe013680be7cc5056a52bc4ad67bfbceb651f206ca17e9bb21cb46bcb0510203010001028201810080b28ffe674c98a60774038fc02a89ca93a5017e6b468b420c1f3055905e249e9ad9e2965b8777d5e1291acb2364fd299b88a2c3ecb27cefc60554229beb5e08577839bedf2a288dcb6398a78a732dbab49593fb5193e9d912a599680034551efb63aab20006204be199474e657e709e49b2cdc0c585445ed38c7f218097bcf305951f82f9baf19acbff8dc505b31ac70eae37a5c4cec1a2a9c5234941ab17fff08db8ee82f60f4d2d8db01c1b2b8b6a99ea17bff1d74f25885bfba2c8e2e4e75d8bf83a310526f206b85d862a27c8e133310544ae63fff872702d5d917708a77d7c864a3bff98809d8e8a889de71f6e86b3974bde9087222887eb44b85f33d8a1471069dc583e4e4dad9375a85e8b6bfeb3b47b8a6cf2e8418d0b7e43563d523930618f9f9822377c2e9195b291654d69233f378693268bbfb19dc1b3a2f5f52b013f201c8baad0309e28cbba714fc613aa13b1c079aabebccd3d11c348f686f104771a9ea8686f7510467e26e40a589d2a39e5e8e91d5afef8ce66a06ef0c90281c100cee3263bb216908aa5be1f67ac790b3c175923f759549f127d4943fb1cdae528c98e939ca483e27714ca1162d5c976402a17b6a450fc96113420a2b1db8e94b729fb53443ddf77116ffb5d3dbddb0cbab6d6472733690a7e69c08f8a5ffd38bc1fea9f3720a952b395e9106f1f2bb57cdfbcf76e0e5efa011cfd460b9a798410009cc43c6109943fd5a06e3fe54277969710aabd9829ff7e281408267364c004f1df3501dab738234c180202375cf7d55a79dce103811a5c197ce13535fa2b2b0281c100ef7e973838bfe145e18ead87cdb6e32e2e302e7f9b16a80ae66f24ad621c5b78d17275ba96bea2300deb023d48b6282630c95b45d817f145e2e9e7e08de37d953385375b15093f9f28ff177596c327dd42719aa44b182bf6475f1409e3d92e96442ddb57bc60e13a31582471ea80166f4ac1d0f4cf6c6b832027463606068ee9c6f59baffa857435837163ecf39a3ed784b8ca1471300104d9a1f9f95b7c3ad46b514d27f9f566acc3aa2593d248a5b9113c76bf9339e14a5cdb6040d23be4730281c001529c74f73f83af0f3e36ef2fc01a5d48fcede8efee459215b0f9394ac6ef7e2243c217d7496c923c54ca65aa5e3e5e4ca6982956c736a26785e9e45f35fb276ca249b6fefa45c59bc4aca4ef68ce1d077c393a3beee8fd43e9d2411d39fe39ddae5f5437e63d3c1eb23dc3a81c5c6daef4835475cd0fa6202c525d52a08242a3ee5ca6d22c0081a3f9019b70f8cad0f0a84f9f24b0e80c436f555a0194dc516bc6748d4d7bac65356055eaf3b5a973f8bf1cb5679354bad002e761b2b5a5bf0281c03dec3c4b341920b501d1f33a46cd3fc623f91f3cad2bd97d2001a2b915c20140a6def263b1304f1d1fac20e31996c7a0c0427fcffa448e84a45c18312e5ea08ce04a547abf60a9cb8c3d10a2bdbd6de43e96c30631c8692d7f5cad00b5a1e4f2c3641bef7e6c8a2f92ac9897bfab28a1d3f17306a94efe296439e3647a805d99427124b5069054f0b530af4687e1dcd7baa050d7a240683309d6609cc1b3c83e3e15425ed0b94bb7e5cb6b75e20c189556488ce791b88870c2bb921290891dd30281c100874d014704d569d547211369394e7c7d772218240d08aff26704df3fd427daf0e1cbbe28e67403ee1756686e63d79a13407406cc6a986caa0c5438273053f45e018177c5b79cb5aa463da72a87efe3b317182bd5cb78a8b6fd6a9d6e59f3170643082b43f94b0aa7c83dd9cdaf00b7a53f4f6158d7f36a8c970ae57e9910e876d62849044447613446c8b09a81dffa7f279fe1fa6e7e8f9d4e6b58eb821a6eb8ac24bac2d33306d8920af82c89a65901fe1cc61aae9e726cd0f93852eda5bb98").unwrap();
        Self::from_der(&der).unwrap()
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};

    #[async_std::test]
    async fn key_pair_xyz() -> Result<()> {
        let priv_key = RsaPrivateKey::default();
        let pub_key = priv_key.get_public_key();

        let plaintext = b"Hello, world!";
        let ciphertext = pub_key.encrypt(plaintext).await?;
        let decrypted = priv_key.decrypt(&ciphertext).await?;

        assert_eq!(plaintext, &decrypted[..]);
        Ok(())
    }

    #[async_std::test]
    async fn private_key_from_pem_file() -> Result<()> {
        let priv_key = RsaPrivateKey::default();
        let pub_key = priv_key.get_public_key();
        let plaintext = b"Hello, world!";
        let path = "private_key.pem";

        priv_key.to_pem_file(path)?;
        let priv_key_from_file = RsaPrivateKey::from_pem_file(path)?;

        // Remove the file containing the private key
        std::fs::remove_file(path)?;

        let ciphertext_from_file = pub_key.encrypt(plaintext).await?;
        let decrypted_from_file = priv_key_from_file.decrypt(&ciphertext_from_file).await?;

        assert_eq!(plaintext, &decrypted_from_file[..]);
        Ok(())
    }

    #[async_std::test]
    async fn public_key_from_pem_file() -> Result<()> {
        let priv_key = RsaPrivateKey::default();
        let pub_key = priv_key.get_public_key();
        let plaintext = b"Hello, world!";
        let path = "public_key.pem";

        pub_key.to_pem_file(path)?;
        let pub_key_from_file = RsaPublicKey::from_pem_file(path)?;

        // Remove the file containing the private key
        std::fs::remove_file(path)?;

        let ciphertext_from_file = pub_key_from_file.encrypt(plaintext).await?;
        let decrypted_from_file = priv_key.decrypt(&ciphertext_from_file).await?;

        assert_eq!(plaintext, &decrypted_from_file[..]);
        Ok(())
    }

    #[async_std::test]
    async fn key_pair_from_base64_strings() -> Result<()> {
        const SPKI_STRING: &str = "MIIBojANBgkqhkiG9w0BAQEFAAOCAY8AMIIBigKCAYEA1SRtDiytKr0oswH8oEam8MyRPrhYGywYF7zitYen/6mkjgzoabdx7lHfQNhdW84030a5jjmwGrZwoJ12E9vgtatEUYBf6+Oa8wThtigk7/mPgdrBLNsQrTusjrlSsG+zFKDL8fnzu3CaJRHUFqGbmpSJG2aRDEOeBWuVIMFfRbmH2mz7XQlDm3hkHkTefvq9HED8mHcUD9bSLFJjT8Ks6m2XguFmYs5VfiyMVQgmsWrCpvMmqjKzJzmLDnjEIU85eU+kM6vme4BLkMh9OtEOUODusfZe20QlOMqPBcmGEgZeDnYPsKGAVTm/W3y7GUkzxFT6YQDnn9PqMB+nAAL8BeptHc1rkc1U/+UlGuvnI4zawUsPqCL8F7tQR9SHcBHGJkhxdJQVlGOehzHsbKG53vwevLO5pxZ9LkDCzrRV7zs45PI4zJkm856PVbXKMv9jZmt4dv4V5PLx+8nGOmwUZy2HGIJHCpgXQiPsV1AlavXohhIKAwwDbMwyd9Q38/vVAgMBAAE=";
        const PKCS8_STRING: &str = "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";
        let plaintext = b"Hello, world!";
        let spki_bytes = general_purpose::STANDARD.decode(SPKI_STRING)?;
        let pkcs8_bytes = general_purpose::STANDARD.decode(PKCS8_STRING)?;

        let pub_key = RsaPublicKey::from_der(&spki_bytes)?;
        let priv_key = RsaPrivateKey::from_der(&pkcs8_bytes)?;
        let pub_key_from_priv_key = priv_key.get_public_key();

        assert!(pub_key.0.n() == pub_key_from_priv_key.0.n());

        let ciphertext = pub_key.encrypt(plaintext).await?;
        let decrypted = priv_key.decrypt(&ciphertext).await?;

        assert_eq!(plaintext, &decrypted[..]);
        Ok(())
    }

    #[async_std::test]
    async fn public_key_to_der() -> Result<()> {
        let priv_key = RsaPrivateKey::default();
        let pub_key = priv_key.get_public_key();
        let plaintext = b"Hello, world!";

        let pub_key_der = pub_key.to_der()?;
        let pub_key_from_der = RsaPublicKey::from_der(&pub_key_der)?;

        let ciphertext = pub_key_from_der.encrypt(plaintext).await?;
        let decrypted = priv_key.decrypt(&ciphertext).await?;

        assert_eq!(plaintext, &decrypted[..]);
        Ok(())
    }

    #[async_std::test]
    async fn private_key_to_der() -> Result<()> {
        let priv_key = RsaPrivateKey::default();
        let pub_key = priv_key.get_public_key();
        let plaintext = b"Hello, world!";

        let priv_key_der = priv_key.to_der()?;
        let priv_key_from_der = RsaPrivateKey::from_der(&priv_key_der)?;

        let ciphertext = pub_key.encrypt(plaintext).await?;
        let decrypted = priv_key_from_der.decrypt(&ciphertext).await?;

        assert_eq!(plaintext, &decrypted[..]);
        Ok(())
    }

    #[test]
    fn public_key_fingerprint() -> Result<()> {
        const SPKI_STRING: &str = "MIIBojANBgkqhkiG9w0BAQEFAAOCAY8AMIIBigKCAYEA1SRtDiytKr0oswH8oEam8MyRPrhYGywYF7zitYen/6mkjgzoabdx7lHfQNhdW84030a5jjmwGrZwoJ12E9vgtatEUYBf6+Oa8wThtigk7/mPgdrBLNsQrTusjrlSsG+zFKDL8fnzu3CaJRHUFqGbmpSJG2aRDEOeBWuVIMFfRbmH2mz7XQlDm3hkHkTefvq9HED8mHcUD9bSLFJjT8Ks6m2XguFmYs5VfiyMVQgmsWrCpvMmqjKzJzmLDnjEIU85eU+kM6vme4BLkMh9OtEOUODusfZe20QlOMqPBcmGEgZeDnYPsKGAVTm/W3y7GUkzxFT6YQDnn9PqMB+nAAL8BeptHc1rkc1U/+UlGuvnI4zawUsPqCL8F7tQR9SHcBHGJkhxdJQVlGOehzHsbKG53vwevLO5pxZ9LkDCzrRV7zs45PI4zJkm856PVbXKMv9jZmt4dv4V5PLx+8nGOmwUZy2HGIJHCpgXQiPsV1AlavXohhIKAwwDbMwyd9Q38/vVAgMBAAE=";
        let spki_bytes = general_purpose::STANDARD.decode(SPKI_STRING)?;
        let pub_key = RsaPublicKey::from_der(&spki_bytes)?;
        let fingerprint = pub_key.get_fingerprint()?;
        assert_eq!(
            fingerprint,
            "92e24ced72f28061deaf595e0ebfa85c06e631116e36650b01caa423d2282e7d"
        );
        Ok(())
    }

    #[async_std::test]
    async fn key_pair_from_public_key_modulus() -> Result<()> {
        let priv_key = RsaPrivateKey::default();
        let pub_key = priv_key.get_public_key();

        let public_key_modulus = pub_key.get_public_key_modulus()?;
        let pub_key_from_modulus = RsaPublicKey::from_modulus(&public_key_modulus).await?;

        let plaintext = b"Hello, world!";
        let ciphertext = pub_key_from_modulus.encrypt(plaintext).await?;
        let decrypted = priv_key.decrypt(&ciphertext).await?;

        assert_eq!(plaintext, &decrypted[..]);
        Ok(())
    }
}
