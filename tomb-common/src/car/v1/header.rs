use crate::car::{
    error::CARError,
    varint::{encode_varint_u64, read_varint_u64},
};
use crate::traits::streamable::Streamable;
use anyhow::Result;
use libipld::{Cid, Ipld};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::{Read, Seek, Write},
};

/// CARv1 Header
/// | 16-byte varint | n-byte DAG CBOR |
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Header {
    /// The version of the CAR (1 or 2)
    pub version: u64,
    /// The deserialized IPLD encoding the roots of the filesystem
    pub roots: RefCell<Vec<Cid>>,
}

impl Header {
    /// Transforms a DAGCBOR encoded byte vector of the IPLD representation specified by CARv1 into this object
    pub fn from_ipld_bytes(bytes: &[u8]) -> Result<Self> {
        // If the IPLD is a true map and the correct keys exist within it
        if let Ok(ipld) = serde_ipld_dagcbor::from_slice(bytes) &&
            let Ipld::Map(map) = ipld &&
            let Some(Ipld::Integer(int)) = map.get("version") &&
            let Some(Ipld::List(roots_ipld)) = map.get("roots") {
            // Helper function for interpreting a given Cid as a Link
            fn ipld_to_cid(ipld: &Ipld) -> Result<Cid, CARError> {
                if let Ipld::Link(cid) = ipld {
                    Ok(*cid)
                } else {
                    Err(CARError::V1Header)
                }
            }
            // Interpret all of the roots as CIDs
            let roots = roots_ipld.iter().map(ipld_to_cid).collect::<Result<Vec<Cid>, CARError>>()?;

            // Return Ok with new Self
            Ok(Self {
                version: *int as u64,
                roots: RefCell::new(roots),
            })
        } else {
            Err(CARError::V1Header.into())
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

        // Represent ipld as bytes
        serde_ipld_dagcbor::to_vec(&ipld).map_err(anyhow::Error::new)
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

impl Streamable for Header {
    /// Write a Header to a byte stream
    fn write_bytes<W: Write>(&self, w: &mut W) -> Result<()> {
        // Represent as DAGCBOR IPLD
        let ipld_buf = self.to_ipld_bytes()?;
        // Tally bytes in this DAGCBOR, encode as u64
        let varint_buf = encode_varint_u64(ipld_buf.len() as u64);
        // Write the varint, then the IPLD
        w.write_all(&varint_buf)?;
        w.write_all(&ipld_buf)?;
        Ok(())
    }

    /// Read a Header from a byte stream
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        // Determine the length of the remaining IPLD bytes
        let ipld_len = read_varint_u64(r)?;
        // Allocate that space
        let mut ipld_buf: Vec<u8> = vec![0; ipld_len as usize];
        // Read that IPLD in as DAGCBOR bytes
        r.read_exact(&mut ipld_buf)?;
        // Reconstruct this object from those IPLD bytes
        Self::from_ipld_bytes(&ipld_buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use super::Header;
    use anyhow::Result;
    use libipld::Cid;
    use serial_test::serial;
    use std::{
        cell::RefCell,
        fs::File,
        io::{BufReader, Cursor},
        path::Path,
        str::FromStr,
        vec,
    };

    #[test]
    #[serial]
    fn read_disk() -> Result<()> {
        let car_path = Path::new("..").join("car-fixtures").join("carv1-basic.car");
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
    fn output() -> Result<()> {
        let cid1 = Cid::from_str("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")?;
        let cid2 = Cid::from_str("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")?;

        let header = Header {
            version: 1,
            roots: RefCell::new(vec![cid1, cid2]),
        };

        let mut bytes = Cursor::new(<Vec<u8>>::new());
        header.write_bytes(&mut bytes)?;
        Ok(())
    }

    crate::utils::tests::streamable_tests! {
        Header:
        v1header: {
            let header = Header::default(1);
            {
                let mut roots = header.roots.borrow_mut();
                roots.push(Cid::default());
            }
            header
        },
    }
}
