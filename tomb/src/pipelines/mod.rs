/// This module contains the add pipeline function, which is the main entry point for inserting into existing WNFS filesystems.
pub mod add;
/// This module contains configuration functions for the cli
pub mod configure;
/// Pipeline Errors
pub mod error;
/// This module contains the pack pipeline function, which is the main entry point for packing new data.
pub mod pack;
/// This module contains the add pipeline function, which is the main entry point for removing from existing WNFS filesystems.
pub mod remove;
/// This module contains the unpack pipeline function, which is the main entry point for extracting previously packed data.
pub mod unpack;

#[cfg(test)]
mod test {
    use super::add;
    use crate::{
        pipelines::{configure, pack, remove, unpack},
        types::config::globalconfig::GlobalConfig,
        utils::{
            spider::path_to_segments,
            test::{compute_directory_size, test_setup, test_setup_structured, test_teardown},
            wnfsio::{self, decompress_bytes},
        },
    };
    use anyhow::Result;
    use dir_assert::assert_paths;
    use fake_file::{utils::ensure_path_exists_and_is_empty_dir, Strategy, Structure};
    use fs_extra::dir;
    use serial_test::serial;
    use std::{
        fs::{create_dir_all, read_link, rename, symlink_metadata, File},
        io::Write,
        os::unix::fs::symlink,
        path::PathBuf,
    };

