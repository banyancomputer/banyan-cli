use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, Write};

use crate::types::blockstore::car::varint::*;

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct V2Index {
    codec: u128,
    bytes: Vec<u8>
}

impl V2Index {
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Option<Self>> {
        // Grab the codec
        // let codec = read_varint_u128(&mut r)?;
        // let mut bytes: Vec<u8> = Vec::new();
        // r.read_to_end(&mut bytes)?;
        // Ok(Some(Self { codec, bytes }))
        Ok(None)
    }

    pub fn write_bytes<W: Write + Seek>(&self, mut w: W) -> Result<()> {
        // Write codec
        w.write_all(&encode_varint_u128(self.codec))?;
        // Write bytes
        w.write_all(&self.bytes)?;
        Ok(())
    }    
}
