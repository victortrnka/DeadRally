//! The cars' handling (spec M4c): `initParticipantValues` (0x401060) fills six tables on its
//! stack before it works out each driver's engine, grip, steering and armour for a race. The
//! tables are read here by running that code over the player's `dr.exe`, as the market's
//! prices are, so none of their values is copied.

use crate::exe::Exe;
use crate::machine::{Machine, MachineError};

/// The cars, and the four levels a car is set up for: the race's (easy, medium, hard) for the
/// opponents and a level of its own for the player.
pub const CARS: usize = 6;
pub const LEVELS: usize = 4;
/// The upgrades a car's engine, tires and armour have.
pub const UPGRADES: usize = 5;

/// The tables, each indexed as the original indexes it; a car's row is
/// `car + CARS * level`.
#[derive(Clone, Debug, PartialEq)]
pub struct HandlingTables {
    /// The engine's power, by row and engine upgrade (`row * UPGRADES + upgrade`).
    pub engine: Vec<f32>,
    /// The tires' slide, by row and tire upgrade.
    pub tires: Vec<f32>,
    /// The steering's slowness, by row.
    pub steering: Vec<f32>,
    /// The car's armour, by row; and what an armour upgrade adds, by level and upgrade.
    pub armour: Vec<i32>,
    pub armour_upgrade: Vec<i32>,
    /// How far the car slews when it turns, by car.
    pub size: Vec<f32>,
    /// The driver whose armour counts 2.2 times (0x441250, 11 bytes with the NUL), as the
    /// upper-cased name in the race is compared with it.
    pub tough: Vec<u8>,
}

/// The code that fills the tables, and the stores of the sizes' table after it.
const FILL: (u32, u32) = (0x40_1060, 0x40_1D3E);
const SIZES: (u32, u32) = (0x40_1D51, 0x40_1D79);
/// Where each table starts on the stack as the fill leaves it.
const ENGINE: u32 = 0x13C;
const TIRES: u32 = 0x31C;
const STEERING: u32 = 0xDC;
const ARMOUR: u32 = 0x7C;
const ARMOUR_UPGRADE: u32 = 0x2C;
const SIZE: u32 = 0x14;
const TOUGH: (u32, usize) = (0x44_1250, 11);

impl HandlingTables {
    /// The tables as `initParticipantValues` fills them.
    ///
    /// # Errors
    ///
    /// [`MachineError`] when the code is not the known release's.
    pub fn read(exe: &Exe) -> Result<HandlingTables, MachineError> {
        let mut machine = Machine::new(exe);
        machine.run(FILL.0, FILL.1)?;
        machine.run(SIZES.0, SIZES.1)?;
        let stack = machine.stack_pointer();
        let dwords = |offset: u32, count: usize| -> Result<Vec<u32>, MachineError> {
            let bytes = machine.bytes(stack + offset, 4 * count)?;
            Ok(bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&b| u32::from_le_bytes(b))
                .collect())
        };
        let floats = |offset, count| -> Result<Vec<f32>, MachineError> {
            Ok(dwords(offset, count)?
                .into_iter()
                .map(f32::from_bits)
                .collect())
        };
        let ints = |offset, count| -> Result<Vec<i32>, MachineError> {
            Ok(dwords(offset, count)?
                .into_iter()
                .map(|d| d as i32)
                .collect())
        };
        let rows = CARS * LEVELS;
        Ok(HandlingTables {
            engine: floats(ENGINE, rows * UPGRADES)?,
            tires: floats(TIRES, rows * UPGRADES)?,
            steering: floats(STEERING, rows)?,
            armour: ints(ARMOUR, rows)?,
            armour_upgrade: ints(ARMOUR_UPGRADE, LEVELS * UPGRADES)?,
            size: floats(SIZE, CARS)?,
            tough: machine.bytes(TOUGH.0, TOUGH.1)?,
        })
    }
}
