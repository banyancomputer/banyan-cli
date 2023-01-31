use std::path::PathBuf;
use tokio::fs;
use dir_assert::assert_paths;


const MANIFEST_FILE : PathBuf = PathBuf::from("test/manifest.json");
const TEST_DIR: PathBuf = PathBuf::from("test");
const INPUT_DIR : PathBuf = PathBuf::from("test/input");
const OUTPUT_DIR : PathBuf = PathBuf::from("test/output");
const UNPACKED_DIR: PathBuf = PathBuf::from("test/unpacked");

fn setup_structure() {
    // remove any old test crud
    fs::remove_dir_all(TEST_DIR).await.unwrap();
    fs::create_dir(TEST_DIR).await.unwrap();
    // create input directory
    fs::create_dir(INPUT_DIR).await.unwrap();
    // create output directory
    fs::create_dir(OUTPUT_DIR).await.unwrap();
    // create final output directory for unpacked
    fs::create_dir(UNPACKED_DIR).await.unwrap();
}

fn transform_and_check() {
    // run the function
    println!("doing pack pipeline!");
    dataprep_pipelines::pipeline::pack_pipeline::pack_pipeline(
        INPUT_DIR.clone(),
        OUTPUT_DIR.clone(),
        MANIFEST_FILE.clone(),
        1073741824, // 1GB
        true,
    )
        .await
        .unwrap();
    println!("doing unpack pipeline!");
    dataprep_pipelines::pipeline::unpack_pipeline::unpack_pipeline(
        OUTPUT_DIR,
        MANIFEST_FILE,
        UNPACKED_DIR.clone(),
    )
        .await
        .unwrap();
    // checks if two directories are the same
    assert_paths(input_dir, unpacked_dir).unwrap();
}

// TODO: make a function that puts random data into a file with a given length
// so the argumetns are like (path, length) and it reutrns nothign
#[tokio::test]
async fn it_works_for_one_file() {
    setup_structure();
    // create a file in the input directory
    fs::write("test/input/test.txt", b"test").await.unwrap();
    transform_and_check();
}

#[tokio::test]
async fn it_works_for_one_file_in_one_directory() {
    setup_structure();
    // create a directory in the input directory
    fs::create_dir("test/input/test_dir").await.unwrap();
    // create a file in the input directory
    fs::write("test/input/test_dir/test.txt", b"test").await.unwrap();
    transform_and_check();
}
