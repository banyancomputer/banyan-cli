/// This module contains the add pipeline function, which is the main entry point for inserting into existing WNFS filesystems.
pub mod add;
/// This module contains the encryption pipeline function, which is the main entry point for bundling new data.
pub mod bundle;
/// This module contains configuration functions for the cli
pub mod configure;
/// Pipeline Errors
pub mod error;
/// This module contains the decryption pipeline function, which is the main entry point for extracting previously bundled data.
pub mod extract;
/// This module contains the add pipeline function, which is the main entry point for removing from existing WNFS filesystems.
pub mod remove;

#[cfg(test)]
mod test {
    use super::{add, error::TombError};
    use crate::{
        cli::specifiers::BucketSpecifier,
        pipelines::{bundle, configure, extract, remove},
        types::config::globalconfig::GlobalConfig,
        utils::{
            test::{test_setup, test_setup_structured, test_teardown},
            wnfsio::compute_directory_size,
        },
    };
    use tomb_common::utils::wnfsio::{decompress_bytes, path_to_segments};

    use anyhow::Result;
    use dir_assert::assert_paths;
    use fake_file::{utils::ensure_path_exists_and_is_empty_dir, Strategy, Structure};
    use fs_extra::dir;
    use serial_test::serial;
    use std::{
        fs::{create_dir_all, read_link, rename, symlink_metadata, File},
        io::Write,
        os::unix::fs::symlink,
        path::{Path, PathBuf},
    };

    /// Simplified Bundle call function
    async fn bundle_pipeline(bucket_specifier: &BucketSpecifier) -> Result<String, TombError> {
        bundle::pipeline(
            &mut GlobalConfig::from_disk().await?,
            bucket_specifier,
            true,
        )
        .await
    }

    /// Simplified Extract call function
    async fn extract_pipeline(
        bucket_specifier: &BucketSpecifier,
        extracted: &Path,
    ) -> Result<String, TombError> {
        extract::pipeline(
            &GlobalConfig::from_disk().await?,
            bucket_specifier,
            extracted,
        )
        .await
    }

