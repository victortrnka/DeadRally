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

/// The original's window with `-nogl`: always 640x480 (`SDL_SetVideoMode(640, 480, 32, ...)`).
pub const WINDOW_WIDTH: u32 = 640;
pub const WINDOW_HEIGHT: u32 = 480;
/// Where its software path puts a 320x200 screen.
const LOW_RES_TOP: usize = 40;

impl Frame<'_> {
    /// The frame as the original's window shows it with `-nogl` (`refreshScreen` 0x43B580's
    /// software path), in RGBA8: 640x480 screens as they are; 320x200 screens from row 40,
    /// black above and below, each pixel doubled or, `smooth` (`-smooth`, 0x43BA99), on the
    /// even columns of the even rows with the means of its neighbours between. The reference
    /// runner's screenshots are of this picture.
    ///
    /// # Errors
    ///
    /// If the frame is neither 640x480 nor 320x200, which the original never shows.
    ///
    /// # Panics
    ///
    /// If `out.len() != 640 * 480 * 4`.
    pub fn write_window_rgba(&self, smooth: bool, out: &mut [u8]) -> Result<(), String> {
        let (width, height) = (WINDOW_WIDTH as usize, WINDOW_HEIGHT as usize);
        assert_eq!(
            out.len(),
            width * height * 4,
            "RGBA buffer has the wrong size"
        );
        // As the original keeps them: 0x00RRGGBB.
        let lut: [u32; 256] = std::array::from_fn(|i| {
            let [r, g, b] = self.palette[i].map(|c| u32::from(expand_6bit(c)));
            r << 16 | g << 8 | b
        });
        let mut window = vec![0u32; width * height];
        match (self.width, self.height) {
            (WINDOW_WIDTH, WINDOW_HEIGHT) => {
                for (slot, &index) in window.iter_mut().zip(self.pixels) {
                    *slot = lut[usize::from(index)];
                }
            }
            (320, 200) => {
                for (y, source) in self.pixels.chunks(320).enumerate() {
                    let start = (LOW_RES_TOP + 2 * y) * width;
                    let row = &mut window[start..start + width];
                    for (x, &index) in source.iter().enumerate() {
                        let colour = lut[usize::from(index)];
                        row[2 * x] = colour;
                        if !smooth {
                            row[2 * x + 1] = colour;
                        }
                    }
                }
                let picture = LOW_RES_TOP..LOW_RES_TOP + 400;
                if smooth {
                    let mean = |a: u32, b: u32| ((a & 0xFC_FCFC) + (b & 0xFC_FCFC)) >> 1;
                    // Each odd column but the last, then each odd row but its last column.
                    for y in picture.clone().step_by(2) {
                        let row = &mut window[y * width..(y + 1) * width];
                        for x in (1..width - 1).step_by(2) {
                            row[x] = mean(row[x - 1], row[x + 1]);
                        }
                    }
                    for y in picture.skip(1).step_by(2) {
                        for x in 0..width - 1 {
                            let above = window[(y - 1) * width + x];
                            let below = window[(y + 1) * width + x];
                            window[y * width + x] = mean(above, below);
                        }
                    }
                } else {
                    for y in picture.skip(1).step_by(2) {
                        window.copy_within((y - 1) * width..y * width, y * width);
                    }
                }
            }
            (width, height) => {
                return Err(format!("the original has no {width}x{height} screen"));
            }
        }
        for (rgba, colour) in out.as_chunks_mut::<4>().0.iter_mut().zip(window) {
            let [_, r, g, b] = colour.to_be_bytes();
            *rgba = [r, g, b, 255];
        }
        Ok(())
    }
}

