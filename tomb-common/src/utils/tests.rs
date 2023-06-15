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

use crate::types::{
    blockstore::{
        car::carv2::carv2blockstore::CarV2BlockStore, networkblockstore::NetworkBlockStore,
    },
    pipeline::Manifest,
};

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
    local: bool,
    test_name: &str,
) -> Result<(
    PathBuf,
    Manifest,
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let path = Path::new("test").join(test_name);
    ensure_path_exists_and_is_empty_dir(&path, true)?;
    let content_path = path.join("content");
    ensure_path_exists_and_is_empty_dir(&content_path, true)?;
    let content_car = content_path.join("content.car");
    let content = CarV2BlockStore::new(&content_car)?;

    let tomb_path = path.join(".tomb");
    ensure_path_exists_and_is_empty_dir(&tomb_path, true)?;
    let meta_car = tomb_path.join("meta.car");
    let metadata = CarV2BlockStore::new(&meta_car)?;

    // Remote endpoint
    let cold_remote = NetworkBlockStore::new("http://127.0.0.1", 5001);
    let hot_remote = NetworkBlockStore::new("http://127.0.0.1", 5001);

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
    let file = if local {
        root_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                &mut metadata_forest,
                &metadata,
                rng,
            )
            .await?
    } else {
        root_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                &mut metadata_forest,
                &hot_remote,
                rng,
            )
            .await?
    };

    // Set file content
    if local {
        file.set_content(
            Utc::now(),
            "Hello Kitty!".as_bytes(),
            &mut content_forest,
            &content,
            rng,
        )
        .await?;
    } else {
        file.set_content(
            Utc::now(),
            "Hello Kitty!".as_bytes(),
            &mut content_forest,
            &cold_remote,
            rng,
        )
        .await?;
    }

    // Create the Manifest
    let manifest_data = Manifest {
        version: "1.1.0".to_string(),
        content,
        metadata,
    };

    Ok((
        tomb_path,
        manifest_data,
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
