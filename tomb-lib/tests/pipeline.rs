use dir_assert::assert_paths;
use fake_file::{Strategy, Structure};
use std::{path::Path, process::Command};
use tomb_lib::{
    pipelines::{pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline},
    utils::fs::{ensure_path_exists_and_is_dir, ensure_path_exists_and_is_empty_dir},
};

const INPUT_PATH: &str = "input";
const PACKED_PATH: &str = "packed";
const UNPACKED_PATH: &str = "unpacked";

/// Helper function to setup a test
/// # Arguments
/// * test_path: Where we store artefacts for the test
/// * structure: The structure of the test
/// * test_name: The name of the test
fn setup_test(test_path: &Path, structure: Structure, test_name: &str) {
    // Declare Paths for the Input, Packed, Unpacked, and Manifest
    let mut input_path = test_path.join(INPUT_PATH);
    let packed_path = test_path.join(PACKED_PATH);
    let unpacked_path = test_path.join(UNPACKED_PATH);
    // Prepare the test structure
    ensure_path_exists_and_is_empty_dir(&input_path, true).unwrap();
    input_path.push(test_name);
    structure.generate(&input_path).unwrap();
    ensure_path_exists_and_is_empty_dir(&packed_path, true).unwrap();
    ensure_path_exists_and_is_empty_dir(&unpacked_path, true).unwrap();
}

/// Helper function to run a test end to end
/// # Arguments
/// * test_path: Where we store artefacts for the test
async fn run_test(test_path: &Path) {
    // Declare Paths for the Input, Packed, Unpacked, and Manifest
    let input_path = test_path.join(INPUT_PATH);
    let packed_path = test_path.join(PACKED_PATH);
    let unpacked_path = test_path.join(UNPACKED_PATH);

    // Pack the input
    pack_pipeline(&input_path, &packed_path, 262144, true)
        .await
        .unwrap();

    // Unpack the output
    unpack_pipeline(&packed_path, &unpacked_path).await.unwrap();

    // checks if two directories are the same
    assert_paths(input_path.clone(), unpacked_path.clone()).unwrap();
}

//TODO(organizedgrime) - Move this into fakefile
// Determines the size of the contents of a directory.
// This standard unix tool handles far more edge cases than we could ever hope
// to approximate with a hardcoded recursion step, and with more efficiency too.
fn compute_directory_size(path: &Path) -> Result<usize, ()> {
    // Execute the unix du command to evaluate the size of the given path in kilobytes
    let du_result = Command::new("du")
        .arg("-sh")
        .arg("-k")
        .arg(path.display().to_string())
        .output();

    // Handle the output of this command
    match du_result {
        // Command executed successfully
        Ok(output) => {
            // Interpret the output as a string
            let output_str = String::from_utf8(output.stdout).unwrap();
            // Grab all text before the tab
            let size_str = output_str.split('\t').next().unwrap();
            // Parse that text as a number
            let size = size_str.parse::<usize>().unwrap();
            // Ok status with size
            Ok(size)
        }
        // Something went wrong, this should never happen in a test but may in other use cases
        Err(_e) => Err(()),
    }
}

