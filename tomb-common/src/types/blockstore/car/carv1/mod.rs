// Modules
pub mod carv1blockstore;
pub(crate) mod v1block;
pub(crate) mod v1header;
pub(crate) mod v1index;

// Code
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::libipld::Cid;

use self::{v1block::V1Block, v1header::V1Header, v1index::V1Index};

use super::carv2::V2_PH_SIZE;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct CarV1 {
    pub header: V1Header,
    pub(crate) read_header_len: RefCell<u64>,
    pub index: V1Index,
}

impl CarV1 {
    /// Read in a CARv1 object, assuming the Reader is already seeked to the first byte of the CARv1
    pub(crate) fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        let header_start = r.stream_position()?;
        // Read the header
        let header = V1Header::read_bytes(&mut r)?;
        // println!("finished reading v1header: {:?}", header);
        let read_header_len = RefCell::new(r.stream_position()? - header_start);
        // Generate an index
        let index = V1Index::read_bytes(&mut r)?;
        // println!("finished reading v1index: {:?}", index);
        Ok(Self {
            header,
            index,
            read_header_len,
        })
    }

    /// Write out a CARv1 object, assuming the Writer is already seeked to the first byte of the CARv1
    pub(crate) fn write_bytes<R: Read + Seek, W: Write + Seek>(
        &self,
        mut r: R,
        mut w: W,
    ) -> Result<()> {
        // Save our starting point
        let carv1_start = r.stream_position()?;
        w.seek(SeekFrom::Start(carv1_start))?;

        // Write the header into a buffer
        let mut current_header_buf: Vec<u8> = Vec::new();
        self.header.write_bytes(&mut current_header_buf)?;

        // Compute data offset
        let data_offset = current_header_buf.len() as i64 - *self.read_header_len.borrow() as i64;

        // Keep track of the new index being built
        let mut new_index: HashMap<Cid, u64> = HashMap::new();

        // Skip to the point where the old data started
        r.seek(SeekFrom::Start(
            carv1_start + *self.read_header_len.borrow(),
        ))?;

        // Whiel we're able to successfully read in blocks
        while let Ok(block_offset) = r.stream_position() &&
              let Ok((varint, cid)) = V1Block::start_read(&mut r) &&
              let Ok(block) = V1Block::finish_read(varint, cid, &mut r) {
                // Compute the new offset of the block
                let new_offset = (block_offset as i64 + data_offset) as u64;
                // Move to that offset
                w.seek(SeekFrom::Start(new_offset))?;
                // Write the block there
                block.write_bytes(&mut w)?;
                // Insert new offset into index
                new_index.insert(cid, new_offset);
        }

        // Update index
        *self.index.map.borrow_mut() = new_index.clone();
        self.index
            .set_next_block((self.index.get_next_block() as i64 + data_offset) as u64);

        // Move back to the satart
        w.seek(SeekFrom::Start(carv1_start))?;
        // Write the header, now that the bytes it might have overwritten have been moved
        w.write_all(&current_header_buf)?;
        // Flush
        w.flush()?;
        Ok(())
    }

    pub(crate) fn get_block<R: Read + Seek>(&self, cid: &Cid, mut r: R) -> Result<V1Block> {
        let block_offset = self.index.get_offset(cid)?;
        r.seek(SeekFrom::Start(block_offset))?;
        V1Block::read_bytes(&mut r)
    }

    pub(crate) fn put_block<W: Write + Seek>(&self, block: &V1Block, mut w: W) -> Result<()> {
        // Move to the end
        w.seek(SeekFrom::Start(self.index.get_next_block()))?;
        // Insert current offset before bytes are written
        self.index.insert_offset(&block.cid, w.stream_position()?);
        // Write the bytes
        block.write_bytes(&mut w)?;
        // Update the next block position
        self.index.set_next_block(w.stream_position()?);
        // Return Ok
        Ok(())
    }

    pub(crate) fn get_all_cids(&self) -> Vec<Cid> {
        self.index.get_all_cids()
    }

    pub(crate) fn insert_root(&self, root: &Cid) {
        // Grab reference to roots
        let mut roots = self.header.roots.borrow_mut();
        // Insert new root
        roots.push(*root);
    }

    pub(crate) fn empty_roots(&self) {
        // Grab reference to roots
        let mut roots = self.header.roots.borrow_mut();
        // Insert new root
        *roots = Vec::new();
    }

    pub(crate) fn new<R: Read + Seek, W: Write + Seek>(
        version: u64,
        mut r: R,
        mut w: W,
    ) -> Result<Self> {
        let car = Self::default(version);
        car.header.write_bytes(&mut w)?;
        Self::read_bytes(&mut r)
    }
}

