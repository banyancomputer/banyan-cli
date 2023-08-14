use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
/// Possible types of Bucket
pub enum BucketType {
    Backup,
    Interactive,
}

#[derive(Debug, Deserialize, Serialize)]
/// Bucket Definition
pub struct Bucket {
    pub id: String,
    pub name: String,
    pub r#type: BucketType,
}

#[derive(Debug, Deserialize, Serialize)]
/// Account Definition
pub struct Account {
    pub id: String,
    // TODO: Add more fields
}

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
    pub id: String,
    pub bucket_id: String,
    pub state: MetadataState,
}
