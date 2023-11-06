use crate::banyan_car::{
    varint::{encode_varint_u128, read_varint_u128},
    Streamable,
};
use anyhow::Result;
use std::io::{Read, Seek, SeekFrom, Write};
use wnfs::libipld::{
    multihash::{Code, MultihashDigest},
    Cid, IpldCodec,
};

/// CARv1 Data Block
/// | 19-byte varint | x-byte Cid | x-byte content |
#[derive(PartialEq, Debug)]
pub struct Block {
    /// Varint encoding remaining len of block
    pub varint: u128,
    /// Cid of data
    pub cid: Cid,
    /// Data
    pub content: Vec<u8>,
}

impl Block {
    /// Given some data, create a Cid and varint to match
    pub fn new(content: Vec<u8>, codec: IpldCodec) -> Result<Self> {
        // Compute the SHA256 hash of the bytes
        let hash = Code::Sha2_256.digest(&content);
        // Represent the hash as a CID V1
        let cid = Cid::new_v1(codec.into(), hash);
        let varint = (cid.encoded_len() + content.len()) as u128;
        // Create new
        Ok(Self {
            varint,
            cid,
            content,
        })
    }

    /// Read the Varint and Cid from stream only
    pub fn start_read<R: Read + Seek>(mut r: R) -> Result<(u128, Cid)> {
        // Read the varint
        let varint = read_varint_u128(&mut r)?;
        let cid_start = r.stream_position()?;
        // Read the CID with no Multibase
        if let Ok(cid) = Cid::read_bytes(&mut r) {
            return Ok((varint, cid));
        }
        // Skip the Multibase and try again if that didn't work
        r.seek(SeekFrom::Start(cid_start + 1))?;
        let cid = Cid::read_bytes(&mut r)?;
        Ok((varint, cid))
    }

    /// If start read was just called, grab the data that follows it and return a Block
    pub fn finish_read<R: Read + Seek>(varint: u128, cid: Cid, mut r: R) -> Result<Self> {
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

impl Streamable for Block {
    /// Serialize the current object
    fn write_bytes<W: Write>(&self, w: &mut W) -> Result<()> {
        // Represent CID as bytes
        let cid_buf: Vec<u8> = self.cid.to_bytes();
        // Assert that the varint is accurate
        assert_eq!(self.varint, (cid_buf.len() + self.content.len()) as u128);
        // Write all bytes
        w.write_all(&encode_varint_u128(self.varint))?;
        w.write_all(&cid_buf)?;
        w.write_all(&self.content)?;
        // Flush
        w.flush()?;
        // Return size
        Ok(())
    }

    /// Read a Block from stream
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        let (varint, cid) = Self::start_read(&mut *r)?;
        Self::finish_read(varint, cid, r)
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
    #[allow(unused_imports)]
    use super::Block;
    #[allow(unused_imports)]
    use wnfs::libipld::IpldCodec;

    crate::banyan_car::streamable_tests! {
        Block:
        carblock: {
            // Raw bytes
            let data_example = "Hello Kitty!".as_bytes().to_vec();
            // Create new Block with these content bytes
            Block::new(data_example, IpldCodec::Raw).expect("unable to create new Block")
        },
    }
}
