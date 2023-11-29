use super::SharingError;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use wnfs::{common::dagcbor, libipld::Cid, private::share::SharePayload};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFile {
    #[serde(serialize_with = "serialize_payload")]
    #[serde(deserialize_with = "deserialize_payload")]
    pub payload: SharePayload,
    pub forest_cid: Cid,
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

fn serialize_payload<S: Serializer>(
    payload: &SharePayload,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let bytes = dagcbor::encode(&payload).expect("failed to serialize payload");
    bytes.serialize(serializer)
}

fn deserialize_payload<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SharePayload, D::Error> {
    let bytes = <Vec<u8>>::deserialize(deserializer)?;
    Ok(dagcbor::decode::<SharePayload>(&bytes).expect("failed to deserialize payload"))
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
