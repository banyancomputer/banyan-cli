use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "api")]
use crate::banyan::{
    api::buckets::keys::{create::*, delete::*, read::*},
    client::Client,
    error::ClientError,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
/// BucketKey Definition
pub struct BucketKey {
    /// The unique identifier for the bucket key
    pub id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub bucket_id: Uuid,
    /// The public key material for the bucket key
    pub pem: String,
    /// Whether or not the bucket key has been approved
    pub approved: bool,
}

#[cfg(feature = "api")]
impl BucketKey {
    /// Create a new bucket key
    pub async fn create(
        bucket_id: Uuid,
        pem: String,
        client: &mut Client,
    ) -> Result<Self, ClientError> {
        let response: CreateBucketKeyResponse = client
            .call(CreateBucketKey {
                bucket_id,
                pem: pem.clone(),
            })
            .await?;
        Ok(Self {
            id: response.id,
            bucket_id,
            pem,
            approved: response.approved,
        })
    }

    /// Read all bucket keys for a bucket
    pub async fn read_all(bucket_id: Uuid, client: &mut Client) -> Result<Vec<Self>, ClientError> {
        let response: ReadAllBucketKeysResponse =
            client.call(ReadAllBucketKeys { bucket_id }).await?;
        let mut bucket_keys = Vec::new();
        for key in response.0 {
            bucket_keys.push(Self {
                id: key.id,
                bucket_id,
                pem: key.pem,
                approved: key.approved,
            });
        }
        Ok(bucket_keys)
    }

    /// Read a bucket key
    pub async fn read(bucket_id: Uuid, id: Uuid, client: &mut Client) -> Result<Self, ClientError> {
        let response: ReadBucketKeyResponse = client.call(ReadBucketKey { bucket_id, id }).await?;
        Ok(Self {
            id: response.id,
            bucket_id,
            pem: response.pem,
            approved: response.approved,
        })
    }

    /// Delete a bucket key
    pub async fn delete(self, client: &mut Client) -> Result<String, ClientError> {
        let response = client
            .call(DeleteBucketKey {
                bucket_id: self.bucket_id,
                id: self.id,
            })
            .await?;
        Ok(response.id.to_string())
    }

    /// Delete a bucket key by id
    pub async fn delete_by_id(
        bucket_id: Uuid,
        id: Uuid,
        client: &mut Client,
    ) -> Result<String, ClientError> {
        let response = client.call(DeleteBucketKey { bucket_id, id }).await?;
        Ok(response.id.to_string())
    }
}

#[cfg(feature = "api")]
#[cfg(test)]
mod test {
    use super::*;
    use crate::banyan::models::account::generate_bucket_key;
    use crate::banyan::models::account::test::authenticated_client;
    use crate::banyan::models::bucket::test::create_bucket;

    #[tokio::test]
    async fn create() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem.clone(), &mut client).await?;
        assert_eq!(bucket_key.bucket_id, bucket.id);
        assert_eq!(bucket_key.approved, false);
        assert_eq!(bucket_key.pem, pem);
        Ok(())
    }

    #[tokio::test]
    async fn create_read() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem.clone(), &mut client).await?;
        let read_bucket_key = BucketKey::read(bucket.id, bucket_key.id, &mut client).await?;
        assert_eq!(bucket_key.id, read_bucket_key.id);
        assert_eq!(bucket_key.bucket_id, read_bucket_key.bucket_id);
        assert_eq!(bucket_key.approved, read_bucket_key.approved);
        assert_eq!(bucket_key.pem, read_bucket_key.pem);
        Ok(())
    }

    #[tokio::test]
    async fn create_read_all() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem.clone(), &mut client).await?;
        let bucket_keys = BucketKey::read_all(bucket.id, &mut client).await?;
        assert_eq!(bucket_keys.len(), 2);
        assert_eq!(bucket_key.id, bucket_keys[1].id);
        assert_eq!(bucket_key.bucket_id, bucket_keys[1].bucket_id);
        assert_eq!(bucket_key.approved, bucket_keys[1].approved);
        assert_eq!(bucket_key.pem, bucket_keys[1].pem);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem.clone(), &mut client).await?;
        let bucket_key_ = bucket_key.clone();
        let id = bucket_key.delete(&mut client).await?;
        assert_eq!(id, bucket_key_.id.to_string());
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn create_delete_by_id_for_extant_bucket() {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await.unwrap();
        let fake_bucket_key_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let _ = BucketKey::delete_by_id(bucket.id, fake_bucket_key_id, &mut client)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn create_delete_by_id_for_non_extant_bucket() {
        let mut client = authenticated_client().await;
        let fake_bucket_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let fake_bucket_key_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let _ = BucketKey::delete_by_id(fake_bucket_id, fake_bucket_key_id, &mut client)
            .await
            .unwrap();
    }
}
