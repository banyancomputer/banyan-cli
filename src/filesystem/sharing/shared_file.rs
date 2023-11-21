use anyhow::{Result, anyhow};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::{Deserialize, Serialize};
use wnfs::{libipld::Cid, private::share::SharePayload};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFile {
    pub payload: SharePayload,
    pub forest_cid: Cid,
    pub file_name: String,
    pub mime_type: String,
    pub size: u64,
}

impl SharedFile {
    pub fn export_b64_url(&self) -> Result<String> {
        Ok(URL_SAFE.encode(serde_json::to_string(&self.payload)?.as_bytes()))
    }
    
    pub fn import_b64_url(b64_string: String) -> Result<Self> {
        let bytes = URL_SAFE.decode(b64_string)?;
        let json = String::from_utf8(bytes)?;
        println!("json: {:?}", json);
        let payload: SharePayload = serde_json::from_str(&json)?;
        // Ok()
        println!("payload: {:?}", payload);
        Err(anyhow!("dsfsdf"))
    }
}