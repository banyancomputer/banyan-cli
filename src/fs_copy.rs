use anyhow::Result;
use std::path::PathBuf;

use crate::fs_iterator::FilesystemIterator;
use tokio_stream::{Stream, StreamExt, StreamMap};

pub struct CopyMetadata {
    pub(crate) original_file: PathBuf,
    pub(crate) new_file: PathBuf,
}

#[async_recursion::async_recursion]
async fn copy_file_or_dir(from: PathBuf, to_root: PathBuf) -> Result<PathBuf> {
    let new_path = to_root.join(from.strip_prefix("/")?);
    if from.is_dir() {
        tokio::fs::create_dir_all(&new_path).await?;
    } else if from.is_symlink() {
        // follow the symlink and copy what's there
        let target = std::fs::read_link(&from)?;
        copy_file_or_dir(target, to_root).await?;
    } else {
        tokio::fs::copy(from, &new_path).await?;
    }
    Ok(new_path)
}

pub(crate) async fn copy_paths_recursively(
    paths: Vec<PathBuf>,
    scratch_root: PathBuf,
    follow_symlinks: bool,
) -> impl Stream<Item = Result<CopyMetadata>> + Unpin {
    let mut map = StreamMap::new();
    for path in paths {
        let fsi = FilesystemIterator::new(path.clone(), follow_symlinks).await;
        // TODO sucks why are you cloning twice
        let scratch_root = scratch_root.clone();
        let copy_file = Box::pin(fsi.then(move |res| copy_file_or_dir(res, scratch_root.clone())));
        map.insert(path, copy_file);
    }
    map.map(|(path, res)| {
        res.map(|new_path| CopyMetadata {
            original_file: path,
            new_file: new_path,
        })
    })
}
