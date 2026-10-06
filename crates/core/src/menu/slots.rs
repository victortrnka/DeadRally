//! Saved games (spec M3b): the slots to load from (`loadGame` 0x42F2E0) or save into
//! (`savegameWithName` 0x42F6E0), the name prompt, and the confirmation
//! (`confirmationPopup` 0x42DC70). DreeRally `ui/loadSaveGameScreen.c`, `savegame.c`; dRally
//! `drencryption.c`.

use deadrally_gamedata::save_game::{self, DRIVERS_BYTES, NAME_BYTES, SaveGame};

use super::draw::Focus;
use super::licence::Nickname;
use super::{MAIN_MENU, Menu, START_MENU, State, Submenu};
use crate::campaign::{DRIVER_BYTES, DRIVERS, Driver, PLAYER};
use crate::canvas::at;

/// The slots' rows are menu 5's.
const SLOTS_TEXT: usize = 5;
/// An empty slot sounds this when chosen to load from.
const EMPTY_SOUND: u8 = 29;

/// What follows a confirmation popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Confirmed {
    /// A game was loaded: on to the shop.
    Loaded,
    /// A game was saved: back to the Start Racing menu.
    Saved,
}

impl Menu {
    /// The slots' rows as `loadGame` and `savegameWithName` fill them: a saved game's name,
    /// else the empty slot's text; the last row always the quicksave's.
    pub(super) fn open_slots(&mut self, purpose: Submenu) -> State {
        let texts = &self.assets.menu.texts.campaign;
        let (empty, quick) = (texts.empty_slot.clone(), texts.quicksave_slot.clone());
        for slot in 0..save_game::SLOTS {
            let row = match &self.slot_files[slot] {
                Some(file) => {
                    let name = SaveGame::decode(file).name;
                    // The name's 15 bytes go over the empty text's first ones.
                    let mut row = empty.clone();
                    row.resize(row.len().max(NAME_BYTES), 0);
                    row[..NAME_BYTES].copy_from_slice(&name);
                    let end = row.iter().position(|&b| b == 0).unwrap_or(row.len());
                    row.truncate(end);
                    row
                }
                None => empty.clone(),
            };
            self.graphics.set_row(SLOTS_TEXT, slot, row);
        }
        self.graphics
            .set_row(SLOTS_TEXT, save_game::QUICKSAVE_SLOT, quick);
        self.submenu_pass(purpose)
    }

    /// A slot chosen to load from: its game, or the empty slot's sound and the slots again.
    pub(super) fn load_slot(&mut self, slot: usize) -> State {
        let game = self.slot_files[slot].as_deref().map(SaveGame::decode);
        // Only a single-player game (driver 19) loads; another is as good as empty.
        let Some(game) = game.filter(|game| usize::from(game.driver_id) == PLAYER) else {
            self.sound(EMPTY_SOUND);
            return self.submenu_pass(Submenu::Load);
        };
        let texts = &self.assets.menu.texts.campaign;
        let (shop, racing) = (
            texts.enter_shop_row.clone(),
            texts.continue_racing_row.clone(),
        );
        self.graphics.set_row(START_MENU.text, 0, shop);
        self.graphics.set_row(MAIN_MENU.text, 0, racing);
        let start = &mut self.submenus[Submenu::Start.table()];
        for row in [1, 2, 4] {
            start.active[row] = true;
        }
        let campaign = &mut self.campaign;
        campaign.warn_hard = false;
        campaign.warn_medium = false;
        campaign.underground_popup = false;
        campaign.welcome = false;
        campaign.started = true;
        campaign.use_weapons = game.use_weapons != 0;
        for (driver, record) in campaign
            .drivers
            .iter_mut()
            .zip(game.drivers.as_chunks::<DRIVER_BYTES>().0)
        {
            *driver = Driver::from_bytes(record);
        }
        self.config.set_difficulty(u32::from(game.difficulty));
        let loaded = self.assets.menu.texts.campaign.game_loaded.clone();
        self.confirm(&loaded, Confirmed::Loaded)
    }

