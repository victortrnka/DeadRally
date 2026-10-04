//! Indexed images and 6-bit VGA palettes (spec M1a §3.3).

use std::fmt;

/// Bytes in a stored palette: 256 entries of red, green and blue.
pub const PALETTE_BYTES: usize = 768;

/// 256 RGB entries with 6-bit components (0..=63), as the VGA DAC took them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette(pub [[u8; 3]; 256]);

impl Palette {
    pub const BLACK: Palette = Palette([[0; 3]; 256]);

    /// Reads exactly 768 bytes.
    ///
    /// # Errors
    ///
    /// [`PaletteError`] when the length is wrong or a component exceeds 63 (the original data
    /// has none, so it means the wrong bytes were passed).
    pub fn from_bytes(bytes: &[u8]) -> Result<Palette, PaletteError> {
        if bytes.len() != PALETTE_BYTES {
            return Err(PaletteError::Length(bytes.len()));
        }
        if let Some(index) = bytes.iter().position(|&component| component > 63) {
            return Err(PaletteError::NotSixBit {
                index,
                value: bytes[index],
            });
        }
        let mut entries = [[0u8; 3]; 256];
        for (entry, rgb) in entries.iter_mut().zip(bytes.as_chunks::<3>().0) {
            *entry = *rgb;
        }
        Ok(Palette(entries))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteError {
    Length(usize),
    NotSixBit { index: usize, value: u8 },
}

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaletteError::Length(length) => {
                write!(f, "a palette is {PALETTE_BYTES} bytes, not {length}")
            }
            PaletteError::NotSixBit { index, value } => {
                write!(
                    f,
                    "palette byte {index} is {value}, above the 6-bit maximum 63"
                )
            }
        }
    }
}

impl std::error::Error for PaletteError {}

/// An 8-bit indexed image, row-major and tightly packed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Image {
    /// # Panics
    ///
    /// If `pixels.len() != width * height`.
    #[must_use]
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Image {
        assert_eq!(
            pixels.len(),
            (width * height) as usize,
            "{width}x{height} image"
        );
        Image {
            width,
            height,
            pixels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_palette_keeps_its_entries_in_order() {
        let mut bytes = vec![0u8; PALETTE_BYTES];
        bytes[3..6].copy_from_slice(&[63, 32, 1]);
        assert_eq!(Palette::from_bytes(&bytes).unwrap().0[1], [63, 32, 1]);
    }

    #[test]
    fn a_wrong_length_is_rejected() {
        // BGCOP.PAL holds two palettes; passing it whole is a caller bug worth catching.
        assert_eq!(
            Palette::from_bytes(&[0; 1536]),
            Err(PaletteError::Length(1536))
        );
    }

    #[test]
    fn components_above_63_are_rejected() {
        let mut bytes = vec![0u8; PALETTE_BYTES];
        bytes[10] = 64;
        assert_eq!(
            Palette::from_bytes(&bytes),
            Err(PaletteError::NotSixBit {
                index: 10,
                value: 64
            })
        );
    }
}
