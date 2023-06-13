use std::{io::{Write, Read, Seek, self}, collections::BTreeMap};
use anyhow::Result;
use unsigned_varint::encode;
use wnfs::{libipld::{Ipld, prelude::{Encode, Decode}, IpldCodec, ipld, Cid}, common::dagcbor};

use crate::types::blockstore::car::{error::CarDecodeError, varint::read_varint_u64};

use super::varint::encode_varint_u64;

// | 16-byte varint | n-byte DAG CBOR |
#[derive(Debug, PartialEq)]
pub(crate) struct V1Header {
    pub version: u64,
    pub roots: Option<Vec<Cid>>
}

/// CARv1 header structure
///
/// ```nn
/// [-------header---------]
/// [varint][DAG-CBOR block]
/// ```
impl V1Header {
    pub fn write_bytes<W: Write>(&self, mut w: W) -> Result<()> {
        let roots_buf = self.to_ipld_bytes()?;
        let mut varint_buf = encode::u64_buffer();
        let (varint_buf, len) = encode_varint_u64(roots_buf.len() as u64, &mut varint_buf);
        w.write_all(&varint_buf)?;
        w.write_all(&roots_buf)?;
        Ok(())
    }
    
    pub fn read_bytes<R: Read>(mut r: R) -> Result<Self> {
        let (header_len, varint_len) = read_varint_u64(&mut r)?.ok_or(CarDecodeError::InvalidCarV1Header(
            "invalid header varint".to_string(),
        )).unwrap();

        // let header_len = u64::from_le_bytes(varint_buf) as usize;
        println!("header_len: {}, varint_len: {}", header_len, varint_len);
        let mut header_buf: Vec<u8> = Vec::with_capacity(header_len as usize);
        header_buf.resize(header_len as usize, 0);
        r.read_exact(&mut header_buf)?;
        println!("header buf: {:?}", &header_buf);
        Self::from_ipld_bytes(&header_buf)
    }
    pub fn from_ipld_bytes(bytes: &[u8]) -> Result<Self> {
        let ipld: Ipld = dagcbor::decode(bytes)?;
        println!("ipld: {:?}", ipld);
        // If the IPLD is a true map
        let map = if let Ipld::Map(map) = ipld {
            map
        } else { panic!() };

        let roots = match map.get("roots") {
            Some(Ipld::List(roots_ipld)) => {
                let mut roots = Vec::with_capacity(roots_ipld.len());
                for root in roots_ipld {
                    if let Ipld::Link(cid) = root {
                        roots.push(*cid);
                    } else { panic!() }
                }
                Some(roots)
            },
            Some(ipld) => { 
                println!("expected list but found: {:?}", ipld);
                panic!()
            },
            None => None,
        };

        let version = match map.get("version") {
            Some(Ipld::Integer(int)) => *int as u64,
            Some(ipld) => panic!(),
            None => panic!()
        };

        println!("version: {}, roots: {:?}", version, roots);

        Ok(Self {
            version,
            roots
        })
    }

    pub fn to_ipld_bytes(&self) -> Result<Vec<u8>> {
        let mut map = BTreeMap::new();
        map.insert("version".to_string(), Ipld::Integer(self.version as i128));
        if let Some(roots) = &self.roots {
            // Represent the root CIDs as IPLD Links
            let ipld_roots: Vec<Ipld> = roots.iter().map(|&root| Ipld::Link(root)).collect();
            // Insert the roots into the map
            map.insert("roots".to_string(), Ipld::List(ipld_roots));
        }
        // Construct the final IPLD
        let ipld = Ipld::Map(map);
        dagcbor::encode(&ipld)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::{Cursor, BufReader}, fs::File};
    use super::V1Header;
    use anyhow::Result;
    use wnfs::libipld::Cid;

    #[test]
    fn read_write_bytes() -> Result<()> {
        // Construct a V1Header
        let header = V1Header {
            version: 2,
            roots: None
        };
        
        // Write the header into a buffer
        let mut header_bytes: Vec<u8> = Vec::new();
        header.write_bytes(&mut header_bytes)?;

        // Reconstruct the header from this buffer
        let header_cursor = Cursor::new(header_bytes);
        let new_header = V1Header::read_bytes(header_cursor)?;

        // Assert equality
        assert_eq!(header, new_header);
        Ok(())
    }

    #[test]
    fn read_disk() -> Result<()> {
        let mut file = BufReader::new(File::open("carv1-basic.car")?);
        // Read the header
        let header = V1Header::read_bytes(&mut file)?;
        // asserteq!(header.ipld.);
        println!("ipld: {:?}", header.roots);
        Ok(())
    }

    #[test]
    fn decode_carv1_header_basic() -> Result<()> {
        let header_buf = hex::decode("a265726f6f747381d82a58230012205b0995ced69229d26009c53c185a62ea805a339383521edbed1028c4966154486776657273696f6e01").unwrap();
        let cid = Cid::try_from("QmUU2HcUBVSXkfWPUc3WUSeCMrWWeEJTuAgR9uyWBhh9Nf").unwrap();

        assert_eq!(
            V1Header::from_ipld_bytes(&header_buf)?,
            V1Header {
                version: 1,
                roots: Some(vec!(cid))
            }
        );

        Ok(())
    }
}