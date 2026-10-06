//! The cars on the track (`drawCarInRace` 0x40D920): first every car's headlights, three
//! triangles ahead of it that turn the track's colours through its `-LIT.TAB`
//! (`sub_43D530`), then the sprites (`sub_43AEC0`), the player's first and the others over it.

use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::race::RaceError;

use crate::trig::{cos, sin};

use super::buffer::{Buffer, LEFT, STRIDE};
use super::driving::{Car, RADIANS};
use super::raster::nearest;

/// The cars' sprite files in `ENGINE.BPA` by car (0x445038), `-S` with spikes.
const NAMES: [&str; 7] = [
    "KUPLA", "PICKUP", "SEDAN", "CAMARO", "PORCHE", "LOTUS", "SPECIAL",
];
const SPECIAL: usize = 6;
/// A sprite's side, and a car's sprites: one every 3.75 degrees.
const SIDE: usize = 40;
pub(crate) const FRAME: usize = SIDE * SIDE;
pub(crate) const FRAMES: usize = 96;
const SPRITES: usize = FRAME * FRAMES;
/// The palette entries a car's colour has in its sprites, moved 10 along a driver.
const COLOUR: std::ops::RangeInclusive<u8> = 15..=24;
/// The rows the sprites draw in.
const ROWS: i64 = 200;
/// The rocket's flames' side (`ROCKET1.BPK`, `ROCKET2.BPK`: 24 directions), and the timer's
/// ticks between their turns.
const FLAME: usize = 16;
const FLAME_TICKS: u32 = 4;

/// `sub_403050`: every driver's sprites one after another (0x5034FC), their colour's entries
/// moved to the driver's own; on a reversed circuit turned half round (0x40AB40).
pub(crate) fn sprites(
    engine: &Archive,
    cars: &[(usize, bool)],
    reversed: bool,
) -> Result<Vec<u8>, RaceError> {
    let mut all = vec![0; cars.len() * SPRITES];
    for (slot, &(car, spikes)) in cars.iter().enumerate() {
        let suffix = if spikes && car < SPECIAL {
            "-S.BPK"
        } else {
            ".BPK"
        };
        let name = format!("{}{suffix}", NAMES[car]);
        let decoded = super::hud::decoded(engine, &name)?;
        let own = &mut all[slot * SPRITES..(slot + 1) * SPRITES];
        let len = decoded.len().min(SPRITES);
        own[..len].copy_from_slice(&decoded[..len]);
        recolour(own, slot);
        if reversed {
            turn_round(own);
        }
    }
    Ok(all)
}

/// A car's sprites turned half round, as `calculateCircuitReversed` turns them: all read
/// backwards, then each half's frames in the other order, so that the frame for a direction
/// holds the old frame for the opposite direction upside down.
fn turn_round(sprites: &mut [u8]) {
    sprites.reverse();
    for half in sprites.chunks_mut(SPRITES / 2) {
        let mut frames: Vec<Vec<u8>> = half.chunks(FRAME).map(<[u8]>::to_vec).collect();
        frames.reverse();
        half.copy_from_slice(&frames.concat());
    }
}

/// The car's colour entries in driver `slot`'s sprites moved to the driver's own ten.
fn recolour(sprites: &mut [u8], slot: usize) {
    for byte in sprites.iter_mut().filter(|byte| COLOUR.contains(byte)) {
        *byte += 10 * slot as u8;
    }
}

/// `sub_43AEC0`: the 40x40 sprite at `offset` in `sprites` centred on (`x`, `y`), its 0
/// bytes left out, rows outside the race's 200 left out whole.
pub(crate) fn draw_sprite(buffer: &mut Buffer, sprites: &[u8], (x, y): (i32, i32), offset: usize) {
    let mut at = i64::from(y - 20) * STRIDE as i64 + i64::from(x - 20) + LEFT as i64;
    for row in sprites[offset..offset + FRAME].chunks(SIDE) {
        if (0..ROWS * STRIDE as i64).contains(&at) {
            buffer.draw(row, SIDE, 1, at);
        }
        at += STRIDE as i64;
    }
}

/// The three triangles of a car's headlights at (`x`, `y`), the car turned `angle` degrees:
/// the middle one from 170 to 190 degrees behind its sprite's up, reaching 40 across and
/// 33.3 down, and one each side to 162 and 198 degrees, reaching 36 and 30 there.
pub(crate) fn headlights(buffer: &mut Buffer, (x, y): (i32, i32), angle: f32, table: &[u8; 256]) {
    const RADIANS: f64 = 0.017_453_292_519_944_444;
    let at = |degrees: f64| degrees * RADIANS;
    let a = f64::from(angle);
    let far = |turn: f64| {
        (
            x - (crate::trig::sin(turn) * -40.0) as i32,
            y - (crate::trig::cos(turn) * -33.333_32) as i32,
        )
    };
    let near = |turn: f64| {
        (
            x - (crate::trig::sin(turn) * -36.0) as i32,
            y - (crate::trig::cos(turn) * -29.999_988) as i32,
        )
    };
    let from_170 = at((a - 10.0) + 180.0);
    let to_190 = at(a + 190.0);
    super::raster::light_triangle(buffer, [(x, y), far(to_190), far(from_170)], table);
    super::raster::light_triangle(buffer, [(x, y), near(at(a + 198.0)), far(to_190)], table);
    let from_162 = at((a - 18.0) + 180.0);
    super::raster::light_triangle(buffer, [(x, y), far(from_170), near(from_162)], table);
}

