//! The main menu (spec M2a §3.2–§3.5, M2b §3.2, M2c §3), from the title's fade to black to
//! the end screen, with Configure and the Hall of Fame.
//!
//! The original runs this as straight code with waits in it (`waitWithRefresh`, 0x43D870); the
//! screen shown during a tick is what the shown buffer and the palette hold when that tick's
//! wait starts. Here [`State`] names the wait the menu stands at, and [`Menu::tick`] runs the
//! code from it to the next one.

mod configure;
pub(crate) mod draw;
mod hall_of_fame;
mod licence;
pub(crate) mod palette;
mod sign_up;

use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::dr_cfg::DrCfg;

use self::draw::{
    CONFIGURE_MENU, CURSOR_FRAMES, Focus, Graphics, KEYBOARD_MENU, MAIN_MENU, MenuTable, PAD_MENU,
    POPUP_FILL, Panel, START_MENU,
};
use self::palette::MenuPalette;
use crate::audio::Sound;
use crate::campaign::{Campaign, NAME_BYTES};
use crate::canvas::{Canvas, HEIGHT, WIDTH, at};
use crate::keys::{self, Keys};
use crate::{AUDIO_FRAMES_PER_TICK, Frame};

/// The menus' sounds (`loadMenuSoundEffect`, 0x43C380): channel 1, at the configured effects
/// volume and pitch 0x28000.
const SOUND_CHANNEL: usize = 1;
const SOUND_PITCH: u32 = 0x2_8000;
const MOVE_SOUND: u8 = 25;
const BACK_SOUND: u8 = 22;
const CHOOSE_SOUND: u8 = 28;

/// The player's colour at the first start: driver 19's, 0 until a game sets it.
const PLAYER_COLOUR: usize = 0;
/// The main menu's rows: 0 start, 2 configure, 3 Hall of Fame, 4 credits, 5 exit.
const START_ROW: usize = 0;
const CONFIGURE_ROW: usize = 2;
const HALL_OF_FAME_ROW: usize = 3;
const CREDITS_ROW: usize = 4;
const EXIT_ROW: usize = 5;
/// The start submenu's last row returns to the main menu.
const START_MENU_BACK: usize = 5;
/// The exit question's yes/no at (x, y) = (180, 238).
const EXIT_QUESTION: (usize, usize) = (180, 238);
/// The end screen shows for at most 560 ticks; its fade-out lowers the music from 65500 in
/// steps of 2620.
const END_HOLD_TICKS: u32 = 560;
const END_VOLUME: u32 = 65_500;
const END_VOLUME_STEP: u32 = 2620;

/// Fades: 4 % a tick (`fadeIn` 0x427280 25 steps to 96 %, `transitionToBlack` 0x427300 26
/// steps from 100 % to 0), the menu's own fades 2 % a tick over 50 steps.
const FADE_IN_STEPS: u32 = 25;
const FADE_OUT_STEPS: u32 = 26;
const MENU_FADE_STEPS: u32 = 50;

