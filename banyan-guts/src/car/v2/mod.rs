/// Fixture
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod fixture;
/// CarV2 Header
pub mod header;
/// CarV2 Index
pub mod index;

use futures::executor::block_on;
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
use serde::{Deserializer, Serializer};
use std::io::Cursor;
use std::io::{Read, Seek, SeekFrom, Write};
use tokio::sync::RwLock;
use wnfs::libipld::Cid;

// | 11-byte fixed pragma | 40-byte header | optional padding | CarV1 data payload | optional padding | optional index payload |
pub(crate) const PRAGMA_SIZE: usize = 11;
pub(crate) const PH_SIZE: u64 = 51;

// This is the fixed file signature associated with the CarV2 file format
pub(crate) const PRAGMA: [u8; PRAGMA_SIZE] = [
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];

/// Reading / writing a CarV2 from a Byte Stream
#[derive(Debug)]
pub struct CarV2 {
    /// The header
    pub(crate) header: RwLock<Header>,
    /// The CarV1 internal to the CarV2
    pub car: CarV1, // Note that the index is actually stored internally to the CarV1 struct
}

/// WARNING: this will block on the rwlock!
impl PartialEq for CarV2 {
    fn eq(&self, other: &Self) -> bool {
        *futures::executor::block_on(self.header.read())
            == *futures::executor::block_on(other.header.read())
            && self.car == other.car
    }
}

/// WARNING: this will block on the rwlock!
impl Clone for CarV2 {
    fn clone(&self) -> Self {
        Self {
            header: RwLock::new(*futures::executor::block_on(self.header.read())),
            car: self.car.clone(),
        }
    }
}

impl Serialize for CarV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let header = *futures::executor::block_on(self.header.read());
        let car = self.car.clone();

        let state = (header, car);

        state.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CarV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (header, car) = Deserialize::deserialize(deserializer)?;

        Ok(Self {
            header: RwLock::new(header),
            car,
        })
    }
}

impl CarV2 {
    /// Return the data size specified by the CarV2 Header
    pub async fn data_size(&self) -> u64 {
        self.header.read().await.data_size
    }

