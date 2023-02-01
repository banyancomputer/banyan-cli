use dir_assert::assert_paths;
use std::path::PathBuf;
use tokio::fs;

#[tokio::test]
async fn it_works_for_one_file() {
    // remove any old test crud
    fs::remove_dir_all("test").await.unwrap();
    fs::create_dir("test").await.unwrap();
    // create input directory
    fs::create_dir("test/input").await.unwrap();
    let input_dir = PathBuf::from("test/input");
    // create output directory
    fs::create_dir("test/output").await.unwrap();
    let output_dir = PathBuf::from("test/output");
    // create final output directory for unpacked
    fs::create_dir("test/unpacked").await.unwrap();
    let unpacked_dir = PathBuf::from("test/unpacked");
    let manifest_file = PathBuf::from("test/manifest.json");
    // create a file in the input directory
    fs::write("test/input/test.txt", b"test").await.unwrap();

    // run the function
    println!("doing pack pipeline!");
    dataprep_pipelines::pipeline::pack_pipeline::pack_pipeline(
        input_dir.clone(),
        output_dir.clone(),
        manifest_file.clone(),
        1073741824, // 1GB
        true,
    )
    .await
    .unwrap();
    println!("doing unpack pipeline!");
    dataprep_pipelines::pipeline::unpack_pipeline::unpack_pipeline(
        output_dir,
        manifest_file,
        unpacked_dir.clone(),
    )
    .await
    .unwrap();
    // checks if two directories are the same
    assert_paths(input_dir, unpacked_dir).unwrap();
}
