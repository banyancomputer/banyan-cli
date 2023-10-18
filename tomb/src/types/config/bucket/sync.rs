use colored::Colorize;
use std::fmt::Display;

/// Sync State
#[derive(Debug, Clone)]
pub enum SyncState {
    /// There is no remote correlate
    Unpublished,
    /// There is no local correlate
    Unlocalized,
    /// Local bucket is N commits behind the remote
    Behind(usize),
    /// Local and remote are congruent
    Synced,
    /// Local bucket is N commits ahead of the remote
    Ahead(usize),
}

impl Display for SyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            SyncState::Unpublished => "Bucket metadata does not exist remotely".red(),
            SyncState::Unlocalized => "Bucket metadata not exist locally".red(),
            SyncState::Behind(n) => format!("Bucket is {} commits behind remote", n).red(),
            SyncState::Synced => "Bucket is in sync with remote".green(),
            SyncState::Ahead(n) => format!("Bucket is {} commits ahead of remote", n).red(),
        };

        f.write_fmt(format_args!("{}", description))
    }
}

impl SyncState {}
