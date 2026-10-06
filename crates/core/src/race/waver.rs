//! The effect power-up's view (kind 4, `sub_404730`): while the player's count of it runs
//! down, the frame goes onto the screen wavering, the track's view in 2x2 blocks each pulled
//! from up to 4 pixels across and 8 down or up, by sine waves running across and down the
//! view and moving on 3 steps a frame. The HUD left of the view is shown as it is.

use super::buffer::Buffer;
use super::raster::ftol;
use super::{VIEW_HEIGHT, VIEW_WIDTH};
use crate::trig::sin;

/// The waves' table (`0x4A6854`, filled at the game's start, 0x4046C0): 3600 sines of steps
/// of 0.0175 radians, times 1024; the waver reads its first thousand.
const ENTRIES: usize = 3600;
/// Where the waver reads its second wave, 360 entries on.
const SECOND: i32 = 360;
/// The waves' steps run round 360 places, the accumulated steps (in 1/1024ths) round
/// 360 << 10.
const ROUND: i32 = 360;
const ROUND_STEPS: i32 = 360 << 10;

/// The waves' table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Waves(Vec<i32>);

impl Default for Waves {
    fn default() -> Waves {
        Waves(
            (0..ENTRIES)
                .map(|i| ftol(sin(i as f64 * 0.0175) * 1024.0))
                .collect(),
        )
    }
}

impl Waves {
    fn at(&self, index: i32) -> i32 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.0.get(index))
            .copied()
            .unwrap_or(0)
    }
}

/// `sub_404730`: the frame in `buffer` onto `screen` (320x200) wavering right of the HUD's
/// `left` columns, the waves' `phase` (0x456AF4) moved on 3 first.
pub(crate) fn show(screen: &mut [u8], buffer: &Buffer, waves: &Waves, phase: &mut i32, left: i32) {
    *phase += 3;
    if *phase >= ROUND {
        *phase -= ROUND;
    }
    let p = *phase;
    let row_step = 2 * waves.at(2 * p) + 4000;
    let column_step = 2 * waves.at(p + 125) + 4000;
    let left_columns = usize::try_from(left).unwrap_or(0).min(VIEW_WIDTH);
    for y in 0..VIEW_HEIGHT {
        for x in 0..left_columns {
            screen[y * VIEW_WIDTH + x] = buffer.pixel(x, y);
        }
    }
    // The rows' wave starts from the phase itself, not the phase in 1/1024ths.
    let mut across = p;
    for (row, y) in (0..VIEW_HEIGHT as i32).step_by(2).enumerate() {
        across += row_step;
        if across >= ROUND_STEPS {
            across -= ROUND_STEPS;
        }
        let mut down = waves.at(2 * p + row as i32 + 1) << 8;
        let mut k = p + 75;
        for x in (left..VIEW_WIDTH as i32).step_by(2) {
            k += 1;
            if k >= ROUND {
                k -= ROUND;
            }
            let dy = waves.at(SECOND + (((waves.at(k) << 8) + across) >> 10)) >> 7;
            let dx = waves.at(SECOND + (down >> 10)) >> 8;
            down += column_step;
            if down >= ROUND_STEPS {
                down -= ROUND_STEPS;
            }
            let (from_x, from_y) = (x + dx, y + dy);
            let inside = from_x >= left
                && from_x < VIEW_WIDTH as i32 - 1
                && from_y >= 0
                && from_y < VIEW_HEIGHT as i32 - 1;
            for (oy, ox) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let pixel = if inside {
                    buffer.pixel((from_x + ox) as usize, (from_y + oy) as usize)
                } else {
                    0
                };
                screen[(y + oy) as usize * VIEW_WIDTH + (x + ox) as usize] = pixel;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::race::buffer::{LEFT, STRIDE};

    /// A frame whose every shown pixel says where it is.
    fn frame() -> Buffer {
        let mut buffer = Buffer::default();
        for y in 0..VIEW_HEIGHT {
            for x in 0..VIEW_WIDTH {
                buffer.put((y * STRIDE + LEFT + x) as i64, colour(x, y));
            }
        }
        buffer
    }

    fn colour(x: usize, y: usize) -> u8 {
        ((x + 3 * y) % 251 + 1) as u8
    }

    /// The waves are the original's: sines of 0.0175-radian steps, not degrees, so the table
    /// turns negative just past entry 179; another table would waver the view differently.
    #[test]
    fn the_waves_are_sines_of_steps_of_0_0175_radians() {
        let waves = Waves::default();
        assert_eq!(&waves.0[..4], [0, 17, 35, 53]);
        assert_eq!((waves.0[90], waves.0[180], waves.0[270]), (1023, -8, -1023));
    }

    /// While the effect lasts the player sees the track's view in 2x2 blocks pulled from
    /// elsewhere (here 3 pixels left and 2 up), blocks pulled from beyond the view black, and
    /// the HUD steady.
    #[test]
    fn the_view_wavers_in_blocks_and_the_hud_stays() {
        let buffer = frame();
        let mut screen = vec![0xFF; VIEW_WIDTH * VIEW_HEIGHT];
        let mut phase = 0;
        show(&mut screen, &buffer, &Waves::default(), &mut phase, 64);
        assert_eq!(phase, 3);
        let at = |x: usize, y: usize| screen[y * VIEW_WIDTH + x];
        assert_eq!(at(200, 100), colour(197, 98));
        assert_eq!(at(201, 101), colour(198, 99));
        assert_eq!(at(64, 0), 0, "pulled from above the view");
        assert_eq!(at(65, 1), 0);
        assert_eq!(at(63, 0), colour(63, 0), "the HUD");
        assert_eq!(at(10, 150), colour(10, 150));
    }

    /// The waves move on 3 steps a frame and come round after 120 frames.
    #[test]
    fn the_waves_move_on_each_frame() {
        let buffer = frame();
        let mut screen = vec![0; VIEW_WIDTH * VIEW_HEIGHT];
        let mut phase = 357;
        show(&mut screen, &buffer, &Waves::default(), &mut phase, 64);
        assert_eq!(phase, 0);
        assert_eq!(screen[100 * VIEW_WIDTH + 200], colour(197, 98));
    }
}
