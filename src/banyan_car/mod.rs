/// CAR Errors
pub mod error;
/// CARv1
pub mod v1;
/// CARv2
pub mod v2;
/// Varint functionality used by both CARs
mod varint;
/// Streamable Trait and testing Macro
mod streamable;
pub use streamable::*;