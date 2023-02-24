use anyhow::{anyhow, Result};
use rand::Rng;
use std::{
    cmp, fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

// Note (amiller68): I moved these to fake-file because they are useful there, but I'll keep these
// here for now because they are used as utility functions in this crate

/// Creates a random file at the given path with the given size
/// # Arguments
/// * `path` - The path to create the file at
/// * `size` - The size of the file to create in bytes
/// # Panics
/// Panics if the file cannot be created
/// # Examples
/// ```no_run
/// use dataprep_pipelines::utils::fs::create_random_file;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test.txt");
/// create_random_file(path, 1024);
/// ```
#[doc(hidden)]
pub fn create_random_file(path: PathBuf, size: usize) {
    let file = fs::File::create(path).unwrap();
    let mut rng = rand::thread_rng();
    let mut writer = BufWriter::new(file);

    let mut buffer = [0; 1024];
    let mut remaining_size = size;

    while remaining_size > 0 {
        let to_write = cmp::min(remaining_size, buffer.len());
        let buffer = &mut buffer[..to_write];
        rng.fill(buffer);
        writer.write_all(buffer).unwrap();

        remaining_size -= to_write;
    }
}

/// Ensures that the given path exists and is a directory
/// # Arguments
/// path: The path to check
/// # Returns
/// Creates the directory if it doesn't exist, and is a directory
/// Result<()>
/// # Panics
/// Panics if the path exists but is not a directory
#[doc(hidden)]
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
/// use dataprep_pipelines::utils::fs::ensure_path_exists_and_is_empty_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_empty_dir(&path, false).unwrap();
/// ```
#[doc(hidden)]
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
