//! The race's results (spec M5): `postRaceMain` (0x42B290, from 0x42B709) after a race the
//! player was in. The standings and the easy race's places and points fade in; a key shows
//! the medium race's, another the hard race's (Escape adds their points without showing
//! them); then the standings sorted afresh with the player's statistics, the wait while the
//! shop loads, a key, and the way back.

use crate::books::WRECKED;
use crate::campaign::quicksort;
use crate::canvas::{Canvas, at};
use crate::keys;

use super::draw::Focus;
use super::{Menu, State, Submenu, shop};

/// The ranking's frame and the results' panel (0x42B728, 0x42B74C).
const RANKING: (usize, usize) = (300, 84);
const PANEL: (usize, usize) = (354, 84);
/// `writeDriverList` (0x425980): a row every 19 lines from line 87, its pieces at 29, 61 and
/// 163; the rank right-aligned to 62 on line 90, the name at 66 and the points right-aligned
/// to 200 on line 89.
const ROW_TOP: usize = 87;
const ROW_STEP: usize = 19;
const ROW_PIECES: [usize; 3] = [29, 61, 163];
const RANK_RIGHT: (usize, usize) = (62, 90);
const NAME: (usize, usize) = (66, 89);
const POINTS_RIGHT: (usize, usize) = (200, 89);
/// A race's title picture and text (0x429401, 0x42941C), and the points' marker and text by
/// a driver's row (0x429481, 0x42959F).
const TITLE_TEXT: (usize, usize) = (400, 86);
const MARKER_X: usize = 217;
const POINTS_TEXT_X: usize = 230;
const POINTS_TEXT_DY: usize = 2;
/// The points of a race's first three places, by race.
const POINTS: [[i32; 3]; 3] = [[3, 2, 1], [5, 3, 1], [10, 7, 4]];
/// `sub_424420` and `drawRightPositions` (0x425BD0): four places 85 lines apart from line 114;
/// the place's box at 389, its number at 401 (the first) or 396, 7 lines down; the rank
/// right-aligned to 420 52 lines down, the name at 495 57 lines down, the face at 422 3 lines
/// down and the car at 490.
const PLACES: usize = 4;
const PLACE_TOP: usize = 114;
const PLACE_STEP: usize = 85;
const BOX_X: usize = 389;
const NUMBER_X: [usize; 2] = [401, 396];
const NUMBER_DY: usize = 7;
const PLACE_RANK: (usize, usize) = (420, 52);
const PLACE_NAME: (usize, usize) = (495, 57);
const FACE: (usize, usize) = (422, 3);
const CAR_X: usize = 490;
/// The palette entries each place's car is drawn with (0x425D91 on).
const PLACE_RAMPS: [usize; PLACES] = [0x40, 0x50, 0xE0, 0xF0];
/// The line asking for a key and its blink (0x4260D0): small A at (366, 452), small B
/// from the 30th call, small A again at the 60th; the panel's rows under it put back first
/// (`sub_426080`), and the line's strip shown.
const PRESS: (usize, usize) = (366, 452);
const PRESS_STRIP: (usize, usize, usize, usize) = (354, 452, 270, 16);
const PRESS_PANEL_ROWS: std::ops::Range<usize> = 369..386;
const BLINK_B: u32 = 30;
const BLINK_A: u32 = 60;
/// The music's volume a step of the fade in raises it by, when it does.
const FADE_IN_VOLUME_STEP: u32 = 0x51E;
/// The fade in's 50 steps of 2 % and the way out's 51 waits (100 % down to 0).
const FADE_IN_STEPS: u32 = 50;
const OUT_STEPS: u32 = 51;
/// The sound when the shop has loaded (0x42B9A2).
const LOADED_SOUND: u8 = 0x1C;
/// `drawStadistics`: its title, each row's label at 360 and value at 526, 23 lines apart
/// from line 115 with a gap before the race's heading on line 247.
const STATISTICS_TITLE: (usize, usize) = (416, 86);
const STATISTICS_LABEL_X: usize = 360;
const STATISTICS_VALUE_X: usize = 526;
const STATISTICS_ROWS_Y: [usize; 13] = [
    115, 138, 161, 184, 207, 270, 293, 316, 339, 362, 385, 408, 431,
];
const RACE_HEADING_Y: usize = 247;

