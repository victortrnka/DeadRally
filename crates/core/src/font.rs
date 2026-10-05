//! The original's bitmap fonts (`drawTextWithFont`, 0x41A2D0; spec M2a §3.1): glyph `c - 32` of
//! a sheet of equal cells, drawn with colour 0 transparent, the pen moving by the glyph's
//! advance from `dr.exe`'s metrics.

use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::{GAP, Metrics};

use crate::canvas::Canvas;

#[derive(Clone, Debug)]
pub(crate) struct Font {
    glyphs: Vec<Image>,
    advances: Vec<u8>,
}

impl Font {
    pub(crate) fn new(glyphs: Vec<Image>, metrics: &Metrics) -> Font {
        Font {
            glyphs,
            advances: metrics.advances.clone(),
        }
    }

    /// The glyph and advance of byte `c`, if the font has one (characters 32 onwards).
    fn glyph(&self, c: u8) -> Option<(&Image, usize)> {
        let index = usize::from(c.checked_sub(32)?);
        Some((
            self.glyphs.get(index)?,
            usize::from(*self.advances.get(index)?),
        ))
    }

    /// Draws `text` with the pen starting at `offset`; returns where the pen ends.
    pub(crate) fn draw(&self, canvas: &mut Canvas, text: &[u8], offset: usize) -> usize {
        let mut pen = offset;
        for &c in text {
            if c == GAP {
                pen += 1;
            } else if let Some((glyph, advance)) = self.glyph(c) {
                canvas.draw(glyph, pen, true);
                pen += advance;
            }
        }
        pen
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::at;

    /// A 2x1 font: glyph `k` is filled with colour `k + 1`, its left pixel transparent for
    /// odd `k`; every advance is 3.
    pub(crate) fn font() -> Font {
        let glyphs = (0..96u8)
            .map(|k| {
                let left = if k % 2 == 1 { 0 } else { k + 1 };
                Image::new(2, 1, vec![left, k + 1])
            })
            .collect();
        Font::new(
            glyphs,
            &Metrics {
                width: 2,
                height: 1,
                advances: vec![3; 96],
            },
        )
    }

    #[test]
    fn each_byte_draws_its_glyph_and_moves_the_pen_by_its_advance() {
        // A wrong glyph offset prints the neighbouring letter; a wrong advance spaces the text.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, b" !\"", at(0, 0));
        assert_eq!(end, 9);
        assert_eq!(&canvas.pixels()[..8], [1, 1, 0, 0, 2, 0, 3, 3]);
    }

    #[test]
    fn the_gap_byte_moves_the_pen_one_pixel_and_draws_nothing() {
        // The menu table pads rows with 0xFA to place text a pixel at a time.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, &[GAP, b' '], at(0, 0));
        assert_eq!(end, 4);
        assert_eq!(&canvas.pixels()[..3], [0, 1, 1]);
    }

    #[test]
    fn bytes_without_a_glyph_draw_nothing_and_do_not_move_the_pen() {
        let mut canvas = Canvas::default();
        assert_eq!(font().draw(&mut canvas, &[7, 200], at(5, 1)), at(5, 1));
        assert!(canvas.pixels().iter().all(|&p| p == 0));
    }
}
