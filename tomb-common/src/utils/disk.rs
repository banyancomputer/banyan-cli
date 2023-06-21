use crate::{
    types::{
        blockstore::car::carv2::carv2blockstore::CarV2BlockStore,
        config::{bucketconfig::BucketConfig, globalconfig::GlobalConfig},
    },
    utils::serialize::*,
};
use anyhow::Result;
use log::info;
use std::{
    io::{Read, Write},
    path::Path,
    rc::Rc,
};
use wnfs::private::{AesKey, PrivateDirectory, PrivateForest, TemporalKey};

// /// Store all hot objects!
// pub async fn hot_to_disk(
//     config: &BucketConfig,
//     metadata_forest: &mut Rc<PrivateForest>,
//     root_dir: &Rc<PrivateDirectory>,
// ) -> Result<TemporalKey> {
//     let temporal_key = store_all_hot(&config.metadata, metadata_forest, root_dir).await?;
//     Ok(temporal_key)
// }

// /// Load all hot objects!
// pub async fn hot_from_disk(
//     origin: &Path,
// ) -> Result<(
//     TemporalKey,
//     BucketConfig,
//     Rc<PrivateForest>,
//     Rc<PrivateDirectory>,
// )> {
//     let config = GlobalConfig::get_bucket(origin).unwrap();
//     let key = config.get_key("root").unwrap();
//     let (metadata_forest, dir) = load_all_hot(&key, &config.metadata).await?;
//     Ok((key, config, metadata_forest, dir))
// }

#[cfg(test)]
mod test {
    use crate::utils::{disk::*, tests::*};
    use anyhow::Result;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn disk_key() -> Result<()> {
        let test_name = "disk_key";
        // Start er up!
        let (tomb_path, global, config, metadata_forest, content_forest, dir) =
            &mut setup(test_name).await?;

        // Generate key for this directory
        let key = store_all(
            &config.metadata,
            &config.content,
            metadata_forest,
            content_forest,
            dir,
        )
        .await?;

        // Store and load
        config.set_key(&key, "root")?;
        let new_key = config.get_key("root").unwrap();

        // Assert equality
        assert_eq!(key, new_key);

        // Teardown
        teardown(test_name).await
    }

    /*

    #[tokio::test]
    #[serial]
    async fn disk_metadata() -> Result<()> {
        let test_name = "disk_metadata";
        // Setup
        let (origin, config, metadata_forest, _, root_dir) =
            &mut setup(test_name).await?;

        // Save to disk
        let key = &hot_to_disk(origin, config, metadata_forest, root_dir).await?;

        // Reload from disk
        let (new_key, _, new_metadata_forest, new_root_dir) =
            &mut hot_from_disk(&tomb_path).await?;

        // Assert equality
        assert_eq!(key, new_key);
        assert_eq!(
            metadata_forest
                .diff(new_metadata_forest, metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(root_dir, new_root_dir);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn disk_content() -> Result<()> {
        let test_name = "disk_content";
        // Setup
        let (origin, metadata, content, metadata_forest, content_forest, root_dir) =
            &mut setup(test_name).await?;

        let config = GlobalConfig::get_bucket(&origin).unwrap();
        // Save to disk
        let key = &mut all_to_disk(
            &config,
            metadata_forest,
            content_forest,
            root_dir,
        )
        .await?;
        // Reload from disk
        let (
            new_key,
            _,
            _,
            new_metadata_forest,
            new_content_forest,
            new_root_dir,
        ) = &mut all_from_disk(&origin).await?;

        // Assert equality
        assert_eq!(key, new_key);
        // assert_eq!(manifest, new_manifest);
        assert_eq!(
            metadata_forest
                .diff(new_metadata_forest, metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            content_forest
                .diff(new_content_forest, content)
                .await?
                .len(),
            0
        );
        assert_eq!(root_dir, new_root_dir);

        // Teardown
        teardown(test_name).await
    }



    /// Helper function, not a test
    async fn assert_serial_all_cold(local: bool) -> Result<()> {
        let test_name: &String = &format!("serial_all_cold_{}", local);
        // Start er up!
        let (tomb_path, mut manifest, mut metadata_forest, mut content_forest, dir) =
            setup(test_name).await?;

        // Store and load
        let key = all_to_disk(
            &tomb_path,
            &mut manifest,
            &mut metadata_forest,
            &mut content_forest,
            &dir
        )
        .await?;
        let (new_key, new_manifest, new_metadata_forest, new_content_forest, new_dir) =
            all_from_disk(&tomb_path).await?;

        // Assert equality
        assert_eq!(new_key, key);
        assert_eq!(new_manifest, manifest);
        assert_eq!(
            new_metadata_forest
                .diff(&metadata_forest, &new_manifest.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            new_content_forest
                .diff(&content_forest, &new_manifest.content)
                .await?
                .len(),
            0
        );
        assert_eq!(new_dir, dir);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    async fn serial_all_content() -> Result<()> {
        assert_serial_all_cold(true).await
    }

    #[tokio::test]
    async fn serial_all_cold_remote() -> Result<()> {
        assert_serial_all_cold(false).await
    }
     */
}
