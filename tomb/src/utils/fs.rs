use anyhow::{anyhow, Result};
use std::{fs, path::Path};

// Helper functions for dealing with files and directories

// Note (amiller68): I moved these to fake-file because they are useful there, but I'll keep these
// here for now because they are used as utility functions in this crate
// TODO: Scope out how these are used and potentially deprecate them

/// Ensures that a given path exists and is a directory.
/// # Arguments
/// * `path` - The path to check
/// # Returns
/// Returns a `Result<()>` with no value if operation was successful, or an Error if something went wrong.
///
/// # Examples
/// ```no_run
/// use tomb_lib::utils::fs::ensure_path_exists_and_is_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_dir(&path).unwrap();
/// ```
pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    // If the path is non-existent
    if !path.exists() {
        // Create it
        fs::create_dir_all(path)?;
    }

    // If the path is actually a file
    if !path.is_dir() {
        // Throw Error
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }

    // Return Ok if path exists and is a directory
    Ok(())
}

/// Ensures that a given path exists and is a directory and is empty.
/// # Arguments
/// * `path` - The path to check
/// # Returns
/// Returns a `Result<()>` with no value if operation was successful, or an Error if something went wrong.
/// # Examples
/// ```no_run
/// use tomb_lib::utils::fs::ensure_path_exists_and_is_empty_dir;
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
