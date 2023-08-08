use jsonwebtoken::EncodingKey;
use uuid::Uuid;

pub struct Credentials {
    pub account_id: Uuid,
    pub fingerprint: String,
    pub signing_key: EncodingKey,
}