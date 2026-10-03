//! Finds and validates the player's copy of the original game data (spec section 6).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

mod config;
mod known_versions;
mod locate;
mod validate;

pub use config::{Config, ConfigError, config_path, load_config};
pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use locate::{DATA_ENV_VAR, DataSource, LocateError, Located, STEAM_SUBDIR, locate};
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
