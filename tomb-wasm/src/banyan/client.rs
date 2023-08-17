use crate::banyan::{
    bucket::{Bucket, BucketKey, BucketMetadata},
    snapshot::{Snapshot, SnapshotMetadata},
};
use crate::blockstore::CarV2BlockStore as BlockStore;
use crate::error::TombWasmError;
use crate::fetch::get_stream;
use gloo::console::log;
use tomb_crypt::prelude::*;
// TODO: remove this import
use web_sys::CryptoKey;

// TODO: Implement the client

/// Banyan client for interacting with the Banyan API / Remote Storage
pub struct Client {
    endpoint: String,
    account_id: String,
    // api_key: EcSignatureKey,
}

impl Client {
    pub fn new(endpoint: String, account_id: String, _api_key: CryptoKey) -> Self {
        Self {
            endpoint,
            account_id,
            // api_key,
        }
    }

    // Account Metadata

    pub async fn get_total_storage(&self) -> Result<u64, TombWasmError> {
        Ok(1024 * 1024 * 1024)
    }

    pub async fn get_trash_bucket(&self) -> Result<BucketMetadata, TombWasmError> {
        Ok(BucketMetadata {
            id: "3".to_string(),
            bucket_type: "hot".to_string(),
            name: "trash".to_string(),
        })
    }

    pub async fn get_buckets(&self) -> Result<Vec<BucketMetadata>, TombWasmError> {
        // Return some sample data
        Ok([
            BucketMetadata {
                id: "1".to_string(),
                bucket_type: "hot".to_string(),
                name: "cat pics".to_string(),
            },
            BucketMetadata {
                id: "2".to_string(),
                bucket_type: "hot".to_string(),
                name: "dog pics".to_string(),
            },
            BucketMetadata {
                id: "3".to_string(),
                bucket_type: "hot".to_string(),
                name: "monkey photos".to_string(),
            },
        ]
        .to_vec())
    }

    pub async fn get_snapshots(&self) -> Result<Vec<SnapshotMetadata>, TombWasmError> {
        // Return some sample data
        Ok([
            SnapshotMetadata {
                id: "1".to_string(),
                bucket_id: "1".to_string(),
                snapshot_type: "hot".to_string(),
                version: "1".to_string(),
            },
            SnapshotMetadata {
                id: "2".to_string(),
                bucket_id: "1".to_string(),
                snapshot_type: "hot".to_string(),
                version: "2".to_string(),
            },
            SnapshotMetadata {
                id: "3".to_string(),
                bucket_id: "2".to_string(),
                snapshot_type: "cold".to_string(),
                version: "1".to_string(),
            },
        ]
        .to_vec())
    }

    // Bucket Metadata

    pub async fn get_bucket_keys(&self, _bucket_id: &str) -> Result<Vec<BucketKey>, TombWasmError> {
        // Return some sample data
        Ok([
            BucketKey {
                id: "1".to_string(),
                bucket_id: "1".to_string(),
                pem: "string".to_string(),
                approved: false,
            },
            BucketKey {
                id: "2".to_string(),
                bucket_id: "1".to_string(),
                pem: "string".to_string(),
                approved: true,
            },
        ]
        .to_vec())
    }

    pub async fn get_bucket_storage(&self, _bucket_id: &str) -> Result<u64, TombWasmError> {
        Ok(1024 * 1024)
    }

    pub async fn get_bucket_snapshots(
        &self,
        _bucket_id: &str,
    ) -> Result<Vec<SnapshotMetadata>, TombWasmError> {
        // Return some sample data
        Ok([
            SnapshotMetadata {
                id: "1".to_string(),
                bucket_id: "1".to_string(),
                snapshot_type: "hot".to_string(),
                version: "1".to_string(),
            },
            SnapshotMetadata {
                id: "2".to_string(),
                bucket_id: "1".to_string(),
                snapshot_type: "hot".to_string(),
                version: "2".to_string(),
            },
        ]
        .to_vec())
    }

    // Bucket Management

    pub async fn load_bucket(&self, _bucket_id: &str) -> Result<Bucket, TombWasmError> {
        // Get a random
        log!("tomb-wasm/banyan: load_bucket()");
        let url = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/meta.car".to_string();
        let mut stream = get_stream(url).await.unwrap();
        let vec = crate::utils::read_vec_from_readable_stream(&mut stream)
            .await
            .unwrap();
        // Generate a random vec
        // Create a blockstore
        let blockstore = BlockStore::new(vec).map_err(|e| {
            log!("tomb-wasm/banyan: load_bucket() error: {}", e.to_string());
            TombWasmError::car_error(format!("error reading car: {}", e))
        })?;
        // Create a bucket
        let bucket = Bucket::new(
            BucketMetadata {
                id: "1".to_string(),
                bucket_type: "hot".to_string(),
                name: "cat pics".to_string(),
            },
            blockstore,
        );
        Ok(bucket)
    }

    /// Takes all outstanding changes to file/directories + keys , and publishes them to our platform
    pub async fn sync_bucket(&self, _bucket: &Bucket) -> Result<(), TombWasmError> {
        // TODO: What does this response look like?
        Ok(())
    }

    pub async fn delete_bucket(&self, _bucket_id: &str) -> Result<(), TombWasmError> {
        // TODO: What does this response look like?
        Ok(())
    }

    pub async fn request_bucket_access(
        &self,
        _bucket_id: &str,
        _recipient_key: &EcPublicEncryptionKey,
    ) -> Result<(), TombWasmError> {
        // TODO: What does this response look like?
        Ok(())
    }

    // Snapshot Management

    // TODO: What does this response look like?
    pub async fn snapshot_bucket(
        &self,
        _bucket_id: &str,
        _snapshot: &Snapshot,
    ) -> Result<(), TombWasmError> {
        panic!("not implemented")
    }

    pub async fn load_snapshot(&self, _snapshot_id: &str) -> Result<Snapshot, TombWasmError> {
        // Get a random
        let url = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/meta.car".to_string();
        let mut stream = get_stream(url).await.unwrap();
        let vec = crate::utils::read_vec_from_readable_stream(&mut stream)
            .await
            .unwrap();
        // Create a blockstore
        let blockstore = BlockStore::new(vec)?;
        // Create a bucket
        let snapshot = Snapshot::new(
            SnapshotMetadata {
                id: "1".to_string(),
                bucket_id: "1".to_string(),
                snapshot_type: "hot".to_string(),
                version: "1".to_string(),
            },
            blockstore,
        );
        Ok(snapshot)
    }

    pub async fn purge_snapshot(&self, _snapshot_id: &str) -> Result<(), TombWasmError> {
        // TODO: What is the response?
        Ok(())
    }

    pub async fn restore_snapshot_to_bucket(
        &self,
        _snapshot_id: &str,
        _bucket_id: &str,
    ) -> Result<(), TombWasmError> {
        // TODO: What is the response?
        Ok(())
    }
}
