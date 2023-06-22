use wasm_bindgen_futures::JsFuture;
use anyhow::{Result, Error};


#[allow(dead_code)]
#[derive(Default, Clone)]
#[repr(C)]
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
    /// * TODO: JsFuture that resolves to a Vec<Bucket>.
    // pub fn read_buckets(&self) -> Result<JsFuture, Error> {
    pub fn read_buckets(&self) -> Result<Vec<Bucket>, Error> {
        let vec = vec![Bucket {
            id: "id".to_string(),
            name: "name".to_string(),
            owner: "owner".to_string(),
            entrypoint: "entrypoint".to_string(),
        }];
        Ok(vec)
    }

    /// Read the encrypted share key for a bucket.
    /// # Arguments
    /// * `bucket_id` - The unique identifier for the bucket.
    /// * `fingerprint` - The fingerprint of the public key the share key is encrypted with.
    /// # Returns
    /// * JsFuture that resolves to a String.
    pub fn read_share_key(&self, _bucket_id: String, _fingerprint: String) -> Result<JsFuture, Error> {
        unimplemented!()
    }

    /// Return a readable stream over a CAR file describing the metadata for a bucket.
    /// # Arguments
    /// * `bucket_id` - The unique identifier for the bucket.
    /// # Returns
    /// * JsFuture that resolves to a ReadableStream.
    pub fn read_metadata(&self, _bucket_id: String) -> Result<JsFuture, Error> {
        unimplemented!()
    } 

    /* Update */
    // TODO: Update methods
    
    /* Delete */
    // TODO: Delete methods
}
