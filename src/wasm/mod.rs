//! This crate contains modules which are compiled to WASM
/// Compatibility
mod compat;
mod version;
/// Expose all the compatibility types directly
pub use compat::{
    TombResult, TombWasm, TombWasmError, WasmBucket, WasmBucketKey, WasmBucketMetadata,
    WasmFsMetadataEntry, WasmMount, WasmNodeMetadata, WasmSnapshot,
};

use tracing::{debug, warn, Level};
use tracing_wasm::{ConsoleConfig, WASMLayerConfigBuilder};
use version::version;
use wasm_bindgen::prelude::wasm_bindgen;

/// Turn a value into a JsValue
#[macro_export]
macro_rules! value {
    ($value:expr) => {
        wasm_bindgen::JsValue::from($value)
    };
}

#[cfg(feature = "console_error_panic_hook")]
pub(crate) fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(start)]
fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    set_panic_hook();

    let wasm_log_config = if cfg!(debug_assertions) {
        WASMLayerConfigBuilder::default()
            .set_report_logs_in_timings(true)
            .set_max_level(Level::DEBUG)
            .set_console_config(ConsoleConfig::ReportWithoutConsoleColor)
            .build()
    } else {
        WASMLayerConfigBuilder::default()
            .set_report_logs_in_timings(false)
            .set_max_level(Level::WARN)
            .set_console_config(ConsoleConfig::ReportWithoutConsoleColor)
            .build()
    };

    tracing_wasm::set_as_global_default_with_config(wasm_log_config);
    debug!("tomb-wasm: new() with version {}", version());
    warn!("tomb-wasm: warning in case debug does not go though.");
}
