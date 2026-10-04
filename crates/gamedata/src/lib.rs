//! Finds, validates and decodes the player's copy of the original game data (M0 spec section 6,
//! M1a spec section 4).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

pub mod bmp;
pub mod bpa;
pub mod bpk;
pub mod catalog;
mod config;
pub mod haf;
pub mod image;
mod known_versions;
mod locate;
mod lzw;
pub mod track;
mod validate;

pub use config::{Config, ConfigError, config_path, load_config};
pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use locate::{DATA_ENV_VAR, DataSource, LocateError, Located, STEAM_SUBDIR, locate};
pub use lzw::LzwError;
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
