//! The sign-up screen (spec M3a §3): `selectRaceScreen` (0x4357F0) with its screen
//! (`drawSelectRaceScreen` 0x423F40), the side panel (`drawCarRightSide` 0x41FC20), the
//! welcome popup (`welcomePopup` 0x41C840 and its wait `drawPopupCursor_42C780`), the drivers
//! signing up (`addParticipantToRace` 0x423A20) and the warnings. DreeRally
//! `ui/selectRaceScreen.c`, `ui/util/carRightSide.c`, `ui/util/popup.c`; dRally
//! `___31588h.c`, `___3079ch.c`, `___25330h.c`.

use super::draw::Focus;
use super::hall_of_fame::Wipe;
use super::{MOVE_SOUND, Menu, State};
use crate::campaign::{PLAYER, SignUp};
use crate::canvas::{Canvas, at};
use crate::keys;

/// The three races' columns are 160 pixels apart; their snapshots, prices, popups and border.
const COLUMN: usize = 160;
const SNAPSHOT: (usize, usize) = (32, 128);
const PRICE_XS: [usize; 3] = [73, 226, 383];
const PRICE_Y: usize = 224;
const BORDER: (usize, usize, usize, usize) = (22, 118, 148, 132);
/// The entries: rank and name from (34 + 160 race, 256 + 18 place).
const ENTRY: (usize, usize) = (34, 256);
const ENTRY_HEIGHT: usize = 18;
/// Sounds: a sign-up, a full race, a popup.
const SIGN_UP_SOUND: u8 = 21;
const FULL_SOUND: u8 = 29;
const POPUP_SOUND: u8 = 23;
const POPUP_PITCH: u32 = 0x2_5500 - 0x1000;
/// The welcome popup's cursor, and the passes before it takes a key.
const POPUP_CURSOR: (usize, usize) = (164, 321);
const POPUP_DEAF_PASSES: u8 = 10;
/// A driver signs up with a chance of one in 75 a pass while the player chooses, one in 3
/// once the player has.
const CHOOSING_CHANCE: i32 = 75;
const FILLING_CHANCE: i32 = 3;
/// After the sign-up the screen stays up to 280 waits, or until a key.
const LINGER_WAITS: u32 = 280;
/// The hitman's chance rises by 2 % a sign-up without him, up to 97 % (0x431B30).
const HITMAN_RISE: i32 = 2;
const HITMAN_TOP: i32 = 97;

/// What happens on the sign-up screen between its waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The player chooses a race.
    Choosing,
    /// The player is in; the other places fill.
    Filling,
}

impl Menu {
    /// `selectRaceScreen`'s start: the races drawn, the screen built in the second buffer
    /// with the welcome popup on a new game, then wiped in.
    pub(super) fn open_sign_up(&mut self) -> State {
        let order = self.assets.menu.texts.hall_of_fame.circuit_order.clone();
        let campaign = &mut self.campaign;
        campaign.sign_up = Some(SignUp::new(
            &mut campaign.rand,
            &order,
            &mut campaign.last_circuits,
        ));
        campaign.entered_race = None;
        self.palette.show_composed(176..192);
        let mut back = std::mem::take(&mut self.back);
        back.copy_all(&self.graphics.background);
        self.draw_sign_up_screen(&mut back);
        if self.campaign.welcome {
            self.welcome_popup(&mut back);
        } else {
            self.border_at(&mut back, self.campaign.selected_race);
        }
        self.back = back;
        State::Wipe {
            wipe: Wipe::SignUp,
            step: 0,
        }
    }

    /// After the wipe: the welcome popup's wait on a new game, else the first pass.
    pub(super) fn sign_up_shown(&mut self) -> State {
        self.shown = self.screen.clone();
        if self.campaign.welcome {
            self.popup_wait_start()
        } else {
            State::SignUp {
                second: false,
                phase: Phase::Choosing,
            }
        }
    }

