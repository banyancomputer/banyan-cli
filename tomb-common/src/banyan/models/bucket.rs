use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "api")]
use crate::banyan::{
    api::buckets::{create::*, delete::*, read::*},
    client::Client,
    error::ClientError,
    models::bucket_key::BucketKey,
};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
/// Possible types of Bucket
pub enum BucketType {
    /// A bucket for storing backups (Cold)
    Backup,
    /// A bucket for storing interactive data (Hot)
    Interactive,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Bucket Definition
pub struct Bucket {
    /// The unique identifier for the bucket
    pub id: Uuid,
    /// The name of the bucket
    pub name: String,
    /// The type of the bucket
    pub r#type: BucketType,
}

#[cfg(feature = "api")]
impl Bucket {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create(
        name: String,
        initial_bucket_key_pem: String,
        r#type: BucketType,
        client: &mut Client,
    ) -> Result<(Self, BucketKey), ClientError> {
        let response: CreateBucketResponse = client
            .call(CreateBucket {
                name,
                r#type,
                initial_bucket_key_pem: initial_bucket_key_pem.clone(),
            })
            .await?;
        Ok((
            Self {
                id: response.id,
                name: response.name,
                r#type: response.r#type,
            },
            BucketKey {
                id: response.initial_bucket_key.id,
                bucket_id: response.id,
                approved: response.initial_bucket_key.approved,
                pem: initial_bucket_key_pem,
            },
        ))
    }

    /// Read all instances of this model or data structure.
    pub async fn read_all(client: &mut Client) -> Result<Vec<Self>, ClientError> {
        let response: ReadAllBucketsResponse = client.call(ReadAllBuckets).await?;
        // Map the response to the model
        let mut buckets = Vec::new();
        for bucket in response.0 {
            buckets.push(Self {
                id: bucket.id,
                name: bucket.name,
                r#type: bucket.r#type,
            });
        }
        Ok(buckets)
    }

    /// Get the account associated with the current credentials. You do not need to pass an ID for this request.
    pub async fn read(client: &mut Client, id: Uuid) -> Result<Self, ClientError> {
        let response: ReadBucketResponse = client.call(ReadBucket { id }).await?;
        Ok(Self {
            id: response.id,
            name: response.name,
            r#type: response.r#type,
        })
    }

    /// Delete a bucket
    pub async fn delete(self, _client: &mut Client) -> Result<String, ClientError> {
        let response: DeleteBucketResponse = _client.call(DeleteBucket { id: self.id }).await?;
        Ok(response.id.to_string())
    }

    /// Delete a bucket by id
    pub async fn delete_by_id(client: &mut Client, id: Uuid) -> Result<String, ClientError> {
        let response: DeleteBucketResponse = client.call(DeleteBucket { id }).await?;
        Ok(response.id.to_string())
    }
}

#[cfg(test)]
#[cfg(feature = "api")]
pub mod test {
    use super::*;
    use crate::banyan::models::account::generate_bucket_key;
    use crate::banyan::models::account::test::authenticated_client;

    pub async fn create_bucket(client: &mut Client) -> Result<(Bucket, BucketKey), ClientError> {
        let (_, pem) = generate_bucket_key().await;
        let bucket_type = BucketType::Interactive;
        let (bucket, bucket_key) = Bucket::create(
            "test-interactive-bucket".to_string(),
            pem.clone(),
            bucket_type,
            client,
        )
        .await?;
        assert_eq!(bucket.name, "test-interactive-bucket");
        assert_eq!(bucket.r#type, bucket_type.clone());
        assert_eq!(bucket_key.approved, true);
        assert_eq!(bucket_key.pem, pem);
        assert_eq!(bucket_key.approved, true);
        Ok((bucket, bucket_key))
    }
    #[tokio::test]
    async fn create_read() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let read_bucket = Bucket::read(&mut client, bucket.id).await?;
        assert_eq!(read_bucket.name, bucket.name);
        assert_eq!(read_bucket.r#type, bucket.r#type);
        assert_eq!(read_bucket.id, bucket.id);
        Ok(())
    }

    // Rewrite the tests below but with the new pattern i just wrote

    #[tokio::test]
    async fn create_read_all() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let buckets = Bucket::read_all(&mut client).await?;
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].name, bucket.name);
        assert_eq!(buckets[0].r#type, bucket.r#type);
        assert_eq!(buckets[0].id, bucket.id);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let original_bucket_id = bucket.id.clone();
        let bucket_id = bucket.delete(&mut client).await?;
        assert_eq!(bucket_id, original_bucket_id.to_string());
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn delete_by_id() {
        let mut client = authenticated_client().await;
        let fake_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let _ = Bucket::delete_by_id(&mut client, fake_id).await.unwrap();
    }
}
