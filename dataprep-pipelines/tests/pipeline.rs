use dataprep_pipelines::utils::test::{pipeline_test, setup_test_structure};
use fake_file::{Strategy, Structure};
// use lazy_static::lazy_static;

// lazy_static! {
//     // Where to work on test outputs when running tests
//     static ref TEST_PATH: String = "test";
//     static ref INPUT_PATH: &'static str = "input";
//     static ref PACKED_PATH: &'static str = "packed";
//     static ref UNPACKED_PATH:  &'static str = "unpacked";
//     static ref MANIFEST_PATH: &'static str = "manifest.json";
// }

/// Small Input End to End Integration Tests for the Pipeline
#[cfg(test)]
mod test {
    use super::*;
    use dataprep_pipelines::utils::fs::ensure_path_exists_and_is_empty_dir;
    use std::path::Path;

    // Configure where tests are run
    const TEST_PATH: &str = "test";
    const INPUT_PATH: &str = "input";
    const PACKED_PATH: &str = "packed";
    const UNPACKED_PATH: &str = "unpacked";
    const MANIFEST_PATH: &str = "manifest.json";

    const TEST_INPUT_SIZE: usize = 1024 * 1024; // 1MB
    const TEST_MAX_WIDTH: usize = 4;
    const TEST_MAX_DEPTH: usize = 4;

    /// Test the pipeline with a small file structure
    #[tokio::test]
    async fn test_simple() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("simple");

        // Declare Paths for the Input, Packed, Unpacked, and Manifest
        let input_path = test_path.join(INPUT_PATH);
        let packed_path = test_path.join(PACKED_PATH);
        let unpacked_path = test_path.join(UNPACKED_PATH);
        let manifest_path = test_path.join(MANIFEST_PATH);

        // Define the file structure to test
        let desired_structure = Structure::new(
            TEST_MAX_WIDTH, // width
            TEST_MAX_DEPTH, // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );

        // Setup the test structure
        setup_test_structure(
            &test_path,
            &input_path,
            &packed_path,
            &unpacked_path,
            &manifest_path,
            desired_structure,
            "test_simple",
        );

        // Run the transform and check
        pipeline_test(
            input_path.clone(),
            packed_path.clone(),
            unpacked_path.clone(),
            manifest_path.clone(),
        )
        .await;
    }

    /// Test the pipeline with a trivial duplicated file structure
    #[tokio::test]
    #[ignore]
    async fn test_duplicate() {
        // Create a new path for this test
        let test_path = Path::new(TEST_PATH);
        let test_path = test_path.join("duplicate");

        // Declare Paths for the Input, Packed, Unpacked, and Manifest
        let input_path = test_path.join(INPUT_PATH);
        let packed_path = test_path.join(PACKED_PATH);
        let unpacked_path = test_path.join(UNPACKED_PATH);
        let manifest_path = test_path.join(MANIFEST_PATH);

        // Define the file structure to test
        let desired_structure = Structure::new(
            TEST_MAX_WIDTH, // width
            TEST_MAX_DEPTH, // depth
            TEST_INPUT_SIZE,
            Strategy::Simple,
        );

        // Setup the test structure
        setup_test_structure(
            &test_path,
            &input_path,
            &packed_path,
            &unpacked_path,
            &manifest_path,
            desired_structure,
            "test_duplicate_0",
        );

        // Copy $input_path/test_duplicate_0 to $input_path/test_duplicate_1
        let duplicate_1_path = input_path.join("test_duplicate_1");
        let duplicate_0_path = input_path.join("test_duplicate_0");
        ensure_path_exists_and_is_empty_dir(&duplicate_1_path, false).unwrap();
        fs_extra::dir::copy(
            &duplicate_0_path,
            &duplicate_1_path,
            &fs_extra::dir::CopyOptions::new(),
        )
        .unwrap();

        // Run the transform and check
        pipeline_test(
            input_path.clone(),
            packed_path.clone(),
            unpacked_path.clone(),
            manifest_path.clone(),
        )
        .await;
    }
}
