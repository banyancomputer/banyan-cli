//! This crate contains modules which are compiled to WASM
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
/// Fetch remote data
mod fetch;
/// Expose Tomb metadata
mod metadata;
/// Misc utilities
mod utils;
