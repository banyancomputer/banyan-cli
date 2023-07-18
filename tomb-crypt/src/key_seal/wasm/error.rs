use base64::DecodeError;
use js_sys::Error as JsError;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
#[non_exhaustive]
pub struct KeySealError {
    kind: KeySealErrorKind,
}

impl KeySealError {
    pub(crate) fn crypto_unavailable(err: JsError) -> Self {
        Self {
            kind: KeySealErrorKind::CryptoUnavailable(err),
        }
    }

    pub(crate) fn subtle_crypto_error(err: JsError) -> Self {
        Self {
            kind: KeySealErrorKind::SubtleCryptoError(err),
        }
    }

    pub(crate) fn public_key_unavailable() -> Self {
        Self {
            kind: KeySealErrorKind::PublicKeyUnavailable(JsError::new(
                "public key was not imported",
            )),
        }
    }

    pub(crate) fn bad_format(err: JsError) -> Self {
        Self {
            kind: KeySealErrorKind::BadFormat(err),
        }
    }

    pub(crate) fn bad_base64(err: DecodeError) -> Self {
        Self {
            kind: KeySealErrorKind::InvalidBase64(err),
        }
    }

    pub(crate) fn export_failed(err: JsError) -> Self {
        Self {
            kind: KeySealErrorKind::ExportFailed(err),
        }
    }
}

impl Display for KeySealError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use KeySealErrorKind::*;

        match &self.kind {
            CryptoUnavailable(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "SubtleCrypto is not available: {msg}")
            }
            SubtleCryptoError(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "SubtleCrypto error: {msg}")
            }
            PublicKeyUnavailable(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "public key was not imported: {msg}")
            }
            BadFormat(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "imported key was malformed: {msg}")
            }
            ExportFailed(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "failed to export key: {msg}")
            }
            InvalidBase64(err) => {
                let msg = err.to_string();
                write!(f, "invalid base64: {msg}")
            }
        }
    }
}

impl From<KeySealError> for JsError {
    fn from(err: KeySealError) -> Self {
        use KeySealErrorKind::*;

        match err.kind {
            CryptoUnavailable(err) => err,
            SubtleCryptoError(err) => err,
            PublicKeyUnavailable(err) => err,
            BadFormat(err) => err,
            ExportFailed(err) => err,
            InvalidBase64(err) => JsError::new(&err.to_string()),
        }
    }
}

impl From<JsError> for KeySealError {
    fn from(err: JsError) -> Self {
        Self::subtle_crypto_error(err)
    }
}

impl std::error::Error for KeySealError {}

#[derive(Debug)]
#[non_exhaustive]
enum KeySealErrorKind {
    CryptoUnavailable(JsError),
    SubtleCryptoError(JsError),
    PublicKeyUnavailable(JsError),
    BadFormat(JsError),
    ExportFailed(JsError),
    InvalidBase64(DecodeError),
}
