//! The race's help (`keyMenuInRace` 0x407330, on F1 between a pass's logic and its drawing,
//! 0x416B21): the race fades to black, the keys' page (`KEYCOM3.BPK`: the global keys, each
//! control's key and gamepad input) fades in and waits for a key, fades out, the tips' page
//! (`INFO2.BPK`) likewise, then the race fades back in, to the race's own palette. A fade is
//! 63 ticks, a 63rd of each colour a tick in the original's 16.16 arithmetic.

use deadrally_gamedata::image::Palette;
use deadrally_gamedata::text::HelpTexts;

/// The screen's size, and the fades' steps.
const WIDTH: usize = 320;
const PIXELS: usize = WIDTH * 200;
const STEPS: i32 = 63;
/// Where the keys' page's lines start: column 60, the global keys' heading on row 8 and their
/// lines from row 20, the keyboard's heading on row 72 and its controls from row 84, the
/// gamepad's heading on row 142 and its inputs from row 154, 6 rows apart.
const COLUMN: usize = 60;
const GLOBAL_ROWS: (usize, usize) = (8, 20);
const KEYBOARD_ROWS: (usize, usize) = (72, 84);
const GAMEPAD_ROWS: (usize, usize) = (142, 154);
const LINE: usize = 6;
/// The gamepad's lines: the controls but the horn.
const PAD_CONTROLS: usize = 7;

/// The help's pages as loaded: the keys' and the tips' pictures with their palettes.
#[derive(Clone, Debug)]
pub(crate) struct Pages {
    pub(crate) keys: (Vec<u8>, Palette),
    pub(crate) info: (Vec<u8>, Palette),
}

/// What the help shows: the race, or one of its two pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Race,
    Keys,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Fading `Page` out, the ticks left.
    Out(Page, i32),
    /// Fading `Page` in, the ticks left.
    In(Page, i32),
    /// The page shown, waiting for a key.
    Wait(Page),
}

#[derive(Debug)]
pub(crate) struct Help {
    race: (Vec<u8>, Palette),
    keys: (Vec<u8>, Palette),
    info: (Vec<u8>, Palette),
    screen: Vec<u8>,
    palette: Palette,
    /// The palette faded to and from (0x4B4020), its 63rds (0x50EF40) and the fade's tick
    /// (0x456AF8).
    target: Palette,
    steps: [[i32; 3]; 256],
    tick: i32,
    phase: Phase,
}

/// A colour's 63rd in 16.16 (`sub_404A60`: `sub_43B290(c << 16, 0x3F0000)`).
fn step(component: u8) -> i32 {
    (i32::from(component) << 16) / STEPS
}

/// `text` in the small font (6x6, from space) at `row` and [`COLUMN`], its 0 bytes left out
/// (`sub_4072C0`).
fn write(screen: &mut [u8], font: &[u8], row: usize, text: &[u8]) {
    for (index, &c) in text.iter().enumerate() {
        let start = 36 * usize::from(c.saturating_sub(32));
        let glyph = font.get(start..start + 36).unwrap_or(&[]);
        let at = row * WIDTH + COLUMN + 6 * index;
        for (y, pixels) in glyph.chunks(6).enumerate() {
            for (x, &pixel) in pixels.iter().enumerate() {
                if pixel != 0
                    && let Some(slot) = screen.get_mut(at + y * WIDTH + x)
                {
                    *slot = pixel;
                }
            }
        }
    }
}

/// The keys' page: its picture with the global keys, the eight controls' keys (`controls`,
/// scancodes) and seven gamepad inputs (`pads`) written in.
fn keys_page(
    picture: &[u8],
    font: &[u8],
    texts: &HelpTexts,
    controls: &[u32; 8],
    pads: &[u32],
) -> Vec<u8> {
    let mut screen = picture.to_vec();
    screen.resize(PIXELS, 0);
    let named = |label: &[u8], name: Option<&Vec<u8>>| {
        let mut line = label.to_vec();
        line.extend_from_slice(name.map_or(&[][..], Vec::as_slice));
        line
    };
    for (index, line) in texts.global.iter().enumerate() {
        let row = if index == 0 {
            GLOBAL_ROWS.0
        } else {
            GLOBAL_ROWS.1 + LINE * (index - 1)
        };
        write(&mut screen, font, row, line);
    }
    write(&mut screen, font, KEYBOARD_ROWS.0, &texts.keyboard);
    for (index, (label, &code)) in texts.controls.iter().zip(controls).enumerate() {
        let name = usize::try_from(code)
            .ok()
            .and_then(|code| texts.key_names.get(code));
        write(
            &mut screen,
            font,
            KEYBOARD_ROWS.1 + LINE * index,
            &named(label, name),
        );
    }
    write(&mut screen, font, GAMEPAD_ROWS.0, &texts.gamepad);
    for (index, (label, &input)) in texts
        .controls
        .iter()
        .zip(pads)
        .take(PAD_CONTROLS)
        .enumerate()
    {
        let name = usize::try_from(input)
            .ok()
            .and_then(|input| texts.pad_names.get(input));
        write(
            &mut screen,
            font,
            GAMEPAD_ROWS.1 + LINE * index,
            &named(label, name),
        );
    }
    screen
}

