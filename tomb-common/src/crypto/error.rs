use thiserror::Error;

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// RSA related errors
#[derive(Debug, Error)]
pub enum RsaError {
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(anyhow::Error),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(anyhow::Error),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(anyhow::Error),

    #[error("Import from der file failed: {0}")]
    ImportFromDerFileFailed(anyhow::Error),

    #[error("Export to der file failed: {0}")]
    ExportToDerFileFailed(anyhow::Error),

    #[error("Export to pem file failed: {0}")]
    ExportToPemFileFailed(anyhow::Error),

    #[error("Import from pem file failed: {0}")]
    ImportFromPemFileFailed(anyhow::Error),
}
