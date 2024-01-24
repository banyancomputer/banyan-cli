#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod testing;

#[cfg(not(target_arch = "wasm32"))]
mod io;

#[cfg(not(target_arch = "wasm32"))]
pub use io::{get_read, get_read_write, get_write};

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub use io::compute_directory_size;

mod cast;
pub mod varint;

mod error;
pub(crate) use error::UtilityError;
