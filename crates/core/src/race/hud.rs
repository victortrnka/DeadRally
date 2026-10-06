//! The HUD at the race's left (`drawLeftRaceBar_414220`, spec M4 §3): the boards of the four
//! drivers, the player's speed gauge, turbo and weapons bars, damage and lap, the others'
//! laps and damage, everyone's place and name. DreeRally `race/leftBar.c`.

use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::bpk;
use deadrally_gamedata::race::RaceError;

use super::buffer::{Buffer, STRIDE};

/// The speed gauge's 162 marks (`sub_4022A0` 0x4022A0: degrees 11 to 172 on a circle of 26
/// across and 25 high, rounded as it rounds them in doubles).
const GAUGE_X: [i32; 162] = [
    5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 9, 9, 9, 9, 10, 10, 10,
    11, 11, 11, 11, 12, 12, 12, 13, 13, 13, 14, 14, 14, 15, 15, 16, 16, 16, 17, 17, 17, 18, 18, 19,
    19, 19, 20, 20, 21, 21, 22, 22, 22, 23, 23, 24, 24, 25, 25, 25, 26, 26, 27, 27, 28, 28, 29, 29,
    30, 30, 30, 30, 31, 31, 32, 32, 33, 33, 34, 34, 34, 35, 35, 36, 36, 37, 37, 38, 38, 38, 39, 39,
    40, 40, 40, 41, 41, 42, 42, 43, 43, 43, 44, 44, 44, 45, 45, 46, 46, 46, 47, 47, 47, 48, 48, 48,
    49, 49, 49, 50, 50, 50, 50, 51, 51, 51, 52, 52, 52, 52, 52, 53, 53, 53, 53, 54, 54, 54, 54, 54,
    54, 55, 55, 55, 55, 55, 55, 55, 55, 56, 56,
];
const GAUGE_Y: [i32; 162] = [
    23, 22, 22, 21, 21, 21, 20, 20, 19, 19, 19, 18, 18, 17, 17, 17, 16, 16, 15, 15, 15, 14, 14, 14,
    13, 13, 12, 12, 12, 11, 11, 11, 10, 10, 10, 10, 9, 9, 9, 8, 8, 8, 8, 7, 7, 7, 7, 6, 6, 6, 6, 5,
    5, 5, 5, 5, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7,
    8, 8, 8, 8, 9, 9, 9, 10, 10, 10, 10, 11, 11, 11, 12, 12, 12, 13, 13, 14, 14, 14, 15, 15, 15,
    16, 16, 17, 17, 17, 18, 18, 19, 19, 19, 20, 20, 21, 21, 21, 22, 22, 23, 23, 24, 24,
];
/// The boards: the player's 64 x 104 (with weapons, else the timer's), the others' 64 x 32.
const PLAYER_BOARD: usize = 6656;
const OTHER_BOARD: usize = 2048;
const BOARD_STRIDE: usize = 8704;
const NO_WEAPONS_BOARDS: usize = 34816;
/// The damage pictures: 6 of 64 x 21 a car, cars 8064 bytes apart.
const DAMAGE_PICTURES: usize = 8064;

/// The HUD's pictures (`loadRaceImagesHUD`, `IBFILES.BPA`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HudImages {
    /// The four boards one after the other, as the original keeps them.
    boards: Vec<u8>,
    damage: Vec<u8>,
    big_digits: Vec<u8>,
    pub(super) small_font: Vec<u8>,
    own_place: Vec<u8>,
    other_place: Vec<u8>,
    flag: Vec<u8>,
    mine: Vec<u8>,
    slider: Vec<u8>,
    wreck: Vec<u8>,
}

pub(super) fn decoded(archive: &Archive, name: &str) -> Result<Vec<u8>, RaceError> {
    bpk::decode(archive.read(name)?).map_err(|error| RaceError::Track {
        name: name.into(),
        error: deadrally_gamedata::track::TrackError::Bpk(error),
    })
}

fn slice(bytes: &[u8], start: usize, len: usize) -> Vec<u8> {
    let mut out = bytes.get(start..).unwrap_or(&[]).to_vec();
    out.resize(len, 0);
    out.truncate(len);
    out
}

