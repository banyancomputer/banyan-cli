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
    use serial_test::serial;
    use tomb_common::types::blockstore::networkblockstore::NetworkBlockStore;

    use crate::{
        pipelines::{pack, pull, push, remove},
        utils::{
            serialize::load_pipeline,
            spider::path_to_segments,
            tests::{compute_directory_size, start_daemon, test_setup, test_teardown},
            wnfsio::decompress_bytes,
        },
    };

    use super::add;

    #[tokio::test]
    #[serial]
    async fn test_push() -> Result<()> {
        // Start the IPFS daemon
        let mut ipfs = start_daemon();

        // Create the setup conditions
        let (input_dir, output_dir) = test_setup("push").await?;
        pack::pipeline(&input_dir, Some(&output_dir), 262144, true).await?;

        // Construct NetworkBlockStore and run pipeline
        let store = NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        push::pipeline(&output_dir, &store).await?;

        // Kill the daemon
        ipfs.kill()?;

        // Teardown
        test_teardown("push").await
    }

    #[tokio::test]
    #[serial]
    async fn test_pull() -> Result<()> {
        // Start the IPFS daemon
        let mut ipfs = start_daemon();

        // Create the setup conditions
        let (input_dir, output_dir) = test_setup("pull").await?;
        pack::pipeline(&input_dir, Some(&output_dir), 262144, true).await?;

        // Construct NetworkBlockStore
        let store = NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        // Send data to remote endpoint
        push::pipeline(&output_dir, &store).await?;
        // Compute size of original content
        let d1 = compute_directory_size(&output_dir.join("content")).unwrap();

        // Oh no! File corruption, we lost all our data!
        fs::remove_dir_all(output_dir.join("content"))?;

        // Now its time to reconstruct all our data
        pull::pipeline(&output_dir, &store).await?;

        // Compute size of reconstructed content
        let d2 = compute_directory_size(&output_dir.join("content")).unwrap();

        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(d1, d2);

        // Kill the daemon
        ipfs.kill()?;

        // Teardown
        test_teardown("pull").await
    }

    #[tokio::test]
    #[serial]
    async fn test_add() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = test_setup("add").await?;
        // Run the pack pipeline
        pack::pipeline(&input_dir, Some(&output_dir), 262144, true).await?;
        // Grab metadata
        let tomb_path = &output_dir.join(".tomb");
        // This is still in the input dir. Technically we could just
        let input_file = &input_dir.join("hello.txt");
        // Content to be written to the file
        let file_content = String::from("This is just example text.")
            .as_bytes()
            .to_vec();
        // Create and write to the file
        File::create(input_file)?.write_all(&file_content)?;
        // Add the input file to the WNFS
        add::pipeline(input_file, tomb_path, input_file).await?;
        // Now that the pipeline has run, grab all metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(true, tomb_path).await?;
        // Grab the file at this path
        let result = dir
            .get_node(
                &path_to_segments(&input_file)?,
                true,
                forest,
                &manifest.content_local,
            )
            .await?;
        // Assert the node was found
        assert!(result.is_some());
        // Represent the result as a PrivateFile
        let loaded_file = result.unwrap().as_file()?;
        // Get the content of the PrivateFile and decompress it
        let mut loaded_file_content: Vec<u8> = Vec::new();
        decompress_bytes(
            loaded_file
                .get_content(forest, &manifest.content_local)
                .await?
                .as_slice(),
            &mut loaded_file_content,
        )?;
        // Assert that the data matches the original data
        assert_eq!(file_content, loaded_file_content);
        // Teardown
        test_teardown("add").await
    }

    #[tokio::test]
    #[serial]
    async fn test_remove() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = test_setup("remove").await?;
        // Run the pack pipeline
        pack::pipeline(&input_dir, Some(&output_dir), 262144, true).await?;
        // Grab metadata
        let tomb_path = &output_dir.join(".tomb");
        // Write out a reference to where we expect to find this file
        let wnfs_path = &PathBuf::from("").join("0").join("0");
        let wnfs_segments = &path_to_segments(wnfs_path)?;

        // Load metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(true, tomb_path).await?;
        let result = dir
            .get_node(wnfs_segments, true, forest, &manifest.content_local)
            .await?;
        // Assert the node exists presently
        assert!(result.is_some());

        // Remove the PrivateFile at this Path
        remove::pipeline(tomb_path, wnfs_path).await?;

        // Reload metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(true, tomb_path).await?;
        let result = dir
            .get_node(wnfs_segments, true, forest, &manifest.content_local)
            .await?;
        // Assert the node no longer exists
        assert!(result.is_none());

        // Teardown
        test_teardown("remove").await
    }
}
