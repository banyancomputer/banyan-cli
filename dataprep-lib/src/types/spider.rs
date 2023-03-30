use anyhow::Result;
use jwalk::DirEntry;
use serde::{Deserialize, Serialize};
use std::{
    fs::Metadata,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Clone)]
/// Metadata associated with a file, directory, or symlink that was processed by the spider
pub struct SpiderMetadata {
    /// This is the path relative to the root of the backup
    pub original_location: PathBuf,
    /// canonicalized path
    pub canonicalized_path: PathBuf,
    /// this is the metadata of the original file
    pub original_metadata: Metadata,
}

// TODO (organizedgrime) - these fields are literally identical. why not just keep a reference to the original SpiderMetadata?
// there must be a way to make that look pretty.
/// Codable version of the SpiderMetadata struct which can be written to disk using `serde` when required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodableSpiderMetadata {
    /// This is the path relative to the root of the backup
    pub original_location: PathBuf,
    /// The metadata we scraped from the file when it was first processed
    pub original_metadata: CodableMetadata,
}

impl SpiderMetadata {
    /// Creates a new `SpiderMetadata` struct from a `DirEntry` and a root path.
    /// # Arguments
    /// * `path_root` - The root of the path being spidered
    /// * `entry` - The individual file / directory being processed
    pub fn new(path_root: &Path, entry: DirEntry<((), ())>) -> Self {
        // Determine the location of the entry by stripping the root path from it
        let original_location = entry.path().strip_prefix(path_root).unwrap().to_path_buf();
        // Represent this entry location canonically as an absolute path
        let canonicalized_path = entry.path().canonicalize().unwrap();
        // Grab the metadata of the entry
        let original_metadata = entry.metadata().unwrap();
        // Return the SpiderMetadata
        SpiderMetadata {
            original_location,
            canonicalized_path,
            original_metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Enum representing the types of File that the Spider can process.
pub enum FileType {
    /// Directories are files that show us where to find other files.
    Directory,
    /// Symlinks are a special kind of directory.
    Symlink,
    /// Files are just files.
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Codable Metadata struct which can be written to disk using `serde` when required,
/// containing more fields than are typically stored in Metadata.
pub struct CodableMetadata {
    file_type: FileType,
    /// The length of the file in bytes
    pub len: u64,
    permissions: (), // TODO uuuugh permissions
    modified: SystemTime,
    accessed: SystemTime,
    created: SystemTime,
    owner: (), //TODO: figure out how to get owner
               // TODO come up with more metadata to store
}

impl TryFrom<&SpiderMetadata> for CodableMetadata {
    type Error = anyhow::Error;
    fn try_from(value: &SpiderMetadata) -> Result<Self> {
        Ok(CodableMetadata {
            file_type: match value.original_metadata.file_type().is_dir() {
                true => FileType::Directory,
                false => match value.original_metadata.file_type().is_symlink() {
                    true => FileType::Symlink,
                    false => FileType::File,
                },
            },
            len: value.original_metadata.len(),
            permissions: (), // TODO: figure out how to get permissions
            modified: value.original_metadata.modified()?,
            accessed: value.original_metadata.accessed()?,
            created: value.original_metadata.created()?,
            owner: (),
        })
    }
}

// Define how to construct a codable version of the SpiderMetadata struct
impl TryFrom<&SpiderMetadata> for CodableSpiderMetadata {
    type Error = anyhow::Error;
    fn try_from(value: &SpiderMetadata) -> Result<Self> {
        // Most values can be simply cloned
        let original_location = value.original_location.clone();

        // Construct the metadata using the entirety of SpiderMetaData struct.
        // Note that right now, not all of the information contained here is necessary to do this,
        // but it may be in the future.
        let original_metadata = CodableMetadata::try_from(value)?;

        // Construct and return
        Ok(CodableSpiderMetadata {
            original_location,
            original_metadata,
        })
    }
}