/// `sub_40F450` for each car after the guns' flashes: a car whose rocket burned since the
/// last frame, with turbo left, shows the rocket's flame 2.3 car sizes behind it, both
/// offsets rounded half up; the flames' `phase` (0x456AFC), which all cars share, turns
/// when a flame is drawn 4 ticks of the timer `now` after the car's last turn. The burn is
/// forgotten every frame.
pub(super) fn draw_flame(
    buffer: &mut Buffer,
    car: &mut Car,
    flames: &[Vec<u8>; 2],
    phase: &mut usize,
    now: u32,
) {
    let h = &car.handling;
    if h.rocket_used && h.turbo > 0 && h.rocket != 0 {
        let r = (f64::from(car.angle) + 180.0) * RADIANS;
        let behind = f64::from(h.size) * -2.3;
        let bx = nearest(sin(r) * behind);
        let by = nearest(cos(r) * behind * 0.833_333);
        let x = car.screen[0] + bx - 8;
        let y = car.screen[1] + by - 8;
        let side = FLAME as i32;
        if x >= 0 && x + side < 320 && y >= 0 && y + side < 200 {
            let start = usize::try_from(car.direction / 4).unwrap_or(0) * FLAME * FLAME;
            let picture = flames[*phase].get(start..).unwrap_or(&[]);
            let at = i64::from(y) * STRIDE as i64 + i64::from(x) + LEFT as i64;
            buffer.draw(picture, FLAME, FLAME, at);
            if now >= car.flame_time.wrapping_add(FLAME_TICKS) {
                *phase = if *phase + 1 > 1 { 0 } else { *phase + 1 };
                car.flame_time = now;
            }
        }
    }
    car.handling.rocket_used = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a reversed circuit a car's frame for a direction shows the old frame for the
    /// opposite direction (48 steps round) turned upside down, so the cars still face the way
    /// they drive.
    #[test]
    fn reversed_sprites_face_the_other_way() {
        let mut sprites: Vec<u8> = (0..FRAMES)
            .flat_map(|frame| (0..FRAME).map(move |i| (frame * 7 + i % 5) as u8))
            .collect();
        let old = sprites.clone();
        turn_round(&mut sprites);
        for frame in [0, 10, 47, 48, 95] {
            let was = (frame + 48) % FRAMES;
            let mut expected = old[was * FRAME..(was + 1) * FRAME].to_vec();
            expected.reverse();
            assert_eq!(
                &sprites[frame * FRAME..(frame + 1) * FRAME],
                &expected[..],
                "{frame}"
            );
        }
    }

    /// Each driver's car shows in their own colour: the sprites' ten colour entries move ten
    /// along a place on the grid, and nothing else in them moves.
    #[test]
    fn a_drivers_sprites_take_their_own_colour_entries() {
        let mut sprites = vec![14, 15, 24, 25, 0];
        recolour(&mut sprites, 2);
        assert_eq!(sprites, [14, 35, 44, 25, 0]);
    }

    /// A car half off the top of the view shows its lower rows, and its sprite's 0 bytes let
    /// the track show through.
    #[test]
    fn a_car_off_the_top_shows_its_lower_rows() {
        let mut sprites = vec![7; FRAME];
        sprites[39 * SIDE + 20] = 0;
        let mut buffer = Buffer::default();
        draw_sprite(&mut buffer, &sprites, (100, 5), 0);
        // Its rows 15 to 39 show on rows 0 to 24, columns 80 to 119.
        assert_eq!(buffer.pixel(80, 0), 7);
        assert_eq!(buffer.pixel(79, 0), 0);
        assert_eq!(buffer.pixel(101, 24), 7);
        assert_eq!(buffer.pixel(100, 24), 0);
        assert_eq!(buffer.pixel(101, 25), 0);
    }

    fn rocket_car() -> super::super::driving::Car {
        let handling = super::super::driving::Handling {
            car: 1,
            engine: 2.5,
            engine_backup: 2.5,
            tires: 0.5,
            size: 9.0,
            steering: 2.5,
            damage: 0x1_0000,
            armour: 400,
            rocket: 1,
            weapons_bar: 102_400,
            turbo: 102_400,
            rocket_used: true,
            mines: 0,
            money: 0,
            weapons: true,
            guns: Default::default(),
        };
        let mut car = super::super::driving::Car::new((300.0, 200.0, 0), 0, handling, 0);
        car.screen = [150, 100];
        car
    }

    /// A car whose rocket burned this frame shows its flame behind it (facing angle 0, 17
    /// pixels down): the first picture, then the second once 4 ticks of the timer have passed
    /// since the last turn, a phase all cars share; the burn is then forgotten, so a car
    /// that lets go of the turbo shows no flame.
    #[test]
    fn a_burning_rocket_shows_its_flame_and_the_flames_take_turns() {
        let flames = [vec![3; FLAME * FLAME * 24], vec![4; FLAME * FLAME * 24]];
        let mut car = rocket_car();
        let mut phase = 0;
        let mut buffer = Buffer::default();
        draw_flame(&mut buffer, &mut car, &flames, &mut phase, 100);
        assert_eq!(buffer.pixel(142, 109), 3);
        assert_eq!(buffer.pixel(141, 109), 0);
        assert_eq!(buffer.pixel(142, 108), 0);
        assert_eq!((phase, car.flame_time), (1, 100));
        assert!(!car.handling.rocket_used);

        let mut buffer = Buffer::default();
        draw_flame(&mut buffer, &mut car, &flames, &mut phase, 102);
        assert_eq!(buffer.pixel(142, 109), 0);
        car.handling.rocket_used = true;
        draw_flame(&mut buffer, &mut car, &flames, &mut phase, 103);
        assert_eq!(buffer.pixel(142, 109), 4);
        assert_eq!(phase, 1);
        car.handling.rocket_used = true;
        draw_flame(&mut buffer, &mut car, &flames, &mut phase, 104);
        assert_eq!((phase, car.flame_time), (0, 104));
    }
}
