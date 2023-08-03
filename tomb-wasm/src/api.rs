use crate::fetch::{get_json, get_stream};
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;

// TODO: deprecate this in favor of banyan-api-client

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
#[derive(Debug)]
/// Service for interacting with Metadata.
pub struct Api {
    /// Endpoint for a Metadata service.
    endpoint: String,
}

#[allow(dead_code)]
impl Api {
    /// Create a new Service.
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    /// List all buckets accessible to the user.
    /// # Returns
    /// * Vec<Bucket>.
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>, Error> {
        // TODO: Read real data, not fake data
        let url = "http://echo.jsontest.com/id/bucket_id/name/bucket_name/owner/bucket_owner/entrypoint/bucket_entrypoint".to_string();
        let json = get_json(url).await.unwrap();
        let bucket = serde_wasm_bindgen::from_value(json).unwrap();
        let buckets: Vec<Bucket> = [bucket].to_vec();
        Ok(buckets)
    }

    /// Load a bucket from the Metadata service.
    /// # Arguments
    /// * `id` - The unique identifier for the bucket.
    /// # Returns
    /// (Bucket, Vec<u8>) - A tuple of the Bucket and data for the bucket's Metadata CAR.
    pub async fn load_bucket(&self, _id: String) -> Result<(Bucket, Vec<u8>), Error> {
        let url = "http://echo.jsontest.com/id/bucket_id/name/bucket_name/owner/bucket_owner/entrypoint/bucket_entrypoint".to_string();
        let json = get_json(url).await.unwrap();
        let bucket = serde_wasm_bindgen::from_value(json).unwrap();

        let url = "https://www.random.org/cgi-bin/randbyte?nbytes=1024".to_string();
        let mut stream = get_stream(url).await.unwrap();
        let vec = crate::utils::read_vec_from_readable_stream(&mut stream)
            .await
            .unwrap();
        Ok((bucket, vec))
    }
}