/// A time as the statistics write it: minutes, seconds and hundredths, each of one digit
/// with a 0 before it, as "00:05.65".
fn clock([minutes, seconds, hundredths]: [i32; 3]) -> String {
    format!("{minutes:02}:{seconds:02}.{hundredths:02}")
}

/// A time in hundredths, as the records are compared; the original's sums wrap on a
/// hand-made `dr.cfg` (0x42520A).
fn clock_total([minutes, seconds, hundredths]: [i32; 3]) -> i32 {
    minutes
        .wrapping_mul(60)
        .wrapping_add(seconds)
        .wrapping_mul(100)
        .wrapping_add(hundredths)
}

/// 0x425241, 0x425318: whether the best lap becomes the circuit's record for the car: a lap
/// set and better than the record, or any lap on a record with nothing in it.
fn beats_record(best: [i32; 3], record: [u32; 3]) -> bool {
    let record = record.map(|part| part as i32);
    let sum = |time: [i32; 3]| time[0].wrapping_add(time[1]).wrapping_add(time[2]);
    let best_total = clock_total(best);
    (best_total < clock_total(record) && sum(best) != 0) || (sum(record) == 0 && best_total > 0)
}

/// What a race's page does for one of its first three places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Award {
    /// The points' marker by the driver's row, when the page is shown.
    Marker,
    /// The points added and their text drawn.
    Points,
}

/// A race's page's first three places in order (0x42943B–0x4295A7), given which are wrecked
/// and which is the player lapped: a wreck gets nothing; the player lapped gets nothing and,
/// when the page is shown, ends it there, so the places after get nothing either; when
/// Escape skips the page the places after still get their points.
fn page_awards(wrecked: [bool; 3], shut_out: [bool; 3], skip: bool) -> Vec<(usize, Award)> {
    let mut awards = Vec::new();
    for place in 0..3 {
        if !skip && !wrecked[place] {
            if shut_out[place] {
                break;
            }
            awards.push((place, Award::Marker));
        }
        if !wrecked[place] && !shut_out[place] {
            awards.push((place, Award::Points));
        }
    }
    awards
}

/// What the results follow (`postRaceMain`'s argument): a race (0), or no race (1) after
/// signing up for none or Escape on the Adversary's screen, which then also puts the sabotage
/// on sale again unless the player leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResultsFrom {
    Race,
    NoRace,
    AdversaryEscape,
}

/// The animations the menus play (`openAnimation`, 0x4185B0): the Adversary's when a player
/// first leads, and the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Film {
    Adversary,
    End,
}

impl Menu {
    /// The results after signing up for no race (0x435CC2): `postRaceMain(1)`, which leaves
    /// the shop's loading out; every race is full already.
    pub(super) fn results_without_race(&mut self) -> State {
        self.results_from = ResultsFrom::NoRace;
        self.open_results()
    }

    /// `postRaceMain(0)` after a race, `postRaceMain(1)` after none: the standings and the easy
    /// race's page drawn under a black palette, then faded in.
    pub(super) fn open_results(&mut self) -> State {
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_results_frame(&mut screen);
        self.draw_standings(&mut screen);
        self.screen = screen;
        self.race_page(0, false);
        State::ResultsFadeIn { step: 0 }
    }

    /// A step of the fade in, each after a wait: the composed palette at 2 % a step, up to
    /// 98 %.
    pub(super) fn results_fade_in(&mut self, step: u32) -> State {
        if self.shop.market_escaped {
            // 0x42B774: the music comes back up with the screen after signing up for no race.
            self.sound.set_mask((FADE_IN_VOLUME_STEP * step) >> 8);
        }
        self.palette.fade(2 * i64::from(step));
        if step + 1 < FADE_IN_STEPS {
            return State::ResultsFadeIn { step: step + 1 };
        }
        // 0x42B800.
        self.shop.market_escaped = false;
        State::ResultsWait { page: 1 }
    }

