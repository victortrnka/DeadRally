//! The cars driving (spec M4c): each car's handling as `initParticipantValues` (0x401060) sets
//! it up, and its step of a tick, `calculateUserMovements` (0x40BAB0): the keys it holds speed
//! it up, slow it down and turn it, the ground under its corners slows or spins it, and a wall
//! under its sprite turns it away or, when it is stuck, shakes it loose.
//!
//! The original computes in the x87's 53-bit precision and stores floats: the arithmetic here
//! is in `f64`, rounded to `f32` wherever the original stores a float, in the original's order.

use deadrally_gamedata::handling::{CARS, Guns, HandlingTables, UPGRADES};

use crate::campaign::Rand;
use crate::trig::{cos, sin};

use super::Driver;
use super::raster::ftol;

/// The keys a car holds in a tick, as `sub_4138A0` samples them.
pub(super) const ACCELERATE: u32 = 0x01;
pub(super) const BRAKE: u32 = 0x02;
pub(super) const LEFT: u32 = 0x04;
pub(super) const RIGHT: u32 = 0x08;
pub(super) const TURBO: u32 = 0x10;
pub(super) const MINE: u32 = 0x40;

/// A car's sprite: 40x40, 96 directions a car.
pub(super) const SIDE: usize = 40;
pub(super) const FRAME: usize = SIDE * SIDE;
pub(super) const FRAMES: usize = 96;

/// Degrees to radians as the original has it (0x4412B0, a little off pi/180).
pub(super) const RADIANS: f64 = 0.017_453_292_519_944_444;
/// The damage bar of a car without damage, and the turbo's full bar.
const FULL_BAR: i32 = 102_400;
/// The ground's kinds under a corner (a mask byte's low nibble): below 4 a wall, 15 off the
/// track.
const WALL: u32 = 4;
const OFF_TRACK: u32 = 15;
/// How near the track's edges a car may come.
const EDGE: f64 = 20.0;

/// A car's handling and what it has left (`raceParticipant2`, 0x4A6880, 0x94 a car).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Handling {
    /// The car (0 to 5).
    pub(super) car: usize,
    /// The engine's power now, and as set up (0x4A6888), which the balance scales.
    pub(super) engine: f32,
    pub(super) engine_backup: f32,
    pub(super) tires: f32,
    pub(super) size: f32,
    /// The degrees it turns a tick.
    pub(super) steering: f32,
    /// What is left of the car, [`FULL_BAR`] with no damage.
    pub(super) damage: i32,
    pub(super) armour: i32,
    pub(super) rocket: i32,
    /// The machine gun's and the turbo's bars, [`FULL_BAR`] full (0x4A68B0, 0x4A68B4).
    pub(super) weapons_bar: i32,
    pub(super) turbo: i32,
    pub(super) rocket_used: bool,
    /// The mines it has left (0x4A68A8).
    pub(super) mines: i32,
    /// The money power-ups picked up in the race (0x4A68D0).
    pub(super) money: i32,
    /// Whether the race has weapons (0x4A68AC), and the car's machine guns.
    pub(super) weapons: bool,
    pub(super) guns: Guns,
}

