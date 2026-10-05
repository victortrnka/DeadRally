//! What the tests against the developer's real game data share: finding the data, hashing,
//! and the committed manifests they check.

use std::path::{Path, PathBuf};

use deadrally_gamedata::{DATA_ENV_VAR, Located, locate};
use sha2::{Digest, Sha256};

/// The data DEADRALLY_DATA points at; fails (never passes silently) when it is unset.
pub fn located() -> Located {
    let dir = match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    };
    locate(Some(&dir), None, None).unwrap_or_else(|error| panic!("{error}"))
}

pub fn hash(samples: &[i16], hasher: &mut Sha256) {
    for sample in samples {
        hasher.update(sample.to_le_bytes());
    }
}

pub fn hex(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Checks `actual` against the committed manifest `name` in tests/, line by line, or rewrites
/// the manifest when DEADRALLY_BLESS is set. `what` names what the manifest pins.
pub fn check_manifest(name: &str, actual: &str, what: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
    if std::env::var_os("DEADRALLY_BLESS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    let only_in = |these: &str, those: &str| -> Vec<String> {
        these
            .lines()
            .filter(|line| !those.lines().any(|other| other == *line))
            .map(str::to_owned)
            .collect()
    };
    let (new, gone) = (only_in(actual, &expected), only_in(&expected, actual));
    assert!(
        new.is_empty() && gone.is_empty(),
        "{what} differs from {}:\nnow:\n{}\nin the manifest:\n{}\nIf the change is intended, \
         measure it against the original again and rewrite the manifest with \
         DEADRALLY_BLESS=1 cargo test-data",
        path.display(),
        new.join("\n"),
        gone.join("\n")
    );
}
