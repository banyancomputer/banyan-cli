/// Types with WASM wrappers
mod types;
use crate::prelude::api::{
    client::{Client, Credentials},
    models::{
        account::Account,
        bucket::{Bucket, BucketType, StorageClass},
        bucket_key::BucketKey,
    },
    requests::core::auth::device_api_key::{create::CreateDeviceApiKey, regwait::end::EndRegwait},
};
use js_sys::Array;
use std::{
    convert::{From, TryFrom},
    str::FromStr,
};
use tomb_crypt::prelude::{EcEncryptionKey, EcSignatureKey, PrivateKey, PublicKey};
use tracing::{error, info};
pub use types::{
    to_js_error_with_msg, to_wasm_error_with_msg, TombWasmError, WasmBucket, WasmBucketKey,
    WasmBucketMetadata, WasmBucketMount, WasmFsMetadataEntry, WasmMount, WasmNodeMetadata,
    WasmSharedFile, WasmSnapshot,
};
use uuid::Uuid;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

/// Special Result type for WASM builds
pub type TombResult<T> = Result<T, js_sys::Error>;

/// Wrapper around a Client
#[derive(Debug)]
#[wasm_bindgen]
pub struct TombWasm(pub(crate) Client);

/// TombWasm exposes the functionality of Tomb in a WASM module
#[wasm_bindgen]
impl TombWasm {
    fn client(&mut self) -> &mut Client {
        &mut self.0
    }

    // Note: Have to include this here so we can read the API key from the JS CryptoKey
    #[wasm_bindgen(constructor)]
    /// Create a new TombWasm instance
    /// # Arguments
    ///
    /// * `web_signing_key` - The CryptoKeyPair to use for signing requests
    /// * `user_id` - The id of the account to use
    /// * `core_endpoint` - The API endpoint to use for core
    ///
    /// # Returns
    ///
    /// A new TombWasm instance
    ///
    /// Don't call it from multiple threads in parallel!
    pub async fn new(signing_key_pem: String, user_id: String, core_endpoint: String) -> Self {
        info!("tomb-wasm: new()");

        let mut client = Client::new(&core_endpoint).unwrap();
        let signing_key = EcSignatureKey::import(signing_key_pem.as_bytes())
            .await
            .map_err(to_wasm_error_with_msg("signature key from pem"))
            .unwrap();

        let user_id = Uuid::parse_str(&user_id).unwrap();
        let banyan_credentials = Credentials {
            user_id,
            signing_key,
        };
        client.with_credentials(banyan_credentials);
        Self(client)
    }
}

impl From<Client> for TombWasm {
    fn from(client: Client) -> Self {
        Self(client)
    }
}

#[wasm_bindgen]
impl TombWasm {
    /*
     * Top level API Interface
     */

    /// Get the total consume storage space for the current account in bytes
    #[wasm_bindgen(js_name = getUsage)]
    pub async fn get_usage(&mut self) -> TombResult<u64> {
        Account::usage(self.client())
            .await
            .map_err(to_js_error_with_msg("retrieve usage"))
    }

    /// Get the current usage limit for the current account in bytes
    #[wasm_bindgen(js_name = getUsageLimit)]
    pub async fn get_usage_limit(&mut self) -> TombResult<u64> {
        Account::usage_limit(self.client())
            .await
            .map_err(to_js_error_with_msg("retrieve usage limit"))
    }

    /// List the buckets for the current account
    #[wasm_bindgen(js_name = listBuckets)]
    pub async fn list_buckets(&mut self) -> TombResult<Array> {
        let buckets = Bucket::read_all(self.client())
            .await
            .map_err(to_wasm_error_with_msg("read all buckets"))?;

        buckets
            .iter()
            .map(|bucket| {
                let wasm_bucket = WasmBucket::from(bucket.clone());
                JsValue::try_from(wasm_bucket).map_err(to_js_error_with_msg("bucket to JsValue"))
            })
            .collect()
    }

    /// List bucket snapshots for a bucket
    ///
    /// # Arguments
    ///
    /// * `bucket_id` - The id of the bucket to list snapshots for
    ///
    /// # Returns an array WasmSnapshots
    ///
    /// ```json
    /// [
    ///   {
    ///     "id": "ffc1dca2-5155-40be-adc6-c81eb7322fb8",
    ///     "bucket_id": "f0c55cc7-4896-4ff3-95de-76422af271b2",
    ///     "metadata_id": "05d063f1-1e3f-4876-8b16-aeb106af0eb0",
    ///     "created_at": "2023-09-05T19:05:34Z"
    ///   }
    /// ]
    /// ```
    #[wasm_bindgen(js_name = listBucketSnapshots)]
    pub async fn list_bucket_snapshots(&mut self, bucket_id: String) -> TombResult<Array> {
        info!("list_bucket_snapshots()");
        // Parse the bucket id
        let bucket_id =
            Uuid::parse_str(&bucket_id).map_err(to_wasm_error_with_msg("parse UUID"))?;

        // Call the API
        let snapshots = Bucket::list_snapshots_by_bucket_id(self.client(), bucket_id)
            .await
            .map_err(to_wasm_error_with_msg("list snapshots for bucket"))?;

        // Convert the snapshots
        snapshots
            .into_iter()
            .map(|snapshot| {
                let wasm_snapshot = WasmSnapshot::from(snapshot);
                JsValue::try_from(wasm_snapshot)
                    .map_err(to_js_error_with_msg("snapshot to JsValue"))
            })
            .collect()
    }

