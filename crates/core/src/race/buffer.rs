//! The race's drawing buffer as the original allocates it (`0x464F14`: 0x19400 bytes, used
//! from byte 512), rows 512 bytes apart, the picture shown from column 96; and its drawing
//! routines, taking offsets from the buffer's start as the original's do.

/// Bytes a row, the buffer's size and where offsets count from.
pub(crate) const STRIDE: usize = 512;
const SIZE: usize = 0x19400;
const BASE: usize = 512;
/// The column the shown picture starts at.
pub(crate) const LEFT: usize = 96;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Buffer {
    data: Vec<u8>,
}

impl Default for Buffer {
    fn default() -> Buffer {
        Buffer {
            data: vec![0; SIZE],
        }
    }
}

impl Buffer {
    /// The byte at `offset` from the buffer's start, if inside its allocation.
    fn slot(&mut self, offset: i64) -> Option<&mut u8> {
        let at = usize::try_from(offset + BASE as i64).ok()?;
        self.data.get_mut(at)
    }

    /// The shown pixel at (`x`, `y`).
    pub(crate) fn pixel(&self, x: usize, y: usize) -> u8 {
        self.data[BASE + y * STRIDE + LEFT + x]
    }

    /// `copyBuffer2Screen` (0x43B0D0): `bytes` copied to `offset`.
    pub(crate) fn copy(&mut self, offset: i64, bytes: &[u8]) {
        for (index, &byte) in bytes.iter().enumerate() {
            if let Some(slot) = self.slot(offset + index as i64) {
                *slot = byte;
            }
        }
    }

    /// `drawInRaceImageToBuffer_43B160`: a `width` x `height` picture copied whole.
    pub(crate) fn draw_opaque(&mut self, image: &[u8], width: usize, height: usize, offset: i64) {
        for row in 0..height {
            let line = image.get(row * width..(row + 1) * width).unwrap_or(&[]);
            self.copy(offset + (row * STRIDE) as i64, line);
        }
    }

    /// `drawImageInRace_43B240`: a `width` x `height` picture, its 0 bytes left out.
    pub(crate) fn draw(&mut self, image: &[u8], width: usize, height: usize, offset: i64) {
        for row in 0..height {
            for column in 0..width {
                let byte = image.get(row * width + column).copied().unwrap_or(0);
                if byte != 0
                    && let Some(slot) = self.slot(offset + (row * STRIDE + column) as i64)
                {
                    *slot = byte;
                }
            }
        }
    }

    /// `drawTurboBar_43B3A0`: in a `width` x `height` box, every byte of 0x40 or more painted
    /// `colour`.
    pub(crate) fn paint_lit(&mut self, offset: i64, width: i64, height: i64, colour: u8) {
        for row in 0..height.max(0) {
            for column in 0..width.max(0) {
                if let Some(slot) = self.slot(offset + row * STRIDE as i64 + column)
                    && *slot >= 0x40
                {
                    *slot = colour;
                }
            }
        }
    }

    /// `sub_43D530`'s inner loop: the byte at `offset` turned through `table`.
    pub(crate) fn turn(&mut self, offset: i64, table: &[u8; 256]) {
        if let Some(slot) = self.slot(offset) {
            *slot = table[usize::from(*slot)];
        }
    }

    /// `drawWeaponsBar_43BEF0`: a `width` x `height` box filled with `colour`.
    pub(crate) fn fill(&mut self, offset: i64, width: i64, height: i64, colour: u8) {
        for row in 0..height.max(0) {
            for column in 0..width.max(0) {
                if let Some(slot) = self.slot(offset + row * STRIDE as i64 + column) {
                    *slot = colour;
                }
            }
        }
    }

    /// `sub_43C7E0`: a slanted line of 2-pixel steps from (`from`, 0) to (`to`, `rows`),
    /// painting bytes of 0x40 or more `colour`.
    pub(crate) fn slant(&mut self, offset: i64, from: i32, to: i32, rows: i32, colour: u8) {
        let step = (((to - from) << 8) / rows) as i64;
        let mut x = 0i64;
        for row in 0..rows.max(0) {
            x += step;
            let at = offset + row as i64 * STRIDE as i64 + (x >> 8);
            for dx in 0..2 {
                if let Some(slot) = self.slot(at + dx)
                    && *slot >= 0x40
                {
                    *slot = colour;
                }
            }
        }
    }
}
