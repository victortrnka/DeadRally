//! The race's preview (spec M4 §3): `previewRaceScreen` (0x4321B0) up to the race,
//! `startRace` (0x415710): the grid's four drivers with their ranks and faces, the circuit's
//! picture, its laps and prize, wiped in over the sign-up (`sub_42C670`) with the music
//! falling; then the racers set up, the other drivers' weapons drawn. DreeRally
//! `ui/prevRaceScreen.c`; dRally `___33010h.c` (the same laps and draws in the DOS version).
//! The race in the Arena (spec M6) comes through here too, with two cars: the Adversary and
//! the player.

use super::hall_of_fame::Wipe;
use super::{Menu, State};
use crate::campaign::ARENA;
use crate::canvas::at;

/// The banner, the grid's frame and the circuit's picture.
const BANNER: (usize, usize) = (0, 426);
const GRID: (usize, usize) = (13, 100);
const SHAPE: (usize, usize) = (264, 100);
/// The grid's places: the rank's right edge, the name, the face.
const RANKS: [(usize, usize); 4] = [(51, 177), (163, 216), (51, 274), (163, 313)];
const NAMES: [(usize, usize); 4] = [(24, 201), (136, 240), (24, 298), (136, 337)];
const FACES: [(usize, usize); 4] = [(53, 129), (165, 168), (53, 226), (165, 265)];
/// A place without a car is hatched in colour 0xC4, every other pixel, 100 across and 91
/// down: the third's from (19, 222), the fourth's from (131, 261) (0x4323BC, 0x43235E).
const HATCH: u8 = 0xC4;
const HATCHES: [(usize, usize); 2] = [(19, 222), (131, 261)];
const HATCH_SIZE: (usize, usize) = (100, 91);
/// The laps' line, and the prize's right edge, in the medium font.
const LAPS_AT: (usize, usize) = (269, 358);
const PRIZE_RIGHT: (usize, usize) = (619, 358);
/// The laps of the easy, medium and hard race (0x432630, 0x4326FA, 0x4327BF).
const LAPS: [i32; 3] = [4, 5, 6];
/// The Arena (0x4327CE): its picture, the last of the circuits' (`TSHAPE19`), its track
/// (`TR0`, never turned round), its laps; and the circuit whose lap records it reads and
/// writes, the byte after the three races' circuits (0x46126F), which nothing ever sets.
const ARENA_SHAPE: usize = 18;
const ARENA_TRACK: usize = 0;
const ARENA_LAPS: i32 = 9;
pub(super) const ARENA_RECORDS: usize = 0;
/// The Arena's two cars (`previewRaceScreen(2)` from 0x4354E4).
const ARENA_CARS: usize = 2;
/// The Adversary's car, and the colour of the ramps of the places it leaves empty or takes
/// from other drivers, `CARCOL.PAL`'s entry 10 (0x4332EE).
const ADVERSARY_CAR: usize = 6;
const SPARE_COLOUR: usize = 10;
/// The places past which the Adversary's set-up gives the ramps the spare colour.
const SPARE_FROM: usize = 2;
/// The preview's music starts at this order (`musicSetOrder(0x2800)`), at full volume.
const PREVIEW_ORDER: usize = 0x28;
const FULL_MASK: u32 = 0x1_0000 >> 8;
/// The preview held 2000 ms of 14 ms ticks, then 41 waits to black (k = 40 down to 0), the
/// music's volume `k` times 1638.4.
const HOLD_WAITS: u32 = 143;
const FADE_STEPS: i32 = 40;
const VOLUME_STEP: f64 = 1638.4;

impl Menu {
    /// Enter on the Adversary's screen (0x4354C1): the race in the Arena (race 3), the
    /// player driving the second car (0x45FC20 is 1), the race's money and prize cleared
    /// (0x456BE0, 0x456BDC), then `previewRaceScreen(2)`: the preview, the race and what
    /// follows it.
    pub(super) fn start_arena(&mut self) -> State {
        self.campaign.entered_race = Some(ARENA);
        self.books.picked_up = 0;
        self.books.prize = 0;
        self.open_preview()
    }

    /// The grid of the race the player is in: the sign-up's four, or in the Arena the
    /// player's record in both places (0x43288D).
    fn grid(&self) -> Vec<usize> {
        let race = self.campaign.entered_race.expect("the player is in a race");
        if race == ARENA {
            return vec![self.campaign.player_index; ARENA_CARS];
        }
        self.campaign
            .sign_up
            .as_ref()
            .expect("a sign-up is on")
            .entrants[race]
            .to_vec()
    }

