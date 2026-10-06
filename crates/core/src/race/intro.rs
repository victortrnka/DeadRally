//! The race's intro (`sub_404C30`, run once the race loop has drawn its first frame): the
//! track's view tilts up from edge-on as it fades in from black, in grey but for the player's
//! car, while the HUD slides in from the left; the player's car flashes; then the colours come
//! back. It draws straight into the 320x200 screen, and it leaves the palette a little off the
//! track's: its last steps stop at 248/256 of the colours, 250/256 for the player's car.

use deadrally_gamedata::image::Palette;

/// The screen's size, and the view's and the HUD's widths on it.
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const VIEW: usize = 256;
const HUD: usize = 64;

/// 1/90 as the f32 at 0x441628: the zoom keeps its colours divided by 90.
const NINETIETH: u32 = 0x3C36_0B61;
/// The zoom's factor: from 1 up to 90 (degrees of tilt, and 90ths of the colours).
const FACTOR_END: f32 = 90.0;
/// What the factor's step starts at and grows by a tick (0x44509C, 0x4415D8).
const STEP_START: f32 = 0.9;
const STEP_GROWTH: f64 = 1.02;
/// Degrees to radians as the original has it (0x4412B0), and the HUD's width a degree
/// (0x4415E8, 5/7).
const RADIANS: f64 = 0.017_453_292_519_944_444;
const HUD_PER_DEGREE: f64 = 0.714_285_714_285_714_3;
/// The flash's top, 1.7 times the colour (0x4415D0 negated).
const FLASH: f64 = 1.7;

/// The grey a colour fades from (0x404CE3): `(b·11.3 + r·29.9 + g·58.8) · 0.01` truncated.
fn grey([r, g, b]: [u8; 3]) -> i32 {
    ((f64::from(b) * 11.3 + f64::from(r) * 29.9 + f64::from(g) * 58.8) * 0.01) as i32
}

/// Where the intro is: at one of its waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// After a frame of the zoom.
    Zoom,
    /// The zoom done, the waits left before the flash.
    Hold(u32),
    /// The flash rising, at `u`/256 of its way up.
    Flash(i32),
    /// The flash settling, at `t`/256 of its way down.
    Settle(i32),
    /// The flash done, the waits left before the colours.
    Pause(u32),
    /// The colours coming back, at `t`/256.
    Colour(i32),
}

#[derive(Debug)]
pub(crate) struct Intro {
    /// The race's palette as the intro found it (0x509E60).
    base: Palette,
    /// The player's car's ten palette entries.
    car: std::ops::RangeInclusive<usize>,
    /// The palette over 90 (0x4B3400), in grey but for the player's car.
    table: [[f32; 3]; 256],
    /// The track's view, 200 rows of 256 and a blank row (0x481E20).
    view: Vec<u8>,
    /// The HUD, 200 rows of 64 (0x48E720).
    hud: Vec<u8>,
    screen: Vec<u8>,
    palette: Palette,
    factor: f32,
    step: f32,
    phase: Phase,
}

impl Intro {
    /// The intro over the race's first frame, `view` its track's view (200 rows of 256) and
    /// `hud` its HUD (200 rows of 64), the player in place `player`; run to its first wait.
    pub(crate) fn new(base: &Palette, player: usize, view: &[u8], hud: &[u8]) -> Intro {
        let first = 10 * player + 15;
        let car = first..=first + 9;
        let ninetieth = f64::from(f32::from_bits(NINETIETH));
        // The table is filled 32 times, a step of 8/256 more each time; the last stays.
        let scaled = |value: i32| (f64::from((value * 248 + 128) >> 8) * ninetieth) as f32;
        let mut table = [[0.0; 3]; 256];
        for (entry, colour) in table.iter_mut().enumerate() {
            let rgb = base.0[entry];
            *colour = if car.contains(&entry) {
                rgb.map(|c| scaled(i32::from(c)))
            } else {
                [scaled(grey(rgb)); 3]
            };
        }
        let mut view = view.to_vec();
        view.resize((HEIGHT + 1) * VIEW, 0);
        let mut intro = Intro {
            base: base.clone(),
            car,
            table,
            view,
            hud: hud.to_vec(),
            screen: vec![0; WIDTH * HEIGHT],
            palette: Palette::BLACK,
            factor: 1.0,
            step: STEP_START,
            phase: Phase::Zoom,
        };
        intro.zoom(0);
        intro
    }

    pub(crate) fn screen(&self) -> &[u8] {
        &self.screen
    }

