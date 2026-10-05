//! The main menu's palette (spec M2a §3.2, §3.3), as `dr.exe` computes it: `MENU.PAL` with the
//! player colour's ramps (`sub_418B00` 0x418B00, `sub_41ED20` 0x41ED20), the background copper
//! rows (`sub_4224E0` 0x4224E0, 0x42A570) and the pulse of entries 16–31 (`sub_4220D0`
//! 0x4220D0). The ramps use the original's f32 and f64 arithmetic step by step: the x87 runs at
//! 53-bit precision there, so IEEE f64 with the same f32 roundings gives its exact results.

use deadrally_gamedata::image::Palette;

use crate::fade::{fade, fade_component};

/// `BGCOP.PAL`'s rows, three components each.
pub(crate) const BACKGROUND_ROWS: usize = 512;
/// The copper rows step on every 70th call of 0x42A570.
const COPPER_PERIOD: u32 = 70;
/// The pulse runs between these levels, 3 % a tick.
const PULSE_TOP: i64 = 100;
const PULSE_BOTTOM: i64 = 49;
const PULSE_STEP: i64 = 3;

/// The player colour's 32-entry ramp at 64..=95 (`sub_418B00`): from a tenth of the colour up to
/// the colour, then on towards white.
pub(crate) fn player_ramp(colour: [u8; 3]) -> [[u8; 3]; 32] {
    let mut ramp = [[0; 3]; 32];
    for c in 0..3 {
        let a = f32::from(colour[c]);
        let tenth = 0.1 * f64::from(a);
        let rise = ((f64::from(a) - tenth) * 0.0625) as f32;
        let to_white = ((63.0 - f64::from(a)) * 0.0625) as f32;
        for i in 0..16u8 {
            let step = f64::from(f32::from(i));
            ramp[usize::from(i)][c] = (step * f64::from(rise) + tenth) as u8;
            ramp[16 + usize::from(i)][c] = (step * f64::from(to_white) + f64::from(a)) as u8;
        }
    }
    ramp
}

/// The copper ramp at 176..=182 (`sub_41ED20`): from a sixth of the colour up to it.
pub(crate) fn copper_ramp(colour: [u8; 3]) -> [[u8; 3]; 7] {
    // The f32 constant at 0x443198, 1/7 rounded.
    let seventh = f64::from(f32::from_bits(0x3E12_4925));
    let mut ramp = [[0; 3]; 7];
    for c in 0..3 {
        let a = f64::from(colour[c]);
        let step = (a * seventh) as f32;
        let sixth = a * (1.0 / 6.0);
        for (i, entry) in ramp.iter_mut().enumerate() {
            entry[c] = ((f64::from(step) * i as f64 + sixth) as u8).min(63);
        }
    }
    ramp
}

/// Background copper row `row` spread over entries 192..=223: `row * i / 64`.
fn background_ramp(row: [u8; 3]) -> [[u8; 3]; 32] {
    std::array::from_fn(|i| row.map(|component| (usize::from(component) * i / 64) as u8))
}

/// The palette the menu composes once and fades in, and the entries that keep moving after.
#[derive(Clone, Debug)]
pub(crate) struct MenuPalette {
    /// `MENU.PAL` (`palette2`): the pulse scales it.
    menu: Palette,
    /// The composed palette (`palette1`) the fades scale.
    composed: Palette,
    /// The player colour's entry of `COPPER.PAL`.
    colour: [u8; 3],
    background: Vec<[u8; 3]>,
    /// What the hardware shows.
    shown: Palette,
    pulse: i64,
    rising: bool,
    /// Calls of 0x42A570 so far (0x456BA0), and the copper row (0x456754).
    calls: u32,
    row: usize,
}

impl MenuPalette {
    /// The palette as `sub_4224E0` composes it at the first start: background row 511, the
    /// pulse at 100 %. `colour` is the player colour's entry of `COPPER.PAL`; `background` holds
    /// `BGCOP.PAL`'s [`BACKGROUND_ROWS`] rows. Nothing is shown yet: the title has faded to
    /// black.
    pub(crate) fn new(menu: &Palette, colour: [u8; 3], background: &[[u8; 3]]) -> MenuPalette {
        assert_eq!(background.len(), BACKGROUND_ROWS, "BGCOP.PAL has 512 rows");
        let mut palette = MenuPalette {
            menu: menu.clone(),
            composed: menu.clone(),
            colour,
            background: background.to_vec(),
            shown: Palette::BLACK,
            pulse: PULSE_TOP,
            rising: false,
            calls: 0,
            row: BACKGROUND_ROWS - 1,
        };
        palette.compose();
        palette
    }

    /// `sub_4224E0`: the composed palette from `MENU.PAL`, the ramps, the current background
    /// row and entries 16–31 at the pulse's current level.
    pub(crate) fn compose(&mut self) {
        let mut composed = self.menu.clone();
        composed.0[64..96].copy_from_slice(&player_ramp(self.colour));
        composed.0[176..183].copy_from_slice(&copper_ramp(self.colour));
        composed.0[192..224].copy_from_slice(&background_ramp(self.background[self.row]));
        let level = self.pulse << 16;
        for entry in 16..32 {
            composed.0[entry] =
                self.menu.0[entry].map(|component| fade_component(component, level));
        }
        self.composed = composed;
    }

    pub(crate) fn shown(&self) -> &Palette {
        &self.shown
    }

