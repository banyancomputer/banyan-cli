use anyhow::Result;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    plan_copy::plan_copy,
    spider,
    types::{
        pipeline::{CodablePipeline, Pipeline},
        plan::PipelinePlan,
        spider::SpiderMetadata,
    },
    utils::fs as fsutil,
    vacuum,
};

pub async fn pack_pipeline(
    input_dir: PathBuf,
    output_dir: PathBuf,
    manifest_file: PathBuf,
    target_chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    // Get the output DIR from the command line arguments
    let output_dir = output_dir.canonicalize()?;

    fsutil::ensure_path_exists_and_is_empty_dir(&output_dir, false)
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

    let spidered: Vec<SpiderMetadata> = spider::spider(input_dir, follow_links).await?;

    /* Perform deduplication and partitioning on the files */

    // Initialize a struct to memorize the hashes of files
    // TODO bye bye rwlock
    let seen_hashes = Arc::new(std::sync::RwLock::new(HashMap::new()));
    // Iterate over all the futures in the stream map, await all of them together
    let copy_plan = spidered
        .iter()
        .map(|spider_metadata| {
            // Clone the output_dir reference
            let output_dir = output_dir.clone();
            // Move the dir_entry into the future and copy the file.
            plan_copy(
                spider_metadata.clone(),
                output_dir,
                seen_hashes.clone(),
                target_chunk_size,
            )
        })
        .map(|e| e.unwrap())
        .collect::<Vec<PipelinePlan>>();

    // TODO (laudiacay): For now we are doing compression in place, per-file. Make this better.
    let copied = futures::future::join_all(
        copy_plan
            .iter()
            .map(|copy_plan| vacuum::pack::do_file_pipeline(copy_plan.clone())),
    )
    .await
    .into_iter()
    .map(|e| e.unwrap())
    .collect::<Vec<Pipeline>>();

    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let manifest_writer = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(manifest_file)
        .unwrap();

    serde_json::to_writer_pretty(
        manifest_writer,
        // Iterate over the copied files and convert them to codable pipelines
        &copied
            .iter()
            .map(|pipeline| pipeline.clone().into())
            .collect::<Vec<CodablePipeline>>(),
    )
    .map_err(|e| anyhow::anyhow!(e))
}
