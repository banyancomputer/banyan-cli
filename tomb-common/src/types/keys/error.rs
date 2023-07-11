use thiserror::Error;

/// Key errors.
#[derive(Debug, Error)]
pub enum KeyError {
    /// Missing a key
    #[error("There is no key in this bucket matching your PrivateKey")]
    Missing,
}
