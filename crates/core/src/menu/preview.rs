//! The race's preview (spec M4 §3): `previewRaceScreen` (0x4321B0) up to the race,
//! `startRace` (0x415710): the grid's four drivers with their ranks and faces, the circuit's
//! picture, its laps and prize, wiped in over the sign-up (`sub_42C670`) with the music
//! falling; then the racers set up, the other drivers' weapons drawn. DreeRally
//! `ui/prevRaceScreen.c`; dRally `___33010h.c` (the same laps and draws in the DOS version).

use super::hall_of_fame::Wipe;
use super::{Menu, State};
use crate::campaign::Racer;
use crate::canvas::at;

/// The banner, the grid's frame and the circuit's picture.
const BANNER: (usize, usize) = (0, 426);
const GRID: (usize, usize) = (13, 100);
const SHAPE: (usize, usize) = (264, 100);
/// The grid's places: the rank's right edge, the name, the face.
const RANKS: [(usize, usize); 4] = [(51, 177), (163, 216), (51, 274), (163, 313)];
const NAMES: [(usize, usize); 4] = [(24, 201), (136, 240), (24, 298), (136, 337)];
const FACES: [(usize, usize); 4] = [(53, 129), (165, 168), (53, 226), (165, 265)];
/// The laps' line, and the prize's right edge, in the medium font.
const LAPS_AT: (usize, usize) = (269, 358);
const PRIZE_RIGHT: (usize, usize) = (619, 358);
/// The laps of the easy, medium and hard race (0x432630, 0x4326FA, 0x4327BF).
const LAPS: [i32; 3] = [4, 5, 6];
/// The preview's music starts at this order (`musicSetOrder(0x2800)`), at full volume.
const PREVIEW_ORDER: usize = 0x28;
const FULL_MASK: u32 = 0x1_0000 >> 8;
/// An opponent carries each weapon at one chance in five, with weapons on.
const WEAPON_CHANCE: i32 = 5;
const MINES: i32 = 8;
/// The preview held 2000 ms of 14 ms ticks, then 41 waits to black (k = 40 down to 0), the
/// music's volume `k` times 1638.4.
const HOLD_WAITS: u32 = 143;
const FADE_STEPS: i32 = 40;
const VOLUME_STEP: f64 = 1638.4;

impl Menu {
    /// The preview drawn into the second buffer over the background, then wiped in.
    pub(super) fn open_preview(&mut self) -> State {
        let race = self.campaign.entered_race.expect("the player is in a race");
        let entrants = self
            .campaign
            .sign_up
            .as_ref()
            .expect("a sign-up is on")
            .entrants[race];
        let circuit = self
            .campaign
            .sign_up
            .as_ref()
            .expect("a sign-up is on")
            .circuits[race];
        let menu = &self.assets.menu;
        let mut back = std::mem::take(&mut self.back);
        back.copy_all(&self.graphics.background);
        back.draw(&menu.preview_banner, at(BANNER.0, BANNER.1), true);
        back.draw(&menu.preview_grid, at(GRID.0, GRID.1), true);
        back.draw(&menu.track_shapes[circuit], at(SHAPE.0, SHAPE.1), true);
        let medium = &self.graphics.medium;
        let texts = &menu.texts.campaign;
        let laps = [&texts.laps[..], LAPS[race].to_string().as_bytes()].concat();
        medium.draw(&mut back, &laps, at(LAPS_AT.0, LAPS_AT.1));
        let prize = [&texts.prize[..], &texts.race_prices[race]].concat();
        let pen = PRIZE_RIGHT.0.saturating_sub(medium.width(&prize));
        medium.draw(&mut back, &prize, at(pen, PRIZE_RIGHT.1));
        for (place, &driver) in entrants.iter().enumerate() {
            let record = &self.campaign.drivers[driver];
            let rank = format!("{}.", record.rank).into_bytes();
            let (right, y) = RANKS[place];
            let pen = right.saturating_sub(medium.width(&rank));
            medium.draw(&mut back, &rank, at(pen, y));
            let name = record.name().to_ascii_uppercase();
            medium.draw(&mut back, &name, at(NAMES[place].0, NAMES[place].1));
            // A face past the pictures (only from an edited save) is left out.
            if let Some(face) = usize::try_from(record.face)
                .ok()
                .and_then(|f| menu.faces.get(f))
            {
                back.draw(face, at(FACES[place].0, FACES[place].1), false);
            }
        }
        self.back = back;
        State::Wipe {
            wipe: Wipe::Preview,
            step: 0,
        }
    }