impl Handling {
    /// The handling `initParticipantValues` gives a driver's car (`player` for the player's);
    /// the tables' tough name gets its armour 2.2 times over.
    pub(super) fn new(
        tables: &HandlingTables,
        setup: &Driver,
        player: bool,
        weapons: bool,
    ) -> Handling {
        let row = setup.car + CARS * setup.level;
        let upgrade = |level: i32| row * UPGRADES + level.clamp(0, UPGRADES as i32 - 1) as usize;
        let float = |table: &[f32], index: usize| table.get(index).copied().unwrap_or(0.0);
        let int = |table: &[i32], index: usize| table.get(index).copied().unwrap_or(0);
        let steering = f64::from(float(&tables.steering, row));
        let mut armour = int(&tables.armour, row).wrapping_add(int(
            &tables.armour_upgrade,
            UPGRADES * setup.level + setup.armour.clamp(0, UPGRADES as i32 - 1) as usize,
        ));
        if player {
            armour = armour.wrapping_add(100);
        }
        let mut name = setup.name.clone();
        name.push(0);
        let tough = tables.tough.as_slice();
        if name.get(..tough.len()) == Some(tough) {
            armour = ftol(f64::from(armour) * 2.2);
        }
        let engine = float(&tables.engine, upgrade(setup.engine));
        Handling {
            car: setup.car,
            engine,
            engine_backup: engine,
            tires: float(&tables.tires, upgrade(setup.tires)),
            size: float(&tables.size, setup.car),
            steering: (3.75 / (steering - f64::from(setup.engine) * 0.05)) as f32,
            damage: (100 - setup.damage).wrapping_shl(10),
            armour: armour.min(900),
            rocket: setup.rocket,
            weapons_bar: FULL_BAR,
            turbo: FULL_BAR,
            rocket_used: false,
            mines: setup.mines,
            money: 0,
            weapons,
            guns: tables.guns.get(setup.car).cloned().unwrap_or_default(),
        }
    }
}

/// The track's mask, which tells the ground's kind under each pixel.
pub(super) struct Ground<'a> {
    pub(super) mask: &'a [u8],
    pub(super) width: i32,
    pub(super) height: i32,
}

impl Ground<'_> {
    /// The ground's kind at the pixel `index`, off the track past either end.
    fn at(&self, index: i32) -> u32 {
        let size = self.width.wrapping_mul(self.height);
        if index < 0 || index >= size {
            OFF_TRACK
        } else {
            self.mask
                .get(index as usize)
                .map_or(OFF_TRACK, |&byte| u32::from(byte & 15))
        }
    }

    /// The ground's kind under a corner at (`x`, `y`), each truncated.
    fn under(&self, x: f64, y: f64) -> u32 {
        let index = ftol(y).wrapping_mul(self.width).wrapping_add(ftol(x));
        self.at(index)
    }

    /// `sub_43BFE0`: the first pixel, row by row, where the car's sprite (40x40) has more than
    /// colour 3 over a wall, its kind and where it is in the sprite; `None` when there is
    /// none. The original reads past the mask's ends unchecked; nothing out there counts here.
    fn wall_under(&self, sprite: &[u8], start: i32) -> Option<(u32, i32, i32)> {
        for row in 0..SIDE as i32 {
            for column in 0..SIDE as i32 {
                let pixel = sprite.get(row as usize * SIDE + column as usize).copied();
                let index = start
                    .wrapping_add(row.wrapping_mul(self.width))
                    .wrapping_add(column);
                let kind = usize::try_from(index)
                    .ok()
                    .and_then(|i| self.mask.get(i))
                    .map(|&byte| u32::from(byte & 15));
                if let (Some(pixel), Some(kind)) = (pixel, kind)
                    && pixel > 3
                    && kind < WALL
                {
                    return Some((kind, row, column));
                }
            }
        }
        None
    }
}

