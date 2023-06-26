use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use wnfs::{
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest},
};

use crate::types::config::{bucketconfig::BucketConfig, globalconfig::GlobalConfig};

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
    let mut global = GlobalConfig::from_disk()?;
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
