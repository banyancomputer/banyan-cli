use std::path::PathBuf;
use tokio::fs;

use dataprep_pipelines;


#[test]
fn test_macro() {
    // macro to determine if two directories/files are equal
    assert_paths!(input_dir, unpacked_dir);
}

#[test]
fn test_fn() {
    // function to determine if two directories/files are equal
    assert_paths(input_dir, unpacked_dir).unwrap();
}

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
    let final_out = PathBuf::from("test/unpacked/test.txt");
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
        unpacked_dir,
    )
    .await
    .unwrap();
    println!("checking results!");
    let final_out_contents = fs::read(final_out).await.unwrap();
    println!(
        "final_out_contents: {:?}",
        String::from_utf8(final_out_contents.clone())
    );
    assert_eq!(final_out_contents, b"test");
}
