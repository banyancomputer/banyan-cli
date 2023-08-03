use crate::value;
use crate::{blockstore::CarV2BlockStore, error::TombWasmError};
use js_sys::{Array, Object, Reflect};
use std::rc::Rc;
use tomb_common::{types::keys::manager::Manager, utils::serialize::*};
use tomb_crypt::prelude::EcEncryptionKey;
use wasm_bindgen::JsValue;
use wnfs::private::{PrivateDirectory, PrivateForest};
use wnfs::{common::Metadata, libipld::Ipld};

#[derive(Debug)]
#[allow(dead_code)]
/// Fs implementation for TombWasm
pub struct Fs {
    metadata: CarV2BlockStore,
    metadata_forest: Rc<PrivateForest>,
    content_forest: Rc<PrivateForest>,
    dir: Rc<PrivateDirectory>,
    manager: Manager,
}

impl Fs {
    /// Initialize a new Fs instance
    pub async fn new(
        wrapping_key: &EcEncryptionKey,
        metadata: CarV2BlockStore,
    ) -> Result<Self, TombWasmError> {
        let components = load_all(wrapping_key, &metadata)
            .await
            .map_err(TombWasmError::fs_error)?;
        let (metadata_forest, content_forest, dir, manager, _) = components;
        Ok(Self {
            metadata,
            metadata_forest,
            content_forest,
            dir,
            manager,
        })
    }

    /// List the contents of a directory at the given path
    pub async fn ls(&self, path_segments: Vec<&str>) -> Result<Array, TombWasmError> {
        let path_segments = path_segments
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let result = self
            .dir
            .ls(
                path_segments.as_slice(),
                true,
                &self.metadata_forest,
                &self.metadata,
            )
            .await
            .unwrap();
        // Convert the result into a JsValue
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
                    &FsEntry(metadata).try_into().unwrap(),
                )
                .unwrap();
                // Convert the object to JsValue
                value!(item)
            })
            .collect::<Array>();
        Ok(array)
    }
}

pub(crate) struct FsEntry<'a>(pub(crate) &'a Metadata);
impl TryFrom<FsEntry<'_>> for JsValue {
    type Error = js_sys::Error;
    fn try_from(fs_entry: FsEntry<'_>) -> Result<Self, Self::Error> {
        let metadata = Object::new();
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("created") {
            Reflect::set(
                &metadata,
                &value!("created"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("modified") {
            Reflect::set(
                &metadata,
                &value!("modified"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        Ok(value!(metadata))
    }
}

// #[cfg(test)]
// mod test {
//     use crate::blockstore::WasmBlockStore;
//     use crate::fs::crypto::PrivateKey;
//     use crate::fs::tombfs::TombFS;
//     use crate::value;
//     use js_sys::{Array, Reflect};
//     use wasm_bindgen_test::wasm_bindgen_test_configure;
//     use wasm_bindgen_test::*;
//     wasm_bindgen_test_configure!(run_in_browser);

//     const WRAPPINNG_KEY_URL: &str = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/wrapping_key.pem";
//     const METADATA_URL: &str = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/meta.car";

//     #[wasm_bindgen_test]
//     async fn load_tombfs() {
//         let wrapping_key = PrivateKey::new(WRAPPINNG_KEY_URL.to_string())
//             .await
//             .unwrap();
//         let metadata = WasmBlockStore::new(METADATA_URL.to_string()).await.unwrap();
//         let tomb_fs = TombFS::new(wrapping_key, metadata).await;
//         assert!(tomb_fs.is_ok());
//     }

//     #[wasm_bindgen_test]
//     #[ignore]
//     async fn ls() {
//         let wrapping_key = PrivateKey::new(WRAPPINNG_KEY_URL.to_string())
//             .await
//             .unwrap();
//         let metadata = WasmBlockStore::new(METADATA_URL.to_string()).await.unwrap();
//         let tomb_fs = TombFS::new(wrapping_key, metadata).await.unwrap();

//         let path_segments = Array::new_with_length(0);
//         let ls_array = tomb_fs.ls(path_segments).await;
//         assert!(ls_array.is_ok());
//         let ls_array = ls_array.unwrap();
//         let first_folder = ls_array.get(0);
//         let second_folder = ls_array.get(1);
//         let file_name1 = Reflect::get(&first_folder, &value!("file_name")).unwrap();
//         let file_name2 = Reflect::get(&second_folder, &value!("file_name")).unwrap();
//         assert_eq!(file_name1.as_string().unwrap(), "serious images");
//         assert_eq!(file_name2.as_string().unwrap(), "silly images");
//     }
// }
