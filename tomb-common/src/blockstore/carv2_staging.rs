use std::io::Cursor;
use blake3::Hasher;
use bytes::{BufMut, Bytes, BytesMut};
use crate::car::{varint::read_varint_u64, Streamable};
use libipld::Cid;

const CAR_HEADER_UPPER_LIMIT: u64 = 16 * 1024 * 1024; // Limit car headers to 16MiB

const CAR_FILE_UPPER_LIMIT: u64 = 32 * 1024 * 1024 * 1024; // We limit individual CAR files to 32GiB
                                                           //
const CARV2_PRAGMA: &[u8] = &[
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];

#[derive(Debug, PartialEq)]
pub struct BlockMeta {
    cid: Cid,
    offset: u64,
    length: u64,
}

impl BlockMeta {
    pub fn cid(&self) -> &Cid {
        &self.cid
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }
}

#[derive(Clone, Debug, PartialEq)]
enum CarState {
    Pragma,      // 11 bytes
    CarV2Header, // 40 bytes
    CarV1Header {
        // variable length, collects roots
        data_start: u64,
        data_end: u64,
        index_start: u64,

        header_length: Option<u64>,
    },
    Block {
        // advances to each block until we reach data_end
        block_start: u64,
        data_end: u64,
        index_start: u64,

        block_length: Option<u64>,
    },
    Indexes {
        index_start: u64,
    }, // once we're in the indexes here we don't care anymore
    Complete,
}

#[derive(Debug)]
pub struct CarReport {
    integrity_hash: String,
    total_size: u64,
}

impl CarReport {
    pub fn integrity_hash(&self) -> &str {
        self.integrity_hash.as_str()
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }
}

#[derive(Debug)]
pub struct StreamingCarAnalyzer {
    buffer: BytesMut,
    state: CarState,
    stream_offset: u64,

    hasher: blake3::Hasher,
}

impl StreamingCarAnalyzer {
    pub fn add_chunk(&mut self, bytes: impl Into<Bytes>) -> Result<(), StreamingCarAnalyzerError> {
        let bytes: &Bytes = &Into::<Bytes>::into(bytes);
        self.exceeds_buffer_limit(bytes.len() as u64)?;

        // Don't bother copying data in once we're done analyzing the contents
        if matches!(self.state, CarState::Indexes { .. } | CarState::Complete) {
            self.stream_offset += bytes.len() as u64;
            return Ok(());
        }

        // todo: there are more states where we can avoid copying here such as
        self.buffer.extend_from_slice(bytes);

        Ok(())
    }

    fn exceeds_buffer_limit(&self, new_bytes: u64) -> Result<(), StreamingCarAnalyzerError> {
        let new_byte_total = self.stream_offset + new_bytes;

        if new_byte_total > CAR_FILE_UPPER_LIMIT {
            return Err(StreamingCarAnalyzerError::MaxCarSizeExceeded(
                new_byte_total,
            ));
        }

        Ok(())
    }

    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
            state: CarState::Pragma,
            stream_offset: 0,