/// Small Input End to End Integration Tests for the Pipeline
#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use wnfs::private::PrivateNodeOnPathHistory;
    use std::{path::Path, rc::Rc};
    use tokio::{
        fs::{read_link, symlink, symlink_metadata, File},
        io::AsyncWriteExt,
    };
    use tomb_lib::utils::pipeline::{load_pipeline, load_dir};

    // Configure where tests are run
    const TEST_PATH: &str = "test";
    // Configure the test sets
    const TEST_MAX_WIDTH: usize = 4;
    const TEST_MAX_DEPTH: usize = 4;
    const TEST_INPUT_SIZE: usize = 1024 * 1024; // 1MB

    /// Test the pipeline with a small file structure
    #[tokio::test]
    async fn test_simple() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("simple");
        // Define the file structure to test
        let desired_structure = Structure::new(
            TEST_MAX_WIDTH, // width
            TEST_MAX_DEPTH, // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );
        // Setup the test
        setup_test(&test_path, desired_structure, "test_simple");
        // Run the test
        run_test(&test_path).await;
    }

    /// Test the pipeline with a very deep file structure
    #[tokio::test]
    async fn test_deep() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("deep");
        // Define the file structure to test
        let desired_structure = Structure::new(
            2, // width
            8, // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );
        // Setup the test
        setup_test(&test_path, desired_structure, "test_deep");
        // Run the test
        run_test(&test_path).await;
    }

    /// Test the pipeline with a very wide file structure
    #[tokio::test]
    async fn test_wide() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("wide");
        // Define the file structure to test
        let desired_structure = Structure::new(
            16, // width
            2,  // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );
        // Setup the test
        setup_test(&test_path, desired_structure, "test_wide");
        // Run the test
        run_test(&test_path).await;
    }

    /// Test with one very big file -- ignore cuz it takes a while
    #[tokio::test]
    #[ignore]
    async fn test_big_file() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("big_file");
        // Define the file structure to test
        let desired_structure = Structure::new(0, 0, TEST_INPUT_SIZE * 100, Strategy::Simple);
        // Setup the test
        setup_test(&test_path, desired_structure, "test_big_file");
        // Run the test
        run_test(&test_path).await;
    }

    /// Ensure that the pipeline can recover duplicate files
    #[tokio::test]
    async fn test_deduplication_integrity() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH).join("deduplication_integrity");
        // Define the file structure to test
        let structure = Structure::new(2, 2, TEST_INPUT_SIZE, Strategy::Simple);
        // Setup the test
        setup_test(&test_path, structure, "duplicate_directory");
        // Duplicate the test file
        let input_path = test_path.join(INPUT_PATH);
        // Copy $input_path/test_duplicate to $input_path/encloser
        let original_path = input_path.join("duplicate_directory");
        // Enclose the duplicate in multiple parent directories
        let encloser_path = input_path.join("encloser1").join("encloser2");
        // Create the directory
        ensure_path_exists_and_is_dir(&encloser_path).unwrap();
        // Copy the contents of the original directory into the new directory
        fs_extra::dir::copy(
            &original_path,
            &encloser_path,
            &fs_extra::dir::CopyOptions::new(),
        )
        .unwrap();

        // Run the test to ensure input = output
        run_test(&test_path).await;
    }

    /// Ensure that the duplicate data occupies a smaller footprint when packed
    //TODO (organizedgrime) - This test is a bit longer than I would like, might modify it to be more modular / reusable
    #[tokio::test]
    async fn test_deduplication_size() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH).join("deduplication_size");

        // Empty that test directory! Because we're doing setup a little bit differently here,
        // it seems that my OSX machine occasionally generates metadata files that cause the test to fail.
        // Emptying this directory each time prevents this.
        ensure_path_exists_and_is_empty_dir(&test_path, true).unwrap();

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
        run_test(&twin_dups).await;
        run_test(&twin_unique).await;

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
        setup_test(&test_path, desired_structure, "0");

        // Duplicate the file in place
        fs_extra::file::copy(
            test_path.join(INPUT_PATH).join("0"),
            test_path.join(INPUT_PATH).join("1"),
            &fs_extra::file::CopyOptions::new(),
        )
        .unwrap();

        // Run the test
        run_test(&test_path).await;

        // Assert that only one file was packed
        let packed_path = test_path.join(PACKED_PATH);
        let dir_info = fs_extra::dir::get_dir_content(packed_path).unwrap();
        // Expect that the large file was packed into two files
        assert_eq!(dir_info.files.len(), 2);
    }

    #[tokio::test]
    async fn test_double_packing() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("double_pack");
        let test_name = "double_pack";
        // Define the file structure to test
        let desired_structure = Structure::new(
            2, // width
            2, // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );
        // Setup the test once
        setup_test(&test_path, desired_structure, test_name);

        // Run the test twice
        run_test(&test_path).await;
        run_test(&test_path).await;
    }

    // TODO (organizedgrime) - reimplement this when we have migrated from using Ratchets to WNFS's new solution.
    #[tokio::test]
    /// This test fails randomly and succeeds randomly- TODO fix or just wait until WNFS people fix their code.
    async fn test_versioning() -> Result<()> {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("versioning");
        let test_name = "versioning";

        // Define the file structure to test
        let desired_structure = Structure::new(
            2, // width
            2, // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );

        // Setup the test once
        setup_test(&test_path, desired_structure, test_name);

        // Path for the actual file on disk that we'll be writing
        let versioned_file_path = test_path
            .join("input")
            .join("versioning")
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
        File::create(&versioned_file_path)
            .await
            .unwrap()
            .write_all(hello_bytes)
            .await
            .unwrap();

        println!("running for the first time...");

        // Run the test
        run_test(&test_path).await;

        // Write "Still there, World?" out to the same file
        File::create(&versioned_file_path)
            .await
            .unwrap()
            .write_all(still_bytes)
            .await
            .unwrap();

        // Run the test again
        run_test(&test_path).await;

        // Write "Goodbye World!" out to the same file
        File::create(&versioned_file_path)
            .await
            .unwrap()
            .write_all(goodbye_bytes)
            .await
            .unwrap();

        // Run the test again
        run_test(&test_path).await;

        // The path in which we expect to find metadata
        let tomb_path = &test_path.join("unpacked").join(".tomb");
        let (key, manifest_data, mut forest, dir) = load_pipeline(tomb_path).await?;
        let past_dir = load_dir(&manifest_data, &key, &mut forest, "original_root").await?;

        let mut iterator = PrivateNodeOnPathHistory::of(
            dir,
            past_dir,
            1_000_000,
            &[],
            true,
            Rc::clone(&forest),
            &manifest_data.content_store,
        )
        .await
        .unwrap();

        // Get the previous version of the root of the PrivateDirectory
        let previous_root = iterator
            .get_previous(&manifest_data.content_store)
            .await
            .unwrap()
            .unwrap()
            .as_dir()
            .unwrap();

        // Describe path of the PrivateFile relative to the root directory
        let path_segments: Vec<String> =
            vec!["versioning".to_string(), "0".to_string(), "0".to_string()];

        // Grab the previous version of the PrivateFile
        let previous_file = previous_root
            .get_node(&path_segments, true, &forest, &manifest_data.content_store)
            .await
            .unwrap()
            .unwrap()
            .as_file()
            .unwrap();

        // Grab the previous version of the PrivateFile content
        let previous_content = previous_file
            .get_content(&forest, &manifest_data.content_store)
            .await
            .unwrap();

        // Assert that the previous version of the file was retrieved correctly
        assert!(previous_content != goodbye_bytes);

        // Get the original version of the root of the PrivateDirectory
        let original_root = iterator
            .get_previous(&manifest_data.content_store)
            .await
            .unwrap()
            .unwrap()
            .as_dir()
            .unwrap();

        // Grab the original version of the PrivateFile
        let original_file = original_root
            .get_node(&path_segments, true, &forest, &manifest_data.content_store)
            .await
            .unwrap()
            .unwrap()
            .as_file()
            .unwrap();

        // Grab the previous version of the PrivateFile content
        let original_content = original_file
            .get_content(&forest, &manifest_data.content_store)
            .await
            .unwrap();

        // Assert that the previous version of the file was retrieved correctly
        assert!(original_content != goodbye_bytes);

        // Assert that there are no more previous versions to find
        assert!(iterator
            .get_previous(&manifest_data.content_store)
            .await
            .unwrap()
            .is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_symlinks() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("symlinks");
        // Define the file structure to test
        let desired_structure = Structure::new(
            2, // width
            2, // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );

        // Setup the test
        setup_test(&test_path, desired_structure, "symlinks");

        // Path in which Directory symlink will be created
        let sym_dir_root = test_path.join("input").join("symlinks");
        // Path in which File symlink will be created
        let sym_file_root = sym_dir_root.join("0");

        // Point from /input/symlinks/ZZ -> /input/symlinks/0
        let dir_original = sym_dir_root.join("0").canonicalize().unwrap();
        let dir_sym = sym_dir_root.join("ZZ");

        // Point from /input/symlinks/0/ZZ -> /input/symlinks/0/0
        let file_original = sym_file_root.join("0").canonicalize().unwrap();
        let file_sym = sym_file_root.join("ZZ");

        // Create those symbolic links in the actual filesystem
        symlink(&dir_original, &dir_sym).await.unwrap();
        symlink(&file_original, &file_sym).await.unwrap();

        // Assert that both of the paths are symlinks using their metadata
        assert!(symlink_metadata(&dir_sym).await.unwrap().is_symlink());
        assert!(symlink_metadata(&file_sym).await.unwrap().is_symlink());

        // Assert that both of them point to the location we expect them to
        assert_eq!(dir_original, read_link(dir_sym).await.unwrap());
        assert_eq!(file_original, read_link(file_sym).await.unwrap());

        // Run the test on the created filesystem
        run_test(&test_path).await;
    }
}
