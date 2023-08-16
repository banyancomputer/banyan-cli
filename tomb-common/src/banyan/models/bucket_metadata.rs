use bytes::Bytes;
use serde::{Deserialize, Serialize};
use futures_core::stream::Stream;

use crate::banyan::api::buckets::metadata::{push::*, pull::*, read::*};
use crate::banyan::client::Client;
use crate::banyan::models::ModelError;

use super::storage_ticket::StorageTicket;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum BucketMetadataState {
    Uploading,
    UploadFailed,
    Pending,
    Current,
    Outdated,
    Deleted,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
/// Metadata Definition
pub struct BucketMetadata {
    pub id: uuid::Uuid,
    pub bucket_id: uuid::Uuid,

    pub root_cid: String,
    pub metadata_cid: String,
    pub data_size: usize,

    pub state: BucketMetadataState,
}

impl BucketMetadata {
    // TODO: This should probably take a generic trait related to Tomb in order to extract these arguments
    /// Push new Metadata for a bucket. Creates a new metadata records and returns a storage ticket
    pub async fn push<S>(
        bucket_id: uuid::Uuid,
        root_cid: String,
        metadata_cid: String,
        data_size: usize,
        metadata_stream: S,
        client: &mut Client,
    ) -> Result<(Self, StorageTicket), ModelError>    
    where
        reqwest::Body: From<S>,
    {
        let response = client.call(PushBucketMetadata {
            bucket_id: bucket_id.clone(),
            root_cid: root_cid.clone(),
            metadata_cid: metadata_cid.clone(),
            data_size,
            metadata_stream
        }).await.map_err(|_| ModelError::client_error())?;
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
            }
        ))
    }

    /// Pull the metadata file for the bucket metadata
    pub async fn pull(&self, client: &mut Client) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ModelError> {
        client.stream(PullBucketMetadata {
            bucket_id: self.bucket_id.clone(),
            id: self.id.clone(),
        }).await.map_err(|_| ModelError::client_error())
    }

    /// Read the a specific metadata of a bucket 
    pub async fn read(bucket_id: uuid::Uuid, id: uuid::Uuid, client: &mut Client) -> Result<Self, ModelError> {
        let response = client.call(ReadBucketMetadata {
            bucket_id,
            id,
        }).await.map_err(|_| ModelError::client_error())?;
        Ok(Self {
            id: response.id,
            bucket_id: bucket_id,
            root_cid: response.root_cid,
            metadata_cid: response.metadata_cid,
            data_size: response.data_size as usize,
            state: response.state,
        })
    }

    /// Read all the metadata for a bucket
    pub async fn read_all(bucket_id: uuid::Uuid, client: &mut Client) -> Result<Vec<Self>, ModelError> {
        let response = client.call(ReadAllBucketMetadata {
            bucket_id,
        }).await.map_err(|_| ModelError::client_error())?;
        Ok(response.0.into_iter().map(|response| Self {
            id: response.id,
            bucket_id,
            root_cid: response.root_cid,
            metadata_cid: response.metadata_cid,
            data_size: response.data_size as usize,
            state: response.state,
        }).collect())
    }

    // TODO: Delete
}

#[cfg(test)]
mod test {
    use super::*;
    use futures_util::stream::StreamExt;
    use crate::banyan::models::account::test::authenticated_client;
    use crate::banyan::models::bucket::test::create_bucket;

    #[tokio::test]
    async fn push_read_pull() -> Result<(), ModelError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (bucket_metadata, _storage_ticket) = BucketMetadata::push(
            bucket.id,
            "root_cid".to_string(),
            "metadata_cid".to_string(),
            100,
            "metadata_stream".as_bytes(),
            &mut client,
        ).await.unwrap();
        assert_eq!(bucket_metadata.bucket_id, bucket.id);
        assert_eq!(bucket_metadata.root_cid, "root_cid");
        assert_eq!(bucket_metadata.metadata_cid, "metadata_cid");
        assert_eq!(bucket_metadata.data_size, 100);
        assert_eq!(bucket_metadata.state, BucketMetadataState::Pending);

        let read_bucket_metadata = BucketMetadata::read(bucket.id, bucket_metadata.id, &mut client).await?;
        assert_eq!(bucket_metadata, read_bucket_metadata);

        let mut stream = read_bucket_metadata.pull(&mut client).await?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(data, "metadata_stream".as_bytes());
        Ok(())
    }
}