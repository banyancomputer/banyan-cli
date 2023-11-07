//! This crate contains modules which are compiled to WASM
/// Compatibility
mod compat;
/// Utilities
pub mod utils;

/// Expose all the compatibility types directly
pub use compat::*;
