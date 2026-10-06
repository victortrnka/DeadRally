//! Cars running into each other (spec M4c), `sub_40CD10` once a logic tick after every car's
//! step: every car's push back and spin ease off; then for each pair of cars whose sprites
//! overlap (a pixel above colour 3 in both), each is pushed by 0.7 times how much faster the
//! other moved, knocked back on the axes it was moving into the other along, spun when they
//! meet head-on or side-on, and a car with spiked wheels tears into the other.

use super::driving::{Car, SIDE};
use super::raster::ftol;

/// What each logic tick leaves of a car's push back and spin (0x442168, 0x442160).
const PUSH_EASE: f64 = 0.869_565_217_391_304_4;
const SPIN_EASE: f64 = 0.833_333_333_333_333_4;
/// How far apart two cars can be and still touch.
const REACH: i32 = 39;
/// The push one car gives another, the spin and step of a head-on meeting, and the most
/// spin a side-on one gives.
const PUSH: f64 = 0.7;
const HEAD_ON_SPIN: f64 = 2.0;
const HEAD_ON_STEP: f64 = 0.6;
const MOST_SPIN: f64 = 3.0;
/// The most a spiked wheel takes off at once.
const MAX_HURT: i32 = 10_000;
/// A knock on an axis: the car moving into the other along it is put back where it was.
pub(super) const KNOCKED_BACK: i32 = 1;
const KNOCKED: i32 = 2;

/// `sub_40CD10` for every pair of cars; `spikes` whether each car has spiked wheels.
pub(super) fn collide(cars: &mut [Car], sprites: &[u8], spikes: &[bool]) {
    for car in cars.iter_mut() {
        car.push = car.push.map(|d| (f64::from(d) * PUSH_EASE) as f32);
        car.spin = (f64::from(car.spin) * SPIN_EASE) as f32;
    }
    let n = cars.len();
    for a in 0..n.saturating_sub(1) {
        for b in a + 1..n {
            let (front, back) = cars.split_at_mut(b);
            meet(&mut front[a], &mut back[0], sprites, spikes[a]);
        }
    }
}

/// Where two sprites overlap last, row by row: the pixel in `a`'s sprite and in `b`'s, each
/// from the sprite's middle.
fn overlap(a: &Car, b: &Car, sprites: &[u8]) -> Option<([i32; 2], [i32; 2])> {
    let dx = ftol(f64::from(b.x) - f64::from(a.x));
    let dy = ftol(f64::from(b.y) - f64::from(a.y));
    if dx >= REACH || dy >= REACH || dx <= -REACH || dy <= -REACH {
        return None;
    }
    let side = SIDE as i32;
    let pixel = |car: &Car, row: i32, column: i32| {
        sprites
            .get(car.sprite + (row * side + column) as usize)
            .copied()
            .unwrap_or(0)
    };
    let (columns, column_shift) = if dx >= 0 {
        (dx..side, -dx)
    } else {
        (0..dx + side, -dx)
    };
    let (rows, row_shift) = if dy >= 0 {
        (dy..side, -dy)
    } else {
        (0..dy + side, -dy)
    };
    let mut last = None;
    for row in rows {
        for column in columns.clone() {
            let (b_row, b_column) = (row + row_shift, column + column_shift);
            if pixel(a, row, column) > 3 && pixel(b, b_row, b_column) > 3 {
                last = Some(([column - 20, row - 20], [b_column - 20, b_row - 20]));
            }
        }
    }
    last
}

/// The knock on an axis for a car hit at `at` (from its middle) and stepping `step` along it.
fn knock(at: i32, step: f32) -> i32 {
    if (at > 0 && step > 0.0) || (at < 0 && step < 0.0) {
        KNOCKED_BACK
    } else {
        KNOCKED
    }
}

