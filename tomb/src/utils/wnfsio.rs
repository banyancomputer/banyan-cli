use std::{fs::File, io::Write, os::unix::fs::symlink, path::Path, process::Command, rc::Rc};

use crate::pipelines::error::TombError;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tomb_common::utils::wnfsio::decompress_bytes;
use wnfs::{
    common::BlockStore,
    private::{PrivateFile, PrivateForest},
};

/// Writes the decrypted and decompressed contents of a PrivateFile to a specified path
pub async fn file_to_disk(
    file: &Rc<PrivateFile>,
    output_dir: &Path,
    file_path: &Path,
    content_forest: &PrivateForest,
    content: &impl BlockStore,
) -> Result<(), TombError> {
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
        Err(TombError::file_missing_error(file_path.to_path_buf()))
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

/// Determines the size of the contents of a directory.
/// This standard unix tool handles far more edge cases than we could ever hope
/// to approximate with a hardcoded recursion step, and with more efficiency too.
pub fn compute_directory_size(path: &Path) -> Result<usize> {
    // Execute the unix du command to evaluate the size of the given path in kilobytes
    let output = Command::new("du")
        .arg("-sh")
        .arg("-k")
        .arg(path.display().to_string())
        .output()?;
    // Interpret the output as a string
    let output_str = String::from_utf8(output.stdout)?;
    // Grab all text before the tab
    let size_str = output_str
        .split('\t')
        .next()
        .expect("failed to extract size from output");
    // Parse that text as a number
    let size = size_str.parse::<usize>()?;
    // Ok status with size
    Ok(size)
}
