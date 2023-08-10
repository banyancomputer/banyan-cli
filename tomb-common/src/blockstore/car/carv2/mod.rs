/// Fixture
pub mod fixture;
/// CARv2 Header
pub(crate) mod header;
/// CARv2 Index
pub mod index;

// Code
use self::{header::Header, index::indexable::Indexable};
use crate::{
    blockstore::car::carv1::{block::Block, CAR as CARv1},
    Streamable,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::{common::BlockStoreError, libipld::Cid};

// | 11-byte fixed pragma | 40-byte header | optional padding | CARv1 data payload | optional padding | optional index payload |
pub(crate) const PRAGMA_SIZE: usize = 11;
pub(crate) const PH_SIZE: u64 = 51;

// This is the fixed file signature associated with the CARv2 file format
pub(crate) const PRAGMA: [u8; PRAGMA_SIZE] = [
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];

/// Reading / writing a CARv2 from a Byte Stream
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct CAR {
    pub(crate) header: RefCell<Header>,
    /// The CARv1 internal to the CARv2
    pub car: CARv1, // Note that the index is actually stored internally to the CARv1 struct
}

impl CAR {
    /// Load in the CARv2
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Verify the pragma
        Self::verify_pragma(&mut r)?;
        // Load in the header
        let header = Header::read_bytes(&mut r)?;
        // Assert we're at the right spot
        assert_eq!(r.stream_position()?, PH_SIZE);
        // Seek to the data offset
        r.seek(SeekFrom::Start(header.data_offset))?;
        // Load in the CARv1
        let car = CARv1::read_bytes(
            if header.index_offset == 0 {
                None
            } else {
                Some(header.index_offset)
            },
            &mut r,
        )?;
        // Seek to the index offset
        r.seek(SeekFrom::Start(header.index_offset))?;
        // Create the new object
        Ok(Self {
            header: RefCell::new(header),
            car,
        })
    }

    /// Write the CARv2 out to a writer, reading in the content required to write as we go
    pub fn write_bytes<RW: Read + Write + Seek>(&self, mut rw: RW) -> Result<()> {
        // Determine part where the CARv1 will go
        let data_offset = self.header.borrow().data_offset;
        // Skip to it
        rw.seek(SeekFrom::Start(data_offset))?;

        // Write the CARv1
        self.car.write_bytes(&mut rw)?;
        // Update our data size in the Header
        self.update_header(&mut rw)?;
        // Move to index offset
        rw.seek(SeekFrom::Start(self.header.borrow().index_offset))?;
        // Write out the index
        self.car.index.borrow().write_bytes(&mut rw)?;
        // Move back to the start
        rw.seek(SeekFrom::Start(0))?;
        // Write the PRAGMA
        rw.write_all(&PRAGMA)?;
        // Write the updated Header
        self.header.borrow().clone().write_bytes(&mut rw)?;
        // Flush the writer
        rw.flush()?;
        Ok(())
    }

    /// Ensure the validity of the CARv2 PRAGMA in a given stream
    pub(crate) fn verify_pragma<R: Read + Seek>(mut r: R) -> Result<()> {
        // Move to the start of the file
        r.seek(SeekFrom::Start(0))?;
        // Read the pragma
        let mut pragma: [u8; PRAGMA_SIZE] = [0; PRAGMA_SIZE];
        r.read_exact(&mut pragma)?;
        // Ensure correctness
        assert_eq!(pragma, PRAGMA);
        // Return Ok
        Ok(())
    }

    /// Get a Block directly from the CAR
    pub fn get_block<R: Read + Seek>(&self, cid: &Cid, mut r: R) -> Result<Block> {
        // If there is a V2Index
        if let Some(block_offset) = self.car.index.borrow().get_offset(cid) {
            // Move to the start of the block
            r.seek(SeekFrom::Start(block_offset))?;
            // Read the block
            Block::read_bytes(&mut r)
        } else {
            Err(BlockStoreError::CIDNotFound(*cid).into())
        }
    }

    /// Set a Block directly in the CAR
    pub fn put_block<W: Write + Seek>(&self, block: &Block, mut w: W) -> Result<()> {
        // Grab the header
        let header = *self.header.borrow();
        // Determine offset of the next block
        let next_block = header.data_offset + header.data_size;

        // If there is a V2Index
        self.car
            .index
            .borrow_mut()
            .insert_offset(&block.cid, next_block);

        // Move to the end
        w.seek(SeekFrom::Start(next_block))?;
        // Write the bytes
        block.write_bytes(&mut w)?;
        // Update the data size
        self.update_header(&mut w)?;
        // Flush
        w.flush()?;
        // Return Ok
        Ok(())
    }

    /// Create a new CARv2 struct by writing into a stream, then deserializing it
    pub fn new<RW: Read + Write + Seek>(mut rw: RW) -> Result<Self> {
        // Move to CARv1 no padding
        rw.seek(SeekFrom::Start(PH_SIZE))?;
        // Construct a CARv1
        let car = CARv1::default(2);
        // Write CARv1 Header
        car.header.write_bytes(&mut rw)?;
        // Compute the data size
        let data_size = rw.stream_position()? - PH_SIZE;

        // Move to start
        rw.seek(SeekFrom::Start(0))?;
        // Write pragma
        rw.write_all(&PRAGMA)?;
        // Write header with correct data size
        let header = Header {
            characteristics: 0,
            data_offset: PH_SIZE,
            data_size,
            index_offset: 0,
        };
        header.write_bytes(&mut rw)?;
        assert_eq!(rw.stream_position()?, PH_SIZE);

        rw.seek(SeekFrom::Start(0))?;
        Self::read_bytes(&mut rw)
    }

    fn update_header<X: Seek>(&self, mut x: X) -> Result<()> {
        let mut header = self.header.borrow_mut();
        // Update the data size
        let v1_end = x.seek(SeekFrom::End(0))?;
        // Update the data size
        header.data_size = if v1_end > PH_SIZE {
            v1_end - PH_SIZE
        } else {
            0
        };

        // Update the index offset
        header.index_offset = header.data_offset + header.data_size;
        // Mark the characteristics as being fully indexed
        header.characteristics = 1;

        Ok(())
    }

    /// Set the singular root of the CAR
    pub fn set_root(&self, root: &Cid) {
        self.car.set_root(root);
    }

    /// Get the singular root of the CAR
    pub fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::{File, OpenOptions},
        io::{Seek, SeekFrom},
        str::FromStr,
        vec,
    };
    use wnfs::libipld::{Cid, IpldCodec};

    use crate::{
        blockstore::car::{carv1::block::Block, carv2::CAR},
        test::{car_setup, get_read_write},
    };

    #[test]
    #[serial]
    fn from_disk_broken_index() -> Result<()> {
        let car_path = car_setup(2, "basic", "from_disk_basic")?;
        let mut file = File::open(car_path)?;
        // Read the v2 header
        let carv2 = CAR::read_bytes(&mut file)?;

        // Assert version is correct
        assert_eq!(&carv2.car.header.version, &1);

        // CIDs
        let block_cids = vec![
            Cid::from_str("QmfEoLyB5NndqeKieExd1rtJzTduQUPEV8TwAYcUiy3H5Z")?,
            Cid::from_str("QmczfirA7VEH7YVvKPTPoU69XM3qY4DC39nnTsWd4K3SkM")?,
            Cid::from_str("Qmcpz2FHJD7VAhg1fxFXdYJKePtkx1BsHuCrAgWVnaHMTE")?,
            Cid::from_str("bafkreifuosuzujyf4i6psbneqtwg2fhplc2wxptc5euspa2gn3bwhnihfu")?,
            Cid::from_str("bafkreifc4hca3inognou377hfhvu2xfchn2ltzi7yu27jkaeujqqqdbjju")?,
        ];

        // Blocks
        let blocks = vec![
            carv2.get_block(&block_cids[0], &mut file)?,
            carv2.get_block(&block_cids[1], &mut file)?,
            carv2.get_block(&block_cids[2], &mut file)?,
            carv2.get_block(&block_cids[3], &mut file)?,
            carv2.get_block(&block_cids[4], &mut file)?,
        ];

        // Ensure CIDs are matching
        assert_eq!(blocks[0].cid, block_cids[0]);
        assert_eq!(blocks[1].cid, block_cids[1]);
        assert_eq!(blocks[2].cid, block_cids[2]);
        assert_eq!(blocks[3].cid, block_cids[3]);
        assert_eq!(blocks[4].cid, block_cids[4]);

        // Ensure content is correct
        assert_eq!(blocks[0].content, hex::decode("122d0a221220d9c0d5376d26f1931f7ad52d7acc00fc1090d2edb0808bf61eeb0a152826f6261204f09f8da418a401")?);
        assert_eq!(blocks[1].content, hex::decode("12310a221220d745b7757f5b4593eeab7820306c7bc64eb496a7410a0d07df7a34ffec4b97f1120962617272656c657965183a122e0a2401551220a2e1c40da1ae335d4dffe729eb4d5ca23b74b9e51fc535f4a804a261080c294d1204f09f90a11807")?);
        assert_eq!(blocks[2].content, hex::decode("12340a2401551220b474a99a2705e23cf905a484ec6d14ef58b56bbe62e9292783466ec363b5072d120a666973686d6f6e6765721804")?);
        assert_eq!(blocks[3].content, hex::decode("66697368")?);
        assert_eq!(blocks[4].content, hex::decode("6c6f6273746572")?);

        // Construct a vector of the roots we're expecting to find
        let expected_roots = vec![Cid::from_str(
            "QmfEoLyB5NndqeKieExd1rtJzTduQUPEV8TwAYcUiy3H5Z",
        )?];
        // Assert roots are correct
        assert_eq!(&carv2.car.header.roots.borrow().clone(), &expected_roots);

        // Ok
        Ok(())
    }

    #[test]
    #[serial]
    fn put_get_block() -> Result<()> {
        let car_path = &car_setup(2, "indexless", "put_get_block")?;

        // Define reader and writer
        let mut car_file = File::open(car_path)?;

        // Read original CARv2
        let original = CAR::read_bytes(&mut car_file)?;
        let index = original.car.index.borrow().clone();
        let all_cids = index.buckets[0].map.keys().collect::<Vec<&Cid>>();

        // Assert that we can query all CIDs
        for cid in &all_cids {
            assert!(original.get_block(cid, &mut car_file).is_ok());
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
        let new_block = original.get_block(&block.cid, &mut car_file)?;
        assert_eq!(block, new_block);

        // Assert that we can still query all CIDs
        for cid in &all_cids {
            assert!(original.get_block(cid, &mut car_file).is_ok());
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_no_offset() -> Result<()> {
        let car_path = &car_setup(2, "indexless", "to_from_disk_no_offset_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CAR::read_bytes(&mut original_rw)?;
        // Write to updated file
        original.write_bytes(&mut original_rw)?;

        // Reconstruct
        original_rw.seek(SeekFrom::Start(0))?;
        let reconstructed = CAR::read_bytes(&mut original_rw)?;

        // Assert equality
        assert_eq!(original.header, reconstructed.header);
        assert_eq!(original.car.header, reconstructed.car.header);
        assert_eq!(original.car.index, reconstructed.car.index);
        assert_eq!(original, reconstructed);

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_with_data() -> Result<()> {
        let car_path = &car_setup(2, "indexless", "to_from_disk_with_data_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CAR::read_bytes(&mut original_rw)?;

        // Insert a block
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = Block::new(kitty_bytes, IpldCodec::Raw)?;

        // Writable version of the original file
        original.put_block(&block, &mut original_rw)?;
        // Write to updated file
        original.write_bytes(&mut original_rw)?;

        // Reconstruct
        let updated_rw = get_read_write(car_path)?;
        let reconstructed = CAR::read_bytes(&updated_rw)?;

        // Assert equality
        assert_eq!(original.header, reconstructed.header);
        assert_eq!(original.car.header, reconstructed.car.header);
        assert_eq!(original.car.index, reconstructed.car.index);
        assert_eq!(original, reconstructed);

        Ok(())
    }
}