    /// `drawSelectRaceScreen`: the panel, the three circuits with their prices and popups, the
    /// title, the side panel.
    fn draw_sign_up_screen(&mut self, canvas: &mut Canvas) {
        canvas.restore(&self.graphics.background, at(0, 367), 640, 16);
        self.graphics.panel_frame(canvas, 0, 371, 639, 109);
        self.graphics.panel_text(canvas, &self.panel);
        let menu = &self.assets.menu;
        let sign_up = self.campaign.sign_up.as_ref().expect("a sign-up is on");
        for (race, &circuit) in sign_up.circuits.iter().enumerate() {
            canvas.draw(
                &menu.snapshots[circuit],
                at(SNAPSHOT.0 + COLUMN * race, SNAPSHOT.1),
                false,
            );
        }
        let prices = &menu.texts.campaign.race_prices;
        for (price, &x) in prices.iter().zip(&PRICE_XS) {
            self.graphics.small[0].draw(canvas, price, at(x, PRICE_Y));
        }
        for race in 0..3 {
            self.graphics
                .popup(canvas, 22 + COLUMN * race, 256, 148, 105, Focus::Focused);
        }
        canvas.draw(&menu.sign_up_title, at(0, 80), true);
        self.draw_side_panel(canvas);
    }

    /// `drawCarRightSide`: the player's car, name, money, top speed, rank, damage and
    /// upgrades in the panel at the right, the player's colour shown.
    pub(super) fn draw_side_panel(&mut self, canvas: &mut Canvas) {
        let player = *self.campaign.player();
        self.palette
            .set_player_ramp(self.assets.menu.copper.0[player.colour as usize]);
        let menu = &self.assets.menu;
        let car = player.car as usize;
        canvas.draw(&menu.side_panel, at(544, 125), false);
        canvas.draw(&menu.side_cars[car], at(544, 141), false);
        let medium = &self.graphics.medium;
        let name = player.name().to_ascii_uppercase();
        let centre = |width: usize| ((96 - width as i32) / 2) as usize;
        medium.draw(canvas, &name, at(544 + centre(medium.width(&name)), 126));
        let money = format!("${}", player.money.min(9_999_999)).into_bytes();
        let small = &self.graphics.small[0];
        small.draw(canvas, &money, at(544 + centre(small.width(&money)), 205));
        let speed = menu.texts.campaign.speeds[car][player.engine as usize].to_string();
        let speed = if speed.len() == 2 {
            format!(" {speed}")
        } else {
            speed
        };
        medium.draw(canvas, speed.as_bytes(), at(608, 319));
        let rank = player.rank.to_string();
        let rank = if rank.len() == 1 {
            format!(" #{rank}")
        } else {
            format!("#{rank}")
        };
        medium.draw(canvas, rank.as_bytes(), at(608, 334));
        let damage = player.damage.clamp(0, 100);
        // ceil(damage * 0.44) pixels.
        let bar = ((damage * 44 + 99) / 100) as usize;
        canvas.fill(at(548, 306), bar, 5, 63);
        let text = player.damage.to_string().into_bytes();
        small.draw(canvas, &text, at(623 - small.width(&text), 302));
        let spec = menu.texts.campaign.cars[car];
        let levels = [player.engine, player.tires, player.armour];
        for ((&count, &level), y) in spec.upgrades.iter().zip(&levels).zip([233, 257, 281]) {
            for slot in 0..count.max(0) as usize {
                canvas.draw(&menu.upgrade_lamps[4], at(547 + 23 * slot, y), false);
            }
            for slot in 0..level.max(0) as usize {
                canvas.draw(&menu.upgrade_lamps[slot], at(547 + 23 * slot, y), false);
            }
        }
    }

