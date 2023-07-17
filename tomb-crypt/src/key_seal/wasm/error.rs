use wasm_bindgen::JsValue;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
#[non_exhaustive]
pub struct KeySealError {
    kind: KeySealErrorKind,
}

impl KeySealError {
    pub(crate) fn subtle_crypto_unavailable(err: JsValue) -> Self {
        Self {
            kind: KeySealErrorKind::SubtleCryptoUnavailable(err),
        }
    }

    pub(crate) fn subtle_crypto_error(err: JsValue) -> Self {
        Self {
            kind: KeySealErrorKind::SubtleCryptoError(err),
        }
    }

    pub(crate) fn bad_format(err: JsValue) -> Self {
        Self {
            kind: KeySealErrorKind::BadFormat(err),
        }
    }

    pub(crate) fn bad_base64(err: JsValue) -> Self {
        Self {
            kind: KeySealErrorKind::InvalidBase64(err),
        }
    }

    pub(crate) fn export_failed(err: JsValue) -> Self {
        Self {
            kind: KeySealErrorKind::ExportFailed(err),
        }
    }
}

impl Display for KeySealError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use KeySealErrorKind::*;

        match &self.kind {
            SubtleCryptoUnavailable(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "SubtleCrypto is not available: {msg}")
            },
            SubtleCryptoError(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "SubtleCrypto error: {msg}")
            },
            BadFormat(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "imported key was malformed: {msg}")
            },
            ExportFailed(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "failed to export key: {msg}")
            },
            InvalidBase64(err) => {
                let msg = err.as_string().unwrap();
                write!(f, "invalid base64: {msg}")
            }
        }
    }
}

impl From<KeySealError> for JsValue {
    fn from(err: KeySealError) -> Self {
        use KeySealErrorKind::*;

        match err.kind {
            SubtleCryptoUnavailable(err) => err,
            SubtleCryptoError(err) => err,
            BadFormat(err) => err,
            ExportFailed(err) => err,
            InvalidBase64(err) => err,
        }
    }
}

impl std::error::Error for KeySealError {}

#[derive(Debug)]
#[non_exhaustive]
enum KeySealErrorKind {
    SubtleCryptoUnavailable(JsValue),
    SubtleCryptoError(JsValue),
    BadFormat(JsValue),
    ExportFailed(JsValue),
    InvalidBase64(JsValue),
}
