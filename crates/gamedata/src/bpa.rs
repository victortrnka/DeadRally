//! BPA archives, the original's container for everything except the HAF animations
//! (spec M1a §3.1).

use std::fmt;
use std::path::{Path, PathBuf};

const DIRECTORY_ENTRIES: usize = 255;
const NAME_BYTES: usize = 13;
const DIRECTORY_ENTRY_BYTES: usize = NAME_BYTES + 4;

/// Byte offset of the first entry's data: the count plus the whole fixed directory.
pub const DATA_START: usize = 4 + DIRECTORY_ENTRIES * DIRECTORY_ENTRY_BYTES;

/// A whole archive in memory (the largest original archive is 5.7 MB).
pub struct Archive {
    path: PathBuf,
    data: Vec<u8>,
    entries: Vec<Entry>,
}

/// The path and the entry count, not megabytes of data.
impl fmt::Debug for Archive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Archive")
            .field("path", &self.path)
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct Entry {
    name: String,
    start: usize,
    size: usize,
}

#[derive(Debug)]
pub enum BpaError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The header contradicts itself; `problem` says how.
    Malformed {
        path: PathBuf,
        problem: String,
    },
    NotFound {
        path: PathBuf,
        name: String,
    },
}

impl fmt::Display for BpaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BpaError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            BpaError::Malformed { path, problem } => {
                write!(
                    f,
                    "{} is not a valid BPA archive: {problem}",
                    path.display()
                )
            }
            BpaError::NotFound { path, name } => {
                write!(f, "{} has no entry {name}", path.display())
            }
        }
    }
}

impl std::error::Error for BpaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BpaError::Read { source, .. } => Some(source),
            BpaError::Malformed { .. } | BpaError::NotFound { .. } => None,
        }
    }
}

