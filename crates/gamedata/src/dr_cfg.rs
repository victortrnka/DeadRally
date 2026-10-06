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
        Ok(DrCfg { header, payload })
    }

    /// A `dr.cfg` as `loadConfig` takes it: `None` when it is 7 bytes or shorter (the
    /// original then uses its defaults); else its header and payload, bytes it lacks 0 and
    /// extra bytes left out.
    pub fn parse(bytes: &[u8]) -> Option<DrCfg> {
        if bytes.len() <= SHORTEST {
            return None;
        }
        let mut header = [0; HEADER_BYTES];
        let in_header = bytes.len().min(HEADER_BYTES);
        header[..in_header].copy_from_slice(&bytes[..in_header]);
        let mut payload = bytes.get(HEADER_BYTES..).unwrap_or_default().to_vec();
        payload.resize(PAYLOAD_BYTES, 0);
        Some(DrCfg { header, payload })
    }

    /// The file as `saveConfiguration` writes it.
    pub fn to_bytes(&self) -> Vec<u8> {
        [&self.header[..], &self.payload].concat()
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

    /// Record `car` (0–5) of circuit `circuit` (0–17): the driver's name and the time
    /// (minutes, seconds, hundredths).
    pub fn record(&self, circuit: usize, car: usize) -> (&[u8], [u32; 3]) {
        let at = RECORDS + RECORD_BYTES * (circuit + 18 * car);
        let time = [0, 1, 2].map(|i| self.get(at + NAME_BYTES + 4 * i));
        (self.name(at, NAME_BYTES), time)
    }

    /// Entry `rank` (0–9) of the best ten: the name, races and difficulty.
    pub fn hall_of_fame(&self, rank: usize) -> (&[u8], i32, u32) {
        let at = HALL_OF_FAME + ENTRY_BYTES * rank;
        let races = self.get(at + NAME_BYTES) as i32;
        (
            self.name(at, NAME_BYTES),
            races,
            self.get(at + NAME_BYTES + 4),
        )
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
}
