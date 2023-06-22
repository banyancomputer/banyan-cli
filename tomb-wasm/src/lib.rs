mod utils;
mod metadata;

use wasm_bindgen::prelude::*;
use web_sys::console;

extern crate web_sys;

// Our optional WeeAlloc allocator
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Our logging macro
macro_rules! log {
    ( $( $t:tt )* ) => {
        console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
pub struct Tomb {
    pub setup: bool,
}

// Public methods, exported to JavaScript.
#[wasm_bindgen]
impl Tomb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();
        log!("Tomb initialized");
        Tomb { setup: false }
    }

    #[wasm_bindgen]
    pub fn is_setup(&self) -> bool {
        self.setup
    }
}

// Private methods, not exported to JavaScript.
impl Tomb {
    #[allow(dead_code)]
    pub fn setup_method(&mut self) {
        log!("Tomb setup_method");
        self.setup = true;
    }
}




