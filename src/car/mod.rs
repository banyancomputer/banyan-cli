/// CAR Errors
pub(crate) mod error;
/// Streamable Trait and testing Macro
mod streamable;
/// CARv1
pub mod v1;
/// CARv2
pub mod v2;

#[allow(unused)]
pub(crate) use streamable::{streamable_tests, Streamable};
