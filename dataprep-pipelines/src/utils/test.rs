use crate::pipeline::{pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline};
use crate::utils::fs::FileStructure;
use dir_assert::assert_paths;
use std::fs;
use std::path::PathBuf;

/// Generate a directory structure to run tests on
/// # Arguments
/// * `test_path` - The path to the test directory
/// * `input_path` - The path to the input directory
/// * `output_path` - The path to the output directory
/// * `unpacked_path` - The path to the unpacked directory
/// * `manifest_file_path` - The path to the manifest file
/// * `desired_structure` - The file structure to generate
pub fn setup_test_structure(
    test_path: &str,
    input_path: &str,
    output_path: &str,
    unpacked_path: &str,
    manifest_file_path: &str,
    desired_structure: FileStructure,
) {
    // Remove all old test files and directories
    fs::remove_dir_all(PathBuf::from(test_path)).unwrap_or_default();
    // Create the test directory
    fs::create_dir(PathBuf::from(test_path)).unwrap();
    // create a test set directory structure with a width of 2, depth of 2, and a target size of 1024 bytes
    let mut input_path = PathBuf::from(input_path);
    fs::create_dir(input_path.clone()).unwrap();
    input_path.push(desired_structure.to_string());
    // create a directory structure at the given path
    desired_structure.generate(input_path.clone()).unwrap();
    // Create the output directory
    fs::create_dir(PathBuf::from(output_path)).unwrap();
    // Create the unpacked directory
    fs::create_dir(PathBuf::from(unpacked_path)).unwrap();
    // Remove the manifest file if it exists
    fs::remove_file(PathBuf::from(manifest_file_path)).unwrap_or_default();
}

/// Run the pipeline and check if the output is the same as the input
/// # Arguments
/// * `input_dir` - The path to the input directory
/// * `output_dir` - The path to the output directory
/// * `unpacked_dir` - The path to the unpacked directory
/// * `manifest_file` - The path to the manifest file
/// # Panics
/// If the output is not the same as the input
pub async fn pipeline_test(
    input_dir: &str,
    output_dir: &str,
    unpacked_dir: &str,
    manifest_file: &str,
) {
    // run the function
    pack_pipeline(
        PathBuf::from(input_dir),
        PathBuf::from(output_dir),
        PathBuf::from(manifest_file),
        1073741824, // 1GB
        false,
    )
    .await
    .unwrap();
    unpack_pipeline(
        PathBuf::from(output_dir),
        PathBuf::from(unpacked_dir),
        PathBuf::from(manifest_file),
    )
    .await
    .unwrap();
    // checks if two directories are the same
    assert_paths(input_dir, unpacked_dir).unwrap();
}
