use std::fmt::Display;

use colored::Colorize;
use tomb_crypt::prelude::TombCryptError;

#[derive(Debug)]
pub struct SharingError {
    kind: SharingErrorKind,
}

impl SharingError {
    pub fn unauthorized() -> Self {
        Self {
            kind: SharingErrorKind::UnauthorizedDecryption,
        }
    }

    pub fn lost_key() -> Self {
        Self {
            kind: SharingErrorKind::LostKey,
        }
    }

    pub fn invalid_key_data(message: &str) -> Self {
        Self {
            kind: SharingErrorKind::InvalidKeyData(message.to_string()),
        }
    }

    pub fn cryptographic(err: TombCryptError) -> Self {
        Self {
            kind: SharingErrorKind::Cryptographic(err),
        }
    }

    pub fn encoding(msg: &str) -> Self {
        Self {
            kind: SharingErrorKind::BadData(msg.to_owned()),
        }
    }
}

impl Display for SharingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            SharingErrorKind::UnauthorizedDecryption => {
                "You are not authorized to decrypt this Drive, request key access first.".to_owned()
            }
            SharingErrorKind::LostKey => "Lost track of a very important Key".to_owned(),
            SharingErrorKind::InvalidKeyData(message) => format!("Invalid key data: {message}"),
            SharingErrorKind::Cryptographic(err) => {
                format!("{} {err}", "CRYPTOGRAPHIC ERROR:".underline())
            }
            SharingErrorKind::BadData(msg) => format!("Invalid data: {msg}"),
        };

        f.write_str(&string)
    }
}

#[derive(Debug)]
pub enum SharingErrorKind {
    UnauthorizedDecryption,
    LostKey,
    InvalidKeyData(String),
    Cryptographic(TombCryptError),
    BadData(String),
}

impl From<TombCryptError> for SharingError {
    fn from(value: TombCryptError) -> Self {
        Self::cryptographic(value)
    }
}

impl From<serde_json::Error> for SharingError {
    fn from(value: serde_json::Error) -> Self {
        Self::encoding(&value.to_string())
    }
}
