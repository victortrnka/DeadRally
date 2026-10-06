//! The opponents' driving (spec M5, `calculateIAMovements` 0x40AFC0, for each opponent each
//! logic tick before the cars move): the keys an opponent holds and the steering it turns
//! with, from the track's guide (`-LR1.BPK`), its zone's tables (`-DRV.DAT`, `-OHI.DAT`),
//! what lies ahead of it, and `rand()`.

use crate::campaign::Rand;
use crate::trig::{cos, sin};

use super::driving::{ACCELERATE, BRAKE, Car, LEFT, MINE, RADIANS, RIGHT, TURBO};
use super::guns::GUN;
use super::raster::ftol;

/// The horn: the brake and the mine together.
const HORN: u32 = BRAKE | MINE;
/// How near (in pixels each way) a car or a mine counts as there.
const NEAR: i32 = 20;
/// The guide's value on the line, and how far off it an opponent lets itself go either way.
const LINE: u8 = 16;
const RIGHT_OF: u8 = 9;
const LEFT_OF: u8 = 7;
/// The ticks an opponent gets round a car ahead (0x4A7E88), a mine ahead, or one it is knocked
/// about by; the ticks it stops accelerating once stuck at a wall (0x4A7E84); and the ticks
/// between its mines (0x4A7EA4).
const AROUND_CAR: i32 = 100;
const AROUND_MINE: i32 = 60;
const BACK_OFF: i32 = 100;
const MINE_WAIT: i32 = 350;
/// The avoiding tick at which it may sound its horn, one time in five.
const HORN_TICK: i32 = 70;
/// The counter at 0x4A7E80 past which an opponent turns on its own (never set in the original).
const TURNING: i32 = 100;
const TURN_BRAKE: i32 = 148;

/// What an opponent's driving keeps from tick to tick: the counter past which it turns on its
/// own (0x4A7E80), the ticks it holds off the accelerator (0x4A7E84), gets round something
/// (0x4A7E88) and waits for its next mine (0x4A7EA4), and whether it sounds its horn
/// (0x4A805C).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Memory {
    pub(super) turning: i32,
    pub(super) back_off: i32,
    pub(super) avoid: i32,
    pub(super) mine_wait: i32,
    pub(super) horn: bool,
}

/// What an opponent drives by: the track's guide and zones (a byte for each 4x4 pixels, each
/// `width` wide), the zones' tables, the track's size, the mines dropped, and the cars'
/// sprites.
pub(super) struct Guide<'a> {
    pub(super) guide: &'a [u8],
    pub(super) zones: &'a [u8],
    pub(super) width: i32,
    pub(super) speed: &'a [f32],
    pub(super) steering: &'a [f32],
    pub(super) offset: &'a [i32],
    pub(super) track: (i32, i32),
    pub(super) mines: &'a [(i32, i32)],
    pub(super) sprites: &'a [u8],
}

impl Guide<'_> {
    /// The guide under the track's pixel (`x`, `y`), kept on the track.
    fn under(&self, x: i32, y: i32) -> u8 {
        let x = x.min(self.track.0 - 1).max(0);
        let y = y.min(self.track.1 - 1).max(0);
        let at = (y >> 2).wrapping_mul(self.width).wrapping_add(x >> 2);
        usize::try_from(at)
            .ok()
            .and_then(|at| self.guide.get(at))
            .copied()
            .unwrap_or(0)
    }

    /// The zone under the car.
    fn zone(&self, car: &Car) -> usize {
        let at = (ftol(f64::from(car.y)) >> 2)
            .wrapping_mul(self.width)
            .wrapping_add(ftol(f64::from(car.x)) >> 2);
        usize::from(
            usize::try_from(at)
                .ok()
                .and_then(|at| self.zones.get(at))
                .copied()
                .unwrap_or(0),
        )
    }
}

/// The track's pixel `reach` along the car's heading turned by `degrees` (180 ahead), the
/// angle summed in the x87's precision.
fn point(car: &Car, degrees: f64, reach: f64) -> (i32, i32) {
    let r = (f64::from(car.angle) + degrees) * RADIANS;
    (
        ftol(sin(r) * reach + f64::from(car.x)),
        ftol(cos(r) * reach + f64::from(car.y)),
    )
}

/// Whether another car's middle lies within 20 pixels of (`x`, `y`) each way.
fn car_near(
    cars: &[Car],
    slot: usize,
    (x, y): (i32, i32),
) -> impl Iterator<Item = (usize, i32, i32)> + '_ {
    cars.iter().enumerate().filter_map(move |(other, car)| {
        let dx = x.wrapping_sub(ftol(f64::from(car.x)));
        let dy = y.wrapping_sub(ftol(f64::from(car.y)));
        (other != slot && dx.abs() < NEAR && dy.abs() < NEAR).then_some((other, dx, dy))
    })
}

/// Whether the effect power-up's count `effect` is in one of the windows that turn the car
/// (the 30 ticks before each of `first`, `first - 60`, ... down to `last`).
fn wobbles(effect: i32, first: i32, last: i32) -> bool {
    let mut end = first;
    let mut inside = false;
    loop {
        if effect > end - 30 && effect < end {
            inside = true;
        }
        end -= 60;
        if end < last {
            return inside;
        }
    }
}