/// The wait the menu stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// `transitionToBlack` on the title: wait `step` of 26.
    TitleToBlack {
        step: u32,
    },
    /// The menu's fade-in, wait `step` of 50.
    FadeIn {
        step: u32,
    },
    /// `readEventInMenu`: the first or second wait of a pass, in the main menu or a submenu.
    Main {
        second: bool,
    },
    Submenu {
        menu: Submenu,
        second: bool,
    },
    /// A volume popup's loop (`showAdjustOptions`, 0x4309A0): the level 0..=128 and the key
    /// read last.
    Volume {
        music: bool,
        level: i32,
        last: u8,
    },
    /// Define Keyboard waiting for control `control`'s key; `key` is the one read last.
    KeyWait {
        control: usize,
        key: u8,
    },
    /// Define Gamepad waiting for control `control`'s input (0x42CBF0): the polls so far.
    PadWait {
        control: usize,
        polls: u32,
    },
    /// The popup when the gamepad switch finds no gamepad (0x41E3B0), until a key.
    NotDetected,
    /// A wipe's wait before step `step`.
    Wipe {
        wipe: hall_of_fame::Wipe,
        step: u32,
    },
    /// The best ten, waiting for a key.
    FameWait,
    /// The records, circuit `index` of the circuit order; an arrow lit for 8 waits.
    Records {
        index: usize,
    },
    RecordsArrow {
        index: usize,
        right: bool,
        waits: u32,
    },
    /// `drawYesNoMenu` (0x42E310): two waits a pass; `yes` is the side selected.
    YesNo {
        question: Question,
        second: bool,
        yes: bool,
    },
    /// The licence: `readKeyboard`'s wait while the nickname is typed, the ten waits after
    /// a face change, and the difficulty popup's two waits a pass with the key read before
    /// them.
    Nickname,
    FaceChange {
        up: bool,
        waits: u32,
    },
    Difficulty {
        second: bool,
        row: usize,
        key: u8,
    },
    /// The sign-up: its two waits a pass, the welcome popup's wait (`passes` so far, the key
    /// read before its waits), a warning's and the "no race" popup's wait for a key, the fade
    /// after it, and the waits the screen stays after the sign-up.
    SignUp {
        second: bool,
        phase: sign_up::Phase,
    },
    PopupWait {
        second: bool,
        passes: u8,
        key: u8,
    },
    RaceWarning,
    NoSignUp,
    NoSignUpFade {
        step: u32,
    },
    Linger {
        waits: u32,
    },
    /// `showEndScreen`: the menu to black, `END.BMP` in, held, out with the music.
    EndToBlack {
        step: u32,
    },
    EndIn {
        step: u32,
    },
    EndHold {
        ticks: u32,
    },
    EndOut {
        step: u32,
    },
    /// The game has ended.
    Ended,
    /// `showCredits`: the menu out (50 down to 0), each credits screen in, held, out, the
    /// menu back in.
    CreditsOut {
        step: u32,
    },
    CreditsIn {
        screen: usize,
        step: u32,
    },
    CreditsHold {
        screen: usize,
    },
    CreditsToBlack {
        screen: usize,
        step: u32,
    },
    CreditsBack {
        step: u32,
    },
}

/// What a yes/no question asks; its answers sit at [`Question::at`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Question {
    Exit,
    Weapons,
    EndGame,
}

impl Question {
    fn at(self) -> (usize, usize) {
        match self {
            Question::Exit => EXIT_QUESTION,
            Question::Weapons => (193, 323),
            Question::EndGame => (180, 258),
        }
    }
}

/// The menus below the main menu, each read by `readEventInMenu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Submenu {
    Start,
    Configure,
    Keyboard,
    Pad,
}

#[derive(Debug)]
pub(crate) struct Menu {
    assets: Assets,
    graphics: Graphics,
    /// The original's screen buffer, the shown buffer, and the credits' copy of the screen.
    screen: Canvas,
    shown: Canvas,
    saved: Canvas,
    palette: MenuPalette,
    keys: Keys,
    sound: Sound,
    audio: Vec<i16>,
    main: MenuTable,
    /// Start, Configure, Define Keyboard, Define Gamepad, by [`Submenu`].
    submenus: [MenuTable; 4],
    panel: Panel,
    /// The player's `dr.cfg`, and whether the original would write it now.
    config: DrCfg,
    save: bool,
    /// The second screen buffer the Hall of Fame is drawn into before its wipe, and the
    /// menu music's order while the Hall of Fame plays its own.
    back: Canvas,
    music_order: usize,
    /// The cursor's frame (0x45FBF8).
    cursor: usize,
    state: State,
    /// The game in progress, and the licence's nickname entry with the name it replaces.
    campaign: Campaign,
    nickname: licence::Nickname,
    saved_name: [u8; NAME_BYTES],
    /// `sub_42C7F0`'s text cursor count (0x456BD0) and turn toggle (0x456BD4), and the
    /// turning car's frame (0x45FBA0).
    blink: i32,
    car_toggle: bool,
    car_frame: usize,
}

