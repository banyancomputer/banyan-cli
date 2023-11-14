use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};
use tomb_crypt::prelude::{EcEncryptionKey, EcSignatureKey, PrivateKey};

use super::ConfigurationError;

/// Generate a new Ecdsa key to use for authentication
/// Writes the key to the config path
pub async fn new_api_key(path: &PathBuf) -> Result<EcSignatureKey, ConfigurationError> {
    if path.exists() {
        load_api_key(path).await?;
    }
    let key = EcSignatureKey::generate().await?;
    let pem_bytes = key.export().await?;
    let mut f = File::create(path)?;
    f.write_all(&pem_bytes)?;
    Ok(key)
}

/// Read the API key from disk
pub async fn load_api_key(path: &PathBuf) -> Result<EcSignatureKey, ConfigurationError> {
    let mut reader = File::open(path)?;
    let mut pem_bytes = Vec::new();
    reader.read_to_end(&mut pem_bytes)?;
    let key = EcSignatureKey::import(&pem_bytes).await?;
    Ok(key)
}

/// Save the API key to disk
pub async fn save_api_key(path: &PathBuf, key: EcSignatureKey) -> Result<(), ConfigurationError> {
    if let Ok(mut writer) = File::create(path) {
        // Write the PEM bytes
        writer.write_all(&key.export().await?)?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("Cannot write API key at this path"))
    }
}

/// Generate a new Ecdh key to use for key wrapping
/// Writes the key to the config path
pub async fn new_wrapping_key(path: &PathBuf) -> Result<EcEncryptionKey, ConfigurationError> {
    if path.exists() {
        wrapping_key(path).await?;
    }
    let key = EcEncryptionKey::generate().await?;
    let pem_bytes = key.export().await?;
    let mut f = File::create(path)?;
    f.write_all(&pem_bytes)?;
    Ok(key)
}

/// Read the Wrapping key from disk
pub async fn wrapping_key(path: &PathBuf) -> Result<EcEncryptionKey, ConfigurationError> {
    let mut reader = File::open(path)?;
    let mut pem_bytes = Vec::new();
    reader.read_to_end(&mut pem_bytes)?;
    let key = EcEncryptionKey::import(&pem_bytes).await?;
    Ok(key)
}