    /// The preview drawn into the second buffer over the background, then wiped in. In the
    /// Arena the first place is the Adversary's, its name and face without a rank (0x432AF9),
    /// and the places without a car are hatched.
    pub(super) fn open_preview(&mut self) -> State {
        let race = self.campaign.entered_race.expect("the player is in a race");
        let entrants = self.grid();
        let menu = &self.assets.menu;
        let (shape, laps, price) = if race == ARENA {
            (ARENA_SHAPE, ARENA_LAPS, &menu.texts.campaign.arena_prize)
        } else {
            let sign_up = self.campaign.sign_up.as_ref().expect("a sign-up is on");
            (
                sign_up.circuits[race],
                LAPS[race],
                &menu.texts.campaign.race_prices[race],
            )
        };
        let mut back = std::mem::take(&mut self.back);
        back.copy_all(&self.graphics.background);
        back.draw(&menu.preview_banner, at(BANNER.0, BANNER.1), true);
        back.draw(&menu.preview_grid, at(GRID.0, GRID.1), true);
        for (place, &(x, y)) in HATCHES.iter().enumerate().rev() {
            if entrants.len() <= place + 2 {
                hatch(back.pixels_mut(), (x, y));
            }
        }
        back.draw(&menu.track_shapes[shape], at(SHAPE.0, SHAPE.1), true);
        let medium = &self.graphics.medium;
        let texts = &menu.texts.campaign;
        let laps = [&texts.laps[..], laps.to_string().as_bytes()].concat();
        medium.draw(&mut back, &laps, at(LAPS_AT.0, LAPS_AT.1));
        let prize = [&texts.prize[..], price].concat();
        let pen = PRIZE_RIGHT.0.saturating_sub(medium.width(&prize));
        medium.draw(&mut back, &prize, at(pen, PRIZE_RIGHT.1));
        for (place, &driver) in entrants.iter().enumerate() {
            if race == ARENA && place == 0 {
                let name = texts.adversary_preview.to_ascii_uppercase();
                medium.draw(&mut back, &name, at(NAMES[0].0, NAMES[0].1));
                back.draw(&menu.adversary_face, at(FACES[0].0, FACES[0].1), false);
                continue;
            }
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
    /// one chance in five, in that order, before the race; the first made the Adversary when
    /// the player leads.
    pub(super) fn preview_wait(&mut self) -> State {
        self.palette.after_wait();
        self.sound.set_music_order(PREVIEW_ORDER);
        self.sound.set_mask(FULL_MASK);
        self.shown = self.screen.clone();
        let entrants = self.grid();
        self.campaign.set_up_racers(&entrants);
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

    /// What a money power-up is worth in race `race` (0x433361): $50 in the first race; in the
    /// others by the player's rank, more the higher the rank and the harder the race; $400
    /// when the player is ahead of the first drivers on points as the Adversary's set-up
    /// counts them (the Adversary's race).
    fn pickup_money(&self, race: usize) -> i32 {
        if self.campaign.leads_first(self.campaign.racers.len()) {
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

    /// The race on the sign-up's circuit (or in the Arena), the player in their place on the
    /// grid; without its data (as in the tests) the stand-in race.
    fn start_race(&mut self) -> State {
        let race = self.campaign.entered_race.expect("the player is in a race");
        // The track (0x45EA50) and its turning round (0x4A7AA8): the second half's circuits
        // run their tracks the other way round (0x432532).
        let (track, reversed, records, laps) = if race == ARENA {
            (ARENA_TRACK, false, ARENA_RECORDS, ARENA_LAPS)
        } else {
            let circuit = self
                .campaign
                .sign_up
                .as_ref()
                .expect("a sign-up is on")
                .circuits[race];
            (circuit % 9 + 1, circuit > 8, circuit, LAPS[race])
        };
        let player = self.campaign.player_racer();
        let me = self.campaign.player_index;
        let menu = &self.assets.menu;
        let spare = menu.car_colours.0[SPARE_COLOUR];
        let adversary = self.campaign.racers.first().is_some_and(|r| r.adversary);
        let difficulty = self.config.difficulty().min(2) as usize;
        let drivers = self
            .campaign
            .racers
            .iter()
            .enumerate()
            .map(|(place, racer)| {
                let record = &self.campaign.drivers[racer.driver];
                let colours = if racer.driver == me {
                    &menu.copper
                } else {
                    &menu.car_colours
                };
                let colour = if adversary && place >= SPARE_FROM {
                    spare
                } else {
                    colours.0[record.colour.clamp(0, 255) as usize]
                };
                let driver = crate::race::Driver {
                    colour,
                    name: record.name().to_ascii_uppercase(),
                    car: record.car.clamp(0, 5) as usize,
                    level: if racer.driver == me { 3 } else { difficulty },
                    engine: record.engine,
                    tires: record.tires,
                    armour: record.armour,
                    damage: record.damage,
                    rocket: racer.rocket,
                    mines: racer.mines,
                    spikes: racer.spikes != 0,
                };
                if racer.adversary {
                    // 0x433285: the first racer's name, level, damage and car the Adversary's.
                    crate::race::Driver {
                        name: menu.texts.campaign.adversary.clone(),
                        car: ADVERSARY_CAR,
                        level: difficulty,
                        damage: 0,
                        ..driver
                    }
                } else {
                    driver
                }
            })
            .collect();
        let weapons = self.campaign.use_weapons;
        let lines = menu.texts.campaign.abort_race.clone();
        let controls = std::array::from_fn(|control| self.config.key(control));
        let record = self.campaign.player();
        let setup = crate::race::Setup {
            track,
            reversed,
            spare_ramps: adversary.then_some(spare),
            race,
            laps,
            player,
            weapons,
            pause_lines: lines,
            race_over_lines: menu.texts.campaign.race_over.clone(),
            paused_lines: menu.texts.campaign.game_paused.clone(),
            help: menu.texts.help.clone(),
            pads: std::array::from_fn(|control| self.config.pad(control)),
            still: self.campaign.still_opponents,
            controls,
            pickup_money: self.pickup_money(race),
            lap_record: self
                .config
                .record(records, record.car.clamp(0, 5) as usize)
                .1
                .map(|part| part as i32),
            session: self.campaign.race_session,
            flame_phase: self.campaign.flame_phase,
        };
        let race =
            crate::race::Race::new(&self.assets.race, setup, drivers, &mut self.campaign.rand);
        match race {
            Ok(mut race) => {
                let volumes = (self.config.music_volume(), self.config.effects_volume());
                race.begin(&mut self.sound, volumes, &mut self.campaign.rand);
                self.race = Some(race);
                State::Race { ticks: 0 }
            }
            Err(_) => {
                self.after_race();
                self.race_stand_in()
            }
        }
    }

    /// A tick of the race; once it is over, what `previewRaceScreen` does after it: the books
    /// (0x4334F7), the news (0x434512), the menus' sound back but after a won race the player
    /// led (0x4345BA), the palette black (0x434670); then for a player who leads the next
    /// races filled at once (0x434737) and, the race won, the end (0x4347CB); otherwise the
    /// results.
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
            self.campaign.flame_phase = race.flame_phase();
            self.campaign.race_session = race.session();
            self.outcome = race.outcome();
            self.race = None;
            self.books = self.campaign.settle(&self.outcome);
            let headlines = &self.assets.menu.texts.campaign.headlines;
            self.panel.tell_headline(&mut self.campaign.rand, headlines);
            let leading = self.campaign.player_leads();
            let won = self.books.place == 1;
            if !(leading && won) {
                self.after_race();
            }
            self.palette.fade(0);
            if leading {
                let order = self.assets.menu.texts.hall_of_fame.circuit_order.clone();
                self.campaign.fill_races_after_leading(&order);
                if won {
                    return self.the_end();
                }
            }
            self.results_after_race = true;
            return self.open_results();
        }
        State::Race { ticks: ticks + 1 }
    }

    /// The race over (abandoned, or its data not loading): the menus' music and sounds back as
    /// the original brings them back after a race (0x434617), at the full volume the race's
    /// fades took away and the results would give back (M5), and the menus' background under
    /// the stand-in's shop with nothing of the preview or the race left on it.
    pub(super) fn after_race(&mut self) {
        self.sound.stop();
        self.sound.set_mask(FULL_MASK);
        self.sound.load_effects(&self.assets.menu.effects);
        self.sound
            .play_music(&self.assets.menu_music, 0, self.config.music_volume());
        self.screen.copy_all(&self.graphics.background);
        self.shown = self.screen.clone();
    }
}

/// A place without a car hatched (0x43235E): every other pixel of each row in colour 0xC4,
/// starting a pixel further right on odd rows.
fn hatch(pixels: &mut [u8], (x, y): (usize, usize)) {
    let (width, height) = HATCH_SIZE;
    for row in 0..height {
        let start = at(x, y + row) + row % 2;
        for column in (0..width - row % 2).step_by(2) {
            pixels[start + column] = HATCH;
        }
    }
}
