use std::io;

use wnfs::libipld::Cid;

#[derive(Debug)]
pub enum CarDecodeError {
    InvalidCarV1Header(String),
    InvalidCarV2Header(String),
    InvalidMultihash(String),
    InvalidCid(String),
    InvalidBlockHeader(String),
    BlockDigestMismatch(String),
    // UnsupportedHashCode((HashCode, Cid)),
    BlockStartEOF,
    UnsupportedCarVersion { version: u64 },
    IoError(io::Error),
}