impl HudImages {
    /// The pictures for the player in place `player` with car `car`, `weapons` on or off.
    pub(crate) fn load(
        ib_files: &Archive,
        player: usize,
        car: usize,
        weapons: bool,
    ) -> Result<HudImages, RaceError> {
        let boards = decoded(ib_files, "BOARDS.BPK")?;
        let first = if weapons {
            BOARD_STRIDE * player
        } else {
            PLAYER_BOARD * player + NO_WEAPONS_BOARDS
        };
        let mut all = slice(&boards, first, PLAYER_BOARD);
        let mut other = usize::from(player == 0);
        for _ in 0..3 {
            all.extend(slice(
                &boards,
                BOARD_STRIDE * other + PLAYER_BOARD,
                OTHER_BOARD,
            ));
            other += 1;
            if other == player {
                other += 1;
            }
        }
        const DAMAGE: [&str; 6] = [
            "DAM-KUP.BPK",
            "DAM-PIC.BPK",
            "DAM-SED.BPK",
            "DAM-CAM.BPK",
            "DAM-POR.BPK",
            "DAM-LOT.BPK",
        ];
        let damage = slice(
            &decoded(ib_files, DAMAGE[car.min(5)])?,
            DAMAGE_PICTURES * player,
            DAMAGE_PICTURES,
        );
        Ok(HudImages {
            boards: all,
            damage,
            big_digits: decoded(ib_files, "BIGNUM6.BPK")?,
            small_font: decoded(ib_files, "SMALFO4A.BPK")?,
            own_place: decoded(ib_files, "OWN-NUM1.BPK")?,
            other_place: decoded(ib_files, "OTH-NUM1.BPK")?,
            flag: decoded(ib_files, "GOALNUM2.BPK")?,
            mine: decoded(ib_files, "SIDEBOM1.BPK")?,
            slider: decoded(ib_files, "DAMSLID.BPK")?,
            wreck: decoded(ib_files, "RASTI1.BPK")?,
        })
    }
}

/// What the HUD shows of a driver, the player first, then the others in their order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Board {
    pub(crate) name: Vec<u8>,
    pub(crate) lap: i32,
    /// The picture of its place's medal, rolling from place to place ([`Medals`]).
    pub(crate) medal: usize,
    pub(crate) damage_bar: i32,
    pub(crate) finished: bool,
}

/// The player's bars and gauge.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Player {
    pub(crate) speed: f32,
    pub(crate) engine: f32,
    pub(crate) weapons: bool,
    pub(crate) weapons_bar: i32,
    pub(crate) turbo_bar: i32,
    pub(crate) mines: i32,
}

/// The boards' medals rolling from place to place (`drawLeftRaceBar_414220` from 0x4147B0):
/// each board's place at the last frame (0x46E8D0), the picture its medal stops at
/// (0x4AA3F0), where it is (16.16 pictures, 0x4A8A90) and how far it rolls a tick (16.16
/// places, 0x4A7CC0). A place's medal is picture 7 times the place less 7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Medals {
    last: Vec<i32>,
    end: Vec<i32>,
    at: Vec<i32>,
    step: Vec<i32>,
}

/// The first picture of a place's medal.
fn medal(place: i32) -> i32 {
    place.wrapping_mul(7).wrapping_sub(7)
}

/// The original's 16.16 rounding to whole pictures.
fn whole(at: i32) -> i32 {
    at.wrapping_add(0x8000) >> 16
}

impl Medals {
    /// Each medal standing at its board's place (`sub_4023C0` at the race's start).
    pub(crate) fn new(places: &[i32]) -> Medals {
        Medals {
            last: places.to_vec(),
            end: places.iter().map(|&place| medal(place)).collect(),
            at: places.iter().map(|&place| medal(place) << 16).collect(),
            step: vec![0; places.len()],
        }
    }

    /// A frame `between` ticks after the last: a board whose place changed starts rolling
    /// from its last place's medal; each rolls by its step for half of each tick
    /// (0x4AA500), not past its place's medal. The pictures to show.
    pub(crate) fn roll(&mut self, places: &[i32], between: i32) -> Vec<usize> {
        let speed = i64::from(between) << 15;
        let mut shown = Vec::with_capacity(places.len());
        for (index, &place) in places.iter().enumerate() {
            let last = self.last[index];
            if place != last {
                self.at[index] = medal(last) << 16;
                self.end[index] = medal(place);
                self.step[index] = (place - last) << 16;
            }
            let step = self.step[index];
            let at = self.at[index].wrapping_add(((i64::from(step) * speed) >> 16) as i32);
            let past = if step.wrapping_add(0x8000) & !0xFFFF > 0 {
                whole(at) > self.end[index]
            } else {
                whole(at) < self.end[index]
            };
            self.at[index] = if past { medal(place) << 16 } else { at };
            shown.push(usize::try_from(whole(self.at[index])).unwrap_or(0));
            self.last[index] = place;
        }
        shown
    }
}

