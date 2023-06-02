use std::{
    fs::File,
    io::{BufReader, Read, Write},
    os::unix::fs::symlink,
    path::Path,
    rc::Rc,
};

use crate::types::shared::CompressionScheme;
use anyhow::Result;
use blake2::{Blake2b512, Digest};
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use rand::RngCore;
use tokio as _;
use wnfs::{
    common::BlockStore,
    private::{PrivateDirectory, PrivateFile, PrivateForest},
};

/// Compresses bytes
pub fn compress_bytes<R, W>(reader: R, writer: W) -> Result<()>
where
    R: Read,
    W: Write,
{
    Ok(CompressionScheme::new_zstd().encode(reader, writer)?)
}

/// Decompresses bytes
pub fn decompress_bytes<R, W>(reader: R, writer: W) -> Result<()>
where
    R: Read,
    W: Write,
{
    Ok(CompressionScheme::new_zstd().decode(reader, writer)?)
}

/// Compress the contents of a file at a given path
pub fn compress_file(path: &Path) -> Result<Vec<u8>> {
    // Open the original file (just the first one!)
    let file = File::open(path)?;
    // Create a reader for the original file
    let reader = BufReader::new(file);
    // Create a buffer to hold the compressed bytes
    let mut compressed: Vec<u8> = vec![];
    // Compress the chunk before feeding it to WNFS
    compress_bytes(reader, &mut compressed)?;
    // Return compressed bytes
    Ok(compressed)
}

/// Writes content to a given path within a given WNFS filesystem, ensuring that duplicate writing is avoided
pub async fn write_file(
    path_segments: &[String],
    content: Vec<u8>,
    dir: &mut Rc<PrivateDirectory>,
    forest: &mut Rc<PrivateForest>,
    content_local: &impl BlockStore,
    rng: &mut impl RngCore,
) -> Result<()> {
    // Grab the current time
    let time = Utc::now();
    // Search through the PrivateDirectory for a Node that matches the path provided
    let result = dir
        .get_node(path_segments, true, forest, content_local)
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
            content_local,
            rng,
        )
        .await
    }
    // If the file exists in the PrivateForest
    else {
        // Forcibly cast because we know this is a file
        let file: Rc<PrivateFile> = result.unwrap().unwrap().as_file().unwrap();
        // Grab the content that already exists in the PrivateFile at this path
        let existing_file_content = file.get_content(forest, content_local).await?;

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
                content_local,
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
        // Create the file at the desired location
        let mut output_file = File::create(file_path)?;
        // Buffer for decrypted and decompressed file content
        let mut content: Vec<u8> = Vec::new();
        // Get and decompress bytes associated with this file
        decompress_bytes(
            file.get_content(forest, store).await?.as_slice(),
            &mut content,
        )?;
        // Write all contents to the output file
        output_file.write_all(&content)?;
    }

    // Return Ok
    Ok(())
}

/// Create a progress bar for displaying progress through a task with a predetermined style
pub fn get_progress_bar(count: u64) -> Result<ProgressBar> {
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = ProgressBar::new(count);
    // Stylize that progress bar!
    progress_bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?);

    Ok(progress_bar)
}