    /// `welcomePopup`: ten lines and the word under them.
    fn welcome_popup(&mut self, canvas: &mut Canvas) {
        self.graphics
            .popup(canvas, 45, 131, 458, 230, Focus::Focused);
        let texts = &self.assets.menu.texts.campaign;
        for (line, text) in texts.welcome.iter().enumerate() {
            self.graphics
                .write_text(canvas, text, at(60, 141 + 16 * line));
        }
        self.graphics
            .big_a
            .draw(canvas, &texts.continue_word, at(192, 316));
    }

    fn border_at(&self, canvas: &mut Canvas, race: usize) {
        let (x, y, w, h) = BORDER;
        self.border(canvas, x + COLUMN * race, y, w, h);
    }

    /// `removeBorder` (0x421C40): a frame five pixels thick restored from the background.
    fn remove_border_at(&mut self, race: usize) {
        let (x, y, w, h) = BORDER;
        let x = x + COLUMN * race;
        let background = &self.graphics.background;
        self.screen.restore(background, at(x, y), w, 5);
        self.screen.restore(background, at(x, y + h - 5), w, 5);
        self.screen.restore(background, at(x, y), 5, h);
        self.screen.restore(background, at(x + w - 5, y), 5, h);
    }

    /// `drawPopupCursor_42C780`'s start: the key pressed before is dropped.
    fn popup_wait_start(&mut self) -> State {
        self.keys.take();
        State::PopupWait {
            second: false,
            passes: 0,
            key: 0,
        }
    }

    /// A wait of `drawPopupCursor_42C780`: a key is read before each pass's two waits, from
    /// the twelfth pass on; Escape ends it after the pass that read it, Enter before the next.
    pub(super) fn popup_wait(&mut self, second: bool, passes: u8, key: u8) -> State {
        self.palette.after_wait();
        if !second {
            return State::PopupWait {
                second: true,
                passes,
                key,
            };
        }
        self.draw_cursor_at(POPUP_CURSOR.0, POPUP_CURSOR.1);
        let passes = passes.wrapping_add(1);
        if !matches!(key, keys::ESCAPE | keys::ENTER | 0x9C) {
            let key = if passes > POPUP_DEAF_PASSES {
                self.keys.take()
            } else {
                key
            };
            return State::PopupWait {
                second: false,
                passes,
                key,
            };
        }
        self.keys.take();
        self.after_welcome()
    }

    /// The welcome is over: the screen drawn again with the border, the first pass.
    fn after_welcome(&mut self) -> State {
        self.screen
            .restore(&self.graphics.background, at(0, 103), 640, 260);
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_sign_up_screen(&mut screen);
        self.border_at(&mut screen, self.campaign.selected_race);
        self.screen = screen;
        self.shown = self.screen.clone();
        State::SignUp {
            second: false,
            phase: Phase::Choosing,
        }
    }

    /// A wait of the sign-up; after the second, a driver may sign up and, while the player
    /// chooses, the key.
    pub(super) fn sign_up_tick(&mut self, second: bool, phase: Phase) -> State {
        self.palette.after_wait();
        if !second {
            return State::SignUp {
                second: true,
                phase,
            };
        }
        let chance = match phase {
            Phase::Choosing => CHOOSING_CHANCE,
            Phase::Filling => FILLING_CHANCE,
        };
        self.add_driver(chance);
        if phase == Phase::Filling {
            if self.sign_up().full() {
                return self.signed_up();
            }
            return State::SignUp {
                second: false,
                phase,
            };
        }
        match self.keys.take() {
            key @ (keys::LEFT | keys::PAD_LEFT | keys::RIGHT | keys::PAD_RIGHT) => {
                let race = self.campaign.selected_race;
                let left = matches!(key, keys::LEFT | keys::PAD_LEFT);
                if (left && race > 0) || (!left && race < 2) {
                    self.sound(MOVE_SOUND);
                    self.remove_border_at(race);
                    let race = if left { race - 1 } else { race + 1 };
                    self.campaign.selected_race = race;
                    let mut screen = std::mem::take(&mut self.screen);
                    self.border_at(&mut screen, race);
                    self.screen = screen;
                    self.shown = self.screen.clone();
                }
                self.after_choice()
            }
            keys::ENTER | 0x9C => self.choose_race(),
            keys::ESCAPE => {
                let campaign = &mut self.campaign;
                let sign_up = campaign.sign_up.as_mut().expect("a sign-up is on");
                let entries = sign_up.fill_at_once(&mut campaign.rand, &campaign.drivers);
                for (race, place) in entries {
                    let driver = self.sign_up().entrants[race][place];
                    self.draw_entry(race, place, driver);
                }
                self.shown = self.screen.clone();
                self.after_choice()
            }
            _ => self.after_choice(),
        }
    }

