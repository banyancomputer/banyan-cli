use thiserror::Error;

#[derive(Debug, Error)]
/// CAR errors.
pub enum CARError {
    #[error("CARv1 had a malformed header")]
    /// The CARv1 Header was not correct
    V1Header,
    #[error("CARv2 had a malformed index")]
    /// The CARv2 Index was not correct
    Index,
    /// Index codec
    #[error("CARv2 Index codec was wrong")]
    Codec,
}
