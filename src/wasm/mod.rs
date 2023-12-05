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
use time::macros::format_description;
use tracing::info;
use tracing_subscriber::{filter::LevelFilter, prelude::*};
use tracing_subscriber::{
    fmt::{format::Pretty, time::UtcTime},
    reload,
};
use tracing_web::{performance_layer, MakeWebConsoleWriter};
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

        let timer = UtcTime::new(format_description!("[hour]:[minute]:[second]"));
        let (fmt_filter, fmt_handle) = reload::Layer::new(LevelFilter::DEBUG);
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_timer(timer)
            .with_writer(MakeWebConsoleWriter::new())
            .with_filter(fmt_filter);

        let perf_filter = if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::WARN };
        let perf_layer = performance_layer()
            .with_details_from_fields(Pretty::default())
            .with_filter(perf_filter);

        // Install these as subscribers to tracing events
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(perf_layer)
            .init();

        // Print info no matter what
        info!("new() with version {}", version());

        if cfg!(debug_assertions) {
            info!("logging is working. because you built in debug mode you should see all output.");
        } else {
            info!("logging is working, but because you have built for release, only errors and warnings will appear from here on out.");
            let _ = fmt_handle.modify(|filter| *filter = LevelFilter::WARN);
        }
    });
}
