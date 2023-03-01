#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
#![deny(unreachable_pub, private_in_public)]
#![feature(async_closure)]

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

pub mod pipeline;
pub mod plan_copy;
pub mod spider;
pub mod types;
pub mod utils;
pub mod vacuum;