/// `drawLeftRaceBar_414220` at `left` (`leftMenuInRaceWidth` 0x456AA0, 64 once slid in).
pub(crate) fn draw(
    buffer: &mut Buffer,
    images: &HudImages,
    left: i64,
    boards: &[Board],
    player: &Player,
    laps: i32,
) {
    let base = left + 32;
    for (row, line) in images.boards.chunks(64).take(201).enumerate() {
        buffer.copy(base + (row * STRIDE) as i64, line);
    }
    let digit = |n: i32| {
        let start = 80 * n.clamp(0, 10) as usize;
        images.big_digits.get(start..start + 80).unwrap_or(&[])
    };
    for (index, board) in boards.iter().enumerate() {
        if board.finished {
            let at = base - 32 + 39972 + 0x4000 * index as i64;
            buffer.draw(&images.flag, 22, 28, at);
        }
    }
    buffer.draw(digit(laps), 8, 10, left + 46133);
    for index in 1..boards.len() {
        buffer.draw(digit(laps), 8, 10, left + 46138 + 0x4000 * index as i64);
    }
    buffer.draw(digit(boards[0].lap), 8, 10, left + 46114);
    for (index, board) in boards.iter().enumerate().skip(1) {
        buffer.draw(
            digit(board.lap),
            8,
            10,
            left + 46116 + 0x4000 * index as i64,
        );
    }
    for (index, board) in boards.iter().enumerate().skip(1) {
        let full = 64.0 - (f64::from(board.damage_bar) * 0.0009765625).ceil() * 0.6369426751592356;
        let width = full as u8 as i64;
        let at = [53344i64, 69728, 86112][index - 1];
        for row in 0..9 {
            // Past full damage (only from a damaged save) the slider starts before its
            // picture, which the original reads garbage from; that part is left out here.
            let from = 64 - width + 64 * row;
            let line = usize::try_from(from)
                .ok()
                .and_then(|from| images.slider.get(from..from + width as usize))
                .unwrap_or(&[]);
            buffer.copy(left + at - width + STRIDE as i64 * row, line);
        }
    }
    let text = |buffer: &mut Buffer, at: i64, name: &[u8]| {
        for (index, &c) in name.iter().enumerate() {
            let start = 36 * usize::from(c.saturating_sub(32));
            let glyph = images.small_font.get(start..start + 36).unwrap_or(&[]);
            buffer.draw(glyph, 6, 6, at + 6 * index as i64);
        }
    };
    text(buffer, left + 1059, &boards[0].name);
    for (index, board) in boards.iter().enumerate().skip(1) {
        text(buffer, left + 37923 + 0x4000 * index as i64, &board.name);
    }
    let own = boards[0].medal * 1024;
    buffer.draw_opaque(
        images.own_place.get(own..own + 1024).unwrap_or(&[]),
        32,
        32,
        left + 36928,
    );
    for (index, board) in boards.iter().enumerate().skip(1) {
        let start = board.medal * 576;
        buffer.draw_opaque(
            images.other_place.get(start..start + 576).unwrap_or(&[]),
            24,
            24,
            left + 41032 + 0x4000 * index as i64,
        );
    }
    let needle = (f64::from(player.speed) / f64::from(player.engine) * -162.0) as i64;
    let first = if 1 - needle < 1 { 1 } else { 1 - needle };
    for mark in first.max(0) as usize..162 {
        let (x, y) = (GAUGE_X[mark], GAUGE_Y[mark]);
        buffer.slant(left + 32 + ((y as i64) << 9) + x as i64, x, 32, 33 - y, 0);
    }
    if player.weapons {
        let used = (f64::from(player.weapons_bar) * 0.000537109375) as i64;
        let used = if used < 0 { 0 } else { used as u8 as i64 };
        buffer.fill(left + used + 24100, 55 - used, 3, 0);
        for mine in 0..player.mines.max(0) as i64 {
            buffer.draw(&images.mine, 8, 6, left + 21024 + 8 * mine);
        }
    }
    let turbo = (f64::from(player.turbo_bar) * 0.000556640625) as i64;
    let turbo = if turbo < 0 { 0 } else { turbo as u8 as i64 };
    buffer.paint_lit(left + turbo + 17444, 58 - turbo, 7, 0);
    let damage = (100.0 - (f64::from(boards[0].damage_bar) * 0.0009765625).ceil()) as i32;
    let end = number(buffer, &images.big_digits, damage, left + 36398);
    buffer.draw(
        images.big_digits.get(0x320..0x320 + 80).unwrap_or(&[]),
        8,
        10,
        end,
    );
    let level = ((f64::from(boards[0].damage_bar) * 0.000048828125).ceil() as i64).max(0);
    let start = (6720 - 1344 * level).max(0) as usize;
    buffer.draw_opaque(
        images.damage.get(start..start + 1344).unwrap_or(&[]),
        64,
        21,
        left + 25632,
    );
    for (index, board) in boards.iter().enumerate() {
        if board.damage_bar <= 0 {
            buffer.draw(&images.wreck, 64, 32, left + 36896 + 0x4000 * index as i64);
        }
    }
}