    /// List bucket keys for a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to list keys for
    /// # Returns an array of WasmBucketKeys in the form:
    /// ```json
    /// [
    /// {
    /// "id": "uuid",
    /// "bucket_id": "uuid",
    /// "pem": "string"
    /// "approved": "bool"
    /// }
    /// ]
    /// ```
    #[wasm_bindgen(js_name = listBucketKeys)]
    pub async fn list_bucket_keys(&mut self, bucket_id: String) -> TombResult<Array> {
        info!("list_bucket_keys()");
        // Parse the bucket id
        let bucket_id =
            Uuid::parse_str(&bucket_id).map_err(to_wasm_error_with_msg("parse UUID"))?;

        // Call the API
        let keys = BucketKey::read_all(bucket_id, self.client())
            .await
            .map_err(to_wasm_error_with_msg("read bucket keys"))?;

        // Convert the keys
        keys.iter()
            .map(|key| {
                let wasm_key = WasmBucketKey(key.clone());
                JsValue::try_from(wasm_key).map_err(to_js_error_with_msg("bucket key to JsValue"))
            })
            .collect()
    }

    /// Create a new bucket
    /// # Arguments
    /// * `name` - The name of the bucket to create
    /// * `storage_class` - The storage class of the bucket to create
    /// * `bucket_type` - The type of the bucket to create
    /// * `private_pem` - The private encryption key to use for the bucket
    /// * `public_pem` - The public encryption key to use for the bucket
    /// # Returns
    /// The bucket's metadata as a WasmBucket
    /// ```json
    /// {
    /// "name": "string"
    /// "storage_class": "string",
    /// "bucket_type": "string",
    /// "private_pem": "string",
    /// "public_pem": "string",
    /// }
    /// ```
    #[wasm_bindgen(js_name = createBucketAndMount)]
    pub async fn create_bucket_and_mount(
        &mut self,
        name: String,
        storage_class: String,
        bucket_type: String,
        private_pem: String,
        public_pem: String,
    ) -> TombResult<WasmBucketMount> {
        info!("create_bucket()");
        let storage_class = StorageClass::from_str(&storage_class)
            .map_err(|_| TombWasmError::new("invalid storage class"))?;
        let bucket_type = BucketType::from_str(&bucket_type)
            .map_err(|_| TombWasmError::new("invalid drive type"))?;
        // Call the API
        let (bucket, _bucket_key) =
            Bucket::create(name, public_pem, bucket_type, storage_class, self.client())
                .await
                .map_err(to_wasm_error_with_msg("create bucket"))?;
        // Convert the bucket
        let wasm_bucket = WasmBucket::from(bucket);
        let wasm_mount = WasmMount::new(wasm_bucket.clone(), private_pem, self.client()).await?;
        // Ok
        Ok(WasmBucketMount::new(wasm_bucket, wasm_mount))
    }

    /// Create a bucket key for a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to create a key for
    /// # Returns
    /// The WasmBucketKey that was created
    #[wasm_bindgen(js_name = createBucketKey)]
    pub async fn create_bucket_key(&mut self, bucket_id: String) -> TombResult<WasmBucketKey> {
        info!("create_bucket_key()");
        let bucket_id =
            Uuid::parse_str(&bucket_id).map_err(to_wasm_error_with_msg("parse UUID"))?;

        // Load the EcEncryptionKey
        let key = EcEncryptionKey::generate()
            .await
            .map_err(to_wasm_error_with_msg("ec encryption key generation"))?;
        let key = key
            .public_key()
            .map_err(to_wasm_error_with_msg("ec encryption key to public key"))?;

        let key_bytes = key
            .export()
            .await
            .map_err(to_wasm_error_with_msg("export EcPublicEncryptionKey"))?;
        let pem =
            String::from_utf8(key_bytes).map_err(to_wasm_error_with_msg("String from UTF8"))?;

        // Call the API
        let bucket_key = BucketKey::create(bucket_id, pem, self.client())
            .await
            .map_err(to_wasm_error_with_msg("bucket creation"))?;

        Ok(WasmBucketKey(bucket_key))
    }

    /// Rename a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to rename
    /// * `name` - the new name to give to the bucket
    /// # Returns Promise<void> in js speak
    #[wasm_bindgen(js_name = renameBucket)]
    pub async fn rename_bucket(&mut self, bucket_id: String, name: String) -> TombResult<()> {
        info!("rename_bucket()");

        // Parse the bucket id
        let bucket_id =
            Uuid::parse_str(&bucket_id).map_err(to_wasm_error_with_msg("parse UUID"))?;

        // We have to read here since the endpoint is expecting a PUT request
        let mut bucket = Bucket::read(self.client(), bucket_id)
            .await
            .map_err(to_wasm_error_with_msg("read renamed bucket"))?;

        bucket.name = name;
        bucket
            .update(self.client())
            .await
            .map_err(to_js_error_with_msg("rename bucket"))
    }

