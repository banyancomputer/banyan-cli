use dir_assert::assert_paths;
use std::path::PathBuf;
use tokio::fs;

const MANIFEST_FILE: PathBuf = PathBuf::from("test/manifest.json");
const TEST_DIR: PathBuf = PathBuf::from("test");
const INPUT_DIR: PathBuf = PathBuf::from("test/input");
const OUTPUT_DIR: PathBuf = PathBuf::from("test/output");
const UNPACKED_DIR: PathBuf = PathBuf::from("test/unpacked");

async fn setup_structure() {
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
        OUTPUT_DIR,
        MANIFEST_FILE,
        UNPACKED_DIR.clone(),
    )
    .await
    .unwrap();
    // checks if two directories are the same
    assert_paths(INPUT_DIR, UNPACKED_DIR).unwrap();
}

// TODO: make a function that puts random data into a file with a given length
// so the argumetns are like (path, length) and it reutrns nothign

// function is called to create the desired directory structure
#[tokio::test]
async fn test_directory_structure() {
    async fn create_directory_structure(depth: i32, subdirectories_per_dir: i32, parent_dir: &str) {
        let mut current_depth = 0;
        let mut current_dir = parent_dir.to_string();

        while current_depth < depth {
            for i in 0..subdirectories_per_dir {
                let new_dir = format!("{}/{}_{}", current_dir, current_depth, i);
                task::spawn(async move {
                    fs::create_dir(new_dir).await.unwrap();
                })
                .await;

                current_dir = new_dir;
                current_depth += 1;
            }
        }
    }
    // create_directory_structure is called to create the desired directory structure
    create_directory_structure(2, 2, "test/input").await;
    // fs::write is used to create a file and its duplicate in the input directory
    fs::write("test/input/0_0/test.txt", b"test").await.unwrap();
    // duplicate
    fs::write("test/input/0_0/test2.txt", b"test")
        .await
        .unwrap();
    // transform_and_check is then called to transform and check the files
    transform_and_check().await;

    // fs::metadata is used to retrieve the metadata for the two files,
    let metadata = fs::metadata("test/input/0_0/test.txt").await.unwrap();
    // assert! macro is used to ensure that both files were created successfully
    assert!(metadata.is_file());
    let metadata = fs::metadata("test/input/0_0/test2.txt").await.unwrap();
    assert!(metadata.is_file());
}

// #[tokio::test]
// async fn it_works_for_one_file() {
//     setup_structure();
//     // fs::write function is used to create a file and its duplicate in the input directory
//     fs::write("test/input/test.txt", b"test").await.unwrap();
//     transform_and_check();
// }

// #[tokio::test]
// async fn it_works_for_one_file_in_one_directory() {
//     setup_structure();
//     // create a directory in the input directory
//     fs::create_dir("test/input/test_dir").await.unwrap();
//     // create a file in the input directory
//     fs::write("test/input/test_dir/test.txt", b"test").await.unwrap();
//     transform_and_check();
// }

// #[tokio::test]
// async fn it_works_for_one_duplicated_file_in_one_directory() {
//     setup_structure();
//     // create a directory in the input directory
//     fs::create_dir("test/input/test_dir").await.unwrap();
//     // create a file in the input directory
//     fs::write("test/input/test_dir/test.txt", b"test").await.unwrap();
//     // create a duplicate file
//     fs::write("test/input/test_dir/test2.txt", b"test").await.unwrap();
//     transform_and_check();
// }
