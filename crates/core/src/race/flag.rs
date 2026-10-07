//! The chequered flag at the view's top right once a car has finished (`sub_402490`): the
//! race's loop draws it after the HUD for each car in first place while the race is over for
//! the cars (0x417258, 0x456AC8). Its 64 pictures (`GEN-FLA.BPK`, 80x60) wave half a picture
//! a tick, and it slides in from 80 pixels right of its place a pixel a tick.

use super::buffer::Buffer;

/// The pictures' size and count, and where the flag stands in the buffer (0x150: the view's
/// columns 240 to 319 of row 0).
const WIDTH: usize = 80;
const HEIGHT: usize = 60;
const PICTURE: usize = WIDTH * HEIGHT;
const LAST: i32 = 63;
const AT: i64 = 0x150;
/// How far right it starts (0x4161F1).
const SLIDE: i32 = 80;

/// The flag's pictures, the one it shows (16.16, 0x481BE0) and how far right of its place it
/// still is (0x4B3140).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Flag {
    pictures: Vec<u8>,
    at: i32,
    slide: i32,
}

impl Flag {
    /// The flag as the race's set-up leaves it (0x4161F6): on its first picture, 80 pixels
    /// right of its place.
    pub(super) fn new(pictures: Vec<u8>) -> Flag {
        Flag {
            pictures,
            at: 0,
            slide: SLIDE,
        }
    }

    /// A frame `between` ticks after the last: `between` pixels nearer its place, its picture
    /// drawn whole, then half a picture on for each tick, back to the first past the last.
    pub(super) fn draw(&mut self, buffer: &mut Buffer, between: i32) {
        let slide = self.slide - between;
        self.slide = if slide <= 0 { 0 } else { slide };
        let start = usize::try_from(self.at.wrapping_add(0x8000) >> 16).unwrap_or(0) * PICTURE;
        let picture = self.pictures.get(start..start + PICTURE).unwrap_or(&[]);
        buffer.draw_opaque(picture, WIDTH, HEIGHT, AT + i64::from(self.slide));
        self.at = self.at.wrapping_add(between << 15);
        if (self.at.wrapping_add(0x8000) & !0xFFFF) > LAST << 16 {
            self.at = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Picture `k` of the flag, colour `k + 1`.
    fn pictures() -> Vec<u8> {
        (0..64u8).flat_map(|k| [k + 1; PICTURE]).collect()
    }

    /// The flag slides in from the right a pixel a tick and waves half a picture a tick, round
    /// its 64 pictures: one standing still or waving at another pace shows the race over
    /// differently from the original.
    #[test]
    fn the_flag_slides_in_and_waves_half_a_picture_a_tick() {
        let mut flag = Flag::new(pictures());
        let mut buffer = Buffer::default();
        flag.draw(&mut buffer, 2);
        assert_eq!(buffer.pixel(240 + 78, 0), 1, "78 pixels right, picture 0");
        assert_eq!(buffer.pixel(240 + 77, 0), 0, "nothing left of it");
        flag.draw(&mut buffer, 2);
        assert_eq!(buffer.pixel(240 + 76, 0), 2, "picture 1 after two ticks");
        for _ in 0..38 {
            flag.draw(&mut buffer, 2);
        }
        assert_eq!(buffer.pixel(240, 59), 40, "in its place, picture 39");
        for _ in 0..24 {
            flag.draw(&mut buffer, 2);
        }
        assert_eq!(buffer.pixel(240, 0), 64, "the last picture");
        flag.draw(&mut buffer, 2);
        assert_eq!(buffer.pixel(240, 0), 1, "round to the first");
    }
}
