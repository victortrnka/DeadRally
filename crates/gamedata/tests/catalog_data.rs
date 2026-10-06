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
    let menu = &assets.menu;
    assert_eq!((menu.background.width, menu.background.height), (640, 480));
    assert_eq!((menu.panel_line.width, menu.panel_line.height), (640, 10));
    assert_eq!(
        (menu.corners_focused.len(), menu.corners_unfocused.len()),
        (4, 4)
    );
    assert_eq!(menu.cursor.len(), 50);
    for font in [
        &menu.big_a,
        &menu.big_b,
        &menu.big_d,
        &menu.small_a,
        &menu.small_b,
        &menu.small_c,
    ] {
        assert_eq!(font.len(), 96);
    }
    assert_eq!(menu.background_copper.len(), 512);
    assert_eq!(menu.credits.len(), 2);
    assert_eq!(size(&menu.end), (640, 480));
    assert_eq!(menu.effects.instruments.len(), 31);
    // Row counts of the main menu and the start submenu, from dr.exe.
    let rows = |m: usize| {
        menu.texts.menus[m]
            .iter()
            .filter(|row| !row.is_empty())
            .count()
    };
    assert_eq!((rows(0), rows(1)), (6, 6));
    assert_eq!(menu.texts.panel.len(), 4);
    assert_eq!((menu.texts.big.width, menu.texts.small.height), (32, 16));
    assert_eq!((menu.slider.width, menu.slider.height), (172, 24));
    assert_eq!((menu.knob.width, menu.knob.height), (10, 24));
    let configure = &menu.texts.configure;
    assert_eq!(
        (configure.key_names.len(), configure.pad_names.len()),
        (256, 9)
    );
    // The race's help names 101 keys (0x407330 fills its table for them, as running the
    // original's code shows) and the gamepad's nine inputs.
    let help = &menu.texts.help;
    let named = help
        .key_names
        .iter()
        .filter(|name| !name.is_empty())
        .count();
    assert_eq!(
        (named, help.pad_names.len(), help.controls.len()),
        (101, 9, 8)
    );
    // The defaults dr.exe's defaultConfig writes, as a fresh dr.cfg of the original holds
    // them (docs/verification/m2b.md): A, Z, the arrows, left shift, left control, left alt
    // and space; button 1, down, left, right, buttons 2 to 4; gamepad off.
    let defaults = &menu.default_config;
    assert_eq!(
        (defaults.music_volume(), defaults.effects_volume()),
        (0x8000, 0xC000)
    );
    assert_eq!(
        (0..8).map(|c| defaults.key(c)).collect::<Vec<_>>(),
        [0x1E, 0x2C, 0xCB, 0xCD, 0x2A, 0x1D, 0x38, 0x39]
    );
    assert_eq!(
        (0..7).map(|c| defaults.pad(c)).collect::<Vec<_>>(),
        [5, 4, 1, 2, 6, 7, 8]
    );
    assert_eq!((defaults.use_joystick(), defaults.times_played()), (0, 0));
    // The Hall of Fame's pictures and names (spec M2c section 3).
    assert_eq!(menu.medium.len(), 62);
    assert_eq!(
        (
            menu.fame_title.height,
            menu.records_title.height,
            menu.records_bar.height
        ),
        (54, 16, 68)
    );
    assert_eq!(
        (
            menu.snapshots.len(),
            menu.arrows.len(),
            menu.wipe.len(),
            menu.border_corners.len()
        ),
        (20, 4, 10, 4)
    );
    let hall = &menu.texts.hall_of_fame;
    assert_eq!((hall.circuits.len(), hall.cars.len()), (18, 6));
    assert_eq!(hall.circuit_order[..3], [0, 7, 5]);
    // The Underground Market's prices by car, as dr.exe's setUndergroundMarketPrices
    // (0x421FB0) sets them when run: a wrong price changes what a weapon costs the player.
    assert_eq!(
        menu.market_prices,
        [
            [150, 200, 275, 250],
            [200, 225, 350, 325],
            [450, 500, 675, 550],
            [500, 550, 625, 570],
            [1250, 1750, 2250, 2125],
            [2525, 2750, 3275, 2625],
        ]
    );
    assert_eq!((menu.weapons.len(), menu.loan_shark.width), (12, 96));
    // The cars' handling tables, as initParticipantValues (0x401060) fills them when run; the
    // values agree with DreeRally's copy of them. A wrong value changes how a car drives.
    let handling = &assets.race.handling;
    assert_eq!(handling.size, [8.3, 9.7, 9.0, 10.5, 8.5, 9.2]);
    assert_eq!((handling.steering[0], handling.steering[23]), (1.8, 1.35));
    assert_eq!(
        (handling.engine[0], handling.engine[1], handling.engine[119]),
        (2.55, 2.6, 4.2)
    );
    assert_eq!((handling.tires[0], handling.tires[119]), (0.5, 0.0));
    assert_eq!((handling.armour[0], handling.armour[23]), (120, 400));
    assert_eq!(
        (handling.armour_upgrade[0], handling.armour_upgrade[19]),
        (360, 440)
    );
    let guns: Vec<_> = handling
        .guns
        .iter()
        .map(|g| (g.count, g.angle, g.reach, g.flash))
        .collect();
    assert_eq!(
        guns,
        [
            (1, [22, 0], [8, 0], [0, 0]),
            (1, [-18, 0], [17, 0], [1, 0]),
            (1, [-40, 0], [7, 0], [2, 0]),
            (2, [16, -17], [20, 20], [3, 3]),
            (2, [16, -17], [19, 19], [4, 4]),
            (2, [16, -17], [20, 20], [5, 5]),
        ]
    );
    assert_eq!(handling.gun_damage, [0.2, 0.35, 0.5, 0.65, 0.8, 0.95, 0.95]);
    assert_eq!(
        handling.balance,
        [
            0.07, 0.12, 0.11, 0.2, 0.18, 0.32, 0.12, 0.19, 0.06, 0.12, 0.03, 0.06
        ]
    );
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

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_track_loads_with_its_scene_and_shadows() {
    // A race on any of the ten tracks needs its picture, mask, lights, shadows and 3D scene;
    // one that fails to load would leave the player at the stand-in race.
    for number in 0..10 {
        let archive = archive(&format!("TR{number}.BPA"));
        let track = deadrally_gamedata::race::Track::load(&archive, number)
            .unwrap_or_else(|error| panic!("TR{number}: {error}"));
        let mut colours = std::collections::BTreeMap::new();
        for triangle in track.scene.objects.iter().flat_map(|o| &o.triangles) {
            *colours.entry(triangle.colour).or_insert(0) += 1;
        }
        println!(
            "TR{number}: {} objects, {} textures, {} shadows, colours {colours:?}",
            track.scene.objects.len(),
            track.scene.textures.len(),
            track.shadows.triangles.len()
        );
        assert!(!track.scene.objects.is_empty(), "TR{number} has no scene");
    }
}
