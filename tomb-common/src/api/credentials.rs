use std::fmt::Debug;

use jsonwebtoken::EncodingKey;
use uuid::Uuid;

pub struct Credentials {
    pub account_id: Uuid,
    pub fingerprint: String,
    pub signing_key: EncodingKey,
}

impl Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("account_id", &self.account_id)
            .field("fingerprint", &self.fingerprint)
            // .field("signing_key", &self.signing_key)
            .finish()
    }
}
