use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

use crate::api::{
    client::Client,
    error::ClientError,
    models::bucket_key::BucketKey,
    requests::{
        core::buckets::{
            create::*, delete::*, read::*, snapshots::read::ReadAllSnapshots, usage::GetBucketUsage,
        },
        staging::client_grant::authorization::AuthorizationGrants,
    },
};

use super::snapshot::Snapshot;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
/// Possible types of Bucket
pub enum BucketType {
    /// A bucket for storing backups (Cold)
    Backup,
    /// A bucket for storing interactive data (Hot)
    Interactive,
}
impl Display for BucketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BucketType::Backup => write!(f, "backup"),
            BucketType::Interactive => write!(f, "interactive"),
        }
    }
}
impl FromStr for BucketType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backup" => Ok(BucketType::Backup),
            "interactive" => Ok(BucketType::Interactive),
            _ => Err("Invalid bucket type".to_string()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Possible storage classes for Bucket
pub enum StorageClass {
    /// Hot storage
    Hot,
    /// Warm storage
    Warm,
    /// Cold storage
    Cold,
}
impl Display for StorageClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageClass::Hot => write!(f, "hot"),
            StorageClass::Warm => write!(f, "warm"),
            StorageClass::Cold => write!(f, "cold"),
        }
    }
}
impl FromStr for StorageClass {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hot" => Ok(StorageClass::Hot),
            "warm" => Ok(StorageClass::Warm),
            "cold" => Ok(StorageClass::Cold),
            _ => Err("Invalid storage class".to_string()),
        }
    }
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
    /// The storage class of the bucket
    pub storage_class: StorageClass,
}

impl Display for Bucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "name:\t\t\t{}\ndrive_id:\t\t{}\ntype:\t\t\t{}\nstorage class:\t\t{}",
            self.name, self.id, self.r#type, self.storage_class
        ))
    }
}

impl Bucket {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create(
        name: String,
        initial_bucket_key_pem: String,
        r#type: BucketType,
        storage_class: StorageClass,
        client: &mut Client,
    ) -> Result<(Self, BucketKey), ClientError> {
        let response: CreateBucketResponse = client
            .call(CreateBucket {
                name,
                r#type,
                initial_bucket_key_pem: initial_bucket_key_pem.clone(),
                storage_class,
            })
            .await?;
        Ok((
            Self {
                id: response.id,
                name: response.name,
                r#type: response.r#type,
                storage_class: response.storage_class,
            },
            BucketKey {
                id: response.initial_bucket_key.id,
                bucket_id: response.id,
                approved: response.initial_bucket_key.approved,
                pem: initial_bucket_key_pem,
                fingerprint: response.initial_bucket_key.fingerprint,
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
                storage_class: bucket.storage_class,
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
            storage_class: response.storage_class,
        })
    }

    /// Get the snapshots for the bucket
    pub async fn list_snapshots(&self, client: &mut Client) -> Result<Vec<Snapshot>, ClientError> {
        let response = client.call(ReadAllSnapshots { bucket_id: self.id }).await?;
        Ok(response
            .0
            .into_iter()
            .map(|response| Snapshot {
                id: response.id,
                bucket_id: self.id,
                metadata_id: response.metadata_id,
                size: response.size.unwrap_or(0),
                created_at: response.created_at,
            })
            .collect())
    }

    /// List snapshots by a bucket id
    pub async fn list_snapshots_by_bucket_id(
        client: &mut Client,
        bucket_id: Uuid,
    ) -> Result<Vec<Snapshot>, ClientError> {
        let response = client.call(ReadAllSnapshots { bucket_id }).await?;
        Ok(response
            .0
            .into_iter()
            .map(|response| Snapshot {
                id: response.id,
                bucket_id,
                metadata_id: response.metadata_id,
                size: response.size.unwrap_or(0),
                created_at: response.created_at,
            })
            .collect())
    }

    /// Get the usage for the bucket
    pub async fn usage(&self, client: &mut Client) -> Result<u64, ClientError> {
        client
            .call(GetBucketUsage { id: self.id })
            .await
            .map(|response| response.size)
    }

    /// Delete a bucket
    pub async fn delete(&self, client: &mut Client) -> Result<(), ClientError> {
        client.call_no_content(DeleteBucket { id: self.id }).await
    }

    /// Delete a bucket by id
    pub async fn delete_by_id(client: &mut Client, id: Uuid) -> Result<(), ClientError> {
        client.call_no_content(DeleteBucket { id }).await
    }

    /// Authorization grants
    pub async fn get_grants_token(&self, client: &mut Client) -> Result<String, ClientError> {
        client
            .call(AuthorizationGrants { bucket_id: self.id })
            .await
            .map(|value| value.authorization_token)
    }
}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
pub mod test {
    use tomb_crypt::hex_fingerprint;
    use tomb_crypt::prelude::PrivateKey;

