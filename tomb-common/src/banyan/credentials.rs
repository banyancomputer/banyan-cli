use std::fmt::Debug;
use tomb_crypt::prelude::*;
use uuid::Uuid;

#[derive(Clone)]
/// Credentials in order to sign and verify messages for a Banyan account
pub struct Credentials {
    /// The unique account id (used as a JWT subject)
    pub account_id: Uuid,
    /// The signing key (used to sign JWTs)
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
