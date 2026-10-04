//! Track metadata (`TRn-INF.BIN`) and track images with their RIX3 header (spec M1a §3.4).

use std::fmt;

use crate::bpk::{self, BpkError};
use crate::image::{Image, PALETTE_BYTES, Palette, PaletteError};

/// `TRn-INF.BIN`: 127 little-endian `i32`s.
pub const INFO_BYTES: usize = 127 * 4;

const RIX3_HEADER_BYTES: usize = 10;

/// What `TRn-INF.BIN` holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackInfo {
    pub width: u32,
    pub height: u32,
    pub zones: i32,
    /// Start slots: x, y, direction.
    pub starts: [[i32; 3]; 4],
    /// Power-up spots: x, y.
    pub power_ups: [[i32; 2]; 16],
    /// Pedestrians: x, y, id, flag.
    pub pedestrians: [[i32; 4]; 20],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackError {
    InfoLength(usize),
    BadDimensions {
        width: i32,
        height: i32,
    },
    Bpk(BpkError),
    NotRix3,
    Palette(PaletteError),
    /// The image's RIX3 header disagrees with `INF.BIN`, or the pixels do not fill it.
    Mismatch(String),
}

impl fmt::Display for TrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrackError::InfoLength(length) => {
                write!(f, "INF.BIN is {INFO_BYTES} bytes, not {length}")
            }
            TrackError::BadDimensions { width, height } => write!(f, "track size {width}x{height}"),
            TrackError::Bpk(error) => write!(f, "{error}"),
            TrackError::NotRix3 => write!(f, "the track image has no RIX3 header"),
            TrackError::Palette(error) => write!(f, "{error}"),
            TrackError::Mismatch(what) => write!(f, "{what}"),
        }
    }
}

impl std::error::Error for TrackError {}

impl TrackInfo {
    /// # Errors
    ///
    /// [`TrackError`] when the length is wrong or the dimensions are not positive.
    pub fn parse(bytes: &[u8]) -> Result<TrackInfo, TrackError> {
        if bytes.len() != INFO_BYTES {
            return Err(TrackError::InfoLength(bytes.len()));
        }
        let mut values = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| i32::from_le_bytes(*chunk));
        let mut next = || values.next().expect("127 values");
        let (width, height) = (next(), next());
        let (Ok(width_u), Ok(height_u)) = (u32::try_from(width), u32::try_from(height)) else {
            return Err(TrackError::BadDimensions { width, height });
        };
        if width_u == 0 || height_u == 0 {
            return Err(TrackError::BadDimensions { width, height });
        }
        let zones = next();
        let starts = std::array::from_fn(|_| std::array::from_fn(|_| next()));
        let power_ups = std::array::from_fn(|_| std::array::from_fn(|_| next()));
        let pedestrians = std::array::from_fn(|_| std::array::from_fn(|_| next()));
        Ok(TrackInfo {
            width: width_u,
            height: height_u,
            zones,
            starts,
            power_ups,
            pedestrians,
        })
    }
}

/// Decodes a RIX3 track image (`-IMA`, `-MAS` at full size; `-VAI`, `-LR1` at quarter size)
/// and checks its header against the expected size.
///
/// # Errors
///
/// [`TrackError`] when the stream is corrupt, the header is missing or the sizes disagree.
pub fn decode_rix3(stream: &[u8], width: u32, height: u32) -> Result<(Image, Palette), TrackError> {
    let bytes = bpk::decode(stream).map_err(TrackError::Bpk)?;
    if bytes.len() < RIX3_HEADER_BYTES + PALETTE_BYTES || &bytes[..4] != b"RIX3" {
        return Err(TrackError::NotRix3);
    }
    let header_width = u32::from(u16::from_le_bytes([bytes[4], bytes[5]]));
    let header_height = u32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
    if (header_width, header_height) != (width, height) {
        return Err(TrackError::Mismatch(format!(
            "RIX3 header says {header_width}x{header_height}, expected {width}x{height}"
        )));
    }
    let palette_end = RIX3_HEADER_BYTES + PALETTE_BYTES;
    let palette =
        Palette::from_bytes(&bytes[RIX3_HEADER_BYTES..palette_end]).map_err(TrackError::Palette)?;
    let pixels = &bytes[palette_end..];
    if pixels.len() != (width * height) as usize {
        return Err(TrackError::Mismatch(format!(
            "{} pixels for {width}x{height}",
            pixels.len()
        )));
    }
    Ok((Image::new(width, height, pixels.to_vec()), palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(values: &[i32]) -> Vec<u8> {
        let mut all = values.to_vec();
        all.resize(127, 0);
        all.iter().flat_map(|value| value.to_le_bytes()).collect()
    }

    #[test]
    fn the_fields_come_in_the_documented_order() {
        let mut values = vec![960, 600, 3];
        values.extend([10, 20, 1, 11, 21, 2, 12, 22, 3, 13, 23, 4]);
        values.extend([100, 200]);
        let parsed = TrackInfo::parse(&info(&values)).unwrap();
        assert_eq!((parsed.width, parsed.height, parsed.zones), (960, 600, 3));
        assert_eq!(parsed.starts[3], [13, 23, 4]);
        assert_eq!(parsed.power_ups[0], [100, 200]);
    }

    #[test]
    fn a_wrong_length_or_size_is_rejected() {
        assert_eq!(TrackInfo::parse(&[0; 12]), Err(TrackError::InfoLength(12)));
        assert!(matches!(
            TrackInfo::parse(&info(&[0, 600])),
            Err(TrackError::BadDimensions { .. })
        ));
        assert!(matches!(
            TrackInfo::parse(&info(&[-5, 600])),
            Err(TrackError::BadDimensions { .. })
        ));
    }
}
