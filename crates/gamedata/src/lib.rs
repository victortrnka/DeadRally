//! Finds and validates the player's copy of the original game data (spec section 6).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

mod known_versions;
mod validate;

pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
