use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::known_versions::{KNOWN_VERSIONS, KnownVersion, REQUIRED_FILES};

/// One required file as found on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileReport {
    /// Canonical upper-case name from [`REQUIRED_FILES`].
    pub name: &'static str,
    /// The file as named on disk.
    pub path: PathBuf,
    pub size: u64,
    /// Lower-case hex.
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Every file matches this known release.
    Known { version: &'static str },
    /// All files are present, but some match no known release. Usable, with a loud warning:
    /// the game may behave differently.
    Unknown {
        closest: &'static str,
        differing: Vec<&'static str>,
    },
}

/// A directory that holds every required file.
#[derive(Debug)]
pub struct Validation {
    pub dir: PathBuf,
    /// In [`REQUIRED_FILES`] order.
    pub files: Vec<FileReport>,
    pub outcome: Outcome,
}

#[derive(Debug)]
pub enum ValidationError {
    /// The directory itself cannot be listed (missing, not a directory, no permission).
    DirUnreadable { dir: PathBuf, source: io::Error },
    /// The game cannot run from this directory.
    Unusable {
        dir: PathBuf,
        missing: Vec<&'static str>,
        /// Several entries match one required name, differing only in case (possible on
        /// case-sensitive file systems). We refuse to guess which one is the data.
        ambiguous: Vec<(&'static str, Vec<String>)>,
        unreadable: Vec<(&'static str, io::Error)>,
    },
}

impl ValidationError {
    /// True when the directory holds none of the required files: the caller may then try the
    /// nested Steam layout.
    #[must_use]
    pub fn found_no_required_file(&self) -> bool {
        matches!(self, ValidationError::Unusable { missing, .. } if missing.len() == REQUIRED_FILES.len())
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::DirUnreadable { dir, source } => {
                write!(
                    f,
                    "cannot read the game data directory {}: {source}",
                    dir.display()
                )
            }
            ValidationError::Unusable {
                dir,
                missing,
                ambiguous,
                unreadable,
            } => {
                write!(f, "the game data in {} is unusable", dir.display())?;
                if !missing.is_empty() {
                    write!(f, "\n  missing: {}", missing.join(", "))?;
                }
                for (name, candidates) in ambiguous {
                    write!(
                        f,
                        "\n  ambiguous: {name} matches {} (names differ only in case)",
                        candidates.join(", ")
                    )?;
                }
                for (name, error) in unreadable {
                    write!(f, "\n  unreadable: {name}: {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidationError::DirUnreadable { source, .. } => Some(source),
            ValidationError::Unusable { .. } => None,
        }
    }
}

/// Checks that `dir` holds every required file and identifies the release by SHA-256.
/// Files are opened read-only; nothing is written.
///
/// # Errors
///
/// [`ValidationError`] when the directory cannot be listed or a file is missing, ambiguous or
/// unreadable.
pub fn validate(dir: &Path) -> Result<Validation, ValidationError> {
    let unreadable_dir = |source| ValidationError::DirUnreadable {
        dir: dir.to_path_buf(),
        source,
    };
    let mut by_upper_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(dir).map_err(unreadable_dir)? {
        let entry = entry.map_err(unreadable_dir)?;
        // A name that is not UTF-8 cannot be one of the data files.
        if let Some(name) = entry.file_name().to_str() {
            by_upper_name
                .entry(name.to_ascii_uppercase())
                .or_default()
                .push(name.to_owned());
        }
    }

    let mut files = Vec::new();
    let mut missing = Vec::new();
    let mut ambiguous = Vec::new();
    let mut unreadable = Vec::new();
    for name in REQUIRED_FILES {
        match by_upper_name.get(name).map(Vec::as_slice) {
            None | Some([]) => missing.push(name),
            Some([on_disk]) => {
                let path = dir.join(on_disk);
                match hash_file(&path) {
                    Ok((size, sha256)) => files.push(FileReport {
                        name,
                        path,
                        size,
                        sha256,
                    }),
                    Err(error) => unreadable.push((name, error)),
                }
            }
            Some(several) => {
                let mut candidates = several.to_vec();
                candidates.sort();
                ambiguous.push((name, candidates));
            }
        }
    }

    if !(missing.is_empty() && ambiguous.is_empty() && unreadable.is_empty()) {
        return Err(ValidationError::Unusable {
            dir: dir.to_path_buf(),
            missing,
            ambiguous,
            unreadable,
        });
    }
    let outcome = classify(&files, KNOWN_VERSIONS);
    Ok(Validation {
        dir: dir.to_path_buf(),
        files,
        outcome,
    })
}

/// Matches the files against known releases; reports the closest one when none matches fully.
fn classify(files: &[FileReport], versions: &[KnownVersion]) -> Outcome {
    let mut closest: Option<(&KnownVersion, Vec<&'static str>)> = None;
    for version in versions {
        let differing: Vec<&'static str> = files
            .iter()
            .filter(|file| {
                !version.files.iter().any(|known| {
                    known.name == file.name
                        && known.size == file.size
                        && known.sha256 == file.sha256
                })
            })
            .map(|file| file.name)
            .collect();
        if differing.is_empty() {
            return Outcome::Known {
                version: version.name,
            };
        }
        if closest
            .as_ref()
            .is_none_or(|(_, best)| differing.len() < best.len())
        {
            closest = Some((version, differing));
        }
    }
    let (version, differing) = closest.expect("KNOWN_VERSIONS is never empty");
    Outcome::Unknown {
        closest: version.name,
        differing,
    }
}

fn hash_file(path: &Path) -> io::Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut size = 0;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    let hex = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok((size, hex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KnownFile;

    const TWO_VERSIONS: &[KnownVersion] = &[
        KnownVersion {
            name: "first",
            files: &[
                KnownFile {
                    name: "TR0.BPA",
                    size: 1,
                    sha256: "aa",
                },
                KnownFile {
                    name: "TR1.BPA",
                    size: 1,
                    sha256: "bb",
                },
            ],
        },
        KnownVersion {
            name: "second",
            files: &[
                KnownFile {
                    name: "TR0.BPA",
                    size: 1,
                    sha256: "aa",
                },
                KnownFile {
                    name: "TR1.BPA",
                    size: 1,
                    sha256: "cc",
                },
            ],
        },
    ];

    fn report(name: &'static str, sha256: &str) -> FileReport {
        FileReport {
            name,
            path: PathBuf::from(name),
            size: 1,
            sha256: sha256.to_owned(),
        }
    }

    #[test]
    fn a_full_match_is_reported_as_that_version() {
        let files = [report("TR0.BPA", "aa"), report("TR1.BPA", "cc")];
        assert_eq!(
            classify(&files, TWO_VERSIONS),
            Outcome::Known { version: "second" }
        );
    }

    #[test]
    fn a_partial_match_names_the_closest_version_and_the_odd_files() {
        // The warning must point at the files that differ, so a player can re-download them.
        let files = [report("TR0.BPA", "aa"), report("TR1.BPA", "zz")];
        assert_eq!(
            classify(&files, TWO_VERSIONS),
            Outcome::Unknown {
                closest: "first",
                differing: vec!["TR1.BPA"]
            }
        );
    }

    #[test]
    fn a_size_mismatch_alone_is_not_a_match() {
        let mut file = report("TR0.BPA", "aa");
        file.size = 2;
        let files = [file, report("TR1.BPA", "bb")];
        assert!(matches!(
            classify(&files, TWO_VERSIONS),
            Outcome::Unknown { .. }
        ));
    }
}
