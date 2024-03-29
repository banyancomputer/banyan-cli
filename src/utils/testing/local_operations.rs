use crate::{native::operations::configure, utils::UtilityError};
use fake_file::{utils::ensure_path_exists_and_is_empty_dir, Strategy, Structure};
use std::{fs::remove_dir_all, path::PathBuf};

/// Set up temporary filesystem for test cases
pub async fn test_setup(test_name: &str) -> Result<PathBuf, UtilityError> {
    // Run the structured test setup with a default Structure
    test_setup_structured(test_name, Structure::new(2, 2, 2000, Strategy::Simple)).await
}

/// Set up a temporary filesystem for test cases according to specified structure
pub async fn test_setup_structured(
    test_name: &str,
    structure: Structure,
) -> Result<PathBuf, UtilityError> {
    // Deinit all
    configure::deinit_all().await?;
    // Base of the test directory
    let root_path = PathBuf::from("test").join(test_name);
    // Remove anything that might already be there
    if root_path.exists() {
        remove_dir_all(&root_path)?;
    }
    // Create and empty the dir
    ensure_path_exists_and_is_empty_dir(&root_path, true).map_err(Box::from)?;
    // Input and path
    let input_path = root_path.join("input");
    // Generate file structure
    structure.generate(&input_path).map_err(Box::from)?;
    // Deinitialize existing data / metadata
    configure::deinit(&input_path).await?;
    configure::init(test_name, &input_path).await?;
    // Return all paths
    Ok(input_path.clone())
}

/// Remove contents of temporary dir
pub async fn test_teardown(test_name: &str) -> Result<(), UtilityError> {
    Ok(std::fs::remove_dir_all(
        PathBuf::from("test").join(test_name),
    )?)
}
