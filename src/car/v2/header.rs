use crate::{
    car::{error::CarError, streamable::Streamable},
    utils::varint::{read_leu128, read_leu64},
};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Seek, Write};

pub const HEADER_SIZE: usize = 40;

// | 16-byte characteristics | 8-byte data offset | 8-byte data size | 8-byte index offset |
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Header {
    pub characteristics: u128,
    pub data_offset: u64,
    pub data_size: u64,
    pub index_offset: u64,
}

impl Streamable for Header {
    type StreamError = CarError;
    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<(), Self::StreamError> {
        let start = w.stream_position()?;
        // Write
        w.write_all(&self.characteristics.to_le_bytes())?;
        w.write_all(&self.data_offset.to_le_bytes())?;
        w.write_all(&self.data_size.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        // Assert correct number of bytes written
        assert_eq!((w.stream_position()? - start) as usize, HEADER_SIZE);
        // Flush
        w.flush()?;
        Ok(())
    }

    fn read_bytes<R: Read>(r: &mut R) -> Result<Self, Self::StreamError> {
        Ok(Self {
            characteristics: read_leu128(r)?,
            data_offset: read_leu64(r)?,
            data_size: read_leu64(r)?,
            index_offset: read_leu64(r)?,
        })
    }
}

impl Header {
    pub fn to_bytes(self) -> Result<Vec<u8>, CarError> {
        let mut header_bytes = Cursor::new(<Vec<u8>>::new());
        self.write_bytes(&mut header_bytes)?;
        Ok(header_bytes.into_inner())
    }
}

impl Serialize for Header {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_bytes()
            .expect("failed to represent header as bytes")
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Header {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut header_bytes = Cursor::new(<Vec<u8>>::deserialize(deserializer)?);
        let new_header =
            Self::read_bytes(&mut header_bytes).expect("failed to read header as bytes");
        Ok(new_header)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use crate::{
        car::{
            error::CarError,
            v2::{header::Header, PRAGMA, PRAGMA_SIZE},
            Streamable,
        },
        utils::testing::blockstores::car_test_setup,
    };
    use serial_test::serial;
    use std::{
        fs::File,
        io::{Seek, Write},
        path::Path,
    };

    #[test]
    #[serial]
    fn read_disk() -> Result<(), CarError> {
        let car_path = car_test_setup(2, "basic", "read_disk")?;
        let mut file = File::open(car_path)?;
        // Skip the pragma
        file.seek(std::io::SeekFrom::Start(PRAGMA_SIZE as u64))?;
        // Read the header
        let header = Header::read_bytes(&mut file)?;
        // Characteristics are 0
        assert_eq!(header.characteristics, 0);
        assert_eq!(header.data_offset, 51);
        assert_eq!(header.data_size, 448);
        assert_eq!(header.index_offset, 499);
        Ok(())
    }

    #[test]
    fn from_scratch() -> Result<(), CarError> {
        let path = &Path::new("test")
            .join("car")
            .join("carv2_header_from_scratch.car");
        let mut file = File::create(path)?;
        // Write the pragma
        file.write_all(&PRAGMA)?;
        // Read the header
        let header = Header {
            characteristics: 0,
            data_offset: 50,
            data_size: 50,
            index_offset: 0,
        };
        header.write_bytes(&mut file)?;
        Ok(())
    }

    crate::car::streamable_tests! {
        <crate::car::v2::Header, crate::car::error::CarError>:
        v2header: crate::car::v2::Header {
            characteristics: 0,
            data_offset: 50,
            data_size: 50,
            index_offset: 0,
        },
    }
}