    /// A wait for a key after page `page` (the races' 1 to 3, 4 after the shop has loaded):
    /// the menu's pulse and the line blinking. Escape after the first two races' pages adds
    /// the rest's points without showing them.
    pub(super) fn results_wait(&mut self, page: u8) -> State {
        self.palette.after_wait();
        self.blink_press();
        let key = self.keys.take();
        if key == 0 {
            return State::ResultsWait { page };
        }
        let skip = key == keys::ESCAPE;
        match page {
            1 | 2 => {
                let race = usize::from(page);
                self.race_page(race, skip);
                if !skip {
                    return State::ResultsWait { page: page + 1 };
                }
                if race == 1 {
                    self.race_page(2, true);
                }
                self.results_statistics()
            }
            3 => self.results_statistics(),
            _ => {
                // 0x42BA03: with weapons on, the welcome shown and the shop's way on seen,
                // the screen stays as it is; otherwise it fades out.
                self.compose_palette();
                let fade = !(self.campaign.use_weapons
                    && !self.campaign.welcome
                    && self.shop.continue_seen);
                State::ResultsOut { step: 0, fade }
            }
        }
    }

    /// After the races' pages: the standings sorted afresh (`sub_423C90`, `sub_423E20`) with
    /// the player's statistics, the line saying the shop loads while it does.
    fn results_statistics(&mut self) -> State {
        self.campaign.rank_drivers();
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_statistics(&mut screen);
        self.draw_standings(&mut screen);
        if self.results_from == ResultsFrom::Race {
            // 0x42B8B3: the player leading now and not at the last results.
            let leads = self.campaign.player_leads();
            self.newly_leading = leads && !self.campaign.was_leading;
            self.campaign.was_leading = leads;
        }
        if self.results_from != ResultsFrom::Race {
            // 0x42B9C6: no shop to load after no race.
            self.screen = screen;
            self.draw_press(0);
            self.shown = self.screen.clone();
            return State::ResultsWait { page: 4 };
        }
        self.clear_press(&mut screen);
        let wait = self.assets.menu.texts.campaign.please_wait.clone();
        self.graphics.small[0].draw(&mut screen, &wait, at(PRESS.0, PRESS.1));
        self.screen = screen;
        self.shown = self.screen.clone();
        // 0x42B975: loading the menus' pictures (0x419950) turns the cursor back to its first
        // frame, which the shop's popups show.
        self.cursor = 0;
        // 0x42B97A: `sub_41EE40` loads the shop's pictures and starts its loops afresh: the
        // car box on the player's next car, the continue item selected, the pulse at 100 %.
        self.shop.reset();
        self.shop.selected = shop::CONTINUE;
        self.shop.car = (self.campaign.player().car + 1).clamp(1, 5) as usize;
        self.car_frame = 0;
        self.palette.reset_pulse();
        State::ResultsLoading
    }

    /// The shop loaded: the sound, and the line asking for a key in place of the wait's
    /// line.
    pub(super) fn results_loaded(&mut self) -> State {
        let mut screen = std::mem::take(&mut self.screen);
        self.clear_press(&mut screen);
        self.screen = screen;
        self.sound(LOADED_SOUND);
        self.draw_press(0);
        self.shown = self.screen.clone();
        State::ResultsWait { page: 4 }
    }

    /// A wait of the way out: the composed palette from 100 % down 2 % a wait but for entries
    /// 96 to 127, or nothing changing; then the key the wait took is let go of and the shop
    /// comes back, or the Underground Market the race was reached through fades out first
    /// (0x4370C7).
    pub(super) fn results_out(&mut self, step: u32, fade: bool) -> State {
        let level = 100 - 2 * i64::from(step);
        if self.newly_leading {
            // 0x42BA45, 0x42B4CE: every entry for a new leader, whichever way out.
            self.palette.fade(level);
        } else if fade {
            // 0x42BAA0: entries 96 to 127 keep what they show.
            self.palette.fade_market(level);
        }
        if step + 1 < OUT_STEPS {
            return State::ResultsOut {
                step: step + 1,
                fade,
            };
        }
        if self.newly_leading {
            // 0x42BB6F: the Adversary's animation, its music and effects.
            self.sound.stop();
            self.film = Some(crate::animation::Player::new(&self.assets.letterbox));
            self.sound
                .play_music(&self.assets.intro_music, 0, crate::audio::FULL_VOLUME);
            self.sound.load_effects(&self.assets.adversary_effects);
            return State::Film {
                film: Film::Adversary,
                fade,
            };
        }
        self.results_left(fade)
    }

