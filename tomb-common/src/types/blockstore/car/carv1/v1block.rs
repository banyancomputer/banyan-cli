use crate::types::blockstore::car::varint::{encode_varint_u128, read_varint_u128};
use anyhow::Result;
use std::io::{Read, Seek, Write};
use wnfs::libipld::{
    multihash::{Code, MultihashDigest},
    Cid, IpldCodec,
};

// | 19-byte varint | x-byte Cid | x-byte content |
#[derive(PartialEq, Debug)]
pub(crate) struct V1Block {
    pub varint: u128,
    pub cid: Cid,
    pub content: Vec<u8>,
}

impl V1Block {
    pub fn new(content: Vec<u8>, codec: IpldCodec) -> Result<Self> {
        // Compute the SHA256 hash of the bytes
        let hash = Code::Sha2_256.digest(&content);
        // Represent the hash as a V1 CID
        let cid = Cid::new_v1(codec.into(), hash);
        let varint = (cid.to_bytes().len() + content.len()) as u128;
        // Create new
        Ok(Self {
            varint,
            cid,
            content,
        })
    }

    /// Serialize the current object
    pub(crate) fn write_bytes<W: Write>(&self, mut w: W) -> Result<usize> {
        // Encode varint as buf
        let varint_buf: Vec<u8> = encode_varint_u128(self.varint);
        // Represent CID as bytes
        let cid_buf: Vec<u8> = self.cid.to_bytes();
        // Write all bytes
        w.write_all(&varint_buf)?;
        w.write_all(&cid_buf)?;
        w.write_all(&self.content)?;
        // Flush
        w.flush()?;
        // Return size
        Ok(varint_buf.len() + cid_buf.len() + self.content.len())
    }

    pub(crate) fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        let (varint, cid) = Self::start_read(&mut r)?;
        Self::finish_read(varint, cid, &mut r)
    }

    pub(crate) fn start_read<R: Read + Seek>(mut r: R) -> Result<(u128, Cid)> {
        // println!("reading cid from offset {}", r.stream_position()?);
        // Read the varint
        let varint = read_varint_u128(&mut r)?;
        // Read the CID
        let cid = Cid::read_bytes(&mut r)?;
        // Return
        Ok((varint, cid))
    }

    pub(crate) fn finish_read<R: Read + Seek>(varint: u128, cid: Cid, mut r: R) -> Result<Self> {
        // Determine how much data has yet to be read from this block
        let content_length = varint as usize - cid.to_bytes().len();
        // Create a content vector with the specified capacity
        let mut content: Vec<u8> = vec![0; content_length];
        // Read exactly that much content
        r.read_exact(&mut content)?;
        // Create new Self
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
    use wnfs::libipld::IpldCodec;

    #[test]
    fn read_write_bytes() -> Result<()> {
        // Raw bytes
        let data_example = "Hello Kitty!".as_bytes().to_vec();
        // Create new V1Block with these content bytes
        let block = V1Block::new(data_example, IpldCodec::Raw)?;
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
