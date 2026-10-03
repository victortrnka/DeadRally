//! Choosing the data directory: precedence, no fall-through, the Steam layout (spec section 6).

mod common;

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use common::fake_install;
use deadrally_gamedata::{DataSource, LocateError, STEAM_SUBDIR, ValidationError, locate};
use tempfile::tempdir;

fn write_config(path: &Path, data_dir: &Path) {
    let value = data_dir.display().to_string().replace('\\', "\\\\");
    fs::write(path, format!("data_path = \"{value}\"\n")).unwrap();
}

#[test]
fn the_command_line_wins_over_environment_and_config() {
    let root = tempdir().unwrap();
    let (cli, env, from_config) = (
        root.path().join("cli"),
        root.path().join("env"),
        root.path().join("cfg"),
    );
    for dir in [&cli, &env, &from_config] {
        fake_install(dir);
    }
    let config = root.path().join("config.toml");
    write_config(&config, &from_config);

    let located = locate(Some(&cli), Some(env.as_os_str()), Some(&config)).unwrap();
    assert_eq!(located.source, DataSource::CommandLine);
    assert_eq!(located.validation.dir, cli);
}

#[test]
fn the_environment_wins_over_config() {
    let root = tempdir().unwrap();
    let (env, from_config) = (root.path().join("env"), root.path().join("cfg"));
    fake_install(&env);
    fake_install(&from_config);
    let config = root.path().join("config.toml");
    write_config(&config, &from_config);

    let located = locate(None, Some(env.as_os_str()), Some(&config)).unwrap();
    assert_eq!(located.source, DataSource::Environment);
    assert_eq!(located.validation.dir, env);
}

#[test]
fn the_config_file_is_used_when_nothing_else_is_given() {
    let root = tempdir().unwrap();
    let data = root.path().join("data");
    fake_install(&data);
    let config = root.path().join("config.toml");
    write_config(&config, &data);

    let located = locate(None, None, Some(&config)).unwrap();
    assert_eq!(located.source, DataSource::ConfigFile(config));
    assert_eq!(located.validation.dir, data);
}

#[test]
fn an_empty_environment_variable_counts_as_unset() {
    // `DEADRALLY_DATA= deadrally` is a common way to clear a variable for one run.
    let root = tempdir().unwrap();
    let data = root.path().join("data");
    fake_install(&data);
    let config = root.path().join("config.toml");
    write_config(&config, &data);

    let located = locate(None, Some(OsStr::new("")), Some(&config)).unwrap();
    assert!(matches!(located.source, DataSource::ConfigFile(_)));
}

#[test]
fn a_wrong_command_line_path_is_an_error_even_if_the_environment_is_valid() {
    // Silently using a different copy than the one asked for hides mistakes.
    let root = tempdir().unwrap();
    let env = root.path().join("env");
    fake_install(&env);
    let wrong = root.path().join("empty");
    fs::create_dir(&wrong).unwrap();

    match locate(Some(&wrong), Some(env.as_os_str()), None) {
        Err(LocateError::Invalid {
            source: DataSource::CommandLine,
            error,
        }) => {
            assert!(error.found_no_required_file());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_steam_folder_one_level_up_is_accepted() {
    // Players will point at steamapps/common/Death Rally, which holds the data one level down.
    let root = tempdir().unwrap();
    let nested = root.path().join(STEAM_SUBDIR);
    fake_install(&nested);

    let located = locate(Some(root.path()), None, None).unwrap();
    assert_eq!(located.validation.dir, nested);
}

#[test]
fn a_partial_install_is_not_rescued_by_the_steam_subfolder() {
    // If the given directory holds some data files, it is the data directory and its missing
    // files are the error to report.
    let root = tempdir().unwrap();
    fake_install(root.path());
    fs::remove_file(root.path().join("TR0.BPA")).unwrap();
    fake_install(&root.path().join(STEAM_SUBDIR));

    match locate(Some(root.path()), None, None) {
        Err(LocateError::Invalid {
            error: ValidationError::Unusable { missing, .. },
            ..
        }) => {
            assert_eq!(missing, ["TR0.BPA"]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn nothing_specified_names_all_three_sources_and_the_config_path() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    let error = locate(None, None, Some(&config)).unwrap_err();
    let message = error.to_string();
    for needle in [
        "--data",
        "DEADRALLY_DATA",
        "data_path",
        &config.display().to_string(),
    ] {
        assert!(message.contains(needle), "missing {needle:?} in {message}");
    }
}

#[test]
fn a_broken_config_is_an_error_not_a_silent_fallback() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    fs::write(&config, "data_path = \n").unwrap();
    assert!(matches!(
        locate(None, None, Some(&config)),
        Err(LocateError::Config(_))
    ));
}

#[test]
fn config_warnings_are_passed_on() {
    let root = tempdir().unwrap();
    let data = root.path().join("data");
    fake_install(&data);
    let config = root.path().join("config.toml");
    let value = data.display().to_string().replace('\\', "\\\\");
    fs::write(
        &config,
        format!("data_path = \"{value}\"\nfullscreen = true\n"),
    )
    .unwrap();

    let located = locate(None, None, Some(&config)).unwrap();
    assert_eq!(located.config_warnings.len(), 1);
    assert!(located.config_warnings[0].contains("fullscreen"));
}
