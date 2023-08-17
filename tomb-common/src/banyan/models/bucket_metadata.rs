use serde::{Deserialize, Serialize};

#[cfg(feature = "api")]
use {
    bytes::Bytes,
    futures_core::stream::Stream,
    crate::banyan::{
    api::buckets::metadata::{pull::*, push::*, read::*},
    client::Client,
    error::ClientError,
    models::storage_ticket::StorageTicket,
}};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
/// Possible states of BucketMetadata
pub enum BucketMetadataState {
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

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
/// Bucket Metadata Definition
pub struct BucketMetadata {
    /// The unique identifier for the bucket metadata
    pub id: uuid::Uuid,
    /// The unique identifier for the bucket it belongs to
    pub bucket_id: uuid::Uuid,
    /// The CID of the root of the bucket
    pub root_cid: String,
    /// The CID of the metadata forest
    pub metadata_cid: String,
    /// The size of the data in bytes that this metadata points to
    pub data_size: usize,
    /// The state of the metadata
    pub state: BucketMetadataState,
}

#[cfg(feature = "api")]
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
    ) -> Result<(Self, StorageTicket), ClientError>
    where
        reqwest::Body: From<S>,
    {
        let response = client
            .call(PushBucketMetadata {
                bucket_id: bucket_id.clone(),
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
            .stream(PullBucketMetadata {
                bucket_id: self.bucket_id.clone(),
                id: self.id.clone(),
            })
            .await
    }

    /// Read the a specific metadata of a bucket
    pub async fn read(
        bucket_id: uuid::Uuid,
        id: uuid::Uuid,
        client: &mut Client,
    ) -> Result<Self, ClientError> {
        let response = client.call(ReadBucketMetadata { bucket_id, id }).await?;
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
    pub async fn read_all(
        bucket_id: uuid::Uuid,
        client: &mut Client,
    ) -> Result<Vec<Self>, ClientError> {
        let response = client.call(ReadAllBucketMetadata { bucket_id }).await?;
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

    // TODO: Delete
}

#[cfg(feature = "api")]
#[cfg(test)]
mod test {
    use super::*;
    use crate::banyan::models::account::test::authenticated_client;
    use crate::banyan::models::bucket::test::create_bucket;
    use futures_util::stream::StreamExt;

    #[tokio::test]
    async fn push_read_pull() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (bucket_metadata, _storage_ticket) = BucketMetadata::push(
            bucket.id,
            "root_cid".to_string(),
            "metadata_cid".to_string(),
            100,
            "metadata_stream".as_bytes(),
            &mut client,
        )
        .await
        .unwrap();
        assert_eq!(bucket_metadata.bucket_id, bucket.id);
        assert_eq!(bucket_metadata.root_cid, "root_cid");
        assert_eq!(bucket_metadata.metadata_cid, "metadata_cid");
        assert_eq!(bucket_metadata.data_size, 100);
        assert_eq!(bucket_metadata.state, BucketMetadataState::Pending);

        let read_bucket_metadata =
            BucketMetadata::read(bucket.id, bucket_metadata.id, &mut client).await?;
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