impl Help {
    /// The help over the race's `screen` (320x200) and `palette` (0x4A9BA0); its keys' page
    /// written with `texts` in `font` for the `controls`' keys and the `pads`' inputs. Nothing
    /// changes until the next tick.
    pub(crate) fn new(
        race: (&[u8], &Palette),
        pages: &Pages,
        font: &[u8],
        texts: &HelpTexts,
        (controls, pads): (&[u32; 8], &[u32]),
    ) -> Help {
        let keys = keys_page(&pages.keys.0, font, texts, controls, pads);
        let mut help = Help {
            race: (race.0.to_vec(), race.1.clone()),
            keys: (keys, pages.keys.1.clone()),
            info: (pages.info.0.clone(), pages.info.1.clone()),
            screen: race.0.to_vec(),
            palette: race.1.clone(),
            target: race.1.clone(),
            steps: [[0; 3]; 256],
            tick: 0,
            phase: Phase::Out(Page::Race, STEPS),
        };
        help.info.0.resize(PIXELS, 0);
        help.aim(race.1.clone());
        help
    }

    pub(crate) fn screen(&self) -> &[u8] {
        &self.screen
    }

    pub(crate) fn palette(&self) -> &Palette {
        &self.palette
    }

    /// The fades from now on to and from `target`, from their first tick.
    fn aim(&mut self, target: Palette) {
        for (steps, rgb) in self.steps.iter_mut().zip(&target.0) {
            *steps = rgb.map(step);
        }
        self.target = target;
        self.tick = 0;
    }

    /// A tick, `pressed` whether a key has gone down since the page was shown; false once the
    /// race has faded back in.
    pub(crate) fn wait(&mut self, pressed: bool) -> bool {
        self.phase = match self.phase {
            Phase::Out(page, left) => {
                self.fade(|c, done| (c << 16) - done);
                if left > 1 {
                    Phase::Out(page, left - 1)
                } else {
                    let next = match page {
                        Page::Race => Page::Keys,
                        Page::Keys => Page::Info,
                        Page::Info => Page::Race,
                    };
                    let (screen, palette) = match next {
                        Page::Race => &self.race,
                        Page::Keys => &self.keys,
                        Page::Info => &self.info,
                    };
                    self.screen.copy_from_slice(&screen[..PIXELS]);
                    let palette = palette.clone();
                    self.aim(palette);
                    Phase::In(next, STEPS)
                }
            }
            Phase::In(page, left) => {
                self.fade(|_, done| done);
                if left > 1 {
                    Phase::In(page, left - 1)
                } else if page == Page::Race {
                    return false;
                } else {
                    Phase::Wait(page)
                }
            }
            Phase::Wait(page) if pressed => {
                let target = self.target.clone();
                self.aim(target);
                Phase::Out(page, STEPS)
            }
            Phase::Wait(page) => Phase::Wait(page),
        };
        true
    }

    /// Whether the page shown waits for a key (the keys are cleared as it starts to).
    pub(crate) fn waiting(&self) -> bool {
        matches!(self.phase, Phase::Wait(_))
    }

