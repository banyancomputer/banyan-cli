use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
#[non_exhaustive]
pub struct KeySealError {
    kind: KeySealErrorKind,
}

impl KeySealError {
    pub(crate) fn bad_format(err: openssl::error::ErrorStack) -> Self {
        Self {
            kind: KeySealErrorKind::BadFormat(err),
        }
    }

    pub(crate) fn bad_base64(err: base64::DecodeError) -> Self {
        Self {
            kind: KeySealErrorKind::InvalidBase64(err),
        }
    }

    pub(crate) fn export_failed(err: openssl::error::ErrorStack) -> Self {
        Self {
            kind: KeySealErrorKind::ExportFailed(err),
        }
    }

    pub(crate) fn incompatble_derivation(err: openssl::error::ErrorStack) -> Self {
        Self {
            kind: KeySealErrorKind::IncompatibleDerivationKey(err),
        }
    }
}

impl Display for KeySealError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use KeySealErrorKind::*;

        let msg = match &self.kind {
            BadFormat(_) => "imported key was malformed",
            ExportFailed(_) => "attempt to export key was rejected by underlying library",
            _ => "placeholder",
        };

        f.write_str(msg)
    }
}

impl std::error::Error for KeySealError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use KeySealErrorKind::*;

        match &self.kind {
            BadFormat(err) => Some(err),
            ExportFailed(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
enum KeySealErrorKind {
    BadFormat(openssl::error::ErrorStack),
    ExportFailed(openssl::error::ErrorStack),
    InvalidBase64(base64::DecodeError),
    IncompatibleDerivationKey(openssl::error::ErrorStack),
}