    /// Load in the CarV2
    pub async fn read_bytes<R: Read + Seek + Send>(mut r: R) -> Result<Self, CarError> {
        // Verify the pragma
        Self::verify_pragma(&mut r)?;
        // Load in the header
        let header = Header::read_bytes(&mut r).await?;
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
        )
        .await?;
        // Seek to the index offset
        r.seek(SeekFrom::Start(header.index_offset))?;
        // Create the new object
        Ok(Self {
            header: RwLock::new(header),
            car,
        })
    }

    /// Write the CarV2 out to a writer, reading in the content required to write as we go
    pub async fn write_bytes<RW: Read + Write + Seek + Send>(
        &self,
        mut rw: RW,
    ) -> Result<(), CarError> {
        // Determine part where the CarV1 will go
        let data_offset = self.header.read().await.data_offset;
        // Skip to it
        rw.seek(SeekFrom::Start(data_offset))?;
        // Write the CarV1
        let data_end = self.car.write_bytes(&mut rw).await?;
        rw.seek(SeekFrom::Start(data_end))?;
        // Update our data size in the Header
        self.update_header(data_end).await?;
        // Move to index offset
        rw.seek(SeekFrom::Start(self.header.read().await.index_offset))?;
        // Write out the index
        self.car.index.write().await.write_bytes(&mut rw).await?;
        // Move back to the start
        rw.seek(SeekFrom::Start(0))?;
        // Write the PRAGMA
        rw.write_all(&PRAGMA)?;
        // Write the updated Header
        self.header.write().await.write_bytes(&mut rw).await?;
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
        block_on(self.write_bytes(&mut rw))?;
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
    pub async fn get_block<R: Read + Seek + Send>(
        &self,
        cid: &Cid,
        mut r: R,
    ) -> Result<Block, CarError> {
        // If there is a V2Index
        if let Some(block_offset) = self.car.index.read().await.get_offset(cid) {
            // Move to the start of the block
            r.seek(SeekFrom::Start(block_offset))?;
            // Read the block
            Block::read_bytes(&mut r).await
        } else {
            Err(CarError::missing_block(cid))
        }
    }

    /// Set a Block directly in the CarV2
    pub async fn put_block<W: Write + Seek + Send>(
        &self,
        block: &Block,
        mut w: W,
    ) -> Result<(), CarError> {
        // Grab the header
        let header = self.header.write().await;
        // Determine offset of the next block
        let next_block = header.data_offset + header.data_size;

        // Grab index
        let index: &mut Index<Bucket> = &mut *self.car.index.write().await;
        // If the index does not contain the Cid
        if index.get_offset(&block.cid).is_none() {
            // Insert offset
            index.insert_offset(&block.cid, next_block);
            // Move to the end
            w.seek(SeekFrom::Start(next_block))?;
            // Write the bytes
            block.write_bytes(&mut w).await?;
            // Update the data size
            self.update_header(w.stream_position()?).await?;
            // Flush
            w.flush()?;
        }
        // Return Ok
        Ok(())
    }

    /// Create a new CarV2 struct by writing into a stream, then deserializing it
    pub async fn new<RW: Read + Write + Seek + Send>(mut rw: RW) -> Result<Self, CarError> {
        // Move to CarV1 no padding
        rw.seek(SeekFrom::Start(PH_SIZE))?;
        // Construct a CarV1
        let car = CarV1::default(2).await;
        // Write CarV1 Header
        car.header.write_bytes(&mut rw).await?;
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
        header.write_bytes(&mut rw).await?;
        assert_eq!(rw.stream_position()?, PH_SIZE);

        rw.seek(SeekFrom::Start(0))?;
        Self::read_bytes(&mut rw).await
    }

    async fn update_header(&self, data_end: u64) -> Result<(), CarError> {
        let mut header = self.header.write().await;
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
    pub async fn set_root(&self, root: &Cid) {
        self.car.set_root(root).await;
    }

    /// Get the singular root of the CarV2
    pub async fn get_root(&self) -> Option<Cid> {
        self.car.get_root().await
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

    #[tokio::test]
    #[serial]
    async fn from_disk_broken_index() -> Result<(), CarError> {
        let car_path = car_test_setup(2, "basic", "from_disk_basic")?;
        let mut file = File::open(car_path)?;
        assert!(CarV2::read_bytes(&mut file).await.is_err());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_get_block() -> Result<(), CarError> {
        let car_path = &car_test_setup(2, "indexless", "put_get_block")?;

        // Define reader and writer
        let mut car_file = File::open(car_path)?;

        // Read original CarV2
        let original = CarV2::read_bytes(&mut car_file).await?;
        let index = original.car.index.read().await.clone();
        let all_cids = index.buckets[0].map.keys().collect::<Vec<&Cid>>();

        // Assert that we can query all CIDs
        for cid in &all_cids {
            assert!(original.get_block(cid, &mut car_file).await.is_ok());
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
        original.put_block(&block, &mut writable_original).await?;
        let new_block = original.get_block(&block.cid, &mut car_file).await?;
        assert_eq!(block, new_block);

        // Assert that we can still query all CIDs
        for cid in &all_cids {
            assert!(original.get_block(cid, &mut car_file).await.is_ok());
        }

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn to_from_disk_no_offset() -> Result<(), CarError> {
        let car_path = &car_test_setup(2, "indexless", "to_from_disk_no_offset_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CarV2::read_bytes(&mut original_rw).await?;
        // Write to updated file
        original.write_bytes(&mut original_rw).await?;

        // Reconstruct
        original_rw.seek(SeekFrom::Start(0))?;
        let reconstructed = CarV2::read_bytes(&mut original_rw).await?;

        // Assert equality
        assert_eq!(original, reconstructed);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn to_from_disk_with_data() -> Result<(), CarError> {
        let car_path = &car_test_setup(2, "indexless", "to_from_disk_with_data_original")?;
        // Grab read/writer
        let mut original_rw = get_read_write(car_path)?;
        // Read in the car
        let original = CarV2::read_bytes(&mut original_rw).await?;

        // Insert a block
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let block = Block::new(kitty_bytes, IpldCodec::Raw)?;

        // Writable version of the original file
        original.put_block(&block, &mut original_rw).await?;
        // Write to updated file
        original.write_bytes(&mut original_rw).await?;

        // Reconstruct
        let updated_rw = get_read_write(car_path)?;
        let reconstructed = CarV2::read_bytes(&updated_rw).await?;

        // Assert equality
        assert_eq!(original, reconstructed);

        Ok(())
    }
}
