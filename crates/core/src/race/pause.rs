//! The race's pause (`racePauseMenu` 0x4064A0, on Escape): over the frozen frame the box
//! asking whether to abort the race flies in, 4x4 tile by tile from random places along the
//! bottom, waits for Y, N or Enter, then flies apart and falls away.

use crate::campaign::Rand;

/// The box: 204x76 (`GEN-MES.BPK`), lines of text 8 rows apart from row 6 and column 6, and
/// where it lands on the screen beside no HUD (0x406175).
const BOX_WIDTH: usize = 204;
const BOX_HEIGHT: usize = 76;
const BOX_LEFT: i32 = 57;
const BOX_TOP: i32 = 64;
/// The screen, and the rows the box's flight redraws (64 on).
const WIDTH: usize = 320;
const PIXELS: usize = WIDTH * 200;
const BAND: usize = 64 * WIDTH;
/// The tiles: 51 columns of 19.
const TILE: usize = 4;
const TILES: usize = (BOX_WIDTH / TILE) * (BOX_HEIGHT / TILE);
/// A tile not yet started, and how many frames its flight takes.
const WAITING: u8 = 100;
const FLIGHT: u8 = 70;
/// At most 20 tiles start a frame; the flight in is done when 968 have landed.
const STARTS_A_FRAME: i32 = 20;
const LANDED: i32 = 968;
/// Where the tiles start: 196 rows down, at one of 100 random columns below 316.
const START_ROW: i32 = 196;
const START_COLUMNS: usize = 100;
/// The tiles' fall when the box flies apart (0x4A8AA0), 16.16 a frame.
const GRAVITY: i32 = 0x3333;
const RIGHT_WALL: i32 = 316 << 16;
/// The keys that end the pause (scancodes): Enter, the keypad's Enter, Y, N; and F1, which
/// ends it for the help.
const ENTER: u8 = 0x1C;
const PAD_ENTER: u8 = 0x9C;
const YES: u8 = 0x15;
const NO: u8 = 0x31;
const HELP: u8 = 0x3B;

/// `sub_43B290`: `(a << 16) / b`, 0 for a 0 divisor.
fn fixed_divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        0
    } else {
        ((i64::from(a) << 16) / i64::from(b)) as i32
    }
}

/// A 4x4 tile's 16.16 place and step a frame, its pixels and its age.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tile {
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    pixels: [u8; TILE * TILE],
    age: u8,
}

/// How the pause ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Abort,
    Resume,
    /// F1: back to the race, which opens its help.
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Flying in, then waiting; whether the sound has stopped, and the keys seen.
    In { quiet: bool, done: bool },
    /// Flying apart: how many tiles had started.
    Away { tiles: i32 },
}

/// Where a pass of the pause got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Waiting,
    /// The box starts flying apart: the keys held are forgotten.
    Leaving,
    Over(Answer),
}

/// What the pause asks of the race's sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sound {
    /// The box's flying sound (effect 17, then 21 flying apart) on channel 5.
    Play(u8),
    Stop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pause {
    /// The frozen frame (0x47926C), the screen being drawn (0x4A9EB4) and the one shown.
    background: Vec<u8>,
    work: Vec<u8>,
    shown: Vec<u8>,
    /// The box with its text (0x479690).
    picture: Vec<u8>,
    tiles: Vec<Tile>,
    /// Where tiles start (0x50E560) and the next to use (0x479268).
    starts: [i32; START_COLUMNS],
    next_start: usize,
    /// The next tile to start: its column and row in the box (0x503220, 0x464F18) and how many
    /// have started, four for each (0x481E08).
    column: i32,
    row: i32,
    started: i32,
    /// Landed this frame (0x479270), frames since the last (0x5034F0).
    landed: i32,
    steps: i32,
    /// Where the box lands across: half the HUD's width right of its place beside no HUD.
    left: i32,
    /// Y pressed (0x464F68 from -1 to 1), F1 pressed.
    abort: bool,
    help: bool,
    phase: Phase,
}

