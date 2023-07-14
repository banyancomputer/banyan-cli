use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::types::blockstore::car::varint::{
    encode_varint_u128_exact, encode_varint_u64_exact, read_varint_u128_exact,
    read_varint_u64_exact,
};

pub const HEADER_SIZE: usize = 40;

// | 16-byte characteristics | 8-byte data offset | 8-byte data size | 8-byte index offset |
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Header {
    pub characteristics: u128,
    pub data_offset: u64,
    pub data_size: u64,
    pub index_offset: u64,
}

impl Header {
    pub fn write_bytes<W: Write>(&self, mut w: W) -> Result<usize> {
        let mut bytes = 0;
        bytes += w.write(&encode_varint_u128_exact(self.characteristics))?;
        bytes += w.write(&encode_varint_u64_exact(self.data_offset))?;
        bytes += w.write(&encode_varint_u64_exact(self.data_size))?;
        bytes += w.write(&encode_varint_u64_exact(self.index_offset))?;
        assert_eq!(bytes, HEADER_SIZE);
        // Flush
        w.flush()?;
        Ok(bytes)
    }

    pub fn read_bytes<R: Read>(mut r: R) -> Result<Self> {
        let characteristics = read_varint_u128_exact(&mut r)?;

        assert_eq!(characteristics, 0);

        Ok(Self {
            characteristics,
            data_offset: read_varint_u64_exact(&mut r)?,
            data_size: read_varint_u64_exact(&mut r)?,
            index_offset: read_varint_u64_exact(&mut r)?,
        })
    }

    pub fn to_bytes(self) -> Result<Vec<u8>> {
        let mut header_bytes: Vec<u8> = Vec::new();
        self.write_bytes(&mut header_bytes)?;
        Ok(header_bytes)
    }
}

impl Serialize for Header {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_bytes().unwrap().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Header {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let header_bytes = <Vec<u8>>::deserialize(deserializer)?;
        let new_header = Self::read_bytes(header_bytes.as_slice()).unwrap();
        Ok(new_header)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        types::blockstore::car::carv2::{header::HEADER_SIZE, PRAGMA, PRAGMA_SIZE},
        utils::test::car_setup,
    };

    use super::Header;
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::File,
        io::{Cursor, Seek, Write},
        path::Path,
    };

    #[test]
    fn read_write_bytes() -> Result<()> {
        let header = Header {
            characteristics: 0,
            data_offset: 50,
            data_size: 50,
            index_offset: 0,
        };

        let mut header_bytes: Vec<u8> = Vec::new();
        header.write_bytes(&mut header_bytes)?;

        let header_cursor = Cursor::new(header_bytes);
        let new_header = Header::read_bytes(header_cursor)?;
        assert_eq!(header, new_header);
        Ok(())
    }

    #[test]
    #[serial]
    fn read_disk() -> Result<()> {
        let car_path = car_setup(2, "basic", "read_disk")?;
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
    fn from_scratch() -> Result<()> {
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
        let bytes = header.write_bytes(&mut file)?;
        assert_eq!(bytes, HEADER_SIZE);
        Ok(())
    }
}
