pub mod indexbucket;
pub mod indexsorted;
pub mod multihashindexsorted;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    fmt::Debug,
    io::{Cursor, Read, Seek, Write},
};

use crate::types::{
    blockstore::car::{
        error::CARError,
        varint::{encode_varint_u128, read_varint_u128},
    },
    streamable::Streamable,
};
use indexsorted::Bucket as IndexSortedBucket;
use multihashindexsorted::Bucket as MultiHashIndexSortedBucket;

use self::indexbucket::IndexBucket;

#[derive(Debug)]
pub(crate) struct Index {
    codec: u128,
    buckets: Vec<Box<dyn IndexBucket>>,
}

impl Streamable for Index {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        // Grab the codec
        let codec = read_varint_u128(r).expect("Cant read varint from stream");
        // Empty bucket vec
        let mut buckets = <Vec<Box<dyn IndexBucket>>>::new();
        // Match the codec
        match codec {
            // IndexSorted (1024)
            0x0400 => {
                println!("this file is indexsorted");
                // While we can read buckets
                while let Ok(bucket) = IndexSortedBucket::read_bytes(r) {
                    // Push new bucket to list
                    buckets.push(Box::new(bucket));
                }

                Ok(Index { codec, buckets })
            }
            // MultiHashIndexSorted (1025)
            0x0401 => {
                println!("this file is multihashindexsorted");
                // While we can read buckets
                while let Ok(bucket) = MultiHashIndexSortedBucket::read_bytes(r) {
                    // Push new bucket to list
                    buckets.push(Box::new(bucket));
                }

                Ok(Index { codec, buckets })
            }
            _ => {
                println!("this file is unknown in index format: {}", codec);
                Err(CARError::Index.into())
            }
        }
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        // Write codec
        w.write_all(&encode_varint_u128(self.codec))?;
        // For eacchc bucket
        for bucket in &self.buckets {
            // Based on the Codec
            match self.codec {
                // IndexSorted (1024)
                0x0400 => {
                    // Downcast
                    let indexbucket: &IndexSortedBucket = (bucket as &dyn Any)
                        .downcast_ref()
                        .expect("Unable to downcast as IndexSortedBucket");
                    // Write out
                    indexbucket.write_bytes(w)?;
                }
                // MultiHashIndexSorted (1025)
                0x0401 => {
                    // Downcast
                    let hashbucket: &MultiHashIndexSortedBucket = (bucket as &dyn Any)
                        .downcast_ref()
                        .expect("Unable to downcast as MultiHashIndexSortedBucket");
                    // Write out
                    hashbucket.write_bytes(w)?;
                }
                _ => {
                    panic!("unknown codec in write_bytes")
                }
            }
        }
        Ok(())
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = Cursor::new(<Vec<u8>>::new());
        self.write_bytes(&mut bytes)
            .expect("unable to write bytes in Index");
        // Serialize
        bytes.into_inner().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Index {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = &mut Cursor::new(<Vec<u8>>::deserialize(deserializer)?);
        let index = Index::read_bytes(bytes).expect("unable to read bytes in Index");
        Ok(index)
    }
}

impl PartialEq for Index {
    fn eq(&self, other: &Self) -> bool {
        self.codec == other.codec
    }
}

impl Clone for Index {
    fn clone(&self) -> Self {
        let mut bytes = Cursor::new(<Vec<u8>>::new());
        self.write_bytes(&mut bytes).expect("");
        bytes.seek(std::io::SeekFrom::Start(0)).expect("");
        Self::read_bytes(&mut bytes).expect("")
    }
}

#[cfg(test)]
mod test {

    // TODO: Until valid fixtures can be made or obtained this is a waste of time
    /*
    #[test]
    #[serial]
    #[ignore]
    fn read_multihashindex() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = car_setup(2, "basic-index", "read_multihashindex")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = CAR::read_bytes(rw)?;

        Ok(())
    }

    #[test]
    #[serial]
    #[ignore]
    fn read_multihashcar() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = car_setup(2, "rw-bs", "read_multihashcar")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = CAR::read_bytes(rw)?;

        Ok(())
    }

    #[test]
    #[serial]
    #[ignore]
    fn read_sortedindexcar() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = car_setup(2, "rw-bs", "read_sortedindexcar")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = CAR::read_bytes(rw)?;

        Ok(())
    }
    */
}
