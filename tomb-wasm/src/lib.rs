//! This crate contains modules which are compiled to WASM
mod error;
pub mod mount;
pub mod types;
pub mod utils;

mod wasm_bucket;
mod wasm_bucket_key;
use tomb_common::banyan_api::requests::core::auth::device_api_key::regwait::end::EndRegwait;
pub use wasm_bucket::WasmBucket;
pub use wasm_bucket_key::WasmBucketKey;

use std::convert::From;
use std::convert::TryFrom;
use std::str::FromStr;

use gloo::console::log;
use js_sys::Array;
use tomb_common::banyan_api::client::{Client, Credentials};
use tomb_common::banyan_api::models::account::Account;
use tomb_common::banyan_api::models::{
    bucket::{Bucket, BucketType, StorageClass},
    bucket_key::*,
};
use tomb_crypt::prelude::*;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

use crate::error::TombWasmError;
use crate::mount::WasmMount;
use crate::types::*;
use crate::utils::*;

pub type TombResult<T> = Result<T, js_sys::Error>;

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
    /// * `account_id` - The id of the account to use
    /// * `core_endpoint` - The API endpoint to use for core
    /// * `data_endpoint` - The API endpoint to use for data
    ///
    /// # Returns
    ///
    /// A new TombWasm instance
    ///
    /// Don't call it from multiple threads in parallel!
    pub async fn new(
        signing_key_pem: String,
        account_id: String,
        core_endpoint: String,
        data_endpoint: String,
    ) -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        set_panic_hook();

        // todo: this method needs to return a Result type but that would currently change the
        // external API so I'm leaving this one alone for now

        log!("tomb-wasm: new()");

        log!(format!("tomb-wasm pem: {}", signing_key_pem));

        let mut banyan_client = Client::new(&core_endpoint, &data_endpoint).unwrap();
        let signing_key = EcSignatureKey::import(signing_key_pem.as_bytes())
            .await
            .map_err(|err| TombWasmError(format!("unable to create signature key from pem: {err}")))
            .unwrap();

        let account_id = Uuid::parse_str(&account_id).unwrap();
        let banyan_credentials = Credentials {
            account_id,
            signing_key,
        };
        banyan_client.with_credentials(banyan_credentials);

        Self(banyan_client)
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
            .map_err(|err| TombWasmError(format!("failed to retrieve usage: {err}")).into())
    }

    /// Get the current usage limit for the current account in bytes
    #[wasm_bindgen(js_name = getUsageLimit)]
    pub async fn get_usage_limit(&mut self) -> TombResult<u64> {
        Account::usage_limit(self.client())
            .await
            .map_err(|err| TombWasmError(format!("failed to get usage limit: {err}")).into())
    }

    /// List the buckets for the current account
    #[wasm_bindgen(js_name = listBuckets)]
    pub async fn list_buckets(&mut self) -> TombResult<Array> {
        let buckets = Bucket::read_all(self.client())
            .await
            .map_err(|err| TombWasmError(format!("failed to read all buckets: {err}")))?;

        buckets
            .iter()
            .map(|bucket| {
                let wasm_bucket = WasmBucket::from(bucket.clone());
                JsValue::try_from(wasm_bucket).map_err(|err| {
                    TombWasmError(format!("failed to convert bucket to JsValue: {err}")).into()
                })
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
        log!("tomb-wasm: list_bucket_snapshots()");
        // Parse the bucket id
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Call the API
        let snapshots = Bucket::list_snapshots_by_bucket_id(self.client(), bucket_id)
            .await
            .map_err(|err| TombWasmError(format!("unable to list snapshots for bucket: {err}")))?;

        // Convert the snapshots
        snapshots
            .into_iter()
            .map(|snapshot| {
                let wasm_snapshot = WasmSnapshot::from(snapshot);
                JsValue::try_from(wasm_snapshot).map_err(|err| {
                    TombWasmError(format!("failed to convert snapshot to JsValue: {err}")).into()
                })
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
        log!("tomb-wasm: list_bucket_keys()");
        // Parse the bucket id
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Call the API
        let keys = BucketKey::read_all(bucket_id, self.client())
            .await
            .map_err(|err| TombWasmError(format!("unable to read bucket keys: {err}")))?;

        // Convert the keys
        keys.iter()
            .map(|key| {
                let wasm_key = WasmBucketKey(key.clone());
                JsValue::try_from(wasm_key).map_err(|err| {
                    TombWasmError(format!("failed to convert key to JsValue: {err}")).into()
                })
            })
            .collect()
    }

    /// Create a new bucket
    /// # Arguments
    /// * `name` - The name of the bucket to create
    /// * `storage_class` - The storage class of the bucket to create
    /// * `bucket_type` - The type of the bucket to create
    /// * `encryption_key` - The encryption key to use for the bucket
    /// # Returns
    /// The bucket's metadata as a WasmBucket
    /// ```json
    /// {
    /// "id": "uuid",
    /// "name": "string"
    /// "type": "string",
    /// "storage_class": "string",
    /// }
    /// ```
    #[wasm_bindgen(js_name = createBucket)]
    pub async fn create_bucket(
        &mut self,
        name: String,
        storage_class: String,
        bucket_type: String,
        initial_bucket_key_pem: String,
    ) -> TombResult<WasmBucket> {
        log!("tomb-wasm: create_bucket()");
        let storage_class = StorageClass::from_str(&storage_class).expect("Invalid storage class");
        let bucket_type = BucketType::from_str(&bucket_type).expect("Invalid bucket type");
        // Call the API
        let (bucket, _bucket_key) = Bucket::create(
            name,
            initial_bucket_key_pem,
            bucket_type,
            storage_class,
            self.client(),
        )
        .await
        .expect("Failed to create bucket");
        // Convert the bucket
        let wasm_bucket = WasmBucket::from(bucket);
        // Ok
        Ok(wasm_bucket)
    }

    /// Create a bucket key for a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to create a key for
    /// # Returns
    /// The WasmBucketKey that was created
    #[wasm_bindgen(js_name = createBucketKey)]
    pub async fn create_bucket_key(&mut self, bucket_id: String) -> TombResult<WasmBucketKey> {
        log!("tomb-wasm: create_bucket_key()");
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Load the EcEncryptionKey
        let key = EcEncryptionKey::generate()
            .await
            .map_err(|err| TombWasmError(format!("failed to generate ec encryption key: {err}")))?;
        let key = key.public_key().map_err(|err| {
            TombWasmError(format!(
                "unable to get public key from ec encryption key: {err}"
            ))
        })?;

        let pem = String::from_utf8(key.export().await.unwrap()).unwrap();

        // Call the API
        let bucket_key = BucketKey::create(bucket_id, pem, self.client())
            .await
            .map_err(|err| TombWasmError(format!("failed to create bucket: {err}")))?;

        Ok(WasmBucketKey(bucket_key))
    }

    /// Delete a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to delete
    /// # Returns the id of the bucket that was deleted
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&mut self, bucket_id: String) -> TombResult<()> {
        log!("tomb-wasm: delete_bucket()");

        // Parse the bucket id
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Call the API
        Bucket::delete_by_id(self.client(), bucket_id)
            .await
            .map_err(|err| TombWasmError(format!("failed to delete bucket: {err}")).into())
    }

    /// End Registration waiting
    ///
    #[wasm_bindgen(js_name = completeDeviceKeyRegistration)]
    pub async fn complete_device_key_registration(
        &mut self,
        fingerprint: String,
    ) -> TombResult<()> {
        self.client()
            .call_no_content(EndRegwait { fingerprint })
            .await
            .map_err(|err| {
                TombWasmError(format!("failed to complete device key registration: {err}")).into()
            })
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
        log!(format!("tomb-wasm: mount / {}", &bucket_id));

        // Parse the bucket id
        let bucket_id_uuid = Uuid::parse_str(&bucket_id).unwrap();
        log!(format!(
            "tomb-wasm: mount / {} / reading key pair",
            &bucket_id
        ));

        // Load the EcEncryptionKey
        let key = EcEncryptionKey::import(encryption_key_pem.as_bytes())
            .await
            .map_err(|err| TombWasmError(format!("Unable to import encryption key: {err}")))?;
        log!(format!(
            "tomb-wasm: mount / {} / reading bucket",
            &bucket_id
        ));

        // Load the bucket
        let bucket: WasmBucket = Bucket::read(self.client(), bucket_id_uuid)
            .await
            .map_err(|err| TombWasmError(format!("failed to read bucket: {err}")))?
            .into();

        log!(format!("tomb-wasm: mount / {} / pulling mount", &bucket_id));

        // Get the bucket id
        // Try to pull the mount. Otherwise create it and push an initial piece of metadata
        let mount = match WasmMount::pull(bucket.clone(), self.client()).await {
            Ok(mut mount) => {
                log!(format!(
                    "tomb-wasm: mount / {} / pulled mount, unlocking",
                    &bucket_id
                ));

                // Unlock the mount
                mount.unlock(&key).await?;
                log!(format!(
                    "tomb-wasm: mount / {} / unlocked mount",
                    &bucket_id
                ));

                // Ok
                mount
            }
            Err(_) => {
                log!(format!(
                    "tomb-wasm: mount / {} / failed to pull mount, creating",
                    &bucket_id
                ));
                // Create the mount and push an initial piece of metadata
                WasmMount::new(bucket.clone(), &key, self.client()).await?
            }
        };

        // Ok
        Ok(mount)
    }
}
