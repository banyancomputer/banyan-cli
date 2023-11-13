use std::{path::Path, process::Command};

use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress bar for displaying progress through a task with a predetermined style
pub fn get_progress_bar(count: u64) -> ProgressBar {
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = ProgressBar::new(count);
    // Stylize that progress bar!
    progress_bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    ).unwrap());
    progress_bar
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
        .expect("failed to restore size from output");
    // Parse that text as a number
    let size = size_str.parse::<usize>()?;
    // Ok status with size
    Ok(size)
}