    /// A slot chosen to save into: the slots dimmed, the prompt, the name entry with the
    /// slot's name unless it is empty.
    pub(super) fn ask_save_name(&mut self, slot: usize) -> State {
        self.save_slot = slot;
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[Submenu::Save.table()],
            Focus::Unfocused,
            self.cursor,
        );
        self.graphics
            .popup(&mut self.screen, 120, 275, 390, 70, Focus::Focused);
        let texts = &self.assets.menu.texts.campaign;
        let prompt = texts.save_prompt.clone();
        self.graphics.small[0].draw(&mut self.screen, &prompt, at(130, 282));
        let row = self.graphics.row(SLOTS_TEXT, slot).to_vec();
        let name = if row == texts.empty_slot {
            Vec::new()
        } else {
            row
        };
        self.shown = self.screen.clone();
        // `readKeyboard`'s start: the text drawn and its width taken.
        self.graphics
            .big_b
            .draw(&mut self.screen, &name, at(130, 298));
        let width = self.graphics.big_b.width(&name);
        self.shown.copy_from(&self.screen, at(130, 298), width, 32);
        self.nickname = Nickname::save_name(name, width);
        self.palette
            .set_player_ramp(self.assets.menu.copper.0[super::licence::START_COLOUR as usize]);
        State::Nickname
    }

    /// Escape at the name: the slots again.
    pub(super) fn save_name_cancelled(&mut self) -> State {
        self.submenu_pass(Submenu::Save)
    }

    /// The name is in: the game saved with a key from `rand()`, the menus dimmed, the
    /// confirmation.
    pub(super) fn save_name_done(&mut self) -> State {
        let key = (self.campaign.rand.next() % 255) as u8;
        let mut name = [0; NAME_BYTES];
        let length = self.nickname.text.len().min(NAME_BYTES);
        name[..length].copy_from_slice(&self.nickname.text[..length]);
        let mut drivers = Vec::with_capacity(DRIVERS_BYTES);
        for driver in &self.campaign.drivers[..DRIVERS] {
            drivers.extend_from_slice(&driver.to_bytes());
        }
        let game = SaveGame {
            driver_id: PLAYER as u8,
            use_weapons: u8::from(self.campaign.use_weapons),
            difficulty: self.config.difficulty() as u8,
            name,
            drivers,
        };
        let file = game.encode(key);
        self.slot_files[self.save_slot] = Some(file.clone());
        self.written_slot = Some((self.save_slot, file));
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        for menu in [Submenu::Start, Submenu::Save] {
            self.graphics.menu(
                &mut self.screen,
                &self.submenus[menu.table()],
                Focus::Unfocused,
                self.cursor,
            );
        }
        let saved = self.assets.menu.texts.campaign.game_saved.clone();
        self.confirm(&saved, Confirmed::Saved)
    }

    /// `confirmationPopup`: the text centred in big A, "press any key" under it; a key
    /// pressed before is dropped, then the first key ends it.
    fn confirm(&mut self, text: &[u8], then: Confirmed) -> State {
        self.graphics
            .popup(&mut self.screen, 110, 210, 428, 90, Focus::Focused);
        let half = (self.graphics.big_a.width(text) / 2) as i32;
        let x = (324 - half).max(0) as usize;
        self.graphics.big_a.draw(&mut self.screen, text, at(x, 230));
        let press = self.assets.menu.texts.configure.press_any_key.clone();
        self.graphics.small[0].draw(&mut self.screen, &press, at(208, 271));
        self.shown = self.screen.clone();
        self.keys.take();
        if self.keys.take() != 0 {
            return self.confirmed(then);
        }
        State::Confirm { then }
    }

    /// A wait of the confirmation, then its key.
    pub(super) fn confirm_tick(&mut self, then: Confirmed) -> State {
        self.palette.after_wait();
        if self.keys.take() == 0 {
            return State::Confirm { then };
        }
        self.confirmed(then)
    }

    fn confirmed(&mut self, then: Confirmed) -> State {
        self.keys.take();
        match then {
            Confirmed::Loaded => self.enter_shop(),
            Confirmed::Saved => self.start_pass(),
        }
    }
}