    fn sign_up(&self) -> &SignUp {
        self.campaign.sign_up.as_ref().expect("a sign-up is on")
    }

    /// `addParticipantToRace(chance)`: a driver who signs up is listed and shown.
    fn add_driver(&mut self, chance: i32) {
        let campaign = &mut self.campaign;
        let sign_up = campaign.sign_up.as_mut().expect("a sign-up is on");
        let Some((race, place)) = sign_up.add_driver(chance, &mut campaign.rand, &campaign.drivers)
        else {
            return;
        };
        let driver = sign_up.entrants[race][place];
        self.draw_entry(race, place, driver);
        self.shown = self.screen.clone();
    }

    /// An entry: the rank padded to two, a dot, the name upper-cased, in the medium font.
    fn draw_entry(&mut self, race: usize, place: usize, driver: usize) {
        let entrant = self.campaign.drivers[driver];
        let mut text = format!("{:>2}.", entrant.rank).into_bytes();
        text.extend(entrant.name().to_ascii_uppercase());
        let at_entry = at(
            ENTRY.0 + COLUMN * race,
            ENTRY.1 + ENTRY_HEIGHT * (place + 1),
        );
        self.graphics.medium.draw(&mut self.screen, &text, at_entry);
    }

    /// Enter while choosing: a warning, a full race, or the player's sign-up.
    fn choose_race(&mut self) -> State {
        let race = self.campaign.selected_race;
        let car = self.campaign.player().car;
        if race == 1 && self.campaign.warn_medium && car < 2 {
            self.campaign.warn_medium = false;
            return self.race_warning(0);
        }
        if race == 2 && self.campaign.warn_hard && car < 4 {
            self.campaign.warn_hard = false;
            return self.race_warning(1);
        }
        if self.sign_up().counts[race] >= 4 {
            self.sound(FULL_SOUND);
            return self.after_choice();
        }
        self.sound(SIGN_UP_SOUND);
        let place = self.sign_up().counts[race];
        let line = self.assets.menu.sign_line.clone();
        self.screen.draw(
            &line,
            at(27 + COLUMN * race, 280 + ENTRY_HEIGHT * place),
            true,
        );
        let sign_up = self.campaign.sign_up.as_mut().expect("a sign-up is on");
        let place = sign_up.enter(race, PLAYER);
        self.campaign.entered_race = Some(race);
        self.draw_entry(race, place, PLAYER);
        self.shown = self.screen.clone();
        State::SignUp {
            second: false,
            phase: Phase::Filling,
        }
    }

    /// `selectRaceWarningPopup`: the race is too hard for the car; any key.
    fn race_warning(&mut self, warning: usize) -> State {
        self.saved = self.screen.clone();
        self.graphics
            .popup(&mut self.screen, 33, 182, 580, 145, Focus::Focused);
        let lines = self.assets.menu.texts.campaign.race_warnings[warning].clone();
        for (line, text) in lines.iter().enumerate() {
            self.graphics
                .write_text(&mut self.screen, text, at(60, 196 + 16 * line));
        }
        let press = self.assets.menu.texts.configure.press_any_key.clone();
        self.graphics.small[0].draw(&mut self.screen, &press, at(208, 295));
        self.shown = self.screen.clone();
        self.sound_at(POPUP_SOUND, POPUP_PITCH);
        State::RaceWarning
    }

