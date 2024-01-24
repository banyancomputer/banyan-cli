/// Fixture
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod fixture;
/// CarV2 Header
pub mod header;
/// CarV2 Index
pub mod index;

pub use header::{Header, HEADER_SIZE};

// Code
use self::index::indexable::Indexable;
use super::error::CarError;
use crate::car::{
    v1::{Block, CarV1},
    v2::index::{indexsorted::Bucket, Index},
    Streamable,
};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::libipld::Cid;

// | 11-byte fixed pragma | 40-byte header | optional padding | CarV1 data payload | optional padding | optional index payload |
pub(crate) const PRAGMA_SIZE: usize = 11;
pub(crate) const PH_SIZE: u64 = 51;

// This is the fixed file signature associated with the CarV2 file format
pub(crate) const PRAGMA: [u8; PRAGMA_SIZE] = [
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];

/// Reading / writing a CarV2 from a Byte Stream
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct CarV2 {
    /// The header
    pub(crate) header: RefCell<Header>,
    /// The CarV1 internal to the CarV2
    pub car: CarV1, // Note that the index is actually stored internally to the CarV1 struct
}

impl CarV2 {
    /// Return the data size specified by the CarV2 Header
    pub fn data_size(&self) -> u64 {
        self.header.borrow().data_size
    }

