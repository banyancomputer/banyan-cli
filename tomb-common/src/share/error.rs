use thiserror::Error;

/// Key errors.
#[derive(Debug, Error)]
pub enum KeyError {
    /// Missing a key
    #[error("You are not authorized to decrypt this Bucket, request key access first.")]
    Missing,
}
