/// CAR Errors
mod error;
/// Streamable Trait and testing Macro
mod streamable;
/// CARv1
pub mod v1;
/// CARv2
pub mod v2;
/// Varint functionality used by both CARs
mod varint;

#[allow(unused)]
pub(crate) use streamable::{streamable_tests, Streamable};
