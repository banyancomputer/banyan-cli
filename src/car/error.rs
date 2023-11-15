use std::fmt::Display;

use colored::Colorize;
use wnfs::libipld::Cid;

use crate::utils::UtilityError;

#[derive(Debug)]
pub struct CarError {
    kind: CarErrorKind,
}

impl CarError {
    pub fn missing_root() -> Self {
        Self {
            kind: CarErrorKind::MissingRoot,
        }
    }

    pub fn missing_block(cid: &Cid) -> Self {
        Self {
            kind: CarErrorKind::MissingBlock(cid.to_owned()),
        }
    }

    pub fn v1_header() -> Self {
        Self {
            kind: CarErrorKind::V1Header,
        }
    }

    pub fn index() -> Self {
        Self {
            kind: CarErrorKind::Index,
        }
    }

    pub fn codec() -> Self {
        Self {
            kind: CarErrorKind::Codec,
        }
    }

    pub fn end_of_data() -> Self {
        Self {
            kind: CarErrorKind::EndOfData,
        }
    }

    pub fn io_error(err: std::io::Error) -> Self {
        Self {
            kind: CarErrorKind::Io(err),
        }
    }

    pub fn cid_error(err: wnfs::libipld::cid::Error) -> Self {
        Self {
            kind: CarErrorKind::Cid(err),
        }
    }

    pub fn utility_error(err: UtilityError) -> Self {
        Self {
            kind: CarErrorKind::Utility(err),
        }
    }
}

impl Display for CarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            CarErrorKind::MissingRoot => "Missing Root CID".to_owned(),
            CarErrorKind::MissingBlock(cid) => format!("Missing Block with CID: {}", cid),
            CarErrorKind::V1Header => "Malformed CARv1 Header".to_owned(),
            CarErrorKind::Index => "Malformed CARv2 Index".to_owned(),
            CarErrorKind::Codec => "Invalid Index Codec".to_owned(),
            CarErrorKind::EndOfData => "Expected more data in CAR than was present".to_owned(),
            CarErrorKind::Cid(err) => format!("{} {err}", "CID ERROR:".underline()),
            CarErrorKind::Io(err) => format!("{} {err}", "IO ERROR:".underline()),
            CarErrorKind::Utility(err) => format!("{} {err}", "UTILITY ERROR:".underline()),
        };

        f.write_str(&string)
    }
}

#[derive(Debug)]
pub enum CarErrorKind {
    /// No Root Cid even though expected
    MissingRoot,
    MissingBlock(Cid),
    /// The CARv1 Header was not correct
    V1Header,
    /// The CARv2 Index was not correct
    Index,
    /// Index codec
    Codec,
    /// Index codec
    EndOfData,
    Io(std::io::Error),
    Cid(wnfs::libipld::cid::Error),
    Utility(UtilityError),
}

impl From<std::io::Error> for CarError {
    fn from(value: std::io::Error) -> Self {
        Self::io_error(value)
    }
}

impl From<wnfs::libipld::cid::Error> for CarError {
    fn from(value: wnfs::libipld::cid::Error) -> Self {
        Self::cid_error(value)
    }
}
impl From<UtilityError> for CarError {
    fn from(value: UtilityError) -> Self {
        Self::utility_error(value)
    }
}

impl From<CarError> for anyhow::Error {
    fn from(value: CarError) -> Self {
        anyhow::anyhow!("car error: {:?}", value)
    }
}
