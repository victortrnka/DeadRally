//! The race's end (`sub_4055A0`, once the race loop has ended). With the HUD in, the intro's
//! zoom the other way: the track's view, turned upside down and drawn from its bottom, tilts
//! away to edge-on while the colours fade to black and the HUD slides out to the left. With
//! the HUD not all in (the status bar hidden), the whole frame spins away round its top left
//! corner instead. Then the screen is cleared.

use deadrally_gamedata::image::Palette;

use super::intro::{
    HEIGHT, HUD, HUD_PER_DEGREE, NINETIETH, STEP_GROWTH, STEP_START, VIEW, WIDTH, tilt, view_row,
};
use super::raster::ftol;

/// The factor the tilt starts from and ends at (0x4450A0 set to 90; 0x441654).
const FACTOR_START: f32 = 90.0;
const FACTOR_END: f32 = 1.0;

/// The view tilting away, or the frame spinning away.
#[derive(Debug)]
pub(crate) enum Outro {
    Tilt(Tilt),
    Spin(Spin),
}

impl Outro {
    /// The end over the screen as shown (`screen`, 320x200, in the colours `shown`), from the
    /// race's last frame in the buffer (`frame`, 320x200) and the race's palette `base`: the
    /// tilt when the HUD is in (`hud_width`, 0x456AA0, at 64), else the spin (0x4055BF).
    pub(crate) fn new(
        base: &Palette,
        (screen, shown): (&[u8], &Palette),
        frame: &[u8],
        hud_width: i64,
    ) -> Outro {
        if hud_width == HUD as i64 {
            let rows = || frame.chunks(WIDTH).take(HEIGHT);
            let view: Vec<u8> = rows().flat_map(|row| &row[HUD..]).copied().collect();
            let hud: Vec<u8> = rows().flat_map(|row| &row[..HUD]).copied().collect();
            Outro::Tilt(Tilt::new(base, screen, &view, &hud))
        } else {
            Outro::Spin(Spin::new(base, (screen, shown), frame))
        }
    }

    pub(crate) fn screen(&self) -> &[u8] {
        match self {
            Outro::Tilt(tilt) => tilt.screen(),
            Outro::Spin(spin) => &spin.screen,
        }
    }

    pub(crate) fn palette(&self) -> &Palette {
        match self {
            Outro::Tilt(tilt) => tilt.palette(),
            Outro::Spin(spin) => &spin.palette,
        }
    }

    /// The sound's volume as the end last set it, 0..=0xFFFF; none before its first frame.
    pub(crate) fn volume(&self) -> Option<u32> {
        match self {
            Outro::Tilt(tilt) => Some(tilt.volume()),
            Outro::Spin(spin) => spin.volume,
        }
    }

