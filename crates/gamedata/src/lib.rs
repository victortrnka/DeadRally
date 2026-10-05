//! Finds, validates and decodes the player's copy of the original game data (M0 spec section 6,
//! M1a spec section 4, M1b spec section 4.1, M2a spec section 4.1).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

pub mod assets;
pub mod bmp;
pub mod bpa;
pub mod bpk;
pub mod catalog;
pub mod cmf;
mod config;
pub mod exe;
pub mod haf;
pub mod image;
mod known_versions;
mod locate;
mod lzw;
pub mod s3m;
pub mod sound;
pub mod track;
mod validate;
pub mod xm;

pub use config::{Config, ConfigError, config_path, load_config};
pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use locate::{DATA_ENV_VAR, DataSource, LocateError, Located, STEAM_SUBDIR, locate};
pub use lzw::LzwError;
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
