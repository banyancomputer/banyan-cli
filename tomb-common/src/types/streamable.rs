use anyhow::Result;
use std::io::{Read, Seek, Write};

/// Custom Stream-Based Serialization
pub trait Streamable {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> where Self: Sized;
    fn write_bytes<W: Write + Seek>(self: &Self, w: &mut W) -> Result<()> where Self: Sized;
}

#[cfg(test)]
pub mod test {
    use wnfs::libipld::Cid;
    use std::{io::{Cursor, SeekFrom, Seek}, collections::HashMap};
    use crate::types::blockstore::car::carv2::index::{indexsorted::bucket::Bucket as IndexSortedBucket, multihashindexsorted::bucket::Bucket as MultiHashIndexSortedBucket};
    use super::Streamable;

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
                fn test() -> Result<()> {
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

    // Generate example data for IndexSortedBucket
    fn index_sorted_example() -> IndexSortedBucket {
        let mut map = HashMap::new();
        map.insert(Cid::default(), 42);

        IndexSortedBucket { 
            width: 40, 
            count: 5, 
            map
        }
    }

    // Generate example data for MultiHashIndexSortedBucket
    fn multi_sorted_example() -> MultiHashIndexSortedBucket {
        MultiHashIndexSortedBucket {
            code: 1,
            bucket: index_sorted_example(),
        }
    }

    // Run serialization test cases for all of them
    streamable_tests! {
        IndexSortedBucket:
        indexsorted: index_sorted_example(),

        MultiHashIndexSortedBucket:
        multihashindexsorted: multi_sorted_example(),
    }
}