    /// The whole composed palette at `percent` %, as the menu's fades show it.
    pub(crate) fn fade(&mut self, percent: i64) {
        self.shown = fade(&self.composed, percent << 16);
    }

    /// Another picture's palette at `percent` %, for the screens the menu leads to.
    pub(crate) fn show(&mut self, palette: &Palette, percent: i64) {
        self.shown = fade(palette, percent << 16);
    }

    /// What follows each wait of 0x42A570 (and 0x42A480) in the menu: the pulse writes entries
    /// 16–31 at its level and moves on; every 70th call the background steps a row, its 32
    /// entries shown at full brightness.
    pub(crate) fn after_wait(&mut self) {
        self.calls += 1;
        let level = self.pulse << 16;
        for entry in 16..32 {
            self.shown.0[entry] =
                self.menu.0[entry].map(|component| fade_component(component, level));
        }
        if self.pulse == PULSE_BOTTOM {
            self.rising = true;
            self.pulse += PULSE_STEP;
        } else if self.pulse == PULSE_TOP {
            self.rising = false;
            self.pulse -= PULSE_STEP;
        } else if self.rising {
            self.pulse += PULSE_STEP;
        } else {
            self.pulse -= PULSE_STEP;
        }
        if self.calls % COPPER_PERIOD == 1 {
            self.row = self.row.checked_sub(1).unwrap_or(BACKGROUND_ROWS - 1);
            self.shown.0[192..224].copy_from_slice(&background_ramp(self.background[self.row]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> Palette {
        Palette(std::array::from_fn(|i| {
            [(i % 64) as u8, 63, (63 - i % 64) as u8]
        }))
    }

    fn background() -> Vec<[u8; 3]> {
        (0..BACKGROUND_ROWS)
            .map(|row| [(row % 64) as u8, 1, 32])
            .collect()
    }

    #[test]
    fn the_player_ramp_follows_the_originals_float_arithmetic() {
        // Values from an emulation of 0x418B00's f32/f64 steps; plain integer maths is off by
        // one in places, which shows as a different shade of the player's colour.
        let ramp = player_ramp([63, 0, 32]);
        assert_eq!(ramp[0], [6, 0, 3]);
        assert_eq!(ramp[1], [9, 0, 4]);
        assert_eq!(ramp[15], [59, 0, 30]);
        assert_eq!(ramp[16], [63, 0, 32]);
        assert_eq!(ramp[17], [63, 3, 33]);
        assert_eq!(ramp[31], [63, 59, 61]);
        let other = player_ramp([40, 21, 7]);
        assert_eq!(other[4], [13, 6, 2]);
        assert_eq!(other[25], [52, 44, 38]);
    }

    #[test]
    fn the_copper_ramp_has_seven_entries_capped_at_63() {
        // DreeRally's loop writes one entry; the original writes 176..=182.
        assert_eq!(
            copper_ramp([63, 0, 32]),
            [
                [10, 0, 5],
                [19, 0, 9],
                [28, 0, 14],
                [37, 0, 19],
                [46, 0, 23],
                [55, 0, 28],
                [63, 0, 32]
            ]
        );
        assert_eq!(copper_ramp([40, 21, 7])[3], [23, 12, 4]);
    }

    #[test]
    fn the_fade_in_scales_the_composed_palette_and_stops_at_98_percent() {
        // The original never raises the menu to 100 %: a component of 63 shows as 62.
        let mut palette = MenuPalette::new(&menu(), [63, 0, 32], &background());
        assert_eq!(palette.shown(), &Palette::BLACK);
        palette.fade(98);
        assert_eq!(palette.shown().0[1], [1, 62, 61]);
        assert_eq!(palette.shown().0[64], [6, 0, 3], "the player ramp, at 98 %");
        assert_eq!(
            palette.shown().0[192 + 31],
            [
                fade_component(30, 98 << 16),
                0,
                fade_component(15, 98 << 16)
            ],
            "background row 511 (511 % 64 = 63): 63 * 31 / 64 = 30, 32 * 31 / 64 = 15"
        );
    }

    #[test]
    fn the_pulse_runs_from_100_down_to_49_and_back_in_34_calls() {
        // The pulsing entries animate the menu's highlights; another period or phase would
        // show other shades in the screenshots.
        let mut palette = MenuPalette::new(&menu(), [0, 0, 0], &background());
        let levels: Vec<[u8; 3]> = (0..35)
            .map(|_| {
                palette.after_wait();
                palette.shown().0[16 + 15]
            })
            .collect();
        let component = |level: i64| fade_component(31, level << 16);
        assert_eq!(levels[0][0], component(100));
        assert_eq!(levels[1][0], component(97));
        assert_eq!(levels[17][0], component(49));
        assert_eq!(levels[18][0], component(52));
        assert_eq!(levels[34][0], component(100), "a period of 34 calls");
    }

    #[test]
    fn the_background_steps_a_row_on_the_first_call_and_every_70th_after() {
        let mut palette = MenuPalette::new(&menu(), [0, 0, 0], &background());
        // Row r of this background is (r % 64, 1, 32); entry 192 + 31 shows row * 31 / 64.
        palette.after_wait();
        assert_eq!(
            palette.shown().0[192 + 31],
            [30, 0, 15],
            "row 510, full brightness"
        );
        for _ in 1..70 {
            palette.after_wait();
        }
        assert_eq!(palette.shown().0[192 + 31][0], 30, "still row 510");
        palette.after_wait();
        assert_eq!(palette.shown().0[192 + 31][0], 29, "call 71: row 509");
    }
}
