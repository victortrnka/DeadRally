//! The race's power-ups (spec M4c): 20 places on the track (from `INF.BIN`) where the original
//! paints a power-up into the track's picture, keeping the 16x16 pixels under it to put back
//! when it goes. At the start (`generateBigPowerUps` 0x409460) one time in four a big one
//! lies at the 13th or 14th place; from 350 ticks on (`sub_410220`, once a pass of the race
//! loop) the first 12 places get power-ups at random, at most four at once, which blink out
//! after 2000 ticks. The pictures are `ENGINE.BPA`'s `OBSTACLE.BPK`.

use deadrally_gamedata::image::Image;

use crate::campaign::Rand;

use super::buffer::{Buffer, LEFT, STRIDE};
use super::driving::Car;
use super::raster::ftol;

/// The places, the ones power-ups come and go at, and a power-up's picture: 16x16, drawn
/// centred on its place.
const PLACES: usize = 20;
const CHANGING: usize = 12;
const SIDE: usize = 16;
/// The ticks before the first power-up, and before the next once one has gone.
const FIRST: i32 = 350;
const AFTER_ONE_WENT: i32 = 280;
/// The most power-ups out at once.
const MOST: usize = 4;
/// The age a power-up goes at, and how often it blinks before.
const LIFE: i32 = 2000;
const BLINKS: i32 = 24;

/// A place for a power-up (0x501BA0, 0x120 bytes a place).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Place {
    at: [i32; 2],
    /// The power-up lying there (1 to 8), 0 for none.
    kind: i32,
    /// The ticks before one may come here, and how long the one here has lain.
    wait: i32,
    age: i32,
    /// The track's pixels under the power-up.
    under: [u8; SIDE * SIDE],
    /// The ticks the last pick-up's note still floats up from here (0x501BB4), its kind and
    /// the repair it gave in percent.
    shown: i32,
    shown_kind: i32,
    amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PowerUps {
    places: Vec<Place>,
    /// The ticks before the next power-up may come (0x456AC4).
    wait: i32,
    /// The power-ups' pictures one after another.
    pictures: Vec<u8>,
}

impl PowerUps {
    /// The start's power-ups (`generateBigPowerUps`) on `image`, the track's picture: a wait
    /// of 100 to 149 ticks for each place, then one time in four a big power-up at the 13th
    /// or 14th place, if the track has it.
    pub(crate) fn new(
        image: &mut Image,
        spots: &[[i32; 2]],
        pictures: Vec<u8>,
        rand: &mut Rand,
    ) -> PowerUps {
        let places = (0..PLACES)
            .map(|place| Place {
                at: spots.get(place).copied().unwrap_or([0, 0]),
                kind: 0,
                wait: rand.next() % 50 + 100,
                age: 0,
                under: [0; SIDE * SIDE],
                shown: 0,
                shown_kind: 0,
                amount: 0,
            })
            .collect();
        let mut power_ups = PowerUps {
            places,
            wait: FIRST,
            pictures,
        };
        if rand.next() % 4 == 0 {
            let place = (rand.next() % 2 + 12) as usize;
            if power_ups.places[place].at[0] > 0 {
                let kind = rand.next() % 2 + 7;
                power_ups.lay(image, place, kind);
            }
        }
        power_ups
    }

    /// A power-up of `kind` laid at `place`, the pixels under it kept.
    fn lay(&mut self, image: &mut Image, place: usize, kind: i32) {
        let spot = &mut self.places[place];
        spot.kind = kind;
        let width = image.width as i32;
        let [x, y] = spot.at;
        for row in 0..SIDE as i32 {
            for column in 0..SIDE as i32 {
                let at = (y + row - 8) * width + x + column - 8;
                spot.under[(row * SIDE as i32 + column) as usize] = usize::try_from(at)
                    .ok()
                    .and_then(|at| image.pixels.get(at))
                    .copied()
                    .unwrap_or(0);
            }
        }
        self.paint(image, place);
    }

