use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use std::{
    fs::{create_dir_all, File, OpenOptions},
    path::{Path, PathBuf},
    rc::Rc,
};
use wnfs::{
    libipld::Cid,
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest},
};

use crate::types::blockstore::{
    tombblockstore::TombBlockStore, tombmemoryblockstore::TombMemoryBlockStore,
};

/// Create a copy of a given fixture to play around with
pub fn car_setup(
    version: usize,
    fixture_suffix: &str,
    test_name: &str,
) -> Result<PathBuf, std::io::Error> {
    // The existing path
    let fixture_path = Path::new("..")
        .join("car-fixtures")
        .join(format!("carv{}-{}.car", version, fixture_suffix));
    // Root of testing dir
    let test_path = &Path::new("test").join("car");
    // Create it it doesn't exist
    create_dir_all(test_path).ok();
    // The new path
    let new_path = test_path.join(format!("carv{}_{}.car", version, test_name));
    // Remove file if it's already there
    std::fs::remove_file(&new_path).ok();
    // Copy file from fixture path to tmp path
    std::fs::copy(fixture_path, &new_path)?;
    // Return Ok with new path
    Ok(new_path)
}

/// Create a copy of a given fixture to play around with
pub fn carindex_setup(
    version: usize,
    fixture_suffix: &str,
    test_name: &str,
) -> Result<PathBuf, std::io::Error> {
    // The existing path
    let fixture_path = Path::new("..")
        .join("car-fixtures")
        .join(format!("carv{}-{}.carindex", version, fixture_suffix));
    // Root of testing dir
    let test_path = &Path::new("test").join("car");
    // Create it it doesn't exist
    create_dir_all(test_path).ok();
    // The new path
    let new_path = test_path.join(format!("carv{}_{}.carindex", version, test_name));
    // Remove file if it's already there
    std::fs::remove_file(&new_path).ok();
    // Copy file from fixture path to tmp path
    std::fs::copy(fixture_path, &new_path)?;
    // Return Ok with new path
    Ok(new_path)
}

/// Setup using a TombMemoryBlockStore
pub async fn setup_memory(test_name: &str
) -> Result<(
    TombMemoryBlockStore,
    TombMemoryBlockStore,
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    setup(test_name, TombMemoryBlockStore::new(), TombMemoryBlockStore::new()).await
}

/// Create all of the relevant objects, using real BlockStores and real data
pub async fn setup<TBS: TombBlockStore>(
    test_name: &str,
    metadata: TBS,
    content: TBS,
) -> Result<(
    TBS,
    TBS,
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let origin: PathBuf = Path::new("test").join(test_name);
    create_dir_all(&origin)?;

    metadata.set_root(&Cid::default());
    content.set_root(&Cid::default());

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

    Ok((
        metadata,
        content,
        metadata_forest,
        content_forest,
        root_dir,
    ))
}

/// Delete the temporary directory
pub async fn teardown(test_name: &str) -> Result<()> {
    let path = Path::new("test").join(test_name);
    std::fs::remove_dir_all(path)?;
    Ok(())
}

/// Grab a read-only reference to a file
pub fn get_read(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().read(true).open(path)
}

/// Grab a write-only reference to a file
pub fn get_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().append(false).write(true).open(path)
}

/// Get a read-write reference to a File on disk
pub fn get_read_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new()
        .append(false)
        .read(true)
        .write(true)
        .open(path)
}
