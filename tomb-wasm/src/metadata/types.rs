// use js_sys::Uint8Array;
// use wasm_bindgen_futures::JsFuture;
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;

// TODO: move this somewhere else or extend from a common type

use crate::fetch::http::{get_json, get_stream};

// TODO: Work in this file should probably be moved to tomb-common once alignment is reached on struct members and use throughout the project.

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
/// Sum total of Metadata for a bucket managed by Tomb.
pub struct Bucket {
    /// The unique identifier for the bucket.
    pub id: String,
    /// The name of the bucket.
    pub name: String,
    /// The uid of the owner of the bucket.
    pub owner: String,
    // TODO: Is there a better type for this?
    /// A label into the bucket's Metadata Private Forest
    pub entrypoint: String,
}

#[allow(dead_code)]
/// Service for interacting with Metadata.
pub struct Service {
    /// Endpoint for a Metadata service.
    endpoint: String,
    /// Bearer token for a Metadata service.
    token: String,
}

#[allow(dead_code)]
impl Service {
    /// Create a new Service.
    pub fn new(endpoint: String, token: String) -> Self {
        Service { endpoint, token }
    }

    /* Create */
    // TODO: Creation methods

    /* Read */
    /// Read all buckets accessible to the user.
    /// # Returns
    /// * Vec<Bucket>.
    pub async fn read_buckets(&self) -> Result<Vec<Bucket>, Error> {
        // TODO: Read real data, not fake data
        let url = "http://echo.jsontest.com/id/bucket_id/name/bucket_name/owner/bucket_owner/entrypoint/bucket_entrypoint".to_string();
        let json = get_json(url).await.unwrap();
        let bucket = serde_wasm_bindgen::from_value(json).unwrap();
        let buckets: Vec<Bucket> = [bucket].to_vec();
        Ok(buckets)
    }

    /// Read the encrypted share key for a bucket.
    /// # Arguments
    /// * `bucket_id` - The unique identifier for the bucket.
    /// * `fingerprint` - The fingerprint of the public key the share key is encrypted with.
    /// # Returns
    /// * JsFuture that resolves to a String.
    pub async fn read_enc_share_key(
        &self,
        _bucket_id: String,
        _fingerprint: String,
    ) -> Result<Vec<u8>, Error> {
        // TODO: Read real data, not fake data
        let url = "https://www.random.org/cgi-bin/randbyte?nbytes=32".to_string();
        let mut stream = get_stream(url.to_string()).await.unwrap();
        let mut reader = stream.get_reader();
        let mut chunks: Vec<u8> = vec![];
        while let Ok(Some(result)) = reader.read().await {
            let chunk = js_sys::Uint8Array::from(result);
            chunks.extend(chunk.to_vec());
        }
        Ok(chunks)
    }

    /// Return a Uint8Array of encrypted Metadata for a bucket.
    /// # Arguments
    /// * `bucket_id` - The unique identifier for the bucket.
    /// # Returns
    /// * Uint8Array - A Uint8Array of encrypted Metadata.
    /// TODO: This should return a Reader into a CAR file inside the stream
    pub async fn read_metadata(&self, _bucket_id: String) -> Result<Vec<u8>, Error> {
        // TODO: Open a stream to a CAR from the metadata service -- for now this is just loading some random data
        let url = "https://www.random.org/cgi-bin/randbyte?nbytes=1024".to_string();
        let mut stream = get_stream(url).await.unwrap();
        let mut reader = stream.get_reader();
        let mut chunks: Vec<u8> = vec![];
        while let Ok(Some(result)) = reader.read().await {
            let chunk = js_sys::Uint8Array::from(result);
            chunks.extend(chunk.to_vec());
        }
        Ok(chunks)
    }

    /* Update */
    // TODO: Update methods

    /* Delete */
    // TODO: Delete methods
}