/// A car in the race (`raceParticipantIngame`, 0x4A7D00, 0x360 a car).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Car {
    pub(super) x: f32,
    pub(super) y: f32,
    /// Degrees, 0 to 360.
    pub(super) angle: f32,
    pub(super) speed: f32,
    /// Its slide sideways, how hard it grips into a turn, and the spin the ground gives it.
    pub(super) slide: f32,
    pub(super) grip: f32,
    pub(super) spin: f32,
    /// The degrees it turns a tick (`currentSteeringAngleDelta` 0x4A7DA8).
    pub(super) turn: f32,
    /// Its last step, and the push back it carries into the next.
    pub(super) step: [f32; 2],
    pub(super) push: [f32; 2],
    /// Its seven corners (0x4A7DC4), from its middle.
    pub(super) corners: [[f32; 2]; 7],
    /// Its sprite's direction (0 to 95) and where its sprite is among all the cars'.
    pub(super) direction: i32,
    pub(super) sprite: usize,
    /// Whether it ran into a wall this tick (0x4A7D14), and the two knocks a car-car
    /// collision gives (0x4A7D18, 0x4A7D1C).
    pub(super) wall: i32,
    pub(super) knocks: [i32; 2],
    /// The wall it last ran into: its kind (0x4A7E90) and where in its sprite (0x4A7E74).
    pub(super) hit: i32,
    pub(super) hit_at: [i32; 2],
    /// How long it has been at walls (0x4A7E94) and at other cars (0x4A7E98).
    pub(super) stuck: i32,
    pub(super) knocked: i32,
    /// The shakes that work it loose from a wall: their last move, reach and count.
    pub(super) shake: [f32; 2],
    pub(super) shake_reach: i32,
    pub(super) shake_count: i32,
    pub(super) finished: bool,
    /// The ticks before it may drop another mine (0x4A7EA8), and whether its horn sounds
    /// (0x4A8058).
    pub(super) mine_cooldown: i32,
    pub(super) horn: bool,
    /// Where it was a tick ago (0x4A7E50).
    pub(super) previous: [f32; 3],
    /// The keys it holds in each tick of the race loop's pass (0x4A7D20).
    pub(super) keys: [u32; 16],
    pub(super) lap: i32,
    pub(super) place: i32,
    /// Its engine's sound: the pitch over the idle and the rise to full speed (0x4A7E9C,
    /// 0x4A7EA0).
    pub(super) note: [i32; 2],
    /// Its four wheels now and where their marks have got to (0x4A7E10 to 0x4A7E4C).
    pub(super) wheels: [[f32; 2]; 4],
    pub(super) trail: [[f32; 2]; 4],
    /// The ticks it still leaves red tracks for (0x4A8054), and its skid's ticks towards the
    /// next puff (0x4A804C).
    pub(super) bloody: i32,
    pub(super) skids: i32,
    /// The zone it last reached (`actualVaiZone` 0x4A7D00).
    pub(super) zone: i32,
    /// The ticks a power-up's effect on the player's view lasts (0x4A8050).
    pub(super) effect: i32,
    /// Its machine guns' shots, flashes and sparks.
    pub(super) gunfire: super::guns::Gunfire,
    /// The smoke puffs off its rear wheels, left and right: age (0 for none) and place
    /// (0x4A7EE4, 0x4A7F20).
    pub(super) puffs: [[[i32; 3]; super::marks::PUFFS]; 2],
    pub(super) handling: Handling,
}

impl Car {
    /// A car on the grid at (`x`, `y`) facing `rotation` (in 3.75 degree steps).
    pub(super) fn new(
        (x, y, rotation): (f32, f32, i32),
        slot: usize,
        handling: Handling,
        engine_upgrade: i32,
    ) -> Car {
        let angle = (f64::from(rotation) * 3.75) as f32;
        Car {
            x,
            y,
            angle,
            speed: 0.0,
            slide: 0.0,
            grip: 0.0,
            spin: 0.0,
            turn: handling.steering,
            step: [0.0; 2],
            push: [0.0; 2],
            corners: [[0.0; 2]; 7],
            direction: rotation,
            sprite: FRAME * (rotation.max(0) as usize + FRAMES * slot),
            wall: 0,
            knocks: [0; 2],
            hit: 0,
            hit_at: [0; 2],
            stuck: 0,
            knocked: 0,
            shake: [0.0; 2],
            shake_reach: 0,
            shake_count: 0,
            finished: false,
            mine_cooldown: 0,
            horn: false,
            previous: [x, y, angle],
            keys: [0; 16],
            lap: 1,
            place: slot as i32 + 1,
            note: [0, (engine_upgrade + 3).wrapping_mul(5000)],
            wheels: [[0.0; 2]; 4],
            trail: [[0.0; 2]; 4],
            bloody: 0,
            skids: 0,
            zone: 0,
            effect: 0,
            gunfire: super::guns::Gunfire::default(),
            puffs: [[[0; 3]; super::marks::PUFFS]; 2],
            handling,
        }
    }

