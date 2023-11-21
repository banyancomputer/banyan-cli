use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::{Deserialize, Serialize};
use wnfs::{common::dagcbor, libipld::Cid, private::share::SharePayload};

use super::SharingError;

#[derive(Debug, Clone)]
pub struct SharedFile {
    pub payload: SharePayload,
    pub forest_cid: Cid,
    pub file_name: String,
    pub mime_type: String,
    pub size: u64,
}

impl Serialize for SharedFile {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let payload_bytes = dagcbor::encode(&self.payload).expect("failed to serialize payload");
        (
            payload_bytes,
            self.forest_cid,
            self.file_name.clone(),
            self.mime_type.clone(),
            self.size,
        )
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SharedFile {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (payload_bytes, forest_cid, file_name, mime_type, size) =
            <(Vec<u8>, Cid, String, String, u64)>::deserialize(deserializer)?;
        let payload: SharePayload =
            dagcbor::decode(&payload_bytes).expect("failed to deserialize payload");
        Ok(SharedFile {
            payload,
            forest_cid,
            file_name,
            mime_type,
            size,
        })
    }
}

impl SharedFile {
    pub fn export_b64_url(&self) -> Result<String, SharingError> {
        Ok(URL_SAFE.encode(serde_json::to_string(&self)?.as_bytes()))
    }

    pub fn import_b64_url(b64_string: String) -> Result<Self, SharingError> {
        let bytes = URL_SAFE
            .decode(b64_string)
            .map_err(|_| SharingError::invalid_data("invalid url decode"))?;
        let json = String::from_utf8(bytes)
            .map_err(|_| SharingError::invalid_data("invalid url decode utf8"))?;
        let shared_file: SharedFile = serde_json::from_str(&json)?;
        Ok(shared_file)
    }
}
