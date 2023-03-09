use anyhow::{anyhow, Result};
use std::{fs, path::Path};

// Helper functions for dealing with files and directories

// Note (amiller68): I moved these to fake-file because they are useful there, but I'll keep these
// here for now because they are used as utility functions in this crate
// TODO: Scope out how these are used and potentially deprecate them

/// Ensures that the given path exists and is a directory
/// # Arguments
/// path: The path to check
/// # Returns
/// Creates the directory if it doesn't exist, and is a directory
/// Result<()>
/// # Panics
/// Panics if the path exists but is not a directory
/// # Examples
/// ```no_run
/// use dataprep_lib::utils::fs::ensure_path_exists_and_is_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_dir(&path).unwrap();
/// ```
pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        // create path if it doesn't exist
        fs::create_dir_all(path)?;
    }
    if !path.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }
    Ok(())
}

/// Ensures that the given path exists and is a directory and is empty
/// # Arguments
/// path: The path to check
/// # Returns
/// Creates the directory if it doesn't exist. Makes the directory empty if it is not empty.
/// Result<()>
/// # Panics
/// Panics if the path is not an existing directory.
/// Panics if the path is not empty and force is false.
/// # Examples
/// ```no_run
/// use dataprep_lib::utils::fs::ensure_path_exists_and_is_empty_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_empty_dir(&path, false).unwrap();
/// ```
pub fn ensure_path_exists_and_is_empty_dir(path: &Path, force: bool) -> Result<()> {
    // Check the path exists and is a directory
    ensure_path_exists_and_is_dir(path)?;
    // Check the path is empty
    if path.read_dir().unwrap().count() > 0 {
        // If force is true, make the path empty
        if force {
            fs::remove_dir_all(path)?;
            fs::create_dir_all(path)?;
        } else {
            return Err(anyhow!("Path is not empty: {}", path.display()));
        }
    }
    Ok(())
}