    /// The whole pixels to `other` as the sounds measure them: each axis and the root
    /// truncated.
    pub(super) fn distance(&self, other: &Car) -> i32 {
        let dx = ftol(f64::from(self.x) - f64::from(other.x));
        let dy = ftol(f64::from(self.y) - f64::from(other.y));
        ftol(f64::from(dx * dx + dy * dy).sqrt())
    }

    /// `calculateUserMovements` (0x40BAB0) for the car in place `slot` holding `keys`; for
    /// the player, `rocket_ticks` counts the ticks its rocket has burned.
    pub(super) fn drive(
        &mut self,
        keys: u32,
        slot: usize,
        ground: &Ground,
        sprites: &[u8],
        rand: &mut Rand,
        rocket_ticks: Option<&mut i32>,
    ) {
        let f = f64::from;
        self.step = [0.0; 2];
        let mut lateral = 0.0f32;
        self.speed = (f(self.speed) - f(self.speed) * f(0.02f32)) as f32;
        if self.handling.damage > 0 {
            lateral = self.handle(keys, rocket_ticks);
        }
        self.advance(lateral);
        self.place_corners();
        let x = f(self.x);
        let y = f(self.y);
        let under = |[cx, cy]: [f32; 2]| ground.under(f(cx) + x, y + f(cy));
        let near = self.corners[..6]
            .iter()
            .map(|&corner| under(corner))
            .collect::<Vec<_>>();
        // The front's row comes from the unrounded corner (`fst` then `fadd`).
        let front_y = cos(self.radians(180.0)) * 18.333_326 + f(self.step[1]);
        let front = ground.under(f(self.corners[6][0]) + x, front_y + y);
        let ground_at = [near[0], near[1], near[2], near[3], near[4], near[5]];
        self.direction = ftol(f(self.angle) * 0.266_666_666_666_666_66);
        self.sprite = (slot as i32)
            .wrapping_mul(FRAMES as i32)
            .wrapping_add(self.direction)
            .wrapping_mul(FRAME as i32)
            .max(0) as usize;
        let sprite = sprites.get(self.sprite..).unwrap_or(&[]);
        let start = ftol((y - EDGE) + f(self.step[1]))
            .wrapping_mul(ground.width)
            .wrapping_add(ftol(f(self.step[0])))
            .wrapping_add(ftol(x))
            .wrapping_sub(EDGE as i32);
        match ground.wall_under(sprite, start) {
            Some((kind, row, column)) => {
                self.hit = kind as i32;
                self.hit_at = [column - EDGE as i32, row - EDGE as i32];
                self.wall = kind as i32;
                self.bounce(ground, sprite, front, ground_at, rand);
            }
            None => self.wall = 0,
        }
        self.surfaces(ground_at, rand);
        self.x = (f(self.step[0]) + f(self.x)) as f32;
        self.y = (f(self.step[1]) + f(self.y)) as f32;
        self.keep_on_track(ground);
    }

