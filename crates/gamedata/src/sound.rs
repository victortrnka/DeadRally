//! The sound files in `MUSICS.BPA` (spec M1b §3.1): which entry is music and which is a bank
//! of effects, and loading them.

use std::fmt;

use crate::bpa::{Archive, BpaError};
use crate::cmf;
use crate::s3m::{Module, S3mError};
use crate::xm::{Bank, XmError};

/// The archive that holds every sound.
pub const ARCHIVE: &str = "MUSICS.BPA";

/// S3M music: the menus', every track's.
pub const MUSIC: [&str; 11] = [
    "MEN-MUS.CMF",
    "TR0-MUS.CMF",
    "TR1-MUS.CMF",
    "TR2-MUS.CMF",
    "TR3-MUS.CMF",
    "TR4-MUS.CMF",
    "TR5-MUS.CMF",
    "TR6-MUS.CMF",
    "TR7-MUS.CMF",
    "TR8-MUS.CMF",
    "TR9-MUS.CMF",
];

/// XM banks of effects: intro, end animations, races, menus.
pub const EFFECTS: [&str; 5] = [
    "SANIM-E.CMF",
    "ENDANI-E.CMF",
    "ENDANI0E.CMF",
    "GEN-EFE.CMF",
    "MEN-SAM.CMF",
];

#[derive(Debug)]
pub enum SoundError {
    Archive(BpaError),
    Music { name: String, error: S3mError },
    Effects { name: String, error: XmError },
}

impl fmt::Display for SoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoundError::Archive(error) => write!(f, "{error}"),
            SoundError::Music { name, error } => write!(f, "{ARCHIVE}/{name}: {error}"),
            SoundError::Effects { name, error } => write!(f, "{ARCHIVE}/{name}: {error}"),
        }
    }
}

impl std::error::Error for SoundError {}

/// # Errors
///
/// [`SoundError`] when the entry is missing or is not a valid S3M module.
pub fn load_music(archive: &Archive, name: &str) -> Result<Module, SoundError> {
    let bytes = archive.read(name).map_err(SoundError::Archive)?;
    Module::parse(&cmf::decode(bytes)).map_err(|error| SoundError::Music {
        name: name.to_owned(),
        error,
    })
}

/// # Errors
///
/// [`SoundError`] when the entry is missing or is not a valid XM bank.
pub fn load_effects(archive: &Archive, name: &str) -> Result<Bank, SoundError> {
    let bytes = archive.read(name).map_err(SoundError::Archive)?;
    Bank::parse(&cmf::decode(bytes)).map_err(|error| SoundError::Effects {
        name: name.to_owned(),
        error,
    })
}