    /// The picture of the power-up at `place` painted over the track, its 0 bytes left out.
    fn paint(&self, image: &mut Image, place: usize) {
        let spot = &self.places[place];
        let width = image.width as i32;
        let start = usize::try_from(spot.kind - 1).unwrap_or(0) * SIDE * SIDE;
        let [x, y] = spot.at;
        for row in 0..SIDE as i32 {
            for column in 0..SIDE as i32 {
                let pixel = self
                    .pictures
                    .get(start + (row * SIDE as i32 + column) as usize)
                    .copied()
                    .unwrap_or(0);
                let at = (y + row - 8) * width + x + column - 8;
                if pixel != 0
                    && let Some(slot) = usize::try_from(at)
                        .ok()
                        .and_then(|at| image.pixels.get_mut(at))
                {
                    *slot = pixel;
                }
            }
        }
    }

    /// The track's pixels put back where the power-up at `place` lay.
    fn clear(&self, image: &mut Image, place: usize) {
        let spot = &self.places[place];
        let width = image.width as i32;
        let [x, y] = spot.at;
        for row in 0..SIDE as i32 {
            for column in 0..SIDE as i32 {
                let at = (y + row - 8) * width + x + column - 8;
                if let Some(slot) = usize::try_from(at)
                    .ok()
                    .and_then(|at| image.pixels.get_mut(at))
                {
                    *slot = spot.under[(row * SIDE as i32 + column) as usize];
                }
            }
        }
    }

    /// `sub_410220`, once a pass of the race loop, `between` the ticks the HUD's last two
    /// frames were apart: the power-ups age; once the wait is over, the first 12 places are
    /// tried in a random order and an empty one whose own wait is over gets a power-up (by
    /// `rand()`, the weapons' kinds only in a race with `weapons`) while fewer than four lie
    /// out; then the old ones blink and go.
    pub(crate) fn step(&mut self, image: &mut Image, between: i32, weapons: bool, rand: &mut Rand) {
        self.wait = if self.wait > 0 {
            self.wait - between
        } else {
            0
        };
        for spot in &mut self.places[..CHANGING] {
            if spot.age < LIFE && spot.kind != 0 {
                spot.age += between;
            }
        }
        if self.wait == 0 {
            let mut out = self.places[..CHANGING]
                .iter()
                .filter(|spot| spot.kind > 0)
                .count();
            let mut tried = [false; CHANGING];
            for _ in 0..CHANGING {
                let place = loop {
                    let place = (rand.next() % CHANGING as i32) as usize;
                    if !tried[place] {
                        break place;
                    }
                };
                tried[place] = true;
                let spot = &mut self.places[place];
                spot.wait = if spot.wait > 0 {
                    spot.wait - between
                } else {
                    0
                };
                if spot.kind == 0 && spot.at[0] > 0 && spot.wait == 0 && out < MOST {
                    let kind = kind(rand.next() % 100, weapons);
                    out += 1;
                    self.places[place].age = 0;
                    self.lay(image, place, kind);
                }
            }
        }
        for place in 0..CHANGING {
            let age = self.places[place].age;
            // Shown in the tens of ticks before 2000, 1980 and so on down to 1540, hidden in
            // those before 1990, 1970 down to 1530.
            let within = |last: i32| {
                (0..BLINKS).any(|k| {
                    let end = last - 20 * k;
                    age > end - 10 && age < end
                })
            };
            if within(LIFE) {
                self.paint(image, place);
            }
            if within(LIFE - 10) {
                self.clear(image, place);
            }
            if age >= LIFE {
                self.clear(image, place);
                let spot = &mut self.places[place];
                spot.age = 0;
                spot.kind = 0;
                self.wait = AFTER_ONE_WENT;
                self.places[place].wait = rand.next() % 200 + 300;
            }
        }
    }
}

/// The places a car can pick power-ups up at, and those whose notes float.
const PICKED_AT: usize = 16;
const NOTED_AT: usize = 15;
/// How long a pick-up's note floats, and the wait for the next power-up after one.
const NOTE_TICKS: i32 = 140;
const AFTER_ONE_PICKED: i32 = 240;
/// The bars' and the damage's full measure.
const FULL_BAR: i32 = 0x1_9000;
/// The ticks the effect of kind 4 lasts.
const EFFECT_TICKS: i32 = 560;

/// What a pick-up asks the race to play: the player's own, or another car's as loud as it is
/// near the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickupSound {
    /// Effect 18 on channel 4 (or 10 for another car) at this volume.
    Picked { channel: usize, volume: u32 },
    /// The effect power-up's call (effect 6 on channel 2).
    Effect,
}

