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
