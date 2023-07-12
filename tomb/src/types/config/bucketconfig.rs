use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    rc::Rc,
};
use tomb_common::{
    crypto::rsa::RsaPrivateKey,
    types::{blockstore::tombblockstore::TombBlockStore, keys::manager::Manager},
    utils::serialize::*,
};
use wnfs::{
    libipld::Cid,
    private::{PrivateDirectory, PrivateForest, PrivateNodeOnPathHistory},
};

use crate::{
    types::blockstore::carv2::{blockstore::BlockStore, multifile::MultifileBlockStore},
    utils::config::xdg_data_home,
};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BucketConfig {
    /// The name of this bucket
    bucket_name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds packed content and key files
    pub(crate) generated: PathBuf,
    /// BlockStore for storing all
    pub metadata: BlockStore,
    pub content: MultifileBlockStore,
}

impl BucketConfig {
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

        let metadata = BlockStore::new(&generated.join("meta.car"))?;
        let content = MultifileBlockStore::new(&generated.join("content"))?;

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

    pub async fn get_all(
        &self,
        wrapping_key: &RsaPrivateKey,
    ) -> Result<(
        Rc<PrivateForest>,
        Rc<PrivateForest>,
        Rc<PrivateDirectory>,
        Manager,
    )> {
        // Load all
        load_all(wrapping_key, &self.metadata).await
    }

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

    pub async fn get_history(
        &self,
        wrapping_key: &RsaPrivateKey,
    ) -> Result<PrivateNodeOnPathHistory> {
        load_history(wrapping_key, &self.metadata).await
    }
}
