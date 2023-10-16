use std::{collections::HashMap, fmt::Display};

use super::LocalBucket;
use crate::{
    cli::specifiers::BucketSpecifier, pipelines::error::TombError,
    types::config::globalconfig::GlobalConfig,
};
use colored::{ColoredString, Colorize};
use tomb_common::banyan_api::{client::Client, models::bucket::Bucket as RemoteBucket};
use uuid::Uuid;

///
#[derive(Debug, Clone)]
pub struct OmniBucket {
    local: Option<LocalBucket>,
    remote: Option<RemoteBucket>,
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
            "{}\nlocal:\t{local_deletion}\nremote:\t{remote_deletion}",
            "<< BUCKET DELETION >>".blue()
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
            info = format!("{info}\nname:\t{}\norigin:\t{}\nremote_id:\t{}\ntype:\t{}\nstorage_class:\t{}",
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
