use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt::Display};
use uuid::Uuid;
use wnfs::libipld::Cid;

use crate::prelude::api::requests::core::buckets::metadata::read::ReadMetadataResponse;

use {
    crate::api::{
        client::Client,
        error::ApiError,
        requests::core::buckets::{
            metadata::{
                pull::PullMetadata,
                push::PushMetadata,
                read::{ReadAllMetadata, ReadCurrentMetadata, ReadMetadata},
            },
            snapshots::create::CreateSnapshot,
        },
    },
    bytes::Bytes,
    futures_core::stream::Stream,
};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
/// Possible states of Metadata
pub enum MetadataState {
    /// The metadata is being uploaded / pushed
    Uploading,
    /// The metadata upload failed
    UploadFailed,
    /// The metadata is pending being made current
    Pending,
    /// The metadata is current
    Current,
    /// The metadata is outdated
    Outdated,
    /// The metadata is deleted
    Deleted,
}
impl Display for MetadataState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}",
            match self {
                MetadataState::Uploading => "Uploading".to_string(),
                MetadataState::UploadFailed => format!("{}", "Upload Failed".red()),
                MetadataState::Pending => "Pending".to_string(),
                MetadataState::Current => format!("{}", "Current".green()),
                MetadataState::Outdated => format!("{}", "Outdated".red()),
                MetadataState::Deleted => format!("{}", "Deleted".red()),
            }
        ))
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
/// Bucket Metadata Definition
pub struct Metadata {
    /// The unique identifier for the bucket metadata
    pub id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub bucket_id: Uuid,
    /// The CID of the Content CAR root
    pub root_cid: String,
    /// The CID of the Metadata CAR root
    pub metadata_cid: String,
    /// The previous CID of the Metadata CAR root
    pub previous_cid: Option<String>,
    /// The size of the data in bytes that this metadata points to
    pub data_size: u64,
    /// The state of the metadata
    pub state: MetadataState,
    /// The snapshot id of the metadata (if any)
    pub snapshot_id: Option<Uuid>,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}\nmetadata_id:\t{}\nroot_cid:\t{}\ndata_size:\t{}\nstatus:\t\t{}",
            "| METADATA INFO |".yellow(),
            self.id,
            self.root_cid,
            self.data_size,
            self.state
        ))
    }
}

impl Metadata {
    // TODO: This should probably take a generic trait related to Tomb in order to restore these arguments
    /// Push new Metadata for a bucket. Creates a new metadata records and returns a storage ticket
    pub async fn push(
        push_metadata: PushMetadata,
        client: &mut Client,
    ) -> Result<(Self, Option<String>, Option<String>), ApiError> {
        let mut metadata = Self {
            id: Uuid::default(),
            bucket_id: push_metadata.bucket_id,
            root_cid: push_metadata.root_cid.clone(),
            metadata_cid: push_metadata.metadata_cid.clone(),
            previous_cid: push_metadata.previous_cid.clone(),
            data_size: 0,
            state: MetadataState::UploadFailed,
            snapshot_id: None,
        };

        let response = client.multipart(push_metadata).await?;
        metadata.id = response.id;
        metadata.state = response.state;

        Ok((
            metadata,
            response.storage_host,
            response.storage_authorization,
        ))
    }

    /// Pull the metadata file for the bucket metadata
    pub async fn pull(
        &self,
        client: &mut Client,
    ) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ApiError> {
        let base_url = client.remote_core.clone();
        client
            .stream(
                PullMetadata {
                    bucket_id: self.bucket_id,
                    id: self.id,
                },
                &base_url,
            )
            .await
    }

    /// Read the a specific metadata of a bucket
    pub async fn read(bucket_id: Uuid, id: Uuid, client: &mut Client) -> Result<Self, ApiError> {
        let response = client.call(ReadMetadata { bucket_id, id }).await?;
        Ok(Self::from_read_response(bucket_id, response))
    }

