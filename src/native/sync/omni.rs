#[cfg(feature = "cli")]
use crate::cli::specifiers::DriveSpecifier;
use crate::{
    api::{
        client::Client,
        models::bucket::{Bucket as RemoteBucket, BucketType, StorageClass},
    },
    native::{
        configuration::globalconfig::GlobalConfig,
        sync::{determine_sync_state, LocalBucket, SyncState},
        NativeError,
    },
};
use colored::{ColoredString, Colorize};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};
use tomb_crypt::prelude::{PrivateKey, PublicKey};
use uuid::Uuid;

/// Struct for representing the ambiguity between local and remote copies of a Bucket
#[derive(Debug, Clone)]
pub struct OmniBucket {
    /// The local Bucket
    local: Option<LocalBucket>,
    /// The remote Bucket
    remote: Option<RemoteBucket>,
    /// The sync state
    pub sync_state: SyncState,
}

impl OmniBucket {
    /// Use local and remote to find
    #[cfg(feature = "cli")]
    pub async fn from_specifier(
        global: &GlobalConfig,
        client: &mut Client,
        drive_specifier: &DriveSpecifier,
    ) -> Self {
        let mut omni = Self {
            local: None,
            remote: None,
            sync_state: SyncState::Unknown,
        };

        // Search for a local bucket
        let local_result = global.buckets.clone().into_iter().find(|bucket| {
            let check_remote = bucket.remote_id == drive_specifier.drive_id;
            let check_origin = Some(bucket.origin.clone()) == drive_specifier.origin;
            let check_name = Some(bucket.name.clone()) == drive_specifier.name;
            check_remote || check_origin || check_name
        });
        omni.local = local_result;

        // Search for a remote bucket
        let all_remote_buckets = RemoteBucket::read_all(client).await.unwrap_or(Vec::new());
        let remote_result = all_remote_buckets.into_iter().find(|bucket| {
            let check_id = Some(bucket.id) == drive_specifier.drive_id;
            let check_name = Some(bucket.name.clone()) == drive_specifier.name;
            check_id || check_name
        });
        omni.remote = remote_result;

        if omni.local.is_some() && omni.remote.is_some() {
            let mut local = omni.get_local().unwrap();
            local.remote_id = Some(omni.get_remote().unwrap().id);
            omni.local = Some(local);
        }

        // Determine the sync state
        let _ = determine_sync_state(&mut omni, client).await;

        omni
    }

    /// Initialize w/ local
    pub fn from_local(bucket: &LocalBucket) -> Self {
        Self {
            local: Some(bucket.clone()),
            remote: None,
            sync_state: SyncState::Unpublished,
        }
    }

    /// Initialize w/ remote
    pub fn from_remote(bucket: &RemoteBucket) -> Self {
        Self {
            local: None,
            remote: Some(bucket.clone()),
            sync_state: SyncState::Unlocalized,
        }
    }

    /// Get the ID from wherever it might be found
    pub fn get_id(&self) -> Result<Uuid, NativeError> {
        if let Some(remote) = self.remote.clone() {
            Ok(remote.id)
        } else if let Some(local) = self.local.clone() {
            local.remote_id.ok_or(NativeError::missing_identifier())
        } else {
            Err(NativeError::missing_identifier())
        }
    }

    /// Get the local config
    pub fn get_local(&self) -> Result<LocalBucket, NativeError> {
        self.local.clone().ok_or(NativeError::missing_local_drive())
    }

    /// Get the remote config
    pub fn get_remote(&self) -> Result<RemoteBucket, NativeError> {
        self.remote
            .clone()
            .ok_or(NativeError::missing_remote_drive())
    }

    /// Update the LocalBucket
    pub fn set_local(&mut self, local: LocalBucket) {
        self.local = Some(local);
    }

    /// Update the RemoteBucket
    pub fn set_remote(&mut self, remote: RemoteBucket) {
        self.remote = Some(remote);
    }

    /// Create a new bucket
    pub async fn create(
        global: &mut GlobalConfig,
        client: &mut Client,
        name: &str,
        origin: &Path,
    ) -> Result<OmniBucket, NativeError> {
        let mut omni = OmniBucket {
            local: None,
            remote: None,
            sync_state: SyncState::Unknown,
        };
        // If this bucket already exists both locally and remotely
        if let Some(bucket) = global.get_bucket(origin) {
            if let Some(remote_id) = bucket.remote_id {
                if RemoteBucket::read(client, remote_id).await.is_ok() {
                    // Prevent the user from re-creating it
                    return Err(NativeError::unique_error());
                }
            }
        }

        // Grab the wrapping key, public key and pem
        let wrapping_key = global.wrapping_key().await?;
        let public_key = wrapping_key.public_key()?;
        let pem = String::from_utf8(public_key.export().await?)?;

        // Initialize remotely
        if let Ok((remote, _)) = RemoteBucket::create(
            name.to_string(),
            pem,
            BucketType::Interactive,
            StorageClass::Hot,
            client,
        )
        .await
        {
            // Update in obj
            omni.set_remote(remote);
        }

        // Initialize locally
        if let Ok(mut local) = global.get_or_init_bucket(name, origin).await {
            // If a remote bucket was made successfully
            if let Ok(remote) = omni.get_remote() {
                // Also save that in the local obj
                local.remote_id = Some(remote.id);
            }
            // Update in global and obj
            global.update_config(&local.clone())?;
            global.to_disk()?;
            omni.local = Some(local);
        }

        Ok(omni)
    }