    /// Load in the CarV2
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self, CarError> {
        // Verify the pragma
        Self::verify_pragma(&mut r)?;
        // Load in the header
        let header = Header::read_bytes(&mut r)?;
        // Assert we're at the right spot
        assert_eq!(r.stream_position()?, PH_SIZE);
        // Seek to the data offset
        r.seek(SeekFrom::Start(header.data_offset))?;
        // Load in the CarV1
        let car = CarV1::read_bytes(
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

    /// Write the CarV2 out to a writer, reading in the content required to write as we go
    pub fn write_bytes<RW: Read + Write + Seek>(&self, mut rw: RW) -> Result<(), CarError> {
        // Determine part where the CarV1 will go
        let data_offset = self.header.borrow().data_offset;
        // Skip to it
        rw.seek(SeekFrom::Start(data_offset))?;
        // Write the CarV1
        let data_end = self.car.write_bytes(&mut rw)?;
        rw.seek(SeekFrom::Start(data_end))?;
        // Update our data size in the Header
        self.update_header(data_end)?;
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

    /// Export the CarV2 as bytes to a Vec<u8>
    pub fn to_bytes(&self) -> Result<Vec<u8>, CarError> {
        // Create a new vec
        let vec = Vec::new();
        // Read everything in the car to the vec
        let mut rw = Cursor::new(vec);
        self.write_bytes(&mut rw)?;
        // Return the vec
        Ok(rw.into_inner())
    }

    /// Ensure the validity of the CarV2 PRAGMA in a given stream
    pub(crate) fn verify_pragma<R: Read + Seek>(mut r: R) -> Result<(), CarError> {
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

    /// Get a Block directly from the CarV2
    pub fn get_block<R: Read + Seek>(&self, cid: &Cid, mut r: R) -> Result<Block, CarError> {
        // If there is a V2Index
        if let Some(block_offset) = self.car.index.borrow().get_offset(cid) {
            // Move to the start of the block
            r.seek(SeekFrom::Start(block_offset))?;
            // Read the block
            Block::read_bytes(&mut r)
        } else {
            Err(CarError::missing_block(cid))
        }
    }

    /// Set a Block directly in the CarV2
    pub fn put_block<W: Write + Seek>(&self, block: &Block, mut w: W) -> Result<(), CarError> {
        // Grab the header
        let header = *self.header.borrow();
        // Determine offset of the next block
        let next_block = header.data_offset + header.data_size;

        // Grab index
        let index: &mut Index<Bucket> = &mut self.car.index.borrow_mut();
        // If the index does not contain the Cid
        if index.get_offset(&block.cid).is_none() {
            // Insert offset
            index.insert_offset(&block.cid, next_block);
            // Move to the end
            w.seek(SeekFrom::Start(next_block))?;
            // Write the bytes
            block.write_bytes(&mut w)?;
            // Update the data size
            self.update_header(w.stream_position()?)?;
            // Flush
            w.flush()?;
        }
        // Return Ok
        Ok(())
    }

    /// Create a new CarV2 struct by writing into a stream, then deserializing it
    pub fn new<RW: Read + Write + Seek>(mut rw: RW) -> Result<Self, CarError> {
        // Move to CarV1 no padding
        rw.seek(SeekFrom::Start(PH_SIZE))?;
        // Construct a CarV1
        let car = CarV1::default(2);
        // Write CarV1 Header
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

    fn update_header(&self, data_end: u64) -> Result<(), CarError> {
        let mut header = self.header.borrow_mut();
        // Update the data size
        header.data_size = if data_end > PH_SIZE {
            data_end - PH_SIZE
        } else {
            0
        };

        // Update the index offset
        header.index_offset = header.data_offset + header.data_size;
        // Mark the characteristics as being fully indexed
        header.characteristics = 1;

        Ok(())
    }

    /// Set the singular root of the CarV2
    pub fn set_root(&self, root: &Cid) {
        self.car.set_root(root);
    }

    /// Get the singular root of the CarV2
    pub fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
    use crate::{
        car::{error::CarError, v1::Block, v2::CarV2},
        utils::{get_read_write, testing::blockstores::car_test_setup},
    };
    use serial_test::serial;
    use std::{
        fs::{File, OpenOptions},
        io::{Seek, SeekFrom},
    };
    use wnfs::libipld::{Cid, IpldCodec};

    #[test]
    #[serial]
    fn from_disk_broken_index() -> Result<(), CarError> {
        let car_path = car_test_setup(2, "basic", "from_disk_basic")?;
        let mut file = File::open(car_path)?;
        assert!(CarV2::read_bytes(&mut file).is_err());
        Ok(())
    }

    #[test]
    #[serial]
    fn put_get_block() -> Result<(), CarError> {
        let car_path = &car_test_setup(2, "indexless", "put_get_block")?;

        // Define reader and writer
        let mut car_file = File::open(car_path)?;

        // Read original CarV2
        let original = CarV2::read_bytes(&mut car_file)?;
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
    fn to_from_disk_no_offset() -> Result<(), CarError> {
        let car_path = &car_test_setup(2, "indexless", "to_from_disk_no_offset_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CarV2::read_bytes(&mut original_rw)?;
        // Write to updated file
        original.write_bytes(&mut original_rw)?;

        // Reconstruct
        original_rw.seek(SeekFrom::Start(0))?;
        let reconstructed = CarV2::read_bytes(&mut original_rw)?;

        // Assert equality
        assert_eq!(original.header, reconstructed.header);
        assert_eq!(original.car.header, reconstructed.car.header);
        assert_eq!(original.car.index, reconstructed.car.index);
        assert_eq!(original, reconstructed);

        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_with_data() -> Result<(), CarError> {
        let car_path = &car_test_setup(2, "indexless", "to_from_disk_with_data_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CarV2::read_bytes(&mut original_rw)?;

        // Insert a block
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = Block::new(kitty_bytes, IpldCodec::Raw)?;

        // Writable version of the original file
        original.put_block(&block, &mut original_rw)?;
        // Write to updated file
        original.write_bytes(&mut original_rw)?;

        // Reconstruct
        let updated_rw = get_read_write(car_path)?;
        let reconstructed = CarV2::read_bytes(&updated_rw)?;

        // Assert equality
        assert_eq!(original.header, reconstructed.header);
        assert_eq!(original.car.header, reconstructed.car.header);
        assert_eq!(original.car.index, reconstructed.car.index);
        assert_eq!(original, reconstructed);

        Ok(())
    }
}
