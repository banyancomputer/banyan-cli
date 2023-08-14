use std::fmt::Debug;
use tomb_crypt::prelude::*;
use uuid::Uuid;

pub struct Credentials {
    pub account_id: Uuid,
    pub signing_key: EcSignatureKey,
}

impl Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Get the pem string for the signing key
        f.debug_struct("Credentials")
            .field("account_id", &self.account_id)
            .finish()
    }
}
