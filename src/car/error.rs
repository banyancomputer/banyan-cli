#[derive(Debug)]
pub(crate) struct CarError {
    pub kind: CarErrorKind,
}

impl CarError {
    pub fn missing_root() -> Self {
        Self {
            kind: CarErrorKind::MissingRoot,
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
}

#[derive(Debug)]
pub enum CarErrorKind {
    /// No Root Cid even though expected
    MissingRoot,
    /// The CARv1 Header was not correct
    V1Header,
    /// The CARv2 Index was not correct
    Index,
    /// Index codec
    Codec,
    /// Index codec
    EndOfData,
}
