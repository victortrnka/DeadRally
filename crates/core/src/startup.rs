//! The original's startup sequence: the intro, the Apogee and Remedy logos and the title screen
//! (spec M1a §3.6 and §5.2). Silent until M1b.
//!
//! Each step follows the Windows version's loops tick for tick, so a screenshot of the original
//! can be found in our timeline: `openAnimation` (0x4185B0), `apogeeScreen` (0x427380) and
//! `showStartScreen` (0x427880).

use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::haf::FRAME_PIXELS;
use deadrally_gamedata::image::Palette;

use crate::fade::{FADE_FULL, FADE_STEP, fade};
use crate::{AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, Frame, InputEvent};

/// The intro's screen: 320x200, with the animation's 320x120 frames from row 40.
const INTRO_WIDTH: u32 = 320;
const INTRO_HEIGHT: u32 = 200;
const INTRO_FIRST_ROW: usize = 40;
/// The letterbox owns palette entries 0..=15, the animation frames the rest.
const LETTERBOX_COLOURS: usize = 16;

/// Fade-in ticks: brightness 0, 4, ..., 96 %. The original's loop stops before 100 %.
const FADE_IN_TICKS: u32 = 25;
/// Fade-out ticks: brightness 100, 96, ..., 0 %.
const FADE_OUT_TICKS: u32 = 26;
/// Longest hold of a logo, in ticks.
const HOLD_TICKS: u32 = 180;

/// A logo or the title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Apogee,
    Remedy,
    Title,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// `next` is the frame being waited for; `waited` counts ticks since the previous frame.
    Intro {
        next: usize,
        waited: u32,
    },
    FadeIn {
        screen: Screen,
        ticks: u32,
    },
    Hold {
        screen: Screen,
        ticks: u32,
    },
    FadeOut {
        screen: Screen,
        ticks: u32,
    },
    /// The title after its fade-in, while the original loads the main menu (M2).
    Title,
}

#[derive(Debug)]
pub(crate) struct Startup {
    assets: Assets,
    stage: Stage,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    palette: Palette,
    /// The original keeps the last key press until something asks for it (`eventDetected`,
    /// 0x417EB0, reads and clears it), so a press during a fade-in ends the following hold
    /// after one tick.
    key: bool,
    silent_ticks: usize,
}

impl Startup {
    pub(crate) fn new(assets: Assets) -> Startup {
        let mut startup = Startup {
            assets,
            stage: Stage::Intro { next: 0, waited: 0 },
            width: 0,
            height: 0,
            pixels: Vec::new(),
            palette: Palette::BLACK,
            key: false,
            silent_ticks: 0,
        };
        if startup.assets.intro.is_empty() {
            // `openAnimation` plays nothing when the file has no frames.
            startup.stage = startup.show(Screen::Apogee);
        } else {
            startup.show_letterbox();
        }
        startup
    }

    pub(crate) fn input(&mut self, event: InputEvent) {
        if let InputEvent::Key { pressed: true, .. } | InputEvent::PadButton { pressed: true, .. } =
            event
        {
            self.key = true;
        }
    }

