//! The driver's licence (spec M3a §3): `licenseScreen` (0x434800) with its nickname entry
//! (`readKeyboard` 0x42E7F0 and its wait `sub_42C7F0` 0x42C7F0), the weapons question
//! (`drawYesNoMenu` 0x42E310) and the difficulty popup. DreeRally `ui/licenseScreen.c`,
//! `ui/util/input.c`; dRally `___3ab5ch.c`, `___17510h.c`, `___17384h.c`.

use deadrally_gamedata::image::Image;

use super::draw::{Focus, POPUP_FILL};
use super::{CHOOSE_SOUND, MOVE_SOUND, Menu, State};
use crate::canvas::{Canvas, at};
use crate::keys;

/// Where the licence lies, and its parts.
const LICENCE: (usize, usize) = (63, 112);
const LICENCE_WIDTH: usize = 530;
const NAME: (usize, usize) = (173, 208);
const FACE_FRAME: (usize, usize) = (88, 172);
const FACE: (usize, usize) = (90, 191);
/// The face frame's arrows: up at the frame's top, down at its foot.
const UP_ARROW_Y: usize = 172;
const DOWN_ARROW_Y: usize = 258;
const CAR_BOX: (usize, usize) = (479, 176);
const CAR_TURNING: (usize, usize) = (479, 192);
const PRICE_Y: usize = 258;
const SLIDER: (usize, usize) = (170, 255);
/// The colour knob is drawn at x 184 + colour.
const KNOB_X: usize = 184;
const KNOB_Y: usize = 251;
/// A nickname holds up to 10 characters and 300 pixels.
const NAME_CHARACTERS: usize = 10;
const NAME_PIXELS: usize = 300;
/// The colour slider starts in the middle of `COPPER.PAL` and moves 2 a key.
pub(super) const START_COLOUR: i32 = 128;
/// The car turns through 64 frames.
const TURNING_FRAMES: usize = 64;
/// The difficulty popup's rows.
const DIFFICULTY_X: usize = 139;
const DIFFICULTY_Y: usize = 216;
const DIFFICULTY_CURSOR_X: usize = 115;
/// The effects channel the voices play on, and their pitch.
pub(super) const VOICE_CHANNEL: usize = 5;
pub(super) const VOICE_PITCH: u32 = 0x2_4000;
/// Duke's voice when the player takes his face (face 2), and the difficulties' voices.
const DUKE_FACE: i32 = 2;
const DUKE_VOICE: u8 = 6;
const DIFFICULTY_VOICES: [u8; 3] = [1, 2, 3];

/// What `readKeyboard` keeps while a name is typed: the licence's nickname, or a saved
/// game's name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Nickname {
    pub(super) text: Vec<u8>,
    /// The typed text's width in the big font.
    pub(super) width: usize,
    face: i32,
    colour: i32,
    /// Where the text starts, how many characters and pixels it may take, and whether the
    /// licence's face and colour keys work (and its car turns).
    at: (usize, usize),
    max_characters: usize,
    max_pixels: usize,
    licence: bool,
}

impl Nickname {
    /// A saved game's name entry (`savegameWithName`): `text` prefilled, 15 characters, 320
    /// pixels, from (130, 298), no face or colour.
    pub(super) fn save_name(text: Vec<u8>, width: usize) -> Nickname {
        Nickname {
            text,
            width,
            face: 0,
            colour: START_COLOUR,
            at: (130, 298),
            max_characters: 15,
            max_pixels: 320,
            licence: false,
        }
    }
}

/// `drawInGamePrices` (0x41A370) centred in a 96-pixel box from `x`
/// (`getBoxTextOffset`, 0x41FAB0): "$" is glyph 0, the digits 1 to 10, drawn opaque.
pub(super) fn draw_price(
    canvas: &mut Canvas,
    menu: &deadrally_gamedata::assets::MenuAssets,
    text: &[u8],
    x: usize,
    y: usize,
) {
    let advances = &menu.texts.campaign.price.advances;
    let index = |c: u8| -> usize {
        if c == b'$' {
            0
        } else {
            usize::from(c.wrapping_sub(b'/'))
        }
    };
    let width: i32 = text
        .iter()
        .map(|&c| i32::from(advances.get(index(c)).copied().unwrap_or(0)))
        .sum();
    let mut pen = (x as i32 + (96 - width) / 2) as usize;
    for &c in text {
        if let Some(glyph) = menu.price_digits.get(index(c)) {
            canvas.draw(glyph, at(pen, y), false);
        }
        pen += usize::from(advances.get(index(c)).copied().unwrap_or(0));
    }
}

