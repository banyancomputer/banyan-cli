#[macro_use]
extern crate lazy_static;
extern crate rand;

use dir_assert::assert_paths;
use rand::Rng;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

lazy_static! {
    static ref MANIFEST_FILE: PathBuf = PathBuf::from("test/manifest.json");
    static ref TEST_DIR: PathBuf = PathBuf::from("test");
    static ref INPUT_DIR: PathBuf = PathBuf::from("test/input");
    static ref OUTPUT_DIR: PathBuf = PathBuf::from("test/output");
    static ref UNPACKED_DIR: PathBuf = PathBuf::from("test/unpacked");
}

async fn setup_structure() {
    // remove any old test crud
    fs::remove_dir_all(&*TEST_DIR).await.unwrap();
    fs::create_dir(&*TEST_DIR).await.unwrap();
    // create input directory
    fs::create_dir(&*INPUT_DIR).await.unwrap();
    // create output directory
    fs::create_dir(&*OUTPUT_DIR).await.unwrap();
    // create final output directory for unpacked
    fs::create_dir(&*UNPACKED_DIR).await.unwrap();
}

async fn transform_and_check() {
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
        OUTPUT_DIR.to_path_buf(),
        MANIFEST_FILE.to_path_buf(),
        UNPACKED_DIR.clone(),
    )
    .await
    .unwrap();
    // checks if two directories are the same
    assert_paths(&*INPUT_DIR, &*UNPACKED_DIR).unwrap();
}

// TODO: make a function that puts random data into a file with a given length
// so the argumetns are like (path, length) and it reutrns nothign

async fn create_directory_structure(path: &str, length: usize) {
    // creating a directory at the given path
    fs::create_dir(path).await.unwrap();
    // creates length number of directories and files within the directory at the given path
    for i in 0..length {
        // new directory path made
        let dir_path = format!("{}/{}", path, i);
        // create directory in the new directory path
        fs::create_dir(dir_path.clone()).await.unwrap();
        // new file path made
        let file_path = format!("{}/file{}", dir_path, i);
        // file is then created in the directory
        let mut file = fs::File::create(file_path).await.unwrap();
        // random data
        let random_data = rand::thread_rng().gen::<[u8; 32]>();
        file.write_all(&random_data);
    }
}
#[tokio::test]
async fn test_create_directory_structure() {
    // assert and check the created directory structure
    create_directory_structure("test_directory", 10);
}