/// What a car took (`sub_410B90`): the sounds, and whether the player took the bonus.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Taken {
    pub(crate) sounds: Vec<PickupSound>,
    pub(crate) bonus: bool,
}

impl PowerUps {
    /// `sub_410B90` for one car: every power-up under its sprite (its middle within 16 pixels
    /// and one of the four pixels round the power-up's middle drawn in the car's sprite) is
    /// taken: its pixels put back, its kind's gift given, its note started, the places' waits
    /// set; `player` is the player's car's place for the sound's distance, or `None` for the
    /// player's own car.
    pub(crate) fn pick_up(
        &mut self,
        image: &mut Image,
        car: &mut Car,
        sprites: &[u8],
        player: Option<(f32, f32)>,
        rand: &mut Rand,
    ) -> Taken {
        let mut taken = Taken::default();
        for place in 0..PICKED_AT {
            let [px, py] = self.places[place].at;
            let dx = ftol(f64::from(car.x)) - px;
            let dy = ftol(f64::from(car.y)) - py;
            if dx.abs() >= 17 || dy.abs() >= 17 {
                continue;
            }
            let pixel = |offset: i32| {
                usize::try_from(car.sprite as i32 + dy * 40 + dx + offset)
                    .ok()
                    .and_then(|at| sprites.get(at))
                    .is_some_and(|&p| p > 3)
            };
            let touched = [0x2E2, 0x2E6, 0x382, 0x386].into_iter().any(pixel);
            let kind = self.places[place].kind;
            if !touched || kind <= 0 {
                continue;
            }
            match player {
                None => taken.sounds.push(PickupSound::Picked {
                    channel: 4,
                    volume: 0x9000,
                }),
                Some((x, y)) => {
                    let dx = ftol(f64::from(car.x) - f64::from(x));
                    let dy = ftol(f64::from(car.y) - f64::from(y));
                    let distance = ftol(f64::from(dx * dx + dy * dy).sqrt());
                    let volume = 0x9000 - 75 * distance;
                    if volume > 0x1000 {
                        taken.sounds.push(PickupSound::Picked {
                            channel: 10,
                            volume: volume as u32,
                        });
                    }
                }
            }
            self.clear(image, place);
            let h = &mut car.handling;
            match kind {
                1 => h.weapons_bar = (h.weapons_bar + 0x7800).min(FULL_BAR),
                2 => h.turbo = (h.turbo + 0x3C00).min(FULL_BAR),
                3 => h.money += 1,
                4 => {
                    car.effect = EFFECT_TICKS;
                    if player.is_none() {
                        taken.sounds.push(PickupSound::Effect);
                    }
                }
                5 => {
                    let amount = rand.next() % 4 + 2;
                    self.places[place].amount = amount;
                    h.damage = (h.damage + (amount << 10)).min(FULL_BAR);
                }
                6 => taken.bonus |= player.is_none(),
                7 => h.money += 10,
                8 => {
                    self.places[place].amount = 20;
                    h.damage = (h.damage + 0x5000).min(FULL_BAR);
                }
                _ => {}
            }
            let spot = &mut self.places[place];
            spot.shown = NOTE_TICKS;
            spot.shown_kind = kind;
            spot.kind = 0;
            self.wait = AFTER_ONE_PICKED;
            let wait = rand.next() % 150 + 200;
            let spot = &mut self.places[place];
            spot.age = 0;
            spot.wait = wait;
        }
        taken
    }

