//! A frame as the original's window shows it with `-window -nogl`, so it can be compared with
//! the reference runner's screenshots (spec M1a §8 and §9).
//!
//! The window is always 640x480 (`SDL_SetVideoMode(640, 480, 32, ...)`). Its software path,
//! `refreshScreen` (0x43B580), copies 640x480 screens as they are and doubles 320x200 screens
//! to 640x400 from row 40, leaving black above and below. (Its default OpenGL path stretches
//! 320x200 over the whole window instead, as DeadRally's frontends do.)

use deadrally_core::{Frame, expand_6bit};

use crate::rgb::Rgb;

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;
const LOW_RES_TOP: u32 = 40;

/// The window's picture of `frame`.
pub fn present(frame: &Frame<'_>) -> Result<Rgb, String> {
    let colours: Vec<[u8; 3]> = frame
        .palette
        .iter()
        .map(|rgb| rgb.map(expand_6bit))
        .collect();
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 3) as usize);
    match (frame.width, frame.height) {
        (WIDTH, HEIGHT) => {
            pixels.extend(
                frame
                    .pixels
                    .iter()
                    .flat_map(|&index| colours[usize::from(index)]),
            );
        }
        (320, 200) => {
            let band = (LOW_RES_TOP * WIDTH * 3) as usize;
            pixels.resize(band, 0);
            for row in frame.pixels.as_chunks::<320>().0 {
                let start = pixels.len();
                pixels.extend(
                    row.iter()
                        .flat_map(|&index| [colours[usize::from(index)]; 2])
                        .flatten(),
                );
                pixels.extend_from_within(start..);
            }
            pixels.resize(pixels.len() + band, 0);
        }
        (width, height) => {
            return Err(format!("the original has no {width}x{height} screen"));
        }
    }
    Ok(Rgb {
        width: WIDTH,
        height: HEIGHT,
        pixels,
    })
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

    #[test]
    fn low_resolution_screens_are_doubled_between_black_bands() {
        // Measured on the original under Wine: the intro fills rows 40..440, two by two.
        let mut palette = [[0; 3]; 256];
        palette[1] = [63, 0, 0];
        let mut pixels = vec![0; 320 * 200];
        pixels[0] = 1;
        let window = present(&frame(320, 200, &pixels, &palette)).unwrap();
        let at = |x: u32, y: u32| {
            let start = ((y * WIDTH + x) * 3) as usize;
            [
                window.pixels[start],
                window.pixels[start + 1],
                window.pixels[start + 2],
            ]
        };
        assert_eq!(at(0, 39), [0, 0, 0]);
        for (x, y) in [(0, 40), (1, 40), (0, 41), (1, 41)] {
            assert_eq!(at(x, y), [252, 0, 0], "({x}, {y})");
        }
        assert_eq!(at(2, 40), [0, 0, 0]);
    }

    #[test]
    fn full_resolution_screens_are_copied_as_they_are() {
        let mut palette = [[0; 3]; 256];
        palette[7] = [1, 2, 3];
        let pixels = vec![7; 640 * 480];
        let window = present(&frame(640, 480, &pixels, &palette)).unwrap();
        assert!(
            window
                .pixels
                .as_chunks::<3>()
                .0
                .iter()
                .all(|rgb| *rgb == [4, 8, 12])
        );
    }

    #[test]
    fn other_sizes_are_rejected() {
        let palette = [[0; 3]; 256];
        let pixels = vec![0; 640 * 360];
        assert!(present(&frame(640, 360, &pixels, &palette)).is_err());
    }
}
