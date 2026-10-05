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

    /// Copies `other`'s pixels into this canvas where `mask` (placed at `offset`) is not 0
    /// (0x43B080, the Hall of Fame's wipe).
    pub(crate) fn blit_mask(&mut self, mask: &Image, other: &Canvas, offset: usize) {
        let width = (mask.width as usize).max(1);
        for (row, line) in mask.pixels.chunks(width).enumerate() {
            for (column, &cover) in line.iter().enumerate() {
                let at = offset + row * WIDTH + column;
                if cover != 0 && at < self.pixels.len() {
                    self.pixels[at] = other.pixels[at];
                }
            }
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

    #[test]
    fn a_masked_copy_takes_the_other_canvas_where_the_mask_is_set() {
        // The wipe's tiles: a pixel of the new screen shows only where the mask has one.
        let mut old = Canvas::default();
        let mut new = Canvas::default();
        new.fill(0, WIDTH, 2, 9);
        old.blit_mask(&Image::new(3, 2, vec![1, 0, 1, 0, 1, 0]), &new, at(10, 0));
        assert_eq!(&old.pixels()[at(10, 0)..at(13, 0)], [9, 0, 9]);
        assert_eq!(&old.pixels()[at(10, 1)..at(13, 1)], [0, 9, 0]);
    }

    #[test]
    fn an_empty_mask_copies_nothing_instead_of_crashing() {
        // Damaged data could hold a mask without columns; `draw` already copes with that.
        let mut old = Canvas::default();
        old.blit_mask(&Image::new(0, 0, Vec::new()), &Canvas::default(), 0);
        assert!(old.pixels().iter().all(|&p| p == 0));
    }
}