    /// `sub_410050`: the notes of what was picked up float up from their places, `$` and the
    /// money for money (`money`, what one is worth in this race), the repair in percent.
    pub(crate) fn draw_notes(
        &mut self,
        buffer: &mut Buffer,
        font: &[u8],
        money: i32,
        (camera_x, camera_y): (i32, i32),
        left: i32,
        between: i32,
    ) {
        for spot in &mut self.places[..NOTED_AT] {
            if spot.shown <= 0 || money <= 0 {
                continue;
            }
            let [x, y] = spot.at;
            let column = x - camera_x + left;
            let row = (spot.shown >> 3) - camera_y + y - 10;
            if (-18..320).contains(&column) && row >= 0 && row + 6 < 200 {
                let note = match spot.shown_kind {
                    3 => Some(format!("${money}")),
                    5 | 8 => Some(format!("{}%", spot.amount)),
                    7 => Some(format!("${}", money * 10)),
                    _ => None,
                };
                if let Some(note) = note {
                    let at = i64::from(row) * STRIDE as i64 + i64::from(column) + LEFT as i64;
                    for (index, c) in note.bytes().enumerate() {
                        let start = 36 * usize::from(c.saturating_sub(32));
                        let glyph = font.get(start..start + 36).unwrap_or(&[]);
                        buffer.draw(glyph, 6, 6, at + 6 * index as i64);
                    }
                }
            }
            spot.shown -= between;
        }
    }
}

