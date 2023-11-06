/* General Project Chores */
/* Bugs */
// TODO (amiller68 and laudiacay): Pipeline restores single file input to a empty directory with the same name as the file, instead of the file

/* Speculative Lifts */
// TODO (laudiacay): Can / Should we include an option to prepare chunks into a CAR file? Look into this.
// TODO (laudiacay) : Handle pinning threads to CPU cores (with tokio localsets and runtimes?) correctly so that disk throughput is maximized

/* tomb:
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

// We only use this in main.rs
use env_logger as _;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use lazy_static as _;

#[allow(unused_extern_crates)]
extern crate core;

/// This module contains both the prepare_pipeline and the restore_pipeline, which allow the main CLI to run bundling an restoring pipelines.
pub mod pipelines;
/// This module contains types unique to this project.
pub mod types;
/// This module contains filesytem helper functions and hasher helper functions.
pub mod utils;
