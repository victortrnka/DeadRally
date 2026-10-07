use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

/// DeadRally's own settings. DeadRally writes the file only to keep the data folder chosen at
/// the first start ([`save_data_path`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// `data_path`: the directory with the original game data.
    pub data_path: Option<PathBuf>,
    /// The original's command-line options, kept here (spec M7): `window` (`-window`), `smooth`
    /// (`-smooth`), `nogl` (`-nogl`), and `vsync` (off as `-novsync`).
    pub window: Option<bool>,
    pub smooth: Option<bool>,
    pub nogl: Option<bool>,
    pub vsync: Option<bool>,
    /// Human-readable notes about keys that were ignored.
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Write {
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
            ConfigError::Write { path, source } => {
                write!(f, "cannot write config file {}: {source}", path.display())
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
            ConfigError::Read { source, .. } | ConfigError::Write { source, .. } => Some(source),
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
            key @ ("window" | "smooth" | "nogl" | "vsync") => {
                // A display option is no reason to refuse the file's `data_path`.
                let Some(on) = value.as_bool() else {
                    config.warnings.push(format!(
                        "`{key}` in {} must be true or false (ignored)",
                        path.display()
                    ));
                    continue;
                };
                let slot = match key {
                    "window" => &mut config.window,
                    "smooth" => &mut config.smooth,
                    "nogl" => &mut config.nogl,
                    _ => &mut config.vsync,
                };
                *slot = Some(on);
            }
            unknown => config.warnings.push(format!(
                "unknown key `{unknown}` in {} (ignored)",
                path.display()
            )),
        }
    }
    Ok(Some(config))
}

/// Keeps `dir` as `data_path` in the config file at `path`, creating the file and its folder
/// if need be. The file's other keys are kept, its comments are not.
///
/// # Errors
///
/// [`ConfigError`] when the file exists but cannot be read or is not TOML, or cannot be written.
pub fn save_data_path(path: &Path, dir: &Path) -> Result<(), ConfigError> {
    let write_error = |source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    };
    let mut table: toml::Table = match std::fs::read_to_string(path) {
        Ok(text) => text
            .parse()
            .map_err(|error: toml::de::Error| ConfigError::Parse {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => toml::Table::new(),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    table.insert(
        "data_path".to_owned(),
        toml::Value::String(dir.to_string_lossy().into_owned()),
    );
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(write_error)?;
    }
    std::fs::write(path, table.to_string()).map_err(write_error)
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