    /// Delete an individual Bucket
    pub async fn delete(
        &self,
        global: &mut GlobalConfig,
        client: &mut Client,
        local_deletion: bool,
        mut remote_deletion: bool,
    ) -> Result<String, NativeError> {
        if local_deletion {
            if let Ok(local) = self.get_local() {
                local.remove_data()?;
                // Find index of bucket
                let index = global
                    .buckets
                    .iter()
                    .position(|b| b == &local)
                    .ok_or(NativeError::missing_local_drive())?;
                // Remove bucket config from global config
                global.buckets.remove(index);
            }
        }

        if remote_deletion {
            if let Ok(remote) = self.get_remote() {
                remote_deletion = RemoteBucket::delete_by_id(client, remote.id).await.is_ok();
            }
        }

        Ok(format!(
            "{}\ndeleted locally:\t{}\ndeleted remotely:\t{}",
            "<< BUCKET DELETION >>".blue(),
            bool_colorized(local_deletion),
            bool_colorized(remote_deletion)
        ))
    }

    /// List all available Buckets
    pub async fn ls(global: &GlobalConfig, client: &mut Client) -> Vec<OmniBucket> {
        let local_buckets = &global.buckets;
        let remote_buckets = match RemoteBucket::read_all(client).await {
            Ok(buckets) => buckets,
            Err(_) => {
                error!(
                    "{}",
                    "Unable to fetch remote Buckets. Check your authentication!".red()
                );
                <Vec<RemoteBucket>>::new()
            }
        };

        let mut map: HashMap<Option<Uuid>, OmniBucket> = HashMap::new();

        for local in local_buckets {
            map.insert(local.remote_id, OmniBucket::from_local(local));
        }

        for remote in remote_buckets {
            let key = Some(remote.id);
            if let Some(omni) = map.get(&key) {
                let mut omni = OmniBucket {
                    local: omni.local.clone(),
                    remote: Some(remote),
                    sync_state: SyncState::Unknown,
                };

                let _ = determine_sync_state(&mut omni, client).await;

                map.insert(key, omni);
            } else {
                map.insert(key, OmniBucket::from_remote(&remote));
            }
        }

        let omnis: Vec<OmniBucket> = map.into_values().collect();
        omnis
    }

    /// Get the origin for this bucket or create one in the default tomb directory if a local bucket does not yet exist
    pub async fn get_or_init_origin(
        &mut self,
        global: &mut GlobalConfig,
    ) -> Result<PathBuf, NativeError> {
        if let Ok(local) = self.get_local() {
            Ok(local.origin)
        } else {
            let new_local_origin = PathBuf::from(env!("HOME"))
                .join("tomb")
                .join(self.get_remote()?.name);
            // Remove existing contents and create a enw directory
            remove_dir_all(&new_local_origin).ok();
            create_dir_all(&new_local_origin)?;

            // Create a new local bucket
            self.set_local({
                let mut value = global
                    .get_or_init_bucket(&self.get_remote()?.name, &new_local_origin)
                    .await?;
                value.remote_id = Some(self.get_remote()?.id);
                value
            });

            Ok(new_local_origin)
        }
    }
}

#[inline]
fn bool_colorized(value: bool) -> ColoredString {
    if value {
        "Yes".green()
    } else {
        "No".red()
    }
}

impl Display for OmniBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut info = format!(
            "{}\nlocally tracked:\t{}\nremotely tracked:\t{}",
            "| DRIVE INFO |".yellow(),
            bool_colorized(self.local.is_some()),
            bool_colorized(self.remote.is_some()),
        );

        match (self.get_local(), self.get_remote()) {
            (Ok(local), Ok(remote)) => {
                info = format!(
                    "{info}\nname:\t\t\t{}\ndrive_id:\t\t{}\norigin:\t\t\t{}\ntype:\t\t\t{}\nstorage_class:\t\t{}\nstorage_ticket:\t\t{}",
                    remote.name,
                    remote.id,
                    local.origin.display(),
                    remote.r#type,
                    remote.storage_class,
                    if let Some(storage_ticket) = local.storage_ticket.clone() {
                        storage_ticket.host
                    } else {
                        format!("{}", "None".yellow())
                    }
                );
            }
            (Ok(local), Err(_)) => {
                info = format!("{info}\n{}", local);
            }
            (Err(_), Ok(remote)) => {
                info = format!("{info}\n{}", remote);
            }
            (Err(_), Err(_)) => {}
        }

        f.write_fmt(format_args!(
            "{info}\nsync_status:\t\t{}\n",
            self.sync_state
        ))
    }
}