    /// The keys' work while the car still runs: its speed, turbo, slide and turn; the
    /// sideways push its turning gives (0x50E71C).
    fn handle(&mut self, keys: u32, rocket_ticks: Option<&mut i32>) -> f32 {
        let f = f64::from;
        let thirtieth = f(1.0f32 / 30.0);
        let h = &mut self.handling;
        if !self.finished {
            if keys & ACCELERATE != 0 {
                let boost = if keys & TURBO != 0 && h.turbo > 0 {
                    if h.rocket != 0 {
                        h.damage = (h.damage - 22).max(0);
                        h.rocket_used = true;
                        1.5
                    } else {
                        1.3
                    }
                } else {
                    0.8
                };
                self.speed = (f(h.engine) * boost * (1.0 / 30.0) + f(self.speed)) as f32;
                let t = f(self.speed) - f(self.speed) * thirtieth;
                self.speed = (f(0.02f32) * t + t) as f32;
            }
            if let Some(ticks) = rocket_ticks {
                if h.rocket_used {
                    *ticks += 1;
                } else {
                    *ticks = 0;
                }
            }
            if keys & BRAKE != 0 && keys & MINE == 0 {
                let t = f(self.speed) - f(h.engine) * f(1.0f32 / 3.0) * (1.0 / 30.0);
                let t = t - thirtieth * t;
                self.speed = (f(0.02f32) * t + t) as f32;
            }
            if keys & TURBO != 0 {
                if h.turbo > 0 && h.damage > 0 {
                    h.turbo = (h.turbo - 400).max(0);
                }
            } else if h.damage > 0 {
                h.turbo = (h.turbo + 20).min(FULL_BAR);
            }
        }
        if h.damage <= 0 {
            h.engine = 0.0;
        }
        if !self.finished && h.engine > 0.0 {
            let slide = f(self.grip) * f(self.speed) * (f(h.tires) / f(h.engine));
            self.slide = slide as f32;
            if f(self.slide).abs() > 20.0 {
                self.speed = (f(self.speed) - 0.02 * f(self.speed)) as f32;
            }
        } else {
            self.slide = (f(self.slide) - 0.1 * f(self.slide)) as f32;
        }
        let quarter = || 90.0 / f(self.turn);
        let slew = f(self.slide) * f(1.0f32 / 36.0);
        let free = self.knocks == [0, 0];
        let mut lateral = 0.0f32;
        if keys & RIGHT != 0 && free {
            let a = -(f(h.size) / quarter());
            lateral = (a - 2.0 * (slew * a)) as f32;
            self.angle = (f(self.angle) - f(self.turn)) as f32;
            if self.grip > -36.0 {
                self.grip = (f(self.grip) - 2.0) as f32;
            }
        }
        if self.angle < 0.0 {
            self.angle = (f(self.angle) + 360.0) as f32;
        }
        if keys & LEFT != 0 && free {
            let b = f(h.size) / quarter() + f(lateral);
            lateral = (b * (2.0 * slew + 1.0)) as f32;
            self.angle = (f(self.angle) + f(self.turn)) as f32;
            if self.grip < 36.0 {
                self.grip = (f(self.grip) + 2.0) as f32;
            }
        }
        if self.angle >= 360.0 {
            self.angle = (f(self.angle) - 360.0) as f32;
        }
        if self.grip > 0.0 {
            self.grip = (f(self.grip) - 1.0) as f32;
        }
        if self.grip < 0.0 {
            self.grip = (f(self.grip) + 1.0) as f32;
        }
        if h.damage > 0 {
            self.angle = (f(self.spin) + f(self.angle)) as f32;
        }
        self.wrap_angle();
        lateral
    }

    fn wrap_angle(&mut self) {
        if self.angle < 0.0 {
            self.angle = (f64::from(self.angle) + 360.0) as f32;
        }
        if self.angle >= 360.0 {
            self.angle = (f64::from(self.angle) - 360.0) as f32;
        }
    }

    /// The car's angle plus `degrees`, in radians.
    fn radians(&self, degrees: f64) -> f64 {
        (f64::from(self.angle) + degrees) * RADIANS
    }