    /// The next frame, a tick after the last; false once the end's last frame is drawn, which
    /// the screen's clearing follows at once.
    pub(crate) fn wait(&mut self) -> bool {
        match self {
            Outro::Tilt(tilt) => tilt.wait(),
            Outro::Spin(spin) => spin.wait(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Tilt {
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

impl Tilt {
    /// The tilt over the screen as shown (`screen`, 320x200), its colours from `base`, from
    /// the race's last frame: `view` its track's view (200 rows of 256) and `hud` its HUD
    /// (200 rows of 64); run to its first wait.
    pub(crate) fn new(base: &Palette, screen: &[u8], view: &[u8], hud: &[u8]) -> Tilt {
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
        let mut outro = Tilt {
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

/// The spin's factor: from 8 (0x405EC5) up past 150 (0x441638) by 1.04 a tick (0x441630); its
/// colours at 150 less the factor over 150 (1/150 as the f32 at 0x441650); the sound at 436 a
/// step (0x405F44).
const SPIN_START: f32 = 8.0;
const SPIN_END: f64 = 150.0;
const SPIN_GROWTH: f64 = 1.04;
const HUNDRED_FIFTIETH: u32 = 0x3BDA_740E;
const SPIN_VOLUME: i32 = 0x1B4;
/// The spin's turns: 600 steps of 0.0104667 radians (0x441648), cosine and sine times 1024
/// (0x4415B0); the factor's whole part less 8 steps back from the last.
const TURNS: usize = 600;
const TURN_STEP: f64 = 0.010_466_666_666_666_668;
const TURN_SCALE: f64 = 1024.0;

/// The other way out (from 0x405C40): the race's last frame turned round its top left corner,
/// a square of two pixels at a time, from a little short of a whole turn to a quarter turn
/// back, while its colours and the sound fade. Each frame comes after a wait.
#[derive(Debug)]
pub(crate) struct Spin {
    /// The race's palette over 150 (0x4B3400).
    table: [[f32; 3]; 256],
    /// The race's last frame, 320x200 (0x481E20).
    frame: Vec<u8>,
    /// Each step's cosine and sine times 1024 (the tables at 0x46F204 and 0x4A6854).
    turns: Vec<(i32, i32)>,
    screen: Vec<u8>,
    palette: Palette,
    factor: f32,
    volume: Option<u32>,
}

impl Spin {
    /// The spin over the screen as shown (`screen`, in the colours `shown`), from the race's
    /// last frame (`frame`, 320x200) and its palette `base`; nothing drawn until its first
    /// wait.
    fn new(base: &Palette, (screen, shown): (&[u8], &Palette), frame: &[u8]) -> Spin {
        let fraction = f64::from(f32::from_bits(HUNDRED_FIFTIETH));
        let mut table = [[0.0; 3]; 256];
        for (entry, colour) in table.iter_mut().zip(&base.0) {
            *entry = colour.map(|c| (f64::from(c) * fraction) as f32);
        }
        let turns = (0..TURNS as i32)
            .map(|step| {
                let angle = f64::from(step) * TURN_STEP;
                (
                    ftol(crate::trig::cos(angle) * TURN_SCALE),
                    ftol(crate::trig::sin(angle) * TURN_SCALE),
                )
            })
            .collect();
        Spin {
            table,
            frame: frame.to_vec(),
            turns,
            screen: screen.to_vec(),
            palette: shown.clone(),
            factor: SPIN_START,
            volume: None,
        }
    }

    /// A frame at the current factor, then the factor's step for the tick waited and the one
    /// the frame took (0x40606A); false once the factor has reached 150.
    fn wait(&mut self) -> bool {
        let factor = f64::from(self.factor);
        let left = SPIN_END - factor;
        for (entry, colour) in self.table.iter().enumerate() {
            self.palette.0[entry] = colour.map(|c| ftol(f64::from(c) * left) as u8);
        }
        self.volume = Some((ftol(left) * SPIN_VOLUME) as u32);
        let step = TURNS as i32 - 1 - (ftol(factor) - SPIN_START as i32);
        let (cos, sin) = self.turns[step.clamp(0, TURNS as i32 - 1) as usize];
        self.draw(cos, sin);
        let mut next = factor;
        for _ in 0..2 {
            if next < SPIN_END {
                next *= SPIN_GROWTH;
            }
        }
        self.factor = next as f32;
        next < SPIN_END
    }

    /// The screen's squares of two pixels, each from the frame's square at the place it turns
    /// to, or black when that place is not inside the frame's outer rows and columns.
    fn draw(&mut self, cos: i32, sin: i32) {
        for row in 0..HEIGHT / 2 {
            let down = 2 * row as i32 + 1;
            for column in 0..WIDTH / 2 {
                let across = 2 * column as i32 + 1;
                let x = ((cos * across) >> 10) - ((sin * down) >> 10);
                let y = ((sin * across) >> 10) + ((cos * down) >> 10);
                let inside = x > 0 && x < WIDTH as i32 - 1 && y > 0 && y < HEIGHT as i32 - 1;
                for dy in 0..2 {
                    let to = (2 * row + dy) * WIDTH + 2 * column;
                    let square = &mut self.screen[to..to + 2];
                    if inside {
                        let from = (y as usize + dy) * WIDTH + x as usize;
                        square.copy_from_slice(&self.frame[from..from + 2]);
                    } else {
                        square.fill(0);
                    }
                }
            }
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

    fn outro() -> Tilt {
        let view: Vec<u8> = (0..HEIGHT * VIEW).map(|i| (i % 251) as u8 + 1).collect();
        let hud: Vec<u8> = (0..HEIGHT * HUD).map(|i| (i % 13) as u8 + 1).collect();
        Tilt::new(&palette(), &vec![7; WIDTH * HEIGHT], &view, &hud)
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
        let outro = Tilt::new(&palette(), &vec![7; WIDTH * HEIGHT], &view, &hud);
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

    /// A 320x200 frame whose every pixel tells where it was.
    fn frame() -> Vec<u8> {
        (0..HEIGHT * WIDTH)
            .map(|i| ((i % WIDTH * 7 + i / WIDTH * 13) % 250) as u8 + 1)
            .collect()
    }

    fn spin() -> Outro {
        let shown = Palette([[1, 2, 3]; 256]);
        Outro::new(&palette(), (&vec![7; WIDTH * HEIGHT], &shown), &frame(), 0)
    }

    /// With the HUD in, the race ends tilting away; with the status bar hidden (the HUD not all
    /// in), the original takes its other way out: nothing changes until a tick has passed,
    /// then 38 frames spin the race away, the last cleared at once.
    #[test]
    fn with_the_hud_hidden_the_race_spins_away_instead_of_tilting() {
        let tilt = Outro::new(&palette(), (&[7; WIDTH * HEIGHT], &palette()), &frame(), 64);
        assert!(matches!(tilt, Outro::Tilt(_)));
        let mut outro = spin();
        assert!(matches!(outro, Outro::Spin(_)));
        assert!(outro.screen().iter().all(|&p| p == 7));
        assert_eq!(outro.palette().0[9], [1, 2, 3]);
        assert_eq!(outro.volume(), None);
        let mut frames = 1;
        while outro.wait() {
            frames += 1;
        }
        assert_eq!(frames, 38);
    }

    /// The spin's first frame: the race turned a little (cosine 1023/1024, sine -13/1024)
    /// round its top left corner, two pixels square at a time, black where it turns off the
    /// frame or onto its outer rows and columns.
    #[test]
    fn the_spin_turns_the_frame_round_its_top_left_corner_in_squares() {
        let mut outro = spin();
        outro.wait();
        let (screen, frame) = (outro.screen(), frame());
        let square = |pixels: &[u8], (x, y): (usize, usize)| {
            [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| pixels[(y + dy) * WIDTH + x + dx])
        };
        for ((x, y), from) in [
            ((20, 20), (21, 19)),
            ((200, 100), (202, 97)),
            ((2, 2), (3, 1)),
            ((0, 10), (1, 9)),
        ] {
            assert_eq!(
                square(screen, (x, y)),
                square(&frame, from),
                "at ({x}, {y})"
            );
        }
        // (1, -1) and (321, 193) lie off the frame.
        assert_eq!(square(screen, (0, 0)), [0; 4]);
        assert_eq!(square(screen, (318, 198)), [0; 4]);
    }

    /// The spin fades the race's colours from 142/150 of themselves as its factor grows by
    /// 1.04 a tick from 8, and the sound with them (436 a step); its last frame is nearly
    /// black.
    #[test]
    fn the_spins_colours_and_sound_fade_over_150_steps() {
        let mut outro = spin();
        outro.wait();
        // Entry 63's red is 63, entry 7's green 49.
        assert_eq!(outro.palette().0[63][0], 59);
        assert_eq!(outro.palette().0[7][1], 46);
        assert_eq!(outro.volume(), Some(142 * 436));
        while outro.wait() {}
        assert_eq!(outro.palette().0[63][0], 1);
        assert_eq!(outro.volume(), Some(4 * 436));
    }
}
