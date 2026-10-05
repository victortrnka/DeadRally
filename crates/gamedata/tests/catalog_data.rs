//! The decoders and the catalogue against the developer's real game data. Run with
//! `cargo test-data`; the tests read DEADRALLY_DATA and fail (never pass silently) when it is
//! unset.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::catalog::{self, Layout};
use deadrally_gamedata::haf::{Animation, FRAME_HEIGHT, FRAME_WIDTH};
use deadrally_gamedata::track::TrackInfo;
use deadrally_gamedata::{DATA_ENV_VAR, bmp, locate};
use sha2::{Digest, Sha256};

const ARCHIVES: [&str; 13] = [
    "ENGINE.BPA",
    "IBFILES.BPA",
    "MENU.BPA",
    "TR0.BPA",
    "TR1.BPA",
    "TR2.BPA",
    "TR3.BPA",
    "TR4.BPA",
    "TR5.BPA",
    "TR6.BPA",
    "TR7.BPA",
    "TR8.BPA",
    "TR9.BPA",
];

fn data_dir() -> PathBuf {
    let dir = match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    };
    locate(Some(&dir), None, None)
        .unwrap_or_else(|error| panic!("{error}"))
        .validation
        .dir
}

fn archive(name: &str) -> Archive {
    Archive::open(&data_dir().join(name)).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_archive_has_its_documented_entries() {
    // Opening checks that the sizes add up to the file; the counts catch a misread directory.
    let counts = [
        ("ENGINE.BPA", 39),
        ("IBFILES.BPA", 17),
        ("MENU.BPA", 167),
        ("MUSICS.BPA", 16),
    ];
    for (name, count) in counts {
        assert_eq!(archive(name).names().count(), count, "{name}");
    }
    assert_eq!(archive("TR0.BPA").names().count(), 12);
    for track in 1..10 {
        assert_eq!(
            archive(&format!("TR{track}.BPA")).names().count(),
            13,
            "TR{track}"
        );
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_animations_have_their_documented_frames_and_length() {
    // The intro's length in ticks is what its music (M1b) is cut to.
    for (name, frames, ticks) in [
        ("SANIM.haf", 1626, 5732),
        ("ENDANI.haf", 368, 1620),
        ("ENDANI0.HAF", 383, 1915),
    ] {
        let animation = Animation::open(&data_dir().join(name)).unwrap();
        assert_eq!(animation.len(), frames, "{name}");
        let total: u32 = animation.delays.iter().map(|&delay| u32::from(delay)).sum();
        assert_eq!(total, ticks, "{name}");
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_bpk_entry_is_catalogued_or_explained() {
    // An uncatalogued image could never be drawn or dumped; the gap would surface only when a
    // later milestone needs it.
    let mut missing = Vec::new();
    for name in ARCHIVES {
        for entry in archive(name).names() {
            let explained = catalog::NOT_IMAGES
                .iter()
                .any(|not| not.archive == name && not.name == entry);
            if entry.ends_with(".BPK") && catalog::find(name, entry).is_none() && !explained {
                missing.push(format!("{name}/{entry}"));
            }
        }
    }
    assert!(missing.is_empty(), "uncatalogued: {missing:?}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_catalogued_image_decodes_to_its_shape() {
    // A wrong width with the right total size is caught by the dump and the reference
    // screenshots; a wrong total size is caught here.
    let mut failures = Vec::new();
    for name in ARCHIVES {
        let archive = archive(name);
        for entry in catalog::IMAGES.iter().filter(|entry| entry.archive == name) {
            let bytes = archive
                .read(entry.name)
                .unwrap_or_else(|error| panic!("{error}"));
            match entry.decode(bytes) {
                Ok(frames) if frames.len() == entry.frames as usize => {}
                Ok(frames) => failures.push(format!("{}: {} frames", entry.name, frames.len())),
                Err(error) => failures.push(format!("{}: {error}", entry.name)),
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn track_images_match_their_info_files() {
    // The race (M4) takes the track size from INF.BIN; the images must agree with it.
    for track in 0..10 {
        let archive = archive(&format!("TR{track}.BPA"));
        let info = TrackInfo::parse(archive.read(&format!("TR{track}-INF.BIN")).unwrap()).unwrap();
        for part in ["IMA", "MAS", "VAI", "LR1"] {
            let entry =
                catalog::find(&format!("TR{track}.BPA"), &format!("TR{track}-{part}.BPK")).unwrap();
            assert_eq!(entry.layout, Layout::Rix3Track);
            let divisor = if matches!(part, "VAI" | "LR1") { 4 } else { 1 };
            assert_eq!(
                (entry.width, entry.height),
                (info.width / divisor, info.height / divisor),
                "TR{track}-{part}"
            );
        }
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_startup_assets_load_with_their_documented_shapes() {
    // The game refuses to start when these fail, so a wrong shape here is a broken start.
    let dir = data_dir();
    let validation = deadrally_gamedata::validate(&dir).unwrap();
    let assets = deadrally_gamedata::assets::Assets::load(&validation)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(assets.intro.len(), 1626);
    let size =
        |picture: &deadrally_gamedata::assets::Picture| (picture.image.width, picture.image.height);
    assert_eq!(size(&assets.letterbox), (320, 200));
    assert_eq!(size(&assets.apogee), (640, 480));
    assert_eq!(size(&assets.remedy), (640, 480));
    assert_eq!(size(&assets.title), (640, 480));
    assert_eq!(assets.intro_music.orders.len(), 42);
    assert_eq!(assets.intro_effects.instruments.len(), 40);
    assert_eq!(assets.menu_music.orders.len(), 94);
}

/// One line per decoded picture: the SHA-256 of its frames' pixels (palettes first where the
/// file stores them), its name and its shape as width x height x frames. The shape is part of
/// the picture: the same bytes at another width draw a skewed sprite.
fn manifest(dir: &Path) -> String {
    let mut lines = String::new();
    let mut line = |name: &str, shape: (u32, u32, usize), hasher: Sha256| {
        let hash: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let (width, height, frames) = shape;
        writeln!(lines, "{hash}  {name} {width}x{height}x{frames}").unwrap();
    };
    for name in ARCHIVES {
        let archive = archive(name);
        for entry in catalog::IMAGES.iter().filter(|entry| entry.archive == name) {
            let bytes = archive.read(entry.name).unwrap();
            let mut hasher = Sha256::new();
            if let Some(palette) = entry.embedded_palette(bytes).unwrap() {
                hasher.update(palette.0.as_flattened());
            }
            let frames = entry.decode(bytes).unwrap();
            for frame in &frames {
                hasher.update(&frame.pixels);
            }
            let shape = (frames[0].width, frames[0].height, frames.len());
            line(&format!("{name}/{}", entry.name), shape, hasher);
        }
    }
    for name in ["SANIM.haf", "ENDANI.haf", "ENDANI0.HAF"] {
        let animation = Animation::open(&dir.join(name)).unwrap();
        let mut hasher = Sha256::new();
        for index in 0..animation.len() {
            let frame = animation
                .frame(index)
                .unwrap_or_else(|error| panic!("{error}"));
            hasher.update(frame.palette.0.as_flattened());
            hasher.update(&frame.pixels);
        }
        line(name, (FRAME_WIDTH, FRAME_HEIGHT, animation.len()), hasher);
    }
    for name in ["rmd.bmp", "end.bmp"] {
        let (image, palette) = bmp::decode(&std::fs::read(dir.join(name)).unwrap()).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(palette.0.as_flattened());
        hasher.update(&image.pixels);
        line(name, (image.width, image.height, 1), hasher);
    }
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn decoded_pictures_match_the_committed_manifest() {
    // The manifest was written after the pictures were checked by eye and against the original
    // (docs/verification/m1a.md); a decoder change must not alter any of them unnoticed.
    let actual = manifest(&data_dir());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/decoded-images.sha256");
    if std::env::var_os("DEADRALLY_BLESS").is_some() {
        std::fs::write(&path, &actual).unwrap();
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
    let (new, gone) = (only_in(&actual, &expected), only_in(&expected, &actual));
    assert!(
        new.is_empty() && gone.is_empty(),
        "decoded pictures differ from {}:\ndecoded now:\n{}\nin the manifest:\n{}\nIf the \
         change is intended, check the pictures again and rewrite the manifest with \
         DEADRALLY_BLESS=1 cargo test-data",
        path.display(),
        new.join("\n"),
        gone.join("\n")
    );
}
