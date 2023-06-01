use std::{
    fs::File,
    io::{BufReader, Write},
    os::unix::fs::symlink,
    path::Path,
    rc::Rc,
};

use crate::types::shared::CompressionScheme;
use anyhow::{anyhow, Result};
use blake2::{Blake2b512, Digest};
use chrono::Utc;
use rand::RngCore;
use tokio as _;
use wnfs::{
    common::BlockStore,
    private::{PrivateDirectory, PrivateFile, PrivateForest},
};

/// Compress the contents of a file at a given path
pub async fn compress_file(path: &Path) -> Result<Vec<u8>> {
    // Open the original file (just the first one!)
    let file = File::open(path).map_err(|e| {
        anyhow!(
            "could not find canonicalized path when trying to open reader to original file! {}",
            e
        )
    })?;
    // Create a reader for the original file
    let file_reader = BufReader::new(file);
    // Create a buffer to hold the compressed bytes
    let mut compressed_bytes: Vec<u8> = vec![];
    // Compress the chunk before feeding it to WNFS
    CompressionScheme::new_zstd()
        .encode(file_reader, &mut compressed_bytes)
        .unwrap();
    // Return compressed bytes
    Ok(compressed_bytes)
}

///
pub async fn decompress_bytes(content: Vec<u8>) -> Result<Vec<u8>> {
    // Get the bytes associated with this file
    // let file_content = file.get_content(forest, store).await.unwrap();
    // Create a buffer to hold the decompressed bytes
    let mut decompressed_bytes: Vec<u8> = vec![];
    // Decompress the chunk before writing to disk
    CompressionScheme::new_zstd()
        .decode(content.as_slice(), &mut decompressed_bytes)
        .unwrap();

    Ok(decompressed_bytes)
}

/// Writes content to a given path within a given WNFS filesystem, ensuring that duplicate writing is avoided
pub async fn write_file(
    path_segments: &[String],
    content: Vec<u8>,
    dir: &mut Rc<PrivateDirectory>,
    forest: &mut Rc<PrivateForest>,
    content_store: &impl BlockStore,
    rng: &mut impl RngCore,
) -> Result<()> {
    // Grab the current time
    let time = Utc::now();
    // Search through the PrivateDirectory for a Node that matches the path provided
    let result = dir
        .get_node(path_segments, true, forest, content_store)
        .await;
    // If the file does not exist in the PrivateForest or an error occurred in searching for it
    if result.is_err() || result.as_ref().unwrap().is_none() {
        // Write the compressed bytes to the BlockStore / PrivateForest / PrivateDirectory
        dir.write(
            path_segments,
            true,
            time,
            content,
            forest,
            content_store,
            rng,
        )
        .await
    }
    // If the file exists in the PrivateForest
    else {
        // Forcibly cast because we know this is a file
        let file: Rc<PrivateFile> = result.unwrap().unwrap().as_file().unwrap();
        // Grab the content that already exists in the PrivateFile at this path
        let existing_file_content = file.get_content(forest, content_store).await?;

        // Create Hashers for both the new content and the old content
        let mut h1 = Blake2b512::new();
        let mut h2 = Blake2b512::new();
        h1.update(&content);
        h2.update(&existing_file_content);

        // If the file has been modified since the last time it was packed
        if h1.finalize() != h2.finalize() {
            println!(
                "The file at {:?} has changed between the previous packing and now, rewriting",
                path_segments
            );
            // Write the new bytes to the path where the file was originally
            // TODO (organizedgrime) - Here we need to do something with versioning!
            dir.write(
                path_segments,
                true,
                time,
                content,
                forest,
                content_store,
                rng,
            )
            .await
            .unwrap();
        }

        // Return OK
        Ok(())
    }
}

/// Writes the decrypted and decompressed contents of a PrivateFile to a specified path
pub async fn file_to_disk(
    file: &Rc<PrivateFile>,
    output_dir: &Path,
    file_path: &Path,
    forest: &PrivateForest,
    store: &impl BlockStore,
) -> Result<()> {
    // If this file is a symlink
    if let Some(path) = file.symlink_origin() {
        // Write out the symlink
        symlink(output_dir.join(path), file_path)?;
    }
    // If this is a real file
    else {
        // Get the bytes associated with this file
        let content = decompress_bytes(file.get_content(forest, store).await?).await?;
        // Create the file at the desired location
        let mut output_file = File::create(file_path)?;
        // Write all contents to the output file
        output_file.write_all(&content)?;
    }

    // Return Ok
    Ok(())
}