    /// The step of a tick: its speed ahead and its slide sideways scaled to its speed, the
    /// turn's push, and the push back from a wall.
    fn advance(&mut self, lateral: f32) {
        let f = f64::from;
        let r = self.radians(270.0);
        let push_x = (sin(r) * f(lateral)) as f32;
        let push_y = cos(r) * f(lateral) * 0.833_333;
        let r = self.radians(180.0);
        let ahead_x = (sin(r) * f(self.speed)) as f32;
        let ahead_y = cos(r) * f(self.speed) * 0.833_333;
        let r = ((f(self.angle) - 90.0) + 180.0) * RADIANS;
        let side_x = (sin(r) * f(self.slide) * 0.090_909_090_909_090_91) as f32;
        let side_y = cos(r) * f(self.slide) * 0.075_757_545_454_545_45;
        let x = f(side_x) + f(ahead_x);
        let y = side_y + ahead_y;
        let length = x * x + y * y;
        let scale = if length == 0.0 {
            1.0
        } else {
            f(self.speed).abs() / length.sqrt()
        };
        let step_x = ((f(side_x) + f(ahead_x)) * scale) as f32;
        let step_y = ((side_y + ahead_y) * scale) as f32;
        let step_x = (f(push_x) + f(step_x)) as f32;
        let step_y = (push_y + f(step_y)) as f32;
        self.step = [
            (f(self.push[0]) + f(step_x)) as f32,
            (f(self.push[1]) + f(step_y)) as f32,
        ];
    }

    /// The seven corners where the car feels the ground (0x40C0A1), around where its step
    /// takes it.
    fn place_corners(&mut self) {
        let f = f64::from;
        let [dx, dy] = self.step.map(f);
        let at =
            |r: f64, (rx, ry): (f64, f64)| [(sin(r) * rx + dx) as f32, (cos(r) * ry + dy) as f32];
        let far = (18.0, 14.999_994);
        let near = (8.0, 6.666_664);
        let back = |degrees: f64| ((f(self.angle) + 180.0) - degrees) * RADIANS;
        self.corners = [
            at(self.radians(206.0), far),
            at(back(26.0), far),
            at(self.radians(334.0), far),
            at(back(154.0), far),
            at(self.radians(270.0), near),
            at(back(90.0), near),
            at(self.radians(180.0), (22.0, 18.333_326)),
        ];
    }

    /// A wall under the sprite (0x40C590): stuck for a while, it is shaken loose; just stuck,
    /// it is pushed back; fresh, it turns away from the side that hit and slows.
    fn bounce(
        &mut self,
        ground: &Ground,
        sprite: &[u8],
        front: u32,
        ground_at: [u32; 6],
        rand: &mut Rand,
    ) {
        let f = f64::from;
        if self.stuck > 3 {
            self.speed = 0.0;
            self.shake_reach = 1;
            self.shake_count = 0;
            let mut tries = 0;
            while self.wall < WALL as i32 && tries < 100 {
                tries += 1;
                let reach = self.shake_reach;
                let mut shake = || (rand.next() % (2 * reach + 1) - reach) as f32;
                self.shake = [shake(), shake()];
                self.shake_count += 1;
                if self.shake_count % 5 == 0 && self.shake_reach < 30 {
                    self.shake_reach += 1;
                }
                self.x = (f(self.shake[0]) + f(self.x)) as f32;
                self.y = (f(self.y) + f(self.shake[1])) as f32;
                self.keep_on_track(ground);
                let row = ((f(self.y) - EDGE) + f(self.step[1])).floor();
                let start = ftol(row)
                    .wrapping_mul(ground.width)
                    .wrapping_add(ftol(f(self.x).floor()))
                    .wrapping_add(ftol(f(self.step[0]).floor()))
                    .wrapping_sub(EDGE as i32);
                self.wall = ground
                    .wall_under(sprite, start)
                    .map_or(OFF_TRACK as i32, |(kind, _, _)| kind as i32);
                if self.wall < WALL as i32 {
                    self.x = (f(self.x) - f(self.shake[0])) as f32;
                    self.y = (f(self.y) - f(self.shake[1])) as f32;
                }
            }
            self.wall = 0;
        }
        if (1..=3).contains(&self.stuck) {
            // The step times -1.0: its negation, exactly.
            self.push = self.step.map(|d| -d);
            self.speed = 0.0;
            self.wall = 1;
        }
        if self.stuck == 0 {
            // Turned away from the corners on a wall while the front is clear.
            if (ground_at[1] < WALL || ground_at[2] < WALL) && front >= WALL {
                self.angle = (f(self.angle) + 20.0) as f32;
            }
            if (ground_at[0] < WALL || ground_at[3] < WALL) && front >= WALL {
                self.angle = (f(self.angle) - 20.0) as f32;
            }
            self.wrap_angle();
            self.speed = (f(self.speed) - 0.125 * f(self.speed)) as f32;
            self.wall = 1;
        }
    }