impl Menu {
    /// Takes over from the startup when the title has faded in: the title is shown at its
    /// last fade step, `title_shown`, and the menu stands at `transitionToBlack`'s first wait.
    pub(crate) fn new(
        assets: Assets,
        sound: Sound,
        keys: Keys,
        audio: Vec<i16>,
        title_shown: &deadrally_gamedata::image::Palette,
        (config, save): (DrCfg, bool),
        seed: u32,
    ) -> Menu {
        let menu_assets = &assets.menu;
        let colour = menu_assets.copper.0[PLAYER_COLOUR];
        let mut palette =
            MenuPalette::new(&menu_assets.palette, colour, &menu_assets.background_copper);
        palette.show(title_shown, 100);
        let mut graphics = Graphics::new(menu_assets);
        graphics.set_row(
            CONFIGURE_MENU.text,
            configure::SWITCH_ROW,
            configure::switch_text(&menu_assets.texts.configure, &config),
        );
        let panel = Panel::startup(&menu_assets.texts);
        let mut shown = Canvas::default();
        shown.copy_all(&assets.title.image);
        Menu {
            graphics,
            screen: Canvas::default(),
            shown,
            saved: Canvas::default(),
            palette,
            keys,
            sound,
            audio,
            main: MAIN_MENU,
            submenus: [START_MENU, CONFIGURE_MENU, KEYBOARD_MENU, PAD_MENU],
            panel,
            config,
            save,
            back: Canvas::default(),
            music_order: 0,
            cursor: 0,
            state: State::TitleToBlack { step: 0 },
            assets,
            campaign: Campaign::new(seed),
            nickname: licence::Nickname::default(),
            saved_name: [0; NAME_BYTES],
            blink: 0,
            car_toggle: false,
            car_frame: 0,
        }
    }

    pub(crate) fn input(&mut self, event: crate::InputEvent) {
        self.keys.event(event);
    }

    /// The player chose to exit and the end screen is over.
    pub(crate) fn quit_requested(&self) -> bool {
        self.state == State::Ended
    }

