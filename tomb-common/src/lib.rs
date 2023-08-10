//! This crate contains modules which are shared by both CLI and WASM clients
#![feature(read_buf)]
#![feature(seek_stream_len)]
#![feature(associated_type_bounds)]
#![feature(let_chains)]
#![feature(file_create_new)]
#![feature(duration_constants)]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
// #![deny(unused_crate_dependencies)]

/// Server API
pub mod api;
/// BlockStores
pub mod blockstore;
/// Errors
pub mod error;
/// Keys and Auth
pub mod keys;
/// Serialization
pub mod serialize;
/// Tests
pub mod test;

mod streamable;
pub use streamable::Streamable;
