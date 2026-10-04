//! The two BMPs the Windows version added: `rmd.bmp` (Remedy logo) and `end.bmp` (exit screen)
//! (spec M1a §3.3). Only what they use is supported: 8 bits per pixel, uncompressed.

use std::fmt;

use crate::image::{Image, Palette};

const FILE_HEADER_BYTES: usize = 14;
const INFO_HEADER_BYTES: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BmpError {
    NotBmp,
    Unsupported(String),
    Truncated,
}

impl fmt::Display for BmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BmpError::NotBmp => write!(f, "not a BMP file"),
            BmpError::Unsupported(what) => write!(f, "unsupported BMP: {what}"),
            BmpError::Truncated => write!(f, "the BMP file is cut short"),
        }
    }
}

impl std::error::Error for BmpError {}

/// Decodes an 8-bit uncompressed BMP. Colours become 6-bit as the original converts them,
/// `(c & 0xFC) >> 2`; palette entries the file does not define are black.
///
/// # Errors
///
/// [`BmpError`] for anything but an 8-bit, uncompressed, bottom-up BMP, or a short file.
pub fn decode(bytes: &[u8]) -> Result<(Image, Palette), BmpError> {
    if bytes.len() < FILE_HEADER_BYTES + INFO_HEADER_BYTES {
        return Err(if bytes.starts_with(b"BM") {
            BmpError::Truncated
        } else {
            BmpError::NotBmp
        });
    }
    if !bytes.starts_with(b"BM") {
        return Err(BmpError::NotBmp);
    }
    let u16_at = |offset: usize| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    let u32_at =
        |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes"));
    let pixel_offset = u32_at(10) as usize;
    let header_size = u32_at(14) as usize;
    let width = i32::from_le_bytes(bytes[18..22].try_into().expect("4 bytes"));
    let height = i32::from_le_bytes(bytes[22..26].try_into().expect("4 bytes"));
    let bits = u16_at(28);
    let compression = u32_at(30);
    let colours_used = match u32_at(46) {
        0 => 256,
        used => used as usize,
    };
    if header_size < INFO_HEADER_BYTES {
        return Err(BmpError::Unsupported(format!(
            "{header_size}-byte info header"
        )));
    }
    if bits != 8 || compression != 0 {
        return Err(BmpError::Unsupported(format!(
            "{bits} bits per pixel, compression {compression}"
        )));
    }
    if width <= 0 || height <= 0 || colours_used > 256 {
        return Err(BmpError::Unsupported(format!(
            "{width}x{height}, {colours_used} colours"
        )));
    }
    let (width, height) = (width.unsigned_abs(), height.unsigned_abs());

    let palette_start = FILE_HEADER_BYTES + header_size;
    let palette_end = palette_start + colours_used * 4;
    let stride = (width as usize).div_ceil(4) * 4;
    if bytes.len() < palette_end || bytes.len() < pixel_offset + stride * height as usize {
        return Err(BmpError::Truncated);
    }
    let mut palette = Palette::BLACK;
    for (entry, bgra) in palette
        .0
        .iter_mut()
        .zip(bytes[palette_start..palette_end].as_chunks::<4>().0)
    {
        *entry = [six_bit(bgra[2]), six_bit(bgra[1]), six_bit(bgra[0])];
    }
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for row in (0..height as usize).rev() {
        let start = pixel_offset + row * stride;
        pixels.extend_from_slice(&bytes[start..start + width as usize]);
    }
    Ok((Image::new(width, height, pixels), palette))
}

fn six_bit(component: u8) -> u8 {
    (component & 0xFC) >> 2
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3x2 BMP with `colours` palette entries; row 0 on screen is [1, 2, 3].
    fn build(colours: u32) -> Vec<u8> {
        let stride = 4;
        let pixel_offset = 14 + 40 + colours * 4;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&(pixel_offset + 2 * stride).to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&pixel_offset.to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&3i32.to_le_bytes());
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8u16.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        bytes.extend_from_slice(&colours.to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        for index in 0..colours {
            let value = u8::try_from(index * 4).unwrap();
            bytes.extend_from_slice(&[value, value + 1, value + 3, 0]);
        }
        bytes.extend_from_slice(&[4, 5, 6, 0]);
        bytes.extend_from_slice(&[1, 2, 3, 0]);
        bytes
    }

    #[test]
    fn rows_are_stored_bottom_up() {
        // Read top-down, the Remedy logo would appear upside down.
        let (image, _) = decode(&build(8)).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.pixels, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn colours_become_six_bit_and_blue_comes_first() {
        let (_, palette) = decode(&build(8)).unwrap();
        // Entry 2 is stored B=8, G=9, R=11.
        assert_eq!(palette.0[2], [11 >> 2, 9 >> 2, 8 >> 2]);
    }

    #[test]
    fn entries_beyond_the_used_colours_are_black() {
        // end.bmp defines only 223 colours.
        let (_, palette) = decode(&build(8)).unwrap();
        assert_eq!(palette.0[8], [0, 0, 0]);
    }

    #[test]
    fn other_formats_are_refused() {
        let mut bytes = build(8);
        bytes[28] = 24;
        assert!(matches!(decode(&bytes), Err(BmpError::Unsupported(_))));
        assert_eq!(decode(b"GIF89a"), Err(BmpError::NotBmp));
        let mut short = build(8);
        short.truncate(short.len() - 3);
        assert_eq!(decode(&short), Err(BmpError::Truncated));
    }
}