    /// The wait after the wipe; then the preview's music at full volume, the preview shown,
    /// and the racers set up (0x432F46): each opponent's rocket, spikes and mines drawn at
    /// one chance in five, in that order, before the race.
    pub(super) fn preview_wait(&mut self) -> State {
        self.palette.after_wait();
        self.sound.set_music_order(PREVIEW_ORDER);
        self.sound.set_mask(FULL_MASK);
        self.shown = self.screen.clone();
        let race = self.campaign.entered_race.expect("the player is in a race");
        let entrants = self
            .campaign
            .sign_up
            .as_ref()
            .expect("a sign-up is on")
            .entrants[race];
        let campaign = &mut self.campaign;
        let weapons = campaign.use_weapons;
        let me = campaign.player_index;
        campaign.racers = entrants
            .iter()
            .map(|&driver| {
                let record = campaign.drivers[driver];
                if driver == me {
                    return Racer {
                        driver,
                        rocket: record.rocket,
                        spikes: record.spikes,
                        mines: record.mines,
                    };
                }
                let mut draw = |carried: i32| {
                    if campaign.rand.next() % WEAPON_CHANCE == 0 && weapons {
                        carried
                    } else {
                        0
                    }
                };
                Racer {
                    driver,
                    rocket: draw(1),
                    spikes: draw(1),
                    mines: draw(MINES),
                }
            })
            .collect();
        State::PreviewHold { waits: 0 }
    }

    /// `startRace` (0x415710) loads the race, then holds the preview two seconds
    /// (`waitTwoSeconds` 0x43CBB0, 2000 ms of the clock); `drawToBlackScreen` (0x404920)
    /// then sets the palette at 40 fortieths before its first wait.
    pub(super) fn preview_hold(&mut self, waits: u32) -> State {
        if waits + 1 < HOLD_WAITS {
            return State::PreviewHold { waits: waits + 1 };
        }
        self.saved_palette = self.palette.shown().clone();
        self.palette.darken(&self.saved_palette.clone(), FADE_STEPS);
        State::ToBlack { k: FADE_STEPS }
    }

    /// After a wait of `drawToBlackScreen`: the music at `k` fortieths, then the palette at
    /// one fortieth less before the next wait; after the last, the race (M4b) or, until it
    /// exists, the stand-in.
    pub(super) fn fading_out(&mut self, k: i32) -> State {
        self.sound
            .set_mask(((f64::from(k) * VOLUME_STEP) as u32) >> 8);
        if k == 0 {
            return self.start_race();
        }
        let from = self.saved_palette.clone();
        self.palette.darken(&from, k - 1);
        State::ToBlack { k: k - 1 }
    }

    /// The race on the sign-up's circuit, the player in their place on the grid; without its
    /// data (as in the tests) the stand-in race.
    /// What a money power-up is worth in race `race` (0x433361): $50 in the first race; in the
    /// others by the player's rank, more the higher the rank and the harder the race; $400
    /// when the player leads everyone on points (the Adversary's race).
    fn pickup_money(&self, race: usize) -> i32 {
        if self.campaign.player_leads() {
            return 400;
        }
        let rank = self.campaign.player().rank;
        let by_race = |second: i32, third: i32| match race {
            1 => Some(second),
            2 => Some(third),
            _ => None,
        };
        let mut money = if race == 0 { 50 } else { 0 };
        for (ranks, second, third) in [
            (1..6, 260, 500),
            (6..11, 200, 300),
            (11..16, 120, 150),
            (16..21, 60, 80),
        ] {
            if ranks.contains(&rank)
                && let Some(value) = by_race(second, third)
            {
                money = value;
            }
        }
        money
    }

