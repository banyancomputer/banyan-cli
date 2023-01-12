use std::path::Path;
use anyhow::{anyhow, Result};


pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        // create path if it doesn't exist
        std::fs::create_dir_all(path)?;
    }
    if !path.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }
    Ok(())
}

pub fn ensure_path_exists_and_is_empty_dir(path: &Path) -> Result<()> {
    ensure_path_exists_and_is_dir(path)?;
    if path.read_dir().unwrap().count() > 0 {
        return Err(anyhow!("Path is not empty: {}", path.display()));
    }
    Ok(())
}
