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
    /// `balanceIAEngineInRace`'s (0x40B920) twelve fractions: by level, what an opponent a
    /// zone and two zones behind the player gains (`2 * level` and `2 * level + 1`) and, from
    /// 6 on, what one ahead loses.
    pub balance: Vec<f32>,
    /// Each car's machine guns (`initParticipantValues` 0x401EC7): their count, and for each
    /// its angle off the car's, its reach and its muzzle flash's kind.
    pub guns: Vec<Guns>,
    /// What a hit from each car's guns takes off, times the target's armour short of 1024
    /// (0x4A6AE0, set at 0x401D79).
    pub gun_damage: Vec<f32>,
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
/// The stores of the balance's fractions on the stack (0x40B92A on), the first 4 bytes up.
const BALANCE: (u32, u32) = (0x40_B92A, 0x40_B98A);
const BALANCE_FRACTIONS: usize = 12;

/// A car's machine guns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Guns {
    pub count: i32,
    pub angle: [i32; 2],
    pub reach: [i32; 2],
    pub flash: [i32; 2],
}

/// The guns' setting-up for a car (eax its number, ebp the car's record), and where its
/// fields lie in the record.
const GUNS: (u32, u32) = (0x40_1E9F, 0x40_1F99);
const GUN_RECORD: u32 = 0x0010_4000;
const GUN_SCRATCH: u32 = 0x0010_5000;
const GUN_FIELDS: (u32, u32, u32, u32) = (0x40, 0x48, 0x58, 0x68);
/// The stores of the guns' damage factors, seven floats from 0x4A6AE0.
const GUN_DAMAGE: (u32, u32, u32) = (0x40_1D79, 0x40_1DBF, 0x4A_6AE0);

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
            guns: (0..CARS as u32)
                .map(|car| {
                    let mut guns = Machine::new(exe);
                    // eax the car, edx 0; ebx a driver's record, read but not used here.
                    guns.set_register(0, car);
                    guns.set_register(2, 0);
                    guns.set_register(3, GUN_SCRATCH);
                    guns.set_register(5, GUN_RECORD);
                    for offset in (0..0x40).step_by(4) {
                        guns.poke(GUN_SCRATCH - 0x40 + offset, 0);
                    }
                    for offset in (0..0xA0).step_by(4) {
                        guns.poke(GUN_RECORD - 0x20 + offset, 0);
                    }
                    guns.run(GUNS.0, GUNS.1)?;
                    let int = |offset: u32| -> Result<i32, MachineError> {
                        let b = guns.bytes(GUN_RECORD + offset, 4)?;
                        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                    };
                    let (count, angle, reach, flash) = GUN_FIELDS;
                    Ok(Guns {
                        count: int(count)?,
                        angle: [int(angle)?, int(angle + 4)?],
                        reach: [int(reach)?, int(reach + 4)?],
                        flash: [int(flash)?, int(flash + 4)?],
                    })
                })
                .collect::<Result<_, MachineError>>()?,
            gun_damage: {
                let mut stores = Machine::new(exe);
                stores.run(GUN_DAMAGE.0, GUN_DAMAGE.1)?;
                stores
                    .bytes(GUN_DAMAGE.2, 4 * 7)?
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&b| f32::from_le_bytes(b))
                    .collect()
            },
            balance: {
                let mut balance = Machine::new(exe);
                balance.run(BALANCE.0, BALANCE.1)?;
                let bytes = balance.bytes(balance.stack_pointer() + 4, 4 * BALANCE_FRACTIONS)?;
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&b| f32::from_le_bytes(b))
                    .collect()
            },
        })
    }
}
