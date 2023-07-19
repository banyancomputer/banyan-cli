/// CARv1 Block
pub mod block;
/// CARv1 Header
pub mod header;
/// Custom Index of CARv1 content
pub mod index;

// Code
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::libipld::Cid;

use self::{block::Block, header::Header, index::Index};
use super::carv2::PH_SIZE;

/// Reading / writing a CARv1 from a Byte Stream
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CAR {
    /// The CARv1 Header
    pub header: Header,
    /// The CARv1 Index
    pub index: RefCell<Index>,
    /// The length of the CARv1 Header when read in
    pub(crate) read_header_len: RefCell<u64>,
}

impl CAR {
    /// Read in a CARv1 object, assuming the Reader is already seeked to the first byte of the CARv1
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Track the part of the stream where the V1Header starts
        let header_start = r.stream_position()?;
        // Read the Header
        let header = Header::read_bytes(&mut r)?;
        // Determine the length of the header that we just read
        let read_header_len = RefCell::new(r.stream_position()? - header_start);
        println!("read_header_len: {}", read_header_len.borrow().clone());
        // Generate an index
        let index = RefCell::new(Index::read_bytes(&mut r)?);
        Ok(Self {
            header,
            index,
            read_header_len,
        })
    }

    /// Write out a CARv1 object, assuming the Writer is already seeked to the first byte of the CARv1
    pub fn write_bytes<RW: Read + Write + Seek>(&self, mut rw: RW) -> Result<()> {
        // Save our starting point
        let carv1_start = rw.stream_position()?;

        // Write the header into a buffer
        let mut current_header_buf: Vec<u8> = Vec::new();
        self.header.write_bytes(&mut current_header_buf)?;

        // Compute data offset
        let data_offset = current_header_buf.len() as i64 - *self.read_header_len.borrow() as i64;

        // Keep track of the new index being built
        let mut new_index: HashMap<Cid, u64> = HashMap::new();
        // Grab all offsets
        let mut offsets: Vec<u64> = self.index.borrow().clone().map.into_values().collect();
        // Sort those offsets so the first offsets occur first
        offsets.sort();
        // If the header got bigger
        if data_offset > 0 {
            // Sort those offsets so the final offsets occur first
            offsets.reverse();
        }

        // For each offset tallied
        for block_offset in offsets {
            // Move to the existing block location
            rw.seek(SeekFrom::Start(block_offset))?;
            // Read the block
            let block = Block::read_bytes(&mut rw)?;
            // Compute the new offset of the block
            let new_offset = (block_offset as i64 + data_offset) as u64;
            // Move to that offset
            rw.seek(SeekFrom::Start(new_offset))?;
            // Write the block there
            block.write_bytes(&mut rw)?;
            // Insert new offset into index
            new_index.insert(block.cid, new_offset);
        }

        {
            // Update index
            let mut index = self.index.borrow_mut();
            index.map = new_index;
            index.next_block = (index.next_block as i64 + data_offset) as u64;
        }

        println!(
            "index before write bytes finished: {:?}",
            self.index.borrow().clone()
        );

        // Move back to the satart
        rw.seek(SeekFrom::Start(carv1_start))?;
        // Write the header, now that the bytes it might have overwritten have been moved
        rw.write_all(&current_header_buf)?;
        // Flush
        rw.flush()?;
        Ok(())
    }

    /// Get a Block directly from the CAR
    pub fn get_block<R: Read + Seek>(&self, cid: &Cid, mut r: R) -> Result<Block> {
        let block_offset = self.index.borrow().get_offset(cid)?;
        r.seek(SeekFrom::Start(block_offset))?;
        Block::read_bytes(&mut r)
    }

    /// Set a Block directly in the CAR
    pub fn put_block<W: Write + Seek>(&self, block: &Block, mut w: W) -> Result<()> {
        let mut index = self.index.borrow_mut();
        // Move to the end
        w.seek(SeekFrom::Start(index.next_block))?;
        // Insert current offset before bytes are written
        index.map.insert(block.cid, w.stream_position()?);
        // Write the bytes
        block.write_bytes(&mut w)?;
        // Update the next block position
        index.next_block = w.stream_position()?;
        // Return Ok
        Ok(())
    }

    /// Create a new CARv1 struct by writing into a stream, then deserializing it
    pub fn new<RW: Read + Write + Seek>(version: u64, mut rw: RW) -> Result<Self> {
        let car = Self::default(version);
        car.header.write_bytes(&mut rw)?;
        rw.seek(SeekFrom::Start(0))?;
        Self::read_bytes(rw)
    }

    /// Set the singular root of the CAR
    pub fn set_root(&self, root: &Cid) {
        *self.header.roots.borrow_mut() = vec![*root];
    }

    /// Get the singular root of the CAR
    pub fn get_root(&self) -> Option<Cid> {
        let roots = self.header.roots.borrow();
        if roots.len() > 0 {
            Some(roots[0])
        } else {
            None
        }
    }
}

