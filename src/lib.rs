//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![feature(io_error_more)]
#![feature(let_chains)]
#![feature(buf_read_has_data_left)]
#![feature(async_closure)]
#![feature(dec2flt)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]
#![deny(private_interfaces)]
// #![deny(unreachable_pub)]
#![feature(seek_stream_len)]

/// CLI Parsing
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "cli")]
pub mod cli;

/// Native functionality
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(not(target_arch = "wasm32"))]
#[macro_use]
extern crate log;

pub(crate) mod api;
pub(crate) mod blockstore;
pub(crate) mod car;
pub(crate) mod filesystem;
pub(crate) mod utils;
#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm;

pub mod prelude {
    pub mod api {
        pub use crate::api::*;
    }
    pub mod blockstore {
        pub use crate::blockstore::*;
    }
    pub mod car {
        pub use crate::car::*;
    }
    pub mod filesystem {
        pub use crate::filesystem::*;
    }
    #[cfg(target_arch = "wasm32")]
    pub mod wasm {
        pub use crate::wasm::*;
    }
}