            hasher: blake3::Hasher::new(),
        }
    }

    pub async fn next(&mut self) -> Result<Option<BlockMeta>, StreamingCarAnalyzerError> {
        loop {
            println!("attempting to read state: {:?}", self.state);
            match &mut self.state {
                CarState::Pragma => {
                    if self.buffer.len() < 11 {
                        return Ok(None);
                    }

                    let pragma_bytes = self.buffer.split_to(11);
                    self.stream_offset += 11;

                    if &pragma_bytes[..] != CARV2_PRAGMA {
                        return Err(StreamingCarAnalyzerError::PragmaMismatch);
                    }

                    self.state = CarState::CarV2Header;
                }
                CarState::CarV2Header => {
                    if self.buffer.len() < 40 {
                        return Ok(None);
                    }

                    println!("reading CARv2 header from position {} (staging)", self.stream_offset);

                    let _capability_bytes = self.buffer.split_to(16);

                    let data_start_bytes = self.buffer.split_to(8);
                    let data_start = u64::from_le_bytes(
                        data_start_bytes[..].try_into().expect("the exact size"),
                    );

                    let data_size_bytes = self.buffer.split_to(8);
                    let data_size =
                        u64::from_le_bytes(data_size_bytes[..].try_into().expect("the exact size"));

                    let index_start_bytes = self.buffer.split_to(8);
                    let index_start = u64::from_le_bytes(
                        index_start_bytes[..].try_into().expect("the exact size"),
                    );


                    // let header = crate::car::v2::header::Header::read_bytes(&mut Cursor::new(self.buffer.split_to(40))).expect("faaiill");

                    println!("read: data_size: {}", data_size);

                    self.stream_offset += 40;

                    let data_end = data_start + data_size;
                    if data_end > CAR_FILE_UPPER_LIMIT {
                        return Err(StreamingCarAnalyzerError::MaxCarSizeExceeded(data_end));
                    }

                    if index_start > CAR_FILE_UPPER_LIMIT {
                        return Err(StreamingCarAnalyzerError::MaxCarSizeExceeded(index_start));
                    }

                    self.state = CarState::CarV1Header {
                        data_start,
                        data_end,
                        index_start,
                        header_length: None,
                    };
                }
                CarState::CarV1Header {
                    data_start,
                    data_end,
                    index_start,
                    ref mut header_length,
                } => {
                    let data_start = *data_start;

                    // Skip any padding or whitespace until the beginning of our header
                    if self.stream_offset < data_start {
                        let skippable_bytes = data_start - self.stream_offset;
                        let available_bytes = self.buffer.len() as u64;

                        let skipped_byte_count = available_bytes.min(skippable_bytes);
                        let _ = self.buffer.split_to(skipped_byte_count as usize);
                        self.stream_offset += skipped_byte_count;

                        if self.stream_offset != data_start {
                            return Ok(None);
                        }
                    }

                    let hdr_len = match header_length {
                        Some(l) => *l,
                        None => match try_read_varint_u64(&self.buffer[..])? {
                            Some((length, bytes_read)) => {
                                *header_length = Some(length);

                                self.stream_offset += bytes_read;
                                let _ = self.buffer.split_to(bytes_read as usize);

                                length
                            }
                            None => return Ok(None),
                        },
                    };

                    println!("header_length: {hdr_len}");

                    if hdr_len >= CAR_HEADER_UPPER_LIMIT {
                        return Err(StreamingCarAnalyzerError::HeaderSegmentSizeExceeded(
                            hdr_len,
                        ));
                    }

                    // todo: decode dag-cbor inside of block
                    // todo: parse out expected roots and record them... can skip for now

                    // into the blocks!
                    self.state = CarState::Block {
                        block_start: self.stream_offset + hdr_len,
                        data_end: *data_end,
                        index_start: *index_start,

                        block_length: None,
                    };
                    println!("onto the blocks!");
                }
                CarState::Block {
                    block_start,
                    data_end,
                    index_start,
                    ref mut block_length,
                } => {
                    let block_start = *block_start;

                    // Skip any left over data and padding until we reach our goal
                    if self.stream_offset < block_start {
                        let skippable_bytes = block_start - self.stream_offset; // 171 - 72 = 99
                        let available_bytes = self.buffer.len() as u64; //

                        let skipped_byte_count = available_bytes.min(skippable_bytes);
                        let _ = self.buffer.split_to(skipped_byte_count as usize);
                        self.stream_offset += skipped_byte_count;

                        if self.stream_offset != block_start {
                            return Ok(None);
                        }
                    }

                    if block_start == *data_end {
                        self.state = CarState::Indexes {
                            index_start: *index_start,
                        };

                        continue;
                    }

                    println!("read: block varint_start: {}", self.stream_offset);

                    let blk_len = match block_length {
                        Some(bl) => *bl,
                        None => match try_read_varint_u64(&self.buffer[..])? {
                            Some((length, bytes_read)) => {
                                *block_length = Some(length);

                                self.stream_offset += bytes_read;

                                let _ = self.buffer.split_to(bytes_read as usize);

                                length
                            }
                            None => return Ok(None),
                        },
                    };

                    // We would need to pass this through our state if we want to do streaming
                    // parsing on the block contents, but since we don't we can use the current
                    // stream offset as a proxy for "just after the block length" we can avoid
                    // storing it in state.
                    let length_varint_len = self.stream_offset - block_start;

                    println!("read: block cid_start: {}", self.stream_offset);

                    // 64-bytes is the longest reasonable CID we're going to care about it. We're
                    // going to wait until we have that much then try and decode the CID from
                    // there. The edge case here is if the total block length (CID included) is
                    // less than 64-bytes we'll just wait for the entire block. The CID has to be
                    // included and we'll decode it from there just as neatly.
                    let minimum_cid_blocks = blk_len.min(64) as usize;
                    if self.buffer.len() < minimum_cid_blocks {
                        return Ok(None);
                    }

                    let cid = match Cid::read_bytes(&self.buffer[..]) {
                        Ok(cid) => {
                            println!("read: read cid! {}", self.stream_offset);
                            cid
                        },
                        Err(err) => {
                            println!("read: failed to read cid! {}", err);
                            // tracing::error!("uploaded car file contained an invalid CID: {err}");
                            return Err(StreamingCarAnalyzerError::InvalidBlockCid(self.stream_offset));
                        }
                    };
                    let cid_length = cid.encoded_len() as u64;

                    println!("read: block data_start: {}", self.stream_offset);

                    // This might be the end of all data, we'll check once we reach the block_start
                    // offset
                    self.state = CarState::Block {
                        block_start: block_start + length_varint_len + blk_len,
                        data_end: *data_end,
                        index_start: *index_start,
                        block_length: None,
                    };

                    return Ok(Some(BlockMeta {
                        cid,
                        offset: block_start + length_varint_len + cid_length,
                        length: blk_len - cid_length,
                    }));
                }
                CarState::Indexes { index_start } => {
                    // we don't actually care about the indexes right now so I'm going to use this
                    // just as a convenient place to drain our buffer
                    self.stream_offset += self.buffer.len() as u64;
                    self.buffer.clear();

                    // We do want to make sure we at least get to the indexes...
                    if self.stream_offset >= *index_start {
                        self.state = CarState::Complete;
                    }

                    return Ok(None);
                }
                CarState::Complete => return Ok(None),
            }
        }
    }

    pub fn report(self) -> Result<CarReport, StreamingCarAnalyzerError> {
        if !matches!(self.state, CarState::Complete) {
            return Err(StreamingCarAnalyzerError::IncompleteData);
        }

        Ok(CarReport {
            integrity_hash: self.hasher.finalize().to_string(),
            total_size: self.stream_offset,
        })
    }

    pub fn seen_bytes(&self) -> u64 {
        self.stream_offset
    }
}