fn meet(a: &mut Car, b: &mut Car, sprites: &[u8], spiked: bool) {
    let f = f64::from;
    let Some((a_at, b_at)) = overlap(a, b, sprites) else {
        return;
    };
    let [a_column, a_row] = a_at;
    let [b_column, b_row] = b_at;
    a.hit_at = a_at;
    b.hit_at = b_at;
    let b_moved = [f(b.x) - f(b.previous[0]), f(b.y) - f(b.previous[1])];
    let i = f64::from;
    let a_turn = ftol(i(a_row) * b_moved[0] - i(a_column) * b_moved[1]);
    let a_moved_x = (f(a.x) - f(a.previous[0])) as f32;
    let a_moved_y = f(a.y) - f(a.previous[1]);
    let b_turn = ftol(i(b_row) * f(a_moved_x) - i(b_column) * a_moved_y);
    a.push = [
        ((b_moved[0] - f(a_moved_x)) * PUSH + f(a.push[0])) as f32,
        ((b_moved[1] - a_moved_y) * PUSH + f(a.push[1])) as f32,
    ];
    a.knocks = [knock(a_column, a.step[0]), knock(a_row, a.step[1])];
    let a_moved = [f(a.x) - f(a.previous[0]), f(a.y) - f(a.previous[1])];
    b.push = [
        ((a_moved[0] - b_moved[0]) * PUSH + f(b.push[0])) as f32,
        ((a_moved[1] - b_moved[1]) * PUSH + f(b.push[1])) as f32,
    ];
    b.knocks = [knock(b_column, b.step[0]), knock(b_row, b.step[1])];
    let head_on = (a.knocks[0] == KNOCKED_BACK && b.knocks[0] == KNOCKED_BACK)
        || (a.knocks[1] == KNOCKED_BACK && b.knocks[1] == KNOCKED_BACK);
    if head_on {
        let turn_a_back = a.angle > b.angle && (f(a.angle) - f(b.angle)).abs() < 100.0;
        let spin = if turn_a_back {
            -HEAD_ON_SPIN
        } else {
            HEAD_ON_SPIN
        };
        a.spin = (f(a.spin) + spin) as f32;
        b.spin = (f(b.spin) - spin) as f32;
        let apart = |a: f32, b: f32| {
            let step = if a >= b { HEAD_ON_STEP } else { -HEAD_ON_STEP };
            ((f(a) + step) as f32, (f(b) - step) as f32)
        };
        (a.x, b.x) = apart(a.x, b.x);
        (a.y, b.y) = apart(a.y, b.y);
    }
    let apart = (f(a.angle) - f(b.angle)) as f32;
    let apart = f(apart).abs();
    let side_on = apart < 315.0 && apart > 45.0 && (apart < 135.0 || apart > 225.0);
    if side_on {
        let spin = |turn: i32| f(((turn / 4) as f32).min(MOST_SPIN as f32));
        a.spin = (spin(a_turn) + f(a.spin)) as f32;
        b.spin = (spin(b_turn) + f(b.spin)) as f32;
    }
    if spiked && !b.finished {
        // `a`'s front corners against `b`'s sprite.
        let touches = |corner: [f32; 2]| {
            let dx = ftol(f(b.x)) - ftol(f(corner[0]) + f(a.x));
            let dy = ftol(f(b.y)) - ftol(f(corner[1]) + f(a.y));
            dx.abs() < 20
                && dy.abs() < 20
                && sprites
                    .get((b.sprite as i32 + dy * SIDE as i32 + dx + 820) as usize)
                    .is_some_and(|&pixel| pixel > 3)
        };
        if touches(a.corners[1]) || touches(a.corners[0]) {
            let [x, y] = b.push.map(f);
            let tear = 3 * (0x400 - b.handling.armour);
            let hurt = ftol((x * x + y * y) * f64::from(tear)).min(MAX_HURT);
            b.handling.damage = (b.handling.damage - hurt).max(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::race::driving::{FRAME, FRAMES, Handling};
    use deadrally_gamedata::handling::Guns;

    fn car(slot: usize, x: f32) -> Car {
        let handling = Handling {
            car: 0,
            engine: 2.5,
            engine_backup: 2.5,
            tires: 0.5,
            size: 9.0,
            steering: 2.5,
            damage: 102_400,
            armour: 300,
            rocket: 0,
            weapons_bar: 102_400,
            turbo: 102_400,
            rocket_used: false,
            mines: 0,
            money: 0,
            weapons: true,
            guns: Guns::default(),
        };
        Car::new((x, 100.0, 72), slot, handling, 0)
    }

    /// Solid 40x40 sprites for two cars.
    fn sprites() -> Vec<u8> {
        vec![5; 2 * FRAMES * FRAME]
    }

    /// Two cars driven into each other along x are each put back (knocked back on x), pushed
    /// by the other's speed and spun apart: a crash, not a pass through.
    #[test]
    fn cars_meeting_head_on_bounce_and_spin() {
        let mut cars = vec![car(0, 100.0), car(1, 130.0)];
        cars[0].previous = [98.0, 100.0, 270.0];
        cars[0].step = [2.0, 0.0];
        cars[1].previous = [132.0, 100.0, 270.0];
        cars[1].step = [-2.0, 0.0];
        collide(&mut cars, &sprites(), &[false, false]);
        assert_eq!(cars[0].knocks[0], KNOCKED_BACK);
        assert_eq!(cars[1].knocks[0], KNOCKED_BACK);
        assert!(cars[0].push[0] < 0.0 && cars[1].push[0] > 0.0, "{:?}", cars);
        assert_eq!((cars[0].spin, cars[1].spin), (2.0, -2.0));
        assert_eq!((cars[0].x, cars[1].x), (99.4, 130.6));
    }

    /// Cars further apart than their sprites reach do not touch; every push and spin eases.
    #[test]
    fn cars_apart_only_ease() {
        let mut cars = vec![car(0, 100.0), car(1, 140.0)];
        cars[0].spin = 1.2;
        cars[0].push = [2.3, 0.0];
        collide(&mut cars, &sprites(), &[false, false]);
        assert_eq!(cars[0].knocks, [0, 0]);
        assert_eq!(cars[0].spin, (1.2f32 as f64 * SPIN_EASE) as f32);
        assert_eq!(cars[0].push[0], (2.3f32 as f64 * PUSH_EASE) as f32);
    }

    /// Spiked wheels on the first car tear into the second it rams, by the push it gets.
    #[test]
    fn spiked_wheels_tear_into_the_car_rammed() {
        let mut cars = vec![car(0, 100.0), car(1, 125.0)];
        cars[0].previous = [95.0, 100.0, 270.0];
        cars[0].step = [5.0, 0.0];
        cars[0].corners[1] = [18.0, 0.0];
        collide(&mut cars, &sprites(), &[true, false]);
        assert!(cars[1].handling.damage < 102_400);
        let mut plain = vec![car(0, 100.0), car(1, 125.0)];
        plain[0].previous = [95.0, 100.0, 270.0];
        plain[0].step = [5.0, 0.0];
        plain[0].corners[1] = [18.0, 0.0];
        collide(&mut plain, &sprites(), &[false, false]);
        assert_eq!(plain[1].handling.damage, 102_400);
    }
}