    /// Delete a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to delete
    /// # Returns the id of the bucket that was deleted
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&mut self, bucket_id: String) -> TombResult<()> {
        info!("delete_bucket()");

        // Parse the bucket id
        let bucket_id =
            Uuid::parse_str(&bucket_id).map_err(to_wasm_error_with_msg("parse UUID"))?;

        // Call the API
        Bucket::delete_by_id(self.client(), bucket_id)
            .await
            .map_err(to_js_error_with_msg("delete bucket"))
    }

    /// Approve the device key and end Registration waiting
    /// # Arguments
    /// * pem - The public PEM of the key being approved
    /// # Returns
    /// The result of ending the registration wait
    #[wasm_bindgen(js_name = approveDeviceApiKey)]
    pub async fn approve_device_api_key(&mut self, pem: String) -> TombResult<()> {
        // Create the Device Api Key
        let fingerprint = self
            .client()
            .call(CreateDeviceApiKey { pem })
            .await
            .map_err(to_wasm_error_with_msg("create device api key"))?
            .fingerprint;

        // End the registration waiting task, now that the updated shared metadata has triggered an internal approval
        self.client()
            .call_no_content(EndRegwait { fingerprint })
            .await
            .map_err(to_js_error_with_msg("end regwait"))
    }

    /* Bucket Mounting interface */

    /// Mount a bucket as a File System that can be managed by the user
    /// # Arguments
    /// * bucket_id - The id of the bucket to mount
    /// * key - The key to use to mount the bucket. This should be the crypto key pair that was used to create the bucket
    ///         or that has access to the bucket
    /// # Returns
    /// A WasmMount instance
    #[wasm_bindgen(js_name = mount)]
    pub async fn mount(
        &mut self,
        bucket_id: String,
        encryption_key_pem: String,
    ) -> TombResult<WasmMount> {
        let prefix = format!("mount()/{}", bucket_id);
        info!(prefix);

        // Parse the bucket id
        let bucket_id_uuid =
            Uuid::parse_str(&bucket_id).map_err(to_wasm_error_with_msg("parse UUID"))?;
        info!("{}: reading key pair", prefix);

        // Load the EcEncryptionKey
        let key = EcEncryptionKey::import(encryption_key_pem.as_bytes())
            .await
            .map_err(to_wasm_error_with_msg("import encryption key"))?;
        info!("{}: reading bucket", prefix);

        // Load the bucket
        let bucket: WasmBucket = Bucket::read(self.client(), bucket_id_uuid)
            .await
            .map_err(to_wasm_error_with_msg("read bucket"))?
            .into();

        let prefix = format!("mount()/{}", bucket.name());

        info!("{}: pulling mount", prefix);

        // Get the bucket id
        // Try to pull the mount. Otherwise create it and push an initial piece of metadata
        let mount = match WasmMount::pull(bucket.clone(), self.client()).await {
            Ok(mut mount) => {
                info!("{}: pulled mount, unlocking", prefix);

                // Unlock the mount
                let unlock_result = mount.unlock(&key).await;

                // TODO: This should be checking against a defined error type,
                // but it's pretty safe to assume here that a failure here means the key just
                // doesn't have access to the bucket.

                // Check the result
                match unlock_result {
                    Ok(_) => info!("{}: unlocked mount", prefix),
                    Err(_) => info!("{}: could not unlock mount", prefix),
                };

                mount
            }
            Err(err) => {
                error!("{}: failure to pull mount: {}", prefix, err);
                return Err(err.into());
            }
        };

        // Ok
        Ok(mount)
    }
}

impl TombWasm {
    pub async fn read_shared_file_from_bs(
        &mut self,
        shared_file_payload: String,
        bs: &impl wnfs::common::BlockStore,
    ) -> TombResult<Vec<u8>> {
        use crate::prelude::filesystem::{sharing::SharedFile, FsMetadata};

        let shared_file = SharedFile::import_b64_url(shared_file_payload).unwrap();

        let data = FsMetadata::receive_file_content(shared_file, bs)
            .await
            .unwrap();

        Ok(data)
    }

    pub async fn read_shared_file(&mut self, shared_file_payload: String) -> TombResult<Vec<u8>> {
        use crate::prelude::{
            blockstore::BanyanApiBlockStore,
            filesystem::{sharing::SharedFile, FsMetadata},
        };

        let api_blockstore_client = self.client().clone();
        let api_blockstore = BanyanApiBlockStore::from(api_blockstore_client);

        let shared_file = SharedFile::import_b64_url(shared_file_payload).unwrap();

        let data = FsMetadata::receive_file_content(shared_file, &api_blockstore)
            .await
            .unwrap();

        Ok(data)
    }
}