#[derive(Debug, thiserror::Error)]
pub enum StreamingCarAnalyzerError {
    #[error(
        "received {0} bytes while still decoding the header which exceeds our allowed header sizes"
    )]
    HeaderSegmentSizeExceeded(u64),

    #[error("parser wasn't finished with the data stream before it ended")]
    IncompleteData,

    #[error("CID located at offset {0} was not valid")]
    InvalidBlockCid(u64),

    #[error("received {0} bytes which exceeds our upper limit for an individual CAR upload")]
    MaxCarSizeExceeded(u64),

    #[error("received car file did not have the expected pragma")]
    PragmaMismatch,

    #[error("a varint in the car file was larger than our acceptable value")]
    ValueToLarge,
}
// e encoding, for every 7 bits we add an extra 1
// bit or ceil(64 / 7) + 64 = 74 bits. 74 bits pack into 10 bytes so that is the maximum number of
// bytes we care about.
const U64_MAX_ENCODED_LENGTH: usize = 10;

// fn try_read_varint_u64(buf: &[u8]) -> Result<Option<(u64, u64)>, StreamingCarAnalyzerError> {
//     let mut stream = Cursor::new(buf);

//     match read_varint_u64(&mut stream) {
//         Ok(value) => {
//             if stream.position() > 0 {
//                 Ok(Some((value, stream.position())))
//             }
//             else {
//                 Ok(None)
//             }

