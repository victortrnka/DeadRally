//! The pedestrians (`drawShotPedestrian` 0x4111F0): up to 20 people standing on the track
//! where its `INF.BIN` puts them, stepping every 5 ticks of the timer and turned towards the
//! player's car, drawn after the track and before the cars; the run-over ones as a splat.

use deadrally_gamedata::track::TrackInfo;

use super::buffer::{Buffer, LEFT, STRIDE};

/// A sprite's side; three steps for each of four directions for each kind of pedestrian.
const SIDE: usize = 16;
const FRAME: usize = SIDE * SIDE;
/// The ticks between steps, and the splat's last frame.
const STEP_TICKS: u32 = 5;
const LAST_SPLAT: i32 = 7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Pedestrian {
    x: i32,
    y: i32,
    kind: i32,
    /// Towards the player's car: 0 down right, 1 down left, 2 up left, 3 up right of it.
    facing: i32,
    frame: i32,
    /// Stepping back down the frames (0x479AAC).
    back: bool,
    dead: bool,
    /// The timer's tick at the last step.
    stepped: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pedestrians {
    people: Vec<Pedestrian>,
    sprites: Vec<u8>,
    splats: [Vec<u8>; 2],
}

impl Pedestrians {
    /// The track's pedestrians (0x409DED: x, y, kind and facing from `INF.BIN`), with
    /// `PEDESTR.BPK`'s sprites and the splats `SPLAT3.BPK` and `SPLAT4.BPK`.
    pub(crate) fn new(info: &TrackInfo, sprites: Vec<u8>, splats: [Vec<u8>; 2]) -> Pedestrians {
        let people = info
            .pedestrians
            .iter()
            .map(|&[x, y, kind, facing]| Pedestrian {
                x,
                y,
                kind,
                facing,
                ..Pedestrian::default()
            })
            .collect();
        Pedestrians {
            people,
            sprites,
            splats,
        }
    }

    /// A frame of them at timer tick `now`, the player's car at `car`, the view's top left at
    /// `camera` on the track and `left` from the screen's left.
    pub(crate) fn draw(
        &mut self,
        buffer: &mut Buffer,
        now: u32,
        car: (f32, f32),
        (camera_x, camera_y): (i32, i32),
        left: i32,
    ) {
        let (car_x, car_y) = (car.0 as i32, car.1 as i32);
        for person in &mut self.people {
            let due = now >= person.stepped.wrapping_add(STEP_TICKS);
            if person.dead {
                if due && person.frame < LAST_SPLAT {
                    person.frame += 1;
                    person.stepped = now;
                }
                continue;
            }
            if due {
                if !person.back {
                    if person.frame < 2 {
                        person.frame += 1;
                    } else {
                        person.back = true;
                    }
                } else if person.frame > 0 {
                    person.frame -= 1;
                } else {
                    person.back = false;
                }
                person.stepped = now;
            }
            let (left_of, right_of) = (car_x < person.x, car_x > person.x);
            let (above, below) = (car_y < person.y, car_y > person.y);
            if left_of && above {
                person.facing = 2;
            }
            if right_of && above {
                person.facing = 3;
            }
            if left_of && below {
                person.facing = 1;
            }
            if right_of && below {
                person.facing = 0;
            }
        }
        for person in &self.people {
            let (x, y) = (person.x, person.y);
            let near = x > camera_x - 16 && x < camera_x + 320 && y > camera_y - 16;
            if !near || y >= camera_y + 200 || x == 0 || y == 0 {
                continue;
            }
            let column = x - camera_x + left;
            let row = y - camera_y;
            // Cut at the top (rows skipped) or at the bottom (rows left).
            let (row, skip, rows) = if row < 0 {
                (0, -row, SIDE as i32 + row)
            } else if row > 183 {
                (row, 0, 200 - row)
            } else {
                (row, 0, SIDE as i32)
            };
            let sprites = if person.dead {
                &self.splats[usize::from(x % 2 != 0)]
            } else {
                &self.sprites
            };
            let frame = if person.dead {
                person.frame
            } else {
                (person.facing + 4 * person.kind) * 3 + person.frame
            };
            let from = usize::try_from(frame).unwrap_or(0) * FRAME + skip as usize * SIDE;
            let at = i64::from(row) * STRIDE as i64 + i64::from(column) + LEFT as i64;
            let picture = sprites.get(from..).unwrap_or(&[]);
            buffer.draw(picture, SIDE, rows.max(0) as usize, at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(x: i32, y: i32) -> Pedestrians {
        let mut info = TrackInfo {
            width: 1000,
            height: 1000,
            zones: 0,
            starts: [[0; 3]; 4],
            power_ups: [[0; 2]; 16],
            pedestrians: [[0; 4]; 20],
        };
        info.pedestrians[0] = [x, y, 0, 0];
        // Each 16x16 frame filled with its own number.
        let sprites = (0..12u8).flat_map(|frame| vec![frame + 1; FRAME]).collect();
        Pedestrians::new(&info, sprites, [vec![], vec![]])
    }

    fn shown(people: &mut Pedestrians, now: u32, car: (f32, f32)) -> u8 {
        let mut buffer = Buffer::default();
        people.draw(&mut buffer, now, car, (0, 0), 64);
        buffer.pixel(64 + 100, 100)
    }

    /// A pedestrian steps every 5 ticks, through frames 1, 2, back to 1 and 0 and on.
    #[test]
    fn a_pedestrian_steps_back_and_forth_every_5_ticks() {
        let mut people = one(100, 100);
        let car = (150.0, 150.0);
        let frames: Vec<u8> = [1000, 1004, 1005, 1010, 1015, 1020, 1025, 1030]
            .iter()
            .map(|&now| shown(&mut people, now, car) - 1)
            .collect();
        // Facing 0 (the car down right): frames 0 to 2.
        assert_eq!(frames, [1, 1, 2, 2, 1, 0, 0, 1]);
    }

    /// A pedestrian turns to the player's car: up left of it they face direction 2.
    #[test]
    fn a_pedestrian_faces_the_player_s_car() {
        let mut people = one(100, 100);
        assert_eq!(shown(&mut people, 1000, (50.0, 50.0)) - 1, 2 * 3 + 1);
    }
}
