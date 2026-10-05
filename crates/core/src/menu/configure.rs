//! The submenus of the main menu (spec M2a §3.3, M2b §3.2): the start submenu, Configure
//! (`showAdjustOptions`, 0x4309A0) with its volume popups and gamepad switch, Define Keyboard
//! (0x42FB00) and Define Gamepad (0x4302E0).

use deadrally_gamedata::dr_cfg::{DrCfg, KEY_COUNT, PAD_COUNT};
use deadrally_gamedata::text::ConfigureTexts;

use super::draw::{Focus, POPUP_FILL};
use super::{BACK_SOUND, CHOOSE_SOUND, MOVE_SOUND, Menu, START_MENU_BACK, State, Submenu};
use crate::canvas::at;
use crate::keys;

/// Configure's rows.
const MUSIC_ROW: usize = 0;
const EFFECTS_ROW: usize = 1;
const KEYBOARD_ROW: usize = 2;
const PAD_ROW: usize = 3;
pub(super) const SWITCH_ROW: usize = 4;
/// A volume popup's level runs 0..=128 in steps of 2: the volume / 512.
const LEVEL_STEP: i32 = 2;
const LEVEL_TOP: i32 = 128;
const LEVEL_UNIT: i32 = 512;
/// Effect 29 sounds when no gamepad is found.
const NOT_DETECTED_SOUND: u8 = 29;
/// A key Define Keyboard never takes.
const NOT_A_KEY: u8 = 0xAA;
/// Define Gamepad looks at the gamepad only after this many polls.
const PAD_SETTLE_POLLS: u32 = 15;

/// The gamepad switch's row: on when `dr.cfg` says 1 or 2, as `mainMenu` sets it.
pub(super) fn switch_text(texts: &ConfigureTexts, config: &DrCfg) -> Vec<u8> {
    if matches!(config.use_joystick(), 1 | 2) {
        texts.gamepad_on.clone()
    } else {
        texts.gamepad_off.clone()
    }
}

