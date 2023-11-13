mod test {
    use crate::car::{
        v1::Header,
        v2::{CarV2, HEADER_SIZE, PH_SIZE}, error::CarError,
    };
        use base58::ToBase58;
    use sha2::Digest;
    use std::io::{Cursor, Seek, SeekFrom};
    use std::{cell::RefCell, str::FromStr};
    use wnfs::libipld::Cid;

    /// Quick Specification Reference Links:
    ///
    /// * https://github.com/multiformats/cid
    /// * https://github.com/multiformats/multicodec
    /// * https://github.com/multiformats/multihash
    /// * https://ipld.io/docs/data-model/
    /// * https://ipld.io/specs/advanced-data-layouts/hamt/spec/
    /// * https://ipld.io/specs/codecs/dag-cbor/spec/
    /// * https://ipld.io/specs/transport/car/carv1/
    /// * https://ipld.io/specs/transport/car/carv2/
    /// * https://www.iana.org/assignments/cbor-tags/cbor-tags.xhtml
    /// * https://www.rfc-editor.org/rfc/rfc7049.txt
    /// * https://www.rfc-editor.org/rfc/rfc8949.txt
    ///
    /// All variable integer were calculated by hand. Rust primitive operations were used on the direct
    /// values.

    /// Block 0
    const BLOCK_ZERO_DATA: &[u8] = &[
        0x00, // Multibase: Identity Encoding (unsigned varint 0)
        0xa1, // map 0, elements=1
        0x64, // map 0 key 0 = text-string, length=4
        0x6b, 0x65, 0x79, 0x7,  // "keys"
        0xa2, // map 0 value 0 = map 1, elements 1
        0x58, 0x22, // map 0 key 1 = byte-string, length=34
        0x1e, 0x20, // SHA2-256
        0xd9, 0x40, 0x0a, 0x52, 0x95, 0x22, 0xe1,
        0x9c, // a random SHA2-256 fingerprint bytes of an EC public key
        0x31, 0xee, 0x57, 0x67, 0x09, 0xe3, 0x51, 0xb1, 0x77, 0x32, 0x54, 0xf9, 0xbf, 0xac, 0x91,
        0xa6, 0x68, 0xc8, 0x93, 0xa6, 0x19, 0x99, 0xd5, 0xfa, 0x58,
        0x78, // map 0 value 1 = byte-string, length=120
        0x30, 0x76, 0x30, 0x10, 0x06, 0x07, 0x2a,
        0x86, // the EC public key DER encoded for the fingerprint
        0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x05, 0x2b, 0x81, 0x04, 0x00, 0x22, 0x03, 0x62, 0x00,
        0x04, 0x03, 0x82, 0x13, 0x9e, 0xac, 0x43, 0x1e, 0x79, 0xf2, 0xf3, 0xf3, 0x5e, 0x3b, 0x05,
        0x2a, 0x13, 0x23, 0x74, 0x35, 0x13, 0xe1, 0x35, 0x64, 0x92, 0xd3, 0xb3, 0x5d, 0xcc, 0x2b,
        0xaf, 0x4d, 0x2f, 0xa8, 0x67, 0x39, 0x0e, 0xa2, 0xee, 0x55, 0x20, 0xcb, 0x94, 0xe5, 0x00,
        0xa3, 0x9d, 0x8a, 0x86, 0x15, 0x46, 0x59, 0xc2, 0x54, 0xdf, 0x0b, 0x26, 0x65, 0x71, 0x96,
        0x9a, 0xea, 0x6a, 0x89, 0x8b, 0xef, 0xe4, 0x2e, 0x8c, 0x74, 0x36, 0xdf, 0x6e, 0x1d, 0xb0,
        0x8f, 0x5f, 0x44, 0x2b, 0x42, 0x52, 0xb0, 0x7b, 0x10, 0x2c, 0x7f, 0x55, 0x82, 0x30, 0x6a,
        0x05, 0x51, 0x92, 0x93, 0xec, 0x86, 0x83,
    ];

    /// Block 2: A minimal "identity" block
    const BLOCK_TWO_DATA: &[u8] = &[
        0x00, // Multibase: Identity Encoding (unsigned varint 0)
        0x00, // Multicodec: Identity
    ];

    /// Block 3: Unrelated unknown data
    const BLOCK_THREE_DATA: &[u8] = &[
        0xd9, 0xd9, 0xf7, 0x5a, 0x57, 0x46, 0x7a, 0x64, 0x47, 0x56, 0x79, 0x4c, 0x57, 0x56, 0x6e,
        0x5a, 0x77,
    ];

    /// Block 5
    const BLOCK_FIVE_DATA: &[u8] = &[
        0x00, // Multibase: Identity Encoding (unsigned varint 0)
        0xa1, // map 0, elements=1
        0x6c, // map 0 key 0 = text-string, length=12
        0x77, 0x6e, 0x66, 0x73, 0x2f, 0x70, 0x75, 0x62, // "wnfs/pub/dir"
        0x2f, 0x64, 0x69, 0x72, 0xa4, // map 0 value 0 = map 1, elements=4
        0x67, // map 1 key 0 = text-string, length=7
        0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, // "version"
        0x65, // map 1 value 0 = text-string, length=5
        0x30, 0x2e, 0x32, 0x2e, 0x30, // "0.2.0"
        0x68, // map 1 key 1 = text-string, length=8
        0x6d, 0x65, 0x74, 0x61, 0x64, 0x61, 0x74, 0x61, // "metadata"
        0xa2, // map 1 value 1 = map 2, elements=2
        0x67, // map 2 key 0 = text-string, length=7
        0x63, 0x72, 0x65, 0x61, 0x74, 0x65, 0x64, // "created"
        0xc1, // map 2 value 0 = tag(type=1)
        0x18, 0x64, 0xc3, 0xea, 0xd8, // unsigned(1690561240)
        0x68, // map 2 key 1 = text-string, length=8
        0x6d, 0x6f, 0x64, 0x69, 0x66, 0x69, 0x65, 0x64, // "modified"
        0xc1, // map 2 value 1 = tag(type=1)
        0x18, 0x64, 0xc3, 0xeb, 0x39, // unsigned(1690561337)
        0x68, // map 1 key 2 = text-string, length=8
        0x70, 0x72, 0x65, 0x76, 0x69, 0x6f, 0x75, 0x73, // "previous"
        0x80, // map 1 value 2 = array, length=0
        0x68, // map 1 key 3 = text-string, length=8
        0x75, 0x73, 0x65, 0x72, 0x6c, 0x61, 0x6e, 0x64, // "userland"
        0x80, // map 1 value 3 = array, length=0
    ];

    /// The byte values for this section were taken straight from the specification and should not
    /// vary. They've been documented and decoded by hand for completeness and for familiarity with the
    /// specification.
    const CARV2_PRAGMA: &[u8] = &[
        // A variable integer (LEB128) value indicating the total length remaining of this type.
        0x0a,
        // This begins the DAG-CBOR encoding defined in the CARv1 specification until the end of the
        // PRAGMA according to the IPLD Schema:
        //
        // ```ipld
        // type CarHeader struct {
        //   version Int
        //   roots [&Any]
        // }
        // ```
        //
        // A struct in DAG-CBOR is represented as a map type, with two elements. DAG-CBOR restricts key
        // types to text-string, and defines an explicit ordering on the order of keys getting encoded.
        // The order is length first (shorter precedes longer), followed by byte ordering if the
        // lengths are the same. Our map will be encoded with two key/value pairs starting with
        // "roots", then "version".
        //
        // The "roots" value is an array type that includes zero or more "Link" IPLD schema
        // equivalents. The IPLD data model specifies the Link kind as pointing to more data in another
        // IPLD block. Specifically:
        //
        // > Links are concretely implemented as CIDs.
        //
        // Additionally the CARv1 specification includes the following constraint on the "roots" array:
        //
        // > The roots array must contain one or more CIDs
        //
        // Spoiler, there are zero CIDs in this fixed header so I'm skipping the encoding/decoding
        // notes until I get to them.
        //
        // Next up is the "version" key with the a value of type "Int". The Int type can be either CBOR
        // major type 0 or 1 depending on whether the value is positive or negative. We're expecting a
        // value of 2 for the version so unsigned major type 0.
        //
        // For each I'm going to manually decode this value to confirm its correctness and my
        // understanding. This byte represents the structure of the data. The first 3 bits are the
        // major type, and the remaining 5 bits are the "short count". Breaking up this value we have
        // 0b101 and 0b0_0001. 0b101 = 5 which matches Major Type "map" (so far so good). 0b0_0001 is
        // simply a value of 1, indicating our map is going to contain a single value.
        //
        // So we know the spec has been violated already but unsure how.
        0xa1,
        // We get to our first map key-value pair, which we're expecting to be "roots". The type is
        // 0b011, major type 3 or "text-string" which is what we're looking for but it has a length of
        // 7 (0b0_0111)...
        0x67,
        // And sure enough we get an ASCII encoded "version", matching our length but telling us the
        // roots array is missing (since there are no other keys in the map).
        0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e,
        // Decoding this CBOR type gets us 0b000 which is an unsigned integer, the next five bits are a
        // "short-count" value of 0b0_0010, or the unsigned number 2. This matches our expected
        // version.
        0x02,
        // Side Note:
        //
        // There is an "unresolved item" noting that it is "unresolved" whether a valid CAR file must
        // contain _at least one_ block (which presumably would be listed here in the omitted roots
        // section.
        //
        // There is _no discussion_ about whether the "roots" key is allowed to be entirely omitted OR
        // if the "roots" array is ever allowed to be empty when there is at least one data block. This
        // fixed header is a direct violation of the original CARv1 specification.
        //
        // Some additional relevant sections from specifications:
        //
        // IPLD schema spec:
        //
        // > Optional or implicit fields are not possible with the tuple Struct representation strategy,
        // > all elements must be present.
        //
        // RFC8949 (decode strictness):
        //
        // > If the encoded sequence of bytes ends before the end of a data item, that item is not well-formed.
        //
        // DAG-CBOR variant requirements in RFC8949:
        //
        // > An encoder MUST produce only well-formed encoded data items.
        //
        // It sounds like CARv2 might also prevent us from loading the CAR data on FIL due to this
        // requirement and the spec violation (this is listed in Unresolved Items -> Number of Roots in
        // the CARv1 spec):
        //
        // > Current usage of the CAR format in Filecoin requires exactly one CID
    ];

    const OPTIONAL_PADDING_ONE: &[u8] = &[
        // These can be any value and any length. Added five random bytes here to test conformance.
        // Should be entirely ignored.
        0x11, 0x22, 0x33, 0xca, 0xfe,
    ];

    /// Padding before
    const OPTIONAL_PADDING_TWO: &[u8] = &[
        // These can be any value and any length. Added some more random bytes here to test
        // conformance.
        0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
    ];

    /// Represent Cid bytes as a base58 String
    pub fn binary_cid_to_base58_cid(cid: &[u8]) -> String {
        format!("z{}", cid[1..].to_base58())
    }

    fn blake3(bytes: &[u8]) -> Vec<u8> {
        blake3::hash(bytes).as_bytes().to_vec()
    }

    fn blake3_cid(codec: &[u8], data: &[u8]) -> Vec<u8> {
        let mut cid_bytes = vec![
            0x00, // Multibase: Identity Encoding (unsigned varint)
            0x01, // CID Version: 1 (unsigned varint)
        ];

        cid_bytes.extend_from_slice(codec); // CIDv1 Multicodec: (as provided)
        cid_bytes.extend_from_slice(&[0x1e, 0x20]); // CIDv1 Multihash: blake3, 32-byte digest
        cid_bytes.extend_from_slice(&blake3(data));

        cid_bytes
    }

    /// NOTE: The contents of this header _ARE NOT_ encoded as CBOR values, they are direct raw
    /// encodings.
    fn carv2_header(
        fully_indexed: bool,
        data_offset: u64,
        data_size: u64,
        index_offset: u64,
    ) -> Vec<u8> {
        let mut header_bytes = Vec::new();

        // Characteristics (16 bytes)
        //
        // Currently only one characteristic is defined, the left-most bit represents whether the
        // internal CAR file is fully indexed, which ours will be. If it is not fully indexed when
        // injesting the file we'll want to do our own scan and build-up/replace the existing index.
        //
        // There is one edge case mentioned about fully-sorted later on in the index specific section
        // about identity CIDs still be indexed as such I'm going to explicitly include one of these
        // blocks in the data section AND include it in the index as an example.
        let mut characteristics: u128 = 0;
        if fully_indexed {
            characteristics |= 0x8000_0000_0000_0000;
        }
        header_bytes.extend_from_slice(&characteristics.to_le_bytes());

        // Data offset (8 bytes)
        //
        // Value is the offset from the start of the CARv2 byte stream where you can find the first
        // byte of the first block.
        header_bytes.extend_from_slice(&data_offset.to_le_bytes());

        // Data size (8 bytes)
        //
        // Value is the number of bytes contained in the CARv2 payload. Value can be calculated in this
        // file by taking the address tagged with @data-end and subtracting the address tagged with
        // @data-start.
        header_bytes.extend_from_slice(&data_size.to_le_bytes());

        // Index offset (8 bytes)
        //
        // Value is the address located at @index-start. When this value is zero, the file is
        // considered to not have any index at all.
        header_bytes.extend_from_slice(&index_offset.to_le_bytes());

        assert_eq!(header_bytes.len(), HEADER_SIZE);

        header_bytes
    }

    /// Create a varint
    fn dirty_varint(mut val: usize) -> Vec<u8> {
        let mut var_bytes = vec![];

        loop {
            let mut current_byte = (val & 0b0111_1111) as u8; // take the lower 7 bits
            val >>= 7; // shift them away

            if val > 0 {
                // This isn't the last byte, set the high bit
                current_byte |= 0b1000_0000;
            }

            // append our current byte to the byte list (this is doing the MSB to LSB conversion)
            var_bytes.push(current_byte);

            // if nothing is remaining drop out of the loop
            if val == 0 {
                break;
            }
        }

        var_bytes
    }

    /// Create an IPLD link
    fn ipld_link(cid: &[u8]) -> Vec<u8> {
        let mut cid_bytes = vec![
            0x00, // Multibase: Identity Encoding (unsigned varint 0)
            0xd8, 0x2a, // IPLD Tag(42)
        ];

        cid_bytes.extend_from_slice(&dirty_varint(cid.len()));
        cid_bytes.extend_from_slice(cid);

        cid_bytes
    }

    /// Hash using sha1
    fn sha1(bytes: &[u8]) -> Vec<u8> {
        let mut hasher = sha1::Sha1::new();
        hasher.update(bytes);
        hasher.finalize().to_vec()
    }

    /// Create Cid using sha1
    fn sha1_cid(codec: &[u8], data: &[u8]) -> Vec<u8> {
        let mut cid_bytes = vec![
            0x00, // Multibase: Identity Encoding (unsigned varint)
            0x01, // CID Version: 1 (unsigned varint)
        ];

        cid_bytes.extend_from_slice(codec); // CIDv1 Multicodec: (as provided)
        cid_bytes.extend_from_slice(&[0x11, 0x14]); // CIDv1 Multihash: SHA1, 20-byte digest
        cid_bytes.extend_from_slice(&sha1(data));

        cid_bytes
    }

    /// Hash using sha2
    fn sha2_256(bytes: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        hasher.finalize().to_vec()
    }

    /// Create Cid using sha256
    fn sha256_cid(codec: &[u8], data: &[u8]) -> Vec<u8> {
        let mut cid_bytes = vec![
            0x00, // Multibase: Identity Encoding (unsigned varint)
            0x01, // CID Version: 1 (unsigned varint)
        ];

        cid_bytes.extend_from_slice(codec); // CIDv1 Multicodec: (as provided)
        cid_bytes.extend_from_slice(&[0x12, 0x20]); // CIDv1 Multihash: SHA2-256, 32-byte digest
        cid_bytes.extend_from_slice(&sha2_256(data));

        cid_bytes
    }

    /// Construct fixture
    fn build_full_car() -> Vec<u8> {
        // We're going to cheat and pre-generate all the data we need before we start generating the
        // actual CAR file
        let mut all_car_bytes: Vec<u8> = Vec::new();

        // Base blocks
        let block_zero_cid = blake3_cid(&[0x51], BLOCK_ZERO_DATA); // CBOR data
        let block_zero_length = block_zero_cid.len() + BLOCK_ZERO_DATA.len();
        let block_zero_length_bytes = dirty_varint(block_zero_length);

        let block_two_cid = blake3_cid(&[0x30], BLOCK_TWO_DATA); // multicodec data
        let block_two_length = block_two_cid.len() + BLOCK_TWO_DATA.len();
        let block_two_length_bytes = dirty_varint(block_two_length);

        let block_three_cid = blake3_cid(&[0x55], BLOCK_THREE_DATA); // raw data
        let block_three_length = block_three_cid.len() + BLOCK_THREE_DATA.len();
        let block_three_length_bytes = dirty_varint(block_three_length);

        let block_five_cid = sha256_cid(&[0x51], BLOCK_FIVE_DATA); // CBOR data
        let block_five_length = block_five_cid.len() + BLOCK_FIVE_DATA.len();
        let block_five_length_bytes = dirty_varint(block_five_length);

        // Derived blocks
        let block_one_data = ipld_link(&block_five_cid);
        let block_one_cid = blake3_cid(&[0x71], &block_one_data); // DAG-CBOR data
        let block_one_length = block_one_cid.len() + block_one_data.len();
        let block_one_length_bytes = dirty_varint(block_one_length);

        let block_four_data = ipld_link(&block_three_cid);
        let block_four_cid = sha1_cid(&[0x71], &block_four_data); // DAG-CBOR data
        let block_four_length = block_four_cid.len() + block_four_data.len();
        let block_four_length_bytes = dirty_varint(block_four_length);

        // PRAGMA
        all_car_bytes.extend_from_slice(CARV2_PRAGMA);

        // CARv1 Header
        let header = Header {
            version: 2,
            roots: RefCell::new(vec![
                Cid::from_str(&binary_cid_to_base58_cid(&block_zero_cid))
                    .expect("failed to represent binary as CID"),
                Cid::from_str(&binary_cid_to_base58_cid(&block_five_cid))
                    .expect("failed to represent binary as CID"),
            ]),
        };
        let header_bytes = header
            .to_ipld_bytes()
            .expect("failed to convert header to IPLD");
        let header_length_bytes = dirty_varint(header_bytes.len());

        // CARv2 Header
        let data_offset = PH_SIZE + OPTIONAL_PADDING_ONE.len() as u64;

        let data_size: u64 = vec![
            header_bytes.len() as u64 + 1,
            (block_zero_length_bytes.len() + block_zero_length) as u64,
            (block_one_length_bytes.len() + block_one_length) as u64,
            (block_two_length_bytes.len() + block_two_length) as u64,
            (block_three_length_bytes.len() + block_three_length) as u64,
            (block_four_length_bytes.len() + block_four_length) as u64,
            (block_five_length_bytes.len() + block_five_length) as u64,
        ]
        .into_iter()
        .sum();

        let index_offset = data_offset + data_size + OPTIONAL_PADDING_TWO.len() as u64;
        all_car_bytes.extend_from_slice(&carv2_header(true, data_offset, data_size, index_offset));

        // Optional Padding
        all_car_bytes.extend_from_slice(OPTIONAL_PADDING_ONE);

        // CARv1 Header (Beginning of CARv2 Data Payload)
        all_car_bytes.extend_from_slice(&header_length_bytes);
        all_car_bytes.extend_from_slice(&header_bytes);

        // CARv1 Payload

        // Here we have several blocks encoded as:
        //
        // | length | CID | block data |
        //
        // * Length is an unsigned varint representing the length of CID + data combined
        // * CID
        //   * Can be CIDv0 first byte would be 0x12, followed by 0x20 which specifies a 32 byte length
        //     digest, and is always a SHA2-256 bit digest.
        //   * Can be CIDv1, first two bytes are not 0x12, 0x20 we use alternate encoding rules:
        //      * Decode unsigned varint value (should be 1 after decoding)
        //      * Decode unsigned varint value (codec as defined in the multicodec table)
        //      * the raw bytes of a multihash (codec dependent)
        // * Data: This should be data encoded according to the format defined in the CIDv1 codec, for
        //   CIDv0 the implied codec is DAG-PB
        //
        // Going to ignore CIDv0, I don't want to mess around with protobuf

        // Block Zero
        let block_zero_offset = all_car_bytes.len() as u64;
        all_car_bytes.extend_from_slice(&block_zero_length_bytes);
        all_car_bytes.extend_from_slice(&block_zero_cid);
        all_car_bytes.extend_from_slice(BLOCK_ZERO_DATA);
        let block_zero_cid_string = binary_cid_to_base58_cid(&block_zero_cid);
        println!("Block 0 CID Base58: {}", block_zero_cid_string);
        assert!(Cid::from_str(&block_zero_cid_string).is_ok());

        // Block One
        let block_one_offset = all_car_bytes.len() as u64;
        all_car_bytes.extend_from_slice(&block_one_length_bytes);
        all_car_bytes.extend_from_slice(&block_one_cid);
        all_car_bytes.extend_from_slice(&block_one_data);
        let block_one_cid_string = binary_cid_to_base58_cid(&block_one_cid);
        println!("Block 1 CID Base58: {}", block_one_cid_string);
        assert!(Cid::from_str(&block_one_cid_string).is_ok());

        // Block Two
        let block_two_offset = all_car_bytes.len() as u64;
        all_car_bytes.extend_from_slice(&block_two_length_bytes);
        all_car_bytes.extend_from_slice(&block_two_cid);
        all_car_bytes.extend_from_slice(BLOCK_TWO_DATA);
        let block_two_cid_string = binary_cid_to_base58_cid(&block_two_cid);
        println!("Block 2 CID Base58: {}", block_two_cid_string);
        assert!(Cid::from_str(&block_two_cid_string).is_ok());

        // Block Three
        let block_three_offset = all_car_bytes.len() as u64;
        all_car_bytes.extend_from_slice(&block_three_length_bytes);
        all_car_bytes.extend_from_slice(&block_three_cid);
        all_car_bytes.extend_from_slice(BLOCK_THREE_DATA);
        let block_three_cid_string = binary_cid_to_base58_cid(&block_three_cid);
        println!("Block 3 CID Base58: {}", block_three_cid_string);
        assert!(Cid::from_str(&block_three_cid_string).is_ok());

        // Block Four
        let block_four_offset = all_car_bytes.len() as u64;
        all_car_bytes.extend_from_slice(&block_four_length_bytes);
        all_car_bytes.extend_from_slice(&block_four_cid);
        all_car_bytes.extend_from_slice(&block_four_data);
        let block_four_cid_string = binary_cid_to_base58_cid(&block_four_cid);
        println!("Block 4 CID Base58: {}", block_four_cid_string);
        assert!(Cid::from_str(&block_four_cid_string).is_ok());

        // Block Five
        let block_five_offset = all_car_bytes.len() as u64;
        all_car_bytes.extend_from_slice(&block_five_length_bytes);
        all_car_bytes.extend_from_slice(&block_five_cid);
        all_car_bytes.extend_from_slice(BLOCK_FIVE_DATA);
        let block_five_cid_string = binary_cid_to_base58_cid(&block_five_cid);
        println!("Block 5 CID Base58: {}", block_five_cid_string);
        assert!(Cid::from_str(&block_five_cid_string).is_ok());

        // Optional Padding 2
        all_car_bytes.extend_from_slice(OPTIONAL_PADDING_TWO);

        // Index

        println!("len before index starts: {}", all_car_bytes.len());

        // > At present, once read from a CARv2, the index data provides a mapping of hash digest bytes
        // > to block location in byte offset from the beginning of the CARv1 data payload (not the
        // > begining of the CARv2). The index only uses the hash digest—it does not use the full bytes
        // > of a CID, nor does it use any of the multihash prefix bytes.
        //
        // > The first byte(s) of a CARv2 index (at the "Index offset" position) contain an unsigned
        // > LEB128 integer ("varint") that indicates the index format type as a Multicodec code. The
        // > remaining bytes follow the encoding rules of that index format type.
        //
        // For IndexSorted the VALUE should be 0x0400
        all_car_bytes.extend_from_slice(&dirty_varint(0x0400));

        // Each bucket has the following format:
        //
        // * 32-bit unsigned little endian integer, represents the size of each index entry (bytes in
        //   the digest + 8 byte location offset).
        // * 64-bit unsigned little endian integer, the number of entries in this index
        //
        // Each index entry is made up of:
        //
        // * The CID / digest itself, should be 8 fewer bytes in length then the index entry size
        // * 64-bit unsigned little endian integer, this is the offset since the beginning of the CARv1
        //   payload where the block length varint can be found.
        //
        // When multiple digests are used that produce different length hashes, the shorter hashes
        // should come before the longer hashes

        // Our SHA1 bucket, only one entry
        let digest_len: u32 = block_four_cid.len() as u32 + 8;
        let digest_count: u64 = 1;
        all_car_bytes.extend_from_slice(&digest_len.to_le_bytes());
        all_car_bytes.extend_from_slice(&digest_count.to_le_bytes());

        // And our only entry for this index
        all_car_bytes.extend_from_slice(&block_four_cid);
        all_car_bytes.extend_from_slice(&block_four_offset.to_le_bytes());

        // Our SHA2-256 & Blake3 Digests
        let digest_len: u32 = block_two_cid.len() as u32 + 8;
        let digest_count: u64 = 5;
        all_car_bytes.extend_from_slice(&digest_len.to_le_bytes());
        all_car_bytes.extend_from_slice(&digest_count.to_le_bytes());

        // These need to be byte sorted which I've printed out to manually sort
        let index_entries: Vec<(Vec<u8>, u64)> = vec![
            (block_two_cid, block_two_offset),
            (block_five_cid, block_five_offset),
            (block_zero_cid, block_zero_offset),
            (block_three_cid, block_three_offset),
            (block_one_cid, block_one_offset),
        ];

        for (cid, offset) in index_entries.iter() {
            all_car_bytes.extend_from_slice(cid);
            all_car_bytes.extend_from_slice(&offset.to_le_bytes());
        }

        // MultihashIndexSorted is the same but has a different type ID (0x0401) and each bucket has an
        // additional attribute a 64-bit little-endian unsigned integer indicating the multihash code
        // used to generate the index as its first attribute. It basically amounts to an entire normal
        // IndexSorted with a single hash used to generate it. That's especially interesting for this
        // sample file as it uses two CID hashes already... I'll make a sample for that as well... The
        // sorting of multihashes is based on the multihash value, then digest length if it differs.

        // Type ID 0x0401: MultihashIndexSorted
        //0x80, 0x88, 0x00,

        // Begin a new Multihash bucket
        //
        // The spec is ambiguous here. This is supposed to be an unsigned little-endian integer
        // encoding a common multihash code. A multihash code is identified with a varint for the
        // code and a varint for the size of the digest, then usually contains the hash of the function
        // output. If the digest is omitted (as we don't have anything to hash here) how do we encode
        // the two values? Give them each 4 bytes and encode the 1 byte a-piece in each one? Have the
        // bytes consecutively adjacent? Which end of the 8-bytes are those one set on?
        //0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Multihash Code
        //0x28, 0x00, 0x00, 0x00,                         // 32 byte digest, 8 byte address for each entry
        //0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 5 entries

        // std::fs::write("carv2-indexed-fixture.car", all_car_bytes.clone())
        //     .expect("write hardcoded data to disk");

        all_car_bytes
    }

    #[test]
    fn read_data() -> Result<(), CarError> {
        let car_data = build_full_car();
        let mut data = Cursor::new(car_data.clone());
        let car = CarV2::read_bytes(&mut data)?;
        // Assert that reading it didnt modify the data
        assert_eq!(data.clone().into_inner(), car_data);

        let mut all_cids = vec![];
        for bucket in car.car.index.borrow().clone().buckets {
            all_cids.extend_from_slice(&bucket.map.into_keys().collect::<Vec<Cid>>())
        }

        // For every cid
        for cid in all_cids {
            // Read the block
            let block = car.get_block(&cid, &mut data)?;
            // Assert that its CID matches
            assert_eq!(cid, block.cid);
        }

        Ok(())
    }

    #[test]
    fn read_write_data() -> Result<(), CarError> {
        let car_data = build_full_car();
        let mut data = Cursor::new(car_data);
        let car = CarV2::read_bytes(&mut data)?;
        car.write_bytes(&mut data)?;
        data.seek(SeekFrom::Start(0))?;
        let car2 = CarV2::read_bytes(&mut data)?;

        assert_eq!(car.header, car2.header);
        assert_eq!(car.car.header, car2.car.header);
        assert_eq!(car.car.index, car2.car.index);
        Ok(())
    }
}
