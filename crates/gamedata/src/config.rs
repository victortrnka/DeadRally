use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

/// DeadRally's own settings. M0 knows one key; DeadRally only reads the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// `data_path`: the directory with the original game data.
    pub data_path: Option<PathBuf>,
    /// Human-readable notes about keys that were ignored.
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    /// Not valid TOML; `message` includes the line and column.
    Parse {
        path: PathBuf,
        message: String,
    },
    WrongType {
        path: PathBuf,
        key: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(f, "cannot read config file {}: {source}", path.display())
            }
            ConfigError::Parse { path, message } => {
                write!(
                    f,
                    "config file {} is not valid TOML: {message}",
                    path.display()
                )
            }
            ConfigError::WrongType {
                path,
                key,
                expected,
            } => {
                write!(
                    f,
                    "config file {}: `{key}` must be a {expected}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse { .. } | ConfigError::WrongType { .. } => None,
        }
    }
}

/// Where this platform keeps DeadRally's `config.toml`, or `None` when it has no config
/// directory (no home directory, for example).
#[must_use]
pub fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "DeadRally").map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Reads the config file. A file that does not exist is not an error: `Ok(None)`.
///
/// # Errors
///
/// [`ConfigError`] when the file exists but cannot be read, is not TOML, or `data_path` is not
/// a string.
pub fn load_config(path: &Path) -> Result<Option<Config>, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let table: toml::Table = text
        .parse()
        .map_err(|error: toml::de::Error| ConfigError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    let mut config = Config::default();
    for (key, value) in &table {
        match key.as_str() {
            "data_path" => match value.as_str() {
                Some(data_path) => config.data_path = Some(expand_home(data_path)),
                None => {
                    return Err(ConfigError::WrongType {
                        path: path.to_path_buf(),
                        key: "data_path",
                        expected: "string",
                    });
                }
            },
            unknown => config.warnings.push(format!(
                "unknown key `{unknown}` in {} (ignored)",
                path.display()
            )),
        }
    }
    Ok(Some(config))
}

/// Expands a leading `~`. The shell does this for `--data` and `DEADRALLY_DATA`, but nothing
/// does it inside a file, and `~` is the natural thing to write there.
fn expand_home(value: &str) -> PathBuf {
    let rest = if value == "~" {
        Some("")
    } else {
        value
            .strip_prefix("~/")
            .or_else(|| value.strip_prefix("~\\"))
    };
    match (rest, BaseDirs::new()) {
        (Some(rest), Some(base)) => base.home_dir().join(rest),
        _ => PathBuf::from(value),
    }
}
