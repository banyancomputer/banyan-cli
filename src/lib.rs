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

pub mod banyan_cli;
pub mod banyan_common;

#[cfg(target_arch = "wasm32")]
pub mod banyan_wasm;

#[macro_use]
extern crate log;
