//! The machine guns (spec M4c): `sub_40E180` for each car each logic tick while it holds the
//! gun key: a shot from the active gun's muzzle, spread by `rand()`, traced 130 pixels ahead in
//! steps of 5 until it hits a car (which loses its armour short of 1024 times the shooter's
//! car's factor), a wall, or a pedestrian; and the drawing: the muzzle's flash
//! (`sub_40E960`, which also moves on to the next gun every frame) and the sparks where shots
//! hit (`sub_40EBC0`), three frames each.

use deadrally_gamedata::handling::Guns;

use crate::campaign::Rand;
use crate::trig::{cos, sin};

use super::buffer::{Buffer, STRIDE};
use super::driving::{Car, RADIANS};
use super::pedestrians::Pedestrians;
use super::raster::ftol;

/// The gun key's bit, and the frame from which the guns fire.
pub(super) const GUN: u32 = 0x20;
const FIRST_FRAME: i32 = 430;
/// How far a shot reaches, in steps of 5, and what a shot takes off the weapons bar.
const REACH: i32 = 130;
const STEP: i32 = 5;
const BAR_PER_SHOT: i32 = 260;
/// The flashes' and the sparks' pictures: 8x8.
const SIDE: i32 = 8;

/// What a car's guns are doing (fields 0x4A7EAC to 0x4A7ED8 of the car).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Gunfire {
    /// The gun firing next (0x4A68E0), and whether a shot left it this frame (0x4A7EB8).
    pub(super) active: usize,
    pub(super) fired: bool,
    /// The last shot's muzzle on the track (0x4A7ED4).
    pub(super) muzzle: [i32; 2],
    /// The sparks: each stage's picture (1-based, 0 for none) and place, one frame apart
    /// (0x4A7EAC with 0x4A7EBC, 0x4A7EB0 with 0x4A7EC4, 0x4A7EB4 with 0x4A7ECC).
    pub(super) sparks: [(i32, [i32; 2]); 3],
}

/// The race's last hit and muzzle, which the original keeps in globals shared by every car's
/// shots (0x4A7CF0, 0x4AA3E4, 0x50350C, 0x4A6AD8): a shot that hits nothing leaves the last
/// hit where it was.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Shared {
    pub(super) hit: [i32; 2],
}

/// A coordinate rounded half up.
fn nearest(value: f64) -> i32 {
    let whole = ftol(value);
    if value - f64::from(whole) < 0.5 {
        whole
    } else {
        ftol(value + 1.0)
    }
}

/// What a shot asks the race to play: channel, effect, volume.
pub(super) type Sound = (usize, u8, u32);

