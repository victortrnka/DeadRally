//! The Adversary's screen (spec M6): `adversaryPreviewScreen` (0x435320), where a player who
//! leads every other driver on points goes from the shop. Enter starts the race in the
//! Arena; Escape backs out to the other races' results.

use crate::campaign::SignUp;
use crate::canvas::at;
use crate::keys;

use super::hall_of_fame::Wipe;
use super::results::ResultsFrom;
use super::{Menu, State};

/// The screen's pictures and border (0x43541E on): the title, the Adversary, the box saying
/// Escape backs out.
const TITLE: (usize, usize) = (40, 92);
const ADVERSARY: (usize, usize) = (176, 128);
const ESCAPE_BOX: (usize, usize) = (28, 242);
const BORDER: (usize, usize, usize, usize) = (170, 120, 268, 244);
/// The menus' background rows the screen takes afresh: 273 from line 90, 16 from line 367.
const ROWS: [(usize, usize); 2] = [(90, 273), (367, 16)];
/// The copper entries shown at full brightness first.
const COPPER: std::ops::Range<usize> = 176..183;
/// The waits before the race starts by itself.
const WAITS: u32 = 480;
/// Escape's sound (0x4355CF): effect 23 on channel 1, a little lower than the menu's.
const ESCAPE_SOUND: u8 = 0x17;
const ESCAPE_PITCH: u32 = 0x2_8000 - 0x1000;
/// The fade after Escape: 51 waits from 100 % down 2 % a wait, the music from 0xFFDC down
/// 0x51E a wait when the screen was reached through the Underground Market.
const OUT_STEPS: u32 = 51;
const VOLUME: u32 = 0xFFDC;
const VOLUME_STEP: u32 = 0x51E;
/// The four cars' places drawn after Escape (0x4356F3).
const PLACES: usize = 4;

impl Menu {
    /// The shop's way on for a leader: the copper entries lit, the screen drawn into the second
    /// buffer over a copy of the shop, then wiped in (`sub_42C4A0`).
    pub(super) fn open_adversary(&mut self) -> State {
        self.palette.show_composed(COPPER);
        let mut back = self.screen.clone();
        for (first, rows) in ROWS {
            back.copy_rows(&self.graphics.background, first, rows);
        }
        self.graphics.panel_frame(&mut back, 0, 371, 639, 109);
        self.graphics.panel_text(&mut back, &self.panel);
        self.draw_side_panel(&mut back);
        let menu = &self.assets.menu;
        back.draw(&menu.adversary_title, at(TITLE.0, TITLE.1), true);
        back.draw(&menu.adversary, at(ADVERSARY.0, ADVERSARY.1), true);
        back.draw(&menu.escape_box, at(ESCAPE_BOX.0, ESCAPE_BOX.1), true);
        let (x, y, w, h) = BORDER;
        self.border(&mut back, x, y, w, h);
        self.back = back;
        State::Wipe {
            wipe: Wipe::Adversary,
            step: 0,
        }
    }

    /// The screen wiped in: shown, then its wait.
    pub(super) fn adversary_shown(&mut self) -> State {
        self.shown = self.screen.clone();
        State::AdversaryWait { waits: 0 }
    }

    /// A wait of the screen: Enter, or the 480th wait, starts the race in the Arena; Escape
    /// backs out; other keys do nothing.
    pub(super) fn adversary_wait(&mut self, waits: u32) -> State {
        self.palette.after_wait();
        let key = self.keys.take();
        let waits = waits + 1;
        match key {
            keys::ESCAPE => {
                self.sound_at(ESCAPE_SOUND, ESCAPE_PITCH);
                self.compose_palette();
                State::AdversaryOut { step: 0 }
            }
            keys::ENTER | 0x9C => self.start_arena(),
            _ if waits >= WAITS => self.start_arena(),
            _ => State::AdversaryWait { waits },
        }
    }

    /// A wait of the fade after Escape (0x4355E6), the music fading with the screen when it
    /// was reached through the Underground Market; then that music's order back as an Escape
    /// from the market brings it, every race filled with the others, four places drawn for
    /// the cars, and the results (`postRaceMain(1)`).
    pub(super) fn adversary_out(&mut self, step: u32) -> State {
        let through_market = self.campaign.use_weapons && self.shop.continue_seen;
        if through_market {
            self.sound.set_mask((VOLUME - VOLUME_STEP * step) >> 8);
        }
        self.palette.fade(100 - 2 * i64::from(step));
        if step + 1 < OUT_STEPS {
            return State::AdversaryOut { step: step + 1 };
        }
        if through_market {
            self.sound.set_music_order(self.music_order);
            self.shop.market_escaped = true;
            self.sound.stop_channel(1);
        }
        let campaign = &mut self.campaign;
        let drivers = campaign.drivers;
        let player = campaign.player_index;
        campaign
            .sign_up
            .get_or_insert_with(|| SignUp::untouched(player))
            .fill_at_once(&mut campaign.rand, &drivers);
        // 0x4356F3: the cars' places, drawn until each is new; the race last entered (0 in a
        // game loaded since the program started) shows them.
        let mut taken = [false; PLACES];
        let mut places = [0; PLACES];
        for place in &mut places {
            loop {
                let drawn = (campaign.rand.next() % PLACES as i32) as usize;
                if !taken[drawn] {
                    taken[drawn] = true;
                    *place = drawn as i32 + 1;
                    break;
                }
            }
        }
        campaign.entered_race = Some(campaign.entered_race.unwrap_or(0));
        self.outcome = crate::books::Outcome {
            finishes: places
                .map(|place| crate::books::Finish {
                    place,
                    ..crate::books::Finish::default()
                })
                .to_vec(),
            ..crate::books::Outcome::default()
        };
        self.results_from = ResultsFrom::AdversaryEscape;
        self.open_results()
    }
}
