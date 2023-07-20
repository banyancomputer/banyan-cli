use std::rc::Rc;

use crate::{
    metadata::{blockstore::WasmBlockStore, error::WasmError, JsMetadata},
    value,
};
use js_sys::{Array, Object, Reflect};
use tomb_common::{types::keys::manager::Manager, utils::serialize::*};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wnfs::private::{PrivateDirectory, PrivateForest};

#[wasm_bindgen]
#[allow(dead_code)]
struct TombFS {
    metadata: WasmBlockStore,
    metadata_forest: Rc<PrivateForest>,
    content_forest: Rc<PrivateForest>,
    dir: Rc<PrivateDirectory>,
    manager: Manager,
}

#[wasm_bindgen]
#[allow(dead_code)]
impl TombFS {
    #[wasm_bindgen]
    pub async fn new(
        wrapping_key: PrivateKey,
        metadata: WasmBlockStore,
    ) -> Result<TombFS, JsValue> {
        // If we can successfully deserialize from key and metadata
        if let Ok((metadata_forest, content_forest, dir, manager, _)) =
            load_all(&wrapping_key.0, &metadata).await
        {
            // Init
            Ok(TombFS {
                metadata,
                metadata_forest,
                content_forest,
                dir,
                manager,
            })
        } else {
            Err(WasmError::FS.into())
        }
    }
}

#[wasm_bindgen]
#[allow(dead_code)]
impl TombFS {
    #[wasm_bindgen]
    /// List the
    pub async fn ls(&self, path_segments: Array) -> Result<Array, JsValue> {
        // Convert the array into a Vec of Strings
        let path_segments = path_segments
            .entries()
            .into_iter()
            .map(|value| value.unwrap().as_string().unwrap())
            .collect::<Vec<String>>();
        // Call LS
        if let Ok(result) = self
            .dir
            .ls(
                path_segments.as_slice(),
                true,
                &self.metadata_forest,
                &self.metadata,
            )
            .await
        {
            let array = result
                .iter()
                .map(|(file_name, metadata)| {
                    // Create a new object for this tuple
                    let item = Object::new();
                    // Set the file_name field
                    Reflect::set(&item, &value!("file_name"), &value!(file_name)).unwrap();
                    // Set the metadata field
                    Reflect::set(
                        &item,
                        &value!("metadata"),
                        &JsMetadata(metadata).try_into().unwrap(),
                    )
                    .unwrap();
                    // Convert the object to JsValue
                    value!(item)
                })
                .collect::<Array>();

            // Return Ok
            Ok(array)
        } else {
            // Return error
            Err(WasmError::LS(path_segments.to_vec()).into())
        }
    }
}


#[cfg(test)]
mod test {
    use crate::metadata::blockstore::WasmBlockStore;
    use crate::metadata::tombfs::TombFS;
    use crate::value;
    use js_sys::{Array, Reflect};
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    const WRAPPINNG_KEY_URL: &str = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/wrapping_key.pem";
    const METADATA_URL: &str = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/meta.car";

    #[wasm_bindgen_test]
    async fn load_tombfs() {
        let wrapping_key = PrivateKey::new(WRAPPINNG_KEY_URL.to_string())
            .await
            .unwrap();
        let metadata = WasmBlockStore::new(METADATA_URL.to_string()).await.unwrap();
        let tomb_fs = TombFS::new(wrapping_key, metadata).await;
        assert!(tomb_fs.is_ok());
    }

    #[wasm_bindgen_test]
    #[ignore]
    async fn ls() {
        let wrapping_key = PrivateKey::new(WRAPPINNG_KEY_URL.to_string())
            .await
            .unwrap();
        let metadata = WasmBlockStore::new(METADATA_URL.to_string()).await.unwrap();
        let tomb_fs = TombFS::new(wrapping_key, metadata).await.unwrap();

        let path_segments = Array::new_with_length(0);
        let ls_array = tomb_fs.ls(path_segments).await;
        assert!(ls_array.is_ok());
        let ls_array = ls_array.unwrap();
        let first_folder = ls_array.get(0);
        let second_folder = ls_array.get(1);
        let file_name1 = Reflect::get(&first_folder, &value!("file_name")).unwrap();
        let file_name2 = Reflect::get(&second_folder, &value!("file_name")).unwrap();
        assert_eq!(file_name1.as_string().unwrap(), "serious images");
        assert_eq!(file_name2.as_string().unwrap(), "silly images");
    }
}
