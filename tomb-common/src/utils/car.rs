
use anyhow::Result;
use std::{path::Path, fs::{OpenOptions, File}};

pub fn get_read(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().read(true).open(path)?)
}
pub fn get_write(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .append(false)
        .write(true)
        .open(path)?)
}