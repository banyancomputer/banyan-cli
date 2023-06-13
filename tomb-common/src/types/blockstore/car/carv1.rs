use std::io::{Read, SeekFrom, Seek, Write};
use anyhow::Result;
use super::{v1header::V1Header, v1block::V1Block};

#[derive(Debug)]
pub struct CarV1 {
    header: V1Header,
    payload: Vec<V1Block>
}

impl CarV1 {
    /// Read an entire CARv2 Payload at once
    fn read_all_blocks<R: Read + Seek>(
        mut r: R,
    ) -> Result<Vec<V1Block>> {
        let mut blocks: Vec<V1Block> = Vec::new();
        println!("carv2_read_all_blocks: stream_position {}", r.stream_position()?);
        let mut potential_block: Option<V1Block> = V1Block::read_bytes(&mut r).ok();
        while let Some(block) = potential_block {
            println!("Loaded new block: {:?}", &block);
            // Append the new block
            blocks.push(block);
            // Try again
            potential_block = V1Block::read_bytes(&mut r).ok();
        }
        Ok(blocks)
    }

    /// Write an entire CARv1 Payload at once
    fn write_all_blocks<W: Write + Seek>(&self, data_offset: u64, data_size: u64, mut w: W) -> Result<usize> {
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

    pub fn write_bytes<W: Write + Seek>(&self, data_offset: u64, data_size: u64, mut w: W) -> Result<usize> {
        self.write_all_blocks(data_offset, data_size, &mut w)
    }

    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Read the header
        let header = V1Header::read_bytes(&mut r)?;
        println!("carv1_read_bytes: header {:?}", header);
        
        let payload = Self::read_all_blocks(&mut r)?;
        Ok(Self {
            header,
            payload
        })
    }
    
}


#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::{fs::File, io::BufReader, path::PathBuf};
    use crate::types::blockstore::car::carv1::CarV1;

    #[test]
    fn from_disk() -> Result<()> {
        // let car_path = PathBuf::from("src").join(path)
        let mut file = BufReader::new(File::open("carv1-basic.car")?);
        let car = CarV1::read_bytes(&mut file)?;

        println!("loaded carfile: {:?}", car);

        Ok(())
    }

    // 63a265726f6f747382d82a582500
    // 01711220f88bc853804cf294fe417e4fa83028689fcdb1b1592c5102e1474dbc200fab8b - v1 header root (bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm)
    // d82a582500
    // 0171122069ea0740f9807a28f4d932c62e7c1c83be055e55072c90266ab3e79df63a365b - v1 header root (bafyreidj5idub6mapiupjwjsyyxhyhedxycv4vihfsicm2vt46o7morwlm)
    // 6776657273696f6e01
    // 5b - block 0 len = 91, block_len = 55
    // 01711220f88bc853804cf294fe417e4fa83028689fcdb1b1592c5102e1474dbc200fab8b - block 0 cid (bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm)
    // a2646c696e6bd82a582300122002acecc5de2438ea4126a3010ecb1f8a599c8eff22fff1a1dcffe999b27fd3de646e616d6564626c6970 - block 0 data
    // 8301 - block 1 len = 131, block_len = 97
    // 122002acecc5de2438ea4126a3010ecb1f8a599c8eff22fff1a1dcffe999b27fd3de - block 1 cid (QmNX6Tffavsya4xgBi2VJQnSuqy9GsxongxZZ9uZBqp16d)
    // 122e0a2401551220b6fbd675f98e2abd22d4ed29fdc83150fedc48597e92dd1a7a24381d44a274511204626561721804122f0a22122079a982de3c9907953d4d323cee1d0fb1ed8f45f8ef02870c0cb9e09246bd530a12067365636f6e64189501 - block 1 data
    // 28 - block 2 len = 40, block_len = 4
    // 01551220b6fbd675f98e2abd22d4ed29fdc83150fedc48597e92dd1a7a24381d44a27451 - block 2 cid (bafkreifw7plhl6mofk6sfvhnfh64qmkq73oeqwl6sloru6rehaoujituke)
    // 63636363 - block 2 data
    // 8001 - block 3 len = 128, block_len = 94
    // 122079a982de3c9907953d4d323cee1d0fb1ed8f45f8ef02870c0cb9e09246bd530a - block 3 cid (QmWXZxVQ9yZfhQxLD35eDR8LiMRsYtHxYqTFCBbJoiJVys)
    // 122d0a240155122081cc5b17018674b401b42f35ba07bb79e211239c23bffe658da1577e3e6468771203646f671804122d0a221220e7dc486e97e6ebe5cdabab3e392bdad128b6e09acc94bb4e2aa2af7b986d24d0120566697273741833 - block 3 data
    // 28 - block 4 len = 40, block_len = 4
    // 0155122081cc5b17018674b401b42f35ba07bb79e211239c23bffe658da1577e3e646877 - block 4 cid(bafkreiebzrnroamgos2adnbpgw5apo3z4iishhbdx77gldnbk57d4zdio4)
    // 62626262 - block 4 data
    // 51 - block 5 len = 81, block_len = 47
    // 1220e7dc486e97e6ebe5cdabab3e392bdad128b6e09acc94bb4e2aa2af7b986d24d0 - block 5 cid (QmdwjhxpxzcMsR3qUuj7vUL8pbA7MgR3GAxWi2GLHjsKCT)
    // 122d0a240155122061be55a8e2f6b4e172338bddf184d6dbee29c98853e0a0485ecee7f27b9af0b412036361741804 - block 5 data
    // 28 - block 6 len = 40, block_len = 4
    // 0155122061be55a8e2f6b4e172338bddf184d6dbee29c98853e0a0485ecee7f27b9af0b4 - block 6 cid (bafkreidbxzk2ryxwwtqxem4l3xyyjvw35yu4tcct4cqeqxwo47zhxgxqwq)
    // 61616161 - block 6 data
    // 36 - block 7 len = 54, block_len = 18
    // 0171122069ea0740f9807a28f4d932c62e7c1c83be055e55072c90266ab3e79df63a365b - block 7 cid (bafyreidj5idub6mapiupjwjsyyxhyhedxycv4vihfsicm2vt46o7morwlm)
    // a2646c696e6bf6646e616d65656c696d626f - block 7 data
}