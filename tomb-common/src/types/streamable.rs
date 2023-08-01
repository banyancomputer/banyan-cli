use anyhow::Result;
use std::io::{Read, Seek, Write};

/// Custom Stream-Based Serialization
pub trait Streamable: Sized {
    /// Read the bytes
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self>;
    /// Write the bytes
    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()>;
}

#[cfg(test)]
mod test {
    use super::Streamable;
    use crate::types::blockstore::car::{
        carv1::block::Block,
        carv1::header::Header as V1Header,
        carv2::{
            header::Header as V2Header,
            index::{
                indexsorted::Bucket as IndexSortedBucket,
                multihashindexsorted::Bucket as MultiHashIndexSortedBucket, Index as V2Index,
            },
        },
    };
    use std::{
        collections::HashMap,
        io::{Cursor, Seek, SeekFrom},
    };
    use wnfs::libipld::{Cid, IpldCodec};

    // Macro for generating a serialization test for any type which conforms to the trait
    macro_rules! streamable_tests {
        ($(
            $type:ty:
            $name:ident: $value:expr,
        )*) => {
        $(
            mod $name {
                use anyhow::Result;
                use super::*;
                #[test]
                fn to_from_bytes() -> Result<()> {
                    // Serialize
                    let mut bytes = Cursor::new(<Vec<u8>>::new());
                    $value.write_bytes(&mut bytes)?;
                    // Rewind
                    bytes.seek(SeekFrom::Start(0))?;
                    // Reconstruct
                    let new_value = <$type>::read_bytes(&mut bytes)?;
                    // Reserialize
                    let mut new_bytes = Cursor::new(<Vec<u8>>::new());
                    new_value.write_bytes(&mut new_bytes)?;
                    // Assert equality of byte arrays
                    assert_eq!(bytes.into_inner(), new_bytes.into_inner());
                    // Ok
                    Ok(())
                }
            }
        )*
        }
    }

    /// Generate example data for V2Index
    fn v2_multi_index_example() -> V2Index<MultiHashIndexSortedBucket> {
        <V2Index<MultiHashIndexSortedBucket>>::new()
    }

    /// Generate example data for V2Index
    fn v2_sorted_index_example() -> V2Index<IndexSortedBucket> {
        <V2Index<IndexSortedBucket>>::new()
    }

    /// Generate example data for IndexSortedBucket
    fn index_sorted_example() -> IndexSortedBucket {
        let mut map = HashMap::new();
        map.insert(Cid::default(), 42);

        IndexSortedBucket {
            width: Cid::default().to_bytes().len() as u32,
            map,
        }
    }

    /// Generate example data for MultiHashIndexSortedBucket
    fn multi_sorted_example() -> MultiHashIndexSortedBucket {
        MultiHashIndexSortedBucket {
            code: 1,
            bucket: index_sorted_example(),
        }
    }

    /// Generate example data for CARv1 Header
    fn v1_header_example() -> V1Header {
        let header = V1Header::default(1);
        {
            let mut roots = header.roots.borrow_mut();
            roots.push(Cid::default());
        }
        header
    }

    /// Generate example data for CARv2 Header
    fn v2_header_example() -> V2Header {
        V2Header {
            characteristics: 0,
            data_offset: 50,
            data_size: 50,
            index_offset: 0,
        }
    }

    /// Generate example data for CAR data Block
    fn block_example() -> Block {
        // Raw bytes
        let data_example = "Hello Kitty!".as_bytes().to_vec();
        // Create new Block with these content bytes
        Block::new(data_example, IpldCodec::Raw).expect("unable to create new Block")
    }

    // Run serialization test cases for all of them
    streamable_tests! {
        IndexSortedBucket:
        indexsorted: index_sorted_example(),

        MultiHashIndexSortedBucket:
        multihashindexsorted: multi_sorted_example(),

        V1Header:
        carv1header: v1_header_example(),

        Block:
        carblock: block_example(),

        V2Header:
        carv2header: v2_header_example(),

        V2Index<IndexSortedBucket>:
        carv2sortedindex: v2_sorted_index_example(),

        V2Index<MultiHashIndexSortedBucket>:
        carv2multiindex: v2_multi_index_example(),
    }
}