    #[tokio::test]
    #[serial]
    async fn init() -> Result<()> {
        let test_name = "init";
        // Create the setup conditions
        let (origin, bucket_specifier) = &test_setup(test_name).await?;
        // Deinitialize for user
        configure::deinit(origin).await?;
        // Assert that bundling fails
        assert!(bundle_pipeline(bucket_specifier).await.is_err());
        // Initialize for this user
        configure::init(origin).await?;
        // Assert that a config exists for this bucket now
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket_by_origin(origin)
            .is_some());
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn configure_remote() -> Result<()> {
        let address = "http://app.tomb.com.net.org:5423/";
        configure::deinit_all().await?;
        let _ = GlobalConfig::from_disk().await?;
        // Configure the remote endpoint
        configure::remote_core(address).await?;
        // Assert it was actually modified
        assert_eq!(
            GlobalConfig::from_disk().await?.remote_core,
            address.to_string()
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn bundle() -> Result<()> {
        let test_name = "bundle";
        // Create the setup conditions
        let (origin, bucket_specifier) = &test_setup(test_name).await?;
        // Initialize
        configure::init(origin).await?;
        // Bundle
        bundle_pipeline(bucket_specifier).await?;
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn extract() -> Result<()> {
        let test_name = "extract";
        // Create the setup conditions
        let (origin, bucket_specifier) = &test_setup(test_name).await?;
        // Initialize
        configure::init(origin).await?;
        // Bundle locally
        bundle_pipeline(bucket_specifier).await?;
        // Create a new dir to extract in
        let extracted_dir = &origin
            .parent()
            .expect("origin has no parent")
            .join(format!("{}_extracted", test_name));
        create_dir_all(extracted_dir)?;
        // Run the extracting pipeline
        extract_pipeline(bucket_specifier, extracted_dir).await?;
        // Assert the pre-bundled and extracted directories are identical
        assert_paths(origin, extracted_dir).expect("extracted dir does not match origin");
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn add() -> Result<()> {
        let test_name = "add";
        // Create the setup conditions
        let (origin, bucket_specifier) = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(origin).await?;
        // Run the bundle pipeline
        bundle_pipeline(bucket_specifier).await?;
        // This is still in the input dir. Technically we could just
        let input_file = &origin.join("hello.txt");
        // Content to be written to the file
        let file_content = String::from("This is just example text.")
            .as_bytes()
            .to_vec();
        // Create and write to the file
        File::create(input_file)?.write_all(&file_content)?;
        // Add the input file to the WNFS
        add::pipeline(bucket_specifier, input_file, input_file).await?;

        // Now that the pipeline has run, grab all metadata
        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.clone().wrapping_key().await?;
        let config = global
            .get_bucket_by_origin(origin)
            .expect("bucket config does not exist for this origin");
        let fs = config.unlock_fs(&wrapping_key).await?;

        // Grab the file at this path
        let file = fs
            .root_dir
            .get_node(
                &path_to_segments(input_file)?,
                true,
                &fs.forest,
                &config.metadata,
            )
            .await?
            .expect("node does not exist in WNFS PrivateDirectory")
            .as_file()?;
        // Get the content of the PrivateFile and decompress it
        let mut loaded_file_content: Vec<u8> = Vec::new();
        decompress_bytes(
            file.get_content(&fs.forest, &config.content)
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
        let (origin, bucket_specifier) = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(origin).await?;
        // Run the bundle pipeline
        bundle_pipeline(bucket_specifier).await?;
        // Write out a reference to where we expect to find this file
        let wnfs_path = &PathBuf::from("").join("0").join("0");
        let wnfs_segments = path_to_segments(wnfs_path)?;
        // Load metadata
        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.clone().wrapping_key().await?;
        let config = global
            .get_bucket_by_origin(origin)
            .expect("bucket config does not exist for this origin");
        let fs = config.unlock_fs(&wrapping_key).await?;
        let result = fs
            .root_dir
            .get_node(&wnfs_segments, true, &fs.forest, &config.metadata)
            .await?;
        // Assert the node exists presently
        assert!(result.is_some());
        // Remove the PrivateFile at this Path
        remove::pipeline(bucket_specifier, wnfs_path).await?;
        // Reload metadata
        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.clone().wrapping_key().await?;
        let config = global
            .get_bucket_by_origin(origin)
            .expect("bucket config does not exist for this origin");
        let fs = config.unlock_fs(&wrapping_key).await?;
        let result = fs
            .root_dir
            .get_node(&wnfs_segments, true, &fs.forest, &config.metadata)
            .await?;
        // Assert the node no longer exists
        assert!(result.is_none());
        // Teardown
        test_teardown(test_name).await?;

        Ok(())
    }

    // Helper function for structure tests
    async fn assert_bundle_extract(test_name: &str) -> Result<()> {
        // Grab directories
        let root_path = PathBuf::from("test").join(test_name);
        let origin = &root_path.join("input");
        let bucket_specifier = &BucketSpecifier::with_origin(origin);
        // Initialize
        configure::init(origin).await?;
        // Bundle locally
        bundle_pipeline(bucket_specifier).await?;
        println!("finished bundling...");
        // Create a new dir to extract in
        let extracted_dir = &origin
            .parent()
            .expect("origin has no parent")
            .join("extracted");
        create_dir_all(extracted_dir)?;
        // Run the extracting pipeline
        extract_pipeline(bucket_specifier, extracted_dir).await?;
        // Assert the pre-bundled and extracted directories are identical
        assert_paths(origin, extracted_dir).expect("extracted dir does not match origin");
        Ok(())
    }

    const TEST_INPUT_SIZE: usize = 1024;

    #[tokio::test]
    #[serial]
    async fn structure_simple() -> Result<()> {
        let test_name = "structure_simple";
        let structure = Structure::new(4, 4, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_bundle_extract(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn structure_deep() -> Result<()> {
        let test_name = "structure_deep";
        let structure = Structure::new(2, 8, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_bundle_extract(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn structure_wide() -> Result<()> {
        let test_name = "structure_deep";
        let structure = Structure::new(16, 1, TEST_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_bundle_extract(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn big_file() -> Result<()> {
        let test_name = "big_file";
        let structure = Structure::new(1, 1, 1024 * 1024 * 10, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_bundle_extract(test_name).await?;
        test_teardown(test_name).await
    }

    /// Ensure that the pipeline can recover duplicate files
    #[tokio::test]
    #[serial]
    async fn deduplication_integrity() -> Result<()> {
        let test_name = "deduplication_integrity";
        // Setup the test
        let (origin, _) = &test_setup(test_name).await?;
        let dup_origin = &origin.parent().expect("origin has no parent").join("dups");
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
        assert_bundle_extract(test_name).await?;
        test_teardown(test_name).await
    }

    // / Ensure that the duplicate data occupies a smaller footprint when bundled
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
        assert_bundle_extract(test_name_dup).await?;
        assert_bundle_extract(test_name_unique).await?;

        // Get configs
        let global = GlobalConfig::from_disk().await?;
        // Compute the sizes of these directories
        let bundled_dups_size = compute_directory_size(
            &global
                .get_bucket_by_origin(origin_dup)
                .expect("bucket config does not exist for this origin")
                .content
                .path,
        )? as f64;
        let bundled_unique_size = compute_directory_size(
            &global
                .get_bucket_by_origin(origin_unique)
                .expect("bucket config does not exist for this origin")
                .content
                .path,
        )? as f64;

        // Ensure that the size of the bundled duplicates directory is approximately half that of the unique directory
        println!("unique {} dup {}", bundled_unique_size, bundled_dups_size);
        assert!(bundled_unique_size / bundled_dups_size >= 1.8);

        test_teardown(test_name_dup).await?;
        test_teardown(test_name_unique).await
    }

    #[tokio::test]
    #[serial]
    async fn double_bundling() -> Result<()> {
        let test_name = "double_bundling";
        // Setup the test once
        test_setup(test_name).await?;
        // Run the test twice
        assert_bundle_extract(test_name).await?;
        assert_bundle_extract(test_name).await?;
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn versioning_complex() -> Result<()> {
        let test_name = "versioning_complex";
        let structure = Structure::new(2, 2, 2000, Strategy::Simple);
        // Setup the test once
        let (origin, _) = &test_setup_structured(test_name, structure).await?;

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
        assert_bundle_extract(test_name).await?;
        // Write "Still there, World?" out to the same file
        File::create(&versioned_file_path)?.write_all(still_bytes)?;
        // Run the test again
        assert_bundle_extract(test_name).await?;
        // Write "Goodbye World!" out to the same file
        File::create(&versioned_file_path)?.write_all(goodbye_bytes)?;
        // Run the test again
        assert_bundle_extract(test_name).await?;

        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.clone().wrapping_key().await?;
        let config = global
            .get_bucket_by_origin(origin)
            .expect("bucket config does not exist for this origin");
        let fs = config.unlock_fs(&wrapping_key).await?;

        // Describe path of the PrivateFile relative to the root directory
        let path_segments: Vec<String> = vec!["0".to_string(), "0".to_string()];
        let current_file = fs
            .root_dir
            .get_node(&path_segments, false, &fs.forest, &config.metadata)
            .await?
            .expect("node does not exist in WNFS PrivateDirectory")
            .as_file()?;
        let current_content = current_file
            .get_content(&fs.forest, &config.content)
            .await?;
        let mut current_content_decompressed: Vec<u8> = Vec::new();
        decompress_bytes(
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
            .expect("cannot traverse history iterator")
            .as_dir()?;

        // Grab the previous version of the PrivateFile
        let previous_file = previous_root
            .get_node(&path_segments, false, &fs.forest, &config.metadata)
            .await?
            .expect("node does not exist in WNFS PrivateDirectory")
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let previous_content = previous_file
            .get_content(&fs.forest, &config.content)
            .await
            .expect("failed to retrieve file content");
        let mut previous_content_decompressed: Vec<u8> = Vec::new();
        decompress_bytes(
            previous_content.as_slice(),
            &mut previous_content_decompressed,
        )?;
        // Assert that the previous version of the file was retrieved correctly
        assert_eq!(previous_content_decompressed, still_bytes);

        // Get the original version of the root of the PrivateDirectory
        let original_root = iterator
            .get_previous(&config.metadata)
            .await?
            .expect("cannot traverse history iterator")
            .as_dir()?;

        // Grab the original version of the PrivateFile
        let original_file = original_root
            .get_node(&path_segments, false, &fs.forest, &config.metadata)
            .await?
            .expect("node does not exist in WNFS PrivateDirectory")
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let original_content = original_file
            .get_content(&fs.forest, &config.content)
            .await
            .expect("failed to retrieve file content");
        let mut original_content_decompressed: Vec<u8> = Vec::new();
        decompress_bytes(
            original_content.as_slice(),
            &mut original_content_decompressed,
        )?;
        // Assert that the previous version of the file was retrieved correctly
        assert_eq!(original_content_decompressed, hello_bytes);

        // Assert that there are no more previous versions to find
        assert!(iterator
            .get_previous(&config.metadata)
            .await
            .expect("cannot traverse history iterator")
            .is_none());

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn versioning_simple() -> Result<()> {
        let test_name = "versioning_simple";
        let structure = Structure::new(1, 1, 2000, Strategy::Simple);
        // Setup the test once
        let (origin, _) = &test_setup_structured(test_name, structure).await?;

        // Path for the actual file on disk that we'll be writing
        let versioned_file_path = origin.join("0");

        // Define bytes for each message
        let hello_bytes = "Hello World!".as_bytes();
        let goodbye_bytes = "Goodbye World!".as_bytes();

        println!("hb: {:?}\ngb: {:?}", hello_bytes, goodbye_bytes);

        // Write "Hello World!" out to the file; v0
        File::create(&versioned_file_path)?.write_all(hello_bytes)?;
        // Run the test
        assert_bundle_extract(test_name).await?;
        // Write "Goodbye World!" out to the same file
        File::create(&versioned_file_path)?.write_all(goodbye_bytes)?;
        // Run the test again
        assert_bundle_extract(test_name).await?;

        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.clone().wrapping_key().await?;
        let config = global
            .get_bucket_by_origin(origin)
            .expect("bucket config does not exist for this origin");
        let fs = config.unlock_fs(&wrapping_key).await?;

        // Describe path of the PrivateFile relative to the root directory
        let path_segments: Vec<String> = vec!["0".to_string()];
        let current_file = fs
            .root_dir
            .get_node(&path_segments, false, &fs.forest, &config.metadata)
            .await?
            .expect("node does not exist in WNFS PrivateDirectory")
            .as_file()?;
        let current_content = current_file
            .get_content(&fs.forest, &config.content)
            .await?;
        // Assert that the current version of the file was retrieved correctly
        assert_eq!(goodbye_bytes, current_content);

        // Now grab history
        let mut iterator = config.get_history(&wrapping_key).await?;

        // Get the previous version of the root of the PrivateDirectory
        let previous_root = iterator
            .get_previous(&config.metadata)
            .await?
            .expect("cannot traverse history iterator")
            .as_dir()?;

        // Grab the previous version of the PrivateFile
        let previous_file = previous_root
            .get_node(&path_segments, false, &fs.forest, &config.metadata)
            .await?
            .expect("node does not exist in WNFS PrivateDirectory")
            .as_file()?;

        // Grab the previous version of the PrivateFile content
        let previous_content = previous_file
            .get_content(&fs.forest, &config.content)
            .await
            .expect("failed to retrieve file content");

        // Assert that the previous version of the file was retrieved correctly
        assert_eq!(previous_content, hello_bytes);

        // pull off the last, empty version
        let _empty_dir = iterator
            .get_previous(&config.metadata)
            .await?
            .expect("cannot traverse history iterator")
            .as_dir()?;

        // Assert that there are no more previous versions to find
        assert!(iterator
            .get_previous(&config.metadata)
            .await
            .expect("cannot traverse history iterator")
            .is_none());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn symlinks() -> Result<()> {
        let test_name = "symlinks";

        // Setup the test
        let (origin, _) = &test_setup(test_name).await?;

        // Path in which File symlink will be created
        let sym_file_root = origin.join("0");

        // Point from /input/symlinks/ZZ -> /input/symlinks/0
        let dir_original = origin.join("0").canonicalize()?;
        let dir_sym = origin.join("ZZ");

        // Point from /input/symlinks/0/ZZ -> /input/symlinks/0/0
        let file_original = sym_file_root.join("0").canonicalize()?;
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
        assert_bundle_extract(test_name).await?;

        // Teardown
        test_teardown(test_name).await
    }
}
