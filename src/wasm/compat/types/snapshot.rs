use crate::api::models::snapshot::*;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

/// WASM Compatible Snapshot struct
#[derive(Debug)]
#[wasm_bindgen]
pub struct WasmSnapshot {
    id: Uuid,

    bucket_id: Uuid,
    metadata_id: Uuid,

    size: usize,
    created_at: usize,
}

#[wasm_bindgen]
impl WasmSnapshot {
    /// Bucket ID
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.bucket_id.to_string()
    }

    /// Creation time
    #[wasm_bindgen(getter = createdAt)]
    pub fn created_at(&self) -> f64 {
        self.created_at as f64
    }

    /// Snapshot ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.to_string()
    }

    /// Metadata ID
    #[wasm_bindgen(getter = metadataId)]
    pub fn metadata_id(&self) -> String {
        self.metadata_id.to_string()
    }

    /// Size of Snapshot data
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> f64 {
        self.size as f64
    }
}

impl From<Snapshot> for WasmSnapshot {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            id: snapshot.id,

            bucket_id: snapshot.bucket_id,
            metadata_id: snapshot.metadata_id,

            size: snapshot.size as usize,
            created_at: snapshot.created_at as usize,
        }
    }
}

impl From<WasmSnapshot> for Snapshot {
    fn from(snapshot: WasmSnapshot) -> Self {
        Self {
            id: snapshot.id,

            bucket_id: snapshot.bucket_id,
            metadata_id: snapshot.metadata_id,

            size: snapshot.size as u64,
            created_at: snapshot.created_at as i64,
        }
    }
}
