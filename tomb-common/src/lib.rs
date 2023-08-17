//! This crate contains modules which are shared by both CLI and WASM clients
#![feature(read_buf)]
#![feature(seek_stream_len)]
#![feature(associated_type_bounds)]
#![feature(let_chains)]
#![feature(file_create_new)]
#![feature(duration_constants)]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
// #![deny(unused_crate_dependencies)]

/// Banyan types and client
pub mod banyan;
/// BlockStore implementations
pub mod blockstore;
/// Car types and implementations
pub mod car;
/// Our encyption key types and helpers
pub mod keys;
/// Utilities
pub mod utils;

/// Common Traits
mod traits;
