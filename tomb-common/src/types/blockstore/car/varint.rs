use std::io::{Read, self};

const U64_LEN: usize = 10;

pub(crate) fn read_varint_u64<R: Read>(
    stream: &mut R,
) -> Result<Option<(u64, usize)>, io::Error> {
    let mut result: u64 = 0;

    for i in 0..U64_LEN {
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf)?;

        let byte = buf[0];
        result |= u64::from(byte & 0b0111_1111) << (i * 7);

        // If is last byte = leftmost bit is zero
        if byte & 0b1000_0000 == 0 {
            return Ok(Some((result, i + 1)));
        }
    }

    // TODO: Return error
    Ok(None)
}

pub(crate) fn encode_varint_u64(input: u64, buf: &mut [u8; U64_LEN]) -> (&[u8], usize) {
    let mut n = input;
    let mut i = 0;
    for b in buf.iter_mut() {
        *b = n as u8 | 0b1000_0000;
        n >>= 7;
        if n == 0 {
            *b &= 0b0111_1111;
            break;
        }
        i += 1
    }
    debug_assert_eq!(n, 0);
    (&buf[0..=i], i + 1)
}