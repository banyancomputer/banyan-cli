use crate::types::spider::{SpiderMetadata, SpiderMetadataToDisk};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

/// This struct is used to describe how a file was processed. Either it was a duplicate/symlink/
/// directory and there isn't much to do, or else we need to go through compression, partition, and
/// encryption steps.
#[derive(Debug, Clone)]
pub enum DataProcessDirective<T> {
    /// The file was a duplicate, use the processed data from the original- here's where to find it
    /// once everything else is restored
    Duplicate(Rc<SpiderMetadata>),
    /// It was a directory, just create it
    Directory,
    /// it was a symlink, just create it
    Symlink,
    /// it was a file, here's the metadata for how it was encrypted and compressed
    File(T),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataProcessDirectiveToDisk<T> {
    /// The file was a duplicate, use the processed data from the original- here's where to find it
    /// once everything else is restored
    Duplicate(SpiderMetadataToDisk),
    /// It was a directory, just create it
    Directory,
    /// it was a symlink, just create it
    Symlink,
    /// it was a file, here's the metadata for how it was encrypted and compressed
    File(T),
}

impl<T> TryFrom<DataProcessDirective<T>> for DataProcessDirectiveToDisk<T> {
    type Error = anyhow::Error;
    fn try_from(data_process_directive: DataProcessDirective<T>) -> Result<Self, Self::Error> {
        Ok(match data_process_directive {
            DataProcessDirective::Duplicate(spider) => {
                DataProcessDirectiveToDisk::Duplicate(spider.as_ref().try_into()?)
            }
            DataProcessDirective::Directory => DataProcessDirectiveToDisk::Directory,
            DataProcessDirective::Symlink => DataProcessDirectiveToDisk::Symlink,
            DataProcessDirective::File(data_process) => {
                DataProcessDirectiveToDisk::File(data_process)
            }
        })
    }
}
