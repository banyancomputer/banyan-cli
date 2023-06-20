use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{anyhow, Result};
use chrono::Utc;
use rand::thread_rng;
use wnfs::{
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest},
};

use crate::types::config::{bucketconfig::BucketConfig, globalconfig::GlobalConfig};

pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    // If the path is non-existent
    if !path.exists() {
        // Create it
        create_dir_all(path)?;
    }

    // If the path is actually a file
    if !path.is_dir() {
        // Throw Error
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }

    // Return Ok if path exists and is a directory
    Ok(())
}

pub fn ensure_path_exists_and_is_empty_dir(path: &Path, force: bool) -> Result<()> {
    // Check the path exists and is a directory
    ensure_path_exists_and_is_dir(path)?;
    // Check the path is empty
    if path.read_dir().unwrap().count() > 0 {
        // If force is true, make the path empty
        if force {
            remove_dir_all(path)?;
            create_dir_all(path)?;
        } else {
            return Err(anyhow!("Path is not empty: {}", path.display()));
        }
    }
    Ok(())
}

// Create all of the relevant objects, using real BlockStores and real data
pub async fn setup(
    test_name: &str,
) -> Result<(
    PathBuf,
    BucketConfig,
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let origin: PathBuf = Path::new("test").join(test_name);
    create_dir_all(&origin)?;
    GlobalConfig::remove(&origin)?;
    let config = GlobalConfig::new_bucket(&origin)?;
    let content = config.get_metadata()?;
    let metadata = config.get_content()?;

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
            &metadata,
            rng,
        )
        .await?;

    // Set file content
    file.set_content(
        Utc::now(),
        "Hello Kitty!".as_bytes(),
        &mut content_forest,
        &content,
        rng,
    )
    .await?;

    Ok((origin, config, metadata_forest, content_forest, root_dir))
}

// Delete the temporary directory
pub async fn teardown(test_name: &str) -> Result<()> {
    let path = Path::new("test").join(test_name);
    std::fs::remove_dir_all(path)?;
    Ok(())
}
