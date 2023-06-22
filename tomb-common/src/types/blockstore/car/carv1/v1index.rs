use super::v1block::V1Block;
use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, Seek, SeekFrom},
    str::FromStr,
};
use wnfs::{common::BlockStoreError, libipld::Cid};

#[derive(Debug, PartialEq, Default, Clone)]
pub struct V1Index(pub(crate) RefCell<HashMap<Cid, u64>>);

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

        Ok(Self(RefCell::new(offsets)))
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

    pub fn get_all_cids(&self) -> Vec<Cid> {
        self.0.borrow().clone().into_keys().collect()
    }
}

impl Serialize for V1Index {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Grab the map
        let map: HashMap<Cid, u64> = self.0.borrow().clone();
        // Rewrite the map using strings
        let new_map: HashMap<String, u64> =
            map.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        // Serialize the String based map
        new_map.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for V1Index {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the map
        let map: HashMap<String, u64> = <HashMap<String, u64>>::deserialize(deserializer)?;
        // Rewrite the map using CIDs
        let new_map: HashMap<Cid, u64> = map
            .into_iter()
            .map(|(k, v)| (Cid::from_str(&k).unwrap(), v))
            .collect();
        // Create new self
        Ok(Self(RefCell::new(new_map)))
    }
}

#[cfg(test)]
mod tests {
    use super::V1Index;
    use crate::types::blockstore::car::carv1::{v1block::V1Block, v1header::V1Header};
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::File,
        io::{BufReader, Cursor, Seek, SeekFrom},
        path::Path,
        str::FromStr,
    };
    use wnfs::libipld::Cid;

    #[test]
    fn read_write_bytes() -> Result<()> {
        // Construct a V1Header
        let header = V1Header::default();
        // Write the header into a buffer
        let mut header_bytes: Vec<u8> = Vec::new();
        header.write_bytes(&mut header_bytes)?;

        // Reconstruct the header from this buffer
        let header_cursor = Cursor::new(header_bytes);
        let new_header = V1Header::read_bytes(header_cursor)?;

        // Assert equality
        assert_eq!(header, new_header);
        Ok(())
    }

    #[test]
    #[serial]
    fn read_disk() -> Result<()> {
        let car_path = Path::new("car-fixtures").join("carv1-basic.car");
        // Open the CARv1
        let mut file = BufReader::new(File::open(car_path)?);
        // Read the header
        let _ = V1Header::read_bytes(&mut file)?;
        // Load index
        let index = V1Index::read_bytes(&mut file)?;
        // Find offset of a known block
        let block_offset = index.get_offset(&Cid::from_str(
            "bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm",
        )?)?;
        // Move to offset
        file.seek(SeekFrom::Start(block_offset))?;
        // Successfully read the block at this offset
        V1Block::read_bytes(&mut file)?;
        // Return Ok
        Ok(())
    }
}
