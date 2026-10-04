//! BPK images: LZW with swapped clear/end codes, every output byte rotated right by 3
//! (spec M1a §3.2).

use std::fmt;

use crate::lzw::{self, LzwError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BpkError {
    Lzw(LzwError),
    /// A range asked for more bytes than the stream holds.
    TooShort {
        wanted: usize,
        decoded: usize,
    },
}

impl fmt::Display for BpkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BpkError::Lzw(error) => write!(f, "{error}"),
            BpkError::TooShort { wanted, decoded } => {
                write!(f, "the image holds {decoded} bytes, {wanted} were needed")
            }
        }
    }
}

impl std::error::Error for BpkError {}

impl From<LzwError> for BpkError {
    fn from(error: LzwError) -> BpkError {
        BpkError::Lzw(error)
    }
}

/// Decodes a whole stream.
///
/// # Errors
///
/// [`BpkError::Lzw`] on corrupt data.
pub fn decode(stream: &[u8]) -> Result<Vec<u8>, BpkError> {
    let mut out = Vec::new();
    lzw::decode(stream, lzw::BPK, usize::MAX, &mut out)?;
    rotate(&mut out);
    Ok(out)
}

/// Skips `skip` decoded bytes and returns the next `len`, as the original does to cut one
/// stream into sub-images.
///
/// # Errors
///
/// [`BpkError::TooShort`] when the stream ends first; [`BpkError::Lzw`] on corrupt data.
pub fn decode_range(stream: &[u8], skip: usize, len: usize) -> Result<Vec<u8>, BpkError> {
    let wanted = skip + len;
    let mut out = Vec::new();
    lzw::decode(stream, lzw::BPK, wanted, &mut out)?;
    if out.len() < wanted {
        return Err(BpkError::TooShort {
            wanted,
            decoded: out.len(),
        });
    }
    rotate(&mut out);
    Ok(out.split_off(skip))
}

/// Splits an entry made of several concatenated streams (shop and car animations) into its
/// streams. After a stream's end code the next one starts at the following byte, or one byte
/// later when that byte does not begin with a clear code.
///
/// # Errors
///
/// [`BpkError::Lzw`] when a stream is corrupt.
pub fn split_streams(entry: &[u8]) -> Result<Vec<&[u8]>, BpkError> {
    let mut streams = Vec::new();
    let mut start = 0;
    let mut scratch = Vec::new();
    while start < entry.len() {
        scratch.clear();
        let decoded = lzw::decode(&entry[start..], lzw::BPK, usize::MAX, &mut scratch)?;
        let mut end = start + decoded.bits.div_ceil(8);
        if end < entry.len() && !starts_with_clear(&entry[end..]) {
            end += 1;
        }
        let end = end.min(entry.len());
        streams.push(&entry[start..end]);
        start = end;
    }
    Ok(streams)
}

/// The original's "decryption": every output byte rotated right by 3.
fn rotate(bytes: &mut [u8]) {
    for byte in bytes {
        *byte = byte.rotate_right(3);
    }
}

fn starts_with_clear(bytes: &[u8]) -> bool {
    match bytes {
        [low, high, ..] => (u16::from(*low) | (u16::from(*high & 1) << 8)) == lzw::BPK.clear,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs 9-bit codes LSB-first.
    fn pack9(codes: &[u16]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let (mut accumulator, mut bits) = (0u32, 0u32);
        for &code in codes {
            accumulator |= u32::from(code) << bits;
            bits += 9;
            while bits >= 8 {
                bytes.push((accumulator & 0xFF) as u8);
                accumulator >>= 8;
                bits -= 8;
            }
        }
        if bits > 0 {
            bytes.push((accumulator & 0xFF) as u8);
        }
        bytes
    }

    /// The stored literal that decodes to `byte` after the rotation.
    fn stored(byte: u8) -> u16 {
        u16::from(byte.rotate_left(3))
    }

    #[test]
    fn output_bytes_are_rotated_right_by_three() {
        // Track images decode to "RIX3" only with the rotation; without it they are noise.
        let stream = pack9(&[
            257,
            stored(b'R'),
            stored(b'I'),
            stored(b'X'),
            stored(b'3'),
            256,
        ]);
        assert_eq!(decode(&stream).unwrap(), b"RIX3");
    }

    #[test]
    fn a_range_skips_and_limits() {
        let stream = pack9(&[257, stored(1), stored(2), stored(3), stored(4), 256]);
        assert_eq!(decode_range(&stream, 1, 2).unwrap(), [2, 3]);
    }

    #[test]
    fn a_range_past_the_end_is_an_error() {
        let stream = pack9(&[257, stored(1), 256]);
        assert_eq!(
            decode_range(&stream, 0, 2).unwrap_err(),
            BpkError::TooShort {
                wanted: 2,
                decoded: 1
            }
        );
    }

    #[test]
    fn concatenated_streams_split_at_their_end_codes() {
        // Two one-pixel frames: 27 bits each, so each occupies 4 bytes and the next starts
        // with a clear code.
        let mut entry = pack9(&[257, stored(7), 256]);
        entry.extend(pack9(&[257, stored(9), 256]));
        let streams = split_streams(&entry).unwrap();
        assert_eq!(streams.len(), 2);
        assert_eq!(decode(streams[0]).unwrap(), [7]);
        assert_eq!(decode(streams[1]).unwrap(), [9]);
    }

    #[test]
    fn a_padding_byte_between_streams_is_skipped() {
        // The original's encoder sometimes leaves one extra byte before the next frame.
        let mut entry = pack9(&[257, stored(7), 256]);
        entry.push(0);
        entry.extend(pack9(&[257, stored(9), 256]));
        let streams = split_streams(&entry).unwrap();
        assert_eq!(streams.len(), 2);
        assert_eq!(decode(streams[1]).unwrap(), [9]);
    }
}
