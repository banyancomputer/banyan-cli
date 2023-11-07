#[cfg(not(target_arch = "wasm32"))]
mod io;

#[cfg(not(target_arch = "wasm32"))]
pub use io::{get_read, get_write, get_read_write};