use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataState {
    Pending,
    Current,
    Outdated,
    Deleted,
}

#[derive(Debug, Deserialize, Serialize)]
/// Metadata Definition
pub struct Metadata {
    pub id: Option<uuid::Uuid>,
    pub bucket_id: Option<uuid::Uuid>,
    pub state: Option<MetadataState>,
}
