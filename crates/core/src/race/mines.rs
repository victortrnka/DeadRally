//! Mines (spec M4c), `sub_40F6A0` for each car each logic tick: the mine key drops one behind
//! the car (painted into the track's picture, `MINES1A.BPK`), not within 8 pixels of another
//! and at most every 50 ticks; a car whose sprite meets an armed mine sets it off: a crater is
//! painted, the car is hurt, jolted and spun by `rand()` and thrown back; and the blast is
//! drawn (`sub_40FE20`, `BLOWI.BPK`), a picture each 5 ticks of the timer.

use crate::campaign::Rand;
use crate::trig::{cos, sin};

use super::buffer::{Buffer, STRIDE};
use super::driving::{BRAKE, Car, MINE, RADIANS};
use super::raster::{ftol, nearest};

/// The mines the race keeps (0x481C00, 16 bytes each), and an armed one's state.
const SLOTS: usize = 32;
const ARMED: i32 = -1;
/// The frame from which mines drop, and the ticks between two from one car.
const FIRST_FRAME: i32 = 430;
const COOLDOWN: i32 = 50;
/// The blast's pictures (16x16, the sixth the crater) and the mine's (8x8).
const BLAST: usize = 16;
const CRATER: usize = 5;
const MINE_SIDE: usize = 8;
/// The timer's ticks between the blast's pictures.
const BLAST_TICKS: u32 = 5;

/// A mine: where it lies, its state (armed, or the blast's picture) and the timer's tick of
/// its last picture. A slot keeps what it last held when the mines after it move down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mine {
    x: i32,
    y: i32,
    state: i32,
    time: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mines {
    slots: [Mine; SLOTS],
    count: usize,
    /// The mine's picture and the blast's.
    picture: Vec<u8>,
    blast: Vec<u8>,
}

/// What the mines ask the race to play: channel, effect, volume.
pub(super) type Sound = (usize, u8, u32);

/// The track the mines lie on.
pub(super) struct Track<'a> {
    pub(super) image: &'a mut [u8],
    pub(super) width: i32,
}

impl Track<'_> {
    /// A picture's non-zero pixels painted with its top left at (`x`, `y`).
    fn paint(&mut self, picture: &[u8], side: usize, (x, y): (i32, i32)) {
        for row in 0..side {
            for column in 0..side {
                let pixel = picture.get(row * side + column).copied().unwrap_or(0);
                let at = (y + row as i32)
                    .wrapping_mul(self.width)
                    .wrapping_add(x + column as i32);
                if pixel != 0
                    && let Some(slot) = usize::try_from(at)
                        .ok()
                        .and_then(|at| self.image.get_mut(at))
                {
                    *slot = pixel;
                }
            }
        }
    }
}

/// The volume of a sound at car `from` as the player hears it: 0x9000 from the player's
/// own car, else 0x10000 less 75 a pixel, nothing at 0x1000 or less.
fn heard(cars: &[Car], from: usize, player: usize) -> Option<u32> {
    if from == player {
        return Some(0x9000);
    }
    let volume = 0x1_0000 - 75 * cars[from].distance(&cars[player]);
    (volume > 0x1000).then_some(volume as u32)
}

impl Mines {
    /// None dropped yet; `picture` the mine's, `blast` the blast's six. Every race starts its
    /// slots afresh, armed and their times 0, as `initRaceValues` sets them (0x409B13).
    pub(crate) fn new(picture: Vec<u8>, blast: Vec<u8>) -> Mines {
        Mines {
            slots: [Mine {
                x: 0,
                y: 0,
                state: ARMED,
                time: 0,
            }; SLOTS],
            count: 0,
            picture,
            blast,
        }
    }

    /// Where the mines dropped lie, blown or not.
    pub(super) fn places(&self) -> Vec<(i32, i32)> {
        self.slots[..self.count]
            .iter()
            .map(|mine| (mine.x, mine.y))
            .collect()
    }

