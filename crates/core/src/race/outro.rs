//! The race's end (`sub_4055A0`, once the race loop has ended): the intro's zoom the other
//! way. The track's view, turned upside down and drawn from its bottom, tilts away to
//! edge-on while the colours fade to black and the HUD slides out to the left; then the
//! screen is cleared.

use deadrally_gamedata::image::Palette;

use super::intro::{
    HEIGHT, HUD, HUD_PER_DEGREE, NINETIETH, STEP_GROWTH, STEP_START, VIEW, WIDTH, tilt, view_row,
};
use super::raster::ftol;

/// The factor the tilt starts from and ends at (0x4450A0 set to 90; 0x441654).
const FACTOR_START: f32 = 90.0;
const FACTOR_END: f32 = 1.0;

#[derive(Debug)]
pub(crate) struct Outro {
    /// The race's palette over 90 (0x4B3400).
    table: [[f32; 3]; 256],
    /// The track's view turned upside down, and a blank row (0x481E20).
    view: Vec<u8>,
    /// The HUD, 200 rows of 64 (0x48E720).
    hud: Vec<u8>,
    screen: Vec<u8>,
    palette: Palette,
    factor: f32,
    step: f32,
    /// The HUD's width at the last frame (0x4AA504).
    hud_width: i32,
    /// The sound's volume as the tilt last set it.
    volume: u32,
}

impl Outro {
    /// The end over the screen as shown (`screen`, 320x200), its colours from `base`, from
    /// the race's last frame: `view` its track's view (200 rows of 256) and `hud` its HUD
    /// (200 rows of 64); run to its first wait.
    pub(crate) fn new(base: &Palette, screen: &[u8], view: &[u8], hud: &[u8]) -> Outro {
        let ninetieth = f64::from(f32::from_bits(NINETIETH));
        let mut table = [[0.0; 3]; 256];
        for (entry, colour) in table.iter_mut().zip(&base.0) {
            *entry = colour.map(|c| (f64::from(c) * ninetieth) as f32);
        }
        let mut turned: Vec<u8> = view
            .chunks(VIEW)
            .take(HEIGHT)
            .rev()
            .flatten()
            .copied()
            .collect();
        turned.resize((HEIGHT + 1) * VIEW, 0);
        let mut outro = Outro {
            table,
            view: turned,
            hud: hud.to_vec(),
            screen: screen.to_vec(),
            palette: base.clone(),
            factor: FACTOR_START,
            step: STEP_START,
            hud_width: HUD as i32,
            volume: 0,
        };
        outro.zoom(0);
        outro
    }

    pub(crate) fn screen(&self) -> &[u8] {
        &self.screen
    }

    pub(crate) fn palette(&self) -> &Palette {
        &self.palette
    }

    /// The sound's volume, falling with the tilt: the factor times 728, at most 0xFFFF.
    pub(crate) fn volume(&self) -> u32 {
        self.volume
    }

    /// The next frame, a tick after the last; false once the tilt has reached edge-on.
    pub(crate) fn wait(&mut self) -> bool {
        self.zoom(1)
    }

    /// A frame at the current factor (0x405980), then, short of edge-on, the factor's next
    /// step down, `ticks` after the last.
    fn zoom(&mut self, ticks: u32) -> bool {
        let factor = f64::from(self.factor);
        for (entry, colour) in self.table.iter().enumerate() {
            self.palette.0[entry] = colour.map(|c| (factor * f64::from(c)) as u8);
        }
        self.volume = ((factor as u32) * 728).min(0xFFFF);
        self.draw(factor);
        if self.factor == FACTOR_END {
            return false;
        }
        let mut step = f64::from(self.step);
        for _ in 0..=ticks {
            step *= STEP_GROWTH;
        }
        self.step = step as f32;
        let left = (factor - step) as f32;
        self.factor = if left < FACTOR_END { FACTOR_END } else { left };
        true
    }

