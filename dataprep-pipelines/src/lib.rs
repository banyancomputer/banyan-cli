#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]

/* General Project Chores */
// TODO (xBalbinus & thea-exe): Handle panics appropriately/get rid of all the unwraps
// TODO (xBalbinus & thea-exe): get rid of all the clones and stop copying around pathbufs
// TODO (xBalbinus & thea-exe): generally clean up imports and naming. the fs_yadayadayada stuff is particularly bad.

/* Hardcore project TODOs before mvp */
// TODO (laudiacay): We can implement the pipeline with a single FS read maybe. Look into this. Be sure to tally up the reads before attempting this.
// TODO (laudiacay): Encrypt filenames and other metadata. Need to hide directory structure.

/* Speculative Lifts */
// TODO (laudiacay): Can / Should we include an option to pack chunks into a CAR file? Look into this.
// TODO (laudiacay): What if we tried encrypting the file in place with one file handle. Look into this.
// TODO (laudiacay) : Handle pinning threads to CPU cores (with tokio localsets and runtimes?) correctly so that disk throughput is maximized

/* Dataprep:
 * 1. Copy files to scratch space from `input` directories to 'output-dir' directory
 * 2. Partition files into chunks of max size `target-chunk-size`
 * 3. Compress and encrypt each chunk in place. These chunks should be randomly named.
 * 4. Write out a manifest file that maps:
 *      - original file path to random chunk name / path
 *      - random chunk paths point to the key-path used to encrypt the chunk.
 *      - keys stored in csv file
 * 5. TODO (laudiacay): Encyprpt the manifest file in place with some master key.
 * 6. Use manifest file to repopulate the original directory structure
 * 7. TODO (laudiacay): Make car file with it.
 */

use dir_assert as _;
use lazy_static as _;

extern crate core;

pub mod crypto_tools;
pub mod fs_carfiler;
pub mod fsutil;
pub mod pipeline;
pub mod plan_copy;
pub mod spider;
pub mod types;
pub mod vacuum;