impl PartialEq for CarV1 {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header && self.index == other.index
    }
}

impl CarV1 {
    pub(crate) fn default(version: u64) -> Self {
        let header = V1Header::default(version);
        let mut buf: Vec<u8> = Vec::new();
        header.write_bytes(&mut buf).unwrap();

        // Header length
        let hlen = buf.len() as u64;

        Self {
            header,
            read_header_len: RefCell::new(hlen),
            index: V1Index {
                map: RefCell::new(HashMap::new()),
                next_block: RefCell::new(if version == 1 {
                    hlen
                } else {
                    hlen + V2_PH_SIZE
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::carv1::{v1block::V1Block, CarV1};
    use anyhow::Result;
    use fs_extra::file;
    use serial_test::serial;
    use std::{
        fs::{copy, remove_file, File, OpenOptions},
        io::{Seek, SeekFrom},
        path::Path,
        str::FromStr,
    };
    use wnfs::libipld::{Cid, IpldCodec};

    #[test]
    #[serial]
    fn from_disk_basic() -> Result<()> {
        let car_path = Path::new("car-fixtures").join("carv1-basic.car");
        let mut file = File::open(car_path)?;
        let car = CarV1::read_bytes(&mut file)?;

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
            car.get_block(&block_cids[0], &mut file)?,
            car.get_block(&block_cids[1], &mut file)?,
            car.get_block(&block_cids[2], &mut file)?,
            car.get_block(&block_cids[3], &mut file)?,
            car.get_block(&block_cids[4], &mut file)?,
            car.get_block(&block_cids[5], &mut file)?,
            car.get_block(&block_cids[6], &mut file)?,
            car.get_block(&block_cids[7], &mut file)?,
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
    fn insert_root() -> Result<()> {
        let fixture_path = Path::new("car-fixtures").join("carv1-basic.car");
        let test_path = &Path::new("test");
        let original_path = &test_path.join("carv1-basic-inert-root-original.car");
        let new_path = &test_path.join("carv1-basic-inert-root-updated.car");

        copy(fixture_path, original_path)?;
        copy(original_path, new_path)?;

        let mut r = File::open(original_path)?;
        let mut w = File::create(new_path)?;

        // Read in the car
        let car = CarV1::read_bytes(&mut r)?;

        // Insert a root
        car.insert_root(&Cid::default());

        r.seek(std::io::SeekFrom::Start(0))?;
        car.write_bytes(&mut r, &mut w)?;

        // Read in the car
        let mut r2 = File::open(&new_path)?;
        let new_car = CarV1::read_bytes(&mut r2)?;

        assert_eq!(car.header, new_car.header);
        assert_eq!(car.index, new_car.index);
        assert_eq!(car, new_car);

        Ok(())
    }

    #[test]
    #[serial]
    fn put_get_block() -> Result<()> {
        let car_path = &Path::new("car-fixtures").join("carv1-basic.car");
        let original_path = &Path::new("test").join("carv1-put-get-block.car");

        // Copy from fixture to original path
        remove_file(original_path).ok();

        file::copy(car_path, original_path, &file::CopyOptions::new())?;

        // Define reader and writer
        let mut original_file = File::open(original_path)?;

        // Read original CARv2
        let original = CarV1::read_bytes(&mut original_file)?;
        let all_cids = original.get_all_cids();

        // Assert that we can query all CIDs
        for cid in &all_cids {
            assert!(original.get_block(cid, &mut original_file).is_ok());
        }

        // Insert a block
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = V1Block::new(kitty_bytes, IpldCodec::Raw)?;

        // Writable version of the original file
        let mut writable_original = OpenOptions::new()
            .append(false)
            .write(true)
            .open(original_path)?;

        // Put a new block in
        original.put_block(&block, &mut writable_original)?;
        let new_block = original.get_block(&block.cid, &mut original_file)?;
        assert_eq!(block, new_block);

        // Assert that we can still query all CIDs
        for cid in &all_cids {
            println!("getting block {}", cid);
            original.get_block(cid, &mut original_file)?;
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_no_offset() -> Result<()> {
        let car_path = &Path::new("car-fixtures").join("carv1-basic.car");
        let original_path = &Path::new("test").join("carv1-to-from-disk-no-offset-original.car");
        let updated_path = &Path::new("test").join("carv1-to-from-disk-no-offset-updated.car");

        remove_file(original_path).ok();
        remove_file(updated_path).ok();

        // Copy from fixture to original path
        file::copy(car_path, original_path, &file::CopyOptions::new())?;

        // Define reader and writer
        let mut original_file = File::open(original_path)?;
        let mut updated_file = File::create(updated_path)?;

        // Read original CARv1
        let original = CarV1::read_bytes(&mut original_file)?;
        original_file.seek(std::io::SeekFrom::Start(0))?;
        // Write to updated file
        original.write_bytes(&mut original_file, &mut updated_file)?;

        // Reconstruct
        let mut updated_file = File::open(updated_path)?;
        let reconstructed = CarV1::read_bytes(&mut updated_file)?;

        // Assert equality
        assert_eq!(original.header, reconstructed.header);
        assert_eq!(original.index.next_block, reconstructed.index.next_block);
        assert_eq!(original.index.map, reconstructed.index.map);
        assert_eq!(original, reconstructed);

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_with_offset() -> Result<()> {
        let car_path = &Path::new("car-fixtures").join("carv1-basic.car");
        let original_path = &Path::new("test").join("carv1-to-from-disk-offset-original.car");
        let updated_path = &Path::new("test").join("carv1-to-from-disk-offset-updated.car");

        // Copy from fixture to original path
        remove_file(original_path).ok();
        remove_file(updated_path).ok();

        file::copy(car_path, original_path, &file::CopyOptions::new())?;

        // Define reader and writer
        let mut original_file = File::open(original_path)?;
        let mut updated_file = File::create(updated_path)?;

        // Read original CARv1
        let original = CarV1::read_bytes(&mut original_file)?;

        // Insert a block as a root
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = V1Block::new(kitty_bytes, IpldCodec::Raw)?;
        original.insert_root(&block.cid);

        // Write to updated file
        original_file.seek(SeekFrom::Start(0))?;
        updated_file.seek(SeekFrom::Start(0))?;

        original.write_bytes(&mut original_file, &mut updated_file)?;

        // Reconstruct
        let mut updated_file = File::open(updated_path)?;
        let reconstructed = CarV1::read_bytes(&mut updated_file)?;

        // Assert equality
        assert_eq!(original.header, reconstructed.header);
        assert_eq!(original.index.next_block, reconstructed.index.next_block);
        assert_eq!(original.index.map, reconstructed.index.map);
        assert_eq!(original, reconstructed);

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_with_data() -> Result<()> {
        let car_path = &Path::new("car-fixtures").join("carv1-basic.car");
        let original_path = &Path::new("test").join("carv1-to-from-disk-data-original.car");
        let updated_path = &Path::new("test").join("carv1-to-from-disk-data-updated.car");

        // Copy from fixture to original path
        remove_file(original_path).ok();
        remove_file(updated_path).ok();

        file::copy(car_path, original_path, &file::CopyOptions::new())?;

        // Define reader and writer
        let mut original_file = File::open(original_path)?;
        let mut updated_file = File::create(updated_path)?;

        // Read original CARv1
        let original = CarV1::read_bytes(&mut original_file)?;

        // Insert a block as a root
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = V1Block::new(kitty_bytes, IpldCodec::DagCbor)?;
        original.insert_root(&block.cid);
        let mut writable_original = OpenOptions::new()
            .append(false)
            .write(true)
            .open(original_path)?;
        original.put_block(&block, &mut writable_original)?;

        // Write to updated file
        original_file.seek(SeekFrom::Start(0))?;
        updated_file.seek(SeekFrom::Start(0))?;

        original.write_bytes(&mut original_file, &mut updated_file)?;

        // Reconstruct
        let mut updated_file = File::open(updated_path)?;
        let reconstructed = CarV1::read_bytes(&mut updated_file)?;

        println!("reconstructed index: {:?}", reconstructed.index);

        // Assert equality
        assert_eq!(original.header, reconstructed.header);
        assert_eq!(original.index.next_block, reconstructed.index.next_block);
        assert_eq!(original.index.map, reconstructed.index.map);

        // assert_eq!(original, reconstructed);

        Ok(())
    }
}
