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

    /// Sets register `index` (eax, ecx, edx, ebx, esp, ebp, esi, edi) before a run, as the
    /// code's caller would have left it.
    pub fn set_register(&mut self, index: usize, value: u32) {
        self.registers[index] = value;
    }

    /// The stack pointer as the code left it, for the locals of a function stopped before
    /// its end.
    pub fn stack_pointer(&self) -> u32 {
        self.registers[4]
    }

    /// Sets the 32-bit value at `address` before a run, as the original's globals would hold
    /// it.
    pub fn poke(&mut self, address: u32, value: u32) {
        self.write(address, 4, value);
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
            // imul r, r/m, imm8.
            0x6B => {
                let (reg, place, next) = self.modrm(at_op + 1)?;
                let factor = i32::from(self.byte(next)? as i8);
                let (product, overflow) = (self.get(place, 4)? as i32).overflowing_mul(factor);
                self.set(Place::Register(reg), 4, product as u32);
                self.flags(product as u32, overflow);
                next + 1
            }
            // test r/m, r.
            0x85 => {
                let (reg, place, next) = self.modrm(at_op + 1)?;
                let value = self.get(place, 4)? & self.get(Place::Register(reg), 4)?;
                self.flags(value, false);
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
            // add, sub, cmp and xor, between a register and a register or memory.
            0x01 | 0x03 | 0x29 | 0x2B | 0x31 | 0x33 | 0x39 | 0x3B => {
                let (reg, place, next) = self.modrm(at_op + 1)?;
                let (target, source) = if opcode & 2 == 0 {
                    (place, Place::Register(reg))
                } else {
                    (Place::Register(reg), place)
                };
                let (a, b) = (self.get(target, 4)?, self.get(source, 4)?);
                let (result, overflow) = match opcode & 0xF8 {
                    0x00 => (
                        a.wrapping_add(b),
                        (a as i32).checked_add(b as i32).is_none(),
                    ),
                    0x28 | 0x38 => (
                        a.wrapping_sub(b),
                        (a as i32).checked_sub(b as i32).is_none(),
                    ),
                    _ => (a ^ b, false),
                };
                if opcode & 0xF8 != 0x38 {
                    self.set(target, 4, result);
                }
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

    #[test]
    fn a_price_table_indexed_by_a_poked_record_is_read_as_the_original_computes_it() {
        // setUndergroundMarketPrices (0x421FB0) finds the player's car through imul and
        // tests it with test; the market's prices come from running it.
        let code = [
            0xA1, 0x00, 0x11, 0x40, 0x00, // mov eax, [0x401100] (the driver)
            0x6B, 0xC0, 0x08, // imul eax, eax, 8
            0x8B, 0x80, 0x10, 0x11, 0x40, 0x00, // mov eax, [eax + 0x401110] (the car)
            0x85, 0xC0, // test eax, eax
            0x75, 0x0A, // jne +10
            0xC7, 0x05, 0x40, 0x11, 0x40, 0x00, 0x96, 0x00, 0x00, 0x00, // mov [0x401140], 150
            0xC3, // ret
        ];
        let image = exe(&code, &[0; 0x100]);
        let mut machine = Machine::new(&image);
        machine.poke(0x40_1100, 3);
        machine.poke(0x40_1110 + 24, 0);
        machine.run(0x40_1000, 0x40_1100).unwrap();
        assert_eq!(
            machine.bytes(0x40_1140, 4).unwrap(),
            [150, 0, 0, 0],
            "car 0's price"
        );
        let mut machine = Machine::new(&image);
        machine.poke(0x40_1100, 3);
        machine.poke(0x40_1110 + 24, 2);
        machine.run(0x40_1000, 0x40_1100).unwrap();
        assert_eq!(
            machine.bytes(0x40_1140, 4).unwrap(),
            [0, 0, 0, 0],
            "another car jumps"
        );
    }
}
