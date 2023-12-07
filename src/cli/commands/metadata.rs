use std::collections::BTreeSet;

use crate::{
    api::{client::Client, models::metadata::Metadata},
    native::{configuration::globalconfig::GlobalConfig, sync::OmniBucket, NativeError}, prelude::blockstore::RootedBlockStore, filesystem::FilesystemError,
};

use super::{
    super::specifiers::{DriveSpecifier, MetadataSpecifier},
    RunnableCommand,
};
use async_trait::async_trait;
use clap::Subcommand;
use wnfs::{libipld::Cid, private::PrivateNode};

/// Subcommand for Bucket Metadata
#[derive(Subcommand, Clone, Debug)]
pub enum MetadataCommand {
    /// List all Metadatas associated with Bucket
    Ls(DriveSpecifier),
    /// Read an individual Metadata Id
    Read(MetadataSpecifier),
    /// Read the currently active Metadata
    ReadCurrent(DriveSpecifier),
    /// Grab Snapshot
    Snapshot(MetadataSpecifier),
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for MetadataCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, NativeError> {
        match self {
            // List all Metadata for a Bucket
            MetadataCommand::Ls(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let bucket_id = omni.get_id()?;
                Metadata::read_all(bucket_id, client)
                    .await
                    .map(|metadatas| {
                        metadatas.iter().fold(String::from("\n"), |acc, metadata| {
                            format!("{}\n\n{}", acc, metadata)
                        })
                    })
                    .map_err(NativeError::api)
            }
            // Read an existing metadata
            MetadataCommand::Read(metadata_specifier) => {
                // Get Bucket config
                let omni =
                    OmniBucket::from_specifier(global, client, &metadata_specifier.drive_specifier)
                        .await;
                // If we can get the metadata
                let remote_id = omni.get_id()?;
                Metadata::read(remote_id, metadata_specifier.metadata_id, client)
                    .await
                    .map(|metadata| format!("{:?}", metadata))
                    .map_err(NativeError::api)
            }
            // Read the current Metadata
            MetadataCommand::ReadCurrent(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let bucket_id = omni.get_id()?;
                Metadata::read_current(bucket_id, client)
                    .await
                    .map(|metadata| format!("{:?}", metadata))
                    .map_err(NativeError::api)
            }
            // Take a Cold Snapshot of the remote metadata
            MetadataCommand::Snapshot(metadata_specifier) => {
                let omni =
                    OmniBucket::from_specifier(global, client, &metadata_specifier.drive_specifier)
                        .await;

                let bucket_id = omni.get_id().expect("no remote id");
                let metadata =
                    Metadata::read(bucket_id, metadata_specifier.metadata_id, client).await?;

                // Grab the local filesystem
                let local = omni.get_local()?;
                
                // If the root of our currently stored metadata BlockStore doesn't actually match the metadata we're trying to snapshot
                if local.metadata.get_root().map(|cid| cid.to_string()) != Some(metadata.metadata_cid.clone()) {
                    return Err(NativeError::custom_error("this is the wrong metadata"));
                }

                // Finish loading the filesystem
                let wrapping_key = global.wrapping_key().await?;
                let fs = local.unlock_fs(&wrapping_key).await?;

                // Start off by considering all CIDs in the metatadata CAR as 'active'
                let index = local.metadata.car.car.index.borrow().clone();
                let mut active_cids = index.buckets[0].map.clone().into_keys().collect::<BTreeSet<Cid>>();

                // For every node that is a PrivateFile
                for (node, _) in fs.get_all_nodes(&local.metadata).await? {
                    if let PrivateNode::File(file) = node {
                        // Extend with all the cids in the file
                        active_cids.extend(file.get_cids(&fs.forest, &local.content).await.map_err(|err| FilesystemError::wnfs(Box::from(err)))?)
                    }
                }

                metadata
                    .snapshot(active_cids, client)
                    .await
                    .map(|snapshot| format!("{:?}", snapshot))
                    .map_err(NativeError::api)
            }
        }
    }
}
