//! The original's settings file, `dr.cfg` (spec M2b §3.1): 8 bytes of header, then the
//! settings, the records and the Hall of Fame at the offsets `saveConfiguration` (0x4264E0)
//! writes them. DeadRally keeps every byte it does not use as it was.

use std::io;
use std::path::{Path, PathBuf};

use crate::exe::Exe;
use crate::machine::{Machine, MachineError};

/// DeadRally's own `dr.cfg`, next to its `config.toml` (spec M2b decision 2).
pub fn own_path() -> Option<PathBuf> {
    crate::config_path().map(|config| config.with_file_name("dr.cfg"))
}

/// The `dr.cfg` to start with: DeadRally's own (`own`), else the game folder's, read once,
/// else `defaults`. Nothing is written.
///
/// # Errors
///
/// An [`io::Error`] when a file exists but cannot be read; starting from the defaults would
/// overwrite the player's settings and records at the next save.
pub fn load(own: Option<&Path>, game_dir: &Path, defaults: &DrCfg) -> io::Result<DrCfg> {
    let game = ["dr.cfg", "DR.CFG"].map(|name| game_dir.join(name));
    for path in own.into_iter().chain(game.iter().map(PathBuf::as_path)) {
        match std::fs::read(path) {
            Ok(bytes) => return Ok(DrCfg::parse(&bytes).unwrap_or_else(|| defaults.clone())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(defaults.clone())
}

/// Writes `bytes` to DeadRally's own `dr.cfg` at `own`, never into the game folder.
///
/// # Errors
///
/// An [`io::Error`] when the directory or the file cannot be written.
pub fn save(own: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = own.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(own, bytes)
}

/// Three bytes, a 32-bit value and a byte `rand()` wrote, which nothing reads.
pub const HEADER_BYTES: usize = 8;
/// What `saveConfiguration` writes after the header.
pub const PAYLOAD_BYTES: usize = 0xB76;
/// A file this short or shorter is replaced by the defaults (`loadConfig`).
const SHORTEST: usize = 7;

/// Offsets in the payload.
const MUSIC_VOLUME: usize = 0x00;
const EFFECTS_VOLUME: usize = 0x04;
/// The difficulty a new game is set to (0x456738): 0, 1 or 2.
const DIFFICULTY: usize = 0x0C;
const USE_JOYSTICK: usize = 0x10;
/// Accelerate, brake, left, right, turbo, gun, mine, horn: set-1 scancodes.
const KEYS: usize = 0xB36;
pub const KEY_COUNT: usize = 8;
/// Accelerate, brake, left, right, turbo, gun, mine: gamepad inputs (0 none, 1–4 the
/// stick's left, right, up, down, 5–8 buttons 1–4).
const PADS: usize = 0xB56;
pub const PAD_COUNT: usize = 7;
const TIMES_PLAYED: usize = 0xB72;
/// The circuits' records: 18 circuits by 6 cars, 24 bytes each (a name of up to 12 bytes,
/// minutes, seconds, hundredths), record `circuit + 18 * car`.
const RECORDS: usize = 0x4E;
const RECORD_BYTES: usize = 24;
/// The Arena's lap records, DeadRally's own (the original reads and writes circuit 0's): a
/// record for each car as the circuits' have, after the original's bytes. A file without them
/// is the original's, and they start empty.
pub const ARENA: usize = 18;
const ARENA_BYTES: usize = RECORD_BYTES * 6;
/// DeadRally's race records, after the Arena's lap records: for each count of laps of
/// [`RACE_LAPS`], for each car, for each of the 18 circuits, a record as a lap record has it;
/// then the Arena's races of [`ARENA_LAPS`] laps, one for each car.
pub const RACE_LAPS: [i32; 3] = [4, 5, 6];
pub const ARENA_LAPS: i32 = 9;
const RACE_RECORDS_BYTES: usize = RECORD_BYTES * (18 * 6 * RACE_LAPS.len() + 6);
/// The best ten: 20 bytes each (a name of up to 12 bytes, races, difficulty).
const HALL_OF_FAME: usize = 0xA6E;
const ENTRY_BYTES: usize = 20;
pub const HALL_OF_FAME_ENTRIES: usize = 10;
const NAME_BYTES: usize = 12;

/// `defaultConfig`, up to its jump into `saveConfiguration`.
const DEFAULT_CONFIG: (u32, u32) = (0x42_6700, 0x42_71E8);
/// Where the original keeps what it writes: (offset in the file, address, bytes).
const HEADER_SOURCES: [(usize, u32, usize); 4] = [
    (0, 0x45_FB6C, 1),
    (1, 0x45_FBF0, 1),
    (2, 0x46_3D9C, 1),
    (3, 0x46_2D54, 4),
];
const PAYLOAD_SOURCES: [(usize, u32, usize); 29] = [
    (0x00, 0x45_DC14, 4),
    (0x04, 0x45_DC18, 4),
    (0x08, 0x44_57CC, 4),
    (0x0C, 0x45_6738, 4),
    (0x10, 0x45_EA00, 4),
    (0x14, 0x46_3D00, 21),
    (0x29, 0x45_EB80, 21),
    (0x3E, 0x45_6734, 4),
    (0x42, 0x45_FB68, 4),
    (0x46, 0x45_DC40, 4),
    (0x4A, 0x45_DC1C, 4),
    (0x4E, 0x45_F040, 0xA20),
    (0xA6E, 0x46_1F20, 0xC8),
    (0xB36, 0x46_1EA8, 4),
    (0xB3A, 0x46_3CA8, 4),
    (0xB3E, 0x46_1FF8, 4),
    (0xB42, 0x45_EA68, 4),
    (0xB46, 0x45_FBF4, 4),
    (0xB4A, 0x46_3CE4, 4),
    (0xB4E, 0x46_1270, 4),
    (0xB52, 0x46_3D18, 4),
    (0xB56, 0x46_1294, 4),
    (0xB5A, 0x46_1F14, 4),
    (0xB5E, 0x45_EEA0, 4),
    (0xB62, 0x46_2D6C, 4),
    (0xB66, 0x46_3D8C, 4),
    (0xB6A, 0x46_2CFC, 4),
    (0xB6E, 0x46_3CA4, 4),
    (0xB72, 0x46_3CEC, 4),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrCfg {
    header: [u8; HEADER_BYTES],
    payload: Vec<u8>,
    /// DeadRally's own, after the original's bytes: the Arena's lap records, the race
    /// records, and the best ten's entries past the tenth (20 bytes each).
    arena: Vec<u8>,
    races: Vec<u8>,
    fame: Vec<u8>,
}

impl DrCfg {
    /// The file the original writes when it has none: `defaultConfig` run over the player's
    /// `dr.exe`, its variables then gathered as `saveConfiguration` gathers them. The random
    /// byte is 0.
    ///
    /// # Errors
    ///
    /// [`MachineError`] when the code is not the known release's.
    pub fn defaults(exe: &Exe) -> Result<DrCfg, MachineError> {
        let mut machine = Machine::new(exe);
        machine.run(DEFAULT_CONFIG.0, DEFAULT_CONFIG.1)?;
        let mut header = [0; HEADER_BYTES];
        for (offset, address, length) in HEADER_SOURCES {
            header[offset..offset + length].copy_from_slice(&machine.bytes(address, length)?);
        }
        let mut payload = vec![0; PAYLOAD_BYTES];
        for (offset, address, length) in PAYLOAD_SOURCES {
            payload[offset..offset + length].copy_from_slice(&machine.bytes(address, length)?);
        }
        Ok(DrCfg {
            header,
            payload,
            arena: vec![0; ARENA_BYTES],
            races: vec![0; RACE_RECORDS_BYTES],
            fame: Vec::new(),
        })
    }

    /// A `dr.cfg` as `loadConfig` takes it: `None` when it is 7 bytes or shorter (the
    /// original then uses its defaults); else its header and payload, bytes it lacks 0 and
    /// extra bytes left out but for DeadRally's own after them: the Arena's lap records, the
    /// race records, and a count of the best ten's entries past the tenth with as many of
    /// them as the file holds.
    pub fn parse(bytes: &[u8]) -> Option<DrCfg> {
        if bytes.len() <= SHORTEST {
            return None;
        }
        let mut header = [0; HEADER_BYTES];
        let in_header = bytes.len().min(HEADER_BYTES);
        header[..in_header].copy_from_slice(&bytes[..in_header]);
        let mut payload = bytes.get(HEADER_BYTES..).unwrap_or_default().to_vec();
        payload.resize(PAYLOAD_BYTES, 0);
        let ours = bytes
            .get(HEADER_BYTES + PAYLOAD_BYTES..)
            .unwrap_or_default();
        let block = |from: usize, length: usize| {
            ours.get(from..from + length)
                .map_or_else(|| vec![0; length], <[u8]>::to_vec)
        };
        let fame_at = ARENA_BYTES + RACE_RECORDS_BYTES;
        let count = ours
            .get(fame_at..fame_at + 4)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        let entries = ours.get(fame_at + 4..).unwrap_or_default();
        let count = count.min(entries.len() / ENTRY_BYTES);
        Some(DrCfg {
            header,
            payload,
            arena: block(0, ARENA_BYTES),
            races: block(ARENA_BYTES, RACE_RECORDS_BYTES),
            fame: entries[..count * ENTRY_BYTES].to_vec(),
        })
    }

    /// The file as `saveConfiguration` writes it, then DeadRally's own blocks up to the last
    /// with anything in it, the ones before it written empty: a file without any is the
    /// original's.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = [&self.header[..], &self.payload].concat();
        let fame = !self.fame.is_empty();
        let races = fame || self.races.iter().any(|&b| b != 0);
        if races || self.arena.iter().any(|&b| b != 0) {
            bytes.extend_from_slice(&self.arena);
        }
        if races {
            bytes.extend_from_slice(&self.races);
        }
        if fame {
            let count = (self.fame.len() / ENTRY_BYTES) as u32;
            bytes.extend_from_slice(&count.to_le_bytes());
            bytes.extend_from_slice(&self.fame);
        }
        bytes
    }

    fn get(&self, offset: usize) -> u32 {
        let b = &self.payload[offset..offset + 4];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }

    fn put(&mut self, offset: usize, value: u32) {
        self.payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// The difficulty the licence last chose.
    pub fn difficulty(&self) -> u32 {
        self.get(DIFFICULTY)
    }

    pub fn set_difficulty(&mut self, difficulty: u32) {
        self.put(DIFFICULTY, difficulty);
    }

    /// The byte `saveConfiguration` writes from `rand()` after the header's values.
    pub fn set_random_byte(&mut self, byte: u8) {
        self.header[HEADER_BYTES - 1] = byte;
    }

    /// The music and effects volumes, 0..=0x10000.
    pub fn music_volume(&self) -> u32 {
        self.get(MUSIC_VOLUME)
    }

    pub fn set_music_volume(&mut self, volume: u32) {
        self.put(MUSIC_VOLUME, volume);
    }

    pub fn effects_volume(&self) -> u32 {
        self.get(EFFECTS_VOLUME)
    }

    pub fn set_effects_volume(&mut self, volume: u32) {
        self.put(EFFECTS_VOLUME, volume);
    }

    /// The gamepad switch: 0 off, 1 or 2 on.
    pub fn use_joystick(&self) -> u32 {
        self.get(USE_JOYSTICK)
    }

    pub fn set_use_joystick(&mut self, on: u32) {
        self.put(USE_JOYSTICK, on);
    }

    pub fn key(&self, control: usize) -> u32 {
        assert!(control < KEY_COUNT);
        self.get(KEYS + 4 * control)
    }

    pub fn set_key(&mut self, control: usize, scancode: u32) {
        assert!(control < KEY_COUNT);
        self.put(KEYS + 4 * control, scancode);
    }

    pub fn pad(&self, control: usize) -> u32 {
        assert!(control < PAD_COUNT);
        self.get(PADS + 4 * control)
    }

    pub fn set_pad(&mut self, control: usize, input: u32) {
        assert!(control < PAD_COUNT);
        self.put(PADS + 4 * control, input);
    }

    /// The name at `offset`, up to its NUL within `room` bytes.
    fn name(&self, offset: usize, room: usize) -> &[u8] {
        let field = &self.payload[offset..offset + room];
        &field[..field.iter().position(|&b| b == 0).unwrap_or(room)]
    }

    /// Record `car` (0–5) of circuit `circuit` (0–17, or [`ARENA`]): the driver's name and
    /// the time (minutes, seconds, hundredths); an Arena's record never set is empty.
    pub fn record(&self, circuit: usize, car: usize) -> (&[u8], [u32; 3]) {
        let at = self.lap_record_at(circuit, car);
        let record = if circuit == ARENA {
            &self.arena[at..at + RECORD_BYTES]
        } else {
            &self.payload[at..at + RECORD_BYTES]
        };
        read_record(record)
    }

    /// A new record `car` of circuit `circuit` (or [`ARENA`]): `name` upper-cased (`strcpy`
    /// then `_strupr`, so the rest of the field keeps what it held after the NUL) and the
    /// time.
    pub fn set_record(&mut self, circuit: usize, car: usize, name: &[u8], time: [u32; 3]) {
        let at = self.lap_record_at(circuit, car);
        let record = if circuit == ARENA {
            &mut self.arena[at..at + RECORD_BYTES]
        } else {
            &mut self.payload[at..at + RECORD_BYTES]
        };
        write_record(record, name, time);
    }

    /// Where lap record `car` of `circuit` starts: in the payload, or in the Arena's block.
    fn lap_record_at(&self, circuit: usize, car: usize) -> usize {
        assert!(
            circuit <= ARENA && car < 6,
            "no lap record {circuit}, {car}"
        );
        if circuit == ARENA {
            RECORD_BYTES * car
        } else {
            RECORDS + RECORD_BYTES * (circuit + 18 * car)
        }
    }

    /// The race record of car `car` (0–5) over `laps` laps of circuit `circuit` (0–17, with
    /// laps of [`RACE_LAPS`]), or of the Arena ([`ARENA`], [`ARENA_LAPS`]): the driver's name
    /// and the race's time; empty when never set.
    pub fn race_record(&self, circuit: usize, laps: i32, car: usize) -> (&[u8], [u32; 3]) {
        let at = race_record_at(circuit, laps, car);
        read_record(&self.races[at..at + RECORD_BYTES])
    }

    /// A new race record, as [`DrCfg::set_record`] sets a lap record.
    pub fn set_race_record(
        &mut self,
        circuit: usize,
        laps: i32,
        car: usize,
        name: &[u8],
        time: [u32; 3],
    ) {
        let at = race_record_at(circuit, laps, car);
        write_record(&mut self.races[at..at + RECORD_BYTES], name, time);
    }

    /// Entry `rank` of the Hall of Fame (0–9 the original's best ten, then DeadRally's
    /// entries past them): the name, races and difficulty.
    pub fn hall_of_fame(&self, rank: usize) -> (&[u8], i32, u32) {
        let entry = self.entry(rank);
        let name = &entry[..NAME_BYTES];
        let word = |at: usize| {
            u32::from_le_bytes([entry[at], entry[at + 1], entry[at + 2], entry[at + 3]])
        };
        (
            &name[..name.iter().position(|&b| b == 0).unwrap_or(NAME_BYTES)],
            word(NAME_BYTES) as i32,
            word(NAME_BYTES + 4),
        )
    }

    /// The Hall of Fame's entries: the best ten and DeadRally's past them.
    pub fn hall_of_fame_len(&self) -> usize {
        HALL_OF_FAME_ENTRIES + self.fame.len() / ENTRY_BYTES
    }

    fn entry(&self, rank: usize) -> &[u8] {
        if rank < HALL_OF_FAME_ENTRIES {
            let at = HALL_OF_FAME + ENTRY_BYTES * rank;
            &self.payload[at..at + ENTRY_BYTES]
        } else {
            let at = ENTRY_BYTES * (rank - HALL_OF_FAME_ENTRIES);
            &self.fame[at..at + ENTRY_BYTES]
        }
    }

    /// A game won into the best ten (`showHallOfFameEndGame_430FA0`): before the first entry
    /// with more races than `races`, the entries below moving down a place and the last
    /// dropping out; the name copied over the old one up to its NUL (the bytes after it stay)
    /// and upper-cased. `None` when ten entries have as few races or fewer.
    pub fn insert_hall_of_fame(
        &mut self,
        name: &[u8],
        races: i32,
        difficulty: u32,
    ) -> Option<usize> {
        let rank = (0..HALL_OF_FAME_ENTRIES).find(|&rank| races < self.hall_of_fame(rank).1)?;
        for k in (rank + 1..HALL_OF_FAME_ENTRIES).rev() {
            let from = HALL_OF_FAME + ENTRY_BYTES * (k - 1);
            self.payload
                .copy_within(from..from + ENTRY_BYTES, from + ENTRY_BYTES);
        }
        let at = HALL_OF_FAME + ENTRY_BYTES * rank;
        write_entry(
            &mut self.payload[at..at + ENTRY_BYTES],
            name,
            races,
            difficulty,
        );
        Some(rank)
    }

    /// A game won into DeadRally's Hall of Fame, which grows: put as the original puts it
    /// (before the first entry with more races, the name over the old one), but nobody drops
    /// out; past the last when no entry has more races. The first ten stay in the original's
    /// slots. Its rank.
    pub fn add_to_hall_of_fame(&mut self, name: &[u8], races: i32, difficulty: u32) -> usize {
        let len = self.hall_of_fame_len();
        let rank = (0..len)
            .find(|&rank| races < self.hall_of_fame(rank).1)
            .unwrap_or(len);
        let mut entries: Vec<Vec<u8>> = (0..len).map(|rank| self.entry(rank).to_vec()).collect();
        let mut entry = entries
            .get(rank)
            .cloned()
            .unwrap_or_else(|| vec![0; ENTRY_BYTES]);
        write_entry(&mut entry, name, races, difficulty);
        entries.insert(rank, entry);
        let (best_ten, past) = entries.split_at(HALL_OF_FAME_ENTRIES);
        let at = HALL_OF_FAME;
        self.payload[at..at + ENTRY_BYTES * HALL_OF_FAME_ENTRIES]
            .copy_from_slice(&best_ten.concat());
        self.fame = past.concat();
        rank
    }

    /// Upper-cases the best ten's names in place, as `seeHallOfFame` (0x431510) does with
    /// `_strupr` before drawing them.
    pub fn upper_case_hall_of_fame(&mut self) {
        for rank in 0..HALL_OF_FAME_ENTRIES {
            let at = HALL_OF_FAME + ENTRY_BYTES * rank;
            let length = self.name(at, NAME_BYTES).len();
            self.payload[at..at + length].make_ascii_uppercase();
        }
    }

    pub fn times_played(&self) -> u32 {
        self.get(TIMES_PLAYED)
    }

    pub fn set_times_played(&mut self, times: u32) {
        self.put(TIMES_PLAYED, times);
    }
}

/// Where the race record of `car` over `laps` laps of `circuit` starts in the race records.
fn race_record_at(circuit: usize, laps: i32, car: usize) -> usize {
    assert!(car < 6, "no car {car}");
    let index = if circuit == ARENA {
        assert_eq!(
            laps, ARENA_LAPS,
            "the Arena's races are of {ARENA_LAPS} laps"
        );
        18 * 6 * RACE_LAPS.len() + car
    } else {
        assert!(circuit < 18, "no circuit {circuit}");
        let kind = RACE_LAPS
            .iter()
            .position(|&count| count == laps)
            .unwrap_or_else(|| panic!("no race of {laps} laps"));
        circuit + 18 * (car + 6 * kind)
    };
    RECORD_BYTES * index
}

/// A record's name up to its NUL and its time.
fn read_record(record: &[u8]) -> (&[u8], [u32; 3]) {
    let time = [0, 1, 2].map(|i| {
        let at = NAME_BYTES + 4 * i;
        u32::from_le_bytes([record[at], record[at + 1], record[at + 2], record[at + 3]])
    });
    let name = &record[..NAME_BYTES];
    (
        &name[..name.iter().position(|&b| b == 0).unwrap_or(NAME_BYTES)],
        time,
    )
}

/// `name` upper-cased over the record's old one up to the NUL, and the time.
fn write_record(record: &mut [u8], name: &[u8], time: [u32; 3]) {
    let length = name.len().min(NAME_BYTES - 1);
    record[..length].copy_from_slice(&name[..length].to_ascii_uppercase());
    record[length] = 0;
    for (i, part) in time.into_iter().enumerate() {
        let at = NAME_BYTES + 4 * i;
        record[at..at + 4].copy_from_slice(&part.to_le_bytes());
    }
}

/// `name` upper-cased over the entry's old one up to the NUL, the races and the difficulty.
fn write_entry(entry: &mut [u8], name: &[u8], races: i32, difficulty: u32) {
    let length = name.len().min(NAME_BYTES - 1);
    entry[..length].copy_from_slice(&name[..length].to_ascii_uppercase());
    entry[length] = 0;
    entry[NAME_BYTES..NAME_BYTES + 4].copy_from_slice(&(races as u32).to_le_bytes());
    entry[NAME_BYTES + 4..NAME_BYTES + 8].copy_from_slice(&difficulty.to_le_bytes());
}
#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> Vec<u8> {
        (0..HEADER_BYTES + PAYLOAD_BYTES)
            .map(|i| (i % 251) as u8)
            .collect()
    }

    #[test]
    fn a_file_reads_and_writes_back_byte_for_byte() {
        // The records and the Hall of Fame are only carried in M2b; a byte lost on the way
        // would lose a player's records.
        assert_eq!(DrCfg::parse(&file()).unwrap().to_bytes(), file());
    }

    fn dummy() -> DrCfg {
        DrCfg {
            header: [9; HEADER_BYTES],
            payload: vec![9; PAYLOAD_BYTES],
            arena: vec![0; ARENA_BYTES],
            races: vec![0; RACE_RECORDS_BYTES],
            fame: Vec::new(),
        }
    }

    #[test]
    fn a_file_of_seven_bytes_or_fewer_is_no_file() {
        assert_eq!(DrCfg::parse(&[1; 7]), None);
        assert!(DrCfg::parse(&[1; 8]).is_some());
    }

    #[test]
    fn a_short_file_lacks_its_last_bytes_as_zeros_and_a_long_one_is_cut() {
        let short = DrCfg::parse(&file()[..20]).unwrap().to_bytes();
        assert_eq!(&short[..20], &file()[..20]);
        assert!(short[20..].iter().all(|&b| b == 0));
        assert_eq!(short.len(), HEADER_BYTES + PAYLOAD_BYTES);
        let mut long = file();
        long.extend([7; 100]);
        assert_eq!(DrCfg::parse(&long).unwrap().to_bytes(), file());
    }

    #[test]
    fn the_settings_sit_at_the_offsets_save_configuration_writes() {
        // A setting at another offset would read the player's records as their volume.
        let mut cfg = dummy();
        cfg.set_music_volume(0x7400);
        cfg.set_effects_volume(0xC400);
        cfg.set_use_joystick(1);
        cfg.set_key(0, 0x10);
        cfg.set_key(7, 0x39);
        cfg.set_pad(6, 8);
        cfg.set_times_played(3);
        let bytes = cfg.to_bytes();
        let at = |offset: usize| &bytes[HEADER_BYTES + offset..HEADER_BYTES + offset + 4];
        assert_eq!(at(0), [0x00, 0x74, 0, 0]);
        assert_eq!(at(4), [0x00, 0xC4, 0, 0]);
        assert_eq!(at(0x10), [1, 0, 0, 0]);
        assert_eq!(at(0xB36), [0x10, 0, 0, 0]);
        assert_eq!(at(0xB52), [0x39, 0, 0, 0]);
        assert_eq!(at(0xB6E), [8, 0, 0, 0]);
        assert_eq!(at(0xB72), [3, 0, 0, 0]);
        assert_eq!(
            (cfg.music_volume(), cfg.key(7), cfg.pad(6)),
            (0x7400, 0x39, 8)
        );
    }

    #[test]
    fn the_saved_variables_cover_the_payload_without_gaps() {
        let mut next = 0;
        for (offset, _, length) in PAYLOAD_SOURCES {
            assert_eq!(offset, next);
            next = offset + length;
        }
        assert_eq!(next, PAYLOAD_BYTES);
    }

    #[test]
    fn deadrallys_own_file_comes_first_then_the_games_once_then_the_defaults() {
        // The player's records survive: DeadRally starts from its own copy, picks up the
        // original's settings the first time, and never writes into the game folder.
        let home = tempfile::tempdir().unwrap();
        let game = tempfile::tempdir().unwrap();
        let own = home.path().join("deadrally/dr.cfg");
        assert_eq!(load(Some(&own), game.path(), &dummy()).unwrap(), dummy());
        std::fs::write(game.path().join("dr.cfg"), file()).unwrap();
        let imported = load(Some(&own), game.path(), &dummy()).unwrap();
        assert_eq!(imported.to_bytes(), file());
        let mut changed = imported;
        changed.set_times_played(9);
        save(&own, &changed.to_bytes()).unwrap();
        assert_eq!(load(Some(&own), game.path(), &dummy()).unwrap(), changed);
        assert_eq!(std::fs::read(game.path().join("dr.cfg")).unwrap(), file());
    }

    #[test]
    fn a_file_that_cannot_be_read_is_an_error_not_the_defaults() {
        // Starting from the defaults would overwrite the player's file at the next save.
        let home = tempfile::tempdir().unwrap();
        let unreadable = home.path().join("dr.cfg");
        std::fs::create_dir(&unreadable).unwrap();
        assert!(load(Some(&unreadable), home.path(), &dummy()).is_err());
    }

    #[test]
    fn a_won_game_enters_the_best_ten_before_the_first_with_more_races() {
        // The best ten are sorted by races, fewest first; a winner with as many races as an
        // entry goes below it, and the last entry drops out.
        let mut cfg = DrCfg::parse(&[0; 8]).unwrap();
        for rank in 0..HALL_OF_FAME_ENTRIES {
            let at = HALL_OF_FAME + ENTRY_BYTES * rank;
            cfg.payload[at] = b'a' + rank as u8;
            cfg.put(at + NAME_BYTES, 10 * (rank as u32 + 1));
        }
        assert_eq!(cfg.insert_hall_of_fame(b"Tom", 30, 2), Some(3));
        assert_eq!(cfg.hall_of_fame(2), (&b"c"[..], 30, 0));
        assert_eq!(cfg.hall_of_fame(3), (&b"TOM"[..], 30, 2));
        assert_eq!(cfg.hall_of_fame(4), (&b"d"[..], 40, 0));
        assert_eq!(cfg.hall_of_fame(9), (&b"i"[..], 90, 0));
        assert_eq!(cfg.insert_hall_of_fame(b"slow", 100, 0), None);
    }

    #[test]
    fn a_new_record_keeps_the_name_upper_cased_and_the_time() {
        // The statistics after a race write a lap record into dr.cfg's table; the next race
        // shows it in the HUD and the next statistics as the best lap ever.
        let mut cfg = DrCfg::parse(&[0; 8]).unwrap();
        cfg.set_record(3, 2, b"Tester", [1, 2, 3]);
        assert_eq!(cfg.record(3, 2), (&b"TESTER"[..], [1, 2, 3]));
        assert_eq!(cfg.record(3, 1).1, [0, 0, 0]);
    }

    #[test]
    fn records_and_the_best_ten_sit_where_save_configuration_puts_them() {
        // Circuit-major within each car: record 17 + 18 * 5 is the last of the 2592 bytes.
        let mut bytes = vec![0; HEADER_BYTES + PAYLOAD_BYTES];
        let last = HEADER_BYTES + RECORDS + RECORD_BYTES * (17 + 18 * 5);
        bytes[last..last + 3].copy_from_slice(b"Ann");
        bytes[last + 16..last + 20].copy_from_slice(&59u32.to_le_bytes());
        let first = HEADER_BYTES + HALL_OF_FAME;
        bytes[first..first + 12].copy_from_slice(b"Bob Twelve!!");
        bytes[first + 12..first + 16].copy_from_slice(&7u32.to_le_bytes());
        bytes[first + 16..first + 20].copy_from_slice(&2u32.to_le_bytes());
        let mut cfg = DrCfg::parse(&bytes).unwrap();
        assert_eq!(cfg.record(17, 5), (b"Ann".as_slice(), [0, 59, 0]));
        assert_eq!(cfg.hall_of_fame(0), (b"Bob Twelve!!".as_slice(), 7, 2));
        cfg.upper_case_hall_of_fame();
        assert_eq!(cfg.hall_of_fame(0).0, b"BOB TWELVE!!");
        assert_eq!(cfg.record(17, 5).0, b"Ann", "records keep their case");
    }

    #[test]
    fn the_arena_keeps_lap_records_of_its_own_after_the_original_s_bytes() {
        // A lap in the Arena is no lap of Suburbia: its record goes to the Arena's own table,
        // leaves every circuit's as it was, and survives the file being written and read.
        let mut cfg = DrCfg::parse(&file()).unwrap();
        assert_eq!(
            cfg.record(ARENA, 2),
            (&b""[..], [0, 0, 0]),
            "empty at first"
        );
        let suburbia = cfg.record(0, 2).0.to_vec();
        cfg.set_record(ARENA, 2, b"Ann", [0, 9, 12]);
        assert_eq!(cfg.record(ARENA, 2), (&b"ANN"[..], [0, 9, 12]));
        assert_eq!(cfg.record(ARENA, 1), (&b""[..], [0, 0, 0]));
        assert_eq!(cfg.record(0, 2).0, suburbia);
        let bytes = cfg.to_bytes();
        assert_eq!(&bytes[..HEADER_BYTES + PAYLOAD_BYTES], file());
        assert_eq!(bytes.len(), HEADER_BYTES + PAYLOAD_BYTES + RECORD_BYTES * 6);
        assert_eq!(DrCfg::parse(&bytes).unwrap(), cfg);
    }

    #[test]
    fn race_records_are_kept_by_circuit_laps_and_car_and_survive_the_file() {
        // A 6-lap time is no 4-lap record, and a race record is no lap record: each lands in
        // its own place, the rest stay empty, and the file keeps them.
        let mut cfg = DrCfg::parse(&file()).unwrap();
        cfg.set_race_record(3, 5, 2, b"Ann", [1, 2, 3]);
        cfg.set_race_record(ARENA, 9, 4, b"Bob", [2, 0, 50]);
        assert_eq!(cfg.race_record(3, 5, 2), (&b"ANN"[..], [1, 2, 3]));
        assert_eq!(cfg.race_record(ARENA, 9, 4), (&b"BOB"[..], [2, 0, 50]));
        for (circuit, laps, car) in [(3, 4, 2), (3, 6, 2), (3, 5, 1), (4, 5, 2), (ARENA, 9, 3)] {
            assert_eq!(cfg.race_record(circuit, laps, car), (&b""[..], [0, 0, 0]));
        }
        assert_eq!(
            cfg.record(3, 2),
            DrCfg::parse(&file()).unwrap().record(3, 2)
        );
        assert_eq!(cfg.record(ARENA, 4), (&b""[..], [0, 0, 0]));
        let bytes = cfg.to_bytes();
        assert_eq!(&bytes[..file().len()], file());
        assert_eq!(bytes.len(), file().len() + ARENA_BYTES + RACE_RECORDS_BYTES);
        assert_eq!(DrCfg::parse(&bytes).unwrap(), cfg);
    }

    /// A best ten whose entry `rank` is named "a" + rank, with 10 * (rank + 1) races.
    fn best_ten() -> DrCfg {
        let mut cfg = DrCfg::parse(&file()).unwrap();
        for rank in 0..HALL_OF_FAME_ENTRIES {
            let at = HALL_OF_FAME + ENTRY_BYTES * rank;
            cfg.payload[at..at + NAME_BYTES].fill(0);
            cfg.payload[at] = b'a' + rank as u8;
            cfg.put(at + NAME_BYTES, 10 * (rank as u32 + 1));
            cfg.put(at + NAME_BYTES + 4, 0);
        }
        cfg
    }

    #[test]
    fn a_won_game_grows_the_hall_of_fame_and_nobody_drops_out() {
        // The original drops its tenth entry for a winner, and a winner with more races than
        // the tenth never enters. Here every winner enters, sorted by races, fewest first.
        let mut cfg = best_ten();
        assert_eq!(cfg.hall_of_fame_len(), 10);
        assert_eq!(cfg.add_to_hall_of_fame(b"slow", 150, 2), 10);
        assert_eq!(cfg.add_to_hall_of_fame(b"Tom", 30, 1), 3);
        assert_eq!(cfg.hall_of_fame_len(), 12);
        assert_eq!(cfg.hall_of_fame(2), (&b"c"[..], 30, 0));
        assert_eq!(cfg.hall_of_fame(3), (&b"TOM"[..], 30, 1));
        assert_eq!(cfg.hall_of_fame(4), (&b"d"[..], 40, 0));
        assert_eq!(
            cfg.hall_of_fame(10),
            (&b"j"[..], 100, 0),
            "the old tenth kept"
        );
        assert_eq!(cfg.hall_of_fame(11), (&b"SLOW"[..], 150, 2));
        let bytes = cfg.to_bytes();
        assert_eq!(DrCfg::parse(&bytes).unwrap(), cfg);
        // The first ten stay in the original's slots, where the original reads them.
        let original = DrCfg::parse(&bytes[..file().len()]).unwrap();
        assert_eq!(original.hall_of_fame(3), (&b"TOM"[..], 30, 1));
        assert_eq!(original.hall_of_fame(9), (&b"i"[..], 90, 0));
    }

    #[test]
    fn entries_past_the_tenth_write_the_records_before_them_empty() {
        // The blocks after the original's bytes come in a fixed order; a file with only the
        // best ten's extra entries still reads its records as empty, not as names and times.
        let mut cfg = best_ten();
        cfg.add_to_hall_of_fame(b"late", 200, 0);
        let bytes = cfg.to_bytes();
        assert_eq!(
            bytes.len(),
            file().len() + ARENA_BYTES + RACE_RECORDS_BYTES + 4 + ENTRY_BYTES
        );
        let read = DrCfg::parse(&bytes).unwrap();
        assert_eq!(read.record(ARENA, 0), (&b""[..], [0, 0, 0]));
        assert_eq!(read.race_record(0, 4, 0), (&b""[..], [0, 0, 0]));
        assert_eq!(read.hall_of_fame(10), (&b"LATE"[..], 200, 0));
    }

    #[test]
    fn a_file_of_1_0_0_reads_and_writes_back_as_it_was() {
        // 1.0.0 wrote the Arena's lap records after the original's bytes and nothing more.
        let mut old = file();
        old.extend((0..ARENA_BYTES).map(|i| (i % 7) as u8 + b'A'));
        let cfg = DrCfg::parse(&old).unwrap();
        assert_eq!(cfg.hall_of_fame_len(), 10);
        assert_eq!(cfg.race_record(5, 6, 1), (&b""[..], [0, 0, 0]));
        assert_eq!(cfg.to_bytes(), old);
    }

    #[test]
    fn a_file_without_arena_records_stays_in_the_original_s_format() {
        // Until a lap is driven in the Arena, DeadRally's dr.cfg is the original's, byte for
        // byte, so the original can still read it.
        let cfg = DrCfg::parse(&file()).unwrap();
        assert_eq!(cfg.to_bytes(), file());
    }
}