/// `calculateIAMovements` for the opponent in place `slot` in tick `tick` of the pass.
pub(super) fn steer(cars: &mut [Car], slot: usize, tick: usize, guide: &Guide, rand: &mut Rand) {
    let f = f64::from;
    // A car ahead: get round it.
    let ahead = point(&cars[slot], 180.0, 35.0);
    let blocked = car_near(cars, slot, ahead).any(|(other, dx, dy)| {
        let at = cars[other].sprite as i64 + i64::from(dy) * 40 + i64::from(dx) + 0x334;
        usize::try_from(at)
            .ok()
            .and_then(|at| guide.sprites.get(at))
            .is_some_and(|&pixel| pixel > 3)
    });
    let car = &mut cars[slot];
    if blocked && car.ai.avoid == 0 && !car.finished {
        car.ai.avoid = AROUND_CAR;
    }
    // A mine ahead: likewise.
    let (mx, my) = point(car, 180.0, 25.0);
    for &(x, y) in guide.mines {
        if mx.wrapping_sub(x).abs() < NEAR
            && my.wrapping_sub(y).abs() < NEAR
            && car.ai.avoid == 0
            && !car.finished
        {
            car.ai.avoid = AROUND_MINE;
        }
    }
    if car.stuck > 5 && car.ai.back_off == 0 {
        car.ai.back_off = BACK_OFF;
    }
    if car.knocked > 3 && car.ai.avoid == 0 {
        car.ai.avoid = AROUND_CAR;
    }
    // The guide under two feelers 40 pixels out, 26 degrees either side of ahead.
    let (x, y) = (f(car.x), f(car.y));
    let feel = |r: f64| (ftol(sin(r) * 40.0 + x), ftol(cos(r) * 40.0 + y));
    let one = feel((f(car.angle) + 206.0) * RADIANS);
    let other = feel((f(car.angle) + 180.0 - 26.0) * RADIANS);
    let right = guide.under(one.0, one.1);
    let left = guide.under(other.0, other.1);
    let alive = !car.finished && car.handling.damage > 0;
    let mut keys = 0;
    if alive && wobbles(car.effect, 560, 60) {
        keys |= LEFT;
    }
    if alive && wobbles(car.effect, 530, 30) {
        keys |= RIGHT;
    }
    if car.ai.turning < TURNING {
        let zone = guide.zone(car);
        let (right_off, left_off) = (LINE.wrapping_sub(right), LINE.wrapping_sub(left));
        if car.handling.engine > 0.0 || f(car.speed) > 0.5 {
            if car.ai.avoid == 0 {
                if right_off > RIGHT_OF {
                    keys |= RIGHT;
                }
                if left_off < LEFT_OF {
                    keys |= LEFT;
                }
            } else {
                let offset = guide.offset.get(zone).copied().unwrap_or(0);
                if i32::from(right_off) > offset.wrapping_add(1) {
                    keys |= RIGHT;
                }
                if i32::from(left_off) < offset.wrapping_sub(1) {
                    keys |= LEFT;
                }
            }
        }
        if rand.next() % 2 == 0 && car.knocked > 3 && alive {
            keys |= RIGHT;
        }
        let speed = guide.speed.get(zone).copied().unwrap_or(0.0);
        if f(speed) * f(car.handling.engine) > f(car.speed) && car.ai.back_off == 0 {
            keys |= ACCELERATE;
        }
        let steering = guide.steering.get(zone).copied().unwrap_or(0.0);
        car.turn = (f(steering) * f(car.handling.steering)) as f32;
    } else {
        if car.ai.turning > TURN_BRAKE && f(car.speed) > 0.5 {
            keys |= BRAKE;
        }
        if car.ai.turning < TURN_BRAKE {
            keys |= LEFT;
        }
        if car.ai.back_off > 0 {
            keys = BRAKE;
        }
    }
    if car.ai.avoid > 0 {
        keys |= TURBO;
    }
    for counter in [&mut car.ai.turning, &mut car.ai.back_off, &mut car.ai.avoid] {
        if *counter > 0 {
            *counter -= 1;
        }
    }
    cars[slot].keys[tick] = keys;
    // A car close behind: a mine for it.
    let behind = point(&cars[slot], 0.0, 55.0);
    let followed: Vec<usize> = car_near(cars, slot, behind)
        .map(|(other, ..)| other)
        .collect();
    let car = &mut cars[slot];
    for _ in followed {
        if car.ai.mine_wait == 0 {
            car.keys[tick] |= MINE;
            car.ai.mine_wait = MINE_WAIT;
        }
    }
    if car.ai.mine_wait > 0 {
        car.ai.mine_wait -= 1;
    }
    // A car 20, 50 or 80 pixels ahead: the guns.
    for reach in [20.0, 50.0, 80.0] {
        let at = point(&cars[slot], 180.0, reach);
        if car_near(cars, slot, at).next().is_some() {
            cars[slot].keys[tick] |= GUN;
        }
    }
    // Now and then the horn while getting round a car.
    let car = &mut cars[slot];
    if car.ai.avoid == HORN_TICK && rand.next() % 5 == 0 {
        car.ai.horn = true;
    }
    if car.ai.avoid == 0 {
        car.ai.horn = false;
    }
    if car.ai.horn {
        car.keys[tick] |= HORN;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The effect power-up's windows: 30 ticks before 560, 500, ... 80 for the left; before
    /// 530, 470, ... 50 for the right.
    #[test]
    fn the_effect_turns_the_car_in_windows_of_30_ticks() {
        assert!(!wobbles(560, 560, 60));
        assert!(wobbles(559, 560, 60));
        assert!(!wobbles(530, 560, 60));
        assert!(wobbles(79, 560, 60));
        assert!(!wobbles(50, 560, 60));
        assert!(wobbles(529, 530, 30));
        assert!(wobbles(49, 530, 30));
        assert!(!wobbles(20, 530, 30));
    }
}
