//! `dump-assets`: every catalogued image as a PNG, for looking through by eye (spec M1a §7).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::catalog::{self, ImageEntry, PaletteSource};
use deadrally_gamedata::image::Palette;
use deadrally_gamedata::{STEAM_SUBDIR, Validation};

use crate::rgb::Rgb;

/// Race graphics take the palette of the track they are drawn on; dumps use track 1's.
const DUMP_TRACK: (&str, &str) = ("TR1.BPA", "TR1-IMA.BPK");

/// Writes `<out>/<archive>/<entry>.png` for every catalogued image, frames side by side, and
/// returns how many it wrote.
pub fn dump_assets(validation: &Validation, out: &Path) -> Result<usize, String> {
    if is_inside(out, &validation.dir)? {
        return Err(format!(
            "{} is inside the game data directory, which is never written to",
            out.display()
        ));
    }
    // With Steam's layout the data sits in a "Death Rally" folder inside the install, and the
    // folder around it is the game's install too.
    if let Some(install) = validation.dir.parent().filter(|_| {
        validation
            .dir
            .file_name()
            .is_some_and(|name| name == STEAM_SUBDIR)
    }) && is_inside(out, install)?
    {
        return Err(format!(
            "{} is inside the game's install ({}), which is never written to",
            out.display(),
            install.display()
        ));
    }
    let mut archives = Archives {
        validation,
        open: BTreeMap::new(),
    };
    let track = catalog::find(DUMP_TRACK.0, DUMP_TRACK.1).expect("track 1 is catalogued");
    let track_palette = archives
        .entry(DUMP_TRACK.0, DUMP_TRACK.1)
        .and_then(|bytes| embedded_palette(track, bytes))?;
    for entry in catalog::IMAGES {
        let bytes = archives.entry(entry.archive, entry.name)?;
        let frames = entry
            .decode(bytes)
            .map_err(|error| format!("{}/{}: {error}", entry.archive, entry.name))?;
        let palette = match entry.palette {
            PaletteSource::Named { archive, entry } => {
                Palette::from_bytes(archives.entry(archive, entry)?)
                    .map_err(|error| format!("{archive}/{entry}: {error}"))?
            }
            PaletteSource::Track => track_palette.clone(),
            PaletteSource::Embedded => {
                embedded_palette(entry, archives.entry(entry.archive, entry.name)?)?
            }
            PaletteSource::Mask => mask_palette(),
        };
        let dir = out.join(stem(entry.archive));
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
        Rgb::strip(&frames, &palette).write_png(&dir.join(format!("{}.png", stem(entry.name))))?;
    }
    Ok(catalog::IMAGES.len())
}

/// The archives opened so far, by canonical name.
struct Archives<'a> {
    validation: &'a Validation,
    open: BTreeMap<&'static str, Archive>,
}

impl Archives<'_> {
    fn entry(&mut self, archive: &'static str, name: &str) -> Result<&[u8], String> {
        if !self.open.contains_key(archive) {
            let file = self
                .validation
                .files
                .iter()
                .find(|file| file.name == archive)
                .ok_or_else(|| format!("{archive} is not a required file"))?;
            let opened = Archive::open(&file.path).map_err(|error| error.to_string())?;
            self.open.insert(archive, opened);
        }
        self.open[archive]
            .read(name)
            .map_err(|error| error.to_string())
    }
}

fn embedded_palette(entry: &ImageEntry, bytes: &[u8]) -> Result<Palette, String> {
    let fail = |problem: String| format!("{}/{}: {problem}", entry.archive, entry.name);
    entry
        .embedded_palette(bytes)
        .map_err(|error| fail(error.to_string()))?
        .ok_or_else(|| fail("has no embedded palette".into()))
}

/// Masks are not pictures: 0 shows black, anything else white.
fn mask_palette() -> Palette {
    let mut palette = Palette([[63; 3]; 256]);
    palette.0[0] = [0; 3];
    palette
}

fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// Whether `path`, which may not exist yet, would lie inside `dir`.
fn is_inside(path: &Path, dir: &Path) -> Result<bool, String> {
    let fail = |path: &Path, error: std::io::Error| format!("{}: {error}", path.display());
    let mut existing = std::path::absolute(path).map_err(|error| fail(path, error))?;
    let mut missing: Vec<OsString> = Vec::new();
    while !existing.exists() {
        let Some(last) = existing.components().next_back() else {
            break;
        };
        missing.push(last.as_os_str().to_owned());
        if !existing.pop() {
            break;
        }
    }
    let mut resolved = existing
        .canonicalize()
        .map_err(|error| fail(&existing, error))?;
    // What does not exist yet cannot be a symbolic link, so `..` there is plain text.
    for part in missing.iter().rev() {
        if part == ".." {
            resolved.pop();
        } else {
            resolved.push(part);
        }
    }
    let dir = dir.canonicalize().map_err(|error| fail(dir, error))?;
    Ok(resolved.starts_with(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_inside_the_data_directory_is_detected_before_it_exists() {
        // Writing dumps next to the original files would mix generated art into the install.
        let data = tempfile::tempdir().unwrap();
        assert!(is_inside(&data.path().join("dumps/new"), data.path()).unwrap());
        assert!(is_inside(data.path(), data.path()).unwrap());
        let elsewhere = tempfile::tempdir().unwrap();
        assert!(!is_inside(&elsewhere.path().join("dumps"), data.path()).unwrap());
    }

    #[test]
    fn dot_dot_cannot_hide_the_data_directory() {
        let data = tempfile::tempdir().unwrap();
        std::fs::create_dir(data.path().join("sub")).unwrap();
        for sneaky in ["sub/../dumps", "missing/../dumps"] {
            assert!(
                is_inside(&data.path().join(sneaky), data.path()).unwrap(),
                "{sneaky}"
            );
        }
        let name = data.path().file_name().unwrap().to_str().unwrap();
        let around = data.path().join(format!("../missing/../{name}/dumps"));
        assert!(is_inside(&around, data.path()).unwrap());
    }
}
