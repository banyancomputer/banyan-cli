use anyhow::Result;
use chrono::Utc;
use libipld::Cid;
use rand::thread_rng;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    rc::Rc,
};
use wnfs::private::{
    forest::{hamt::HamtForest, traits::PrivateForest},
    AccessKey, PrivateDirectory,
};

use crate::blockstore::memory::MemoryBlockStore;
use crate::traits::blockstore::RootedBlockStore;

/// Macro for testing streamable implementations
pub mod streamable;
// TODO: Is anything using this?
/// Macro for testing whether a type can be serialized into DagCbor
pub mod serialize;

// Allows us to use this macro within this crate
#[allow(unused_imports)]
pub(crate) use streamable::streamable_tests;

/// Create a copy of a given fixture to play around with
pub fn car_test_setup(
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
pub fn car_index_test_setup(
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

/// Setup using a MemoryBlockStore
pub async fn setup_memory_test(
    test_name: &str,
) -> Result<(
    MemoryBlockStore,
    MemoryBlockStore,
    Rc<HamtForest>,
    Rc<HamtForest>,
    Rc<PrivateDirectory>,
)> {
    setup_test(test_name, MemoryBlockStore::new(), MemoryBlockStore::new()).await
}

/// Setup a key test
pub async fn setup_key_test(test_name: &str) -> Result<AccessKey> {
    let (metadata, _, metadata_forest, _, root_dir) =
        &mut setup_test(test_name, MemoryBlockStore::new(), MemoryBlockStore::new()).await?;
    let access_key = root_dir
        .as_node()
        .store(metadata_forest, metadata, &mut thread_rng())
        .await?;
    Ok(access_key)
}

/// Create all of the relevant objects, using real BlockStores and real data
pub async fn setup_test<RBS: RootedBlockStore>(
    test_name: &str,
    metadata: RBS,
    content: RBS,
) -> Result<(
    RBS,
    RBS,
    Rc<HamtForest>,
    Rc<HamtForest>,
    Rc<PrivateDirectory>,
)> {
    let rng = &mut thread_rng();
    let origin: PathBuf = Path::new("test").join(test_name);
    create_dir_all(&origin)?;

    metadata.set_root(&Cid::default());
    content.set_root(&Cid::default());

    // Hot Forest and cold Forest
    let mut metadata_forest = HamtForest::new_trusted_rc(rng);
    let mut content_forest = HamtForest::new_trusted_rc(rng);
    // PrivateDirectory
    let mut root_dir = PrivateDirectory::new_rc(&metadata_forest.empty_name(), Utc::now(), rng);

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

    Ok((metadata, content, metadata_forest, content_forest, root_dir))
}

/// Delete the temporary directory
pub async fn teardown_test(test_name: &str) -> Result<()> {
    let path = Path::new("test").join(test_name);
    std::fs::remove_dir_all(path)?;
    Ok(())
}
