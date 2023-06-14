use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, Seek, SeekFrom},
};
use wnfs::{common::BlockStoreError, libipld::Cid};

use crate::types::blockstore::car::v1block::V1Block;

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct V1Index(RefCell<HashMap<Cid, u64>>);

impl V1Index {
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        let mut offsets = HashMap::<Cid, u64>::new();
        // While we're able to peek varints and CIDs
        while let Ok(block_offset) = r.stream_position() && 
              let Ok((varint, cid)) = V1Block::start_read(&mut r) {
            // Log where we found this block
            offsets.insert(cid, block_offset);
            // Skip the rest of the block
            r.seek(SeekFrom::Current(
                varint as i64 - cid.to_bytes().len() as i64,
            ))?;
        }

        Ok(Self {
            0: RefCell::new(offsets),
        })
    }

    pub fn get_offset(&self, cid: &Cid) -> Result<u64> {
        if let Some(offset) = self.0.borrow().get(cid) {
            Ok(*offset)
        } else {
            Err(BlockStoreError::CIDNotFound(*cid).into())
        }
    }

    pub fn insert_offset(&self, cid: &Cid, offset: u64) {
        self.0.borrow_mut().insert(*cid, offset);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::{v1block::V1Block, v1header::V1Header};

    use super::V1Index;
    use anyhow::Result;
    use std::{
        fs::File,
        io::{BufReader, Cursor, Seek, SeekFrom},
        path::Path,
        str::FromStr,
        vec,
    };
    use wnfs::libipld::Cid;

    // #[test]
    // fn read_write_bytes() -> Result<()> {
    //     // Construct a V1Header
    //     let header = V1Header {
    //         version: 2,
    //         roots: None,
    //     };

    //     // Write the header into a buffer
    //     let mut header_bytes: Vec<u8> = Vec::new();
    //     header.write_bytes(&mut header_bytes)?;

    //     // Reconstruct the header from this buffer
    //     let header_cursor = Cursor::new(header_bytes);
    //     let new_header = V1Header::read_bytes(header_cursor)?;

    //     // Assert equality
    //     assert_eq!(header, new_header);
    //     Ok(())
    // }

    #[test]
    fn read_disk() -> Result<()> {
        let car_path = Path::new("car-fixtures").join("carv1-basic.car");
        // Open the CARv1
        let mut file = BufReader::new(File::open(car_path)?);
        // Read the header
        let _ = V1Header::read_bytes(&mut file)?;
        let index = V1Index::read_bytes(&mut file)?;
        println!("index: {:?}", index.0);
        let block_offset = index.get_offset(&Cid::from_str(
            "bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm",
        )?)?;
        file.seek(SeekFrom::Start(block_offset))?;
        let block = V1Block::read_bytes(&mut file)?;
        println!("block: {:?}", block);
        // Return Ok
        Ok(())
    }
}
