use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, ReadableStream as WebSysReadableStream};
use wasm_streams::ReadableStream;

// Our logging macro
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[allow(dead_code)]
/// Fetches JSON from the given URL
/// 
/// # Arguments
/// * `url` - A string slice that holds the URL to fetch
pub async fn fetch_json(url: String) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    request.headers().set("Accept", "application/json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    let json = JsFuture::from(resp.json()?).await?;

    Ok(json)
}

#[allow(dead_code)]
/// Fetch a Reable Stream from the given URL
/// 
/// # Arguments
/// * `url` - A string slice that holds the URL to fetch
pub async fn fetch_stream(url: String) -> Result<ReadableStream, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    request.headers().set("Accept", "application/octet-stream")?;

    let window = web_sys::window().unwrap_throw();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await
        .map_err(|_| "fetch failed")?;

    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap_throw();

    // Get the response's body as a JS ReadableStream
    let raw_body: WebSysReadableStream = resp.body().unwrap_throw();
    let stream: ReadableStream = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());
    Ok(stream)
}

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use js_sys::Uint8Array;
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(serde::Deserialize, Debug)]
    struct Todo {
        #[serde(rename = "userId")]
        pub user_id: u32,
        pub id: u32,
        pub title: String,
        pub completed: bool,
    }

    #[wasm_bindgen_test]
    async fn test_fetch_json() {
        
        // Note: this is a public API that returns fake data for testing.
        let url = "https://jsonplaceholder.typicode.com/todos/1".to_string();
        let json = super::fetch_json(url).await.unwrap();
        let todo = serde_wasm_bindgen::from_value::<Todo>(json).unwrap();
        assert_eq!(todo.user_id, 1);
        assert_eq!(todo.id, 1);
        assert_eq!(todo.title, "delectus aut autem");
        assert_eq!(todo.completed, false);
    }

    #[wasm_bindgen_test]
    async fn test_fetch_stream() {
        let url = "https://jsonplaceholder.typicode.com/todos/1".to_string();
        let mut stream = super::fetch_stream(url).await.unwrap();
        let mut reader = stream.get_reader();
        let mut chunks: Vec<u8> = vec![];
       
        while let Ok(Some(result)) = reader.read().await {
            let chunk = Uint8Array::from(result);
            chunks.extend(chunk.to_vec());
        }

        let json_string = String::from_utf8(chunks).unwrap();
        // Parse the JSON string into a Todo struct
        let todo = serde_json::from_str::<Todo>(&json_string).unwrap();
        assert_eq!(todo.user_id, 1);
        assert_eq!(todo.id, 1);
        assert_eq!(todo.title, "delectus aut autem");
        assert_eq!(todo.completed, false);
    }
}