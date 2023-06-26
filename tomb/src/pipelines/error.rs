use thiserror::Error;

/// Pipeline errors.
#[derive(Debug, Error)]
pub(crate) enum PipelineError {
    #[error("Directory has not been initialized")]
    Uninitialized(),
    // #[error("Cannot find specified CID in block store: {0}")]
    // CIDNotFound(Cid),

    // #[error("Lock poisoned")]
    // LockPoisoned,
}
