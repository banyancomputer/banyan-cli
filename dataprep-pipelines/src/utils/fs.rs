use anyhow::{anyhow, Error, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    cmp, fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

/// Enum for describing how to generate a file structure
/// This is used for generating random data for testing
#[derive(Serialize, Deserialize, Clone, Debug, strum::Display)]
pub enum FileStructureStrategy {
    /// Generate a balanced file structure
    Balanced,
    /// Generate a Random file structure
    Random,
}

// impl ToString for FileStructureStrategy {
//     /// Convert the FileStructureStrategy to a string that can be used as a filename
//     fn to_string(&self) -> String {
//         // Note (amiller68): We don't need to worry anything except balanced for right now
//         match self {
//             FileStructureStrategy::Balanced => "balanced".to_string(),
//             FileStructureStrategy::Random => "random".to_string(),
//         }
//     }
// }

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
    ) -> Self {
        Self {
            width,
            depth,
            target_size,
            strategy,
        }
    }

    /// Convert the FileStructure to a string that can be used as a filename
    /// # Example
    /// ```no_run
    /// use dataprep_pipelines::utils::fs::FileStructure;
    /// use dataprep_pipelines::utils::fs::FileStructureStrategy;
    /// let file_structure = FileStructure::new(
    ///    4,                               // width
    ///   4,                               // depth
    ///  1024 * 1024,                     // target size in bytes (1Mb)
    /// FileStructureStrategy::Balanced, // Balanced
    /// );
    /// assert_eq!(file_structure.to_path_string(), "w4_d4_s1048576_balanced");
    /// ```
    pub fn to_path_string(&self) -> String {
        let strategy_str: String = self.strategy.to_string();
        format!(
            "w{}_d{}_s{}_{}",
            self.width, self.depth, self.target_size, strategy_str
        )
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
            create_random_file(file_path, self.target_size);
            return Ok(()); // We're done here
        }
        let file_path = path.clone();
        // Create a directory at the given path
        fs::create_dir(file_path).unwrap();
        // Generate a new FileStructure with the new path
        match self.strategy {
            // Note (amiller68): We don't need to worry anything except balanced for right now
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
                    )
                    .generate(new_path)
                    .unwrap();
                }
            }
            FileStructureStrategy::Random => {
                // Note (amiller68): We don't need to worry anything except balanced for right now
                unimplemented!()
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use fs_extra::dir::get_size;

    const TEST_SCRATCH_SPACE: &str = "test";
    const TEST_SIZE: usize = 1024;
    const TEST_WIDTH: usize = 2;
    const TEST_DEPTH: usize = 2;

    /// Create a balanced file structure, 1 KB in size
    #[test]
    fn test_balanced_file_structure() {
        use super::*;
        let mut test_scratch_space = PathBuf::from(format!(
            "{}/{}",
            TEST_SCRATCH_SPACE, "balanced_file_structure"
        ));
        // Remove the scratch space and recreate it
        fs::remove_dir_all(&test_scratch_space).unwrap_or(());
        fs::create_dir_all(&test_scratch_space).unwrap();
        // Create a file structure
        let file_structure = FileStructure::new(
            TEST_WIDTH,
            TEST_DEPTH,
            TEST_SIZE,
            FileStructureStrategy::Balanced,
        );
        // Push another path onto the scratch space
        test_scratch_space.push(file_structure.to_path_string());
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
        assert_eq!(file_structure_size, TEST_SIZE as u64);
    }
}

/* Miscellaneous filesystem utilities */

/// Creates a random file at the given path with the given size
/// # Arguments
/// * `path` - The path to create the file at
/// * `size` - The size of the file to create in bytes
/// # Panics
/// Panics if the file cannot be created
/// # Examples
/// ```no_run
/// use dataprep_pipelines::utils::fs::create_random_file;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test.txt");
/// create_random_file(path, 1024);
/// ```
#[doc(hidden)]
pub fn create_random_file(path: PathBuf, size: usize) {
    let file = fs::File::create(path).unwrap();
    let mut rng = rand::thread_rng();
    let mut writer = BufWriter::new(file);

    let mut buffer = [0; 1024];
    let mut remaining_size = size;

    while remaining_size > 0 {
        let to_write = cmp::min(remaining_size, buffer.len());
        let buffer = &mut buffer[..to_write];
        rng.fill(buffer);
        writer.write_all(buffer).unwrap();

        remaining_size -= to_write;
    }
}

/// Ensures that the given path exists and is a directory
/// # Arguments
/// path: The path to check
/// # Returns
/// Creates the directory if it doesn't exist, and is a directory
/// Result<()>
/// # Panics
/// Panics if the path exists but is not a directory
/// # Examples
/// ```no_run
/// use dataprep_pipelines::utils::fs::ensure_path_exists_and_is_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_dir(&path).unwrap();
/// ```
#[doc(hidden)]
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

/// Ensures that the given path exists and is a directory and is empty
/// # Arguments
/// path: The path to check
/// # Returns
/// Creates the directory if it doesn't exist. Makes the directory empty if it is not empty.
/// Result<()>
/// # Panics
/// Panics if the path is not an existing directory.
/// Panics if the path is not empty and force is false.
/// # Examples
/// ```no_run
/// use dataprep_pipelines::utils::fs::ensure_path_exists_and_is_empty_dir;
/// use std::path::PathBuf;
/// let path = PathBuf::from("test");
/// ensure_path_exists_and_is_empty_dir(&path, false).unwrap();
/// ```
#[doc(hidden)]
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