/// `sub_40E180` for the car in place `shooter`; `ground` the track's mask and size,
/// `pedestrians` their corners and states (only in the single player race), `player` the
/// player's place on the grid.
#[allow(clippy::too_many_arguments)]
pub(super) fn fire(
    cars: &mut [Car],
    shooter: usize,
    keys: u32,
    frame: i32,
    sprites: &[u8],
    (mask, width, height): (&[u8], i32, i32),
    pedestrians: &mut Pedestrians,
    player: usize,
    shared: &mut Shared,
    damage_factors: &[f32],
    rand: &mut Rand,
) -> Vec<Sound> {
    let f = |value: f32| f64::from(value);
    let i = |value: i32| f64::from(value);
    let mut sounds = Vec::new();
    let car = &cars[shooter];
    let h = &car.handling;
    if h.damage <= 0
        || car.finished
        || frame <= FIRST_FRAME
        || keys & GUN == 0
        || !h.weapons
        || h.weapons_bar <= 0
    {
        return sounds;
    }
    let guns: Guns = h.guns.clone();
    let gun = cars[shooter].gunfire.active.min(1);
    let (x, y, angle) = (cars[shooter].x, cars[shooter].y, cars[shooter].angle);
    cars[shooter].gunfire.fired = true;
    let reach = i(guns.reach[gun]);
    let r = (i(guns.angle[gun]) + f(angle) + 180.0) * RADIANS;
    let mx = nearest(reach * sin(r));
    let my = nearest(cos(r) * reach * 0.833_333);
    let muzzle = [ftol((f(x) - 4.0) + i(mx)), ftol((f(y) - 4.0) + i(my))];
    if shooter == player {
        let effect = (rand.next() % 2 + 19) as u8;
        sounds.push((9, effect, 0x9000));
    } else if let Some(volume) = loudness(cars, shooter, player, 0x9000) {
        let effect = (rand.next() % 2 + 19) as u8;
        sounds.push((10, effect, volume));
    }
    let r = (f(angle) + 180.0) * RADIANS;
    let sx = nearest(sin(r) * 256.0) + rand.next() % 6 - 3;
    let sy = nearest(cos(r) * 213.333_248) + rand.next() % 6 - 3;
    let flash = guns.flash[gun];
    let shooter_x = ftol(f(x));
    let shooter_y = ftol(f(y));
    let mut step = 0;
    while step < REACH {
        let bx = (step * sx + 0x80) >> 8;
        let by = (step * sy + 0x80) >> 8;
        for target in 0..cars.len() {
            if target == shooter {
                continue;
            }
            let t = &cars[target];
            let dx = ftol(f(t.x)) - shooter_x - bx - mx;
            let dy = ftol(f(t.y)) - shooter_y - by - my;
            if dx.abs() >= 20 || dy.abs() >= 20 {
                continue;
            }
            let over = usize::try_from(t.sprite as i32 + dy * 40 + dx + 0x334)
                .ok()
                .and_then(|at| sprites.get(at))
                .is_some_and(|&pixel| pixel > 3);
            if !over {
                continue;
            }
            step = REACH;
            let factor = damage_factors
                .get(cars[shooter].handling.car)
                .copied()
                .unwrap_or(0.0);
            let t = &mut cars[target];
            if !t.finished {
                let loss = i(0x400 - t.handling.armour) * f(factor);
                t.handling.damage = ftol(i(t.handling.damage) - loss);
            }
            t.handling.damage = t.handling.damage.max(0);
            shared.hit = [muzzle[0] + bx, muzzle[1] + by];
            cars[shooter].gunfire.sparks[0].0 = flash * 3 + 1;
        }
        let tx = shooter_x + bx + mx;
        let ty = shooter_y + by + my;
        if ty >= 0 && ty < height && tx >= 0 && tx < width {
            let kind = mask
                .get((ty * width + tx) as usize)
                .map_or(15, |&byte| byte & 15);
            if kind < 4 {
                shared.hit = [muzzle[0] + bx, muzzle[1] + by];
                step = REACH;
                cars[shooter].gunfire.sparks[0].0 = flash * 3 + 1;
            }
        }
        // A shot meets every fifth pedestrian from a random one.
        let first = (rand.next() % 5) as usize;
        let killed = pedestrians.shot(first, (shooter_x + bx + mx, shooter_y + by + my));
        for _ in 0..killed {
            let volume = if shooter == player {
                Some(0x9000)
            } else {
                loudness(cars, shooter, player, 0x1_0000)
            };
            if let Some(volume) = volume {
                sounds.push((3, (rand.next() % 3 + 7) as u8, volume));
            }
        }
        step += STEP;
    }
    let h = &mut cars[shooter].handling;
    h.weapons_bar = (h.weapons_bar - BAR_PER_SHOT).max(0);
    let gunfire = &mut cars[shooter].gunfire;
    gunfire.sparks[0].1 = shared.hit;
    gunfire.muzzle = muzzle;
    sounds
}

/// The volume of a sound at car `from` as the player hears it: `loudest` less 75 for each
/// pixel to the player's car, nothing at 0x1000 or less.
fn loudness(cars: &[Car], from: usize, player: usize, loudest: i32) -> Option<u32> {
    let (a, b) = (&cars[from], &cars[player]);
    let dx = ftol(f64::from(a.x) - f64::from(b.x));
    let dy = ftol(f64::from(a.y) - f64::from(b.y));
    let distance = ftol(f64::from(dx * dx + dy * dy).sqrt());
    let volume = loudest - 75 * distance;
    (volume > 0x1000).then_some(volume as u32)
}

/// An 8x8 picture at (`x`, `y`) on the track, drawn where the view shows it whole.
fn put(buffer: &mut Buffer, picture: &[u8], [x, y]: [i32; 2], camera: (i32, i32), left: i32) {
    let column = x - camera.0 + left;
    let row = y - camera.1;
    if column >= 0 && column + SIDE < 320 && row >= 0 && row + SIDE < 200 {
        let at = i64::from(row) * STRIDE as i64 + i64::from(column) + 0x60;
        buffer.draw(picture, SIDE as usize, SIDE as usize, at);
    }
}

