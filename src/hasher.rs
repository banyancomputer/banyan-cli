use anyhow::Result;
use blake2::{Blake2s256, Digest};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

pub async fn hash_file(path: &PathBuf) -> Result<String> {
    let mut hasher = Blake2s256::new();
    let mut file = tokio::fs::File::open(path).await?;
    let mut buf = [0u8; 4096];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize()[..]
        .to_vec()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
