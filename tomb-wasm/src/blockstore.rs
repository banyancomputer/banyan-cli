use async_trait::async_trait;
use std::{borrow::Cow, io::Cursor};
use tomb_common::blockstore::{car::v2::CarV2, TombBlockStore};
use wnfs::{
    common::blockstore::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};

use crate::error::TombWasmError;

#[derive(Debug)]
/// CarV2 formatted memory blockstore
pub struct CarV2BlockStore {
    data: Vec<u8>,
    car: CarV2,
}

impl CarV2BlockStore {
    /// Create a new CarV2BlockStore from a readable stream
    pub fn new(vec: Vec<u8>) -> Result<CarV2BlockStore, TombWasmError> {
        // Read data
        let data = vec;
        // Load car
        let car = CarV2::read_bytes(Cursor::new(&data))
            .map_err(|e| TombWasmError::car_error(format!("error reading car: {}", e)))?;
        // Ok
        Ok(Self { data, car })
    }
}

#[async_trait(?Send)]
/// WnfsBlockStore implementation for CarV2BlockStore
impl WnfsBlockStore for CarV2BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, anyhow::Error> {
        let mut reader = Cursor::new(&self.data);
        let block = self.car.get_block(cid, &mut reader)?;
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, _: Vec<u8>, _: IpldCodec) -> Result<Cid, anyhow::Error> {
        panic!("not implemented")
    }
}

#[async_trait(?Send)]
/// TombBlockStore implementation for CarV2BlockStore -- needed in order to interact with the Fs
impl TombBlockStore for CarV2BlockStore {
    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }

    fn set_root(&self, _: &Cid) {
        panic!("not implemented")
    }

    async fn update_content(
        &self,
        _: &Cid,
        _: Vec<u8>,
        _: IpldCodec,
    ) -> Result<Cid, anyhow::Error> {
        panic!("not implemented")
    }
}

// #[cfg(test)]
// mod test {
//     use crate::fetch::get_stream;
//     use wasm_bindgen_test::wasm_bindgen_test_configure;
//     use wasm_bindgen_test::*;
//     wasm_bindgen_test_configure!(run_in_browser);

//     #[wasm_bindgen_test]
//     async fn test_load_car() {
//         let url = "https://raw.githubusercontent.com/ipld/go-car/master/v2/testdata/sample-v2-indexless.car".to_string();
//         let mut car_stream = get_stream(url).await.unwrap();
//         let vec = crate::utils::read_vec_from_readable_stream(&mut car_stream)
//             .await
//             .unwrap();
//         assert!(super::CarV2BlockStore::from_vec(vec).await.is_ok());
//     }
// }
