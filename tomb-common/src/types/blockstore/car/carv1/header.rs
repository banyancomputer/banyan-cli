use crate::types::blockstore::car::{
    error::CarError,
    varint::{encode_varint_u64, read_varint_u64},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::{Read, Seek, Write},
};
use wnfs::{
    common::dagcbor,
    libipld::{Cid, Ipld},
};

// | 16-byte varint | n-byte DAG CBOR |
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) struct Header {
    pub version: u64,
    pub roots: RefCell<Vec<Cid>>,
}

impl Header {
    pub fn write_bytes<W: Write>(&self, mut w: W) -> Result<()> {
        // Represent as DAGCBOR IPLD
        let ipld_buf = self.to_ipld_bytes()?;
        // Tally bytes in this DAGCBOR, encode as u64
        let varint_buf = encode_varint_u64(ipld_buf.len() as u64);
        // Write the varint, then the IPLD
        w.write_all(&varint_buf)?;
        w.write_all(&ipld_buf)?;
        Ok(())
    }

    pub fn read_bytes<R: Read + Seek>(mut r: R) -> Result<Self> {
        // Determine the length of the remaining IPLD bytes
        let ipld_len = read_varint_u64(&mut r)?;
        // Allocate that space
        let mut ipld_buf: Vec<u8> = vec![0; ipld_len as usize];
        // Read that IPLD in as DAGCBOR bytes
        r.read_exact(&mut ipld_buf)?;
        // Reconstruct this object from those IPLD bytes
        Self::from_ipld_bytes(&ipld_buf)
    }

    /// Transforms a DAGCBOR encoded byte vector of the IPLD representation specified by CARv1 into this object
    pub fn from_ipld_bytes(bytes: &[u8]) -> Result<Self> {
        let ipld: Ipld = dagcbor::decode(bytes)?;
        // If the IPLD is a true map and the correct keys exist within it
        if let Ipld::Map(map) = ipld &&
            let Some(Ipld::Integer(int)) = map.get("version") &&
            let Some(Ipld::List(roots_ipld)) = map.get("roots") {
            // Helper function for interpreting a given Cid as a Link
            fn ipld_to_cid(ipld: &Ipld) -> Result<Cid, CarError> {
                if let Ipld::Link(cid) = ipld {
                    Ok(*cid)
                } else {
                    Err(CarError::MalformedV1Header)
                }
            }
            // Interpret all of the roots as CIDs
            let roots = roots_ipld.iter().map(ipld_to_cid).collect::<Result<Vec<Cid>, CarError>>()?;

            // Return Ok with new Self
            Ok(Self {
                version: *int as u64,
                roots: RefCell::new(roots),
            })
        } else {
            Err(CarError::MalformedV1Header.into())
        }
    }

    /// Transforms this object into a DAGCBOR encoded byte vector of the IPLD representation specified by CARv1
    pub fn to_ipld_bytes(&self) -> Result<Vec<u8>> {
        let mut map = BTreeMap::new();
        map.insert("version".to_string(), Ipld::Integer(self.version as i128));
        // Represent the root CIDs as IPLD Links
        let ipld_roots: Vec<Ipld> = self
            .roots
            .borrow()
            .iter()
            .map(|&root| Ipld::Link(root))
            .collect();
        // Insert the roots into the map
        map.insert("roots".to_string(), Ipld::List(ipld_roots));
        // Construct the final IPLD
        let ipld = Ipld::Map(map);
        dagcbor::encode(&ipld)
    }
}

impl Header {
    pub(crate) fn default(version: u64) -> Self {
        Self {
            version,
            roots: RefCell::new(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Header;
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::File,
        io::{BufReader, Cursor},
        path::Path,
        str::FromStr,
        vec,
    };
    use wnfs::libipld::Cid;

    #[test]
    fn read_write_bytes() -> Result<()> {
        // Construct a Header
        let header = Header::default(1);
        // Write the header into a buffer
        let mut header_bytes: Vec<u8> = Vec::new();
        header.write_bytes(&mut header_bytes)?;

        // Reconstruct the header from this buffer
        let header_cursor = Cursor::new(header_bytes);
        let new_header = Header::read_bytes(header_cursor)?;

        // Assert equality
        assert_eq!(header, new_header);
        Ok(())
    }

    #[test]
    #[serial]
    fn read_disk() -> Result<()> {
        let car_path = Path::new("car-fixtures").join("carv1-basic.car");
        // Open the CARv1
        let mut file = BufReader::new(File::open(car_path)?);
        // Read the header
        let header = Header::read_bytes(&mut file)?;
        // Assert version is correct
        assert_eq!(header.version, 1);
        // Construct a vector of the roots we're expecting to find
        let expected_roots = vec![
            Cid::from_str("bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm")?,
            Cid::from_str("bafyreidj5idub6mapiupjwjsyyxhyhedxycv4vihfsicm2vt46o7morwlm")?,
        ];
        // Assert that the roots loaded match the roots expected in this file
        assert_eq!(header.roots.borrow().clone(), expected_roots);
        // Return Ok
        Ok(())
    }

    #[test]
    fn modify_roots() -> Result<()> {
        // Construct a Header
        let header = Header::default(1);
        {
            let mut roots = header.roots.borrow_mut();
            roots.push(Cid::default());
        }

        // Write the header into a buffer
        let mut header_bytes: Vec<u8> = Vec::new();
        header.write_bytes(&mut header_bytes)?;

        // Reconstruct the header from this buffer
        let header_cursor = Cursor::new(header_bytes);
        let new_header = Header::read_bytes(header_cursor)?;

        // Assert equality
        assert_eq!(header, new_header);
        Ok(())
    }
}