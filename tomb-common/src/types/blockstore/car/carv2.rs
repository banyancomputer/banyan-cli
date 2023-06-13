use super::carv1::CarV1;
use crate::types::blockstore::car::v2header::V2Header;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Seek, Write};

// | 11-byte fixed pragma | 40-byte header | optional padding | CARv1 data payload | optional padding | optional index payload |
pub(crate) const V2_PRAGMA_SIZE: usize = 11;

// This is the fixed file signature associated with the CARV2 file format
pub(crate) const V2_PRAGMA: [u8; V2_PRAGMA_SIZE] = [
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];

#[derive(Debug)]
struct CarV2 {
    header: V2Header,
    carv1: CarV1,
}

impl CarV2 {
    /// Load in the CARv2 all at once
    fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Read the pragma
        let mut pragma: [u8; V2_PRAGMA_SIZE] = [0; V2_PRAGMA_SIZE];
        r.read_exact(&mut pragma)?;
        println!("pragma: {:?}", pragma);
        // Ensure correctness
        assert_eq!(pragma, V2_PRAGMA);
        println!("pragma was correct. continuing");
        // Load in the header
        let header = V2Header::read_bytes(&mut r)?;
        println!("header: {:?}", header);
        // Load in all payload blocks
        let carv1 = CarV1::read_bytes(&mut r)?;
        // Create new object
        Ok(Self { header, carv1 })
    }

    /// Write the CARv2 all at once
    fn write_bytes<W: Write + Seek>(&self, mut w: W) -> Result<usize> {
        let mut bytes = 0;
        // Write the pragma
        bytes += w.write(&V2_PRAGMA)?;
        // Write the header
        bytes += self.header.write_bytes(&mut w)?;
        // Write the payload
        bytes += self
            .carv1
            .write_bytes(self.header.data_offset, self.header.data_size, &mut w)?;
        // Return Ok with number of bytes written
        Ok(bytes)
    }
}

impl Serialize for CarV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        self.write_bytes(&mut buf).unwrap();

        buf.get_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CarV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let buf: Cursor<Vec<u8>> = Cursor::new(Vec::<u8>::deserialize(deserializer)?);
        let car = CarV2::read_bytes(buf).unwrap();
        Ok(car)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::{fs::File, io::BufReader, str::FromStr, vec};
    use wnfs::libipld::Cid;

    use crate::types::blockstore::car::carv2::CarV2;

    #[test]
    fn from_disk() -> Result<()> {
        let mut file = BufReader::new(File::open("carv2-basic.car")?);
        let carv2 = CarV2::read_bytes(&mut file)?;
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

        Ok(())
    }

    // 0aa16776657273696f6e02  - v2 pragma
    // 00000000000000000000000000000000  - v2 header characteristics
    // 3300000000000000  - v2 header data_offset
    // c001000000000000  - v2 header data_size
    // f301000000000000  - v2 header index_offset
    // 38a265726f6f747381d82a582300
    // 1220fb16f5083412ef1371d031ed4aa239903d84efdadf1ba3cd678e6475b1a232f8 - v1 header root (QmfEoLyB5NndqeKieExd1rtJzTduQUPEV8TwAYcUiy3H5Z)
    // 6776657273696f6e01
    // 51 - block 0 len = 81, block_len = 47
    // 1220fb16f5083412ef1371d031ed4aa239903d84efdadf1ba3cd678e6475b1a232f8 - block 0 cid (QmfEoLyB5NndqeKieExd1rtJzTduQUPEV8TwAYcUiy3H5Z)
    // 122d0a221220d9c0d5376d26f1931f7ad52d7acc00fc1090d2edb0808bf61eeb0a152826f6261204f09f8da418a401 - block 0 data
    // 8501 -  block 1 len = 133, block_len = 99
    // 1220d9c0d5376d26f1931f7ad52d7acc00fc1090d2edb0808bf61eeb0a152826f626 - block 1 cid (QmczfirA7VEH7YVvKPTPoU69XM3qY4DC39nnTsWd4K3SkM)
    // 12310a221220d745b7757f5b4593eeab7820306c7bc64eb496a7410a0d07df7a34ffec4b97f1120962617272656c657965183a122e0a2401551220a2e1c40da1ae335d4dffe729eb4d5ca23b74b9e51fc535f4a804a261080c294d1204f09f90a11807 - block 1 data
    // 58 - block 2 len = 88, block_len = 54
    // 1220d745b7757f5b4593eeab7820306c7bc64eb496a7410a0d07df7a34ffec4b97f1 - block 2 cid (Qmcpz2FHJD7VAhg1fxFXdYJKePtkx1BsHuCrAgWVnaHMTE)
    // 12340a2401551220b474a99a2705e23cf905a484ec6d14ef58b56bbe62e9292783466ec363b5072d120a666973686d6f6e6765721804 - block 2 data
    // 28 - block 3 len = 40, block_len 4
    // 01551220b474a99a2705e23cf905a484ec6d14ef58b56bbe62e9292783466ec363b5072d - block 3 cid (bafkreifuosuzujyf4i6psbneqtwg2fhplc2wxptc5euspa2gn3bwhnihfu)
    // 66697368 - block 3 data
    // 2b - block 4 len = 43, block_len 7
    // 01551220a2e1c40da1ae335d4dffe729eb4d5ca23b74b9e51fc535f4a804a261080c294d - block 4 cid (bafkreifc4hca3inognou377hfhvu2xfchn2ltzi7yu27jkaeujqqqdbjju)
    // 6c6f6273746572 - block 4 data
    // 0100000028000000c800000000000000a2e1c40da1ae335d4dffe729eb4d5ca23b74b9e51fc535f4a804a261080c294d9401000000000000b474a99a2705e23cf905a484ec6d14ef58b56bbe62e9292783466ec363b5072d6b01000000000000d745b7757f5b4593eeab7820306c7bc64eb496a7410a0d07df7a34ffec4b97f11201000000000000d9c0d5376d26f1931f7ad52d7acc00fc1090d2edb0808bf61eeb0a152826f6268b00000000000000fb16f5083412ef1371d031ed4aa239903d84efdadf1ba3cd678e6475b1a232f83900000000000000
}