    /// `sub_40F6A0` for the car in place `slot` holding `keys`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step(
        &mut self,
        cars: &mut [Car],
        slot: usize,
        keys: u32,
        frame: i32,
        sprites: &[u8],
        track: &mut Track,
        player: usize,
        rand: &mut Rand,
    ) -> Vec<Sound> {
        let f = |value: f32| f64::from(value);
        let mut sounds = Vec::new();
        let car = &mut cars[slot];
        if car.mine_cooldown > 0 {
            car.mine_cooldown -= 1;
        }
        let dropping = car.handling.damage > 0
            && !car.finished
            && keys & MINE != 0
            && car.handling.mines > 0
            && frame > FIRST_FRAME
            && car.mine_cooldown == 0
            && keys & BRAKE == 0;
        if dropping {
            car.mine_cooldown = COOLDOWN;
            let r = f(car.angle) * RADIANS;
            let size = f(car.handling.size);
            let bx = nearest(sin(r) * size * 2.3);
            let mx = ftol((f(car.x) - 4.0) + f64::from(bx));
            let by = nearest(cos(r) * size * 1.916_665_9);
            let my = ftol((f(car.y) - 4.0) + f64::from(by));
            let crowded = self.slots[..self.count]
                .iter()
                .any(|mine| (mine.x - mx - 4).abs() < 8 && (mine.y - my - 4).abs() < 8);
            if !crowded && self.count < SLOTS {
                if let Some(volume) = heard(cars, slot, player) {
                    sounds.push((4, 18, volume));
                }
                let mine = &mut self.slots[self.count];
                mine.x = mx + 4;
                mine.y = my + 4;
                self.count += 1;
                track.paint(&self.picture, MINE_SIDE, (mx, my));
                cars[slot].handling.mines -= 1;
            }
        }
        for index in 0..self.count {
            let mine = self.slots[index];
            let car = &cars[slot];
            let dx = ftol(f(car.x)) - mine.x;
            let dy = ftol(f(car.y)) - mine.y;
            if dx.abs() >= 20 || dy.abs() >= 20 {
                continue;
            }
            let over = usize::try_from(car.sprite as i32 + dy * 40 + dx + 0x334)
                .ok()
                .and_then(|at| sprites.get(at))
                .is_some_and(|&pixel| pixel > 3);
            if !over || mine.state != ARMED {
                continue;
            }
            let crater = self.blast.get(CRATER * BLAST * BLAST..).unwrap_or(&[]);
            track.paint(crater, BLAST, (mine.x - 8, mine.y - 8));
            self.slots[index].state = 0;
            let car = &mut cars[slot];
            let h = &mut car.handling;
            if !car.finished {
                h.damage += 20 * (h.armour - 0x400);
            }
            h.damage = h.damage.max(0);
            let mut jolt = |push: f32| (f64::from(rand.next() % 3 - 1) * 0.5 + f(push)) as f32;
            car.push = [jolt(car.push[0]), jolt(car.push[1])];
            car.x = (f64::from(rand.next() % 11 - 5) + f(car.x)) as f32;
            car.y = (f64::from(rand.next() % 11 - 5) + f(car.y)) as f32;
            car.spin = (rand.next() % 22 - 10) as f32;
            car.speed = (f(car.speed) - 1.7 * f(car.speed)) as f32;
            if let Some(volume) = heard(cars, slot, player) {
                sounds.push((4, 23, volume));
            }
        }
        sounds
    }

    /// `sub_40FE20`: each blast's picture where the view shows it (not the crater, already
    /// painted), the next picture each 5 ticks of the timer `now`, and the mine gone after the
    /// last.
    pub(super) fn draw(
        &mut self,
        buffer: &mut Buffer,
        (camera_x, camera_y): (i32, i32),
        left: i32,
        now: u32,
    ) {
        let mut index = 0;
        while index < self.count {
            let mine = self.slots[index];
            if mine.state == ARMED {
                index += 1;
                continue;
            }
            let column = mine.x - camera_x + left;
            let row = mine.y - camera_y;
            let shown = column >= 8 && column + 8 < 320 && row >= 8 && row + 8 < 200;
            if shown && mine.state != CRATER as i32 {
                let start = usize::try_from(mine.state).unwrap_or(0) * BLAST * BLAST;
                let picture = self.blast.get(start..).unwrap_or(&[]);
                let at = i64::from(row - 8) * STRIDE as i64 + i64::from(column - 8) + 0x60;
                buffer.draw(picture, BLAST, BLAST, at);
            }
            let mine = &mut self.slots[index];
            if now >= mine.time + BLAST_TICKS {
                mine.state += 1;
                mine.time = now;
            }
            if mine.state > CRATER as i32 {
                mine.state = ARMED;
                // The later mines move down a slot; each slot keeps its own time.
                for later in index..self.count - 1 {
                    let next = self.slots[later + 1];
                    let slot = &mut self.slots[later];
                    slot.x = next.x;
                    slot.y = next.y;
                    slot.state = next.state;
                }
                self.count -= 1;
            } else {
                index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::race::driving::{FRAME, FRAMES, Handling, SIDE};
    use deadrally_gamedata::handling::Guns;

    const WIDTH: i32 = 400;

    fn car(slot: usize, (x, y): (f32, f32), mines: i32) -> Car {
        let handling = Handling {
            car: 1,
            engine: 2.5,
            engine_backup: 2.5,
            tires: 0.5,
            size: 9.0,
            steering: 2.5,
            damage: 0x1_0000,
            armour: 400,
            rocket: 0,
            weapons_bar: 102_400,
            turbo: 102_400,
            rocket_used: false,
            mines,
            money: 0,
            weapons: true,
            guns: Guns::default(),
        };
        Car::new((x, y, 0), slot, handling, 0)
    }

    /// Every car's sprite: solid in its middle 20x20, clear round it.
    fn sprites() -> Vec<u8> {
        let mut sprites = vec![0u8; 2 * FRAMES * FRAME];
        for frame in sprites.chunks_mut(FRAME) {
            for row in 10..30 {
                frame[row * SIDE + 10..row * SIDE + 30].fill(5);
            }
        }
        sprites
    }

    /// The mine's picture all colour 7; the blast's six pictures each their number plus 1.
    fn mines() -> Mines {
        let blast = (0..6u8).flat_map(|k| [k + 1; BLAST * BLAST]).collect();
        Mines::new(vec![7; MINE_SIDE * MINE_SIDE], blast)
    }

    fn step(
        mines: &mut Mines,
        cars: &mut [Car],
        slot: usize,
        keys: u32,
        frame: i32,
        image: &mut [u8],
    ) -> Vec<Sound> {
        let mut track = Track {
            image,
            width: WIDTH,
        };
        mines.step(
            cars,
            slot,
            keys,
            frame,
            &sprites(),
            &mut track,
            0,
            &mut Rand::new(5),
        )
    }

    /// The mine key drops a mine behind the car (painted into the track, one fewer left) only
    /// from the race's 431st frame, not with the brake held, then not again for 50 ticks, and
    /// never onto another mine: a player can't stack mines or drop them on the grid.
    #[test]
    fn a_mine_drops_behind_the_car_once_per_cooldown_and_never_onto_another() {
        let mut image = vec![0u8; (WIDTH * 200) as usize];
        let mut cars = vec![car(0, (200.0, 100.0), 3)];
        let mut mines = mines();
        assert!(step(&mut mines, &mut cars, 0, MINE, 430, &mut image).is_empty());
        assert!(step(&mut mines, &mut cars, 0, MINE | BRAKE, 431, &mut image).is_empty());
        assert_eq!((mines.count, cars[0].handling.mines), (0, 3));

        let sounds = step(&mut mines, &mut cars, 0, MINE, 431, &mut image);
        assert_eq!(sounds, vec![(4, 18, 0x9000)]);
        assert_eq!((mines.count, cars[0].handling.mines), (1, 2));
        // Facing angle 0 the car's back is down: 9 * 1.9166659 rounds to 17.
        assert_eq!((mines.slots[0].x, mines.slots[0].y), (200, 117));
        assert_eq!(image[(113 * WIDTH + 196) as usize], 7);
        assert_eq!(image[(112 * WIDTH + 196) as usize], 0);

        for _ in 0..49 {
            step(&mut mines, &mut cars, 0, MINE, 431, &mut image);
        }
        assert_eq!(mines.count, 1);
        // The cooldown is over, but the new mine would lie on the old one.
        assert!(step(&mut mines, &mut cars, 0, MINE, 431, &mut image).is_empty());
        assert_eq!((mines.count, cars[0].handling.mines), (1, 2));
    }

    /// A car whose sprite meets an armed mine sets it off: a crater in the track, its armour's
    /// shortfall twenty times over off its damage bar, thrown back at 0.7 of its speed and
    /// spun; the blast is heard. A mine goes off once.
    #[test]
    fn a_car_over_an_armed_mine_is_hurt_and_thrown_back_once() {
        let mut image = vec![0u8; (WIDTH * 200) as usize];
        let mut cars = vec![car(0, (200.0, 100.0), 1), car(1, (300.0, 100.0), 0)];
        let mut mines = mines();
        step(&mut mines, &mut cars, 0, MINE, 431, &mut image);
        cars[1].x = 200.0;
        cars[1].y = 117.0;
        cars[1].speed = 4.0;

        let sounds = step(&mut mines, &mut cars, 1, 0, 432, &mut image);
        assert_eq!(sounds.len(), 1);
        assert_eq!((sounds[0].0, sounds[0].1), (4, 23));
        assert_eq!(cars[1].handling.damage, 0x1_0000 - 20 * (0x400 - 400));
        assert_eq!(cars[1].speed, (4.0 - 1.7 * 4.0) as f32);
        assert_eq!(mines.slots[0].state, 0);
        // The crater: the blast's sixth picture, centred on the mine.
        assert_eq!(image[(109 * WIDTH + 192) as usize], 6);

        let damage = cars[1].handling.damage;
        assert!(step(&mut mines, &mut cars, 1, 0, 433, &mut image).is_empty());
        assert_eq!(cars[1].handling.damage, damage);
    }

    /// The blast shows a picture each 5 ticks of the timer, then the mine is gone and the
    /// mines after it move down: a later mine stays armed where it was dropped.
    #[test]
    fn the_blast_runs_its_pictures_five_ticks_apart_then_the_mine_is_gone() {
        let mut image = vec![0u8; (WIDTH * 200) as usize];
        let mut cars = vec![car(0, (200.0, 100.0), 2), car(1, (300.0, 100.0), 0)];
        let mut mines = mines();
        step(&mut mines, &mut cars, 0, MINE, 431, &mut image);
        cars[0].x = 260.0;
        for _ in 0..50 {
            step(&mut mines, &mut cars, 0, 0, 431, &mut image);
        }
        step(&mut mines, &mut cars, 0, MINE, 431, &mut image);
        assert_eq!(mines.count, 2);
        (cars[1].x, cars[1].y) = (200.0, 117.0);
        step(&mut mines, &mut cars, 1, 0, 432, &mut image);

        let mut buffer = Buffer::default();
        let view = (100, 50);
        mines.draw(&mut buffer, view, 64, 1000);
        // The blast's first picture, centred where the mine lies on the view.
        assert_eq!(buffer.pixel(200 - 100 + 64 - 8, 117 - 50 - 8), 1);
        let mut states = vec![mines.slots[0].state];
        for now in 1001..1030 {
            mines.draw(&mut buffer, view, 64, now);
            states.push(mines.slots[0].state);
        }
        // The first draw moves on at once (the slot's time is old), then every 5 ticks.
        assert_eq!(&states[..6], &[1, 1, 1, 1, 1, 2]);
        assert_eq!(mines.count, 1);
        assert_eq!(
            (mines.slots[0].x, mines.slots[0].y, mines.slots[0].state),
            (260, 117, ARMED)
        );
    }
}
