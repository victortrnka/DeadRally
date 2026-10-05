//! The original's 640x480 drawing buffer and its operations (spec M2a §4.2). Positions are
//! linear offsets, `y * 640 + x`, as the original passes them: a picture drawn past the right
//! edge goes on at the start of the next row, as it does there. Nothing is drawn past the end.

use deadrally_gamedata::image::Image;

/// Menu screens are 640x480.
pub(crate) const WIDTH: usize = 640;
pub(crate) const HEIGHT: usize = 480;

/// The offset of column `x`, row `y`.
pub(crate) const fn at(x: usize, y: usize) -> usize {
    y * WIDTH + x
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Canvas {
    pixels: Vec<u8>,
}

impl Default for Canvas {
    fn default() -> Canvas {
        Canvas {
            pixels: vec![0; WIDTH * HEIGHT],
        }
    }
}

impl Canvas {
    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Copies the whole of `picture`, which must be 640x480.
    pub(crate) fn copy_all(&mut self, picture: &Image) {
        self.pixels.copy_from_slice(&picture.pixels);
    }

    /// Copies rows `first..first + rows` of `picture` (640 wide) into the same rows.
    pub(crate) fn copy_rows(&mut self, picture: &Image, first: usize, rows: usize) {
        let range = at(0, first)..at(0, first + rows).min(self.pixels.len());
        self.pixels[range.clone()].copy_from_slice(&picture.pixels[range]);
    }

    /// Copies a `width` x `height` region of `picture` (640 wide) at `offset` into the same place.
    pub(crate) fn restore(&mut self, picture: &Image, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&picture.pixels[start..end]);
        }
    }

    /// Copies a `width` x `height` region at `offset` from `other` into the same place
    /// (`copyRectToVram`, 0x41AA40, and `refreshAllScreen`, 0x41A210, for the whole screen).
    pub(crate) fn copy_from(&mut self, other: &Canvas, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&other.pixels[start..end]);
        }
    }

    /// Fills `width` x `height` pixels from `offset` with `colour`.
    pub(crate) fn fill(&mut self, offset: usize, width: usize, height: usize, colour: u8) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].fill(colour);
        }
    }

    /// Draws `image` at `offset`; with `transparent`, colour 0 leaves the canvas as it is.
    pub(crate) fn draw(&mut self, image: &Image, offset: usize, transparent: bool) {
        let width = image.width as usize;
        for (row, source) in image.pixels.chunks_exact(width.max(1)).enumerate() {
            let start = offset + row * WIDTH;
            if start >= self.pixels.len() {
                break;
            }
            let end = (start + width).min(self.pixels.len());
            let target = &mut self.pixels[start..end];
            for (pixel, &colour) in target.iter_mut().zip(source) {
                if !transparent || colour != 0 {
                    *pixel = colour;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_drawing_keeps_what_colour_0_covers() {
        // Glyphs, corners and the cursor are drawn this way; an opaque 0 would cut boxes
        // into the background around every letter.
        let mut canvas = Canvas::default();
        canvas.fill(at(0, 0), 4, 1, 9);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), true);
        assert_eq!(&canvas.pixels()[..4], [1, 9, 2, 9]);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), false);
        assert_eq!(&canvas.pixels()[..4], [1, 0, 2, 9]);
    }

    #[test]
    fn drawing_past_the_right_edge_goes_on_in_the_next_row() {
        // The original works on linear offsets; text that runs past column 639 shows at the
        // left of the next row, and a faithful copy must too.
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(3, 1, vec![5, 6, 7]), at(638, 0), false);
        assert_eq!(&canvas.pixels()[at(638, 0)..at(1, 1)], [5, 6, 7]);
    }

    #[test]
    fn nothing_is_drawn_past_the_end() {
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(2, 2, vec![1; 4]), at(639, 479), false);
        canvas.fill(at(639, 479), 5, 5, 3);
        assert_eq!(canvas.pixels()[at(639, 479)], 3);
        assert_eq!(canvas.pixels().len(), WIDTH * HEIGHT);
    }

    #[test]
    fn restoring_copies_a_region_back_from_a_picture() {
        let background = Image::new(
            640,
            480,
            (0..WIDTH * HEIGHT).map(|i| (i % 251) as u8).collect(),
        );
        let mut canvas = Canvas::default();
        canvas.restore(&background, at(10, 20), 3, 2);
        assert_eq!(canvas.pixels()[at(10, 20)], background.pixels[at(10, 20)]);
        assert_eq!(canvas.pixels()[at(12, 21)], background.pixels[at(12, 21)]);
        assert_eq!(canvas.pixels()[at(13, 21)], 0, "only the region");
        canvas.copy_rows(&background, 100, 2);
        assert_eq!(
            canvas.pixels()[at(639, 101)],
            background.pixels[at(639, 101)]
        );
        assert_eq!(canvas.pixels()[at(0, 102)], 0);
    }
}
