/// Configuration
pub mod config;
/// this has a custom fclones logger to make fclones look and act right
pub mod custom_fclones_logger;
/// This module contains code designed to traverse directory structure and get a bundling plan for files, including deduplication
pub mod grouper;
/// Utils specific to the Prepare pipeline
pub mod prepare;
/// This module contains code designed to traverse directory structure and get a bundling plan for directories and symlinks.
pub mod spider;
/// This module contains testing fns
#[cfg(test)]
pub mod test;
/// Native utils for working with files
pub mod wnfsio;
