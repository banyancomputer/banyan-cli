use std::{str::Utf8Error, string::FromUtf8Error};

#[cfg(test)]
use crate::native::NativeError;

#[derive(Debug)]
pub struct UtilityError {
    pub kind: UtilityErrorKind,
}

impl UtilityError {
    pub fn varint(err: unsigned_varint::decode::Error) -> Self {
        Self {
            kind: UtilityErrorKind::Varint(err),
        }
    }

    pub fn io(err: std::io::Error) -> Self {
        Self {
            kind: UtilityErrorKind::Io(err),
        }
    }

    pub fn utf8(err: FromUtf8Error) -> Self {
        Self {
            kind: UtilityErrorKind::Utf8(err),
        }
    }

    pub fn custom(msg: &str) -> Self {
        Self {
            kind: UtilityErrorKind::Custom(msg.to_owned()),
        }
    }

    #[cfg(test)]
    pub fn native(err: NativeError) -> Self {
        Self {
            kind: UtilityErrorKind::Native(err),
        }
    }
}

#[derive(Debug)]
pub enum UtilityErrorKind {
    Varint(unsigned_varint::decode::Error),
    Io(std::io::Error),
    Utf8(FromUtf8Error),
    Custom(String),
    #[cfg(test)]
    Native(NativeError),
}

impl From<unsigned_varint::decode::Error> for UtilityError {
    fn from(value: unsigned_varint::decode::Error) -> Self {
        Self::varint(value)
    }
}

impl From<std::io::Error> for UtilityError {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}

impl From<FromUtf8Error> for UtilityError {
    fn from(value: FromUtf8Error) -> Self {
        Self::utf8(value)
    }
}

impl From<anyhow::Error> for UtilityError {
    fn from(value: anyhow::Error) -> Self {
        Self::custom(&value.to_string())
    }
}

#[cfg(test)]
impl From<NativeError> for UtilityError {
    fn from(value: NativeError) -> Self {
        Self::native(value)
    }
}