    /// The results left: the key the last wait took let go of, then the shop or the
    /// Underground Market the race was entered through.
    fn results_left(&mut self, fade: bool) -> State {
        self.keys.take();
        if self.results_from == ResultsFrom::AdversaryEscape {
            // 0x4357B3: the sabotage on sale again unless the player leads.
            self.campaign.stock[3] = i32::from(!self.campaign.player_leads());
        }
        if fade {
            return self.shop_again();
        }
        self.compose_palette();
        State::MarketLeave { step: 0 }
    }

    /// The Start Racing menu's statistics row (`sub_42C940`): the palette composed, then the menu out from
    /// 100 % in 51 waits, the Start Racing menu's cursor turning every other one.
    pub(super) fn open_statistics(&mut self) -> State {
        self.compose_palette();
        State::StatsMenuOut {
            step: OUT_STEPS - 1,
        }
    }

    /// A wait of the menu's way out, `step` counting down from 50: all but entries 96 to 127
    /// (the title's, which stay as they are through the statistics).
    pub(super) fn stats_menu_out(&mut self, step: u32) -> State {
        if step % 2 == 1 {
            self.update_cursor_start();
        }
        self.palette.fade_market(2 * i64::from(step));
        if step > 0 {
            return State::StatsMenuOut { step: step - 1 };
        }
        self.keys.take();
        self.keys.take();
        // `postRaceMain(2)`: the statistics without a race's part, the standings as they
        // stand, the line, all shown under the black palette.
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_statistics(&mut screen);
        self.draw_standings(&mut screen);
        self.screen = screen;
        self.draw_press(0);
        self.shown = self.screen.clone();
        self.compose_palette();
        State::StatsIn { step: 0 }
    }

    /// A wait of the statistics' way in: all but entries 96 to 127 up to 98 %.
    pub(super) fn stats_in(&mut self, step: u32) -> State {
        self.palette.fade_market(2 * i64::from(step));
        if step + 1 < FADE_IN_STEPS {
            return State::StatsIn { step: step + 1 };
        }
        State::StatsWait
    }

    /// The statistics' wait for a key, the pulse and the line blinking as on the results.
    pub(super) fn stats_wait(&mut self) -> State {
        self.palette.after_wait();
        self.blink_press();
        if self.keys.take() == 0 {
            return State::StatsWait;
        }
        self.compose_palette();
        State::StatsOut { step: 0 }
    }

    /// A wait of the statistics' way out: all but entries 96 to 127 from 100 % down to 0.
    pub(super) fn stats_out(&mut self, step: u32) -> State {
        self.palette.fade_market(100 - 2 * i64::from(step));
        if step + 1 < OUT_STEPS {
            return State::StatsOut { step: step + 1 };
        }
        // 0x42B4B2: the palette composed, then 51 waits with nothing changing.
        self.compose_palette();
        State::StatsHold { waits: 0 }
    }