    pub(crate) fn tick(&mut self) {
        self.keys.tick();
        self.state = self.run();
        self.sound.render(AUDIO_FRAMES_PER_TICK, &mut self.audio);
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            pixels: self.shown.pixels(),
            palette: &self.palette.shown().0,
            aspect: (4, 3),
        }
    }

    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }

    /// The code from the wait at `self.state` to the next wait.
    fn run(&mut self) -> State {
        match self.state {
            State::TitleToBlack { step } => {
                let title = self.assets.title.palette.clone();
                self.palette.show(&title, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::TitleToBlack { step: step + 1 };
                }
                self.set_up();
                State::FadeIn { step: 0 }
            }
            State::FadeIn { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    return State::FadeIn { step: step + 1 };
                }
                self.shown = self.screen.clone();
                State::Main { second: false }
            }
            State::Main { second: false } => {
                self.palette.after_wait();
                State::Main { second: true }
            }
            State::Main { second: true } => {
                self.palette.after_wait();
                self.update_cursor_main();
                self.main_key()
            }
            State::Submenu {
                menu,
                second: false,
            } => {
                self.palette.after_wait();
                State::Submenu { menu, second: true }
            }
            State::Submenu { menu, second: true } => {
                self.palette.after_wait();
                self.graphics.update_cursor(
                    &mut self.screen,
                    &mut self.shown,
                    &self.submenus[menu as usize],
                    self.cursor,
                );
                self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
                self.submenu_key(menu)
            }
            State::Volume { music, level, last } => self.volume_tick(music, level, last),
            State::KeyWait { control, key } => self.key_wait(control, key),
            State::PadWait { control, polls } => self.pad_wait(control, polls),
            State::NotDetected => self.not_detected(),
            State::Wipe { wipe, step } => self.wipe_tick(wipe, step),
            State::FameWait => self.fame_wait(),
            State::Records { index } => self.records_tick(index),
            State::RecordsArrow {
                index,
                right,
                waits,
            } => self.records_arrow(index, right, waits),
            State::YesNo {
                question,
                second: false,
                yes,
            } => {
                self.palette.after_wait();
                State::YesNo {
                    question,
                    second: true,
                    yes,
                }
            }
            State::YesNo {
                question,
                second: true,
                yes,
            } => {
                self.palette.after_wait();
                match self.yes_no_key(question.at(), yes) {
                    Ok(yes) => State::YesNo {
                        question,
                        second: false,
                        yes,
                    },
                    Err(answer) => match question {
                        Question::Exit => self.exit_answer(answer),
                        Question::Weapons => self.weapons_answer(answer),
                        Question::EndGame => self.end_game_answer(answer),
                    },
                }
            }
            State::Nickname => self.nickname_tick(),
            State::FaceChange { up, waits } => self.face_change(up, waits),
            State::Difficulty { second, row, key } => self.difficulty_tick(second, row, key),
            State::SignUp { second, phase } => self.sign_up_tick(second, phase),
            State::PopupWait {
                second,
                passes,
                key,
            } => self.popup_wait(second, passes, key),
            State::RaceWarning => self.race_warning_tick(),
            State::NoSignUp => self.no_sign_up_tick(),
            State::NoSignUpFade { step } => self.no_sign_up_fade(step),
            State::Linger { waits } => self.linger_tick(waits),
            State::EndToBlack { step } => {
                self.palette.fade(100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::EndToBlack { step: step + 1 };
                }
                self.screen.copy_all(&self.assets.menu.end.image);
                self.shown = self.screen.clone();
                State::EndIn { step: 0 }
            }
            State::EndIn { step } => {
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    State::EndIn { step: step + 1 }
                } else {
                    State::EndHold { ticks: 0 }
                }
            }
            State::EndHold { ticks } => {
                // `do { wait; i++ } while (!eventDetected() && i < 560)`.
                if self.keys.take() != 0 || ticks + 1 >= END_HOLD_TICKS {
                    State::EndOut { step: 0 }
                } else {
                    State::EndHold { ticks: ticks + 1 }
                }
            }
            State::EndOut { step } => {
                self.sound
                    .set_mask((END_VOLUME - END_VOLUME_STEP * step) >> 8);
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    State::EndOut { step: step + 1 }
                } else {
                    // `mainMenu` writes `dr.cfg` after the end screen.
                    self.save_config();
                    State::Ended
                }
            }
            State::Ended => State::Ended,
            State::CreditsOut { step } => {
                // `for (e = 50; e >= 0; e--)`: here `step` counts e down.
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step > 0 {
                    return State::CreditsOut { step: step - 1 };
                }
                self.show_credits(0);
                State::CreditsIn { screen: 0, step: 0 }
            }
            State::CreditsIn { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    return State::CreditsIn {
                        screen,
                        step: step + 1,
                    };
                }
                // A key is checked before the first wait: one pressed during the fade-in moves
                // on at once.
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsHold { screen } => {
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsToBlack { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::CreditsToBlack {
                        screen,
                        step: step + 1,
                    };
                }
                if screen == 0 {
                    self.show_credits(1);
                    return State::CreditsIn { screen: 1, step: 0 };
                }
                self.palette.compose();
                self.screen = self.saved.clone();
                self.shown = self.screen.clone();
                State::CreditsBack { step: 0 }
            }
            State::CreditsBack { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    State::CreditsBack { step: step + 1 }
                } else {
                    self.main_pass()
                }
            }
        }
    }

    /// `mainMenu` after the title: the background, the bottom panel, the main menu, the
    /// palette composed; the menu's fade-in follows.
    fn set_up(&mut self) {
        self.screen.copy_all(&self.graphics.background);
        self.graphics
            .panel_frame(&mut self.screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut self.screen, &self.panel);
        self.draw_main();
        self.shown = self.screen.clone();
        self.palette.compose();
    }

    /// The top of `mainMenu`'s loop: rows 84..=366 restored, the main menu drawn with focus.
    fn draw_main(&mut self) {
        self.screen.copy_rows(&self.graphics.background, 84, 283);
        self.main.active[1] = false;
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Focused, self.cursor);
    }

    /// A pass of `mainMenu`'s loop without the fade: drawn, shown, waiting for a key.
    fn main_pass(&mut self) -> State {
        self.draw_main();
        self.shown = self.screen.clone();
        State::Main { second: false }
    }

    fn update_cursor_main(&mut self) {
        self.graphics
            .update_cursor(&mut self.screen, &mut self.shown, &self.main, self.cursor);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
    }

    fn sound(&mut self, effect: u8) {
        self.sound.trigger_at(
            SOUND_CHANNEL,
            effect,
            self.config.effects_volume(),
            SOUND_PITCH,
        );
    }

    /// Moves the highlight of `menu` (the main menu when `None`) for Up, Down or Escape.
    fn move_highlight(&mut self, menu: Option<Submenu>, key: u8) {
        let menu = match menu {
            None => &mut self.main,
            Some(submenu) => &mut self.submenus[submenu as usize],
        };
        let (to, base) = match key {
            keys::UP | keys::PAD_UP => {
                let mut row = menu.selected;
                loop {
                    row = if row == 0 { menu.rows - 1 } else { row - 1 };
                    if menu.active[row] {
                        break (row, 6);
                    }
                }
            }
            keys::DOWN | keys::PAD_DOWN => {
                let mut row = menu.selected;
                loop {
                    row = if row + 1 >= menu.rows { 0 } else { row + 1 };
                    if menu.active[row] {
                        break (row, 5);
                    }
                }
            }
            _ => (menu.rows - 1, 6),
        };
        self.graphics.move_highlight(
            &mut self.screen,
            &mut self.shown,
            menu,
            to,
            base,
            self.cursor,
        );
    }

    /// The key read at the end of a main menu pass.
    fn main_key(&mut self) -> State {
        match self.keys.take() {
            keys::ESCAPE => {
                if self.main.selected != self.main.rows - 1 {
                    self.move_highlight(None, keys::ESCAPE);
                    self.sound(MOVE_SOUND);
                }
            }
            keys::ENTER | keys::SPACE | 0x9C => {
                self.sound(CHOOSE_SOUND);
                return self.choose(self.main.selected);
            }
            key @ (keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN) => {
                self.move_highlight(None, key);
                self.sound(MOVE_SOUND);
            }
            _ => {}
        }
        State::Main { second: false }
    }

    /// What a main menu row does.
    fn choose(&mut self, row: usize) -> State {
        match row {
            START_ROW => self.submenu_pass(Submenu::Start),
            CONFIGURE_ROW => self.submenu_pass(Submenu::Configure),
            HALL_OF_FAME_ROW => self.open_hall_of_fame(),
            CREDITS_ROW => {
                self.saved = self.screen.clone();
                self.palette.compose();
                State::CreditsOut {
                    step: MENU_FADE_STEPS,
                }
            }
            EXIT_ROW => self.ask_exit(),
            // Multiplayer is never active.
            _ => self.main_pass(),
        }
    }

    /// `mainMenu`'s exit question: the main menu dimmed over itself, the question's popup,
    /// "no" selected.
    fn ask_exit(&mut self) -> State {
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics
            .popup(&mut self.screen, 170, 200, 300, 80, Focus::Focused);
        let question = &self.assets.menu.texts.exit_question;
        self.graphics.small[0].draw(&mut self.screen, question, at(253, 208));
        self.yes_no_open(Question::Exit, false)
    }

    /// `drawYesNoMenu`'s start: the answers drawn, the screen shown.
    fn yes_no_open(&mut self, question: Question, yes: bool) -> State {
        self.draw_yes_no(question.at(), yes);
        self.shown = self.screen.clone();
        State::YesNo {
            question,
            second: false,
            yes,
        }
    }

    /// The two answers, the selected one in big A: "yes" at (x + 30, y − 7), "no" at
    /// (x + 200, y − 7).
    fn draw_yes_no(&mut self, (x, y): (usize, usize), yes: bool) {
        let texts = &self.assets.menu.texts;
        let (yes_font, no_font) = if yes {
            (&self.graphics.big_a, &self.graphics.big_b)
        } else {
            (&self.graphics.big_b, &self.graphics.big_a)
        };
        yes_font.draw(&mut self.screen, &texts.yes, at(x + 30, y - 7));
        no_font.draw(&mut self.screen, &texts.no, at(x + 200, y - 7));
    }

    /// The end of a pass of `drawYesNoMenu`: the cursor beside the selected answer, then the
    /// key. `Ok` with the side selected to go on; `Err` with the answer, `None` for Escape.
    fn yes_no_key(&mut self, (x, y): (usize, usize), yes: bool) -> Result<bool, Option<bool>> {
        let cursor_x = if yes { x + 7 } else { x + 177 };
        let cursor_at = at(cursor_x, y);
        self.screen.fill(cursor_at, 20, 20, POPUP_FILL);
        let cursor = self.graphics.cursor(self.cursor).clone();
        self.screen.draw(&cursor, cursor_at, true);
        self.shown.copy_from(&self.screen, at(x + 2, y), 240, 28);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
        let key = match self.keys.take() {
            keys::Y => keys::PAD_LEFT,
            keys::N => keys::PAD_RIGHT,
            key => key,
        };
        let answer = match key {
            keys::LEFT | keys::PAD_LEFT | keys::RIGHT | keys::PAD_RIGHT => {
                let left = matches!(key, keys::LEFT | keys::PAD_LEFT);
                if left != yes {
                    self.sound(MOVE_SOUND);
                }
                self.screen.fill(at(x + 2, y), 240, 25, POPUP_FILL);
                self.draw_yes_no((x, y), left);
                return Ok(left);
            }
            keys::ESCAPE => None,
            keys::ENTER | 0x9C => Some(yes),
            _ => return Ok(yes),
        };
        self.sound(CHOOSE_SOUND);
        Err(answer)
    }

    /// `saveConfiguration` (0x4264E0): one `rand()` for its last header byte, then the file
    /// is written.
    fn save_config(&mut self) {
        let byte = self.campaign.rand.next() as u8;
        self.config.set_random_byte(byte);
        self.save = true;
    }

    fn sound_at(&mut self, effect: u8, pitch: u32) {
        self.sound
            .trigger_at(SOUND_CHANNEL, effect, self.config.effects_volume(), pitch);
    }

    /// `COPPER.PAL`'s entry for the player's colour.
    fn player_copper(&self) -> [u8; 3] {
        self.assets.menu.copper.0[self.campaign.player().colour as usize]
    }

    /// A pass of `startRacingMenu`'s loop: the Start Racing menu over the dimmed main menu.
    fn start_pass(&mut self) -> State {
        self.submenu_pass(Submenu::Start)
    }

    /// The loop's pass after a game's screens: the menus drawn into the second buffer and
    /// wiped in (`gameStarted_456B5C`, `sub_42C4A0`).
    fn start_wipe(&mut self) -> State {
        let mut back = self.screen.clone();
        back.copy_rows(&self.graphics.background, 92, 275);
        self.graphics
            .menu(&mut back, &self.main, Focus::Unfocused, self.cursor);
        self.graphics.menu(
            &mut back,
            &self.submenus[Submenu::Start as usize],
            Focus::Focused,
            self.cursor,
        );
        self.back = back;
        State::Wipe {
            wipe: hall_of_fame::Wipe::StartMenu,
            step: 0,
        }
    }

    /// "Start A New Game" or, with a game on, "Enter The Shop" (`startRacingMenu`).
    fn start_or_enter(&mut self) -> State {
        if !self.campaign.started {
            self.graphics.menu(
                &mut self.screen,
                &self.submenus[Submenu::Start as usize],
                Focus::Unfocused,
                self.cursor,
            );
            return self.open_licence();
        }
        // The shop comes with M3b; until then the shop's way on, the sign-up.
        self.open_sign_up()
    }

    /// The licence is done: the drivers set up, the menus renamed, then the sign-up.
    fn new_game(&mut self) -> State {
        let colour = self.campaign.player().colour;
        let start = &mut self.submenus[Submenu::Start as usize];
        for row in [1, 2, 4] {
            start.active[row] = true;
        }
        let campaign = &mut self.campaign;
        campaign.warn_hard = true;
        campaign.warn_medium = true;
        campaign.underground_popup = true;
        campaign.welcome = true;
        self.init_drivers();
        self.campaign.player_mut().colour = colour;
        self.palette.fade(100);
        let texts = &self.assets.menu.texts.campaign;
        let (shop, racing) = (
            texts.enter_shop_row.clone(),
            texts.continue_racing_row.clone(),
        );
        self.graphics.set_row(START_MENU.text, 0, shop);
        self.graphics.set_row(MAIN_MENU.text, 0, racing);
        self.campaign.started = true;
        self.palette.set_colour(self.player_copper());
        self.palette.compose();
        self.open_sign_up()
    }

    /// `initDrivers` (0x428930), with the globals it resets; it ends composing the palette
    /// for the player's colour (`sub_4224E0`).
    fn init_drivers(&mut self) {
        let texts = &self.assets.menu.texts.campaign;
        let campaign = &mut self.campaign;
        crate::campaign::init_drivers(
            &mut campaign.drivers,
            &mut campaign.rand,
            &texts.cars,
            &texts.driver_names,
        );
        campaign.selected_race = 0;
        self.palette.set_colour(self.player_copper());
        self.palette.compose();
    }

    /// "End Current Game": the question, "yes" selected.
    fn ask_end_game(&mut self) -> State {
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[Submenu::Start as usize],
            Focus::Unfocused,
            self.cursor,
        );
        self.graphics
            .popup(&mut self.screen, 170, 220, 300, 80, Focus::Focused);
        let question = self.assets.menu.texts.campaign.end_game.clone();
        self.graphics.small[0].draw(&mut self.screen, &question, at(232, 228));
        self.yes_no_open(Question::EndGame, true)
    }

    /// "Yes" ends the game: the menus as at the start, the drivers set up afresh.
    fn end_game_answer(&mut self, answer: Option<bool>) -> State {
        if answer == Some(true) {
            let texts = &self.assets.menu.texts.campaign;
            let (new, racing) = (texts.new_game_row.clone(), texts.start_racing_row.clone());
            self.graphics.set_row(START_MENU.text, 0, new);
            self.graphics.set_row(MAIN_MENU.text, 0, racing);
            let start = &mut self.submenus[Submenu::Start as usize];
            for row in [1, 2, 4] {
                start.active[row] = false;
            }
            let campaign = &mut self.campaign;
            campaign.warn_hard = false;
            campaign.warn_medium = false;
            campaign.underground_popup = false;
            campaign.welcome = false;
            campaign.started = false;
            self.init_drivers();
            self.palette.fade(100);
        }
        self.start_pass()
    }

    fn exit_answer(&mut self, answer: Option<bool>) -> State {
        if answer == Some(true) {
            self.palette.compose();
            State::EndToBlack { step: 0 }
        } else {
            self.main_pass()
        }
    }

    /// Credits screen `screen` drawn and shown, its palette black.
    fn show_credits(&mut self, screen: usize) {
        self.screen
            .copy_all(&self.assets.menu.credits[screen].image);
        self.shown = self.screen.clone();
    }
}
