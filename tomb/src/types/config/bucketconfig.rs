use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    rc::Rc,
};
use tomb_common::{
    types::{blockstore::tombblockstore::TombBlockStore, keys::manager::Manager},
    utils::serialize::*,
};
use tomb_crypt::prelude::EcEncryptionKey;
use wnfs::{
    libipld::Cid,
    private::{PrivateDirectory, PrivateForest, PrivateNodeOnPathHistory},
};

use crate::{
    types::blockstore::{carv2, multi},
    utils::config::xdg_data_home,
};

/// Configuration for an individual Bucket / FileSystem
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BucketConfig {
    /// The name of this bucket
    bucket_name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds packed content and key files
    pub(crate) generated: PathBuf,
    /// BlockStore for storing metadata only
    pub metadata: carv2::BlockStore,
    /// BlockStore for storing metadata and file content
    pub content: multi::BlockStore,
}

impl BucketConfig {
    /// Given a directory, initialize a configuration for it
    pub fn new(origin: &Path) -> Result<Self> {
        let bucket_name = origin.file_name().unwrap().to_str().unwrap().to_string();
        // Generate a name for the generated directory
        let generated_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        // Compose the generated directory
        let generated = xdg_data_home().join(generated_name);

        // TODO (organized grime) prevent collision
        create_dir_all(&generated)?;

        let metadata = carv2::BlockStore::new(&generated.join("meta.car"))?;
        let content = multi::BlockStore::new(&generated.join("content"))?;

        // Start with default roots such that we never have to shift blocks
        metadata.set_root(&Cid::default());
        content.set_root(&Cid::default());

        Ok(Self {
            bucket_name,
            origin: origin.to_path_buf(),
            generated,
            metadata,
            content,
        })
    }

    pub(crate) fn remove_data(&self) -> Result<()> {
        // Remove dir if it exists
        if self.generated.exists() {
            remove_dir_all(&self.generated)?;
        }
        Ok(())
    }

    /// Shortcut for serialize::load_all
    pub async fn get_all(
        &self,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<(
        Rc<PrivateForest>,
        Rc<PrivateForest>,
        Rc<PrivateDirectory>,
        Manager,
        Cid,
    )> {
        // Load all
        load_all(wrapping_key, &self.metadata).await
    }

    /// Shortcut for serialize::store_all
    pub async fn set_all(
        &self,
        metadata_forest: &mut Rc<PrivateForest>,
        content_forest: &mut Rc<PrivateForest>,
        root_dir: &mut Rc<PrivateDirectory>,
        manager: &mut Manager,
        manager_cid: &Cid,
    ) -> Result<()> {
        store_all(
            &self.metadata,
            &self.content,
            metadata_forest,
            content_forest,
            root_dir,
            manager,
            manager_cid,
        )
        .await
    }

    /// Shortcut for serialize::load_history
    pub async fn get_history(
        &self,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<PrivateNodeOnPathHistory> {
        load_history(wrapping_key, &self.metadata).await
    }
}


#[cfg(test)]
mod test {
    use std::{fs::{create_dir_all, remove_dir_all}, path::Path, rc::Rc};

    use anyhow::Result;
    use chrono::Utc;
    use rand::thread_rng;
    use tomb_common::{types::keys::manager::Manager, utils::serialize::*};
    use tomb_crypt::prelude::WrappingPrivateKey;
    use wnfs::{private::{PrivateForest, PrivateDirectory}, namefilter::Namefilter};

    use crate::{types::config::globalconfig::GlobalConfig, utils::test::setup_v2};

    #[tokio::test]
    async fn set_get_all() -> Result<()> {
        let test_name = "config_set_get_all";
        let origin = &Path::new("test").join(test_name);
        if origin.exists() {
            remove_dir_all(origin)?;
        }
        create_dir_all(origin)?;

        let mut global = GlobalConfig::from_disk().await?;
        let config = global.find_or_create_config(origin)?;
        // let (metadata, content, metadata_forest, content_forest, root_dir) = setup_v2(test_name).await?;
        // config.metadata = metadata;
        // config.content = content;



        let mut manager = Manager::default();
        let wrapping_key = global.load_key().await?;
        let public_key = wrapping_key.public_key()?;
        manager.insert(&public_key).await?;
        let manager_cid = store_manager(&manager, &config.metadata, &config.content).await?;

        // let mut root_dir = Rc::new(PrivateDirectory::new(
        //     Namefilter::default(),
        //     Utc::now(),
        //     &mut thread_rng(),
        // ));

        // config.set_all(&mut Rc::new(PrivateForest::new()), &mut Rc::new(PrivateForest::new()), &mut root_dir, &mut manager, &manager_cid).await?;

        // // Get structs
        // let (metadata_forest, content_forest, root_dir, manager, manager_cid) =
        //     &mut config.get_all(&wrapping_key).await?;

        Ok(())
    }

}