    /// One of the 51 waits after the statistics; then the key let go of and the menus drawn
    /// afresh under what the palette shows: the background, the main menu dimmed, the Start
    /// Racing menu, the bottom panel.
    pub(super) fn stats_hold(&mut self, waits: u32) -> State {
        if waits + 1 < OUT_STEPS {
            return State::StatsHold { waits: waits + 1 };
        }
        self.keys.take();
        let mut screen = std::mem::take(&mut self.screen);
        screen.copy_all(&self.graphics.background);
        self.graphics
            .menu(&mut screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics.menu(
            &mut screen,
            &self.submenus[Submenu::Start.table()],
            Focus::Focused,
            self.cursor,
        );
        self.graphics.panel_frame(&mut screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut screen, &self.panel);
        self.screen = screen;
        self.shown = self.screen.clone();
        self.compose_palette();
        State::StatsMenuIn { step: 0 }
    }

    /// A wait of the menu's way back in: all but entries 96 to 127 up to 98 %, the cursor
    /// turning every other wait; then the keys pressed meanwhile let go of.
    pub(super) fn stats_menu_in(&mut self, step: u32) -> State {
        if step % 2 == 1 {
            self.update_cursor_start();
        }
        self.palette.fade_market(2 * i64::from(step));
        if step + 1 < FADE_IN_STEPS {
            return State::StatsMenuIn { step: step + 1 };
        }
        self.keys.take();
        self.keys.take();
        State::Submenu {
            menu: Submenu::Start,
            second: false,
        }
    }

    /// The Start Racing menu's cursor turned a frame (`sub_41AB50(1)`).
    fn update_cursor_start(&mut self) {
        self.graphics.update_cursor(
            &mut self.screen,
            &mut self.shown,
            &self.submenus[Submenu::Start.table()],
            self.cursor,
        );
        self.cursor = (self.cursor + 1) % super::CURSOR_FRAMES;
    }

    /// A tick of an animation; when it ends, the menus' music back. After the Adversary's
    /// (0x42BC45), the screen shown cleared, the palette composed with the title's entries 96
    /// to 127 lit, then the way out of the results goes on.
    pub(super) fn film_tick(&mut self, film: Film, fade: bool) -> State {
        let animation = match film {
            Film::Adversary => &self.assets.adversary_animation,
            Film::End => &self.assets.end_animation,
        };
        let player = self.film.as_mut().expect("an animation is playing");
        if player.tick(animation, &mut self.keys, &mut self.sound)
            == crate::animation::Tick::Playing
        {
            return State::Film { film, fade };
        }
        self.film = None;
        if film == Film::End {
            return self.after_the_end();
        }
        self.menu_sound_back();
        // 0x42BC87, 0x42B657: `sub_43BE60` clears the screen shown as it sets the menus' mode
        // back. The shop draws over it at once; the Underground Market's fade on the way out
        // that keeps the screen shows it black.
        self.shown.pixels_mut().fill(0);
        self.newly_leading = false;
        if !fade {
            // 0x42B66C: the way out that keeps the screen forgets what the sponsors would have
            // paid for; the fading one does not.
            self.campaign.win_streak = 0;
            self.campaign.clean_race = false;
            self.campaign.all_wrecked = false;
        }
        self.compose_palette();
        self.palette.show_composed(96..128);
        self.results_left(fade)
    }

    /// The menu's background with the ranking's frame and the results' panel.
    fn draw_results_frame(&self, canvas: &mut Canvas) {
        let results = &self.assets.menu.results;
        canvas.copy_all(&self.graphics.background);
        canvas.draw(&results.ranking, at(RANKING.0, RANKING.1), true);
        canvas.draw(&results.panel, at(PANEL.0, PANEL.1), true);
    }

    /// `writeDriverList(20)`: every driver's row in the drivers' order, the player's in its
    /// own pieces.
    fn draw_standings(&self, canvas: &mut Canvas) {
        let results = &self.assets.menu.results;
        let medium = &self.graphics.medium;
        for (index, driver) in self.campaign.drivers.iter().enumerate() {
            let dy = ROW_STEP * index;
            let pieces = if index == self.campaign.player_index {
                &results.player_row
            } else {
                &results.other_row
            };
            for (piece, &x) in pieces.iter().zip(&ROW_PIECES) {
                canvas.draw(piece, at(x, ROW_TOP + dy), true);
            }
            let rank = format!("{}.", driver.rank).into_bytes();
            let pen = RANK_RIGHT.0.saturating_sub(medium.width(&rank));
            medium.draw(canvas, &rank, at(pen, RANK_RIGHT.1 + dy));
            let name = driver.name().to_ascii_uppercase();
            medium.draw(canvas, &name, at(NAME.0, NAME.1 + dy));
            let points = driver.points.to_string().into_bytes();
            let pen = POINTS_RIGHT.0.saturating_sub(medium.width(&points));
            medium.draw(canvas, &points, at(pen, POINTS_RIGHT.1 + dy));
        }
    }

    /// A race's page (`easyRaceResults` 0x429280 and the medium's and hard's after it): its
    /// places, the first three's points added to their drivers (none for a wreck, nor for
    /// the player lapped), shown unless `skip`.
    fn race_page(&mut self, race: usize, skip: bool) {
        let places = self.race_places(race);
        let mut screen = std::mem::take(&mut self.screen);
        if !skip {
            self.place_boxes(&mut screen);
            let results = &self.assets.menu.results;
            screen.draw(&results.races[race], at(PANEL.0, PANEL.1), true);
            let title = &self.assets.menu.texts.campaign.results_titles[race];
            self.graphics.small[0].draw(&mut screen, title, at(TITLE_TEXT.0, TITLE_TEXT.1));
            self.right_positions(&mut screen, &places);
        }
        let lapped = self.outcome.lapped;
        let player = self.campaign.player_index;
        let wrecked = [0, 1, 2].map(|place| self.campaign.drivers[places[place]].damage == WRECKED);
        let shut_out = [0, 1, 2].map(|place| places[place] == player && lapped);
        for (place, award) in page_awards(wrecked, shut_out, skip) {
            let driver = places[place];
            match award {
                Award::Marker => {
                    let marker = &self.assets.menu.results.points[race];
                    screen.draw(marker, at(MARKER_X, ROW_TOP + ROW_STEP * driver), true);
                }
                Award::Points => {
                    let record = &mut self.campaign.drivers[driver];
                    record.points = record.points.wrapping_add(POINTS[race][place]);
                    let text = &self.assets.menu.texts.campaign.results_points[race][place];
                    let y = ROW_TOP + POINTS_TEXT_DY + ROW_STEP * driver;
                    self.graphics
                        .medium
                        .draw(&mut screen, text, at(POINTS_TEXT_X, y));
                }
            }
        }
        self.screen = screen;
        if skip {
            return;
        }
        if race == 0 {
            self.draw_press(0);
        } else {
            // 0x429915: the composed palette at 100 %.
            self.palette.fade(100);
        }
        self.shown = self.screen.clone();
    }

    /// The drivers of race `race` by their places: the player's race as it finished; the
    /// others by their cars, the best first (`sub_424510` sorts the entries by car, lowest
    /// first, in the sign-up itself, and the page reads them backwards).
    fn race_places(&mut self, race: usize) -> [usize; PLACES] {
        let player_race = self.campaign.entered_race;
        let drivers = self.campaign.drivers;
        let sign_up = self.campaign.sign_up.as_mut().expect("a sign-up is on");
        let entrants = &mut sign_up.entrants[race];
        if player_race == Some(race) {
            let mut places = *entrants;
            for (car, &driver) in entrants.iter().enumerate() {
                let place = self.outcome.finishes.get(car).map_or(0, |f| f.place);
                if (1..=PLACES as i32).contains(&place) {
                    places[place as usize - 1] = driver;
                }
            }
            return places;
        }
        quicksort(
            entrants,
            0,
            PLACES - 1,
            &|&driver: &usize| drivers[driver].car,
            &mut |_, _| {},
        );
        let mut places = *entrants;
        places.reverse();
        places
    }

    /// `sub_424420`: the four places' boxes and numbers.
    fn place_boxes(&self, canvas: &mut Canvas) {
        let placing = &self.assets.menu.results.placing;
        for place in 0..PLACES {
            let y = PLACE_TOP + PLACE_STEP * place;
            canvas.draw(placing, at(BOX_X, y), true);
            let number = (place + 1).to_string().into_bytes();
            let x = NUMBER_X[usize::from(place > 0)];
            self.graphics
                .big_a
                .draw(canvas, &number, at(x, y + NUMBER_DY));
        }
    }

    /// `drawRightPositions(4, places)`: each place's rank, name, face and car; then the palette
    /// composed with each place's car in its driver's colour (the player's from `COPPER.PAL`,
    /// the others' from the cars' colours).
    fn right_positions(&mut self, canvas: &mut Canvas, places: &[usize; PLACES]) {
        let menu = &self.assets.menu;
        let medium = &self.graphics.medium;
        for (place, &driver) in places.iter().enumerate() {
            let record = &self.campaign.drivers[driver];
            let y = PLACE_TOP + PLACE_STEP * place;
            let rank = format!("{}.", record.rank).into_bytes();
            let pen = PLACE_RANK.0.saturating_sub(medium.width(&rank));
            medium.draw(canvas, &rank, at(pen, y + PLACE_RANK.1));
            let name = record.name().to_ascii_uppercase();
            medium.draw(canvas, &name, at(PLACE_NAME.0, y + PLACE_NAME.1));
            // A face or car past the pictures (only from an edited save) is left out.
            if let Some(face) = usize::try_from(record.face)
                .ok()
                .and_then(|f| menu.faces.get(f))
            {
                canvas.draw(face, at(FACE.0, y + FACE.1), false);
            }
            if let Some(car) = usize::try_from(record.car)
                .ok()
                .and_then(|car| menu.results.cars.get(car * PLACES + place))
            {
                canvas.draw(car, at(CAR_X, y), false);
            }
        }
        let colours = places.map(|driver| {
            let colour = self.campaign.drivers[driver].colour.clamp(0, 255) as usize;
            if driver == self.campaign.player_index {
                menu.copper.0[colour]
            } else {
                menu.car_colours.0[colour]
            }
        });
        self.compose_palette();
        for (&first, &colour) in PLACE_RAMPS.iter().zip(&colours) {
            self.palette.set_place_ramp(first, colour);
        }
    }

    /// The player's best lap of the race just over becomes the record of their car on its
    /// circuit when better, or the first (0x425241, 0x425318). Whether it did.
    fn keep_lap_record(&mut self) -> bool {
        let race = self.campaign.entered_race.unwrap_or(0);
        let circuit = self.records_circuit(race);
        let player = *self.campaign.player();
        let car = player.car.clamp(0, 5) as usize;
        let best = self.outcome.best_lap;
        let (_, record) = self.config.record(circuit, car);
        let kept = beats_record(best, record);
        if kept {
            self.config
                .set_record(circuit, car, player.name(), best.map(|part| part as u32));
        }
        kept
    }

    /// DeadRally's race record: the time of a race the player drove to its end becomes the
    /// record of their car over the race's laps of its circuit when better, or the first.
    /// Whether it did.
    fn keep_race_record(&mut self) -> bool {
        if !self.outcome.whole {
            return false;
        }
        let race = self.campaign.entered_race.unwrap_or(0);
        let circuit = self.records_circuit(race);
        let laps = self.outcome.laps;
        let player = *self.campaign.player();
        let car = player.car.clamp(0, 5) as usize;
        let time = self.outcome.race_time;
        let (_, record) = self.config.race_record(circuit, laps, car);
        let kept = beats_record(time, record);
        if kept {
            self.config.set_race_record(
                circuit,
                laps,
                car,
                player.name(),
                time.map(|part| part as u32),
            );
        }
        kept
    }

    /// DeadRally's records as the race ends: the lap record and the race record. The
    /// original keeps the lap record on the statistics, which a won Arena never shows, and
    /// writes `dr.cfg` only at the main menu's Quit, in Configure and for the best ten, so a
    /// window closed before lost the records; DeadRally hands `dr.cfg` over at once.
    pub(super) fn keep_records(&mut self) {
        let lap = self.keep_lap_record();
        let race = self.keep_race_record();
        if lap || race {
            self.save = true;
        }
    }

    /// `drawStadistics` (0x4245D0): the player's statistics in small A beside the standings,
    /// and after a race (the player's place known) the race's with the record of the player's
    /// car on its circuit, which the Windows version keeps here.
    fn draw_statistics(&mut self, canvas: &mut Canvas) {
        if self.books.place > 0 && self.campaign.windows_version {
            self.keep_lap_record();
        }
        self.draw_results_frame(canvas);
        let texts = &self.assets.menu.texts.campaign;
        let font = &self.graphics.small[0];
        font.draw(
            canvas,
            &texts.statistics,
            at(STATISTICS_TITLE.0, STATISTICS_TITLE.1),
        );
        let player = *self.campaign.player();
        let separator = &texts.label_separator;
        let plain = |value: String| [&separator[..], value.as_bytes()].concat();
        let money = |value: i32| plain(format!("${value}"));
        let mut rows = vec![
            plain(format!("{}.", player.rank)),
            plain(player.wins.to_string()),
            plain(player.races.to_string()),
            money(player.total_income),
            money(player.money),
        ];
        let place = self.books.place;
        if place > 0 {
            let race = self.campaign.entered_race.unwrap_or(0);
            // The Windows version's Arena reads the never-set circuit after the three races'
            // (0x4251DF).
            let circuit = self.records_circuit(race);
            let car = player.car.clamp(0, 5) as usize;
            let best = self.outcome.best_lap;
            let record = self.config.record(circuit, car).1.map(|part| part as i32);
            let books = self.books;
            rows.extend([
                plain(format!("{place}.")),
                money(books.prize),
                money(books.picked_up),
                money(books.prize + books.picked_up),
                plain(self.outcome.laps.to_string()),
                plain(clock(self.outcome.race_time)),
                plain(clock(best)),
                plain(clock(record)),
            ]);
            let texts = &self.assets.menu.texts;
            let kind = &texts.campaign.race_kinds[race.min(3)];
            let heading = if race == 3 {
                kind.clone()
            } else {
                [&texts.hall_of_fame.circuits[circuit][..], kind].concat()
            };
            let font = &self.graphics.small[0];
            font.draw(canvas, &heading, at(STATISTICS_LABEL_X, RACE_HEADING_Y));
            // 0x425956: the place is told once.
            self.books.place = 0;
        }
        let texts = &self.assets.menu.texts.campaign;
        let font = &self.graphics.small[0];
        for (row, value) in rows.iter().enumerate() {
            let y = STATISTICS_ROWS_Y[row];
            font.draw(
                canvas,
                &texts.statistics_rows[row],
                at(STATISTICS_LABEL_X, y),
            );
            font.draw(canvas, value, at(STATISTICS_VALUE_X, y));
        }
    }

    /// `sub_426080`: the panel's rows under the line put back.
    fn clear_press(&self, canvas: &mut Canvas) {
        let panel = &self.assets.menu.results.panel;
        let width = panel.width as usize;
        for row in PRESS_PANEL_ROWS {
            let from = &panel.pixels[row * width..(row + 1) * width];
            let to = at(PANEL.0, PANEL.1 + row);
            canvas.pixels_mut()[to..to + width].copy_from_slice(from);
        }
    }

    /// The line asking for a key in small A (`font` 0) or small B (1) on the screen.
    fn draw_press(&mut self, font: usize) {
        let text = self.assets.menu.texts.campaign.press_to_go_on.clone();
        let mut screen = std::mem::take(&mut self.screen);
        self.graphics.small[font].draw(&mut screen, &text, at(PRESS.0, PRESS.1));
        self.screen = screen;
    }

    /// `sub_4260D0`: the line in small B at the 30th call, in small A at the 60th, its strip
    /// shown each time; the count goes on from screen to screen.
    fn blink_press(&mut self) {
        self.press_blink += 1;
        let font = match self.press_blink {
            BLINK_B => 1,
            BLINK_A => 0,
            _ => return,
        };
        let mut screen = std::mem::take(&mut self.screen);
        self.clear_press(&mut screen);
        self.screen = screen;
        self.draw_press(font);
        let (x, y, w, h) = PRESS_STRIP;
        self.shown.copy_from(&self.screen, at(x, y), w, h);
        if self.press_blink == BLINK_A {
            self.press_blink = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lapped_player_on_a_shown_page_ends_its_points_but_not_a_skipped_one() {
        // 0x42945C breaks out of the page's loop; the skipped page's loop only steps over
        // the player (0x4295A7), so the third place's driver loses points only when the page
        // is shown.
        let shut_out = [false, true, false];
        let wrecked = [false; 3];
        assert_eq!(
            page_awards(wrecked, shut_out, false),
            [(0, Award::Marker), (0, Award::Points)]
        );
        assert_eq!(
            page_awards(wrecked, shut_out, true),
            [(0, Award::Points), (2, Award::Points)]
        );
    }

    #[test]
    fn a_wreck_in_the_first_three_gets_neither_marker_nor_points() {
        let awards = page_awards([false, true, false], [false; 3], false);
        assert_eq!(
            awards,
            [
                (0, Award::Marker),
                (0, Award::Points),
                (2, Award::Marker),
                (2, Award::Points)
            ]
        );
    }

    #[test]
    fn a_best_lap_beats_the_record_when_better_or_the_record_is_empty() {
        // The record is written into dr.cfg and shown as the best lap ever; a slower lap or a
        // race without a lap must leave it alone.
        assert!(beats_record([0, 26, 0], [0, 26, 96]));
        assert!(!beats_record([0, 26, 96], [0, 26, 96]), "an equal lap");
        assert!(!beats_record([0, 27, 0], [0, 26, 96]), "a slower lap");
        assert!(beats_record([1, 2, 3], [0, 0, 0]), "the first lap");
        assert!(!beats_record([0, 0, 0], [0, 26, 96]), "no lap at all");
        assert!(
            !beats_record([0, 0, 0], [0, 0, 0]),
            "no lap on an empty record"
        );
        // A hand-made dr.cfg's huge times wrap as the original's sums do, and do not stop the
        // game.
        let _ = beats_record([0, 26, 0], [u32::MAX, u32::MAX, u32::MAX]);
        let _ = beats_record([i32::MAX, i32::MAX, i32::MAX], [400_000, 0, 0]);
    }
}
