//! Reading DeadRally's config file (spec section 6).

use std::fs;

use deadrally_gamedata::{ConfigError, config_path, load_config, save_data_path};
use tempfile::tempdir;

#[test]
fn a_missing_config_file_is_not_an_error() {
    // Most players never write one.
    let dir = tempdir().unwrap();
    assert_eq!(load_config(&dir.path().join("config.toml")).unwrap(), None);
}

#[test]
fn malformed_toml_reports_where() {
    // Users edit this file by hand in M0; the error must point at the broken line.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "# comment\ndata_path = \"unterminated\n").unwrap();
    let error = load_config(&path).unwrap_err();
    assert!(matches!(error, ConfigError::Parse { .. }));
    assert!(error.to_string().contains("line 2"), "{error}");
}

#[test]
fn data_path_must_be_a_string() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "data_path = 42\n").unwrap();
    assert!(matches!(
        load_config(&path),
        Err(ConfigError::WrongType {
            key: "data_path",
            ..
        })
    ));
}

#[test]
fn unknown_keys_are_warnings_not_errors() {
    // Later versions add keys; an older binary must still start with a newer config.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "data_path = \"/games/dr\"\nwidescreen = true\n").unwrap();
    let config = load_config(&path).unwrap().unwrap();
    assert_eq!(
        config.data_path.as_deref(),
        Some(std::path::Path::new("/games/dr"))
    );
    assert_eq!(config.warnings.len(), 1);
    assert!(config.warnings[0].contains("widescreen"));
}

#[test]
fn a_leading_tilde_means_the_home_directory() {
    // Nothing expands `~` inside a file, but it is what people write.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "data_path = \"~/games/DeathRally\"\n").unwrap();
    let config = load_config(&path).unwrap().unwrap();
    let home = directories::BaseDirs::new()
        .unwrap()
        .home_dir()
        .to_path_buf();
    assert_eq!(config.data_path, Some(home.join("games/DeathRally")));
}

#[test]
fn the_config_file_lives_in_a_deadrally_directory() {
    let path = config_path().expect("CI machines and developers have a home directory");
    assert_eq!(path.file_name().unwrap(), "config.toml");
    let parent = path.parent().unwrap().to_string_lossy().to_lowercase();
    assert!(parent.contains("deadrally"), "{parent}");
}

#[test]
fn the_originals_display_options_can_be_kept_in_the_file() {
    // The original takes -window, -smooth and -nogl on its command line only; a player who
    // wants them every time writes them once here (spec M7).
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "window = true\nsmooth = true\nnogl = false\n").unwrap();
    let config = load_config(&path).unwrap().unwrap();
    assert_eq!(
        (config.window, config.smooth, config.nogl, config.vsync),
        (Some(true), Some(true), Some(false), None)
    );
    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
}

#[test]
fn display_options_must_be_true_or_false() {
    // "yes" silently read as off would leave the player wondering why nothing changed.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "smooth = \"yes\"\n").unwrap();
    assert!(matches!(
        load_config(&path),
        Err(ConfigError::WrongType {
            key: "smooth",
            expected: "boolean",
            ..
        })
    ));
}

#[test]
fn the_chosen_data_folder_is_written_and_the_other_settings_kept() {
    // The first start asks for the game's folder once; the next start must find it there
    // without losing what the player wrote.
    let dir = tempdir().unwrap();
    let path = dir.path().join("DeadRally").join("config.toml");
    save_data_path(&path, std::path::Path::new("/games/first")).unwrap();
    let config = load_config(&path).unwrap().unwrap();
    assert_eq!(
        config.data_path.as_deref(),
        Some(std::path::Path::new("/games/first"))
    );
    fs::write(&path, "smooth = true\ndata_path = \"/games/old\"\n").unwrap();
    save_data_path(&path, std::path::Path::new("/games/new")).unwrap();
    let config = load_config(&path).unwrap().unwrap();
    assert_eq!(
        config.data_path.as_deref(),
        Some(std::path::Path::new("/games/new"))
    );
    assert_eq!(config.smooth, Some(true));
}
