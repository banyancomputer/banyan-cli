use crate::{
    car::{error::CarError, Streamable},
    utils::varint::{encode_varint_u64, read_varint_u64},
};

use async_trait::async_trait;

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, Write},
};
use tokio::sync::RwLock;

use wnfs::{
    common::dagcbor,
    libipld::{Cid, Ipld},
};

/// CARv1 Header
/// | 16-byte varint | n-byte DAG CBOR |
#[derive(Debug)]
pub struct Header {
    /// The version of the CAR (1 or 2)
    pub version: u64,
    /// The deserialized IPLD encoding the roots of the filesystem
    pub roots: RwLock<Vec<Cid>>,
}

impl Clone for Header {
    fn clone(&self) -> Self {
        Header {
            version: self.version,
            roots: RwLock::new(futures::executor::block_on(self.roots.read()).clone()),
        }
    }
}

impl PartialEq for Header {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && *futures::executor::block_on(self.roots.read())
                == *futures::executor::block_on(other.roots.read())
    }
}

impl Serialize for Header {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Header", 2)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("roots", &*futures::executor::block_on(self.roots.read()))?;
        state.end()
    }
}
impl<'de> Deserialize<'de> for Header {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct HeaderData {
            version: u64,
            roots: Vec<Cid>,
        }

        let HeaderData { version, roots } = HeaderData::deserialize(deserializer)?;
        Ok(Header {
            version,
            roots: RwLock::new(roots),
        })
    }
}

impl Header {
    /// Transforms a DAGCBOR encoded byte vector of the IPLD representation specified by CARv1 into this object
    pub fn from_ipld_bytes(bytes: &[u8]) -> Result<Self, CarError> {
        // If the IPLD is a true map and the correct keys exist within it
        let Ok(ipld) = dagcbor::decode(bytes) else {
            return Err(CarError::v1_header());
        };
        let Ipld::Map(map) = ipld else {
            return Err(CarError::v1_header());
        };

        let Some(Ipld::Integer(int)) = map.get("version") else {
            return Err(CarError::v1_header());
        };

        let Some(Ipld::List(roots_ipld)) = map.get("roots") else {
            return Err(CarError::v1_header());
        };

        // Helper function for interpreting a given Cid as a Link
        fn ipld_to_cid(ipld: &Ipld) -> Result<Cid, CarError> {
            if let Ipld::Link(cid) = ipld {
                Ok(*cid)
            } else {
                Err(CarError::v1_header())
            }
        }
        // Interpret all of the roots as CIDs
        let roots = roots_ipld
            .iter()
            .map(ipld_to_cid)
            .collect::<Result<Vec<Cid>, CarError>>()?;

        // Return Ok with new Self
        Ok(Self {
            version: *int as u64,
            roots: RwLock::new(roots),
        })
    }

    /// Transforms this object into a DAGCBOR encoded byte vector of the IPLD representation specified by CARv1
    pub async fn to_ipld_bytes(&self) -> Result<Vec<u8>, CarError> {
        let mut map = BTreeMap::new();
        map.insert("version".to_string(), Ipld::Integer(self.version as i128));
        // Represent the root CIDs as IPLD Links
        let ipld_roots: Vec<Ipld> = self
            .roots
            .read()
            .await
            .iter()
            .map(|&root| Ipld::Link(root))
            .collect();
        // Insert the roots into the map
        map.insert("roots".to_string(), Ipld::List(ipld_roots));
        // Construct the final IPLD
        let ipld = Ipld::Map(map);
        dagcbor::encode(&ipld).map_err(|_| CarError::v1_header())
    }
}

impl Header {
    pub(crate) fn default(version: u64) -> Self {
        Self {
            version,
            roots: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Streamable for Header {
    type StreamError = CarError;

    /// Write a Header to a byte stream
    async fn write_bytes<W: Write + Send>(&self, w: &mut W) -> Result<(), Self::StreamError> {
        // Represent as DAGCBOR IPLD
        let ipld_buf = self.to_ipld_bytes().await?;
        // Tally bytes in this DAGCBOR, encode as u64
        let varint_buf = encode_varint_u64(ipld_buf.len() as u64);
        // Write the varint, then the IPLD
        w.write_all(&varint_buf)?;
        w.write_all(&ipld_buf)?;
        Ok(())
    }

    /// Read a Header from a byte stream
    async fn read_bytes<R: Read + Seek + Send>(r: &mut R) -> Result<Self, Self::StreamError> {
        // Determine the length of the remaining IPLD bytes
        let ipld_len = read_varint_u64(r)?;
        // Allocate that space
        let ipld_buf: RwLock<Vec<u8>> = RwLock::new(vec![0; ipld_len as usize]);
        // Read that IPLD in as DAGCBOR bytes
        r.read_exact(&mut ipld_buf.write().await)?;

        let ipld_buf = ipld_buf.into_inner();
        // Assert
        assert_eq!(ipld_buf.len() as u64, ipld_len);
        // Reconstruct this object from those IPLD bytes
        Self::from_ipld_bytes(&ipld_buf)
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
    use super::Header;
    use crate::car::{error::CarError, Streamable};
    use serial_test::serial;
    use std::{
        fs::File,
        io::{BufReader, Cursor},
        path::Path,
        str::FromStr,
        vec,
    };
    use tokio::sync::RwLock;
    use wnfs::libipld::Cid;

    #[tokio::test]
    #[serial]
    async fn read_disk() -> Result<(), CarError> {
        let car_path = Path::new("car-fixtures").join("carv1-basic.car");
        // Open the CARv1
        let mut file = BufReader::new(File::open(car_path)?);
        // Read the header
        let header = Header::read_bytes(&mut file).await?;
        // Assert version is correct
        assert_eq!(header.version, 1);
        // Construct a vector of the roots we're expecting to find
        let expected_roots = vec![
            Cid::from_str("bafyreihyrpefhacm6kkp4ql6j6udakdit7g3dmkzfriqfykhjw6cad5lrm")?,
            Cid::from_str("bafyreidj5idub6mapiupjwjsyyxhyhedxycv4vihfsicm2vt46o7morwlm")?,
        ];
        // Assert that the roots loaded match the roots expected in this file
        assert_eq!(header.roots.read().await.clone(), expected_roots);
        // Return Ok
        Ok(())
    }

    #[tokio::test]
    async fn output() -> Result<(), CarError> {
        let cid1 = Cid::from_str("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")?;
        let cid2 = Cid::from_str("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")?;

        let header = Header {
            version: 1,
            roots: RwLock::new(vec![cid1, cid2]),
        };

        let mut bytes = Cursor::new(<Vec<u8>>::new());
        header.write_bytes(&mut bytes).await?;
        Ok(())
    }

    crate::car::streamable_tests! {
        <crate::car::v1::Header, crate::car::error::CarError>:
        v1header: {
            let header = crate::car::v1::Header::default(1);
            {
                let mut roots = header.roots.write().await;
                roots.push(wnfs::libipld::Cid::default());
            }
            header
        },
    }
}
