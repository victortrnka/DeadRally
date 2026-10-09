//! The original's startup sequence: the intro, the Apogee and Remedy logos and the title screen
//! (spec M1a §3.6 and §5.2), with the intro's music and effects and then the menu music
//! (spec M1b §4.3).
//!
//! Each step follows the Windows version's loops tick for tick, so a screenshot of the original
//! can be found in our timeline: `openAnimation` (0x4185B0), `apogeeScreen` (0x427380) and
//! `showStartScreen` (0x427880).

use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::dr_cfg::DrCfg;
use deadrally_gamedata::image::Palette;

use crate::animation::{Player, Tick};

use crate::audio::{FULL_VOLUME, Sound};
use crate::fade::{FADE_FULL, FADE_STEP, fade};
use crate::keys::Keys;
use crate::menu::Menu;
use crate::{AUDIO_FRAMES_PER_TICK, Frame, InputEvent};

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
    /// The intro playing (see [`Player`]).
    Intro,
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
    /// The intro's animation.
    intro: Player,
    /// Samples rendered since the last `take_audio`.
    audio: Vec<i16>,
    /// The player's `dr.cfg`, and whether the original would write it now.
    config: DrCfg,
    save: bool,
    /// What `mainMenu` seeds `rand()` with once the intro is under way (0x43A191).
    seed: u32,
    /// The saved games' files, slot by slot.
    pub(crate) slot_files: Vec<Option<Vec<u8>>>,
    /// The clock the sabotage reads, when fixed (see [`crate::Game::fix_sabotage_clock`]).
    pub(crate) sabotage_clock: Option<u32>,
    /// Whether the races keep the opponents still (see [`crate::Game::keep_opponents_still`]).
    pub(crate) still_opponents: bool,
    /// Whether it plays as the Windows version (see [`crate::Game::as_the_windows_version`]).
    pub(crate) windows_version: bool,
}

impl Startup {
    /// `mainMenu` (0x43A020) reads `dr.cfg`, counts the start, writes it back and only then
    /// plays the intro.
    pub(crate) fn new(assets: Assets, mut config: DrCfg, seed: u32) -> Startup {
        config.set_times_played(config.times_played().wrapping_add(1));
        let mut keys = Keys::default();
        keys.set_pad_on(config.use_joystick() as i32 > 0);
        let intro = Player::new(&assets.letterbox);
        let mut startup = Startup {
            assets,
            stage: Stage::Intro,
            width: 0,
            height: 0,
            pixels: Vec::new(),
            palette: Palette::BLACK,
            keys,
            sound: Sound::default(),
            intro,
            audio: Vec::new(),
            config,
            save: true,
            seed,
            slot_files: vec![None; deadrally_gamedata::save_game::SLOTS],
            sabotage_clock: None,
            still_opponents: false,
            windows_version: false,
        };
        if startup.assets.intro.is_empty() {
            // `openAnimation` plays nothing when the file has no frames.
            startup.stage = startup.end_intro();
        } else {
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
            (
                self.seed,
                self.slot_files,
                self.sabotage_clock,
                (self.still_opponents, self.windows_version),
            ),
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
            Stage::Intro => {
                match self
                    .intro
                    .tick(&self.assets.intro, &mut self.keys, &mut self.sound)
                {
                    Tick::Playing => Stage::Intro,
                    Tick::Ended => self.end_intro(),
                }
            }
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
        if self.stage == Stage::Intro {
            return self.intro.frame();
        }
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
