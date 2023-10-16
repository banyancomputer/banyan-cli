use std::{collections::HashMap, fmt::Display, path::Path};

use super::LocalBucket;
use crate::{
    cli::{
        commands::{MetadataCommand, RunnableCommand},
        specifiers::BucketSpecifier,
    },
    pipelines::error::TombError,
    types::config::globalconfig::GlobalConfig,
};
use colored::{ColoredString, Colorize};
use tomb_common::{
    banyan_api::{
        client::Client,
        models::bucket::{Bucket as RemoteBucket, BucketType, StorageClass},
    },
    metadata::FsMetadata,
};
use tomb_crypt::prelude::{PrivateKey, PublicKey};
use uuid::Uuid;

///
#[derive(Debug, Clone)]
pub struct OmniBucket {
    pub local: Option<LocalBucket>,
    pub remote: Option<RemoteBucket>,
}

impl OmniBucket {
    /// Use local and remote to find
    pub async fn from_specifier(
        global: &GlobalConfig,
        client: &mut Client,
        bucket_specifier: &BucketSpecifier,
    ) -> Self {
        let mut new_object = Self {
            local: None,
            remote: None,
        };

        // Search for a local bucket
        let local_result = global.buckets.clone().into_iter().find(|bucket| {
            bucket.remote_id == bucket_specifier.bucket_id
                && (if let Some(origin) = &bucket_specifier.origin {
                    bucket.origin == *origin
                } else {
                    true
                })
                && (if let Some(name) = &bucket_specifier.name {
                    bucket.name == *name
                } else {
                    true
                })
        });

        // Search for a remote bucket id
        let mut remote_id = None;
        if let Some(id) = bucket_specifier.bucket_id {
            remote_id = Some(id);
        }
        if let Some(bucket) = local_result {
            if let Some(id) = bucket.remote_id {
                remote_id = Some(id);
            }
            new_object.local = Some(bucket);
        }
        // If we found one
        if let Some(remote_id) = remote_id && let Ok(bucket) = RemoteBucket::read(client, remote_id).await {
            new_object.remote = Some(bucket)
        }

        new_object
    }

    /// Initialize w/ local
    pub fn from_local(bucket: &LocalBucket) -> Self {
        Self {
            local: Some(bucket.clone()),
            remote: None,
        }
    }

    /// Initialize w/ remote
    pub fn from_remote(bucket: &RemoteBucket) -> Self {
        Self {
            local: None,
            remote: Some(bucket.clone()),
        }
    }

    /// Create a new bucket
    pub async fn create(
        global: &mut GlobalConfig,
        client: &mut Client,
        name: &str,
        origin: &Path,
    ) -> Result<OmniBucket, TombError> {
        let mut omni = OmniBucket {
            local: None,
            remote: None,
        };

        // If this bucket already exists both locally and remotely
        if let Some(bucket) = global.get_bucket_by_origin(origin) &&
            let Some(remote_id) = bucket.remote_id &&
            RemoteBucket::read(client, remote_id).await.is_ok() {
            // If we are able to read the bucket
            return Err(TombError::custom_error("Bucket already exists at this origin and is persisted remotely"));
        }

        // Initialize in the configs
        let mut local = global.get_or_create_bucket(name, origin).await?;
        global.to_disk()?;
        local.content.add_delta()?;
        omni.local = Some(local.clone());

        let wrapping_key = global.wrapping_key().await?;

        // Initialize and save metadata so that it can be pushed
        let mut metadata = FsMetadata::init(&wrapping_key).await?;
        metadata.save(&local.metadata, &local.content).await?;

        // Update the config globally
        global.update_config(&local)?;

        let public_key = wrapping_key.public_key()?;
        let pem = String::from_utf8(public_key.export().await?)
            .map_err(|_| TombError::custom_error("unable to represent pem from utf8"))?;

        // Initialize on the remote endpoint
        if let Ok((remote, _)) = RemoteBucket::create(
            name.to_string(),
            pem,
            BucketType::Interactive,
            StorageClass::Hot,
            client,
        )
        .await
        {
            // Update the bucket config id
            local.remote_id = Some(remote.id);
            omni.local = Some(local.clone());
            omni.remote = Some(remote.clone());
            // Update the config globally
            global.update_config(&local)?;

            // Attempt to push the first metadata
            let bucket_specifier = BucketSpecifier::with_id(remote.id);
            let command = MetadataCommand::Push(bucket_specifier);
            let metadata_result = command.run_internal(global, client).await;
            println!("metadata_result: {:?}", metadata_result);
        }

        Ok(omni)
    }

    /// Delete an individual Bucket
    pub async fn delete(
        &self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, TombError> {
        let local_deletion = if let Some(local) = &self.local {
            local.remove_data()?;
            // Find index of bucket
            let index = global
                .buckets
                .iter()
                .position(|b| b == local)
                .expect("cannot find index in buckets");
            // Remove bucket config from global config
            global.buckets.remove(index);
            true
        } else {
            false
        };

        let remote_deletion = if let Some(remote) = &self.remote {
            RemoteBucket::delete_by_id(client, remote.id).await.is_ok()
        } else {
            false
        };

        Ok(format!(
            "{}\nlocal:\t{}\nremote:\t{}",
            "<< BUCKET DELETION >>".blue(),
            bool_colorized(local_deletion),
            bool_colorized(remote_deletion)
        ))
    }

    /// List all available Buckets
    pub async fn ls(global: &GlobalConfig, client: &mut Client) -> Vec<OmniBucket> {
        let local_buckets = &global.buckets;
        let remote_buckets = RemoteBucket::read_all(client).await.unwrap_or(Vec::new());

        let mut map: HashMap<Option<Uuid>, OmniBucket> = HashMap::new();

        for local in local_buckets {
            map.insert(local.remote_id, OmniBucket::from_local(local));
        }

        for remote in remote_buckets {
            let key = Some(remote.id);
            if let Some(omni) = map.get(&key) {
                map.insert(
                    key,
                    OmniBucket {
                        local: omni.local.clone(),
                        remote: Some(remote),
                    },
                );
            } else {
                map.insert(key, OmniBucket::from_remote(&remote));
            }
        }

        let omnis: Vec<OmniBucket> = map.into_values().collect();
        omnis
    }
}

#[inline]
fn bool_colorized(value: bool) -> ColoredString {
    if value {
        "true".green()
    } else {
        "false".red()
    }
}

impl Display for OmniBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut info = format!(
            "{}\ntracked locally:  {}\ntracked remotely: {}",
            "| BUCKET INFO |".yellow(),
            bool_colorized(self.local.is_some()),
            bool_colorized(self.remote.is_some()),
        );

        // If we have both present
        if let Some(local) = &self.local && let Some(remote) = &self.remote {
            info = format!("{info}\nname:\t\t{}\norigin:\t\t{}\nremote_id:\t{}\ntype:\t{}\nstorage_class:\t{}",
                local.name,
                local.origin.display(),
                remote.id,
                remote.r#type,
                remote.storage_class
            );
        }
        else if let Some(local) = &self.local {
            info = format!("{info}\n{}", local);
        }
        else if let Some(remote) = &self.remote {
            info = format!("{info}\n{}", remote);
        }

        f.write_fmt(format_args!("{info}\n"))
    }
}
