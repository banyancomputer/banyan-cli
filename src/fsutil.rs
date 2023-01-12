use crate::fs_iterator::FilesystemIterator;
use anyhow::{anyhow, Result};
use tokio_stream::{Stream, StreamExt, StreamMap};

use std::path::{Path, PathBuf};

pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        // create path if it doesn't exist
        std::fs::create_dir_all(path)?;
    }
    if !path.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }
    Ok(())
}

pub fn ensure_path_exists_and_is_empty_dir(path: &Path) -> Result<()> {
    ensure_path_exists_and_is_dir(path)?;
    if path.read_dir().unwrap().count() > 0 {
        return Err(anyhow!("Path is not empty: {}", path.display()));
    }
    Ok(())
}

async fn copy_file_or_dir(from: PathBuf, to_root: PathBuf) -> Result<PathBuf> {
    let new_path = to_root.join(from.strip_prefix("/")?);
    if from.is_dir() {
        tokio::fs::create_dir_all(&new_path).await?;
    } else {
        tokio::fs::copy(from, &new_path).await?;
    }
    Ok(new_path)
}

// TODO don't copy around pathbufs you utter trainwreck
// TODO make a nice generalized function. make an iterable trait i guess.
async fn copy_paths_recursively<'a>(
    paths: Vec<PathBuf>,
    scratch_root: PathBuf,
    follow_symlinks: bool,
) -> impl Stream<Item = (Option<&'a str>, Result<PathBuf>)> + Unpin {
    // TODO do the multiplexing correctly
    let mut map = StreamMap::new();
    for path in paths {
        let fsi = FilesystemIterator::new(path, follow_symlinks).await;
            map.insert(path.to_str(), fsi.then(|res| async move {
                copy_file_or_dir(res, scratch_root.clone()).await
            }));
    };
    Box::pin(map)
}
