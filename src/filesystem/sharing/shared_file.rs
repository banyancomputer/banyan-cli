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