    fn start_race(&mut self) -> State {
        let race = self.campaign.entered_race.expect("the player is in a race");
        let circuit = self
            .campaign
            .sign_up
            .as_ref()
            .expect("a sign-up is on")
            .circuits[race];
        let player = self
            .campaign
            .racers
            .iter()
            .position(|racer| racer.driver == self.campaign.player_index)
            .unwrap_or(0);
        let drivers = self
            .campaign
            .racers
            .iter()
            .map(|racer| {
                let record = &self.campaign.drivers[racer.driver];
                let colours = if racer.driver == self.campaign.player_index {
                    &self.assets.menu.copper
                } else {
                    &self.assets.menu.car_colours
                };
                crate::race::Driver {
                    colour: colours.0[record.colour.clamp(0, 255) as usize],
                    name: record.name().to_ascii_uppercase(),
                    car: record.car.clamp(0, 5) as usize,
                    level: if racer.driver == self.campaign.player_index {
                        3
                    } else {
                        self.config.difficulty().min(2) as usize
                    },
                    engine: record.engine,
                    tires: record.tires,
                    armour: record.armour,
                    damage: record.damage,
                    rocket: racer.rocket,
                    mines: racer.mines,
                    spikes: racer.spikes != 0,
                }
            })
            .collect();
        let laps = LAPS[race];
        let weapons = self.campaign.use_weapons;
        let lines = self.assets.menu.texts.campaign.abort_race.clone();
        let controls = std::array::from_fn(|control| self.config.key(control));
        let record = self.campaign.player();
        let setup = crate::race::Setup {
            circuit,
            race,
            laps,
            player,
            weapons,
            pause_lines: lines,
            race_over_lines: self.assets.menu.texts.campaign.race_over.clone(),
            paused_lines: self.assets.menu.texts.campaign.game_paused.clone(),
            help: self.assets.menu.texts.help.clone(),
            pads: std::array::from_fn(|control| self.config.pad(control)),
            still: self.campaign.still_opponents,
            controls,
            pickup_money: self.pickup_money(race),
            lap_record: self
                .config
                .record(circuit, record.car.clamp(0, 5) as usize)
                .1
                .map(|part| part as i32),
            session: self.campaign.race_session,
        };
        let race =
            crate::race::Race::new(&self.assets.race, setup, drivers, &mut self.campaign.rand);
        match race {
            Ok(mut race) => {
                let volumes = (self.config.music_volume(), self.config.effects_volume());
                race.begin(
                    &mut self.sound,
                    volumes,
                    &mut self.keys,
                    &mut self.campaign.rand,
                );
                self.race = Some(race);
                State::Race { ticks: 0 }
            }
            Err(_) => {
                self.after_race();
                self.race_stand_in()
            }
        }
    }

    /// A tick of the race; Escape leaves it for the stand-in until the pause menu (M4b).
    pub(super) fn race_tick(&mut self, ticks: u32) -> State {
        let Some(race) = self.race.as_mut() else {
            self.after_race();
            return self.race_stand_in();
        };
        let outcome = race.tick(&mut self.sound, &mut self.keys, &mut self.campaign.rand);
        race.present(self.shown.pixels_mut());
        let palette = race.shown().clone();
        self.palette.show(&palette, 100);
        if outcome != crate::race::Outcome::Racing {
            self.campaign.race_session = race.session();
            // 0x4334F7: the books settled, then the news in the panel (0x434512).
            self.outcome = race.outcome();
            self.race = None;
            self.books = self.campaign.settle(&self.outcome);
            let headlines = &self.assets.menu.texts.campaign.headlines;
            self.panel.tell_headline(&mut self.campaign.rand, headlines);
            self.after_race();
            // 0x434670: every entry black before the results fade in.
            self.palette.fade(0);
            return self.open_results();
        }
        State::Race { ticks: ticks + 1 }
    }

    /// The race over (abandoned, or its data not loading): the menus' music and sounds back as
    /// the original brings them back after a race (0x434617), at the full volume the race's
    /// fades took away and the results would give back (M5), and the menus' background under
    /// the stand-in's shop with nothing of the preview or the race left on it.
    fn after_race(&mut self) {
        self.sound.stop();
        self.sound.set_mask(FULL_MASK);
        self.sound.load_effects(&self.assets.menu.effects);
        self.sound
            .play_music(&self.assets.menu_music, 0, self.config.music_volume());
        self.screen.copy_all(&self.graphics.background);
        self.shown = self.screen.clone();
    }
}
