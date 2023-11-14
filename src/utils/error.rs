#[derive(Debug)]
pub(crate) struct UtilityError {
    pub(crate) kind: UtilityErrorKind,
}

impl UtilityError {
    pub(crate) fn varint(err: unsigned_varint::decode::Error) -> Self {
        Self {
            kind: UtilityErrorKind::Varint(err),
        }
    }

    pub(crate) fn io(err: std::io::Error) -> Self {
        Self {
            kind: UtilityErrorKind::Io(err),
        }
    }
}

#[derive(Debug)]
enum UtilityErrorKind {
    Varint(unsigned_varint::decode::Error),
    Io(std::io::Error),
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
