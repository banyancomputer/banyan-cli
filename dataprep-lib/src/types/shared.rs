use serde::{Deserialize, Serialize};

/// This struct is used to describe how a file was processed. Either it was a duplicate/symlink/
/// directory and there isn't much to do, or else we need to go through compression, partition, and
/// encryption steps.
#[derive(Debug, Clone)]
pub enum DataProcessDirective<T> {
    /// It was a directory, just create it
    Directory,
    /// it was a symlink, just create it
    Symlink,
    /// it was a file, here's the metadata for how it was encrypted and compressed
    File(T),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodableDataProcessDirective<T> {
    /// It was a directory, just create it
    Directory,
    /// it was a symlink, just create it
    Symlink,
    /// it was a file, here's the metadata for how it was encrypted and compressed
    File(T),
}

impl<T> From<DataProcessDirective<T>> for CodableDataProcessDirective<T> {
    fn from(data_process_directive: DataProcessDirective<T>) -> Self {
        match data_process_directive {
            DataProcessDirective::Directory => CodableDataProcessDirective::Directory,
            DataProcessDirective::Symlink => CodableDataProcessDirective::Symlink,
            DataProcessDirective::File(data_process) => {
                CodableDataProcessDirective::File(data_process)
            }
        }
    }
}
