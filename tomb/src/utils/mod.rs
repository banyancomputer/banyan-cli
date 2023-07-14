pub mod car;
pub mod config;
/// this has a custom fclones logger to make fclones look and act right
pub mod custom_fclones_logger;
/// This module contains code designed to traverse directory structure and get a packing plan for files, including deduplication
pub mod grouper;
/// Utils specific to the Pack pipeline
pub mod pack;
/// This module contains code designed to traverse directory structure and get a packing plan for directories and symlinks.
pub mod spider;
/// This module contains testing fns
#[cfg(test)]
pub mod test;
pub mod unpack;
/// This module contains WNFS IO fns
pub mod wnfsio;
