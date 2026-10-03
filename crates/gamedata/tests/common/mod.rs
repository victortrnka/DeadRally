//! Fixtures: fake installs in temporary directories. Real game data is never committed.

use std::fs;
use std::path::Path;

use deadrally_gamedata::REQUIRED_FILES;

/// Writes every required file into `dir` with placeholder contents, so the set is complete
/// but matches no known release.
pub fn fake_install(dir: &Path) {
    fake_install_named(dir, |name| name.to_owned());
}

/// Like [`fake_install`], with each canonical name mapped through `rename` first.
pub fn fake_install_named(dir: &Path, rename: impl Fn(&str) -> String) {
    fs::create_dir_all(dir).unwrap();
    for name in REQUIRED_FILES {
        fs::write(dir.join(rename(name)), format!("placeholder for {name}")).unwrap();
    }
}
