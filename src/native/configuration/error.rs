// use std::{fmt::Display, fs::File};

// use tomb_crypt::prelude::TombCryptError;

// use crate::{filesystem::FilesystemError, car::error::CarError, blockstore::BlockStoreError};

// #[derive(Debug)]
// pub(crate) struct SyncError {
//     kind: SyncErrorKind,
// }

// impl SyncError {
    

//     pub(crate) fn cryptographic(err: TombCryptError) -> Self {
//         Self {
//             kind: SyncErrorKind::Cryptographic(err)
//         }
//     }

//     pub(crate) fn io(err: std::io::Error) -> Self {
//         Self {
//             kind: SyncErrorKind::Io(err)
//         }
//     }

// }

// impl Display for SyncError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let args = match self.kind {
//             SyncErrorKind::MissingCredentials => todo!(),
//             SyncErrorKind::MissingIdentifier => {
//                 format_args!("Unable to find a remote Identifier associated with that Drive")
//             }
//             SyncErrorKind::MissingLocalDrive => {
//                 format_args!("Unable to find a local Drive with that query")
//             }
//             SyncErrorKind::MissingRemoteDrive => {
//                 format_args!("Unable to find a remote Drive with that query")
//             }
//             SyncErrorKind::UniqueDriveError => {
//                 format_args!("There is already a unique Drive with these specs")
//             }
//             SyncErrorKind::Io(_) => todo!(),
//             SyncErrorKind::Cryptographic(_) => todo!(),
//             SyncErrorKind::BadData => todo!(),
//         };
//         f.write_fmt(args)
//     }
// }

// #[derive(Debug)]
// enum SyncErrorKind {
    
//     Io(std::io::Error),
//     Cryptographic(TombCryptError),
// }

// impl From<std::io::Error> for SyncError {
//     fn from(value: std::io::Error) -> Self {
//         Self::io(value)
//     }
// }

// impl From<TombCryptError> for SyncError {
//     fn from(value: TombCryptError) -> Self {
//         Self::cryptographic(value)
//     }
// }