    /// The ground under the corners (0x40C955): grass and gravel slow the car, oil spins it,
    /// a boost strip speeds it up.
    fn surfaces(&mut self, ground: [u32; 6], rand: &mut Rand) {
        let f = f64::from;
        let any = |kind: u32| ground.contains(&kind);
        let either = |a: usize, b: usize, kind: u32| ground[a] == kind || ground[b] == kind;
        let slow = |speed: f32, by: f32| (f(speed) - f(speed) * f(by)) as f32;
        let thirtieth = 1.0f32 / 30.0;
        if any(4) {
            self.speed = slow(self.speed, 1.0 / 35.0);
        }
        if either(1, 2, 5) {
            self.spin = (f(self.spin) + 0.2) as f32;
            self.speed = slow(self.speed, thirtieth);
        }
        if either(0, 3, 5) {
            self.spin = (f(self.spin) - 0.2) as f32;
            self.speed = slow(self.speed, thirtieth);
        }
        if any(6) {
            self.speed = slow(self.speed, 1.0 / 9.0);
        }
        if any(7) {
            self.speed = slow(self.speed, 0.05);
        }
        if either(0, 2, 10) {
            self.spin = (f(self.spin) - 0.15) as f32;
        }
        if either(1, 3, 10) {
            self.spin = (f(self.spin) + 0.15) as f32;
        }
        if any(11) {
            self.speed = (f(self.speed) * f(1.0f32 / 75.0) + f(self.speed)) as f32;
        }
        if either(0, 2, 12) {
            self.spin = (f(self.spin) - 0.05) as f32;
        }
        if either(1, 3, 12) {
            self.spin = (f(self.spin) + 0.05) as f32;
        }
        if any(13) {
            let jolt =
                |rand: &mut Rand| f64::from(rand.next()) * (f(self.speed) * 7.0) * (1.0 / 65536.0);
            let turned = jolt(rand) + f(self.angle);
            self.angle = turned as f32;
            let turned = turned - jolt(rand);
            self.angle = turned as f32;
            if turned < 0.0 {
                self.angle = (f(self.angle) + 360.0) as f32;
            }
            if self.angle >= 360.0 {
                self.angle = (f(self.angle) - 360.0) as f32;
            }
            self.speed = slow(self.speed, thirtieth);
        }
        if any(14) {
            self.speed = slow(self.speed, 0.025);
        }
    }

