use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "banyan-api")]
use {
    crate::banyan::{
        api::buckets::{
            metadata::{pull::*, push::*, read::*},
            snapshots::create::*,
        },
        client::Client,
        error::ClientError,
        models::snapshot::Snapshot,
        models::storage_ticket::StorageTicket,
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
    /// The CID of the root of the bucket
    pub root_cid: String,
    /// The CID of the metadata forest
    pub metadata_cid: String,
    /// The size of the data in bytes that this metadata points to
    pub data_size: usize,
    /// The state of the metadata
    pub state: MetadataState,
}

#[cfg(feature = "banyan-api")]
impl Metadata {
    // TODO: This should probably take a generic trait related to Tomb in order to extract these arguments
    /// Push new Metadata for a bucket. Creates a new metadata records and returns a storage ticket
    pub async fn push<S>(
        bucket_id: Uuid,
        root_cid: String,
        metadata_cid: String,
        data_size: usize,
        metadata_stream: S,
        client: &mut Client,
    ) -> Result<(Self, StorageTicket), ClientError>
    where
        reqwest::Body: From<S>,
    {
        let response = client
            .call(PushMetadata {
                bucket_id,
                root_cid: root_cid.clone(),
                metadata_cid: metadata_cid.clone(),
                data_size,
                metadata_stream,
            })
            .await?;
        Ok((
            Self {
                id: response.id,
                bucket_id,
                root_cid,
                metadata_cid,
                data_size,
                state: response.state,
            },
            StorageTicket {
                host: response.storage_host,
                authorization: response.storage_authorization,
            },
        ))
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
            data_size: response.data_size as usize,
            state: response.state,
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
                data_size: response.data_size as usize,
                state: response.state,
            })
            .collect())
    }

    /// Snapshot the current metadata
    pub async fn snapshot(&self, client: &mut Client) -> Result<Snapshot, ClientError> {
        let response = client
            .call(CreateSnapshot {
                bucket_id: self.bucket_id,
                metadata_id: self.id,
            })
            .await?;
        Ok(Snapshot {
            id: response.id,
            bucket_id: self.bucket_id,
            metadata_id: self.id,
            created_at: response.created_at,
        })
    }
    // TODO: Delete
}

#[cfg(feature = "banyan-api")]
#[cfg(test)]
pub mod test {
    use super::*;
    use crate::banyan::models::account::test::authenticated_client;
    use crate::banyan::models::bucket::test::create_bucket;
    use futures_util::stream::StreamExt;
    pub async fn push_metadata(
        bucket_id: Uuid,
        client: &mut Client,
    ) -> Result<(Metadata, StorageTicket), ClientError> {
        let (metadata, storage_ticket) = Metadata::push(
            bucket_id,
            "root_cid".to_string(),
            "metadata_cid".to_string(),
            100,
            "metadata_stream".as_bytes(),
            client,
        )
        .await?;
        Ok((metadata, storage_ticket))
    }
    pub async fn push_metadata_and_snapshot(
        bucket_id: Uuid,
        client: &mut Client,
    ) -> Result<(Metadata, StorageTicket, Snapshot), ClientError> {
        let (metadata, storage_ticket) = push_metadata(bucket_id, client).await?;
        let snapshot = metadata.snapshot(client).await?;
        Ok((metadata, storage_ticket, snapshot))
    }
    #[tokio::test]
    async fn push_read_pull() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _storage_ticket) = push_metadata(bucket.id, &mut client).await?;
        assert_eq!(metadata.bucket_id, bucket.id);
        assert_eq!(metadata.root_cid, "root_cid");
        assert_eq!(metadata.metadata_cid, "metadata_cid");
        assert_eq!(metadata.data_size, 100);
        assert_eq!(metadata.state, MetadataState::Pending);

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
    async fn push_read_pull_snapshot() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _storage_ticket) = push_metadata(bucket.id, &mut client).await?;
        assert_eq!(metadata.bucket_id, bucket.id);
        assert_eq!(metadata.root_cid, "root_cid");
        assert_eq!(metadata.metadata_cid, "metadata_cid");
        assert_eq!(metadata.data_size, 100);
        assert_eq!(metadata.state, MetadataState::Pending);

        let read_metadata = Metadata::read(bucket.id, metadata.id, &mut client).await?;
        assert_eq!(metadata, read_metadata);

        let mut stream = read_metadata.pull(&mut client).await?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(data, "metadata_stream".as_bytes());

        let snapshot = read_metadata.snapshot(&mut client).await?;
        assert_eq!(snapshot.bucket_id, bucket.id);
        assert_eq!(snapshot.metadata_id, metadata.id);
        assert!(snapshot.created_at > 0);
        Ok(())
    }
}
