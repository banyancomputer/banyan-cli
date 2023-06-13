use anyhow::Result;
use std::io::{Read, Seek};

use crate::types::blockstore::car::varint::read_varint_u128;

#[derive(Debug, PartialEq)]
pub(crate) struct V2Index {}

impl V2Index {
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Option<Self>> {
        // Grab the codec
        let codec = read_varint_u128(&mut r)?;
        // Read all remaining bytes
        let mut remaining_bytes: Vec<u8> = Vec::new();
        r.read_to_end(&mut remaining_bytes)?;

        match codec {
            // Format 0x0400: IndexSorted
            0x0400 => {
                println!("IndexSorted");
                Ok(Some(Self {

                }))
            },
            // Format 0x0401: MultihashIndexSorted
            0x0401 => {
                println!("MultihashIndexSorted");
                Ok(Some(Self {

                }))
            },
            _ => Ok(None)
        }
    }
}