impl PartialEq for CAR {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header && self.index == other.index
    }
}

impl CAR {
    pub(crate) fn default(version: u64) -> Self {
        let header = Header::default(version);
        let mut buf: Vec<u8> = Vec::new();
        header.write_bytes(&mut buf).unwrap();

        // Header length
        let hlen = buf.len() as u64;

        Self {
            header,
            read_header_len: RefCell::new(hlen),
            index: RefCell::new(Index {
                map: HashMap::new(),
                next_block: if version == 1 { hlen } else { hlen + PH_SIZE },
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        types::blockstore::car::carv1::{block::Block, CAR},
        utils::test::car_setup,
    };
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::{File, OpenOptions},
        io::{Seek, SeekFrom},
        path::Path,
        str::FromStr,
    };
    use wnfs::libipld::{Cid, IpldCodec};

    fn get_read_write(path: &Path) -> Result<File, std::io::Error> {
        OpenOptions::new()
            .append(false)
            .read(true)
            .write(true)
            .open(path)
    }

    #[test]
    #[serial]
    fn from_disk_basic() -> Result<()> {
        let car_path = &car_setup(1, "basic", "from_disk_basic")?;
        // Grab read/writer
        let mut rw = get_read_write(car_path)?;
        // Read in the car
        let car = CAR::read_bytes(&mut rw)?;

        // Header tests exist separately, let's just ensure content is correct!

        // CIDs
        let block_cids = vec![
            Cid::from_str("bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm")?,
            Cid::from_str("QmNX6Tffavsya4xgBi2VJQnSuqy9GsxongxZZ9uZBqp16d")?,
            Cid::from_str("bafkreifw7plhl6mofk6sfvhnfh64qmkq73oeqwl6sloru6rehaoujituke")?,
            Cid::from_str("QmWXZxVQ9yZfhQxLD35eDR8LiMRsYtHxYqTFCBbJoiJVys")?,
            Cid::from_str("bafkreiebzrnroamgos2adnbpgw5apo3z4iishhbdx77gldnbk57d4zdio4")?,
            Cid::from_str("QmdwjhxpxzcMsR3qUuj7vUL8pbA7MgR3GAxWi2GLHjsKCT")?,
            Cid::from_str("bafkreidbxzk2ryxwwtqxem4l3xyyjvw35yu4tcct4cqeqxwo47zhxgxqwq")?,
            Cid::from_str("bafyreidj5idub6mapiupjwjsyyxhyhedxycv4vihfsicm2vt46o7morwlm")?,
        ];

        // Blocks
        let blocks = vec![
            car.get_block(&block_cids[0], &mut rw)?,
            car.get_block(&block_cids[1], &mut rw)?,
            car.get_block(&block_cids[2], &mut rw)?,
            car.get_block(&block_cids[3], &mut rw)?,
            car.get_block(&block_cids[4], &mut rw)?,
            car.get_block(&block_cids[5], &mut rw)?,
            car.get_block(&block_cids[6], &mut rw)?,
            car.get_block(&block_cids[7], &mut rw)?,
        ];

        // Ensure CIDs are matching
        assert_eq!(blocks[0].cid, block_cids[0]);
        assert_eq!(blocks[1].cid, block_cids[1]);
        assert_eq!(blocks[2].cid, block_cids[2]);
        assert_eq!(blocks[3].cid, block_cids[3]);
        assert_eq!(blocks[4].cid, block_cids[4]);
        assert_eq!(blocks[5].cid, block_cids[5]);
        assert_eq!(blocks[6].cid, block_cids[6]);
        assert_eq!(blocks[7].cid, block_cids[7]);

        // Ensure content is correct
        assert_eq!(blocks[0].content, hex::decode("a2646c696e6bd82a582300122002acecc5de2438ea4126a3010ecb1f8a599c8eff22fff1a1dcffe999b27fd3de646e616d6564626c6970")?);
        assert_eq!(blocks[1].content, hex::decode("122e0a2401551220b6fbd675f98e2abd22d4ed29fdc83150fedc48597e92dd1a7a24381d44a274511204626561721804122f0a22122079a982de3c9907953d4d323cee1d0fb1ed8f45f8ef02870c0cb9e09246bd530a12067365636f6e64189501")?);
        assert_eq!(blocks[2].content, hex::decode("63636363")?);
        assert_eq!(blocks[3].content, hex::decode("122d0a240155122081cc5b17018674b401b42f35ba07bb79e211239c23bffe658da1577e3e6468771203646f671804122d0a221220e7dc486e97e6ebe5cdabab3e392bdad128b6e09acc94bb4e2aa2af7b986d24d0120566697273741833")?);
        assert_eq!(blocks[4].content, hex::decode("62626262")?);
        assert_eq!(blocks[5].content, hex::decode("122d0a240155122061be55a8e2f6b4e172338bddf184d6dbee29c98853e0a0485ecee7f27b9af0b412036361741804")?);
        assert_eq!(blocks[6].content, hex::decode("61616161")?);
        assert_eq!(
            blocks[7].content,
            hex::decode("a2646c696e6bf6646e616d65656c696d626f")?
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn set_root() -> Result<()> {
        let car_path = &car_setup(1, "basic", "set_root_original")?;
        // Grab read/writer
        let mut rw = get_read_write(car_path)?;
        // Read in the car
        let car = CAR::read_bytes(&mut rw)?;

        // Insert a root
        car.set_root(&Cid::default());

        rw.seek(SeekFrom::Start(0))?;
        car.write_bytes(&mut rw)?;

        // Read in the car
        let mut r2 = File::open(car_path)?;
        let new_car = CAR::read_bytes(&mut r2)?;

        assert_eq!(car.header, new_car.header);
        assert_eq!(car.index, new_car.index);
        assert_eq!(car, new_car);

        Ok(())
    }

    #[test]
    #[serial]
    fn put_get_block() -> Result<()> {
        let car_path = &car_setup(1, "basic", "put_get_block")?;
        // Define reader and writer
        let mut original_file = File::open(car_path)?;

        // Read original CARv2
        let original = CAR::read_bytes(&mut original_file)?;
        let index = original.index.borrow().clone();
        let all_cids = index.map.keys().collect::<Vec<&Cid>>();

        // Assert that we can query all CIDs
        for cid in &all_cids {
            assert!(original.get_block(cid, &mut original_file).is_ok());
        }

        // Insert a block
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = Block::new(kitty_bytes, IpldCodec::Raw)?;

        // Writable version of the original file
        let mut writable_original = OpenOptions::new()
            .append(false)
            .write(true)
            .open(car_path)?;

        // Put a new block in
        original.put_block(&block, &mut writable_original)?;
        let new_block = original.get_block(&block.cid, &mut original_file)?;
        assert_eq!(block, new_block);

        // Assert that we can still query all CIDs
        for cid in &all_cids {
            original.get_block(cid, &mut original_file)?;
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_no_offset() -> Result<()> {
        let car_path = &car_setup(1, "basic", "to_from_disk_no_offset_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CAR::read_bytes(&mut original_rw)?;
        // Move back to start
        original_rw.seek(std::io::SeekFrom::Start(0))?;
        // Write to updated file
        original.write_bytes(&mut original_rw)?;

        // Reconstruct
        let mut updated_rw = File::open(car_path)?;
        let updated = CAR::read_bytes(&mut updated_rw)?;

        // Assert equality
        assert_eq!(original.header, updated.header);
        assert_eq!(original.index, updated.index);
        assert_eq!(original, updated);

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_with_offset() -> Result<()> {
        let car_path = &car_setup(1, "basic", "to_from_disk_with_offset_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CAR::read_bytes(&mut original_rw)?;

        // Insert a block as a root
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = Block::new(kitty_bytes, IpldCodec::Raw)?;
        original.set_root(&block.cid);

        // Write to updated file
        original_rw.seek(SeekFrom::Start(0))?;
        // Rewrite
        original.write_bytes(&mut original_rw)?;

        // Reconstruct
        let mut updated_rw = get_read_write(car_path)?;
        // Read in the car
        let updated = CAR::read_bytes(&mut updated_rw)?;

        // Assert equality
        assert_eq!(original.header, updated.header);
        assert_eq!(original.index, updated.index);
        assert_eq!(original, updated);

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_with_data() -> Result<()> {
        let car_path = &car_setup(1, "basic", "to_from_disk_with_data_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CAR::read_bytes(&mut original_rw)?;

        // Insert a block as a root
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = Block::new(kitty_bytes, IpldCodec::DagCbor)?;
        original.set_root(&block.cid);
        original.put_block(&block, &mut original_rw)?;

        // Write updates to file
        original_rw.seek(SeekFrom::Start(0))?;
        original.write_bytes(&mut original_rw)?;

        // Reconstruct
        let mut updated_rw = get_read_write(car_path)?;
        // Read in the car
        let updated = CAR::read_bytes(&mut updated_rw)?;

        // Assert equality
        assert_eq!(original.header, updated.header);
        assert_eq!(original.index, updated.index);
        assert_eq!(original.index, updated.index);

        // assert_eq!(original, reconstructed);

        Ok(())
    }
}
