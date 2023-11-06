//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![feature(io_error_more)]
#![feature(let_chains)]
#![feature(buf_read_has_data_left)]
#![feature(async_closure)]
#![feature(dec2flt)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
#![deny(private_interfaces)]
// #![deny(unreachable_pub)]
#![feature(seek_stream_len)]

#[cfg(not(target_arch = "wasm32"))]
/// CLI Parsing
pub mod banyan_cli;
#[cfg(not(target_arch = "wasm32"))]
/// Native functionality
pub mod banyan_native;

#[cfg(not(target_arch = "wasm32"))]
#[macro_use]
extern crate log;

/// API Interaction
pub mod banyan_api;
/// BlockStores
pub mod banyan_blockstore;
/// CAR Format Parsing
pub mod banyan_car;
/// Architecture-Independent functionality
pub mod banyan_common;

#[cfg(target_arch = "wasm32")]
pub mod banyan_wasm;