    use super::*;
    use crate::api::{
        models::{
            account::test::{authenticated_client, unauthenticated_client},
            metadata::test::push_metadata_and_snapshot,
        },
        utils::generate_bucket_key,
    };

    pub async fn create_bucket(client: &mut Client) -> Result<(Bucket, BucketKey), ClientError> {
        let (key, pem) = generate_bucket_key().await;
        let bucket_type = BucketType::Interactive;
        let bucket_class = StorageClass::Hot;
        let bucket_name = format!("{}", rand::random::<u64>());
        let fingerprint = hex_fingerprint(
            key.fingerprint()
                .await
                .expect("create fingerprint")
                .as_slice(),
        );
        let (bucket, bucket_key) = Bucket::create(
            bucket_name.clone(),
            pem.clone(),
            bucket_type,
            bucket_class,
            client,
        )
        .await?;
        assert_eq!(bucket.name, bucket_name.clone());
        assert_eq!(bucket.r#type, bucket_type.clone());
        assert!(bucket_key.approved);
        assert_eq!(bucket_key.pem, pem);
        assert_eq!(bucket_key.fingerprint, fingerprint);
        assert!(bucket_key.approved);
        Ok((bucket, bucket_key))
    }
    pub fn fake_bucket() -> Bucket {
        Bucket {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
            name: "fake-bucket".to_string(),
            r#type: BucketType::Interactive,
            storage_class: StorageClass::Hot,
        }
    }
    #[tokio::test]
    async fn create_read() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let read_bucket = Bucket::read(&mut client, bucket.id).await?;
        assert_eq!(read_bucket.name, bucket.name);
        assert_eq!(read_bucket.r#type, bucket.r#type);
        assert_eq!(read_bucket.id, bucket.id);
        assert_eq!(read_bucket.storage_class, bucket.storage_class);
        Ok(())
    }
    #[tokio::test]
    async fn create_read_unauthorized() -> Result<(), ClientError> {
        let mut good_client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut good_client).await?;
        let mut bad_client = authenticated_client().await;
        let read_bucket = Bucket::read(&mut bad_client, bucket.id).await;
        assert!(read_bucket.is_err());
        let read_bucket = Bucket::read(&mut bad_client, fake_bucket().id).await;
        assert!(read_bucket.is_err());
        Ok(())
    }
    #[tokio::test]
    async fn create_usage() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let usage = bucket.usage(&mut client).await?;
        assert_eq!(usage, 0);
        Ok(())
    }
    #[tokio::test]
    async fn create_read_all() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let buckets = Bucket::read_all(&mut client).await?;
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].name, bucket.name);
        assert_eq!(buckets[0].r#type, bucket.r#type);
        assert_eq!(buckets[0].id, bucket.id);
        assert_eq!(buckets[0].storage_class, bucket.storage_class);
        Ok(())
    }
    #[tokio::test]
    async fn create_list_no_snapshots() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let snapshots = bucket.list_snapshots(&mut client).await?;
        assert_eq!(snapshots.len(), 0);
        Ok(())
    }
    #[tokio::test]
    async fn create_list_snapshots() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _host, _authorization, _snapshot) =
            push_metadata_and_snapshot(bucket.id, &mut client).await?;
        let snapshots = bucket.list_snapshots(&mut client).await?;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].bucket_id, bucket.id);
        assert_eq!(snapshots[0].metadata_id, metadata.id);
        Ok(())
    }
    #[tokio::test]
    async fn create_delete() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        bucket.delete(&mut client).await?;
        Ok(())
    }
    #[tokio::test]
    async fn create_delete_unauthorized() -> Result<(), ClientError> {
        let mut good_client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut good_client).await?;
        let mut bad_client = unauthenticated_client().await;
        let delete_result = bucket.delete(&mut bad_client).await;
        assert!(delete_result.is_err());
        Ok(())
    }
    #[tokio::test]
    #[should_panic]
    async fn delete_by_id() {
        let mut client = authenticated_client().await;
        let fake_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        Bucket::delete_by_id(&mut client, fake_id).await.unwrap();
    }
}
