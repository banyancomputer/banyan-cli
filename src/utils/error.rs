use colored::Colorize;
use std::{fmt::Display, string::FromUtf8Error};

#[cfg(test)]
use crate::{filesystem::FilesystemError, native::NativeError};

#[derive(Debug)]
pub struct UtilityError {
    kind: UtilityErrorKind,
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

impl Display for UtilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            UtilityErrorKind::Varint(err) => format!("{} {err}", "VARINT ERROR:".underline()),
            UtilityErrorKind::Io(err) => format!("{} {err}", "IO ERROR:".underline()),
            UtilityErrorKind::Utf8(err) => format!("{} {err}", "UTF8 ERROR:".underline()),
            #[cfg(test)]
            UtilityErrorKind::Native(err) => format!("{} {err}", "NATIVE ERROR:".underline()),
            UtilityErrorKind::Custom(msg) => msg.to_owned(),
        };

        f.write_str(&string)
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

#[cfg(test)]
impl From<FilesystemError> for UtilityError {
    fn from(value: FilesystemError) -> Self {
        Self::native(NativeError::filesytem(value))
    }
}