/// The characters of set-1 scancodes on the original's table (`readKeyboard`, 0x45EEE0).
fn character(scancode: u8) -> Option<u8> {
    let c = match scancode {
        0x02..=0x0A => b'1' + (scancode - 0x02),
        0x0B => b'0',
        0x0C => b'-',
        0x0D => b'=',
        0x10..=0x19 => b"QWERTYUIOP"[usize::from(scancode - 0x10)],
        0x1E..=0x26 => b"ASDFGHJKL"[usize::from(scancode - 0x1E)],
        0x2C..=0x32 => b"ZXCVBNM"[usize::from(scancode - 0x2C)],
        0x33 => b',',
        0x34 => b'.',
        0x35 => b'/',
        0x39 => b' ',
        _ => return None,
    };
    Some(c)
}

/// `rows` rows from `first` on: from column `x` (or `x + 1` on odd rows) every second
/// pixel `POPUP_FILL`, `width` columns wide: the licence's shading.
fn shade(screen: &mut Canvas, x: usize, first: usize, rows: usize, width: usize) {
    for row in 0..rows {
        let mut column = row % 2;
        while column < width {
            screen.fill(at(x + column, first + row), 1, 1, POPUP_FILL);
            column += 2;
        }
    }
}

impl Menu {
    /// The licence's first screen, then the nickname's first wait (`licenseScreen` with
    /// weapons asked, as a single-player game does).
    pub(super) fn open_licence(&mut self) -> State {
        self.car_frame = 0;
        self.campaign.player_mut().colour = 0;
        self.graphics
            .panel_frame(&mut self.screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut self.screen, &self.panel);
        self.palette.set_player_ramp(self.copper(START_COLOUR));
        let menu = &self.assets.menu;
        self.screen
            .draw(&menu.licence, at(LICENCE.0, LICENCE.1), true);
        self.screen
            .draw(&menu.face_frame, at(FACE_FRAME.0, FACE_FRAME.1), false);
        self.screen.draw(&menu.faces[0], at(FACE.0, FACE.1), false);
        let texts = &menu.texts;
        self.graphics
            .big_b
            .draw(&mut self.screen, &texts.yes, at(223, 316));
        self.graphics
            .big_b
            .draw(&mut self.screen, &texts.no, at(393, 316));
        shade(&mut self.screen, LICENCE.0, 291, 69, LICENCE_WIDTH);
        self.draw_car_box();
        self.draw_turning_car();
        self.car_frame = (self.car_frame + 1) % TURNING_FRAMES;
        let menu = &self.assets.menu;
        self.screen
            .draw(&menu.colour_slider, at(SLIDER.0, SLIDER.1), true);
        self.screen.draw(
            &menu.colour_knob,
            at(KNOB_X + START_COLOUR as usize, KNOB_Y),
            true,
        );
        self.shown = self.screen.clone();
        // `eventDetected` and `sub_418090`: a key pressed before the licence is dropped.
        self.keys.take();
        self.saved_name = self.campaign.player().name;
        self.campaign.player_mut().set_name(b"");
        self.nickname = Nickname {
            text: Vec::new(),
            width: 0,
            face: 0,
            colour: START_COLOUR,
            at: NAME,
            max_characters: NAME_CHARACTERS,
            max_pixels: NAME_PIXELS,
            licence: true,
        };
        State::Nickname
    }

    /// `COPPER.PAL`'s entry `colour`.
    fn copper(&self, colour: i32) -> [u8; 3] {
        self.assets.menu.copper.0[colour as usize]
    }

    /// The car's box: `CARBAS2`, the Vagabond's name and its price centred under it.
    fn draw_car_box(&mut self) {
        let menu = &self.assets.menu;
        self.screen
            .draw(&menu.car_box, at(CAR_BOX.0, CAR_BOX.1), false);
        self.screen
            .draw(&menu.car_names[0], at(CAR_BOX.0, CAR_BOX.1), false);
        let price = format!("${}", menu.texts.campaign.cars[0].price).into_bytes();
        self.draw_price(&price, CAR_BOX.0, PRICE_Y);
    }

    /// `drawInGamePrices` (0x41A370) centred in a 96-pixel box from `x`
    /// (`getBoxTextOffset`, 0x41FAB0) on the screen.
    pub(super) fn draw_price(&mut self, text: &[u8], x: usize, y: usize) {
        draw_price(&mut self.screen, &self.assets.menu, text, x, y);
    }

    /// The turning car's current frame, drawn and shown.
    fn draw_turning_car(&mut self) {
        let frame: &Image = &self.assets.menu.car_turning[0][self.car_frame];
        self.screen
            .draw(frame, at(CAR_TURNING.0, CAR_TURNING.1), false);
        self.shown
            .copy_from(&self.screen, at(CAR_TURNING.0, CAR_TURNING.1), 96, 64);
    }

