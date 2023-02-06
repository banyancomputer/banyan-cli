#[macro_use]
extern crate lazy_static;
extern crate rand;

use dir_assert::assert_paths;
use rand::Rng;
use std::path::PathBuf;
use std::{
    io::{Write},
    fs
};

lazy_static! {
    static ref MANIFEST_FILE: PathBuf = PathBuf::from("test/manifest.json");
    static ref TEST_DIR: PathBuf = PathBuf::from("test");
    static ref INPUT_DIR: PathBuf = PathBuf::from("test/input");
    static ref OUTPUT_DIR: PathBuf = PathBuf::from("test/output");
    static ref UNPACKED_DIR: PathBuf = PathBuf::from("test/unpacked");
}

fn setup_structure() {
    // remove any old test crud
    fs::remove_dir_all(&*TEST_DIR).unwrap();
    fs::create_dir(&*TEST_DIR).unwrap();
    // create input directory
    fs::create_dir(&*INPUT_DIR).unwrap();
    // create output directory
    fs::create_dir(&*OUTPUT_DIR).unwrap();
    // create final output directory for unpacked
    fs::create_dir(&*UNPACKED_DIR).unwrap();
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

// TODO (thea-exe): Add a quick test for this
/// Create a random file with a given size
pub fn create_random_file(path: PathBuf, size: usize) {
    let mut file = fs::File::create(path).unwrap();
    let mut rng = rand::thread_rng();
    for _ in 0..size {
        file.write(&[rng.gen()]).unwrap();
    }
}

/// Everything is a file in Unix :) including directories
/// Struct for representing a file structure, regardless of depth (i.e. a file or a directory)
pub struct FileStructure {
    /// How many files should be in the file (if it has depth > 0)
    pub width: usize,
    /// How deep the directory structure should be. 0 means this is a file
    pub depth: usize,
    /// How much data should be in the file
    pub target_size: usize
}

// TODO (amiller68) : maybe benchmark
// TODO (amiller68) : Can we use stream iterators here to improve performance?
impl FileStructure {
    /// Create a new FileStructure
    /// width: Desired width of the file structure
    /// depth: Desired depth of the file structure
    /// target_size: Desired size of the file structure
    pub fn new(width: usize, depth: usize, target_size: usize) -> Self {
        Self {
            width,
            depth,
            target_size,
        }
    }

    /// Generate a balanced FileStructure with the given path
    /// This means that the number of files in each directory is the same
    /// Only leaf directories contain files
    /// # Arguments
    /// * `path` - The path to generate the file structure at
    /// # Panics
    /// Panics if the path already exists
    pub fn generate_balanced(&self, path: PathBuf) {
        // Panic if the path already exists. We don't want to overwrite anything!
        assert!(!path.exists());

        // If this is 0, we're creating a file
        if self.depth == 0 {
            // Create a file with the target size
            create_random_file(path, self.target_size);
        }
        // Otherwise, we're creating a directory
        else {
            // Create a directory at the given path
            fs::create_dir(&path).unwrap();
            for i in 0..self.width {
                // Read a fixed amount of data from target size
                let target_size = self.target_size / self.width;
                // Push the new path onto the path
                let mut new_path = path.clone();
                new_path.push(i.to_string());
                // TODO: Is it ok to recurse here?
                // Generate a new FileStructure with the new path
                FileStructure::new(self.width, self.depth - 1, target_size).generate_balanced(new_path);
            }
        }
    }

    // TODO (thea-exe) : Can we introduce randomness here?
    /// Generate a random FileStructure with the given path
    /// This means that the number of files in each directory is not the same
    /// The Size of each file is not the same
    /// Any directory can contain files
    /// # Arguments
    /// * `path` - The path to generate the file structure at
    /// # Panics
    /// Panics if the path already exists
    pub fn generate_random(&self, path: PathBuf) {
        // Panic if the path already exists. We don't want to overwrite anything!
        assert!(!path.exists());
        todo!("Implement me!");
    }
}

#[test]
fn test_create_directory_structure() {
    // create a directory structure with a width of 2, depth of 2, and a target size of 100 bytes
    let file_structure = FileStructure::new(2, 2, 1024 * 1024);
    // create a directory structure at the given path
    file_structure.generate_balanced(PathBuf::from("test_set"));

    // TODO (thea-exe): Is this the right / expected size?
    // TODO (thea-exe): Does this break with certain descriptors (e.g. 0 width, 0 depth, etc.)
}
