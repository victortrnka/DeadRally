/// The picture the core hands to a frontend: 8-bit indexed pixels, the 6-bit VGA palette they
/// index into, and the aspect ratio the picture must be shown at.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    /// Indexed pixels, row-major and tightly packed: `pixels.len() == width * height`.
    pub pixels: &'a [u8],
    /// 256 RGB entries with 6-bit components (0..=63), as on VGA hardware.
    pub palette: &'a [[u8; 3]; 256],
    /// Display aspect ratio as (width, height). 320x200 is shown at 4:3, so its pixels are not
    /// square; frontends must scale to this ratio, not to the pixel dimensions.
    pub aspect: (u32, u32),
}

impl Frame<'_> {
    /// Converts the frame to RGBA8 (4 bytes per pixel, alpha 255). Every frontend uses this one
    /// conversion so colours are identical everywhere.
    ///
    /// # Panics
    ///
    /// If `out.len() != width * height * 4`.
    pub fn write_rgba(&self, out: &mut [u8]) {
        assert_eq!(
            out.len(),
            self.pixels.len() * 4,
            "RGBA buffer has the wrong size"
        );
        let lut: [[u8; 4]; 256] = std::array::from_fn(|i| {
            let [r, g, b] = self.palette[i];
            [expand_6bit(r), expand_6bit(g), expand_6bit(b), 255]
        });
        for (rgba, &index) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.pixels) {
            *rgba = lut[usize::from(index)];
        }
    }
}

/// Expands a 6-bit VGA colour component to 8 bits the way the Windows version does: a plain
/// shift, so 63 becomes 252, not 255 (`setPaletteAndGetValue`, 0x43C0A0). The top two bits are
/// ignored, as the VGA DAC ignores them.
#[must_use]
pub fn expand_6bit(component: u8) -> u8 {
    (component & 0x3F) << 2
}
