//! Saved games, `DR.SG0` to `DR.SG7` (spec M3b): 2179 bytes, the first a key the rest is
//! encrypted with (`decryptEntireSavegame` 0x41C910, `loadGame` 0x42F2E0, `savegameWithName`
//! 0x42F6E0; dRally `drencryption.c`). Byte by byte from the second: the key subtracted, 17
//! times the byte's offset added, rotated right by the offset modulo 6.

use crate::dr_cfg::{ARENA, ARENA_LAPS, RACE_LAPS};

/// A saved game's file length.
pub const SAVE_BYTES: usize = 0x883;
/// Slots 0 to 7; the last is the quicksave.
pub const SLOTS: usize = 8;
pub const QUICKSAVE_SLOT: usize = 7;
/// The name the player gave the saved game, up to 15 bytes.
pub const NAME_BYTES: usize = 15;
/// The twenty drivers' records, 108 bytes each, as the original keeps them.
pub const DRIVERS_BYTES: usize = 0x870;

const NAME_AT: usize = 4;

/// Slot `slot`'s file name.
pub fn file_name(slot: usize) -> String {
    format!("DR.SG{slot}")
}

/// The eight slots' files: DeadRally's own in `own_dir` first, else the game folder's, which
/// is only read (a game saved with the original goes on in DeadRally); `None` where neither
/// has one or it cannot be read.
pub fn load_slots(
    own_dir: Option<&std::path::Path>,
    game_dir: &std::path::Path,
) -> Vec<Option<Vec<u8>>> {
    (0..SLOTS)
        .map(|slot| {
            let name = file_name(slot);
            own_dir
                .and_then(|dir| std::fs::read(dir.join(&name)).ok())
                .or_else(|| std::fs::read(game_dir.join(&name)).ok())
        })
        .collect()
}

/// Writes slot `slot` into DeadRally's own folder, never the game's.
///
/// # Errors
///
/// When the folder cannot be made or the file written.
pub fn write_slot(own_dir: &std::path::Path, slot: usize, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(own_dir)?;
    // Written beside it and renamed over it, so an interrupted write never leaves half a
    // game where a whole one was.
    let path = own_dir.join(file_name(slot));
    let partial = own_dir.join(format!("{}.partial", file_name(slot)));
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, &path)
}
const DRIVERS_AT: usize = NAME_AT + NAME_BYTES;

/// What a saved game holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveGame {
    pub driver_id: u8,
    pub use_weapons: u8,
    pub difficulty: u8,
    /// NUL-padded.
    pub name: [u8; NAME_BYTES],
    pub drivers: Vec<u8>,
}

impl SaveGame {
    /// The file decrypted as `loadGame` reads it: a shorter file is read as far as it goes
    /// into zeros, a longer one only to its 2179th byte.
    pub fn decode(file: &[u8]) -> SaveGame {
        let mut bytes = [0u8; SAVE_BYTES];
        let length = file.len().min(SAVE_BYTES);
        bytes[..length].copy_from_slice(&file[..length]);
        let key = bytes[0];
        for (offset, byte) in bytes.iter_mut().enumerate().skip(1) {
            *byte = byte
                .rotate_left((offset % 6) as u32)
                .wrapping_sub((17 * offset) as u8)
                .wrapping_add(key);
        }
        let mut name = [0; NAME_BYTES];
        name.copy_from_slice(&bytes[NAME_AT..DRIVERS_AT]);
        SaveGame {
            driver_id: bytes[1],
            use_weapons: bytes[2],
            difficulty: bytes[3],
            name,
            drivers: bytes[DRIVERS_AT..].to_vec(),
        }
    }

    /// The file `savegameWithName` writes with `key` (`rand() % 255` there).
    ///
    /// # Panics
    ///
    /// If `drivers` is not [`DRIVERS_BYTES`] long.
    pub fn encode(&self, key: u8) -> Vec<u8> {
        assert_eq!(
            self.drivers.len(),
            DRIVERS_BYTES,
            "twenty drivers of 108 bytes"
        );
        let mut bytes = vec![0u8; SAVE_BYTES];
        bytes[0] = key;
        bytes[1] = self.driver_id;
        bytes[2] = self.use_weapons;
        bytes[3] = self.difficulty;
        bytes[NAME_AT..DRIVERS_AT].copy_from_slice(&self.name);
        bytes[DRIVERS_AT..].copy_from_slice(&self.drivers);
        for (offset, byte) in bytes.iter_mut().enumerate().skip(1) {
            *byte = byte
                .wrapping_sub(key)
                .wrapping_add((17 * offset) as u8)
                .rotate_right((offset % 6) as u32);
        }
        bytes
    }
}

