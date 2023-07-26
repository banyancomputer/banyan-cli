use crate::types::{blockstore::car::carv2::index::indexbucket::IndexBucket, streamable::Streamable};

use super::block::Block;
use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
    str::FromStr,
};
use wnfs::libipld::Cid;

#[derive(Debug, PartialEq, Default, Clone)]
/// Custom index type that is used to track blocks
pub struct Index {
    /// A HashMap associating Cids with their locations in stream
    pub map: HashMap<Cid, u64>,
    pub(crate) next_block: u64,
}

impl Streamable for Index {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        let mut map = HashMap::<Cid, u64>::new();
        let mut next_block: u64 = r.stream_position()?;
        // While we're able to peek varints and CIDs
        while let Ok(block_offset) = r.stream_position() &&
              let Ok((varint, cid)) = Block::start_read(&mut *r) {
            // Log where we found this block
            map.insert(cid, block_offset);
            // Skip the rest of the block
            r.seek(SeekFrom::Current(varint as i64 - cid.to_bytes().len() as i64))?;
            next_block = r.stream_position()?;
        }

        Ok(Self { map, next_block })
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        Ok(())
    }
}

impl IndexBucket for Index {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        if let Some(offset) = self.map.get(cid) {
            Some(*offset)
        } else {
            None
        }
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        self.map.insert(*cid, offset)
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Rewrite the map using strings
        let new_map: HashMap<String, u64> =
            self.map.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        // Serialize the String based map
        (new_map, self.next_block).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Index {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the map
        let (str_map, next_block): (HashMap<String, u64>, u64) =
            <(HashMap<String, u64>, u64)>::deserialize(deserializer)?;
        // Rewrite the map using CIDs
        let map: HashMap<Cid, u64> = str_map
            .into_iter()
            .map(|(k, v)| (Cid::from_str(&k).unwrap(), v))
            .collect();
        // Create new self
        Ok(Self { map, next_block })
    }
}

#[cfg(test)]
mod test {
    use super::Index;
    use crate::{
        types::{
            blockstore::car::{carv1::{block::Block, header::Header}, carv2::index::indexbucket::IndexBucket},
            streamable::Streamable,
        },
        utils::test::car_setup,
    };
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::File,
        io::{Seek, SeekFrom},
        str::FromStr,
    };
    use wnfs::libipld::Cid;

    #[test]
    #[serial]
    fn read_disk() -> Result<()> {
        let car_path = &car_setup(1, "basic", "index_read_disk")?;
        // Open the CARv1
        let mut file = File::open(car_path)?;
        // Read the header
        let _ = Header::read_bytes(&mut file)?;
        // Load index
        let index = Index::read_bytes(&mut file)?;
        // Find offset of a known block
        let block_offset = index.get_offset(&Cid::from_str(
            "bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm",
        )?);
        // Move to offset
        file.seek(SeekFrom::Start(block_offset.unwrap()))?;
        // Successfully read the block at this offset
        Block::read_bytes(&mut file)?;
        // Return Ok
        Ok(())
    }
}
