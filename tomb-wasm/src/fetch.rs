use crate::error::TombWasmError;
use gloo::{console::log, utils::window};
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_streams::ReadableStream;
use web_sys::{
    ReadableStream as WebSysReadableStream, Request, RequestInit, RequestMode, Response,
};

// TODO: Move to banyan-api-client
// TODO: Add JWT logic to this module

/// Fetches JSON from the given URL
/// # Arguments
/// * `url` - A string slice that holds the URL to fetch
pub(crate) async fn get_json(url: String) -> Result<JsValue, TombWasmError> {
    log!("tomb-wasm: fetch/get_json()");

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request =
        Request::new_with_str_and_init(&url, &opts).map_err(TombWasmError::fetch_error)?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(TombWasmError::fetch_error)?;

    let resp_value = JsFuture::from(window().fetch_with_request(&request))
        .await
        .map_err(TombWasmError::fetch_error)?;
    assert!(resp_value.is_instance_of::<Response>());

    let resp: Response = resp_value.dyn_into().map_err(TombWasmError::fetch_error)?;
    let json: Promise = resp.json().map_err(TombWasmError::fetch_error)?;
    let json = JsFuture::from(json)
        .await
        .map_err(TombWasmError::fetch_error)?;

    Ok(json)
}

/// Fetch a Reable Stream from the given URL
/// # Arguments
/// * `url` - A string slice that holds the URL to fetch
pub(crate) async fn get_stream(url: String) -> Result<ReadableStream, TombWasmError> {
    log!("tomb-wasm: fetch/get_stream()");

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request =
        Request::new_with_str_and_init(&url, &opts).map_err(TombWasmError::fetch_error)?;
    request
        .headers()
        .set("Accept", "application/octet-stream")
        .map_err(TombWasmError::fetch_error)?;

    let resp_value = JsFuture::from(window().fetch_with_request(&request))
        .await
        .map_err(TombWasmError::fetch_error)?;
    assert!(resp_value.is_instance_of::<Response>());

    let resp: Response = resp_value.dyn_into().map_err(TombWasmError::fetch_error)?;
    let raw_body: WebSysReadableStream = resp.body().unwrap_throw();
    let stream = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());
    Ok(stream)
}

#[cfg(test)]
mod test {
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;

    use crate::error::TombWasmError;
    use crate::utils::read_vec_from_readable_stream;

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
    async fn test_get_json() -> Result<(), TombWasmError> {
        let url = "https://jsonplaceholder.typicode.com/todos/1".to_string();
        let json = super::get_json(url).await?;
        let todo: Todo = serde_wasm_bindgen::from_value(json).unwrap();
        assert_eq!(todo.user_id, 1);
        assert_eq!(todo.id, 1);
        assert_eq!(todo.title, "delectus aut autem");
        assert!(!todo.completed);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_get_vec() -> Result<(), TombWasmError> {
        let url = "https://jsonplaceholder.typicode.com/todos/1".to_string();
        let mut stream = super::get_stream(url).await?;
        let data = read_vec_from_readable_stream(&mut stream).await.unwrap();
        let todo = serde_json::from_slice::<Todo>(&data).unwrap();
        assert_eq!(todo.user_id, 1);
        assert_eq!(todo.id, 1);
        assert_eq!(todo.title, "delectus aut autem");
        assert!(!todo.completed);
        Ok(())
    }
}