    /// The car kept 20 pixels inside the track's edges.
    fn keep_on_track(&mut self, ground: &Ground) {
        let clamp = |value: f32, size: i32| {
            let far = f64::from(size - EDGE as i32);
            let mut value = f64::from(value);
            if far < value {
                value = far;
            }
            if value < EDGE {
                value = EDGE;
            }
            value as f32
        };
        self.x = clamp(self.x, ground.width);
        self.y = clamp(self.y, ground.height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The float constants the code divides out are the original's own (0x442108, 0x442104,
    /// 0x4420F4, 0x442068, 0x442064, 0x442050): a different last bit drifts a car's speed.
    #[test]
    fn the_fractions_are_the_originals_floats() {
        assert_eq!((1.0f32 / 30.0).to_bits(), 0x3D08_8889);
        assert_eq!((1.0f32 / 3.0).to_bits(), 0x3EAA_AAAB);
        assert_eq!((1.0f32 / 36.0).to_bits(), 0x3CE3_8E39);
        assert_eq!((1.0f32 / 35.0).to_bits(), 0x3CEA_0EA1);
        assert_eq!((1.0f32 / 9.0).to_bits(), 0x3DE3_8E39);
        assert_eq!((1.0f32 / 75.0).to_bits(), 0x3C5A_740E);
        assert_eq!(RADIANS.to_bits(), 0x3F91_DF46_A252_9E84);
    }

    const WIDTH: i32 = 400;

    fn handling() -> Handling {
        Handling {
            car: 0,
            engine: 2.5,
            engine_backup: 2.5,
            tires: 0.5,
            size: 9.0,
            steering: 2.5,
            damage: FULL_BAR,
            armour: 300,
            rocket: 0,
            weapons_bar: FULL_BAR,
            turbo: FULL_BAR,
            rocket_used: false,
            mines: 0,
            money: 0,
            weapons: true,
            guns: Guns::default(),
        }
    }

    /// A car facing right (direction 72) in the middle of open road.
    fn car() -> Car {
        Car::new((200.0, 200.0, 72), 0, handling(), 0)
    }

    fn open_road() -> Vec<u8> {
        vec![15; (WIDTH * WIDTH) as usize]
    }

    fn drive(car: &mut Car, mask: &[u8], keys: u32, ticks: usize) {
        let ground = Ground {
            mask,
            width: WIDTH,
            height: WIDTH,
        };
        let sprites = vec![5u8; FRAME * FRAMES];
        let mut rand = Rand::new(1);
        for _ in 0..ticks {
            car.drive(keys, 0, &ground, &sprites, &mut rand, None);
        }
    }

    /// A car standing on the grid keeps its place and its sprite once the race is on: 72
    /// steps of 3.75 degrees times the original's 0.2666... is 72 again in 53 bits.
    #[test]
    fn a_car_on_the_grid_stays_as_it_stands() {
        let mut car = car();
        drive(&mut car, &open_road(), 0, 1);
        assert_eq!((car.direction, car.sprite), (72, 72 * FRAME));
        assert_eq!((car.x, car.y, car.angle), (200.0, 200.0, 270.0));
    }

    /// Held, the accelerator takes the car to a top speed along its direction (270 degrees is
    /// to the right), not on for ever.
    #[test]
    fn the_accelerator_takes_the_car_to_a_top_speed() {
        let mut car = car();
        drive(&mut car, &open_road(), ACCELERATE, 20);
        let early = car.speed;
        assert!(car.x > 200.0 && (car.y - 200.0).abs() < 0.01, "{car:?}");
        drive(&mut car, &open_road(), ACCELERATE, 600);
        let top = car.speed;
        drive(&mut car, &open_road(), ACCELERATE, 100);
        assert!(
            early < top && (car.speed - top).abs() < 1e-4,
            "{early} {top}"
        );
    }

    /// Right turns the car clockwise on the screen, its angle down by its steering a tick;
    /// left the other way.
    #[test]
    fn right_and_left_turn_the_car_by_its_steering() {
        let mut car = car();
        drive(&mut car, &open_road(), RIGHT, 1);
        assert_eq!(car.angle, 267.5);
        drive(&mut car, &open_road(), LEFT, 2);
        assert_eq!(car.angle, 272.5);
    }

    /// Driven into a wall ahead, a car slows to an eighth less each tick and is pushed back
    /// once it has hit it twice running.
    #[test]
    fn a_wall_slows_the_car_and_pushes_it_back() {
        let mut mask = open_road();
        for y in 0..WIDTH {
            for x in 225..WIDTH {
                mask[(y * WIDTH + x) as usize] = 0;
            }
        }
        let mut car = car();
        car.speed = 3.0;
        car.x = 210.0;
        drive(&mut car, &mask, 0, 1);
        assert_eq!(car.wall, 1);
        assert!(car.speed < 3.0 * 0.98 * 0.875 + 1e-3, "{}", car.speed);
        car.stuck = 2;
        drive(&mut car, &mask, 0, 1);
        assert_eq!(car.speed, 0.0);
        assert!(car.push[0] < 0.0, "{:?}", car.push);
    }
}
