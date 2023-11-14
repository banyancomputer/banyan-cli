#[derive(Debug)]
pub(crate) struct UtilityError {
    pub(crate) kind: UtilityErrorKind,
}

impl UtilityError {
    pub(crate) fn varint_error(err: unsigned_varint::decode::Error) -> Self {
        Self {
            kind: UtilityErrorKind::Varint(err),
        }
    }
}

enum UtilityErrorKind {
    Varint(unsigned_varint::decode::Error),
}

impl From<unsigned_varint::decode::Error> for UtilityError {
    fn from(value: unsigned_varint::decode::Error) -> Self {
        Self::varint_error(value)
    }
}
