use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

// use tomb_common::types::blockstore::networkblockstore::NetworkBlockStore;
// use wasm_bindgen::{prelude::wasm_bindgen, JsError};
// use wnfs::{
//     common::BlockStore,
//     libipld::{Cid, IpldCodec},
// };

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_fetch_json() {
        #[derive(serde::Deserialize, Debug)]
        struct Todo {
            #[serde(rename = "userId")]
            pub user_id: u32,
            pub id: u32,
            pub title: String,
            pub completed: bool,
        }
        // Note: this is a public API that returns fake data for testing.
        let url = "https://jsonplaceholder.typicode.com/todos/1".to_string();
        let json = super::fetch_json(url).await.unwrap();
        let todo = serde_wasm_bindgen::from_value::<Todo>(json).unwrap();
        assert_eq!(todo.user_id, 1);
        assert_eq!(todo.id, 1);
        assert_eq!(todo.title, "delectus aut autem");
        assert_eq!(todo.completed, false);
    }
}


// // Module imports
// use crate::metadata::types::Manifest;

// // Provide a default manifest
// #[wasm_bindgen]
// pub async fn default_manifest() -> Result<Manifest, JsError> {
//     Ok(Manifest::default())
// }

// #[wasm_bindgen]
// pub async fn save_metadata(value: String) -> Result<String, JsError> {
//     let store: &NetworkBlockStore = &NetworkBlockStore::new("http://127.0.0.1", 5001);
//     let cid = store
//         .put_block(value.as_bytes().to_vec(), IpldCodec::Raw)
//         .await
//         .unwrap();
//     Ok(cid.to_string())
// }

// #[wasm_bindgen]
// pub async fn load_metadata(cid: String) -> Result<String, JsError> {
//     let store: &NetworkBlockStore = &NetworkBlockStore::new("http://127.0.0.1", 5001);
//     let bytes = store.get_block(&Cid::try_from(cid)?).await.unwrap();
//     let value: String = std::str::from_utf8(&bytes)?.to_string();
//     Ok(value)
// }

// #[cfg(test)]
// mod tests {
//     use wasm_bindgen_test::wasm_bindgen_test_configure;
//     use wasm_bindgen_test::*;

//     wasm_bindgen_test_configure!(run_in_browser);

//     #[wasm_bindgen_test]
//     fn simple() {
//         assert!(true);
//     }
// }
