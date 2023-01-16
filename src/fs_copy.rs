use anyhow::Result;
use std::collections::HashMap;

use jwalk::DirEntry;
use std::fs::Metadata;
use std::path::PathBuf;

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use crate::hasher;

pub enum DuplicateOrOriginal {
    Duplicate(PathBuf),
    Original,
}

impl DuplicateOrOriginal {
    fn is_original(&self) -> bool {
        match self {
            DuplicateOrOriginal::Duplicate(_) => false,
            DuplicateOrOriginal::Original => true,
        }
    }
}

pub struct CopyMetadata {
    pub(crate) original_root: PathBuf,
    pub(crate) original_location: DirEntry<(u64, Option<u64>)>,
    pub(crate) original_metadata: Metadata,
    pub(crate) new_location: PathBuf,
    pub(crate) blake2_file_hash: Option<String>,
    pub(crate) duplicate_or_original: DuplicateOrOriginal,
}

pub async fn copy_file_or_dir(
    original_root: PathBuf,
    original_location: DirEntry<(u64, Option<u64>)>,
    to_root: PathBuf,
    seen_hashes: Arc<RwLock<HashMap<String, PathBuf>>>,
) -> Result<CopyMetadata> {
    let new_path = to_root.join(original_location.path().strip_prefix(&original_root)?);
    let original_metadata = original_location.metadata()?;
    if original_metadata.is_dir() {
        tokio::fs::create_dir_all(&new_path).await?;
        Ok(CopyMetadata {
            original_root,
            original_location,
            original_metadata: original_metadata.clone(),
            new_location: new_path,
            blake2_file_hash: None,
            duplicate_or_original: DuplicateOrOriginal::Original,
        })
    } else if original_metadata.is_symlink() {
        panic!("symlinks not supported");
        // TODO currently, reading symlinks from the filesystem looks... annoying. come back to this later.
        // when we DO do this, you'll need to touch the destination file if it hasn't been passed over yet, then set the symlink
        // then fill in the destination file later as this iterator continues...
    } else {
        tokio::fs::create_dir_all(new_path.parent().unwrap()).await?;
        // compute the file hash
        let file_hash = hasher::hash_file(&original_location.path()).await?;
        // check if we've seen this file before
        let duplicate_or_original = {
            let mut seen_hashes = seen_hashes.write().await;
            if let Some(duplicate_path) = seen_hashes.get(&file_hash) {
                DuplicateOrOriginal::Duplicate(duplicate_path.clone())
            } else {
                seen_hashes.insert(file_hash.clone(), new_path.clone());
                DuplicateOrOriginal::Original
            }
        };
        // do the copy if this file is new
        if duplicate_or_original.is_original() {
            // copy the file
            let mut original_file = tokio::fs::File::open(&original_location.path()).await?;
            let mut new_file = tokio::fs::File::create(&new_path).await?;
            let mut buf = [0u8; 1024 * 1024];
            loop {
                let n = original_file.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                let n2 = new_file.write(&buf[..n]).await?;
                assert_eq!(n, n2);
            }
            {
                // lock guards
                let mut seen_hashes = seen_hashes.write().await;
                seen_hashes.insert(file_hash.clone(), new_path.clone());
            }
        }

        Ok(CopyMetadata {
            original_root,
            original_location,
            original_metadata,
            new_location: new_path,
            duplicate_or_original,
            blake2_file_hash: Some(file_hash),
        })
    }
}
