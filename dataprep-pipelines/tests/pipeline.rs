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

#[cfg(test)]
mod test {
    use super::*;
    // use std::io::_print;

    #[tokio::test]
    /// A simple end to end integration test of a small file structure
    async fn test_pipeline() {
        // Define the file structure to test
        let desired_structure = FileStructure::new(
            2,                               // width
            2,                               // depth
            1024,                            // target size in bytes (1KB)
            FileStructureStrategy::Balanced, // Balanced
            true,                            // utf8 only
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
