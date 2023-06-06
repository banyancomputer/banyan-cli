use anyhow::Result;
use fake_file::{Strategy, Structure};
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    process::{Child, Command},
};

use super::fs::ensure_path_exists_and_is_empty_dir;

/// Set up temporary filesystem for test cases
pub async fn test_setup(test_name: &str) -> Result<(PathBuf, PathBuf)> {
    // Run the structured test setup with a default Structure
    test_setup_structured(test_name, Structure::new(2, 2, 2000, Strategy::Simple)).await
}

/// Set up a temporary filesystem for test cases according to specified structure
pub async fn test_setup_structured(
    test_name: &str,
    structure: Structure,
) -> Result<(PathBuf, PathBuf)> {
    // Base of the test directory
    let root_path = PathBuf::from("test").join(test_name);
    // Remove anything that might already be there
    remove_dir_all(&root_path).ok();
    // Create and empty the dir
    ensure_path_exists_and_is_empty_dir(&root_path, true)?;
    // Input and output paths
    let input_path = root_path.join("input");
    let output_path = root_path.join("output");
    create_dir_all(&output_path)?;
    // Generate file structure
    structure.generate(&input_path)?;
    // Return all paths
    Ok((input_path, output_path))
}

/// Remove contents of temporary dir
pub async fn test_teardown(test_name: &str) -> Result<()> {
    Ok(std::fs::remove_dir_all(
        PathBuf::from("test").join(test_name),
    )?)
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
    let size_str = output_str.split('\t').next().unwrap();
    // Parse that text as a number
    let size = size_str.parse::<usize>()?;
    // Ok status with size
    Ok(size)
}

/// Starts a local IPFS daemon for running network tests
pub fn start_daemon() -> Child {
    Command::new("ipfs")
        .arg("daemon")
        .spawn()
        .expect("ipfs daemon failed to start. ensure it is installed.")
}