impl Pause {
    /// The pause over `frame` (the race's 320x200), the box `picture` (204x76, its text
    /// drawn), beside a HUD `left` pixels wide (0x456AA0), up to its first wait: 100 `rand()`
    /// calls for the tiles' starts.
    pub(crate) fn new(
        frame: &[u8],
        picture: Vec<u8>,
        left: i32,
        rand: &mut Rand,
    ) -> (Pause, Vec<Sound>) {
        let tile = Tile {
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            pixels: [0; TILE * TILE],
            age: WAITING,
        };
        let starts = std::array::from_fn(|_| rand.next() % 316);
        let mut pause = Pause {
            background: frame.to_vec(),
            work: frame.to_vec(),
            shown: frame.to_vec(),
            picture,
            tiles: vec![tile; TILES],
            starts,
            next_start: 0,
            column: 0,
            row: 0,
            started: 0,
            landed: 0,
            steps: 0,
            left: BOX_LEFT + (left >> 1),
            abort: false,
            help: false,
            phase: Phase::In {
                quiet: false,
                done: false,
            },
        };
        let mut sounds = vec![Sound::Stop, Sound::Play(17)];
        pause.look(&mut sounds);
        (pause, sounds)
    }

    /// The screen as shown.
    pub(crate) fn screen(&self) -> &[u8] {
        &self.shown
    }

    /// The keys, as the loop reads them before each wait (0x406657): the first 968 tiles in,
    /// the sound stops; Enter, Y or N ends the pause, Y aborting the race, F1 too.
    fn keys(&mut self, held: impl Fn(u8) -> bool, sounds: &mut Vec<Sound>) {
        let Phase::In { quiet, done } = &mut self.phase else {
            return;
        };
        if self.landed >= LANDED && !*quiet {
            *quiet = true;
            sounds.push(Sound::Stop);
        }
        if held(ENTER) || held(PAD_ENTER) || held(YES) || held(NO) {
            *done = true;
        }
        if held(HELP) {
            *done = true;
            self.help = true;
        }
        if held(YES) {
            self.abort = true;
        }
    }

    /// Runs from this wait to the next, the keys `held` now.
    pub(crate) fn wait(
        &mut self,
        held: impl Fn(u8) -> bool,
        rand: &mut Rand,
        sounds: &mut Vec<Sound>,
    ) -> Step {
        match self.phase {
            Phase::In { done, .. } => {
                // 0x4066D9: the band shown, the background restored under it, the tiles on.
                self.shown[BAND..].copy_from_slice(&self.work[BAND..]);
                self.work[BAND..].copy_from_slice(&self.background[BAND..]);
                self.fly_in();
                if done {
                    self.leave(rand, sounds);
                    return Step::Leaving;
                }
                self.keys(held, sounds);
                Step::Waiting
            }
            Phase::Away { tiles } => {
                self.shown.copy_from_slice(&self.work);
                self.work.copy_from_slice(&self.background);
                if self.fly_away() < tiles {
                    return Step::Waiting;
                }
                sounds.push(Sound::Stop);
                self.shown.copy_from_slice(&self.background);
                Step::Over(if self.abort {
                    Answer::Abort
                } else if self.help {
                    Answer::Help
                } else {
                    Answer::Resume
                })
            }
        }
    }

    /// The first keys, read before the first wait.
    fn look(&mut self, sounds: &mut Vec<Sound>) {
        self.keys(|_| false, sounds);
    }

    /// `sub_406100`: every tile flown on `steps` frames; up to 20 more started, each from
    /// its random start to its place in 70 frames; the landed drawn and counted.
    fn fly_in(&mut self) {
        let mut starting = 0;
        self.landed = 0;
        for index in 0..TILES {
            if self.tiles[index].age == WAITING {
                starting += 1;
                if self.started < 4 * TILES as i32 && starting < STARTS_A_FRAME {
                    self.start(index);
                }
                continue;
            }
            let tile = &mut self.tiles[index];
            let age = i32::from(tile.age) + self.steps;
            if age >= i32::from(FLIGHT) {
                if tile.age < FLIGHT {
                    let left = i32::from(FLIGHT - tile.age);
                    tile.x = tile.x.wrapping_add(left.wrapping_mul(tile.dx));
                    tile.y = tile.y.wrapping_add(left.wrapping_mul(tile.dy));
                }
                tile.age = FLIGHT;
            } else {
                tile.age = age as u8;
                if self.steps > 0 {
                    tile.x = tile.x.wrapping_add(tile.dx.wrapping_mul(self.steps));
                    tile.y = tile.y.wrapping_add(tile.dy.wrapping_mul(self.steps));
                }
                let tile = *tile;
                self.draw(&tile);
                if tile.age < FLIGHT {
                    continue;
                }
            }
            let tile = self.tiles[index];
            self.draw(&tile);
            self.landed += 1;
        }
        self.steps = 1;
    }

