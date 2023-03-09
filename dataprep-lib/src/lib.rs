//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
#![deny(private_in_public)]
// #![deny(unreachable_pub)]
#![feature(async_closure)]
#![feature(dec2flt)]

/* General Project Chores */
// TODO (xBalbinus & thea-exe): Handle panics appropriately/get rid of all the unwraps
// TODO (xBalbinus & thea-exe): get rid of all the clones and stop copying around pathbufs

/* Bugs */
// TODO (amiller68 and laudiacay): Pipeline unpacks single file input to a empty directory with the same name as the file, instead of the file

/* Speculative Lifts */
// TODO (laudiacay): Can / Should we include an option to pack chunks into a CAR file? Look into this.
// TODO (laudiacay) : Handle pinning threads to CPU cores (with tokio localsets and runtimes?) correctly so that disk throughput is maximized

/* Dataprep:
* 1. Copy files to scratch space from `input` directories to 'output-dir' directory
* 2. Partition files into chunks of max size `target-chunk-size`
* 3. Compress and encrypt each chunk in place. These chunks should be randomly named.
* 4. Write out a manifest file that maps:
*      - original file path to random chunk name / path
*      - random chunk paths point to the key-path used to encrypt the chunk.
*      - keys stored in csv file
* 5. Encyprpt the manifest file in place with some master key. (later, optional)
* 6. Use manifest file to repopulate the original directory structure
* 7. TODO (laudiacay): Make car file with it.
*/

// Used by benchmarking and testing
use criterion as _;
use dir_assert as _;
use fake_file as _;
use fs_extra as _;
use lazy_static as _;

#[allow(unused_extern_crates)]
extern crate core;

/// This module contains both the pack_pipeline and the unpack_pipeline, which allow the main CLI to run packing an unpacking pipelines.
pub mod do_pipeline_and_write_metadata;
/// This module contains code designed to analyze directory metadata by traversal.
pub mod spider;
/// This module contains types unique to this project.
pub mod types;
/// This module contains filesytem helper functions and hasher helper functions.
pub mod utils;
/// This module contains the packing pipeline.
pub mod vacuum;