    pub(crate) fn tick(&mut self) {
        self.silent_ticks += 1;
        self.stage = match self.stage {
            Stage::Intro { next, waited } => self.tick_intro(next, waited + 1),
            // The title's last step is set but never shown: the original goes on to load the
            // main menu without presenting another frame, so the title stays at 92 %.
            Stage::FadeIn {
                screen: Screen::Title,
                ticks,
            } if ticks + 1 == FADE_IN_TICKS => Stage::Title,
            Stage::FadeIn { screen, ticks } => {
                let level = i64::from(ticks) * FADE_STEP;
                self.palette = fade(&picture(&self.assets, screen).palette, level);
                if ticks + 1 < FADE_IN_TICKS {
                    Stage::FadeIn {
                        screen,
                        ticks: ticks + 1,
                    }
                } else {
                    Stage::Hold { screen, ticks: 0 }
                }
            }
            Stage::Hold { screen, ticks } => {
                // `do { wait } while (!eventDetected() && ticks < 180)`: the key is read first,
                // so even the last hold tick consumes a pending press.
                if std::mem::take(&mut self.key) || ticks + 1 >= HOLD_TICKS {
                    Stage::FadeOut { screen, ticks: 0 }
                } else {
                    Stage::Hold {
                        screen,
                        ticks: ticks + 1,
                    }
                }
            }
            Stage::FadeOut { screen, ticks } => {
                let level = FADE_FULL - i64::from(ticks) * FADE_STEP;
                self.palette = fade(&picture(&self.assets, screen).palette, level);
                if ticks + 1 < FADE_OUT_TICKS {
                    Stage::FadeOut {
                        screen,
                        ticks: ticks + 1,
                    }
                } else {
                    self.show(match screen {
                        Screen::Apogee => Screen::Remedy,
                        Screen::Remedy | Screen::Title => Screen::Title,
                    })
                }
            }
            Stage::Title => Stage::Title,
        };
    }

    /// One tick of `openAnimation`: when frame `next` is due it replaces the previous one, then
    /// the original checks for a key before showing it. So a key press ends the intro at the
    /// next frame, which is never shown, and the last frame is never shown either.
    fn tick_intro(&mut self, mut next: usize, mut waited: u32) -> Stage {
        let intro = &self.assets.intro;
        let mut due = None;
        while waited >= u32::from(intro.delays[next]) {
            due = Some(next);
            next += 1;
            waited = 0;
            if next == intro.len() || std::mem::take(&mut self.key) {
                return self.show(Screen::Apogee);
            }
        }
        if let Some(index) = due {
            match intro.frame(index) {
                Ok(frame) => {
                    self.palette.0[LETTERBOX_COLOURS..]
                        .copy_from_slice(&frame.palette.0[LETTERBOX_COLOURS..]);
                    let start = INTRO_FIRST_ROW * INTRO_WIDTH as usize;
                    self.pixels[start..start + FRAME_PIXELS].copy_from_slice(&frame.pixels);
                }
                // Only data of an unknown version can get here (the known version's frames are
                // all tested), and the player was warned about it at start-up.
                Err(_) => return self.show(Screen::Apogee),
            }
        }
        Stage::Intro { next, waited }
    }

    /// Black screen with the letterbox's colours set, as `openAnimation` starts.
    fn show_letterbox(&mut self) {
        let letterbox = &self.assets.letterbox;
        assert_eq!(
            (letterbox.image.width, letterbox.image.height),
            (INTRO_WIDTH, INTRO_HEIGHT),
            "the intro letterbox is 320x200"
        );
        self.pixels.clear();
        self.pixels.extend_from_slice(&letterbox.image.pixels);
        (self.width, self.height) = (INTRO_WIDTH, INTRO_HEIGHT);
        self.palette = Palette::BLACK;
        self.palette.0[..LETTERBOX_COLOURS]
            .copy_from_slice(&letterbox.palette.0[..LETTERBOX_COLOURS]);
    }

    /// The picture drawn under a black palette, ready to fade in.
    #[must_use]
    fn show(&mut self, screen: Screen) -> Stage {
        let image = &picture(&self.assets, screen).image;
        self.pixels.clear();
        self.pixels.extend_from_slice(&image.pixels);
        (self.width, self.height) = (image.width, image.height);
        self.palette = Palette::BLACK;
        Stage::FadeIn { screen, ticks: 0 }
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: self.width,
            height: self.height,
            pixels: &self.pixels,
            palette: &self.palette.0,
            aspect: (4, 3),
        }
    }

    /// Silence until M1b brings the intro music and effects.
    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        let samples =
            std::mem::take(&mut self.silent_ticks) * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS;
        out.resize(out.len() + samples, 0);
    }
}

fn picture(assets: &Assets, screen: Screen) -> &Picture {
    match screen {
        Screen::Apogee => &assets.apogee,
        Screen::Remedy => &assets.remedy,
        Screen::Title => &assets.title,
    }
}