    /// `sub_42C7F0` after its wait: the text cursor blinks (shown for 10 waits, hidden for
    /// 11), and every second wait the car turns a frame.
    fn nickname_wait(&mut self) {
        self.palette.after_wait();
        let (x, y) = self.nickname.at;
        let x = x + self.nickname.width;
        if self.blink <= 9 {
            let cursor = self.assets.menu.texts.campaign.text_cursor.clone();
            self.graphics
                .big_b
                .draw(&mut self.screen, &cursor, at(x, y));
        } else {
            self.screen.fill(at(x, y + 2), 20, 30, POPUP_FILL);
        }
        self.blink += 1;
        if self.blink > 20 {
            self.blink = 0;
        }
        self.shown.copy_from(&self.screen, at(x, y), 20, 32);
        if self.car_toggle {
            self.car_toggle = false;
        } else {
            if self.nickname.licence {
                self.draw_turning_car();
                self.car_frame = (self.car_frame + 1) % TURNING_FRAMES;
            }
            self.car_toggle = true;
        }
    }

    /// A wait of the nickname entry, then its key.
    pub(super) fn nickname_tick(&mut self) -> State {
        self.nickname_wait();
        let key = self.keys.take();
        let licence = self.nickname.licence;
        match key {
            keys::ESCAPE if licence => return self.licence_cancelled(),
            keys::ESCAPE => return self.save_name_cancelled(),
            keys::ENTER | 0x9C => {
                if licence {
                    let player = self.campaign.player_mut();
                    player.face = self.nickname.face;
                    player.colour = self.nickname.colour;
                }
                if !self.nickname.text.is_empty() {
                    if !licence {
                        return self.save_name_done();
                    }
                    let text = self.nickname.text.clone();
                    self.campaign.player_mut().set_name(&text);
                    return self.nickname_done();
                }
            }
            keys::UP
            | keys::PAD_UP
            | keys::DOWN
            | keys::PAD_DOWN
            | keys::LEFT
            | keys::PAD_LEFT
            | keys::RIGHT
            | keys::PAD_RIGHT
                if !licence => {}
            keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN => {
                let up = matches!(key, keys::UP | keys::PAD_UP);
                self.sound(MOVE_SOUND);
                let face = &mut self.nickname.face;
                *face = match (up, *face) {
                    (true, f) if f <= 0 => 19,
                    (true, f) => f - 1,
                    (false, f) if f >= 19 => 0,
                    (false, f) => f + 1,
                };
                let face = self.nickname.face as usize;
                let image = self.assets.menu.faces[face].clone();
                self.screen.draw(&image, at(FACE.0, FACE.1), false);
                self.shown
                    .copy_from(&self.screen, at(FACE.0, FACE.1), 64, 64);
                return State::FaceChange { up, waits: 0 };
            }
            keys::LEFT | keys::PAD_LEFT | keys::RIGHT | keys::PAD_RIGHT => {
                let colour = &mut self.nickname.colour;
                if matches!(key, keys::LEFT | keys::PAD_LEFT) {
                    if *colour > 0 {
                        *colour -= 2;
                    }
                } else if *colour < 253 {
                    *colour += 2;
                }
                let colour = self.nickname.colour;
                self.palette.set_player_ramp(self.copper(colour));
                self.screen.fill(at(164, KNOB_Y), 294, 24, POPUP_FILL);
                let menu = &self.assets.menu;
                self.screen
                    .draw(&menu.colour_slider, at(SLIDER.0, SLIDER.1), true);
                self.screen.draw(
                    &menu.colour_knob,
                    at(KNOB_X + colour as usize, KNOB_Y),
                    true,
                );
                self.shown.copy_from(
                    &self.screen,
                    at(KNOB_X - 2 + colour as usize, KNOB_Y),
                    14,
                    24,
                );
            }
            0x0E => self.erase_character(),
            _ => self.type_character(key),
        }
        State::Nickname
    }

    fn erase_character(&mut self) {
        let Some(&last) = self.nickname.text.last() else {
            return;
        };
        let advance = self.graphics.big_b.width(&[last]);
        self.nickname.width -= advance;
        let (x, y) = self.nickname.at;
        let x = x + self.nickname.width;
        self.screen.fill(at(x, y), advance + 20, 32, POPUP_FILL);
        self.shown
            .copy_from(&self.screen, at(x, y), advance + 20, 32);
        self.nickname.text.pop();
    }

