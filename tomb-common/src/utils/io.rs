use anyhow::Result;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

/// Grab a read-only reference to a file
pub fn get_read(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().read(true).open(path)
}

/// Grab a write-only reference to a file
pub fn get_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().create(true).append(false).write(true).open(path)
}

/// Get a read-write reference to a File on disk
pub fn get_read_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new()
        .append(false)
        .read(true)
        .write(true)
        .open(path)
}
