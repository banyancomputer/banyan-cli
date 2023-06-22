/// This module contains the add pipeline function, which is the main entry point for inserting into existing WNFS filesystems.
pub mod add;
/// This module contains configuration functions for the cli
pub mod configure;
mod error;
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
            spider::path_to_segments,
            tests::{test_setup, test_setup_structured, test_teardown},
            wnfsio::decompress_bytes,
        },
    };
    use anyhow::Result;
    use dir_assert::assert_paths;
    use fake_file::{Strategy, Structure, utils::ensure_path_exists_and_is_dir};
    use fclones::config;
    use fs_extra::dir::{CopyOptions, self};
    use serial_test::serial;
    use wnfs::private::PrivateNodeOnPathHistory;
    use std::{
        fs::{create_dir_all, metadata, remove_file, File, symlink_metadata, read_link},
        io::Write,
        path::PathBuf, rc::Rc, os::unix::fs::symlink,
    };
    use tomb_common::{types::config::globalconfig::GlobalConfig, utils::serialize::load_dir};

    #[tokio::test]
    #[serial]
    async fn pipeline_init() -> Result<()> {
        let test_name = "pipeline_init";
        // Create the setup conditions
        let input_dir = &test_setup(test_name).await?;
        // Deinitialize for user
        configure::deinit(input_dir)?;
        // Assert that packing fails
        assert!(pack::pipeline(input_dir, true).await.is_err());
        // Initialize for this user
        configure::init(input_dir)?;
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_configure_remote() -> Result<()> {
        let address = "http://app.tomb.com.net.org:5423";
        // Configure the remote endpoint
        configure::remote(address)?;
        // Assert it was actually modified
        assert_eq!(
            GlobalConfig::from_disk()?.remote,
            address
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_pack_local() -> Result<()> {
        let test_name = "pipeline_pack_local";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize
        configure::init(origin)?;
        // Load config
        let mut config = GlobalConfig::from_disk()?.get_bucket(origin).unwrap();
        // Assert no key yet
        assert!(config.get_key("root").is_err());
        // Pack
        pack::pipeline(origin, true).await?;
        // Update config
        config = GlobalConfig::from_disk()?.get_bucket(origin).unwrap();
        // Ensure content exists and works
        assert!(config.get_key("root").is_ok());
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_pack_unpack_local() -> Result<()> {
        let test_name = "pipeline_pack_unpack_local";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize
        configure::init(origin)?;
        // Pack locally
        pack::pipeline(origin, true).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &origin
            .parent()
            .unwrap()
            .join(format!("{}_unpacked", test_name));
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(&origin, unpacked_dir).await?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(origin, unpacked_dir).unwrap();
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn pipeline_pack_push() -> Result<()> {
        let test_name = "pipeline_pack_pull_unpack";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize
        configure::init(&origin)?;
        // Configure the remote endpoint
        configure::remote("http://127.0.0.1:5001")?;
        // Pack locally
        pack::pipeline(origin, true).await?;
        // Push
        push::pipeline(origin).await?;
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn pipeline_pack_push_pull() -> Result<()> {
        let test_name = "pipeline_pack_push_pull";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(origin)?;
        // Configure the remote endpoint
        configure::remote("http://127.0.0.1:5001")?;
        // Pack locally
        pack::pipeline(&origin, true).await?;
        // Send data to remote endpoint
        push::pipeline(&origin).await?;
        // The content path of the current content BlockStore
        let v1_content = &GlobalConfig::from_disk()?
            .get_bucket(origin)
            .unwrap()
            .content
            .path;
        // Compute size of original content
        let d1 = metadata(v1_content)?.len();
        // Oh no! File corruption, we lost all our data!
        remove_file(v1_content)?;
        // Now its time to reconstruct all our data
        pull::pipeline(&origin).await?;
        // The content path of the current content BlockStore
        let v2_content = GlobalConfig::from_disk()?
            .get_bucket(origin)
            .unwrap()
            .content
            .path;
        // Compute size of reconstructed content
        let d2 = metadata(v2_content)?.len();
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(d1, d2);
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_add_local() -> Result<()> {
        let test_name = "pipeline_add_local";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(origin)?;
        // Run the pack pipeline
        pack::pipeline(origin, true).await?;
        // This is still in the input dir. Technically we could just
        let input_file = &origin.join("hello.txt");
        // Content to be written to the file
        let file_content = String::from("This is just example text.")
            .as_bytes()
            .to_vec();
        // Create and write to the file
        File::create(input_file)?.write_all(&file_content)?;
        // Add the input file to the WNFS
        add::pipeline(origin, input_file, input_file).await?;

        // Now that the pipeline has run, grab all metadata
        let config = GlobalConfig::from_disk()?.get_bucket(origin).unwrap();
        let (metadata_forest, content_forest, dir) = &mut config.get_all().await?;

        // Grab the file at this path
        let file = dir
            .get_node(
                &path_to_segments(&input_file)?,
                true,
                metadata_forest,
                &config.metadata,
            )
            .await?
            .unwrap()
            .as_file()?;
        // Get the content of the PrivateFile and decompress it
        let mut loaded_file_content: Vec<u8> = Vec::new();
        decompress_bytes(
            file.get_content(content_forest, &config.content)
                .await?
                .as_slice(),
            &mut loaded_file_content,
        )?;
        // Assert that the data matches the original data
        assert_eq!(file_content, loaded_file_content);
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_remove_local() -> Result<()> {
        let test_name = "pipeline_remove_local";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(origin)?;
        // Run the pack pipeline
        pack::pipeline(origin, true).await?;
        // Write out a reference to where we expect to find this file
        let wnfs_path = &PathBuf::from("").join("0").join("0");
        let wnfs_segments = &path_to_segments(wnfs_path)?;
        // Load metadata
        let config = GlobalConfig::from_disk()?.get_bucket(origin).unwrap();
        let (metadata_forest, _, dir) = &mut config.get_all().await?;
        let result = dir
            .get_node(wnfs_segments, true, metadata_forest, &config.metadata)
            .await?;
        // Assert the node exists presently
        assert!(result.is_some());
        // Remove the PrivateFile at this Path
        remove::pipeline(origin, wnfs_path).await?;
        // Reload metadata
        let config = GlobalConfig::from_disk()?.get_bucket(origin).unwrap();
        let (metadata_forest, _, dir) = &mut config.get_all().await?;
        let result = dir
            .get_node(wnfs_segments, true, metadata_forest, &config.metadata)
            .await?;
        // Assert the node no longer exists
        assert!(result.is_none());
        // Teardown
        test_teardown(test_name).await?;

        Ok(())
    }

    // Helper function for structure tests
    async fn assert_pack_unpack_local(test_name: &str) -> Result<()> {
        // Grab directories
        let root_path = PathBuf::from("test").join(test_name);
        let origin = &root_path.join("input");
        // Initialize
        configure::init(origin)?;
        // Pack locally
        pack::pipeline(origin, true).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &origin.parent().unwrap().join("unpacked");
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(origin, unpacked_dir).await?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(origin, unpacked_dir).unwrap();
        Ok(())
    }

    const TEST_INPUT_SIZE: usize = 1024;

    #[tokio::test]
    #[serial]
    async fn pipeline_structure_simple() -> Result<()> {
        let test_name = "pipeline_structure_simple";
        let structure = Structure::new(4, 4, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_structure_deep() -> Result<()> {
        let test_name = "pipeline_structure_deep";
        let structure = Structure::new(2, 8, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_structure_wide() -> Result<()> {
        let test_name = "pipeline_structure_deep";
        let structure = Structure::new(16, 1, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    /// Test with one very big file -- ignore cuz it takes a while
    #[tokio::test]
    #[serial]
    async fn test_big_file() -> Result<()> {
        let test_name = "test_big_file";    
        let structure = Structure::new(1, 1, TEST_INPUT_SIZE * 100, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    /// Ensure that the pipeline can recover duplicate files
    #[tokio::test]
    #[serial]
    async fn test_deduplication_integrity() -> Result<()> {
        let test_name = "test_deduplication_integrity";
        // Define the file structure to test
        let structure = Structure::new(2, 2, TEST_INPUT_SIZE, Strategy::Simple);
        // Setup the test
        let origin = &test_setup_structured(test_name, structure).await?;
        let new_origin = &origin.join("original");
        // Move the contents of this directory into a subdirectory
        dir::move_dir(origin, new_origin, &CopyOptions::new())?;

        let duplicate_origin = origin.join("duplicate");
        // Create the directory
        ensure_path_exists_and_is_dir(&duplicate_origin).unwrap();
        // Copy the contents of the original directory into the new directory
        dir::copy(
            &new_origin,
            &duplicate_origin,
            &CopyOptions::new(),
        )
        .unwrap();

        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    /*

    /// Ensure that the duplicate data occupies a smaller footprint when packed
    //TODO (organizedgrime) - This test is a bit longer than I would like, might modify it to be more modular / reusable
    #[tokio::test]
    async fn test_deduplication_size() {
        let test_name = "test_deduplication_size";
        

        // We will be comparing two twin directories, one with duplicates and one without
        let twin_dups = test_path.join("twin_dups");
        let twin_unique = test_path.join("twin_unique");

        // Define the file structure to test in both cases
        let structure = Structure::new(2, 2, TEST_INPUT_SIZE, Strategy::Simple);

        // Setup the duplicates directory
        setup_test(&twin_dups, structure.clone(), "duplicate_directory");
        // Duplicate the test file
        let input_path = twin_dups.join(INPUT_PATH);
        // Copy $input_path/test_duplicate to $input_path/encloser
        let original_path = input_path.join("duplicate_directory");
        // Enclose the duplicate in a parent directory
        let encloser_path = input_path.join("encloser");
        // Create the directory
        ensure_path_exists_and_is_dir(&encloser_path).unwrap();
        // Copy the contents of the original directory into the new directory
        fs_extra::dir::copy(
            &original_path,
            &encloser_path,
            &fs_extra::dir::CopyOptions::new(),
        )
        .unwrap();

        // Setup the first unique directory
        setup_test(&twin_unique, structure.clone(), "unique1");
        // Duplicate the test file
        let input_path = twin_unique.join(INPUT_PATH);
        // The directory that will contain the other unique directory
        let mut encloser_path = input_path.join("encloser");
        // Create the directory
        ensure_path_exists_and_is_dir(&encloser_path).unwrap();
        // Push the subdirectory name
        encloser_path.push("unique2");
        // Generate the structure inside this directory, which will be unique
        structure.generate(&encloser_path).unwrap();

        // Now we can actually start testing things!
        // Ensure that the twin_dups directory is the same size as the twin_unique directory
        let twin_dups_size = compute_directory_size(&twin_dups).unwrap();
        let twin_unique_size = compute_directory_size(&twin_unique).unwrap();
        assert_eq!(twin_dups_size, twin_unique_size);

        // Run the pipelines on both directories, also ensuring output = input
        run_test(&twin_dups, "duplicate_directory").await;
        run_test(&twin_unique, "unique1").await;

        // Write out the paths to both packed directories
        let packed_dups_path = twin_dups.join(PACKED_PATH);
        let packed_unique_path = twin_unique.join(PACKED_PATH);
        // Compute the sizes of these directories
        let packed_dups_size = compute_directory_size(&packed_dups_path).unwrap() as f32;
        let packed_unique_size = compute_directory_size(&packed_unique_path).unwrap() as f32;
        // Ensure that the size of the packed duplicates directory is approximately half that of the unique directory
        // TODO (organizedgrime) determine the threshold for this test that is most appropriate
        assert!(packed_unique_size / packed_dups_size >= 1.8);
    }

    /// Ensure that deduplication is equally effective in the case of large files
    /// This also ensures that deduplication works in cases where file contents are identical, but file names are not,
    /// as well as ensuring that deduplication works when both files are in the same directory.
    #[tokio::test]
    #[ignore]
    async fn test_deduplication_large() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("deduplication_large");
        // Define the file structure to test. Note that the input size is slightly larger than the maximum 0.25 GiB chunk size
        let desired_structure = Structure::new(0, 0, TEST_INPUT_SIZE * 100, Strategy::Simple);

        // Setup the test
        setup_test(&test_path, desired_structure, "test_large");

        // Duplicate the file in place
        fs_extra::file::copy(
            test_path.join(INPUT_PATH).join("0"),
            test_path.join(INPUT_PATH).join("1"),
            &fs_extra::file::CopyOptions::new(),
        )
        .unwrap();

        // Run the test
        run_test(&test_path, "test_large").await;

        // Assert that only one file was packed
        let packed_path = test_path.join(PACKED_PATH);
        let dir_info = fs_extra::dir::get_dir_content(packed_path).unwrap();
        // Expect that the large file was packed into two files
        assert_eq!(dir_info.files.len(), 2);
    }

     */

    #[tokio::test]
    #[serial]
    async fn test_double_packing() -> Result<()> {
        let test_name = "test_double_packing";
        // Define the file structure to test
        let structure = Structure::new(2, 2, TEST_INPUT_SIZE, Strategy::Simple);
        // Setup the test once
        test_setup_structured(test_name, structure).await?;
        // Run the test twice
        assert_pack_unpack_local(test_name).await?;
        assert_pack_unpack_local(test_name).await
    }

    // TODO (organizedgrime) - reimplement this when we have migrated from using Ratchets to WNFS's new solution.
    #[tokio::test]
    #[serial]
    #[ignore]
    /// This test fails randomly and succeeds randomly- TODO fix or just wait until WNFS people fix their code.
    async fn test_versioning() -> Result<()> {
        let test_name = "test_versioning";

        // Define the file structure to test
        let structure = Structure::new(2, 2, TEST_INPUT_SIZE, Strategy::Simple);

        // Setup the test once
        let origin = &test_setup_structured(test_name, structure).await?;

        // Path for the actual file on disk that we'll be writing
        let versioned_file_path = origin
            .join("0")
            .join("0");

        // Define bytes for each message
        let hello_bytes = "Hello World!".as_bytes();
        let still_bytes = "Still there, World?".as_bytes();
        let goodbye_bytes = "Goodbye World!".as_bytes();

        println!(
            "hb: {:?}\nsb: {:?}\ngb: {:?}",
            hello_bytes, still_bytes, goodbye_bytes
        );

        // Write "Hello World!" out to the file; v0
        File::create(&versioned_file_path)?.write_all(hello_bytes)?;

        println!("running for the first time...");

        // Run the test
        assert_pack_unpack_local(test_name).await?;

        // Write "Still there, World?" out to the same file
        File::create(&versioned_file_path)?.write_all(still_bytes)?;

        // Run the test again
        assert_pack_unpack_local(test_name).await?;

        // Write "Goodbye World!" out to the same file
        File::create(&versioned_file_path)?.write_all(goodbye_bytes)?;

        // Run the test again
        assert_pack_unpack_local(test_name).await?;

        
        let global = GlobalConfig::from_disk()?;
        let config = global.get_bucket(origin).unwrap();
        let (mut metadata_forest, content_forest, dir) = config.get_all_metadata().await?;

        // Get keys
        let original_key = &config.get_key("original")?;
        let key = &config.get_key("root")?;
        
        // Grab the original PrivateDirectory
        let original_dir = load_dir(&config.metadata, original_key, &mut metadata_forest).await?;

        assert_ne!(key, original_key);
        assert_ne!(dir, original_dir);

        let mut iterator = PrivateNodeOnPathHistory::of(
            dir,
            original_dir,
            1_000_000,
            &[],
            true,
            Rc::clone(&metadata_forest),
            &config.metadata,
        )
        .await
        .unwrap();

        // Describe path of the PrivateFile relative to the root directory
        let path_segments: Vec<String> =
            vec!["versioning".to_string(), "0".to_string(), "0".to_string()];

        // Get the previous version of the root of the PrivateDirectory
        let previous_root = iterator
            .get_previous(&config.metadata)
            .await?
            .unwrap()
            .as_dir()?;

        // Grab the previous version of the PrivateFile
        let previous_file = previous_root
            .get_node(&path_segments, true, &mut metadata_forest, &config.metadata)
            .await
            .unwrap()
            .unwrap()
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let previous_content = previous_file
            .get_content(&content_forest, &config.content)
            .await
            .unwrap();

        // Assert that the previous version of the file was retrieved correctly
        assert!(previous_content != goodbye_bytes);

        // Get the original version of the root of the PrivateDirectory
        let original_root = iterator
            .get_previous(&config.metadata)
            .await?
            .unwrap()
            .as_dir()?;

        // Grab the original version of the PrivateFile
        let original_file = original_root
            .get_node(&path_segments, true, &metadata_forest, &config.metadata)
            .await?
            .unwrap()
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let original_content = original_file
            .get_content(&content_forest, &config.content)
            .await?;

        // Assert that the previous version of the file was retrieved correctly
        assert!(original_content != goodbye_bytes);

        unsafe {
            println!(
                "oc: {:?}",
                String::from_utf8_unchecked(original_content.clone())
            );
            println!(
                "pc: {:?}",
                String::from_utf8_unchecked(previous_content.clone())
            );
        }

        assert_eq!(original_content, hello_bytes);
        assert_eq!(previous_content, still_bytes);

        // Assert that there are no more previous versions to find
        assert!(iterator
            .get_previous(&config.metadata)
            .await
            .unwrap()
            .is_none());

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_symlinks() -> Result<()> {
        let test_name = "test_symlinks";
        // Define the file structure to test
        let structure = Structure::new(2, 2, TEST_INPUT_SIZE, Strategy::Simple);

        // Setup the test
        let origin = &test_setup_structured(test_name, structure).await?;

        // Path in which Directory symlink will be created
        // let sym_dir_root = test_path.join("input").join("symlinks");
        // Path in which File symlink will be created
        let sym_file_root = origin.join("0");

        // Point from /input/symlinks/ZZ -> /input/symlinks/0
        let dir_original = origin.join("0").canonicalize().unwrap();
        let dir_sym = origin.join("ZZ");

        // Point from /input/symlinks/0/ZZ -> /input/symlinks/0/0
        let file_original = sym_file_root.join("0").canonicalize().unwrap();
        let file_sym = sym_file_root.join("ZZ");

        // Create those symbolic links in the actual filesystem
        symlink(&dir_original, &dir_sym)?;
        symlink(&file_original, &file_sym)?;

        // Assert that both of the paths are symlinks using their metadata
        assert!(symlink_metadata(&dir_sym)?.is_symlink());
        assert!(symlink_metadata(&file_sym)?.is_symlink());

        // Assert that both of them point to the location we expect them to
        assert_eq!(dir_original, read_link(dir_sym)?);
        assert_eq!(file_original, read_link(file_sym)?);

        // Run the test on the created filesystem
        assert_pack_unpack_local(test_name).await?;

        Ok(())
    }
}
