//! The cars' wheels on the track (spec M4c), `sub_411D10` each tick for every car: where its
//! wheels are, the shake a wheelspin at low speed gives it, the marks its tires leave in the
//! track's picture when it skids (red ones for a while after a pedestrian), the smoke puffs
//! off its rear wheels, and how loud the player's tires squeal; and the puffs drawn
//! (`sub_40F070`).

use crate::campaign::Rand;
use crate::trig::{cos, sin};

use super::buffer::{Buffer, STRIDE};
use super::driving::{ACCELERATE, BRAKE, Car, MINE, RADIANS};
use super::raster::ftol;

/// The puffs each rear wheel has, the age one is gone at, and the skid ticks between them.
pub(super) const PUFFS: usize = 15;
const PUFF_AGE: i32 = 13;
const PUFF_EVERY: i32 = 6;
/// A puff's picture (`ENGINE.BPA`'s `SMOKE.BPK`): 8x8, three of them as it ages.
const PUFF_SIDE: usize = 8;
/// The loudest squeal.
pub(super) const LOUDEST: i32 = 0x1_0000;

/// The track the wheels run on: its mask, its picture (which the marks are drawn into) and
/// the tables the marks turn its colours through.
pub(super) struct Track<'a> {
    pub(super) mask: &'a [u8],
    pub(super) image: &'a mut [u8],
    pub(super) width: i32,
    pub(super) skid: &'a [u8; 256],
    pub(super) blood: &'a [u8; 256],
}

impl Track<'_> {
    /// A mark at a wheel: the 2x2 pixels from it turned through `table`, on plain track only.
    fn mark(&mut self, [x, y]: [f32; 2], table: &[u8; 256]) {
        let at = nearest(y).wrapping_mul(self.width).wrapping_add(nearest(x));
        let plain = usize::try_from(at)
            .ok()
            .and_then(|at| self.mask.get(at))
            .is_some_and(|&kind| kind & 15 == 15);
        if !plain {
            return;
        }
        for offset in [0, 1, self.width, self.width + 1] {
            if let Some(pixel) = usize::try_from(at.wrapping_add(offset))
                .ok()
                .and_then(|at| self.image.get_mut(at))
            {
                *pixel = table[usize::from(*pixel)];
            }
        }
    }
}

/// A coordinate rounded half up as the original does it: truncated, and one more when the
/// part cut off is a half or more.
fn nearest(value: f32) -> i32 {
    let value = f64::from(value);
    let whole = ftol(value);
    if value - f64::from(whole) < 0.5 {
        whole
    } else {
        ftol(value + 1.0)
    }
}

