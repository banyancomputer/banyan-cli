// use crate::{native::configuration::ConfigurationError, filesystem::FilesystemError, blockstore::BlockStoreError};

// #[derive(Debug)]
// pub(crate) struct NativeError {
//     kind: NativeErrorKind,
// }

// impl NativeError {
//     pub(crate) fn configuration(err: ConfigurationError) -> Self {
//         Self {
//             kind: NativeErrorKind::Configuration(err)
//         }
//     }

//     pub(crate) fn filesystem(err: FilesystemError) -> Self {
//         Self {
//             kind: NativeErrorKind::Filesystem(err)
//         }
//     }
// }

// #[derive(Debug)]
// enum NativeErrorKind {
//     Configuration(ConfigurationError),
//     Filesystem(FilesystemError)
// }

// impl From<ConfigurationError> for NativeError {
//     fn from(value: ConfigurationError) -> Self {
//         Self::configuration(value)
//     }
// }

// impl From<FilesystemError> for NativeError {
//     fn from(value: FilesystemError) -> Self {
//         Self::filesystem(value)
//     }
// }

// impl From<std::io::Error> for NativeError {
//     fn from(value: std::io::Error) -> Self {
//         Self::filesystem(FilesystemError::io(value))
//     }
// }

// impl From<anyhow::Error> for NativeError {
//     fn from(value: anyhow::Error) -> Self {
//         Self::filesystem(FilesystemError::wnfs(value))
//     }
// }

// impl From<BlockStoreError> for NativeError {
//     fn from(value: BlockStoreError) -> Self {
//         Self::filesystem(FilesystemError::blockstore(value))
//     }
// }
