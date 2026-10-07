//! A frame as the original's window shows it with `-window -nogl`, so it can be compared with
//! the reference runner's screenshots (spec M1a §8 and §9, M7): the window is always 640x480
//! and its software path is `Frame::write_window_rgba`. (Its default OpenGL path stretches
//! 320x200 over the whole window instead, as DeadRally's frontend does without `-nogl`.)

use deadrally_core::{Frame, WINDOW_HEIGHT, WINDOW_WIDTH};

use crate::rgb::Rgb;

pub const WIDTH: u32 = WINDOW_WIDTH;
pub const HEIGHT: u32 = WINDOW_HEIGHT;

/// The window's picture of `frame`, smoothed as `-smooth` smooths it when `smooth`.
pub fn present(frame: &Frame<'_>, smooth: bool) -> Result<Rgb, String> {
    let mut rgba = vec![0; (WIDTH * HEIGHT * 4) as usize];
    frame.write_window_rgba(smooth, &mut rgba)?;
    Ok(Rgb {
        width: WIDTH,
        height: HEIGHT,
        pixels: rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, _]| [r, g, b])
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_the_windows_without_alpha() {
        // The screenshots are RGB; a stray alpha byte would shift every pixel after the first.
        let mut palette = [[0; 3]; 256];
        palette[7] = [1, 2, 3];
        let pixels = vec![7; 640 * 480];
        let frame = Frame {
            width: 640,
            height: 480,
            pixels: &pixels,
            palette: &palette,
            aspect: (4, 3),
        };
        let window = present(&frame, false).unwrap();
        assert_eq!(window.pixels.len(), 640 * 480 * 3);
        assert_eq!(window.pixels[..6], [4, 8, 12, 4, 8, 12]);
    }
}
