use anyhow::{anyhow, Error, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// Enum for describing how to generate a file structure
/// This is used for generating random data for testing
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileStructureStrategy {
    /// Generate a balanced file structure
    Balanced,
    /// Generate a random file structure
    Random,
}

impl ToString for FileStructureStrategy {
    /// Convert the FileStructureStrategy to a string that can be used as a filename
    fn to_string(&self) -> String {
        match self {
            FileStructureStrategy::Balanced => "balanced".to_string(),
            FileStructureStrategy::Random => "random".to_string(),
        }
    }
}

/// Everything is a file in Unix :) including directories
/// Struct for representing a file structure, regardless of depth (i.e. a file or a directory)
/// We use this for generating random data for testing
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileStructure {
    /// How many files should be in the file (if it has depth > 0)
    pub width: usize,
    /// How deep the directory structure should be. 0 means this is a file
    pub depth: usize,
    /// How much data should be in the file
    pub target_size: usize,
    /// What strategy to use for generating the file structure
    pub strategy: FileStructureStrategy,
    // TODO (amiller68): Deprecate when we figure out how to handle non-utf8 characters
    /// Whether or not files can include non-utf8 characters
    pub utf8_only: bool,
}

impl ToString for FileStructure {
    /// Convert the FileStructure to a string that can be used as a filename
    fn to_string(&self) -> String {
        format!(
            "w{}_d{}_s{}_{}",
            self.width,
            self.depth,
            self.target_size,
            self.strategy.to_string()
        )
    }
}

impl FileStructure {
    /// Create a new FileStructure
    /// # Arguments
    /// width: Desired width of the file structure, upper bound
    /// depth: Desired depth of the file structure, upper bound
    /// target_size: Desired size of the file structure, upper bound
    /// strategy: The strategy to use for generating the file structure
    /// utf8_only: Whether or not files can include non-utf8 characters
    pub fn new(
        width: usize,
        depth: usize,
        target_size: usize,
        strategy: FileStructureStrategy,
        utf8_only: bool,
    ) -> Self {
        Self {
            width,
            depth,
            target_size,
            strategy,
            utf8_only,
        }
    }

    /// Generate a FileStructure with the given path. Does not check if the path can hold
    /// the file structure. Use with caution!
    /// # Arguments
    /// path: The path to generate the file structure at
    /// # Panics
    /// Panics if it cant create a directory at the given path (i.e. the path parent doesn't exist)
    /// Panics if the path already exists
    /// Errors if the file structure cannot be generated
    pub fn generate(&self, path: PathBuf) -> Result<(), Error> {
        // Panic if the path already exists. We don't want to overwrite anything!
        assert!(!path.exists());
        // If this is 0, we're creating a file
        if self.depth == 0 {
            let file_path = path;
            // Create a file with the target size
            create_random_file(file_path, self.target_size, self.utf8_only);
            return Ok(()); // We're done here
        }
        let file_path = path.clone();
        // Create a directory at the given path
        fs::create_dir(file_path).unwrap();
        // Generate a new FileStructure with the new path
        match self.strategy {
            /*
               Generate a balanced file structure
               This means that each file will have the same amount of data, and only leaf
               directories will hold files
            */
            FileStructureStrategy::Balanced => {
                for i in 0..self.width {
                    // Read a fixed amount of data from target size
                    let target_size = self.target_size / self.width;
                    // Push the new path onto the path
                    let mut new_path = path.clone();
                    new_path.push(i.to_string());
                    // Generate a new FileStructure with the new path
                    FileStructure::new(
                        self.width,
                        self.depth - 1,
                        target_size,
                        self.strategy.clone(),
                        self.utf8_only,
                    )
                    .generate(new_path)
                    .unwrap();
                }
            }
            /*
               Generate a random file structure
               This means that each dir will have a random amount of data, bounded by the
               target size. Non-leaf directories can have files of leftover data, or fewer
               files than the width
            */
            FileStructureStrategy::Random => {
                panic!("Random file structure generation not implemented yet");
                // TODO (amiller68 & thea-exe): This is not safe to run, fix it!
                // Track how much data we've written
                // let mut data_written = 0;
                // let mut target_size = self.target_size;
                // for i in 0..self.width {
                //     // Read a random amount of data from (target size - data written)
                //     target_size = rand::random::<usize>() % (self.target_size - data_written);
                //     // Chop off the target size from the total target size
                //     data_written += target_size;
                //     // Push the new path onto the path
                //     let mut new_path = path.clone();
                //     new_path.push(i.to_string());
                //     // Recurse and generate a new FileStructure with the new path
                //     FileStructure::new(
                //         self.width,
                //         self.depth - 1,
                //         target_size,
                //         self.strategy.clone(),
                //         self.utf8_only,
                //     )
                //     .generate(new_path)?;
                // }
                // // If we haven't written all the data, keep chopping into files
                // // Keep an index for the file name
                // let mut i = 0;
                // while data_written < self.target_size {
                //     // Read a random amount of data from (target size - data written)
                //     target_size = rand::random::<usize>() % (self.target_size - data_written);
                //     // Chop off the target size from the total target size
                //     data_written += target_size;
                //     // Push the new path onto the path
                //     let mut new_path = path.clone();
                //     new_path.push(i.to_string() + "_file");
                //     // Recurse and generate a new FileStructure with the new path
                //     create_random_file(new_path, target_size, self.utf8_only);
                //     i += 1;
                // }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use fs_extra::dir::get_size;
    // TODO (thea-exe): Add more qualified tests for FileStructure
    #[test]
    fn test_balanced_file_structure() {
        use super::*;
        let mut test_scratch_space = PathBuf::from("test/test_balanced_file_structure");
        // Remove the scratch space and recreate it
        fs::remove_dir_all(&test_scratch_space).unwrap_or(());
        fs::create_dir_all(&test_scratch_space).unwrap();
        // Create a balanced file structure, 1 KB in size
        let file_structure = FileStructure::new(3, 2, 1024, FileStructureStrategy::Balanced, true);
        // Push another path onto the scratch space
        test_scratch_space.push(file_structure.to_string());
        // Generate the file structure
        file_structure
            .generate(test_scratch_space.clone())
            .map_err(|e| {
                println!("Error Generating FS: {}", e);
            })
            .unwrap();
        // Check that the file structure was created
        assert!(test_scratch_space.exists());
        // Check the the file structure is around the right size
        let file_structure_size = get_size(&test_scratch_space).unwrap();
        println!("File structure size: {}", file_structure_size);
        assert!(file_structure_size > 1000 && file_structure_size < 1024);
    }

    // TODO (thea-exe): debug this test plz
    // #[test]
    // fn test_random_file_structure() {
    //     use super::*;
    //     let mut test_scratch_space = PathBuf::from("test_scratch_space/test_random_file_structure");
    //     // Remove the scratch space and recreate it
    //     fs::remove_dir_all(&test_scratch_space).unwrap_or(());
    //     fs::create_dir_all(&test_scratch_space).unwrap();
    //     // Push another path onto the scratch space
    //     test_scratch_space.push("fs");
    //     // Create a balanced file structure, 1 KB in size
    //     let file_structure = FileStructure::new(3, 2, 1024, FileStructureStrategy::Random,true);
    //     file_structure.generate(test_scratch_space.clone());
    //     // Check that the file structure was created
    //     assert!(test_scratch_space.exists());
    //     // Check the the file structure is around the right size
    //     let file_structure_size = get_size(&test_scratch_space).unwrap();
    //     assert!(file_structure_size > 90 && file_structure_size < 110);
    // }
}

/* Miscellaneous filesystem utilities */

/// Create a random at the given path with the given size
/// # Arguments
/// * `path` - The path to create the file at
/// * `size` - The size of the file to create
/// * `utf8_only` - Whether or not to only write utf-8 characters to the file
/// # Panics
/// Panics if the file cannot be created
/// # Examples
/// ```no_run
/// use dataprep_pipelines::utils::fs::create_random_file;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test.txt");
/// create_random_file(path, 100, false);
/// ```
pub fn create_random_file(path: PathBuf, size: usize, utf8_only: bool) {
    let mut file = fs::File::create(path).unwrap();
    let mut rng = rand::thread_rng();
    let mut buf = [0u8; 1];
    for _ in 0..size {
        if utf8_only {
            buf[0] = rng.gen_range(0x20..0x7F);
        } else {
            buf[0] = rng.gen();
        }
        let n = file.write(&buf).unwrap();
        assert_eq!(n, 1);
    }
}

/// Check if a path is an existing directory
/// # Arguments
/// path: The path to check
/// # Returns
/// Result<(), anyhow::Error> - Ok if the path is an existing directory, Err otherwise
/// # Examples
/// ```no_run
/// use dataprep_pipelines::utils::fs::ensure_path_exists_and_is_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_dir(&path).unwrap();
/// ```
pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        // create path if it doesn't exist
        fs::create_dir_all(path)?;
    }
    if !path.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }
    Ok(())
}

/// Check if a path is an existing empty directory
/// # Arguments
/// path: The path to check
/// # Returns
/// Result<(), anyhow::Error> - Ok if the path is an existing empty directory, Err otherwise
/// # Examples
/// ```no_run
/// use dataprep_pipelines::utils::fs::ensure_path_exists_and_is_empty_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_empty_dir(&path, false).unwrap();
/// ```
pub fn ensure_path_exists_and_is_empty_dir(path: &Path, force: bool) -> Result<()> {
    // Check the path exists and is a directory
    ensure_path_exists_and_is_dir(path)?;
    // Check the path is empty
    if path.read_dir().unwrap().count() > 0 {
        // If force is true, make the path empty
        if force {
            fs::remove_dir_all(path)?;
            fs::create_dir_all(path)?;
        } else {
            return Err(anyhow!("Path is not empty: {}", path.display()));
        }
    }
    Ok(())
}
