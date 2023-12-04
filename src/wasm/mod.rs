//! This crate contains modules which are compiled to WASM
/// Compatibility
mod compat;
mod version;
/// Expose all the compatibility types directly
pub use compat::{
    to_js_error_with_msg, to_wasm_error_with_msg, TombResult, TombWasm, TombWasmError, WasmBucket,
    WasmBucketKey, WasmBucketMetadata, WasmFsMetadataEntry, WasmMount, WasmNodeMetadata,
    WasmSharedFile, WasmSnapshot,
};

use std::sync::Once;
use tracing::{debug, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{
    fmt::{format::Pretty, time::UtcTime},
    prelude::*,
};
use tracing_web::{performance_layer, MakeConsoleWriter};
use version::version;
use wasm_bindgen::prelude::wasm_bindgen;

/// Turn a value into a JsValue
#[macro_export]
macro_rules! value {
    ($value:expr) => {
        wasm_bindgen::JsValue::from($value)
    };
}

#[cfg(debug_assertions)]
pub(crate) fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

static INIT: Once = Once::new();

#[wasm_bindgen(start)]
pub fn register_log() {
    INIT.call_once(|| {
        #[cfg(debug_assertions)]
        set_panic_hook();

        let filter = if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::ERROR
        };

        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_timer(UtcTime::rfc_3339())
            .with_writer(MakeConsoleWriter)
            .with_filter(filter);

        let perf_layer = performance_layer()
            .with_details_from_fields(Pretty::default())
            .with_filter(filter);

        // Install these as subscribers to tracing events
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(perf_layer)
            .init();

        debug!("tomb-wasm: new() with version {}", version());
        warn!("tomb-wasm: warning in case debug does not go though.");
    });
}
