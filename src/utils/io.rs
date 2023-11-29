use std::{
    fs::{File, OpenOptions},
    path::Path,
};

#[cfg(test)]
use {super::UtilityError, std::process::Command};

/// Grab a read-only reference to a file
pub fn get_read(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().read(true).open(path)
}

/// Grab a write-only reference to a file
pub fn get_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new()
        .create(true)
        .append(false)
        .write(true)
        .open(path)
}

/// Get a read-write reference to a File on disk
pub fn get_read_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
    // .append(false)
}

/// Determines the size of the contents of a directory.
/// This standard unix tool handles far more edge cases than we could ever hope
/// to approximate with a hardcoded recursion step, and with more efficiency too.
#[cfg(test)]
#[allow(dead_code)]
pub fn compute_directory_size(path: &Path) -> Result<usize, UtilityError> {
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
    size_str
        .parse::<usize>()
        .map_err(|_| UtilityError::custom("int parse"))
}
