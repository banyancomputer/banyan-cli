use anyhow::Result;
use std::{fs::File, io, path::PathBuf};

use blake2::{Blake2s256, Digest};

/// Hash a file using blake2s256
pub fn hash_file(path: &PathBuf) -> Result<String> {
    let mut hasher = Blake2s256::new();
    let mut file = File::open(path)?;
    io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize()[..]
        .to_vec()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