    /// A wait of the warning; a key puts the sign-up back.
    pub(super) fn race_warning_tick(&mut self) -> State {
        self.palette.after_wait();
        if self.keys.take() == 0 {
            return State::RaceWarning;
        }
        self.screen.copy_from(&self.saved, at(0, 170), 640, 200);
        self.shown = self.screen.clone();
        self.after_choice()
    }

    /// The end of a choosing pass (`LABEL_26`): with every race full and the player in none,
    /// the popup saying so.
    fn after_choice(&mut self) -> State {
        if !self.sign_up().full() {
            return State::SignUp {
                second: false,
                phase: Phase::Choosing,
            };
        }
        self.graphics
            .popup(&mut self.screen, 33, 200, 580, 112, Focus::Focused);
        let texts = &self.assets.menu.texts;
        let none = texts.campaign.no_sign_up.clone();
        let press = texts.configure.press_any_key.clone();
        self.graphics
            .big_a
            .draw(&mut self.screen, &none, at(48, 225));
        self.graphics.small[0].draw(&mut self.screen, &press, at(208, 261));
        self.shown = self.screen.clone();
        self.sound_at(POPUP_SOUND, POPUP_PITCH);
        State::NoSignUp
    }

    /// A wait of the "no race" popup; a key fades the screen out.
    pub(super) fn no_sign_up_tick(&mut self) -> State {
        self.palette.after_wait();
        if self.keys.take() == 0 {
            return State::NoSignUp;
        }
        self.palette.set_colour(self.player_copper());
        self.palette.compose();
        State::NoSignUpFade { step: 0 }
    }

    /// The fade to black after the "no race" popup: 51 steps of 2 %.
    pub(super) fn no_sign_up_fade(&mut self, step: u32) -> State {
        self.palette.fade(100 - 2 * i64::from(step));
        if step < 50 {
            return State::NoSignUpFade { step: step + 1 };
        }
        self.race_stand_in()
    }

    /// Every race is full: the entrants sorted, the other drivers' damage repaired
    /// (`sabotageScreen` 0x42DD10), the hitman's chance (0x431B30), then the screen lingers.
    fn signed_up(&mut self) -> State {
        let campaign = &mut self.campaign;
        campaign
            .sign_up
            .as_mut()
            .expect("a sign-up is on")
            .sort_entrants();
        for (index, driver) in campaign.drivers.iter_mut().enumerate() {
            if index != PLAYER {
                driver.damage = 0;
            }
        }
        if campaign.rand.next() % 100 < campaign.hitman_chance && campaign.use_weapons {
            // The hitman's offer comes with the Underground Market (M3c); until then his
            // visit is skipped after the original's first draw.
            campaign.hitman_chance = 5;
        } else if campaign.hitman_chance < HITMAN_TOP {
            campaign.hitman_chance += HITMAN_RISE;
        }
        State::Linger { waits: 0 }
    }

    /// A wait after the sign-up: a key or the 280th wait ends it.
    pub(super) fn linger_tick(&mut self, waits: u32) -> State {
        self.palette.after_wait();
        let waits = waits + 1;
        if self.keys.take() == 0 && waits < LINGER_WAITS {
            return State::Linger { waits };
        }
        self.race_stand_in()
    }

    /// Until races exist (M4), the race the player signed up for ends at once: back to the
    /// Start Racing menu with nothing changed but the welcome, which the shop would have
    /// shown by now (spec M3a §2).
    fn race_stand_in(&mut self) -> State {
        self.campaign.welcome = false;
        self.campaign.sign_up = None;
        self.palette.set_colour(self.player_copper());
        self.palette.compose();
        self.palette.fade(100);
        self.start_wipe()
    }
}
