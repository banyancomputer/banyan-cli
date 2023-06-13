use anyhow::Result;
use std::io::{Read, Seek, Write, SeekFrom};
use wnfs::libipld::{
        cid::Version,
        multihash::{Code, MultihashDigest},
        Cid, IpldCodec,
    };

use super::varint::encode_varint_u128;

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

    /// Serialize the current object 
    pub fn write_bytes<W: Write>(&self, mut w: W) -> Result<usize> {
        // Create a buffer to store the u128 varint
        let varint_buf: Vec<u8> = encode_varint_u128(self.varint);
        let cid_buf: Vec<u8> = self.cid.to_bytes();
        // Write all bytes
        w.write_all(&varint_buf)?;
        w.write_all(&cid_buf)?;
        w.write_all(&self.content)?;
        // Return size
        Ok(varint_buf.len() + cid_buf.len() + self.content.len())
    }

    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Create and fill a buffer for the varint
        let mut varint_buf = unsigned_varint::encode::u128_buffer();
        r.read_exact(&mut varint_buf)?;
        // Extract the varint
        let (varint, remaining) = unsigned_varint::decode::u128(&varint_buf)?;
        // Rewind the reader so only the varint has been processed
        r.seek(SeekFrom::Current(-(remaining.len() as i64)))?;
        // Read the CID
        let cid = Cid::read_bytes(&mut r)?;
        // Determine how much data has yet to be read from this block
        let content_length = varint as usize - cid.to_bytes().len();
        // Create a content vector with the specified capacity
        let mut content: Vec<u8> = vec![0; content_length];
        // Read exactly that much content
        r.read_exact(&mut content)?;
        // Create a new Self
        Ok(Self {
            varint,
            cid,
            content,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::V1Block;
    use anyhow::Result;

    #[test]
    fn read_write_bytes() -> Result<()> {
        // Raw bytes
        let data_example = "Hello Kitty!".as_bytes().to_vec();
        // Create new V1Block with these content bytes
        let block = V1Block::new(data_example)?;
        // Create a buffer and fill with serialized verison
        let mut block_bytes: Vec<u8> = Vec::new();
        block.write_bytes(&mut block_bytes)?;
        // Reader with Seek
        let block_cursor = Cursor::new(block_bytes);
        // Reconstruct
        let new_block = V1Block::read_bytes(block_cursor)?;
        // Assert equality of reconstruction
        assert_eq!(block, new_block);
        // Ok
        Ok(())
    }
}
