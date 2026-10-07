use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::config::{Config, ConfigError, load_config};
use crate::validate::{Validation, ValidationError, validate};

/// Environment variable naming the game data directory.
pub const DATA_ENV_VAR: &str = "DEADRALLY_DATA";

/// Steam installs the data one level down: `steamapps/common/Death Rally/Death Rally/`.
pub const STEAM_SUBDIR: &str = "Death Rally";

/// Where the data directory came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataSource {
    CommandLine,
    Environment,
    ConfigFile(PathBuf),
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataSource::CommandLine => write!(f, "command line (--data)"),
            DataSource::Environment => write!(f, "environment ({DATA_ENV_VAR})"),
            DataSource::ConfigFile(path) => write!(f, "config file ({})", path.display()),
        }
    }
}

#[derive(Debug)]
pub struct Located {
    pub source: DataSource,
    pub validation: Validation,
    /// Ignored config keys, worth showing to the user.
    pub config_warnings: Vec<String>,
}

#[derive(Debug)]
pub enum LocateError {
    /// No source names a directory. `config_path` is where the config file would be.
    NotSpecified {
        config_path: Option<PathBuf>,
    },
    Config(ConfigError),
    /// The chosen source names a directory that does not hold usable data. Lower-precedence
    /// sources are not tried: a wrong explicit path must not be silently replaced.
    Invalid {
        source: DataSource,
        error: ValidationError,
    },
}

impl fmt::Display for LocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocateError::NotSpecified { config_path } => {
                write!(
                    f,
                    "no game data directory given: pass --data <dir>, set {DATA_ENV_VAR}, or set data_path in "
                )?;
                match config_path {
                    Some(path) => write!(f, "{}", path.display()),
                    None => write!(f, "the config file (this system has no config directory)"),
                }
            }
            LocateError::Config(error) => write!(f, "{error}"),
            LocateError::Invalid { source, error } => {
                write!(f, "game data from the {source}: {error}")
            }
        }
    }
}

impl std::error::Error for LocateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LocateError::NotSpecified { .. } => None,
            LocateError::Config(error) => Some(error),
            LocateError::Invalid { error, .. } => Some(error),
        }
    }
}

/// Finds and validates the game data. The first source that is *specified* wins, in this
/// order: `cli` (`--data`), `env` (the value of `DEADRALLY_DATA`; empty counts as unset), then
/// `data_path` in the config file at `config_path`. The config file is read only when the
/// first two are absent.
///
/// # Errors
///
/// [`LocateError`] when no source is specified, the config file is broken, or the chosen
/// directory does not hold usable data.
pub fn locate(
    cli: Option<&Path>,
    env: Option<&OsStr>,
    config_path: Option<&Path>,
) -> Result<Located, LocateError> {
    let (source, dir, config_warnings) = if let Some(dir) = cli {
        (DataSource::CommandLine, dir.to_path_buf(), Vec::new())
    } else if let Some(dir) = env.filter(|value| !value.is_empty()) {
        (DataSource::Environment, PathBuf::from(dir), Vec::new())
    } else {
        let not_specified = || LocateError::NotSpecified {
            config_path: config_path.map(Path::to_path_buf),
        };
        let path = config_path.ok_or_else(not_specified)?;
        match load_config(path).map_err(LocateError::Config)? {
            Some(Config {
                data_path: Some(dir),
                warnings,
                ..
            }) => (DataSource::ConfigFile(path.to_path_buf()), dir, warnings),
            Some(Config {
                data_path: None, ..
            })
            | None => return Err(not_specified()),
        }
    };
    let validation = validate_with_steam_fallback(&dir).map_err(|error| LocateError::Invalid {
        source: source.clone(),
        error,
    })?;
    Ok(Located {
        source,
        validation,
        config_warnings,
    })
}

/// Validates `dir`; if it holds none of the files but has a `Death Rally` subdirectory, that
/// subdirectory is validated instead.
fn validate_with_steam_fallback(dir: &Path) -> Result<Validation, ValidationError> {
    match validate(dir) {
        Err(error) if error.found_no_required_file() && dir.join(STEAM_SUBDIR).is_dir() => {
            validate(&dir.join(STEAM_SUBDIR))
        }
        result => result,
    }
}
