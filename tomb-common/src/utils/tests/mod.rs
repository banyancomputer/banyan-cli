use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    rc::Rc,
};
use wnfs::{
    libipld::Cid,
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest},
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
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    setup_test(test_name, MemoryBlockStore::new(), MemoryBlockStore::new()).await
}

/// Create all of the relevant objects, using real BlockStores and real data
pub async fn setup_test<RBS: RootedBlockStore>(
    test_name: &str,
    metadata: RBS,
    content: RBS,
) -> Result<(
    RBS,
    RBS,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let origin: PathBuf = Path::new("test").join(test_name);
    create_dir_all(&origin)?;

    metadata.set_root(&Cid::default());
    content.set_root(&Cid::default());

    // Hot Forest and cold Forest
    let mut forest = Rc::new(PrivateForest::new());

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
            &mut forest,
            &metadata,
            rng,
        )
        .await?;

    // Set file content
    file.set_content(
        Utc::now(),
        "Hello Kitty!".as_bytes(),
        &mut forest,
        &content,
        rng,
    )
    .await?;

    Ok((metadata, content, forest, root_dir))
}

/// Delete the temporary directory
pub async fn teardown_test(test_name: &str) -> Result<()> {
    let path = Path::new("test").join(test_name);
    std::fs::remove_dir_all(path)?;
    Ok(())
}
