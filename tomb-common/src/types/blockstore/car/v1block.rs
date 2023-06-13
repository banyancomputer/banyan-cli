use anyhow::Result;
use std::{io::{self, Cursor}, borrow::Borrow};
use wnfs::{libipld::{
    cid::Version,
    multihash::{Code, MultihashDigest},
    Cid, IpldCodec,
}, common::dagcbor::encode};

// | 19-byte varint | x-byte Cid | x-byte content |
#[derive(PartialEq, Debug)]
pub(crate) struct V1Block {
    pub varint: u128,
    pub cid: Cid,
    pub content: Vec<u8>,
}

impl V1Block {
    pub fn new(content: Vec<u8>) -> Result<Self> {
        // Compute the SHA256 hash of the bytes
        let hash = Code::Sha2_256.digest(&content);
        // Represent the hash as a V1 CID
        let cid = Cid::new(Version::V1, IpldCodec::Raw.into(), hash)?;
        let varint = (cid.to_bytes().len() + content.len()) as u128;
        // Create new
        Ok(Self {
            varint,
            cid,
            content,
        })
    }

    pub fn write_bytes<W: io::Write>(&self, mut w: W) -> Result<usize> {
        // Create a buffer to store the u128 varint
        let varint_buf: [u8; 16] = self.varint.to_le_bytes();
        // Let all bytes be the content after the varint
        let all_bytes = [varint_buf.to_vec(), self.cid.to_bytes(), self.content.clone()].concat();
        // Write all bytes
        w.write_all(&all_bytes)?;
        // Return size
        Ok(all_bytes.len())
    }

    pub fn read_bytes<R: io::Read>(mut r: R) -> Result<Self> {
        let mut varint_buf: [u8; 16] = [0; 16];
        r.read_exact(&mut varint_buf)?;
        let varint = u128::from_le_bytes(varint_buf);
        let cid = Cid::read_bytes(&mut r)?;
        // Determine how much data has yet to be read from this block
        let content_length = varint as usize - cid.to_bytes().len();
        let mut content: Vec<u8> = Vec::with_capacity(content_length);
        content.resize(content_length, 0);
        r.read_exact(&mut content)?;
        // assert_eq!(bytes, varintu);
        Ok(Self { varint, cid, content })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let written = self.write_bytes(&mut bytes).unwrap();
        debug_assert_eq!(written, bytes.len());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::V1Block;
    use anyhow::Result;

    #[test]
    fn read_write_bytes() -> Result<()> {
        let data_example = "Hello Kitty!".as_bytes().to_vec();
        let block = V1Block::new(data_example)?;
        let mut block_bytes: Vec<u8> = Vec::new();
        block.write_bytes(&mut block_bytes)?;
        let new_block = V1Block::read_bytes(block_bytes.as_slice())?;
        assert_eq!(block, new_block);
        Ok(())
    }

    #[test]
    fn to_from_bytes() -> Result<()> {
        let data_example = "Hello Kitty!".as_bytes().to_vec();
        let block = V1Block::new(data_example)?;
        let block_bytes: Vec<u8> = block.to_bytes();
        let new_block = V1Block::read_bytes(block_bytes.as_slice())?;
        assert_eq!(block, new_block);
        Ok(())
    }
}
