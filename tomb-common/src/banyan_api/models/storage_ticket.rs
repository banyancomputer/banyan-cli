use serde::{Deserialize, Serialize};
use std::fmt::Display;
#[cfg(target_arch = "wasm32")]
use std::io::Read;
use uuid::Uuid;

use crate::banyan_api::{
    client::Client, error::ClientError, requests::staging::client_grant::create::*,
    requests::staging::upload::push::*,
};
use tomb_crypt::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// StorageTicket is a ticket that can be used authenticate requests to stage data to a storage host
pub struct StorageTicket {
    /// The host to stage data to
    pub host: String,
    /// The authorization token to use when staging data. Generated by the core service
    pub authorization: String,
}

impl Display for StorageTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "\n| STORAGE TICKET INFO |\nhost:\t{}\nauthorization:\t{}",
            self.host, self.authorization
        ))
    }
}

impl StorageTicket {
    /// Create a new grant for a client to stage data to a storage host
    /// Allows us to upload data to a storage host using our signing key
    pub async fn create_grant(self, client: &mut Client) -> Result<(), ClientError> {
        let signing_key = client
            .signing_key
            .as_ref()
            .expect("Client signing key not set");
        let public_key_bytes = signing_key
            .public_key()
            .expect("Failed to get public key")
            .export()
            .await
            .expect("Failed to export public key");
        let public_key =
            String::from_utf8(public_key_bytes).expect("Failed to convert public key to string");
        client
            .call_no_content(CreateGrant {
                host_url: self.host.clone(),
                bearer_token: self.authorization.clone(),
                public_key,
            })
            .await
    }

    // TODO: This should probably take a generic trait related to Tomb in order to extract these arguments
    /// Push new Metadata for a bucket. Creates a new metadata records and returns a storage ticket
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn upload_content<S>(
        self,
        // TODO: This should probably be a metadata cid
        metadata_id: Uuid,
        content: S,
        client: &mut Client,
    ) -> Result<(), ClientError>
    where
        reqwest::Body: From<S>,
    {
        client
            .multipart_no_content(PushContent {
                host_url: self.host.clone(),
                metadata_id,
                content,
            })
            .await
    }

    #[cfg(target_arch = "wasm32")]
    /// Push new metadata for a bucket. Creates a new metadata record and returns a storage ticket if needed
    /// WASM implementation because reqwest hates me
    pub async fn upload_content<S>(
        self,
        metadata_id: Uuid,
        content: S,
        client: &mut Client,
    ) -> Result<(), ClientError>
    where
        S: Read,
    {
        client
            .multipart_no_content(PushContent {
                host_url: self.host.clone(),
                metadata_id,
                content,
            })
            .await
    }
}

#[cfg(test)]
pub mod test {
    use tomb_crypt::pretty_fingerprint;

    use super::*;
    use crate::banyan_api::blockstore::BanyanApiBlockStore;
    use crate::banyan_api::models::account::test::authenticated_client;
    use crate::banyan_api::models::bucket::{Bucket, BucketType, StorageClass};
    use crate::banyan_api::models::bucket_key::BucketKey;
    use crate::banyan_api::models::metadata::Metadata;
    use crate::banyan_api::utils::generate_bucket_key;
    use crate::blockstore::carv2_memory::CarV2MemoryBlockStore;
    use crate::blockstore::RootedBlockStore;
    use crate::metadata::FsMetadata;

    #[tokio::test]
    async fn create_grant() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (
            _bucket,
            _bucket_key,
            _key,
            metadata,
            storage_ticket,
            metadata_store,
            content_store,
            mut fs_metadata,
            add_path_segments,
        ) = setup(&mut client).await?;
        storage_ticket.clone().create_grant(&mut client).await?;
        storage_ticket
            .clone()
            .upload_content(metadata.id, content_store.get_data(), &mut client)
            .await?;
        let mut blockstore_client = client.clone();
        blockstore_client
            .with_remote(&storage_ticket.host)
            .expect("Failed to create blockstore client");
        let banyan_api_blockstore = BanyanApiBlockStore::from(blockstore_client);
        let bytes = fs_metadata
            .read(add_path_segments, &metadata_store, &banyan_api_blockstore)
            .await
            .expect("Failed to get file");
        assert_eq!(bytes, "test".as_bytes().to_vec());
        Ok(())
    }

    async fn create_bucket(
        client: &mut Client,
    ) -> Result<(Bucket, BucketKey, EcEncryptionKey), ClientError> {
        let (key, pem) = generate_bucket_key().await;
        let bucket_type = BucketType::Interactive;
        let bucket_class = StorageClass::Hot;
        let bucket_name = format!("{}", rand::random::<u64>());
        let fingerprint = pretty_fingerprint(&key.fingerprint().await.expect("create fingerprint"));
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
        Ok((bucket, bucket_key, key))
    }

    async fn setup(
        client: &mut Client,
    ) -> Result<
        (
            Bucket,
            BucketKey,
            EcEncryptionKey,
            Metadata,
            StorageTicket,
            CarV2MemoryBlockStore,
            CarV2MemoryBlockStore,
            FsMetadata,
            Vec<String>,
        ),
        ClientError,
    > {
        let (bucket, bucket_key, key) = create_bucket(client).await?;
        let metadata_store = CarV2MemoryBlockStore::new().expect("Failed to create metadata store");
        let content_store = CarV2MemoryBlockStore::new().expect("Failed to create content store");
        let mut fs_metadata = FsMetadata::init(&key)
            .await
            .expect("Failed to create fs metadata");
        let mkdir_path_segments = vec!["test".to_string(), "path".to_string()];
        let add_path_segments = vec!["test".to_string(), "path".to_string(), "file".to_string()];
        let file_content = "test".as_bytes().to_vec();
        fs_metadata
            .mkdir(mkdir_path_segments, &metadata_store)
            .await
            .expect("Failed to create directory");
        fs_metadata
            .add(
                add_path_segments.clone(),
                file_content,
                &metadata_store,
                &content_store,
            )
            .await
            .expect("Failed to add file");
        fs_metadata
            .save(&metadata_store, &content_store)
            .await
            .expect("Failed to save fs metadata");
        let root_cid = &content_store.get_root().expect("Failed to get root cid");
        let metadata_cid = &metadata_store
            .get_root()
            .expect("Failed to get metadata cid");
        let data_size = content_store.data_size();
        let metadata_bytes = metadata_store.get_data();
        let (metadata, storage_ticket) = Metadata::push(
            bucket.id,
            root_cid.to_string(),
            metadata_cid.to_string(),
            data_size,
            vec![],
            metadata_bytes,
            client,
        )
        .await?;
        let storage_ticket = storage_ticket.expect("Storage ticket not returned");
        Ok((
            bucket,
            bucket_key,
            key,
            metadata,
            storage_ticket,
            metadata_store,
            content_store,
            fs_metadata,
            add_path_segments,
        ))
    }
}
