use crate::api::{
    client::Client,
    error::ApiError,
    requests::core::buckets::keys::{
        create::{CreateBucketKey, CreateBucketKeyResponse},
        delete::DeleteBucketKey,
        read::{
            ReadAllBucketKeys, ReadAllBucketKeysResponse, ReadBucketKey, ReadBucketKeyResponse,
        },
        reject::RejectBucketKey,
    },
};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
/// BucketKey Definition
pub struct BucketKey {
    /// The unique identifier for the Bucket Key
    pub id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub bucket_id: Uuid,
    /// The public key material for the Bucket Key
    pub pem: String,
    /// The public key fingerprint for the Bucket Key
    pub fingerprint: String,
    /// Whether or not the Bucket Key has been approved
    pub approved: bool,
}

impl Display for BucketKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.approved {
            "Approved".green()
        } else {
            "Unapproved".red()
        };
        f.write_fmt(format_args!(
            "{}\ndrive_id:\t{}\nfingerprint:\t{}\nstatus:\t\t{}",
            "| KEY INFO |".yellow(),
            self.bucket_id,
            self.fingerprint,
            status
        ))
    }
}

impl BucketKey {
    /// Create a new Bucket Key
    pub async fn create(
        bucket_id: Uuid,
        pem: String,
        client: &mut Client,
    ) -> Result<Self, ApiError> {
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
            fingerprint: response.fingerprint,
            approved: response.approved,
        })
    }

    /// Read all Bucket Keys for a bucket
    pub async fn read_all(bucket_id: Uuid, client: &mut Client) -> Result<Vec<Self>, ApiError> {
        let response: ReadAllBucketKeysResponse =
            client.call(ReadAllBucketKeys { bucket_id }).await?;
        let mut bucket_keys = Vec::new();
        for key in response.0 {
            bucket_keys.push(Self {
                id: key.id,
                bucket_id,
                pem: key.pem,
                fingerprint: key.fingerprint,
                approved: key.approved,
            });
        }
        Ok(bucket_keys)
    }

    /// Read a Bucket Key
    pub async fn read(bucket_id: Uuid, id: Uuid, client: &mut Client) -> Result<Self, ApiError> {
        let response: ReadBucketKeyResponse = client.call(ReadBucketKey { bucket_id, id }).await?;
        Ok(Self {
            id: response.id,
            bucket_id,
            pem: response.pem,
            fingerprint: response.fingerprint,
            approved: response.approved,
        })
    }

    /// Delete a Bucket Key
    pub async fn delete(self, client: &mut Client) -> Result<(), ApiError> {
        client
            .call_no_content(DeleteBucketKey {
                bucket_id: self.bucket_id,
                id: self.id,
            })
            .await
    }

    /// Delete a Bucket Key by id
    pub async fn delete_by_id(
        bucket_id: Uuid,
        id: Uuid,
        client: &mut Client,
    ) -> Result<(), ApiError> {
        client
            .call_no_content(DeleteBucketKey { bucket_id, id })
            .await
    }

    /// Reject a Bucket Key
    pub async fn reject(bucket_id: Uuid, id: Uuid, client: &mut Client) -> Result<(), ApiError> {
        client
            .call_no_content(RejectBucketKey { bucket_id, id })
            .await
    }

    /// Context aware fingerprint using the locally known device fingerprint
    pub fn context_fmt(&self, my_fingerprint: &String) -> String {
        if &self.fingerprint == my_fingerprint {
            format!("{}\n{}", "| THIS IS YOUR KEY |".green(), self)
        } else {
            format!("{}", self)
        }
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod test {
    use crate::{
        api::{
            models::{
                account::test::authenticated_client,
                bucket::test::create_bucket,
                bucket_key::{ApiError, BucketKey},
                metadata::Metadata,
            },
            utils::generate_bucket_key,
        },
        prelude::api::requests::core::buckets::metadata::push::PushMetadata,
    };
    use reqwest::Body;
    use std::collections::BTreeSet;
    use tomb_crypt::{hex_fingerprint, prelude::PrivateKey};
    use uuid::Uuid;

    #[tokio::test]
    async fn create() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem.clone(), &mut client).await?;
        assert_eq!(bucket_key.bucket_id, bucket.id);
        assert!(!bucket_key.approved);
        assert_eq!(bucket_key.pem, pem);
        Ok(())
    }

    #[tokio::test]
    async fn create_read() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (key, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let our_fingerprint = hex_fingerprint(
            key.fingerprint()
                .await
                .expect("cant fingerprint")
                .as_slice(),
        );
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        assert_eq!(our_fingerprint, bucket_key.fingerprint);
        let read_bucket_key = BucketKey::read(bucket.id, bucket_key.id, &mut client).await?;
        assert_eq!(bucket_key.id, read_bucket_key.id);
        assert_eq!(bucket_key.bucket_id, read_bucket_key.bucket_id);
        assert_eq!(bucket_key.approved, read_bucket_key.approved);
        assert_eq!(bucket_key.pem, read_bucket_key.pem);
        assert_eq!(bucket_key.fingerprint, read_bucket_key.fingerprint);
        Ok(())
    }

    #[tokio::test]
    async fn create_read_all() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        let bucket_keys = BucketKey::read_all(bucket.id, &mut client).await?;
        assert_eq!(bucket_keys.len(), 2);
        assert_eq!(bucket_key.id, bucket_keys[1].id);
        assert_eq!(bucket_key.bucket_id, bucket_keys[1].bucket_id);
        assert_eq!(bucket_key.approved, bucket_keys[1].approved);
        assert_eq!(bucket_key.pem, bucket_keys[1].pem);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        let bucket_key_id = bucket_key.id;
        bucket_key.delete(&mut client).await?;
        let all_remaining = BucketKey::read_all(bucket.id, &mut client).await?;
        assert!(!all_remaining.iter().any(|value| value.id == bucket_key_id));
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn create_delete_by_id_for_extant_bucket() {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await.unwrap();
        let fake_bucket_key_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        BucketKey::delete_by_id(bucket.id, fake_bucket_key_id, &mut client)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn create_delete_by_id_for_non_extant_bucket() {
        let mut client = authenticated_client().await;
        let fake_bucket_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let fake_bucket_key_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();

        BucketKey::delete_by_id(fake_bucket_id, fake_bucket_key_id, &mut client)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_reject() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        assert!(!bucket_key.approved);
        BucketKey::reject(bucket.id, bucket_key.id, &mut client).await?;
        let all_remaining = BucketKey::read_all(bucket.id, &mut client).await?;
        assert!(!all_remaining.iter().any(|value| value.id == bucket_key.id));
        Ok(())
    }

    #[tokio::test]
    async fn reject_approved_key() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (bucket, initial_bucket_key) = create_bucket(&mut client).await?;
        assert!(initial_bucket_key.approved);
        BucketKey::reject(bucket.id, initial_bucket_key.id, &mut client).await?;
        Ok(())
    }

    #[tokio::test]
    async fn approve_new_key() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (bucket, initial_bucket_key) = create_bucket(&mut client).await?;
        // Create a new bucket key
        let (_, pem) = generate_bucket_key().await;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        assert!(!bucket_key.approved);

        // Push metadata with the new BucketKey listed as valid
        Metadata::push(
            PushMetadata {
                bucket_id: bucket.id,
                expected_data_size: 0,
                root_cid: String::from("root_cid"),
                metadata_cid: String::from("metadata_cid"),
                previous_cid: None,
                valid_keys: vec![initial_bucket_key.fingerprint, bucket_key.fingerprint],
                deleted_block_cids: BTreeSet::new(),
                metadata_stream: Body::from("metadata_stream"),
            },
            &mut client,
        )
        .await?;

        // Read the bucket key again
        let updated_bucket_key = BucketKey::read(bucket.id, bucket_key.id, &mut client).await?;
        assert!(updated_bucket_key.approved);

        Ok(())
    }
}