    /// Read all the metadata for a bucket
    pub async fn read_all(bucket_id: Uuid, client: &mut Client) -> Result<Vec<Self>, ApiError> {
        let response = client.call(ReadAllMetadata { bucket_id }).await?;
        Ok(response
            .0
            .into_iter()
            .map(|response| Self::from_read_response(bucket_id, response))
            .collect())
    }

    /// Read the current metadata for a bucket
    pub async fn read_current(bucket_id: Uuid, client: &mut Client) -> Result<Self, ApiError> {
        let response = client.call(ReadCurrentMetadata { bucket_id }).await?;
        Ok(Self::from_read_response(bucket_id, response))
    }

    /// Snapshot the current metadata
    pub async fn snapshot(
        &self,
        active_cids: BTreeSet<Cid>,
        client: &mut Client,
    ) -> Result<Uuid, ApiError> {
        let snapshot_resp = client
            .call(CreateSnapshot {
                bucket_id: self.bucket_id,
                metadata_id: self.id,
                active_cids,
            })
            .await?;

        Ok(snapshot_resp.id)
    }

    /// Given a bucket id and a ReadMetadataResponse, create a new Metadata object
    fn from_read_response(bucket_id: Uuid, response: ReadMetadataResponse) -> Self {
        Self {
            id: response.id,
            bucket_id,
            root_cid: response.root_cid,
            metadata_cid: response.metadata_cid,
            previous_cid: response.previous_cid,
            data_size: response.data_size as u64,
            state: response.state,
            snapshot_id: response.snapshot_id,
        }
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
pub(crate) mod test {
    use futures_util::stream::StreamExt;
    use reqwest::Body;
    use serial_test::serial;
    use std::{collections::BTreeSet, time::Duration};
    use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey};
    use uuid::Uuid;
    use wnfs::{libipld::Cid, private::PrivateNode};

    use crate::{
        api::{
            client::Client,
            error::ApiError,
            models::{
                account::test::authenticated_client,
                bucket::{Bucket, BucketType, StorageClass},
                metadata::{Metadata, MetadataState},
                storage_ticket::StorageTicket,
            },
        },
        blockstore::{CarV2MemoryBlockStore, RootedBlockStore},
        filesystem::FsMetadata,
        prelude::api::requests::{
            core::buckets::{metadata::push::PushMetadata, snapshots::read::ReadSingleSnapshot},
            staging::upload::content::UploadContent,
        },
    };

    pub async fn push_empty_metadata(
        bucket_id: Uuid,
        client: &mut Client,
    ) -> Result<(Metadata, Option<String>, Option<String>), ApiError> {
        let (metadata, host, authorization) = Metadata::push(
            PushMetadata {
                bucket_id,
                expected_data_size: 0,
                root_cid: String::from("root_cid"),
                metadata_cid: String::from("metadata_cid"),
                previous_cid: None,
                valid_keys: vec![],
                deleted_block_cids: BTreeSet::new(),
                metadata_stream: Body::from("metadata_stream".as_bytes()),
            },
            client,
        )
        .await?;
        Ok((metadata, host, authorization))
    }

    pub async fn push_metadata_and_snapshot(
        bucket_id: Uuid,
        client: &mut Client,
    ) -> Result<(Metadata, Option<String>, Option<String>, Uuid), ApiError> {
        let (metadata, host, authorization) = push_empty_metadata(bucket_id, client).await?;
        let snapshot_id = metadata.snapshot(BTreeSet::new(), client).await?;
        Ok((metadata, host, authorization, snapshot_id))
    }

    /// Simple struct for hosting the variables generated by this advanced test
    #[derive(Debug)]
    pub struct AdvancedTestSetup {
        pub client: Client,
        pub fs: FsMetadata,
        pub metadata_store: CarV2MemoryBlockStore,
        pub content_store: CarV2MemoryBlockStore,
        pub bucket: Bucket,
        pub metadata: Metadata,
        pub storage_ticket: StorageTicket,
    }

    // Helper function to set up an environment with a small amount of delta data and push the metadata associated
    pub async fn setup_and_push_metadata(test_name: &str) -> Result<AdvancedTestSetup, ApiError> {
        let mut client = authenticated_client().await;
        // let bucket_key = client.signing_key.unwrap();
        let wrapping_key = EcEncryptionKey::generate().await?;
        let public_key = wrapping_key.public_key()?;
        let initial_bucket_key_pem =
            String::from_utf8(public_key.export().await?).expect("could not utf8");
        // Create the bucket remotely
        let (bucket, _) = Bucket::create(
            test_name.to_owned(),
            initial_bucket_key_pem,
            BucketType::Interactive,
            StorageClass::Warm,
            &mut client,
        )
        .await?;
        // Create stores and filesystem
        let metadata_store = CarV2MemoryBlockStore::new()?;
        let content_store = CarV2MemoryBlockStore::new()?;
        let mut fs = FsMetadata::init(&wrapping_key).await?;
        // Write a file to that bucket
        fs.write(
            &["cat.txt".to_string()],
            &metadata_store,
            &content_store,
            b"cat content".to_vec(),
        )
        .await?;
        // Write a file to that bucket
        fs.write(
            &["dog.txt".to_string()],
            &metadata_store,
            &content_store,
            b"dog content".to_vec(),
        )
        .await?;
        // Save
        fs.save(&metadata_store, &content_store).await?;

        // Push metadata
        let (metadata, host, authorization) = Metadata::push(
            PushMetadata {
                bucket_id: bucket.id,
                expected_data_size: content_store.data_size(),
                root_cid: content_store
                    .get_root()
                    .ok_or(ApiError::missing_data("root_cid"))?
                    .to_string(),
                metadata_cid: metadata_store
                    .get_root()
                    .ok_or(ApiError::missing_data("metadata_cid"))?
                    .to_string(),
                previous_cid: None,
                valid_keys: fs.share_manager.public_fingerprints(),
                deleted_block_cids: BTreeSet::new(),
                metadata_stream: content_store.get_data().into(),
            },
            &mut client,
        )
        .await?;

        let storage_ticket = StorageTicket {
            host: host.unwrap(),
            authorization: authorization.unwrap(),
        };

        Ok(AdvancedTestSetup {
            client,
            fs,
            metadata_store,
            content_store,
            bucket,
            metadata,
            storage_ticket,
        })
    }

    // TODO: this test fails if not serial. This should be fixed
    #[tokio::test]
    #[serial]
    async fn push_and_pull() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("push_read_pull").await?;
        assert_eq!(setup.metadata.bucket_id, setup.bucket.id);
        assert_eq!(
            setup.metadata.root_cid,
            setup.metadata_store.get_root().unwrap().to_string()
        );
        assert_eq!(setup.metadata.data_size, 0);
        assert_eq!(setup.metadata.state, MetadataState::Pending);

        let read_metadata =
            Metadata::read(setup.bucket.id, setup.metadata.id, &mut setup.client).await?;

        let mut stream = read_metadata.pull(&mut setup.client).await?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn push_and_pull_and_snapshot() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("push_read_pull_snapshot").await?;
        // Create a grant and upload content
        setup
            .storage_ticket
            .clone()
            .create_grant(&mut setup.client)
            .await?;
        setup
            .content_store
            .upload(
                setup.storage_ticket.host.clone(),
                setup.metadata.id,
                &mut setup.client,
            )
            .await?;

        std::thread::sleep(Duration::new(3, 0));

        let mut active_cids = BTreeSet::<Cid>::new();
        let all_nodes = setup.fs.get_all_nodes(&setup.metadata_store).await?;
        for (node, _) in all_nodes {
            if let PrivateNode::File(file) = node {
                let cids = file
                    .get_cids(&setup.fs.forest, &setup.metadata_store)
                    .await
                    .unwrap();
                active_cids.extend(cids);
            }
        }
        let snapshot_id = setup
            .metadata
            .snapshot(active_cids, &mut setup.client)
            .await?;
        let _snapshot = setup
            .client
            .call(ReadSingleSnapshot {
                bucket_id: setup.bucket.id,
                snapshot_id,
            })
            .await?;
        Ok(())
    }
}
