use std::path::PathBuf;
use anyhow::Result;
use fake_file::{Structure, Strategy};

use super::fs::ensure_path_exists_and_is_empty_dir;

/// Set up temporary filesystem for test cases
pub async fn test_setup(test_name: &str) -> Result<(PathBuf, PathBuf)> {
    // Base of the test directory
    let root_path = PathBuf::from("test").join(test_name);
    // Create and empty the dir
    ensure_path_exists_and_is_empty_dir(&root_path, true)?;
    // Input and output paths
    let input_path = root_path.join("input");
    let output_path = root_path.join("output");
    // Generate file structure
    Structure::new(2, 2, 2000, Strategy::Simple).generate(&input_path)?;
    // Return all paths
    Ok((input_path, output_path))
}

/// Remove contents of temporary dir
pub async fn test_teardown(test_name: &str) -> Result<()> {
    Ok(std::fs::remove_dir_all(PathBuf::from("test").join(test_name))?)
}