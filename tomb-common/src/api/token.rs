use jsonwebtoken::{get_current_timestamp, Algorithm, EncodingKey, Header};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Token {
    #[serde(rename = "nnc")]
    nonce: String,

    #[serde(rename = "aud")]
    audience: String,

    #[serde(rename = "sub")]
    subject: String,

    #[serde(rename = "exp")]
    expiration: u64,

    #[serde(rename = "nbf")]
    not_before: u64,
}

impl Token {
    pub(crate) fn expiration(&self) -> u64 {
        self.expiration
    }

    pub(crate) fn new(audience: &str, subject: &str) -> Self {
        let nonce = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        Self {
            nonce,

            audience: audience.to_string(),
            subject: subject.to_string(),

            expiration: get_current_timestamp() + 870,
            not_before: get_current_timestamp() - 30,
        }
    }

    pub(crate) fn sign(&self, fingerprint: &str, signing_key: &EncodingKey) -> String {
        let bearer_header = Header {
            alg: Algorithm::ES384,
            kid: Some(fingerprint.to_string()),
            ..Default::default()
        };

        jsonwebtoken::encode(&bearer_header, &self, signing_key).unwrap()
    }
}
