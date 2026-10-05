# M2b Configure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Configure works as in the original (both volume popups, Define Keyboard, Define Gamepad, the gamepad switch), and the settings last in a `dr.cfg` byte-compatible with the original's, kept in DeadRally's own config directory.

**Architecture:** `deadrally-gamedata` gains `machine` (a tiny interpreter that runs the original's `defaultConfig` over `dr.exe` to get the default file), `dr_cfg` (the file's layout, loading and saving), Configure's texts and the slider pictures. `deadrally-core` takes the configuration in `Game::new`, hands the bytes out through `Game::take_config` whenever the original writes the file, and gains the Configure states of the menu scene. The frontend loads and writes DeadRally's `dr.cfg` and reports gamepad connections.

**Tech Stack:** Rust 1.99.0 (edition 2024); no new crates. Reference runs as in M2a.

**Spec:** `docs/superpowers/specs/2026-10-05-m2b-configure-design.md` (written without the owner, who asked for M2 to go on without questions and for the usage limit to be spared). Read it first.

## Global Constraints

- As in M2a: toolchain 1.99.0, edition 2024, no `unsafe`, overflow checks everywhere, no new dependencies or lint exceptions; the core's determinism bans; never quote the game's text (fixtures use made-up bytes); never commit game data, screenshots, recordings or the original's `dr.cfg` (it holds the original's names); hashes may be committed.
- **DeadRally never writes into the game folder.** Its `dr.cfg` lives next to `config.toml`. Any run of the frontend in a check gets its own absolute `XDG_CONFIG_HOME`, so it never touches the build machine's own settings.
- **Nothing reaches the speakers:** reference runs only through `scripts/reference-run.sh`; frontend runs only with SDL's disk audio driver and the sound servers unreachable.
- Fixed values (spec 3): the file is 8 header bytes and 2934 payload bytes at `saveConfiguration`'s offsets; the defaults come from running 0x426700..0x4271E8; volume levels are the volume / 512, 0..=128 in steps of 2; key 0xAA is never taken; Define Gamepad looks at the gamepad after 15 polls and hides it from key reads while it waits.
- Commits as in M2a: prefixes, subject at most 50 characters, a `- ` list body only when several things changed, no attribution.

## Review Focus

1. **A `dr.cfg` that is short, long, unreadable or from another version** → short files give the defaults (as the original), missing bytes are 0, an unreadable file stops the start rather than be overwritten. Task 1, `a_file_that_cannot_be_read_is_an_error_not_the_defaults` and the parse tests.
2. **A `dr.exe` whose `defaultConfig` is other code** → a load error naming the address, never made-up defaults. Task 1, `an_instruction_it_does_not_know_stops_the_run_with_its_address`.
3. **The game folder is never written** → Task 1, `deadrallys_own_file_comes_first_then_the_games_once_then_the_defaults`.
4. **A gamepad button pressed in Define Gamepad** → taken as the input, not as Enter or Escape. Task 3, `define_gamepad_waits_for_the_pad_to_settle_and_enter_means_none`.
5. **Leaving Configure any way writes `dr.cfg`; Escape keeps the row, "previous menu" resets it** → Task 3, `configure_opens_over_the_dimmed_main_menu_and_escape_returns_writing_dr_cfg` and `previous_menu_returns_and_starts_configure_over_at_its_first_row`.

## Before You Start

As in M2a's plan: `/home/trashcan/DeadRally`, branch `m2b-menus` (spec and plan committed); `export DEADRALLY_DATA=~/games/DeathRally`; apply code blocks with `/tmp/apply-plan.py` (its source is in `docs/superpowers/plans/2026-10-05-m2a-main-menu.md`, "Applying code blocks"); every expected output below is real, from 2026-10-05.

---

### Task 1: `dr.cfg` and the original's defaults

Spec 2 (decisions 2, 5) and 3.1.

**Files:**
- Create: `crates/gamedata/src/machine.rs`, `crates/gamedata/src/dr_cfg.rs`
- Modify: `crates/gamedata/src/exe.rs` (`image_byte`), `crates/gamedata/src/lib.rs`

**Interfaces:**
- Produces: `machine::{Machine, MachineError}` (`Machine::new(&Exe)`, `run(start, end)`, `byte`, `bytes`); `dr_cfg::{DrCfg, HEADER_BYTES, PAYLOAD_BYTES, KEY_COUNT = 8, PAD_COUNT = 7, own_path, load, save}`; `DrCfg::{defaults(&Exe), parse(&[u8]) -> Option<DrCfg>, to_bytes, music_volume, effects_volume, use_joystick, key(i), pad(i), times_played}` and their setters.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/gamedata/src/machine.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::exe::tests::build_at;

    /// Code at 0x401000 and data from 0x401100, in a section of 0x1000 bytes.
    fn exe(code: &[u8], data: &[u8]) -> Exe {
        let mut section = code.to_vec();
        section.resize(0x100, 0xCC);
        section.extend_from_slice(data);
        Exe::parse(build_at(0x1000, 0x1000, &section)).unwrap()
    }

    #[test]
    fn stores_registers_loops_and_a_tail_call_leave_the_values_the_code_writes() {
        // The shapes defaultConfig uses: immediates and registers stored at absolute
        // addresses, a counted loop writing through a pointer, a copy from read-only data,
        // and the jump to saveConfiguration that ends it.
        let code = [
            0xC7, 0x05, 0x00, 0x12, 0x40, 0x00, 0x00, 0x80, 0x00,
            0x00, // mov [0x401200], 0x8000
            0xBA, 0x01, 0x00, 0x00, 0x00, // mov edx, 1
            0x89, 0x15, 0x04, 0x12, 0x40, 0x00, // mov [0x401204], edx
            0xBE, 0x10, 0x12, 0x40, 0x00, // mov esi, 0x401210
            0xBF, 0x03, 0x00, 0x00, 0x00, // mov edi, 3
            0x8B, 0x0D, 0x00, 0x11, 0x40, 0x00, // loop: mov ecx, [0x401100]
            0x89, 0x0E, // mov [esi], ecx
            0x83, 0xC6, 0x04, // add esi, 4
            0x4F, // dec edi
            0x75, 0xF2, // jne loop
            0xC6, 0x05, 0x20, 0x12, 0x40, 0x00, 0x2A, // mov byte [0x401220], 42
            0xE9, 0x00, 0x00, 0x10, 0x00, // jmp far away
        ];
        let exe = exe(&code, &[0xEF, 0xBE, 0xAD, 0xDE]);
        let mut machine = Machine::new(&exe);
        machine.run(0x40_1000, 0x40_1100).unwrap();
        assert_eq!(machine.bytes(0x40_1200, 4).unwrap(), [0x00, 0x80, 0, 0]);
        assert_eq!(machine.bytes(0x40_1204, 4).unwrap(), [1, 0, 0, 0]);
        assert_eq!(
            machine.bytes(0x40_1210, 12).unwrap(),
            [0xEF, 0xBE, 0xAD, 0xDE].repeat(3),
            "three turns of the loop"
        );
        assert_eq!(
            machine.bytes(0x40_121C, 4).unwrap(),
            [0; 4],
            "and no fourth"
        );
        assert_eq!(machine.byte(0x40_1220), Ok(42));
    }

    #[test]
    fn an_instruction_it_does_not_know_stops_the_run_with_its_address() {
        // Another dr.exe has other code there; running it blind would make up defaults.
        let exe = exe(&[0x90, 0x0F, 0x0B], &[]);
        assert_eq!(
            Machine::new(&exe).run(0x40_1000, 0x40_1100),
            Err(MachineError::Unknown(0x40_1000))
        );
    }

    #[test]
    fn code_that_never_leaves_is_stopped() {
        let exe = exe(&[0xEB, 0xFE], &[]);
        assert_eq!(
            Machine::new(&exe).run(0x40_1000, 0x40_1100),
            Err(MachineError::Runaway)
        );
    }
}
```

<!-- write: crates/gamedata/src/dr_cfg.rs -->
```rust
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
}
```

<!-- write: crates/gamedata/src/lib.rs -->
```rust
//! Finds, validates and decodes the player's copy of the original game data (M0 spec section 6,
//! M1a spec section 4, M1b spec section 4.1, M2a spec section 4.1).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

pub mod assets;
pub mod bmp;
pub mod bpa;
pub mod bpk;
pub mod catalog;
pub mod cmf;
mod config;
pub mod dr_cfg;
pub mod exe;
pub mod haf;
pub mod image;
mod known_versions;
mod locate;
mod lzw;
pub mod machine;
pub mod s3m;
pub mod sound;
pub mod text;
pub mod track;
mod validate;
pub mod xm;

pub use config::{Config, ConfigError, config_path, load_config};
pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use locate::{DATA_ENV_VAR, DataSource, LocateError, Located, STEAM_SUBDIR, locate};
pub use lzw::LzwError;
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-gamedata --lib -- machine dr_cfg`
Expected: FAIL to compile (`Machine`, `DrCfg` and friends do not exist yet).

- [ ] **Step 3: Implement**

<!-- write: crates/gamedata/src/exe.rs -->
```rust
//! The Windows `dr.exe` as a data file (spec M2a §3.1, §4.1): its strings and tables are read
//! at the virtual addresses the original uses. It is only read, never run.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExeError {
    /// Not a 32-bit PE file, or its headers do not hold together.
    NotPe(String),
    /// Nothing of the file lies at this address.
    Address(u32),
    /// The string at this address does not end within the bytes allowed for it.
    Unterminated(u32),
}

impl fmt::Display for ExeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExeError::NotPe(problem) => write!(f, "not a Windows executable: {problem}"),
            ExeError::Address(address) => {
                write!(f, "the executable has no data at address {address:#x}")
            }
            ExeError::Unterminated(address) => {
                write!(f, "the string at address {address:#x} does not end")
            }
        }
    }
}

impl std::error::Error for ExeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Section {
    /// Virtual address (image base included) and size in memory.
    address: u32,
    virtual_size: u32,
    /// Where its bytes are in the file, and how many the file holds.
    file_offset: usize,
    file_size: usize,
}

/// A PE file's bytes and its sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exe {
    bytes: Vec<u8>,
    sections: Vec<Section>,
}

impl Exe {
    /// # Errors
    ///
    /// [`ExeError::NotPe`] when the bytes are not a PE file with 32-bit optional headers.
    pub fn parse(bytes: Vec<u8>) -> Result<Exe, ExeError> {
        let bad = |problem: &str| ExeError::NotPe(problem.to_owned());
        let u16_at = |offset: usize| {
            bytes
                .get(offset..offset + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or_else(|| bad("the headers run past the end"))
        };
        let u32_at = |offset: usize| {
            bytes
                .get(offset..offset + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| bad("the headers run past the end"))
        };
        if !bytes.starts_with(b"MZ") {
            return Err(bad("no MZ header"));
        }
        let pe = u32_at(0x3C)? as usize;
        if bytes.get(pe..pe + 4) != Some(b"PE\0\0".as_slice()) {
            return Err(bad("no PE signature"));
        }
        let section_count = usize::from(u16_at(pe + 6)?);
        let optional_size = usize::from(u16_at(pe + 20)?);
        let optional = pe + 24;
        if u16_at(optional)? != 0x10B {
            return Err(bad("not a 32-bit executable"));
        }
        let image_base = u32_at(optional + 28)?;
        let table = optional + optional_size;
        let sections = (0..section_count)
            .map(|index| {
                let at = table + 40 * index;
                Ok(Section {
                    address: image_base
                        .checked_add(u32_at(at + 12)?)
                        .ok_or_else(|| bad("a section lies past 4 GB"))?,
                    virtual_size: u32_at(at + 8)?,
                    file_offset: u32_at(at + 20)? as usize,
                    file_size: u32_at(at + 16)? as usize,
                })
            })
            .collect::<Result<Vec<_>, ExeError>>()?;
        Ok(Exe { bytes, sections })
    }

    /// The file's bytes from virtual address `address` to the end of its section.
    fn rest_of_section(&self, address: u32) -> Option<&[u8]> {
        self.sections.iter().find_map(|section| {
            let offset = address.checked_sub(section.address)? as usize;
            let end = section.file_size.min(section.virtual_size as usize);
            (offset < end)
                .then(|| {
                    self.bytes
                        .get(section.file_offset + offset..section.file_offset + end)
                })
                .flatten()
        })
    }

    /// The `length` bytes at virtual address `address`, which must lie in one section's bytes
    /// in the file.
    ///
    /// # Errors
    ///
    /// [`ExeError::Address`] when they do not.
    pub fn bytes_at(&self, address: u32, length: usize) -> Result<&[u8], ExeError> {
        self.rest_of_section(address)
            .and_then(|rest| rest.get(..length))
            .ok_or(ExeError::Address(address))
    }

    /// The byte at `address` as the loaded image holds it: the file's byte, or 0 past the
    /// file's bytes within the section's size in memory; `None` outside every section.
    pub(crate) fn image_byte(&self, address: u32) -> Option<u8> {
        self.sections.iter().find_map(|section| {
            let offset = address.checked_sub(section.address)? as usize;
            if offset >= section.virtual_size as usize {
                return None;
            }
            Some(if offset < section.file_size {
                self.bytes
                    .get(section.file_offset + offset)
                    .copied()
                    .unwrap_or(0)
            } else {
                0
            })
        })
    }

    /// The NUL-terminated string at `address`, without its NUL, at most `max` bytes long.
    ///
    /// # Errors
    ///
    /// [`ExeError::Address`] when nothing lies there, [`ExeError::Unterminated`] when no NUL
    /// follows within `max` bytes.
    pub fn string_at(&self, address: u32, max: usize) -> Result<&[u8], ExeError> {
        let rest = self
            .rest_of_section(address)
            .ok_or(ExeError::Address(address))?;
        let window = &rest[..rest.len().min(max.saturating_add(1))];
        let end = window
            .iter()
            .position(|&b| b == 0)
            .ok_or(ExeError::Unterminated(address))?;
        Ok(&window[..end])
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal 32-bit PE: image base 0x400000 and one section at 0x401000 (virtual size
    /// 0x200) whose file bytes, at offset 0x200, are `data`.
    pub(crate) fn build(data: &[u8]) -> Vec<u8> {
        build_at(0x1000, 0x200, data)
    }

    /// Like [`build`], with the section at relative address `rva` and `virtual_size` long.
    pub(crate) fn build_at(rva: u32, virtual_size: u32, data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x200];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xE0u16.to_le_bytes());
        let optional = 0x98;
        bytes[optional..optional + 2].copy_from_slice(&0x10Bu16.to_le_bytes());
        bytes[optional + 28..optional + 32].copy_from_slice(&0x40_0000u32.to_le_bytes());
        let section = optional + 0xE0;
        bytes[section..section + 5].copy_from_slice(b".data");
        bytes[section + 8..section + 12].copy_from_slice(&virtual_size.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&rva.to_le_bytes());
        bytes[section + 16..section + 20]
            .copy_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());
        bytes.extend_from_slice(data);
        bytes
    }

    #[test]
    fn strings_are_read_at_their_virtual_address() {
        // The game's texts are found by the addresses the original uses; reading the file
        // offset instead would land 0x400000 bytes off.
        let exe = Exe::parse(build(b"xyHello\0World\0")).unwrap();
        assert_eq!(exe.string_at(0x40_1002, 50).unwrap(), b"Hello");
        assert_eq!(exe.string_at(0x40_1008, 50).unwrap(), b"World");
        assert_eq!(exe.bytes_at(0x40_1000, 2).unwrap(), b"xy");
    }

    #[test]
    fn addresses_outside_the_file_and_unterminated_strings_are_errors() {
        // A different release keeps its strings elsewhere; reading garbage would show it.
        let exe = Exe::parse(build(b"abc\0defg")).unwrap();
        assert_eq!(
            exe.string_at(0x40_0000, 10),
            Err(ExeError::Address(0x40_0000))
        );
        assert_eq!(
            exe.string_at(0x40_1004, 10),
            Err(ExeError::Unterminated(0x40_1004))
        );
        assert_eq!(
            exe.string_at(0x40_1000, 2),
            Err(ExeError::Unterminated(0x40_1000))
        );
        assert_eq!(
            exe.bytes_at(0x40_1006, 4),
            Err(ExeError::Address(0x40_1006))
        );
    }

    #[test]
    fn other_files_are_not_executables() {
        assert!(matches!(
            Exe::parse(b"BM".to_vec()),
            Err(ExeError::NotPe(_))
        ));
        let mut no_pe = build(b"");
        no_pe[0x80] = b'X';
        assert!(matches!(Exe::parse(no_pe), Err(ExeError::NotPe(_))));
    }
}
```

<!-- prepend: crates/gamedata/src/machine.rs -->
```rust
//! Runs a straight piece of the original's machine code over its own data (spec M2b §3.1).
//! `defaultConfig` (0x426700) writes the default settings, records and Hall of Fame into the
//! original's variables, with the values in its instructions; running it gives DeadRally those
//! defaults from the player's `dr.exe` without copying any of them. Only the few 32-bit
//! instructions such code uses are known; anything else stops the run with its address.

use std::collections::BTreeMap;
use std::fmt;

use crate::exe::Exe;

/// Where the stack lives: below anything the original maps.
const STACK_TOP: u32 = 0x0010_0000;
/// A run that takes more steps than this is not the known code.
const MAX_STEPS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MachineError {
    /// An instruction this machine does not know, at this address.
    Unknown(u32),
    /// A read from an address no section holds.
    Address(u32),
    /// The code ran too long without leaving.
    Runaway,
}

impl fmt::Display for MachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MachineError::Unknown(address) => {
                write!(
                    f,
                    "dr.exe: unexpected code at address {address:#x}; is this the known release?"
                )
            }
            MachineError::Address(address) => {
                write!(
                    f,
                    "dr.exe: the code reads address {address:#x}, which holds nothing"
                )
            }
            MachineError::Runaway => write!(
                f,
                "dr.exe: the code does not end; is this the known release?"
            ),
        }
    }
}

impl std::error::Error for MachineError {}

/// Where an operand is: a register (by number) or a memory address.
#[derive(Clone, Copy, Debug)]
enum Place {
    Register(usize),
    Memory(u32),
}

/// The original's memory as the code leaves it: its image, with the bytes the code wrote.
#[derive(Debug)]
pub struct Machine<'a> {
    exe: &'a Exe,
    written: BTreeMap<u32, u8>,
    /// eax, ecx, edx, ebx, esp, ebp, esi, edi.
    registers: [u32; 8],
    zero: bool,
    sign: bool,
    overflow: bool,
}

impl<'a> Machine<'a> {
    pub fn new(exe: &'a Exe) -> Machine<'a> {
        let mut registers = [0; 8];
        registers[4] = STACK_TOP;
        Machine {
            exe,
            written: BTreeMap::new(),
            registers,
            zero: false,
            sign: false,
            overflow: false,
        }
    }

    /// The byte at `address` as the code left it.
    ///
    /// # Errors
    ///
    /// [`MachineError::Address`] when no section holds it.
    pub fn byte(&self, address: u32) -> Result<u8, MachineError> {
        match self.written.get(&address) {
            Some(&byte) => Ok(byte),
            None => self
                .exe
                .image_byte(address)
                .ok_or(MachineError::Address(address)),
        }
    }

    /// `length` bytes from `address`.
    ///
    /// # Errors
    ///
    /// [`MachineError::Address`] when one is outside every section.
    pub fn bytes(&self, address: u32, length: usize) -> Result<Vec<u8>, MachineError> {
        (0..length as u32)
            .map(|i| self.byte(address.wrapping_add(i)))
            .collect()
    }

    fn read(&self, address: u32, size: usize) -> Result<u32, MachineError> {
        let mut value = 0;
        for i in (0..size).rev() {
            value = value << 8 | u32::from(self.byte(address.wrapping_add(i as u32))?);
        }
        Ok(value)
    }

    fn write(&mut self, address: u32, size: usize, value: u32) {
        for i in 0..size {
            self.written
                .insert(address.wrapping_add(i as u32), (value >> (8 * i)) as u8);
        }
    }

    fn get(&self, place: Place, size: usize) -> Result<u32, MachineError> {
        match place {
            Place::Memory(address) => self.read(address, size),
            Place::Register(r) => Ok(match size {
                4 => self.registers[r],
                2 => self.registers[r] & 0xFFFF,
                // al, cl, dl, bl, then ah, ch, dh, bh.
                _ if r < 4 => self.registers[r] & 0xFF,
                _ => (self.registers[r - 4] >> 8) & 0xFF,
            }),
        }
    }

    fn set(&mut self, place: Place, size: usize, value: u32) {
        match place {
            Place::Memory(address) => self.write(address, size, value),
            Place::Register(r) => match size {
                4 => self.registers[r] = value,
                2 => self.registers[r] = self.registers[r] & !0xFFFF | value & 0xFFFF,
                _ if r < 4 => self.registers[r] = self.registers[r] & !0xFF | value & 0xFF,
                _ => {
                    self.registers[r - 4] = self.registers[r - 4] & !0xFF00 | (value & 0xFF) << 8;
                }
            },
        }
    }

    fn flags(&mut self, result: u32, overflow: bool) {
        self.zero = result == 0;
        self.sign = result & 0x8000_0000 != 0;
        self.overflow = overflow;
    }

    /// Runs the code from `start` until it returns or jumps out of `start..end`, as a tail
    /// call does.
    ///
    /// # Errors
    ///
    /// [`MachineError`] for an instruction this machine does not know, a read outside the
    /// image, or a run that does not end.
    pub fn run(&mut self, start: u32, end: u32) -> Result<(), MachineError> {
        let mut at = start;
        for _ in 0..MAX_STEPS {
            if !(start..end).contains(&at) {
                return Ok(());
            }
            match self.step(at)? {
                Some(next) => at = next,
                None => return Ok(()),
            }
        }
        Err(MachineError::Runaway)
    }

    /// Decodes a ModRM byte (and its SIB and displacement) at `at`: the `reg` field, the
    /// operand, and the address after them.
    fn modrm(&self, at: u32) -> Result<(usize, Place, u32), MachineError> {
        let modrm = self.byte(at)?;
        let (mode, reg, rm) = (
            modrm >> 6,
            usize::from(modrm >> 3 & 7),
            usize::from(modrm & 7),
        );
        let mut next = at + 1;
        if mode == 3 {
            return Ok((reg, Place::Register(rm), next));
        }
        let mut base = if rm == 4 {
            let sib = self.byte(next)?;
            next += 1;
            let (scale, index, sib_base) =
                (sib >> 6, usize::from(sib >> 3 & 7), usize::from(sib & 7));
            if sib_base == 5 && mode == 0 {
                return Err(MachineError::Unknown(at));
            }
            let indexed = if index == 4 {
                0
            } else {
                self.registers[index] << scale
            };
            self.registers[sib_base].wrapping_add(indexed)
        } else if mode == 0 && rm == 5 {
            let address = self.read(next, 4)?;
            return Ok((reg, Place::Memory(address), next + 4));
        } else {
            self.registers[rm]
        };
        match mode {
            1 => {
                base = base.wrapping_add(self.byte(next)? as i8 as u32);
                next += 1;
            }
            2 => {
                base = base.wrapping_add(self.read(next, 4)?);
                next += 4;
            }
            _ => {}
        }
        Ok((reg, Place::Memory(base), next))
    }

    /// Executes the instruction at `at`; returns the next one's address, `None` after `ret`.
    fn step(&mut self, at: u32) -> Result<Option<u32>, MachineError> {
        let (size, at_op) = if self.byte(at)? == 0x66 {
            (2, at + 1)
        } else {
            (4, at)
        };
        let opcode = self.byte(at_op)?;
        let unknown = Err(MachineError::Unknown(at));
        let next = match opcode {
            // mov r/m, r and mov r, r/m (8, 16 and 32 bits).
            0x88..=0x8B => {
                let width = if opcode & 1 == 0 { 1 } else { size };
                let (reg, place, next) = self.modrm(at_op + 1)?;
                if opcode & 2 == 0 {
                    let value = self.get(Place::Register(reg), width)?;
                    self.set(place, width, value);
                } else {
                    let value = self.get(place, width)?;
                    self.set(Place::Register(reg), width, value);
                }
                next
            }
            // mov r/m, imm.
            0xC6 | 0xC7 => {
                let width = if opcode == 0xC6 { 1 } else { size };
                let (_, place, next) = self.modrm(at_op + 1)?;
                let value = self.read(next, width)?;
                self.set(place, width, value);
                next + width as u32
            }
            // mov al/ax/eax, [moffs] and back.
            0xA0..=0xA3 => {
                let width = if opcode & 1 == 0 { 1 } else { size };
                let address = self.read(at_op + 1, 4)?;
                if opcode & 2 == 0 {
                    let value = self.read(address, width)?;
                    self.set(Place::Register(0), width, value);
                } else {
                    let value = self.get(Place::Register(0), width)?;
                    self.write(address, width, value);
                }
                at_op + 5
            }
            0xB8..=0xBF => {
                let value = self.read(at_op + 1, size)?;
                self.set(Place::Register(usize::from(opcode - 0xB8)), size, value);
                at_op + 1 + size as u32
            }
            0x0F if self.byte(at_op + 1)? == 0xB6 => {
                let (reg, place, next) = self.modrm(at_op + 2)?;
                let value = self.get(place, 1)?;
                self.set(Place::Register(reg), 4, value);
                next
            }
            0x8D => {
                let (reg, place, next) = self.modrm(at_op + 1)?;
                let Place::Memory(address) = place else {
                    return unknown;
                };
                self.set(Place::Register(reg), 4, address);
                next
            }
            // add, xor (r, r/m) and add (r/m, r).
            0x01 | 0x03 | 0x31 | 0x33 => {
                let (reg, place, next) = self.modrm(at_op + 1)?;
                let (target, source) = if opcode & 2 == 0 {
                    (place, Place::Register(reg))
                } else {
                    (Place::Register(reg), place)
                };
                let (a, b) = (self.get(target, 4)?, self.get(source, 4)?);
                let (result, overflow) = if opcode < 0x30 {
                    (
                        a.wrapping_add(b),
                        (a as i32).checked_add(b as i32).is_none(),
                    )
                } else {
                    (a ^ b, false)
                };
                self.set(target, 4, result);
                self.flags(result, overflow);
                next
            }
            // add, sub, cmp r/m, imm32 or imm8.
            0x81 | 0x83 => {
                let (operation, place, next) = self.modrm(at_op + 1)?;
                let (b, next) = if opcode == 0x81 {
                    (self.read(next, 4)?, next + 4)
                } else {
                    (self.byte(next)? as i8 as u32, next + 1)
                };
                let a = self.get(place, 4)?;
                let (result, overflow) = match operation {
                    0 => (
                        a.wrapping_add(b),
                        (a as i32).checked_add(b as i32).is_none(),
                    ),
                    5 | 7 => (
                        a.wrapping_sub(b),
                        (a as i32).checked_sub(b as i32).is_none(),
                    ),
                    _ => return unknown,
                };
                if operation != 7 {
                    self.set(place, 4, result);
                }
                self.flags(result, overflow);
                next
            }
            0x40..=0x4F => {
                let r = usize::from(opcode & 7);
                let a = self.registers[r];
                let (result, overflow) = if opcode < 0x48 {
                    (a.wrapping_add(1), a == 0x7FFF_FFFF)
                } else {
                    (a.wrapping_sub(1), a == 0x8000_0000)
                };
                self.registers[r] = result;
                self.flags(result, overflow);
                at_op + 1
            }
            0x50..=0x57 => {
                let value = self.registers[usize::from(opcode - 0x50)];
                self.registers[4] = self.registers[4].wrapping_sub(4);
                self.write(self.registers[4], 4, value);
                at_op + 1
            }
            0x58..=0x5F => {
                let value = self.read(self.registers[4], 4)?;
                self.registers[4] = self.registers[4].wrapping_add(4);
                self.registers[usize::from(opcode - 0x58)] = value;
                at_op + 1
            }
            0x74 | 0x75 | 0x7C | 0xEB => {
                let target = (at_op + 2).wrapping_add(self.byte(at_op + 1)? as i8 as u32);
                let taken = match opcode {
                    0x74 => self.zero,
                    0x75 => !self.zero,
                    0x7C => self.sign != self.overflow,
                    _ => true,
                };
                if taken { target } else { at_op + 2 }
            }
            0xE9 => (at_op + 5).wrapping_add(self.read(at_op + 1, 4)?),
            0xC3 => return Ok(None),
            _ => return unknown,
        };
        Ok(Some(next))
    }
}
```

<!-- prepend: crates/gamedata/src/dr_cfg.rs -->
```rust
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
const USE_JOYSTICK: usize = 0x10;
/// Accelerate, brake, left, right, turbo, gun, mine, horn: set-1 scancodes.
const KEYS: usize = 0xB36;
pub const KEY_COUNT: usize = 8;
/// Accelerate, brake, left, right, turbo, gun, mine: gamepad inputs (0 none, 1–4 the
/// stick's left, right, up, down, 5–8 buttons 1–4).
const PADS: usize = 0xB56;
pub const PAD_COUNT: usize = 7;
const TIMES_PLAYED: usize = 0xB72;

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

    pub fn times_played(&self) -> u32 {
        self.get(TIMES_PLAYED)
    }

    pub fn set_times_played(&mut self, times: u32) {
        self.put(TIMES_PLAYED, times);
    }
}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (279 passed, 17 ignored).

- [ ] **Step 5: Commit**

```bash
git add crates/gamedata
git commit -m "feat: read and write the original's dr.cfg"
```

---

### Task 2: Configure's texts and pictures

Spec 3.2 and 4. The texts are strings of `dr.exe` at fixed addresses; key names lie 16 bytes apart. `Assets` gains the slider, its knob and the default `dr.cfg`.

**Files:**
- Modify: `crates/gamedata/src/text.rs`, `crates/gamedata/src/assets.rs`, `crates/gamedata/tests/catalog_data.rs`, `crates/core/src/menu/draw.rs` (test fixture), `crates/core/tests/common/mod.rs`

**Interfaces:**
- Consumes: `DrCfg::defaults` (Task 1).
- Produces: `text::{ConfigureTexts, PAD_INPUTS}`, `Texts { configure, .. }`; `MenuAssets { slider, knob, default_config, .. }`; `AssetError::Machine`.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/gamedata/tests/catalog_data.rs -->
```rust
//! The decoders and the catalogue against the developer's real game data. Run with
//! `cargo test-data`; the tests read DEADRALLY_DATA and fail (never pass silently) when it is
//! unset.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::catalog::{self, Layout};
use deadrally_gamedata::haf::{Animation, FRAME_HEIGHT, FRAME_WIDTH};
use deadrally_gamedata::track::TrackInfo;
use deadrally_gamedata::{DATA_ENV_VAR, bmp, locate};
use sha2::{Digest, Sha256};

const ARCHIVES: [&str; 13] = [
    "ENGINE.BPA",
    "IBFILES.BPA",
    "MENU.BPA",
    "TR0.BPA",
    "TR1.BPA",
    "TR2.BPA",
    "TR3.BPA",
    "TR4.BPA",
    "TR5.BPA",
    "TR6.BPA",
    "TR7.BPA",
    "TR8.BPA",
    "TR9.BPA",
];

fn data_dir() -> PathBuf {
    let dir = match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    };
    locate(Some(&dir), None, None)
        .unwrap_or_else(|error| panic!("{error}"))
        .validation
        .dir
}

fn archive(name: &str) -> Archive {
    Archive::open(&data_dir().join(name)).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_archive_has_its_documented_entries() {
    // Opening checks that the sizes add up to the file; the counts catch a misread directory.
    let counts = [
        ("ENGINE.BPA", 39),
        ("IBFILES.BPA", 17),
        ("MENU.BPA", 167),
        ("MUSICS.BPA", 16),
    ];
    for (name, count) in counts {
        assert_eq!(archive(name).names().count(), count, "{name}");
    }
    assert_eq!(archive("TR0.BPA").names().count(), 12);
    for track in 1..10 {
        assert_eq!(
            archive(&format!("TR{track}.BPA")).names().count(),
            13,
            "TR{track}"
        );
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_animations_have_their_documented_frames_and_length() {
    // The intro's length in ticks is what its music (M1b) is cut to.
    for (name, frames, ticks) in [
        ("SANIM.haf", 1626, 5732),
        ("ENDANI.haf", 368, 1620),
        ("ENDANI0.HAF", 383, 1915),
    ] {
        let animation = Animation::open(&data_dir().join(name)).unwrap();
        assert_eq!(animation.len(), frames, "{name}");
        let total: u32 = animation.delays.iter().map(|&delay| u32::from(delay)).sum();
        assert_eq!(total, ticks, "{name}");
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_bpk_entry_is_catalogued_or_explained() {
    // An uncatalogued image could never be drawn or dumped; the gap would surface only when a
    // later milestone needs it.
    let mut missing = Vec::new();
    for name in ARCHIVES {
        for entry in archive(name).names() {
            let explained = catalog::NOT_IMAGES
                .iter()
                .any(|not| not.archive == name && not.name == entry);
            if entry.ends_with(".BPK") && catalog::find(name, entry).is_none() && !explained {
                missing.push(format!("{name}/{entry}"));
            }
        }
    }
    assert!(missing.is_empty(), "uncatalogued: {missing:?}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_catalogued_image_decodes_to_its_shape() {
    // A wrong width with the right total size is caught by the dump and the reference
    // screenshots; a wrong total size is caught here.
    let mut failures = Vec::new();
    for name in ARCHIVES {
        let archive = archive(name);
        for entry in catalog::IMAGES.iter().filter(|entry| entry.archive == name) {
            let bytes = archive
                .read(entry.name)
                .unwrap_or_else(|error| panic!("{error}"));
            match entry.decode(bytes) {
                Ok(frames) if frames.len() == entry.frames as usize => {}
                Ok(frames) => failures.push(format!("{}: {} frames", entry.name, frames.len())),
                Err(error) => failures.push(format!("{}: {error}", entry.name)),
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn track_images_match_their_info_files() {
    // The race (M4) takes the track size from INF.BIN; the images must agree with it.
    for track in 0..10 {
        let archive = archive(&format!("TR{track}.BPA"));
        let info = TrackInfo::parse(archive.read(&format!("TR{track}-INF.BIN")).unwrap()).unwrap();
        for part in ["IMA", "MAS", "VAI", "LR1"] {
            let entry =
                catalog::find(&format!("TR{track}.BPA"), &format!("TR{track}-{part}.BPK")).unwrap();
            assert_eq!(entry.layout, Layout::Rix3Track);
            let divisor = if matches!(part, "VAI" | "LR1") { 4 } else { 1 };
            assert_eq!(
                (entry.width, entry.height),
                (info.width / divisor, info.height / divisor),
                "TR{track}-{part}"
            );
        }
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_startup_assets_load_with_their_documented_shapes() {
    // The game refuses to start when these fail, so a wrong shape here is a broken start.
    let dir = data_dir();
    let validation = deadrally_gamedata::validate(&dir).unwrap();
    let assets = deadrally_gamedata::assets::Assets::load(&validation)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(assets.intro.len(), 1626);
    let size =
        |picture: &deadrally_gamedata::assets::Picture| (picture.image.width, picture.image.height);
    assert_eq!(size(&assets.letterbox), (320, 200));
    assert_eq!(size(&assets.apogee), (640, 480));
    assert_eq!(size(&assets.remedy), (640, 480));
    assert_eq!(size(&assets.title), (640, 480));
    assert_eq!(assets.intro_music.orders.len(), 42);
    assert_eq!(assets.intro_effects.instruments.len(), 40);
    assert_eq!(assets.menu_music.orders.len(), 94);
    let menu = &assets.menu;
    assert_eq!((menu.background.width, menu.background.height), (640, 480));
    assert_eq!((menu.panel_line.width, menu.panel_line.height), (640, 10));
    assert_eq!(
        (menu.corners_focused.len(), menu.corners_unfocused.len()),
        (4, 4)
    );
    assert_eq!(menu.cursor.len(), 50);
    for font in [
        &menu.big_a,
        &menu.big_b,
        &menu.big_d,
        &menu.small_a,
        &menu.small_b,
        &menu.small_c,
    ] {
        assert_eq!(font.len(), 96);
    }
    assert_eq!(menu.background_copper.len(), 512);
    assert_eq!(menu.credits.len(), 2);
    assert_eq!(size(&menu.end), (640, 480));
    assert_eq!(menu.effects.instruments.len(), 31);
    // Row counts of the main menu and the start submenu, from dr.exe.
    let rows = |m: usize| {
        menu.texts.menus[m]
            .iter()
            .filter(|row| !row.is_empty())
            .count()
    };
    assert_eq!((rows(0), rows(1)), (6, 6));
    assert_eq!(menu.texts.panel.len(), 4);
    assert_eq!((menu.texts.big.width, menu.texts.small.height), (32, 16));
    assert_eq!((menu.slider.width, menu.slider.height), (172, 24));
    assert_eq!((menu.knob.width, menu.knob.height), (10, 24));
    let configure = &menu.texts.configure;
    assert_eq!(
        (configure.key_names.len(), configure.pad_names.len()),
        (256, 9)
    );
    // The defaults dr.exe's defaultConfig writes, as a fresh dr.cfg of the original holds
    // them (docs/verification/m2b.md): A, Z, the arrows, left shift, left control, left alt
    // and space; button 1, down, left, right, buttons 2 to 4; gamepad off.
    let defaults = &menu.default_config;
    assert_eq!(
        (defaults.music_volume(), defaults.effects_volume()),
        (0x8000, 0xC000)
    );
    assert_eq!(
        (0..8).map(|c| defaults.key(c)).collect::<Vec<_>>(),
        [0x1E, 0x2C, 0xCB, 0xCD, 0x2A, 0x1D, 0x38, 0x39]
    );
    assert_eq!(
        (0..7).map(|c| defaults.pad(c)).collect::<Vec<_>>(),
        [5, 4, 1, 2, 6, 7, 8]
    );
    assert_eq!((defaults.use_joystick(), defaults.times_played()), (0, 0));
}

/// One line per decoded picture: the SHA-256 of its frames' pixels (palettes first where the
/// file stores them), its name and its shape as width x height x frames. The shape is part of
/// the picture: the same bytes at another width draw a skewed sprite.
fn manifest(dir: &Path) -> String {
    let mut lines = String::new();
    let mut line = |name: &str, shape: (u32, u32, usize), hasher: Sha256| {
        let hash: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let (width, height, frames) = shape;
        writeln!(lines, "{hash}  {name} {width}x{height}x{frames}").unwrap();
    };
    for name in ARCHIVES {
        let archive = archive(name);
        for entry in catalog::IMAGES.iter().filter(|entry| entry.archive == name) {
            let bytes = archive.read(entry.name).unwrap();
            let mut hasher = Sha256::new();
            if let Some(palette) = entry.embedded_palette(bytes).unwrap() {
                hasher.update(palette.0.as_flattened());
            }
            let frames = entry.decode(bytes).unwrap();
            for frame in &frames {
                hasher.update(&frame.pixels);
            }
            let shape = (frames[0].width, frames[0].height, frames.len());
            line(&format!("{name}/{}", entry.name), shape, hasher);
        }
    }
    for name in ["SANIM.haf", "ENDANI.haf", "ENDANI0.HAF"] {
        let animation = Animation::open(&dir.join(name)).unwrap();
        let mut hasher = Sha256::new();
        for index in 0..animation.len() {
            let frame = animation
                .frame(index)
                .unwrap_or_else(|error| panic!("{error}"));
            hasher.update(frame.palette.0.as_flattened());
            hasher.update(&frame.pixels);
        }
        line(name, (FRAME_WIDTH, FRAME_HEIGHT, animation.len()), hasher);
    }
    for name in ["rmd.bmp", "end.bmp"] {
        let (image, palette) = bmp::decode(&std::fs::read(dir.join(name)).unwrap()).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(palette.0.as_flattened());
        hasher.update(&image.pixels);
        line(name, (image.width, image.height, 1), hasher);
    }
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn decoded_pictures_match_the_committed_manifest() {
    // The manifest was written after the pictures were checked by eye and against the original
    // (docs/verification/m1a.md); a decoder change must not alter any of them unnoticed.
    let actual = manifest(&data_dir());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/decoded-images.sha256");
    if std::env::var_os("DEADRALLY_BLESS").is_some() {
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    let only_in = |these: &str, those: &str| -> Vec<String> {
        these
            .lines()
            .filter(|line| !those.lines().any(|other| other == *line))
            .map(str::to_owned)
            .collect()
    };
    let (new, gone) = (only_in(&actual, &expected), only_in(&expected, &actual));
    assert!(
        new.is_empty() && gone.is_empty(),
        "decoded pictures differ from {}:\ndecoded now:\n{}\nin the manifest:\n{}\nIf the \
         change is intended, check the pictures again and rewrite the manifest with \
         DEADRALLY_BLESS=1 cargo test-data",
        path.display(),
        new.join("\n"),
        gone.join("\n")
    );
}
```

<!-- write: crates/core/src/menu/draw.rs -->
```rust
//! The menu screen's drawing, as `dr.exe` does it into its screen buffer and copies to the
//! shown buffer (spec M2a §3.3, §3.4): popups (`createPopup` 0x41A530), menus (`drawMenu`
//! 0x41A880), the cursor (`updateCursor` 0x41AB50), the highlight's moves (`refreshMenuUp`
//! 0x41AF40, `refreshMenuDown` 0x41B1A0, 0x41ACF0) and the bottom panel (0x41A7A0, 0x41E810).

use deadrally_gamedata::assets::MenuAssets;
use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::Texts;

use crate::canvas::{Canvas, at};
use crate::font::Font;

/// The fill colour of popups and of the cursor's box.
pub(crate) const POPUP_FILL: u8 = 0xC4;
/// Popup lines: a focused popup's, an unfocused one's.
const LINE_FOCUSED: u8 = 7;
const LINE_UNFOCUSED: u8 = 4;
/// Corner pictures are 32x20, the cursor 20x20, a big glyph 32 high.
const CORNER_WIDTH: usize = 32;
const CORNER_HEIGHT: usize = 20;
const CURSOR_SIZE: usize = 20;
pub(crate) const CURSOR_FRAMES: usize = 50;

/// One menu of the table at 0x4456F0 and its active rows (0x4457F0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuTable {
    /// Which menu of `dr.exe`'s text table its rows are.
    pub(crate) text: usize,
    pub(crate) rows: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) row_height: usize,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) selected: usize,
    pub(crate) active: [bool; 9],
}

/// The main menu: start, multiplayer (inactive), configure, hall of fame, credits, exit.
pub(crate) const MAIN_MENU: MenuTable = MenuTable {
    text: 0,
    rows: 6,
    x: 145,
    y: 124,
    row_height: 28,
    width: 349,
    height: 192,
    selected: 0,
    active: [true, false, true, true, true, true, false, false, false],
};

/// The start submenu at the first start: rows 0, 3 and 5 active.
pub(crate) const START_MENU: MenuTable = MenuTable {
    text: 1,
    rows: 6,
    x: 109,
    y: 171,
    row_height: 28,
    width: 421,
    height: 192,
    selected: 0,
    active: [true, false, false, true, false, true, false, false, false],
};

/// How a menu is drawn: unfocused (mode 0) or focused (mode 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Unfocused,
    Focused,
}

/// The menu's pictures, fonts and rows.
#[derive(Clone, Debug)]
pub(crate) struct Graphics {
    pub(crate) background: Image,
    panel_line: Image,
    corners_focused: Vec<Image>,
    corners_unfocused: Vec<Image>,
    cursor: Vec<Image>,
    pub(crate) big_a: Font,
    pub(crate) big_b: Font,
    pub(crate) big_d: Font,
    pub(crate) small: [Font; 3],
    /// `dr.exe`'s menu text table: `menus[m][r]` is row `r` of menu `m`.
    menus: Vec<Vec<Vec<u8>>>,
}

impl Graphics {
    pub(crate) fn new(assets: &MenuAssets) -> Graphics {
        let texts = &assets.texts;
        Graphics {
            background: assets.background.clone(),
            panel_line: assets.panel_line.clone(),
            corners_focused: assets.corners_focused.clone(),
            corners_unfocused: assets.corners_unfocused.clone(),
            cursor: assets.cursor.clone(),
            big_a: Font::new(assets.big_a.clone(), &texts.big),
            big_b: Font::new(assets.big_b.clone(), &texts.big),
            big_d: Font::new(assets.big_d.clone(), &texts.big),
            small: [
                Font::new(assets.small_a.clone(), &texts.small),
                Font::new(assets.small_b.clone(), &texts.small),
                Font::new(assets.small_c.clone(), &texts.small),
            ],
            menus: texts.menus.clone(),
        }
    }

    pub(crate) fn cursor(&self, frame: usize) -> &Image {
        &self.cursor[frame % self.cursor.len()]
    }

    /// `createPopup(x, y, w, h, focus)`: fill, corners, then lines; nothing outside is
    /// cleared, so a popup drawn over another blends their corners.
    pub(crate) fn popup(
        &self,
        screen: &mut Canvas,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        focus: Focus,
    ) {
        if h > 8 {
            screen.fill(at(x + 2, y + 2), w - 6, h - 8, POPUP_FILL);
        }
        let (corners, line) = match focus {
            Focus::Unfocused => (&self.corners_unfocused, LINE_UNFOCUSED),
            Focus::Focused => (&self.corners_focused, LINE_FOCUSED),
        };
        let right = x + w - CORNER_WIDTH;
        let bottom = y + h - CORNER_HEIGHT;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            screen.draw(corner, offset, true);
        }
        if w > 64 {
            screen.fill(at(x + 32, y + 1), w - 64, 1, line);
            screen.fill(at(x + 32, y + h - 7), w - 64, 1, line);
        }
        if h > 40 {
            screen.fill(at(x + 1, y + 20), 1, h - 40, line);
            screen.fill(at(x + w - 5, y + 20), 1, h - 40, line);
        }
    }

    /// `drawMenu(menu, focus)`: its popup and rows; the selected row with the cursor when the
    /// menu has focus. Nothing reaches the shown buffer.
    pub(crate) fn menu(&self, screen: &mut Canvas, menu: &MenuTable, focus: Focus, cursor: usize) {
        self.popup(screen, menu.x, menu.y, menu.width, menu.height, focus);
        for row in 0..menu.rows {
            let text = &self.menus[menu.text][row];
            let at_text = at(menu.x + 32, menu.y + 5 + row * menu.row_height);
            let font = if row == menu.selected {
                if focus == Focus::Focused {
                    screen.draw(self.cursor(cursor), self.cursor_at(menu), true);
                    &self.big_a
                } else {
                    &self.big_d
                }
            } else if menu.active[row] && focus == Focus::Focused {
                &self.big_b
            } else {
                &self.big_d
            };
            font.draw(screen, text, at_text);
        }
    }

    /// Where the selected row's cursor goes.
    fn cursor_at(&self, menu: &MenuTable) -> usize {
        at(menu.x + 9, menu.y + 11 + menu.selected * menu.row_height)
    }

    /// `updateCursor`: the cursor's box refilled, frame `frame` drawn, the box copied to the
    /// shown buffer.
    pub(crate) fn update_cursor(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &MenuTable,
        frame: usize,
    ) {
        let offset = self.cursor_at(menu);
        screen.fill(offset, CURSOR_SIZE, CURSOR_SIZE, POPUP_FILL);
        screen.draw(self.cursor(frame), offset, true);
        shown.copy_from(screen, offset, CURSOR_SIZE, CURSOR_SIZE);
    }

    /// Moves the highlight to row `to` as `refreshMenuUp`/`refreshMenuDown` and 0x41ACF0 do:
    /// both rows' areas refilled and redrawn, the cursor drawn with frame `frame`, both
    /// copied to the shown buffer. `base` is 6 for Up and the jump to the last row, 5 for Down.
    pub(crate) fn move_highlight(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &mut MenuTable,
        to: usize,
        base: usize,
        frame: usize,
    ) {
        let rows_at = |row: usize| menu.y + base + row * menu.row_height;
        let text_at = |row: usize| at(menu.x + 32, menu.y + 5 + row * menu.row_height);
        let old = menu.selected;
        screen.fill(
            at(menu.x + 9, rows_at(old) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_b
            .draw(screen, &self.menus[menu.text][old], text_at(old));
        menu.selected = to;
        screen.fill(
            at(menu.x + 9, rows_at(to) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_a
            .draw(screen, &self.menus[menu.text][to], text_at(to));
        screen.draw(self.cursor(frame), self.cursor_at(menu), true);
        shown.copy_from(screen, at(menu.x + 7, rows_at(old)), menu.width - 10, 32);
        shown.copy_from(screen, at(menu.x + 7, rows_at(to)), menu.width - 10, 32);
    }

    /// `drawTransparentBlock(x, y, w, h)`: the background restored, then the panel's two
    /// frame lines.
    pub(crate) fn panel_frame(&self, screen: &mut Canvas, x: usize, y: usize, w: usize, h: usize) {
        screen.restore(&self.background, at(x + 2, y - 4), w - 6, h);
        screen.draw(&self.panel_line, at(0, y + 1), true);
        screen.draw(&self.panel_line, at(0, y + h - 9), true);
    }

    /// `drawBottomMenuText`: rows 380..=468 restored, then the panel's last six lines at
    /// (12, 378 + 15k), each in its own small font.
    pub(crate) fn panel_text(&self, screen: &mut Canvas, panel: &Panel) {
        screen.copy_rows(&self.background, 380, 89);
        for (k, line) in panel.lines[16..].iter().enumerate() {
            if let Some(font) = self.small.get(usize::from(line.font)) {
                font.draw(screen, &line.text, at(12, 378 + 15 * k));
            }
        }
    }
}

/// One line of the bottom panel and its font (0, 1, 2: small A, B, C).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: Vec<u8>,
    pub(crate) font: u8,
}

/// The bottom message panel: 22 lines, new ones pushed in at the bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Panel {
    lines: Vec<PanelLine>,
}

impl Panel {
    /// The panel as `mainMenu` fills it at start-up: the four start-up lines in small B, an
    /// empty line before the last.
    pub(crate) fn startup(texts: &Texts) -> Panel {
        let mut panel = Panel {
            lines: vec![PanelLine::default(); 22],
        };
        let [first, second, third, last] = [0, 1, 2, 3].map(|i| texts.panel[i].clone());
        for text in [first, second, third, Vec::new(), last] {
            panel.push(text, 1);
        }
        panel
    }

    fn push(&mut self, text: Vec<u8>, font: u8) {
        self.lines.remove(0);
        self.lines.push(PanelLine { text, font });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::{HEIGHT, WIDTH};

    /// Graphics where every picture has its own colour: corners 11..=14 (focused) and 21..=24,
    /// cursor frame k colour 100 + k with a transparent top-left pixel, glyphs as in
    /// `font::tests::font` but 32x32 (big) or 16x16 (small) and colour 50 + font.
    pub(crate) fn graphics() -> Graphics {
        let solid = |w: u32, h: u32, colour: u8| Image::new(w, h, vec![colour; (w * h) as usize]);
        let glyphs = |size: u32, colour: u8| {
            (0..96)
                .map(|_| solid(size, size, colour))
                .collect::<Vec<_>>()
        };
        let metrics = |size: u8| deadrally_gamedata::text::Metrics {
            width: size,
            height: size,
            advances: vec![size; 96],
        };
        Graphics {
            background: Image::new(
                640,
                480,
                (0..WIDTH * HEIGHT).map(|i| (i % 7) as u8 + 1).collect(),
            ),
            panel_line: solid(640, 10, 99),
            corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
            corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
            cursor: (0..50)
                .map(|k| {
                    let mut frame = solid(20, 20, 100 + k);
                    frame.pixels[0] = 0;
                    frame
                })
                .collect(),
            big_a: Font::new(glyphs(32, 50), &metrics(32)),
            big_b: Font::new(glyphs(32, 51), &metrics(32)),
            big_d: Font::new(glyphs(32, 52), &metrics(32)),
            small: [
                Font::new(glyphs(16, 60), &metrics(16)),
                Font::new(glyphs(16, 61), &metrics(16)),
                Font::new(glyphs(16, 62), &metrics(16)),
            ],
            menus: texts().menus,
        }
    }

    pub(crate) fn texts() -> Texts {
        let metrics = deadrally_gamedata::text::Metrics {
            width: 32,
            height: 32,
            advances: vec![32; 96],
        };
        Texts {
            menus: (0..9)
                .map(|m| {
                    (0..9)
                        .map(|r| {
                            if r < 6 {
                                vec![b'A' + m as u8]
                            } else {
                                Vec::new()
                            }
                        })
                        .collect()
                })
                .collect(),
            panel: (0..4).map(|i| vec![b'a' + i]).collect(),
            exit_question: b"?".to_vec(),
            yes: b"Y".to_vec(),
            no: b"N".to_vec(),
            big: metrics.clone(),
            small: metrics.clone(),
            medium: metrics,
            configure: configure_texts(),
        }
    }

    /// Configure's texts: one letter each, `k` for key and pad names.
    pub(crate) fn configure_texts() -> deadrally_gamedata::text::ConfigureTexts {
        let one = |c: u8| vec![c];
        deadrally_gamedata::text::ConfigureTexts {
            adjust_music: one(b'm'),
            adjust_effects: one(b'e'),
            gamepad_on: one(b'+'),
            gamepad_off: one(b'-'),
            not_detected: one(b'!'),
            press_any_key: one(b'.'),
            controls: (0..8).map(|i| one(b'0' + i)).collect(),
            key_prompts: (0..8).map(|_| one(b'k')).collect(),
            pad_prompts: (0..7).map(|_| one(b'p')).collect(),
            key_names: (0..256).map(|_| one(b'k')).collect(),
            pad_names: (0..9).map(|_| one(b'p')).collect(),
        }
    }

    #[test]
    fn a_popup_fills_exactly_w_minus_6_columns() {
        // DreeRally fills two columns short on the main menu; the original fills x+2..=x+w-5.
        let mut screen = Canvas::default();
        graphics().popup(&mut screen, 145, 124, 349, 192, Focus::Focused);
        let p = screen.pixels();
        assert_eq!(p[at(147, 200)], POPUP_FILL);
        assert_eq!(
            p[at(489 - 1, 200)],
            POPUP_FILL,
            "x + w - 5 is the right line"
        );
        assert_eq!(p[at(489, 200)], LINE_FOCUSED);
        assert_eq!(p[at(146, 200)], LINE_FOCUSED, "left line at x + 1");
        assert_eq!(p[at(490, 200)], 0, "the shadow columns are left alone");
        assert_eq!(p[at(177, 125)], LINE_FOCUSED, "top line from x + 32");
        assert_eq!(
            p[at(461, 309)],
            LINE_FOCUSED,
            "bottom line at y + h - 7 to x + w - 33"
        );
        assert_eq!(p[at(145, 124)], 11, "top-left corner");
        assert_eq!(p[at(462, 124)], 12, "top-right corner");
        assert_eq!(p[at(145, 296)], 13, "bottom-left corner");
        assert_eq!(p[at(462 + 31, 296 + 19)], 14, "bottom-right corner");
    }

    #[test]
    fn a_focused_menu_shows_the_cursor_and_its_rows_in_three_fonts() {
        let mut screen = Canvas::default();
        let mut menu = MAIN_MENU;
        menu.selected = 2;
        graphics().menu(&mut screen, &menu, Focus::Focused, 7);
        let p = screen.pixels();
        let text_row = |row: usize| p[at(177, 129 + 28 * row)];
        assert_eq!(text_row(2), 50, "selected: big A");
        assert_eq!(text_row(0), 51, "active: big B");
        assert_eq!(text_row(1), 52, "inactive: big D");
        assert_eq!(
            p[at(155, 135 + 56)],
            107,
            "cursor frame 7 at (x + 9, y + 11 + 28 * 2)"
        );
        let mut dim = Canvas::default();
        graphics().menu(&mut dim, &menu, Focus::Unfocused, 7);
        assert_eq!(
            dim.pixels()[at(177, 129 + 56)],
            52,
            "unfocused: everything big D"
        );
        assert_eq!(dim.pixels()[at(155, 191)], POPUP_FILL, "no cursor");
        assert_eq!(dim.pixels()[at(145, 124)], 21, "unfocused corners");
    }

    #[test]
    fn the_cursor_update_copies_its_box_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        graphics().update_cursor(&mut screen, &mut shown, &MAIN_MENU, 3);
        assert_eq!(
            shown.pixels()[at(154, 135)],
            POPUP_FILL,
            "transparent pixel over the fill"
        );
        assert_eq!(shown.pixels()[at(155, 135)], 103);
        assert_eq!(shown.pixels()[at(174, 135)], 0, "only the 20x20 box");
    }

    #[test]
    fn moving_the_highlight_redraws_both_rows_and_copies_both_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        let mut menu = MAIN_MENU;
        graphics().move_highlight(&mut screen, &mut shown, &mut menu, 2, 5, 9);
        assert_eq!(menu.selected, 2);
        let s = shown.pixels();
        assert_eq!(s[at(177, 129)], 51, "the old row in big B");
        assert_eq!(s[at(177, 129 + 56)], 50, "the new row in big A");
        assert_eq!(s[at(155, 135 + 56)], 109, "cursor frame 9");
        assert_eq!(s[at(152, 129 + 28)], 0, "row 1 is not copied");
    }

    #[test]
    fn the_panel_shows_its_last_six_lines_with_the_startup_text_in_small_b() {
        let panel = Panel::startup(&texts());
        let mut screen = Canvas::default();
        graphics().panel_text(&mut screen, &panel);
        let p = screen.pixels();
        assert_eq!(
            p[at(12, 380)],
            graphics().background.pixels[at(12, 380)],
            "line 16 is empty; the restore starts at row 380"
        );
        for k in [1, 2, 3, 5] {
            assert_eq!(p[at(12, 378 + 15 * k)], 61, "line {}: small B", 16 + k);
        }
        assert_eq!(
            p[at(12, 445)],
            graphics().background.pixels[at(12, 445)],
            "line 20 is empty (the glyphs above reach row 438)"
        );
    }
}
```

<!-- write: crates/core/tests/common/mod.rs -->
```rust
//! Synthetic menu assets: every picture is one colour of its own, so a test can tell from a
//! pixel which picture, font or cursor frame the menu drew there. Real game data is never
//! committed.

use deadrally_gamedata::assets::{MenuAssets, Picture};
use deadrally_gamedata::dr_cfg::{DrCfg, HEADER_BYTES, PAYLOAD_BYTES};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::text::{ConfigureTexts, Metrics, Texts};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// `MENUBG5`, white in `MENU.PAL`.
pub const BACKGROUND: u8 = 1;
/// The big fonts: selected row (A), active row (B), dim row (D).
pub const BIG_A: u8 = 50;
pub const BIG_B: u8 = 51;
pub const BIG_D: u8 = 52;
/// The small fonts A, B and C.
pub const SMALL: [u8; 3] = [60, 61, 62];
/// Cursor frame k is colour `CURSOR + k`.
pub const CURSOR: u8 = 100;
/// The two credits screens and the end screen, each full red, green or blue in its palette.
pub const CREDITS: [u8; 2] = [200, 201];
pub const END: u8 = 202;
/// The menu's effects (`MEN-SAM`): the back sound plays on the left only, the move sound on
/// the right only, the choose sound on both sides.
pub const BACK_SOUND: usize = 22;
pub const MOVE_SOUND: usize = 25;
pub const CHOOSE_SOUND: usize = 28;
/// The volume popups' slider and knob.
pub const SLIDER: u8 = 70;
pub const KNOB: u8 = 71;

fn solid(width: u32, height: u32, colour: u8) -> Image {
    Image::new(width, height, vec![colour; (width * height) as usize])
}

fn glyphs(size: u32, colour: u8) -> Vec<Image> {
    (0..96).map(|_| solid(size, size, colour)).collect()
}

fn full_screen(colour: u8, rgb: [u8; 3]) -> Picture {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(colour)] = rgb;
    Picture {
        image: solid(640, 480, colour),
        palette,
    }
}

/// A short tone, panned (255 left, 0 right, 128 both).
fn sound(panning: u8) -> Instrument {
    Instrument {
        name: "Beep".into(),
        data: vec![2000; 2000],
        looping: Looping::None,
        volume: 64,
        finetune: 0,
        relative_note: 0,
        panning,
        fadeout: 0,
    }
}

fn texts() -> Texts {
    let metrics = |size: u8| Metrics {
        width: size,
        height: size,
        advances: vec![size; 96],
    };
    Texts {
        // Every menu has six one-letter rows.
        menus: vec![
            (0..9)
                .map(|row| if row < 6 { b"M".to_vec() } else { Vec::new() })
                .collect();
            9
        ],
        panel: (0..4).map(|line| vec![b'a' + line]).collect(),
        exit_question: b"?".to_vec(),
        yes: b"Y".to_vec(),
        no: b"N".to_vec(),
        big: metrics(32),
        small: metrics(16),
        medium: metrics(9),
        configure: ConfigureTexts {
            adjust_music: b"m".to_vec(),
            adjust_effects: b"e".to_vec(),
            gamepad_on: b"+".to_vec(),
            gamepad_off: b"-".to_vec(),
            not_detected: b"!".to_vec(),
            press_any_key: b".".to_vec(),
            controls: vec![b"c".to_vec(); 8],
            key_prompts: vec![b"k".to_vec(); 8],
            pad_prompts: vec![b"p".to_vec(); 7],
            key_names: vec![b"K".to_vec(); 256],
            pad_names: vec![b"P".to_vec(); 9],
        },
    }
}

pub fn menu_assets() -> MenuAssets {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(BACKGROUND)] = [63, 63, 63];
    // The pulsing entries.
    palette.0[16..32].fill([63, 63, 63]);
    let mut copper = Palette::BLACK;
    copper.0[0] = [63, 0, 32];
    let mut effects = vec![None; CHOOSE_SOUND];
    effects[BACK_SOUND - 1] = Some(sound(255));
    effects[MOVE_SOUND - 1] = Some(sound(0));
    effects[CHOOSE_SOUND - 1] = Some(sound(128));
    MenuAssets {
        background: solid(640, 480, BACKGROUND),
        panel_line: solid(640, 10, 99),
        corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
        corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
        cursor: (0..50).map(|k| solid(20, 20, CURSOR + k)).collect(),
        big_a: glyphs(32, BIG_A),
        big_b: glyphs(32, BIG_B),
        big_d: glyphs(32, BIG_D),
        small_a: glyphs(16, SMALL[0]),
        small_b: glyphs(16, SMALL[1]),
        small_c: glyphs(16, SMALL[2]),
        palette,
        copper,
        background_copper: (0..512).map(|row| [(row % 64) as u8, 0, 0]).collect(),
        credits: vec![
            full_screen(CREDITS[0], [63, 0, 0]),
            full_screen(CREDITS[1], [0, 63, 0]),
        ],
        end: full_screen(END, [0, 0, 63]),
        effects: Bank {
            linear_frequencies: true,
            instruments: effects,
        },
        texts: texts(),
        slider: solid(172, 24, SLIDER),
        knob: solid(10, 24, KNOB),
        default_config: DrCfg::parse(&[0; HEADER_BYTES + PAYLOAD_BYTES]).unwrap(),
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --workspace --no-run`
Expected: FAIL to compile (no `ConfigureTexts`; `MenuAssets` has no `slider`).

- [ ] **Step 3: Implement**

<!-- write: crates/gamedata/src/text.rs -->
```rust
//! The original's menu strings and font metrics, read from the player's `dr.exe` at the known
//! release's addresses (spec M2a §3.1). DeadRally ships none of the game's text.

use std::fmt;

use crate::exe::{Exe, ExeError};

/// Menus in the text table, and rows per menu.
pub const MENUS: usize = 9;
pub const MENU_ROWS: usize = 9;
/// The byte that draws nothing and moves the pen one pixel.
pub const GAP: u8 = 0xFA;

/// The menu text table: 50 bytes per row (`dr.exe` 0x446368).
const MENU_TABLE: u32 = 0x44_6368;
const MENU_ROW_BYTES: u32 = 50;
/// The bottom panel's start-up lines, in the order `mainMenu` (0x43A020) adds them; an empty
/// line comes between the third and the fourth.
const PANEL_LINES: [u32; 4] = [0x44_4370, 0x44_433C, 0x44_4300, 0x44_42C0];
const EXIT_QUESTION: u32 = 0x44_42B0;
const YES: u32 = 0x44_3CDC;
const NO: u32 = 0x44_3CD8;
/// Font descriptors: width, height, then one advance per glyph from character 32.
const BIG_METRICS: u32 = 0x44_5848;
const SMALL_METRICS: u32 = 0x44_58B0;
const MEDIUM_METRICS: u32 = 0x44_5928;
/// The longest string read anywhere but the menu table.
const MAX_LINE: usize = 150;
/// The fonts' cell sizes in the known release: big, small and medium.
const BIG_SIZE: (u8, u8) = (32, 32);
const SMALL_SIZE: (u8, u8) = (16, 16);
const MEDIUM_SIZE: (u8, u8) = (9, 12);
/// The main menu (0) and the start submenu (1) show six rows each, all of them text.
const SHOWN_MENUS: usize = 2;
const SHOWN_ROWS: usize = 6;

/// Configure's popups (spec M2b §3.2): the volume captions, the gamepad switch's two texts and
/// the popup when no gamepad is found.
const ADJUST_MUSIC: u32 = 0x44_3F90;
const ADJUST_EFFECTS: u32 = 0x44_3F6C;
const GAMEPAD_ON: u32 = 0x44_3F50;
const GAMEPAD_OFF: u32 = 0x44_3F34;
const NOT_DETECTED: u32 = 0x44_3170;
const PRESS_ANY_KEY: u32 = 0x44_29A4;
/// The eight controls' names (accelerate, brake, left, right, turbo, gun, mine, horn), with
/// the prompts for a key and, but for the horn, for a gamepad input.
const CONTROLS: [u32; 8] = [
    0x44_2AD0, 0x44_2A60, 0x44_2A4C, 0x44_2A34, 0x44_2A18, 0x44_29FC, 0x44_29E0, 0x44_29C0,
];
const KEY_PROMPTS: [u32; 8] = [
    0x44_3E34, 0x44_3E18, 0x44_3DF8, 0x44_3DD8, 0x44_3DB8, 0x44_3D98, 0x44_3D78, 0x44_3D60,
];
const PAD_PROMPTS: [u32; 7] = [
    0x44_3F14, 0x44_3EF8, 0x44_3ED8, 0x44_3EB8, 0x44_3E98, 0x44_3E74, 0x44_3E54,
];
/// The names of the gamepad inputs 0 (none) to 8, 16 bytes apart going down.
const PAD_NAMES: u32 = 0x44_3160;
pub const PAD_INPUTS: usize = 9;
/// Key names, 16 bytes apart going down from here in scancode order: 0x01..=0x54, then
/// `MORE_NAMED_KEYS`; one slot after 0xCB's holds a control's name. Other keys are
/// "unavailable".
const KEY_NAMES: u32 = 0x44_30C0;
const MORE_NAMED_KEYS: [u8; 17] = [
    0x57, 0x58, 0x9C, 0x9D, 0xB5, 0xB7, 0xB8, 0xC7, 0xC8, 0xC9, 0xCB, 0xCD, 0xCF, 0xD0, 0xD1, 0xD2,
    0xD3,
];
const UNAVAILABLE_KEY: u32 = 0x44_30D0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    Exe(ExeError),
    /// A byte that is neither printable ASCII nor the gap.
    Unprintable {
        address: u32,
    },
    /// An empty string where the menus show text, or a font of another size.
    Unexpected {
        address: u32,
    },
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextError::Exe(error) => write!(f, "dr.exe: {error}"),
            TextError::Unprintable { address } => write!(
                f,
                "dr.exe: the text at address {address:#x} is not the original's; is this the known release?"
            ),
            TextError::Unexpected { address } => write!(
                f,
                "dr.exe: the data at address {address:#x} is not where the known release keeps it; is this the known release?"
            ),
        }
    }
}

impl std::error::Error for TextError {}

impl From<ExeError> for TextError {
    fn from(error: ExeError) -> TextError {
        TextError::Exe(error)
    }
}

/// The address of scancode `code`'s name.
fn key_name(code: u8) -> u32 {
    match code {
        0x01..=0x54 => KEY_NAMES - 16 * (u32::from(code) - 1),
        _ => match MORE_NAMED_KEYS.iter().position(|&named| named == code) {
            Some(k) => {
                let skip = if code > 0xCB { 16 } else { 0 };
                KEY_NAMES - 16 * (0x54 + k as u32) - skip
            }
            None => UNAVAILABLE_KEY,
        },
    }
}

/// Configure's texts (spec M2b §3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigureTexts {
    pub adjust_music: Vec<u8>,
    pub adjust_effects: Vec<u8>,
    pub gamepad_on: Vec<u8>,
    pub gamepad_off: Vec<u8>,
    pub not_detected: Vec<u8>,
    pub press_any_key: Vec<u8>,
    /// The eight controls, each padded so its key's name lines up.
    pub controls: Vec<Vec<u8>>,
    pub key_prompts: Vec<Vec<u8>>,
    pub pad_prompts: Vec<Vec<u8>>,
    /// `key_names[scancode]`, all 256.
    pub key_names: Vec<Vec<u8>>,
    /// `pad_names[input]`, 0 (none) to 8.
    pub pad_names: Vec<Vec<u8>>,
}

/// A font's cell size and the pen advance of each glyph, character 32 first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub width: u8,
    pub height: u8,
    pub advances: Vec<u8>,
}

/// Everything M2a reads from `dr.exe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texts {
    /// `menus[m][r]`: row `r` of menu `m`, empty where the menu has no such row.
    pub menus: Vec<Vec<Vec<u8>>>,
    /// The bottom panel's four start-up lines.
    pub panel: Vec<Vec<u8>>,
    pub exit_question: Vec<u8>,
    pub yes: Vec<u8>,
    pub no: Vec<u8>,
    pub big: Metrics,
    pub small: Metrics,
    pub medium: Metrics,
    pub configure: ConfigureTexts,
}

impl Texts {
    /// # Errors
    ///
    /// [`TextError`] when a string is missing, does not end, or holds a byte the original's
    /// strings never do, when a string the menus show is empty, or when a font's size is not
    /// the known release's: the executable is not the known release.
    pub fn read(exe: &Exe) -> Result<Texts, TextError> {
        let text = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = exe.string_at(address, max)?;
            if bytes.iter().all(|&b| (32..127).contains(&b) || b == GAP) {
                Ok(bytes.to_vec())
            } else {
                Err(TextError::Unprintable { address })
            }
        };
        let shown = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = text(address, max)?;
            if bytes.is_empty() {
                Err(TextError::Unexpected { address })
            } else {
                Ok(bytes)
            }
        };
        let all = |addresses: &[u32]| -> Result<Vec<Vec<u8>>, TextError> {
            addresses
                .iter()
                .map(|&address| shown(address, MAX_LINE))
                .collect()
        };
        let menus = (0..MENUS)
            .map(|menu| {
                (0..MENU_ROWS)
                    .map(|row| {
                        let address = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS * menu + row) as u32;
                        if menu < SHOWN_MENUS && row < SHOWN_ROWS {
                            shown(address, MENU_ROW_BYTES as usize - 1)
                        } else {
                            text(address, MENU_ROW_BYTES as usize - 1)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metrics = |address: u32, glyphs: usize, size: (u8, u8)| -> Result<Metrics, TextError> {
            let bytes = exe.bytes_at(address, 2 + glyphs)?;
            if (bytes[0], bytes[1]) != size {
                return Err(TextError::Unexpected { address });
            }
            Ok(Metrics {
                width: bytes[0],
                height: bytes[1],
                advances: bytes[2..].to_vec(),
            })
        };
        Ok(Texts {
            menus,
            panel: PANEL_LINES
                .iter()
                .map(|&address| shown(address, MAX_LINE))
                .collect::<Result<Vec<_>, _>>()?,
            exit_question: shown(EXIT_QUESTION, MAX_LINE)?,
            yes: shown(YES, MAX_LINE)?,
            no: shown(NO, MAX_LINE)?,
            configure: ConfigureTexts {
                adjust_music: shown(ADJUST_MUSIC, MAX_LINE)?,
                adjust_effects: shown(ADJUST_EFFECTS, MAX_LINE)?,
                gamepad_on: shown(GAMEPAD_ON, MAX_LINE)?,
                gamepad_off: shown(GAMEPAD_OFF, MAX_LINE)?,
                not_detected: shown(NOT_DETECTED, MAX_LINE)?,
                press_any_key: shown(PRESS_ANY_KEY, MAX_LINE)?,
                controls: all(&CONTROLS)?,
                key_prompts: all(&KEY_PROMPTS)?,
                pad_prompts: all(&PAD_PROMPTS)?,
                key_names: (0..=255)
                    .map(|code| shown(key_name(code), MAX_LINE))
                    .collect::<Result<_, _>>()?,
                pad_names: (0..PAD_INPUTS as u32)
                    .map(|input| shown(PAD_NAMES - 16 * input, MAX_LINE))
                    .collect::<Result<_, _>>()?,
            },
            big: metrics(BIG_METRICS, 96, BIG_SIZE)?,
            small: metrics(SMALL_METRICS, 96, SMALL_SIZE)?,
            medium: metrics(MEDIUM_METRICS, 62, MEDIUM_SIZE)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::exe::tests::{build, build_at};

    /// A section from 0x442000 to 0x448000 with every string and table where the known release
    /// keeps it: menu `m` row `r` reads "m.r", the other strings made-up words.
    fn known_layout() -> Vec<u8> {
        let mut data = vec![0u8; 0x6000];
        let mut put = |address: u32, bytes: &[u8]| {
            let at = (address - 0x44_2000) as usize;
            data[at..at + bytes.len()].copy_from_slice(bytes);
        };
        for menu in 0..MENUS as u32 {
            for row in 0..MENU_ROWS as u32 {
                let address = MENU_TABLE + MENU_ROW_BYTES * (9 * menu + row);
                put(address, format!("{menu}.{row}").as_bytes());
            }
        }
        for (index, &address) in PANEL_LINES.iter().enumerate() {
            put(address, format!("line {index}").as_bytes());
        }
        put(EXIT_QUESTION, b"Quit?");
        put(YES, b"Y");
        put(NO, b"N");
        put(BIG_METRICS, &[32, 32, 20, 9]);
        put(SMALL_METRICS, &[16, 16, 10, 5]);
        put(MEDIUM_METRICS, &[9, 12, 9, 9]);
        for address in [
            ADJUST_MUSIC,
            ADJUST_EFFECTS,
            GAMEPAD_ON,
            GAMEPAD_OFF,
            NOT_DETECTED,
            PRESS_ANY_KEY,
        ] {
            put(address, format!("text {address:x}").as_bytes());
        }
        for address in CONTROLS.iter().chain(&KEY_PROMPTS).chain(&PAD_PROMPTS) {
            put(*address, format!("text {address:x}").as_bytes());
        }
        for code in 0..=255 {
            put(key_name(code), format!("key {code:02x}").as_bytes());
        }
        for input in 0..PAD_INPUTS as u32 {
            put(PAD_NAMES - 16 * input, format!("pad {input}").as_bytes());
        }
        build_at(0x4_2000, 0x6000, &data)
    }

    #[test]
    fn menu_rows_are_fifty_bytes_apart_and_menus_nine_rows() {
        // A wrong stride shows another menu's text, or half of two rows.
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.menus[0][0], b"0.0");
        assert_eq!(texts.menus[3][4], b"3.4");
        assert_eq!(texts.menus[8][8], b"8.8");
        assert_eq!(texts.panel[3], b"line 3");
        assert_eq!(
            (texts.exit_question.as_slice(), texts.yes.as_slice()),
            (b"Quit?".as_slice(), b"Y".as_slice())
        );
        assert_eq!(
            (texts.big.width, texts.big.height, &texts.big.advances[..2]),
            (32, 32, [20, 9].as_slice())
        );
        assert_eq!(texts.small.advances.len(), 96);
        assert_eq!(
            texts.medium.advances.len(),
            62,
            "the medium font has 62 glyphs"
        );
    }

    #[test]
    fn a_byte_the_original_never_uses_is_refused_with_its_address() {
        let mut bytes = known_layout();
        let at = offset(&bytes, YES);
        bytes[at] = 0x01;
        assert_eq!(
            Texts::read(&Exe::parse(bytes).unwrap()),
            Err(TextError::Unprintable { address: YES })
        );
    }

    #[test]
    fn each_key_has_its_name_and_keys_without_one_are_unavailable() {
        // Define Keyboard shows these names; one slot off names every key after it wrongly.
        // The slot after 0xCB's holds the accelerate control's name, so 0xCD skips it.
        assert_eq!(key_name(0x01), 0x44_30C0);
        assert_eq!(key_name(0x54), 0x44_30C0 - 16 * 0x53);
        assert_eq!(key_name(0x57), 0x44_2B80);
        assert_eq!(key_name(0xCB), 0x44_2AE0);
        assert_eq!(key_name(0xCD), 0x44_2AC0);
        assert_eq!(key_name(0xD3), 0x44_2A70);
        for code in [0x00, 0x55, 0x56, 0x59, 0xCC, 0xD4, 0xFF] {
            assert_eq!(key_name(code), UNAVAILABLE_KEY, "{code:#x}");
        }
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.configure.key_names[0x1E], b"key 1e");
        assert_eq!(texts.configure.pad_names[8], b"pad 8");
        assert_eq!(texts.configure.controls.len(), 8);
        assert_eq!(texts.configure.pad_prompts.len(), 7);
    }

    /// Where `address` lies in the bytes of [`known_layout`]'s file.
    fn offset(bytes: &[u8], address: u32) -> usize {
        bytes.len() - 0x6000 + (address - 0x44_2000) as usize
    }

    #[test]
    fn a_layout_moved_by_a_few_bytes_is_refused() {
        // Another build of dr.exe may keep the same strings a little further on. Every read
        // would then land on blanks, other strings' tails or other bytes that pass as text,
        // and the menus would show fragments: the rows the menus show and the fonts' sizes
        // must be where the known release keeps them.
        let mut moved = known_layout();
        let start = offset(&moved, 0x44_2000);
        moved[start..].rotate_right(3);
        let error = Texts::read(&Exe::parse(moved).unwrap()).unwrap_err();
        assert!(matches!(error, TextError::Unexpected { .. }), "{error}");
    }

    #[test]
    fn an_empty_shown_string_or_another_font_size_is_refused_with_its_address() {
        let mut no_yes = known_layout();
        let at = offset(&no_yes, YES);
        no_yes[at] = 0;
        assert_eq!(
            Texts::read(&Exe::parse(no_yes).unwrap()),
            Err(TextError::Unexpected { address: YES })
        );
        let mut empty_row = known_layout();
        let row = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS as u32 + 5);
        let at = offset(&empty_row, row);
        empty_row[at] = 0;
        assert_eq!(
            Texts::read(&Exe::parse(empty_row).unwrap()),
            Err(TextError::Unexpected { address: row }),
            "the start submenu's last row"
        );
        let mut narrow = known_layout();
        let at = offset(&narrow, SMALL_METRICS);
        narrow[at] = 15;
        assert_eq!(
            Texts::read(&Exe::parse(narrow).unwrap()),
            Err(TextError::Unexpected {
                address: SMALL_METRICS
            })
        );
    }

    #[test]
    fn a_file_that_is_not_the_known_release_is_refused_with_the_address() {
        // Another executable keeps different bytes at these addresses; drawing them would fill
        // the menus with garbage instead of saying what is wrong.
        let exe = Exe::parse(build(&[0x7F; 0x200])).unwrap();
        let error = Texts::read(&exe).unwrap_err();
        assert!(
            matches!(error, TextError::Exe(ExeError::Address(_))),
            "{error}"
        );
    }
}
```

<!-- write: crates/gamedata/src/assets.rs -->
```rust
//! The decoded data the startup sequence and the main menu need (spec M1a §4.2, M1b §4.1,
//! M2a §4.1).

use std::fmt;
use std::path::PathBuf;

use crate::bmp::{self, BmpError};
use crate::bpa::{Archive, BpaError};
use crate::catalog::{self, CatalogError};
use crate::dr_cfg::DrCfg;
use crate::exe::{Exe, ExeError};
use crate::haf::{Animation, HafError};
use crate::image::{Image, Palette, PaletteError};
use crate::machine::MachineError;
use crate::s3m::Module;
use crate::sound::{self, SoundError};
use crate::text::{TextError, Texts};
use crate::validate::Validation;
use crate::xm::Bank;

/// An image with the palette it is shown with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub image: Image,
    pub palette: Palette,
}

/// Everything the startup sequence shows.
#[derive(Debug)]
pub struct Assets {
    /// `SANIM.haf`.
    pub intro: Animation,
    /// `FRAMES.BPK`: the intro's 320x200 letterbox; the game uses palette entries 0..=15.
    pub letterbox: Picture,
    /// `APOGEE.BPK` with `APOGEE.PAL`.
    pub apogee: Picture,
    /// `rmd.bmp`.
    pub remedy: Picture,
    /// `STARTSCR.BPK` with `STARTSCR.PAL`.
    pub title: Picture,
    /// `TR0-MUS.CMF`: the music under the intro.
    pub intro_music: Module,
    /// `SANIM-E.CMF`: the intro's effects, numbered as in `SANIM.haf`'s table.
    pub intro_effects: Bank,
    /// `MEN-MUS.CMF`: the music that starts when the intro ends and goes on into the menus.
    pub menu_music: Module,
    pub menu: MenuAssets,
}

/// What the main menu draws and plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuAssets {
    /// `MENUBG5.BPK`, 640x480.
    pub background: Image,
    /// `CHATLIN1.BPK`, 640x10: the bottom panel's frame lines.
    pub panel_line: Image,
    /// `CORN3A.BPK` and `CORN3B.BPK`: popup corners (top left, top right, bottom left, bottom
    /// right) of a focused and of an unfocused popup.
    pub corners_focused: Vec<Image>,
    pub corners_unfocused: Vec<Image>,
    /// `CURSOR.BPK`: 50 frames, 20x20.
    pub cursor: Vec<Image>,
    /// `F-BIG3A`, `-B`, `-D` (32x32) and `F-SMA3A`, `-B`, `-C` (16x16): 96 glyphs each.
    pub big_a: Vec<Image>,
    pub big_b: Vec<Image>,
    pub big_d: Vec<Image>,
    pub small_a: Vec<Image>,
    pub small_b: Vec<Image>,
    pub small_c: Vec<Image>,
    /// `MENU.PAL`.
    pub palette: Palette,
    /// `COPPER.PAL`: one colour per player colour, whose ramps the menu palette gets.
    pub copper: Palette,
    /// `BGCOP.PAL`: 512 colours for the background copper rows.
    pub background_copper: Vec<[u8; 3]>,
    /// `CREDIT1.BPK` and `CREDIT2.BPK` with their palettes.
    pub credits: Vec<Picture>,
    /// `end.bmp`: the screen the game ends on.
    pub end: Picture,
    /// `MEN-SAM.CMF`: the menus' effects.
    pub effects: Bank,
    /// The strings and font metrics in `dr.exe`.
    pub texts: Texts,
    /// `SLIDMUS2.BPK` and `VOLCUR2.BPK`: the volume popups' slider and its knob.
    pub slider: Image,
    pub knob: Image,
    /// The `dr.cfg` the original writes when it has none, from `dr.exe`'s `defaultConfig`.
    pub default_config: DrCfg,
}

#[derive(Debug)]
pub enum AssetError {
    Archive(BpaError),
    Image {
        name: &'static str,
        error: CatalogError,
    },
    Palette {
        name: &'static str,
        error: PaletteError,
    },
    Bmp {
        path: PathBuf,
        error: BmpError,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Animation(HafError),
    Sound(SoundError),
    Exe(ExeError),
    Machine(MachineError),
    Text(TextError),
    Size {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Archive(error) => write!(f, "{error}"),
            AssetError::Image { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Palette { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Bmp { path, error } => write!(f, "{}: {error}", path.display()),
            AssetError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            AssetError::Animation(error) => write!(f, "{error}"),
            AssetError::Sound(error) => write!(f, "{error}"),
            AssetError::Exe(error) => write!(f, "dr.exe: {error}"),
            AssetError::Machine(error) => write!(f, "{error}"),
            AssetError::Text(error) => write!(f, "{error}"),
            AssetError::Size {
                name,
                expected,
                actual,
            } => write!(f, "MENU.BPA/{name}: {actual} bytes, not {expected}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<BpaError> for AssetError {
    fn from(error: BpaError) -> AssetError {
        AssetError::Archive(error)
    }
}

impl Assets {
    /// Loads the startup sequence's data from a validated data directory.
    ///
    /// # Errors
    ///
    /// [`AssetError`] naming the file and entry that failed.
    pub fn load(validation: &Validation) -> Result<Assets, AssetError> {
        let path = |name: &str| -> PathBuf {
            validation
                .files
                .iter()
                .find(|file| file.name == name)
                .map(|file| file.path.clone())
                .unwrap_or_else(|| panic!("{name} is a required file, so validation found it"))
        };
        let menu = Archive::open(&path("MENU.BPA"))?;
        let musics = Archive::open(&path(sound::ARCHIVE))?;
        let remedy_path = path("RMD.BMP");
        let remedy_bytes = std::fs::read(&remedy_path).map_err(|source| AssetError::Read {
            path: remedy_path.clone(),
            source,
        })?;
        let (image, palette) = bmp::decode(&remedy_bytes).map_err(|error| AssetError::Bmp {
            path: remedy_path,
            error,
        })?;
        Ok(Assets {
            intro: Animation::open(&path("SANIM.HAF")).map_err(AssetError::Animation)?,
            letterbox: letterbox(&menu)?,
            apogee: picture(&menu, "APOGEE.BPK", "APOGEE.PAL")?,
            remedy: Picture { image, palette },
            title: picture(&menu, "STARTSCR.BPK", "STARTSCR.PAL")?,
            intro_music: sound::load_music(&musics, "TR0-MUS.CMF").map_err(AssetError::Sound)?,
            intro_effects: sound::load_effects(&musics, "SANIM-E.CMF")
                .map_err(AssetError::Sound)?,
            menu_music: sound::load_music(&musics, "MEN-MUS.CMF").map_err(AssetError::Sound)?,
            menu: menu_assets(&menu, &musics, &path("END.BMP"), &path("DR.EXE"))?,
        })
    }
}

/// All frames of a catalogued `MENU.BPA` image.
fn frames(menu: &Archive, name: &'static str) -> Result<Vec<Image>, AssetError> {
    let entry = catalog::find("MENU.BPA", name).expect("menu images are catalogued");
    entry
        .decode(menu.read(name)?)
        .map_err(|error| AssetError::Image { name, error })
}

fn palette(menu: &Archive, name: &'static str) -> Result<Palette, AssetError> {
    Palette::from_bytes(menu.read(name)?).map_err(|error| AssetError::Palette { name, error })
}

fn bmp_picture(path: &std::path::Path) -> Result<Picture, AssetError> {
    let bytes = std::fs::read(path).map_err(|source| AssetError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let (image, palette) = bmp::decode(&bytes).map_err(|error| AssetError::Bmp {
        path: path.to_path_buf(),
        error,
    })?;
    Ok(Picture { image, palette })
}

/// A picture the menu copies over its whole screen, which must be 640x480.
fn full_screen(picture: Picture, path: &std::path::Path) -> Result<Picture, AssetError> {
    let (width, height) = (picture.image.width, picture.image.height);
    if (width, height) == (640, 480) {
        Ok(picture)
    } else {
        Err(AssetError::Bmp {
            path: path.to_path_buf(),
            error: BmpError::Unsupported(format!("{width}x{height}, not 640x480")),
        })
    }
}

fn menu_assets(
    menu: &Archive,
    musics: &Archive,
    end: &std::path::Path,
    exe: &std::path::Path,
) -> Result<MenuAssets, AssetError> {
    const BGCOP: &str = "BGCOP.PAL";
    let background_copper = menu.read(BGCOP)?;
    // 512 colours of 6-bit components.
    if background_copper.len() != 3 * 512 {
        return Err(AssetError::Size {
            name: BGCOP,
            expected: 3 * 512,
            actual: background_copper.len(),
        });
    }
    if let Some((index, &value)) = background_copper.iter().enumerate().find(|(_, c)| **c > 63) {
        return Err(AssetError::Palette {
            name: BGCOP,
            error: PaletteError::NotSixBit { index, value },
        });
    }
    let exe_bytes = std::fs::read(exe).map_err(|source| AssetError::Read {
        path: exe.to_path_buf(),
        source,
    })?;
    let exe = Exe::parse(exe_bytes).map_err(AssetError::Exe)?;
    Ok(MenuAssets {
        background: frames(menu, "MENUBG5.BPK")?.remove(0),
        panel_line: frames(menu, "CHATLIN1.BPK")?.remove(0),
        corners_focused: frames(menu, "CORN3A.BPK")?,
        corners_unfocused: frames(menu, "CORN3B.BPK")?,
        cursor: frames(menu, "CURSOR.BPK")?,
        big_a: frames(menu, "F-BIG3A.BPK")?,
        big_b: frames(menu, "F-BIG3B.BPK")?,
        big_d: frames(menu, "F-BIG3D.BPK")?,
        small_a: frames(menu, "F-SMA3A.BPK")?,
        small_b: frames(menu, "F-SMA3B.BPK")?,
        small_c: frames(menu, "F-SMA3C.BPK")?,
        palette: palette(menu, "MENU.PAL")?,
        copper: palette(menu, "COPPER.PAL")?,
        background_copper: background_copper.as_chunks::<3>().0.to_vec(),
        credits: vec![
            picture(menu, "CREDIT1.BPK", "CREDIT1.PAL")?,
            picture(menu, "CREDIT2.BPK", "CREDIT2.PAL")?,
        ],
        end: full_screen(bmp_picture(end)?, end)?,
        effects: sound::load_effects(musics, "MEN-SAM.CMF").map_err(AssetError::Sound)?,
        texts: Texts::read(&exe).map_err(AssetError::Text)?,
        slider: frames(menu, "SLIDMUS2.BPK")?.remove(0),
        knob: frames(menu, "VOLCUR2.BPK")?.remove(0),
        default_config: DrCfg::defaults(&exe).map_err(AssetError::Machine)?,
    })
}

fn picture(
    menu: &Archive,
    image: &'static str,
    palette: &'static str,
) -> Result<Picture, AssetError> {
    let entry = catalog::find("MENU.BPA", image).expect("these images are catalogued");
    let mut frames = entry
        .decode(menu.read(image)?)
        .map_err(|error| AssetError::Image { name: image, error })?;
    let palette =
        Palette::from_bytes(menu.read(palette)?).map_err(|error| AssetError::Palette {
            name: palette,
            error,
        })?;
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

fn letterbox(menu: &Archive) -> Result<Picture, AssetError> {
    const NAME: &str = "FRAMES.BPK";
    let entry = catalog::find("MENU.BPA", NAME).expect("FRAMES.BPK is catalogued");
    let bytes = menu.read(NAME)?;
    let image_error = |error| AssetError::Image { name: NAME, error };
    let mut frames = entry.decode(bytes).map_err(image_error)?;
    let palette = entry
        .embedded_palette(bytes)
        .map_err(image_error)?
        .expect("FRAMES.BPK embeds its palette");
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_end_screen_of_another_size_is_an_error_not_a_crash_later() {
        // The menu copies END.BMP over its whole 640x480 screen; a BMP of another size (from
        // an unknown release) is reported when the data loads, naming the file.
        let path = std::path::Path::new("END.BMP");
        let screen = |width: u32, height: u32| Picture {
            image: Image::new(width, height, vec![0; (width * height) as usize]),
            palette: Palette::BLACK,
        };
        assert!(full_screen(screen(640, 480), path).is_ok());
        let error = full_screen(screen(320, 200), path).unwrap_err();
        assert_eq!(
            error.to_string(),
            "END.BMP: unsupported BMP: 320x200, not 640x480"
        );
    }
}
```

- [ ] **Step 4: Run the checks, with the data**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (280 passed, 17 ignored), among them `each_key_has_its_name_and_keys_without_one_are_unavailable`.

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test -p deadrally-gamedata --test catalog_data -- --ignored`
Expected: all pass (7 passed): the defaults run from the real `dr.exe` hold the original's keys, gamepad inputs and volumes.

- [ ] **Step 5: Commit**

```bash
git add crates/gamedata crates/core/src/menu/draw.rs crates/core/tests/common/mod.rs
git commit -m "feat: load Configure's texts and pictures"
```

---

### Task 3: Configure

Spec 2 (decisions 3, 4) and 3.2. The game takes the player's `dr.cfg`, counts the start and hands the file out whenever the original writes it; the menu scene gains Configure, its volume popups, Define Keyboard, Define Gamepad and the gamepad switch; the frontend loads and writes DeadRally's `dr.cfg` and reports gamepad connections.

**Files:**
- Create: `crates/core/src/menu/configure.rs`
- Modify: `crates/core/src/{input,keys,font,game,startup,test_scene}.rs`, `crates/core/src/audio/mod.rs`, `crates/core/src/menu/{mod,draw}.rs`, `crates/core/tests/{menu,startup}.rs`, `crates/core/tests/common/mod.rs`, `crates/deadrally/src/main.rs`, `crates/headless/src/main.rs`, `crates/headless/tests/{rendered_audio,menu_run}.rs`

**Interfaces:**
- Consumes: `DrCfg` (Task 1), `ConfigureTexts`, `MenuAssets::{slider, knob, default_config}` (Task 2).
- Produces: `Game::new(Assets, DrCfg)`, `Game::take_config() -> Option<Vec<u8>>`; `InputEvent::PadConnected { connected }`; `--key-at T:q`.

- [ ] **Step 1: Write the failing tests**

The unit tests of the input, the fonts and the sound over M2a's code, and the scene's tests:

<!-- write: crates/core/src/keys.rs -->
```rust
//! The player's input as the original reads it (spec M2a §3.3): one remembered key, its PC
//! set-1 scancode as SDL 1.2's `windib` driver reports it, kept until `eventDetected`
//! (0x417EB0) reads and clears it; SDL 1.2's key repeat (`SDL_EnableKeyRepeat(500, 30)`); and
//! the joystick, polled by `eventDetected` itself.

use crate::input::{InputEvent, Key, PadAxis, PadButton};

/// Scancodes the menus act on.
pub(crate) const ESCAPE: u8 = 0x01;
pub(crate) const ENTER: u8 = 0x1C;
pub(crate) const SPACE: u8 = 0x39;
pub(crate) const UP: u8 = 0x48;
pub(crate) const DOWN: u8 = 0x50;
pub(crate) const LEFT: u8 = 0x4B;
pub(crate) const RIGHT: u8 = 0x4D;
pub(crate) const Y: u8 = 0x15;
pub(crate) const N: u8 = 0x31;
/// The joystick's codes are DirectInput's: set-1 with the extended bit.
pub(crate) const PAD_LEFT: u8 = 0xCB;
pub(crate) const PAD_RIGHT: u8 = 0xCD;
pub(crate) const PAD_UP: u8 = 0xC8;
pub(crate) const PAD_DOWN: u8 = 0xD0;

/// One tick, in SDL milliseconds.
const TICK_MS: i64 = 14;
/// SDL 1.2 key repeat: the delay before the first repeat and the interval after it.
const REPEAT_DELAY_MS: i64 = 500;
const REPEAT_INTERVAL_MS: i64 = 30;
/// `eventDetected`'s joystick: stick positions past ±50 (raw / 256) count; a first push holds
/// the repeat off for 700 ms; 400 ms tell a fresh push from a held one.
const STICK_THRESHOLD: i32 = 50;
const HOLD_OFF_MS: i64 = 700;
const FRESH_MS: i64 = 400;
const UNSET_MS: i64 = 250;

/// The PC set-1 scancode SDL 1.2's `windib` driver reports for a key (no 0xE0 prefix, so the
/// arrows share the keypad's codes).
pub(crate) fn scancode(key: Key) -> u8 {
    match key {
        Key::Escape => 0x01,
        Key::Digit1 => 0x02,
        Key::Digit2 => 0x03,
        Key::Digit3 => 0x04,
        Key::Digit4 => 0x05,
        Key::Digit5 => 0x06,
        Key::Digit6 => 0x07,
        Key::Digit7 => 0x08,
        Key::Digit8 => 0x09,
        Key::Digit9 => 0x0A,
        Key::Digit0 => 0x0B,
        Key::Backspace => 0x0E,
        Key::Tab => 0x0F,
        Key::Q => 0x10,
        Key::W => 0x11,
        Key::E => 0x12,
        Key::R => 0x13,
        Key::T => 0x14,
        Key::Y => 0x15,
        Key::U => 0x16,
        Key::I => 0x17,
        Key::O => 0x18,
        Key::P => 0x19,
        Key::Enter | Key::KpEnter => 0x1C,
        Key::LeftCtrl | Key::RightCtrl => 0x1D,
        Key::A => 0x1E,
        Key::S => 0x1F,
        Key::D => 0x20,
        Key::F => 0x21,
        Key::G => 0x22,
        Key::H => 0x23,
        Key::J => 0x24,
        Key::K => 0x25,
        Key::L => 0x26,
        Key::LeftShift => 0x2A,
        Key::Z => 0x2C,
        Key::X => 0x2D,
        Key::C => 0x2E,
        Key::V => 0x2F,
        Key::B => 0x30,
        Key::N => 0x31,
        Key::M => 0x32,
        Key::KpDivide => 0x35,
        Key::RightShift => 0x36,
        Key::KpMultiply => 0x37,
        Key::LeftAlt | Key::RightAlt => 0x38,
        Key::Space => 0x39,
        Key::F1 => 0x3B,
        Key::F2 => 0x3C,
        Key::F3 => 0x3D,
        Key::F4 => 0x3E,
        Key::F5 => 0x3F,
        Key::F6 => 0x40,
        Key::F7 => 0x41,
        Key::F8 => 0x42,
        Key::F9 => 0x43,
        Key::F10 => 0x44,
        Key::Kp7 => 0x47,
        Key::Up | Key::Kp8 => 0x48,
        Key::Kp9 => 0x49,
        Key::KpMinus => 0x4A,
        Key::Left | Key::Kp4 => 0x4B,
        Key::Kp5 => 0x4C,
        Key::Right | Key::Kp6 => 0x4D,
        Key::KpPlus => 0x4E,
        Key::Kp1 => 0x4F,
        Key::Down | Key::Kp2 => 0x50,
        Key::Kp3 => 0x51,
        Key::Kp0 => 0x52,
        Key::KpPeriod => 0x53,
        Key::F11 => 0x57,
        Key::F12 => 0x58,
    }
}

/// SDL 1.2 repeats every key but the modifiers.
fn repeats(key: Key) -> bool {
    !matches!(
        key,
        Key::LeftShift
            | Key::RightShift
            | Key::LeftCtrl
            | Key::RightCtrl
            | Key::LeftAlt
            | Key::RightAlt
    )
}

/// The key SDL repeats while it is held.
#[derive(Clone, Copy, Debug)]
struct Repeat {
    key: Key,
    /// Still waiting for the first delay.
    first: bool,
    /// Milliseconds since SDL's repeat timestamp.
    elapsed: i64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Keys {
    /// `[0x456BF8]`: the last key-down's scancode, 0 when read.
    remembered: u8,
    repeat: Option<Repeat>,
    /// Ticks polled so far: SDL's clock is `14 * ticks` milliseconds.
    ticks: i64,
    stick: [i32; 2],
    buttons: [bool; 4],
    /// `eventDetected`'s joystick timestamps: 0x456B28, 0x456B2C and the hold-off 0x456B1C.
    pad_pushed_ms: Option<i64>,
    pad_called_ms: Option<i64>,
    hold_off_until_ms: i64,
}

impl Keys {
    /// An input event from the frontend. Key-downs become the remembered key at once, as the
    /// next poll of the original's event loop would make them.
    pub(crate) fn event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, pressed: true } => {
                self.remembered = scancode(key);
                if repeats(key) {
                    self.repeat = Some(Repeat {
                        key,
                        first: true,
                        elapsed: 0,
                    });
                }
            }
            InputEvent::Key {
                key,
                pressed: false,
            } => {
                if self.repeat.is_some_and(|repeat| repeat.key == key) {
                    self.repeat = None;
                }
            }
            InputEvent::PadButton { button, pressed } => {
                self.buttons[PadButton::ALL
                    .iter()
                    .position(|&b| b == button)
                    .expect("every button is listed")] = pressed;
            }
            InputEvent::PadAxis { axis, value } => {
                self.stick[usize::from(axis == PadAxis::StickY)] = i32::from(value) / 256;
            }
        }
    }

    /// One poll of the event loop, once per tick: SDL 1.2 repeats the held key.
    pub(crate) fn tick(&mut self) {
        self.ticks += 1;
        if let Some(repeat) = &mut self.repeat {
            repeat.elapsed += TICK_MS;
            if repeat.first {
                if repeat.elapsed > REPEAT_DELAY_MS {
                    repeat.first = false;
                    repeat.elapsed = 0;
                }
            } else if repeat.elapsed > REPEAT_INTERVAL_MS {
                repeat.elapsed = 0;
                self.remembered = scancode(repeat.key);
            }
        }
    }

    /// `eventDetected`: the remembered key, cleared, unless the joystick says something.
    pub(crate) fn take(&mut self) -> u8 {
        let key = std::mem::take(&mut self.remembered);
        let now = TICK_MS * self.ticks;
        let pushed = self.pad_pushed_ms.unwrap_or(now - UNSET_MS);
        let called = *self.pad_called_ms.get_or_insert(now - UNSET_MS);
        let pad = self.pad_code();
        if now >= self.hold_off_until_ms {
            if pad != 0 {
                if now - pushed >= FRESH_MS && now - called < FRESH_MS {
                    self.hold_off_until_ms = now + HOLD_OFF_MS;
                }
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
                pad
            } else {
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
                key
            }
        } else if pad != 0 {
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
            0
        } else {
            self.hold_off_until_ms = now;
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
            key
        }
    }

    /// The joystick's code, later checks winning as in the original: buttons over the stick,
    /// the vertical axis over the horizontal one.
    fn pad_code(&self) -> u8 {
        let [x, y] = self.stick;
        let mut code = 0;
        if x < -STICK_THRESHOLD {
            code = PAD_LEFT;
        }
        if x > STICK_THRESHOLD {
            code = PAD_RIGHT;
        }
        if y < -STICK_THRESHOLD {
            code = PAD_UP;
        }
        if y > STICK_THRESHOLD {
            code = PAD_DOWN;
        }
        for (pressed, button_code) in self.buttons.iter().zip([ENTER, ESCAPE, ENTER, ESCAPE]) {
            if *pressed {
                code = button_code;
            }
        }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, pressed: bool) -> InputEvent {
        InputEvent::Key { key, pressed }
    }

    #[test]
    fn the_last_key_down_is_remembered_until_it_is_read() {
        // One byte in the original: a second press replaces the first, and reading clears it.
        let mut keys = Keys::default();
        keys.event(key(Key::Up, true));
        keys.event(key(Key::Up, false));
        keys.event(key(Key::Escape, true));
        keys.event(key(Key::Escape, false));
        assert_eq!(keys.take(), ESCAPE);
        assert_eq!(keys.take(), 0);
    }

    #[test]
    fn arrows_report_the_keypads_codes() {
        // windib drops the 0xE0 prefix, so the menus see 0x48 for both Up and keypad 8.
        assert_eq!(scancode(Key::Up), scancode(Key::Kp8));
        assert_eq!(scancode(Key::KpEnter), ENTER);
        assert_eq!(scancode(Key::Y), Y);
        assert_eq!(scancode(Key::N), N);
    }

    #[test]
    fn a_held_key_repeats_after_half_a_second_then_every_third_tick() {
        // SDL 1.2's repeat, polled once a tick: the delay passes at the 36th poll (504 ms),
        // the first repeat comes 3 polls later (42 ms > 30 ms), then every 3 polls.
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        assert_eq!(keys.take(), DOWN);
        let polls: Vec<u32> = (1..=48)
            .filter(|_| {
                keys.tick();
                keys.take() == DOWN
            })
            .collect();
        assert_eq!(polls, [39, 42, 45, 48]);
    }

    #[test]
    fn releasing_the_key_or_holding_a_modifier_does_not_repeat() {
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        keys.event(key(Key::Down, false));
        keys.take();
        keys.event(key(Key::LeftShift, true));
        assert_eq!(keys.take(), 0x2A, "a modifier is still a key press");
        for _ in 0..100 {
            keys.tick();
            assert_eq!(keys.take(), 0);
        }
    }

    #[test]
    fn a_fresh_push_of_the_stick_holds_its_repeat_off_for_700_ms() {
        // The menus read every 2 ticks: a held push moves once, waits 700 ms, then moves on
        // every read.
        let mut keys = Keys::default();
        keys.set_pad_on(true);
        let reads: Vec<u8> = (0..60)
            .map(|read| {
                if read == 1 {
                    keys.event(InputEvent::PadAxis {
                        axis: PadAxis::StickY,
                        value: 20_000,
                    });
                }
                keys.tick();
                keys.tick();
                keys.take()
            })
            .collect();
        assert_eq!(reads[1], PAD_DOWN);
        // 700 ms is 25 reads of 28 ms; the hold-off ends at read 1 + 25.
        assert!(reads[2..26].iter().all(|&code| code == 0), "{reads:?}");
        assert!(
            reads[26..].iter().all(|&code| code == PAD_DOWN),
            "{reads:?}"
        );
    }

    #[test]
    fn pad_buttons_confirm_and_go_back() {
        // Buttons 0 and 2 answer like Enter, 1 and 3 like Escape.
        let mut keys = Keys::default();
        keys.set_pad_on(true);
        let push = |keys: &mut Keys, button, pressed| {
            keys.event(InputEvent::PadButton { button, pressed });
        };
        push(&mut keys, PadButton::A, true);
        assert_eq!(keys.take(), ENTER);
        push(&mut keys, PadButton::A, false);
        keys.tick();
        assert_eq!(keys.take(), 0);
        push(&mut keys, PadButton::B, true);
        keys.tick();
        assert_eq!(keys.take(), ESCAPE);
    }

    #[test]
    fn the_gamepad_counts_only_when_dr_cfg_switches_it_on() {
        // A fresh dr.cfg has it off, as the original's: a pad does nothing until the player
        // switches it on in Configure.
        let mut keys = Keys::default();
        keys.event(InputEvent::PadButton {
            button: PadButton::A,
            pressed: true,
        });
        assert_eq!(keys.take(), 0);
        keys.set_pad_on(true);
        assert_eq!(keys.take(), ENTER);
    }

    #[test]
    fn define_gamepad_takes_buttons_over_the_stick_and_the_last_button() {
        // 0x42CBF0 checks the stick, then buttons 1 to 4, each later one winning.
        let mut keys = Keys::default();
        assert_eq!(keys.pad_input(), 0);
        keys.event(InputEvent::PadAxis {
            axis: PadAxis::StickX,
            value: -20_000,
        });
        assert_eq!(keys.pad_input(), 1, "left");
        keys.event(InputEvent::PadAxis {
            axis: PadAxis::StickY,
            value: 20_000,
        });
        assert_eq!(keys.pad_input(), 4, "down over left");
        for (button, input) in [(PadButton::B, 6), (PadButton::Y, 8)] {
            keys.event(InputEvent::PadButton {
                button,
                pressed: true,
            });
            assert_eq!(keys.pad_input(), input);
        }
    }
}
```

<!-- write: crates/core/src/font.rs -->
```rust
//! The original's bitmap fonts (`drawTextWithFont`, 0x41A2D0; spec M2a §3.1): glyph `c - 32` of
//! a sheet of equal cells, drawn with colour 0 transparent, the pen moving by the glyph's
//! advance from `dr.exe`'s metrics.

use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::{GAP, Metrics};

use crate::canvas::Canvas;

#[derive(Clone, Debug)]
pub(crate) struct Font {
    glyphs: Vec<Image>,
    advances: Vec<u8>,
}

impl Font {
    pub(crate) fn new(glyphs: Vec<Image>, metrics: &Metrics) -> Font {
        Font {
            glyphs,
            advances: metrics.advances.clone(),
        }
    }

    /// The glyph and advance of byte `c`, if the font has one (characters 32 onwards).
    fn glyph(&self, c: u8) -> Option<(&Image, usize)> {
        let index = usize::from(c.checked_sub(32)?);
        Some((
            self.glyphs.get(index)?,
            usize::from(*self.advances.get(index)?),
        ))
    }

    /// Draws `text` with the pen starting at `offset`; returns where the pen ends.
    pub(crate) fn draw(&self, canvas: &mut Canvas, text: &[u8], offset: usize) -> usize {
        let mut pen = offset;
        for &c in text {
            if c == GAP {
                pen += 1;
            } else if let Some((glyph, advance)) = self.glyph(c) {
                canvas.draw(glyph, pen, true);
                pen += advance;
            }
        }
        pen
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::at;

    /// A 2x1 font: glyph `k` is filled with colour `k + 1`, its left pixel transparent for
    /// odd `k`; every advance is 3.
    pub(crate) fn font() -> Font {
        let glyphs = (0..96u8)
            .map(|k| {
                let left = if k % 2 == 1 { 0 } else { k + 1 };
                Image::new(2, 1, vec![left, k + 1])
            })
            .collect();
        Font::new(
            glyphs,
            &Metrics {
                width: 2,
                height: 1,
                advances: vec![3; 96],
            },
        )
    }

    #[test]
    fn each_byte_draws_its_glyph_and_moves_the_pen_by_its_advance() {
        // A wrong glyph offset prints the neighbouring letter; a wrong advance spaces the text.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, b" !\"", at(0, 0));
        assert_eq!(end, 9);
        assert_eq!(&canvas.pixels()[..8], [1, 1, 0, 0, 2, 0, 3, 3]);
    }

    #[test]
    fn the_gap_byte_moves_the_pen_one_pixel_and_draws_nothing() {
        // The menu table pads rows with 0xFA to place text a pixel at a time.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, &[GAP, b' '], at(0, 0));
        assert_eq!(end, 4);
        assert_eq!(&canvas.pixels()[..3], [0, 1, 1]);
    }

    #[test]
    fn a_texts_width_is_the_sum_of_its_advances() {
        // Configure's percentages are right-aligned by it; another width moves them.
        assert_eq!(font().width(b" !\""), 9);
        assert_eq!(font().width(&[7, b' ']), 3, "no glyph, no advance");
    }

    #[test]
    fn bytes_without_a_glyph_draw_nothing_and_do_not_move_the_pen() {
        let mut canvas = Canvas::default();
        assert_eq!(font().draw(&mut canvas, &[7, 200], at(5, 1)), at(5, 1));
        assert!(canvas.pixels().iter().all(|&p| p == 0));
    }
}
```

<!-- write: crates/core/src/audio/mod.rs -->
```rust
//! Sound: the music and effect players and the mixer they share (spec M1b §4.2).

pub(crate) mod effects;
pub(crate) mod mixer;
pub(crate) mod music;
pub(crate) mod tables;

use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::xm::Bank;

use self::effects::Effects;
use self::mixer::{UNITY, Voice, clip};
use self::music::Music;
use crate::AUDIO_CHANNELS;

/// FMOD's master volume for music, 0..=256, at a music volume of the game's configuration
/// (0..=0x10000): `mask * (volume >> 8) >> 9`, as `musicSetmusicVolume` (0x43C280) sets it,
/// with the game's volume mask (0x456A34) at 255 unless the end screen lowers it.
fn music_master(volume: u32, mask: u32) -> i64 {
    (i64::from(mask) * i64::from(volume >> 8)) >> 9
}

/// The volume mask's normal value.
const FULL_MASK: u32 = 255;

/// `dr.cfg`'s default effects volume, 75 % (`defaultConfig`, 0x426700).
pub(crate) const DEFAULT_EFFECTS_VOLUME: u32 = 0xC000;

/// The music volume the intro plays at, whatever `dr.cfg` says: the game's volume globals
/// start at 255 and take `dr.cfg`'s values only when the menu music starts.
pub(crate) const FULL_VOLUME: u32 = 0xFF00;

/// `dr.cfg`'s default music volume, 50 % (`defaultConfig`, 0x426700). The menus play at it
/// until M2's Configure menu can change it.
pub(crate) const DEFAULT_MUSIC_VOLUME: u32 = 0x8000;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug)]
pub(crate) struct Sound {
    music: Option<Music>,
    /// Voices of music that newer music replaced, fading out.
    fading: Vec<Voice>,
    effects: Option<Effects>,
    /// Voices of a bank that a newer bank replaced, fading out.
    fading_effects: Vec<Voice>,
    mix: Vec<i64>,
    /// The original's volume globals: the mask (0x456A34), the music's (0x456A30) and the
    /// effects' (0x456A2C) volumes as `dr.cfg` gives them; all full until the intro ends.
    mask: u32,
    music_volume: u32,
    effects_volume: u32,
}

impl Default for Sound {
    fn default() -> Sound {
        Sound {
            music: None,
            fading: Vec::new(),
            effects: None,
            fading_effects: Vec::new(),
            mix: Vec::new(),
            mask: FULL_MASK,
            music_volume: FULL_VOLUME,
            effects_volume: FULL_VOLUME,
        }
    }
}

impl Sound {
    /// Starts `module` at order `first_order` (counted as the game counts them) and the
    /// configured music `volume` (0..=0x10000), fading out any music playing.
    pub(crate) fn play_music(&mut self, module: &Module, first_order: usize, volume: u32) {
        if let Some(old) = self.music.take() {
            self.fading.extend(old.into_fading());
        }
        self.music_volume = volume;
        self.music = Some(Music::new(module, self.music_gain(), first_order));
    }

    fn music_gain(&self) -> i64 {
        UNITY * music_master(self.music_volume, self.mask) / 256
    }

    /// The effects stream's share: `mask * (volume >> 8) >> 8` of 255 (`musicSetVolume`,
    /// 0x43C250), 254 of 255 while the intro plays.
    fn effects_gain(&self) -> i64 {
        UNITY * ((i64::from(self.mask) * i64::from(self.effects_volume >> 8)) >> 8) / 255
    }

    /// The configured effects volume (0..=0x10000) for the effects stream.
    pub(crate) fn set_effects_volume(&mut self, volume: u32) {
        self.effects_volume = volume;
    }

    /// The volume mask (`setMusicVolume`, 0x43C2B0): 0..=255 over music and effects alike.
    pub(crate) fn set_mask(&mut self, mask: u32) {
        self.mask = mask;
        let gain = self.music_gain();
        if let Some(music) = &mut self.music {
            music.set_gain(gain);
        }
    }

    /// Makes `bank` the source of [`Sound::trigger`]; the old bank's effects fade out.
    pub(crate) fn load_effects(&mut self, bank: &Bank) {
        if let Some(old) = self.effects.take() {
            self.fading_effects.extend(old.into_fading());
        }
        self.effects = Some(Effects::new(bank));
    }

    /// Plays effect `effect` of the loaded bank on `channel` (both 1-based) at full volume and
    /// normal pitch, as the intro does.
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8) {
        self.trigger_at(channel, effect, effects::FULL, effects::FULL);
    }

    /// Plays effect `effect` on `channel` at `volume` and `pitch` (16.16, 0x10000 full and
    /// normal), as `loadMenuSoundEffect` (0x43C380) does.
    pub(crate) fn trigger_at(&mut self, channel: usize, effect: u8, volume: u32, pitch: u32) {
        if let Some(effects) = &mut self.effects {
            effects.trigger(channel, effect, volume, pitch);
        }
    }

    /// Stops the music and every effect, with a short fade so nothing clicks.
    pub(crate) fn stop(&mut self) {
        if let Some(music) = &mut self.music {
            music.stop();
        }
        if let Some(effects) = &mut self.effects {
            effects.stop_all();
        }
    }

    /// Appends `frames` interleaved stereo frames to `out`.
    pub(crate) fn render(&mut self, frames: usize, out: &mut Vec<i16>) {
        self.mix.clear();
        self.mix.resize(frames * AUDIO_CHANNELS, 0);
        if let Some(music) = &mut self.music {
            music.mix_into(&mut self.mix);
        }
        for voice in &mut self.fading {
            voice.mix_into(&mut self.mix);
        }
        self.fading.retain(|voice| !voice.finished());
        let mut part = vec![0; self.mix.len()];
        if let Some(effects) = &mut self.effects {
            effects.mix_into(&mut part);
        }
        for voice in &mut self.fading_effects {
            voice.mix_into(&mut part);
        }
        self.fading_effects.retain(|voice| !voice.finished());
        let gain = self.effects_gain();
        for (sum, effect) in self.mix.iter_mut().zip(part) {
            *sum += (effect * gain) >> 16;
        }
        out.extend(self.mix.iter().map(|&value| clip(value)));
    }
}

/// Renders `frames` stereo frames of `module` from its first order at the default music
/// volume, mixed as the game plays music in its menus and races.
#[must_use]
pub fn render_music(module: &Module, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.play_music(module, 0, DEFAULT_MUSIC_VOLUME);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

/// Renders effect `effect` (1-based) of `bank` at full volume and normal pitch, as the game
/// triggers it, for `frames` stereo frames.
#[must_use]
pub fn render_effect(bank: &Bank, effect: u8, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.load_effects(bank);
    sound.trigger(1, effect);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{self, Cell, Channel, Pattern, Sample};
    use deadrally_gamedata::xm::{Instrument, Looping};

    fn bank() -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![Some(Instrument {
                name: "Loud".into(),
                data: vec![i16::MAX; 20_000],
                looping: Looping::None,
                volume: 64,
                finetune: 0,
                relative_note: 0,
                panning: 255,
                fadeout: 0,
            })],
        }
    }

    #[test]
    fn nothing_loaded_is_silence_of_the_right_length() {
        // The frontend paces itself on the audio queue; every tick must deliver its samples.
        let mut out = Vec::new();
        Sound::default().render(672, &mut out);
        assert_eq!(out.len(), 672 * AUDIO_CHANNELS);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn overlapping_effects_clip_instead_of_wrapping() {
        // Four full-scale effects add up to twice the 16-bit range; wrapping would turn the
        // overload into noise, clipping only flattens it.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        for channel in 1..=4 {
            sound.trigger(channel, 1);
        }
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 900], i16::MAX, "left, panned hard left");
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn new_music_fades_the_old_out_instead_of_cutting_it() {
        // The intro's music gives way to the menu music; a cut at full level would click.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut silent = loud.clone();
        silent.orders.clear();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        assert!(before > 0);
        sound.play_music(&silent, 0, FULL_VOLUME);
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_configured_music_volume_sets_fmods_master_volume() {
        // musicSetmusicVolume (0x43C280): 255 * (volume >> 8) >> 9.
        assert_eq!(music_master(FULL_VOLUME, FULL_MASK), 127);
        assert_eq!(music_master(DEFAULT_MUSIC_VOLUME, FULL_MASK), 63);
        assert_eq!(music_master(0x1_0000, FULL_MASK), 127);
        assert_eq!(music_master(0, FULL_MASK), 0);
    }

    #[test]
    fn a_new_bank_lets_the_old_banks_effects_fade_out() {
        // The intro's effects stop as the menu's bank is loaded; cutting them at full level
        // would click.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        sound.stop();
        sound.load_effects(&bank());
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_effects_volume_and_the_mask_scale_the_effects_stream() {
        // The menus play effects at dr.cfg's 75 %: the stream at 255 * 192 >> 8 = 191 of 255
        // instead of the intro's 254. The end screen's mask lowers everything.
        let level = |sound: &mut Sound| {
            sound.load_effects(&bank());
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(1000, &mut out);
            i64::from(out[2 * 999])
        };
        let full = level(&mut Sound::default());
        let mut menu = Sound::default();
        menu.set_effects_volume(0xC000);
        let at_75 = level(&mut menu);
        assert!(
            (at_75 * 254 - full * 191).abs() <= 254 * 2,
            "{at_75} vs {full}"
        );
        let mut quiet = Sound::default();
        quiet.set_mask(0);
        assert_eq!(level(&mut quiet), 0);
    }

    /// One endless loud note on the first channel.
    fn loud() -> Module {
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        loud
    }

    #[test]
    fn the_mask_scales_the_music_while_it_plays() {
        // The end screen fades the music out through the mask, 255 down to 0.
        let loud = loud();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        sound.set_mask(0x80);
        out.clear();
        sound.render(2000, &mut out);
        // 255 * 255 >> 9 = 127 against 128 * 255 >> 9 = 63.
        assert!(
            (i64::from(out[2 * 1999]) * 127 - full * 63).abs() <= 127 * 2,
            "{} vs {full}",
            out[2 * 1999]
        );
    }

    #[test]
    fn the_music_volume_changes_the_music_while_it_plays() {
        // Configure's popup applies each step at once (`musicSetmusicVolume`): 50 % plays at
        // master volume 63, 100 % at 127.
        let mut sound = Sound::default();
        sound.play_music(&loud(), 0, DEFAULT_MUSIC_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let half = i64::from(out[2 * 1999]);
        sound.set_music_volume(0x1_0000);
        out.clear();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        assert!(
            (full * 63 - half * 127).abs() <= 127 * 2,
            "{half} vs {full}"
        );
    }

    #[test]
    fn stopping_fades_everything_out() {
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        sound.stop();
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 999], 0);
    }
}
```

<!-- write: crates/core/tests/common/mod.rs -->
```rust
//! Synthetic menu assets: every picture is one colour of its own, so a test can tell from a
//! pixel which picture, font or cursor frame the menu drew there. Real game data is never
//! committed.

use deadrally_gamedata::assets::{MenuAssets, Picture};
use deadrally_gamedata::dr_cfg::{DrCfg, HEADER_BYTES, PAYLOAD_BYTES};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::text::{ConfigureTexts, Metrics, Texts};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// `MENUBG5`, white in `MENU.PAL`.
pub const BACKGROUND: u8 = 1;
/// The big fonts: selected row (A), active row (B), dim row (D).
pub const BIG_A: u8 = 50;
pub const BIG_B: u8 = 51;
pub const BIG_D: u8 = 52;
/// The small fonts A, B and C.
pub const SMALL: [u8; 3] = [60, 61, 62];
/// Cursor frame k is colour `CURSOR + k`.
pub const CURSOR: u8 = 100;
/// The two credits screens and the end screen, each full red, green or blue in its palette.
pub const CREDITS: [u8; 2] = [200, 201];
pub const END: u8 = 202;
/// The menu's effects (`MEN-SAM`): the back sound plays on the left only, the move sound on
/// the right only, the choose sound on both sides.
pub const BACK_SOUND: usize = 22;
pub const MOVE_SOUND: usize = 25;
pub const CHOOSE_SOUND: usize = 28;
/// The volume popups' slider and knob.
pub const SLIDER: u8 = 70;
pub const KNOB: u8 = 71;

/// A `dr.cfg` with the original's default volumes, gamepad off.
pub fn config() -> DrCfg {
    let mut config = DrCfg::parse(&[0; HEADER_BYTES + PAYLOAD_BYTES]).unwrap();
    config.set_music_volume(0x8000);
    config.set_effects_volume(0xC000);
    config
}

fn solid(width: u32, height: u32, colour: u8) -> Image {
    Image::new(width, height, vec![colour; (width * height) as usize])
}

fn glyphs(size: u32, colour: u8) -> Vec<Image> {
    (0..96).map(|_| solid(size, size, colour)).collect()
}

fn full_screen(colour: u8, rgb: [u8; 3]) -> Picture {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(colour)] = rgb;
    Picture {
        image: solid(640, 480, colour),
        palette,
    }
}

/// A short tone, panned (255 left, 0 right, 128 both).
fn sound(panning: u8) -> Instrument {
    Instrument {
        name: "Beep".into(),
        data: vec![2000; 2000],
        looping: Looping::None,
        volume: 64,
        finetune: 0,
        relative_note: 0,
        panning,
        fadeout: 0,
    }
}

fn texts() -> Texts {
    let metrics = |size: u8| Metrics {
        width: size,
        height: size,
        advances: vec![size; 96],
    };
    Texts {
        // Every menu has six one-letter rows.
        menus: vec![
            (0..9)
                .map(|row| if row < 6 { b"M".to_vec() } else { Vec::new() })
                .collect();
            9
        ],
        panel: (0..4).map(|line| vec![b'a' + line]).collect(),
        exit_question: b"?".to_vec(),
        yes: b"Y".to_vec(),
        no: b"N".to_vec(),
        big: metrics(32),
        small: metrics(16),
        medium: metrics(9),
        configure: ConfigureTexts {
            adjust_music: b"m".to_vec(),
            adjust_effects: b"e".to_vec(),
            gamepad_on: b"+".to_vec(),
            gamepad_off: b"-".to_vec(),
            not_detected: b"!".to_vec(),
            press_any_key: b".".to_vec(),
            controls: vec![b"c".to_vec(); 8],
            key_prompts: vec![b"k".to_vec(); 8],
            pad_prompts: vec![b"p".to_vec(); 7],
            key_names: vec![b"K".to_vec(); 256],
            pad_names: vec![b"P".to_vec(); 9],
        },
    }
}

pub fn menu_assets() -> MenuAssets {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(BACKGROUND)] = [63, 63, 63];
    // The pulsing entries.
    palette.0[16..32].fill([63, 63, 63]);
    let mut copper = Palette::BLACK;
    copper.0[0] = [63, 0, 32];
    let mut effects = vec![None; CHOOSE_SOUND];
    effects[BACK_SOUND - 1] = Some(sound(255));
    effects[MOVE_SOUND - 1] = Some(sound(0));
    effects[CHOOSE_SOUND - 1] = Some(sound(128));
    MenuAssets {
        background: solid(640, 480, BACKGROUND),
        panel_line: solid(640, 10, 99),
        corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
        corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
        cursor: (0..50).map(|k| solid(20, 20, CURSOR + k)).collect(),
        big_a: glyphs(32, BIG_A),
        big_b: glyphs(32, BIG_B),
        big_d: glyphs(32, BIG_D),
        small_a: glyphs(16, SMALL[0]),
        small_b: glyphs(16, SMALL[1]),
        small_c: glyphs(16, SMALL[2]),
        palette,
        copper,
        background_copper: (0..512).map(|row| [(row % 64) as u8, 0, 0]).collect(),
        credits: vec![
            full_screen(CREDITS[0], [63, 0, 0]),
            full_screen(CREDITS[1], [0, 63, 0]),
        ],
        end: full_screen(END, [0, 0, 63]),
        effects: Bank {
            linear_frequencies: true,
            instruments: effects,
        },
        texts: texts(),
        slider: solid(172, 24, SLIDER),
        knob: solid(10, 24, KNOB),
        default_config: config(),
    }
}
```

<!-- write: crates/core/tests/menu.rs -->
```rust
//! The main menu on synthetic assets (spec M2a §3.2–§3.5): what a player of the original sees
//! and hears while moving through it. Every picture of the fixture has a colour of its own
//! (`common`), so a pixel tells which font, cursor frame or screen is drawn there.

mod common;

use common::{BACKGROUND, BIG_A, BIG_B, BIG_D, CREDITS, CURSOR, END, KNOB, SLIDER, SMALL};
use deadrally_core::{Game, InputEvent, Key, PadAxis, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::dr_cfg::DrCfg;
use deadrally_gamedata::haf::Animation;
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::Bank;

/// Without an intro: two logos of 25 + 180 + 26 ticks, the title's 25-tick fade-in, its
/// 26-tick fade to black and the menu's 50-tick fade-in.
const MENU_SHOWN: u32 = 2 * (25 + 180 + 26) + 25 + 26 + 50;
/// Where the main menu and the start submenu stand.
const MAIN: (usize, usize) = (145, 124);
const START: (usize, usize) = (109, 171);
const CONFIGURE: (usize, usize) = (95, 146);
/// Inside the exit question's "yes" and "no".
const YES: (usize, usize) = (212, 241);
const NO: (usize, usize) = (382, 241);

/// Which sides an effect sounded on: the fixture's back sound is left only, the move sound
/// right only, the choose sound both.
const SILENT: (bool, bool) = (false, false);
const BACK: (bool, bool) = (true, false);
const MOVE: (bool, bool) = (false, true);
const CHOOSE: (bool, bool) = (true, true);

fn picture(pixel: u8, width: u32, height: u32) -> Picture {
    Picture {
        image: Image::new(width, height, vec![pixel; (width * height) as usize]),
        palette: Palette::BLACK,
    }
}

/// A module that is silent, or plays one endless tone from order 45 on, where the original
/// starts the menu music.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let silent = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    let mut playing = silent.clone();
    playing.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone {
            [vec![1; 45], vec![0]].concat()
        } else {
            Vec::new()
        },
        initial_speed: 6,
        initial_tempo: 125,
        global_volume: 64,
        master_volume: 48,
        stereo: false,
        channels,
        samples: vec![Sample {
            name: "Tone".into(),
            c2spd: 8363,
            volume: 64,
            looped: Some((0, 1000)),
            data: vec![4000; 1000],
        }],
        patterns: vec![playing, silent],
    }
}

fn assets() -> Assets {
    Assets {
        intro: Animation::from_frames(Vec::new(), Vec::new()),
        letterbox: picture(0, 320, 200),
        apogee: picture(1, 4, 3),
        remedy: picture(2, 4, 3),
        title: picture(3, 640, 480),
        intro_music: music(false),
        intro_effects: Bank {
            linear_frequencies: true,
            instruments: Vec::new(),
        },
        menu_music: music(false),
        menu: common::menu_assets(),
    }
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// A game in the main menu, its fade-in over.
fn in_menu(assets: Assets) -> Game {
    in_menu_with(assets, common::config())
}

fn in_menu_with(assets: Assets, config: DrCfg) -> Game {
    let mut game = Game::new(assets, config);
    run(&mut game, MENU_SHOWN);
    game
}

fn press(game: &mut Game, key: Key) {
    game.input(InputEvent::Key { key, pressed: true });
    game.input(InputEvent::Key {
        key,
        pressed: false,
    });
}

/// Presses `key` and runs two menu passes: the first reads it, the second shows the result
/// everywhere (the exit question copies its words to the screen a pass later).
fn step(game: &mut Game, key: Key) {
    press(game, key);
    run(game, 4);
}

/// Waits for earlier effects to end, presses `key`, and tells which sides sounded.
fn sound_after(game: &mut Game, key: Key) -> (bool, bool) {
    run(game, 100);
    game.take_audio(&mut Vec::new());
    step(game, key);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    let side = |side: usize| audio.iter().skip(side).step_by(2).any(|&s| s != 0);
    (side(0), side(1))
}

fn pixel(game: &Game, (x, y): (usize, usize)) -> u8 {
    game.frame().pixels[y * 640 + x]
}

/// The font each of a menu's six rows is drawn in, read inside the row's glyph.
fn row_fonts(game: &Game, (x, y): (usize, usize)) -> Vec<u8> {
    (0..6)
        .map(|row| pixel(game, (x + 33, y + 15 + 28 * row)))
        .collect()
}

/// The main menu's highlighted row.
fn selected(game: &Game) -> usize {
    let fonts = row_fonts(game, MAIN);
    fonts
        .iter()
        .position(|&font| font == BIG_A)
        .unwrap_or_else(|| panic!("no row highlighted: {fonts:?}"))
}

/// How the shown palette makes `colour`.
fn colour(game: &Game, colour: u8) -> [u8; 3] {
    game.frame().palette[usize::from(colour)]
}

#[test]
fn the_menu_fades_in_after_the_title_and_stays_just_below_full_brightness() {
    // The original's fade-in stops at 98 %: white shows as 62, never 63.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, MENU_SHOWN - 1);
    let before = colour(&game, BACKGROUND);
    game.tick();
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (640, 480, (4, 3))
    );
    assert_eq!(pixel(&game, (5, 5)), BACKGROUND);
    assert!(before[0] < 62, "{before:?}");
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    run(&mut game, 300);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
}

#[test]
fn the_main_menu_highlights_start_and_dims_multiplayer() {
    let game = in_menu(assets());
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
    let cursor = pixel(&game, (MAIN.0 + 15, MAIN.1 + 16));
    assert!((CURSOR..CURSOR + 50).contains(&cursor), "{cursor}");
}

#[test]
fn the_bottom_panel_shows_the_start_up_lines_with_a_gap_before_the_last() {
    // `mainMenu` pushes three lines, an empty one and the last into the 22-line panel, which
    // shows its last six at (12, 378 + 15k) in small B.
    let game = in_menu(assets());
    let line = |k: usize| pixel(&game, (13, 378 + 15 * k + 8));
    assert_eq!(
        (0..6).map(line).collect::<Vec<_>>(),
        [
            BACKGROUND, SMALL[1], SMALL[1], SMALL[1], BACKGROUND, SMALL[1]
        ]
    );
}

#[test]
fn up_and_down_move_the_highlight_past_the_inactive_row_and_around() {
    let mut game = in_menu(assets());
    let rows: Vec<usize> = [Key::Down, Key::Down, Key::Up, Key::Up, Key::Up, Key::Down]
        .into_iter()
        .map(|key| {
            step(&mut game, key);
            selected(&game)
        })
        .collect();
    assert_eq!(rows, [2, 3, 2, 0, 5, 0]);
    let fonts = row_fonts(&game, MAIN);
    assert_eq!(fonts, [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]);
}

#[test]
fn each_move_sounds_and_other_keys_are_ignored() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(sound_after(&mut game, Key::Up), MOVE);
    assert_eq!(sound_after(&mut game, Key::A), SILENT);
    assert_eq!(selected(&game), 0);
}

#[test]
fn escape_jumps_to_exit_once() {
    // Escape in the main menu only moves the highlight to the last row; there it does
    // nothing, so a player cannot leave the game by pressing it twice.
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Escape), MOVE);
    assert_eq!(selected(&game), 5);
    assert_eq!(sound_after(&mut game, Key::Escape), SILENT);
    assert_eq!(selected(&game), 5);
    assert!(!game.quit_requested());
}

#[test]
fn the_cursor_turns_one_frame_every_menu_pass() {
    // A pass waits two ticks; the cursor runs through its 50 frames in 100 ticks.
    let mut game = in_menu(assets());
    let at = (MAIN.0 + 15, MAIN.1 + 16);
    let frames: Vec<u8> = (0..6)
        .map(|_| {
            game.tick();
            pixel(&game, at)
        })
        .collect();
    let k = frames[0];
    assert_eq!(frames, [k, k + 1, k + 1, k + 2, k + 2, k + 3]);
    run(&mut game, 100);
    assert_eq!(pixel(&game, at), k + 3);
}

#[test]
fn the_highlights_pulse_while_the_menu_waits() {
    // Entries 16–31 go down from 100 % to 49 % and back up in 34 ticks.
    let mut game = in_menu(assets());
    let levels: Vec<u8> = (0..35)
        .map(|_| {
            game.tick();
            colour(&game, 16)[0]
        })
        .collect();
    assert_eq!(levels[34], levels[0]);
    let lowest = *levels.iter().min().unwrap();
    assert!((30..=31).contains(&lowest), "{levels:?}");
    assert_eq!(*levels.iter().max().unwrap(), 63, "{levels:?}");
}

#[test]
fn a_held_key_moves_the_highlight_again_after_half_a_second() {
    let mut game = in_menu(assets());
    game.input(InputEvent::Key {
        key: Key::Down,
        pressed: true,
    });
    run(&mut game, 30);
    assert_eq!(selected(&game), 2, "one move in the first 420 ms");
    run(&mut game, 14);
    assert_ne!(selected(&game), 2, "SDL's repeat has started");
}

#[test]
fn the_gamepad_moves_and_chooses_like_the_keys() {
    // With the gamepad switched on in dr.cfg. `eventDetected` treats a push as fresh when it
    // was last called within 400 ms without the stick: a pass of the menu first.
    let mut config = common::config();
    config.set_use_joystick(1);
    let mut game = in_menu_with(assets(), config);
    run(&mut game, 2);
    game.input(InputEvent::PadAxis {
        axis: PadAxis::StickY,
        value: 20_000,
    });
    run(&mut game, 20);
    assert_eq!(
        selected(&game),
        2,
        "a push moves once, then holds off for 700 ms"
    );
    let stick = |game: &mut Game, value: i16| {
        game.input(InputEvent::PadAxis {
            axis: PadAxis::StickY,
            value,
        });
        run(game, 4);
    };
    stick(&mut game, 0);
    stick(&mut game, -20_000);
    stick(&mut game, 0);
    assert_eq!(selected(&game), 0);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: false,
    });
    run(&mut game, 4);
    assert_eq!(
        row_fonts(&game, MAIN)[0],
        BIG_D,
        "the main menu has lost focus"
    );
}

#[test]
fn start_opens_its_submenu_and_escape_closes_it() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(
        row_fonts(&game, START),
        [BIG_A, BIG_D, BIG_D, BIG_B, BIG_D, BIG_B],
        "only new game, load game and back are active at the first start"
    );
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(row_fonts(&game, START)[3], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Escape), BACK);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
}

#[test]
fn the_submenus_last_row_returns_and_starts_it_over_at_the_top() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    assert_eq!(row_fonts(&game, START)[5], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Space), CHOOSE);
    assert_eq!(selected(&game), 0, "back in the main menu");
    step(&mut game, Key::Enter);
    assert_eq!(row_fonts(&game, START)[0], BIG_A);
}

#[test]
fn the_hall_of_fame_does_nothing_when_chosen_until_m2c() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_B, BIG_A, BIG_B, BIG_B]
    );
}

#[test]
fn the_exit_question_starts_on_no_and_escape_answers_it() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Escape);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(sound_after(&mut game, Key::Left), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_A, BIG_B));
    assert_eq!(sound_after(&mut game, Key::Y), SILENT, "already on yes");
    assert_eq!(sound_after(&mut game, Key::N), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(sound_after(&mut game, Key::Escape), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_B, BIG_B, BIG_B, BIG_A]
    );
    run(&mut game, 1_000);
    assert!(!game.quit_requested());
}

/// Answers the exit question with yes; returns with the key not yet read.
fn answer_yes(game: &mut Game) {
    step(game, Key::Escape);
    step(game, Key::Enter);
    step(game, Key::Left);
    press(game, Key::Enter);
}

#[test]
fn yes_shows_the_end_screen_until_a_key_and_then_the_game_quits() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    // A pass reads the key; the menu takes 26 ticks to black, the end screen 25 to 96 %.
    run(&mut game, 2 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), END);
    assert_eq!(colour(&game, END), [0, 0, 60]);
    run(&mut game, 300);
    assert!(!game.quit_requested());
    press(&mut game, Key::Space);
    // The hold reads the key at once; the fade-out takes 26 ticks.
    run(&mut game, 26);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
    assert_eq!(colour(&game, END), [0, 0, 0]);
}

#[test]
fn the_end_screen_stops_waiting_after_560_ticks() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    run(&mut game, 2 + 26 + 25 + 560 + 25);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
}

#[test]
fn the_music_fades_out_with_the_end_screen() {
    let mut with_music = assets();
    with_music.menu_music = music(true);
    let mut game = in_menu(with_music);
    answer_yes(&mut game);
    run(&mut game, 100);
    let loudest = |game: &mut Game| {
        let mut audio = Vec::new();
        game.take_audio(&mut audio);
        audio.iter().map(|&s| s.unsigned_abs()).max().unwrap()
    };
    let held = loudest(&mut game);
    assert!(held > 0);
    press(&mut game, Key::Space);
    run(&mut game, 14);
    let halfway = loudest(&mut game);
    assert!(halfway < held, "{halfway} vs {held}");
    run(&mut game, 13);
    assert!(game.quit_requested());
    // The music's next tick takes the last step to silence.
    run(&mut game, 2);
    loudest(&mut game);
    run(&mut game, 1);
    assert_eq!(loudest(&mut game), 0, "silent once the game has ended");
}

#[test]
fn the_credits_show_two_screens_each_until_a_key_and_return_to_the_menu() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    assert_eq!(selected(&game), 4);
    press(&mut game, Key::Enter);
    // A pass reads the key; the menu fades out in 51 ticks, the first screen in in 25.
    run(&mut game, 2 + 51 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0]);
    assert_eq!(colour(&game, CREDITS[0]), [60, 0, 0]);
    run(&mut game, 500);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "it waits for a key");
    press(&mut game, Key::Space);
    run(&mut game, 1 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
    press(&mut game, Key::Space);
    // Back to the menu as it was, fading in over 50 ticks.
    run(&mut game, 1 + 26 + 50);
    assert_eq!(selected(&game), 4);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    step(&mut game, Key::Down);
    assert_eq!(selected(&game), 5, "the menu works again");
}

#[test]
fn a_key_during_a_credits_fade_in_moves_on_as_soon_as_it_is_done() {
    // The original reads the key before the screen's first wait, so an impatient player does
    // not see it held at all.
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 51 + 10);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "fading in");
    press(&mut game, Key::Space);
    // The rest of the fade-in, the fade to black and the second screen's fade-in.
    run(&mut game, 15 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
}

/// A game in Configure, the file written at start-up taken.
fn in_configure(config: DrCfg) -> Game {
    let mut game = in_menu_with(assets(), config);
    game.take_config();
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    game
}

/// The `dr.cfg` the game hands out now, if it writes one.
fn written(game: &mut Game) -> Option<DrCfg> {
    game.take_config()
        .map(|bytes| DrCfg::parse(&bytes).unwrap())
}

/// Where the volume popup's knob shows level `level`.
fn knob_at(level: usize) -> (usize, usize) {
    (329 + level + 5, 260)
}

#[test]
fn dr_cfg_is_written_at_start_up_counting_the_start() {
    // mainMenu counts every start and writes the file before the intro.
    let mut config = common::config();
    config.set_times_played(4);
    let mut game = Game::new(assets(), config);
    game.tick();
    assert_eq!(written(&mut game).map(|cfg| cfg.times_played()), Some(5));
    run(&mut game, 50);
    assert!(game.take_config().is_none(), "only once");
}

#[test]
fn configure_opens_over_the_dimmed_main_menu_and_escape_returns_writing_dr_cfg() {
    let mut game = in_configure(common::config());
    assert_eq!(
        row_fonts(&game, CONFIGURE),
        [BIG_A, BIG_B, BIG_B, BIG_B, BIG_B, BIG_B]
    );
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert!(game.take_config().is_none());
    assert_eq!(sound_after(&mut game, Key::Escape), BACK);
    assert_eq!(selected(&game), 2, "back on the configure row");
    assert!(
        written(&mut game).is_some(),
        "leaving Configure writes dr.cfg"
    );
}

#[test]
fn the_music_volume_moves_by_two_levels_and_is_kept_on_enter() {
    // The level is the volume / 512: 64 for the default 50 %; three steps left make 58.
    let mut game = in_configure(common::config());
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, (320, 255)), SLIDER);
    assert_eq!(pixel(&game, knob_at(64)), KNOB);
    for _ in 0..3 {
        press(&mut game, Key::Left);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(58)), KNOB);
    assert_eq!(sound_after(&mut game, Key::Enter), BACK);
    assert_eq!(row_fonts(&game, CONFIGURE)[0], BIG_A, "back in Configure");
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.music_volume(), 58 * 512);
    assert_eq!(config.effects_volume(), 0xC000, "the effects untouched");
}

#[test]
fn a_volume_stops_at_its_ends() {
    let mut game = in_configure(common::config());
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    for _ in 0..80 {
        press(&mut game, Key::Right);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(128)), KNOB);
    for _ in 0..80 {
        press(&mut game, Key::Left);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(0)), KNOB);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().effects_volume(), 0);
}

#[test]
fn define_keyboard_takes_the_next_key_for_the_chosen_control() {
    let mut game = in_configure(common::config());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    run(&mut game, 10);
    step(&mut game, Key::Q);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.key(1), 0x10, "brake on Q");
    assert_eq!(config.key(0), 0, "accelerate untouched");
}

#[test]
fn define_gamepad_waits_for_the_pad_to_settle_and_enter_means_none() {
    // While it waits, the original's key reads leave the gamepad alone: a button is taken as
    // an input, not as Enter or Escape.
    let mut config = common::config();
    config.set_use_joystick(1);
    config.set_pad(0, 5);
    config.set_pad(1, 4);
    let mut game = in_configure(config);
    game.input(InputEvent::PadConnected { connected: true });
    for _ in 0..3 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    let button = |game: &mut Game, pressed: bool| {
        game.input(InputEvent::PadButton {
            button: PadButton::Y,
            pressed,
        });
    };
    button(&mut game, true);
    run(&mut game, 30);
    button(&mut game, false);
    run(&mut game, 30);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    run(&mut game, 4);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.pad(0), 8, "button 4");
    assert_eq!(config.pad(1), 0, "none");
}

#[test]
fn the_gamepad_switch_without_a_gamepad_shows_the_popup_until_a_key() {
    let mut game = in_configure(common::config());
    for _ in 0..4 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, (141, 218)), BIG_A, "not detected");
    run(&mut game, 100);
    assert_eq!(pixel(&game, (141, 218)), BIG_A, "until a key");
    step(&mut game, Key::Space);
    assert_eq!(row_fonts(&game, CONFIGURE)[4], BIG_A, "back in Configure");
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 0);
}

#[test]
fn the_gamepad_switch_turns_a_connected_gamepad_on_and_off() {
    let mut game = in_configure(common::config());
    game.input(InputEvent::PadConnected { connected: true });
    for _ in 0..4 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 1);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 0);
}

#[test]
fn previous_menu_returns_and_starts_configure_over_at_its_first_row() {
    // Escape keeps Configure's row; its last row also sets it back.
    let mut game = in_configure(common::config());
    for _ in 0..5 {
        step(&mut game, Key::Down);
    }
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(selected(&game), 2);
    assert!(written(&mut game).is_some());
    step(&mut game, Key::Enter);
    assert_eq!(row_fonts(&game, CONFIGURE)[0], BIG_A);
}

#[test]
fn dr_cfg_is_written_after_the_end_screen() {
    let mut game = in_menu(assets());
    game.take_config();
    step(&mut game, Key::Escape);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Left);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 26 + 25 + 560 + 25);
    assert!(game.take_config().is_none());
    game.tick();
    assert!(game.quit_requested());
    assert!(game.take_config().is_some());
}
```

<!-- write: crates/core/tests/startup.rs -->
```rust
//! The startup sequence's timeline on synthetic assets (spec M1a §5.2). Each test pins down
//! something a player of the original would notice: a logo that holds too long, a fade that
//! ends at the wrong brightness, a key that does not skip.

mod common;

use deadrally_core::{AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, Game, InputEvent, Key, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use std::path::PathBuf;

use deadrally_gamedata::haf::{Animation, FRAME_PIXELS, HafFrame};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// Intro delays: frame 0 at tick 4, frame 1 at tick 6, the last frame at tick 9.
const DELAYS: [u8; 3] = [4, 2, 3];
const INTRO_END: u32 = 9;
/// Pixel values that tell the pictures apart; each picture's palette makes its own colour
/// full red, green or blue.
const APOGEE: u8 = 1;
const REMEDY: u8 = 2;
const TITLE: u8 = 3;
/// A logo takes 25 fade-in ticks, 180 hold ticks and 26 fade-out ticks.
const FADE_IN: u32 = 25;
const HOLD: u32 = 180;
const LOGO: u32 = FADE_IN + HOLD + 26;

fn palette(entries: &[(usize, [u8; 3])]) -> Palette {
    let mut palette = Palette::BLACK;
    for &(index, rgb) in entries {
        palette.0[index] = rgb;
    }
    palette
}

fn picture(pixel: u8, rgb: [u8; 3]) -> Picture {
    Picture {
        image: Image::new(4, 3, vec![pixel; 12]),
        palette: palette(&[(usize::from(pixel), rgb)]),
    }
}

/// Frame `k` is all pixel `16 + k`, coloured grey level `10 + k`.
fn intro_frame(k: u8) -> HafFrame {
    HafFrame {
        // Entries below 16 belong to the letterbox; the intro must not take them from frames.
        palette: palette(&[(0, [63, 63, 63]), (usize::from(16 + k), [10 + k; 3])]),
        pixels: vec![16 + k; FRAME_PIXELS],
    }
}

fn assets() -> Assets {
    let mut letterbox = vec![0u8; 320 * 200];
    letterbox[..320].fill(5);
    Assets {
        intro: Animation::from_frames(DELAYS.to_vec(), (0..3).map(intro_frame).collect()),
        letterbox: Picture {
            image: Image::new(320, 200, letterbox),
            // Entries from 16 on must stay black until the first frame sets them.
            palette: palette(&[(5, [20, 30, 40]), (16, [63, 63, 63])]),
        },
        apogee: picture(APOGEE, [63, 0, 0]),
        remedy: picture(REMEDY, [0, 63, 0]),
        // The main menu takes the title over as its 640x480 screen.
        title: Picture {
            image: Image::new(640, 480, vec![TITLE; 640 * 480]),
            ..picture(TITLE, [0, 0, 63])
        },
        intro_music: music(false),
        intro_effects: effects(),
        menu_music: music(false),
        menu: common::menu_assets(),
    }
}

/// A module that is silent, or plays one endless tone from its first row.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let mut pattern = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    pattern.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone { vec![0] } else { Vec::new() },
        initial_speed: 6,
        initial_tempo: 125,
        global_volume: 64,
        master_volume: 48,
        stereo: false,
        channels,
        samples: vec![Sample {
            name: "Tone".into(),
            c2spd: 8363,
            volume: 64,
            looped: Some((0, 1000)),
            data: vec![4000; 1000],
        }],
        patterns: vec![pattern],
    }
}

/// Music that is silent from its first order and plays a tone from order 45, where the original
/// starts the menu music.
fn menu_music() -> Module {
    let mut module = music(true);
    module.patterns.push(s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    });
    module.orders = [vec![1; 45], vec![0]].concat();
    module
}

/// Effect 1 is an endless constant tone.
fn effects() -> Bank {
    Bank {
        linear_frequencies: true,
        instruments: vec![Some(Instrument {
            name: "Hum".into(),
            data: vec![1000; 1000],
            looping: Looping::Forward {
                start: 0,
                length: 1000,
            },
            volume: 64,
            finetune: 0,
            relative_note: 0,
            panning: 128,
            fadeout: 0,
        })],
    }
}

/// The last left sample of each of the next `ticks` ticks.
fn loudness(game: &mut Game, ticks: u32) -> Vec<i16> {
    (0..ticks)
        .map(|_| {
            game.tick();
            let mut audio = Vec::new();
            game.take_audio(&mut audio);
            audio[audio.len() - 2]
        })
        .collect()
}

fn press(game: &mut Game) {
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: true,
    });
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: false,
    });
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// Which picture is on screen and how bright its colour is (0..=63).
fn shown(game: &Game) -> (u8, u8) {
    let frame = game.frame();
    let pixel = frame.pixels[0];
    let brightness = frame.palette[usize::from(pixel)].into_iter().max().unwrap();
    (pixel, brightness)
}

/// The intro row shown at row 40 (the first animation row) and its colour.
fn intro_row(game: &Game) -> (u8, [u8; 3]) {
    let frame = game.frame();
    assert_eq!((frame.width, frame.height), (320, 200));
    let pixel = frame.pixels[40 * 320];
    (pixel, frame.palette[usize::from(pixel)])
}

#[test]
fn the_intro_starts_with_the_letterbox_and_black_animation_colours() {
    let game = Game::new(assets(), common::config());
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (320, 200, (4, 3))
    );
    assert_eq!(frame.pixels[0], 5);
    assert_eq!(frame.palette[5], [20, 30, 40]);
    assert_eq!(frame.palette[16], [0, 0, 0]);
}

#[test]
fn each_intro_frame_appears_its_delay_after_the_previous_one() {
    // The intro is cut to its music (M1b); a frame early or late drifts out of sync.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, 3);
    assert_eq!(intro_row(&game), (0, [0, 0, 0]), "nothing before tick 4");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (16, [10, 10, 10]), "frame 0 at tick 4");
    assert_eq!(
        game.frame().palette[0],
        [0, 0, 0],
        "entries below 16 stay the letterbox's"
    );
    run(&mut game, 1);
    assert_eq!(intro_row(&game).0, 16, "frame 0 still at tick 5");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (17, [11, 11, 11]), "frame 1 at tick 6");
}

#[test]
fn the_last_intro_frame_is_never_shown() {
    // openAnimation blacks the palette as soon as the last frame is drawn, before it is shown.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, INTRO_END - 1);
    assert_eq!(intro_row(&game).0, 17);
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn a_key_ends_the_intro_when_the_next_frame_is_due() {
    // The original checks for a key once per frame, so the intro runs on until the next frame
    // would have been shown.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, 1);
    press(&mut game);
    run(&mut game, 2);
    assert_eq!(intro_row(&game).0, 0, "still waiting for frame 0");
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0), "frame 0 is skipped too");
}

#[test]
fn a_corrupt_intro_frame_ends_the_intro_instead_of_crashing() {
    // Only data of an unknown version can hold one, and the player was warned at start-up;
    // the game should still reach its menus. The broken frame comes first: the last frame is
    // never decoded, so it could not show the problem.
    let mut record = vec![0u8; 768];
    record.extend([8, 2, 0xFF, 0xFF, 0, 0x3B]);
    let mut haf = vec![2, 0, 0, 0, 1, 1];
    for _ in 0..2 {
        haf.extend(u16::try_from(record.len()).unwrap().to_le_bytes());
        haf.extend(&record);
    }
    let mut broken = assets();
    broken.intro = Animation::from_bytes(PathBuf::from("BROKEN.HAF"), haf).unwrap();
    assert!(broken.intro.frame(0).is_err());
    let mut game = Game::new(broken, common::config());
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn an_empty_intro_goes_straight_to_the_logos() {
    let mut empty = assets();
    empty.intro = Animation::from_frames(Vec::new(), Vec::new());
    let mut game = Game::new(empty, common::config());
    assert_eq!(shown(&game), (APOGEE, 0));
    run(&mut game, FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
}

#[test]
fn an_empty_intro_still_starts_the_menu_music() {
    // The original starts the menu music after `checkAndOpenAnimation`, whether or not that
    // played anything.
    let mut empty = assets();
    empty.intro = Animation::from_frames(Vec::new(), Vec::new());
    empty.menu_music = menu_music();
    let mut game = Game::new(empty, common::config());
    assert!(loudness(&mut game, 3).iter().all(|&level| level > 0));
}

#[test]
fn a_pad_button_skips_like_a_key_once_dr_cfg_switches_the_gamepad_on() {
    let mut config = common::config();
    config.set_use_joystick(1);
    let mut game = Game::new(assets(), config);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    assert_eq!(shown(&game).0, APOGEE);
}

/// Red brightness of the Apogee logo for each tick after the intro.
fn apogee_brightness(game: &mut Game, ticks: u32) -> Vec<u8> {
    (0..ticks)
        .map(|_| {
            game.tick();
            let (pixel, brightness) = shown(game);
            assert_eq!(pixel, APOGEE);
            brightness
        })
        .collect()
}

#[test]
fn a_logo_fades_in_holds_and_fades_out_like_the_original() {
    let mut game = Game::new(assets(), common::config());
    run(&mut game, INTRO_END);
    let brightness = apogee_brightness(&mut game, LOGO - 1);
    // The fade-in climbs one 4 % step per tick from black and stops at 96 %: 63 shows as 60.
    assert_eq!(&brightness[..4], [0, 3, 5, 8]);
    assert_eq!(brightness[24], 60);
    // The hold lasts 180 ticks when nobody presses a key.
    assert!(brightness[25..205].iter().all(|&level| level == 60));
    // The fade-out starts at 100 %, a visible flash from 60 to 63, and steps down to black.
    assert_eq!(&brightness[205..208], [63, 60, 58]);
    assert_eq!(brightness[229], 3);
    // Its last, black step already has Remedy drawn under it: the same black screen.
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 0));
}

#[test]
fn a_key_during_the_fade_in_ends_the_hold_after_one_tick() {
    // The original remembers the press until the hold asks, so impatient players see the logo
    // at full fade for a single tick.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, INTRO_END + 5);
    press(&mut game);
    let brightness = apogee_brightness(&mut game, FADE_IN - 5 + 1);
    assert_eq!(brightness.last(), Some(&60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63), "the fade-out starts");
}

#[test]
fn a_key_during_a_fade_out_ends_the_next_logos_hold_after_one_tick() {
    // The remembered press survives the change of screen, as in the original: a player who
    // presses while Apogee fades out sees the Remedy logo for a single hold tick.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, INTRO_END + FADE_IN + HOLD + 10);
    assert_eq!(shown(&game).0, APOGEE, "Apogee is fading out");
    press(&mut game);
    run(&mut game, LOGO - FADE_IN - HOLD - 10 + FADE_IN);
    assert_eq!(shown(&game), (REMEDY, 60), "Remedy's fade-in is done");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 63), "the fade-out starts");
}

#[test]
fn a_key_with_the_last_intro_frame_carries_into_the_apogee_hold() {
    // openAnimation stops after its last frame without checking for a key, so the press waits
    // for the Apogee hold.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, 7);
    press(&mut game);
    run(&mut game, INTRO_END - 7 + FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn a_key_during_the_hold_starts_the_fade_out_on_the_next_tick() {
    let mut game = Game::new(assets(), common::config());
    run(&mut game, INTRO_END + FADE_IN + 50);
    press(&mut game);
    game.tick();
    assert_eq!(
        shown(&game),
        (APOGEE, 60),
        "the hold tick that reads the key"
    );
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn the_title_fades_in_after_both_logos_and_then_to_black_for_the_menu() {
    // Measured on the original: the title fades in to 92 % (63 as 58); loading the main menu
    // shows the pending 96 % step, and `transitionToBlack` then takes the title from 100 % to
    // black in 26 ticks. The original's load takes a moment, so it may skip a step or two of
    // the fade to black; here loading takes no time (spec M2a decision 5).
    let mut game = Game::new(assets(), common::config());
    run(&mut game, INTRO_END + 2 * LOGO);
    assert_eq!(shown(&game), (TITLE, 0));
    run(&mut game, FADE_IN - 1);
    assert_eq!(shown(&game), (TITLE, 58));
    let brightness: Vec<u8> = (0..27)
        .map(|_| {
            game.tick();
            shown(&game).1
        })
        .collect();
    assert_eq!(&brightness[..4], [60, 63, 60, 58], "{brightness:?}");
    assert_eq!(brightness[25], 3, "{brightness:?}");
    assert_eq!(brightness[26], 0, "{brightness:?}");
}

#[test]
fn the_audio_stream_stays_full_when_nothing_plays() {
    // The frontend paces itself on the audio queue; missing samples would stall or drift it.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, 30);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), 30 * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS);
    assert!(
        audio.iter().all(|&sample| sample == 0),
        "no music, no effects in these assets"
    );
}

#[test]
fn the_intro_plays_its_music_and_stops_it_when_it_ends() {
    // The original stops the song as the intro ends; music running on would play over the
    // logos.
    let mut with_music = assets();
    with_music.intro_music = music(true);
    let mut game = Game::new(with_music, common::config());
    let during = loudness(&mut game, INTRO_END - 1);
    assert!(during.iter().all(|&level| level > 0), "{during:?}");
    let after = loudness(&mut game, 5);
    assert_eq!(&after[1..], [0, 0, 0, 0], "{after:?}");
}

#[test]
fn the_menu_music_starts_when_the_intro_ends_and_plays_through_the_logos() {
    // `mainMenu` starts the menu music from order 45 right after the intro, before the logos;
    // they and the title are not silent.
    let mut with_menu = assets();
    with_menu.menu_music = menu_music();
    let mut game = Game::new(with_menu, common::config());
    let intro = loudness(&mut game, INTRO_END - 1);
    assert!(intro.iter().all(|&level| level == 0), "{intro:?}");
    let after = loudness(&mut game, LOGO + 10);
    assert!(after[1..].iter().all(|&level| level > 0), "{after:?}");
    assert_eq!(
        shown(&game).0,
        REMEDY,
        "the music goes on through the logos"
    );
}

#[test]
fn the_menu_music_plays_at_the_default_configurations_half_volume() {
    // The intro always plays at full volume: the original applies dr.cfg's volumes only when it
    // starts the menu music, and a fresh dr.cfg has the music at 50 % (FMOD master volume 63
    // against the intro's 127).
    let mut same = assets();
    let mut tone = music(true);
    tone.orders = vec![0; 46];
    same.intro_music = tone.clone();
    same.menu_music = tone;
    let mut game = Game::new(same, common::config());
    let intro = i32::from(loudness(&mut game, INTRO_END - 1)[2]);
    let menu = i32::from(loudness(&mut game, 5)[4]);
    assert!(intro > 0, "{intro}");
    assert!(
        (menu * 127 - intro * 63).abs() <= 127 * 2,
        "{menu} vs {intro}"
    );
}

#[test]
fn a_key_that_ends_the_intro_stops_its_sound() {
    // As in the original, the intro's music stops, and frames that were still due never start
    // their effects (the menu music, silent in these assets, takes over).
    let mut with_music = assets();
    with_music.intro_music = music(true);
    with_music.intro.effects = vec![1, 1, 1];
    let mut game = Game::new(with_music, common::config());
    run(&mut game, 1);
    press(&mut game);
    loudness(&mut game, 3);
    let after = loudness(&mut game, 10);
    assert!(after[1..].iter().all(|&level| level == 0), "{after:?}");
}

#[test]
fn a_frames_effect_sounds_when_the_frame_is_shown() {
    // Effects mark moments of the intro's picture; one early or late is out of sync with it.
    let mut timed = assets();
    timed.intro.effects = vec![0, 1, 0];
    let mut game = Game::new(timed, common::config());
    let levels = loudness(&mut game, 6);
    assert_eq!(&levels[..5], [0, 0, 0, 0, 0], "frame 1 appears at tick 6");
    assert!(levels[5] > 0);
}

#[test]
fn the_intros_effects_take_channels_one_to_six_in_turn() {
    // The seventh effect reuses channel 1 and cuts the first one off, so at most six effects
    // sound together, as in the original.
    let mut busy = assets();
    busy.intro = Animation::from_frames(vec![1; 10], (0..10).map(|k| intro_frame(k % 3)).collect());
    busy.intro.effects = vec![1; 10];
    let mut game = Game::new(busy, common::config());
    let levels = loudness(&mut game, 9);
    let one = i32::from(levels[0]);
    assert!(one > 0);
    for (voices, &level) in levels.iter().enumerate().take(6) {
        let expected = one * (voices as i32 + 1);
        assert!(
            (i32::from(level) - expected).abs() <= 6,
            "{voices}: {levels:?}"
        );
    }
    assert!(
        (i32::from(levels[7]) - 6 * one).abs() <= 6,
        "still six voices: {levels:?}"
    );
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-core --no-run`
Expected: FAIL to compile (`set_pad_on`, `width`, `set_music_volume` do not exist; `Game::new` takes one argument).

- [ ] **Step 3: Implement**

<!-- write: crates/core/src/input.rs -->
```rust
/// Something the player did. Frontends forward state changes only (no OS key repeat) and keep
/// presentation keys (Alt+Enter, F12) to themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Key {
        key: Key,
        pressed: bool,
    },
    PadButton {
        button: PadButton,
        pressed: bool,
    },
    /// Stick position from -32768 (left or up) to 32767 (right or down).
    PadAxis {
        axis: PadAxis,
        value: i16,
    },
    /// Whether a gamepad is connected now.
    PadConnected {
        connected: bool,
    },
}

/// The four face buttons of "one stick, four buttons" (brief §1), named by position with Xbox
/// labels: A bottom, B right, X left, Y top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PadButton {
    A,
    B,
    X,
    Y,
}

impl PadButton {
    /// Every button; `PadButton::ALL[b as usize] == b`.
    pub const ALL: [PadButton; 4] = [PadButton::A, PadButton::B, PadButton::X, PadButton::Y];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PadAxis {
    StickX,
    StickY,
}

macro_rules! keys {
    ($($key:ident),+ $(,)?) => {
        /// A physical key position named after the US layout, independent of the active
        /// keyboard layout. M2 adds the mapping to the original's scancodes.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Key {
            $($key),+
        }

        impl Key {
            /// Every key in declaration order; `Key::ALL[k as usize] == k`.
            pub const ALL: &'static [Key] = &[$(Key::$key),+];
        }
    };
}

keys![
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Digit0, Digit1,
    Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, F1, F2, F3, F4, F5, F6, F7, F8,
    F9, F10, F11, F12, Up, Down, Left, Right, Enter, Escape, Space, Backspace, Tab, LeftShift,
    RightShift, LeftCtrl, RightCtrl, LeftAlt, RightAlt, Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7,
    Kp8, Kp9, KpPlus, KpMinus, KpMultiply, KpDivide, KpEnter, KpPeriod,
];
```

<!-- write: crates/core/src/keys.rs -->
```rust
//! The player's input as the original reads it (spec M2a §3.3): one remembered key, its PC
//! set-1 scancode as SDL 1.2's `windib` driver reports it, kept until `eventDetected`
//! (0x417EB0) reads and clears it; SDL 1.2's key repeat (`SDL_EnableKeyRepeat(500, 30)`); and
//! the joystick, polled by `eventDetected` itself.

use crate::input::{InputEvent, Key, PadAxis, PadButton};

/// Scancodes the menus act on.
pub(crate) const ESCAPE: u8 = 0x01;
pub(crate) const ENTER: u8 = 0x1C;
pub(crate) const SPACE: u8 = 0x39;
pub(crate) const UP: u8 = 0x48;
pub(crate) const DOWN: u8 = 0x50;
pub(crate) const LEFT: u8 = 0x4B;
pub(crate) const RIGHT: u8 = 0x4D;
pub(crate) const Y: u8 = 0x15;
pub(crate) const N: u8 = 0x31;
/// The joystick's codes are DirectInput's: set-1 with the extended bit.
pub(crate) const PAD_LEFT: u8 = 0xCB;
pub(crate) const PAD_RIGHT: u8 = 0xCD;
pub(crate) const PAD_UP: u8 = 0xC8;
pub(crate) const PAD_DOWN: u8 = 0xD0;

/// One tick, in SDL milliseconds.
const TICK_MS: i64 = 14;
/// SDL 1.2 key repeat: the delay before the first repeat and the interval after it.
const REPEAT_DELAY_MS: i64 = 500;
const REPEAT_INTERVAL_MS: i64 = 30;
/// `eventDetected`'s joystick: stick positions past ±50 (raw / 256) count; a first push holds
/// the repeat off for 700 ms; 400 ms tell a fresh push from a held one.
const STICK_THRESHOLD: i32 = 50;
const HOLD_OFF_MS: i64 = 700;
const FRESH_MS: i64 = 400;
const UNSET_MS: i64 = 250;

/// The PC set-1 scancode SDL 1.2's `windib` driver reports for a key (no 0xE0 prefix, so the
/// arrows share the keypad's codes).
pub(crate) fn scancode(key: Key) -> u8 {
    match key {
        Key::Escape => 0x01,
        Key::Digit1 => 0x02,
        Key::Digit2 => 0x03,
        Key::Digit3 => 0x04,
        Key::Digit4 => 0x05,
        Key::Digit5 => 0x06,
        Key::Digit6 => 0x07,
        Key::Digit7 => 0x08,
        Key::Digit8 => 0x09,
        Key::Digit9 => 0x0A,
        Key::Digit0 => 0x0B,
        Key::Backspace => 0x0E,
        Key::Tab => 0x0F,
        Key::Q => 0x10,
        Key::W => 0x11,
        Key::E => 0x12,
        Key::R => 0x13,
        Key::T => 0x14,
        Key::Y => 0x15,
        Key::U => 0x16,
        Key::I => 0x17,
        Key::O => 0x18,
        Key::P => 0x19,
        Key::Enter | Key::KpEnter => 0x1C,
        Key::LeftCtrl | Key::RightCtrl => 0x1D,
        Key::A => 0x1E,
        Key::S => 0x1F,
        Key::D => 0x20,
        Key::F => 0x21,
        Key::G => 0x22,
        Key::H => 0x23,
        Key::J => 0x24,
        Key::K => 0x25,
        Key::L => 0x26,
        Key::LeftShift => 0x2A,
        Key::Z => 0x2C,
        Key::X => 0x2D,
        Key::C => 0x2E,
        Key::V => 0x2F,
        Key::B => 0x30,
        Key::N => 0x31,
        Key::M => 0x32,
        Key::KpDivide => 0x35,
        Key::RightShift => 0x36,
        Key::KpMultiply => 0x37,
        Key::LeftAlt | Key::RightAlt => 0x38,
        Key::Space => 0x39,
        Key::F1 => 0x3B,
        Key::F2 => 0x3C,
        Key::F3 => 0x3D,
        Key::F4 => 0x3E,
        Key::F5 => 0x3F,
        Key::F6 => 0x40,
        Key::F7 => 0x41,
        Key::F8 => 0x42,
        Key::F9 => 0x43,
        Key::F10 => 0x44,
        Key::Kp7 => 0x47,
        Key::Up | Key::Kp8 => 0x48,
        Key::Kp9 => 0x49,
        Key::KpMinus => 0x4A,
        Key::Left | Key::Kp4 => 0x4B,
        Key::Kp5 => 0x4C,
        Key::Right | Key::Kp6 => 0x4D,
        Key::KpPlus => 0x4E,
        Key::Kp1 => 0x4F,
        Key::Down | Key::Kp2 => 0x50,
        Key::Kp3 => 0x51,
        Key::Kp0 => 0x52,
        Key::KpPeriod => 0x53,
        Key::F11 => 0x57,
        Key::F12 => 0x58,
    }
}

/// SDL 1.2 repeats every key but the modifiers.
fn repeats(key: Key) -> bool {
    !matches!(
        key,
        Key::LeftShift
            | Key::RightShift
            | Key::LeftCtrl
            | Key::RightCtrl
            | Key::LeftAlt
            | Key::RightAlt
    )
}

/// The key SDL repeats while it is held.
#[derive(Clone, Copy, Debug)]
struct Repeat {
    key: Key,
    /// Still waiting for the first delay.
    first: bool,
    /// Milliseconds since SDL's repeat timestamp.
    elapsed: i64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Keys {
    /// `[0x456BF8]`: the last key-down's scancode, 0 when read.
    remembered: u8,
    repeat: Option<Repeat>,
    /// Ticks polled so far: SDL's clock is `14 * ticks` milliseconds.
    ticks: i64,
    stick: [i32; 2],
    buttons: [bool; 4],
    /// `eventDetected`'s joystick timestamps: 0x456B28, 0x456B2C and the hold-off 0x456B1C.
    pad_pushed_ms: Option<i64>,
    pad_called_ms: Option<i64>,
    hold_off_until_ms: i64,
    /// `dr.cfg`'s gamepad switch (0x45EA00): the joystick path runs only when it is on.
    pad_on: bool,
    pad_connected: bool,
    /// 0x456B00: set while Define Gamepad waits, so `eventDetected` leaves the gamepad to it.
    calibrating: bool,
}

impl Keys {
    /// An input event from the frontend. Key-downs become the remembered key at once, as the
    /// next poll of the original's event loop would make them.
    pub(crate) fn event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, pressed: true } => {
                self.remembered = scancode(key);
                if repeats(key) {
                    self.repeat = Some(Repeat {
                        key,
                        first: true,
                        elapsed: 0,
                    });
                }
            }
            InputEvent::Key {
                key,
                pressed: false,
            } => {
                if self.repeat.is_some_and(|repeat| repeat.key == key) {
                    self.repeat = None;
                }
            }
            InputEvent::PadButton { button, pressed } => {
                self.buttons[PadButton::ALL
                    .iter()
                    .position(|&b| b == button)
                    .expect("every button is listed")] = pressed;
            }
            InputEvent::PadAxis { axis, value } => {
                self.stick[usize::from(axis == PadAxis::StickY)] = i32::from(value) / 256;
            }
            InputEvent::PadConnected { connected } => self.pad_connected = connected,
        }
    }

    pub(crate) fn set_pad_on(&mut self, on: bool) {
        self.pad_on = on;
    }

    pub(crate) fn set_calibrating(&mut self, calibrating: bool) {
        self.calibrating = calibrating;
    }

    pub(crate) fn pad_connected(&self) -> bool {
        self.pad_connected
    }

    /// The gamepad input Define Gamepad takes (0x42CBF0), later checks winning: the stick's
    /// left, right, up, down (1–4), then buttons 1–4 (5–8); 0 for none.
    pub(crate) fn pad_input(&self) -> u8 {
        let [x, y] = self.stick;
        let mut input = 0;
        for (pushed, code) in [
            (x < -STICK_THRESHOLD, 1),
            (x > STICK_THRESHOLD, 2),
            (y < -STICK_THRESHOLD, 3),
            (y > STICK_THRESHOLD, 4),
        ] {
            if pushed {
                input = code;
            }
        }
        for (button, code) in self.buttons.iter().zip(5..) {
            if *button {
                input = code;
            }
        }
        input
    }

    /// One poll of the event loop, once per tick: SDL 1.2 repeats the held key.
    pub(crate) fn tick(&mut self) {
        self.ticks += 1;
        if let Some(repeat) = &mut self.repeat {
            repeat.elapsed += TICK_MS;
            if repeat.first {
                if repeat.elapsed > REPEAT_DELAY_MS {
                    repeat.first = false;
                    repeat.elapsed = 0;
                }
            } else if repeat.elapsed > REPEAT_INTERVAL_MS {
                repeat.elapsed = 0;
                self.remembered = scancode(repeat.key);
            }
        }
    }

    /// `eventDetected`: the remembered key, cleared, unless the joystick says something.
    pub(crate) fn take(&mut self) -> u8 {
        let key = std::mem::take(&mut self.remembered);
        if !self.pad_on {
            return key;
        }
        let now = TICK_MS * self.ticks;
        let pushed = self.pad_pushed_ms.unwrap_or(now - UNSET_MS);
        let called = *self.pad_called_ms.get_or_insert(now - UNSET_MS);
        let pad = self.pad_code();
        if now >= self.hold_off_until_ms {
            if pad != 0 {
                if now - pushed >= FRESH_MS && now - called < FRESH_MS {
                    self.hold_off_until_ms = now + HOLD_OFF_MS;
                }
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
                pad
            } else {
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
                key
            }
        } else if pad != 0 {
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
            0
        } else {
            self.hold_off_until_ms = now;
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
            key
        }
    }

    /// The joystick's code, later checks winning as in the original: buttons over the stick,
    /// the vertical axis over the horizontal one.
    fn pad_code(&self) -> u8 {
        if self.calibrating {
            return 0;
        }
        let [x, y] = self.stick;
        let mut code = 0;
        if x < -STICK_THRESHOLD {
            code = PAD_LEFT;
        }
        if x > STICK_THRESHOLD {
            code = PAD_RIGHT;
        }
        if y < -STICK_THRESHOLD {
            code = PAD_UP;
        }
        if y > STICK_THRESHOLD {
            code = PAD_DOWN;
        }
        for (pressed, button_code) in self.buttons.iter().zip([ENTER, ESCAPE, ENTER, ESCAPE]) {
            if *pressed {
                code = button_code;
            }
        }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, pressed: bool) -> InputEvent {
        InputEvent::Key { key, pressed }
    }

    #[test]
    fn the_last_key_down_is_remembered_until_it_is_read() {
        // One byte in the original: a second press replaces the first, and reading clears it.
        let mut keys = Keys::default();
        keys.event(key(Key::Up, true));
        keys.event(key(Key::Up, false));
        keys.event(key(Key::Escape, true));
        keys.event(key(Key::Escape, false));
        assert_eq!(keys.take(), ESCAPE);
        assert_eq!(keys.take(), 0);
    }

    #[test]
    fn arrows_report_the_keypads_codes() {
        // windib drops the 0xE0 prefix, so the menus see 0x48 for both Up and keypad 8.
        assert_eq!(scancode(Key::Up), scancode(Key::Kp8));
        assert_eq!(scancode(Key::KpEnter), ENTER);
        assert_eq!(scancode(Key::Y), Y);
        assert_eq!(scancode(Key::N), N);
    }

    #[test]
    fn a_held_key_repeats_after_half_a_second_then_every_third_tick() {
        // SDL 1.2's repeat, polled once a tick: the delay passes at the 36th poll (504 ms),
        // the first repeat comes 3 polls later (42 ms > 30 ms), then every 3 polls.
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        assert_eq!(keys.take(), DOWN);
        let polls: Vec<u32> = (1..=48)
            .filter(|_| {
                keys.tick();
                keys.take() == DOWN
            })
            .collect();
        assert_eq!(polls, [39, 42, 45, 48]);
    }

    #[test]
    fn releasing_the_key_or_holding_a_modifier_does_not_repeat() {
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        keys.event(key(Key::Down, false));
        keys.take();
        keys.event(key(Key::LeftShift, true));
        assert_eq!(keys.take(), 0x2A, "a modifier is still a key press");
        for _ in 0..100 {
            keys.tick();
            assert_eq!(keys.take(), 0);
        }
    }

    #[test]
    fn a_fresh_push_of_the_stick_holds_its_repeat_off_for_700_ms() {
        // The menus read every 2 ticks: a held push moves once, waits 700 ms, then moves on
        // every read.
        let mut keys = Keys::default();
        keys.set_pad_on(true);
        let reads: Vec<u8> = (0..60)
            .map(|read| {
                if read == 1 {
                    keys.event(InputEvent::PadAxis {
                        axis: PadAxis::StickY,
                        value: 20_000,
                    });
                }
                keys.tick();
                keys.tick();
                keys.take()
            })
            .collect();
        assert_eq!(reads[1], PAD_DOWN);
        // 700 ms is 25 reads of 28 ms; the hold-off ends at read 1 + 25.
        assert!(reads[2..26].iter().all(|&code| code == 0), "{reads:?}");
        assert!(
            reads[26..].iter().all(|&code| code == PAD_DOWN),
            "{reads:?}"
        );
    }

    #[test]
    fn pad_buttons_confirm_and_go_back() {
        // Buttons 0 and 2 answer like Enter, 1 and 3 like Escape.
        let mut keys = Keys::default();
        keys.set_pad_on(true);
        let push = |keys: &mut Keys, button, pressed| {
            keys.event(InputEvent::PadButton { button, pressed });
        };
        push(&mut keys, PadButton::A, true);
        assert_eq!(keys.take(), ENTER);
        push(&mut keys, PadButton::A, false);
        keys.tick();
        assert_eq!(keys.take(), 0);
        push(&mut keys, PadButton::B, true);
        keys.tick();
        assert_eq!(keys.take(), ESCAPE);
    }

    #[test]
    fn the_gamepad_counts_only_when_dr_cfg_switches_it_on() {
        // A fresh dr.cfg has it off, as the original's: a pad does nothing until the player
        // switches it on in Configure.
        let mut keys = Keys::default();
        keys.event(InputEvent::PadButton {
            button: PadButton::A,
            pressed: true,
        });
        assert_eq!(keys.take(), 0);
        keys.set_pad_on(true);
        assert_eq!(keys.take(), ENTER);
    }

    #[test]
    fn define_gamepad_takes_buttons_over_the_stick_and_the_last_button() {
        // 0x42CBF0 checks the stick, then buttons 1 to 4, each later one winning.
        let mut keys = Keys::default();
        assert_eq!(keys.pad_input(), 0);
        keys.event(InputEvent::PadAxis {
            axis: PadAxis::StickX,
            value: -20_000,
        });
        assert_eq!(keys.pad_input(), 1, "left");
        keys.event(InputEvent::PadAxis {
            axis: PadAxis::StickY,
            value: 20_000,
        });
        assert_eq!(keys.pad_input(), 4, "down over left");
        for (button, input) in [(PadButton::B, 6), (PadButton::Y, 8)] {
            keys.event(InputEvent::PadButton {
                button,
                pressed: true,
            });
            assert_eq!(keys.pad_input(), input);
        }
    }
}
```

<!-- write: crates/core/src/font.rs -->
```rust
//! The original's bitmap fonts (`drawTextWithFont`, 0x41A2D0; spec M2a §3.1): glyph `c - 32` of
//! a sheet of equal cells, drawn with colour 0 transparent, the pen moving by the glyph's
//! advance from `dr.exe`'s metrics.

use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::{GAP, Metrics};

use crate::canvas::Canvas;

#[derive(Clone, Debug)]
pub(crate) struct Font {
    glyphs: Vec<Image>,
    advances: Vec<u8>,
}

impl Font {
    pub(crate) fn new(glyphs: Vec<Image>, metrics: &Metrics) -> Font {
        Font {
            glyphs,
            advances: metrics.advances.clone(),
        }
    }

    /// The glyph and advance of byte `c`, if the font has one (characters 32 onwards).
    fn glyph(&self, c: u8) -> Option<(&Image, usize)> {
        let index = usize::from(c.checked_sub(32)?);
        Some((
            self.glyphs.get(index)?,
            usize::from(*self.advances.get(index)?),
        ))
    }

    /// Draws `text` with the pen starting at `offset`; returns where the pen ends.
    pub(crate) fn draw(&self, canvas: &mut Canvas, text: &[u8], offset: usize) -> usize {
        let mut pen = offset;
        for &c in text {
            if c == GAP {
                pen += 1;
            } else if let Some((glyph, advance)) = self.glyph(c) {
                canvas.draw(glyph, pen, true);
                pen += advance;
            }
        }
        pen
    }

    /// How far `text` moves the pen (`getBoxBigTextOffset`, 0x41C9F0, for the big font): the
    /// sum of its advances, which right-aligns text.
    pub(crate) fn width(&self, text: &[u8]) -> usize {
        text.iter()
            .filter_map(|&c| self.glyph(c).map(|(_, advance)| advance))
            .sum()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::at;

    /// A 2x1 font: glyph `k` is filled with colour `k + 1`, its left pixel transparent for
    /// odd `k`; every advance is 3.
    pub(crate) fn font() -> Font {
        let glyphs = (0..96u8)
            .map(|k| {
                let left = if k % 2 == 1 { 0 } else { k + 1 };
                Image::new(2, 1, vec![left, k + 1])
            })
            .collect();
        Font::new(
            glyphs,
            &Metrics {
                width: 2,
                height: 1,
                advances: vec![3; 96],
            },
        )
    }

    #[test]
    fn each_byte_draws_its_glyph_and_moves_the_pen_by_its_advance() {
        // A wrong glyph offset prints the neighbouring letter; a wrong advance spaces the text.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, b" !\"", at(0, 0));
        assert_eq!(end, 9);
        assert_eq!(&canvas.pixels()[..8], [1, 1, 0, 0, 2, 0, 3, 3]);
    }

    #[test]
    fn the_gap_byte_moves_the_pen_one_pixel_and_draws_nothing() {
        // The menu table pads rows with 0xFA to place text a pixel at a time.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, &[GAP, b' '], at(0, 0));
        assert_eq!(end, 4);
        assert_eq!(&canvas.pixels()[..3], [0, 1, 1]);
    }

    #[test]
    fn a_texts_width_is_the_sum_of_its_advances() {
        // Configure's percentages are right-aligned by it; another width moves them.
        assert_eq!(font().width(b" !\""), 9);
        assert_eq!(font().width(&[7, b' ']), 3, "no glyph, no advance");
    }

    #[test]
    fn bytes_without_a_glyph_draw_nothing_and_do_not_move_the_pen() {
        let mut canvas = Canvas::default();
        assert_eq!(font().draw(&mut canvas, &[7, 200], at(5, 1)), at(5, 1));
        assert!(canvas.pixels().iter().all(|&p| p == 0));
    }
}
```

<!-- write: crates/core/src/audio/mod.rs -->
```rust
//! Sound: the music and effect players and the mixer they share (spec M1b §4.2).

pub(crate) mod effects;
pub(crate) mod mixer;
pub(crate) mod music;
pub(crate) mod tables;

use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::xm::Bank;

use self::effects::Effects;
use self::mixer::{UNITY, Voice, clip};
use self::music::Music;
use crate::AUDIO_CHANNELS;

/// FMOD's master volume for music, 0..=256, at a music volume of the game's configuration
/// (0..=0x10000): `mask * (volume >> 8) >> 9`, as `musicSetmusicVolume` (0x43C280) sets it,
/// with the game's volume mask (0x456A34) at 255 unless the end screen lowers it.
fn music_master(volume: u32, mask: u32) -> i64 {
    (i64::from(mask) * i64::from(volume >> 8)) >> 9
}

/// The volume mask's normal value.
const FULL_MASK: u32 = 255;

/// The music volume the intro plays at, whatever `dr.cfg` says: the game's volume globals
/// start at 255 and take `dr.cfg`'s values only when the menu music starts.
pub(crate) const FULL_VOLUME: u32 = 0xFF00;

/// `dr.cfg`'s default music volume, 50 % (`defaultConfig`, 0x426700). The menus play at it
/// until M2's Configure menu can change it.
pub(crate) const DEFAULT_MUSIC_VOLUME: u32 = 0x8000;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug)]
pub(crate) struct Sound {
    music: Option<Music>,
    /// Voices of music that newer music replaced, fading out.
    fading: Vec<Voice>,
    effects: Option<Effects>,
    /// Voices of a bank that a newer bank replaced, fading out.
    fading_effects: Vec<Voice>,
    mix: Vec<i64>,
    /// The original's volume globals: the mask (0x456A34), the music's (0x456A30) and the
    /// effects' (0x456A2C) volumes as `dr.cfg` gives them; all full until the intro ends.
    mask: u32,
    music_volume: u32,
    effects_volume: u32,
}

impl Default for Sound {
    fn default() -> Sound {
        Sound {
            music: None,
            fading: Vec::new(),
            effects: None,
            fading_effects: Vec::new(),
            mix: Vec::new(),
            mask: FULL_MASK,
            music_volume: FULL_VOLUME,
            effects_volume: FULL_VOLUME,
        }
    }
}

impl Sound {
    /// Starts `module` at order `first_order` (counted as the game counts them) and the
    /// configured music `volume` (0..=0x10000), fading out any music playing.
    pub(crate) fn play_music(&mut self, module: &Module, first_order: usize, volume: u32) {
        if let Some(old) = self.music.take() {
            self.fading.extend(old.into_fading());
        }
        self.music_volume = volume;
        self.music = Some(Music::new(module, self.music_gain(), first_order));
    }

    fn music_gain(&self) -> i64 {
        UNITY * music_master(self.music_volume, self.mask) / 256
    }

    /// The effects stream's share: `mask * (volume >> 8) >> 8` of 255 (`musicSetVolume`,
    /// 0x43C250), 254 of 255 while the intro plays.
    fn effects_gain(&self) -> i64 {
        UNITY * ((i64::from(self.mask) * i64::from(self.effects_volume >> 8)) >> 8) / 255
    }

    /// The configured effects volume (0..=0x10000) for the effects stream.
    pub(crate) fn set_effects_volume(&mut self, volume: u32) {
        self.effects_volume = volume;
    }

    /// The volume mask (`setMusicVolume`, 0x43C2B0): 0..=255 over music and effects alike.
    pub(crate) fn set_mask(&mut self, mask: u32) {
        self.mask = mask;
        self.apply_music_gain();
    }

    /// The configured music volume (0..=0x10000) for the music playing, as Configure's popup
    /// sets it (`musicSetmusicVolume`, 0x43C280).
    pub(crate) fn set_music_volume(&mut self, volume: u32) {
        self.music_volume = volume;
        self.apply_music_gain();
    }

    fn apply_music_gain(&mut self) {
        let gain = self.music_gain();
        if let Some(music) = &mut self.music {
            music.set_gain(gain);
        }
    }

    /// Makes `bank` the source of [`Sound::trigger`]; the old bank's effects fade out.
    pub(crate) fn load_effects(&mut self, bank: &Bank) {
        if let Some(old) = self.effects.take() {
            self.fading_effects.extend(old.into_fading());
        }
        self.effects = Some(Effects::new(bank));
    }

    /// Plays effect `effect` of the loaded bank on `channel` (both 1-based) at full volume and
    /// normal pitch, as the intro does.
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8) {
        self.trigger_at(channel, effect, effects::FULL, effects::FULL);
    }

    /// Plays effect `effect` on `channel` at `volume` and `pitch` (16.16, 0x10000 full and
    /// normal), as `loadMenuSoundEffect` (0x43C380) does.
    pub(crate) fn trigger_at(&mut self, channel: usize, effect: u8, volume: u32, pitch: u32) {
        if let Some(effects) = &mut self.effects {
            effects.trigger(channel, effect, volume, pitch);
        }
    }

    /// Stops the music and every effect, with a short fade so nothing clicks.
    pub(crate) fn stop(&mut self) {
        if let Some(music) = &mut self.music {
            music.stop();
        }
        if let Some(effects) = &mut self.effects {
            effects.stop_all();
        }
    }

    /// Appends `frames` interleaved stereo frames to `out`.
    pub(crate) fn render(&mut self, frames: usize, out: &mut Vec<i16>) {
        self.mix.clear();
        self.mix.resize(frames * AUDIO_CHANNELS, 0);
        if let Some(music) = &mut self.music {
            music.mix_into(&mut self.mix);
        }
        for voice in &mut self.fading {
            voice.mix_into(&mut self.mix);
        }
        self.fading.retain(|voice| !voice.finished());
        let mut part = vec![0; self.mix.len()];
        if let Some(effects) = &mut self.effects {
            effects.mix_into(&mut part);
        }
        for voice in &mut self.fading_effects {
            voice.mix_into(&mut part);
        }
        self.fading_effects.retain(|voice| !voice.finished());
        let gain = self.effects_gain();
        for (sum, effect) in self.mix.iter_mut().zip(part) {
            *sum += (effect * gain) >> 16;
        }
        out.extend(self.mix.iter().map(|&value| clip(value)));
    }
}

/// Renders `frames` stereo frames of `module` from its first order at the default music
/// volume, mixed as the game plays music in its menus and races.
#[must_use]
pub fn render_music(module: &Module, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.play_music(module, 0, DEFAULT_MUSIC_VOLUME);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

/// Renders effect `effect` (1-based) of `bank` at full volume and normal pitch, as the game
/// triggers it, for `frames` stereo frames.
#[must_use]
pub fn render_effect(bank: &Bank, effect: u8, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.load_effects(bank);
    sound.trigger(1, effect);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{self, Cell, Channel, Pattern, Sample};
    use deadrally_gamedata::xm::{Instrument, Looping};

    fn bank() -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![Some(Instrument {
                name: "Loud".into(),
                data: vec![i16::MAX; 20_000],
                looping: Looping::None,
                volume: 64,
                finetune: 0,
                relative_note: 0,
                panning: 255,
                fadeout: 0,
            })],
        }
    }

    #[test]
    fn nothing_loaded_is_silence_of_the_right_length() {
        // The frontend paces itself on the audio queue; every tick must deliver its samples.
        let mut out = Vec::new();
        Sound::default().render(672, &mut out);
        assert_eq!(out.len(), 672 * AUDIO_CHANNELS);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn overlapping_effects_clip_instead_of_wrapping() {
        // Four full-scale effects add up to twice the 16-bit range; wrapping would turn the
        // overload into noise, clipping only flattens it.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        for channel in 1..=4 {
            sound.trigger(channel, 1);
        }
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 900], i16::MAX, "left, panned hard left");
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn new_music_fades_the_old_out_instead_of_cutting_it() {
        // The intro's music gives way to the menu music; a cut at full level would click.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut silent = loud.clone();
        silent.orders.clear();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        assert!(before > 0);
        sound.play_music(&silent, 0, FULL_VOLUME);
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_configured_music_volume_sets_fmods_master_volume() {
        // musicSetmusicVolume (0x43C280): 255 * (volume >> 8) >> 9.
        assert_eq!(music_master(FULL_VOLUME, FULL_MASK), 127);
        assert_eq!(music_master(DEFAULT_MUSIC_VOLUME, FULL_MASK), 63);
        assert_eq!(music_master(0x1_0000, FULL_MASK), 127);
        assert_eq!(music_master(0, FULL_MASK), 0);
    }

    #[test]
    fn a_new_bank_lets_the_old_banks_effects_fade_out() {
        // The intro's effects stop as the menu's bank is loaded; cutting them at full level
        // would click.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        sound.stop();
        sound.load_effects(&bank());
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_effects_volume_and_the_mask_scale_the_effects_stream() {
        // The menus play effects at dr.cfg's 75 %: the stream at 255 * 192 >> 8 = 191 of 255
        // instead of the intro's 254. The end screen's mask lowers everything.
        let level = |sound: &mut Sound| {
            sound.load_effects(&bank());
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(1000, &mut out);
            i64::from(out[2 * 999])
        };
        let full = level(&mut Sound::default());
        let mut menu = Sound::default();
        menu.set_effects_volume(0xC000);
        let at_75 = level(&mut menu);
        assert!(
            (at_75 * 254 - full * 191).abs() <= 254 * 2,
            "{at_75} vs {full}"
        );
        let mut quiet = Sound::default();
        quiet.set_mask(0);
        assert_eq!(level(&mut quiet), 0);
    }

    /// One endless loud note on the first channel.
    fn loud() -> Module {
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        loud
    }

    #[test]
    fn the_mask_scales_the_music_while_it_plays() {
        // The end screen fades the music out through the mask, 255 down to 0.
        let loud = loud();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        sound.set_mask(0x80);
        out.clear();
        sound.render(2000, &mut out);
        // 255 * 255 >> 9 = 127 against 128 * 255 >> 9 = 63.
        assert!(
            (i64::from(out[2 * 1999]) * 127 - full * 63).abs() <= 127 * 2,
            "{} vs {full}",
            out[2 * 1999]
        );
    }

    #[test]
    fn the_music_volume_changes_the_music_while_it_plays() {
        // Configure's popup applies each step at once (`musicSetmusicVolume`): 50 % plays at
        // master volume 63, 100 % at 127.
        let mut sound = Sound::default();
        sound.play_music(&loud(), 0, DEFAULT_MUSIC_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let half = i64::from(out[2 * 1999]);
        sound.set_music_volume(0x1_0000);
        out.clear();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        assert!(
            (full * 63 - half * 127).abs() <= 127 * 2,
            "{half} vs {full}"
        );
    }

    #[test]
    fn stopping_fades_everything_out() {
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        sound.stop();
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 999], 0);
    }
}
```

<!-- write: crates/core/src/test_scene.rs -->
```rust
//! Throwaway test scene for M0. It exercises the frame, palette, input and audio contract so
//! the platform spike has something to show and play. Removed when M2 brings the real menus.

use crate::{AUDIO_FRAMES_PER_TICK, Frame, InputEvent, Key, PadButton};

// Palette entries 0..=3 are pinned so the bar, the grid and the stick dot do not cycle with
// the ramp.
const WHITE: u8 = 1;
const GREY: u8 = 2;
const RED: u8 = 3;
const PINNED: [[u8; 3]; 4] = [[0, 0, 0], [63, 63, 63], [16, 16, 16], [63, 0, 0]];

const BAR_WIDTH: u32 = 4;
const GRID_COLUMNS: u32 = 16;

/// 440 Hz in 32-bit phase units per sample: 440 * 2^32 / 48 000, rounded.
const TONE_STEP: u32 = 39_370_534;
const TONE_AMPLITUDE: i32 = 2_048;
/// The click is 5 ms (240 frames at 48 kHz) of square wave at 1 kHz.
const CLICK_FRAMES: u32 = 240;
const CLICK_HALF_PERIOD: u32 = 24;
const CLICK_AMPLITUDE: i32 = 8_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Vga640x480,
    Vga320x200,
    Wide640x360,
}

impl Mode {
    fn size(self) -> (u32, u32) {
        match self {
            Mode::Vga640x480 => (640, 480),
            Mode::Vga320x200 => (320, 200),
            Mode::Wide640x360 => (640, 360),
        }
    }

    fn aspect(self) -> (u32, u32) {
        match self {
            Mode::Vga640x480 | Mode::Vga320x200 => (4, 3),
            Mode::Wide640x360 => (16, 9),
        }
    }

    fn next(self) -> Mode {
        match self {
            Mode::Vga640x480 => Mode::Vga320x200,
            Mode::Vga320x200 => Mode::Wide640x360,
            Mode::Wide640x360 => Mode::Vga640x480,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TestScene {
    tick: u64,
    mode: Mode,
    palette: [[u8; 3]; 256],
    pixels: Vec<u8>,
    held_keys: Vec<bool>,
    held_buttons: [bool; 4],
    stick: [i16; 2],
    tone_on: bool,
    tone_phase: u32,
    click_frames_left: u32,
    audio: Vec<i16>,
}

impl TestScene {
    pub(crate) fn new() -> TestScene {
        let mut scene = TestScene {
            tick: 0,
            mode: Mode::Vga640x480,
            palette: [[0; 3]; 256],
            pixels: Vec::new(),
            held_keys: vec![false; Key::ALL.len()],
            held_buttons: [false; 4],
            stick: [0; 2],
            tone_on: true,
            tone_phase: 0,
            click_frames_left: 0,
            audio: Vec::new(),
        };
        scene.set_mode(Mode::Vga640x480);
        scene.update_palette();
        scene
    }

    pub(crate) fn input(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, pressed } => {
                let was_held = std::mem::replace(&mut self.held_keys[key as usize], pressed);
                if pressed && !was_held {
                    self.click_frames_left = CLICK_FRAMES;
                    match key {
                        Key::Tab => self.set_mode(self.mode.next()),
                        Key::T => self.tone_on = !self.tone_on,
                        _ => {}
                    }
                }
            }
            InputEvent::PadButton { button, pressed } => {
                let was_held = std::mem::replace(&mut self.held_buttons[button as usize], pressed);
                if pressed && !was_held {
                    self.click_frames_left = CLICK_FRAMES;
                }
            }
            InputEvent::PadAxis { axis, value } => {
                self.stick[axis as usize] = value;
            }
            InputEvent::PadConnected { .. } => {}
        }
    }

    pub(crate) fn tick(&mut self) {
        self.tick += 1;
        self.update_palette();
        self.render();
        self.mix_audio();
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        let (width, height) = self.mode.size();
        Frame {
            width,
            height,
            pixels: &self.pixels,
            palette: &self.palette,
            aspect: self.mode.aspect(),
        }
    }

    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }

    /// Switches mode and redraws at once, so `frame()` never pairs the new size with old
    /// pixels.
    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        let (width, height) = mode.size();
        self.pixels = vec![0; (width * height) as usize];
        self.render();
    }

    /// Rotates the ramp by one entry per tick, then re-pins the fixed colours.
    fn update_palette(&mut self) {
        let shift = (self.tick % 256) as usize;
        for (index, entry) in self.palette.iter_mut().enumerate() {
            *entry = ramp_color(((index + shift) % 256) as u8);
        }
        self.palette[..PINNED.len()].copy_from_slice(&PINNED);
    }

    fn render(&mut self) {
        let (width, height) = self.mode.size();
        for (y, row) in self.pixels.chunks_exact_mut(width as usize).enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = ((x + y) & 0xFF) as u8;
            }
        }

        let bar_x = (self.tick % u64::from(width)) as u32;
        self.fill_rect(bar_x, 0, BAR_WIDTH, height, WHITE);

        // A frame pixel is shown aspect.0 / width wide and aspect.1 / height tall, so this many
        // pixels across look as long on screen as `tall` pixels down. Squares stay squares in
        // every mode; a wider mode shows more, it never stretches.
        let (aspect_width, aspect_height) = self.mode.aspect();
        let across = |tall: u32| tall * width * aspect_height / (height * aspect_width);

        let cell_height = height / 16;
        let cell_width = across(cell_height);
        let key_count = Key::ALL.len();
        for cell in 0..key_count + PadButton::ALL.len() {
            let held = if cell < key_count {
                self.held_keys[cell]
            } else {
                self.held_buttons[cell - key_count]
            };
            let column = cell as u32 % GRID_COLUMNS;
            let row = cell as u32 / GRID_COLUMNS;
            self.fill_rect(
                cell_width * (2 + column),
                cell_height * (2 + row),
                cell_width - 1,
                cell_height - 1,
                if held { WHITE } else { GREY },
            );
        }

        let radius_y = height / 8;
        let radius_x = across(radius_y);
        let dot_height = (cell_height / 2).max(2);
        let dot_width = across(dot_height).max(2);
        let offset = |value: i16, radius: u32, dot: u32, centre: u32| {
            let radius = i32::try_from(radius).expect("radius fits i32");
            let corner = i32::try_from(centre - dot / 2).expect("centre fits i32");
            u32::try_from(corner + i32::from(value) * radius / 32_768).expect("dot stays on screen")
        };
        let dot_x = offset(self.stick[0], radius_x, dot_width, width / 2);
        let dot_y = offset(self.stick[1], radius_y, dot_height, height * 3 / 4);
        self.fill_rect(dot_x, dot_y, dot_width, dot_height, RED);
    }

    /// Fills a rectangle, clipped to the frame.
    fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: u8) {
        let (frame_width, frame_height) = self.mode.size();
        if x >= frame_width || y >= frame_height {
            return;
        }
        let x_end = (x + width).min(frame_width);
        let y_end = (y + height).min(frame_height);
        for row in y..y_end {
            let start = (row * frame_width + x) as usize;
            let end = (row * frame_width + x_end) as usize;
            self.pixels[start..end].fill(color);
        }
    }

    fn mix_audio(&mut self) {
        for _ in 0..AUDIO_FRAMES_PER_TICK {
            let mut sample = 0;
            if self.tone_on {
                sample += triangle(self.tone_phase) * TONE_AMPLITUDE / 32_768;
            }
            self.tone_phase = self.tone_phase.wrapping_add(TONE_STEP);
            if self.click_frames_left > 0 {
                let elapsed = CLICK_FRAMES - self.click_frames_left;
                sample += if (elapsed / CLICK_HALF_PERIOD).is_multiple_of(2) {
                    CLICK_AMPLITUDE
                } else {
                    -CLICK_AMPLITUDE
                };
                self.click_frames_left -= 1;
            }
            let sample = sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            self.audio.extend_from_slice(&[sample, sample]);
        }
    }
}

/// A full-scale triangle wave (-32768..=32767) from a 32-bit phase.
fn triangle(phase: u32) -> i32 {
    let p = (phase >> 16) as i32;
    if p < 32_768 {
        p * 2 - 32_768
    } else {
        (65_535 - p) * 2 - 32_767
    }
}

fn ramp_color(index: u8) -> [u8; 3] {
    [index / 4, index % 64, 63 - index / 4]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_spans_full_scale_without_overflow() {
        // The mixer multiplies this by the amplitude in i32; out-of-range values would clip.
        assert_eq!(triangle(0), -32_768);
        assert_eq!(triangle(0x7FFF_0000), 32_766);
        assert_eq!(triangle(0x8000_0000), 32_767);
        assert_eq!(triangle(u32::MAX), -32_767);
    }

    #[test]
    fn ramp_colours_stay_within_six_bits() {
        // VGA palettes are 6-bit; a value above 63 would be silently masked by the DAC.
        for index in 0..=255u8 {
            assert!(ramp_color(index).iter().all(|&c| c <= 63), "index {index}");
        }
    }
}
```

<!-- write: crates/core/src/startup.rs -->
```rust
//! The original's startup sequence: the intro, the Apogee and Remedy logos and the title screen
//! (spec M1a §3.6 and §5.2), with the intro's music and effects and then the menu music
//! (spec M1b §4.3).
//!
//! Each step follows the Windows version's loops tick for tick, so a screenshot of the original
//! can be found in our timeline: `openAnimation` (0x4185B0), `apogeeScreen` (0x427380) and
//! `showStartScreen` (0x427880).

use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::dr_cfg::DrCfg;
use deadrally_gamedata::haf::FRAME_PIXELS;
use deadrally_gamedata::image::Palette;

use crate::audio::{FULL_VOLUME, Sound};
use crate::fade::{FADE_FULL, FADE_STEP, fade};
use crate::keys::Keys;
use crate::menu::Menu;
use crate::{AUDIO_FRAMES_PER_TICK, Frame, InputEvent};

/// The intro's screen: 320x200, with the animation's 320x120 frames from row 40.
const INTRO_WIDTH: u32 = 320;
const INTRO_HEIGHT: u32 = 200;
const INTRO_FIRST_ROW: usize = 40;
/// The letterbox owns palette entries 0..=15, the animation frames the rest.
const LETTERBOX_COLOURS: usize = 16;
/// The intro's effects take channels 1..=6 in turn.
const INTRO_EFFECT_CHANNELS: usize = 6;
/// The menu music starts at this order (`musicSetOrder(0x2D00)` in `mainMenu`, 0x43A0C5).
const MENU_MUSIC_ORDER: usize = 45;

/// Fade-in ticks: brightness 0, 4, ..., 96 %. The original's loop stops before 100 %.
const FADE_IN_TICKS: u32 = 25;
/// Fade-out ticks: brightness 100, 96, ..., 0 %.
const FADE_OUT_TICKS: u32 = 26;
/// Longest hold of a logo, in ticks.
const HOLD_TICKS: u32 = 180;

/// A logo or the title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Apogee,
    Remedy,
    Title,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// `next` is the frame being waited for; `waited` counts ticks since the previous frame.
    Intro {
        next: usize,
        waited: u32,
    },
    FadeIn {
        screen: Screen,
        ticks: u32,
    },
    Hold {
        screen: Screen,
        ticks: u32,
    },
    FadeOut {
        screen: Screen,
        ticks: u32,
    },
    /// The title has faded in; the main menu takes over (`mainMenu` goes on to load it).
    Done,
}

#[derive(Debug)]
pub(crate) struct Startup {
    assets: Assets,
    stage: Stage,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    palette: Palette,
    /// The original keeps the last key press until something asks for it (`eventDetected`,
    /// 0x417EB0, reads and clears it), so a press during a fade-in ends the following hold
    /// after one tick.
    keys: Keys,
    sound: Sound,
    /// The channel the intro's next effect plays on.
    effect_channel: usize,
    /// Samples rendered since the last `take_audio`.
    audio: Vec<i16>,
    /// The player's `dr.cfg`, and whether the original would write it now.
    config: DrCfg,
    save: bool,
}

impl Startup {
    /// `mainMenu` (0x43A020) reads `dr.cfg`, counts the start, writes it back and only then
    /// plays the intro.
    pub(crate) fn new(assets: Assets, mut config: DrCfg) -> Startup {
        config.set_times_played(config.times_played().wrapping_add(1));
        let mut keys = Keys::default();
        keys.set_pad_on(config.use_joystick() as i32 > 0);
        let mut startup = Startup {
            assets,
            stage: Stage::Intro { next: 0, waited: 0 },
            width: 0,
            height: 0,
            pixels: Vec::new(),
            palette: Palette::BLACK,
            keys,
            sound: Sound::default(),
            effect_channel: 1,
            audio: Vec::new(),
            config,
            save: true,
        };
        if startup.assets.intro.is_empty() {
            // `openAnimation` plays nothing when the file has no frames.
            startup.stage = startup.end_intro();
        } else {
            startup.show_letterbox();
            // `openAnimation` loads the music and the effects and starts the music just before
            // the first frame, at full volume: `dr.cfg`'s volumes apply only after the intro.
            startup
                .sound
                .play_music(&startup.assets.intro_music, 0, FULL_VOLUME);
            startup.sound.load_effects(&startup.assets.intro_effects);
        }
        startup
    }

    pub(crate) fn input(&mut self, event: InputEvent) {
        self.keys.event(event);
    }

    /// The title has faded in and the main menu should take over.
    pub(crate) fn finished(&self) -> bool {
        self.stage == Stage::Done
    }

    /// The main menu, taking over the data, the sound, the remembered key and `dr.cfg`.
    pub(crate) fn into_menu(self) -> Menu {
        Menu::new(
            self.assets,
            self.sound,
            self.keys,
            self.audio,
            &self.palette,
            (self.config, self.save),
        )
    }

    /// `dr.cfg`'s bytes when the original writes the file, once.
    pub(crate) fn take_config(&mut self) -> Option<Vec<u8>> {
        std::mem::take(&mut self.save).then(|| self.config.to_bytes())
    }

    pub(crate) fn tick(&mut self) {
        self.keys.tick();
        self.stage = self.next_stage();
        self.sound.render(AUDIO_FRAMES_PER_TICK, &mut self.audio);
    }

    fn next_stage(&mut self) -> Stage {
        match self.stage {
            Stage::Intro { next, waited } => self.tick_intro(next, waited + 1),
            // After the title's last step the original loads the main menu without presenting
            // a frame; the menu's first wait shows this step (spec M2a decision 5: loading takes
            // no time here).
            Stage::FadeIn {
                screen: Screen::Title,
                ticks,
            } if ticks + 1 == FADE_IN_TICKS => {
                self.palette = fade(&self.assets.title.palette, i64::from(ticks) * FADE_STEP);
                Stage::Done
            }
            Stage::FadeIn { screen, ticks } => {
                let level = i64::from(ticks) * FADE_STEP;
                self.palette = fade(&picture(&self.assets, screen).palette, level);
                if ticks + 1 < FADE_IN_TICKS {
                    Stage::FadeIn {
                        screen,
                        ticks: ticks + 1,
                    }
                } else {
                    Stage::Hold { screen, ticks: 0 }
                }
            }
            Stage::Hold { screen, ticks } => {
                // `do { wait } while (!eventDetected() && ticks < 180)`: the key is read first,
                // so even the last hold tick consumes a pending press.
                if self.keys.take() != 0 || ticks + 1 >= HOLD_TICKS {
                    Stage::FadeOut { screen, ticks: 0 }
                } else {
                    Stage::Hold {
                        screen,
                        ticks: ticks + 1,
                    }
                }
            }
            Stage::FadeOut { screen, ticks } => {
                let level = FADE_FULL - i64::from(ticks) * FADE_STEP;
                self.palette = fade(&picture(&self.assets, screen).palette, level);
                if ticks + 1 < FADE_OUT_TICKS {
                    Stage::FadeOut {
                        screen,
                        ticks: ticks + 1,
                    }
                } else {
                    self.show(match screen {
                        Screen::Apogee => Screen::Remedy,
                        Screen::Remedy | Screen::Title => Screen::Title,
                    })
                }
            }
            Stage::Done => Stage::Done,
        }
    }

    /// One tick of `openAnimation`: when frame `next` is due it replaces the previous one, then
    /// the original checks for a key before showing it. So a key press ends the intro at the
    /// next frame, which is never shown, and the last frame is never shown either.
    ///
    /// The original also checks once before frame 0. A press made while the game loads is read
    /// only when a frame is next shown (`refreshScreen`, 0x43B580), that is during frame 0's
    /// wait, so it ends the intro when frame 0 is due, as here.
    fn tick_intro(&mut self, mut next: usize, mut waited: u32) -> Stage {
        let intro = &self.assets.intro;
        let mut due = None;
        while waited >= u32::from(intro.delays[next]) {
            due = Some(next);
            next += 1;
            waited = 0;
            if next == intro.len() || self.keys.take() != 0 {
                // The frame ending the intro is never shown, and its effect, which the
                // original starts and cuts at once, never sounds.
                return self.end_intro();
            }
            // The original triggers a frame's effect right after drawing it.
            let effect = intro.effects[next - 1];
            if effect != 0 {
                self.sound.trigger(self.effect_channel, effect);
                self.effect_channel = self.effect_channel % INTRO_EFFECT_CHANNELS + 1;
            }
        }
        if let Some(index) = due {
            match intro.frame(index) {
                Ok(frame) => {
                    self.palette.0[LETTERBOX_COLOURS..]
                        .copy_from_slice(&frame.palette.0[LETTERBOX_COLOURS..]);
                    let start = INTRO_FIRST_ROW * INTRO_WIDTH as usize;
                    self.pixels[start..start + FRAME_PIXELS].copy_from_slice(&frame.pixels);
                }
                // Only data of an unknown version can get here (the known version's frames are
                // all tested), and the player was warned about it at start-up.
                Err(_) => return self.end_intro(),
            }
        }
        Stage::Intro { next, waited }
    }

    /// The intro's sound stops (`openAnimation`, `checkAndOpenAnimation`); `mainMenu` then
    /// starts the menu music at the configured volume and shows the logos.
    fn end_intro(&mut self) -> Stage {
        self.sound.stop();
        self.sound.play_music(
            &self.assets.menu_music,
            MENU_MUSIC_ORDER,
            self.config.music_volume(),
        );
        self.sound.load_effects(&self.assets.menu.effects);
        self.sound.set_effects_volume(self.config.effects_volume());
        self.show(Screen::Apogee)
    }

    /// Black screen with the letterbox's colours set, as `openAnimation` starts.
    fn show_letterbox(&mut self) {
        let letterbox = &self.assets.letterbox;
        assert_eq!(
            (letterbox.image.width, letterbox.image.height),
            (INTRO_WIDTH, INTRO_HEIGHT),
            "the intro letterbox is 320x200"
        );
        self.pixels.clear();
        self.pixels.extend_from_slice(&letterbox.image.pixels);
        (self.width, self.height) = (INTRO_WIDTH, INTRO_HEIGHT);
        self.palette = Palette::BLACK;
        self.palette.0[..LETTERBOX_COLOURS]
            .copy_from_slice(&letterbox.palette.0[..LETTERBOX_COLOURS]);
    }

    /// The picture drawn under a black palette, ready to fade in.
    #[must_use]
    fn show(&mut self, screen: Screen) -> Stage {
        let image = &picture(&self.assets, screen).image;
        self.pixels.clear();
        self.pixels.extend_from_slice(&image.pixels);
        (self.width, self.height) = (image.width, image.height);
        self.palette = Palette::BLACK;
        Stage::FadeIn { screen, ticks: 0 }
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: self.width,
            height: self.height,
            pixels: &self.pixels,
            palette: &self.palette.0,
            aspect: (4, 3),
        }
    }

    /// The startup's sound: the intro's music and effects, then the menu music.
    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }
}

fn picture(assets: &Assets, screen: Screen) -> &Picture {
    match screen {
        Screen::Apogee => &assets.apogee,
        Screen::Remedy => &assets.remedy,
        Screen::Title => &assets.title,
    }
}
```

<!-- write: crates/core/src/game.rs -->
```rust
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::dr_cfg::DrCfg;

use crate::menu::Menu;
use crate::startup::Startup;
use crate::test_scene::TestScene;
use crate::{Frame, InputEvent};

/// The whole game state: the original's startup sequence and then its main menu, or the M0
/// test scene.
#[derive(Debug)]
pub struct Game {
    scene: Scene,
}

#[derive(Debug)]
enum Scene {
    Test(Box<TestScene>),
    Startup(Box<Startup>),
    Menu(Box<Menu>),
    /// Only while one scene hands over to the next.
    Handover,
}

impl Game {
    /// Starts the original's startup sequence (intro, Apogee, Remedy, title), then the main
    /// menu.
    ///
    /// # Panics
    ///
    /// If the intro letterbox is not 320x200 ([`Assets::load`] guarantees it is).
    #[must_use]
    /// `config` is the player's `dr.cfg` (`assets.menu.default_config` when there is none).
    pub fn new(assets: Assets, config: DrCfg) -> Game {
        Game {
            scene: Scene::Startup(Box::new(Startup::new(assets, config))),
        }
    }

    /// The M0 test scene, which needs no game data: `-testscene`, headless runs and CI.
    #[must_use]
    pub fn test_scene() -> Game {
        Game {
            scene: Scene::Test(Box::new(TestScene::new())),
        }
    }

    pub fn input(&mut self, event: InputEvent) {
        match &mut self.scene {
            Scene::Test(scene) => scene.input(event),
            Scene::Startup(scene) => scene.input(event),
            Scene::Menu(scene) => scene.input(event),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    /// Advances the simulation by exactly one 14 ms tick.
    pub fn tick(&mut self) {
        match &mut self.scene {
            Scene::Test(scene) => scene.tick(),
            Scene::Startup(scene) => scene.tick(),
            Scene::Menu(scene) => scene.tick(),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
        if matches!(&self.scene, Scene::Startup(startup) if startup.finished())
            && let Scene::Startup(startup) = std::mem::replace(&mut self.scene, Scene::Handover)
        {
            self.scene = Scene::Menu(Box::new(startup.into_menu()));
        }
    }

    /// The player chose to exit the game and its end screen is over: the frontend should close.
    #[must_use]
    pub fn quit_requested(&self) -> bool {
        matches!(&self.scene, Scene::Menu(menu) if menu.quit_requested())
    }

    /// The bytes of `dr.cfg` when the original would write the file (at start-up, on leaving
    /// Configure, after the end screen); `None` otherwise. The frontend writes them to
    /// DeadRally's own copy.
    pub fn take_config(&mut self) -> Option<Vec<u8>> {
        match &mut self.scene {
            Scene::Startup(scene) => scene.take_config(),
            Scene::Menu(scene) => scene.take_config(),
            Scene::Test(_) => None,
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    #[must_use]
    pub fn frame(&self) -> Frame<'_> {
        match &self.scene {
            Scene::Test(scene) => scene.frame(),
            Scene::Startup(scene) => scene.frame(),
            Scene::Menu(scene) => scene.frame(),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    /// Appends the interleaved stereo samples produced since the last call
    /// (`AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS` per tick).
    pub fn take_audio(&mut self, out: &mut Vec<i16>) {
        match &mut self.scene {
            Scene::Test(scene) => scene.take_audio(out),
            Scene::Startup(scene) => scene.take_audio(out),
            Scene::Menu(scene) => scene.take_audio(out),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }
}
```

<!-- write: crates/core/src/menu/draw.rs -->
```rust
//! The menu screen's drawing, as `dr.exe` does it into its screen buffer and copies to the
//! shown buffer (spec M2a §3.3, §3.4): popups (`createPopup` 0x41A530), menus (`drawMenu`
//! 0x41A880), the cursor (`updateCursor` 0x41AB50), the highlight's moves (`refreshMenuUp`
//! 0x41AF40, `refreshMenuDown` 0x41B1A0, 0x41ACF0) and the bottom panel (0x41A7A0, 0x41E810).

use deadrally_gamedata::assets::MenuAssets;
use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::Texts;

use crate::canvas::{Canvas, at};
use crate::font::Font;

/// The fill colour of popups and of the cursor's box.
pub(crate) const POPUP_FILL: u8 = 0xC4;
/// Popup lines: a focused popup's, an unfocused one's.
const LINE_FOCUSED: u8 = 7;
const LINE_UNFOCUSED: u8 = 4;
/// Corner pictures are 32x20, the cursor 20x20, a big glyph 32 high.
const CORNER_WIDTH: usize = 32;
const CORNER_HEIGHT: usize = 20;
const CURSOR_SIZE: usize = 20;
pub(crate) const CURSOR_FRAMES: usize = 50;

/// One menu of the table at 0x4456F0 and its active rows (0x4457F0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuTable {
    /// Which menu of `dr.exe`'s text table its rows are.
    pub(crate) text: usize,
    pub(crate) rows: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) row_height: usize,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) selected: usize,
    pub(crate) active: [bool; 9],
}

/// The main menu: start, multiplayer (inactive), configure, hall of fame, credits, exit.
pub(crate) const MAIN_MENU: MenuTable = MenuTable {
    text: 0,
    rows: 6,
    x: 145,
    y: 124,
    row_height: 28,
    width: 349,
    height: 192,
    selected: 0,
    active: [true, false, true, true, true, true, false, false, false],
};

/// The start submenu at the first start: rows 0, 3 and 5 active.
pub(crate) const START_MENU: MenuTable = MenuTable {
    text: 1,
    rows: 6,
    x: 109,
    y: 171,
    row_height: 28,
    width: 421,
    height: 192,
    selected: 0,
    active: [true, false, false, true, false, true, false, false, false],
};

/// Configure (menu 3): music volume, effect volume, define keyboard, define gamepad, the
/// gamepad switch, previous menu.
pub(crate) const CONFIGURE_MENU: MenuTable = MenuTable {
    text: 3,
    rows: 6,
    x: 95,
    y: 146,
    row_height: 28,
    width: 485,
    height: 192,
    selected: 0,
    active: [true, true, true, true, true, true, false, false, false],
};

/// Define Keyboard (menu 6): the eight controls and previous menu.
pub(crate) const KEYBOARD_MENU: MenuTable = MenuTable {
    text: 6,
    rows: 9,
    x: 50,
    y: 93,
    row_height: 28,
    width: 532,
    height: 278,
    selected: 0,
    active: [true; 9],
};

/// Define Gamepad (menu 8): seven controls (no horn) and previous menu.
pub(crate) const PAD_MENU: MenuTable = MenuTable {
    text: 8,
    rows: 8,
    x: 50,
    y: 113,
    row_height: 28,
    width: 532,
    height: 250,
    selected: 0,
    active: [true, true, true, true, true, true, true, true, false],
};

/// How a menu is drawn: unfocused (mode 0) or focused (mode 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Unfocused,
    Focused,
}

/// The menu's pictures, fonts and rows.
#[derive(Clone, Debug)]
pub(crate) struct Graphics {
    pub(crate) background: Image,
    panel_line: Image,
    corners_focused: Vec<Image>,
    corners_unfocused: Vec<Image>,
    cursor: Vec<Image>,
    pub(crate) big_a: Font,
    pub(crate) big_b: Font,
    pub(crate) big_d: Font,
    pub(crate) small: [Font; 3],
    /// `dr.exe`'s menu text table: `menus[m][r]` is row `r` of menu `m`. The original rewrites
    /// some rows as settings change.
    menus: Vec<Vec<Vec<u8>>>,
    /// The volume popups' slider and its knob.
    pub(crate) slider: Image,
    pub(crate) knob: Image,
}

impl Graphics {
    pub(crate) fn new(assets: &MenuAssets) -> Graphics {
        let texts = &assets.texts;
        Graphics {
            background: assets.background.clone(),
            panel_line: assets.panel_line.clone(),
            corners_focused: assets.corners_focused.clone(),
            corners_unfocused: assets.corners_unfocused.clone(),
            cursor: assets.cursor.clone(),
            big_a: Font::new(assets.big_a.clone(), &texts.big),
            big_b: Font::new(assets.big_b.clone(), &texts.big),
            big_d: Font::new(assets.big_d.clone(), &texts.big),
            small: [
                Font::new(assets.small_a.clone(), &texts.small),
                Font::new(assets.small_b.clone(), &texts.small),
                Font::new(assets.small_c.clone(), &texts.small),
            ],
            menus: texts.menus.clone(),
            slider: assets.slider.clone(),
            knob: assets.knob.clone(),
        }
    }

    /// Rewrites row `row` of menu `menu`, as the original copies a setting's text into its
    /// table.
    pub(crate) fn set_row(&mut self, menu: usize, row: usize, text: Vec<u8>) {
        self.menus[menu][row] = text;
    }

    pub(crate) fn cursor(&self, frame: usize) -> &Image {
        &self.cursor[frame % self.cursor.len()]
    }

    /// `createPopup(x, y, w, h, focus)`: fill, corners, then lines; nothing outside is
    /// cleared, so a popup drawn over another blends their corners.
    pub(crate) fn popup(
        &self,
        screen: &mut Canvas,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        focus: Focus,
    ) {
        if h > 8 {
            screen.fill(at(x + 2, y + 2), w - 6, h - 8, POPUP_FILL);
        }
        let (corners, line) = match focus {
            Focus::Unfocused => (&self.corners_unfocused, LINE_UNFOCUSED),
            Focus::Focused => (&self.corners_focused, LINE_FOCUSED),
        };
        let right = x + w - CORNER_WIDTH;
        let bottom = y + h - CORNER_HEIGHT;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            screen.draw(corner, offset, true);
        }
        if w > 64 {
            screen.fill(at(x + 32, y + 1), w - 64, 1, line);
            screen.fill(at(x + 32, y + h - 7), w - 64, 1, line);
        }
        if h > 40 {
            screen.fill(at(x + 1, y + 20), 1, h - 40, line);
            screen.fill(at(x + w - 5, y + 20), 1, h - 40, line);
        }
    }

    /// `drawMenu(menu, focus)`: its popup and rows; the selected row with the cursor when the
    /// menu has focus. Nothing reaches the shown buffer.
    pub(crate) fn menu(&self, screen: &mut Canvas, menu: &MenuTable, focus: Focus, cursor: usize) {
        self.popup(screen, menu.x, menu.y, menu.width, menu.height, focus);
        for row in 0..menu.rows {
            let text = &self.menus[menu.text][row];
            let at_text = at(menu.x + 32, menu.y + 5 + row * menu.row_height);
            let font = if row == menu.selected {
                if focus == Focus::Focused {
                    screen.draw(self.cursor(cursor), self.cursor_at(menu), true);
                    &self.big_a
                } else {
                    &self.big_d
                }
            } else if menu.active[row] && focus == Focus::Focused {
                &self.big_b
            } else {
                &self.big_d
            };
            font.draw(screen, text, at_text);
        }
    }

    /// Where the selected row's cursor goes.
    fn cursor_at(&self, menu: &MenuTable) -> usize {
        at(menu.x + 9, menu.y + 11 + menu.selected * menu.row_height)
    }

    /// `updateCursor`: the cursor's box refilled, frame `frame` drawn, the box copied to the
    /// shown buffer.
    pub(crate) fn update_cursor(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &MenuTable,
        frame: usize,
    ) {
        let offset = self.cursor_at(menu);
        screen.fill(offset, CURSOR_SIZE, CURSOR_SIZE, POPUP_FILL);
        screen.draw(self.cursor(frame), offset, true);
        shown.copy_from(screen, offset, CURSOR_SIZE, CURSOR_SIZE);
    }

    /// Moves the highlight to row `to` as `refreshMenuUp`/`refreshMenuDown` and 0x41ACF0 do:
    /// both rows' areas refilled and redrawn, the cursor drawn with frame `frame`, both
    /// copied to the shown buffer. `base` is 6 for Up and the jump to the last row, 5 for Down.
    pub(crate) fn move_highlight(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &mut MenuTable,
        to: usize,
        base: usize,
        frame: usize,
    ) {
        let rows_at = |row: usize| menu.y + base + row * menu.row_height;
        let text_at = |row: usize| at(menu.x + 32, menu.y + 5 + row * menu.row_height);
        let old = menu.selected;
        screen.fill(
            at(menu.x + 9, rows_at(old) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_b
            .draw(screen, &self.menus[menu.text][old], text_at(old));
        menu.selected = to;
        screen.fill(
            at(menu.x + 9, rows_at(to) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_a
            .draw(screen, &self.menus[menu.text][to], text_at(to));
        screen.draw(self.cursor(frame), self.cursor_at(menu), true);
        shown.copy_from(screen, at(menu.x + 7, rows_at(old)), menu.width - 10, 32);
        shown.copy_from(screen, at(menu.x + 7, rows_at(to)), menu.width - 10, 32);
    }

    /// `drawTransparentBlock(x, y, w, h)`: the background restored, then the panel's two
    /// frame lines.
    pub(crate) fn panel_frame(&self, screen: &mut Canvas, x: usize, y: usize, w: usize, h: usize) {
        screen.restore(&self.background, at(x + 2, y - 4), w - 6, h);
        screen.draw(&self.panel_line, at(0, y + 1), true);
        screen.draw(&self.panel_line, at(0, y + h - 9), true);
    }

    /// `drawBottomMenuText`: rows 380..=468 restored, then the panel's last six lines at
    /// (12, 378 + 15k), each in its own small font.
    pub(crate) fn panel_text(&self, screen: &mut Canvas, panel: &Panel) {
        screen.copy_rows(&self.background, 380, 89);
        for (k, line) in panel.lines[16..].iter().enumerate() {
            if let Some(font) = self.small.get(usize::from(line.font)) {
                font.draw(screen, &line.text, at(12, 378 + 15 * k));
            }
        }
    }
}

/// One line of the bottom panel and its font (0, 1, 2: small A, B, C).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: Vec<u8>,
    pub(crate) font: u8,
}

/// The bottom message panel: 22 lines, new ones pushed in at the bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Panel {
    lines: Vec<PanelLine>,
}

impl Panel {
    /// The panel as `mainMenu` fills it at start-up: the four start-up lines in small B, an
    /// empty line before the last.
    pub(crate) fn startup(texts: &Texts) -> Panel {
        let mut panel = Panel {
            lines: vec![PanelLine::default(); 22],
        };
        let [first, second, third, last] = [0, 1, 2, 3].map(|i| texts.panel[i].clone());
        for text in [first, second, third, Vec::new(), last] {
            panel.push(text, 1);
        }
        panel
    }

    fn push(&mut self, text: Vec<u8>, font: u8) {
        self.lines.remove(0);
        self.lines.push(PanelLine { text, font });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::{HEIGHT, WIDTH};

    /// Graphics where every picture has its own colour: corners 11..=14 (focused) and 21..=24,
    /// cursor frame k colour 100 + k with a transparent top-left pixel, glyphs as in
    /// `font::tests::font` but 32x32 (big) or 16x16 (small) and colour 50 + font.
    pub(crate) fn graphics() -> Graphics {
        let solid = |w: u32, h: u32, colour: u8| Image::new(w, h, vec![colour; (w * h) as usize]);
        let glyphs = |size: u32, colour: u8| {
            (0..96)
                .map(|_| solid(size, size, colour))
                .collect::<Vec<_>>()
        };
        let metrics = |size: u8| deadrally_gamedata::text::Metrics {
            width: size,
            height: size,
            advances: vec![size; 96],
        };
        Graphics {
            background: Image::new(
                640,
                480,
                (0..WIDTH * HEIGHT).map(|i| (i % 7) as u8 + 1).collect(),
            ),
            panel_line: solid(640, 10, 99),
            corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
            corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
            cursor: (0..50)
                .map(|k| {
                    let mut frame = solid(20, 20, 100 + k);
                    frame.pixels[0] = 0;
                    frame
                })
                .collect(),
            big_a: Font::new(glyphs(32, 50), &metrics(32)),
            big_b: Font::new(glyphs(32, 51), &metrics(32)),
            big_d: Font::new(glyphs(32, 52), &metrics(32)),
            small: [
                Font::new(glyphs(16, 60), &metrics(16)),
                Font::new(glyphs(16, 61), &metrics(16)),
                Font::new(glyphs(16, 62), &metrics(16)),
            ],
            menus: texts().menus,
            slider: solid(172, 24, 70),
            knob: solid(10, 24, 71),
        }
    }

    pub(crate) fn texts() -> Texts {
        let metrics = deadrally_gamedata::text::Metrics {
            width: 32,
            height: 32,
            advances: vec![32; 96],
        };
        Texts {
            menus: (0..9)
                .map(|m| {
                    (0..9)
                        .map(|r| {
                            if r < 6 {
                                vec![b'A' + m as u8]
                            } else {
                                Vec::new()
                            }
                        })
                        .collect()
                })
                .collect(),
            panel: (0..4).map(|i| vec![b'a' + i]).collect(),
            exit_question: b"?".to_vec(),
            yes: b"Y".to_vec(),
            no: b"N".to_vec(),
            big: metrics.clone(),
            small: metrics.clone(),
            medium: metrics,
            configure: configure_texts(),
        }
    }

    /// Configure's texts: one letter each, `k` for key and pad names.
    pub(crate) fn configure_texts() -> deadrally_gamedata::text::ConfigureTexts {
        let one = |c: u8| vec![c];
        deadrally_gamedata::text::ConfigureTexts {
            adjust_music: one(b'm'),
            adjust_effects: one(b'e'),
            gamepad_on: one(b'+'),
            gamepad_off: one(b'-'),
            not_detected: one(b'!'),
            press_any_key: one(b'.'),
            controls: (0..8).map(|i| one(b'0' + i)).collect(),
            key_prompts: (0..8).map(|_| one(b'k')).collect(),
            pad_prompts: (0..7).map(|_| one(b'p')).collect(),
            key_names: (0..256).map(|_| one(b'k')).collect(),
            pad_names: (0..9).map(|_| one(b'p')).collect(),
        }
    }

    #[test]
    fn a_popup_fills_exactly_w_minus_6_columns() {
        // DreeRally fills two columns short on the main menu; the original fills x+2..=x+w-5.
        let mut screen = Canvas::default();
        graphics().popup(&mut screen, 145, 124, 349, 192, Focus::Focused);
        let p = screen.pixels();
        assert_eq!(p[at(147, 200)], POPUP_FILL);
        assert_eq!(
            p[at(489 - 1, 200)],
            POPUP_FILL,
            "x + w - 5 is the right line"
        );
        assert_eq!(p[at(489, 200)], LINE_FOCUSED);
        assert_eq!(p[at(146, 200)], LINE_FOCUSED, "left line at x + 1");
        assert_eq!(p[at(490, 200)], 0, "the shadow columns are left alone");
        assert_eq!(p[at(177, 125)], LINE_FOCUSED, "top line from x + 32");
        assert_eq!(
            p[at(461, 309)],
            LINE_FOCUSED,
            "bottom line at y + h - 7 to x + w - 33"
        );
        assert_eq!(p[at(145, 124)], 11, "top-left corner");
        assert_eq!(p[at(462, 124)], 12, "top-right corner");
        assert_eq!(p[at(145, 296)], 13, "bottom-left corner");
        assert_eq!(p[at(462 + 31, 296 + 19)], 14, "bottom-right corner");
    }

    #[test]
    fn a_focused_menu_shows_the_cursor_and_its_rows_in_three_fonts() {
        let mut screen = Canvas::default();
        let mut menu = MAIN_MENU;
        menu.selected = 2;
        graphics().menu(&mut screen, &menu, Focus::Focused, 7);
        let p = screen.pixels();
        let text_row = |row: usize| p[at(177, 129 + 28 * row)];
        assert_eq!(text_row(2), 50, "selected: big A");
        assert_eq!(text_row(0), 51, "active: big B");
        assert_eq!(text_row(1), 52, "inactive: big D");
        assert_eq!(
            p[at(155, 135 + 56)],
            107,
            "cursor frame 7 at (x + 9, y + 11 + 28 * 2)"
        );
        let mut dim = Canvas::default();
        graphics().menu(&mut dim, &menu, Focus::Unfocused, 7);
        assert_eq!(
            dim.pixels()[at(177, 129 + 56)],
            52,
            "unfocused: everything big D"
        );
        assert_eq!(dim.pixels()[at(155, 191)], POPUP_FILL, "no cursor");
        assert_eq!(dim.pixels()[at(145, 124)], 21, "unfocused corners");
    }

    #[test]
    fn the_cursor_update_copies_its_box_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        graphics().update_cursor(&mut screen, &mut shown, &MAIN_MENU, 3);
        assert_eq!(
            shown.pixels()[at(154, 135)],
            POPUP_FILL,
            "transparent pixel over the fill"
        );
        assert_eq!(shown.pixels()[at(155, 135)], 103);
        assert_eq!(shown.pixels()[at(174, 135)], 0, "only the 20x20 box");
    }

    #[test]
    fn moving_the_highlight_redraws_both_rows_and_copies_both_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        let mut menu = MAIN_MENU;
        graphics().move_highlight(&mut screen, &mut shown, &mut menu, 2, 5, 9);
        assert_eq!(menu.selected, 2);
        let s = shown.pixels();
        assert_eq!(s[at(177, 129)], 51, "the old row in big B");
        assert_eq!(s[at(177, 129 + 56)], 50, "the new row in big A");
        assert_eq!(s[at(155, 135 + 56)], 109, "cursor frame 9");
        assert_eq!(s[at(152, 129 + 28)], 0, "row 1 is not copied");
    }

    #[test]
    fn the_panel_shows_its_last_six_lines_with_the_startup_text_in_small_b() {
        let panel = Panel::startup(&texts());
        let mut screen = Canvas::default();
        graphics().panel_text(&mut screen, &panel);
        let p = screen.pixels();
        assert_eq!(
            p[at(12, 380)],
            graphics().background.pixels[at(12, 380)],
            "line 16 is empty; the restore starts at row 380"
        );
        for k in [1, 2, 3, 5] {
            assert_eq!(p[at(12, 378 + 15 * k)], 61, "line {}: small B", 16 + k);
        }
        assert_eq!(
            p[at(12, 445)],
            graphics().background.pixels[at(12, 445)],
            "line 20 is empty (the glyphs above reach row 438)"
        );
    }
}
```

<!-- write: crates/core/src/menu/configure.rs -->
```rust
//! The submenus of the main menu (spec M2a §3.3, M2b §3.2): the start submenu, Configure
//! (`showAdjustOptions`, 0x4309A0) with its volume popups and gamepad switch, Define Keyboard
//! (0x42FB00) and Define Gamepad (0x4302E0).

use deadrally_gamedata::dr_cfg::{DrCfg, KEY_COUNT, PAD_COUNT};
use deadrally_gamedata::text::ConfigureTexts;

use super::draw::{Focus, POPUP_FILL};
use super::{BACK_SOUND, CHOOSE_SOUND, MOVE_SOUND, Menu, START_MENU_BACK, State, Submenu};
use crate::canvas::at;
use crate::keys;

/// Configure's rows.
const MUSIC_ROW: usize = 0;
const EFFECTS_ROW: usize = 1;
const KEYBOARD_ROW: usize = 2;
const PAD_ROW: usize = 3;
pub(super) const SWITCH_ROW: usize = 4;
/// A volume popup's level runs 0..=128 in steps of 2: the volume / 512.
const LEVEL_STEP: i32 = 2;
const LEVEL_TOP: i32 = 128;
const LEVEL_UNIT: i32 = 512;
/// Effect 29 sounds when no gamepad is found.
const NOT_DETECTED_SOUND: u8 = 29;
/// A key Define Keyboard never takes.
const NOT_A_KEY: u8 = 0xAA;
/// Define Gamepad looks at the gamepad only after this many polls.
const PAD_SETTLE_POLLS: u32 = 15;

/// The gamepad switch's row: on when `dr.cfg` says 1 or 2, as `mainMenu` sets it.
pub(super) fn switch_text(texts: &ConfigureTexts, config: &DrCfg) -> Vec<u8> {
    if matches!(config.use_joystick(), 1 | 2) {
        texts.gamepad_on.clone()
    } else {
        texts.gamepad_off.clone()
    }
}

impl Menu {
    /// A pass of a submenu's loop: the background restored under it, the menus above it
    /// dimmed, the submenu with focus.
    pub(super) fn submenu_pass(&mut self, menu: Submenu) -> State {
        let (first_row, rows) = match menu {
            Submenu::Start => (92, 275),
            Submenu::Configure => (84, 283),
            Submenu::Keyboard | Submenu::Pad => (105, 262),
        };
        self.screen
            .copy_rows(&self.graphics.background, first_row, rows);
        self.draw_dimmed(menu);
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[menu as usize],
            Focus::Focused,
            self.cursor,
        );
        self.shown = self.screen.clone();
        State::Submenu {
            menu,
            second: false,
        }
    }

    /// The menus above `menu`, without focus.
    fn draw_dimmed(&mut self, menu: Submenu) {
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        if matches!(menu, Submenu::Keyboard | Submenu::Pad) {
            self.graphics.menu(
                &mut self.screen,
                &self.submenus[Submenu::Configure as usize],
                Focus::Unfocused,
                self.cursor,
            );
        }
    }

    /// The key read at the end of a submenu's pass (`readEventInMenu`, 0x42E0B0).
    pub(super) fn submenu_key(&mut self, menu: Submenu) -> State {
        match self.keys.take() {
            keys::ESCAPE => {
                self.sound(BACK_SOUND);
                match menu {
                    Submenu::Start => self.main_pass(),
                    Submenu::Configure => {
                        self.save = true;
                        self.main_pass()
                    }
                    Submenu::Keyboard | Submenu::Pad => self.submenu_pass(Submenu::Configure),
                }
            }
            keys::ENTER | keys::SPACE | 0x9C => {
                self.sound(CHOOSE_SOUND);
                let row = self.submenus[menu as usize].selected;
                self.choose_in(menu, row)
            }
            key @ (keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN) => {
                self.move_highlight(Some(menu), key);
                self.sound(MOVE_SOUND);
                State::Submenu {
                    menu,
                    second: false,
                }
            }
            _ => State::Submenu {
                menu,
                second: false,
            },
        }
    }

    /// What a submenu's row does. A menu's "previous" row also sets its selection back to its
    /// first row; Escape keeps it.
    fn choose_in(&mut self, menu: Submenu, row: usize) -> State {
        let table = &mut self.submenus[menu as usize];
        match (menu, row) {
            (Submenu::Start, START_MENU_BACK) => {
                table.selected = 0;
                self.main_pass()
            }
            // New game and loading wait for M3.
            (Submenu::Start, _) => self.submenu_pass(Submenu::Start),
            (Submenu::Configure, MUSIC_ROW) => self.volume_open(true),
            (Submenu::Configure, EFFECTS_ROW) => self.volume_open(false),
            (Submenu::Configure, KEYBOARD_ROW) => {
                self.control_rows();
                self.submenu_pass(Submenu::Keyboard)
            }
            (Submenu::Configure, PAD_ROW) => {
                self.control_rows();
                self.submenu_pass(Submenu::Pad)
            }
            (Submenu::Configure, SWITCH_ROW) => self.switch(),
            // 5: previous menu.
            (Submenu::Configure, _) => {
                table.selected = 0;
                self.save = true;
                self.main_pass()
            }
            (Submenu::Keyboard, KEY_COUNT) | (Submenu::Pad, PAD_COUNT) => {
                table.selected = 0;
                self.submenu_pass(Submenu::Configure)
            }
            (Submenu::Keyboard, control) => self.prompt(Submenu::Keyboard, control),
            (Submenu::Pad, control) => self.prompt(Submenu::Pad, control),
        }
    }

    /// The volume popup over the dimmed Configure, then its loop's first turn in the same
    /// tick.
    fn volume_open(&mut self, music: bool) -> State {
        let texts = &self.assets.menu.texts.configure;
        let caption = if music {
            &texts.adjust_music
        } else {
            &texts.adjust_effects
        };
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[Submenu::Configure as usize],
            Focus::Unfocused,
            self.cursor,
        );
        self.graphics
            .popup(&mut self.screen, 214, 218, 330, 70, Focus::Focused);
        self.graphics.small[0].draw(&mut self.screen, caption, at(224, 226));
        self.screen.draw(&self.graphics.slider, at(314, 250), true);
        self.shown = self.screen.clone();
        let volume = if music {
            self.config.music_volume()
        } else {
            self.config.effects_volume()
        };
        self.volume_turn(music, volume as i32 / LEVEL_UNIT)
    }

    /// The code after a volume popup's wait: Enter, Escape or keypad Enter read last end it,
    /// keeping the level; else another turn.
    pub(super) fn volume_tick(&mut self, music: bool, level: i32, last: u8) -> State {
        self.palette.after_wait();
        if matches!(last, keys::ENTER | keys::ESCAPE | 0x9C) {
            let volume = (level * LEVEL_UNIT) as u32;
            if music {
                self.config.set_music_volume(volume);
            } else {
                self.config.set_effects_volume(volume);
            }
            self.sound(BACK_SOUND);
            return self.submenu_pass(Submenu::Configure);
        }
        self.volume_turn(music, level)
    }

    /// One turn of the loop: a key, the slider and its knob at the level, the percentage, the
    /// volume applied at once.
    fn volume_turn(&mut self, music: bool, mut level: i32) -> State {
        let key = self.keys.take();
        match key {
            keys::LEFT | keys::PAD_LEFT if level > 0 => level -= LEVEL_STEP,
            keys::RIGHT | keys::PAD_RIGHT if level < LEVEL_TOP => level += LEVEL_STEP,
            _ => {}
        }
        let shift = |offset: usize| offset.wrapping_add_signed(level as isize);
        self.screen.fill(at(220, 246), 275, 30, POPUP_FILL);
        self.screen.draw(&self.graphics.slider, at(314, 250), true);
        self.screen
            .draw(&self.graphics.knob, shift(at(329, 250)), true);
        self.shown
            .copy_from(&self.screen, shift(at(327, 250)), 14, 24);
        // `(int)(level * 0.78125)`: 25/32 exactly.
        let percent = format!("{}%", level.max(0) * 25 / 32).into_bytes();
        let pen = at(309, 245) - self.graphics.big_a.width(&percent);
        self.graphics.big_a.draw(&mut self.screen, &percent, pen);
        self.shown.copy_from(&self.screen, at(224, 245), 120, 32);
        let volume = (level * LEVEL_UNIT) as u32;
        if music {
            self.sound.set_music_volume(volume);
        } else {
            self.sound.set_effects_volume(volume);
        }
        State::Volume {
            music,
            level,
            last: key,
        }
    }

    /// The prompt for control `control`'s key or gamepad input over the dimmed menus.
    fn prompt(&mut self, menu: Submenu, control: usize) -> State {
        self.draw_dimmed(menu);
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[menu as usize],
            Focus::Unfocused,
            self.cursor,
        );
        let texts = &self.assets.menu.texts.configure;
        let prompt = if menu == Submenu::Keyboard {
            &texts.key_prompts[control]
        } else {
            &texts.pad_prompts[control]
        };
        let y = 121 + 28 * control;
        self.graphics
            .popup(&mut self.screen, 295, y, 323, 48, Focus::Focused);
        self.graphics.small[0].draw(&mut self.screen, prompt, at(305, y + 13));
        self.shown = self.screen.clone();
        if menu == Submenu::Keyboard {
            State::KeyWait {
                control,
                key: self.keys.take(),
            }
        } else {
            self.keys.set_calibrating(true);
            State::PadWait { control, polls: 0 }
        }
    }

    /// `do { key = eventDetected(); waitWithRefresh(); } while (!key || key == 0xAA)`, then
    /// the key is the control's.
    pub(super) fn key_wait(&mut self, control: usize, key: u8) -> State {
        if key == 0 || key == NOT_A_KEY {
            return State::KeyWait {
                control,
                key: self.keys.take(),
            };
        }
        self.config.set_key(control, u32::from(key));
        self.keys.take();
        self.control_rows();
        self.submenu_pass(Submenu::Keyboard)
    }

    /// One poll of 0x42CBF0 after its wait: the gamepad (once it has settled), or Enter,
    /// keypad Enter or Escape for none.
    pub(super) fn pad_wait(&mut self, control: usize, polls: u32) -> State {
        self.palette.after_wait();
        let mut input = None;
        if self.config.use_joystick() as i32 > 0 && polls > PAD_SETTLE_POLLS {
            input = Some(self.keys.pad_input()).filter(|&input| input != 0);
        }
        if matches!(self.keys.take(), keys::ENTER | 0x9C | keys::ESCAPE) {
            input = Some(0);
        }
        let Some(input) = input else {
            return State::PadWait {
                control,
                polls: polls + 1,
            };
        };
        self.keys.set_calibrating(false);
        self.config.set_pad(control, u32::from(input));
        self.keys.take();
        self.control_rows();
        self.submenu_pass(Submenu::Pad)
    }

    /// Define Keyboard's and Define Gamepad's rows: each control's name and its key's or
    /// gamepad input's (0x41CA40).
    fn control_rows(&mut self) {
        let texts = &self.assets.menu.texts.configure;
        let named = |name: &[u8], value: &Vec<u8>| [name, value].concat();
        let pick =
            |names: &[Vec<u8>], value: u32| names.get(value as usize).unwrap_or(&names[0]).clone();
        for control in 0..KEY_COUNT {
            let row = named(
                &texts.controls[control],
                &pick(&texts.key_names, self.config.key(control)),
            );
            self.graphics
                .set_row(self.submenus[Submenu::Keyboard as usize].text, control, row);
        }
        for control in 0..PAD_COUNT {
            let row = named(
                &texts.controls[control],
                &pick(&texts.pad_names, self.config.pad(control)),
            );
            self.graphics
                .set_row(self.submenus[Submenu::Pad as usize].text, control, row);
        }
    }

    /// The gamepad switch: off when on; on when a gamepad is connected, else the "not
    /// detected" popup until a key.
    fn switch(&mut self) -> State {
        let configure = Submenu::Configure as usize;
        let was_on = self.config.use_joystick() != 0;
        let found = !was_on && self.keys.pad_connected();
        self.config.set_use_joystick(u32::from(found));
        self.keys.set_pad_on(found);
        let row = switch_text(&self.assets.menu.texts.configure, &self.config);
        self.graphics
            .set_row(self.submenus[configure].text, SWITCH_ROW, row);
        if was_on || found {
            return self.submenu_pass(Submenu::Configure);
        }
        self.sound(NOT_DETECTED_SOUND);
        self.screen.copy_rows(&self.graphics.background, 105, 262);
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics.menu(
            &mut self.screen,
            &self.submenus[configure],
            Focus::Unfocused,
            self.cursor,
        );
        self.graphics
            .popup(&mut self.screen, 28, 198, 595, 86, Focus::Focused);
        let texts = &self.assets.menu.texts.configure;
        self.graphics
            .big_a
            .draw(&mut self.screen, &texts.not_detected, at(140, 208));
        self.graphics
            .big_a
            .draw(&mut self.screen, &texts.press_any_key, at(97, 240));
        self.shown = self.screen.clone();
        self.keys.take();
        self.keys.take();
        self.not_detected()
    }

    /// `while (!eventDetected()) waitWithRefresh(); eventDetected();`, then back to Configure.
    pub(super) fn not_detected(&mut self) -> State {
        if self.keys.take() == 0 {
            return State::NotDetected;
        }
        self.keys.take();
        self.submenu_pass(Submenu::Configure)
    }

    /// `dr.cfg`'s bytes when the original writes the file, once.
    pub(crate) fn take_config(&mut self) -> Option<Vec<u8>> {
        std::mem::take(&mut self.save).then(|| self.config.to_bytes())
    }
}
```

<!-- write: crates/core/src/menu/mod.rs -->
```rust
//! The main menu (spec M2a §3.2–§3.5, M2b §3.2), from the title's fade to black to the end
//! screen, with Configure.
//!
//! The original runs this as straight code with waits in it (`waitWithRefresh`, 0x43D870); the
//! screen shown during a tick is what the shown buffer and the palette hold when that tick's
//! wait starts. Here [`State`] names the wait the menu stands at, and [`Menu::tick`] runs the
//! code from it to the next one.

mod configure;
pub(crate) mod draw;
pub(crate) mod palette;

use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::dr_cfg::DrCfg;

use self::draw::{
    CONFIGURE_MENU, CURSOR_FRAMES, Focus, Graphics, KEYBOARD_MENU, MAIN_MENU, MenuTable, PAD_MENU,
    POPUP_FILL, Panel, START_MENU,
};
use self::palette::MenuPalette;
use crate::audio::Sound;
use crate::canvas::{Canvas, HEIGHT, WIDTH, at};
use crate::keys::{self, Keys};
use crate::{AUDIO_FRAMES_PER_TICK, Frame};

/// The menus' sounds (`loadMenuSoundEffect`, 0x43C380): channel 1, at the configured effects
/// volume and pitch 0x28000.
const SOUND_CHANNEL: usize = 1;
const SOUND_PITCH: u32 = 0x2_8000;
const MOVE_SOUND: u8 = 25;
const BACK_SOUND: u8 = 22;
const CHOOSE_SOUND: u8 = 28;

/// The player's colour at the first start: driver 19's, 0 until a game sets it.
const PLAYER_COLOUR: usize = 0;
/// The main menu's rows: 0 start, 2 configure, 4 credits, 5 exit.
const START_ROW: usize = 0;
const CONFIGURE_ROW: usize = 2;
const CREDITS_ROW: usize = 4;
const EXIT_ROW: usize = 5;
/// The start submenu's last row returns to the main menu.
const START_MENU_BACK: usize = 5;
/// The exit question's popup and its yes/no at (x, y) = (180, 238).
const YES_NO_X: usize = 180;
const YES_NO_Y: usize = 238;
/// The end screen shows for at most 560 ticks; its fade-out lowers the music from 65500 in
/// steps of 2620.
const END_HOLD_TICKS: u32 = 560;
const END_VOLUME: u32 = 65_500;
const END_VOLUME_STEP: u32 = 2620;

/// Fades: 4 % a tick (`fadeIn` 0x427280 25 steps to 96 %, `transitionToBlack` 0x427300 26
/// steps from 100 % to 0), the menu's own fades 2 % a tick over 50 steps.
const FADE_IN_STEPS: u32 = 25;
const FADE_OUT_STEPS: u32 = 26;
const MENU_FADE_STEPS: u32 = 50;

/// The wait the menu stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// `transitionToBlack` on the title: wait `step` of 26.
    TitleToBlack {
        step: u32,
    },
    /// The menu's fade-in, wait `step` of 50.
    FadeIn {
        step: u32,
    },
    /// `readEventInMenu`: the first or second wait of a pass, in the main menu or a submenu.
    Main {
        second: bool,
    },
    Submenu {
        menu: Submenu,
        second: bool,
    },
    /// A volume popup's loop (`showAdjustOptions`, 0x4309A0): the level 0..=128 and the key
    /// read last.
    Volume {
        music: bool,
        level: i32,
        last: u8,
    },
    /// Define Keyboard waiting for control `control`'s key; `key` is the one read last.
    KeyWait {
        control: usize,
        key: u8,
    },
    /// Define Gamepad waiting for control `control`'s input (0x42CBF0): the polls so far.
    PadWait {
        control: usize,
        polls: u32,
    },
    /// The popup when the gamepad switch finds no gamepad (0x41E3B0), until a key.
    NotDetected,
    /// `drawYesNoMenu` for the exit question; `yes` is the side selected.
    Exit {
        second: bool,
        yes: bool,
    },
    /// `showEndScreen`: the menu to black, `END.BMP` in, held, out with the music.
    EndToBlack {
        step: u32,
    },
    EndIn {
        step: u32,
    },
    EndHold {
        ticks: u32,
    },
    EndOut {
        step: u32,
    },
    /// The game has ended.
    Ended,
    /// `showCredits`: the menu out (50 down to 0), each credits screen in, held, out, the
    /// menu back in.
    CreditsOut {
        step: u32,
    },
    CreditsIn {
        screen: usize,
        step: u32,
    },
    CreditsHold {
        screen: usize,
    },
    CreditsToBlack {
        screen: usize,
        step: u32,
    },
    CreditsBack {
        step: u32,
    },
}

/// The menus below the main menu, each read by `readEventInMenu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Submenu {
    Start,
    Configure,
    Keyboard,
    Pad,
}

#[derive(Debug)]
pub(crate) struct Menu {
    assets: Assets,
    graphics: Graphics,
    /// The original's screen buffer, the shown buffer, and the credits' copy of the screen.
    screen: Canvas,
    shown: Canvas,
    saved: Canvas,
    palette: MenuPalette,
    keys: Keys,
    sound: Sound,
    audio: Vec<i16>,
    main: MenuTable,
    /// Start, Configure, Define Keyboard, Define Gamepad, by [`Submenu`].
    submenus: [MenuTable; 4],
    panel: Panel,
    /// The player's `dr.cfg`, and whether the original would write it now.
    config: DrCfg,
    save: bool,
    /// The cursor's frame (0x45FBF8).
    cursor: usize,
    state: State,
}

impl Menu {
    /// Takes over from the startup when the title has faded in: the title is shown at its
    /// last fade step, `title_shown`, and the menu stands at `transitionToBlack`'s first wait.
    pub(crate) fn new(
        assets: Assets,
        sound: Sound,
        keys: Keys,
        audio: Vec<i16>,
        title_shown: &deadrally_gamedata::image::Palette,
        (config, save): (DrCfg, bool),
    ) -> Menu {
        let menu_assets = &assets.menu;
        let colour = menu_assets.copper.0[PLAYER_COLOUR];
        let mut palette =
            MenuPalette::new(&menu_assets.palette, colour, &menu_assets.background_copper);
        palette.show(title_shown, 100);
        let mut graphics = Graphics::new(menu_assets);
        graphics.set_row(
            CONFIGURE_MENU.text,
            configure::SWITCH_ROW,
            configure::switch_text(&menu_assets.texts.configure, &config),
        );
        let panel = Panel::startup(&menu_assets.texts);
        let mut shown = Canvas::default();
        shown.copy_all(&assets.title.image);
        Menu {
            graphics,
            screen: Canvas::default(),
            shown,
            saved: Canvas::default(),
            palette,
            keys,
            sound,
            audio,
            main: MAIN_MENU,
            submenus: [START_MENU, CONFIGURE_MENU, KEYBOARD_MENU, PAD_MENU],
            panel,
            config,
            save,
            cursor: 0,
            state: State::TitleToBlack { step: 0 },
            assets,
        }
    }

    pub(crate) fn input(&mut self, event: crate::InputEvent) {
        self.keys.event(event);
    }

    /// The player chose to exit and the end screen is over.
    pub(crate) fn quit_requested(&self) -> bool {
        self.state == State::Ended
    }

    pub(crate) fn tick(&mut self) {
        self.keys.tick();
        self.state = self.run();
        self.sound.render(AUDIO_FRAMES_PER_TICK, &mut self.audio);
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            pixels: self.shown.pixels(),
            palette: &self.palette.shown().0,
            aspect: (4, 3),
        }
    }

    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }

    /// The code from the wait at `self.state` to the next wait.
    fn run(&mut self) -> State {
        match self.state {
            State::TitleToBlack { step } => {
                let title = self.assets.title.palette.clone();
                self.palette.show(&title, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::TitleToBlack { step: step + 1 };
                }
                self.set_up();
                State::FadeIn { step: 0 }
            }
            State::FadeIn { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    return State::FadeIn { step: step + 1 };
                }
                self.shown = self.screen.clone();
                State::Main { second: false }
            }
            State::Main { second: false } => {
                self.palette.after_wait();
                State::Main { second: true }
            }
            State::Main { second: true } => {
                self.palette.after_wait();
                self.update_cursor_main();
                self.main_key()
            }
            State::Submenu {
                menu,
                second: false,
            } => {
                self.palette.after_wait();
                State::Submenu { menu, second: true }
            }
            State::Submenu { menu, second: true } => {
                self.palette.after_wait();
                self.graphics.update_cursor(
                    &mut self.screen,
                    &mut self.shown,
                    &self.submenus[menu as usize],
                    self.cursor,
                );
                self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
                self.submenu_key(menu)
            }
            State::Volume { music, level, last } => self.volume_tick(music, level, last),
            State::KeyWait { control, key } => self.key_wait(control, key),
            State::PadWait { control, polls } => self.pad_wait(control, polls),
            State::NotDetected => self.not_detected(),
            State::Exit { second: false, yes } => {
                self.palette.after_wait();
                State::Exit { second: true, yes }
            }
            State::Exit { second: true, yes } => {
                self.palette.after_wait();
                self.exit_key(yes)
            }
            State::EndToBlack { step } => {
                self.palette.fade(100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::EndToBlack { step: step + 1 };
                }
                self.screen.copy_all(&self.assets.menu.end.image);
                self.shown = self.screen.clone();
                State::EndIn { step: 0 }
            }
            State::EndIn { step } => {
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    State::EndIn { step: step + 1 }
                } else {
                    State::EndHold { ticks: 0 }
                }
            }
            State::EndHold { ticks } => {
                // `do { wait; i++ } while (!eventDetected() && i < 560)`.
                if self.keys.take() != 0 || ticks + 1 >= END_HOLD_TICKS {
                    State::EndOut { step: 0 }
                } else {
                    State::EndHold { ticks: ticks + 1 }
                }
            }
            State::EndOut { step } => {
                self.sound
                    .set_mask((END_VOLUME - END_VOLUME_STEP * step) >> 8);
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    State::EndOut { step: step + 1 }
                } else {
                    // `mainMenu` writes `dr.cfg` after the end screen.
                    self.save = true;
                    State::Ended
                }
            }
            State::Ended => State::Ended,
            State::CreditsOut { step } => {
                // `for (e = 50; e >= 0; e--)`: here `step` counts e down.
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step > 0 {
                    return State::CreditsOut { step: step - 1 };
                }
                self.show_credits(0);
                State::CreditsIn { screen: 0, step: 0 }
            }
            State::CreditsIn { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    return State::CreditsIn {
                        screen,
                        step: step + 1,
                    };
                }
                // A key is checked before the first wait: one pressed during the fade-in moves
                // on at once.
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsHold { screen } => {
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsToBlack { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::CreditsToBlack {
                        screen,
                        step: step + 1,
                    };
                }
                if screen == 0 {
                    self.show_credits(1);
                    return State::CreditsIn { screen: 1, step: 0 };
                }
                self.palette.compose();
                self.screen = self.saved.clone();
                self.shown = self.screen.clone();
                State::CreditsBack { step: 0 }
            }
            State::CreditsBack { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    State::CreditsBack { step: step + 1 }
                } else {
                    self.main_pass()
                }
            }
        }
    }

    /// `mainMenu` after the title: the background, the bottom panel, the main menu, the
    /// palette composed; the menu's fade-in follows.
    fn set_up(&mut self) {
        self.screen.copy_all(&self.graphics.background);
        self.graphics
            .panel_frame(&mut self.screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut self.screen, &self.panel);
        self.draw_main();
        self.shown = self.screen.clone();
        self.palette.compose();
    }

    /// The top of `mainMenu`'s loop: rows 84..=366 restored, the main menu drawn with focus.
    fn draw_main(&mut self) {
        self.screen.copy_rows(&self.graphics.background, 84, 283);
        self.main.active[1] = false;
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Focused, self.cursor);
    }

    /// A pass of `mainMenu`'s loop without the fade: drawn, shown, waiting for a key.
    fn main_pass(&mut self) -> State {
        self.draw_main();
        self.shown = self.screen.clone();
        State::Main { second: false }
    }

    fn update_cursor_main(&mut self) {
        self.graphics
            .update_cursor(&mut self.screen, &mut self.shown, &self.main, self.cursor);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
    }

    fn sound(&mut self, effect: u8) {
        self.sound.trigger_at(
            SOUND_CHANNEL,
            effect,
            self.config.effects_volume(),
            SOUND_PITCH,
        );
    }

    /// Moves the highlight of `menu` (the main menu when `None`) for Up, Down or Escape.
    fn move_highlight(&mut self, menu: Option<Submenu>, key: u8) {
        let menu = match menu {
            None => &mut self.main,
            Some(submenu) => &mut self.submenus[submenu as usize],
        };
        let (to, base) = match key {
            keys::UP | keys::PAD_UP => {
                let mut row = menu.selected;
                loop {
                    row = if row == 0 { menu.rows - 1 } else { row - 1 };
                    if menu.active[row] {
                        break (row, 6);
                    }
                }
            }
            keys::DOWN | keys::PAD_DOWN => {
                let mut row = menu.selected;
                loop {
                    row = if row + 1 >= menu.rows { 0 } else { row + 1 };
                    if menu.active[row] {
                        break (row, 5);
                    }
                }
            }
            _ => (menu.rows - 1, 6),
        };
        self.graphics.move_highlight(
            &mut self.screen,
            &mut self.shown,
            menu,
            to,
            base,
            self.cursor,
        );
    }

    /// The key read at the end of a main menu pass.
    fn main_key(&mut self) -> State {
        match self.keys.take() {
            keys::ESCAPE => {
                if self.main.selected != self.main.rows - 1 {
                    self.move_highlight(None, keys::ESCAPE);
                    self.sound(MOVE_SOUND);
                }
            }
            keys::ENTER | keys::SPACE | 0x9C => {
                self.sound(CHOOSE_SOUND);
                return self.choose(self.main.selected);
            }
            key @ (keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN) => {
                self.move_highlight(None, key);
                self.sound(MOVE_SOUND);
            }
            _ => {}
        }
        State::Main { second: false }
    }

    /// What a main menu row does.
    fn choose(&mut self, row: usize) -> State {
        match row {
            START_ROW => self.submenu_pass(Submenu::Start),
            CONFIGURE_ROW => self.submenu_pass(Submenu::Configure),
            CREDITS_ROW => {
                self.saved = self.screen.clone();
                self.palette.compose();
                State::CreditsOut {
                    step: MENU_FADE_STEPS,
                }
            }
            EXIT_ROW => self.ask_exit(),
            // The Hall of Fame comes with M2c.
            _ => self.main_pass(),
        }
    }

    /// `mainMenu`'s exit question: the main menu dimmed over itself, the question's popup,
    /// "no" selected.
    fn ask_exit(&mut self) -> State {
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics
            .popup(&mut self.screen, 170, 200, 300, 80, Focus::Focused);
        let question = &self.assets.menu.texts.exit_question;
        self.graphics.small[0].draw(&mut self.screen, question, at(253, 208));
        self.draw_yes_no(false);
        self.shown = self.screen.clone();
        State::Exit {
            second: false,
            yes: false,
        }
    }

    /// The two answers, the selected one in big A.
    fn draw_yes_no(&mut self, yes: bool) {
        let texts = &self.assets.menu.texts;
        let (yes_font, no_font) = if yes {
            (&self.graphics.big_a, &self.graphics.big_b)
        } else {
            (&self.graphics.big_b, &self.graphics.big_a)
        };
        yes_font.draw(
            &mut self.screen,
            &texts.yes,
            at(YES_NO_X + 30, YES_NO_Y - 7),
        );
        no_font.draw(
            &mut self.screen,
            &texts.no,
            at(YES_NO_X + 200, YES_NO_Y - 7),
        );
    }

    fn exit_key(&mut self, yes: bool) -> State {
        let cursor_x = if yes { YES_NO_X + 7 } else { YES_NO_X + 177 };
        let cursor_at = at(cursor_x, YES_NO_Y);
        self.screen.fill(cursor_at, 20, 20, POPUP_FILL);
        let cursor = self.graphics.cursor(self.cursor).clone();
        self.screen.draw(&cursor, cursor_at, true);
        self.shown
            .copy_from(&self.screen, at(YES_NO_X + 2, YES_NO_Y), 240, 28);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
        let key = match self.keys.take() {
            keys::Y => keys::PAD_LEFT,
            keys::N => keys::PAD_RIGHT,
            key => key,
        };
        let answer = match key {
            keys::LEFT | keys::PAD_LEFT | keys::RIGHT | keys::PAD_RIGHT => {
                let left = matches!(key, keys::LEFT | keys::PAD_LEFT);
                if left != yes {
                    self.sound(MOVE_SOUND);
                }
                self.screen
                    .fill(at(YES_NO_X + 2, YES_NO_Y), 240, 25, POPUP_FILL);
                self.draw_yes_no(left);
                return State::Exit {
                    second: false,
                    yes: left,
                };
            }
            keys::ESCAPE => false,
            keys::ENTER | 0x9C => yes,
            _ => {
                return State::Exit { second: false, yes };
            }
        };
        self.sound(CHOOSE_SOUND);
        if answer {
            self.palette.compose();
            State::EndToBlack { step: 0 }
        } else {
            self.main_pass()
        }
    }

    /// Credits screen `screen` drawn and shown, its palette black.
    fn show_credits(&mut self, screen: usize) {
        self.screen
            .copy_all(&self.assets.menu.credits[screen].image);
        self.shown = self.screen.clone();
    }
}
```

<!-- write: crates/deadrally/src/main.rs -->
```rust
//! DeadRally: the game's frontend (see docs/adr/0001-platform-layer.md). It plays the
//! original's startup sequence on SDL3, silently until M1b.
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-novsync` turns vsync off, for measuring present cost; `-testscene` runs the
//! M0 test scene, which needs no game data; `--data <dir>` names the game data directory (else
//! `DEADRALLY_DATA`, else `data_path` in the config file). Alt+Enter toggles fullscreen, F12
//! toggles bilinear smoothing, closing the window quits. One stats line per second goes to
//! stdout.

mod keymap;

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioGate, Pacer, RunStats, letterbox};
use deadrally_core::{AUDIO_CHANNELS, AUDIO_SAMPLE_RATE, Game, InputEvent, PadAxis};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::dr_cfg;
use deadrally_gamedata::{DATA_ENV_VAR, LocateError, Outcome, config_path, locate};
use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Gamepad};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FRect, ScaleMode};
use sdl3::video::FullscreenType;

const BYTES_PER_SAMPLE: usize = 2;

#[derive(Debug, PartialEq, Eq)]
struct Options {
    windowed: bool,
    vsync: bool,
    test_scene: bool,
    data: Option<PathBuf>,
}

fn parse_options(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut options = Options {
        windowed: false,
        vsync: true,
        test_scene: false,
        data: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-window") => options.windowed = true,
            Some("-novsync") => options.vsync = false,
            Some("-testscene") => options.test_scene = true,
            Some("--data") => {
                options.data = Some(PathBuf::from(
                    args.next().ok_or("--data needs a directory")?,
                ));
            }
            _ => {
                return Err(format!(
                    "unknown option {}; known: -window, -novsync, -testscene, --data <dir>",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(options)
}

/// The startup sequence on the player's data and `dr.cfg`, a warning to show when the data is
/// not a known release, and where DeadRally keeps its `dr.cfg`.
type Loaded = (Game, Option<String>, Option<PathBuf>);

fn load_game(data: Option<&Path>) -> Result<Loaded, String> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let hint = || {
        let file = config.as_deref().map_or_else(
            || "the config file".to_owned(),
            |path| path.display().to_string(),
        );
        format!(
            "Point DeadRally at your copy of Death Rally (the folder that holds MENU.BPA) with \
             --data <dir>, the {DATA_ENV_VAR} environment variable, or data_path in {file}."
        )
    };
    let located = locate(data, env.as_deref(), config.as_deref()).map_err(|error| match error {
        // This one already names all three ways.
        LocateError::NotSpecified { .. } => error.to_string(),
        _ => format!("{error}\n\n{}", hint()),
    })?;
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    let dir = &located.validation.dir;
    let warning = match &located.validation.outcome {
        Outcome::Known { .. } => None,
        Outcome::Unknown { closest, differing } => Some(format!(
            "The game data in {} is not a release DeadRally knows (closest: {closest}; \
             different: {}). The game starts anyway, but it may not match the original.",
            dir.display(),
            differing.join(", ")
        )),
    };
    let assets = Assets::load(&located.validation).map_err(|error| {
        format!(
            "cannot read the game data in {}: {error}\n\n{}",
            dir.display(),
            hint()
        )
    })?;
    let own = dr_cfg::own_path();
    let config = dr_cfg::load(own.as_deref(), dir, &assets.menu.default_config)
        .map_err(|error| format!("cannot read dr.cfg: {error}"))?;
    Ok((Game::new(assets, config), warning, own))
}

/// Shows `message` in a dialog as well as on stderr; the dialog is best effort (there may be
/// no display at all).
fn tell(flag: MessageBoxFlag, title: &str, message: &str) {
    eprintln!("{}: {message}", title.to_lowercase());
    let _ = show_simple_message_box(flag, &format!("DeadRally: {title}"), message, None);
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options(std::env::args_os().skip(1))?;
    let (mut game, dr_cfg_path) = if options.test_scene {
        (Game::test_scene(), None)
    } else {
        match load_game(options.data.as_deref()) {
            Ok((game, warning, own)) => {
                if let Some(warning) = warning {
                    tell(MessageBoxFlag::WARNING, "Warning", &warning);
                }
                (game, own)
            }
            Err(message) => {
                tell(MessageBoxFlag::ERROR, "Error", &message);
                std::process::exit(1);
            }
        }
    };
    sdl3::hint::set("SDL_RENDER_VSYNC", if options.vsync { "1" } else { "0" });

    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let gamepads = sdl.gamepad()?;
    let audio = sdl.audio()?;

    let mut window = video.window("DR", 640, 480);
    window.resizable();
    if !options.windowed {
        window.fullscreen();
    }
    let mut canvas = window.build()?.into_canvas();
    let texture_creator = canvas.texture_creator();

    let spec = AudioSpec {
        freq: Some(i32::try_from(AUDIO_SAMPLE_RATE)?),
        channels: Some(i32::try_from(AUDIO_CHANNELS)?),
        format: Some(AudioFormat::s16_sys()),
    };
    let stream = audio
        .open_playback_device(&spec)?
        .open_device_stream(Some(&spec))?;
    stream.resume()?;

    let mut texture_size = (0, 0);
    let mut texture = None;
    let mut rgba = Vec::new();
    let mut samples = Vec::new();
    let mut outgoing = Vec::new();
    let mut smooth = false;
    let mut open_pads: Vec<Gamepad> = Vec::new();

    let mut pacer = Pacer::new();
    let mut gate = AudioGate::new();
    let mut stats = RunStats::new();
    let start = Instant::now();
    let mut last = start;
    let mut last_report = start;

    let mut events = sdl.event_pump()?;
    'running: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    scancode: Some(Scancode::Return),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) => {
                    // Alt's own press still reaches the game before this, so Alt+Enter skips
                    // the intro or a logo. The original does the same: refreshScreen (0x43B580)
                    // remembers every key press except F12 and Enter with Alt.
                    let window = canvas.window_mut();
                    let fullscreen = window.fullscreen_state() != FullscreenType::Off;
                    window.set_fullscreen(!fullscreen)?;
                }
                Event::KeyDown {
                    scancode: Some(Scancode::F12),
                    repeat: false,
                    ..
                } => smooth = !smooth,
                Event::KeyUp {
                    scancode: Some(Scancode::F12),
                    ..
                } => {}
                Event::KeyDown {
                    scancode: Some(scancode),
                    repeat: false,
                    ..
                } => {
                    if let Some(key) = keymap::key(scancode) {
                        game.input(InputEvent::Key { key, pressed: true });
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = keymap::key(scancode) {
                        game.input(InputEvent::Key {
                            key,
                            pressed: false,
                        });
                    }
                }
                Event::GamepadAdded { which, .. } => {
                    match gamepads.open(which) {
                        Ok(pad) => open_pads.push(pad),
                        Err(error) => eprintln!("cannot open gamepad: {error}"),
                    }
                    game.input(InputEvent::PadConnected {
                        connected: !open_pads.is_empty(),
                    });
                }
                Event::GamepadRemoved { which, .. } => {
                    open_pads.retain(|pad| pad.id().ok() != Some(which));
                    game.input(InputEvent::PadConnected {
                        connected: !open_pads.is_empty(),
                    });
                }
                Event::GamepadButtonDown { button, .. } => {
                    if let Some(button) = keymap::pad_button(button) {
                        game.input(InputEvent::PadButton {
                            button,
                            pressed: true,
                        });
                    }
                }
                Event::GamepadButtonUp { button, .. } => {
                    if let Some(button) = keymap::pad_button(button) {
                        game.input(InputEvent::PadButton {
                            button,
                            pressed: false,
                        });
                    }
                }
                Event::GamepadAxisMotion {
                    axis: Axis::LeftX,
                    value,
                    ..
                } => {
                    game.input(InputEvent::PadAxis {
                        axis: PadAxis::StickX,
                        value,
                    });
                }
                Event::GamepadAxisMotion {
                    axis: Axis::LeftY,
                    value,
                    ..
                } => {
                    game.input(InputEvent::PadAxis {
                        axis: PadAxis::StickY,
                        value,
                    });
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let ticks = pacer.advance(nanos(now - last));
        last = now;
        for _ in 0..ticks {
            game.tick();
            samples.clear();
            game.take_audio(&mut samples);
            let queued_frames =
                usize::try_from(stream.queued_bytes()?)? / (AUDIO_CHANNELS * BYTES_PER_SAMPLE);
            outgoing.clear();
            gate.feed(queued_frames, &samples, &mut outgoing);
            if !outgoing.is_empty() {
                stream.put_data_i16(&outgoing)?;
            }
        }
        stats.add_ticks(ticks);
        if let (Some(bytes), Some(path)) = (game.take_config(), &dr_cfg_path)
            && let Err(error) = dr_cfg::save(path, &bytes)
        {
            eprintln!("warning: cannot write {}: {error}", path.display());
        }
        if game.quit_requested() {
            break 'running;
        }

        let present_start = Instant::now();
        let frame = game.frame();
        if texture.is_none() || texture_size != (frame.width, frame.height) {
            texture = Some(texture_creator.create_texture_streaming(
                PixelFormat::RGBA32,
                frame.width,
                frame.height,
            )?);
            texture_size = (frame.width, frame.height);
            rgba.resize(frame.pixels.len() * 4, 0);
        }
        let texture = texture.as_mut().expect("created above");
        frame.write_rgba(&mut rgba);
        texture.update(None, &rgba, frame.width as usize * 4)?;
        texture.set_scale_mode(if smooth {
            ScaleMode::Linear
        } else {
            ScaleMode::Nearest
        });

        let (output_width, output_height) = canvas.output_size()?;
        let viewport = letterbox(output_width, output_height, frame.aspect);
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        if viewport.width > 0 && viewport.height > 0 {
            let target = FRect::new(
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
            );
            canvas.copy(texture, None, Some(target))?;
        }
        canvas.present();
        stats.add_present(u32::try_from(present_start.elapsed().as_micros()).unwrap_or(u32::MAX));

        if now - last_report >= Duration::from_secs(1) {
            last_report = now;
            let queued_frames =
                usize::try_from(stream.queued_bytes()?)? / (AUDIO_CHANNELS * BYTES_PER_SAMPLE);
            let audio = gate.report(queued_frames);
            println!(
                "{}",
                stats.line(nanos(now - start), pacer.dropped_ticks(), audio)
            );
        }
    }

    let audio = gate.report(0);
    println!(
        "final {}",
        stats.line(nanos(start.elapsed()), pacer.dropped_ticks(), audio)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(list: &[&str]) -> Result<Options, String> {
        parse_options(list.iter().map(OsString::from))
    }

    #[test]
    fn options_select_the_scene_and_the_data() {
        let options = parse(&["-window", "-testscene", "--data", "/games/dr"]).unwrap();
        assert!(options.windowed && options.test_scene && options.vsync);
        assert_eq!(options.data, Some(PathBuf::from("/games/dr")));
        assert_eq!(parse(&[]).unwrap().data, None);
    }

    #[test]
    fn unusable_data_says_how_to_point_at_other_data() {
        // A player whose copy is incomplete or damaged must learn how to choose another one.
        let empty = tempfile::tempdir().unwrap();
        let message = load_game(Some(empty.path())).expect_err("an empty folder is no game data");
        for needle in ["--data", "DEADRALLY_DATA", "data_path"] {
            assert!(message.contains(needle), "{needle} missing in: {message}");
        }
    }

    #[test]
    fn unknown_or_incomplete_options_are_errors() {
        // A typo such as -testcsene must not silently start the real game instead.
        assert!(parse(&["-testcsene"]).unwrap_err().contains("-testscene"));
        assert!(parse(&["--data"]).is_err());
    }
}
```

<!-- write: crates/headless/src/main.rs -->
```rust
//! Runs the core without a window (spec section 8, spec M1a section 7).
//!
//! `run --ticks N` hashes every frame and every audio sample, so two runs, or the same run on
//! two operating systems, can be compared with one line. `check-data` reports where the game
//! data was found and whether it is a known release. `dump-assets` writes every catalogued
//! image as a PNG. `render`, `compare` and `find` check the startup sequence and the menus
//! against screenshots of the original (scripts/reference-run.sh). `render-audio` writes what
//! the game plays as a WAV, and `compare-audio` checks it against a recording of the original
//! (spec M1b sections 4.4 and 5).

mod audio_compare;
mod dump;
mod rgb;
mod wav;
mod window;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deadrally_core::{
    AUDIO_SAMPLE_RATE, Game, InputEvent, Key, TICK_NANOS, render_effect, render_music,
};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::sound;
use deadrally_gamedata::{DATA_ENV_VAR, Located, Outcome, config_path, locate};
use sha2::{Digest, Sha256};

use crate::rgb::{Difference, Rgb};
use crate::wav::Wav;

const USAGE: &str = "usage:
  deadrally-headless run --ticks N
  deadrally-headless check-data [--data PATH]
  deadrally-headless dump-assets [--data PATH] [--out DIR]
  deadrally-headless render [--data PATH] --tick T [--key-at T[:KEY]]... --out FILE.png
  deadrally-headless compare A.png B.png
  deadrally-headless find [--data PATH] [--key-at T[:KEY]]... [--ticks N] SHOT.png...
  deadrally-headless render-audio [--data PATH] --startup [--key-at T[:KEY]]... [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --music NAME [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --effect BANK --number K --out FILE.wav
  deadrally-headless compare-audio ORIGINAL.wav OURS.wav [--min-overlap S]";

/// Exit status of `check-data` when the data is usable but not a known release.
const EXIT_UNKNOWN_VERSION: u8 = 2;

/// `find` runs this many ticks by default: the whole startup sequence of the known version
/// (6219 ticks) and then some.
const FIND_TICKS: u64 = 7_000;

/// `render-audio --startup` renders the whole intro and then this many ticks (2 s) by default:
/// the menu music starting.
const STARTUP_AFTER_INTRO_TICKS: u64 = 143;

/// `render-audio --music` renders this many seconds by default.
const MUSIC_SECONDS: u64 = 30;
/// `render-audio --effect` renders at most this many seconds, then trims the silence.
const EFFECT_SECONDS: u64 = 10;

/// `compare-audio` tolerances (spec M1b §5).
const LOUDNESS_MEDIAN_DB: f64 = 1.5;
const LOUDNESS_MAX_DB: f64 = 4.0;
const BAND_DB: f64 = 3.0;
const TEMPO_PERCENT: f64 = 0.15;
const PITCH_CENTS: f64 = 10.0;
const BALANCE_DB: f64 = 1.0;
const MIN_OVERLAP_SECONDS: u64 = 25;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run {
        ticks: u64,
    },
    CheckData {
        data: Option<PathBuf>,
    },
    DumpAssets {
        data: Option<PathBuf>,
        out: PathBuf,
    },
    Render {
        data: Option<PathBuf>,
        tick: u64,
        keys: Vec<Press>,
        out: PathBuf,
    },
    Compare {
        a: PathBuf,
        b: PathBuf,
    },
    Find {
        data: Option<PathBuf>,
        keys: Vec<Press>,
        ticks: u64,
        shots: Vec<PathBuf>,
    },
    RenderAudio {
        data: Option<PathBuf>,
        source: AudioSource,
        keys: Vec<Press>,
        seconds: Option<u64>,
        out: PathBuf,
    },
    CompareAudio {
        original: PathBuf,
        ours: PathBuf,
        min_overlap: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum AudioSource {
    Startup,
    Music(String),
    Effect { bank: String, number: u8 },
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("error: {message}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let result = match command {
        Command::Run { ticks } => {
            println!("{}", run(ticks));
            Ok(ExitCode::SUCCESS)
        }
        Command::CheckData { data } => Ok(check_data(data.as_deref())),
        Command::DumpAssets { data, out } => locate_data(data.as_deref()).and_then(|located| {
            let count = dump::dump_assets(&located.validation, &out)?;
            println!("wrote {count} images to {}", out.display());
            Ok(ExitCode::SUCCESS)
        }),
        Command::Render {
            data,
            tick,
            keys,
            out,
        } => render(data.as_deref(), tick, &keys, &out).map(|()| ExitCode::SUCCESS),
        Command::Compare { a, b } => compare(&a, &b),
        Command::Find {
            data,
            keys,
            ticks,
            shots,
        } => find(data.as_deref(), &keys, ticks, &shots),
        Command::RenderAudio {
            data,
            source,
            keys,
            seconds,
            out,
        } => {
            render_audio(data.as_deref(), &source, &keys, seconds, &out).map(|()| ExitCode::SUCCESS)
        }
        Command::CompareAudio {
            original,
            ours,
            min_overlap,
        } => compare_audio(&original, &ours, min_overlap),
    };
    result.unwrap_or_else(|message| {
        eprintln!("error: {message}");
        ExitCode::FAILURE
    })
}

fn parse(args: &[OsString]) -> Result<Command, String> {
    let mut args = args.iter();
    let command = args.next().ok_or("missing command")?.to_string_lossy();
    let command = &*command;
    let options: &[&str] = match command {
        "run" => &["--ticks"],
        "check-data" => &["--data"],
        "dump-assets" => &["--data", "--out"],
        "render" => &["--data", "--tick", "--key-at", "--out"],
        "compare" => &[],
        "find" => &["--data", "--key-at", "--ticks"],
        "render-audio" => &[
            "--data",
            "--music",
            "--effect",
            "--number",
            "--seconds",
            "--key-at",
            "--out",
        ],
        "compare-audio" => &["--min-overlap"],
        _ => return Err(format!("unknown command: {command}")),
    };
    let takes_files = matches!(command, "compare" | "find" | "compare-audio");
    let (mut data, mut out, mut ticks, mut tick) = (None, None, None, None);
    let (mut startup, mut music, mut effect, mut effect_number, mut seconds, mut min_overlap) =
        (false, None, None, None, None, None);
    let mut keys = Vec::new();
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        let name = arg.to_str().unwrap_or_default();
        if command == "render-audio" && name == "--startup" {
            startup = true;
        } else if options.contains(&name) {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            let number = || {
                value
                    .to_str()
                    .and_then(|text| text.parse::<u64>().ok())
                    .ok_or(format!("{name}: not a number: {}", value.to_string_lossy()))
            };
            match name {
                "--data" => data = Some(PathBuf::from(value)),
                "--out" => out = Some(PathBuf::from(value)),
                "--ticks" => ticks = Some(number()?),
                "--tick" => tick = Some(number()?),
                "--key-at" => keys.push(press(&value.to_string_lossy())?),
                "--music" => music = Some(value.to_string_lossy().into_owned()),
                "--effect" => effect = Some(value.to_string_lossy().into_owned()),
                "--number" => {
                    effect_number = Some(
                        u8::try_from(number()?).map_err(|_| "--number: an effect is 1 to 255")?,
                    );
                }
                "--seconds" => seconds = Some(number()?),
                "--min-overlap" => min_overlap = Some(number()?),
                _ => unreachable!("every option is handled"),
            }
        } else if takes_files && !name.starts_with("--") {
            files.push(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument: {}", arg.to_string_lossy()));
        }
    }
    match command {
        "run" => Ok(Command::Run {
            ticks: ticks.ok_or("run needs --ticks N")?,
        }),
        "check-data" => Ok(Command::CheckData { data }),
        "dump-assets" => Ok(Command::DumpAssets {
            data,
            out: out.unwrap_or_else(|| PathBuf::from("dumps")),
        }),
        "render" => Ok(Command::Render {
            data,
            tick: tick.ok_or("render needs --tick T")?,
            keys,
            out: out.ok_or("render needs --out FILE.png")?,
        }),
        "compare" => match <[PathBuf; 2]>::try_from(files) {
            Ok([a, b]) => Ok(Command::Compare { a, b }),
            Err(_) => Err("compare needs exactly two PNG files".into()),
        },
        "compare-audio" => match <[PathBuf; 2]>::try_from(files) {
            Ok([original, ours]) => Ok(Command::CompareAudio {
                original,
                ours,
                min_overlap: min_overlap.unwrap_or(MIN_OVERLAP_SECONDS),
            }),
            Err(_) => Err("compare-audio needs exactly two WAV files".into()),
        },
        "render-audio" => {
            let source = match (startup, music, effect, effect_number) {
                (true, None, None, None) => AudioSource::Startup,
                (false, Some(name), None, None) => AudioSource::Music(name),
                (false, None, Some(bank), Some(number)) if number > 0 => {
                    AudioSource::Effect { bank, number }
                }
                _ => {
                    return Err("render-audio needs one of --startup, --music NAME, or --effect BANK --number K".into());
                }
            };
            if source != AudioSource::Startup && !keys.is_empty() {
                return Err("--key-at only applies to --startup".into());
            }
            Ok(Command::RenderAudio {
                data,
                source,
                keys,
                seconds,
                out: out.ok_or("render-audio needs --out FILE.wav")?,
            })
        }
        _ => {
            if files.is_empty() {
                return Err("find needs at least one screenshot".into());
            }
            Ok(Command::Find {
                data,
                keys,
                ticks: ticks.unwrap_or(FIND_TICKS),
                shots: files,
            })
        }
    }
}

/// Runs `ticks` ticks without input and hashes, per tick in order: width, height and both
/// aspect terms as little-endian u32, the 768 palette bytes, the pixels; and every audio sample
/// as little-endian i16.
fn run(ticks: u64) -> String {
    let mut game = Game::test_scene();
    let mut frames = Sha256::new();
    let mut audio = Sha256::new();
    let mut samples = Vec::new();
    let mut sample_bytes = Vec::new();
    for _ in 0..ticks {
        game.tick();
        let frame = game.frame();
        for value in [frame.width, frame.height, frame.aspect.0, frame.aspect.1] {
            frames.update(value.to_le_bytes());
        }
        frames.update(frame.palette.as_flattened());
        frames.update(frame.pixels);

        samples.clear();
        game.take_audio(&mut samples);
        sample_bytes.clear();
        sample_bytes.extend(samples.iter().flat_map(|sample| sample.to_le_bytes()));
        audio.update(&sample_bytes);
    }
    format!(
        "ticks={ticks} frames_sha256={} audio_sha256={}",
        hex(&frames.finalize()),
        hex(&audio.finalize())
    )
}

fn check_data(cli: Option<&Path>) -> ExitCode {
    let config = config_path();
    match &config {
        Some(path) => println!("config file: {}", path.display()),
        None => println!("config file: unavailable (this system has no config directory)"),
    }
    let env = std::env::var_os(DATA_ENV_VAR);
    let located = match locate(cli, env.as_deref(), config.as_deref()) {
        Ok(located) => located,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    println!("source: {}", located.source);
    println!("directory: {}", located.validation.dir.display());
    for file in &located.validation.files {
        println!("  {:<12} {:>9}  {}", file.name, file.size, file.sha256);
    }
    match &located.validation.outcome {
        Outcome::Known { version } => {
            println!("outcome: known version: {version}");
            ExitCode::SUCCESS
        }
        Outcome::Unknown { closest, differing } => {
            println!("outcome: UNKNOWN VERSION (closest: {closest})");
            eprintln!(
                "warning: unknown version, the game may behave differently; files that differ from {closest}: {}",
                differing.join(", ")
            );
            ExitCode::from(EXIT_UNKNOWN_VERSION)
        }
    }
}

/// Finds and validates the data like `check-data`, warning instead of failing on an unknown
/// version.
fn locate_data(cli: Option<&Path>) -> Result<Located, String> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let located =
        locate(cli, env.as_deref(), config.as_deref()).map_err(|error| error.to_string())?;
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    if let Outcome::Unknown { closest, .. } = &located.validation.outcome {
        eprintln!(
            "warning: unknown data version (closest: {closest}); it may not match the original"
        );
    }
    Ok(located)
}

/// A key pressed and released after a number of ticks (0: before the first tick).
type Press = (u64, Key);

/// The keys `--key-at T:KEY` can name; `T` alone presses Space.
const KEY_NAMES: [(&str, Key); 10] = [
    ("space", Key::Space),
    ("enter", Key::Enter),
    ("escape", Key::Escape),
    ("up", Key::Up),
    ("down", Key::Down),
    ("left", Key::Left),
    ("right", Key::Right),
    ("y", Key::Y),
    ("n", Key::N),
    ("q", Key::Q),
];

/// Parses `T` or `T:KEY`.
fn press(text: &str) -> Result<Press, String> {
    let (tick, name) = text.split_once(':').unwrap_or((text, "space"));
    let tick = tick
        .parse()
        .map_err(|_| format!("--key-at: not a number: {tick}"))?;
    let key = KEY_NAMES
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|&(_, key)| key)
        .ok_or_else(|| {
            let names: Vec<&str> = KEY_NAMES.iter().map(|(known, _)| *known).collect();
            format!(
                "--key-at: unknown key {name}; known keys: {}",
                names.join(", ")
            )
        })?;
    Ok((tick, key))
}

/// Presses and releases the keys due before tick `done + 1`, in the order given.
fn press_due(game: &mut Game, keys: &[Press], done: u64) {
    for &(_, key) in keys.iter().filter(|(tick, _)| *tick == done) {
        for pressed in [true, false] {
            game.input(InputEvent::Key { key, pressed });
        }
    }
}

/// Runs `ticks` ticks, pressing and releasing the keys in `keys`, and calls `each` with the tick
/// count and the game after every tick.
fn play(game: &mut Game, ticks: u64, keys: &[Press], mut each: impl FnMut(u64, &Game)) {
    for done in 0..ticks {
        press_due(game, keys, done);
        game.tick();
        each(done + 1, game);
    }
}

/// Writes the frame after `tick` ticks as the original's window would show it.
fn render(data: Option<&Path>, tick: u64, keys: &[Press], out: &Path) -> Result<(), String> {
    let located = locate_data(data)?;
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let config = assets.menu.default_config.clone();
    let mut game = Game::new(assets, config);
    play(&mut game, tick, keys, |_, _| {});
    window::present(&game.frame())?.write_png(out)
}

/// Exit status 0 only when the pictures are identical.
fn compare(a: &Path, b: &Path) -> Result<ExitCode, String> {
    let (first, second) = (Rgb::read_png(a)?, Rgb::read_png(b)?);
    let Some(difference) = first.difference(&second) else {
        println!(
            "sizes differ: {}x{} and {}x{}",
            first.width, first.height, second.width, second.height
        );
        return Ok(ExitCode::FAILURE);
    };
    println!(
        "{}x{}: {} pixels differ, largest channel difference {}",
        first.width, first.height, difference.pixels, difference.max_channel
    );
    Ok(if difference.pixels == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// For each screenshot of the original, the ticks of our startup sequence that show exactly
/// the same picture. Exit status 0 only when every screenshot has a match.
fn find(
    data: Option<&Path>,
    keys: &[Press],
    ticks: u64,
    shots: &[PathBuf],
) -> Result<ExitCode, String> {
    let pictures = shots
        .iter()
        .map(|path| Rgb::read_png(path))
        .collect::<Result<Vec<_>, _>>()?;
    for (path, picture) in shots.iter().zip(&pictures) {
        if (picture.width, picture.height) != (window::WIDTH, window::HEIGHT) {
            return Err(format!(
                "{}: {}x{}, the original's window is {}x{}",
                path.display(),
                picture.width,
                picture.height,
                window::WIDTH,
                window::HEIGHT
            ));
        }
    }
    let located = locate_data(data)?;
    // First only exact matches, which fail fast on the first differing byte.
    let mut matches = vec![Vec::new(); shots.len()];
    let mut equal = vec![false; shots.len()];
    timeline(&located, keys, ticks, |tick, window, changed| {
        for (index, picture) in pictures.iter().enumerate() {
            if changed {
                equal[index] = window.pixels == picture.pixels;
            }
            if equal[index] {
                matches[index].push(tick);
            }
        }
    })?;
    // Then, for screenshots without a match, the nearest picture, to help find out why.
    let unmatched: Vec<usize> = (0..shots.len())
        .filter(|&index| matches[index].is_empty())
        .collect();
    let mut closest: Vec<Option<(Difference, u64)>> = vec![None; shots.len()];
    if !unmatched.is_empty() {
        timeline(&located, keys, ticks, |tick, window, changed| {
            if !changed {
                return;
            }
            for &index in &unmatched {
                let difference = window.difference(&pictures[index]).expect("window-sized");
                if closest[index].is_none_or(|(best, _)| difference < best) {
                    closest[index] = Some((difference, tick));
                }
            }
        })?;
    }
    for (index, path) in shots.iter().enumerate() {
        match closest[index] {
            None => println!("{}: ticks {}", path.display(), ranges(&matches[index])),
            Some((difference, tick)) => println!(
                "{}: no exact match; closest is tick {tick}: {} pixels differ, largest channel difference {}",
                path.display(),
                difference.pixels,
                difference.max_channel
            ),
        }
    }
    Ok(if unmatched.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Plays the startup sequence for `ticks` ticks and calls `each` with every tick count from 0,
/// the window picture and whether it changed since the previous call. Most ticks repeat the
/// previous picture, so callers can skip work on those.
fn timeline(
    located: &Located,
    keys: &[Press],
    ticks: u64,
    mut each: impl FnMut(u64, &Rgb, bool),
) -> Result<(), String> {
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let config = assets.menu.default_config.clone();
    let mut game = Game::new(assets, config);
    let mut previous: Option<(Vec<u8>, Vec<[u8; 3]>, Rgb)> = None;
    let mut failure = None;
    let mut visit = |tick: u64, game: &Game| {
        let frame = game.frame();
        let changed = previous
            .as_ref()
            .is_none_or(|(pixels, palette, _)| pixels != frame.pixels || palette != frame.palette);
        if changed {
            match window::present(&frame) {
                Ok(window) => {
                    previous = Some((frame.pixels.to_vec(), frame.palette.to_vec(), window));
                }
                Err(error) => {
                    failure.get_or_insert(format!("tick {tick}: {error}"));
                    return;
                }
            }
        }
        if let Some((_, _, window)) = &previous {
            each(tick, window, changed);
        }
    };
    visit(0, &game);
    play(&mut game, ticks, keys, &mut visit);
    failure.map_or(Ok(()), Err)
}

/// A sound file's entry name from `TR0-MUS` or `tr0-mus.cmf`.
fn sound_entry(name: &str) -> String {
    let upper = name.to_ascii_uppercase();
    if upper.ends_with(".CMF") {
        upper
    } else {
        format!("{upper}.CMF")
    }
}

/// Writes the startup's sound (the intro, then the menu music), a piece of music or one effect
/// as a 48 kHz 16-bit WAV.
fn render_audio(
    data: Option<&Path>,
    source: &AudioSource,
    keys: &[Press],
    seconds: Option<u64>,
    out: &Path,
) -> Result<(), String> {
    let located = locate_data(data)?;
    let frames = |seconds: u64| {
        usize::try_from(seconds * u64::from(AUDIO_SAMPLE_RATE)).unwrap_or(usize::MAX)
    };
    let musics = || {
        let file = located
            .validation
            .files
            .iter()
            .find(|file| file.name == sound::ARCHIVE)
            .ok_or("MUSICS.BPA is not among the validated files")?;
        Archive::open(&file.path).map_err(|error| error.to_string())
    };
    let samples = match source {
        AudioSource::Startup => {
            let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
            let intro_ticks = assets
                .intro
                .delays
                .iter()
                .map(|&delay| u64::from(delay))
                .sum::<u64>();
            let ticks = seconds.map_or(intro_ticks + STARTUP_AFTER_INTRO_TICKS, |seconds| {
                seconds * 1_000_000_000 / TICK_NANOS
            });
            let config = assets.menu.default_config.clone();
            let mut game = Game::new(assets, config);
            let mut audio = Vec::new();
            for done in 0..ticks {
                press_due(&mut game, keys, done);
                game.tick();
                game.take_audio(&mut audio);
            }
            audio
        }
        AudioSource::Music(name) => {
            let module = sound::load_music(&musics()?, &sound_entry(name))
                .map_err(|error| error.to_string())?;
            render_music(&module, frames(seconds.unwrap_or(MUSIC_SECONDS)))
        }
        AudioSource::Effect { bank, number } => {
            let bank = sound::load_effects(&musics()?, &sound_entry(bank))
                .map_err(|error| error.to_string())?;
            let mut samples =
                render_effect(&bank, *number, frames(seconds.unwrap_or(EFFECT_SECONDS)));
            let end = samples
                .iter()
                .rposition(|&sample| sample != 0)
                .map_or(0, |last| (last / 2 + 1) * 2);
            samples.truncate(end);
            samples
        }
    };
    Wav {
        rate: AUDIO_SAMPLE_RATE,
        channels: 2,
        samples,
    }
    .write(out)
}

/// Exit status 0 only when every measure is within the spec's tolerances.
fn compare_audio(original: &Path, ours: &Path, min_overlap: u64) -> Result<ExitCode, String> {
    let report = audio_compare::compare(&Wav::read(original)?, &Wav::read(ours)?);
    println!(
        "lag: {:.0} ms (envelope correlation {:.3})",
        report.lag_ms, report.correlation
    );
    println!("overlap: {:.1} s", report.overlap_seconds);
    println!(
        "loudness per second, ours - original: median {:.2} dB, largest {:.2} dB",
        report.loudness_median_db, report.loudness_max_db
    );
    let bands: Vec<String> = report
        .bands
        .iter()
        .map(|(hz, difference)| format!("{hz:.0} Hz {difference:+.1} dB"))
        .collect();
    println!("octave bands, ours - original: {}", bands.join(", "));
    println!(
        "tempo, ours - original: {:+.3} % (over {} pieces of 10 s)",
        report.tempo_percent, report.tempo_pieces
    );
    println!("pitch, ours - original: {:+.0} cents", report.pitch_cents);
    println!(
        "stereo balance (left - right), ours - original: largest {:+.2} dB (over {} pieces of 10 s)",
        report.balance_db, report.balance_pieces
    );
    let mut failures = Vec::new();
    if report.overlap_seconds < min_overlap as f64 {
        failures.push(format!("overlap below {min_overlap} s"));
    }
    if report.loudness_median_db > LOUDNESS_MEDIAN_DB {
        failures.push(format!(
            "median loudness difference above {LOUDNESS_MEDIAN_DB} dB"
        ));
    }
    if report.loudness_max_db > LOUDNESS_MAX_DB {
        failures.push(format!(
            "largest loudness difference above {LOUDNESS_MAX_DB} dB"
        ));
    }
    for (hz, difference) in &report.bands {
        if difference.abs() > BAND_DB {
            failures.push(format!("{hz:.0} Hz band off by more than {BAND_DB} dB"));
        }
    }
    // A measure that could not be taken fails: passing it would hide the difference it exists
    // to find.
    if report.tempo_pieces < audio_compare::MIN_TEMPO_PIECES {
        failures.push(format!(
            "tempo measured on fewer than {} pieces of 10 s",
            audio_compare::MIN_TEMPO_PIECES
        ));
    } else if report.tempo_percent.abs() > TEMPO_PERCENT {
        failures.push(format!("tempo off by more than {TEMPO_PERCENT} %"));
    }
    if report.pitch_cents.abs() > PITCH_CENTS {
        failures.push(format!("pitch off by more than {PITCH_CENTS} cents"));
    }
    if report.balance_pieces == 0 {
        failures.push("stereo balance not measured (a mono or silent file)".to_owned());
    } else if report.balance_db.abs() > BALANCE_DB {
        failures.push(format!("stereo balance off by more than {BALANCE_DB} dB"));
    }
    if failures.is_empty() {
        println!("result: PASS");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("result: FAIL ({})", failures.join("; "));
        Ok(ExitCode::FAILURE)
    }
}

/// `[3, 4, 5, 9]` as `3-5, 9`.
fn ranges(ticks: &[u64]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0;
    for index in 1..=ticks.len() {
        if index == ticks.len() || ticks[index] != ticks[index - 1] + 1 {
            parts.push(if index - 1 == start {
                ticks[start].to_string()
            } else {
                format!("{}-{}", ticks[start], ticks[index - 1])
            });
            start = index;
        }
    }
    parts.join(", ")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_both_commands() {
        assert_eq!(
            parse(&args(&["run", "--ticks", "7000"])),
            Ok(Command::Run { ticks: 7000 })
        );
        assert_eq!(
            parse(&args(&["check-data"])),
            Ok(Command::CheckData { data: None })
        );
        assert_eq!(
            parse(&args(&["check-data", "--data", "/x"])),
            Ok(Command::CheckData {
                data: Some(PathBuf::from("/x"))
            })
        );
    }

    #[test]
    fn rejects_options_of_the_other_command() {
        // A misplaced option must not be silently ignored.
        assert!(parse(&args(&["run", "--ticks", "1", "--data", "/x"])).is_err());
        assert!(parse(&args(&["check-data", "--ticks", "1"])).is_err());
    }

    #[test]
    fn rejects_bad_tick_counts() {
        for bad in [
            &["run"][..],
            &["run", "--ticks"],
            &["run", "--ticks", "-1"],
            &["run", "--ticks", "ten"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_key_name_the_scenarios_do_not_use_is_an_error_that_lists_the_known_ones() {
        // A typo would otherwise press nothing, and a shot taken after it would match the
        // wrong frame.
        let error = parse(&args(&[
            "render", "--tick", "3", "--key-at", "2:enetr", "--out", "a.png",
        ]))
        .unwrap_err();
        assert!(error.contains("unknown key enetr"), "{error}");
        assert!(error.contains("escape"), "{error}");
    }

    #[test]
    fn parses_the_asset_commands() {
        assert_eq!(
            parse(&args(&["dump-assets"])),
            Ok(Command::DumpAssets {
                data: None,
                out: PathBuf::from("dumps")
            })
        );
        assert_eq!(
            parse(&args(&[
                "render", "--tick", "300", "--key-at", "10", "--key-at", "20:Down", "--out",
                "a.png"
            ])),
            Ok(Command::Render {
                data: None,
                tick: 300,
                keys: vec![(10, Key::Space), (20, Key::Down)],
                out: PathBuf::from("a.png")
            })
        );
        assert_eq!(
            parse(&args(&["compare", "a.png", "b.png"])),
            Ok(Command::Compare {
                a: PathBuf::from("a.png"),
                b: PathBuf::from("b.png")
            })
        );
        assert_eq!(
            parse(&args(&["find", "--data", "/x", "a.png", "b.png"])),
            Ok(Command::Find {
                data: Some(PathBuf::from("/x")),
                keys: vec![],
                ticks: FIND_TICKS,
                shots: vec![PathBuf::from("a.png"), PathBuf::from("b.png")]
            })
        );
    }

    #[test]
    fn rejects_incomplete_asset_commands() {
        for bad in [
            &["render", "--out", "a.png"][..],
            &["render", "--tick", "3"],
            &["render", "--tick", "3", "--out", "a.png", "extra.png"],
            &["compare", "a.png"],
            &["compare", "a.png", "b.png", "c.png"],
            &["find"],
            &["find", "--ticks", "many", "a.png"],
            &["dump-assets", "--tick", "3"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn tick_lists_are_printed_as_ranges() {
        assert_eq!(ranges(&[3, 4, 5, 9]), "3-5, 9");
        assert_eq!(ranges(&[7]), "7");
        assert_eq!(ranges(&[1, 3]), "1, 3");
    }
}
```

<!-- write: crates/headless/tests/rendered_audio.rs -->
```rust
//! Rendered sound against the developer's real game data. Run with `cargo test-data`; the
//! test reads DEADRALLY_DATA and fails (never passes silently) when it is unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{AUDIO_SAMPLE_RATE, Game, render_effect, render_music};
use deadrally_gamedata::Located;
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::sound::{self, EFFECTS, MUSIC};
use sha2::{Digest, Sha256};

/// Seconds of each module: several patterns, so most of each module's commands play.
const MUSIC_SECONDS: usize = 30;
/// Seconds of each effect: all of most samples, and a few loops of the looped ones.
const EFFECT_SECONDS: usize = 2;
/// Ticks of the startup after the intro: 10 s of the menu music.
const MENU_MUSIC_TICKS: u32 = 714;

/// One line per sound: the startup as the game plays it (the whole intro and the first 10 s of
/// the menu music after it), the start of every module and every effect of every bank.
fn manifest(located: &Located) -> String {
    let mut lines = String::new();
    let assets = Assets::load(&located.validation).unwrap_or_else(|error| panic!("{error}"));
    let ticks = assets
        .intro
        .delays
        .iter()
        .map(|&delay| u32::from(delay))
        .sum::<u32>()
        + MENU_MUSIC_TICKS;
    let config = assets.menu.default_config.clone();
    let mut game = Game::new(assets, config);
    let mut audio = Vec::new();
    for _ in 0..ticks {
        game.tick();
        game.take_audio(&mut audio);
    }
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  startup {ticks} ticks", hex(hasher)).unwrap();

    let archive = Archive::open(&located.validation.dir.join(sound::ARCHIVE))
        .unwrap_or_else(|error| panic!("{error}"));
    let frames = |seconds: usize| seconds * AUDIO_SAMPLE_RATE as usize;
    for name in MUSIC {
        let module = sound::load_music(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        let mut hasher = Sha256::new();
        hash(&render_music(&module, frames(MUSIC_SECONDS)), &mut hasher);
        writeln!(
            lines,
            "{}  {}/{name} first {MUSIC_SECONDS} s",
            hex(hasher),
            sound::ARCHIVE
        )
        .unwrap();
    }
    for name in EFFECTS {
        let bank = sound::load_effects(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        let mut hasher = Sha256::new();
        for (index, instrument) in bank.instruments.iter().enumerate() {
            if instrument.is_some() {
                let effect = u8::try_from(index + 1).expect("banks have under 256 instruments");
                hash(
                    &render_effect(&bank, effect, frames(EFFECT_SECONDS)),
                    &mut hasher,
                );
            }
        }
        writeln!(
            lines,
            "{}  {}/{name} every effect, {EFFECT_SECONDS} s each",
            hex(hasher),
            sound::ARCHIVE
        )
        .unwrap();
    }
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn rendered_sound_matches_the_committed_manifest() {
    // The manifest was written after the intro and the menu music were measured against the
    // original (docs/verification/m1b.md); a player change must not alter any sound unnoticed.
    check_manifest(
        "rendered-audio.sha256",
        &manifest(&located()),
        "rendered sound",
    );
}
```

<!-- write: crates/headless/tests/menu_run.rs -->
```rust
//! A run through the main menu against the developer's real game data. Run with
//! `cargo test-data`; the test reads DEADRALLY_DATA and fails (never passes silently) when it is
//! unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{Game, InputEvent, Key};
use deadrally_gamedata::assets::Assets;
use sha2::{Digest, Sha256};

/// The keys of `scripts/reference/menu-keys.scenario`, at the ticks where the original read
/// them in the run its screenshots come from (docs/verification/m2a.md): the highlight up and
/// down, Escape, the start submenu, the exit question, the credits and the end screen.
const KEYS: [(u64, Key); 30] = [
    (6423, Key::Down),
    (6495, Key::Down),
    (6566, Key::Up),
    (6638, Key::Up),
    (6709, Key::Down),
    (6780, Key::Escape),
    (6852, Key::Up),
    (6923, Key::Up),
    (6995, Key::Up),
    (7066, Key::Up),
    (7137, Key::Enter),
    (7209, Key::Down),
    (7280, Key::Down),
    (7352, Key::Enter),
    (7423, Key::Enter),
    (7495, Key::Escape),
    (7566, Key::Escape),
    (7637, Key::Enter),
    (7709, Key::Left),
    (7780, Key::Right),
    (7852, Key::Escape),
    (7923, Key::Up),
    (7995, Key::Enter),
    (8138, Key::Space),
    (8280, Key::Space),
    (8423, Key::Down),
    (8495, Key::Enter),
    (8566, Key::Left),
    (8638, Key::Enter),
    (8923, Key::Space),
];

/// The frame after each of these ticks equals the scenario's screenshot of that name.
const SHOTS: [(u64, &str); 47] = [
    (6388, "idle"),
    (6459, "after-90000"),
    (6530, "after-91000"),
    (6602, "after-92000"),
    (6673, "after-93000"),
    (6745, "after-94000"),
    (6816, "after-95000"),
    (6888, "after-96000"),
    (6959, "after-97000"),
    (7030, "after-98000"),
    (7102, "after-99000"),
    (7173, "after-100000"),
    (7245, "after-101000"),
    (7316, "after-102000"),
    (7387, "after-103000"),
    (7459, "after-104000"),
    (7530, "after-105000"),
    (7602, "after-106000"),
    (7673, "after-107000"),
    (7745, "after-108000"),
    (7816, "after-109000"),
    (7887, "after-110000"),
    (7959, "after-111000"),
    (8009, "credits-112200"),
    (8023, "credits-112400"),
    (8045, "credits-112700"),
    (8066, "credits-113000"),
    (8101, "credits-113500"),
    (8151, "credits-114200"),
    (8172, "credits-114500"),
    (8194, "credits-114800"),
    (8223, "credits-115200"),
    (8294, "credits-116200"),
    (8315, "credits-116500"),
    (8344, "credits-116900"),
    (8387, "credits-117500"),
    (8458, "after-118000"),
    (8530, "after-119000"),
    (8601, "after-120000"),
    (8651, "end-121200"),
    (8672, "end-121500"),
    (8694, "end-121800"),
    (8723, "end-122200"),
    (8780, "end-123000"),
    (8851, "end-124000"),
    (8929, "end-125100"),
    (8937, "end-125200"),
];

/// The run ends when the game asks to quit, a little after the last screenshot.
const MAX_TICKS: u64 = 9_000;

/// One line per screenshot (the frame's pixels and palette) and one for the whole run's sound.
fn manifest() -> String {
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let config = assets.menu.default_config.clone();
    let mut game = Game::new(assets, config);
    let mut lines = String::new();
    let mut audio = Vec::new();
    let mut ticks = 0;
    while !game.quit_requested() && ticks < MAX_TICKS {
        for &(_, key) in KEYS.iter().filter(|(at, _)| *at == ticks) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        game.tick();
        game.take_audio(&mut audio);
        ticks += 1;
        for &(_, name) in SHOTS.iter().filter(|(at, _)| *at == ticks) {
            let frame = game.frame();
            let mut hasher = Sha256::new();
            hasher.update(frame.pixels);
            hasher.update(frame.palette.as_flattened());
            writeln!(lines, "{}  frame after tick {ticks} ({name})", hex(hasher)).unwrap();
        }
    }
    assert!(game.quit_requested(), "the end screen asks to quit");
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  sound of {ticks} ticks", hex(hasher)).unwrap();
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_menu_run_matches_the_committed_manifest() {
    // The manifest was written after every screenshot of the run equalled our frame at its
    // tick and the sound was measured against the recording (docs/verification/m2a.md); a
    // change to the menu must not alter a frame or a sound unnoticed.
    check_manifest("menu-run.sha256", &manifest(), "the menu run");
}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (294 passed, 17 ignored).

Check that the tests can fail, one mutation at a time, each undone after (`configure.rs` is not in Git yet: undo by hand, or apply Step 3 again): `LEVEL_STEP` 2 → 1 fails both volume tests; dropping `self.keys.set_calibrating(true)` fails the gamepad test; dropping `self.save = true` for Escape in Configure fails seven tests; `let found = !was_on;` fails the "not detected" test.

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (17 passed): the default `dr.cfg` keeps the volumes the startup and the menu run were measured with, so their manifests hold.

- [ ] **Step 5: Run the game through Configure**

The frontend on its own Xvfb display, sound to a file, its configuration in the run's folder (an absolute `XDG_CONFIG_HOME`: the directories library ignores a relative one and would use the build machine's own). Save as `/tmp/frontend-configure.sh` (`chmod +x`):

```bash
#!/usr/bin/env bash
# End-to-end: the real frontend on its own Xvfb, sound to a file; keys skip the startup, answer
# the exit question with yes and end the end screen; the game must exit by itself.
set -uo pipefail
out=$(realpath -m "$1"); mkdir -p "$out"
fd=$(mktemp)
Xvfb -displayfd 3 -screen 0 1280x1024x24 -nolisten tcp 3>"$fd" 2>/dev/null &
xvfb=$!
for _ in $(seq 100); do [ -s "$fd" ] && break; sleep 0.05; done
export DISPLAY=":$(head -n1 "$fd")"
alsa=$(mktemp); printf '%s\n' 'pcm.!default { type null }' > "$alsa"
SDL_AUDIO_DRIVER=disk SDL_AUDIO_DISK_OUTPUT_FILE="$out/audio.raw" PULSE_SERVER=unix:/nonexistent \
  PIPEWIRE_REMOTE=/nonexistent ALSA_CONFIG_PATH="$alsa" DEADRALLY_DATA=~/games/DeathRally XDG_CONFIG_HOME="$out/config" \
  target/release/deadrally -window > "$out/stats.log" 2> "$out/stderr.log" &
app=$!
timeout 20 xdotool search --sync --name '^DR$' >/dev/null; sleep 1
win=$(xdotool search --name '^DR$' | tail -n1); xdotool windowfocus --sync "$win"
key() { xdotool key --window "$win" "$1"; sleep "$2"; }
key space 1; key space 1; key space 1                # intro, both logos
sleep 3                                              # title, fade to black, menu fade-in
import -window "$win" "$out/menu.png"
key Down 0.3; key Return 0.5; key Return 0.5; key Left 0.2; key Left 0.2; key Return 0.5; key Escape 0.5; key Escape 0.3; key Return 0.5; key Left 0.5
import -window "$win" "$out/question.png"
key Return 2.5
import -window "$win" "$out/end.png"
key space 0.2
for i in $(seq 50); do kill -0 $app 2>/dev/null || break; sleep 0.1; done
if kill -0 $app 2>/dev/null; then echo "FAIL: still running"; kill $app; status=1; else wait $app; echo "exited by itself, status $?"; status=0; fi
kill $xvfb; rm -f "$fd" "$alsa"
exit $status
```

Run: `cargo build --release -p deadrally && /tmp/watch-silent.sh /tmp/frontend-configure.sh captures/fe-configure && ls ~/.config/deadrally`
Expected: `exited by itself, status 0` and `watch: command exit 0, leaks 0`; `captures/fe-configure/config/deadrally/dr.cfg` holds the music at 0x7800 (two steps down) and one start; `~/.config/deadrally` holds only what it held before.

- [ ] **Step 6: Commit**

```bash
git add crates
git commit -m "feat: add Configure and keep dr.cfg" -m "- the volume popups, Define Keyboard, Define Gamepad and the gamepad switch
- Game takes dr.cfg and hands it out when the original writes it
- the frontend keeps DeadRally's own dr.cfg and reports gamepads"
```

---

### Task 4: Checking Configure against the original

Spec 5. A scenario of the original's Configure; its shots must equal our frames and its last `dr.cfg` ours but for the random byte; then a manifest pins the run.

**Files:**
- Create: `scripts/reference/menu-configure.scenario`, `crates/headless/tests/configure-run.sha256`
- Modify: `crates/headless/tests/menu_run.rs`, `crates/headless/tests/menu-run.sha256`

- [ ] **Step 1: Screenshots and the original's `dr.cfg`**

<!-- write: scripts/reference/menu-configure.scenario -->
```text
# The Configure menu (spec M2b section 5): both volume popups, Define Keyboard with one key
# redefined, Define Gamepad, and the gamepad switch, which finds no gamepad under Wine.
at 89500 shot idle
at 90000 key Down
at 91000 key Return
at 91500 shot configure
at 92000 key Return
at 92500 shot music
at 93000 key Left
at 93300 key Left
at 93600 key Left
at 94000 shot music-left
at 94500 key Return
at 95000 shot after-music
at 95500 key Down
at 96000 key Return
at 96500 shot effects
at 97000 key Right
at 97500 shot effects-right
at 98000 key Return
at 98500 key Down
at 99000 key Return
at 99500 shot keyboard
at 100000 key Return
at 100500 shot press-key
at 101000 key q
at 101500 shot after-q
at 102000 key Escape
at 102500 shot after-keyboard
at 103000 key Down
at 103500 key Return
at 104000 shot gamepad
at 104500 key Escape
at 105000 shot after-gamepad
at 105500 key Down
at 106000 key Return
at 106500 shot not-detected
at 107000 key space
at 107500 shot after-not-detected
at 108000 key Escape
at 108500 shot escape
at 109000 key Return
at 109500 shot previous
```

`/tmp/key-ticks.py` is M2a's (its plan, Task 6 Step 7) with `"q": "q"` added to `NAMES`. Run:
```bash
scripts/reference-run.sh scripts/reference/menu-configure.scenario captures/menu-configure
cp ~/.cache/deadrally/reference/run/dr.cfg captures/menu-configure/dr.cfg
cargo build --release -p deadrally-headless
python3 /tmp/key-ticks.py target/release/deadrally-headless captures/menu-configure 9200 > captures/menu-configure/keys.args
target/release/deadrally-headless find --ticks 9200 $(cat captures/menu-configure/keys.args) captures/menu-configure/*.png
```
Expected: `done: 17 shots`; `17 of 17 shots match`; `find` prints a `ticks` line for every shot and exits 0.

- [ ] **Step 2: Pin the run**

The test runs the record's keys (docs/verification/m2b.md), hashes our frames at the shots' ticks, the sound and the last `dr.cfg` written; the menu run's manifest gains its last `dr.cfg` too.

<!-- write: crates/headless/tests/menu_run.rs -->
```rust
//! A run through the main menu against the developer's real game data. Run with
//! `cargo test-data`; the test reads DEADRALLY_DATA and fails (never passes silently) when it is
//! unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{Game, InputEvent, Key};
use deadrally_gamedata::assets::Assets;
use sha2::{Digest, Sha256};

/// The keys of `scripts/reference/menu-keys.scenario`, at the ticks where the original read
/// them in the run its screenshots come from (docs/verification/m2a.md): the highlight up and
/// down, Escape, the start submenu, the exit question, the credits and the end screen.
const KEYS: [(u64, Key); 30] = [
    (6423, Key::Down),
    (6495, Key::Down),
    (6566, Key::Up),
    (6638, Key::Up),
    (6709, Key::Down),
    (6780, Key::Escape),
    (6852, Key::Up),
    (6923, Key::Up),
    (6995, Key::Up),
    (7066, Key::Up),
    (7137, Key::Enter),
    (7209, Key::Down),
    (7280, Key::Down),
    (7352, Key::Enter),
    (7423, Key::Enter),
    (7495, Key::Escape),
    (7566, Key::Escape),
    (7637, Key::Enter),
    (7709, Key::Left),
    (7780, Key::Right),
    (7852, Key::Escape),
    (7923, Key::Up),
    (7995, Key::Enter),
    (8138, Key::Space),
    (8280, Key::Space),
    (8423, Key::Down),
    (8495, Key::Enter),
    (8566, Key::Left),
    (8638, Key::Enter),
    (8923, Key::Space),
];

/// The frame after each of these ticks equals the scenario's screenshot of that name.
const SHOTS: [(u64, &str); 47] = [
    (6388, "idle"),
    (6459, "after-90000"),
    (6530, "after-91000"),
    (6602, "after-92000"),
    (6673, "after-93000"),
    (6745, "after-94000"),
    (6816, "after-95000"),
    (6888, "after-96000"),
    (6959, "after-97000"),
    (7030, "after-98000"),
    (7102, "after-99000"),
    (7173, "after-100000"),
    (7245, "after-101000"),
    (7316, "after-102000"),
    (7387, "after-103000"),
    (7459, "after-104000"),
    (7530, "after-105000"),
    (7602, "after-106000"),
    (7673, "after-107000"),
    (7745, "after-108000"),
    (7816, "after-109000"),
    (7887, "after-110000"),
    (7959, "after-111000"),
    (8009, "credits-112200"),
    (8023, "credits-112400"),
    (8045, "credits-112700"),
    (8066, "credits-113000"),
    (8101, "credits-113500"),
    (8151, "credits-114200"),
    (8172, "credits-114500"),
    (8194, "credits-114800"),
    (8223, "credits-115200"),
    (8294, "credits-116200"),
    (8315, "credits-116500"),
    (8344, "credits-116900"),
    (8387, "credits-117500"),
    (8458, "after-118000"),
    (8530, "after-119000"),
    (8601, "after-120000"),
    (8651, "end-121200"),
    (8672, "end-121500"),
    (8694, "end-121800"),
    (8723, "end-122200"),
    (8780, "end-123000"),
    (8851, "end-124000"),
    (8929, "end-125100"),
    (8937, "end-125200"),
];

/// The menu run ends when the game asks to quit, a little after its last screenshot.
const MAX_TICKS: u64 = 9_000;

/// The keys of `scripts/reference/menu-configure.scenario` in the run of
/// docs/verification/m2b.md: both volume popups, Define Keyboard with Q for accelerate, Define
/// Gamepad, the gamepad switch (no gamepad under Wine), Escape and "previous menu".
const CONFIGURE_KEYS: [(u64, Key); 24] = [
    (6420, Key::Down),
    (6491, Key::Enter),
    (6563, Key::Enter),
    (6634, Key::Left),
    (6656, Key::Left),
    (6677, Key::Left),
    (6742, Key::Enter),
    (6813, Key::Down),
    (6849, Key::Enter),
    (6920, Key::Right),
    (6992, Key::Enter),
    (7027, Key::Down),
    (7063, Key::Enter),
    (7134, Key::Enter),
    (7206, Key::Q),
    (7277, Key::Escape),
    (7349, Key::Down),
    (7385, Key::Enter),
    (7456, Key::Escape),
    (7528, Key::Down),
    (7563, Key::Enter),
    (7635, Key::Space),
    (7706, Key::Escape),
    (7778, Key::Enter),
];

const CONFIGURE_SHOTS: [(u64, &str); 17] = [
    (6384, "idle"),
    (6527, "configure"),
    (6598, "music"),
    (6705, "music-left"),
    (6777, "after-music"),
    (6884, "effects"),
    (6955, "effects-right"),
    (7098, "keyboard"),
    (7170, "press-key"),
    (7241, "after-q"),
    (7312, "after-keyboard"),
    (7419, "gamepad"),
    (7491, "after-gamepad"),
    (7598, "not-detected"),
    (7670, "after-not-detected"),
    (7741, "escape"),
    (7812, "previous"),
];

/// One line per screenshot (the frame's pixels and palette), one for the run's sound and one
/// for the last `dr.cfg` it wrote. The run stops at `ticks`, or earlier when the game quits.
fn manifest(keys: &[(u64, Key)], shots: &[(u64, &str)], ticks: u64) -> String {
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let config = assets.menu.default_config.clone();
    let mut game = Game::new(assets, config);
    let mut lines = String::new();
    let mut audio = Vec::new();
    let mut written = None;
    let mut done = 0;
    while !game.quit_requested() && done < ticks {
        for &(_, key) in keys.iter().filter(|(at, _)| *at == done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        game.tick();
        game.take_audio(&mut audio);
        written = game.take_config().or(written);
        done += 1;
        for &(_, name) in shots.iter().filter(|(at, _)| *at == done) {
            let frame = game.frame();
            let mut hasher = Sha256::new();
            hasher.update(frame.pixels);
            hasher.update(frame.palette.as_flattened());
            writeln!(lines, "{}  frame after tick {done} ({name})", hex(hasher)).unwrap();
        }
    }
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  sound of {done} ticks", hex(hasher)).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(written.expect("dr.cfg is written at start-up"));
    writeln!(lines, "{}  dr.cfg written last", hex(hasher)).unwrap();
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_menu_run_matches_the_committed_manifest() {
    // The manifest was written after every screenshot of the run equalled our frame at its
    // tick and the sound was measured against the recording (docs/verification/m2a.md); a
    // change to the menu must not alter a frame or a sound unnoticed.
    let lines = manifest(&KEYS, &SHOTS, MAX_TICKS);
    assert!(
        lines.contains("after tick 8937"),
        "the end screen asks to quit after its fade"
    );
    check_manifest("menu-run.sha256", &lines, "the menu run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_configure_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and the
    // dr.cfg written last equalled the original's but for its random byte
    // (docs/verification/m2b.md).
    let lines = manifest(&CONFIGURE_KEYS, &CONFIGURE_SHOTS, 7_900);
    check_manifest("configure-run.sha256", &lines, "the configure run");
}
```

<!-- write: crates/headless/tests/menu-run.sha256 -->
```text
4091f209f53873669b575594a55d8d529b10866af3ee09636b78ec5fb5a88ffc  frame after tick 6388 (idle)
ef966cc32439295fcf34bde1850e533f2b2331afcb5b4b66294e826d4f6a7170  frame after tick 6459 (after-90000)
b07819bd8cc55c1455860c54fda520cd63189ed5ea94e25ef3540346fd0c1922  frame after tick 6530 (after-91000)
01854e40d80c07270e3b36b08631e784fba8000ff6c5f391fb88e477c0bcbe67  frame after tick 6602 (after-92000)
fadd7683d2dae99861e80bf9833ff0846a5aa4310ea81d1aa5d620dbf20ed905  frame after tick 6673 (after-93000)
ad9d61b963b4eaf3d80613c0ee31b9110a8d28252b86446a18d697cd8b841bb0  frame after tick 6745 (after-94000)
1acf2ce5e6d30b7a34c913b6b323d424c3009889d91f3cd70038275dd28e3df0  frame after tick 6816 (after-95000)
640e0c16c8320386d735e39a61d8e41312857669532a5b5cbeb87468761f1748  frame after tick 6888 (after-96000)
d0bc27dc83207189d402fc7d5dcbd795d5345f5653a99dd652034571c7a33a2f  frame after tick 6959 (after-97000)
1b773537df59b122953371584cf0656af186c1570054b96e6f219941a8ed737c  frame after tick 7030 (after-98000)
80331e091c0b5e52b3b66090b4ec77185a3764ef6d5c7fd1b270056370d786c2  frame after tick 7102 (after-99000)
6ab169852c9f953a15f6226d3debce997d080b10f822c4c77ac20e8d6ba8fd2a  frame after tick 7173 (after-100000)
e3b9bfb4b1a8ff509e3f0b9259df34f002f942b8a9450978c4c3e0dc8748f543  frame after tick 7245 (after-101000)
7427840326c71b49d052b2529f92aaaa443cc669432ba3823a7e8bcf8195a6ab  frame after tick 7316 (after-102000)
98c8743c58e7a29dfaa3badccab4858367d7c4f91864f4cdc4011e8f9367caa6  frame after tick 7387 (after-103000)
b281e2a4327828e06a87ecf3bd12b5612f485236ea722d3dc3552bd26084e982  frame after tick 7459 (after-104000)
6fe7e6b60bbcb94b3dac25e04781d1d620ffa71f5c3d07069e1a1dcda1d2f5c2  frame after tick 7530 (after-105000)
21e48b7d57ebc8978a701c4b4c1d0ae0273a9f201bf80a1b7889bf02c2efab73  frame after tick 7602 (after-106000)
063c6d3b0d544d0f1a267a871027d22c15f4c9366aed7eafa1458811c8d7bb15  frame after tick 7673 (after-107000)
509018dbbbfbdc8d8edbde8eb9e29dfc525348fd9f1f4875a97592bb61325656  frame after tick 7745 (after-108000)
498021ccc73ab7ce1916176a2b985af5ef44fc13fd33bbc833118df6daa5d9f3  frame after tick 7816 (after-109000)
da71965e7e30f057b691e737f52dc8b256be2850de82cfe4cffcee08b7a79613  frame after tick 7887 (after-110000)
40574faea663331b128c485a76dd5c2dc74dd96d10d8750268209bae1105c27b  frame after tick 7959 (after-111000)
1094740e3da60c169fa7adc5e6627eee0db2ed5d487ab088df55c96fd1cafd7d  frame after tick 8009 (credits-112200)
c9e67afd45e2624dd96a5c3a453c32fceb16dae70edb972e87c2a1d208409957  frame after tick 8023 (credits-112400)
c1b7ac8185dcc0380f523de59e5c1c94dd3b67f342019b0478bc5a7f9b9c2b59  frame after tick 8045 (credits-112700)
50eaf6ac5362b18de4da2b0e4746ed8bfbc067bdc83bbb0c11f38221ff3b5a6a  frame after tick 8066 (credits-113000)
d9f3fbfdc473cc63fb9b00bdc358d6418fd100c31056eb50219d2ab74a5aa125  frame after tick 8101 (credits-113500)
77be5323a9e7279a229bc2d88a080af933c6dc62a4a8388021036c7d9f4c33e8  frame after tick 8151 (credits-114200)
7a845707162c6342184e5ae6a486403355935a68e3be435a41020a0899637a09  frame after tick 8172 (credits-114500)
d41c68a51b922ed2651b140016d0c6f8ceec2d06b602af0c4e7903137101513e  frame after tick 8194 (credits-114800)
d41c68a51b922ed2651b140016d0c6f8ceec2d06b602af0c4e7903137101513e  frame after tick 8223 (credits-115200)
378bcbe5a034c323d022631d5a7a52eb546552b973620ede5b5eba6b65b14fcc  frame after tick 8294 (credits-116200)
16244e24809f55ca1685095f59bd896bec67fdd362a402db9eb234bcb383b803  frame after tick 8315 (credits-116500)
a32a59d003d866c5db1b92e1df2e2285df0438c415715e8a56e111b0e9e04883  frame after tick 8344 (credits-116900)
2c172d19dd1d4d7c11439cffe8c242ef4c9d53b2cf5bb7ef03fcf19cab7d2d5b  frame after tick 8387 (credits-117500)
e92184dbc4786883202185c916cfec3ef6a76b8125fe6e86b591e0047394cb76  frame after tick 8458 (after-118000)
d9be32a0bb2c4c5cb56b83bbc079c729d923760a09cc2ea563f90fffb94e7f7e  frame after tick 8530 (after-119000)
657f69ff1e9000129fef6473e8543f8020596f52347303726a8ef39fcfecc5d1  frame after tick 8601 (after-120000)
0986c35e10dd394700a7db11cbbafba71f50a41219867fa4f74d403334c62fa4  frame after tick 8651 (end-121200)
5057bd4d15b516a7f32cc5e436b46d0348c1f3aa02dd117222c6bc2904ee0fe1  frame after tick 8672 (end-121500)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8694 (end-121800)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8723 (end-122200)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8780 (end-123000)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8851 (end-124000)
1e2e2786cf91022acf1426e0a82dd36ea556c2aff152f43c2674427746bb8fd6  frame after tick 8929 (end-125100)
ded37fa17b599c118051f025dab24ce4e618ac7ebd2cebce5b92b66a2e4c8a4e  frame after tick 8937 (end-125200)
58bfe20cc23819042164f1355af7939fec741aa26581c123a212279ed2af9120  sound of 8950 ticks
5045e30a6992de3656ed1881731431a4096885fa2fbcf79a032a8e5fd639d366  dr.cfg written last
```

<!-- write: crates/headless/tests/configure-run.sha256 -->
```text
4ac714f40a92a352946d68c4c326af9f8bf6bbdf3543ca73a486265aeb418571  frame after tick 6384 (idle)
36caa490d743f1cb9bf48aa44ecde0dce7888c76e491e1ea4db319a95f3cf71d  frame after tick 6527 (configure)
e1b0765f2d341049a7c7bb0502eb4f53dff3d95b739588d98c65e643da6a0eb1  frame after tick 6598 (music)
8337a230c66284cffc8c0d2ebf0f41c44bf414726c23d7ea3ce2208a913f8a76  frame after tick 6705 (music-left)
9cd7e344fe0030d33fc8b864a9fe53efaefbbdbc0df15cbc150f78df88d71127  frame after tick 6777 (after-music)
dd8e67880aecbc0938ef7b23300e38bb741014048c55f527bd73fa77c4e1688f  frame after tick 6884 (effects)
cb031b7698d7dbce060df5d398234a27e3ad954387abc4e74c5399b15017389d  frame after tick 6955 (effects-right)
8b295207505b66df256c8bb2a7c4e937d712ec66f43ffd9bb51ff46afc630aeb  frame after tick 7098 (keyboard)
f2c7cf3760ca6362f3c33f8d754bee260aa70f83d8c211fb9cf9fbaec0fa3147  frame after tick 7170 (press-key)
061a108f170114f15e3e0cf454fb9bc7216626d8caa791d99516cff6309fa27e  frame after tick 7241 (after-q)
1cc4e6dfd5c29e850baa390f37db004a50ebae760d44f4a814d6703a9b06428b  frame after tick 7312 (after-keyboard)
31ea5ff7012a979c195400a30c56179820b750d3aa77e2c938ee864707c82ec2  frame after tick 7419 (gamepad)
f1e115141c1d3d030bc2324f3589ba8cac7507d5940edc6a38e0eb04f7db6202  frame after tick 7491 (after-gamepad)
096627ac3b083bbabc80fadfe638e6d56e232dfa6297a450d875f56f93a8c139  frame after tick 7598 (not-detected)
94ab42936f3fd1ca01a33358b27f273d007ad34b52c501874fcfb55d87ddff6b  frame after tick 7670 (after-not-detected)
f420152c82b03db385f78bb882aeb28a9cc9ff77aef82741248824087111627b  frame after tick 7741 (escape)
0cb6fdef9d4a519e304a3fbfcdda2b1d02b32f5fdd58cbbb122b0f3bfbb8d9e7  frame after tick 7812 (previous)
6dd38a7368cb8379367c578172a0f0fed2b9ad6b1326dac3e12093e489efa4dd  sound of 7900 ticks
4fea713ed9dada86326cd89707782dbd3926d1825eb34405e76caf3c3b59fc52  dr.cfg written last
```

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (18 passed), among them `the_configure_run_matches_the_committed_manifest`.

Then compare with the original's file: `python3 -c "import hashlib,sys; d=bytearray(open('captures/menu-configure/dr.cfg','rb').read()); d[7]=0; print(hashlib.sha256(d).hexdigest())"`
Expected: `4fea713ed9dada86326cd89707782dbd3926d1825eb34405e76caf3c3b59fc52`, the `dr.cfg written last` line of `configure-run.sha256`. A new run of the original writes the same file but for the random byte: the keys change the same three settings.

- [ ] **Step 3: Commit**

```bash
git add crates/headless/tests scripts/reference/menu-configure.scenario
git commit -m "test: check Configure against the original"
```

---

### Task 5: The record and the documentation

- [ ] **Step 1: Write them**

<!-- write: docs/verification/m2b.md -->
````markdown
# M2b: verification against the original

The checks of the M2b spec (section 5), run against the original `dr.exe` under Wine as in [M2a](m2a.md). Screenshots and the original's `dr.cfg` stay under `captures/` and are never committed: the file holds the original's default names.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| Configure screens | `menu-configure` | **Pass.** All 17 shots equal one of our frames: Configure over the main menu, both volume popups before and after keys, Define Keyboard before and after Q became accelerate's key, its prompt, Define Gamepad, the "not detected" popup, Escape back to the main menu and the Configure row kept. |
| `dr.cfg` | `menu-configure` | **Pass.** The file the original wrote at the end equals ours after the same keys in all 2942 bytes but the 8th, the random byte nothing reads. |
| Defaults | the same file | **Pass.** `defaultConfig` run over `dr.exe` gives the original's fresh file: with the run's three changes (music 45 %, effects 76 %, Q for accelerate) and one start counted, every byte but the random one is equal. |

## What the original does that the spec did not say

- **Define Gamepad hides the gamepad from key reads while it waits** (0x456B00, set by 0x42CBF0): otherwise button 1 would read as Enter and button 2 as Escape, both meaning "none".
- **Only the knob's box and the percentage reach the screen** in a volume popup's turn; the slider is redrawn into the buffer only (`showAdjustOptions`; the shots agree).

## The game itself

The game binary, on its own Xvfb display with SDL's disk audio driver and its own configuration directory (`XDG_CONFIG_HOME`), went through Configure (music volume down two steps), back to the main menu and out through the exit question; it wrote `dr.cfg` there with the music at 0x7800 and one start counted, and exited by itself. No stream reached the sound server.

An earlier attempt gave a relative `XDG_CONFIG_HOME`, which the directories library ignores: that run wrote `~/.config/deadrally/dr.cfg` on the build machine. The file was new and was removed at once; the script now passes an absolute path.

## The manifest

`crates/headless/tests/configure-run.sha256` holds our frames at the 17 shots' ticks, the run's sound and the last `dr.cfg` written, whose hash equals the original's file with its random byte set to 0. `menu-run.sha256` gained the last `dr.cfg` of the M2a run.

## Runs

```
$ python3 /tmp/key-ticks.py target/release/deadrally-headless captures/menu-configure 9200
13.994 ms per tick; 17 of 17 shots match
$ deadrally-headless find --ticks 9200 $(cat captures/menu-configure/keys.args) captures/menu-configure/*.png
idle.png: ticks 6383-6384
configure.png: ticks 6527-6528
music.png: ticks 6576-6634
music-left.png: ticks 6678-6743
after-music.png: ticks 6776-6777
effects.png: ticks 6856-6920
effects-right.png: ticks 6926-6993
keyboard.png: ticks 7098-7099
press-key.png: ticks 7136-7207
after-q.png: ticks 7240-7241
after-keyboard.png: ticks 7312-7313
gamepad.png: ticks 7418-7419
after-gamepad.png: ticks 7490-7491
not-detected.png: ticks 7564-7635
after-not-detected.png: ticks 7670-7671
escape.png: ticks 7740-7741
previous.png: ticks 7812-7813
```
````

<!-- write: README.md -->
````markdown
# DeadRally

Original Death Rally reincarnation for modern systems: a clean, native, 64-bit reimplementation of *Death Rally for Windows* (Remedy, 2009) for Windows, macOS and Linux, written in Rust.

**Status:** M2b, Configure. The game starts like the original: the intro with its music and effects, the Apogee and Remedy logos, the title screen, then the main menu with Configure (volumes, keys, gamepad), the credits and the exit, under the menu music. Settings are kept in a `dr.cfg` like the original's. Nothing else is playable yet. [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) covers the goal, the approach and the roadmap.

The repository contains no game data. You need your own copy of the game: Death Rally (Classic) on Steam (free) or Remedy's 2009 freeware release.

## Quick start

```
scripts/install-linux-deps.sh               # Linux; see CONTRIBUTING.md for macOS and Windows
export DEADRALLY_DATA=~/games/DeathRally    # your copy of the game
cargo run -p deadrally-headless -- check-data
cargo run --release -p deadrally -- -window
```

[CONTRIBUTING.md](CONTRIBUTING.md) has the full setup, the data configuration and the rules.

Licence: GPL-3.0-or-later, see [LICENSE](LICENSE).
````

<!-- write: CONTRIBUTING.md -->
````markdown
# Contributing to DeadRally

Thank you for helping. Read [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) first; this file covers the practical side.

## Rules

1. **Faithfulness first.** Gameplay must match the original before anything is improved; improvements are options that default to the original behaviour.
2. **No game data in the repository**, ever: no BPA or HAF files, no executables or DLLs, no saves, sound, music, or screenshots of original art. CI rejects tracked files that match `.gitignore`.
3. **Know where code comes from.** Our code is GPL-3.0-or-later. Facts and formats from [DreeRally](https://github.com/victortrnka/DreeRally/tree/0.4.x) and [dRally](https://github.com/urxp/dRally) are welcome with credit; copied dRally code keeps its MIT notice; do not paste decompiled DreeRally code.
4. **Evidence:** every gameplay change comes with a parity log, a side-by-side screenshot, or a reference to the original's code.

## You need the original game

DeadRally ships no game data. Install *Death Rally (Classic)* from Steam (free, appid 358270), or use Remedy's 2009 freeware release. On Linux or macOS you can fetch the Windows files with steamcmd:

```
steamcmd +@sSteamCmdForcePlatformType windows +force_install_dir ~/games/DeathRally +login <steam-user> +app_update 358270 validate +quit
```

Tell DeadRally where the data is. The first of these that is set wins:

1. `--data <dir>` on the command line;
2. the `DEADRALLY_DATA` environment variable;
3. `data_path = "<dir>"` in `config.toml` in your config directory (on Linux `~/.config/deadrally/config.toml`; `check-data` prints the path on every system).

DeadRally keeps the original's settings, records and Hall of Fame in its own `dr.cfg` next to `config.toml`. When it has none, it reads the game folder's `dr.cfg` once, if there is one; it never writes into the game folder.

`<dir>` may be the folder holding `ENGINE.BPA` or Steam's `Death Rally` folder above it. Check your setup:

```
cargo run -p deadrally-headless -- check-data
```

Exit status 0 means a known release, 2 an unknown release (usable, but parity checks may differ), 1 unusable.

## Setting up

- **Rust:** install [rustup](https://rustup.rs). The toolchain version is pinned in `rust-toolchain.toml` and installs itself.
- **Linux (Debian, Ubuntu, Mint):** `scripts/install-linux-deps.sh`; add `--local` for Xvfb, the screenshot tools and Wine (for reference runs of the original).
- **macOS:** Xcode command line tools (`xcode-select --install`) and CMake (`brew install cmake`).
- **Windows:** Visual Studio Build Tools with the "Desktop development with C++" workload, and CMake.

## Build, run, test

```
cargo build --workspace
cargo run --release -p deadrally -- -window     # the game: intro, logos, title, with sound
cargo test --workspace
DEADRALLY_DATA=~/games/DeathRally cargo test-data
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

In the game: `-window` starts windowed, `-testscene` shows the M0 test scene instead (no game data needed), `--data <dir>` names the data directory; Alt+Enter toggles fullscreen, F12 toggles smoothing.

## Looking at the pictures

```
cargo run --release -p deadrally-headless -- dump-assets    # every image as PNG under dumps/
```

`dumps/` is ignored by Git. Never commit what is in it.

## Listening to the sound

```
cargo build --release -p deadrally-headless
target/release/deadrally-headless render-audio --startup --seconds 100 --out captures/startup.wav
target/release/deadrally-headless render-audio --music MEN-MUS --seconds 60 --out captures/menu.wav
target/release/deadrally-headless render-audio --effect SANIM-E --number 29 --out captures/effect.wav
```

The files are 48 kHz WAVs of the original's music and effects: keep them under `captures/`, which Git ignores.

## Checking against the original

The original `dr.exe` is the reference. On Linux, `scripts/reference-run.sh` runs it under Wine on a virtual display (no window appears, nothing reaches the speakers), presses keys and takes screenshots as a scenario file says:

```
scripts/reference-run.sh scripts/reference/startup.scenario captures/startup
cargo build --release -p deadrally-headless
target/release/deadrally-headless find captures/startup/*.png
```

`find` reports, for each screenshot, the ticks of DeadRally's startup sequence that show exactly the same picture. `docs/verification/m1a.md` lists the scenarios and what they must show. Screenshots stay under `captures/`, which Git ignores: they show the original's art.

With `--sound`, the runner also records what the original plays, from a PulseAudio null sink, and stops if the game's sound is not on that sink. `compare-audio` then says whether our render sounds the same:

```
scripts/reference-run.sh --sound scripts/reference/startup-sound.scenario captures/startup-sound
target/release/deadrally-headless render-audio --startup --seconds 122 --out captures/startup-sound/ours.wav
target/release/deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115
```

`docs/verification/m1b.md` has the scenarios and the numbers they gave.

`find` and `render-audio` press keys with `--key-at TICK:KEY` (space when no key is named), so they follow a scenario through the menus; `docs/verification/m2a.md` says how to read the ticks off a run's `run.log`.

CI does not run `cargo test-data`, because GitHub has no game data. Run it yourself when you touch data code.

## Builds from CI

GitHub artifacts lose the executable bit, so on Linux and macOS make the binary runnable first. CI builds are not signed, so macOS also blocks them until you remove the quarantine flag:

```
chmod +x deadrally
xattr -d com.apple.quarantine deadrally     # macOS only
```

## Commits and pull requests

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`; subject at most 50 characters, imperative mood.
- A body only when several things changed, as a `- ` list.
- One topic per pull request; CI must be green.
````

<!-- write: CLAUDE.md -->
```markdown
# DeadRally: instructions for AI agents

DeadRally is a clean, native reimplementation of *Death Rally* (Remedy, 2009) in Rust. Read `docs/PROJECT_BRIEF.md` for the goal and `docs/superpowers/specs/` for the current design. `CONTRIBUTING.md` has the setup.

## Ground rules (brief §2)

1. **Faithfulness first.** Anything that changes how the game plays (timings, physics, prices, AI) must match the original first. Improvements come later, as options that default to the original behaviour.
2. **Never commit game data:** BPA, HAF, the original exe or DLLs, saves, sound, music, or screenshots that are mostly original art. `.gitignore` and `scripts/check-no-game-data.sh` (run in CI) enforce this. Never `git add -f` such files.
3. **Provenance.** New code is ours (GPL-3.0-or-later). Facts, file formats and constants from DreeRally or dRally are fine: describe them in your own words and credit them. Code copied from dRally (MIT) keeps its notice. Do not paste decompiled DreeRally code; re-implement from understanding. When unsure, ask the owner.
4. **Evidence for every gameplay claim:** a parity log, a side-by-side screenshot, or a reference to the original's code (a DreeRally function with its original address).

## Determinism (`crates/core`)

- No clocks, threads, environment reads, `HashMap`/`HashSet` or libm transcendental functions: `crates/core/clippy.toml` bans them. Frontends pace ticks with `deadrally_core::host::Pacer`.
- Overflow checks are on in every profile. Write intentional wrap-around as `wrapping_*`.
- `unsafe` is forbidden in the whole workspace.
- `deadrally-core` must not depend on platform crates; CI's `core-purity` job checks it.
- Everything a frontend shares (pacing, audio gate, letterbox, stats) belongs in `deadrally_core::host`, not in a frontend.

## Commands

| Command | What it does |
|---|---|
| `cargo fmt --all` | format |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint; CI denies warnings |
| `cargo test --workspace` | tests that need no game data |
| `DEADRALLY_DATA=~/games/DeathRally cargo test-data` | tests that need the original data; they fail when it is unset |
| `cargo run -p deadrally-headless -- check-data` | where the data was found and whether it is a known release |
| `cargo run --release -p deadrally-headless -- run --ticks 7000` | determinism hashes; CI compares them across OSes |
| `cargo run --release -p deadrally -- -window` | the game: the original's startup sequence with its sound, then the main menu (`-testscene`: the M0 test scene) |
| `cargo run --release -p deadrally-headless -- dump-assets` | every catalogued image as PNG under `dumps/` (ignored) |
| `scripts/reference-run.sh scripts/reference/startup.scenario captures/startup` | screenshots of the original under Wine on a virtual display |
| `target/release/deadrally-headless find captures/startup/*.png` | the ticks of our startup sequence that match each screenshot exactly; `--key-at TICK:KEY` presses keys, `--ticks N` runs on into the menus |
| `target/release/deadrally-headless render-audio --startup --seconds 100 --out captures/startup.wav` | the startup's sound as the game plays it, the intro and then the menu music; also `--music NAME`, `--effect BANK --number K` |
| `scripts/reference-run.sh --sound scripts/reference/startup-sound.scenario captures/startup-sound` | the original's sound, recorded from a null sink (nothing reaches the speakers); `--cfg FILE` starts it with another `dr.cfg` |
| `target/release/deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115` | does our render sound like the recording; PASS or FAIL against spec M1b §5 |
| `DEADRALLY_BLESS=1 cargo test-data` | rewrite the manifests `crates/gamedata/tests/decoded-images.sha256`, `crates/headless/tests/rendered-audio.sha256`, `menu-run.sha256` and `configure-run.sha256`, only after checking the pictures and the sound against the original again |
| `scripts/spike-check.sh screens target/release/deadrally captures/x 10` | screenshots and stats without a monitor (Xvfb; sound to a file) |
| `scripts/fullscreen-check.sh target/release/deadrally captures/fs` | four fullscreen toggles on the real GPU without a monitor (headless Weston) |

## Tests

- Tests encode **why**: the name or a comment says what goes wrong for a player if the behaviour changes.
- `#[ignore]` is only for tests that need game data: `#[ignore = "needs game data (DEADRALLY_DATA)"]`. They read the data through `DEADRALLY_DATA` and fail when it is unset.
- Fixtures are generated by the tests in temporary directories. Never commit files derived from game data; hashes of decoded data are facts and may be committed.
- "Done" means verified. Say which checks ran, and say so when one could not run (CI never runs `cargo test-data`).

## Commits

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`.
- Subject at most 50 characters, imperative, English. A body only when several things changed, as a `- ` list.
- No Co-Authored-By or any other attribution.

## Working as an agent (brief §11)

- Work from a written task: goal, files, evidence required.
- One git worktree per task (under `.worktrees/`, which is ignored). Merge only after an independent review.
- Re-run the key checks yourself before reporting success. Checks that silently did not run are the most common false "done".
```

- [ ] **Step 2: Run every check**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && scripts/check-no-game-data.sh && DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (294 passed, 18 ignored; data 18 passed).

- [ ] **Step 3: Commit**

```bash
git add docs README.md CONTRIBUTING.md CLAUDE.md
git commit -m "docs: record the M2b verification"
```

- [ ] **Step 4: Final review and fixes**

Follow superpowers:executing-plans, Final Review: a fresh reviewer on the whole branch with this plan's Review Focus, then one fix pass for Critical and Important findings, each with a test that failed first.

- [ ] **Step 5: Push, wait for CI, merge**

As M2a's plan (Task 7, Steps 5 and 6) with the branch `m2b-menus` and the merge message `feat: merge M2b configure`.
