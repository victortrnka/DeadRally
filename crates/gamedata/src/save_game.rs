//! Saved games, `DR.SG0` to `DR.SG7` (spec M3b): 2179 bytes, the first a key the rest is
//! encrypted with (`decryptEntireSavegame` 0x41C910, `loadGame` 0x42F2E0, `savegameWithName`
//! 0x42F6E0; dRally `drencryption.c`). Byte by byte from the second: the key subtracted, 17
//! times the byte's offset added, rotated right by the offset modulo 6.

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

#[cfg(test)]
mod tests {
    use super::*;

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
