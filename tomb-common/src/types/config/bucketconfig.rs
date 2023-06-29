use crate::{
    types::blockstore::car::carv2::blockstore::BlockStore,
    utils::{config::*, serialize::*},
};
use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, remove_dir_all},
    io::{Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};
use wnfs::{
    libipld::Cid,
    private::{AesKey, PrivateDirectory, PrivateForest, TemporalKey},
};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BucketConfig {
    /// The name of this bucket
    bucket_name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds packed content and key files
    pub(crate) generated: PathBuf,
    /// roots: [
    ///     PrivateDirectory,
    ///     Metadata PrivateForest,
    ///     Content PrivateForest
    /// ]
    pub metadata: BlockStore,
    /// roots: [
    ///     Content PrivateForest
    /// ]
    pub content: BlockStore,
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
        let content = BlockStore::new(&generated.join("content.car"))?;

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
        remove_dir_all(&self.generated).ok();
        Ok(())
    }

    /*

    async fn get_existing_keys(&self) -> Result<Vec<(RsaPublicKey, Vec<u8>)>> {
        // Get the existing cid associated with our list of public keys
        let keys_cid = self.metadata.get_roots()[3];

        // Create new list
        let mut keys: Vec<(RsaPublicKey, Vec<u8>)> = Vec::new();

        // If we can grab a list of IPLDs from this cid
        if let Ipld::List(list) = self.metadata.get_deserializable::<Ipld>(&keys_cid).await? {
            // Iterate over them
            for ipld in list.into_iter() {
                if let Ipld::Map(map) = ipld {
                    if let Ipld::Bytes(public_bytes) = map.get("public_key").unwrap() &&
                        let Ipld::Bytes(encrypted_bytes) = map.get("encrypted_key").unwrap() {
                        // Construct RSA Public Keys using these bytes as the DER bytes
                        keys.push((RsaPublicKey::from_der(&public_bytes)?, encrypted_bytes.to_vec()));
                    }
                }
            }
        }

        Ok(keys)
    }

    async fn save_keys(&self, keys: &Vec<(RsaPublicKey, Vec<u8>)>) -> Result<()> {
        // Create the IPLD that we're going to serialize
        let ipld = Ipld::List(keys.iter().map(|(public_key, encrypted_key)| {
            let mut map = BTreeMap::new();
            map.insert("public_key".to_string(), Ipld::Bytes(public_key.to_der().unwrap()));
            map.insert("encrypted_key".to_string(), Ipld::Bytes(encrypted_key.to_vec()));
            Ipld::Map(map)
        }).collect());
        // New CID for the keys
        let new_keys_cid = self.metadata.put_serializable(&ipld).await?;
        // Overwrite the CID at this index
        self.metadata.carv2.carv1.header.roots.borrow_mut()[0] = new_keys_cid;
        // Return Ok
        Ok(())
    }


    async fn update_keys(&self, temporal_key: &TemporalKey) -> Result<()> {
        // Create new list
        let mut keys: Vec<(RsaPublicKey, Vec<u8>)> = Vec::new();
        // Grab existing list
        for (key, _) in self.get_existing_keys().await? {
            // Append this key and an encrypted copy of the temporal key to the new list
            keys.push((key.clone(), key.encrypt(temporal_key.0.as_bytes()).await?))
        }
        // Save these updated keys back into the Metadata CAR
        self.save_keys(&keys).await?;

        Ok(())
    }


    // /// Adds a public key to the permissioned list of public keys which are used to
    // pub async fn add_public_key(&self, new_key: &RsaPublicKey) -> Result<()> {
    //     // Grab the fingerprint of the file being added
    //     let new_fingerprint = new_key.get_sha1_fingerprint()?;
    //     // Grab existing public keys
    //     let mut keys = self.get_existing_public_keys().await?;

    //     // If the key does not already exist in the current list
    //     if keys.iter().find(|&key| key.get_sha1_fingerprint().unwrap() == new_fingerprint).is_none() {
    //         // Append the new keys to the list
    //         keys.push(new_key.clone());
    //         // Re-store them in the metadata CAR
    //         self.save_public_keys(&keys).await?;
    //     }

    //     Ok(())
    // }

    ///
    fn get_existing_temporal_keys() {

    }

    */

    /// Load a TemporalKey
    pub fn get_key(&self, label: &str) -> Result<TemporalKey> {
        // The path in which we expect to find the Manifest JSON file
        let key_file = self.generated.join(format!("{}.key", label));

        // Read in the key file from the key path
        match std::fs::File::open(key_file) {
            std::result::Result::Ok(mut key_reader) => {
                // Deserialize the data read as the latest version of manifestdata
                let mut key_data: [u8; 32] = [0; 32];
                key_reader.read_exact(&mut key_data).unwrap();
                let key: TemporalKey = TemporalKey(AesKey::new(key_data));
                Ok(key)
            }
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }

    /// Store a TemporalKey
    pub fn set_key(&self, temporal_key: &TemporalKey, label: &str) -> Result<()> {
        // The path in which we expect to find the Manifest JSON file
        let key_file = &self.generated.join(format!("{}.key", label));
        let mut key_writer = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(key_file)
            .expect("Failed to create key file");

        // Write the key
        key_writer.write_all(temporal_key.0.as_bytes())?;

        Ok(())
    }

    pub async fn get_all(
        &self,
    ) -> Result<(Rc<PrivateForest>, Rc<PrivateForest>, Rc<PrivateDirectory>)> {
        let key = self.get_key("root").unwrap();
        load_all(&key, &self.metadata, &self.content).await
    }

    pub async fn set_all(
        &self,
        metadata_forest: &mut Rc<PrivateForest>,
        content_forest: &mut Rc<PrivateForest>,
        root_dir: &Rc<PrivateDirectory>,
    ) -> Result<()> {
        let temporal_key = store_all(
            &self.metadata,
            &self.content,
            metadata_forest,
            content_forest,
            root_dir,
        )
        .await?;
        self.set_key(&temporal_key, "root")
    }
}