    fn type_character(&mut self, scancode: u8) {
        let Some(c) = character(scancode) else {
            return;
        };
        let allowed = &self.assets.menu.texts.campaign.name_characters;
        if !allowed[usize::from(c)]
            || self.nickname.text.len() >= self.nickname.max_characters
            || self.nickname.width >= self.nickname.max_pixels
        {
            return;
        }
        let c = c.to_ascii_lowercase();
        let (x, y) = self.nickname.at;
        let x = x + self.nickname.width;
        self.screen.fill(at(x, y), 32, 32, POPUP_FILL);
        self.graphics.big_b.draw(&mut self.screen, &[c], at(x, y));
        let advance = self.graphics.big_b.width(&[c]);
        self.shown.copy_from(&self.screen, at(x, y), advance, 32);
        self.nickname.text.push(c);
        self.nickname.width += advance;
    }

    /// The waits after a face change: five, the arrow lit, five more, the arrow back.
    pub(super) fn face_change(&mut self, up: bool, waits: u32) -> State {
        self.nickname_wait();
        let (y, lit, normal) = if up {
            (UP_ARROW_Y, 2, 0)
        } else {
            (DOWN_ARROW_Y, 3, 1)
        };
        let waits = waits + 1;
        if waits == 5 || waits == 10 {
            let frame = if waits == 5 { lit } else { normal };
            let arrow = self.assets.menu.face_arrows[frame].clone();
            self.screen.draw(&arrow, at(FACE_FRAME.0, y), false);
            self.shown
                .copy_from(&self.screen, at(FACE_FRAME.0, y), 68, 16);
        }
        if waits < 10 {
            return State::FaceChange { up, waits };
        }
        self.campaign.player_mut().face = self.nickname.face;
        State::Nickname
    }

    /// Escape at the nickname: the old name back, no game.
    fn licence_cancelled(&mut self) -> State {
        self.sound(MOVE_SOUND);
        self.campaign.player_mut().name = self.saved_name;
        self.start_pass()
    }

    /// The nickname is in: the licence again with it, then the weapons question.
    fn nickname_done(&mut self) -> State {
        self.sound(CHOOSE_SOUND);
        if self.campaign.player().face == DUKE_FACE {
            self.sound.trigger_at(
                VOICE_CHANNEL,
                DUKE_VOICE,
                self.config.effects_volume(),
                VOICE_PITCH,
            );
        }
        let menu = &self.assets.menu;
        self.screen
            .draw(&menu.licence, at(LICENCE.0, LICENCE.1), true);
        let name = self.campaign.player().name().to_vec();
        self.graphics
            .big_a
            .draw(&mut self.screen, &name, at(NAME.0, NAME.1));
        let menu = &self.assets.menu;
        self.screen
            .draw(&menu.face_frame, at(FACE_FRAME.0, FACE_FRAME.1), false);
        let face = self.campaign.player().face as usize;
        self.screen
            .draw(&menu.faces[face], at(FACE.0, FACE.1), false);
        self.draw_car_box();
        self.draw_turning_car();
        self.car_frame = (self.car_frame + 1) % TURNING_FRAMES;
        let colour = self.campaign.player().colour as usize;
        let menu = &self.assets.menu;
        self.screen
            .draw(&menu.colour_slider, at(SLIDER.0, SLIDER.1), true);
        self.screen
            .draw(&menu.colour_knob, at(KNOB_X + colour, KNOB_Y), true);
        shade(&mut self.screen, LICENCE.0, LICENCE.1, 178, LICENCE_WIDTH);
        self.campaign.use_weapons = true;
        self.yes_no_open(super::Question::Weapons, true)
    }

    /// The weapons question's answer: Escape ends the licence without a game, Enter goes on
    /// to the difficulty.
    pub(super) fn weapons_answer(&mut self, answer: Option<bool>) -> State {
        let Some(yes) = answer else {
            self.campaign.use_weapons = false;
            self.sound(MOVE_SOUND);
            self.licence_end();
            return self.start_pass();
        };
        self.campaign.use_weapons = yes;
        self.screen.copy_rows(&self.graphics.background, 84, 283);
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics
            .popup(&mut self.screen, 93, 142, 440, 186, Focus::Focused);
        let title = self.assets.menu.texts.campaign.select_difficulty.clone();
        self.graphics
            .big_a
            .draw(&mut self.screen, &title, at(173, 160));
        // A damaged dr.cfg may hold any number; the popup has three rows.
        let row = self.config.difficulty().min(2) as usize;
        self.draw_cursor_at(DIFFICULTY_CURSOR_X, 28 * row + 221);
        self.draw_difficulties(row);
        self.shown = self.screen.clone();
        State::Difficulty {
            second: false,
            row,
            key: self.keys.take(),
        }
    }