/// `sub_40E960`: the car's muzzle flash where its last shot left, the flash's kind by the
/// gun, turned to the car's direction; then the next gun is the active one.
pub(super) fn draw_flash(
    buffer: &mut Buffer,
    car: &mut Car,
    flashes: &[Vec<u8>],
    camera: (i32, i32),
    left: i32,
) {
    let guns = &car.handling.guns;
    if car.gunfire.fired {
        let kind = usize::try_from(guns.flash[car.gunfire.active.min(1)]).unwrap_or(0);
        let start = usize::try_from(car.direction / 2).unwrap_or(0) * 64;
        if let Some(picture) = flashes.get(kind).and_then(|f| f.get(start..)) {
            put(buffer, picture, car.gunfire.muzzle, camera, left);
        }
        car.gunfire.fired = false;
    }
    car.gunfire.active += 1;
    if car.gunfire.active as i32 > guns.count - 1 {
        car.gunfire.active = 0;
    }
}

/// `sub_40EBC0`: the sparks of the car's hits, each stage drawn and handed on to the next
/// frame's stage with the next picture.
pub(super) fn draw_sparks(
    buffer: &mut Buffer,
    car: &mut Car,
    sparks: &[u8],
    camera: (i32, i32),
    left: i32,
) {
    let picture = |stage: i32| {
        let start = usize::try_from(stage * 64 - 64).unwrap_or(0);
        sparks.get(start..).unwrap_or(&[])
    };
    let stages = &mut car.gunfire.sparks;
    if stages[2].0 > 0 {
        put(buffer, picture(stages[2].0), stages[2].1, camera, left);
        stages[2].0 = 0;
    }
    if stages[1].0 > 0 {
        stages[2].1 = stages[1].1;
        put(buffer, picture(stages[1].0), stages[1].1, camera, left);
        stages[2].0 = stages[1].0 + 1;
        stages[1].0 = 0;
    }
    if stages[0].0 > 0 {
        stages[1].1 = stages[0].1;
        put(buffer, picture(stages[0].0), stages[0].1, camera, left);
        stages[1].0 = stages[0].0 + 1;
        stages[0].0 = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::race::driving::{FRAME, FRAMES, Handling};
    use deadrally_gamedata::track::TrackInfo;

    fn car(slot: usize, x: f32, guns: Guns) -> Car {
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
            mines: 0,
            money: 0,
            weapons: true,
            guns,
        };
        Car::new((x, 100.0, 72), slot, handling, 0)
    }

    fn pedestrians() -> Pedestrians {
        let info = TrackInfo {
            width: 400,
            height: 200,
            zones: 0,
            starts: [[0; 3]; 4],
            power_ups: [[0; 2]; 16],
            pedestrians: [[0; 4]; 20],
        };
        Pedestrians::new(&info, false, vec![], [vec![], vec![]])
    }

    /// A shot from a car facing right meets the car 60 pixels ahead: it loses its armour short
    /// of 1024 times the shooter's car's factor, the hit sparks, the bar goes down by 260; the
    /// guns take turns frame by frame.
    #[test]
    fn a_shot_hurts_the_car_ahead_and_the_guns_take_turns() {
        let guns = Guns {
            count: 2,
            angle: [16, -17],
            reach: [20, 20],
            flash: [3, 3],
        };
        let mut cars = vec![car(0, 100.0, guns.clone()), car(1, 160.0, guns)];
        let sprites = vec![5u8; 2 * FRAMES * FRAME];
        let mask = vec![15u8; 400 * 200];
        let mut shared = Shared::default();
        let mut rand = Rand::new(3);
        let sounds = fire(
            &mut cars,
            0,
            GUN,
            431,
            &sprites,
            (&mask, 400, 200),
            &mut pedestrians(),
            0,
            &mut shared,
            &[0.2, 0.35],
            &mut rand,
        );
        assert_eq!(sounds.len(), 1);
        let loss = (f64::from(0x400 - 400) * f64::from(0.35f32)) as i32;
        assert_eq!(cars[1].handling.damage, 0x1_0000 - loss - 1);
        assert_eq!(cars[0].handling.weapons_bar, 102_400 - 260);
        assert_eq!(cars[0].gunfire.sparks[0].0, 10);
        let mut buffer = Buffer::default();
        draw_flash(&mut buffer, &mut cars[0], &[], (0, 0), 64);
        assert_eq!(cars[0].gunfire.active, 1);
        draw_flash(&mut buffer, &mut cars[0], &[], (0, 0), 64);
        assert_eq!(cars[0].gunfire.active, 0);
        // Before the race's 431st frame the guns are quiet.
        let before = cars[1].handling.damage;
        fire(
            &mut cars,
            0,
            GUN,
            430,
            &sprites,
            (&mask, 400, 200),
            &mut pedestrians(),
            0,
            &mut shared,
            &[0.2, 0.35],
            &mut rand,
        );
        assert_eq!(cars[1].handling.damage, before);
    }
}