    /// The frame: the HUD's right part, `factor * 5/7` wide, at the left, the columns it left
    /// since the last frame cleared; the turned view tilted `factor` degrees from edge-on,
    /// from its bottom up.
    fn draw(&mut self, factor: f64) {
        let width = ftol(factor * HUD_PER_DEGREE);
        let left = usize::try_from(width).unwrap_or(0).min(HUD);
        for y in 0..HEIGHT {
            let from = y * HUD + HUD - left;
            self.screen[y * WIDTH..y * WIDTH + left].copy_from_slice(&self.hud[from..from + left]);
            let cleared = usize::try_from(self.hud_width - width).unwrap_or(0);
            let end = (left + cleared).min(WIDTH);
            self.screen[y * WIDTH + left..y * WIDTH + end].fill(0);
        }
        self.hud_width = width;
        let (row_step, narrowing) = tilt(factor);
        let mut down = row_step.wrapping_mul(HEIGHT as i32);
        for y in 0..HEIGHT {
            view_row(&mut self.screen, &self.view, y, down, narrowing);
            down = down.wrapping_sub(row_step);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        let mut palette = Palette::BLACK;
        for (i, entry) in palette.0.iter_mut().enumerate() {
            *entry = [(i % 64) as u8, ((i * 7) % 64) as u8, ((i * 13) % 64) as u8];
        }
        palette
    }

    fn outro() -> Outro {
        let view: Vec<u8> = (0..HEIGHT * VIEW).map(|i| (i % 251) as u8 + 1).collect();
        let hud: Vec<u8> = (0..HEIGHT * HUD).map(|i| (i % 13) as u8 + 1).collect();
        Outro::new(&palette(), &vec![7; WIDTH * HEIGHT], &view, &hud)
    }

    /// The tilt takes the intro's zoom's 42 frames the other way: the race ends 41 frames after
    /// it starts to tilt away, the last at a factor of 1 cleared at once.
    #[test]
    fn the_view_tilts_away_in_the_zooms_frames() {
        let mut outro = outro();
        let mut waits = 0;
        while outro.wait() {
            waits += 1;
        }
        assert_eq!(waits, 40);
    }

    /// The first frame shows the race as it was, but for colours a step darker where the
    /// float of a ninetieth rounds down, the HUD whole, the view upright but a row lower, its
    /// top row black (the turned view is drawn from its bottom, starting past its last row).
    #[test]
    fn the_first_frame_shows_the_race_a_row_lower() {
        let view: Vec<u8> = (0..HEIGHT * VIEW).map(|i| (i % 251) as u8 + 1).collect();
        let hud: Vec<u8> = (0..HEIGHT * HUD).map(|i| (i % 13) as u8 + 1).collect();
        let outro = Outro::new(&palette(), &vec![7; WIDTH * HEIGHT], &view, &hud);
        for (shown, was) in outro.palette().0.iter().zip(palette().0) {
            for (c, w) in shown.iter().zip(was) {
                assert!(*c == w || *c + 1 == w, "{shown:?} {was:?}");
            }
        }
        // (7, 49, 27): 49 over 90 as a float is a little short.
        assert_eq!(outro.palette().0[7], [7, 48, 27]);
        let screen = outro.screen();
        for y in [0, 100, 199] {
            assert_eq!(
                &screen[y * WIDTH..y * WIDTH + HUD],
                &hud[y * HUD..(y + 1) * HUD]
            );
        }
        assert!(screen[HUD..WIDTH - 1].iter().all(|&p| p == 0));
        for y in [1, 100, 199] {
            let row = &screen[y * WIDTH + HUD..(y + 1) * WIDTH - 1];
            assert_eq!(row, &view[(y - 1) * VIEW..y * VIEW - 1], "row {y}");
        }
    }

    /// The colours fade with the tilt to 1/90 of themselves, black, and the sound with them;
    /// the HUD slides out, the columns it leaves cleared.
    #[test]
    fn the_colours_and_the_hud_go_with_the_tilt() {
        let mut outro = outro();
        assert_eq!(outro.volume(), 90 * 728);
        while outro.wait() {}
        assert!(outro.palette().0.iter().all(|&rgb| rgb == [0, 0, 0]));
        assert_eq!(outro.volume(), 728);
        let screen = outro.screen();
        assert!((0..HEIGHT).all(|y| screen[y * WIDTH..y * WIDTH + HUD].iter().all(|&p| p == 0)));
    }
}
