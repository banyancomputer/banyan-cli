use anyhow::Result;
use std::{
    io::{Read, Write},
    path::Path,
    rc::Rc,
};
use tomb_common::{types::pipeline::Manifest, utils::serialize::*};
use wnfs::{
    common::dagcbor,
    private::{AesKey, PrivateDirectory, PrivateForest, TemporalKey},
};

/// Store a Manifest
pub fn manifest_to_disk(tomb_path: &Path, manifest: &Manifest) -> Result<()> {
    // The path in which we expect to find the Manifest JSON file
    let manifest_file = tomb_path.join("manifest.cbor");

    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let mut manifest_writer = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&manifest_file)
    {
        Ok(f) => f,
        Err(e) => Err(anyhow::anyhow!(
            "Failed to create manifest file at {}: {}",
            manifest_file.display(),
            e
        ))?,
    };

    info!(
        "📄 Writing out a data manifest file to {}",
        manifest_file.display()
    );

    // Write the manifest in DAG CBOR
    manifest_writer
        .write_all(&dagcbor::encode(&manifest)?)
        .map_err(anyhow::Error::new)
}

/// Deserializes the Manifest struct from a given .tomb dir
pub fn manifest_from_disk(tomb_path: &Path) -> Result<Manifest> {
    info!("Loading in Manifest from disk");
    let manifest_file = tomb_path.join("manifest.cbor");

    // Read in the manifest file from the metadata path
    let mut manifest_reader = std::fs::File::open(manifest_file)
        .map_err(|e| anyhow::anyhow!("Failed to open manifest file: {}", e))?;

    let mut manifest_buf: Vec<u8> = Vec::new();
    manifest_reader.read_to_end(&mut manifest_buf)?;

    // Deserialize the data read as the latest version of manifestdata
    let manifest: Manifest = match dagcbor::decode(&manifest_buf) {
        Ok(data) => data,
        Err(e) => {
            panic!("Failed to deserialize manifest file: {e}");
        }
    };

    // If the major version of the manifest is not the same as the major version of the program
    if manifest.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        panic!(
            "Unsupported manifest version. Using {} but found {}",
            env!("CARGO_PKG_VERSION"),
            manifest.version
        );
    }

    Ok(manifest)
}

/// Store a TemporalKey
pub fn key_to_disk(tomb_path: &Path, temporal_key: &TemporalKey, label: &str) -> Result<()> {
    // The path in which we expect to find the Manifest JSON file
    let key_file = tomb_path.join(format!("{}.key", label));
    let mut key_writer = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&key_file)
    {
        Ok(f) => f,
        Err(e) => Err(anyhow::anyhow!(
            "Failed to create key file at {}: {}",
            key_file.display(),
            e
        ))?,
    };

    // Write the key
    key_writer.write_all(temporal_key.0.as_bytes())?;

    Ok(())
}
/// Load a TemporalKey
pub fn key_from_disk(tomb_path: &Path, label: &str) -> Result<TemporalKey> {
    info!("Loading in {} Key from disk", label);
    // The path in which we expect to find the Manifest JSON file
    let key_file = tomb_path.join(format!("{}.key", label));

    // Read in the key file from the key path
    let mut key_reader = std::fs::File::open(key_file)
        .map_err(|e| anyhow::anyhow!("Failed to open key file: {}", e))?;
    // Deserialize the data read as the latest version of manifestdata
    let mut key_data: [u8; 32] = [0; 32];
    key_reader.read_exact(&mut key_data)?;
    let key: TemporalKey = TemporalKey(AesKey::new(key_data));

    Ok(key)
}

/// Store everything at once!
pub async fn all_to_disk(
    tomb_path: &Path,
    manifest: &mut Manifest,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    let temporal_key = store_all(manifest, metadata_forest, content_forest, root_dir).await?;
    manifest_to_disk(tomb_path, manifest)?;
    key_to_disk(tomb_path, &temporal_key, "root")?;
    Ok(temporal_key)
}

/// Load everything at once!
pub async fn all_from_disk(
    tomb_path: &Path,
) -> Result<(
    TemporalKey,
    Manifest,
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let key = key_from_disk(tomb_path, "root")?;
    let manifest = manifest_from_disk(tomb_path)?;
    let (metadata_forest, content_forest, dir) = load_all(&key, &manifest).await?;
    Ok((key, manifest, metadata_forest, content_forest, dir))
}

/// Store all hot objects!
pub async fn hot_to_disk(
    tomb_path: &Path,
    manifest: &mut Manifest,
    metadata_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    let temporal_key = store_all_hot(manifest, metadata_forest, root_dir).await?;
    manifest_to_disk(tomb_path, manifest)?;
    key_to_disk(tomb_path, &temporal_key, "root")?;
    Ok(temporal_key)
}

