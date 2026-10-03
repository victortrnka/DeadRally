//! Tests against the developer's real game data. Run with `cargo test-data`; they read the
//! directory from DEADRALLY_DATA and fail (never pass silently) when it is unset.

use std::path::PathBuf;

use deadrally_gamedata::{DATA_ENV_VAR, Outcome, locate};

fn data_dir() -> PathBuf {
    match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_developers_install_is_a_known_release() {
    // Parity work assumes the data we compare against is exactly a release we know.
    let located = locate(Some(&data_dir()), None, None).unwrap_or_else(|error| panic!("{error}"));
    match located.validation.outcome {
        Outcome::Known { version } => println!("recognised: {version}"),
        Outcome::Unknown { closest, differing } => {
            panic!("unknown release (closest: {closest}); differing: {differing:?}")
        }
    }
}