    pub(crate) fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Runs from this wait to the next; false once the intro has ended instead.
    pub(crate) fn wait(&mut self) -> bool {
        self.phase = match self.phase {
            Phase::Zoom => return self.zoom(1),
            Phase::Hold(0) => self.flash(0),
            Phase::Hold(left) => Phase::Hold(left - 1),
            Phase::Flash(u) if u + 50 <= 256 => self.flash(u + 50),
            Phase::Flash(_) => self.settle(0),
            Phase::Settle(t) if t + 10 <= 256 => self.settle(t + 10),
            Phase::Settle(_) => Phase::Pause(49),
            Phase::Pause(0) => self.colour(0),
            Phase::Pause(left) => Phase::Pause(left - 1),
            Phase::Colour(t) if t + 8 < 256 => self.colour(t + 8),
            Phase::Colour(_) => return false,
        };
        true
    }

    /// A frame of the zoom at the current factor, `ticks` after the last; then the factor's
    /// next step, or the first of the 20 waits once the factor has reached 90.
    fn zoom(&mut self, ticks: u32) -> bool {
        let factor = f64::from(self.factor);
        for (entry, colour) in self.table.iter().enumerate() {
            self.palette.0[entry] = colour.map(|c| (factor * f64::from(c)) as u8);
        }
        self.draw(factor);
        if self.factor == FACTOR_END {
            self.phase = Phase::Hold(19);
            return true;
        }
        let mut step = f64::from(self.step);
        for _ in 0..=ticks {
            step *= STEP_GROWTH;
        }
        self.step = step as f32;
        let sum = step + factor;
        self.factor = if sum > f64::from(FACTOR_END) {
            FACTOR_END
        } else {
            sum as f32
        };
        true
    }

    /// The zoom's frame (0x404E74): the HUD's right part, `factor * 5/7` wide, at the left;
    /// the view tilted `factor` degrees from edge-on, a row of the screen `256 / sin` 256ths of
    /// a row of the view further down it, each row narrowed by `170 * cos` 65536ths of its
    /// distance down and centred, black beyond the view's last row.
    fn draw(&mut self, factor: f64) {
        let width = (factor * HUD_PER_DEGREE) as usize;
        for y in 0..HEIGHT {
            let from = y * HUD + HUD - width;
            self.screen[y * WIDTH..y * WIDTH + width]
                .copy_from_slice(&self.hud[from..from + width]);
        }
        let angle = factor * RADIANS;
        let row_step = (51200.0 / (crate::trig::sin(angle) * 200.0)) as i32;
        let narrowing = (crate::trig::cos(angle) * 170.0) as i32;
        let mut down = 0i32;
        for y in 0..HEIGHT {
            let row = y * WIDTH + HUD;
            let from = (down & !0xFF).min((HEIGHT * VIEW) as i32);
            let narrow = (from * narrowing) >> 16;
            // Ten pixels cleared each side of the row, as far as the view's edges.
            for x in (narrow >> 1) - 10..narrow >> 1 {
                if x >= 0 {
                    self.screen[row + x as usize] = 0;
                }
            }
            // 256 pixels from the one before the row's start (`sub_43B370`), each written
            // where the last was until 256 steps of `255 - narrow` have passed.
            let mut x = (narrow >> 1) as usize;
            let mut passed = 0;
            for i in 0..VIEW as i32 {
                let at = from + i - 1;
                self.screen[row + x] = if at < 0 { 0 } else { self.view[at as usize] };
                passed += 255 - narrow;
                if passed & 0x100 != 0 {
                    passed &= 0xFF;
                    x += 1;
                }
            }
            let right = 255 - (narrow - (narrow >> 1));
            for x in right..(right + 10).min(VIEW as i32) {
                self.screen[row + x as usize] = 0;
            }
            down += row_step;
        }
    }

    /// The flash rising (0x405070): the car's colours `(c·(256-u) + 1.7·c·u) / 256`, at most 63.
    fn flash(&mut self, u: i32) -> Phase {
        self.car_step(256 - u, u);
        Phase::Flash(u)
    }

    /// The flash settling (0x4051A0): the car's colours `(c·t + 1.7·c·(256-t)) / 256`.
    fn settle(&mut self, t: i32) -> Phase {
        self.car_step(t, 256 - t);
        Phase::Settle(t)
    }

    /// The car's colours at `plain`/256 of themselves and `flash`/256 of 1.7 times themselves.
    fn car_step(&mut self, plain: i32, flash: i32) {
        for entry in self.car.clone() {
            self.palette.0[entry] = self.base.0[entry].map(|c| {
                let bright = (f64::from(c) * f64::from(flash) * -FLASH) as i32;
                ((i32::from(c) * plain - bright + 0x80) >> 8).min(63) as u8
            });
        }
    }

