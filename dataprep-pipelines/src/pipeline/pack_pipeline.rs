use crate::plan_copy::plan_copy;
use crate::types::pipeline::PipelineToDisk;
use crate::{fsutil, spider, vacuum};
use anyhow::Result;
use futures::FutureExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

pub async fn pack_pipeline(
    input_dir: PathBuf,
    output_dir: PathBuf,
    manifest_file: PathBuf,
    target_chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    // Get the output DIR from the command line arguments
    let output_dir = output_dir.canonicalize()?;
    fsutil::ensure_path_exists_and_is_empty_dir(&output_dir)
        .expect("output directory must exist and be empty");

    // Note (amiller68): We don't necessarily need to create the keys dir, removing for now.
    // // Get the key output DIR from the command line arguments
    // let keys_dir = args.keys_dir.canonicalize().unwrap();
    // fsutil::ensure_path_exists_and_is_empty_dir(&keys_dir)
    //     .expect("keys directory must exist and be empty");

    // TODO: We need to change how we are finalizing the output of the program. For now keep this struct.
    // let mut final_output = FinalMetadata {
    //     original_prefix_to_final_prefix: Vec::new(),
    // };

    /* Copy all the files over to a scratch directory */

    let spidered = spider::spider(input_dir, follow_links)?;

    /* Perform deduplication and partitioning on the files */

    // Initialize a struct to memoize the hashes of files
    let seen_hashes = Arc::new(RwLock::new(HashMap::new()));
    // Iterate over all the futures in the stream map.
    let copy_plan = spidered.then(|origin_data| {
        let origin_data = origin_data.unwrap(); // TODO kill this unwrap
        let output_dir = output_dir.clone();
        // Clone the references to the seen_hashes map
        let local_seen_hashes = seen_hashes.clone();
        // Move the dir_entry into the future and copy the file.
        async move {
            plan_copy(
                origin_data,
                output_dir,
                local_seen_hashes,
                target_chunk_size,
            )
            .await
            .expect("copy failed")
        }
    });

    // TODO (laudiacay): For now we are doing compression in place, per-file. Make this better.
    let copied =
        copy_plan.then(|copy_plan| vacuum::pack::do_file_pipeline(copy_plan).map(|e| e.unwrap()));

    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let manifest_writer = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(manifest_file)
        .unwrap();
    serde_json::to_writer_pretty(
        manifest_writer,
        &copied
            .map(|pipeline| pipeline.try_into())
            .collect::<Result<Vec<PipelineToDisk>, anyhow::Error>>()
            .await?,
    )
    .map_err(|e| anyhow::anyhow!(e))
}
