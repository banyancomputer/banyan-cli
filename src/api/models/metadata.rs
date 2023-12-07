use colored::Colorize;
use serde::{Deserialize, Serialize};
use wnfs::libipld::Cid;
use std::collections::BTreeSet;
use std::fmt::Display;
use uuid::Uuid;

use {
    crate::api::{
        client::Client,
        error::ApiError,
        requests::core::buckets::{
            metadata::{
                pull::PullMetadata,
                push::{MetadataStreamType, PushMetadata},
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
    #[allow(clippy::too_many_arguments)]
    pub async fn push(
        bucket_id: Uuid,
        root_cid: String,
        metadata_cid: String,
        expected_data_size: u64,
        valid_keys: Vec<String>,
        deleted_block_cids: BTreeSet<String>,
        metadata_stream: MetadataStreamType,
        client: &mut Client,
    ) -> Result<(Self, Option<String>, Option<String>), ApiError> {
        let response = client
            .multipart(PushMetadata {
                bucket_id,
                root_cid: root_cid.clone(),
                metadata_cid: metadata_cid.clone(),
                expected_data_size,
                valid_keys,
                deleted_block_cids,
                metadata_stream,
            })
            .await?;
        let metadata = Self {
            id: response.id,
            bucket_id,
            root_cid,
            metadata_cid,
            data_size: 0,
            state: response.state,
            snapshot_id: None,
        };

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
        Ok(Self {
            id: response.id,
            bucket_id,
            root_cid: response.root_cid,
            metadata_cid: response.metadata_cid,
            data_size: response.data_size as u64,
            state: response.state,
            snapshot_id: response.snapshot_id,
        })
    }

    /// Read all the metadata for a bucket
    pub async fn read_all(bucket_id: Uuid, client: &mut Client) -> Result<Vec<Self>, ApiError> {
        let response = client.call(ReadAllMetadata { bucket_id }).await?;
        Ok(response
            .0
            .into_iter()
            .map(|response| Self {
                id: response.id,
                bucket_id,
                root_cid: response.root_cid,
                metadata_cid: response.metadata_cid,
                data_size: response.data_size as u64,
                state: response.state,
                snapshot_id: response.snapshot_id,
            })
            .collect())
    }

    /// Read the current metadata for a bucket
    pub async fn read_current(bucket_id: Uuid, client: &mut Client) -> Result<Self, ApiError> {
        let response = client.call(ReadCurrentMetadata { bucket_id }).await?;
        Ok(Self {
            id: response.id,
            bucket_id,
            root_cid: response.root_cid,
            metadata_cid: response.metadata_cid,
            data_size: response.data_size as u64,
            state: response.state,
            snapshot_id: response.snapshot_id,
        })
    }

    /// Snapshot the current metadata
    pub async fn snapshot(&self, active_cids: BTreeSet<Cid>, client: &mut Client) -> Result<Uuid, ApiError> {
        let snapshot_resp = client
            .call(CreateSnapshot {
                bucket_id: self.bucket_id,
                metadata_id: self.id,
                active_cids
            })
            .await?;

        Ok(snapshot_resp.id)
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
pub(crate) mod test {
    use futures_util::stream::StreamExt;
    use reqwest::Body;
    use serial_test::serial;
    use std::collections::BTreeSet;
    use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey};
    use uuid::Uuid;

    use crate::{
        api::{
            client::Client,
            error::ApiError,
            models::{
                account::test::authenticated_client,
                bucket::{test::create_bucket, Bucket, BucketType, StorageClass},
                metadata::{Metadata, MetadataState},
                storage_ticket::StorageTicket,
            },
        },
        blockstore::{CarV2MemoryBlockStore, RootedBlockStore},
        filesystem::FsMetadata,
    };

    pub async fn push_empty_metadata(
        bucket_id: Uuid,
        client: &mut Client,
    ) -> Result<(Metadata, Option<String>, Option<String>), ApiError> {
        let (metadata, host, authorization) = Metadata::push(
            bucket_id,
            "root_cid".to_string(),
            "metadata_cid".to_string(),
            0,
            vec![],
            BTreeSet::new(),
            Body::from("metadata_stream".as_bytes()),
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
            b"Example content".to_vec(),
        )
        .await?;
        // Save
        fs.save(&metadata_store, &content_store).await?;
        // Push metadata
        let (metadata, host, authorization) = Metadata::push(
            bucket.id,
            content_store.get_root().unwrap().to_string(),
            metadata_store.get_root().unwrap().to_string(),
            content_store.data_size(),
            fs.share_manager.public_fingerprints(),
            BTreeSet::new(),
            content_store.get_data().into(),
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
    async fn push_read_pull() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _host, _authorization) = push_empty_metadata(bucket.id, &mut client).await?;
        assert_eq!(metadata.bucket_id, bucket.id);
        assert_eq!(metadata.root_cid, "root_cid");
        assert_eq!(metadata.data_size, 0);
        assert_eq!(metadata.state, MetadataState::Current);

        let read_metadata = Metadata::read(bucket.id, metadata.id, &mut client).await?;
        assert_eq!(metadata, read_metadata);

        let mut stream = read_metadata.pull(&mut client).await?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(data, "metadata_stream".as_bytes());
        Ok(())
    }
    #[tokio::test]
    async fn push_read_unauthorized() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _host, _authorization) = push_empty_metadata(bucket.id, &mut client).await?;
        assert_eq!(metadata.bucket_id, bucket.id);

        let mut bad_client = authenticated_client().await;
        let read_metadata = Metadata::read(bucket.id, metadata.id, &mut bad_client).await;
        assert!(read_metadata.is_err());
        Ok(())
    }
    #[tokio::test]
    async fn push_read_wrong_bucket() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (other_bucket, _) = create_bucket(&mut client).await?;
        let (_metadata, _host, _authorization) =
            push_empty_metadata(bucket.id, &mut client).await?;
        let (other_metadata, _host, _authorization) =
            push_empty_metadata(other_bucket.id, &mut client).await?;
        let read_metadata = Metadata::read(bucket.id, other_metadata.id, &mut client).await;
        assert!(read_metadata.is_err());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn push_read_pull_snapshot() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _host, _authorization) = push_empty_metadata(bucket.id, &mut client).await?;
        assert_eq!(metadata.bucket_id, bucket.id);
        assert_eq!(metadata.root_cid, "root_cid");
        assert_eq!(metadata.data_size, 0);
        assert_eq!(metadata.state, MetadataState::Current);

        let read_metadata = Metadata::read(bucket.id, metadata.id, &mut client).await?;
        assert_eq!(metadata, read_metadata);

        let mut stream = read_metadata.pull(&mut client).await?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(data, "metadata_stream".as_bytes());

        let _snapshot_id = read_metadata.snapshot(BTreeSet::new(), &mut client).await?;
        //assert_eq!(snapshot.bucket_id, bucket.id);
        //assert_eq!(snapshot.metadata_id, metadata.id);
        //assert!(snapshot.created_at > 0);
        Ok(())
    }
}