//         },
//         Err(err) =>{
//             if buf.len() < U64_MAX_ENCODED_LENGTH {
//                 return Ok(None);
//             }
//             println!("error: {:?}", err);
//             panic!("o no");
//         },
//     }
// }

fn try_read_varint_u64(buf: &[u8]) -> Result<Option<(u64, u64)>, StreamingCarAnalyzerError> {
    let mut result: u64 = 0;

    // The length check doesn't make this loop very efficient but it should be sufficient for now
    for i in 0..U64_MAX_ENCODED_LENGTH {
        // We don't have enough data
        if buf.len() <= i {
            return Ok(None);
        }

        result |= u64::from(buf[i] & 0x7F) << (i * 7);

        // The leftmost bit being cleared indicates we're done with the decoding
        if buf[i] & 0x80 == 0 {
            let encoded_length = i + 1;
            return Ok(Some((result, encoded_length as u64)));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Seek};
    use std::io::Read;

    use crate::car::v1::block::Block;
    use crate::car::v2::CarV2;
    use crate::car::v2::fixture::test::build_full_car;
    use crate::utils::tests::car_test_setup;

    use super::*;
    use anyhow::Result;
    use libipld::multihash::{Code, MultihashDigest};
    use serial_test::serial;
    use wnfs::libipld::IpldCodec;

    fn encode_v2_header(chars: u128, data_offset: u64, data_size: u64, index_offset: u64) -> Bytes {
        let mut buffer = BytesMut::new();

        buffer.extend_from_slice(&chars.to_le_bytes());
        buffer.extend_from_slice(&data_offset.to_le_bytes());
        buffer.extend_from_slice(&data_size.to_le_bytes());
        buffer.extend_from_slice(&index_offset.to_le_bytes());

        assert_eq!(buffer.len(), 40);
        buffer.freeze()
    }

    fn encode_varint_u64(mut val: u64) -> Bytes {
        let mut bytes = BytesMut::new();

        loop {
            let mut current_byte = (val & 0b0111_1111) as u8; // take the lower 7 bits
            val >>= 7; // shift them away

            if val > 0 {
                // This isn't the last byte, set the high bit
                current_byte |= 0b1000_0000;
            }

            // append our current byte to the byte list (this is doing the MSB to LSB conversion)
            bytes.put_u8(current_byte);

            // if nothing is remaining drop out of the loop
            if val == 0 {
                break;
            }
        }

        bytes.freeze()
    }

    #[test]
    fn test_varint_roundtrip() {
        let reference_numbers: &[(u64, u64)] =
            &[(0, 1), (100, 1), (1000, 2), (10000, 2), (100000000, 4)];
        for (num, size) in reference_numbers.iter() {
            let encoded_version = encode_varint_u64(*num).to_vec();
            assert_eq!(
                try_read_varint_u64(encoded_version.as_slice()).unwrap(),
                Some((*num, *size))
            );
        }

        assert_eq!(try_read_varint_u64(&[]).unwrap(), None);
    }

    #[tokio::test]
    async fn test_streaming_lifecycle() {
        let mut sca = StreamingCarAnalyzer::new();
        assert_eq!(sca.state, CarState::Pragma);

        // No data shouldn't transition
        assert!(sca.next().await.expect("still valid").is_none());
        assert_eq!(sca.stream_offset, 0);
        assert_eq!(sca.state, CarState::Pragma);
        assert_eq!(sca.buffer.len(), 0);

        // Some data but still not enough, shouldn't transition
        sca.add_chunk(&CARV2_PRAGMA[0..4]).unwrap();
        assert!(sca.next().await.expect("still valid").is_none()); // no blocks yet
        assert_eq!(sca.stream_offset, 0);
        assert_eq!(sca.state, CarState::Pragma);
        assert_eq!(sca.buffer.len(), 4);

        // The rest of the Pragma should do the trick
        sca.add_chunk(&CARV2_PRAGMA[4..]).unwrap();
        assert!(sca.next().await.expect("still valid").is_none()); // no blocks yet
        assert_eq!(sca.stream_offset, 11);
        assert_eq!(sca.state, CarState::CarV2Header);
        assert_eq!(sca.buffer.len(), 0);

        // data size is missing the size of the CID...
        let data_length = 1 + 99 + 1 + 36 + 57;
        let mut v2_header = encode_v2_header(0, 71, data_length, 285);

        // Some data but still not enough, shouldn't transition
        sca.add_chunk(v2_header.split_to(17)).unwrap();
        assert!(sca.next().await.expect("still valid").is_none()); // no blocks yet
        assert_eq!(sca.stream_offset, 11);
        assert_eq!(sca.state, CarState::CarV2Header);
        assert_eq!(sca.buffer.len(), 17);

        // The rest of the header
        sca.add_chunk(v2_header).unwrap();
        assert_eq!(sca.buffer.len(), 40);

        let car_v1_header = CarState::CarV1Header {
            data_start: 71,
            data_end: 71 + data_length,
            index_start: 285,
            header_length: None,
        };

        assert!(sca.next().await.expect("still valid").is_none()); // no blocks yet
        assert_eq!(sca.stream_offset, 51);
        assert_eq!(sca.state, car_v1_header); // this is taking one too many bytes...
        assert_eq!(sca.buffer.len(), 0);

        // We should automatically consume all of the padding and take us up to our first byte in
        // the car v1 header, which should also match the data_start value
        sca.add_chunk([0u8; 20].as_slice()).unwrap();
        assert_eq!(sca.buffer.len(), 20);

        assert!(sca.next().await.expect("still valid").is_none());
        assert_eq!(sca.stream_offset, 71);
        assert_eq!(sca.state, car_v1_header);
        assert_eq!(sca.buffer.len(), 0);

        // We're next going to advance our state until we can read the length of the header, we
        // don't care about the contents of the header so we're going to immediately start looking
        // for the first block before we get past the header contents.
        sca.add_chunk(encode_varint_u64(99)).unwrap();
        assert_eq!(sca.buffer.len(), 1); // 1 byte

        // The parser will now know how long the header is, there is an intermediate state that is
        // hidden due to the loop where we have the length in the CarV1Header state, but since
        // we're not doing anything with that data we jump immediately to the first block...

        assert!(sca.next().await.expect("still valid").is_none());
        assert_eq!(sca.stream_offset, 72);

        let first_block = CarState::Block {
            block_start: 171,
            data_end: 71 + data_length,
            index_start: 71 + data_length + 20,
            block_length: None,
        };
        assert_eq!(sca.state, first_block);
        assert_eq!(sca.buffer.len(), 0);

        // Add in all the bytes that make up our header and advance to the start of our first block
        sca.add_chunk([0u8; 99].as_slice()).unwrap();
        assert_eq!(sca.buffer.len(), 99);

        assert!(sca.next().await.expect("still valid").is_none());
        assert_eq!(sca.stream_offset, 171);
        assert_eq!(sca.state, first_block);
        assert_eq!(sca.buffer.len(), 0);

        let block_data = b"some internal blockity block data, this is real I promise";
        // we'll use the RAW codec for our data...
        let block_cid = Cid::new_v1(0x55, Code::Sha3_256.digest(block_data));

        let inner_block_size = (block_data.len() + block_cid.encoded_len()) as u64;
        let length_bytes = encode_varint_u64(inner_block_size);

        sca.add_chunk(length_bytes).unwrap();
        sca.add_chunk(block_cid.to_bytes()).unwrap();
        sca.add_chunk(block_data.to_vec()).unwrap();

        let next_meta = Some(BlockMeta {
            cid: block_cid,
            offset: 208,
            length: inner_block_size - block_cid.encoded_len() as u64,
        });
        println!("{next_meta:?}");
        assert_eq!(sca.next().await.expect("still valid"), next_meta);
        assert_eq!(
            sca.state,
            CarState::Block {
                block_start: 265,
                data_end: 265,
                index_start: 285,
                block_length: None
            }
        );
        assert_eq!(sca.stream_offset, 172); // we've read the length & CID but haven't advanced the stream
                                            // offset yet

        sca.add_chunk([0u8; 10].as_slice()).unwrap(); // take us into the padding past the data but before the indexes
        assert!(sca.next().await.expect("still valid").is_none()); // we're at the end of the data, this should transition to indexes
        assert_eq!(sca.state, CarState::Indexes { index_start: 285 });
        assert_eq!(sca.stream_offset, 275); // we read right up to the start of the virtual end
                                            // block

        // take us past the index so we can complete
        sca.add_chunk([0u8; 50].as_slice()).unwrap();
        assert!(sca.next().await.expect("still valid").is_none());
        assert_eq!(sca.state, CarState::Complete);

        let report = sca.report().unwrap();
        assert_eq!(report.total_size, 325);
    }

    #[tokio::test]
    #[serial]
    async fn fixture() -> Result<()> {
        let all_data = build_full_car();

        let mut car_stream = StreamingCarAnalyzer::new();

        for chunk in all_data.chunks(23) {
            car_stream.add_chunk(chunk.to_owned())?;
        }

        loop {
            match car_stream.next().await {
                Ok(Some(meta)) => {
                    println!("meta: {:?}", meta);
                },
                Ok(None) => {
                    break;
                },
                Err(_) => {
                    break;
                }
            }
        }
        
        let report = car_stream.report()?;

        println!("report: {:?}", report);

        Ok(())
    }


    #[tokio::test]
    // #[serial]
    async fn carv2_known() -> Result<()> {
        let mut rw = Cursor::new(<Vec<u8>>::new());
        let car = CarV2::new(&mut rw)?;
        let block1 = Block::new([0x55u8; 55].to_vec(), IpldCodec::Raw)?;
        let block2 = Block::new([0x66u8; 66].to_vec(), IpldCodec::Raw)?;

        car.put_block(&block1, &mut rw)?;
        car.put_block(&block2, &mut rw)?;
        car.write_bytes(&mut rw)?;
        // car.set_root(root);/
        let car = CarV2::read_bytes(&mut rw)?;

        println!("the size of car is {}", rw.stream_len()?);

        println!("car2header: {:?}", car.header.borrow().clone());
        println!("car1header: {:?}", car.car.header);

        println!("hex: {}", hex::encode(rw.clone().into_inner().to_vec()));

        let mut car_stream = StreamingCarAnalyzer::new();
        for chunk in rw.into_inner().chunks(20) {
            car_stream.add_chunk(chunk.to_owned())?;
        }

        loop {
            match car_stream.next().await {
                Ok(Some(meta)) => {
                    println!("meta: {:?}", meta);
                },
                Ok(None) => {
                    println!("none!");
                    break;
                },
                Err(err) => {
                    println!("error!: {}", err);
                    break;
                }
            }
        }
        
        let report = car_stream.report()?;

        println!("report: {:?}", report);

        Ok(())
    }
}
