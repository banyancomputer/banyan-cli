use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BucketKey {
    pub id: Option<uuid::Uuid>,
    pub bucket_id: Option<uuid::Uuid>,
    pub pem: Option<String>,
    pub approved: Option<bool>,
}

impl BucketKey {
    pub fn new(bucket_id: uuid::Uuid, pem: String) -> Self {
        Self {
            id: None,
            bucket_id: Some(bucket_id),
            pem: Some(pem),
            approved: Some(false),
        }
    }

    pub fn bucket_id(&self) -> Result<uuid::Uuid, RequestableError> {
        self.bucket_id.clone().ok_or(RequestableError::missing_field("bucket_id".into()))
    }

    pub fn pem(&self) -> Result<String, RequestableError> {
        self.pem.clone().ok_or(RequestableError::missing_field("pem".into()))
    }

    pub fn approved(&self) -> Result<bool, RequestableError> {
        self.approved.clone().ok_or(RequestableError::missing_field("approved".into()))
    }
}