/// Expands a 6-bit VGA colour component to 8 bits the way the Windows version does: a plain
/// shift, so 63 becomes 252, not 255 (`setPaletteAndGetValue`, 0x43C0A0). The top two bits are
/// ignored, as the VGA DAC ignores them.
#[must_use]
pub fn expand_6bit(component: u8) -> u8 {
    (component & 0x3F) << 2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame<'a>(
        width: u32,
        height: u32,
        pixels: &'a [u8],
        palette: &'a [[u8; 3]; 256],
    ) -> Frame<'a> {
        Frame {
            width,
            height,
            pixels,
            palette,
            aspect: (4, 3),
        }
    }

    fn window(frame: &Frame<'_>, smooth: bool) -> Vec<[u8; 4]> {
        let mut out = vec![0; (WINDOW_WIDTH * WINDOW_HEIGHT * 4) as usize];
        frame.write_window_rgba(smooth, &mut out).unwrap();
        out.as_chunks::<4>().0.to_vec()
    }

    fn at(window: &[[u8; 4]], x: u32, y: u32) -> [u8; 4] {
        window[(y * WINDOW_WIDTH + x) as usize]
    }

    #[test]
    fn low_resolution_screens_are_doubled_between_black_bands() {
        // Measured on the original under Wine: the intro fills rows 40..440, two by two.
        let mut palette = [[0; 3]; 256];
        palette[1] = [63, 0, 0];
        let mut pixels = vec![0; 320 * 200];
        pixels[0] = 1;
        let window = window(&frame(320, 200, &pixels, &palette), false);
        assert_eq!(at(&window, 0, 39), [0, 0, 0, 255]);
        for (x, y) in [(0, 40), (1, 40), (0, 41), (1, 41)] {
            assert_eq!(at(&window, x, y), [252, 0, 0, 255], "({x}, {y})");
        }
        assert_eq!(at(&window, 2, 40), [0, 0, 0, 255]);
    }

    #[test]
    fn full_resolution_screens_are_copied_as_they_are() {
        // The menus are drawn at the window's size; any scaling or smoothing of them in the
        // software picture would blur the original's text (0x43BBF9 copies them).
        let mut palette = [[0; 3]; 256];
        palette[7] = [1, 2, 3];
        let pixels = vec![7; 640 * 480];
        for smooth in [false, true] {
            let window = window(&frame(640, 480, &pixels, &palette), smooth);
            assert!(window.iter().all(|rgba| *rgba == [4, 8, 12, 255]));
        }
    }

    #[test]
    fn other_sizes_are_rejected() {
        // The original has no other screen; a frame of another size is a bug to report,
        // not a picture to guess at.
        let palette = [[0; 3]; 256];
        let pixels = vec![0; 640 * 360];
        let mut out = vec![0; (WINDOW_WIDTH * WINDOW_HEIGHT * 4) as usize];
        assert!(
            frame(640, 360, &pixels, &palette)
                .write_window_rgba(false, &mut out)
                .is_err()
        );
    }

    #[test]
    fn smoothing_puts_the_means_of_the_neighbours_between_the_pixels() {
        // With -smooth the original's software picture (0x43BA99) is not doubled: a player
        // who asks for it sees each pixel once with the means of its neighbours between,
        // each channel's low two bits dropped before a mean is taken.
        let mut palette = [[0; 3]; 256];
        palette[1] = [1, 0, 0];
        palette[2] = [2, 0, 63];
        let mut pixels = vec![0; 320 * 200];
        pixels[0] = 1;
        pixels[1] = 2;
        let window = window(&frame(320, 200, &pixels, &palette), true);
        assert_eq!(at(&window, 0, 40), [4, 0, 0, 255]);
        assert_eq!(at(&window, 2, 40), [8, 0, 252, 255]);
        assert_eq!(at(&window, 1, 40), [6, 0, 126, 255]);
        // The row under is the mean of the one above and the black one below; 6 loses its
        // low bits first.
        assert_eq!(at(&window, 0, 41), [2, 0, 0, 255]);
        assert_eq!(at(&window, 1, 41), [2, 0, 62, 255]);
        assert_eq!(at(&window, 3, 40), [4, 0, 126, 255]);
    }

    #[test]
    fn smoothing_leaves_the_last_column_black_and_halves_the_last_row() {
        // Measured on the original under Wine (`-nogl -smooth`): column 639 is never written,
        // and the last row is a mean with the black row under the picture.
        let mut palette = [[0; 3]; 256];
        palette[5] = [40, 40, 40];
        let pixels = vec![5; 320 * 200];
        let window = window(&frame(320, 200, &pixels, &palette), true);
        assert_eq!(at(&window, 638, 100), [160, 160, 160, 255]);
        assert_eq!(at(&window, 639, 100), [0, 0, 0, 255]);
        assert_eq!(at(&window, 100, 438), [160, 160, 160, 255]);
        assert_eq!(at(&window, 100, 439), [80, 80, 80, 255]);
        assert_eq!(at(&window, 100, 39), [0, 0, 0, 255]);
        assert_eq!(at(&window, 100, 440), [0, 0, 0, 255]);
    }
}
