use std::{
    fs::File,
    io::{BufReader, Read, Write},
    os::unix::fs::symlink,
    path::Path,
    rc::Rc,
};

use crate::{pipelines::error::PipelineError, types::shared::CompressionScheme};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tokio as _;
use wnfs::{
    common::BlockStore,
    private::{PrivateFile, PrivateForest},
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

/// Writes the decrypted and decompressed contents of a PrivateFile to a specified path
pub async fn file_to_disk(
    file: &Rc<PrivateFile>,
    output_dir: &Path,
    file_path: &Path,
    content_forest: &PrivateForest,
    content: &impl BlockStore,
) -> Result<(), PipelineError> {
    // If this file is a symlink
    if let Some(path) = file.symlink_origin() {
        // Write out the symlink
        symlink(output_dir.join(path), file_path)?;
        Ok(())
    }
    // If this is a real file, try to read in the content
    else if let Ok(compressed_buf) = file.get_content(content_forest, content).await {
        // Create the file at the desired location
        let mut output_file = File::create(file_path)?;
        // Buffer for decrypted and decompressed file content
        let mut decompressed_buf: Vec<u8> = Vec::new();
        // Decompress
        decompress_bytes(compressed_buf.as_slice(), &mut decompressed_buf)?;
        // Write out the content to disk
        output_file.write_all(&decompressed_buf)?;
        Ok(())
    } else {
        Err(PipelineError::FileNotFound(
            file_path.to_str().unwrap().to_string(),
        ))
    }
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
