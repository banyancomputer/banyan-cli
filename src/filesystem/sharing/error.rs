use std::fmt::Display;

use tomb_crypt::prelude::TombCryptError;

pub(crate) struct SharingError {
    pub kind: SharingErrorKind,
}

impl SharingError {
    pub(crate) fn unauthorized() -> Self {
        Self {
            kind: SharingErrorKind::UnauthorizedDecryption,
        }
    }

    pub(crate) fn lost_key() -> Self {
        Self {
            kind: SharingErrorKind::LostKey,
        }
    }

    pub(crate) fn invalid_key_data(message: &str) -> Self {
        Self {
            kind: SharingErrorKind::InvalidKeyData(message.to_string()),
        }
    }
}

pub(crate) enum SharingErrorKind {
    UnauthorizedDecryption,
    LostKey,
    InvalidKeyData(String),
    Cryptographic(TombCryptError),
}

impl Display for SharingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args = match self.kind {
            SharingErrorKind::UnauthorizedDecryption => format_args!(
                "You are not authorized to decrypt this Drive, request key access first."
            ),
            SharingErrorKind::LostKey => format_args!("expected to find a key but didnt"),
            SharingErrorKind::InvalidKeyData(message) => {
                format_args!("key data invalid: {}", message)
            }
            SharingErrorKind::Cryptographic(err) => format_args!("crypto error: {}", err),
        };

        f.write_fmt(args)
    }
}

impl From<TombCryptError> for SharingError {
    fn from(value: TombCryptError) -> Self {
        Self {
            kind: SharingErrorKind::Cryptographic(value),
        }
    }
}
