use anyhow::Result;
use chrono::Utc;
use fake_file::{utils::ensure_path_exists_and_is_empty_dir, Strategy, Structure};
use rand::thread_rng;
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};
use wnfs::{
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest},
};

use crate::{
    pipelines::configure,
    types::config::{bucketconfig::BucketConfig, globalconfig::GlobalConfig},
};

/// Set up temporary filesystem for test cases
pub async fn test_setup(test_name: &str) -> Result<PathBuf> {
    // Run the structured test setup with a default Structure
    test_setup_structured(test_name, Structure::new(2, 2, 2000, Strategy::Simple)).await
}

/// Set up a temporary filesystem for test cases according to specified structure
pub async fn test_setup_structured(test_name: &str, structure: Structure) -> Result<PathBuf> {
    // Deinit all
    configure::deinit_all().await?;
    // Base of the test directory
    let root_path = PathBuf::from("test").join(test_name);
    // Remove anything that might already be there
    remove_dir_all(&root_path).ok();
    // Create and empty the dir
    ensure_path_exists_and_is_empty_dir(&root_path, true)?;
    // Input and path
    let input_path = root_path.join("input");
    // Generate file structure
    structure.generate(&input_path)?;
    // Deinitialize existing data / metadata
    configure::deinit(&input_path).await.ok();
    // Return all paths
    Ok(input_path)
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

// Create all of the relevant objects, using real BlockStores and real data
pub async fn setup(
    test_name: &str,
) -> Result<(
    PathBuf,
    GlobalConfig,
    BucketConfig,
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let origin: PathBuf = Path::new("test").join(test_name);
    create_dir_all(&origin)?;
    let mut global = GlobalConfig::from_disk().await?;
    global.remove(&origin)?;
    let config = global.new_bucket(&origin)?;

    // Hot Forest and cold Forest
    let mut metadata_forest = Rc::new(PrivateForest::new());
    let mut content_forest = Rc::new(PrivateForest::new());

    // Rng
    let rng = &mut thread_rng();
    // PrivateDirectory
    let mut root_dir = Rc::new(PrivateDirectory::new(
        Namefilter::default(),
        Utc::now(),
        rng,
    ));

    // Open new file
    let file = root_dir
        .open_file_mut(
            &["cats".to_string()],
            true,
            Utc::now(),
            &mut metadata_forest,
            &config.metadata,
            rng,
        )
        .await?;

    // Set file content
    file.set_content(
        Utc::now(),
        "Hello Kitty!".as_bytes(),
        &mut content_forest,
        &config.content,
        rng,
    )
    .await?;

    Ok((
        origin,
        global,
        config,
        metadata_forest,
        content_forest,
        root_dir,
    ))
}

// Delete the temporary directory
pub async fn teardown(test_name: &str) -> Result<()> {
    let path = Path::new("test").join(test_name);
    std::fs::remove_dir_all(path)?;
    Ok(())
}
