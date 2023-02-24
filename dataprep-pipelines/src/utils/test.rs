use crate::pipeline::{pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline};
use crate::utils::fs::ensure_path_exists_and_is_empty_dir;
use dir_assert::assert_paths;
use fake_file::Structure;
use std::fs;
use std::path::PathBuf;

/// Generate a directory structure to run tests on
/// # Arguments
/// * `test_path` - The path to the test directory
/// * `input_path` - The path to the input directory
/// * `packed_path` - The path to the output directory
/// * `unpacked_path` - The path to the unpacked directory
/// * `manifest_path` - The path to the manifest directory
/// * `desired_structure` - The file structure to generate
#[doc(hidden)]
pub fn setup_test_structure(
    test_path: &PathBuf,
    input_path: &PathBuf,
    packed_path: &PathBuf,
    unpacked_path: &PathBuf,
    manifest_path: &PathBuf,
    desired_structure: Structure,
    structure_name: &str,
) {
    fs::create_dir_all(test_path).unwrap();
    ensure_path_exists_and_is_empty_dir(input_path, true).unwrap();
    // Push the structure name to the input path
    let mut input_path = input_path.clone();
    input_path.push(structure_name);
    desired_structure.generate(&input_path.clone()).unwrap();
    ensure_path_exists_and_is_empty_dir(packed_path, true).unwrap();
    ensure_path_exists_and_is_empty_dir(unpacked_path, true).unwrap();
    fs::remove_file(manifest_path).unwrap_or_default();
}

/// Run the pipeline and check if the output is the same as the input
/// # Panics
/// If the output is not the same as the input
#[doc(hidden)]
pub async fn pipeline_test(
    input_path: PathBuf,
    packed_path: PathBuf,
    unpacked_path: PathBuf,
    manifest_path: PathBuf,
) {
    // let manifest_file = format!("{}/manifest.json", manifest_dir);
    // run the function
    pack_pipeline(
        input_path.clone(),
        packed_path.clone(),
        manifest_path.clone(),
        1073741824, // 1GB
        false,
    )
    .await
    .unwrap();
    unpack_pipeline(
        packed_path.clone(),
        unpacked_path.clone(),
        manifest_path.clone(),
    )
    .await
    .unwrap();

    // checks if two directories are the same
    assert_paths(&input_path.clone(), &unpacked_path.clone()).unwrap();
}