    /// Starts tile `index` with the box's next 4x4 (down each column, then the next column).
    fn start(&mut self, index: usize) {
        let mut pixels = [0; TILE * TILE];
        for (y, row) in pixels.chunks_mut(TILE).enumerate() {
            let at = (self.row as usize + y) * BOX_WIDTH + self.column as usize;
            row.copy_from_slice(self.picture.get(at..at + TILE).unwrap_or(&[0; TILE]));
        }
        let x = self.starts[self.next_start] << 16;
        let y = START_ROW << 16;
        let to_x = (self.left + self.column) << 16;
        let to_y = (self.row + BOX_TOP) << 16;
        self.tiles[index] = Tile {
            x,
            y,
            dx: fixed_divide(to_x - x, i32::from(FLIGHT) << 16),
            dy: fixed_divide(to_y - y, i32::from(FLIGHT) << 16),
            pixels,
            age: 0,
        };
        self.started += 4;
        self.row += 4;
        if self.row == BOX_HEIGHT as i32 {
            self.row = 0;
            self.column += 4;
        }
        self.next_start = (self.next_start + 1) % START_COLUMNS;
    }

    /// The flight apart set up (0x406757): the keys forgotten, every started tile given a
    /// throw up its column (`(i / -25)` from the box's middle outwards, plus 0 to 2) and a
    /// sideways push of -2 to 1, two `rand()` calls a tile.
    fn leave(&mut self, rand: &mut Rand, sounds: &mut Vec<Sound>) {
        sounds.push(Sound::Stop);
        let mut throw = [0i32; 204];
        for (i, slot) in throw.iter_mut().enumerate().take(102) {
            *slot = i as i32 / -25;
        }
        for i in 103..204 {
            throw[i] = throw[204 - i];
        }
        let (mut column, mut row) = (0usize, 0usize);
        let mut tiles = 0;
        for tile in &mut self.tiles {
            if tile.age != WAITING {
                tiles += 1;
            }
            tile.dy = (throw[column] + rand.next() % 3) << 16;
            tile.dx = ((rand.next() % 4) - 2) << 16;
            for (y, out) in tile.pixels.chunks_mut(TILE).enumerate() {
                let at = (row + y) * BOX_WIDTH + column;
                out.copy_from_slice(self.picture.get(at..at + TILE).unwrap_or(&[0; TILE]));
            }
            row += 4;
            if row == BOX_HEIGHT {
                row = 0;
                column += 4;
            }
        }
        sounds.push(Sound::Play(21));
        self.phase = Phase::Away { tiles };
    }

    /// `sub_406330`: every landed tile falling `steps` frames, bouncing off the screen's
    /// sides; the number below the bottom.
    fn fly_away(&mut self) -> i32 {
        let steps = self.steps;
        let mut gone = 0;
        for index in 0..TILES {
            let tile = &mut self.tiles[index];
            if tile.age > FLIGHT {
                continue;
            }
            if steps > 0 {
                tile.dy = tile.dy.wrapping_add(GRAVITY.wrapping_mul(steps));
                tile.x = tile.x.wrapping_add(tile.dx.wrapping_mul(steps));
                tile.y = tile.y.wrapping_add(tile.dy.wrapping_mul(steps));
            }
            if tile.x <= 0 {
                tile.x = 0;
                tile.dx = -tile.dx;
            }
            if tile.x >= RIGHT_WALL {
                tile.x = RIGHT_WALL;
                tile.dx = -tile.dx;
            }
            if (tile.y.wrapping_add(0x8000) >> 16) > START_ROW {
                tile.age = FLIGHT;
                gone += 1;
                continue;
            }
            let tile = *tile;
            self.draw(&tile);
        }
        gone
    }

