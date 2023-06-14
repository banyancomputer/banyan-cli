use super::{carv1::CarV1, v2index::V2Index};
use crate::types::blockstore::car::v2header::V2Header;
use anyhow::Result;
use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom, Write},
};

// | 11-byte fixed pragma | 40-byte header | optional padding | CARv1 data payload | optional padding | optional index payload |
pub(crate) const V2_PRAGMA_SIZE: usize = 11;

// This is the fixed file signature associated with the CARV2 file format
pub(crate) const V2_PRAGMA: [u8; V2_PRAGMA_SIZE] = [
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];

#[derive(Debug)]
pub struct CarV2 {
    header: RefCell<V2Header>,
    carv1: CarV1,
    index: Option<V2Index>,
}

impl CarV2 {
    /// Load in the CARv2 all at once
    fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Verify the pragma
        Self::verify_pragma(&mut r)?;
        // Load in the header
        let header = V2Header::read_bytes(&mut r)?;
        // Seek to the data offset
        r.seek(SeekFrom::Start(header.data_offset))?;
        // Load in the CARv1
        let carv1 = CarV1::read_bytes(&mut r)?;
        // Seek to the index offset
        r.seek(SeekFrom::Start(header.index_offset))?;
        // Load the index if one is present
        let index: Option<V2Index> = if header.index_offset != 0 {
            // Load in the index
            V2Index::read_bytes(&mut r)?
        } else {
            None
        };
        // Create the new object
        Ok(Self {
            header: RefCell::new(header),
            carv1,
            index,
        })
    }

    /// Write the CARv2 all at once
    fn write_bytes<W: Write + Seek>(&self, mut w: W) -> Result<usize> {
        let mut bytes = 0;
        // Write the pragma
        bytes += w.write(&V2_PRAGMA)?;
        let header = self.header.borrow();
        // Write the header
        bytes += header.write_bytes(&mut w)?;
        // Write the payload
        bytes += self
            .carv1
            .write_bytes(header.data_offset, header.data_size, &mut w)?;
        // Return Ok with number of bytes written
        Ok(bytes)
    }

    pub(crate) fn verify_pragma<R: Read + Seek>(mut r: R) -> Result<()> {
        // Move to the start of the file
        r.seek(SeekFrom::Start(0))?;
        // Read the pragma
        let mut pragma: [u8; V2_PRAGMA_SIZE] = [0; V2_PRAGMA_SIZE];
        r.read_exact(&mut pragma)?;
        // Ensure correctness
        assert_eq!(pragma, V2_PRAGMA);
        // Return Ok
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::carv2::CarV2;
    use anyhow::Result;
    use std::{fs::File, io::BufReader, str::FromStr, vec};
    use wnfs::libipld::Cid;

    #[test]
    fn from_disk_basic() -> Result<()> {
        let mut file = BufReader::new(File::open("carv2-basic.car")?);
        let carv2 = CarV2::read_bytes(&mut file)?;
        // Assert that this index was in an unrecognized format
        assert_eq!(carv2.index, None);
        // Header tests exist separately, let's just ensure content is correct!
        // In the case of the CARv2, the content is an entire CARv1
        let carv1 = carv2.carv1;
        // Assert version is correct
        assert_eq!(carv1.header.version, 1);
        // Construct a vector of the roots we're expecting to find
        let expected_roots = vec![Cid::from_str(
            "QmfEoLyB5NndqeKieExd1rtJzTduQUPEV8TwAYcUiy3H5Z",
        )?];
        assert_eq!(carv1.header.roots.unwrap(), expected_roots);

        // Load content blocks
        let block0 = carv1.payload.get(0).unwrap();
        let block1 = carv1.payload.get(1).unwrap();
        let block2 = carv1.payload.get(2).unwrap();
        let block3 = carv1.payload.get(3).unwrap();
        let block4 = carv1.payload.get(4).unwrap();

        let block_cids = vec![
            Cid::from_str("QmfEoLyB5NndqeKieExd1rtJzTduQUPEV8TwAYcUiy3H5Z")?,
            Cid::from_str("QmczfirA7VEH7YVvKPTPoU69XM3qY4DC39nnTsWd4K3SkM")?,
            Cid::from_str("Qmcpz2FHJD7VAhg1fxFXdYJKePtkx1BsHuCrAgWVnaHMTE")?,
            Cid::from_str("bafkreifuosuzujyf4i6psbneqtwg2fhplc2wxptc5euspa2gn3bwhnihfu")?,
            Cid::from_str("bafkreifc4hca3inognou377hfhvu2xfchn2ltzi7yu27jkaeujqqqdbjju")?,
        ];

        // Ensure CIDs are matching
        assert_eq!(&block0.cid, block_cids.get(0).unwrap());
        assert_eq!(&block1.cid, block_cids.get(1).unwrap());
        assert_eq!(&block2.cid, block_cids.get(2).unwrap());
        assert_eq!(&block3.cid, block_cids.get(3).unwrap());
        assert_eq!(&block4.cid, block_cids.get(4).unwrap());

        // Ensure content is correct
        assert_eq!(block0.content, hex::decode("122d0a221220d9c0d5376d26f1931f7ad52d7acc00fc1090d2edb0808bf61eeb0a152826f6261204f09f8da418a401")?);
        assert_eq!(block1.content, hex::decode("12310a221220d745b7757f5b4593eeab7820306c7bc64eb496a7410a0d07df7a34ffec4b97f1120962617272656c657965183a122e0a2401551220a2e1c40da1ae335d4dffe729eb4d5ca23b74b9e51fc535f4a804a261080c294d1204f09f90a11807")?);
        assert_eq!(block2.content, hex::decode("12340a2401551220b474a99a2705e23cf905a484ec6d14ef58b56bbe62e9292783466ec363b5072d120a666973686d6f6e6765721804")?);
        assert_eq!(block3.content, hex::decode("66697368")?);
        assert_eq!(block4.content, hex::decode("6c6f6273746572")?);

        // Ok
        Ok(())
    }
}
