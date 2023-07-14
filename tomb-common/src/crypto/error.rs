use thiserror::Error;

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// RSA related errors
#[derive(Debug, Error)]
pub enum RsaError {
    #[error("Invalid public key: {0}")]
    /// PublicKey was invalid
    InvalidPublicKey(anyhow::Error),

    #[error("Encryption failed: {0}")]
    /// Encrypt op failed
    EncryptionFailed(anyhow::Error),

    #[error("Decryption failed: {0}")]
    /// Decrypt op failed
    DecryptionFailed(anyhow::Error),

    #[error("Import from der file failed: {0}")]
    /// Import DER op failed
    ImportFromDerFileFailed(anyhow::Error),

    #[error("Export to der file failed: {0}")]
    /// Export DER op failed
    ExportToDerFileFailed(anyhow::Error),

    /// Import PEM op failed
    #[error("Export to pem file failed: {0}")]
    ExportToPemFileFailed(anyhow::Error),

    /// Export PEM op failed
    #[error("Import from pem file failed: {0}")]
    ImportFromPemFileFailed(anyhow::Error),
}
