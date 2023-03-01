use dataprep_pipelines::utils::test::{
    pipeline_test as _pipeline_test, setup_test_structure as _setup_test_structure,
};
use fake_file::{Strategy, Structure};
use std::path::Path;
use std::process::Command;

const INPUT_PATH: &str = "input";
const PACKED_PATH: &str = "packed";
const UNPACKED_PATH: &str = "unpacked";
const MANIFEST_PATH: &str = "manifest.json";

// TODO (amiller68): This is gross
/// Helper function to setup a test
/// # Arguments
/// * test_path: Where we store artefacts for the test
/// * structure: The structure of the test
/// * test_name: The name of the test
fn setup_test(test_path: &Path, structure: Structure, test_name: &str) {
    // Declare Paths for the Input, Packed, Unpacked, and Manifest
    let input_path = test_path.join(INPUT_PATH);
    let packed_path = test_path.join(PACKED_PATH);
    let unpacked_path = test_path.join(UNPACKED_PATH);
    let manifest_path = test_path.join(MANIFEST_PATH);

    // Setup the test structure
    _setup_test_structure(
        &input_path,
        &packed_path,
        &unpacked_path,
        &manifest_path,
        structure,
        test_name,
    );
}

/// Helper function to run a test end to end
/// # Arguments
/// * test_path: Where we store artefacts for the test
/// * test_name: The name of the test
async fn run_test(test_path: &Path) {
    // Declare Paths for the Input, Packed, Unpacked, and Manifest
    let input_path = test_path.join(INPUT_PATH);
    let packed_path = test_path.join(PACKED_PATH);
    let unpacked_path = test_path.join(UNPACKED_PATH);
    let manifest_path = test_path.join(MANIFEST_PATH);

    // Run the pipeline and check
    _pipeline_test(
        input_path.clone(),
        packed_path.clone(),
        unpacked_path.clone(),
        manifest_path.clone(),
    )
    .await
}

//TODO(organizedgrime) - Move this into fakefile
// Determines the size of the contents of a directory.
// This standard unix tool handles far more edge cases than we could ever hope
// to approximate with a hardcoded recursion step.
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
            let size_str = output_str.split("\t").next().unwrap();
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
    use dataprep_pipelines::utils::fs::ensure_path_exists_and_is_dir;
    use std::path::Path;

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
        let desired_structure = Structure::new(0, 0, TEST_INPUT_SIZE * 1024, Strategy::Simple);
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
        let packed_dups_size = compute_directory_size(&packed_dups_path).unwrap();
        let packed_unique_size = compute_directory_size(&packed_unique_path).unwrap();
        // Ensure that the size of the packed duplicates directory is smaller than the packed unique directory
        assert!(packed_dups_size < packed_unique_size);
    }
}
