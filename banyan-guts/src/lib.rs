//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

pub mod shared;

type WnfsError = Box<dyn std::error::Error>;
type LibipldError = wnfs::libipld::error::Error;

/// CLI Parsing
// #[cfg(not(target_arch = "wasm32"))]
// #[cfg(feature = "cli")]
// pub mod cli;

/// CLI Parsing (new version)
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "cli")]
pub mod cli2;

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
        pub use crate::api::{client, models, requests};
    }
    pub mod blockstore {
        pub use crate::blockstore::{
            BanyanApiBlockStore, BanyanBlockStore, CarV2MemoryBlockStore, DoubleSplitStore,
            MemoryBlockStore, RootedBlockStore,
        };
        #[cfg(not(target_arch = "wasm32"))]
        pub use crate::blockstore::{CarV2DiskBlockStore, MultiCarV2DiskBlockStore};
    }
    pub mod car {
        pub use crate::car::{v1, v2};
    }
    pub mod filesystem {
        pub use crate::filesystem::{serialize, sharing, wnfsio, FilesystemError, FsMetadata};
    }
    #[cfg(target_arch = "wasm32")]
    pub mod wasm {
        pub use crate::wasm::{
            register_log, TombResult, TombWasm, TombWasmError, WasmBucket, WasmBucketKey,
            WasmBucketMetadata, WasmBucketMount, WasmFsMetadataEntry, WasmMount, WasmNodeMetadata,
            WasmSnapshot,
        };
    }
}