/// `drawSprite_402590` with the big digits (8 x 10, step −8, hundreds −16): `n` right of
/// `at`, its digits left to right; where the next picture goes.
fn number(buffer: &mut Buffer, digits: &[u8], n: i32, at: i64) -> i64 {
    let glyph = |d: i32| {
        let start = 80 * d.clamp(0, 9) as usize;
        digits.get(start..start + 80).unwrap_or(&[]).to_vec()
    };
    let (width, step, hundreds) = (8i64, -8i64, -16i64);
    let n = n.max(0);
    let mut x = at;
    if n < 10 {
        buffer.draw(&glyph(n), 8, 10, x);
        x += width;
    }
    if (10..100).contains(&n) {
        x += step;
        buffer.draw(&glyph(n / 10), 8, 10, x);
        x += width;
        buffer.draw(&glyph(n % 10), 8, 10, x);
        x += width;
    }
    if (100..1000).contains(&n) {
        x += hundreds;
        buffer.draw(&glyph(n / 100), 8, 10, x);
        x += width;
        buffer.draw(&glyph(n % 100 / 10), 8, 10, x);
        x += width;
        buffer.draw(&glyph(n % 10), 8, 10, x);
        x += width;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The full bars (`initParticipantValues`, 102400).
    const FULL_BAR: i32 = 102_400;

    fn images() -> HudImages {
        let some = |len: usize| vec![1; len];
        HudImages {
            boards: some(64 * 201),
            damage: some(8064),
            big_digits: some(4000),
            small_font: some(36 * 96),
            own_place: some(4000),
            other_place: some(4000),
            flag: some(4000),
            mine: some(4000),
            slider: some(64 * 9),
            wreck: some(4000),
        }
    }

    fn board(damage: i32) -> Board {
        Board {
            name: b"A".to_vec(),
            lap: 1,
            medal: 7,
            damage_bar: (100 - damage) << 10,
            finished: false,
        }
    }

    /// An opponent recorded past 100 % damage (a damaged or edited save; the original never
    /// writes one) must not stop the race: its slider is longer than the HUD's, and the part
    /// before the slider's picture is left out.
    #[test]
    fn an_opponent_past_full_damage_does_not_stop_the_race() {
        let player = Player {
            speed: 0.0,
            engine: 1.0,
            weapons: true,
            weapons_bar: FULL_BAR,
            turbo_bar: FULL_BAR,
            mines: 0,
        };
        let boards = [board(0), board(110), board(255), board(40)];
        draw(&mut Buffer::default(), &images(), 64, &boards, &player, 4);
    }

    /// A place changed rolls the board's medal through the pictures between the old place
    /// and the new (7 a place), half a picture a tick for each place it moves, and stops at
    /// the new place's: a wrecked leader's medal rolls down to 4th over 14 frames while the
    /// others roll up one place over 14 frames too.
    #[test]
    fn a_changed_place_rolls_the_medal_to_the_new_one() {
        let mut medals = Medals::new(&[1, 2, 3, 4]);
        assert_eq!(medals.roll(&[1, 2, 3, 4], 1), [0, 7, 14, 21]);
        assert_eq!(medals.roll(&[4, 1, 2, 3], 1), [2, 7, 14, 21]);
        assert_eq!(medals.roll(&[4, 1, 2, 3], 1), [3, 6, 13, 20]);
        let mut last = Vec::new();
        for _ in 0..20 {
            last = medals.roll(&[4, 1, 2, 3], 1);
        }
        assert_eq!(last, [21, 0, 7, 14]);
        // Two ticks between frames roll twice as far.
        let mut medals = Medals::new(&[1, 2]);
        assert_eq!(medals.roll(&[2, 1], 2), [1, 6]);
    }
}
