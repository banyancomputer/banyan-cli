use anyhow::Result;
use std::io::{Read, Seek};

use crate::types::blockstore::car::varint::*;

#[derive(Debug, PartialEq)]
pub(crate) struct V2Index {}

impl V2Index {
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Option<Self>> {
        // Grab the codec
        let _codec = read_varint_u128(&mut r)?;
        Ok(None)

        //TODO (organizedgrime) - implement index parsing if we want good lookup times
        /*
        match codec {
            // Format 0x0400: IndexSorted
            0x0400 => {
                println!("IndexSorted");
                Ok(Some(Self {

                }))
            },
            // Format: MultihashIndexSorted
            0x0401 => {
                // | multihash-code (uint64) | width (uint32) | count (uint64) | digest1 | digest1 offset (uint64) | digest2 | digest2 offset (uint64) ...
                println!("MultihashIndexSorted");
                // let len = r.stream_len()?

                // while r.stream_position()? < r.stream_len()? {
                //     let multihash_code = read_varint_u64(&mut r)?;
                //     let width = read_varint_u32(&mut r)?;
                //     let count = read_varint_u64(&mut r)?;
                //     println!("| multihash {} | width {} | count {} |", multihash_code, width, count);

                //     for
                // }

                Ok(Some(Self {

                }))
            },
            _ => Ok(None)
        }
         */
    }
}