/// DeadRally's personal best of one game (docs/design/2026-10-13-personal-best-design.md):
/// the player's best lap and best whole race over each count of laps, by circuit and car, and
/// the Arena's, in hundredths (0 for none). A saved game's file ends with it after the
/// original's bytes, which are all the original reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonalBest {
    times: Vec<u32>,
}

/// The block's tag, and its times: for each circuit and car the lap and the races of
/// [`RACE_LAPS`], then for each car the Arena's lap and its race of [`ARENA_LAPS`].
const PERSONAL_BEST_TAG: &[u8; 4] = b"DRPB";
const CIRCUIT_TIMES: usize = 1 + RACE_LAPS.len();
const TIMES: usize = 18 * 6 * CIRCUIT_TIMES + 6 * 2;

impl Default for PersonalBest {
    fn default() -> PersonalBest {
        PersonalBest {
            times: vec![0; TIMES],
        }
    }
}

impl PersonalBest {
    /// The personal best a saved game's file holds after the original's bytes; none when it
    /// holds no whole block (a game the original saved).
    pub fn read(file: &[u8]) -> PersonalBest {
        let block = file.get(SAVE_BYTES..).unwrap_or_default();
        match block.split_first_chunk::<4>() {
            Some((tag, times)) if tag == PERSONAL_BEST_TAG && times.len() >= 4 * TIMES => {
                PersonalBest {
                    times: times
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .take(TIMES)
                        .map(|&bytes| u32::from_le_bytes(bytes))
                        .collect(),
                }
            }
            _ => PersonalBest::default(),
        }
    }

    /// The block a saved game's file ends with.
    pub fn to_bytes(&self) -> Vec<u8> {
        let times = self.times.iter().flat_map(|time| time.to_le_bytes());
        PERSONAL_BEST_TAG.iter().copied().chain(times).collect()
    }

    /// The best lap of `car` on `circuit` (or [`ARENA`]).
    pub fn lap(&self, circuit: usize, car: usize) -> Option<[u32; 3]> {
        clock(self.times[index(circuit, None, car)])
    }

    /// The best whole race of `car` over `laps` laps of `circuit` (or [`ARENA`]).
    pub fn race(&self, circuit: usize, laps: i32, car: usize) -> Option<[u32; 3]> {
        clock(self.times[index(circuit, Some(laps), car)])
    }

    /// `time` kept as the best lap when it is the first or better; whether it was.
    pub fn keep_lap(&mut self, circuit: usize, car: usize, time: [u32; 3]) -> bool {
        self.keep(index(circuit, None, car), time)
    }

    /// `time` kept as the best whole race over `laps` laps when it is the first or better;
    /// whether it was.
    pub fn keep_race(&mut self, circuit: usize, laps: i32, car: usize, time: [u32; 3]) -> bool {
        self.keep(index(circuit, Some(laps), car), time)
    }

    fn keep(&mut self, at: usize, [minutes, seconds, hundredths]: [u32; 3]) -> bool {
        let time = minutes
            .saturating_mul(6000)
            .saturating_add(seconds.saturating_mul(100))
            .saturating_add(hundredths);
        let best = &mut self.times[at];
        let kept = time > 0 && (*best == 0 || time < *best);
        if kept {
            *best = time;
        }
        kept
    }
}

/// Where the time of `car` over `laps` laps (none: the lap) of `circuit` sits in the block.
fn index(circuit: usize, laps: Option<i32>, car: usize) -> usize {
    assert!(car < 6, "no car {car}");
    if circuit == ARENA {
        let kind = match laps {
            None => 0,
            Some(ARENA_LAPS) => 1,
            Some(laps) => panic!("no race of {laps} laps in the Arena"),
        };
        return 18 * 6 * CIRCUIT_TIMES + car * 2 + kind;
    }
    assert!(circuit < 18, "no circuit {circuit}");
    let kind = match laps {
        None => 0,
        Some(laps) => {
            1 + RACE_LAPS
                .iter()
                .position(|&count| count == laps)
                .unwrap_or_else(|| panic!("no race of {laps} laps"))
        }
    };
    (circuit * 6 + car) * CIRCUIT_TIMES + kind
}

