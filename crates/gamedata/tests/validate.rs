//! Validation of a data directory (spec section 6).

mod common;

use std::fs;

use common::{fake_install, fake_install_named};
use deadrally_gamedata::{Outcome, REQUIRED_FILES, ValidationError, validate};
use tempfile::tempdir;

#[test]
fn a_complete_but_unrecognised_install_is_usable_with_every_file_named() {
    // Modded or unknown releases may still run; the player must learn which files differ.
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    let validation = validate(dir.path()).unwrap();
    assert_eq!(validation.files.len(), REQUIRED_FILES.len());
    match validation.outcome {
        Outcome::Unknown { closest, differing } => {
            assert_eq!(closest, "Steam: Death Rally (Classic), appid 358270");
            assert_eq!(differing, REQUIRED_FILES.to_vec());
        }
        Outcome::Known { .. } => panic!("placeholder files cannot match a real release"),
    }
}

#[test]
fn a_missing_track_is_an_error_that_names_it_and_the_directory() {
    // Without TR5.BPA the game would crash mid-campaign on track 5; fail at start instead.
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    fs::remove_file(dir.path().join("TR5.BPA")).unwrap();
    let error = validate(dir.path()).unwrap_err();
    match &error {
        ValidationError::Unusable { missing, .. } => assert_eq!(missing, &["TR5.BPA"]),
        ValidationError::DirUnreadable { .. } => panic!("the directory is readable"),
    }
    let message = error.to_string();
    assert!(message.contains("TR5.BPA"), "{message}");
    assert!(
        message.contains(&dir.path().display().to_string()),
        "{message}"
    );
}

#[test]
fn names_match_regardless_of_case() {
    // The Steam copy mixes cases (SANIM.haf, end.bmp); other copies may be all lower case.
    let dir = tempdir().unwrap();
    fake_install_named(dir.path(), str::to_ascii_lowercase);
    let validation = validate(dir.path()).unwrap();
    assert!(validation.files[0].path.ends_with("engine.bpa"));
}

#[test]
fn two_names_differing_only_in_case_are_refused_on_case_sensitive_file_systems() {
    // Picking one at random could load the wrong data; on case-insensitive systems (Windows,
    // macOS default) the second write replaces the first, so there is only one file.
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    fs::write(dir.path().join("sanim.haf"), "a second copy").unwrap();
    let entries = fs::read_dir(dir.path()).unwrap().count();
    let case_sensitive = entries == REQUIRED_FILES.len() + 1;
    match validate(dir.path()) {
        Err(ValidationError::Unusable {
            ambiguous, missing, ..
        }) if case_sensitive => {
            assert!(missing.is_empty());
            assert_eq!(
                ambiguous,
                vec![(
                    "SANIM.HAF",
                    vec!["SANIM.HAF".to_owned(), "sanim.haf".to_owned()]
                )]
            );
        }
        Ok(_) if !case_sensitive => {}
        other => panic!("case_sensitive={case_sensitive}, got {other:?}"),
    }
}

#[test]
fn a_directory_in_place_of_a_file_is_reported_as_unreadable() {
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    fs::remove_file(dir.path().join("MENU.BPA")).unwrap();
    fs::create_dir(dir.path().join("MENU.BPA")).unwrap();
    match validate(dir.path()) {
        Err(ValidationError::Unusable {
            unreadable,
            missing,
            ..
        }) => {
            assert!(missing.is_empty());
            assert_eq!(unreadable.len(), 1);
            assert_eq!(unreadable[0].0, "MENU.BPA");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn read_only_data_validates() {
    // Steam and package managers install data read-only; validation must never need write
    // access (and must never write).
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    for name in REQUIRED_FILES {
        let path = dir.path().join(name);
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
    }
    assert!(validate(dir.path()).is_ok());
}

#[test]
fn a_missing_directory_is_an_error_that_names_it() {
    let dir = tempdir().unwrap();
    let absent = dir.path().join("no-such-dir");
    let error = validate(&absent).unwrap_err();
    assert!(matches!(error, ValidationError::DirUnreadable { .. }));
    assert!(error.to_string().contains("no-such-dir"), "{error}");
}
