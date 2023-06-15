// Modules
pub mod carv2blockstore;
pub(crate) mod v2header;
pub(crate) mod v2index;

// Code
use self::{v2header::V2Header, v2index::V2Index};
use crate::types::blockstore::car::carv1::{v1block::V1Block, CarV1};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::libipld::Cid;

// | 11-byte fixed pragma | 40-byte header | optional padding | CARv1 data payload | optional padding | optional index payload |
pub(crate) const V2_PRAGMA_SIZE: usize = 11;
pub(crate) const V2_PH_SIZE: u64 = 41;

// This is the fixed file signature associated with the CARV2 file format
pub(crate) const V2_PRAGMA: [u8; V2_PRAGMA_SIZE] = [
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct CarV2 {
    header: RefCell<V2Header>,
    carv1: CarV1,
    index: Option<V2Index>,
}

impl CarV2 {
    /// Load in the CARv2
    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
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

    pub(crate) fn get_block<R: Read + Seek>(&self, cid: &Cid, mut r: R) -> Result<V1Block> {
        let block_offset = self.carv1.index.get_offset(cid)?;
        r.seek(SeekFrom::Start(block_offset))?;
        V1Block::read_bytes(&mut r)
    }

    pub(crate) fn put_block<W: Write + Seek>(&self, block: &V1Block, mut w: W) -> Result<()> {
        // Move to the end
        w.seek(SeekFrom::End(0))?;
        // Insert current offset before bytes are written
        self.carv1
            .index
            .insert_offset(&block.cid, w.stream_position()?);
        // Write the bytes
        let bytes_written = block.write_bytes(&mut w)?;
        // Update the data size
        self.header.borrow_mut().data_size += bytes_written as u64;
        // Return Ok
        Ok(())
    }

    pub(crate) fn initialize<W: Write + Seek>(mut w: W) -> Result<()> {
        // Move to start
        w.seek(SeekFrom::Start(0))?;
        // Write pragma
        w.write_all(&V2_PRAGMA)?;
        // Write new header
        V2Header::default().write_bytes(&mut w)?;
        // Initialize a CARv1 inside the CARv2
        CarV1::initialize(&mut w)?;
        // Return Ok
        Ok(())
    }

    pub(crate) fn get_all_cids(&self) -> Vec<Cid> {
        self.carv1.get_all_cids()
    }

    fn read_to_v1<R: Read + Seek>(&self, mut r: R) -> Result<()> {
        // Skip past the Pragma and Header on the reader
        r.seek(SeekFrom::Start(V2_PH_SIZE))?;
        Ok(())
    }

    fn write_to_v1<W: Write + Seek>(&self, mut w: W) -> Result<()> {
        // Write the pragma and header into the writer
        w.seek(SeekFrom::Start(0))?;
        w.write_all(&V2_PRAGMA)?;
        // Write the header
        self.header.borrow().clone().write_bytes(&mut w)?;
        Ok(())
    }

    fn update_data_size<X: Seek>(&self, mut x: X) -> Result<()> {
        // Update the data size
        let v1_end = x.stream_len()?;
        // Update the data size
        self.header.borrow_mut().data_size = v1_end - V2_PH_SIZE;
        Ok(())
    }

    pub(crate) fn insert_root<R: Read + Seek, W: Write + Seek>(
        &self,
        root: &Cid,
        mut r: R,
        mut w: W,
    ) -> Result<()> {
        // Read up to the CARv1
        self.read_to_v1(&mut r)?;
        // Write up to the CARv1
        self.write_to_v1(&mut w)?;

        // Insert the root
        self.carv1.insert_root(root, &mut r, &mut w)?;

        // The writer now contains the fully modified CARv1
        self.update_data_size(&mut w)
    }

    pub(crate) fn empty_roots<R: Read + Seek, W: Write + Seek>(
        &self,
        mut r: R,
        mut w: W,
    ) -> Result<()> {
        // Read up to the CARv1
        self.read_to_v1(&mut r)?;
        // Write up to the CARv1
        self.write_to_v1(&mut w)?;
        // Insert the root
        self.carv1.empty_roots(&mut r, &mut w)?;
        // The writer now contains the fully modified CARv1
        self.update_data_size(&mut w)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::{fs::File, io::BufReader, path::Path, str::FromStr, vec};
    use wnfs::libipld::Cid;

    use crate::types::blockstore::car::carv2::CarV2;

    #[test]
    fn from_disk_basic() -> Result<()> {
        let car_path = Path::new("car-fixtures").join("carv2-basic.car");
        let mut file = BufReader::new(File::open(car_path)?);
        let carv2 = CarV2::read_bytes(&mut file)?;
        // Assert that this index was in an unrecognized format
        assert_eq!(carv2.index, None);

        // Assert version is correct
        assert_eq!(&carv2.carv1.header.version, &1);

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
        assert_eq!(&carv2.carv1.header.roots.borrow().clone(), &expected_roots);

        // Ok
        Ok(())
    }
}
