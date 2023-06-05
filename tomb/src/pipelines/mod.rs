/// This module contains the add pipeline function, which is the main entry point for inserting into existing WNFS filesystems.
pub mod add;
/// This module contains configuration functions for the cli
pub mod configure;
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
    use super::add;
    use crate::{
        pipelines::{configure, pack, pull, push, remove, unpack},
        utils::{
            serialize::{load_manifest, load_pipeline},
            spider::path_to_segments,
            tests::{compute_directory_size, test_setup, test_setup_structured, test_teardown},
            wnfsio::decompress_bytes,
        },
    };
    use anyhow::Result;
    use dir_assert::assert_paths;
    use fake_file::{Strategy, Structure};
    use fs_extra::{copy_items, dir::CopyOptions};
    use std::{
        fs::{self, create_dir_all, remove_dir_all, File},
        io::Write,
        path::PathBuf,
    };
    use tomb_common::types::pipeline::Manifest;

    #[tokio::test]
    async fn pipeline_init() -> Result<()> {
        // Create the setup conditions
        let (input_dir, _) = test_setup("pipeline_init").await?;
        // Initialize
        configure::init(&input_dir)?;
        let manifest = load_manifest(&input_dir.join(".tomb"))?;
        // Expect that the default Manifest was serialized
        assert_eq!(manifest, Manifest::default());
        // Teardown
        test_teardown("pipeline_init").await
    }

    #[tokio::test]
    async fn pipeline_configure_remote() -> Result<()> {
        // Create the setup conditions
        let (input_dir, _) = test_setup("pipeline_configure_remote").await?;
        // Initialize
        configure::init(&input_dir)?;
        // Configure the remote endpoint
        configure::remote(&input_dir, "http://127.0.0.1", 5001)?;
        // Load the Manifest
        let manifest = load_manifest(&input_dir.join(".tomb"))?;
        // Expect that the default Manifest was serialized
        assert_eq!(manifest.cold_remote.addr, "http://127.0.0.1:5001");
        // Teardown
        test_teardown("pipeline_configure_remote").await
    }

    #[tokio::test]
    async fn pipeline_pack_unpack_local() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup("pipeline_pack_unpack_local").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Pack locally
        pack::pipeline(input_dir, Some(output_dir), 0, true).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &output_dir.parent().unwrap().join("unpacked");
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(Some(output_dir), unpacked_dir).await?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(input_dir, unpacked_dir).unwrap();
        // Teardown
        test_teardown("pipeline_pack_unpack_local").await
    }

    #[tokio::test]
    async fn pipeline_pack_unpack_remote() -> Result<()> {
        // Create the setup conditions
        let (input_dir, unpacked_dir) = &test_setup("pipeline_pack_unpack_remote").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Configure the remote endpoint
        configure::remote(input_dir, "http://127.0.0.1", 5001)?;
        // Pack remotely
        pack::pipeline(input_dir, None, 262144, true).await?;
        // Copy the .tomb into the unpack dir
        copy_items(
            &[input_dir.join(".tomb")],
            &unpacked_dir,
            &CopyOptions::new(),
        )?;
        // Unpack from remote
        unpack::pipeline(None, unpacked_dir).await?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(input_dir, unpacked_dir).unwrap();
        // Teardown
        test_teardown("pipeline_pack_unpack_remote").await
    }

    #[tokio::test]
    #[ignore]
    async fn pipeline_pack_pull_unpack() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup("pipeline_pack_pull_unpack").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Configure the remote endpoint
        configure::remote(input_dir, "http://127.0.0.1", 5001)?;
        // Pack remotely
        pack::pipeline(input_dir, None, 262144, true).await?;
        // Move .tomb into the output dir
        fs_extra::copy_items(&[input_dir.join(".tomb")], output_dir, &CopyOptions::new())?;
        // Pull into the output dir
        pull::pipeline(&output_dir).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &output_dir.parent().unwrap().join("unpacked");
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(Some(output_dir), unpacked_dir).await?;
        // Remove metadata such that it is not factored in comparison
        remove_dir_all(input_dir.join(".tomb"))?;
        remove_dir_all(unpacked_dir.join(".tomb"))?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(input_dir, unpacked_dir).unwrap();
        // Teardown
        test_teardown("pipeline_pack_pull_unpack").await
    }

    #[tokio::test]
    async fn pipeline_pack_push() -> Result<()> {
        // Start the IPFS daemon
        // let mut ipfs = start_daemon();
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup("pipeline_pack_push").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Configure the remote endpoint
        configure::remote(input_dir, "http://127.0.0.1", 5001)?;
        // Pack locally
        pack::pipeline(input_dir, Some(&output_dir), 262144, true).await?;
        // Push
        push::pipeline(output_dir).await?;
        // Kill the daemon
        // ipfs.kill()?;
        // Teardown
        test_teardown("pipeline_pack_push").await
    }

    #[tokio::test]
    #[ignore]
    async fn pipeline_pack_push_pull() -> Result<()> {
        // Start the IPFS daemon
        // let mut ipfs = start_daemon();

        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup("pipeline_pack_push_pull").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Configure the remote endpoint
        configure::remote(&input_dir, "http://127.0.0.1", 5001)?;
        // Pack locally
        pack::pipeline(&input_dir, Some(&output_dir), 262144, true).await?;
        // Send data to remote endpoint
        push::pipeline(&output_dir).await?;

        // Compute size of original content
        let d1 = compute_directory_size(&output_dir.join("content")).unwrap();
        // Oh no! File corruption, we lost all our data!
        fs::remove_dir_all(output_dir.join("content"))?;
        // Now its time to reconstruct all our data
        pull::pipeline(&output_dir).await?;
        // Compute size of reconstructed content
        let d2 = compute_directory_size(&output_dir.join("content")).unwrap();
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(d1, d2);
        // Kill the daemon
        // ipfs.kill()?;
        // Teardown
        test_teardown("pipeline_pack_push_pull").await
    }

    #[tokio::test]
    async fn pipeline_add_local() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup("pipeline_add_local").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Run the pack pipeline
        pack::pipeline(input_dir, Some(&output_dir), 262144, true).await?;
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
        add::pipeline(true, input_file, tomb_path, input_file).await?;
        // Now that the pipeline has run, grab all metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(tomb_path).await?;
        // Grab the file at this path
        let file = dir
            .get_node(
                &path_to_segments(&input_file)?,
                true,
                forest,
                &manifest.hot_local,
            )
            .await?
            .unwrap()
            .as_file()?;
        // Get the content of the PrivateFile and decompress it
        let mut loaded_file_content: Vec<u8> = Vec::new();
        decompress_bytes(
            file.get_content(forest, &manifest.cold_local)
                .await?
                .as_slice(),
            &mut loaded_file_content,
        )?;
        // Assert that the data matches the original data
        assert_eq!(file_content, loaded_file_content);
        // Teardown
        test_teardown("pipeline_add_local").await
    }

    #[tokio::test]
    async fn pipeline_add_remote() -> Result<()> {
        // Create the setup conditions
        let (input_dir, _) = &test_setup("pipeline_add_remote").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Configure the remote endpoint
        configure::remote(input_dir, "http://127.0.0.1", 5001)?;
        // Run the pack pipeline
        pack::pipeline(input_dir, None, 262144, true).await?;
        // Grab metadata
        let tomb_path = &input_dir.join(".tomb");
        // This is still in the input dir. Technically we could just
        let input_file = &input_dir.join("hello.txt");
        // Content to be written to the file
        let file_content = String::from("This is just example text.")
            .as_bytes()
            .to_vec();
        // Create and write to the file
        File::create(input_file)?.write_all(&file_content)?;
        // Add the input file to the WNFS
        add::pipeline(false, input_file, tomb_path, input_file).await?;
        // Now that the pipeline has run, grab all metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(tomb_path).await?;
        // Grab the file at this path
        let file = dir
            .get_node(
                &path_to_segments(&input_file)?,
                true,
                forest,
                &manifest.hot_local,
            )
            .await?
            .unwrap()
            .as_file()?;
        // Get the content of the PrivateFile and decompress it
        let mut loaded_file_content: Vec<u8> = Vec::new();
        decompress_bytes(
            file.get_content(forest, &manifest.cold_remote)
                .await?
                .as_slice(),
            &mut loaded_file_content,
        )?;
        // Assert that the data matches the original data
        assert_eq!(file_content, loaded_file_content);
        // Teardown
        test_teardown("pipeline_add_remote").await
    }

    #[tokio::test]
    async fn pipeline_remove() -> Result<()> {
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup("pipeline_remove").await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Run the pack pipeline
        pack::pipeline(input_dir, Some(&output_dir), 262144, true).await?;
        // Grab metadata
        let tomb_path = &output_dir.join(".tomb");
        // Write out a reference to where we expect to find this file
        let wnfs_path = &PathBuf::from("").join("0").join("0");
        let wnfs_segments = &path_to_segments(wnfs_path)?;
        // Load metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(tomb_path).await?;
        let result = dir
            .get_node(wnfs_segments, true, forest, &manifest.hot_local)
            .await?;
        // Assert the node exists presently
        assert!(result.is_some());
        // Remove the PrivateFile at this Path
        remove::pipeline(tomb_path, wnfs_path).await?;
        // Reload metadata
        let (_, manifest, forest, dir) = &mut load_pipeline(tomb_path).await?;
        let result = dir
            .get_node(wnfs_segments, true, forest, &manifest.hot_local)
            .await?;
        // Assert the node no longer exists
        assert!(result.is_none());
        // Teardown
        test_teardown("pipeline_remove").await
    }

    // Helper function for structure tests
    async fn assert_pack_unpack_local(test_name: &str) -> Result<()> {
        // Grab directories
        let root_path = PathBuf::from("test").join(test_name);
        let (input_dir, output_dir) = &(root_path.join("input"), root_path.join("output"));
        // Pack locally
        pack::pipeline(input_dir, Some(output_dir), 0, true).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &output_dir.parent().unwrap().join("unpacked");
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(Some(output_dir), unpacked_dir).await?;

        // Assert the pre-packed and unpacked directories are identical
        assert_paths(input_dir, unpacked_dir).unwrap();

        Ok(())
    }

    // 1MB
    const TEST_INPUT_SIZE: usize = 100;

    #[tokio::test]
    async fn pipeline_structure_simple() -> Result<()> {
        let test_name = "pipeline_structure_simple";
        let structure = Structure::new(4, 4, TEST_INPUT_SIZE, Strategy::Simple);
        // Setup
        let (input_dir, _) = test_setup_structured(test_name, structure).await?;
        configure::init(&input_dir)?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn pipeline_structure_deep() -> Result<()> {
        let test_name = "pipeline_structure_deep";
        let structure = Structure::new(2, 8, TEST_INPUT_SIZE, Strategy::Simple);
        // Setup
        let (input_dir, _) = test_setup_structured(test_name, structure).await?;
        configure::init(&input_dir)?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn pipeline_structure_wide() -> Result<()> {
        let test_name = "pipeline_structure_deep";
        let structure = Structure::new(16, 1, TEST_INPUT_SIZE, Strategy::Simple);
        // Setup
        let (input_dir, _) = test_setup_structured(test_name, structure).await?;
        configure::init(&input_dir)?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }
}