impl Menu {
    /// A pass of a submenu's loop: the background restored under it, the menus above it
    /// dimmed, the submenu with focus.
    pub(super) fn submenu_pass(&mut self, menu: Submenu) -> State {
        let (first_row, rows) = match menu {
            Submenu::Start => (92, 275),
            Submenu::Configure => (84, 283),
            Submenu::Keyboard | Submenu::Pad => (105, 262),
        };
        self.screen
            .copy_rows(&self.graphics.background, first_row, rows);
        self.draw_dimmed(menu);
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[menu as usize],
            Focus::Focused,
            self.cursor,
        );
        self.shown = self.screen.clone();
        State::Submenu {
            menu,
            second: false,
        }
    }

    /// The menus above `menu`, without focus.
    fn draw_dimmed(&mut self, menu: Submenu) {
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        if matches!(menu, Submenu::Keyboard | Submenu::Pad) {
            self.graphics.menu(
                &mut self.screen,
                &self.submenus[Submenu::Configure as usize],
                Focus::Unfocused,
                self.cursor,
            );
        }
    }

    /// The key read at the end of a submenu's pass (`readEventInMenu`, 0x42E0B0).
    pub(super) fn submenu_key(&mut self, menu: Submenu) -> State {
        match self.keys.take() {
            keys::ESCAPE => {
                self.sound(BACK_SOUND);
                match menu {
                    Submenu::Start => self.main_pass(),
                    Submenu::Configure => {
                        self.save = true;
                        self.main_pass()
                    }
                    Submenu::Keyboard | Submenu::Pad => self.submenu_pass(Submenu::Configure),
                }
            }
            keys::ENTER | keys::SPACE | 0x9C => {
                self.sound(CHOOSE_SOUND);
                let row = self.submenus[menu as usize].selected;
                self.choose_in(menu, row)
            }
            key @ (keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN) => {
                self.move_highlight(Some(menu), key);
                self.sound(MOVE_SOUND);
                State::Submenu {
                    menu,
                    second: false,
                }
            }
            _ => State::Submenu {
                menu,
                second: false,
            },
        }
    }

    /// What a submenu's row does. A menu's "previous" row also sets its selection back to its
    /// first row; Escape keeps it.
    fn choose_in(&mut self, menu: Submenu, row: usize) -> State {
        let table = &mut self.submenus[menu as usize];
        match (menu, row) {
            (Submenu::Start, START_MENU_BACK) => {
                table.selected = 0;
                self.main_pass()
            }
            // New game and loading wait for M3.
            (Submenu::Start, _) => self.submenu_pass(Submenu::Start),
            (Submenu::Configure, MUSIC_ROW) => self.volume_open(true),
            (Submenu::Configure, EFFECTS_ROW) => self.volume_open(false),
            (Submenu::Configure, KEYBOARD_ROW) => {
                self.control_rows();
                self.submenu_pass(Submenu::Keyboard)
            }
            (Submenu::Configure, PAD_ROW) => {
                self.control_rows();
                self.submenu_pass(Submenu::Pad)
            }
            (Submenu::Configure, SWITCH_ROW) => self.switch(),
            // 5: previous menu.
            (Submenu::Configure, _) => {
                table.selected = 0;
                self.save = true;
                self.main_pass()
            }
            (Submenu::Keyboard, KEY_COUNT) | (Submenu::Pad, PAD_COUNT) => {
                table.selected = 0;
                self.submenu_pass(Submenu::Configure)
            }
            (Submenu::Keyboard, control) => self.prompt(Submenu::Keyboard, control),
            (Submenu::Pad, control) => self.prompt(Submenu::Pad, control),
        }
    }

    /// The volume popup over the dimmed Configure, then its loop's first turn in the same
    /// tick.
    fn volume_open(&mut self, music: bool) -> State {
        let texts = &self.assets.menu.texts.configure;
        let caption = if music {
            &texts.adjust_music
        } else {
            &texts.adjust_effects
        };
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[Submenu::Configure as usize],
            Focus::Unfocused,
            self.cursor,
        );
        self.graphics
            .popup(&mut self.screen, 214, 218, 330, 70, Focus::Focused);
        self.graphics.small[0].draw(&mut self.screen, caption, at(224, 226));
        self.screen.draw(&self.graphics.slider, at(314, 250), true);
        self.shown = self.screen.clone();
        let volume = if music {
            self.config.music_volume()
        } else {
            self.config.effects_volume()
        };
        self.volume_turn(music, volume as i32 / LEVEL_UNIT)
    }

    /// The code after a volume popup's wait: Enter, Escape or keypad Enter read last end it,
    /// keeping the level; else another turn.
    pub(super) fn volume_tick(&mut self, music: bool, level: i32, last: u8) -> State {
        self.palette.after_wait();
        if matches!(last, keys::ENTER | keys::ESCAPE | 0x9C) {
            let volume = (level * LEVEL_UNIT) as u32;
            if music {
                self.config.set_music_volume(volume);
            } else {
                self.config.set_effects_volume(volume);
            }
            self.sound(BACK_SOUND);
            return self.submenu_pass(Submenu::Configure);
        }
        self.volume_turn(music, level)
    }

    /// One turn of the loop: a key, the slider and its knob at the level, the percentage, the
    /// volume applied at once.
    fn volume_turn(&mut self, music: bool, mut level: i32) -> State {
        let key = self.keys.take();
        match key {
            keys::LEFT | keys::PAD_LEFT if level > 0 => level -= LEVEL_STEP,
            keys::RIGHT | keys::PAD_RIGHT if level < LEVEL_TOP => level += LEVEL_STEP,
            _ => {}
        }
        let shift = |offset: usize| offset.wrapping_add_signed(level as isize);
        self.screen.fill(at(220, 246), 275, 30, POPUP_FILL);
        self.screen.draw(&self.graphics.slider, at(314, 250), true);
        self.screen
            .draw(&self.graphics.knob, shift(at(329, 250)), true);
        self.shown
            .copy_from(&self.screen, shift(at(327, 250)), 14, 24);
        // `(int)(level * 0.78125)`: 25/32 exactly.
        let percent = format!("{}%", level.max(0) * 25 / 32).into_bytes();
        let pen = at(309, 245) - self.graphics.big_a.width(&percent);
        self.graphics.big_a.draw(&mut self.screen, &percent, pen);
        self.shown.copy_from(&self.screen, at(224, 245), 120, 32);
        let volume = (level * LEVEL_UNIT) as u32;
        if music {
            self.sound.set_music_volume(volume);
        } else {
            self.sound.set_effects_volume(volume);
        }
        State::Volume {
            music,
            level,
            last: key,
        }
    }

    /// The prompt for control `control`'s key or gamepad input over the dimmed menus.
    fn prompt(&mut self, menu: Submenu, control: usize) -> State {
        self.draw_dimmed(menu);
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[menu as usize],
            Focus::Unfocused,
            self.cursor,
        );
        let texts = &self.assets.menu.texts.configure;
        let prompt = if menu == Submenu::Keyboard {
            &texts.key_prompts[control]
        } else {
            &texts.pad_prompts[control]
        };
        let y = 121 + 28 * control;
        self.graphics
            .popup(&mut self.screen, 295, y, 323, 48, Focus::Focused);
        self.graphics.small[0].draw(&mut self.screen, prompt, at(305, y + 13));
        self.shown = self.screen.clone();
        if menu == Submenu::Keyboard {
            State::KeyWait {
                control,
                key: self.keys.take(),
            }
        } else {
            self.keys.set_calibrating(true);
            State::PadWait { control, polls: 0 }
        }
    }

    /// `do { key = eventDetected(); waitWithRefresh(); } while (!key || key == 0xAA)`, then
    /// the key is the control's.
    pub(super) fn key_wait(&mut self, control: usize, key: u8) -> State {
        if key == 0 || key == NOT_A_KEY {
            return State::KeyWait {
                control,
                key: self.keys.take(),
            };
        }
        self.config.set_key(control, u32::from(key));
        self.keys.take();
        self.control_rows();
        self.submenu_pass(Submenu::Keyboard)
    }

    /// One poll of 0x42CBF0 after its wait: the gamepad (once it has settled), or Enter,
    /// keypad Enter or Escape for none.
    pub(super) fn pad_wait(&mut self, control: usize, polls: u32) -> State {
        self.palette.after_wait();
        let mut input = None;
        if self.config.use_joystick() as i32 > 0 && polls > PAD_SETTLE_POLLS {
            input = Some(self.keys.pad_input()).filter(|&input| input != 0);
        }
        if matches!(self.keys.take(), keys::ENTER | 0x9C | keys::ESCAPE) {
            input = Some(0);
        }
        let Some(input) = input else {
            return State::PadWait {
                control,
                polls: polls + 1,
            };
        };
        self.keys.set_calibrating(false);
        self.config.set_pad(control, u32::from(input));
        self.keys.take();
        self.control_rows();
        self.submenu_pass(Submenu::Pad)
    }

    /// Define Keyboard's and Define Gamepad's rows: each control's name and its key's or
    /// gamepad input's (0x41CA40).
    fn control_rows(&mut self) {
        let texts = &self.assets.menu.texts.configure;
        let named = |name: &[u8], value: &Vec<u8>| [name, value].concat();
        let pick =
            |names: &[Vec<u8>], value: u32| names.get(value as usize).unwrap_or(&names[0]).clone();
        for control in 0..KEY_COUNT {
            let row = named(
                &texts.controls[control],
                &pick(&texts.key_names, self.config.key(control)),
            );
            self.graphics
                .set_row(self.submenus[Submenu::Keyboard as usize].text, control, row);
        }
        for control in 0..PAD_COUNT {
            let row = named(
                &texts.controls[control],
                &pick(&texts.pad_names, self.config.pad(control)),
            );
            self.graphics
                .set_row(self.submenus[Submenu::Pad as usize].text, control, row);
        }
    }

    /// The gamepad switch: off when on; on when a gamepad is connected, else the "not
    /// detected" popup until a key.
    fn switch(&mut self) -> State {
        let configure = Submenu::Configure as usize;
        let was_on = self.config.use_joystick() != 0;
        let found = !was_on && self.keys.pad_connected();
        self.config.set_use_joystick(u32::from(found));
        self.keys.set_pad_on(found);
        let row = switch_text(&self.assets.menu.texts.configure, &self.config);
        self.graphics
            .set_row(self.submenus[configure].text, SWITCH_ROW, row);
        if was_on || found {
            return self.submenu_pass(Submenu::Configure);
        }
        self.sound(NOT_DETECTED_SOUND);
        self.screen.copy_rows(&self.graphics.background, 105, 262);
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[configure],
            Focus::Unfocused,
            self.cursor,
        );
        self.graphics
            .popup(&mut self.screen, 28, 198, 595, 86, Focus::Focused);
        let texts = &self.assets.menu.texts.configure;
        self.graphics
            .big_a
            .draw(&mut self.screen, &texts.not_detected, at(140, 208));
        self.graphics
            .big_a
            .draw(&mut self.screen, &texts.press_any_key, at(97, 240));
        self.shown = self.screen.clone();
        self.keys.take();
        self.keys.take();
        self.not_detected()
    }

    /// `while (!eventDetected()) waitWithRefresh(); eventDetected();`, then back to Configure.
    pub(super) fn not_detected(&mut self) -> State {
        if self.keys.take() == 0 {
            return State::NotDetected;
        }
        self.keys.take();
        self.submenu_pass(Submenu::Configure)
    }

    /// `dr.cfg`'s bytes when the original writes the file, once.
    pub(crate) fn take_config(&mut self) -> Option<Vec<u8>> {
        std::mem::take(&mut self.save).then(|| self.config.to_bytes())
    }
}
