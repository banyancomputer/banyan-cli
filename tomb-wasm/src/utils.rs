use tomb_common::blockstore::{carv2_memory::CarV2MemoryBlockStore, carv2_staging::StreamingCarAnalyzer};

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

pub async fn validate_car(data: &Vec<u8>) {
    let mut car_stream = StreamingCarAnalyzer::new();
    for chunk in data.chunks(20) {
        car_stream.add_chunk(chunk.to_owned()).expect("add_chunk");
    }

    loop {
        match car_stream.next().await {
            Ok(Some(meta)) => {
                gloo::console::log!(format!("meta: {:?}", meta));
            },
            Ok(None) => {
                gloo::console::log!(format!("none!"));
                break;
            },
            Err(err) => {
                gloo::console::log!(format!("error!: {}", err));
                break;
            }
        }
    }
    
    let report = car_stream.report().expect("report");

    println!("report: {:?}", report);
}