impl Archive {
    /// Reads and checks an archive. The file is only opened for reading.
    ///
    /// # Errors
    ///
    /// [`BpaError`] when the file cannot be read or its header is inconsistent.
    pub fn open(path: &Path) -> Result<Archive, BpaError> {
        let data = std::fs::read(path).map_err(|source| BpaError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Archive::from_bytes(path.to_path_buf(), data)
    }

    /// Checks an archive already in memory; `path` is only used in messages.
    ///
    /// # Errors
    ///
    /// [`BpaError::Malformed`] when the header is inconsistent.
    pub fn from_bytes(path: PathBuf, data: Vec<u8>) -> Result<Archive, BpaError> {
        let malformed = |problem: String| BpaError::Malformed {
            path: path.clone(),
            problem,
        };
        if data.len() < DATA_START {
            return Err(malformed(format!(
                "{} bytes is shorter than the {DATA_START}-byte directory",
                data.len()
            )));
        }
        let count = read_u32(&data, 0) as usize;
        if count > DIRECTORY_ENTRIES {
            return Err(malformed(format!(
                "{count} entries, at most {DIRECTORY_ENTRIES} fit"
            )));
        }
        let mut entries = Vec::with_capacity(count);
        let mut start = DATA_START;
        for index in 0..count {
            let offset = 4 + index * DIRECTORY_ENTRY_BYTES;
            let stored: [u8; NAME_BYTES] = data[offset..offset + NAME_BYTES]
                .try_into()
                .expect("13 bytes");
            if stored[NAME_BYTES - 1] != 0 {
                return Err(malformed(format!("entry {index} has an unterminated name")));
            }
            let name = decode_name(&stored);
            let size = read_u32(&data, offset + NAME_BYTES) as usize;
            entries.push(Entry { name, start, size });
            start = start
                .checked_add(size)
                .ok_or_else(|| malformed("entry sizes overflow".into()))?;
        }
        if start != data.len() {
            return Err(malformed(format!(
                "the entries add up to {} bytes of data, the file has {}",
                start - DATA_START,
                data.len() - DATA_START
            )));
        }
        Ok(Archive {
            path,
            data,
            entries,
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Entry names in directory order, as stored (upper case).
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.name.as_str())
    }

    /// An entry's bytes; the name matches regardless of ASCII case.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
            .map(|entry| &self.data[entry.start..entry.start + entry.size])
    }

    /// Like [`Archive::get`], but a missing entry is an error naming the archive.
    ///
    /// # Errors
    ///
    /// [`BpaError::NotFound`].
    pub fn read(&self, name: &str) -> Result<&[u8], BpaError> {
        self.get(name).ok_or_else(|| BpaError::NotFound {
            path: self.path.clone(),
            name: name.to_owned(),
        })
    }
}

/// Stored name bytes are `c + (117 - 3 * i)` modulo 256 for every byte before the terminator.
fn decode_name(stored: &[u8; NAME_BYTES]) -> String {
    stored
        .iter()
        .take_while(|&&byte| byte != 0)
        .enumerate()
        .map(|(index, &byte)| char::from(byte.wrapping_sub(name_key(index))))
        .collect()
}

fn name_key(index: usize) -> u8 {
    117u8.wrapping_sub(u8::try_from(3 * index).expect("index < 13"))
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().expect("4 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an archive the way the original's tool lays them out.
    fn build(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut data = vec![0u8; DATA_START];
        data[..4].copy_from_slice(&u32::try_from(entries.len()).unwrap().to_le_bytes());
        for (index, (name, bytes)) in entries.iter().enumerate() {
            let offset = 4 + index * DIRECTORY_ENTRY_BYTES;
            for (position, byte) in name.bytes().enumerate() {
                data[offset + position] = byte.wrapping_add(name_key(position));
            }
            data[offset + NAME_BYTES..offset + DIRECTORY_ENTRY_BYTES]
                .copy_from_slice(&u32::try_from(bytes.len()).unwrap().to_le_bytes());
        }
        for (_, bytes) in entries {
            data.extend_from_slice(bytes);
        }
        data
    }

    fn open(data: Vec<u8>) -> Result<Archive, BpaError> {
        Archive::from_bytes(PathBuf::from("TEST.BPA"), data)
    }

    #[test]
    fn debug_output_names_the_archive_instead_of_dumping_it() {
        // A failing assertion that prints an archive must stay readable: MENU.BPA is 3 MB.
        let archive = open(build(&[("A.BPK", &[7; 5000])])).unwrap();
        let debug = format!("{archive:?}");
        assert!(
            debug.contains("TEST.BPA") && debug.contains("entries: 1"),
            "{debug}"
        );
        assert!(debug.len() < 200, "{} characters", debug.len());
    }

    #[test]
    fn the_name_codec_matches_a_known_stored_name() {
        // "PORCHE.BPK" as stored in ENGINE.BPA.
        let stored = [
            0xc5, 0xc1, 0xc1, 0xaf, 0xb1, 0xab, 0x91, 0xa2, 0xad, 0xa5, 0, 0, 0,
        ];
        assert_eq!(decode_name(&stored), "PORCHE.BPK");
    }

    #[test]
    fn entries_follow_each_other_after_the_directory() {
        let archive = open(build(&[("A.PAL", b"abc"), ("B.BPK", b"defgh")])).unwrap();
        assert_eq!(archive.names().collect::<Vec<_>>(), ["A.PAL", "B.BPK"]);
        assert_eq!(archive.get("B.BPK"), Some(&b"defgh"[..]));
    }

    #[test]
    fn lookup_ignores_case() {
        // The game asks for "apogee.bpk"; the archive stores "APOGEE.BPK".
        let archive = open(build(&[("APOGEE.BPK", b"x")])).unwrap();
        assert_eq!(archive.get("apogee.bpk"), Some(&b"x"[..]));
    }

    #[test]
    fn a_missing_entry_names_the_archive() {
        let archive = open(build(&[("A.PAL", b"a")])).unwrap();
        let message = archive.read("MENU.PAL").unwrap_err().to_string();
        assert!(
            message.contains("TEST.BPA") && message.contains("MENU.PAL"),
            "{message}"
        );
    }

    #[test]
    fn more_than_255_entries_is_malformed() {
        let mut data = build(&[]);
        data[..4].copy_from_slice(&256u32.to_le_bytes());
        assert!(matches!(open(data), Err(BpaError::Malformed { .. })));
    }

    #[test]
    fn an_unterminated_name_is_malformed() {
        let mut data = build(&[("A.PAL", b"a")]);
        data[4 + NAME_BYTES - 1] = b'X';
        assert!(matches!(open(data), Err(BpaError::Malformed { .. })));
    }

    #[test]
    fn sizes_that_do_not_add_up_are_malformed() {
        // A truncated download must not be read as a shorter, valid archive.
        let mut data = build(&[("A.PAL", b"abcd")]);
        data.pop();
        let message = open(data).unwrap_err().to_string();
        assert!(message.contains("add up"), "{message}");
    }

    #[test]
    fn a_file_shorter_than_the_directory_is_malformed() {
        assert!(matches!(
            open(vec![0; 100]),
            Err(BpaError::Malformed { .. })
        ));
    }
}
