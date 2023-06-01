/// This module contains the add pipeline function, which is the main entry point for inserting into existing WNFS filesystems.
pub mod add;
/// This module contains the pack pipeline function, which is the main entry point for packing new data.
pub mod pack;
/// This module contains the pull pipeline function, which downloads packed content from disk to a remote server.
pub mod pull;
/// This module contains the push pipeline function, which uploads packed content from disk to a remote server.
pub mod push;
/// This module contains the add pipeline function, which is the main entry point for removing from existing WNFS filesystems.
pub mod remove;
/// This module contains the unpack pipeline function, which is the main entry point for extracting previously packed data.
pub mod unpack;

#[cfg(test)]
mod test {
    use std::{
        fs::{self, File},
        io::Write,
        net::Ipv4Addr,
        path::PathBuf,
    };

    use anyhow::Result;
    use fake_file::{utils::ensure_path_exists_and_is_empty_dir, Strategy, Structure};
    use serial_test::serial;
    use tomb_common::types::blockstore::networkblockstore::NetworkBlockStore;

    use crate::{
        pipelines::{pack, pull, push},
        utils::{serialize::{load_manifest, load_pipeline}, spider::path_to_segments, wnfsio::decompress_bytes},
    };

    use super::add;

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
        pack::pipeline(&input_dir, &output_dir, 262144, true).await?;

        // Construct NetworkBlockStore and run pipeline
        let store = NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        push::pipeline(&output_dir, &store).await?;

        // Teardown
        teardown("push").await
    }

    #[tokio::test]
    #[serial]
    async fn test_pull() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = setup("pull").await?;
        pack::pipeline(&input_dir, &output_dir, 262144, true).await?;

        // Construct NetworkBlockStore
        let store = NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        // Send data to remote endpoint
        push::pipeline(&output_dir, &store).await?;
        let tomb_path = output_dir.join(".tomb");
        let manifest = load_manifest(&tomb_path).await?;

        // Oh no! File corruption, we lost all our data!
        fs::remove_dir_all(output_dir.join("content"))?;

        // Now its time to reconstruct all our data
        pull::pipeline(&output_dir, &store).await?;

        let new_manifest = load_manifest(&tomb_path).await?;

        // Assert that the reconstructed manifest and blocks contained therein are identical
        assert_eq!(manifest, new_manifest);

        // Teardown
        teardown("pull").await
    }

    #[tokio::test]
    #[serial]
    async fn test_add() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = setup("add").await?;
        // Run the pack pipeline
        pack::pipeline(&input_dir, &output_dir, 262144, true).await?;
        // Grab metadata
        let tomb_path = &output_dir.join(".tomb");
        // This is still in the input dir. Technically we could just
        let input_file = &input_dir.join("hello.txt");
        // Content to be written to the file
        let file_content = String::from("This is just example text.").as_bytes().to_vec();
        // Create and write to the file
        File::create(input_file)?.write_all(&file_content)?;
        // Add the input file to the WNFS
        add::pipeline(input_file, tomb_path, input_file).await?;
        // Now that the pipeline has run, grab all metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(tomb_path).await?;
        // Grab the file at this path
        let result = dir.get_node(&path_to_segments(&input_file)?, true, forest, &manifest.content_store).await?;
        // Assert the node was found
        assert!(result.is_some());
        // Represent the result as a PrivateFile
        let loaded_file = result.unwrap().as_file()?;
        // Get the content of the PrivateFile and decompress it
        let loaded_file_content = decompress_bytes(loaded_file.get_content(forest, &manifest.content_store).await?).await?;
        // Assert that the data matches the original data
        assert_eq!(file_content, loaded_file_content);
        // Teardown
        teardown("add").await
    }
}
