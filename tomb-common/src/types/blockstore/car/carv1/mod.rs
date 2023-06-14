// Modules
pub mod carv1blockstore;
pub(crate) mod v1block;
pub(crate) mod v1header;
pub(crate) mod v1index;

// Code
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use wnfs::libipld::Cid;

use self::{v1block::V1Block, v1header::V1Header, v1index::V1Index};

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub(crate) struct CarV1 {
    pub header: V1Header,
    pub index: V1Index,
}

impl CarV1 {
    pub(crate) fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Read the header
        let header = V1Header::read_bytes(&mut r)?;
        // Generate an index
        let index = V1Index::read_bytes(&mut r)?;
        Ok(Self { header, index })
    }

    pub(crate) fn get_block<R: Read + Seek>(&self, cid: &Cid, mut r: R) -> Result<V1Block> {
        let block_offset = self.index.get_offset(cid)?;
        r.seek(SeekFrom::Start(block_offset))?;
        V1Block::read_bytes(&mut r)
    }

    pub(crate) fn put_block<W: Write + Seek>(&self, block: &V1Block, mut w: W) -> Result<()> {
        // Move to the end
        w.seek(SeekFrom::End(0))?;
        // Insert current offset before bytes are written
        self.index.insert_offset(&block.cid, w.stream_position()?);
        // Write the bytes
        block.write_bytes(&mut w)?;
        // Return Ok
        Ok(())
    }

    pub(crate) fn initialize<W: Write + Seek>(mut w: W) -> Result<()> {
        // Write new header
        V1Header::default().write_bytes(&mut w)?;
        // Ok
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::carv1::CarV1;
    use anyhow::Result;
    use std::{fs::File, io::BufReader, path::Path, str::FromStr};
    use wnfs::libipld::Cid;

    #[test]
    fn from_disk_basic() -> Result<()> {
        let car_path = Path::new("car-fixtures").join("carv1-basic.car");
        let mut file = BufReader::new(File::open(car_path)?);
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
}
