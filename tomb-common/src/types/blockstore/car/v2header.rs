use anyhow::Result;
use std::io::{Read, Write};

const V2_HEADER_SIZE: usize = 40;

// | 16-byte characteristics | 8-byte data offset | 8-byte data size | 8-byte index offset |
#[derive(Debug, PartialEq)]
pub(crate) struct V2Header {
    pub characteristics: u128,
    pub data_offset: u64,
    pub data_size: u64,
    pub index_offset: u64,
}

impl V2Header {
    pub fn write_bytes<W: Write>(&self, mut w: W) -> Result<usize> {
        let mut bytes = 0;
        bytes += w.write(&self.characteristics.to_le_bytes())?;
        bytes += w.write(&self.data_offset.to_le_bytes())?;
        bytes += w.write(&self.data_size.to_le_bytes())?;
        bytes += w.write(&self.index_offset.to_le_bytes())?;
        assert_eq!(bytes, V2_HEADER_SIZE);
        Ok(bytes)
    }

    pub fn read_bytes<R: Read>(mut r: R) -> Result<Self> {
        let mut characteristics_bytes: [u8; 16] = [0; 16];
        let mut data_offset_bytes: [u8; 8] = [0; 8];
        let mut data_size_bytes: [u8; 8] = [0; 8];
        let mut index_offset_bytes: [u8; 8] = [0; 8];

        r.read_exact(&mut characteristics_bytes)?;
        r.read_exact(&mut data_offset_bytes)?;
        r.read_exact(&mut data_size_bytes)?;
        r.read_exact(&mut index_offset_bytes)?;

        Ok(Self {
            characteristics: u128::from_le_bytes(characteristics_bytes),
            data_offset: u64::from_le_bytes(data_offset_bytes),
            data_size: u64::from_le_bytes(data_size_bytes),
            index_offset: u64::from_le_bytes(index_offset_bytes),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::V2Header;
    use anyhow::Result;
    use crate::types::blockstore::car::{
        carv2::{V2_PRAGMA, V2_PRAGMA_SIZE},
        v2header::V2_HEADER_SIZE,
    };
    use std::{
        fs::{self, File},
        io::{BufReader, BufWriter, Seek, Write},
        path::Path,
    };

    #[test]
    fn read_write_bytes() -> Result<()> {
        let header = V2Header {
            characteristics: 0,
            data_offset: 50,
            data_size: 50,
            index_offset: 0,
        };

        let mut header_bytes: Vec<u8> = Vec::new();
        header.write_bytes(&mut header_bytes)?;
        let new_header = V2Header::read_bytes(header_bytes.as_slice())?;
        assert_eq!(header, new_header);
        Ok(())
    }

    #[test]
    fn read_disk() -> Result<()> {
        let mut file = BufReader::new(File::open("carv2-basic.car")?);
        // Skip the pragma
        file.seek(std::io::SeekFrom::Start(V2_PRAGMA_SIZE as u64))?;
        // Read the header
        let header = V2Header::read_bytes(&mut file)?;
        // Characteristics are 0
        assert_eq!(header.characteristics, 0);
        assert_eq!(header.data_offset, 51);
        assert_eq!(header.data_size, 448);
        assert_eq!(header.index_offset, 499);
        Ok(())
    }

    #[test]
    fn write_disk() -> Result<()> {
        let path = Path::new("carv2-new.car");
        let mut file = BufWriter::new(File::create(path)?);
        // Write the pragma
        file.write_all(&V2_PRAGMA)?;
        // Read the header
        let header = V2Header {
            characteristics: 0,
            data_offset: 50,
            data_size: 50,
            index_offset: 0,
        };
        let bytes = header.write_bytes(&mut file)?;
        assert_eq!(bytes, V2_HEADER_SIZE);
        // Remove the temporary file
        fs::remove_file(path)?;
        Ok(())
    }
}