/// `sub_411D10` for one car holding `keys` this tick; `squeal` the player's squeal, for the
/// player's car; `bloody` whether the player races with weapons, which the red tracks need.
pub(super) fn roll(
    car: &mut Car,
    keys: u32,
    track: &mut Track,
    rand: &mut Rand,
    squeal: Option<&mut i32>,
    bloody: bool,
) {
    let f = f64::from;
    let angle = f(car.angle);
    let (x, y) = (f(car.x), f(car.y));
    let wheel = |degrees: f64| {
        let r = degrees * RADIANS;
        [(sin(r) * 12.0 + x) as f32, (cos(r) * 9.999_996 + y) as f32]
    };
    car.wheels = [
        wheel((angle + 180.0) - 22.0),
        wheel(angle + 202.0),
        wheel(angle - 22.0),
        wheel(angle + 22.0),
    ];
    let speed = f(car.speed);
    let engine = f(car.handling.engine);
    let pulling = speed > 0.0 && speed < engine * 0.55 && keys & ACCELERATE != 0;
    if car.handling.damage > 0 && pulling {
        // The wheels spin: the car shakes by up to half a degree either way.
        let shake = |rand: &mut Rand| f64::from(rand.next()) * (1.0 / 65536.0);
        let spin = shake(rand) + f(car.spin);
        car.spin = spin as f32;
        car.spin = (spin - shake(rand)) as f32;
    }
    if car.bloody > 0 && bloody {
        car.bloody -= 1;
        let (steps, step) = steps(car);
        for _ in 0..steps.max(0) {
            for trail in car.trail {
                track.mark(trail, track.blood);
            }
            car.advance_trail(step);
        }
    } else {
        let slide = f(car.slide).abs();
        let threshold = f(car.handling.car as f32 + 13.0);
        let skids = slide > threshold || (speed > 0.0 && keys & BRAKE != 0) || pulling;
        if skids && !(keys & BRAKE != 0 && keys & MINE != 0) {
            if let Some(squeal) = squeal {
                if slide > threshold {
                    *squeal = ftol(slide * 2048.0);
                }
                *squeal = (*squeal).min(LOUDEST);
                let racing = speed < engine * 0.85 && keys & ACCELERATE != 0;
                if speed > 0.0 && (keys & BRAKE != 0 || racing) {
                    *squeal = LOUDEST;
                }
            }
            let (steps, step) = steps(car);
            car.skids += 1;
            if car.skids == PUFF_EVERY {
                car.puff();
                car.skids = 0;
            }
            // The front wheels leave no marks while the car pulls away.
            let front = speed < 0.0 || engine * 0.85 < speed || keys & ACCELERATE == 0;
            for _ in 0..steps.max(0) {
                if front {
                    track.mark(car.trail[0], track.skid);
                    track.mark(car.trail[1], track.skid);
                }
                track.mark(car.trail[2], track.skid);
                track.mark(car.trail[3], track.skid);
                car.advance_trail(step);
            }
        }
    }
    car.trail = car.wheels;
}

/// The marks' steps this tick, one a pixel of speed, and each wheel's move a step.
fn steps(car: &Car) -> (i32, [[f32; 2]; 4]) {
    let steps = ftol(f64::from(car.speed));
    let step = std::array::from_fn(|w| {
        std::array::from_fn(|axis| {
            ((f64::from(car.wheels[w][axis]) - f64::from(car.trail[w][axis])) / f64::from(steps))
                as f32
        })
    });
    (steps, step)
}

impl Car {
    fn advance_trail(&mut self, step: [[f32; 2]; 4]) {
        for (trail, step) in self.trail.iter_mut().zip(step) {
            for axis in 0..2 {
                trail[axis] = (f64::from(step[axis]) + f64::from(trail[axis])) as f32;
            }
        }
    }

    /// A puff off each rear wheel, in the first slot where either side has room.
    fn puff(&mut self) {
        for slot in 0..PUFFS {
            let mut placed = false;
            for (side, wheel) in [(0, 2), (1, 3)] {
                if self.puffs[side][slot][0] == 0 {
                    let [x, y] = self.wheels[wheel];
                    self.puffs[side][slot] = [1, nearest(x), nearest(y)];
                    placed = true;
                }
            }
            if placed {
                return;
            }
        }
    }
}

/// `sub_40F070`: a car's puffs drawn where the view shows them whole, each a picture by its
/// age, then aged by the ticks between frames.
pub(super) fn draw_puffs(
    buffer: &mut Buffer,
    car: &mut Car,
    pictures: &[u8],
    (camera_x, camera_y): (i32, i32),
    left: i32,
    between: i32,
) {
    for slot in 0..PUFFS {
        for side in 0..2 {
            let [age, x, y] = car.puffs[side][slot];
            if age <= 0 {
                continue;
            }
            let column = x - camera_x + left;
            let row = y - camera_y;
            if column >= 4 && column + 4 < 320 && row >= 4 && row + 4 < 200 {
                let picture = match age {
                    ..=4 => 0,
                    5..=8 => 1,
                    _ => 2,
                } * PUFF_SIDE
                    * PUFF_SIDE;
                let at = i64::from(row) * STRIDE as i64 + i64::from(column) - 0x7A4;
                let picture = pictures.get(picture..).unwrap_or(&[]);
                buffer.draw(picture, PUFF_SIDE, PUFF_SIDE, at);
            }
            let age = age + between;
            car.puffs[side][slot][0] = if age >= PUFF_AGE { 0 } else { age };
        }
    }
}
