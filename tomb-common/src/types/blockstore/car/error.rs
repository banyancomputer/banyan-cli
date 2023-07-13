use thiserror::Error;

#[derive(Debug, Error)]
/// CAR errors.
pub enum CARError {
    #[error("CAR v1 Had a malformed header")]
    /// The CARv1Header was not correct
    MalformedV1Header,
}
