use gloo::{console::log, utils::window};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_streams::ReadableStream;
use web_sys::{
    ReadableStream as WebSysReadableStream, Request, RequestInit, RequestMode, Response,
};

use crate::utils::JsResult;

#[allow(dead_code)]
/// Fetches JSON from the given URL
///
/// # Arguments
/// * `url` - A string slice that holds the URL to fetch
pub(crate) async fn get_json(url: String) -> JsResult<JsValue> {
    log!("tomb-wasm: fetch_json()");
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Accept", "application/json")?;
    let resp_value = JsFuture::from(window().fetch_with_request(&request)).await?;
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
pub(crate) async fn get_stream(url: String) -> JsResult<ReadableStream> {
    log!("tomb-wasm: fetch_stream()");
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(&url, &opts)?;
    request
        .headers()
        .set("Accept", "application/octet-stream")?;
    let resp_value = JsFuture::from(window().fetch_with_request(&request)).await?;
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap_throw();
    let raw_body: WebSysReadableStream = resp.body().unwrap_throw();
    let stream = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());
    Ok(stream)
}

// pub(crate) async fn get_stream_2(url: String) -> JsResult<ReadableStream> {
//     // Make a fetch request
//     // let url = "https://rustwasm.github.io/assets/wasm-ferris.png";
//     let resp_value = JsFuture::from(window().fetch_with_str(&url))
//         .await
//         .map_err(|_| "fetch failed").unwrap();
//     let resp: Response = resp_value.dyn_into().unwrap_throw();

//     // Get the response's body as a JS ReadableStream
//     let raw_body = resp.body().unwrap_throw();
//     let body = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());

//     // Convert the JS ReadableStream to a Rust stream
//     // let mut stream = body.into_stream();

//     Ok(body)
// }

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
        let url = "https://jsonplaceholder.typicode.com/todos/1".to_string();
        let json = super::get_json(url).await.unwrap();
        let todo: Todo = serde_wasm_bindgen::from_value(json).unwrap();
        assert_eq!(todo.user_id, 1);
        assert_eq!(todo.id, 1);
        assert_eq!(todo.title, "delectus aut autem");
        assert_eq!(todo.completed, false);
    }

    #[wasm_bindgen_test]
    async fn test_fetch_stream() {
        let url = "https://jsonplaceholder.typicode.com/todos/1".to_string();
        let mut stream = super::get_stream(url).await.unwrap();
        let mut reader = stream.get_reader();
        let mut chunks: Vec<u8> = vec![];
        while let Ok(Some(result)) = reader.read().await {
            let chunk = Uint8Array::from(result);
            chunks.extend(chunk.to_vec());
        }
        let todo = serde_json::from_slice::<Todo>(&chunks).unwrap();
        assert_eq!(todo.user_id, 1);
        assert_eq!(todo.id, 1);
        assert_eq!(todo.title, "delectus aut autem");
        assert_eq!(todo.completed, false);
    }

    #[wasm_bindgen_test]
    async fn test_fetch_stream_no_json() {
        let url = "https://raw.githubusercontent.com/ipld/go-car/master/v2/testdata/sample-v2-indexless.car".to_string();
        // let url = "https://rustwasm.github.io/assets/wasm-ferris.png".to_string();
        let mut stream = super::get_stream(url).await.unwrap();
    }
}
