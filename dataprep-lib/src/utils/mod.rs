/// this has a custom fclones logger to make fclones look and act right
pub mod custom_fclones_logger;
/// This module contains filesystem helper functions
pub mod fs;
/// This module contains code designed to traverse directory structure and get a packing plan for files, including deduplication
pub mod grouper;
/// This module contains code designed to traverse directory structure and get a packing plan for directories and symlinks.
pub mod spider;
