/// This module contains the pack_pipeline function, which is the main entry point for packing new data.
pub mod pack_pipeline;
/// This module contains the pull_pipeline function, which downloads packed content from disk to a remote server.
pub mod pull_pipeline;
/// This module contains the push_pipeline function, which uploads packed content from disk to a remote server.
pub mod push_pipeline;
/// This module contains the unpack_pipeline function, which is the main entry point for extracting previously packed data.
pub mod unpack_pipeline;

#[cfg(test)]
mod test {
    use std::{fs, net::Ipv4Addr, path::PathBuf};

    use anyhow::Result;
    use fake_file::{utils::ensure_path_exists_and_is_empty_dir, Strategy, Structure};
    use serial_test::serial;

    use crate::{
        pipelines::{
            pack_pipeline::pack_pipeline, pull_pipeline::pull_pipeline,
            push_pipeline::push_pipeline,
        },
        types::blockstore::networkblockstore::NetworkBlockStore,
        utils::pipeline::load_manifest,
    };

    // Set up temporary filesystem for test cases
    async fn setup(test_name: &str) -> Result<(PathBuf, PathBuf)> {
        // Base of the test directory
        let root_path = PathBuf::from("test").join(test_name);
        // Create and empty the dir
        ensure_path_exists_and_is_empty_dir(&root_path, true)?;
        // Input and output paths
        let input_path = root_path.join("input");
        let output_path = root_path.join("output");
        // Generate file structure
        Structure::new(2, 2, 2000, Strategy::Simple).generate(&input_path)?;
        // Return all paths
        Ok((input_path, output_path))
    }

    // Remove contents of temporary dir
    async fn teardown(test_name: &str) -> Result<()> {
        Ok(fs::remove_dir_all(PathBuf::from("test").join(test_name))?)
    }

    #[tokio::test]
    #[serial]
    async fn test_push() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = setup("push").await?;
        pack_pipeline(&input_dir, &output_dir, 262144, true).await?;

        // Construct NetworkBlockStore and run pipeline
        let store = NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        push_pipeline(&output_dir, &store).await?;

        // Teardown
        teardown("push").await
    }

    #[tokio::test]
    #[serial]
    async fn test_pull() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = setup("pull").await?;
        pack_pipeline(&input_dir, &output_dir, 262144, true).await?;

        // Construct NetworkBlockStore
        let store = NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        // Send data to remote endpoint
        push_pipeline(&output_dir, &store).await?;
        let tomb_path = output_dir.join(".tomb");
        let manifest = load_manifest(&tomb_path).await?;

        // Oh no! File corruption, we lost all our data!
        fs::remove_dir_all(output_dir.join("content"))?;

        // Now its time to reconstruct all our data
        pull_pipeline(&output_dir, &store).await?;

        let new_manifest = load_manifest(&tomb_path).await?;

        // Assert that the reconstructed manifest and blocks contained therein are identical
        assert_eq!(manifest, new_manifest);

        // Teardown
        teardown("pull").await
    }
}