    /// A tick of a fade (`sub_404AD0` out, `sub_404BA0` in): each colour at `level(c, done)`,
    /// `done` its 63rds so far, rounded.
    fn fade(&mut self, level: impl Fn(i32, i32) -> i32) {
        self.tick += 1;
        for ((entry, steps), rgb) in self
            .palette
            .0
            .iter_mut()
            .zip(&self.steps)
            .zip(&self.target.0)
        {
            for ((out, &step), &c) in entry.iter_mut().zip(steps).zip(rgb) {
                let done = ((i64::from(step) * (i64::from(self.tick) << 16)) >> 16) as i32;
                *out = ((level(i32::from(c), done) + 0x8000) >> 16).clamp(0, 63) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette(seed: usize) -> Palette {
        let mut palette = Palette::BLACK;
        for (i, entry) in palette.0.iter_mut().enumerate() {
            *entry = [
                ((i + seed) % 64) as u8,
                ((i * 7) % 64) as u8,
                ((i * 13 + seed) % 64) as u8,
            ];
        }
        palette
    }

    fn texts() -> HelpTexts {
        HelpTexts {
            global: vec![b"!".to_vec(); 8],
            keyboard: b"!".to_vec(),
            gamepad: b"!".to_vec(),
            controls: (0..8).map(|_| b"!!".to_vec()).collect(),
            key_names: (0..256)
                .map(|code| {
                    if code == 0x1E {
                        b"!!!".to_vec()
                    } else {
                        Vec::new()
                    }
                })
                .collect(),
            pad_names: vec![b"!".to_vec(); 9],
        }
    }

    fn help() -> Help {
        let pages = Pages {
            keys: (vec![1; PIXELS], palette(1)),
            info: (vec![2; PIXELS], palette(2)),
        };
        // The font: every glyph's top left pixel 9, for "!" too.
        let font: Vec<u8> = (0..96 * 36)
            .map(|i| if i % 36 == 0 { 9 } else { 0 })
            .collect();
        let controls = [0x1E, 0x2C, 0xCB, 0xCD, 0x2A, 0x1D, 0x38, 0x39];
        Help::new(
            (&vec![3; PIXELS], &palette(3)),
            &pages,
            &font,
            &texts(),
            (&controls, &[0; 7]),
        )
    }

    /// Each fade takes 63 ticks: the race goes to black, then the keys' page comes up to its
    /// own palette and waits; a key starts its fade out on the next tick, and so on through the
    /// tips' page back to the race, which comes back in its own palette: 380 ticks when every
    /// key comes at once.
    #[test]
    fn the_help_fades_through_its_pages_63_ticks_a_fade() {
        let mut help = help();
        for _ in 0..62 {
            assert!(help.wait(false));
        }
        assert_ne!(help.palette(), &Palette::BLACK);
        help.wait(false);
        assert_eq!(help.palette(), &Palette::BLACK);
        assert_eq!(help.screen()[0], 1, "the keys' page, still black");
        for _ in 0..63 {
            help.wait(false);
        }
        assert_eq!(help.palette(), &palette(1));
        assert!(help.waiting());
        for _ in 0..10 {
            help.wait(false);
        }
        assert!(help.waiting(), "no key, no change");
        let mut ticks = 63 + 63 + 10;
        while help.wait(true) {
            ticks += 1;
        }
        assert_eq!(ticks + 1, 63 + 63 + 10 + 1 + 63 + 63 + 1 + 63 + 63);
        assert_eq!(help.palette(), &palette(3));
        assert_eq!(help.screen()[0], 3);
    }

    /// The fades round as the original's 16.16 does: 63 goes down by one a tick, and a colour
    /// of 49 is 49 again at the end of a fade in.
    #[test]
    fn the_fades_take_a_63rd_a_tick() {
        let mut help = help();
        help.wait(false);
        let race = palette(3);
        let entry = (0..256).find(|&e| race.0[e][0] == 63).unwrap();
        assert_eq!(help.palette().0[entry][0], 62);
    }

    /// The keys' page writes each control's label and its key's name after it, 6 rows apart
    /// from row 84, and leaves a key without a name blank after its label.
    #[test]
    fn the_keys_page_names_each_controls_key() {
        let mut help = help();
        for _ in 0..63 {
            help.wait(false);
        }
        let screen = help.screen();
        // Accelerate (A, 0x1E): its label's 2 glyphs and its name's 3.
        for k in 0..5 {
            assert_eq!(screen[84 * WIDTH + COLUMN + 6 * k], 9, "glyph {k}");
        }
        assert_eq!(screen[84 * WIDTH + COLUMN + 6 * 5], 1);
        // Brake (Z, nameless here): its label only.
        assert_eq!(screen[90 * WIDTH + COLUMN + 6], 9);
        assert_eq!(screen[90 * WIDTH + COLUMN + 12], 1);
    }
}