    #[tokio::test]
    #[serial]
    async fn init() -> Result<()> {
        let test_name = "init";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Deinitialize for user
        configure::deinit(origin).await?;
        // Assert that packing fails
        assert!(pack::pipeline(origin, true).await.is_err());
        // Initialize for this user
        configure::init(origin).await?;
        // Assert that a config exists for this bucket now
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket(origin)
            .is_some());
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn configure_remote() -> Result<()> {
        let address = "http://app.tomb.com.net.org:5423";
        // Configure the remote endpoint
        configure::remote(address).await?;
        // Assert it was actually modified
        assert_eq!(GlobalConfig::from_disk().await?.remote, address);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn pack() -> Result<()> {
        let test_name = "pack";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize
        configure::init(origin).await?;
        // Pack
        pack::pipeline(origin, true).await?;
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn unpack() -> Result<()> {
        let test_name = "unpack";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize
        configure::init(origin).await?;
        // Pack locally
        pack::pipeline(origin, true).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &origin
            .parent()
            .unwrap()
            .join(format!("{}_unpacked", test_name));
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(origin, unpacked_dir).await?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(origin, unpacked_dir).unwrap();
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn add() -> Result<()> {
        let test_name = "add";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(origin).await?;
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
        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.load_key().await?;
        let config = global.get_bucket(origin).unwrap();
        let (metadata_forest, content_forest, dir, _, _) =
            &mut config.get_all(&wrapping_key).await?;

        // Grab the file at this path
        let file = dir
            .get_node(
                &path_to_segments(input_file)?,
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
    async fn remove() -> Result<()> {
        let test_name = "remove";
        // Create the setup conditions
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(origin).await?;
        // Run the pack pipeline
        pack::pipeline(origin, true).await?;
        // Write out a reference to where we expect to find this file
        let wnfs_path = &PathBuf::from("").join("0").join("0");
        let wnfs_segments = &path_to_segments(wnfs_path)?;
        // Load metadata
        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.load_key().await?;
        let config = global.get_bucket(origin).unwrap();
        let (metadata_forest, _, dir, _, _) = &mut config.get_all(&wrapping_key).await?;
        let result = dir
            .get_node(wnfs_segments, true, metadata_forest, &config.metadata)
            .await?;
        // Assert the node exists presently
        assert!(result.is_some());
        // Remove the PrivateFile at this Path
        remove::pipeline(origin, wnfs_path).await?;
        // Reload metadata
        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.load_key().await?;
        let config = global.get_bucket(origin).unwrap();
        let (metadata_forest, _, dir, _, _) = &mut config.get_all(&wrapping_key).await?;
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
    async fn assert_pack_unpack(test_name: &str) -> Result<()> {
        // Grab directories
        let root_path = PathBuf::from("test").join(test_name);
        let origin = &root_path.join("input");
        // Initialize
        configure::init(origin).await?;
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
    async fn structure_simple() -> Result<()> {
        let test_name = "structure_simple";
        let structure = Structure::new(4, 4, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn structure_deep() -> Result<()> {
        let test_name = "structure_deep";
        let structure = Structure::new(2, 8, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn structure_wide() -> Result<()> {
        let test_name = "structure_deep";
        let structure = Structure::new(16, 1, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn big_file() -> Result<()> {
        let test_name = "big_file";
        let structure = Structure::new(1, 1, 1024 * 1024 * 10, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack(test_name).await?;
        test_teardown(test_name).await
    }

    /// Ensure that the pipeline can recover duplicate files
    #[tokio::test]
    #[serial]
    async fn deduplication_integrity() -> Result<()> {
        let test_name = "deduplication_integrity";
        // Setup the test
        let origin = &test_setup(test_name).await?;
        let dup_origin = &origin.parent().unwrap().join("dups");
        let original = &dup_origin.join("original");
        let duplicate = &dup_origin.join("duplicate");
        create_dir_all(original)?;
        create_dir_all(duplicate)?;

        // Move the contents of this directory into a subdirectory
        dir::move_dir(origin, original, &dir::CopyOptions::new())?;
        dir::copy(original, duplicate, &dir::CopyOptions::new())?;

        // Remove origin
        dir::remove(origin)?;
        // Rename dup origin to origin
        rename(dup_origin, origin)?;

        // Run test
        assert_pack_unpack(test_name).await?;
        test_teardown(test_name).await
    }

    // / Ensure that the duplicate data occupies a smaller footprint when packed
    //TODO (organizedgrime) - This test is a bit longer than I would like, might modify it to be more modular / reusable
    #[tokio::test]
    #[serial]
    async fn deduplication_size() -> Result<()> {
        let test_name = "deduplication_size";
        let test_name_dup = &format!("{}_dup", test_name);
        let test_name_unique = &format!("{}_unique", test_name);
        // Use bigger files such that metadata comprises a minority of the content CARs
        let structure = Structure::new(2, 2, 1024 * 1024, Strategy::Simple);
        // Deinit all
        configure::deinit_all().await?;

        // Base of the test directory
        let root_path_dup = PathBuf::from("test").join(test_name_dup);
        let root_path_unique = PathBuf::from("test").join(test_name_unique);
        // Create and empty the dir
        ensure_path_exists_and_is_empty_dir(&root_path_dup, true)?;
        ensure_path_exists_and_is_empty_dir(&root_path_unique, true)?;

        // Input and path
        let origin_dup = &root_path_dup.join("input");
        let original_dup = &origin_dup.join("original");
        let duplicate_dup = &origin_dup.join("duplicate");
        // create_dir_all(original_dup)?;
        create_dir_all(duplicate_dup)?;

        // Generate file structure
        structure.generate(original_dup)?;
        // Copy into duplicate path
        dir::copy(original_dup, duplicate_dup, &dir::CopyOptions::new())?;

        // Input and path
        let origin_unique = &root_path_unique.join("input");
        create_dir_all(origin_unique)?;
        let unique1 = &origin_unique.join("unique1");
        let unique2 = &origin_unique.join("unique2");
        // create_dir_all(unique2)?;
        // Generate twice
        structure.generate(unique1)?;
        structure.generate(unique2)?;

        // Run test
        assert_pack_unpack(test_name_dup).await?;
        assert_pack_unpack(test_name_unique).await?;

        // Get configs
        let global = GlobalConfig::from_disk().await?;
        // Compute the sizes of these directories
        let packed_dups_size =
            compute_directory_size(&global.get_bucket(origin_dup).unwrap().content.path)? as f64;
        let packed_unique_size =
            compute_directory_size(&global.get_bucket(origin_unique).unwrap().content.path)? as f64;

        // Ensure that the size of the packed duplicates directory is approximately half that of the unique directory
        assert!(packed_unique_size / packed_dups_size >= 1.8);

        test_teardown(test_name_dup).await?;
        test_teardown(test_name_unique).await
    }

    #[tokio::test]
    #[serial]
    async fn double_packing() -> Result<()> {
        let test_name = "double_packing";
        // Setup the test once
        test_setup(test_name).await?;
        // Run the test twice
        assert_pack_unpack(test_name).await?;
        assert_pack_unpack(test_name).await?;
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn versioning_complex() -> Result<()> {
        let test_name = "versioning_complex";
        let structure = Structure::new(2, 2, 2000, Strategy::Simple);
        // Setup the test once
        let origin = &test_setup_structured(test_name, structure).await?;

        // Path for the actual file on disk that we'll be writing
        let versioned_file_path = origin.join("0").join("0");

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
        // Run the test
        assert_pack_unpack(test_name).await?;
        // Write "Still there, World?" out to the same file
        File::create(&versioned_file_path)?.write_all(still_bytes)?;
        // Run the test again
        assert_pack_unpack(test_name).await?;
        // Write "Goodbye World!" out to the same file
        File::create(&versioned_file_path)?.write_all(goodbye_bytes)?;
        // Run the test again
        assert_pack_unpack(test_name).await?;

        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.load_key().await?;
        let config = global.get_bucket(origin).unwrap();
        let (metadata_forest, content_forest, current_dir, _, _) =
            &mut config.get_all(&wrapping_key).await?;

        // Describe path of the PrivateFile relative to the root directory
        let path_segments: Vec<String> = vec!["0".to_string(), "0".to_string()];
        let current_file = current_dir
            .get_node(&path_segments, false, metadata_forest, &config.metadata)
            .await?
            .unwrap()
            .as_file()?;
        let current_content = current_file
            .get_content(content_forest, &config.content)
            .await?;
        let mut current_content_decompressed: Vec<u8> = Vec::new();
        wnfsio::decompress_bytes(
            current_content.as_slice(),
            &mut current_content_decompressed,
        )?;
        // Assert that the current version of the file was retrieved correctly
        assert_eq!(goodbye_bytes, current_content_decompressed);

        // Now grab history
        let mut iterator = config.get_history(&wrapping_key).await?;

        // Get the previous version of the root of the PrivateDirectory
        let previous_root = iterator
            .get_previous(&config.metadata)
            .await?
            .unwrap()
            .as_dir()?;

        // Grab the previous version of the PrivateFile
        let previous_file = previous_root
            .get_node(&path_segments, false, metadata_forest, &config.metadata)
            .await?
            .unwrap()
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let previous_content = previous_file
            .get_content(content_forest, &config.content)
            .await
            .unwrap();
        let mut previous_content_decompressed: Vec<u8> = Vec::new();
        wnfsio::decompress_bytes(
            previous_content.as_slice(),
            &mut previous_content_decompressed,
        )?;
        // Assert that the previous version of the file was retrieved correctly
        assert_eq!(previous_content_decompressed, still_bytes);

        // Get the original version of the root of the PrivateDirectory
        let original_root = iterator
            .get_previous(&config.metadata)
            .await?
            .unwrap()
            .as_dir()?;

        // Grab the original version of the PrivateFile
        let original_file = original_root
            .get_node(&path_segments, false, metadata_forest, &config.metadata)
            .await?
            .unwrap()
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let original_content = original_file
            .get_content(content_forest, &config.content)
            .await
            .unwrap();
        let mut original_content_decompressed: Vec<u8> = Vec::new();
        wnfsio::decompress_bytes(
            original_content.as_slice(),
            &mut original_content_decompressed,
        )?;
        // Assert that the previous version of the file was retrieved correctly
        assert_eq!(original_content_decompressed, hello_bytes);

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
    async fn versioning_simple() -> Result<()> {
        let test_name = "versioning_simple";
        let structure = Structure::new(1, 1, 2000, Strategy::Simple);
        // Setup the test once
        let origin = &test_setup_structured(test_name, structure).await?;

        // Path for the actual file on disk that we'll be writing
        let versioned_file_path = origin.join("0");

        // Define bytes for each message
        let hello_bytes = "Hello World!".as_bytes();
        let goodbye_bytes = "Goodbye World!".as_bytes();

        println!("hb: {:?}\ngb: {:?}", hello_bytes, goodbye_bytes);

        // Write "Hello World!" out to the file; v0
        File::create(&versioned_file_path)?.write_all(hello_bytes)?;
        // Run the test
        assert_pack_unpack(test_name).await?;
        // Write "Goodbye World!" out to the same file
        File::create(&versioned_file_path)?.write_all(goodbye_bytes)?;
        // Run the test again
        assert_pack_unpack(test_name).await?;

        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.load_key().await?;
        let config = global.get_bucket(origin).unwrap();
        let (metadata_forest, content_forest, current_dir, _, _) =
            &mut config.get_all(&wrapping_key).await?;

        // Describe path of the PrivateFile relative to the root directory
        let path_segments: Vec<String> = vec!["0".to_string()];
        let current_file = current_dir
            .get_node(&path_segments, false, metadata_forest, &config.metadata)
            .await?
            .unwrap()
            .as_file()?;
        let current_content = current_file
            .get_content(content_forest, &config.content)
            .await?;
        let mut current_content_decompressed: Vec<u8> = Vec::new();
        wnfsio::decompress_bytes(
            current_content.as_slice(),
            &mut current_content_decompressed,
        )?;
        // Assert that the current version of the file was retrieved correctly
        assert_eq!(goodbye_bytes, current_content_decompressed);

        // Now grab history
        let mut iterator = config.get_history(&wrapping_key).await?;

        // Get the previous version of the root of the PrivateDirectory
        let previous_root = iterator
            .get_previous(&config.metadata)
            .await?
            .unwrap()
            .as_dir()?;

        // Grab the previous version of the PrivateFile
        let previous_file = previous_root
            .get_node(&path_segments, false, metadata_forest, &config.metadata)
            .await?
            .unwrap()
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let previous_content = previous_file
            .get_content(content_forest, &config.content)
            .await
            .unwrap();
        let mut previous_content_decompressed: Vec<u8> = Vec::new();
        wnfsio::decompress_bytes(
            previous_content.as_slice(),
            &mut previous_content_decompressed,
        )?;

        // Assert that the previous version of the file was retrieved correctly
        assert_eq!(previous_content_decompressed, hello_bytes);

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
    async fn symlinks() -> Result<()> {
        let test_name = "symlinks";

        // Setup the test
        let origin = &test_setup(test_name).await?;

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
        assert_pack_unpack(test_name).await?;

        // Teardown
        test_teardown(test_name).await
    }
}
