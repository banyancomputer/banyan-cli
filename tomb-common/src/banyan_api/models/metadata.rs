use std::fmt::Display;
#[cfg(target_arch = "wasm32")]
use std::io::Read;

use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

use {
    crate::banyan_api::{
        client::Client,
        error::ClientError,
        models::storage_ticket::StorageTicket,
        requests::core::buckets::{
            metadata::{pull::*, push::*, read::*},
            snapshots::create::*,
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
        match self {
            MetadataState::Uploading => write!(f, "uploading"),
            MetadataState::UploadFailed => write!(f, "upload_failed"),
            MetadataState::Pending => write!(f, "pending"),
            MetadataState::Current => write!(f, "current"),
            MetadataState::Outdated => write!(f, "outdated"),
            MetadataState::Deleted => write!(f, "deleted"),
        }
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
            "\n{}\nmetadata_id:\t{}\nroot_cid:\t{}\ndata_size:\t{}\nstatus:\t\t{}",
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
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn push<S>(
        bucket_id: Uuid,
        root_cid: String,
        metadata_cid: String,
        expected_data_size: u64,
        valid_keys: Vec<String>,
        deleted_blocks: BTreeSet<String>,
        metadata_stream: S,
        client: &mut Client,
    ) -> Result<(Self, Option<StorageTicket>), ClientError>
    where
        reqwest::Body: From<S>,
    {
        let response = client
            .multipart(PushMetadata {
                bucket_id,
                root_cid: root_cid.clone(),
                metadata_cid: metadata_cid.clone(),
                expected_data_size,
                valid_keys,
                deleted_blocks,
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
        match response.storage_host {
            None => Ok((metadata, None)),
            Some(_) => Ok((
                metadata,
                Some(StorageTicket {
                    host: response.storage_host.unwrap(),
                    authorization: response.storage_authorization.unwrap(),
                }),
            )),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(target_arch = "wasm32")]
    /// Push new metadata for a bucket. Creates a new metadata record and returns a storage ticket if needed
    /// WASM implementation because reqwest hates me
    pub async fn push<S>(
        bucket_id: Uuid,
        root_cid: String,
        metadata_cid: String,
        expected_data_size: u64,
        valid_keys: Vec<String>,
        deleted_blocks: BTreeSet<String>,
        metadata_stream: S,
        client: &mut Client,
    ) -> Result<(Self, Option<StorageTicket>), ClientError>
    where
        S: Read,
    {
        let response = client
            .multipart(PushMetadata {
                bucket_id,
                root_cid: root_cid.clone(),
                metadata_cid: metadata_cid.clone(),
                expected_data_size,
                valid_keys,
                deleted_blocks,
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
        match response.storage_host {
            None => Ok((metadata, None)),
            Some(_) => Ok((
                metadata,
                Some(StorageTicket {
                    host: response.storage_host.unwrap(),
                    authorization: response.storage_authorization.unwrap(),
                }),
            )),
        }
    }

    /// Pull the metadata file for the bucket metadata
    pub async fn pull(
        &self,
        client: &mut Client,
    ) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ClientError> {
        client
            .stream(PullMetadata {
                bucket_id: self.bucket_id,
                id: self.id,
            })
            .await
    }

    /// Read the a specific metadata of a bucket
    pub async fn read(bucket_id: Uuid, id: Uuid, client: &mut Client) -> Result<Self, ClientError> {
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
    pub async fn read_all(bucket_id: Uuid, client: &mut Client) -> Result<Vec<Self>, ClientError> {
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
    pub async fn read_current(bucket_id: Uuid, client: &mut Client) -> Result<Self, ClientError> {
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
    pub async fn snapshot(&self, client: &mut Client) -> Result<Uuid, ClientError> {
        let snapshot_resp = client
            .call(CreateSnapshot {
                bucket_id: self.bucket_id,
                metadata_id: self.id,
            })
            .await?;

        Ok(snapshot_resp.id)
    }
}

#[cfg(feature = "fake")]
#[cfg(test)]
pub mod test {
    use futures_util::stream::StreamExt;
    use uuid::Uuid;

    use crate::banyan_api::models::{
        account::test::authenticated_client, bucket::test::create_bucket,
    };

    use super::*;

    pub async fn push_empty_metadata(
        bucket_id: Uuid,
        client: &mut Client,
    ) -> Result<(Metadata, Option<StorageTicket>), ClientError> {
        let (metadata, storage_ticket) = Metadata::push(
            bucket_id,
            "root_cid".to_string(),
            "metadata_cid".to_string(),
            0,
            vec![],
            "metadata_stream".as_bytes(),
            client,
        )
        .await?;
        Ok((metadata, storage_ticket))
    }
    pub async fn push_metadata_and_snapshot(
        bucket_id: Uuid,
        client: &mut Client,
    ) -> Result<(Metadata, Option<StorageTicket>, Uuid), ClientError> {
        let (metadata, storage_ticket) = push_empty_metadata(bucket_id, client).await?;
        let snapshot_id = metadata.snapshot(client).await?;
        Ok((metadata, storage_ticket, snapshot_id))
    }
    #[tokio::test]
    async fn push_read_pull() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _storage_ticket) = push_empty_metadata(bucket.id, &mut client).await?;
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
    async fn push_read_unauthorized() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _storage_ticket) = push_empty_metadata(bucket.id, &mut client).await?;
        assert_eq!(metadata.bucket_id, bucket.id);

        let mut bad_client = authenticated_client().await;
        let read_metadata = Metadata::read(bucket.id, metadata.id, &mut bad_client).await;
        assert!(read_metadata.is_err());
        Ok(())
    }
    #[tokio::test]
    async fn push_read_wrong_bucket() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (other_bucket, _) = create_bucket(&mut client).await?;
        let (_metadata, _storage_ticket) = push_empty_metadata(bucket.id, &mut client).await?;
        let (other_metadata, _storage_ticket) =
            push_empty_metadata(other_bucket.id, &mut client).await?;
        let read_metadata = Metadata::read(bucket.id, other_metadata.id, &mut client).await;
        assert!(read_metadata.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn push_read_pull_snapshot() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _storage_ticket) = push_empty_metadata(bucket.id, &mut client).await?;
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

        let _snapshot_id = read_metadata.snapshot(&mut client).await?;
        //assert_eq!(snapshot.bucket_id, bucket.id);
        //assert_eq!(snapshot.metadata_id, metadata.id);
        //assert!(snapshot.created_at > 0);
        Ok(())
    }
}