    /// The colours coming back (0x4052F0): every entry but the car's at `t`/256 of its colour
    /// and the rest of its grey.
    fn colour(&mut self, t: i32) -> Phase {
        for entry in 0..256 {
            if self.car.contains(&entry) {
                continue;
            }
            let rgb = self.base.0[entry];
            let grey = grey(rgb) * (256 - t);
            self.palette.0[entry] = rgb.map(|c| ((i32::from(c) * t + grey + 0x80) >> 8) as u8);
        }
        Phase::Colour(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A track's palette with every entry its own colour, the player's at 20.
    fn base() -> Palette {
        let mut palette = Palette::BLACK;
        for (i, entry) in palette.0.iter_mut().enumerate() {
            *entry = [(i % 64) as u8, ((i * 7) % 64) as u8, ((i * 13) % 64) as u8];
        }
        palette.0[20] = [32, 49, 8];
        palette.0[100] = [32, 5, 1];
        palette
    }

    fn intro() -> Intro {
        let view: Vec<u8> = (0..HEIGHT * VIEW).map(|i| (i % 251) as u8 + 1).collect();
        let hud: Vec<u8> = (0..HEIGHT * HUD).map(|i| (i % 13) as u8 + 1).collect();
        Intro::new(&base(), 0, &view, &hud)
    }

    /// The intro's waits after the one `new` stops at.
    fn run(intro: &mut Intro) -> u32 {
        let mut waits = 0;
        while intro.wait() {
            waits += 1;
        }
        waits
    }

    /// 42 frames of the zoom (41 waits between them), 20 waits, the flash's 6 and 26 steps,
    /// 50 waits and the colours' 32 steps: the countdown starts 2.5 s after the race appears,
    /// as the original's does.
    #[test]
    fn the_intro_lasts_175_waits() {
        let mut intro = intro();
        assert_eq!(run(&mut intro) + 1, 175);
    }

    /// The race appears from black: the zoom's first frame shows its palette at 1/90.
    #[test]
    fn the_race_appears_from_black() {
        assert_eq!(intro().palette(), &Palette::BLACK);
    }

    /// The player's car flashes to 1.7 times its colour and settles back, its last step at
    /// 250/256 with the flash's rounding left in: (32, 49, 8) comes out (33, 50, 8), as the
    /// original shows the car in the race.
    #[test]
    fn the_players_car_comes_out_of_the_flash_a_step_brighter() {
        let mut intro = intro();
        run(&mut intro);
        assert_eq!(intro.palette().0[20], [33, 50, 8]);
    }

    /// The colours come back from grey in steps of 8/256 and stop at 248/256: (32, 5, 1),
    /// grey 12, comes out (31, 5, 1), as the original shows the track in the race.
    #[test]
    fn the_colours_stop_short_of_the_tracks() {
        let mut intro = intro();
        run(&mut intro);
        assert_eq!(intro.palette().0[100], [31, 5, 1]);
    }

    /// The zoom's last frame shows the view upright, its 256 pixels squeezed into 255 (the
    /// last column black), and the HUD slid fully in.
    #[test]
    fn the_zoom_ends_on_the_whole_view_and_hud() {
        let view: Vec<u8> = (0..HEIGHT * VIEW).map(|i| (i % 251) as u8 + 1).collect();
        let hud: Vec<u8> = (0..HEIGHT * HUD).map(|i| (i % 13) as u8 + 1).collect();
        let mut intro = Intro::new(&base(), 0, &view, &hud);
        for _ in 0..41 {
            assert!(intro.wait());
        }
        let screen = intro.screen();
        for y in [0, 99, 199] {
            let row = &screen[y * WIDTH..(y + 1) * WIDTH];
            assert_eq!(&row[..HUD], &hud[y * HUD..(y + 1) * HUD], "row {y}");
            assert_eq!(
                &row[HUD..WIDTH - 1],
                &view[y * VIEW..(y + 1) * VIEW - 1],
                "row {y}"
            );
            assert_eq!(row[WIDTH - 1], 0, "row {y}");
        }
    }

    /// At the zoom's first frame the track lies nearly edge-on: only its first 4 rows show,
    /// the first of them whole, the HUD not yet in.
    #[test]
    fn the_zoom_starts_with_the_track_edge_on() {
        let intro = intro();
        let screen = intro.screen();
        assert!(screen[..WIDTH - 1].iter().skip(HUD).all(|&p| p != 0));
        assert!(screen[3 * WIDTH + HUD..4 * WIDTH].iter().any(|&p| p != 0));
        assert!(screen[4 * WIDTH..].iter().all(|&p| p == 0));
    }
}