    fn draw_difficulties(&mut self, row: usize) {
        for difficulty in 0..3 {
            let name = self.assets.menu.texts.hall_of_fame.difficulties[difficulty].clone();
            let font = if difficulty == row {
                &self.graphics.big_a
            } else {
                &self.graphics.big_b
            };
            font.draw(
                &mut self.screen,
                &name,
                at(DIFFICULTY_X, DIFFICULTY_Y + 28 * difficulty),
            );
        }
    }

    /// A wait of the difficulty popup; after the second, the cursor and the key read before
    /// the waits.
    pub(super) fn difficulty_tick(&mut self, second: bool, row: usize, key: u8) -> State {
        self.palette.after_wait();
        if !second {
            return State::Difficulty {
                second: true,
                row,
                key,
            };
        }
        self.draw_cursor_at(DIFFICULTY_CURSOR_X, 28 * row + 221);
        let mut row = row;
        let moved = match key {
            keys::UP | keys::PAD_UP if row > 0 => {
                row -= 1;
                true
            }
            keys::DOWN | keys::PAD_DOWN if row < 2 => {
                row += 1;
                true
            }
            _ => false,
        };
        if moved {
            self.screen.fill(at(115, 216), 355, 84, POPUP_FILL);
            self.draw_difficulties(row);
            self.shown.copy_from(&self.screen, at(115, 216), 400, 84);
            self.sound(MOVE_SOUND);
        }
        if matches!(key, keys::ENTER | 0x9C | keys::SPACE) {
            self.config.set_difficulty(row as u32);
            self.sound.trigger_at(
                VOICE_CHANNEL,
                DIFFICULTY_VOICES[row],
                self.config.effects_volume(),
                VOICE_PITCH,
            );
        }
        match key {
            keys::ESCAPE => {
                self.sound(MOVE_SOUND);
                self.licence_end();
                self.start_pass()
            }
            keys::ENTER | 0x9C => {
                self.licence_end();
                self.new_game()
            }
            _ => State::Difficulty {
                second: false,
                row,
                key: self.keys.take(),
            },
        }
    }

    /// The licence's end (0x4351A5 on): the name lower-cased but its first letter, the
    /// player's colour shown, the panel redrawn.
    fn licence_end(&mut self) {
        let player = self.campaign.player_mut();
        let mut name = player.name().to_ascii_lowercase();
        if let Some(first) = name.first_mut() {
            *first = first.to_ascii_uppercase();
        }
        player.set_name(&name);
        let colour = self.campaign.player().colour;
        self.palette.set_player_ramp(self.copper(colour));
        self.graphics.panel_text(&mut self.screen, &self.panel);
        self.shown = self.screen.clone();
    }

    /// `drawCursor` (0x41AC50): the cursor's box cleared, its frame drawn and shown.
    pub(super) fn draw_cursor_at(&mut self, x: usize, y: usize) {
        self.screen.fill(at(x, y), 20, 20, POPUP_FILL);
        let cursor = self.graphics.cursor(self.cursor).clone();
        self.screen.draw(&cursor, at(x, y), true);
        self.shown.copy_from(&self.screen, at(x, y), 20, 20);
        self.cursor = (self.cursor + 1) % super::draw::CURSOR_FRAMES;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scancodes_type_the_us_layouts_characters() {
        // The original's table (0x45EEE0): a nickname typed on another row of keys would
        // come out as other letters.
        assert_eq!(character(0x1E), Some(b'A'));
        assert_eq!(character(0x10), Some(b'Q'));
        assert_eq!(character(0x32), Some(b'M'));
        assert_eq!(character(0x0B), Some(b'0'));
        assert_eq!(character(0x39), Some(b' '));
        assert_eq!(character(0x1C), None, "Enter is no character");
        assert_eq!(character(0x0E), None, "nor is Backspace");
    }

    #[test]
    fn shading_covers_every_second_pixel_in_a_chequer() {
        // The licence's parts not being asked about are dimmed by a chequer of the popup
        // colour; a solid fill would hide them.
        let mut screen = Canvas::default();
        shade(&mut screen, 10, 20, 2, 4);
        let p = screen.pixels();
        assert_eq!(
            [p[at(10, 20)], p[at(11, 20)], p[at(12, 20)], p[at(13, 20)]],
            [POPUP_FILL, 0, POPUP_FILL, 0]
        );
        assert_eq!(
            [p[at(10, 21)], p[at(11, 21)], p[at(12, 21)], p[at(13, 21)]],
            [0, POPUP_FILL, 0, POPUP_FILL]
        );
    }
}