    /// `sub_43AE80`: a tile drawn whole at its rounded place on the screen being drawn.
    fn draw(&mut self, tile: &Tile) {
        let x = tile.x.wrapping_add(0x8000) >> 16;
        let y = tile.y.wrapping_add(0x8000) >> 16;
        for (row, line) in tile.pixels.chunks(TILE).enumerate() {
            let at = (y + row as i32) * WIDTH as i32 + x;
            for (column, &pixel) in line.iter().enumerate() {
                if let Some(slot) = usize::try_from(at + column as i32)
                    .ok()
                    .filter(|&at| at < PIXELS)
                    .and_then(|at| self.work.get_mut(at))
                {
                    *slot = pixel;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paused(rand: &mut Rand) -> Pause {
        paused_at(rand, 64)
    }

    /// The pause with the HUD `left` pixels wide.
    fn paused_at(rand: &mut Rand, left: i32) -> Pause {
        let picture = (0..BOX_WIDTH * BOX_HEIGHT)
            .map(|i| (i % 7) as u8 + 1)
            .collect();
        Pause::new(&vec![0; PIXELS], picture, left, rand).0
    }

    /// Runs the pause with `key` held from pass `at` on; the passes it took and its answer.
    fn answer(key: u8, at: usize) -> (usize, Answer) {
        let mut rand = Rand::new(1);
        let mut pause = paused(&mut rand);
        let mut sounds = Vec::new();
        for pass in 0.. {
            let held = |code: u8| pass >= at && code == key;
            if let Step::Over(answer) = pause.wait(held, &mut rand, &mut sounds) {
                return (pass, answer);
            }
        }
        unreachable!()
    }

    /// The pause draws `rand()` 100 times for where its tiles start and twice a tile (969)
    /// as they fly apart: the races after it depend on the numbers that are left.
    #[test]
    fn the_pause_draws_rand_as_the_original_does() {
        let mut rand = Rand::new(1);
        let mut pause = paused(&mut rand);
        let mut expected = Rand::new(1);
        for _ in 0..100 {
            expected.next();
        }
        assert_eq!(rand, expected);
        let mut sounds = Vec::new();
        pause.wait(|code| code == NO, &mut rand, &mut sounds);
        pause.wait(|_| false, &mut rand, &mut sounds);
        for _ in 0..2 * TILES {
            expected.next();
        }
        assert_eq!(rand, expected);
    }

    /// The box comes in 19 tiles a frame, each flying 70 frames: its last tile lands 121
    /// frames after the pause began.
    #[test]
    fn the_box_lands_tile_by_tile() {
        let mut rand = Rand::new(1);
        let mut pause = paused(&mut rand);
        let mut sounds = Vec::new();
        let mut passes = 0;
        while pause.landed < TILES as i32 {
            pause.wait(|_| false, &mut rand, &mut sounds);
            passes += 1;
        }
        assert_eq!(passes, 121);
        // Landed, the box shows where it belongs: its top left tile at (89, 64).
        assert_eq!(
            &pause.shown[64 * WIDTH + 89..64 * WIDTH + 93],
            &[1, 2, 3, 4]
        );
    }

    /// The box lands in the middle of the track's view: with the status bar slid away (no
    /// HUD) 32 pixels further left than beside the HUD.
    #[test]
    fn the_box_lands_in_the_middle_of_the_view() {
        let mut rand = Rand::new(1);
        let mut pause = paused_at(&mut rand, 0);
        let mut sounds = Vec::new();
        for _ in 0..121 {
            pause.wait(|_| false, &mut rand, &mut sounds);
        }
        assert_eq!(
            &pause.shown[64 * WIDTH + 57..64 * WIDTH + 61],
            &[1, 2, 3, 4]
        );
    }

    /// Y aborts the race, N goes back to it; either only after the box has flown apart.
    #[test]
    fn yes_aborts_and_no_resumes() {
        let (passes, aborted) = answer(YES, 5);
        assert_eq!(aborted, Answer::Abort);
        let (same, resumed) = answer(NO, 5);
        assert_eq!(resumed, Answer::Resume);
        assert_eq!(passes, same);
        assert!(passes > 10);
    }
}
