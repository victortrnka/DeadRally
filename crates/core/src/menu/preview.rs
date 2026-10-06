//! The race's preview (spec M4 §3): `previewRaceScreen` (0x4321B0) up to the race,
//! `startRace` (0x415710): the grid's four drivers with their ranks and faces, the circuit's
//! picture, its laps and prize, wiped in over the sign-up (`sub_42C670`) with the music
//! falling; then the racers set up, the other drivers' weapons drawn. DreeRally
//! `ui/prevRaceScreen.c`; dRally `___33010h.c` (the same laps and draws in the DOS version).

use super::hall_of_fame::Wipe;
use super::{Menu, State};
use crate::campaign::{PLAYER, Racer};
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
            let face = &menu.faces[record.face as usize];
            back.draw(face, at(FACES[place].0, FACES[place].1), false);
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
        campaign.racers = entrants
            .iter()
            .map(|&driver| {
                let record = campaign.drivers[driver];
                if driver == PLAYER {
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
        self.race_stand_in()
    }
}
