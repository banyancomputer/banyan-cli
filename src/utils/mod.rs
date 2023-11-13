#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod testing;

#[cfg(not(target_arch = "wasm32"))]
mod io;

#[cfg(not(target_arch = "wasm32"))]
pub use io::{get_read, get_read_write, get_write};

pub mod varint;