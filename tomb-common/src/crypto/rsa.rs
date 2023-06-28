use crate::error::RsaError;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use wnfs::private::{
    ExchangeKey, PrivateKey,
};
use rsa::{
    pkcs8::{
        DecodePrivateKey, EncodePrivateKey,
        LineEnding
    },
    traits::PublicKeyParts,
    BigUint, Oaep, rand_core,
};
use spki::{
    DecodePublicKey, EncodePublicKey,
    SubjectPublicKeyInfoOwned,
};
use sha2::Sha256;

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
        let fingerprint = spki.fingerprint_base64()?;
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
        let key = rsa::RsaPublicKey::read_public_key_pem_file(path)?;
        Ok(Self(key))
    }

    /// Read the public key from DER bytes.
    /// # Arguments
    /// bytes - The DER bytes to read from.
    pub fn from_der(bytes: &[u8]) -> Result<Self> {
        let key = rsa::RsaPublicKey::from_public_key_der(bytes)?;
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
        let key = rsa::RsaPrivateKey::read_pkcs8_pem_file(path)?;
        Ok(Self(key))
    }

    /// Reads the private key from DER bytes.
    ///
    /// # Arguments
    /// bytes - The DER bytes to read from.
    pub fn from_der(bytes: &[u8]) -> Result<Self> {
        let key = rsa::RsaPrivateKey::from_pkcs8_der(bytes)?;
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

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};

    #[async_std::test]
    async fn test_rsa_key_pair() -> Result<()> {
        let priv_key = RsaPrivateKey::new()?;
        let pub_key = priv_key.get_public_key();

        let plaintext = b"Hello, world!";
        let ciphertext = pub_key.encrypt(plaintext).await?;
        let decrypted = priv_key.decrypt(&ciphertext).await?;

        assert_eq!(plaintext, &decrypted[..]);
        Ok(())
    }

    #[async_std::test]
    async fn test_rsa_priv_key_from_pem_file() -> Result<()> {
        let priv_key = RsaPrivateKey::new()?;
        let pub_key = priv_key.get_public_key();
        let plaintext = b"Hello, world!";
        let path = "private_key.pem";

        priv_key.to_pem_file(path)?;
        let priv_key_from_file = RsaPrivateKey::from_pem_file(path)?;

        // Remove the file containing the private key
        std::fs::remove_file(path)?;

        let ciphertext_from_file = pub_key.encrypt(plaintext).await?;
        let decrypted_from_file = priv_key_from_file
            .decrypt(&ciphertext_from_file)
            .await
            ?;

        assert_eq!(plaintext, &decrypted_from_file[..]);
        Ok(())
    }

    #[async_std::test]
    async fn test_rsa_pub_key_from_pem_file() -> Result<()> {
        let priv_key = RsaPrivateKey::new()?;
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
    async fn test_rsa_key_pair_from_base64_strings() -> Result<()> {
        const SPKI_STRING: &str = "MIIBojANBgkqhkiG9w0BAQEFAAOCAY8AMIIBigKCAYEA1SRtDiytKr0oswH8oEam8MyRPrhYGywYF7zitYen/6mkjgzoabdx7lHfQNhdW84030a5jjmwGrZwoJ12E9vgtatEUYBf6+Oa8wThtigk7/mPgdrBLNsQrTusjrlSsG+zFKDL8fnzu3CaJRHUFqGbmpSJG2aRDEOeBWuVIMFfRbmH2mz7XQlDm3hkHkTefvq9HED8mHcUD9bSLFJjT8Ks6m2XguFmYs5VfiyMVQgmsWrCpvMmqjKzJzmLDnjEIU85eU+kM6vme4BLkMh9OtEOUODusfZe20QlOMqPBcmGEgZeDnYPsKGAVTm/W3y7GUkzxFT6YQDnn9PqMB+nAAL8BeptHc1rkc1U/+UlGuvnI4zawUsPqCL8F7tQR9SHcBHGJkhxdJQVlGOehzHsbKG53vwevLO5pxZ9LkDCzrRV7zs45PI4zJkm856PVbXKMv9jZmt4dv4V5PLx+8nGOmwUZy2HGIJHCpgXQiPsV1AlavXohhIKAwwDbMwyd9Q38/vVAgMBAAE=";
        const PKCS8_STRING: &str = "MIIG/QIBADANBgkqhkiG9w0BAQEFAASCBucwggbjAgEAAoIBgQDVJG0OLK0qvSizAfygRqbwzJE+uFgbLBgXvOK1h6f/qaSODOhpt3HuUd9A2F1bzjTfRrmOObAatnCgnXYT2+C1q0RRgF/r45rzBOG2KCTv+Y+B2sEs2xCtO6yOuVKwb7MUoMvx+fO7cJolEdQWoZualIkbZpEMQ54Fa5UgwV9FuYfabPtdCUObeGQeRN5++r0cQPyYdxQP1tIsUmNPwqzqbZeC4WZizlV+LIxVCCaxasKm8yaqMrMnOYsOeMQhTzl5T6Qzq+Z7gEuQyH060Q5Q4O6x9l7bRCU4yo8FyYYSBl4Odg+woYBVOb9bfLsZSTPEVPphAOef0+owH6cAAvwF6m0dzWuRzVT/5SUa6+cjjNrBSw+oIvwXu1BH1IdwEcYmSHF0lBWUY56HMexsobne/B68s7mnFn0uQMLOtFXvOzjk8jjMmSbzno9Vtcoy/2Nma3h2/hXk8vH7ycY6bBRnLYcYgkcKmBdCI+xXUCVq9eiGEgoDDANszDJ31Dfz+9UCAwEAAQKCAYA9tu5czFLXrS27pzeesNZlotXrczUPqRTQysBaD411WYlsGBCzi4pRlyMtg3iEvJBSlgfkRo/XLDwwRWeLGH9YGt8NOj6L7rtO4nr4Y2dOlNQYpV6JvmR1xHGSYdavf6g6sNRcnCMWguQfF6pxYxnLCHcql+gnxOxcZWoosdUEO1Q6ypN9vND2k0Vp/kbuPWvEYozBGLmWXH0+mBxpW9T1jAXyv5EFyvi2L+/yLwoFFQSHkp//Z+63zNGWvyELBAT4rxm1bzRC8cLeAS32nETVtgm3QYcVy+DSKLYLE1fczsMW8+0RF9HBNSD/YxrfTQx+BjQ3MPMOPgwvWanGvy32ib2wk05hBL40DMGk1OPhXCMNZgcw6tEqocfnNjVi67EJNKxsK4bVUnxOL2/fxuVxCGahuyt45ELX7k2m6/qNpMT6lVTpBWV+vcEF6G4ElmhKykU52Z256cVukmsSbztpY83xTn8YX3151iSD6xjiT9Nt2TcKLvuWRmqUrFfrVYECgcEA+PZHdIBNsgOZq/QwH/DXPdAT+PZaGWFHM8NRBtnnY+PsdomQ3EkFmLXqFX85kmbWn9qWDzE8gdlJF0u+O18MqGNHGBBp+zAA/rSBbWv0VNWA5AHQKZCcTTqbUE5EiOJ3Ur0fqRZtpZcqF1+YQLfV2hPpdXQaCjPRm/Ds4s1fis/RIeHPaginUcciagUDkQl9ymK9aG3dT+Dced8sfnu1K1mcoYMqi7AbPkMA/Cat1ZoSyCocMLlKnX4/oDw1DaDPAoHBANsq7ALT1IyJIsmT0OKdy1Z7D6SO70oMb/Y4xwKx2L4EqFe4owZbdoNv8eu1fS1qMPWaF4L0qBsodwqAruoT5gGv4y9iXNXRJJ1Rln1G2JrpXnwHyHdr7KsqHb/cN1b7O/svEy/BLVFRLn3ozbTjWEGXKQ3OxVUCrHx+0OLsH313rGlYupFD5CnIwDCisU23/VZHixUN9dYoHBYZve6Gav09zNL/Ul2vIvVeIrG5RHYomcQ2AFbGVyyGfXwx9aEaGwKBwG13XFPNVlw/WQJSjBZ/PyTeqOl+6H7gVv5bkvUAOs2hGgfE1P0G3n8W/aYWGqpUrWn8Ip7rdz9g2tJza2GPmXEwtcHO9cqMgON9WqtSHExw1AttAKpF+3O5oTDeOSQ272Bh59nhErUMkmVUkw1hx5Xry2rpccmqny+B76aJxsiyN7I+J4Tn6Sn79RXIvpi3I6gpYj7Yj8bfiBHOHzI+EprM/CHIGpzxAgmOTJCSMT0KUdfRLDQARN6a9D7wOiOT4QKBwQDYPq5lT8rc6wY27DDjGBwj9QIHNLynTERAJd8+KmoXepL7EoNP53i00QRatFSRNcCe4+4k2O7w9OkXpMZw0TdVHM1E2IGOum+tBW49p2Ra3L3MFQXXxtXaQJDf2BGGMhcJjHYa3TiwjjAYLVaiDtrqxJHOPOD5Ms0rfRjvfVjIvAaSXuieIeWC0L/IfQ4CB/LfaXGyUXbpWeP0bmu3aEsyGQL6gM8s/nu4q6wBvTHuf7rQHRQSilpC5WP04Xpg/VcCgcA7E0wEH9m/1H0m61G+FVuFvOYvJ7C/rOVVlu50j4hHMIV2vf9Mxxz01FqhfVNATf+yX9iCRuWrGKKlBOzViPGmbyxXlSZ6b434oEyvOy5q86NKJpQDvcAdyA+VLss/nlZToSoaBXli+6me3FqEccrnOz/1ehip1ZRUf2wBRmveGKElPeJibYNBKp+q/fsLv9/0rLilg6bLO5mBiQsCYaGhPUGse0auZxl2Q0LuCEeszgFFXFFGOyzH/x5wwM4divQ=";
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

    #[test]
    fn test_rsa_pub_key_fingerprint() -> Result<()> {
        const SPKI_STRING: &str = "MIIBojANBgkqhkiG9w0BAQEFAAOCAY8AMIIBigKCAYEA1SRtDiytKr0oswH8oEam8MyRPrhYGywYF7zitYen/6mkjgzoabdx7lHfQNhdW84030a5jjmwGrZwoJ12E9vgtatEUYBf6+Oa8wThtigk7/mPgdrBLNsQrTusjrlSsG+zFKDL8fnzu3CaJRHUFqGbmpSJG2aRDEOeBWuVIMFfRbmH2mz7XQlDm3hkHkTefvq9HED8mHcUD9bSLFJjT8Ks6m2XguFmYs5VfiyMVQgmsWrCpvMmqjKzJzmLDnjEIU85eU+kM6vme4BLkMh9OtEOUODusfZe20QlOMqPBcmGEgZeDnYPsKGAVTm/W3y7GUkzxFT6YQDnn9PqMB+nAAL8BeptHc1rkc1U/+UlGuvnI4zawUsPqCL8F7tQR9SHcBHGJkhxdJQVlGOehzHsbKG53vwevLO5pxZ9LkDCzrRV7zs45PI4zJkm856PVbXKMv9jZmt4dv4V5PLx+8nGOmwUZy2HGIJHCpgXQiPsV1AlavXohhIKAwwDbMwyd9Q38/vVAgMBAAE=";
        let spki_bytes = general_purpose::STANDARD.decode(SPKI_STRING)?;
        let pub_key = RsaPublicKey::from_der(&spki_bytes)?;
        let bytes = pub_key.get_fingerprint()?;
        assert_eq!(
            bytes,
            "kuJM7XLygGHer1leDr+oXAbmMRFuNmULAcqkI9IoLn0="
        );
        Ok(())
    }

    #[async_std::test]
    async fn test_rsa_key_pair_from_public_key_modulus() -> Result<()> {
        let priv_key = RsaPrivateKey::new()?;
        let pub_key = priv_key.get_public_key();

        let public_key_modulus = pub_key.get_public_key_modulus()?;
        let pub_key_from_modulus = RsaPublicKey::from_modulus(&public_key_modulus)
            .await
            ?;

        let plaintext = b"Hello, world!";
        let ciphertext = pub_key_from_modulus.encrypt(plaintext).await?;
        let decrypted = priv_key.decrypt(&ciphertext).await?;

        assert_eq!(plaintext, &decrypted[..]);
        Ok(())
    }
}
