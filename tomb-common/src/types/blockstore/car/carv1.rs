use super::{v1block::V1Block, v1header::V1Header};
use anyhow::Result;
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug)]
pub(crate) struct CarV1 {
    pub header: V1Header,
    pub payload: Vec<V1Block>,
}

impl CarV1 {
    /// Read an entire CARv2 Payload at once
    fn read_all_blocks<R: Read + Seek>(mut r: R) -> Result<Vec<V1Block>> {
        let mut blocks: Vec<V1Block> = Vec::new();
        let mut potential_block: Result<V1Block> = V1Block::read_bytes(&mut r);
        while let Ok(block) = potential_block {
            // Append the new block
            blocks.push(block);
            // Try again
            potential_block = V1Block::read_bytes(&mut r);
        }
        Ok(blocks)
    }

    /// Write an entire CARv1 Payload at once
    fn write_all_blocks<W: Write + Seek>(
        &self,
        data_offset: u64,
        data_size: u64,
        mut w: W,
    ) -> Result<usize> {
        let mut bytes = 0;
        // Start at the data offset
        w.seek(SeekFrom::Start(data_offset))?;
        // For each V1 block
        for block in &self.payload {
            // Write the block bytes into the writer
            bytes += block.write_bytes(&mut w)?;
        }
        // Assert that the correct number of bytes were written
        assert_eq!(bytes as u64, data_size);
        // Return the number of bytes written
        Ok(bytes)
    }

    pub fn write_bytes<W: Write + Seek>(
        &self,
        data_offset: u64,
        data_size: u64,
        mut w: W,
    ) -> Result<usize> {
        self.write_all_blocks(data_offset, data_size, &mut w)
    }

    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Read the header
        let header = V1Header::read_bytes(&mut r)?;
        let payload = Self::read_all_blocks(&mut r)?;
        Ok(Self { header, payload })
    }
}

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::carv1::CarV1;
    use anyhow::Result;
    use std::{fs::File, io::BufReader, str::FromStr};
    use wnfs::libipld::Cid;

    #[test]
    fn from_disk() -> Result<()> {
        let mut file = BufReader::new(File::open("carv1-basic.car")?);
        let car = CarV1::read_bytes(&mut file)?;
        // Header tests exist separately, let's just ensure content is correct!
        // Load content blocks
        let block0 = car.payload.get(0).unwrap();
        let block1 = car.payload.get(1).unwrap();
        let block2 = car.payload.get(2).unwrap();
        let block3 = car.payload.get(3).unwrap();
        let block4 = car.payload.get(4).unwrap();
        let block5 = car.payload.get(5).unwrap();
        let block6 = car.payload.get(6).unwrap();
        let block7 = car.payload.get(7).unwrap();

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

        // Ensure CIDs are matching
        assert_eq!(&block0.cid, block_cids.get(0).unwrap());
        assert_eq!(&block1.cid, block_cids.get(1).unwrap());
        assert_eq!(&block2.cid, block_cids.get(2).unwrap());
        assert_eq!(&block3.cid, block_cids.get(3).unwrap());
        assert_eq!(&block4.cid, block_cids.get(4).unwrap());
        assert_eq!(&block5.cid, block_cids.get(5).unwrap());
        assert_eq!(&block6.cid, block_cids.get(6).unwrap());
        assert_eq!(&block7.cid, block_cids.get(7).unwrap());

        // Ensure content is correct
        assert_eq!(block0.content, hex::decode("a2646c696e6bd82a582300122002acecc5de2438ea4126a3010ecb1f8a599c8eff22fff1a1dcffe999b27fd3de646e616d6564626c6970")?);
        assert_eq!(block1.content, hex::decode("122e0a2401551220b6fbd675f98e2abd22d4ed29fdc83150fedc48597e92dd1a7a24381d44a274511204626561721804122f0a22122079a982de3c9907953d4d323cee1d0fb1ed8f45f8ef02870c0cb9e09246bd530a12067365636f6e64189501")?);
        assert_eq!(block2.content, hex::decode("63636363")?);
        assert_eq!(block3.content, hex::decode("122d0a240155122081cc5b17018674b401b42f35ba07bb79e211239c23bffe658da1577e3e6468771203646f671804122d0a221220e7dc486e97e6ebe5cdabab3e392bdad128b6e09acc94bb4e2aa2af7b986d24d0120566697273741833")?);
        assert_eq!(block4.content, hex::decode("62626262")?);
        assert_eq!(block5.content, hex::decode("122d0a240155122061be55a8e2f6b4e172338bddf184d6dbee29c98853e0a0485ecee7f27b9af0b412036361741804")?);
        assert_eq!(block6.content, hex::decode("61616161")?);
        assert_eq!(block7.content, hex::decode("a2646c696e6bf6646e616d65656c696d626f")?);

        Ok(())
    }
}