/// A time in hundredths as minutes, seconds and hundredths; none for 0.
fn clock(time: u32) -> Option<[u32; 3]> {
    (time > 0).then_some([time / 6000, time / 100 % 60, time % 100])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_personal_best_keeps_the_player_s_better_times_by_circuit_laps_and_car() {
        // The first time and each better one, never a worse one: a slow race is kept until a
        // faster comes, whatever the records are; laps and races and cars kept apart.
        let mut best = PersonalBest::default();
        assert_eq!(best.lap(14, 0), None);
        assert!(best.keep_race(14, 4, 0, [0, 58, 21]));
        assert!(!best.keep_race(14, 4, 0, [0, 59, 0]), "a worse race");
        assert!(best.keep_race(14, 4, 0, [0, 57, 99]), "a better one");
        assert!(best.keep_lap(14, 0, [0, 14, 2]));
        assert!(best.keep_lap(ARENA, 5, [0, 12, 0]));
        assert!(best.keep_race(ARENA, 9, 5, [2, 0, 0]));
        assert!(!best.keep_lap(3, 1, [0, 0, 0]), "no lap driven");
        assert_eq!(best.race(14, 4, 0), Some([0, 57, 99]));
        assert_eq!(best.lap(14, 0), Some([0, 14, 2]));
        assert_eq!(best.race(14, 5, 0), None);
        assert_eq!(best.race(14, 4, 1), None);
        assert_eq!(best.lap(ARENA, 5), Some([0, 12, 0]));
        assert_eq!(best.race(ARENA, 9, 5), Some([2, 0, 0]));
        assert_eq!(best.lap(3, 1), None);
    }

    #[test]
    fn a_saved_game_carries_its_personal_best_after_the_original_s_bytes() {
        // The original reads only its 2179 bytes, so the block after them leaves its game
        // whole; a game the original saved has no personal best.
        let mut best = PersonalBest::default();
        best.keep_race(17, 6, 5, [1, 20, 30]);
        let original = vec![7; SAVE_BYTES];
        let file = [original.clone(), best.to_bytes()].concat();
        assert_eq!(PersonalBest::read(&file), best);
        assert_eq!(SaveGame::decode(&file), SaveGame::decode(&original));
        assert_eq!(PersonalBest::read(&original), PersonalBest::default());
        assert_eq!(
            PersonalBest::read(&file[..file.len() - 1]),
            PersonalBest::default()
        );
    }

    fn game() -> SaveGame {
        SaveGame {
            driver_id: 19,
            use_weapons: 1,
            difficulty: 2,
            name: *b"my race\0\0\0\0\0\0\0\0",
            drivers: (0..DRIVERS_BYTES).map(|i| (i * 7) as u8).collect(),
        }
    }

    #[test]
    fn a_saved_game_reads_back_as_written_with_any_key() {
        // The original's saves must load in DeadRally and DeadRally's in the original; the
        // key changes every byte, the content must not.
        for key in [0, 1, 77, 254] {
            let file = game().encode(key);
            assert_eq!(file.len(), SAVE_BYTES);
            assert_eq!(file[0], key);
            assert_eq!(SaveGame::decode(&file), game());
        }
    }

    #[test]
    fn the_bytes_follow_the_originals_encryption() {
        // Byte 1 (driver 19, key 5): 19 - 5 + 17 = 31, rotated right by 1: 0x8F. Byte 6
        // (the name's third byte, a space, 0x20): 0x20 - 5 + 102 = 0x81, rotated by 0.
        let file = game().encode(5);
        assert_eq!(file[1], 0x8F);
        assert_eq!(file[6], 0x81);
    }

    #[test]
    fn own_saves_come_before_the_game_folders_and_are_written_only_there() {
        // A game saved with the original goes on in DeadRally; DeadRally never writes into
        // the game folder.
        let root = std::env::temp_dir().join(format!("deadrally-saves-{}", std::process::id()));
        let (own, game) = (root.join("own"), root.join("game"));
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("DR.SG0"), b"original 0").unwrap();
        std::fs::write(game.join("DR.SG1"), b"original 1").unwrap();
        write_slot(&own, 1, b"ours 1").unwrap();
        let slots = load_slots(Some(&own), &game);
        assert_eq!(slots[0].as_deref(), Some(&b"original 0"[..]));
        assert_eq!(slots[1].as_deref(), Some(&b"ours 1"[..]));
        assert_eq!(slots[2], None);
        assert_eq!(std::fs::read(game.join("DR.SG1")).unwrap(), b"original 1");
        write_slot(&own, 1, b"ours again").unwrap();
        assert_eq!(std::fs::read(own.join("DR.SG1")).unwrap(), b"ours again");
        assert!(
            !own.join("DR.SG1.partial").exists(),
            "nothing half-written is left"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_short_file_reads_as_far_as_it_goes() {
        // fread into a zeroed buffer: what is missing decrypts from zeros, and nothing panics.
        let file = game().encode(9);
        let short = SaveGame::decode(&file[..100]);
        assert_eq!((short.driver_id, short.name), (19, game().name));
        assert_eq!(short.drivers.len(), DRIVERS_BYTES);
        assert_eq!(SaveGame::decode(&[]).drivers.len(), DRIVERS_BYTES);
    }
}
