use dataprep_pipelines::utils::{
    fs::{FileStructure, FileStructureStrategy},
    test::{pipeline_test, setup_test_structure},
};
use lazy_static::lazy_static;

lazy_static! {
    // Where to work on test outputs when running tests
    static ref TEST_PATH: &'static str = "test";
    static ref INPUT_PATH: &'static str = "test/input";
    static ref OUTPUT_PATH: &'static str = "test/packed";
    static ref UNPACKED_PATH:  &'static str = "test/unpacked";
    static ref MANIFEST_FILE_PATH: &'static str = "test/manifest.json";
}

/// Small Input End to End Integration Tests for the Pipeline
#[cfg(test)]
mod test {
    use super::*;

    const TEST_INPUT_SIZE: usize = 10; // 1MB
    const TEST_MAX_WIDTH: usize = 1;
    const TEST_MAX_DEPTH: usize = 1;

    /// Test the pipeline with a small file structure
    #[tokio::test]
    async fn test_pipeline() {
        // Define the file structure to test
        let desired_structure = FileStructure::new(
            TEST_MAX_WIDTH, // width
            TEST_MAX_DEPTH, // depth
            TEST_INPUT_SIZE,
            FileStructureStrategy::Balanced, // Balanced
        );
        println!("Setting up test structure: {:?}", desired_structure);
        // Setup the test structure
        setup_test_structure(
            &TEST_PATH,
            &INPUT_PATH,
            &OUTPUT_PATH,
            &UNPACKED_PATH,
            &MANIFEST_FILE_PATH,
            desired_structure,
        );

        println!("Running pipeline test");

        // Run the transform and check
        pipeline_test(
            &INPUT_PATH,
            &OUTPUT_PATH,
            &UNPACKED_PATH,
            &MANIFEST_FILE_PATH,
        )
        .await;
    }
    // TODO: (thea-exe) Add more tests - there might be a problem getting them to run in parallel
}
