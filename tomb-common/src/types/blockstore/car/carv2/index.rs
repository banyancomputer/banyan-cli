use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};

use crate::types::blockstore::car::varint::read_varint_u128;

#[derive(Debug, PartialEq, Serialize, Deserialize, Default, Clone)]
pub(crate) struct Index {
    codec: u128,
    bytes: Vec<u8>,
}

impl Index {
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Option<Self>> {
        // Grab the codec
        let codec = read_varint_u128(&mut r)?;

        // Match the codec
        match codec {
            // IndexSorted
            0x0400 => {
                println!("this file is indexsorted");
                Ok(None)
            },
            0x401 => {
                println!("this file is multihashindexsorted");




                Ok(None)
            }
            _ => {
                println!("this file is unknown in index format");
                Ok(None)
            }
        }
    }

    // pub fn write_bytes<W: Write + Seek>(&self, mut w: W) -> Result<()> {
    //     // Write codec
    //     w.write_all(&encode_varint_u128(self.codec))?;
    //     // Write bytes
    //     w.write_all(&self.bytes)?;
    //     Ok(())
    // }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serial_test::serial;
    use crate::utils::test::{get_read_write, carindex_setup};
    use super::Index;

    #[test]
    #[serial]
    fn read_multihashindexsorted() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = carindex_setup(2, "multihashindexsorted", "read_multihashindexsorted")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = Index::read_bytes(rw)?;

        Ok(())
    }

    #[test]
    #[serial]
    fn read_indexsorted() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = carindex_setup(2, "indexsorted", "read_indexsorted")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = Index::read_bytes(rw)?;

        Ok(())
    }
}