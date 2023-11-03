use clap::Args;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Unified way of specifying a Bucket
#[derive(Debug, Clone, Args)]
#[group(required = true, multiple = false)]
#[clap(
    after_help = "If no bucket is specified manually, tomb will try to use the current directory."
)]
pub struct DriveSpecifier {
    /// Bucket Id
    #[arg(short, long)]
    pub bucket_id: Option<Uuid>,
    /// Bucket name
    #[arg(short, long)]
    pub name: Option<String>,
    /// Bucket Root on disk
    #[arg(short, long)]
    pub origin: Option<PathBuf>,
}

impl DriveSpecifier {
    /// Create a new BucketSpecifier with an Id
    pub fn with_id(id: Uuid) -> Self {
        Self {
            bucket_id: Some(id),
            name: None,
            origin: None,
        }
    }

    /// Create a new BucketSpecifier with a Path
    pub fn with_origin(path: &Path) -> Self {
        Self {
            bucket_id: None,
            name: None,
            origin: Some(path.to_path_buf()),
        }
    }

    /// Create a new BucketSpecifier with a Path
    pub fn with_name(name: &str) -> Self {
        Self {
            bucket_id: None,
            name: Some(name.to_string()),
            origin: None,
        }
    }
}
