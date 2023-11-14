pub(crate) struct FilesystemError {
    pub kind: FilesystemErrorKind,
}

impl FilesystemError {
    pub(crate) fn node_not_found(path: &str) -> Self {
        Self {
            kind: FilesystemErrorKind::NodeNotFound(path.to_string()),
        }
    }

    pub(crate) fn missing_metadata(label: &str) -> Self {
        Self {
            kind: FilesystemErrorKind::MissingMetadata(label.to_string()),
        }
    }
}

pub(crate) enum FilesystemErrorKind {
    MissingMetadata(String),
    NodeNotFound(String),
    BadConfig,
}