/// Load all hot objects!
pub async fn hot_from_disk(
    tomb_path: &Path,
) -> Result<(
    TemporalKey,
    Manifest,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let key = key_from_disk(tomb_path, "root")?;
    let manifest = manifest_from_disk(tomb_path)?;
    let (metadata_forest, dir) = load_all_hot(&key, &manifest).await?;
    Ok((key, manifest, metadata_forest, dir))
}

#[cfg(test)]
mod test {
    use crate::utils::{
        disk::*,
    };
    use anyhow::Result;
    use tomb_common::utils::tests::*;

    #[tokio::test]
    async fn disk_key() -> Result<()> {
        let test_name = "disk_key";
        // Start er up!
        let (tomb_path, mut manifest, mut metadata_forest, _, dir) = setup(true, test_name).await?;

        // Generate key for this directory
        let key = store_dir(&mut manifest, &mut metadata_forest, &dir).await?;

        // Store and load
        key_to_disk(&tomb_path, &key, "root")?;
        let new_key = key_from_disk(&tomb_path, "root")?;

        // Assert equality
        assert_eq!(key, new_key);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    async fn disk_manifest() -> Result<()> {
        let test_name = "disk_manifest";
        // Start er up!
        let (tomb_path, manifest, _, _, _) = setup(true, test_name).await?;

        // Store and load
        manifest_to_disk(&tomb_path, &manifest)?;
        let new_manifest = manifest_from_disk(&tomb_path)?;

        // Assert equality
        assert_eq!(manifest, new_manifest);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    async fn disk_metadata() -> Result<()> {
        let test_name = "disk_metadata";
        // Setup
        let (tomb_path, mut manifest, mut metadata_forest, _, root_dir) = setup(true, test_name).await?;
        // Save to disk
        let key = hot_to_disk(&tomb_path, &mut manifest, &mut metadata_forest, &root_dir).await?;
        // Reload from disk
        let (new_key, new_manifest, new_metadata_forest, new_root_dir) =
            hot_from_disk(&tomb_path).await?;

        // Assert equality
        assert_eq!(key, new_key);
        assert_eq!(manifest, new_manifest);
        assert_eq!(
            metadata_forest
                .diff(&new_metadata_forest, &new_manifest.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(root_dir, new_root_dir);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    async fn disk_content() -> Result<()> {
        let test_name = "disk_content";
        // Setup
        let (tomb_path, mut manifest, mut metadata_forest, mut content_forest, root_dir) =
            setup(true, test_name).await?;
        // Save to disk
        let key = all_to_disk(
            &tomb_path,
            &mut manifest,
            &mut metadata_forest,
            &mut content_forest,
            &root_dir,
        )
        .await?;
        // Reload from disk
        let (new_key, new_manifest, new_metadata_forest, new_content_forest, new_root_dir) =
            all_from_disk(&tomb_path).await?;

        // Assert equality
        assert_eq!(key, new_key);
        assert_eq!(manifest, new_manifest);
        assert_eq!(
            metadata_forest
                .diff(&new_metadata_forest, &new_manifest.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            content_forest
                .diff(&new_content_forest, &new_manifest.content)
                .await?
                .len(),
            0
        );
        assert_eq!(root_dir, new_root_dir);

        // Teardown
        teardown(test_name).await
    }

    /*
    
    /// Helper function, not a test
    async fn assert_serial_all_cold(local: bool) -> Result<()> {
        let test_name: &String = &format!("serial_all_cold_{}", local);
        // Start er up!
        let (tomb_path, mut manifest, mut metadata_forest, mut content_forest, dir) =
            setup(true, test_name).await?;

        // Store and load
        let key = all_to_disk(
            &tomb_path,
            &mut manifest,
            &mut metadata_forest,
            &mut content_forest,
            &dir
        )
        .await?;
        let (new_key, new_manifest, new_metadata_forest, new_content_forest, new_dir) =
            all_from_disk(&tomb_path).await?;

        // Assert equality
        assert_eq!(new_key, key);
        assert_eq!(new_manifest, manifest);
        assert_eq!(
            new_metadata_forest
                .diff(&metadata_forest, &new_manifest.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            new_content_forest
                .diff(&content_forest, &new_manifest.content)
                .await?
                .len(),
            0
        );
        assert_eq!(new_dir, dir);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    async fn serial_all_content() -> Result<()> {
        assert_serial_all_cold(true).await
    }

    #[tokio::test]
    async fn serial_all_cold_remote() -> Result<()> {
        assert_serial_all_cold(false).await
    }
     */
}
