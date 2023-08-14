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
    pub id: Option<uuid::Uuid>, 
    pub name: Option<String>,
    pub r#type: Option<BucketType>,
}

impl Bucket {
    pub fn new(name: String, r#type: BucketType) -> Self {
        Self {
            id: None,
            name: Some(name),
            r#type: Some(r#type),
        }
    }
}
