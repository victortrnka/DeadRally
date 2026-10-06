//! The original's startup sequence: the intro, the Apogee and Remedy logos and the title screen
//! (spec M1a §3.6 and §5.2), with the intro's music and effects and then the menu music
//! (spec M1b §4.3).
//!
//! Each step follows the Windows version's loops tick for tick, so a screenshot of the original
//! can be found in our timeline: `openAnimation` (0x4185B0), `apogeeScreen` (0x427380) and
//! `showStartScreen` (0x427880).

use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::dr_cfg::DrCfg;
use deadrally_gamedata::haf::FRAME_PIXELS;
use deadrally_gamedata::image::Palette;

use crate::audio::{FULL_VOLUME, Sound};
use crate::fade::{FADE_FULL, FADE_STEP, fade};
use crate::keys::Keys;
use crate::menu::Menu;
use crate::{AUDIO_FRAMES_PER_TICK, Frame, InputEvent};

/// The intro's screen: 320x200, with the animation's 320x120 frames from row 40.
const INTRO_WIDTH: u32 = 320;
const INTRO_HEIGHT: u32 = 200;
const INTRO_FIRST_ROW: usize = 40;
/// The letterbox owns palette entries 0..=15, the animation frames the rest.
const LETTERBOX_COLOURS: usize = 16;
/// The intro's effects take channels 1..=6 in turn.
const INTRO_EFFECT_CHANNELS: usize = 6;
/// The menu music starts at this order (`musicSetOrder(0x2D00)` in `mainMenu`, 0x43A0C5).
const MENU_MUSIC_ORDER: usize = 45;

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
    /// The title has faded in; the main menu takes over (`mainMenu` goes on to load it).
    Done,
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
    keys: Keys,
    sound: Sound,
    /// The channel the intro's next effect plays on.
    effect_channel: usize,
    /// Samples rendered since the last `take_audio`.
    audio: Vec<i16>,
    /// The player's `dr.cfg`, and whether the original would write it now.
    config: DrCfg,
    save: bool,
    /// What `mainMenu` seeds `rand()` with once the intro is under way (0x43A191).
    seed: u32,
}

impl Startup {
    /// `mainMenu` (0x43A020) reads `dr.cfg`, counts the start, writes it back and only then
    /// plays the intro.
    pub(crate) fn new(assets: Assets, mut config: DrCfg, seed: u32) -> Startup {
        config.set_times_played(config.times_played().wrapping_add(1));
        let mut keys = Keys::default();
        keys.set_pad_on(config.use_joystick() as i32 > 0);
        let mut startup = Startup {
            assets,
            stage: Stage::Intro { next: 0, waited: 0 },
            width: 0,
            height: 0,
            pixels: Vec::new(),
            palette: Palette::BLACK,
            keys,
            sound: Sound::default(),
            effect_channel: 1,
            audio: Vec::new(),
            config,
            save: true,
            seed,
        };
        if startup.assets.intro.is_empty() {
            // `openAnimation` plays nothing when the file has no frames.
            startup.stage = startup.end_intro();
        } else {
            startup.show_letterbox();
            // `openAnimation` loads the music and the effects and starts the music just before
            // the first frame, at full volume: `dr.cfg`'s volumes apply only after the intro.
            startup
                .sound
                .play_music(&startup.assets.intro_music, 0, FULL_VOLUME);
            startup.sound.load_effects(&startup.assets.intro_effects);
        }
        startup
    }

    pub(crate) fn input(&mut self, event: InputEvent) {
        self.keys.event(event);
    }

    /// The title has faded in and the main menu should take over.
    pub(crate) fn finished(&self) -> bool {
        self.stage == Stage::Done
    }

    /// The main menu, taking over the data, the sound, the remembered key and `dr.cfg`.
    pub(crate) fn into_menu(self) -> Menu {
        Menu::new(
            self.assets,
            self.sound,
            self.keys,
            self.audio,
            &self.palette,
            (self.config, self.save),
            self.seed,
        )
    }

    /// `dr.cfg`'s bytes when the original writes the file, once.
    pub(crate) fn take_config(&mut self) -> Option<Vec<u8>> {
        std::mem::take(&mut self.save).then(|| self.config.to_bytes())
    }

    pub(crate) fn tick(&mut self) {
        self.keys.tick();
        self.stage = self.next_stage();
        self.sound.render(AUDIO_FRAMES_PER_TICK, &mut self.audio);
    }

    fn next_stage(&mut self) -> Stage {
        match self.stage {
            Stage::Intro { next, waited } => self.tick_intro(next, waited + 1),
            // After the title's last step the original loads the main menu without presenting
            // a frame; the menu's first wait shows this step (spec M2a decision 5: loading takes
            // no time here).
            Stage::FadeIn {
                screen: Screen::Title,
                ticks,
            } if ticks + 1 == FADE_IN_TICKS => {
                self.palette = fade(&self.assets.title.palette, i64::from(ticks) * FADE_STEP);
                Stage::Done
            }
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
                if self.keys.take() != 0 || ticks + 1 >= HOLD_TICKS {
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
            Stage::Done => Stage::Done,
        }
    }

    /// One tick of `openAnimation`: when frame `next` is due it replaces the previous one, then
    /// the original checks for a key before showing it. So a key press ends the intro at the
    /// next frame, which is never shown, and the last frame is never shown either.
    ///
    /// The original also checks once before frame 0. A press made while the game loads is read
    /// only when a frame is next shown (`refreshScreen`, 0x43B580), that is during frame 0's
    /// wait, so it ends the intro when frame 0 is due, as here.
    fn tick_intro(&mut self, mut next: usize, mut waited: u32) -> Stage {
        let intro = &self.assets.intro;
        let mut due = None;
        while waited >= u32::from(intro.delays[next]) {
            due = Some(next);
            next += 1;
            waited = 0;
            if next == intro.len() || self.keys.take() != 0 {
                // The frame ending the intro is never shown, and its effect, which the
                // original starts and cuts at once, never sounds.
                return self.end_intro();
            }
            // The original triggers a frame's effect right after drawing it.
            let effect = intro.effects[next - 1];
            if effect != 0 {
                self.sound.trigger(self.effect_channel, effect);
                self.effect_channel = self.effect_channel % INTRO_EFFECT_CHANNELS + 1;
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
                Err(_) => return self.end_intro(),
            }
        }
        Stage::Intro { next, waited }
    }

    /// The intro's sound stops (`openAnimation`, `checkAndOpenAnimation`); `mainMenu` then
    /// starts the menu music at the configured volume and shows the logos.
    fn end_intro(&mut self) -> Stage {
        self.sound.stop();
        self.sound.play_music(
            &self.assets.menu_music,
            MENU_MUSIC_ORDER,
            self.config.music_volume(),
        );
        self.sound.load_effects(&self.assets.menu.effects);
        self.sound.set_effects_volume(self.config.effects_volume());
        self.show(Screen::Apogee)
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

    /// The startup's sound: the intro's music and effects, then the menu music.
    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }
}

fn picture(assets: &Assets, screen: Screen) -> &Picture {
    match screen {
        Screen::Apogee => &assets.apogee,
        Screen::Remedy => &assets.remedy,
        Screen::Title => &assets.title,
    }
}