/// A new power-up's kind from a draw of 0 to 99: with weapons 30 % kind 1, 35 % kind 2,
/// 15 % kind 3, 5 % kind 4 and 15 % kind 5; without, no kind 1 (45 %, 35 %, 10 %, 10 %).
fn kind(draw: i32, weapons: bool) -> i32 {
    let bounds: [(i32, i32); 5] = if weapons {
        [(30, 1), (65, 2), (80, 3), (85, 4), (100, 5)]
    } else {
        [(0, 1), (45, 2), (80, 3), (90, 4), (100, 5)]
    };
    bounds
        .iter()
        .find(|&&(below, _)| draw < below)
        .map_or(0, |&(_, kind)| kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> Image {
        Image {
            width: 64,
            height: 64,
            pixels: vec![0; 64 * 64],
        }
    }

    fn pictures() -> Vec<u8> {
        (0..8u8)
            .flat_map(|kind| vec![kind + 1; SIDE * SIDE])
            .collect()
    }

    /// The start draws `rand()` 21 times, and a power-up's place and kind when the 21st comes
    /// out a multiple of 4: later races depend on the numbers left.
    #[test]
    fn the_start_draws_rand_as_the_original_does() {
        let mut spots = [[0; 2]; 16];
        spots[12] = [20, 20];
        spots[13] = [40, 40];
        for seed in 0..64 {
            let mut rand = Rand::new(seed);
            let mut image = track();
            let power_ups = PowerUps::new(&mut image, &spots, pictures(), &mut rand);
            let mut expected = Rand::new(seed);
            for place in 0..20 {
                assert_eq!(power_ups.places[place].wait, expected.next() % 50 + 100);
            }
            if expected.next() % 4 == 0 {
                let place = (expected.next() % 2 + 12) as usize;
                let kind = expected.next() % 2 + 7;
                assert_eq!(power_ups.places[place].kind, kind);
                // Painted centred on the place, its picture the kind's.
                let [x, y] = spots[place];
                assert_eq!(image.pixels[(y * 64 + x) as usize], kind as u8);
            } else {
                assert!(power_ups.places.iter().all(|place| place.kind == 0));
                assert!(image.pixels.iter().all(|&p| p == 0));
            }
            assert_eq!(rand, expected, "seed {seed}");
        }
    }

    /// The weapons' power-up (kind 1) comes only in a race with weapons, and the kinds keep
    /// the original's odds.
    #[test]
    fn the_kinds_keep_the_originals_odds() {
        let count =
            |weapons: bool, kind_: i32| (0..100).filter(|&d| kind(d, weapons) == kind_).count();
        assert_eq!(
            (1..=5).map(|k| count(true, k)).collect::<Vec<_>>(),
            [30, 35, 15, 5, 15]
        );
        assert_eq!(
            (1..=5).map(|k| count(false, k)).collect::<Vec<_>>(),
            [0, 45, 35, 10, 10]
        );
    }

    /// Nothing comes before 350 ticks; then every pass tries all 12 places in a random order,
    /// which draws `rand()` until each has come up, and lays at most four power-ups.
    #[test]
    fn power_ups_come_after_350_ticks_four_at_most() {
        let spots: Vec<[i32; 2]> = (0..16).map(|i| [10 + 3 * i, 30]).collect();
        let mut rand = Rand::new(7);
        let mut image = track();
        let mut power_ups = PowerUps::new(&mut image, &spots, pictures(), &mut rand);
        for place in &mut power_ups.places {
            place.wait = 0;
        }
        power_ups.places[12].kind = 0;
        let before = rand.clone();
        power_ups.step(&mut image, 349, true, &mut rand);
        assert_eq!(rand, before, "no draws while waiting");
        power_ups.step(&mut image, 1, true, &mut rand);
        let out = power_ups.places[..CHANGING]
            .iter()
            .filter(|p| p.kind > 0)
            .count();
        assert_eq!(out, MOST);
        assert!(power_ups.places[..CHANGING].iter().all(|p| p.kind <= 5));
    }

    /// A car over a power-up takes it: the repair power-up mends 20 %, its pixels go back,
    /// the place waits 200 to 349 ticks and every place 240; one of 12 points off its middle
    /// is not over it.
    #[test]
    fn a_car_over_a_power_up_takes_it() {
        use crate::race::driving::{FRAME, FRAMES, Handling};
        use deadrally_gamedata::handling::Guns;
        let mut spots = [[0; 2]; 16];
        spots[0] = [30, 30];
        let mut rand = Rand::new(5);
        let mut image = track();
        let mut power_ups = PowerUps::new(&mut image, &spots, pictures(), &mut rand);
        power_ups.lay(&mut image, 0, 8);
        let handling = Handling {
            car: 0,
            engine: 2.5,
            engine_backup: 2.5,
            tires: 0.5,
            size: 9.0,
            steering: 2.5,
            damage: 0x8000,
            armour: 300,
            rocket: 0,
            weapons_bar: FULL_BAR,
            turbo: FULL_BAR,
            rocket_used: false,
            money: 0,
            weapons: true,
            guns: Guns::default(),
        };
        let sprites = vec![5u8; FRAMES * FRAME];
        let mut far = Car::new((48.0, 30.0, 72), 0, handling.clone(), 0);
        let taken = power_ups.pick_up(&mut image, &mut far, &sprites, None, &mut rand);
        assert!(taken.sounds.is_empty() && power_ups.places[0].kind == 8);
        let mut car = Car::new((40.0, 30.0, 72), 0, handling, 0);
        let taken = power_ups.pick_up(&mut image, &mut car, &sprites, None, &mut rand);
        assert_eq!(
            taken.sounds,
            [PickupSound::Picked {
                channel: 4,
                volume: 0x9000
            }]
        );
        assert_eq!(car.handling.damage, 0x8000 + 0x5000);
        assert_eq!(power_ups.places[0].kind, 0);
        assert_eq!(image.pixels[30 * 64 + 30], 0);
        assert_eq!(power_ups.wait, AFTER_ONE_PICKED);
        assert!((200..350).contains(&power_ups.places[0].wait));
        assert_eq!(
            (power_ups.places[0].shown, power_ups.places[0].amount),
            (140, 20)
        );
    }

    /// A power-up blinks out in its last ticks and goes at 2000, its pixels put back, and the
    /// next waits 280 ticks.
    #[test]
    fn an_old_power_up_goes_and_leaves_the_track_as_it_was() {
        let mut spots = [[0; 2]; 16];
        spots[0] = [20, 20];
        let mut rand = Rand::new(3);
        let mut image = track();
        image.pixels[20 * 64 + 20] = 99;
        let mut power_ups = PowerUps::new(&mut image, &spots, pictures(), &mut rand);
        power_ups.lay(&mut image, 0, 2);
        power_ups.wait = 50;
        assert_eq!(image.pixels[20 * 64 + 20], 2);
        power_ups.places[0].age = 1985;
        power_ups.step(&mut image, 0, true, &mut rand);
        assert_eq!(image.pixels[20 * 64 + 20], 99, "hidden while it blinks");
        power_ups.places[0].age = 1995;
        power_ups.step(&mut image, 0, true, &mut rand);
        assert_eq!(image.pixels[20 * 64 + 20], 2, "shown while it blinks");
        power_ups.step(&mut image, 10, true, &mut rand);
        assert_eq!(power_ups.places[0].kind, 0);
        assert_eq!(image.pixels[20 * 64 + 20], 99);
        assert_eq!(power_ups.wait, AFTER_ONE_WENT);
        assert!((300..500).contains(&power_ups.places[0].wait));
    }
}
