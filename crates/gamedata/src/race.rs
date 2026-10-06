//! What a race reads (spec M4 §3): the tracks' archives `TR0.BPA`..`TR9.BPA`, `ENGINE.BPA`
//! and `IBFILES.BPA`, kept whole and decoded when a race starts, and the track decoded from
//! its archive (`loadCircuitInfFile` 0x409BF0, `loadCircuitPalette` 0x402CF0,
//! `loadCircuitImages1` 0x402EE0).

use std::fmt;

use crate::bpa::{Archive, BpaError};
use crate::image::{Image, Palette};
use crate::track::{self, TrackError, TrackInfo};

/// The race's archives.
#[derive(Debug)]
pub struct RaceArchives {
    /// `TR0.BPA` to `TR9.BPA` by number.
    pub tracks: Vec<Archive>,
    pub engine: Archive,
    pub ib_files: Archive,
}

#[derive(Debug)]
pub enum RaceError {
    Bpa(BpaError),
    Track { name: String, error: TrackError },
}

impl fmt::Display for RaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RaceError::Bpa(error) => write!(f, "{error}"),
            RaceError::Track { name, error } => write!(f, "{name}: {error}"),
        }
    }
}

impl std::error::Error for RaceError {}

impl From<BpaError> for RaceError {
    fn from(error: BpaError) -> RaceError {
        RaceError::Bpa(error)
    }
}

/// A track: its `-INF.BIN`, its picture `-IMA` with the palette in it, its surface mask
/// `-MAS` (the low nibble a surface's kind), and `-LIT.TAB`, the colour each colour turns in
/// the cars' headlights.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub info: TrackInfo,
    pub image: Image,
    pub palette: Palette,
    pub mask: Image,
    pub lit: [u8; 256],
}

impl Track {
    /// The track `TRn` from its archive.
    ///
    /// # Errors
    ///
    /// [`RaceError`] naming the entry that is missing or does not decode.
    pub fn load(archive: &Archive, number: usize) -> Result<Track, RaceError> {
        let name = |suffix: &str| format!("TR{number}-{suffix}");
        let decode = |suffix: &str, width: u32, height: u32| {
            let entry = name(suffix);
            track::decode_rix3(archive.read(&entry)?, width, height)
                .map_err(|error| RaceError::Track { name: entry, error })
        };
        let info_name = name("INF.BIN");
        let info =
            TrackInfo::parse(archive.read(&info_name)?).map_err(|error| RaceError::Track {
                name: info_name,
                error,
            })?;
        let (image, palette) = decode("IMA.BPK", info.width, info.height)?;
        let (mask, _) = decode("MAS.BPK", info.width, info.height)?;
        // Read into a table of 256 as the original reads it (0x4A9EE0).
        let mut lit = [0; 256];
        let table = archive.read(&name("LIT.TAB"))?;
        let len = table.len().min(256);
        lit[..len].copy_from_slice(&table[..len]);
        Ok(Track {
            info,
            image,
            palette,
            mask,
            lit,
        })
    }
}
