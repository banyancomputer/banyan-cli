use anyhow::Result;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub fn get_read(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().read(true).open(path)
}
pub fn get_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().append(false).write(true).open(path)
}
