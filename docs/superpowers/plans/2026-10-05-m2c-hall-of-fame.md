# M2c Hall of Fame Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The main menu's Hall of Fame shows the original's best ten and records by circuit, with its wipes and music, from the player's `dr.cfg`, matching the original pixel for pixel. This completes M2.

**Architecture:** `deadrally-gamedata` reads the circuits', cars' and difficulties' names and the circuit order from `dr.exe`, the six pictures and the medium font, and the records' and best ten's fields of `dr.cfg`. `deadrally-core` gains a masked copy for the wipe, jumping the music to an order, and the Hall of Fame's states in the menu scene.

**Tech Stack:** Rust 1.99.0 (edition 2024); no new crates. Reference runs as in M2a.

**Spec:** `docs/superpowers/specs/2026-10-05-m2c-hall-of-fame-design.md`. Read it first.

## Global Constraints

As in M2b's plan (`docs/superpowers/plans/2026-10-05-m2b-configure.md`): toolchain, no `unsafe`, overflow checks, no new dependencies or lint exceptions, the core's determinism bans, never quote the game's text, never commit game data or captures, nothing reaches the speakers, commits as before. Fixed values: the wipe is 43 steps of 15 pixels with 22 × 10 tiles from offset 48000 and a 150 × 330 window; the music mask falls from 65532 by 1524 a step; the Hall of Fame plays from order 81 at mask 0x10000; arrows stay lit 8 waits; records are 24 bytes at `circuit + 18 * car`, the best ten 20 bytes each.

## Review Focus

1. **A `dr.cfg` with odd records or best ten** (names without a NUL, negative or huge races, a difficulty past 3, huge times) → drawn without panicking. Task 2's code reads names within their field and difficulties with `get`.
2. **Keys during a wipe** → remembered and read after it, as the original's straight-line code does. Task 2.
3. **The menu music comes back** at the order it had, at full mask. Task 2, `the_hall_of_fame_plays_its_own_music_and_the_menu_music_comes_back`.
4. **The best ten's names are written upper-case** after being shown, as the original's `_strupr` in place. Task 2, `the_best_tens_names_are_written_upper_case_once_shown`.
5. **Left and Right wrap** through the original's circuit order. Task 2, `the_records_step_through_the_circuits_in_the_originals_order`.

## Before You Start

As in M2b's plan, on branch `m2c-hall-of-fame` (spec and plan committed).

---

### Task 1: The Hall of Fame's data

**Files:**
- Modify: `crates/gamedata/src/{text,assets,dr_cfg}.rs`, `crates/gamedata/tests/catalog_data.rs`, `crates/core/src/menu/draw.rs` (test fixture), `crates/core/tests/common/mod.rs`

**Interfaces:**
- Produces: `text::{HallOfFameTexts, CIRCUITS, CARS}`, `Texts { hall_of_fame, .. }`; `MenuAssets { medium, fame_title, records_title, records_bar, snapshots, arrows, wipe, border_corners, .. }`; `DrCfg::{record(circuit, car), hall_of_fame(rank), upper_case_hall_of_fame}`, `HALL_OF_FAME_ENTRIES`.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/gamedata/tests/catalog_data.rs -->
```rust
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
```

<!-- write: crates/core/src/menu/draw.rs -->
```rust
//! The menu screen's drawing, as `dr.exe` does it into its screen buffer and copies to the
//! shown buffer (spec M2a §3.3, §3.4): popups (`createPopup` 0x41A530), menus (`drawMenu`
//! 0x41A880), the cursor (`updateCursor` 0x41AB50), the highlight's moves (`refreshMenuUp`
//! 0x41AF40, `refreshMenuDown` 0x41B1A0, 0x41ACF0) and the bottom panel (0x41A7A0, 0x41E810).

use deadrally_gamedata::assets::MenuAssets;
use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::Texts;

use crate::canvas::{Canvas, at};
use crate::font::Font;

/// The fill colour of popups and of the cursor's box.
pub(crate) const POPUP_FILL: u8 = 0xC4;
/// Popup lines: a focused popup's, an unfocused one's.
const LINE_FOCUSED: u8 = 7;
const LINE_UNFOCUSED: u8 = 4;
/// Corner pictures are 32x20, the cursor 20x20, a big glyph 32 high.
const CORNER_WIDTH: usize = 32;
const CORNER_HEIGHT: usize = 20;
const CURSOR_SIZE: usize = 20;
pub(crate) const CURSOR_FRAMES: usize = 50;

/// One menu of the table at 0x4456F0 and its active rows (0x4457F0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuTable {
    /// Which menu of `dr.exe`'s text table its rows are.
    pub(crate) text: usize,
    pub(crate) rows: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) row_height: usize,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) selected: usize,
    pub(crate) active: [bool; 9],
}

/// The main menu: start, multiplayer (inactive), configure, hall of fame, credits, exit.
pub(crate) const MAIN_MENU: MenuTable = MenuTable {
    text: 0,
    rows: 6,
    x: 145,
    y: 124,
    row_height: 28,
    width: 349,
    height: 192,
    selected: 0,
    active: [true, false, true, true, true, true, false, false, false],
};

/// The start submenu at the first start: rows 0, 3 and 5 active.
pub(crate) const START_MENU: MenuTable = MenuTable {
    text: 1,
    rows: 6,
    x: 109,
    y: 171,
    row_height: 28,
    width: 421,
    height: 192,
    selected: 0,
    active: [true, false, false, true, false, true, false, false, false],
};

/// Configure (menu 3): music volume, effect volume, define keyboard, define gamepad, the
/// gamepad switch, previous menu.
pub(crate) const CONFIGURE_MENU: MenuTable = MenuTable {
    text: 3,
    rows: 6,
    x: 95,
    y: 146,
    row_height: 28,
    width: 485,
    height: 192,
    selected: 0,
    active: [true, true, true, true, true, true, false, false, false],
};

/// Define Keyboard (menu 6): the eight controls and previous menu.
pub(crate) const KEYBOARD_MENU: MenuTable = MenuTable {
    text: 6,
    rows: 9,
    x: 50,
    y: 93,
    row_height: 28,
    width: 532,
    height: 278,
    selected: 0,
    active: [true; 9],
};

/// Define Gamepad (menu 8): seven controls (no horn) and previous menu.
pub(crate) const PAD_MENU: MenuTable = MenuTable {
    text: 8,
    rows: 8,
    x: 50,
    y: 113,
    row_height: 28,
    width: 532,
    height: 250,
    selected: 0,
    active: [true, true, true, true, true, true, true, true, false],
};

/// How a menu is drawn: unfocused (mode 0) or focused (mode 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Unfocused,
    Focused,
}

/// The menu's pictures, fonts and rows.
#[derive(Clone, Debug)]
pub(crate) struct Graphics {
    pub(crate) background: Image,
    panel_line: Image,
    corners_focused: Vec<Image>,
    corners_unfocused: Vec<Image>,
    cursor: Vec<Image>,
    pub(crate) big_a: Font,
    pub(crate) big_b: Font,
    pub(crate) big_d: Font,
    pub(crate) small: [Font; 3],
    /// `dr.exe`'s menu text table: `menus[m][r]` is row `r` of menu `m`. The original rewrites
    /// some rows as settings change.
    menus: Vec<Vec<Vec<u8>>>,
    /// The volume popups' slider and its knob.
    pub(crate) slider: Image,
    pub(crate) knob: Image,
}

impl Graphics {
    pub(crate) fn new(assets: &MenuAssets) -> Graphics {
        let texts = &assets.texts;
        Graphics {
            background: assets.background.clone(),
            panel_line: assets.panel_line.clone(),
            corners_focused: assets.corners_focused.clone(),
            corners_unfocused: assets.corners_unfocused.clone(),
            cursor: assets.cursor.clone(),
            big_a: Font::new(assets.big_a.clone(), &texts.big),
            big_b: Font::new(assets.big_b.clone(), &texts.big),
            big_d: Font::new(assets.big_d.clone(), &texts.big),
            small: [
                Font::new(assets.small_a.clone(), &texts.small),
                Font::new(assets.small_b.clone(), &texts.small),
                Font::new(assets.small_c.clone(), &texts.small),
            ],
            menus: texts.menus.clone(),
            slider: assets.slider.clone(),
            knob: assets.knob.clone(),
        }
    }

    /// Rewrites row `row` of menu `menu`, as the original copies a setting's text into its
    /// table.
    pub(crate) fn set_row(&mut self, menu: usize, row: usize, text: Vec<u8>) {
        self.menus[menu][row] = text;
    }

    pub(crate) fn cursor(&self, frame: usize) -> &Image {
        &self.cursor[frame % self.cursor.len()]
    }

    /// `createPopup(x, y, w, h, focus)`: fill, corners, then lines; nothing outside is
    /// cleared, so a popup drawn over another blends their corners.
    pub(crate) fn popup(
        &self,
        screen: &mut Canvas,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        focus: Focus,
    ) {
        if h > 8 {
            screen.fill(at(x + 2, y + 2), w - 6, h - 8, POPUP_FILL);
        }
        let (corners, line) = match focus {
            Focus::Unfocused => (&self.corners_unfocused, LINE_UNFOCUSED),
            Focus::Focused => (&self.corners_focused, LINE_FOCUSED),
        };
        let right = x + w - CORNER_WIDTH;
        let bottom = y + h - CORNER_HEIGHT;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            screen.draw(corner, offset, true);
        }
        if w > 64 {
            screen.fill(at(x + 32, y + 1), w - 64, 1, line);
            screen.fill(at(x + 32, y + h - 7), w - 64, 1, line);
        }
        if h > 40 {
            screen.fill(at(x + 1, y + 20), 1, h - 40, line);
            screen.fill(at(x + w - 5, y + 20), 1, h - 40, line);
        }
    }

    /// `drawMenu(menu, focus)`: its popup and rows; the selected row with the cursor when the
    /// menu has focus. Nothing reaches the shown buffer.
    pub(crate) fn menu(&self, screen: &mut Canvas, menu: &MenuTable, focus: Focus, cursor: usize) {
        self.popup(screen, menu.x, menu.y, menu.width, menu.height, focus);
        for row in 0..menu.rows {
            let text = &self.menus[menu.text][row];
            let at_text = at(menu.x + 32, menu.y + 5 + row * menu.row_height);
            let font = if row == menu.selected {
                if focus == Focus::Focused {
                    screen.draw(self.cursor(cursor), self.cursor_at(menu), true);
                    &self.big_a
                } else {
                    &self.big_d
                }
            } else if menu.active[row] && focus == Focus::Focused {
                &self.big_b
            } else {
                &self.big_d
            };
            font.draw(screen, text, at_text);
        }
    }

    /// Where the selected row's cursor goes.
    fn cursor_at(&self, menu: &MenuTable) -> usize {
        at(menu.x + 9, menu.y + 11 + menu.selected * menu.row_height)
    }

    /// `updateCursor`: the cursor's box refilled, frame `frame` drawn, the box copied to the
    /// shown buffer.
    pub(crate) fn update_cursor(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &MenuTable,
        frame: usize,
    ) {
        let offset = self.cursor_at(menu);
        screen.fill(offset, CURSOR_SIZE, CURSOR_SIZE, POPUP_FILL);
        screen.draw(self.cursor(frame), offset, true);
        shown.copy_from(screen, offset, CURSOR_SIZE, CURSOR_SIZE);
    }

    /// Moves the highlight to row `to` as `refreshMenuUp`/`refreshMenuDown` and 0x41ACF0 do:
    /// both rows' areas refilled and redrawn, the cursor drawn with frame `frame`, both
    /// copied to the shown buffer. `base` is 6 for Up and the jump to the last row, 5 for Down.
    pub(crate) fn move_highlight(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &mut MenuTable,
        to: usize,
        base: usize,
        frame: usize,
    ) {
        let rows_at = |row: usize| menu.y + base + row * menu.row_height;
        let text_at = |row: usize| at(menu.x + 32, menu.y + 5 + row * menu.row_height);
        let old = menu.selected;
        screen.fill(
            at(menu.x + 9, rows_at(old) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_b
            .draw(screen, &self.menus[menu.text][old], text_at(old));
        menu.selected = to;
        screen.fill(
            at(menu.x + 9, rows_at(to) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_a
            .draw(screen, &self.menus[menu.text][to], text_at(to));
        screen.draw(self.cursor(frame), self.cursor_at(menu), true);
        shown.copy_from(screen, at(menu.x + 7, rows_at(old)), menu.width - 10, 32);
        shown.copy_from(screen, at(menu.x + 7, rows_at(to)), menu.width - 10, 32);
    }

    /// `drawTransparentBlock(x, y, w, h)`: the background restored, then the panel's two
    /// frame lines.
    pub(crate) fn panel_frame(&self, screen: &mut Canvas, x: usize, y: usize, w: usize, h: usize) {
        screen.restore(&self.background, at(x + 2, y - 4), w - 6, h);
        screen.draw(&self.panel_line, at(0, y + 1), true);
        screen.draw(&self.panel_line, at(0, y + h - 9), true);
    }

    /// `drawBottomMenuText`: rows 380..=468 restored, then the panel's last six lines at
    /// (12, 378 + 15k), each in its own small font.
    pub(crate) fn panel_text(&self, screen: &mut Canvas, panel: &Panel) {
        screen.copy_rows(&self.background, 380, 89);
        for (k, line) in panel.lines[16..].iter().enumerate() {
            if let Some(font) = self.small.get(usize::from(line.font)) {
                font.draw(screen, &line.text, at(12, 378 + 15 * k));
            }
        }
    }
}

/// One line of the bottom panel and its font (0, 1, 2: small A, B, C).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: Vec<u8>,
    pub(crate) font: u8,
}

/// The bottom message panel: 22 lines, new ones pushed in at the bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Panel {
    lines: Vec<PanelLine>,
}

impl Panel {
    /// The panel as `mainMenu` fills it at start-up: the four start-up lines in small B, an
    /// empty line before the last.
    pub(crate) fn startup(texts: &Texts) -> Panel {
        let mut panel = Panel {
            lines: vec![PanelLine::default(); 22],
        };
        let [first, second, third, last] = [0, 1, 2, 3].map(|i| texts.panel[i].clone());
        for text in [first, second, third, Vec::new(), last] {
            panel.push(text, 1);
        }
        panel
    }

    fn push(&mut self, text: Vec<u8>, font: u8) {
        self.lines.remove(0);
        self.lines.push(PanelLine { text, font });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::{HEIGHT, WIDTH};

    /// Graphics where every picture has its own colour: corners 11..=14 (focused) and 21..=24,
    /// cursor frame k colour 100 + k with a transparent top-left pixel, glyphs as in
    /// `font::tests::font` but 32x32 (big) or 16x16 (small) and colour 50 + font.
    pub(crate) fn graphics() -> Graphics {
        let solid = |w: u32, h: u32, colour: u8| Image::new(w, h, vec![colour; (w * h) as usize]);
        let glyphs = |size: u32, colour: u8| {
            (0..96)
                .map(|_| solid(size, size, colour))
                .collect::<Vec<_>>()
        };
        let metrics = |size: u8| deadrally_gamedata::text::Metrics {
            width: size,
            height: size,
            advances: vec![size; 96],
        };
        Graphics {
            background: Image::new(
                640,
                480,
                (0..WIDTH * HEIGHT).map(|i| (i % 7) as u8 + 1).collect(),
            ),
            panel_line: solid(640, 10, 99),
            corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
            corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
            cursor: (0..50)
                .map(|k| {
                    let mut frame = solid(20, 20, 100 + k);
                    frame.pixels[0] = 0;
                    frame
                })
                .collect(),
            big_a: Font::new(glyphs(32, 50), &metrics(32)),
            big_b: Font::new(glyphs(32, 51), &metrics(32)),
            big_d: Font::new(glyphs(32, 52), &metrics(32)),
            small: [
                Font::new(glyphs(16, 60), &metrics(16)),
                Font::new(glyphs(16, 61), &metrics(16)),
                Font::new(glyphs(16, 62), &metrics(16)),
            ],
            menus: texts().menus,
            slider: solid(172, 24, 70),
            knob: solid(10, 24, 71),
        }
    }

    pub(crate) fn texts() -> Texts {
        let metrics = deadrally_gamedata::text::Metrics {
            width: 32,
            height: 32,
            advances: vec![32; 96],
        };
        Texts {
            menus: (0..9)
                .map(|m| {
                    (0..9)
                        .map(|r| {
                            if r < 6 {
                                vec![b'A' + m as u8]
                            } else {
                                Vec::new()
                            }
                        })
                        .collect()
                })
                .collect(),
            panel: (0..4).map(|i| vec![b'a' + i]).collect(),
            exit_question: b"?".to_vec(),
            yes: b"Y".to_vec(),
            no: b"N".to_vec(),
            big: metrics.clone(),
            small: metrics.clone(),
            medium: metrics,
            configure: configure_texts(),
            hall_of_fame: deadrally_gamedata::text::HallOfFameTexts {
                circuits: vec![b"C".to_vec(); 18],
                cars: vec![b"V".to_vec(); 6],
                difficulties: vec![b"D".to_vec(); 4],
                circuit_order: (0..18).collect(),
            },
        }
    }

    /// Configure's texts: one letter each, `k` for key and pad names.
    pub(crate) fn configure_texts() -> deadrally_gamedata::text::ConfigureTexts {
        let one = |c: u8| vec![c];
        deadrally_gamedata::text::ConfigureTexts {
            adjust_music: one(b'm'),
            adjust_effects: one(b'e'),
            gamepad_on: one(b'+'),
            gamepad_off: one(b'-'),
            not_detected: one(b'!'),
            press_any_key: one(b'.'),
            controls: (0..8).map(|i| one(b'0' + i)).collect(),
            key_prompts: (0..8).map(|_| one(b'k')).collect(),
            pad_prompts: (0..7).map(|_| one(b'p')).collect(),
            key_names: (0..256).map(|_| one(b'k')).collect(),
            pad_names: (0..9).map(|_| one(b'p')).collect(),
        }
    }

    #[test]
    fn a_popup_fills_exactly_w_minus_6_columns() {
        // DreeRally fills two columns short on the main menu; the original fills x+2..=x+w-5.
        let mut screen = Canvas::default();
        graphics().popup(&mut screen, 145, 124, 349, 192, Focus::Focused);
        let p = screen.pixels();
        assert_eq!(p[at(147, 200)], POPUP_FILL);
        assert_eq!(
            p[at(489 - 1, 200)],
            POPUP_FILL,
            "x + w - 5 is the right line"
        );
        assert_eq!(p[at(489, 200)], LINE_FOCUSED);
        assert_eq!(p[at(146, 200)], LINE_FOCUSED, "left line at x + 1");
        assert_eq!(p[at(490, 200)], 0, "the shadow columns are left alone");
        assert_eq!(p[at(177, 125)], LINE_FOCUSED, "top line from x + 32");
        assert_eq!(
            p[at(461, 309)],
            LINE_FOCUSED,
            "bottom line at y + h - 7 to x + w - 33"
        );
        assert_eq!(p[at(145, 124)], 11, "top-left corner");
        assert_eq!(p[at(462, 124)], 12, "top-right corner");
        assert_eq!(p[at(145, 296)], 13, "bottom-left corner");
        assert_eq!(p[at(462 + 31, 296 + 19)], 14, "bottom-right corner");
    }

    #[test]
    fn a_focused_menu_shows_the_cursor_and_its_rows_in_three_fonts() {
        let mut screen = Canvas::default();
        let mut menu = MAIN_MENU;
        menu.selected = 2;
        graphics().menu(&mut screen, &menu, Focus::Focused, 7);
        let p = screen.pixels();
        let text_row = |row: usize| p[at(177, 129 + 28 * row)];
        assert_eq!(text_row(2), 50, "selected: big A");
        assert_eq!(text_row(0), 51, "active: big B");
        assert_eq!(text_row(1), 52, "inactive: big D");
        assert_eq!(
            p[at(155, 135 + 56)],
            107,
            "cursor frame 7 at (x + 9, y + 11 + 28 * 2)"
        );
        let mut dim = Canvas::default();
        graphics().menu(&mut dim, &menu, Focus::Unfocused, 7);
        assert_eq!(
            dim.pixels()[at(177, 129 + 56)],
            52,
            "unfocused: everything big D"
        );
        assert_eq!(dim.pixels()[at(155, 191)], POPUP_FILL, "no cursor");
        assert_eq!(dim.pixels()[at(145, 124)], 21, "unfocused corners");
    }

    #[test]
    fn the_cursor_update_copies_its_box_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        graphics().update_cursor(&mut screen, &mut shown, &MAIN_MENU, 3);
        assert_eq!(
            shown.pixels()[at(154, 135)],
            POPUP_FILL,
            "transparent pixel over the fill"
        );
        assert_eq!(shown.pixels()[at(155, 135)], 103);
        assert_eq!(shown.pixels()[at(174, 135)], 0, "only the 20x20 box");
    }

    #[test]
    fn moving_the_highlight_redraws_both_rows_and_copies_both_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        let mut menu = MAIN_MENU;
        graphics().move_highlight(&mut screen, &mut shown, &mut menu, 2, 5, 9);
        assert_eq!(menu.selected, 2);
        let s = shown.pixels();
        assert_eq!(s[at(177, 129)], 51, "the old row in big B");
        assert_eq!(s[at(177, 129 + 56)], 50, "the new row in big A");
        assert_eq!(s[at(155, 135 + 56)], 109, "cursor frame 9");
        assert_eq!(s[at(152, 129 + 28)], 0, "row 1 is not copied");
    }

    #[test]
    fn the_panel_shows_its_last_six_lines_with_the_startup_text_in_small_b() {
        let panel = Panel::startup(&texts());
        let mut screen = Canvas::default();
        graphics().panel_text(&mut screen, &panel);
        let p = screen.pixels();
        assert_eq!(
            p[at(12, 380)],
            graphics().background.pixels[at(12, 380)],
            "line 16 is empty; the restore starts at row 380"
        );
        for k in [1, 2, 3, 5] {
            assert_eq!(p[at(12, 378 + 15 * k)], 61, "line {}: small B", 16 + k);
        }
        assert_eq!(
            p[at(12, 445)],
            graphics().background.pixels[at(12, 445)],
            "line 20 is empty (the glyphs above reach row 438)"
        );
    }
}
```

<!-- write: crates/core/tests/common/mod.rs -->
```rust
//! Synthetic menu assets: every picture is one colour of its own, so a test can tell from a
//! pixel which picture, font or cursor frame the menu drew there. Real game data is never
//! committed.

use deadrally_gamedata::assets::{MenuAssets, Picture};
use deadrally_gamedata::dr_cfg::{DrCfg, HEADER_BYTES, PAYLOAD_BYTES};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::text::{ConfigureTexts, HallOfFameTexts, Metrics, Texts};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// `MENUBG5`, white in `MENU.PAL`.
pub const BACKGROUND: u8 = 1;
/// The big fonts: selected row (A), active row (B), dim row (D).
pub const BIG_A: u8 = 50;
pub const BIG_B: u8 = 51;
pub const BIG_D: u8 = 52;
/// The small fonts A, B and C.
pub const SMALL: [u8; 3] = [60, 61, 62];
/// Cursor frame k is colour `CURSOR + k`.
pub const CURSOR: u8 = 100;
/// The two credits screens and the end screen, each full red, green or blue in its palette.
pub const CREDITS: [u8; 2] = [200, 201];
pub const END: u8 = 202;
/// The menu's effects (`MEN-SAM`): the back sound plays on the left only, the move sound on
/// the right only, the choose sound on both sides.
pub const BACK_SOUND: usize = 22;
pub const MOVE_SOUND: usize = 25;
pub const CHOOSE_SOUND: usize = 28;
/// The volume popups' slider and knob.
pub const SLIDER: u8 = 70;
pub const KNOB: u8 = 71;
/// The Hall of Fame: the medium font, the titles, the record bar, circuit c's snapshot
/// (`SNAPSHOT + c`), the four arrows (`ARROW + k`) and the border's corners.
pub const MEDIUM: u8 = 72;
pub const FAME_TITLE: u8 = 73;
pub const RECORDS_TITLE: u8 = 74;
pub const RECORDS_BAR: u8 = 75;
pub const SNAPSHOT: u8 = 150;
pub const ARROW: u8 = 180;
pub const BORDER: u8 = 190;

/// A `dr.cfg` with the original's default volumes, gamepad off.
pub fn config() -> DrCfg {
    let mut config = DrCfg::parse(&[0; HEADER_BYTES + PAYLOAD_BYTES]).unwrap();
    config.set_music_volume(0x8000);
    config.set_effects_volume(0xC000);
    config
}

fn solid(width: u32, height: u32, colour: u8) -> Image {
    Image::new(width, height, vec![colour; (width * height) as usize])
}

fn glyphs(size: u32, colour: u8) -> Vec<Image> {
    (0..96).map(|_| solid(size, size, colour)).collect()
}

fn full_screen(colour: u8, rgb: [u8; 3]) -> Picture {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(colour)] = rgb;
    Picture {
        image: solid(640, 480, colour),
        palette,
    }
}

/// A short tone, panned (255 left, 0 right, 128 both).
fn sound(panning: u8) -> Instrument {
    Instrument {
        name: "Beep".into(),
        data: vec![2000; 2000],
        looping: Looping::None,
        volume: 64,
        finetune: 0,
        relative_note: 0,
        panning,
        fadeout: 0,
    }
}

fn texts() -> Texts {
    let metrics = |size: u8| Metrics {
        width: size,
        height: size,
        advances: vec![size; 96],
    };
    Texts {
        // Every menu has six one-letter rows.
        menus: vec![
            (0..9)
                .map(|row| if row < 6 { b"M".to_vec() } else { Vec::new() })
                .collect();
            9
        ],
        panel: (0..4).map(|line| vec![b'a' + line]).collect(),
        exit_question: b"?".to_vec(),
        yes: b"Y".to_vec(),
        no: b"N".to_vec(),
        big: metrics(32),
        small: metrics(16),
        medium: metrics(9),
        configure: ConfigureTexts {
            adjust_music: b"m".to_vec(),
            adjust_effects: b"e".to_vec(),
            gamepad_on: b"+".to_vec(),
            gamepad_off: b"-".to_vec(),
            not_detected: b"!".to_vec(),
            press_any_key: b".".to_vec(),
            controls: vec![b"c".to_vec(); 8],
            key_prompts: vec![b"k".to_vec(); 8],
            pad_prompts: vec![b"p".to_vec(); 7],
            key_names: vec![b"K".to_vec(); 256],
            pad_names: vec![b"P".to_vec(); 9],
        },
        hall_of_fame: HallOfFameTexts {
            circuits: vec![b"C".to_vec(); 18],
            cars: vec![b"V".to_vec(); 6],
            difficulties: vec![b"D".to_vec(); 4],
            // The original's order starts 0, 7, 5.
            circuit_order: vec![0, 7, 5, 3, 4, 2, 8, 1, 6, 9, 16, 14, 12, 13, 11, 17, 10, 15],
        },
    }
}

pub fn menu_assets() -> MenuAssets {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(BACKGROUND)] = [63, 63, 63];
    // The pulsing entries.
    palette.0[16..32].fill([63, 63, 63]);
    let mut copper = Palette::BLACK;
    copper.0[0] = [63, 0, 32];
    let mut effects = vec![None; CHOOSE_SOUND];
    effects[BACK_SOUND - 1] = Some(sound(255));
    effects[MOVE_SOUND - 1] = Some(sound(0));
    effects[CHOOSE_SOUND - 1] = Some(sound(128));
    MenuAssets {
        background: solid(640, 480, BACKGROUND),
        panel_line: solid(640, 10, 99),
        corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
        corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
        cursor: (0..50).map(|k| solid(20, 20, CURSOR + k)).collect(),
        big_a: glyphs(32, BIG_A),
        big_b: glyphs(32, BIG_B),
        big_d: glyphs(32, BIG_D),
        small_a: glyphs(16, SMALL[0]),
        small_b: glyphs(16, SMALL[1]),
        small_c: glyphs(16, SMALL[2]),
        palette,
        copper,
        background_copper: (0..512).map(|row| [(row % 64) as u8, 0, 0]).collect(),
        credits: vec![
            full_screen(CREDITS[0], [63, 0, 0]),
            full_screen(CREDITS[1], [0, 63, 0]),
        ],
        end: full_screen(END, [0, 0, 63]),
        effects: Bank {
            linear_frequencies: true,
            instruments: effects,
        },
        texts: texts(),
        slider: solid(172, 24, SLIDER),
        knob: solid(10, 24, KNOB),
        default_config: config(),
        medium: glyphs(9, MEDIUM),
        fame_title: solid(640, 54, FAME_TITLE),
        records_title: solid(640, 16, RECORDS_TITLE),
        records_bar: solid(640, 68, RECORDS_BAR),
        snapshots: (0..20).map(|c| solid(128, 112, SNAPSHOT + c)).collect(),
        arrows: (0..4).map(|k| solid(16, 84, ARROW + k)).collect(),
        // Frame k covers its tile's rows from k on: frame 0 all of it, so every column is
        // covered once the band's first tile column has passed over it.
        wipe: (0..10u32)
            .map(|k| {
                let pixels = (0..225).map(|i| u8::from(i / 15 >= k)).collect();
                Image::new(15, 15, pixels)
            })
            .collect(),
        border_corners: (0..4).map(|_| solid(24, 24, BORDER)).collect(),
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --workspace --no-run`
Expected: FAIL to compile (no `HallOfFameTexts`; `MenuAssets` has no `medium`).

- [ ] **Step 3: Implement**

<!-- write: crates/gamedata/src/text.rs -->
```rust
//! The original's menu strings and font metrics, read from the player's `dr.exe` at the known
//! release's addresses (spec M2a §3.1). DeadRally ships none of the game's text.

use std::fmt;

use crate::exe::{Exe, ExeError};

/// Menus in the text table, and rows per menu.
pub const MENUS: usize = 9;
pub const MENU_ROWS: usize = 9;
/// The byte that draws nothing and moves the pen one pixel.
pub const GAP: u8 = 0xFA;

/// The menu text table: 50 bytes per row (`dr.exe` 0x446368).
const MENU_TABLE: u32 = 0x44_6368;
const MENU_ROW_BYTES: u32 = 50;
/// The bottom panel's start-up lines, in the order `mainMenu` (0x43A020) adds them; an empty
/// line comes between the third and the fourth.
const PANEL_LINES: [u32; 4] = [0x44_4370, 0x44_433C, 0x44_4300, 0x44_42C0];
const EXIT_QUESTION: u32 = 0x44_42B0;
const YES: u32 = 0x44_3CDC;
const NO: u32 = 0x44_3CD8;
/// Font descriptors: width, height, then one advance per glyph from character 32.
const BIG_METRICS: u32 = 0x44_5848;
const SMALL_METRICS: u32 = 0x44_58B0;
const MEDIUM_METRICS: u32 = 0x44_5928;
/// The longest string read anywhere but the menu table.
const MAX_LINE: usize = 150;
/// The fonts' cell sizes in the known release: big, small and medium.
const BIG_SIZE: (u8, u8) = (32, 32);
const SMALL_SIZE: (u8, u8) = (16, 16);
const MEDIUM_SIZE: (u8, u8) = (9, 12);
/// The main menu (0) and the start submenu (1) show six rows each, all of them text.
const SHOWN_MENUS: usize = 2;
const SHOWN_ROWS: usize = 6;

/// Configure's popups (spec M2b §3.2): the volume captions, the gamepad switch's two texts and
/// the popup when no gamepad is found.
const ADJUST_MUSIC: u32 = 0x44_3F90;
const ADJUST_EFFECTS: u32 = 0x44_3F6C;
const GAMEPAD_ON: u32 = 0x44_3F50;
const GAMEPAD_OFF: u32 = 0x44_3F34;
const NOT_DETECTED: u32 = 0x44_3170;
const PRESS_ANY_KEY: u32 = 0x44_29A4;
/// The eight controls' names (accelerate, brake, left, right, turbo, gun, mine, horn), with
/// the prompts for a key and, but for the horn, for a gamepad input.
const CONTROLS: [u32; 8] = [
    0x44_2AD0, 0x44_2A60, 0x44_2A4C, 0x44_2A34, 0x44_2A18, 0x44_29FC, 0x44_29E0, 0x44_29C0,
];
const KEY_PROMPTS: [u32; 8] = [
    0x44_3E34, 0x44_3E18, 0x44_3DF8, 0x44_3DD8, 0x44_3DB8, 0x44_3D98, 0x44_3D78, 0x44_3D60,
];
const PAD_PROMPTS: [u32; 7] = [
    0x44_3F14, 0x44_3EF8, 0x44_3ED8, 0x44_3EB8, 0x44_3E98, 0x44_3E74, 0x44_3E54,
];
/// The names of the gamepad inputs 0 (none) to 8, 16 bytes apart going down.
const PAD_NAMES: u32 = 0x44_3160;
pub const PAD_INPUTS: usize = 9;
/// Key names, 16 bytes apart going down from here in scancode order: 0x01..=0x54, then
/// `MORE_NAMED_KEYS`; one slot after 0xCB's holds a control's name. Other keys share one
/// fallback name.
const KEY_NAMES: u32 = 0x44_30C0;
const MORE_NAMED_KEYS: [u8; 17] = [
    0x57, 0x58, 0x9C, 0x9D, 0xB5, 0xB7, 0xB8, 0xC7, 0xC8, 0xC9, 0xCB, 0xCD, 0xCF, 0xD0, 0xD1, 0xD2,
    0xD3,
];
const NAMELESS_KEY: u32 = 0x44_30D0;

/// The Hall of Fame (spec M2c §3): the 18 circuits' names (15 bytes apart), the six cars'
/// names (in the car table, 1760 bytes apart, car 5 last), the difficulties' names (24 bytes
/// apart) and the order the records screen steps through the circuits.
const CIRCUIT_NAMES: u32 = 0x44_D148;
pub const CIRCUITS: usize = 18;
const CAR_NAMES: u32 = 0x45_01B0;
pub const CARS: usize = 6;
const DIFFICULTY_NAMES: u32 = 0x44_7340;
const DIFFICULTIES: usize = 4;
const CIRCUIT_ORDER: u32 = 0x45_673C;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    Exe(ExeError),
    /// A byte that is neither printable ASCII nor the gap.
    Unprintable {
        address: u32,
    },
    /// An empty string where the menus show text, or a font of another size.
    Unexpected {
        address: u32,
    },
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextError::Exe(error) => write!(f, "dr.exe: {error}"),
            TextError::Unprintable { address } => write!(
                f,
                "dr.exe: the text at address {address:#x} is not the original's; is this the known release?"
            ),
            TextError::Unexpected { address } => write!(
                f,
                "dr.exe: the data at address {address:#x} is not where the known release keeps it; is this the known release?"
            ),
        }
    }
}

impl std::error::Error for TextError {}

impl From<ExeError> for TextError {
    fn from(error: ExeError) -> TextError {
        TextError::Exe(error)
    }
}

/// The address of scancode `code`'s name.
fn key_name(code: u8) -> u32 {
    match code {
        0x01..=0x54 => KEY_NAMES - 16 * (u32::from(code) - 1),
        _ => match MORE_NAMED_KEYS.iter().position(|&named| named == code) {
            Some(k) => {
                let skip = if code > 0xCB { 16 } else { 0 };
                KEY_NAMES - 16 * (0x54 + k as u32) - skip
            }
            None => NAMELESS_KEY,
        },
    }
}

/// Configure's texts (spec M2b §3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigureTexts {
    pub adjust_music: Vec<u8>,
    pub adjust_effects: Vec<u8>,
    pub gamepad_on: Vec<u8>,
    pub gamepad_off: Vec<u8>,
    pub not_detected: Vec<u8>,
    pub press_any_key: Vec<u8>,
    /// The eight controls, each padded so its key's name lines up.
    pub controls: Vec<Vec<u8>>,
    pub key_prompts: Vec<Vec<u8>>,
    pub pad_prompts: Vec<Vec<u8>>,
    /// `key_names[scancode]`, all 256.
    pub key_names: Vec<Vec<u8>>,
    /// `pad_names[input]`, 0 (none) to 8.
    pub pad_names: Vec<Vec<u8>>,
}

/// The Hall of Fame's texts (spec M2c §3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HallOfFameTexts {
    pub circuits: Vec<Vec<u8>>,
    /// `cars[k]`: car `k`, 0 to 5.
    pub cars: Vec<Vec<u8>>,
    /// `difficulties[d]`; the last is empty.
    pub difficulties: Vec<Vec<u8>>,
    /// The circuits in the order Left and Right step through them.
    pub circuit_order: Vec<u8>,
}

/// A font's cell size and the pen advance of each glyph, character 32 first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub width: u8,
    pub height: u8,
    pub advances: Vec<u8>,
}

/// Everything M2a reads from `dr.exe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texts {
    /// `menus[m][r]`: row `r` of menu `m`, empty where the menu has no such row.
    pub menus: Vec<Vec<Vec<u8>>>,
    /// The bottom panel's four start-up lines.
    pub panel: Vec<Vec<u8>>,
    pub exit_question: Vec<u8>,
    pub yes: Vec<u8>,
    pub no: Vec<u8>,
    pub big: Metrics,
    pub small: Metrics,
    pub medium: Metrics,
    pub configure: ConfigureTexts,
    pub hall_of_fame: HallOfFameTexts,
}

impl Texts {
    /// # Errors
    ///
    /// [`TextError`] when a string is missing, does not end, or holds a byte the original's
    /// strings never do, when a string the menus show is empty, or when a font's size is not
    /// the known release's: the executable is not the known release.
    pub fn read(exe: &Exe) -> Result<Texts, TextError> {
        let text = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = exe.string_at(address, max)?;
            if bytes.iter().all(|&b| (32..127).contains(&b) || b == GAP) {
                Ok(bytes.to_vec())
            } else {
                Err(TextError::Unprintable { address })
            }
        };
        let shown = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = text(address, max)?;
            if bytes.is_empty() {
                Err(TextError::Unexpected { address })
            } else {
                Ok(bytes)
            }
        };
        let all = |addresses: &[u32]| -> Result<Vec<Vec<u8>>, TextError> {
            addresses
                .iter()
                .map(|&address| shown(address, MAX_LINE))
                .collect()
        };
        let menus = (0..MENUS)
            .map(|menu| {
                (0..MENU_ROWS)
                    .map(|row| {
                        let address = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS * menu + row) as u32;
                        if menu < SHOWN_MENUS && row < SHOWN_ROWS {
                            shown(address, MENU_ROW_BYTES as usize - 1)
                        } else {
                            text(address, MENU_ROW_BYTES as usize - 1)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metrics = |address: u32, glyphs: usize, size: (u8, u8)| -> Result<Metrics, TextError> {
            let bytes = exe.bytes_at(address, 2 + glyphs)?;
            if (bytes[0], bytes[1]) != size {
                return Err(TextError::Unexpected { address });
            }
            Ok(Metrics {
                width: bytes[0],
                height: bytes[1],
                advances: bytes[2..].to_vec(),
            })
        };
        Ok(Texts {
            menus,
            panel: PANEL_LINES
                .iter()
                .map(|&address| shown(address, MAX_LINE))
                .collect::<Result<Vec<_>, _>>()?,
            exit_question: shown(EXIT_QUESTION, MAX_LINE)?,
            yes: shown(YES, MAX_LINE)?,
            no: shown(NO, MAX_LINE)?,
            configure: ConfigureTexts {
                adjust_music: shown(ADJUST_MUSIC, MAX_LINE)?,
                adjust_effects: shown(ADJUST_EFFECTS, MAX_LINE)?,
                gamepad_on: shown(GAMEPAD_ON, MAX_LINE)?,
                gamepad_off: shown(GAMEPAD_OFF, MAX_LINE)?,
                not_detected: shown(NOT_DETECTED, MAX_LINE)?,
                press_any_key: shown(PRESS_ANY_KEY, MAX_LINE)?,
                controls: all(&CONTROLS)?,
                key_prompts: all(&KEY_PROMPTS)?,
                pad_prompts: all(&PAD_PROMPTS)?,
                key_names: (0..=255)
                    .map(|code| shown(key_name(code), MAX_LINE))
                    .collect::<Result<_, _>>()?,
                pad_names: (0..PAD_INPUTS as u32)
                    .map(|input| shown(PAD_NAMES - 16 * input, MAX_LINE))
                    .collect::<Result<_, _>>()?,
            },
            hall_of_fame: HallOfFameTexts {
                circuits: (0..CIRCUITS as u32)
                    .map(|c| shown(CIRCUIT_NAMES + 15 * c, MAX_LINE))
                    .collect::<Result<_, _>>()?,
                cars: (0..CARS as u32)
                    .map(|k| shown(CAR_NAMES - 1760 * (CARS as u32 - 1 - k), MAX_LINE))
                    .collect::<Result<_, _>>()?,
                difficulties: (0..DIFFICULTIES as u32)
                    .map(|d| text(DIFFICULTY_NAMES + 24 * d, MAX_LINE))
                    .collect::<Result<_, _>>()?,
                circuit_order: {
                    let order = exe.bytes_at(CIRCUIT_ORDER, CIRCUITS)?.to_vec();
                    if order.iter().any(|&c| usize::from(c) >= CIRCUITS) {
                        return Err(TextError::Unexpected {
                            address: CIRCUIT_ORDER,
                        });
                    }
                    order
                },
            },
            big: metrics(BIG_METRICS, 96, BIG_SIZE)?,
            small: metrics(SMALL_METRICS, 96, SMALL_SIZE)?,
            medium: metrics(MEDIUM_METRICS, 62, MEDIUM_SIZE)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::exe::tests::{build, build_at};

    /// A section from 0x442000 to 0x457000 with every string and table where the known release
    /// keeps it: menu `m` row `r` reads "m.r", the other strings made-up words.
    fn known_layout() -> Vec<u8> {
        let mut data = vec![0u8; 0x1_5000];
        let mut put = |address: u32, bytes: &[u8]| {
            let at = (address - 0x44_2000) as usize;
            data[at..at + bytes.len()].copy_from_slice(bytes);
        };
        for menu in 0..MENUS as u32 {
            for row in 0..MENU_ROWS as u32 {
                let address = MENU_TABLE + MENU_ROW_BYTES * (9 * menu + row);
                put(address, format!("{menu}.{row}").as_bytes());
            }
        }
        for (index, &address) in PANEL_LINES.iter().enumerate() {
            put(address, format!("line {index}").as_bytes());
        }
        put(EXIT_QUESTION, b"Quit?");
        put(YES, b"Y");
        put(NO, b"N");
        put(BIG_METRICS, &[32, 32, 20, 9]);
        put(SMALL_METRICS, &[16, 16, 10, 5]);
        put(MEDIUM_METRICS, &[9, 12, 9, 9]);
        for address in [
            ADJUST_MUSIC,
            ADJUST_EFFECTS,
            GAMEPAD_ON,
            GAMEPAD_OFF,
            NOT_DETECTED,
            PRESS_ANY_KEY,
        ] {
            put(address, format!("text {address:x}").as_bytes());
        }
        for address in CONTROLS.iter().chain(&KEY_PROMPTS).chain(&PAD_PROMPTS) {
            put(*address, format!("text {address:x}").as_bytes());
        }
        for code in 0..=255 {
            put(key_name(code), format!("key {code:02x}").as_bytes());
        }
        for input in 0..PAD_INPUTS as u32 {
            put(PAD_NAMES - 16 * input, format!("pad {input}").as_bytes());
        }
        for c in 0..CIRCUITS as u32 {
            put(CIRCUIT_NAMES + 15 * c, format!("circuit {c}").as_bytes());
        }
        for k in 0..CARS as u32 {
            put(CAR_NAMES - 1760 * k, format!("car {k}").as_bytes());
        }
        for d in 0..3 {
            put(DIFFICULTY_NAMES + 24 * d, format!("level {d}").as_bytes());
        }
        put(
            CIRCUIT_ORDER,
            &[0, 7, 5, 3, 4, 2, 8, 1, 6, 9, 16, 14, 12, 13, 11, 17, 10, 15],
        );
        build_at(0x4_2000, 0x1_5000, &data)
    }

    #[test]
    fn menu_rows_are_fifty_bytes_apart_and_menus_nine_rows() {
        // A wrong stride shows another menu's text, or half of two rows.
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.menus[0][0], b"0.0");
        assert_eq!(texts.menus[3][4], b"3.4");
        assert_eq!(texts.menus[8][8], b"8.8");
        assert_eq!(texts.panel[3], b"line 3");
        assert_eq!(
            (texts.exit_question.as_slice(), texts.yes.as_slice()),
            (b"Quit?".as_slice(), b"Y".as_slice())
        );
        assert_eq!(
            (texts.big.width, texts.big.height, &texts.big.advances[..2]),
            (32, 32, [20, 9].as_slice())
        );
        assert_eq!(texts.small.advances.len(), 96);
        assert_eq!(
            texts.medium.advances.len(),
            62,
            "the medium font has 62 glyphs"
        );
    }

    #[test]
    fn a_byte_the_original_never_uses_is_refused_with_its_address() {
        let mut bytes = known_layout();
        let at = offset(&bytes, YES);
        bytes[at] = 0x01;
        assert_eq!(
            Texts::read(&Exe::parse(bytes).unwrap()),
            Err(TextError::Unprintable { address: YES })
        );
    }

    #[test]
    fn each_key_has_its_name_and_keys_without_one_share_a_fallback() {
        // Define Keyboard shows these names; one slot off names every key after it wrongly.
        // The slot after 0xCB's holds the accelerate control's name, so 0xCD skips it.
        assert_eq!(key_name(0x01), 0x44_30C0);
        assert_eq!(key_name(0x54), 0x44_30C0 - 16 * 0x53);
        assert_eq!(key_name(0x57), 0x44_2B80);
        assert_eq!(key_name(0xCB), 0x44_2AE0);
        assert_eq!(key_name(0xCD), 0x44_2AC0);
        assert_eq!(key_name(0xD3), 0x44_2A70);
        for code in [0x00, 0x55, 0x56, 0x59, 0xCC, 0xD4, 0xFF] {
            assert_eq!(key_name(code), NAMELESS_KEY, "{code:#x}");
        }
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.configure.key_names[0x1E], b"key 1e");
        assert_eq!(texts.configure.pad_names[8], b"pad 8");
        assert_eq!(texts.configure.controls.len(), 8);
        assert_eq!(texts.configure.pad_prompts.len(), 7);
    }

    #[test]
    fn the_hall_of_fames_names_and_circuit_order_come_from_their_tables() {
        // Car 5 is the first in the table's order of reading: a wrong stride names every row's
        // car wrongly; a circuit order naming a circuit past 17 is not the known release.
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        let hall = &texts.hall_of_fame;
        assert_eq!(hall.circuits[17], b"circuit 17");
        assert_eq!(
            (hall.cars[5].as_slice(), hall.cars[0].as_slice()),
            (b"car 0".as_slice(), b"car 5".as_slice())
        );
        assert_eq!(hall.difficulties[1], b"level 1");
        assert!(hall.difficulties[3].is_empty());
        assert_eq!(hall.circuit_order[1], 7);
        let mut bad = known_layout();
        let at = offset(&bad, CIRCUIT_ORDER);
        bad[at] = 18;
        assert_eq!(
            Texts::read(&Exe::parse(bad).unwrap()),
            Err(TextError::Unexpected {
                address: CIRCUIT_ORDER
            })
        );
    }

    /// Where `address` lies in the bytes of [`known_layout`]'s file.
    fn offset(bytes: &[u8], address: u32) -> usize {
        bytes.len() - 0x1_5000 + (address - 0x44_2000) as usize
    }

    #[test]
    fn a_layout_moved_by_a_few_bytes_is_refused() {
        // Another build of dr.exe may keep the same strings a little further on. Every read
        // would then land on blanks, other strings' tails or other bytes that pass as text,
        // and the menus would show fragments: the rows the menus show and the fonts' sizes
        // must be where the known release keeps them.
        let mut moved = known_layout();
        let start = offset(&moved, 0x44_2000);
        moved[start..].rotate_right(3);
        let error = Texts::read(&Exe::parse(moved).unwrap()).unwrap_err();
        assert!(matches!(error, TextError::Unexpected { .. }), "{error}");
    }

    #[test]
    fn an_empty_shown_string_or_another_font_size_is_refused_with_its_address() {
        let mut no_yes = known_layout();
        let at = offset(&no_yes, YES);
        no_yes[at] = 0;
        assert_eq!(
            Texts::read(&Exe::parse(no_yes).unwrap()),
            Err(TextError::Unexpected { address: YES })
        );
        let mut empty_row = known_layout();
        let row = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS as u32 + 5);
        let at = offset(&empty_row, row);
        empty_row[at] = 0;
        assert_eq!(
            Texts::read(&Exe::parse(empty_row).unwrap()),
            Err(TextError::Unexpected { address: row }),
            "the start submenu's last row"
        );
        let mut narrow = known_layout();
        let at = offset(&narrow, SMALL_METRICS);
        narrow[at] = 15;
        assert_eq!(
            Texts::read(&Exe::parse(narrow).unwrap()),
            Err(TextError::Unexpected {
                address: SMALL_METRICS
            })
        );
    }

    #[test]
    fn a_file_that_is_not_the_known_release_is_refused_with_the_address() {
        // Another executable keeps different bytes at these addresses; drawing them would fill
        // the menus with garbage instead of saying what is wrong.
        let exe = Exe::parse(build(&[0x7F; 0x200])).unwrap();
        let error = Texts::read(&exe).unwrap_err();
        assert!(
            matches!(error, TextError::Exe(ExeError::Address(_))),
            "{error}"
        );
    }
}
```

<!-- write: crates/gamedata/src/assets.rs -->
```rust
//! The decoded data the startup sequence and the main menu need (spec M1a §4.2, M1b §4.1,
//! M2a §4.1).

use std::fmt;
use std::path::PathBuf;

use crate::bmp::{self, BmpError};
use crate::bpa::{Archive, BpaError};
use crate::catalog::{self, CatalogError};
use crate::dr_cfg::DrCfg;
use crate::exe::{Exe, ExeError};
use crate::haf::{Animation, HafError};
use crate::image::{Image, Palette, PaletteError};
use crate::machine::MachineError;
use crate::s3m::Module;
use crate::sound::{self, SoundError};
use crate::text::{TextError, Texts};
use crate::validate::Validation;
use crate::xm::Bank;

/// An image with the palette it is shown with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub image: Image,
    pub palette: Palette,
}

/// Everything the startup sequence shows.
#[derive(Debug)]
pub struct Assets {
    /// `SANIM.haf`.
    pub intro: Animation,
    /// `FRAMES.BPK`: the intro's 320x200 letterbox; the game uses palette entries 0..=15.
    pub letterbox: Picture,
    /// `APOGEE.BPK` with `APOGEE.PAL`.
    pub apogee: Picture,
    /// `rmd.bmp`.
    pub remedy: Picture,
    /// `STARTSCR.BPK` with `STARTSCR.PAL`.
    pub title: Picture,
    /// `TR0-MUS.CMF`: the music under the intro.
    pub intro_music: Module,
    /// `SANIM-E.CMF`: the intro's effects, numbered as in `SANIM.haf`'s table.
    pub intro_effects: Bank,
    /// `MEN-MUS.CMF`: the music that starts when the intro ends and goes on into the menus.
    pub menu_music: Module,
    pub menu: MenuAssets,
}

/// What the main menu draws and plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuAssets {
    /// `MENUBG5.BPK`, 640x480.
    pub background: Image,
    /// `CHATLIN1.BPK`, 640x10: the bottom panel's frame lines.
    pub panel_line: Image,
    /// `CORN3A.BPK` and `CORN3B.BPK`: popup corners (top left, top right, bottom left, bottom
    /// right) of a focused and of an unfocused popup.
    pub corners_focused: Vec<Image>,
    pub corners_unfocused: Vec<Image>,
    /// `CURSOR.BPK`: 50 frames, 20x20.
    pub cursor: Vec<Image>,
    /// `F-BIG3A`, `-B`, `-D` (32x32) and `F-SMA3A`, `-B`, `-C` (16x16): 96 glyphs each.
    pub big_a: Vec<Image>,
    pub big_b: Vec<Image>,
    pub big_d: Vec<Image>,
    pub small_a: Vec<Image>,
    pub small_b: Vec<Image>,
    pub small_c: Vec<Image>,
    /// `MENU.PAL`.
    pub palette: Palette,
    /// `COPPER.PAL`: one colour per player colour, whose ramps the menu palette gets.
    pub copper: Palette,
    /// `BGCOP.PAL`: 512 colours for the background copper rows.
    pub background_copper: Vec<[u8; 3]>,
    /// `CREDIT1.BPK` and `CREDIT2.BPK` with their palettes.
    pub credits: Vec<Picture>,
    /// `end.bmp`: the screen the game ends on.
    pub end: Picture,
    /// `MEN-SAM.CMF`: the menus' effects.
    pub effects: Bank,
    /// The strings and font metrics in `dr.exe`.
    pub texts: Texts,
    /// `SLIDMUS2.BPK` and `VOLCUR2.BPK`: the volume popups' slider and its knob.
    pub slider: Image,
    pub knob: Image,
    /// The `dr.cfg` the original writes when it has none, from `dr.exe`'s `defaultConfig`.
    pub default_config: DrCfg,
    /// The Hall of Fame (spec M2c §3): `F-MED1A` (62 glyphs), `FAMETXT`, `RECOTXT`, `RECOBAR`,
    /// the circuits' snapshots (`TRSNAP2M`), the arrows (`TRARR1`), the wipe's masks
    /// (`15X150`) and the border's corners (`CHOO2`).
    pub medium: Vec<Image>,
    pub fame_title: Image,
    pub records_title: Image,
    pub records_bar: Image,
    pub snapshots: Vec<Image>,
    pub arrows: Vec<Image>,
    pub wipe: Vec<Image>,
    pub border_corners: Vec<Image>,
}

#[derive(Debug)]
pub enum AssetError {
    Archive(BpaError),
    Image {
        name: &'static str,
        error: CatalogError,
    },
    Palette {
        name: &'static str,
        error: PaletteError,
    },
    Bmp {
        path: PathBuf,
        error: BmpError,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Animation(HafError),
    Sound(SoundError),
    Exe(ExeError),
    Machine(MachineError),
    Text(TextError),
    Size {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Archive(error) => write!(f, "{error}"),
            AssetError::Image { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Palette { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Bmp { path, error } => write!(f, "{}: {error}", path.display()),
            AssetError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            AssetError::Animation(error) => write!(f, "{error}"),
            AssetError::Sound(error) => write!(f, "{error}"),
            AssetError::Exe(error) => write!(f, "dr.exe: {error}"),
            AssetError::Machine(error) => write!(f, "{error}"),
            AssetError::Text(error) => write!(f, "{error}"),
            AssetError::Size {
                name,
                expected,
                actual,
            } => write!(f, "MENU.BPA/{name}: {actual} bytes, not {expected}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<BpaError> for AssetError {
    fn from(error: BpaError) -> AssetError {
        AssetError::Archive(error)
    }
}

impl Assets {
    /// Loads the startup sequence's data from a validated data directory.
    ///
    /// # Errors
    ///
    /// [`AssetError`] naming the file and entry that failed.
    pub fn load(validation: &Validation) -> Result<Assets, AssetError> {
        let path = |name: &str| -> PathBuf {
            validation
                .files
                .iter()
                .find(|file| file.name == name)
                .map(|file| file.path.clone())
                .unwrap_or_else(|| panic!("{name} is a required file, so validation found it"))
        };
        let menu = Archive::open(&path("MENU.BPA"))?;
        let musics = Archive::open(&path(sound::ARCHIVE))?;
        let remedy_path = path("RMD.BMP");
        let remedy_bytes = std::fs::read(&remedy_path).map_err(|source| AssetError::Read {
            path: remedy_path.clone(),
            source,
        })?;
        let (image, palette) = bmp::decode(&remedy_bytes).map_err(|error| AssetError::Bmp {
            path: remedy_path,
            error,
        })?;
        Ok(Assets {
            intro: Animation::open(&path("SANIM.HAF")).map_err(AssetError::Animation)?,
            letterbox: letterbox(&menu)?,
            apogee: picture(&menu, "APOGEE.BPK", "APOGEE.PAL")?,
            remedy: Picture { image, palette },
            title: picture(&menu, "STARTSCR.BPK", "STARTSCR.PAL")?,
            intro_music: sound::load_music(&musics, "TR0-MUS.CMF").map_err(AssetError::Sound)?,
            intro_effects: sound::load_effects(&musics, "SANIM-E.CMF")
                .map_err(AssetError::Sound)?,
            menu_music: sound::load_music(&musics, "MEN-MUS.CMF").map_err(AssetError::Sound)?,
            menu: menu_assets(&menu, &musics, &path("END.BMP"), &path("DR.EXE"))?,
        })
    }
}

/// All frames of a catalogued `MENU.BPA` image.
fn frames(menu: &Archive, name: &'static str) -> Result<Vec<Image>, AssetError> {
    let entry = catalog::find("MENU.BPA", name).expect("menu images are catalogued");
    entry
        .decode(menu.read(name)?)
        .map_err(|error| AssetError::Image { name, error })
}

fn palette(menu: &Archive, name: &'static str) -> Result<Palette, AssetError> {
    Palette::from_bytes(menu.read(name)?).map_err(|error| AssetError::Palette { name, error })
}

fn bmp_picture(path: &std::path::Path) -> Result<Picture, AssetError> {
    let bytes = std::fs::read(path).map_err(|source| AssetError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let (image, palette) = bmp::decode(&bytes).map_err(|error| AssetError::Bmp {
        path: path.to_path_buf(),
        error,
    })?;
    Ok(Picture { image, palette })
}

/// A picture the menu copies over its whole screen, which must be 640x480.
fn full_screen(picture: Picture, path: &std::path::Path) -> Result<Picture, AssetError> {
    let (width, height) = (picture.image.width, picture.image.height);
    if (width, height) == (640, 480) {
        Ok(picture)
    } else {
        Err(AssetError::Bmp {
            path: path.to_path_buf(),
            error: BmpError::Unsupported(format!("{width}x{height}, not 640x480")),
        })
    }
}

fn menu_assets(
    menu: &Archive,
    musics: &Archive,
    end: &std::path::Path,
    exe: &std::path::Path,
) -> Result<MenuAssets, AssetError> {
    const BGCOP: &str = "BGCOP.PAL";
    let background_copper = menu.read(BGCOP)?;
    // 512 colours of 6-bit components.
    if background_copper.len() != 3 * 512 {
        return Err(AssetError::Size {
            name: BGCOP,
            expected: 3 * 512,
            actual: background_copper.len(),
        });
    }
    if let Some((index, &value)) = background_copper.iter().enumerate().find(|(_, c)| **c > 63) {
        return Err(AssetError::Palette {
            name: BGCOP,
            error: PaletteError::NotSixBit { index, value },
        });
    }
    let exe_bytes = std::fs::read(exe).map_err(|source| AssetError::Read {
        path: exe.to_path_buf(),
        source,
    })?;
    let exe = Exe::parse(exe_bytes).map_err(AssetError::Exe)?;
    Ok(MenuAssets {
        background: frames(menu, "MENUBG5.BPK")?.remove(0),
        panel_line: frames(menu, "CHATLIN1.BPK")?.remove(0),
        corners_focused: frames(menu, "CORN3A.BPK")?,
        corners_unfocused: frames(menu, "CORN3B.BPK")?,
        cursor: frames(menu, "CURSOR.BPK")?,
        big_a: frames(menu, "F-BIG3A.BPK")?,
        big_b: frames(menu, "F-BIG3B.BPK")?,
        big_d: frames(menu, "F-BIG3D.BPK")?,
        small_a: frames(menu, "F-SMA3A.BPK")?,
        small_b: frames(menu, "F-SMA3B.BPK")?,
        small_c: frames(menu, "F-SMA3C.BPK")?,
        palette: palette(menu, "MENU.PAL")?,
        copper: palette(menu, "COPPER.PAL")?,
        background_copper: background_copper.as_chunks::<3>().0.to_vec(),
        credits: vec![
            picture(menu, "CREDIT1.BPK", "CREDIT1.PAL")?,
            picture(menu, "CREDIT2.BPK", "CREDIT2.PAL")?,
        ],
        end: full_screen(bmp_picture(end)?, end)?,
        effects: sound::load_effects(musics, "MEN-SAM.CMF").map_err(AssetError::Sound)?,
        texts: Texts::read(&exe).map_err(AssetError::Text)?,
        slider: frames(menu, "SLIDMUS2.BPK")?.remove(0),
        knob: frames(menu, "VOLCUR2.BPK")?.remove(0),
        default_config: DrCfg::defaults(&exe).map_err(AssetError::Machine)?,
        medium: frames(menu, "F-MED1A.BPK")?,
        fame_title: frames(menu, "FAMETXT.BPK")?.remove(0),
        records_title: frames(menu, "RECOTXT.BPK")?.remove(0),
        records_bar: frames(menu, "RECOBAR.BPK")?.remove(0),
        snapshots: frames(menu, "TRSNAP2M.BPK")?,
        arrows: frames(menu, "TRARR1.BPK")?,
        wipe: frames(menu, "15X150.BPK")?,
        border_corners: frames(menu, "CHOO2.BPK")?,
    })
}

fn picture(
    menu: &Archive,
    image: &'static str,
    palette: &'static str,
) -> Result<Picture, AssetError> {
    let entry = catalog::find("MENU.BPA", image).expect("these images are catalogued");
    let mut frames = entry
        .decode(menu.read(image)?)
        .map_err(|error| AssetError::Image { name: image, error })?;
    let palette =
        Palette::from_bytes(menu.read(palette)?).map_err(|error| AssetError::Palette {
            name: palette,
            error,
        })?;
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

fn letterbox(menu: &Archive) -> Result<Picture, AssetError> {
    const NAME: &str = "FRAMES.BPK";
    let entry = catalog::find("MENU.BPA", NAME).expect("FRAMES.BPK is catalogued");
    let bytes = menu.read(NAME)?;
    let image_error = |error| AssetError::Image { name: NAME, error };
    let mut frames = entry.decode(bytes).map_err(image_error)?;
    let palette = entry
        .embedded_palette(bytes)
        .map_err(image_error)?
        .expect("FRAMES.BPK embeds its palette");
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_end_screen_of_another_size_is_an_error_not_a_crash_later() {
        // The menu copies END.BMP over its whole 640x480 screen; a BMP of another size (from
        // an unknown release) is reported when the data loads, naming the file.
        let path = std::path::Path::new("END.BMP");
        let screen = |width: u32, height: u32| Picture {
            image: Image::new(width, height, vec![0; (width * height) as usize]),
            palette: Palette::BLACK,
        };
        assert!(full_screen(screen(640, 480), path).is_ok());
        let error = full_screen(screen(320, 200), path).unwrap_err();
        assert_eq!(
            error.to_string(),
            "END.BMP: unsupported BMP: 320x200, not 640x480"
        );
    }
}
```

<!-- write: crates/gamedata/src/dr_cfg.rs -->
```rust
//! The original's settings file, `dr.cfg` (spec M2b §3.1): 8 bytes of header, then the
//! settings, the records and the Hall of Fame at the offsets `saveConfiguration` (0x4264E0)
//! writes them. DeadRally keeps every byte it does not use as it was.

use std::io;
use std::path::{Path, PathBuf};

use crate::exe::Exe;
use crate::machine::{Machine, MachineError};

/// DeadRally's own `dr.cfg`, next to its `config.toml` (spec M2b decision 2).
pub fn own_path() -> Option<PathBuf> {
    crate::config_path().map(|config| config.with_file_name("dr.cfg"))
}

/// The `dr.cfg` to start with: DeadRally's own (`own`), else the game folder's, read once,
/// else `defaults`. Nothing is written.
///
/// # Errors
///
/// An [`io::Error`] when a file exists but cannot be read; starting from the defaults would
/// overwrite the player's settings and records at the next save.
pub fn load(own: Option<&Path>, game_dir: &Path, defaults: &DrCfg) -> io::Result<DrCfg> {
    let game = ["dr.cfg", "DR.CFG"].map(|name| game_dir.join(name));
    for path in own.into_iter().chain(game.iter().map(PathBuf::as_path)) {
        match std::fs::read(path) {
            Ok(bytes) => return Ok(DrCfg::parse(&bytes).unwrap_or_else(|| defaults.clone())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(defaults.clone())
}

/// Writes `bytes` to DeadRally's own `dr.cfg` at `own`, never into the game folder.
///
/// # Errors
///
/// An [`io::Error`] when the directory or the file cannot be written.
pub fn save(own: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = own.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(own, bytes)
}

/// Three bytes, a 32-bit value and a byte `rand()` wrote, which nothing reads.
pub const HEADER_BYTES: usize = 8;
/// What `saveConfiguration` writes after the header.
pub const PAYLOAD_BYTES: usize = 0xB76;
/// A file this short or shorter is replaced by the defaults (`loadConfig`).
const SHORTEST: usize = 7;

/// Offsets in the payload.
const MUSIC_VOLUME: usize = 0x00;
const EFFECTS_VOLUME: usize = 0x04;
const USE_JOYSTICK: usize = 0x10;
/// Accelerate, brake, left, right, turbo, gun, mine, horn: set-1 scancodes.
const KEYS: usize = 0xB36;
pub const KEY_COUNT: usize = 8;
/// Accelerate, brake, left, right, turbo, gun, mine: gamepad inputs (0 none, 1–4 the
/// stick's left, right, up, down, 5–8 buttons 1–4).
const PADS: usize = 0xB56;
pub const PAD_COUNT: usize = 7;
const TIMES_PLAYED: usize = 0xB72;
/// The circuits' records: 18 circuits by 6 cars, 24 bytes each (a name of up to 12 bytes,
/// minutes, seconds, hundredths), record `circuit + 18 * car`.
const RECORDS: usize = 0x4E;
const RECORD_BYTES: usize = 24;
/// The best ten: 20 bytes each (a name of up to 12 bytes, races, difficulty).
const HALL_OF_FAME: usize = 0xA6E;
const ENTRY_BYTES: usize = 20;
pub const HALL_OF_FAME_ENTRIES: usize = 10;
const NAME_BYTES: usize = 12;

/// `defaultConfig`, up to its jump into `saveConfiguration`.
const DEFAULT_CONFIG: (u32, u32) = (0x42_6700, 0x42_71E8);
/// Where the original keeps what it writes: (offset in the file, address, bytes).
const HEADER_SOURCES: [(usize, u32, usize); 4] = [
    (0, 0x45_FB6C, 1),
    (1, 0x45_FBF0, 1),
    (2, 0x46_3D9C, 1),
    (3, 0x46_2D54, 4),
];
const PAYLOAD_SOURCES: [(usize, u32, usize); 29] = [
    (0x00, 0x45_DC14, 4),
    (0x04, 0x45_DC18, 4),
    (0x08, 0x44_57CC, 4),
    (0x0C, 0x45_6738, 4),
    (0x10, 0x45_EA00, 4),
    (0x14, 0x46_3D00, 21),
    (0x29, 0x45_EB80, 21),
    (0x3E, 0x45_6734, 4),
    (0x42, 0x45_FB68, 4),
    (0x46, 0x45_DC40, 4),
    (0x4A, 0x45_DC1C, 4),
    (0x4E, 0x45_F040, 0xA20),
    (0xA6E, 0x46_1F20, 0xC8),
    (0xB36, 0x46_1EA8, 4),
    (0xB3A, 0x46_3CA8, 4),
    (0xB3E, 0x46_1FF8, 4),
    (0xB42, 0x45_EA68, 4),
    (0xB46, 0x45_FBF4, 4),
    (0xB4A, 0x46_3CE4, 4),
    (0xB4E, 0x46_1270, 4),
    (0xB52, 0x46_3D18, 4),
    (0xB56, 0x46_1294, 4),
    (0xB5A, 0x46_1F14, 4),
    (0xB5E, 0x45_EEA0, 4),
    (0xB62, 0x46_2D6C, 4),
    (0xB66, 0x46_3D8C, 4),
    (0xB6A, 0x46_2CFC, 4),
    (0xB6E, 0x46_3CA4, 4),
    (0xB72, 0x46_3CEC, 4),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrCfg {
    header: [u8; HEADER_BYTES],
    payload: Vec<u8>,
}

impl DrCfg {
    /// The file the original writes when it has none: `defaultConfig` run over the player's
    /// `dr.exe`, its variables then gathered as `saveConfiguration` gathers them. The random
    /// byte is 0.
    ///
    /// # Errors
    ///
    /// [`MachineError`] when the code is not the known release's.
    pub fn defaults(exe: &Exe) -> Result<DrCfg, MachineError> {
        let mut machine = Machine::new(exe);
        machine.run(DEFAULT_CONFIG.0, DEFAULT_CONFIG.1)?;
        let mut header = [0; HEADER_BYTES];
        for (offset, address, length) in HEADER_SOURCES {
            header[offset..offset + length].copy_from_slice(&machine.bytes(address, length)?);
        }
        let mut payload = vec![0; PAYLOAD_BYTES];
        for (offset, address, length) in PAYLOAD_SOURCES {
            payload[offset..offset + length].copy_from_slice(&machine.bytes(address, length)?);
        }
        Ok(DrCfg { header, payload })
    }

    /// A `dr.cfg` as `loadConfig` takes it: `None` when it is 7 bytes or shorter (the
    /// original then uses its defaults); else its header and payload, bytes it lacks 0 and
    /// extra bytes left out.
    pub fn parse(bytes: &[u8]) -> Option<DrCfg> {
        if bytes.len() <= SHORTEST {
            return None;
        }
        let mut header = [0; HEADER_BYTES];
        let in_header = bytes.len().min(HEADER_BYTES);
        header[..in_header].copy_from_slice(&bytes[..in_header]);
        let mut payload = bytes.get(HEADER_BYTES..).unwrap_or_default().to_vec();
        payload.resize(PAYLOAD_BYTES, 0);
        Some(DrCfg { header, payload })
    }

    /// The file as `saveConfiguration` writes it.
    pub fn to_bytes(&self) -> Vec<u8> {
        [&self.header[..], &self.payload].concat()
    }

    fn get(&self, offset: usize) -> u32 {
        let b = &self.payload[offset..offset + 4];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }

    fn put(&mut self, offset: usize, value: u32) {
        self.payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// The music and effects volumes, 0..=0x10000.
    pub fn music_volume(&self) -> u32 {
        self.get(MUSIC_VOLUME)
    }

    pub fn set_music_volume(&mut self, volume: u32) {
        self.put(MUSIC_VOLUME, volume);
    }

    pub fn effects_volume(&self) -> u32 {
        self.get(EFFECTS_VOLUME)
    }

    pub fn set_effects_volume(&mut self, volume: u32) {
        self.put(EFFECTS_VOLUME, volume);
    }

    /// The gamepad switch: 0 off, 1 or 2 on.
    pub fn use_joystick(&self) -> u32 {
        self.get(USE_JOYSTICK)
    }

    pub fn set_use_joystick(&mut self, on: u32) {
        self.put(USE_JOYSTICK, on);
    }

    pub fn key(&self, control: usize) -> u32 {
        assert!(control < KEY_COUNT);
        self.get(KEYS + 4 * control)
    }

    pub fn set_key(&mut self, control: usize, scancode: u32) {
        assert!(control < KEY_COUNT);
        self.put(KEYS + 4 * control, scancode);
    }

    pub fn pad(&self, control: usize) -> u32 {
        assert!(control < PAD_COUNT);
        self.get(PADS + 4 * control)
    }

    pub fn set_pad(&mut self, control: usize, input: u32) {
        assert!(control < PAD_COUNT);
        self.put(PADS + 4 * control, input);
    }

    /// The name at `offset`, up to its NUL within `room` bytes.
    fn name(&self, offset: usize, room: usize) -> &[u8] {
        let field = &self.payload[offset..offset + room];
        &field[..field.iter().position(|&b| b == 0).unwrap_or(room)]
    }

    /// Record `car` (0–5) of circuit `circuit` (0–17): the driver's name and the time
    /// (minutes, seconds, hundredths).
    pub fn record(&self, circuit: usize, car: usize) -> (&[u8], [u32; 3]) {
        let at = RECORDS + RECORD_BYTES * (circuit + 18 * car);
        let time = [0, 1, 2].map(|i| self.get(at + NAME_BYTES + 4 * i));
        (self.name(at, NAME_BYTES), time)
    }

    /// Entry `rank` (0–9) of the best ten: the name, races and difficulty.
    pub fn hall_of_fame(&self, rank: usize) -> (&[u8], i32, u32) {
        let at = HALL_OF_FAME + ENTRY_BYTES * rank;
        let races = self.get(at + NAME_BYTES) as i32;
        (
            self.name(at, NAME_BYTES),
            races,
            self.get(at + NAME_BYTES + 4),
        )
    }

    /// Upper-cases the best ten's names in place, as `seeHallOfFame` (0x431510) does with
    /// `_strupr` before drawing them.
    pub fn upper_case_hall_of_fame(&mut self) {
        for rank in 0..HALL_OF_FAME_ENTRIES {
            let at = HALL_OF_FAME + ENTRY_BYTES * rank;
            let length = self.name(at, NAME_BYTES).len();
            self.payload[at..at + length].make_ascii_uppercase();
        }
    }

    pub fn times_played(&self) -> u32 {
        self.get(TIMES_PLAYED)
    }

    pub fn set_times_played(&mut self, times: u32) {
        self.put(TIMES_PLAYED, times);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> Vec<u8> {
        (0..HEADER_BYTES + PAYLOAD_BYTES)
            .map(|i| (i % 251) as u8)
            .collect()
    }

    #[test]
    fn a_file_reads_and_writes_back_byte_for_byte() {
        // The records and the Hall of Fame are only carried in M2b; a byte lost on the way
        // would lose a player's records.
        assert_eq!(DrCfg::parse(&file()).unwrap().to_bytes(), file());
    }

    fn dummy() -> DrCfg {
        DrCfg {
            header: [9; HEADER_BYTES],
            payload: vec![9; PAYLOAD_BYTES],
        }
    }

    #[test]
    fn a_file_of_seven_bytes_or_fewer_is_no_file() {
        assert_eq!(DrCfg::parse(&[1; 7]), None);
        assert!(DrCfg::parse(&[1; 8]).is_some());
    }

    #[test]
    fn a_short_file_lacks_its_last_bytes_as_zeros_and_a_long_one_is_cut() {
        let short = DrCfg::parse(&file()[..20]).unwrap().to_bytes();
        assert_eq!(&short[..20], &file()[..20]);
        assert!(short[20..].iter().all(|&b| b == 0));
        assert_eq!(short.len(), HEADER_BYTES + PAYLOAD_BYTES);
        let mut long = file();
        long.extend([7; 100]);
        assert_eq!(DrCfg::parse(&long).unwrap().to_bytes(), file());
    }

    #[test]
    fn the_settings_sit_at_the_offsets_save_configuration_writes() {
        // A setting at another offset would read the player's records as their volume.
        let mut cfg = dummy();
        cfg.set_music_volume(0x7400);
        cfg.set_effects_volume(0xC400);
        cfg.set_use_joystick(1);
        cfg.set_key(0, 0x10);
        cfg.set_key(7, 0x39);
        cfg.set_pad(6, 8);
        cfg.set_times_played(3);
        let bytes = cfg.to_bytes();
        let at = |offset: usize| &bytes[HEADER_BYTES + offset..HEADER_BYTES + offset + 4];
        assert_eq!(at(0), [0x00, 0x74, 0, 0]);
        assert_eq!(at(4), [0x00, 0xC4, 0, 0]);
        assert_eq!(at(0x10), [1, 0, 0, 0]);
        assert_eq!(at(0xB36), [0x10, 0, 0, 0]);
        assert_eq!(at(0xB52), [0x39, 0, 0, 0]);
        assert_eq!(at(0xB6E), [8, 0, 0, 0]);
        assert_eq!(at(0xB72), [3, 0, 0, 0]);
        assert_eq!(
            (cfg.music_volume(), cfg.key(7), cfg.pad(6)),
            (0x7400, 0x39, 8)
        );
    }

    #[test]
    fn the_saved_variables_cover_the_payload_without_gaps() {
        let mut next = 0;
        for (offset, _, length) in PAYLOAD_SOURCES {
            assert_eq!(offset, next);
            next = offset + length;
        }
        assert_eq!(next, PAYLOAD_BYTES);
    }

    #[test]
    fn deadrallys_own_file_comes_first_then_the_games_once_then_the_defaults() {
        // The player's records survive: DeadRally starts from its own copy, picks up the
        // original's settings the first time, and never writes into the game folder.
        let home = tempfile::tempdir().unwrap();
        let game = tempfile::tempdir().unwrap();
        let own = home.path().join("deadrally/dr.cfg");
        assert_eq!(load(Some(&own), game.path(), &dummy()).unwrap(), dummy());
        std::fs::write(game.path().join("dr.cfg"), file()).unwrap();
        let imported = load(Some(&own), game.path(), &dummy()).unwrap();
        assert_eq!(imported.to_bytes(), file());
        let mut changed = imported;
        changed.set_times_played(9);
        save(&own, &changed.to_bytes()).unwrap();
        assert_eq!(load(Some(&own), game.path(), &dummy()).unwrap(), changed);
        assert_eq!(std::fs::read(game.path().join("dr.cfg")).unwrap(), file());
    }

    #[test]
    fn a_file_that_cannot_be_read_is_an_error_not_the_defaults() {
        // Starting from the defaults would overwrite the player's file at the next save.
        let home = tempfile::tempdir().unwrap();
        let unreadable = home.path().join("dr.cfg");
        std::fs::create_dir(&unreadable).unwrap();
        assert!(load(Some(&unreadable), home.path(), &dummy()).is_err());
    }

    #[test]
    fn records_and_the_best_ten_sit_where_save_configuration_puts_them() {
        // Circuit-major within each car: record 17 + 18 * 5 is the last of the 2592 bytes.
        let mut bytes = vec![0; HEADER_BYTES + PAYLOAD_BYTES];
        let last = HEADER_BYTES + RECORDS + RECORD_BYTES * (17 + 18 * 5);
        bytes[last..last + 3].copy_from_slice(b"Ann");
        bytes[last + 16..last + 20].copy_from_slice(&59u32.to_le_bytes());
        let first = HEADER_BYTES + HALL_OF_FAME;
        bytes[first..first + 12].copy_from_slice(b"Bob Twelve!!");
        bytes[first + 12..first + 16].copy_from_slice(&7u32.to_le_bytes());
        bytes[first + 16..first + 20].copy_from_slice(&2u32.to_le_bytes());
        let mut cfg = DrCfg::parse(&bytes).unwrap();
        assert_eq!(cfg.record(17, 5), (b"Ann".as_slice(), [0, 59, 0]));
        assert_eq!(cfg.hall_of_fame(0), (b"Bob Twelve!!".as_slice(), 7, 2));
        cfg.upper_case_hall_of_fame();
        assert_eq!(cfg.hall_of_fame(0).0, b"BOB TWELVE!!");
        assert_eq!(cfg.record(17, 5).0, b"Ann", "records keep their case");
    }
}
```

- [ ] **Step 4: Run the checks, with the data**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (298 passed, 18 ignored), among them `the_hall_of_fames_names_and_circuit_order_come_from_their_tables` and `records_and_the_best_ten_sit_where_save_configuration_puts_them`.

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test -p deadrally-gamedata --test catalog_data -- --ignored`
Expected: all pass (7 passed).

- [ ] **Step 5: Commit**

```bash
git add crates/gamedata crates/core/src/menu/draw.rs crates/core/tests/common/mod.rs
git commit -m "feat: load the Hall of Fame's data"
```

---

### Task 2: The Hall of Fame

**Files:**
- Create: `crates/core/src/menu/hall_of_fame.rs`
- Modify: `crates/core/src/canvas.rs`, `crates/core/src/audio/{mod,music}.rs`, `crates/core/src/menu/{mod,draw}.rs`, `crates/core/tests/menu.rs`

**Interfaces:**
- Consumes: Task 1's texts, pictures and `DrCfg` fields.
- Produces: `Canvas::blit_mask`, `Music::{order, set_order}`, `Sound::{music_order, set_music_order}`, `Graphics::medium`.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/core/src/canvas.rs -->
```rust
//! The original's 640x480 drawing buffer and its operations (spec M2a §4.2). Positions are
//! linear offsets, `y * 640 + x`, as the original passes them: a picture drawn past the right
//! edge goes on at the start of the next row, as it does there. Nothing is drawn past the end.

use deadrally_gamedata::image::Image;

/// Menu screens are 640x480.
pub(crate) const WIDTH: usize = 640;
pub(crate) const HEIGHT: usize = 480;

/// The offset of column `x`, row `y`.
pub(crate) const fn at(x: usize, y: usize) -> usize {
    y * WIDTH + x
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Canvas {
    pixels: Vec<u8>,
}

impl Default for Canvas {
    fn default() -> Canvas {
        Canvas {
            pixels: vec![0; WIDTH * HEIGHT],
        }
    }
}

impl Canvas {
    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Copies the whole of `picture`, which must be 640x480.
    pub(crate) fn copy_all(&mut self, picture: &Image) {
        self.pixels.copy_from_slice(&picture.pixels);
    }

    /// Copies rows `first..first + rows` of `picture` (640 wide) into the same rows.
    pub(crate) fn copy_rows(&mut self, picture: &Image, first: usize, rows: usize) {
        let range = at(0, first)..at(0, first + rows).min(self.pixels.len());
        self.pixels[range.clone()].copy_from_slice(&picture.pixels[range]);
    }

    /// Copies a `width` x `height` region of `picture` (640 wide) at `offset` into the same place.
    pub(crate) fn restore(&mut self, picture: &Image, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&picture.pixels[start..end]);
        }
    }

    /// Copies a `width` x `height` region at `offset` from `other` into the same place
    /// (`copyRectToVram`, 0x41AA40, and `refreshAllScreen`, 0x41A210, for the whole screen).
    pub(crate) fn copy_from(&mut self, other: &Canvas, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&other.pixels[start..end]);
        }
    }

    /// Fills `width` x `height` pixels from `offset` with `colour`.
    pub(crate) fn fill(&mut self, offset: usize, width: usize, height: usize, colour: u8) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].fill(colour);
        }
    }

    /// Draws `image` at `offset`; with `transparent`, colour 0 leaves the canvas as it is.
    pub(crate) fn draw(&mut self, image: &Image, offset: usize, transparent: bool) {
        let width = image.width as usize;
        for (row, source) in image.pixels.chunks_exact(width.max(1)).enumerate() {
            let start = offset + row * WIDTH;
            if start >= self.pixels.len() {
                break;
            }
            let end = (start + width).min(self.pixels.len());
            let target = &mut self.pixels[start..end];
            for (pixel, &colour) in target.iter_mut().zip(source) {
                if !transparent || colour != 0 {
                    *pixel = colour;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_drawing_keeps_what_colour_0_covers() {
        // Glyphs, corners and the cursor are drawn this way; an opaque 0 would cut boxes
        // into the background around every letter.
        let mut canvas = Canvas::default();
        canvas.fill(at(0, 0), 4, 1, 9);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), true);
        assert_eq!(&canvas.pixels()[..4], [1, 9, 2, 9]);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), false);
        assert_eq!(&canvas.pixels()[..4], [1, 0, 2, 9]);
    }

    #[test]
    fn drawing_past_the_right_edge_goes_on_in_the_next_row() {
        // The original works on linear offsets; text that runs past column 639 shows at the
        // left of the next row, and a faithful copy must too.
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(3, 1, vec![5, 6, 7]), at(638, 0), false);
        assert_eq!(&canvas.pixels()[at(638, 0)..at(1, 1)], [5, 6, 7]);
    }

    #[test]
    fn nothing_is_drawn_past_the_end() {
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(2, 2, vec![1; 4]), at(639, 479), false);
        canvas.fill(at(639, 479), 5, 5, 3);
        assert_eq!(canvas.pixels()[at(639, 479)], 3);
        assert_eq!(canvas.pixels().len(), WIDTH * HEIGHT);
    }

    #[test]
    fn restoring_copies_a_region_back_from_a_picture() {
        let background = Image::new(
            640,
            480,
            (0..WIDTH * HEIGHT).map(|i| (i % 251) as u8).collect(),
        );
        let mut canvas = Canvas::default();
        canvas.restore(&background, at(10, 20), 3, 2);
        assert_eq!(canvas.pixels()[at(10, 20)], background.pixels[at(10, 20)]);
        assert_eq!(canvas.pixels()[at(12, 21)], background.pixels[at(12, 21)]);
        assert_eq!(canvas.pixels()[at(13, 21)], 0, "only the region");
        canvas.copy_rows(&background, 100, 2);
        assert_eq!(
            canvas.pixels()[at(639, 101)],
            background.pixels[at(639, 101)]
        );
        assert_eq!(canvas.pixels()[at(0, 102)], 0);
    }

    #[test]
    fn a_masked_copy_takes_the_other_canvas_where_the_mask_is_set() {
        // The wipe's tiles: a pixel of the new screen shows only where the mask has one.
        let mut old = Canvas::default();
        let mut new = Canvas::default();
        new.fill(0, WIDTH, 2, 9);
        old.blit_mask(&Image::new(3, 2, vec![1, 0, 1, 0, 1, 0]), &new, at(10, 0));
        assert_eq!(&old.pixels()[at(10, 0)..at(13, 0)], [9, 0, 9]);
        assert_eq!(&old.pixels()[at(10, 1)..at(13, 1)], [0, 9, 0]);
    }
}
```

<!-- write: crates/core/src/audio/music.rs -->
```rust
//! The music player: Scream Tracker 3 modules with the 13 commands the game's music uses
//! (spec M1b §3.2, §4.2). Timing is counted in output samples, so the tempo never drifts.

use std::sync::Arc;

use deadrally_gamedata::s3m::{self, Cell, Module, NO_NOTE, NOTE_CUT, ORDER_SKIP};

use super::mixer::{Loop, Voice};
use super::tables::{S3M_PERIODS, vibrato};
use crate::AUDIO_SAMPLE_RATE;

/// Scream Tracker's clock: frequency = 14317056 / period.
const CLOCK: u32 = 14_317_056;
/// FMOD mixes at this rate and counts a tick as a whole number of its samples:
/// `44100 * 5 / (2 * tempo)`, rounded down. At tempo 141 that is 781 samples, not 781.9, so
/// the original plays such music 0.1 % fast; at tempo 125 it is exactly 882 (20 ms).
const FMOD_RATE: u32 = 44_100;
/// Period limits of Scream Tracker 3.
const MIN_PERIOD: i32 = 64;
const MAX_PERIOD: i32 = 32_767;

#[derive(Clone, Debug)]
struct SampleData {
    data: Arc<[i16]>,
    looping: Loop,
    c2spd: u32,
    volume: i32,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    enabled: bool,
    /// 0..=255.
    pan: i64,
    voice: Option<Voice>,
    /// Current sample (1-based instrument number), 0 = none yet.
    instrument: u8,
    period: i32,
    target_period: i32,
    /// 0..=64.
    volume: i32,
    /// Vibrato offset of this tick, in period units.
    period_delta: i32,
    command: u8,
    info: u8,
    // Effect memories.
    volume_slide: u8,
    porta: u8,
    tone_porta: u8,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_position: u8,
    offset: u8,
    retrigger: u8,
    retrigger_count: u8,
    /// A row's note held back by SDx until this tick.
    delayed: Option<(u8, Cell)>,
}

#[derive(Debug)]
pub(crate) struct Music {
    orders: Vec<u8>,
    patterns: Vec<s3m::Pattern>,
    samples: Vec<Option<SampleData>>,
    global_volume: i32,
    channels: Vec<Channel>,
    fading: Vec<Voice>,
    speed: u8,
    tempo: u8,
    order: usize,
    row: usize,
    tick: u8,
    /// Output frames left in the current tick, and the carried fraction of a frame.
    frames_left: u32,
    remainder: u32,
    /// Where the next row comes from after a B or C command.
    jump: Option<(usize, usize)>,
    /// Gain applied to every channel, in 16.16 (the original's master volume).
    gain: i64,
    /// The module's own share of it: its master volume, doubled for a stereo module.
    module_gain: i64,
}

impl Music {
    /// A player at order `first_order` (counted as the game counts them, markers included),
    /// row 0 and tick 0. `gain` scales the whole module ([`UNITY`] = as loud as the module
    /// asks). As FMOD does (measured), the module's own master volume scales it by
    /// `master / 64` (the game's music has 48, 2.5 dB below full), and a stereo module plays
    /// at twice a mono module's level.
    pub(crate) fn new(module: &Module, gain: i64, first_order: usize) -> Music {
        let module_gain = i64::from(module.master_volume) * if module.stereo { 2 } else { 1 };
        let gain = gain * module_gain / 64;
        let samples = module
            .samples
            .iter()
            .map(|sample| {
                (!sample.data.is_empty()).then(|| SampleData {
                    data: Arc::from(sample.data.as_slice()),
                    looping: sample
                        .looped
                        .map_or(Loop::None, |(start, end)| Loop::Forward { start, end }),
                    c2spd: sample.c2spd,
                    volume: i32::from(sample.volume),
                })
            })
            .collect();
        let channels = module
            .channels
            .iter()
            .map(|channel| Channel {
                enabled: channel.enabled,
                // FMOD 3 puts a stereo module's channels fully on their side (measured on the
                // menu music); mono modules play in the centre.
                pan: match (module.stereo, channel.pan < 8) {
                    (false, _) => 128,
                    (true, true) => 0,
                    (true, false) => 255,
                },
                ..Channel::default()
            })
            .collect();
        let mut music = Music {
            orders: module.orders.clone(),
            patterns: module.patterns.clone(),
            samples,
            global_volume: i32::from(module.global_volume),
            channels,
            fading: Vec::new(),
            speed: module.initial_speed.max(1),
            tempo: module.initial_tempo.max(32),
            order: first_order,
            row: 0,
            tick: 0,
            frames_left: 0,
            remainder: 0,
            jump: None,
            gain,
            module_gain,
        };
        music.skip_marker_orders();
        music
    }

    /// Fades every channel out, as `FMUSIC_StopSong`.
    pub(crate) fn stop(&mut self) {
        for channel in &mut self.channels {
            if let Some(mut voice) = channel.voice.take() {
                voice.release();
                self.fading.push(voice);
            }
        }
        self.orders.clear();
    }

    /// A new master volume for the song (`FMUSIC_SetMasterVolume`), as [`Music::new`]'s `gain`.
    pub(crate) fn set_gain(&mut self, gain: i64) {
        self.gain = gain * self.module_gain / 64;
    }

    /// Fades every channel out and hands the fading voices over, for music being replaced.
    pub(crate) fn into_fading(mut self) -> Vec<Voice> {
        self.stop();
        self.fading
    }

    /// Adds the music to `out` (interleaved stereo), advancing ticks exactly on time.
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        let mut done = 0;
        let frames = out.len() / 2;
        while done < frames {
            if self.frames_left == 0 {
                if !self.orders.is_empty() {
                    self.process_tick();
                }
                self.start_tick_timer();
            }
            let count = (frames - done).min(self.frames_left as usize);
            let part = &mut out[2 * done..2 * (done + count)];
            for channel in &mut self.channels {
                if let Some(voice) = &mut channel.voice {
                    voice.mix_into(part);
                    if voice.finished() {
                        channel.voice = None;
                    }
                }
            }
            for voice in &mut self.fading {
                voice.mix_into(part);
            }
            self.fading.retain(|voice| !voice.finished());
            done += count;
            self.frames_left -= u32::try_from(count).expect("at most a tick");
        }
    }

    /// The next tick's length: FMOD's whole number of 44.1 kHz samples, converted to our rate
    /// exactly by carrying the remainder.
    fn start_tick_timer(&mut self) {
        let fmod_samples = FMOD_RATE * 5 / (2 * u32::from(self.tempo));
        let scaled = fmod_samples * AUDIO_SAMPLE_RATE + self.remainder;
        self.frames_left = scaled / FMOD_RATE;
        self.remainder = scaled % FMOD_RATE;
    }

    /// Moves past 254 and 255, which FMOD drops from the order list.
    fn skip_marker_orders(&mut self) {
        while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
            self.order += 1;
        }
        if self.order >= self.orders.len() {
            // The song loops from its start, as FMOD's looping music does.
            self.order = 0;
            while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
                self.order += 1;
            }
        }
    }

    fn process_tick(&mut self) {
        if self.tick == 0 {
            self.process_row();
        } else {
            for index in 0..self.channels.len() {
                self.channel_tick(index);
            }
        }
        for index in 0..self.channels.len() {
            self.update_voice(index);
        }
        self.tick += 1;
        if self.tick >= self.speed {
            self.tick = 0;
            self.next_row();
        }
    }

    fn next_row(&mut self) {
        if let Some((order, row)) = self.jump.take() {
            self.order = order;
            self.row = row;
            self.skip_marker_orders();
            return;
        }
        self.row += 1;
        if self.row >= s3m::ROWS {
            self.row = 0;
            self.order += 1;
            self.skip_marker_orders();
        }
    }

    fn process_row(&mut self) {
        let Some(&pattern) = self.orders.get(self.order) else {
            return;
        };
        let cells = self.patterns[usize::from(pattern)].rows[self.row];
        for (index, cell) in cells.iter().enumerate() {
            if !self.channels[index].enabled {
                continue;
            }
            let channel = &mut self.channels[index];
            channel.command = cell.command;
            channel.info = cell.info;
            channel.period_delta = 0;
            if cell.command == command('S') && cell.info >> 4 == 0xD && cell.info & 0xF > 0 {
                channel.delayed = Some((cell.info & 0xF, *cell));
                continue;
            }
            channel.delayed = None;
            self.start_cell(index, cell);
            self.row_effect(index, cell);
        }
    }

    /// The note, instrument and volume of a cell.
    fn start_cell(&mut self, index: usize, cell: &Cell) {
        let tone_porta = cell.command == command('G');
        if cell.instrument != 0 {
            let channel = &mut self.channels[index];
            channel.instrument = cell.instrument;
            if let Some(Some(sample)) = self.samples.get(usize::from(cell.instrument) - 1) {
                channel.volume = sample.volume;
            }
        }
        if cell.note == NOTE_CUT {
            self.cut(index);
        } else if cell.note != NO_NOTE {
            let instrument = self.channels[index].instrument;
            if let Some(Some(sample)) = usize::from(instrument)
                .checked_sub(1)
                .and_then(|i| self.samples.get(i))
            {
                let period = note_period(cell.note, sample.c2spd);
                let channel = &mut self.channels[index];
                if tone_porta && channel.voice.is_some() {
                    channel.target_period = period;
                } else {
                    let offset = if cell.command == command('O') {
                        if cell.info != 0 {
                            channel.offset = cell.info;
                        }
                        u32::from(channel.offset) * 256
                    } else {
                        0
                    };
                    let voice = Voice::new(Arc::clone(&sample.data), sample.looping, offset);
                    if let Some(mut old) = channel.voice.replace(voice) {
                        old.release();
                        self.fading.push(old);
                    }
                    let channel = &mut self.channels[index];
                    channel.period = period;
                    channel.target_period = period;
                    channel.vibrato_position = 0;
                    channel.retrigger_count = 0;
                }
            } else if instrument != 0 && !(tone_porta && self.channels[index].voice.is_some()) {
                // FMOD plays a note of an empty sample slot as silence: the note sounding stops.
                self.cut(index);
            }
        }
        if let Some(volume) = cell.volume {
            self.channels[index].volume = i32::from(volume);
        }
    }

    fn cut(&mut self, index: usize) {
        if let Some(mut voice) = self.channels[index].voice.take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    /// Commands that act on the row's first tick.
    fn row_effect(&mut self, index: usize, cell: &Cell) {
        let info = cell.info;
        let channel = &mut self.channels[index];
        match letter(cell.command) {
            'A' if info > 0 => self.speed = info,
            'T' if info >= 0x20 => self.tempo = info,
            'B' => {
                let row = self.jump.map_or(0, |(_, row)| row);
                self.jump = Some((usize::from(info), row));
            }
            'C' => {
                let row = usize::from((info >> 4) * 10 + (info & 0xF)).min(s3m::ROWS - 1);
                let order = self.jump.map_or(self.order + 1, |(order, _)| order);
                self.jump = Some((order, row));
            }
            'D' | 'K' => {
                if info != 0 {
                    channel.volume_slide = info;
                }
                let slide = channel.volume_slide;
                // Fine slides (DxF up, DFy down) act once, on this tick.
                if slide & 0x0F == 0x0F && slide >> 4 != 0 {
                    channel.volume = (channel.volume + i32::from(slide >> 4)).min(64);
                } else if slide >> 4 == 0x0F && slide & 0x0F != 0 {
                    channel.volume = (channel.volume - i32::from(slide & 0x0F)).max(0);
                }
            }
            'E' | 'F' => {
                if info != 0 {
                    channel.porta = info;
                }
                let porta = channel.porta;
                let sign = if letter(cell.command) == 'E' { 1 } else { -1 };
                // EFx fine (x * 4), EEx extra fine (x), on this tick only.
                match porta >> 4 {
                    0xF => channel.period += sign * 4 * i32::from(porta & 0xF),
                    0xE => channel.period += sign * i32::from(porta & 0xF),
                    _ => {}
                }
                channel.period = channel.period.clamp(MIN_PERIOD, MAX_PERIOD);
            }
            'G' => {
                if info != 0 {
                    channel.tone_porta = info;
                }
            }
            'H' => {
                if info >> 4 != 0 {
                    channel.vibrato_speed = info >> 4;
                }
                if info & 0xF != 0 {
                    channel.vibrato_depth = info & 0xF;
                }
            }
            'Q' if info != 0 => channel.retrigger = info,
            _ => {}
        }
    }

    /// Commands that act on every tick but the first.
    fn channel_tick(&mut self, index: usize) {
        if !self.channels[index].enabled {
            return;
        }
        if let Some((at, cell)) = self.channels[index].delayed {
            if self.tick == at {
                self.channels[index].delayed = None;
                self.start_cell(index, &cell);
            }
            return;
        }
        let channel = &mut self.channels[index];
        channel.period_delta = 0;
        match letter(channel.command) {
            'D' => volume_slide(channel),
            'K' => {
                volume_slide(channel);
                vibrato_tick(channel);
            }
            'E' | 'F' => {
                let porta = channel.porta;
                if porta >> 4 < 0xE {
                    let sign = if letter(channel.command) == 'E' {
                        1
                    } else {
                        -1
                    };
                    channel.period = (channel.period + sign * 4 * i32::from(porta))
                        .clamp(MIN_PERIOD, MAX_PERIOD);
                }
            }
            'G' => {
                let speed = 4 * i32::from(channel.tone_porta);
                if channel.period < channel.target_period {
                    channel.period = (channel.period + speed).min(channel.target_period);
                } else {
                    channel.period = (channel.period - speed).max(channel.target_period);
                }
            }
            'H' => vibrato_tick(channel),
            // Without an interval nothing repeats, and nothing is counted.
            'Q' if channel.retrigger & 0xF != 0 => {
                let interval = channel.retrigger & 0xF;
                channel.retrigger_count += 1;
                if channel.retrigger_count >= interval {
                    channel.retrigger_count = 0;
                    channel.volume = retrigger_volume(channel.volume, channel.retrigger >> 4);
                    let sample = usize::from(channel.instrument)
                        .checked_sub(1)
                        .and_then(|i| self.samples.get(i))
                        .and_then(Option::as_ref);
                    if let Some(sample) = sample {
                        let voice = Voice::new(Arc::clone(&sample.data), sample.looping, 0);
                        if let Some(mut old) = channel.voice.replace(voice) {
                            old.release();
                            self.fading.push(old);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Pushes the channel's pitch and volume into its voice.
    fn update_voice(&mut self, index: usize) {
        let global = i64::from(self.global_volume);
        let gain = self.gain;
        let channel = &mut self.channels[index];
        let Some(voice) = &mut channel.voice else {
            return;
        };
        let period = (channel.period + channel.period_delta).clamp(MIN_PERIOD, MAX_PERIOD);
        voice.set_frequency(CLOCK / u32::try_from(period).expect("positive"));
        let volume = i64::from(channel.volume) * global * gain / (64 * 64);
        let left = volume * (255 - channel.pan) / 255;
        let right = volume * channel.pan / 255;
        voice.set_volume(left, right);
    }

    #[cfg(test)]
    fn position(&self) -> (usize, usize, u8) {
        (self.order, self.row, self.tick)
    }
}

fn command(letter: char) -> u8 {
    letter as u8 - b'@'
}

fn letter(command: u8) -> char {
    if (1..=26).contains(&command) {
        char::from(b'@' + command)
    } else {
        ' '
    }
}

/// Scream Tracker's period of a note byte (octave in the high nibble) for a sample's C2SPD.
/// The octave shift comes last, so high notes keep their precision.
fn note_period(note: u8, c2spd: u32) -> i32 {
    let (octave, semitone) = (u32::from(note >> 4), usize::from(note & 0xF).min(11));
    let period =
        (8363 * 16 * u64::from(S3M_PERIODS[semitone]) / u64::from(c2spd.max(1))) >> octave.min(9);
    i32::try_from(period)
        .unwrap_or(MAX_PERIOD)
        .clamp(MIN_PERIOD, MAX_PERIOD)
}

fn volume_slide(channel: &mut Channel) {
    let slide = channel.volume_slide;
    let (up, down) = (slide >> 4, slide & 0x0F);
    if down == 0 && up != 0 {
        channel.volume = (channel.volume + i32::from(up)).min(64);
    } else if up == 0 && down != 0 {
        channel.volume = (channel.volume - i32::from(down)).max(0);
    }
}

fn vibrato_tick(channel: &mut Channel) {
    let wave = vibrato(channel.vibrato_position);
    channel.period_delta = (wave * i32::from(channel.vibrato_depth)) >> 5;
    channel.vibrato_position = (channel.vibrato_position + channel.vibrato_speed) & 63;
}

/// Qxy's volume change `x` applied to a volume 0..=64.
fn retrigger_volume(volume: i32, change: u8) -> i32 {
    let changed = match change {
        1..=5 => volume - (1 << (change - 1)),
        6 => volume * 2 / 3,
        7 => volume / 2,
        9..=13 => volume + (1 << (change - 9)),
        14 => volume * 3 / 2,
        15 => volume * 2,
        _ => volume,
    };
    changed.clamp(0, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{Channel as S3mChannel, Pattern, Sample};

    use crate::audio::mixer::UNITY;

    const C4: u8 = 0x40;

    fn module(rows: &[(usize, usize, Cell)], speed: u8, tempo: u8) -> Module {
        let mut pattern = Pattern {
            rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
        };
        for &(row, channel, cell) in rows {
            pattern.rows[row][channel] = cell;
        }
        let mut channels = [S3mChannel::default(); s3m::CHANNELS];
        channels[0] = S3mChannel {
            enabled: true,
            pan: 3,
        };
        channels[1] = S3mChannel {
            enabled: true,
            pan: 12,
        };
        Module {
            title: "Test".into(),
            orders: vec![0, 1],
            initial_speed: speed,
            initial_tempo: tempo,
            global_volume: 64,
            master_volume: 48,
            stereo: false,
            channels,
            samples: vec![Sample {
                name: "Tone".into(),
                c2spd: 8363,
                volume: 32,
                looped: Some((0, 100)),
                data: vec![10_000; 100],
            }],
            patterns: vec![pattern.clone(), pattern],
        }
    }

    fn cell(note: u8, instrument: u8, volume: Option<u8>, command: char, info: u8) -> Cell {
        Cell {
            note,
            instrument,
            volume,
            command: if command == ' ' {
                0
            } else {
                super::command(command)
            },
            info,
        }
    }

    fn play(music: &mut Music, frames: usize) -> Vec<i64> {
        let mut out = vec![0; 2 * frames];
        music.mix_into(&mut out);
        out
    }

    #[test]
    fn a_tick_lasts_fmods_whole_number_of_samples() {
        // At tempo 125 a tick is 882 samples at 44.1 kHz, exactly 20 ms: 960 of ours.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 960 * 6);
        assert_eq!(music.position(), (0, 1, 0));
        // At tempo 141 FMOD counts 781 samples (not 781.9) per tick, which is why the menu
        // music runs 0.1 % fast in the original: 147 ticks are exactly 124 960 of our samples.
        let mut faster = Music::new(&module(&[], 1, 141), UNITY, 0);
        play(&mut faster, 124_960);
        assert_eq!(
            faster.position(),
            (0, 19, 0),
            "147 rows: both patterns, then the song loops to row 19 of its start"
        );
    }

    #[test]
    fn middle_c_plays_at_the_samples_c2spd() {
        assert_eq!(note_period(C4, 8363), 1712);
        assert_eq!(CLOCK / 1712, 8362);
        assert_eq!(
            note_period(0x50, 8363),
            856,
            "an octave up halves the period"
        );
        assert_eq!(
            note_period(C4, 16_726),
            856,
            "a doubled C2SPD sounds an octave up"
        );
    }

    #[test]
    fn notes_play_with_the_samples_volume_and_the_volume_column_overrides_it() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(C4, 1, Some(64), ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        let out = play(&mut music, 960);
        // Mono: centred, 32 / 64 of full volume on each side, and the module's master volume
        // 48 / 64 on top.
        let expected = 10_000 * 32 / 64 * 127 / 255 * 48 / 64;
        assert!(
            (out[2 * 900] - expected).abs() <= 2,
            "{} vs {expected}",
            out[2 * 900]
        );
        let louder = play(&mut music, 960);
        assert!(
            (louder[2 * 900] - 2 * expected).abs() <= 4,
            "{}",
            louder[2 * 900]
        );
    }

    #[test]
    fn speed_tempo_jump_and_break_commands_steer_the_song() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(NO_NOTE, 0, None, 'A', 2)),
                    (0, 1, cell(NO_NOTE, 0, None, 'C', 0x10)),
                ],
                6,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 2);
        assert_eq!(
            music.position(),
            (1, 10, 0),
            "speed 2, then a break to row 10 of the next order"
        );
        let mut jumping = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'B', 0))], 1, 125),
            UNITY,
            0,
        );
        play(&mut jumping, 960);
        assert_eq!(jumping.position(), (0, 0, 0), "B00 loops the first order");
        // As in minifmod, a new tempo already sets the length of the tick that sets it.
        let mut tempo = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'T', 250))], 1, 125),
            UNITY,
            0,
        );
        play(&mut tempo, 480 * 3);
        assert_eq!(
            tempo.position(),
            (0, 3, 0),
            "tempo 250: ticks of 480 samples"
        );
    }

    #[test]
    fn stereo_modules_play_each_channel_fully_on_its_side_and_twice_as_loud() {
        // FMOD 3 pans a stereo module's channels hard left or right, and plays them at twice a
        // mono module's level: only that matches the stereo image and the loudness of the
        // original's menu music, recorded at two music volumes (docs/verification/m1b.md).
        let mut stereo = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        stereo.stereo = true;
        let mut music = Music::new(&stereo, UNITY, 0);
        let out = play(&mut music, 960);
        let full = 10_000 * 48 / 64 * 2;
        assert!((out[2 * 900] - full).abs() <= 4, "left {}", out[2 * 900]);
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn a_song_can_start_at_a_later_order() {
        // The game starts the menu music at order 45 (musicSetOrder), not at its beginning.
        let mut later = module(&[], 1, 125);
        later.orders = vec![0, s3m::ORDER_END, 1];
        let music = Music::new(&later, UNITY, 2);
        assert_eq!(music.position(), (2, 0, 0));
        let at_marker = Music::new(&later, UNITY, 1);
        assert_eq!(at_marker.position(), (2, 0, 0), "a marker is skipped");
    }

    #[test]
    fn playback_runs_on_through_section_markers_as_fmod_does() {
        // FMOD drops 254 and 255 from the order list (the game numbers orders for
        // FMUSIC_SetOrder without them), so a pattern before a 255 is followed by the next
        // section, not by the song's start.
        let mut base = module(&[], 1, 125);
        base.orders = vec![0, s3m::ORDER_END, s3m::ORDER_SKIP, 1];
        let mut music = Music::new(&base, UNITY, 0);
        play(&mut music, 960 * s3m::ROWS);
        assert_eq!(music.position(), (3, 0, 0));
    }

    /// A module whose one sample rises steadily (0, 8, 16, ...), so the output shows how far
    /// into the sample a voice is.
    fn rising(rows: &[(usize, usize, Cell)], speed: u8) -> Module {
        let mut rising = module(rows, speed, 125);
        rising.samples[0].data = (0..4000).map(|i| i16::try_from(i * 8).unwrap()).collect();
        rising.samples[0].looped = None;
        rising
    }

    #[test]
    fn the_sample_offset_starts_a_note_further_into_its_sample() {
        // O (545 times in TR5) starts notes part way into their sample; ignoring it would play
        // the sample's beginning instead.
        let mut plain = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 6),
            UNITY,
            0,
        );
        let mut offset = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), 'O', 8))], 6),
            UNITY,
            0,
        );
        let (plain, offset) = (play(&mut plain, 300), play(&mut offset, 300));
        // 8 * 256 = 2048 samples in: about 2100 instead of about 50 at frame 299.
        assert!(plain[2 * 299] > 0);
        assert!(
            offset[2 * 299] > 20 * plain[2 * 299],
            "{} vs {}",
            offset[2 * 299],
            plain[2 * 299]
        );
    }

    #[test]
    fn vibrato_with_volume_slide_does_both() {
        // K (277 times in TR1) keeps an earlier H's vibrato going while it slides the volume;
        // doing only one of the two freezes the note or its level.
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, Some(10), 'H', 0x48)),
                    (1, 0, cell(NO_NOTE, 0, None, 'K', 0x20)),
                ],
                4,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        let position = music.channels[0].vibrato_position;
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        assert_eq!(
            music.channels[0].vibrato_position,
            position + 3 * 4,
            "the vibrato goes on at speed 4"
        );
    }

    #[test]
    fn portamento_up_lowers_the_period_and_its_fine_form_acts_once() {
        // F (252 times in TR5) slides notes up; the wrong direction, or FFx on every tick,
        // would detune whole phrases.
        let mut up = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut up, 960 * 3);
        assert_eq!(up.channels[0].period, 1712 - 2 * 8, "2 ticks of 4 * 2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 0xF3))], 3, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 3);
        assert_eq!(
            fine.channels[0].period,
            1712 - 3 * 4,
            "FF3: once, on the first tick"
        );
    }

    #[test]
    fn retrigger_restarts_the_note_at_its_interval_and_changes_its_volume() {
        // Q (90 times in TR9) drums a note several times per row; without the restarts, or the
        // volume change, a roll becomes one long note.
        let mut music = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(20), 'Q', 0xA3))], 7),
            UNITY,
            0,
        );
        let out = play(&mut music, 960 * 7);
        // Restarted at tick 3: 200 frames later a voice plays again, near the sample's start.
        let (restarted, before) = (out[2 * (3 * 960 + 200)], out[2 * (3 * 960 - 1)]);
        assert!(
            restarted > 0 && restarted < before / 4,
            "{restarted} vs {before}"
        );
        assert_eq!(
            music.channels[0].volume, 24,
            "+2 at each restart, on ticks 3 and 6"
        );
    }

    #[test]
    fn a_retrigger_without_an_interval_never_overflows() {
        // Q00 before any Q with an interval repeats nothing; counting its ticks anyway overflowed
        // after 255 of them and crashed the game in the middle of the music.
        let mut long = module(
            &[
                (0, 0, cell(C4, 1, Some(64), 'Q', 0)),
                (1, 0, cell(NO_NOTE, 0, None, 'Q', 0)),
            ],
            200,
            125,
        );
        long.orders = vec![0];
        let mut music = Music::new(&long, UNITY, 0);
        play(&mut music, 960 * 400);
        assert_eq!(music.position(), (0, 2, 0));
    }

    #[test]
    fn a_song_of_markers_only_stays_silent() {
        // An order list without a pattern must not hang the player looking for one.
        let mut base = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        base.orders = vec![s3m::ORDER_SKIP, s3m::ORDER_END];
        let mut music = Music::new(&base, UNITY, 0);
        assert!(play(&mut music, 960 * 4).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn volume_slides_act_after_the_first_tick_and_fine_ones_on_it() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x20))], 4, 125),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x3F))], 4, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 4);
        assert_eq!(fine.channels[0].volume, 13, "one fine step of +3");
    }

    #[test]
    fn portamentos_move_the_period() {
        let mut down = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'E', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut down, 960 * 3);
        assert_eq!(down.channels[0].period, 1712 + 2 * 8);
        let mut towards = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(0x50, 1, None, 'G', 0xFF)),
                ],
                3,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut towards, 960 * 6);
        assert_eq!(
            towards.channels[0].period, 856,
            "the tone portamento stops at its target"
        );
    }

    #[test]
    fn vibrato_wobbles_around_the_note() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'H', 0x48))], 6, 125),
            UNITY,
            0,
        );
        let mut deltas = Vec::new();
        for _ in 0..6 {
            play(&mut music, 960);
            deltas.push(music.channels[0].period_delta);
        }
        assert_eq!(deltas[0], 0, "no vibrato on the first tick");
        assert!(
            deltas[1..].iter().all(|&delta| delta >= 0) && deltas[5] > 0,
            "{deltas:?}"
        );
        assert_eq!(
            music.channels[0].period, 1712,
            "the note itself does not move"
        );
    }

    #[test]
    fn a_note_of_an_empty_sample_slot_silences_the_channel() {
        // The menu music's order 47 starts with notes of a sample slot its author emptied;
        // FMOD plays them as silence. Ignoring them left the channel's looping note playing,
        // brought back up by their volume: a stray tone in the menu.
        let mut emptied = module(
            &[
                (0, 0, cell(C4, 1, Some(64), ' ', 0)),
                (1, 0, cell(C4, 2, Some(32), ' ', 0)),
            ],
            1,
            125,
        );
        emptied.samples.push(Sample::default());
        let mut music = Music::new(&emptied, UNITY, 0);
        play(&mut music, 960);
        let out = play(&mut music, 960);
        assert!(out[2 * 900..].iter().all(|&sample| sample == 0));
    }

    #[test]
    fn setting_the_order_goes_on_at_its_first_row() {
        // The Hall of Fame moves the menu music to its own part (order 81) this way.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 5000);
        music.set_order(1);
        assert_eq!(music.position(), (1, 0, 0));
        assert_eq!(music.order(), 1);
    }

    #[test]
    fn note_delay_and_cut_and_retrigger() {
        let mut delayed = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'S', 0xD2))], 4, 125),
            UNITY,
            0,
        );
        play(&mut delayed, 960 * 2);
        assert!(delayed.channels[0].voice.is_none(), "not before tick 2");
        play(&mut delayed, 960);
        assert!(delayed.channels[0].voice.is_some());
        let mut cut = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(NOTE_CUT, 0, None, ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut cut, 960 * 2);
        assert!(cut.channels[0].voice.is_none());
        assert_eq!(retrigger_volume(40, 0xF), 64);
        assert_eq!(retrigger_volume(40, 0x7), 20);
        assert_eq!(retrigger_volume(40, 0x3), 36);
    }

    #[test]
    fn the_same_module_always_renders_the_same_samples() {
        let rows = [
            (0, 0, cell(C4, 1, Some(40), 'H', 0x46)),
            (8, 1, cell(0x45, 1, None, 'Q', 0x93)),
        ];
        let render = || {
            let mut music = Music::new(&module(&rows, 3, 131), UNITY, 0);
            play(&mut music, 48_000)
        };
        assert_eq!(render(), render());
    }
}
```

<!-- write: crates/core/tests/menu.rs -->
```rust
//! The main menu on synthetic assets (spec M2a §3.2–§3.5): what a player of the original sees
//! and hears while moving through it. Every picture of the fixture has a colour of its own
//! (`common`), so a pixel tells which font, cursor frame or screen is drawn there.

mod common;

use common::{
    ARROW, BACKGROUND, BIG_A, BIG_B, BIG_D, CREDITS, CURSOR, END, FAME_TITLE, KNOB, MEDIUM,
    RECORDS_TITLE, SLIDER, SMALL, SNAPSHOT,
};
use deadrally_core::{Game, InputEvent, Key, PadAxis, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::dr_cfg::DrCfg;
use deadrally_gamedata::haf::Animation;
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::Bank;

/// Without an intro: two logos of 25 + 180 + 26 ticks, the title's 25-tick fade-in, its
/// 26-tick fade to black and the menu's 50-tick fade-in.
const MENU_SHOWN: u32 = 2 * (25 + 180 + 26) + 25 + 26 + 50;
/// Where the main menu and the start submenu stand.
const MAIN: (usize, usize) = (145, 124);
const START: (usize, usize) = (109, 171);
const CONFIGURE: (usize, usize) = (95, 146);
/// Inside the exit question's "yes" and "no".
const YES: (usize, usize) = (212, 241);
const NO: (usize, usize) = (382, 241);

/// Which sides an effect sounded on: the fixture's back sound is left only, the move sound
/// right only, the choose sound both.
const SILENT: (bool, bool) = (false, false);
const BACK: (bool, bool) = (true, false);
const MOVE: (bool, bool) = (false, true);
const CHOOSE: (bool, bool) = (true, true);

fn picture(pixel: u8, width: u32, height: u32) -> Picture {
    Picture {
        image: Image::new(width, height, vec![pixel; (width * height) as usize]),
        palette: Palette::BLACK,
    }
}

/// A module that is silent, or plays one endless tone from order 45 on, where the original
/// starts the menu music.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let silent = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    let mut playing = silent.clone();
    playing.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone {
            [vec![1; 45], vec![0]].concat()
        } else {
            Vec::new()
        },
        initial_speed: 6,
        initial_tempo: 125,
        global_volume: 64,
        master_volume: 48,
        stereo: false,
        channels,
        samples: vec![Sample {
            name: "Tone".into(),
            c2spd: 8363,
            volume: 64,
            looped: Some((0, 1000)),
            data: vec![4000; 1000],
        }],
        patterns: vec![playing, silent],
    }
}

fn assets() -> Assets {
    Assets {
        intro: Animation::from_frames(Vec::new(), Vec::new()),
        letterbox: picture(0, 320, 200),
        apogee: picture(1, 4, 3),
        remedy: picture(2, 4, 3),
        title: picture(3, 640, 480),
        intro_music: music(false),
        intro_effects: Bank {
            linear_frequencies: true,
            instruments: Vec::new(),
        },
        menu_music: music(false),
        menu: common::menu_assets(),
    }
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// A game in the main menu, its fade-in over.
fn in_menu(assets: Assets) -> Game {
    in_menu_with(assets, common::config())
}

fn in_menu_with(assets: Assets, config: DrCfg) -> Game {
    let mut game = Game::new(assets, config);
    run(&mut game, MENU_SHOWN);
    game
}

fn press(game: &mut Game, key: Key) {
    game.input(InputEvent::Key { key, pressed: true });
    game.input(InputEvent::Key {
        key,
        pressed: false,
    });
}

/// Presses `key` and runs two menu passes: the first reads it, the second shows the result
/// everywhere (the exit question copies its words to the screen a pass later).
fn step(game: &mut Game, key: Key) {
    press(game, key);
    run(game, 4);
}

/// Waits for earlier effects to end, presses `key`, and tells which sides sounded.
fn sound_after(game: &mut Game, key: Key) -> (bool, bool) {
    run(game, 100);
    game.take_audio(&mut Vec::new());
    step(game, key);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    let side = |side: usize| audio.iter().skip(side).step_by(2).any(|&s| s != 0);
    (side(0), side(1))
}

fn pixel(game: &Game, (x, y): (usize, usize)) -> u8 {
    game.frame().pixels[y * 640 + x]
}

/// The font each of a menu's six rows is drawn in, read inside the row's glyph.
fn row_fonts(game: &Game, (x, y): (usize, usize)) -> Vec<u8> {
    (0..6)
        .map(|row| pixel(game, (x + 33, y + 15 + 28 * row)))
        .collect()
}

/// The main menu's highlighted row.
fn selected(game: &Game) -> usize {
    let fonts = row_fonts(game, MAIN);
    fonts
        .iter()
        .position(|&font| font == BIG_A)
        .unwrap_or_else(|| panic!("no row highlighted: {fonts:?}"))
}

/// How the shown palette makes `colour`.
fn colour(game: &Game, colour: u8) -> [u8; 3] {
    game.frame().palette[usize::from(colour)]
}

#[test]
fn the_menu_fades_in_after_the_title_and_stays_just_below_full_brightness() {
    // The original's fade-in stops at 98 %: white shows as 62, never 63.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, MENU_SHOWN - 1);
    let before = colour(&game, BACKGROUND);
    game.tick();
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (640, 480, (4, 3))
    );
    assert_eq!(pixel(&game, (5, 5)), BACKGROUND);
    assert!(before[0] < 62, "{before:?}");
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    run(&mut game, 300);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
}

#[test]
fn the_main_menu_highlights_start_and_dims_multiplayer() {
    let game = in_menu(assets());
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
    let cursor = pixel(&game, (MAIN.0 + 15, MAIN.1 + 16));
    assert!((CURSOR..CURSOR + 50).contains(&cursor), "{cursor}");
}

#[test]
fn the_bottom_panel_shows_the_start_up_lines_with_a_gap_before_the_last() {
    // `mainMenu` pushes three lines, an empty one and the last into the 22-line panel, which
    // shows its last six at (12, 378 + 15k) in small B.
    let game = in_menu(assets());
    let line = |k: usize| pixel(&game, (13, 378 + 15 * k + 8));
    assert_eq!(
        (0..6).map(line).collect::<Vec<_>>(),
        [
            BACKGROUND, SMALL[1], SMALL[1], SMALL[1], BACKGROUND, SMALL[1]
        ]
    );
}

#[test]
fn up_and_down_move_the_highlight_past_the_inactive_row_and_around() {
    let mut game = in_menu(assets());
    let rows: Vec<usize> = [Key::Down, Key::Down, Key::Up, Key::Up, Key::Up, Key::Down]
        .into_iter()
        .map(|key| {
            step(&mut game, key);
            selected(&game)
        })
        .collect();
    assert_eq!(rows, [2, 3, 2, 0, 5, 0]);
    let fonts = row_fonts(&game, MAIN);
    assert_eq!(fonts, [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]);
}

#[test]
fn each_move_sounds_and_other_keys_are_ignored() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(sound_after(&mut game, Key::Up), MOVE);
    assert_eq!(sound_after(&mut game, Key::A), SILENT);
    assert_eq!(selected(&game), 0);
}

#[test]
fn escape_jumps_to_exit_once() {
    // Escape in the main menu only moves the highlight to the last row; there it does
    // nothing, so a player cannot leave the game by pressing it twice.
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Escape), MOVE);
    assert_eq!(selected(&game), 5);
    assert_eq!(sound_after(&mut game, Key::Escape), SILENT);
    assert_eq!(selected(&game), 5);
    assert!(!game.quit_requested());
}

#[test]
fn the_cursor_turns_one_frame_every_menu_pass() {
    // A pass waits two ticks; the cursor runs through its 50 frames in 100 ticks.
    let mut game = in_menu(assets());
    let at = (MAIN.0 + 15, MAIN.1 + 16);
    let frames: Vec<u8> = (0..6)
        .map(|_| {
            game.tick();
            pixel(&game, at)
        })
        .collect();
    let k = frames[0];
    assert_eq!(frames, [k, k + 1, k + 1, k + 2, k + 2, k + 3]);
    run(&mut game, 100);
    assert_eq!(pixel(&game, at), k + 3);
}

#[test]
fn the_highlights_pulse_while_the_menu_waits() {
    // Entries 16–31 go down from 100 % to 49 % and back up in 34 ticks.
    let mut game = in_menu(assets());
    let levels: Vec<u8> = (0..35)
        .map(|_| {
            game.tick();
            colour(&game, 16)[0]
        })
        .collect();
    assert_eq!(levels[34], levels[0]);
    let lowest = *levels.iter().min().unwrap();
    assert!((30..=31).contains(&lowest), "{levels:?}");
    assert_eq!(*levels.iter().max().unwrap(), 63, "{levels:?}");
}

#[test]
fn a_held_key_moves_the_highlight_again_after_half_a_second() {
    let mut game = in_menu(assets());
    game.input(InputEvent::Key {
        key: Key::Down,
        pressed: true,
    });
    run(&mut game, 30);
    assert_eq!(selected(&game), 2, "one move in the first 420 ms");
    run(&mut game, 14);
    assert_ne!(selected(&game), 2, "SDL's repeat has started");
}

#[test]
fn the_gamepad_moves_and_chooses_like_the_keys() {
    // With the gamepad switched on in dr.cfg. `eventDetected` treats a push as fresh when it
    // was last called within 400 ms without the stick: a pass of the menu first.
    let mut config = common::config();
    config.set_use_joystick(1);
    let mut game = in_menu_with(assets(), config);
    run(&mut game, 2);
    game.input(InputEvent::PadAxis {
        axis: PadAxis::StickY,
        value: 20_000,
    });
    run(&mut game, 20);
    assert_eq!(
        selected(&game),
        2,
        "a push moves once, then holds off for 700 ms"
    );
    let stick = |game: &mut Game, value: i16| {
        game.input(InputEvent::PadAxis {
            axis: PadAxis::StickY,
            value,
        });
        run(game, 4);
    };
    stick(&mut game, 0);
    stick(&mut game, -20_000);
    stick(&mut game, 0);
    assert_eq!(selected(&game), 0);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: false,
    });
    run(&mut game, 4);
    assert_eq!(
        row_fonts(&game, MAIN)[0],
        BIG_D,
        "the main menu has lost focus"
    );
}

#[test]
fn start_opens_its_submenu_and_escape_closes_it() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(
        row_fonts(&game, START),
        [BIG_A, BIG_D, BIG_D, BIG_B, BIG_D, BIG_B],
        "only new game, load game and back are active at the first start"
    );
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(row_fonts(&game, START)[3], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Escape), BACK);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
}

#[test]
fn the_submenus_last_row_returns_and_starts_it_over_at_the_top() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    assert_eq!(row_fonts(&game, START)[5], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Space), CHOOSE);
    assert_eq!(selected(&game), 0, "back in the main menu");
    step(&mut game, Key::Enter);
    assert_eq!(row_fonts(&game, START)[0], BIG_A);
}

#[test]
fn the_hall_of_fame_does_nothing_when_chosen_until_m2c() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_B, BIG_A, BIG_B, BIG_B]
    );
}

#[test]
fn the_exit_question_starts_on_no_and_escape_answers_it() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Escape);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(sound_after(&mut game, Key::Left), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_A, BIG_B));
    assert_eq!(sound_after(&mut game, Key::Y), SILENT, "already on yes");
    assert_eq!(sound_after(&mut game, Key::N), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(sound_after(&mut game, Key::Escape), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_B, BIG_B, BIG_B, BIG_A]
    );
    run(&mut game, 1_000);
    assert!(!game.quit_requested());
}

/// Answers the exit question with yes; returns with the key not yet read.
fn answer_yes(game: &mut Game) {
    step(game, Key::Escape);
    step(game, Key::Enter);
    step(game, Key::Left);
    press(game, Key::Enter);
}

#[test]
fn yes_shows_the_end_screen_until_a_key_and_then_the_game_quits() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    // A pass reads the key; the menu takes 26 ticks to black, the end screen 25 to 96 %.
    run(&mut game, 2 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), END);
    assert_eq!(colour(&game, END), [0, 0, 60]);
    run(&mut game, 300);
    assert!(!game.quit_requested());
    press(&mut game, Key::Space);
    // The hold reads the key at once; the fade-out takes 26 ticks.
    run(&mut game, 26);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
    assert_eq!(colour(&game, END), [0, 0, 0]);
}

#[test]
fn the_end_screen_stops_waiting_after_560_ticks() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    run(&mut game, 2 + 26 + 25 + 560 + 25);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
}

#[test]
fn the_music_fades_out_with_the_end_screen() {
    let mut with_music = assets();
    with_music.menu_music = music(true);
    let mut game = in_menu(with_music);
    answer_yes(&mut game);
    run(&mut game, 100);
    let loudest = |game: &mut Game| {
        let mut audio = Vec::new();
        game.take_audio(&mut audio);
        audio.iter().map(|&s| s.unsigned_abs()).max().unwrap()
    };
    let held = loudest(&mut game);
    assert!(held > 0);
    press(&mut game, Key::Space);
    run(&mut game, 14);
    let halfway = loudest(&mut game);
    assert!(halfway < held, "{halfway} vs {held}");
    run(&mut game, 13);
    assert!(game.quit_requested());
    // The music's next tick takes the last step to silence.
    run(&mut game, 2);
    loudest(&mut game);
    run(&mut game, 1);
    assert_eq!(loudest(&mut game), 0, "silent once the game has ended");
}

#[test]
fn the_credits_show_two_screens_each_until_a_key_and_return_to_the_menu() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    assert_eq!(selected(&game), 4);
    press(&mut game, Key::Enter);
    // A pass reads the key; the menu fades out in 51 ticks, the first screen in in 25.
    run(&mut game, 2 + 51 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0]);
    assert_eq!(colour(&game, CREDITS[0]), [60, 0, 0]);
    run(&mut game, 500);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "it waits for a key");
    press(&mut game, Key::Space);
    run(&mut game, 1 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
    press(&mut game, Key::Space);
    // Back to the menu as it was, fading in over 50 ticks.
    run(&mut game, 1 + 26 + 50);
    assert_eq!(selected(&game), 4);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    step(&mut game, Key::Down);
    assert_eq!(selected(&game), 5, "the menu works again");
}

#[test]
fn a_key_during_a_credits_fade_in_moves_on_as_soon_as_it_is_done() {
    // The original reads the key before the screen's first wait, so an impatient player does
    // not see it held at all.
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 51 + 10);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "fading in");
    press(&mut game, Key::Space);
    // The rest of the fade-in, the fade to black and the second screen's fade-in.
    run(&mut game, 15 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
}

/// A game in Configure, the file written at start-up taken.
fn in_configure(config: DrCfg) -> Game {
    let mut game = in_menu_with(assets(), config);
    game.take_config();
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    game
}

/// The `dr.cfg` the game hands out now, if it writes one.
fn written(game: &mut Game) -> Option<DrCfg> {
    game.take_config()
        .map(|bytes| DrCfg::parse(&bytes).unwrap())
}

/// Where the volume popup's knob shows level `level`.
fn knob_at(level: usize) -> (usize, usize) {
    (329 + level + 5, 260)
}

#[test]
fn dr_cfg_is_written_at_start_up_counting_the_start() {
    // mainMenu counts every start and writes the file before the intro.
    let mut config = common::config();
    config.set_times_played(4);
    let mut game = Game::new(assets(), config);
    game.tick();
    assert_eq!(written(&mut game).map(|cfg| cfg.times_played()), Some(5));
    run(&mut game, 50);
    assert!(game.take_config().is_none(), "only once");
}

#[test]
fn configure_opens_over_the_dimmed_main_menu_and_escape_returns_writing_dr_cfg() {
    let mut game = in_configure(common::config());
    assert_eq!(
        row_fonts(&game, CONFIGURE),
        [BIG_A, BIG_B, BIG_B, BIG_B, BIG_B, BIG_B]
    );
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert!(game.take_config().is_none());
    assert_eq!(sound_after(&mut game, Key::Escape), BACK);
    assert_eq!(selected(&game), 2, "back on the configure row");
    assert!(
        written(&mut game).is_some(),
        "leaving Configure writes dr.cfg"
    );
}

#[test]
fn the_music_volume_moves_by_two_levels_and_is_kept_on_enter() {
    // The level is the volume / 512: 64 for the default 50 %; three steps left make 58.
    let mut game = in_configure(common::config());
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, (320, 255)), SLIDER);
    assert_eq!(pixel(&game, knob_at(64)), KNOB);
    for _ in 0..3 {
        press(&mut game, Key::Left);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(58)), KNOB);
    assert_eq!(sound_after(&mut game, Key::Enter), BACK);
    assert_eq!(row_fonts(&game, CONFIGURE)[0], BIG_A, "back in Configure");
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.music_volume(), 58 * 512);
    assert_eq!(config.effects_volume(), 0xC000, "the effects untouched");
}

#[test]
fn a_volume_stops_at_its_ends() {
    let mut game = in_configure(common::config());
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    for _ in 0..80 {
        press(&mut game, Key::Right);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(128)), KNOB);
    for _ in 0..80 {
        press(&mut game, Key::Left);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(0)), KNOB);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().effects_volume(), 0);
}

#[test]
fn define_keyboard_takes_the_next_key_for_the_chosen_control() {
    let mut game = in_configure(common::config());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    run(&mut game, 10);
    step(&mut game, Key::Q);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.key(1), 0x10, "brake on Q");
    assert_eq!(config.key(0), 0, "accelerate untouched");
}

#[test]
fn define_gamepad_waits_for_the_pad_to_settle_and_enter_means_none() {
    // While it waits, the original's key reads leave the gamepad alone: a button is taken as
    // an input, not as Enter or Escape.
    let mut config = common::config();
    config.set_use_joystick(1);
    config.set_pad(0, 5);
    config.set_pad(1, 4);
    let mut game = in_configure(config);
    game.input(InputEvent::PadConnected { connected: true });
    for _ in 0..3 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    let button = |game: &mut Game, pressed: bool| {
        game.input(InputEvent::PadButton {
            button: PadButton::Y,
            pressed,
        });
    };
    button(&mut game, true);
    run(&mut game, 30);
    button(&mut game, false);
    run(&mut game, 30);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    run(&mut game, 4);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.pad(0), 8, "button 4");
    assert_eq!(config.pad(1), 0, "none");
}

#[test]
fn the_gamepad_switch_without_a_gamepad_shows_the_popup_until_a_key() {
    let mut game = in_configure(common::config());
    for _ in 0..4 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, (141, 218)), BIG_A, "not detected");
    run(&mut game, 100);
    assert_eq!(pixel(&game, (141, 218)), BIG_A, "until a key");
    step(&mut game, Key::Space);
    assert_eq!(row_fonts(&game, CONFIGURE)[4], BIG_A, "back in Configure");
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 0);
}

#[test]
fn the_gamepad_switch_turns_a_connected_gamepad_on_and_off() {
    let mut game = in_configure(common::config());
    game.input(InputEvent::PadConnected { connected: true });
    for _ in 0..4 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 1);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 0);
}

#[test]
fn previous_menu_returns_and_starts_configure_over_at_its_first_row() {
    // Escape keeps Configure's row; its last row also sets it back.
    let mut game = in_configure(common::config());
    for _ in 0..5 {
        step(&mut game, Key::Down);
    }
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(selected(&game), 2);
    assert!(written(&mut game).is_some());
    step(&mut game, Key::Enter);
    assert_eq!(row_fonts(&game, CONFIGURE)[0], BIG_A);
}

#[test]
fn dr_cfg_is_written_after_the_end_screen() {
    let mut game = in_menu(assets());
    game.take_config();
    step(&mut game, Key::Escape);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Left);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 26 + 25 + 560 + 25);
    assert!(game.take_config().is_none());
    game.tick();
    assert!(game.quit_requested());
    assert!(game.take_config().is_some());
}

#[test]
fn a_corrupt_volume_in_dr_cfg_opens_the_popup_at_full_instead_of_crashing() {
    // A damaged or hand-edited file can hold any number; the slider shows the most it can.
    let mut config = common::config();
    config.set_music_volume(0xFB1B_6C00);
    let mut game = in_configure(config);
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, knob_at(128)), KNOB);
}

/// The wipe takes 43 ticks.
const WIPE: u32 = 43;

/// A game showing the best ten, its wipe over.
fn in_hall_of_fame(assets: Assets) -> Game {
    let mut game = in_menu(assets);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + WIPE);
    game
}

/// The last left sample of the next tick.
fn level(game: &mut Game) -> i16 {
    game.take_audio(&mut Vec::new());
    game.tick();
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    audio[audio.len() - 2]
}

#[test]
fn the_best_ten_wipe_in_from_the_left() {
    // The band of masks moves 15 pixels a tick from the left edge.
    let mut game = in_menu(assets());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 20);
    assert_eq!(pixel(&game, (5, 90)), FAME_TITLE, "covered on the left");
    assert_ne!(pixel(&game, (600, 90)), FAME_TITLE, "not yet on the right");
    run(&mut game, WIPE - 20);
    assert_eq!(pixel(&game, (600, 90)), FAME_TITLE);
    assert_eq!(pixel(&game, (38, 146)), MEDIUM, "rank 1");
    assert_eq!(pixel(&game, (30, 344)), MEDIUM, "rank 10, further left");
}

#[test]
fn the_hall_of_fame_plays_its_own_music_and_the_menu_music_comes_back() {
    // The music falls with the wipe and comes back at full mask; back in the menu it starts
    // again at the order it had (45, the tone).
    let mut with_music = assets();
    with_music.menu_music = music(true);
    // Every order plays the tone, so whichever order the music is at when it starts again
    // sounds.
    with_music.menu_music.orders = vec![0; 94];
    let mut game = in_menu(with_music);
    let before = level(&mut game);
    assert!(before > 0);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 30);
    let falling = level(&mut game);
    assert!(falling < before / 2, "{falling} vs {before}");
    run(&mut game, 30);
    assert!(level(&mut game) > falling, "the mask back at 0x10000");
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    press(&mut game, Key::Escape);
    run(&mut game, 2 + WIPE + 10);
    let after = level(&mut game);
    assert!(
        i32::from(after) * 10 >= i32::from(before) * 9,
        "the menu music again at full mask: {after} vs {before}"
    );
}

#[test]
fn the_records_step_through_the_circuits_in_the_originals_order() {
    let mut game = in_hall_of_fame(assets());
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    assert_eq!(pixel(&game, (5, 95)), RECORDS_TITLE);
    let snapshot = |game: &Game| pixel(game, (45, 220));
    assert_eq!(snapshot(&game), SNAPSHOT, "circuit 0 first");
    press(&mut game, Key::Right);
    run(&mut game, 2);
    assert_eq!(snapshot(&game), SNAPSHOT + 7, "then circuit 7");
    assert_eq!(pixel(&game, (170, 230)), ARROW + 3, "the right arrow lit");
    run(&mut game, 5);
    assert_eq!(pixel(&game, (170, 230)), ARROW + 3, "for 8 waits");
    run(&mut game, 4);
    assert_eq!(
        pixel(&game, (170, 230)),
        ARROW + 1,
        "and dark after 8 waits"
    );
    for _ in 0..2 {
        press(&mut game, Key::Left);
        run(&mut game, 12);
    }
    assert_eq!(
        snapshot(&game),
        SNAPSHOT + 15,
        "Left wraps to the last, circuit 15"
    );
}

#[test]
fn leaving_the_records_wipes_the_main_menu_back_with_its_row_kept() {
    let mut game = in_hall_of_fame(assets());
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    assert_eq!(
        sound_after(&mut game, Key::Up),
        SILENT,
        "Up does nothing here"
    );
    press(&mut game, Key::Escape);
    run(&mut game, 2 + WIPE);
    assert_eq!(selected(&game), 3);
    step(&mut game, Key::Down);
    assert_eq!(selected(&game), 4, "the menu works again");
}

#[test]
fn the_best_tens_names_are_written_upper_case_once_shown() {
    // seeHallOfFame upper-cases them in the configuration itself.
    let mut config = common::config();
    let mut bytes = config.to_bytes();
    bytes[8 + 0xA6E..8 + 0xA6E + 3].copy_from_slice(b"ann");
    config = DrCfg::parse(&bytes).unwrap();
    let mut game = in_menu_with(assets(), config);
    game.take_config();
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + WIPE);
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    press(&mut game, Key::Escape);
    run(&mut game, 2 + WIPE);
    step(&mut game, Key::Up);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().hall_of_fame(0).0, b"ANN");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-core --no-run`
Expected: FAIL to compile (`blit_mask`, `set_order` do not exist).

- [ ] **Step 3: Implement**

<!-- write: crates/core/src/canvas.rs -->
```rust
//! The original's 640x480 drawing buffer and its operations (spec M2a §4.2). Positions are
//! linear offsets, `y * 640 + x`, as the original passes them: a picture drawn past the right
//! edge goes on at the start of the next row, as it does there. Nothing is drawn past the end.

use deadrally_gamedata::image::Image;

/// Menu screens are 640x480.
pub(crate) const WIDTH: usize = 640;
pub(crate) const HEIGHT: usize = 480;

/// The offset of column `x`, row `y`.
pub(crate) const fn at(x: usize, y: usize) -> usize {
    y * WIDTH + x
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Canvas {
    pixels: Vec<u8>,
}

impl Default for Canvas {
    fn default() -> Canvas {
        Canvas {
            pixels: vec![0; WIDTH * HEIGHT],
        }
    }
}

impl Canvas {
    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Copies the whole of `picture`, which must be 640x480.
    pub(crate) fn copy_all(&mut self, picture: &Image) {
        self.pixels.copy_from_slice(&picture.pixels);
    }

    /// Copies rows `first..first + rows` of `picture` (640 wide) into the same rows.
    pub(crate) fn copy_rows(&mut self, picture: &Image, first: usize, rows: usize) {
        let range = at(0, first)..at(0, first + rows).min(self.pixels.len());
        self.pixels[range.clone()].copy_from_slice(&picture.pixels[range]);
    }

    /// Copies a `width` x `height` region of `picture` (640 wide) at `offset` into the same place.
    pub(crate) fn restore(&mut self, picture: &Image, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&picture.pixels[start..end]);
        }
    }

    /// Copies `other`'s pixels into this canvas where `mask` (placed at `offset`) is not 0
    /// (0x43B080, the Hall of Fame's wipe).
    pub(crate) fn blit_mask(&mut self, mask: &Image, other: &Canvas, offset: usize) {
        let width = mask.width as usize;
        for (row, line) in mask.pixels.chunks(width).enumerate() {
            for (column, &cover) in line.iter().enumerate() {
                let at = offset + row * WIDTH + column;
                if cover != 0 && at < self.pixels.len() {
                    self.pixels[at] = other.pixels[at];
                }
            }
        }
    }

    /// Copies a `width` x `height` region at `offset` from `other` into the same place
    /// (`copyRectToVram`, 0x41AA40, and `refreshAllScreen`, 0x41A210, for the whole screen).
    pub(crate) fn copy_from(&mut self, other: &Canvas, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&other.pixels[start..end]);
        }
    }

    /// Fills `width` x `height` pixels from `offset` with `colour`.
    pub(crate) fn fill(&mut self, offset: usize, width: usize, height: usize, colour: u8) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].fill(colour);
        }
    }

    /// Draws `image` at `offset`; with `transparent`, colour 0 leaves the canvas as it is.
    pub(crate) fn draw(&mut self, image: &Image, offset: usize, transparent: bool) {
        let width = image.width as usize;
        for (row, source) in image.pixels.chunks_exact(width.max(1)).enumerate() {
            let start = offset + row * WIDTH;
            if start >= self.pixels.len() {
                break;
            }
            let end = (start + width).min(self.pixels.len());
            let target = &mut self.pixels[start..end];
            for (pixel, &colour) in target.iter_mut().zip(source) {
                if !transparent || colour != 0 {
                    *pixel = colour;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_drawing_keeps_what_colour_0_covers() {
        // Glyphs, corners and the cursor are drawn this way; an opaque 0 would cut boxes
        // into the background around every letter.
        let mut canvas = Canvas::default();
        canvas.fill(at(0, 0), 4, 1, 9);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), true);
        assert_eq!(&canvas.pixels()[..4], [1, 9, 2, 9]);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), false);
        assert_eq!(&canvas.pixels()[..4], [1, 0, 2, 9]);
    }

    #[test]
    fn drawing_past_the_right_edge_goes_on_in_the_next_row() {
        // The original works on linear offsets; text that runs past column 639 shows at the
        // left of the next row, and a faithful copy must too.
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(3, 1, vec![5, 6, 7]), at(638, 0), false);
        assert_eq!(&canvas.pixels()[at(638, 0)..at(1, 1)], [5, 6, 7]);
    }

    #[test]
    fn nothing_is_drawn_past_the_end() {
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(2, 2, vec![1; 4]), at(639, 479), false);
        canvas.fill(at(639, 479), 5, 5, 3);
        assert_eq!(canvas.pixels()[at(639, 479)], 3);
        assert_eq!(canvas.pixels().len(), WIDTH * HEIGHT);
    }

    #[test]
    fn restoring_copies_a_region_back_from_a_picture() {
        let background = Image::new(
            640,
            480,
            (0..WIDTH * HEIGHT).map(|i| (i % 251) as u8).collect(),
        );
        let mut canvas = Canvas::default();
        canvas.restore(&background, at(10, 20), 3, 2);
        assert_eq!(canvas.pixels()[at(10, 20)], background.pixels[at(10, 20)]);
        assert_eq!(canvas.pixels()[at(12, 21)], background.pixels[at(12, 21)]);
        assert_eq!(canvas.pixels()[at(13, 21)], 0, "only the region");
        canvas.copy_rows(&background, 100, 2);
        assert_eq!(
            canvas.pixels()[at(639, 101)],
            background.pixels[at(639, 101)]
        );
        assert_eq!(canvas.pixels()[at(0, 102)], 0);
    }

    #[test]
    fn a_masked_copy_takes_the_other_canvas_where_the_mask_is_set() {
        // The wipe's tiles: a pixel of the new screen shows only where the mask has one.
        let mut old = Canvas::default();
        let mut new = Canvas::default();
        new.fill(0, WIDTH, 2, 9);
        old.blit_mask(&Image::new(3, 2, vec![1, 0, 1, 0, 1, 0]), &new, at(10, 0));
        assert_eq!(&old.pixels()[at(10, 0)..at(13, 0)], [9, 0, 9]);
        assert_eq!(&old.pixels()[at(10, 1)..at(13, 1)], [0, 9, 0]);
    }
}
```

<!-- write: crates/core/src/audio/music.rs -->
```rust
//! The music player: Scream Tracker 3 modules with the 13 commands the game's music uses
//! (spec M1b §3.2, §4.2). Timing is counted in output samples, so the tempo never drifts.

use std::sync::Arc;

use deadrally_gamedata::s3m::{self, Cell, Module, NO_NOTE, NOTE_CUT, ORDER_SKIP};

use super::mixer::{Loop, Voice};
use super::tables::{S3M_PERIODS, vibrato};
use crate::AUDIO_SAMPLE_RATE;

/// Scream Tracker's clock: frequency = 14317056 / period.
const CLOCK: u32 = 14_317_056;
/// FMOD mixes at this rate and counts a tick as a whole number of its samples:
/// `44100 * 5 / (2 * tempo)`, rounded down. At tempo 141 that is 781 samples, not 781.9, so
/// the original plays such music 0.1 % fast; at tempo 125 it is exactly 882 (20 ms).
const FMOD_RATE: u32 = 44_100;
/// Period limits of Scream Tracker 3.
const MIN_PERIOD: i32 = 64;
const MAX_PERIOD: i32 = 32_767;

#[derive(Clone, Debug)]
struct SampleData {
    data: Arc<[i16]>,
    looping: Loop,
    c2spd: u32,
    volume: i32,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    enabled: bool,
    /// 0..=255.
    pan: i64,
    voice: Option<Voice>,
    /// Current sample (1-based instrument number), 0 = none yet.
    instrument: u8,
    period: i32,
    target_period: i32,
    /// 0..=64.
    volume: i32,
    /// Vibrato offset of this tick, in period units.
    period_delta: i32,
    command: u8,
    info: u8,
    // Effect memories.
    volume_slide: u8,
    porta: u8,
    tone_porta: u8,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_position: u8,
    offset: u8,
    retrigger: u8,
    retrigger_count: u8,
    /// A row's note held back by SDx until this tick.
    delayed: Option<(u8, Cell)>,
}

#[derive(Debug)]
pub(crate) struct Music {
    orders: Vec<u8>,
    patterns: Vec<s3m::Pattern>,
    samples: Vec<Option<SampleData>>,
    global_volume: i32,
    channels: Vec<Channel>,
    fading: Vec<Voice>,
    speed: u8,
    tempo: u8,
    order: usize,
    row: usize,
    tick: u8,
    /// Output frames left in the current tick, and the carried fraction of a frame.
    frames_left: u32,
    remainder: u32,
    /// Where the next row comes from after a B or C command.
    jump: Option<(usize, usize)>,
    /// Gain applied to every channel, in 16.16 (the original's master volume).
    gain: i64,
    /// The module's own share of it: its master volume, doubled for a stereo module.
    module_gain: i64,
}

impl Music {
    /// A player at order `first_order` (counted as the game counts them, markers included),
    /// row 0 and tick 0. `gain` scales the whole module ([`UNITY`] = as loud as the module
    /// asks). As FMOD does (measured), the module's own master volume scales it by
    /// `master / 64` (the game's music has 48, 2.5 dB below full), and a stereo module plays
    /// at twice a mono module's level.
    pub(crate) fn new(module: &Module, gain: i64, first_order: usize) -> Music {
        let module_gain = i64::from(module.master_volume) * if module.stereo { 2 } else { 1 };
        let gain = gain * module_gain / 64;
        let samples = module
            .samples
            .iter()
            .map(|sample| {
                (!sample.data.is_empty()).then(|| SampleData {
                    data: Arc::from(sample.data.as_slice()),
                    looping: sample
                        .looped
                        .map_or(Loop::None, |(start, end)| Loop::Forward { start, end }),
                    c2spd: sample.c2spd,
                    volume: i32::from(sample.volume),
                })
            })
            .collect();
        let channels = module
            .channels
            .iter()
            .map(|channel| Channel {
                enabled: channel.enabled,
                // FMOD 3 puts a stereo module's channels fully on their side (measured on the
                // menu music); mono modules play in the centre.
                pan: match (module.stereo, channel.pan < 8) {
                    (false, _) => 128,
                    (true, true) => 0,
                    (true, false) => 255,
                },
                ..Channel::default()
            })
            .collect();
        let mut music = Music {
            orders: module.orders.clone(),
            patterns: module.patterns.clone(),
            samples,
            global_volume: i32::from(module.global_volume),
            channels,
            fading: Vec::new(),
            speed: module.initial_speed.max(1),
            tempo: module.initial_tempo.max(32),
            order: first_order,
            row: 0,
            tick: 0,
            frames_left: 0,
            remainder: 0,
            jump: None,
            gain,
            module_gain,
        };
        music.skip_marker_orders();
        music
    }

    /// Fades every channel out, as `FMUSIC_StopSong`.
    pub(crate) fn stop(&mut self) {
        for channel in &mut self.channels {
            if let Some(mut voice) = channel.voice.take() {
                voice.release();
                self.fading.push(voice);
            }
        }
        self.orders.clear();
    }

    /// A new master volume for the song (`FMUSIC_SetMasterVolume`), as [`Music::new`]'s `gain`.
    pub(crate) fn set_gain(&mut self, gain: i64) {
        self.gain = gain * self.module_gain / 64;
    }

    /// The order playing, as the game counts them (`FMUSIC_GetOrder` through 0x43C1F0).
    pub(crate) fn order(&self) -> usize {
        self.order
    }

    /// On to order `order` (as the game counts them) at its first row (`FMUSIC_SetOrder`
    /// through 0x43C320).
    pub(crate) fn set_order(&mut self, order: usize) {
        self.order = order;
        self.row = 0;
        self.tick = 0;
        self.jump = None;
        self.skip_marker_orders();
    }

    /// Fades every channel out and hands the fading voices over, for music being replaced.
    pub(crate) fn into_fading(mut self) -> Vec<Voice> {
        self.stop();
        self.fading
    }

    /// Adds the music to `out` (interleaved stereo), advancing ticks exactly on time.
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        let mut done = 0;
        let frames = out.len() / 2;
        while done < frames {
            if self.frames_left == 0 {
                if !self.orders.is_empty() {
                    self.process_tick();
                }
                self.start_tick_timer();
            }
            let count = (frames - done).min(self.frames_left as usize);
            let part = &mut out[2 * done..2 * (done + count)];
            for channel in &mut self.channels {
                if let Some(voice) = &mut channel.voice {
                    voice.mix_into(part);
                    if voice.finished() {
                        channel.voice = None;
                    }
                }
            }
            for voice in &mut self.fading {
                voice.mix_into(part);
            }
            self.fading.retain(|voice| !voice.finished());
            done += count;
            self.frames_left -= u32::try_from(count).expect("at most a tick");
        }
    }

    /// The next tick's length: FMOD's whole number of 44.1 kHz samples, converted to our rate
    /// exactly by carrying the remainder.
    fn start_tick_timer(&mut self) {
        let fmod_samples = FMOD_RATE * 5 / (2 * u32::from(self.tempo));
        let scaled = fmod_samples * AUDIO_SAMPLE_RATE + self.remainder;
        self.frames_left = scaled / FMOD_RATE;
        self.remainder = scaled % FMOD_RATE;
    }

    /// Moves past 254 and 255, which FMOD drops from the order list.
    fn skip_marker_orders(&mut self) {
        while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
            self.order += 1;
        }
        if self.order >= self.orders.len() {
            // The song loops from its start, as FMOD's looping music does.
            self.order = 0;
            while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
                self.order += 1;
            }
        }
    }

    fn process_tick(&mut self) {
        if self.tick == 0 {
            self.process_row();
        } else {
            for index in 0..self.channels.len() {
                self.channel_tick(index);
            }
        }
        for index in 0..self.channels.len() {
            self.update_voice(index);
        }
        self.tick += 1;
        if self.tick >= self.speed {
            self.tick = 0;
            self.next_row();
        }
    }

    fn next_row(&mut self) {
        if let Some((order, row)) = self.jump.take() {
            self.order = order;
            self.row = row;
            self.skip_marker_orders();
            return;
        }
        self.row += 1;
        if self.row >= s3m::ROWS {
            self.row = 0;
            self.order += 1;
            self.skip_marker_orders();
        }
    }

    fn process_row(&mut self) {
        let Some(&pattern) = self.orders.get(self.order) else {
            return;
        };
        let cells = self.patterns[usize::from(pattern)].rows[self.row];
        for (index, cell) in cells.iter().enumerate() {
            if !self.channels[index].enabled {
                continue;
            }
            let channel = &mut self.channels[index];
            channel.command = cell.command;
            channel.info = cell.info;
            channel.period_delta = 0;
            if cell.command == command('S') && cell.info >> 4 == 0xD && cell.info & 0xF > 0 {
                channel.delayed = Some((cell.info & 0xF, *cell));
                continue;
            }
            channel.delayed = None;
            self.start_cell(index, cell);
            self.row_effect(index, cell);
        }
    }

    /// The note, instrument and volume of a cell.
    fn start_cell(&mut self, index: usize, cell: &Cell) {
        let tone_porta = cell.command == command('G');
        if cell.instrument != 0 {
            let channel = &mut self.channels[index];
            channel.instrument = cell.instrument;
            if let Some(Some(sample)) = self.samples.get(usize::from(cell.instrument) - 1) {
                channel.volume = sample.volume;
            }
        }
        if cell.note == NOTE_CUT {
            self.cut(index);
        } else if cell.note != NO_NOTE {
            let instrument = self.channels[index].instrument;
            if let Some(Some(sample)) = usize::from(instrument)
                .checked_sub(1)
                .and_then(|i| self.samples.get(i))
            {
                let period = note_period(cell.note, sample.c2spd);
                let channel = &mut self.channels[index];
                if tone_porta && channel.voice.is_some() {
                    channel.target_period = period;
                } else {
                    let offset = if cell.command == command('O') {
                        if cell.info != 0 {
                            channel.offset = cell.info;
                        }
                        u32::from(channel.offset) * 256
                    } else {
                        0
                    };
                    let voice = Voice::new(Arc::clone(&sample.data), sample.looping, offset);
                    if let Some(mut old) = channel.voice.replace(voice) {
                        old.release();
                        self.fading.push(old);
                    }
                    let channel = &mut self.channels[index];
                    channel.period = period;
                    channel.target_period = period;
                    channel.vibrato_position = 0;
                    channel.retrigger_count = 0;
                }
            } else if instrument != 0 && !(tone_porta && self.channels[index].voice.is_some()) {
                // FMOD plays a note of an empty sample slot as silence: the note sounding stops.
                self.cut(index);
            }
        }
        if let Some(volume) = cell.volume {
            self.channels[index].volume = i32::from(volume);
        }
    }

    fn cut(&mut self, index: usize) {
        if let Some(mut voice) = self.channels[index].voice.take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    /// Commands that act on the row's first tick.
    fn row_effect(&mut self, index: usize, cell: &Cell) {
        let info = cell.info;
        let channel = &mut self.channels[index];
        match letter(cell.command) {
            'A' if info > 0 => self.speed = info,
            'T' if info >= 0x20 => self.tempo = info,
            'B' => {
                let row = self.jump.map_or(0, |(_, row)| row);
                self.jump = Some((usize::from(info), row));
            }
            'C' => {
                let row = usize::from((info >> 4) * 10 + (info & 0xF)).min(s3m::ROWS - 1);
                let order = self.jump.map_or(self.order + 1, |(order, _)| order);
                self.jump = Some((order, row));
            }
            'D' | 'K' => {
                if info != 0 {
                    channel.volume_slide = info;
                }
                let slide = channel.volume_slide;
                // Fine slides (DxF up, DFy down) act once, on this tick.
                if slide & 0x0F == 0x0F && slide >> 4 != 0 {
                    channel.volume = (channel.volume + i32::from(slide >> 4)).min(64);
                } else if slide >> 4 == 0x0F && slide & 0x0F != 0 {
                    channel.volume = (channel.volume - i32::from(slide & 0x0F)).max(0);
                }
            }
            'E' | 'F' => {
                if info != 0 {
                    channel.porta = info;
                }
                let porta = channel.porta;
                let sign = if letter(cell.command) == 'E' { 1 } else { -1 };
                // EFx fine (x * 4), EEx extra fine (x), on this tick only.
                match porta >> 4 {
                    0xF => channel.period += sign * 4 * i32::from(porta & 0xF),
                    0xE => channel.period += sign * i32::from(porta & 0xF),
                    _ => {}
                }
                channel.period = channel.period.clamp(MIN_PERIOD, MAX_PERIOD);
            }
            'G' => {
                if info != 0 {
                    channel.tone_porta = info;
                }
            }
            'H' => {
                if info >> 4 != 0 {
                    channel.vibrato_speed = info >> 4;
                }
                if info & 0xF != 0 {
                    channel.vibrato_depth = info & 0xF;
                }
            }
            'Q' if info != 0 => channel.retrigger = info,
            _ => {}
        }
    }

    /// Commands that act on every tick but the first.
    fn channel_tick(&mut self, index: usize) {
        if !self.channels[index].enabled {
            return;
        }
        if let Some((at, cell)) = self.channels[index].delayed {
            if self.tick == at {
                self.channels[index].delayed = None;
                self.start_cell(index, &cell);
            }
            return;
        }
        let channel = &mut self.channels[index];
        channel.period_delta = 0;
        match letter(channel.command) {
            'D' => volume_slide(channel),
            'K' => {
                volume_slide(channel);
                vibrato_tick(channel);
            }
            'E' | 'F' => {
                let porta = channel.porta;
                if porta >> 4 < 0xE {
                    let sign = if letter(channel.command) == 'E' {
                        1
                    } else {
                        -1
                    };
                    channel.period = (channel.period + sign * 4 * i32::from(porta))
                        .clamp(MIN_PERIOD, MAX_PERIOD);
                }
            }
            'G' => {
                let speed = 4 * i32::from(channel.tone_porta);
                if channel.period < channel.target_period {
                    channel.period = (channel.period + speed).min(channel.target_period);
                } else {
                    channel.period = (channel.period - speed).max(channel.target_period);
                }
            }
            'H' => vibrato_tick(channel),
            // Without an interval nothing repeats, and nothing is counted.
            'Q' if channel.retrigger & 0xF != 0 => {
                let interval = channel.retrigger & 0xF;
                channel.retrigger_count += 1;
                if channel.retrigger_count >= interval {
                    channel.retrigger_count = 0;
                    channel.volume = retrigger_volume(channel.volume, channel.retrigger >> 4);
                    let sample = usize::from(channel.instrument)
                        .checked_sub(1)
                        .and_then(|i| self.samples.get(i))
                        .and_then(Option::as_ref);
                    if let Some(sample) = sample {
                        let voice = Voice::new(Arc::clone(&sample.data), sample.looping, 0);
                        if let Some(mut old) = channel.voice.replace(voice) {
                            old.release();
                            self.fading.push(old);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Pushes the channel's pitch and volume into its voice.
    fn update_voice(&mut self, index: usize) {
        let global = i64::from(self.global_volume);
        let gain = self.gain;
        let channel = &mut self.channels[index];
        let Some(voice) = &mut channel.voice else {
            return;
        };
        let period = (channel.period + channel.period_delta).clamp(MIN_PERIOD, MAX_PERIOD);
        voice.set_frequency(CLOCK / u32::try_from(period).expect("positive"));
        let volume = i64::from(channel.volume) * global * gain / (64 * 64);
        let left = volume * (255 - channel.pan) / 255;
        let right = volume * channel.pan / 255;
        voice.set_volume(left, right);
    }

    #[cfg(test)]
    fn position(&self) -> (usize, usize, u8) {
        (self.order, self.row, self.tick)
    }
}

fn command(letter: char) -> u8 {
    letter as u8 - b'@'
}

fn letter(command: u8) -> char {
    if (1..=26).contains(&command) {
        char::from(b'@' + command)
    } else {
        ' '
    }
}

/// Scream Tracker's period of a note byte (octave in the high nibble) for a sample's C2SPD.
/// The octave shift comes last, so high notes keep their precision.
fn note_period(note: u8, c2spd: u32) -> i32 {
    let (octave, semitone) = (u32::from(note >> 4), usize::from(note & 0xF).min(11));
    let period =
        (8363 * 16 * u64::from(S3M_PERIODS[semitone]) / u64::from(c2spd.max(1))) >> octave.min(9);
    i32::try_from(period)
        .unwrap_or(MAX_PERIOD)
        .clamp(MIN_PERIOD, MAX_PERIOD)
}

fn volume_slide(channel: &mut Channel) {
    let slide = channel.volume_slide;
    let (up, down) = (slide >> 4, slide & 0x0F);
    if down == 0 && up != 0 {
        channel.volume = (channel.volume + i32::from(up)).min(64);
    } else if up == 0 && down != 0 {
        channel.volume = (channel.volume - i32::from(down)).max(0);
    }
}

fn vibrato_tick(channel: &mut Channel) {
    let wave = vibrato(channel.vibrato_position);
    channel.period_delta = (wave * i32::from(channel.vibrato_depth)) >> 5;
    channel.vibrato_position = (channel.vibrato_position + channel.vibrato_speed) & 63;
}

/// Qxy's volume change `x` applied to a volume 0..=64.
fn retrigger_volume(volume: i32, change: u8) -> i32 {
    let changed = match change {
        1..=5 => volume - (1 << (change - 1)),
        6 => volume * 2 / 3,
        7 => volume / 2,
        9..=13 => volume + (1 << (change - 9)),
        14 => volume * 3 / 2,
        15 => volume * 2,
        _ => volume,
    };
    changed.clamp(0, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{Channel as S3mChannel, Pattern, Sample};

    use crate::audio::mixer::UNITY;

    const C4: u8 = 0x40;

    fn module(rows: &[(usize, usize, Cell)], speed: u8, tempo: u8) -> Module {
        let mut pattern = Pattern {
            rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
        };
        for &(row, channel, cell) in rows {
            pattern.rows[row][channel] = cell;
        }
        let mut channels = [S3mChannel::default(); s3m::CHANNELS];
        channels[0] = S3mChannel {
            enabled: true,
            pan: 3,
        };
        channels[1] = S3mChannel {
            enabled: true,
            pan: 12,
        };
        Module {
            title: "Test".into(),
            orders: vec![0, 1],
            initial_speed: speed,
            initial_tempo: tempo,
            global_volume: 64,
            master_volume: 48,
            stereo: false,
            channels,
            samples: vec![Sample {
                name: "Tone".into(),
                c2spd: 8363,
                volume: 32,
                looped: Some((0, 100)),
                data: vec![10_000; 100],
            }],
            patterns: vec![pattern.clone(), pattern],
        }
    }

    fn cell(note: u8, instrument: u8, volume: Option<u8>, command: char, info: u8) -> Cell {
        Cell {
            note,
            instrument,
            volume,
            command: if command == ' ' {
                0
            } else {
                super::command(command)
            },
            info,
        }
    }

    fn play(music: &mut Music, frames: usize) -> Vec<i64> {
        let mut out = vec![0; 2 * frames];
        music.mix_into(&mut out);
        out
    }

    #[test]
    fn a_tick_lasts_fmods_whole_number_of_samples() {
        // At tempo 125 a tick is 882 samples at 44.1 kHz, exactly 20 ms: 960 of ours.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 960 * 6);
        assert_eq!(music.position(), (0, 1, 0));
        // At tempo 141 FMOD counts 781 samples (not 781.9) per tick, which is why the menu
        // music runs 0.1 % fast in the original: 147 ticks are exactly 124 960 of our samples.
        let mut faster = Music::new(&module(&[], 1, 141), UNITY, 0);
        play(&mut faster, 124_960);
        assert_eq!(
            faster.position(),
            (0, 19, 0),
            "147 rows: both patterns, then the song loops to row 19 of its start"
        );
    }

    #[test]
    fn middle_c_plays_at_the_samples_c2spd() {
        assert_eq!(note_period(C4, 8363), 1712);
        assert_eq!(CLOCK / 1712, 8362);
        assert_eq!(
            note_period(0x50, 8363),
            856,
            "an octave up halves the period"
        );
        assert_eq!(
            note_period(C4, 16_726),
            856,
            "a doubled C2SPD sounds an octave up"
        );
    }

    #[test]
    fn notes_play_with_the_samples_volume_and_the_volume_column_overrides_it() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(C4, 1, Some(64), ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        let out = play(&mut music, 960);
        // Mono: centred, 32 / 64 of full volume on each side, and the module's master volume
        // 48 / 64 on top.
        let expected = 10_000 * 32 / 64 * 127 / 255 * 48 / 64;
        assert!(
            (out[2 * 900] - expected).abs() <= 2,
            "{} vs {expected}",
            out[2 * 900]
        );
        let louder = play(&mut music, 960);
        assert!(
            (louder[2 * 900] - 2 * expected).abs() <= 4,
            "{}",
            louder[2 * 900]
        );
    }

    #[test]
    fn speed_tempo_jump_and_break_commands_steer_the_song() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(NO_NOTE, 0, None, 'A', 2)),
                    (0, 1, cell(NO_NOTE, 0, None, 'C', 0x10)),
                ],
                6,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 2);
        assert_eq!(
            music.position(),
            (1, 10, 0),
            "speed 2, then a break to row 10 of the next order"
        );
        let mut jumping = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'B', 0))], 1, 125),
            UNITY,
            0,
        );
        play(&mut jumping, 960);
        assert_eq!(jumping.position(), (0, 0, 0), "B00 loops the first order");
        // As in minifmod, a new tempo already sets the length of the tick that sets it.
        let mut tempo = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'T', 250))], 1, 125),
            UNITY,
            0,
        );
        play(&mut tempo, 480 * 3);
        assert_eq!(
            tempo.position(),
            (0, 3, 0),
            "tempo 250: ticks of 480 samples"
        );
    }

    #[test]
    fn stereo_modules_play_each_channel_fully_on_its_side_and_twice_as_loud() {
        // FMOD 3 pans a stereo module's channels hard left or right, and plays them at twice a
        // mono module's level: only that matches the stereo image and the loudness of the
        // original's menu music, recorded at two music volumes (docs/verification/m1b.md).
        let mut stereo = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        stereo.stereo = true;
        let mut music = Music::new(&stereo, UNITY, 0);
        let out = play(&mut music, 960);
        let full = 10_000 * 48 / 64 * 2;
        assert!((out[2 * 900] - full).abs() <= 4, "left {}", out[2 * 900]);
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn a_song_can_start_at_a_later_order() {
        // The game starts the menu music at order 45 (musicSetOrder), not at its beginning.
        let mut later = module(&[], 1, 125);
        later.orders = vec![0, s3m::ORDER_END, 1];
        let music = Music::new(&later, UNITY, 2);
        assert_eq!(music.position(), (2, 0, 0));
        let at_marker = Music::new(&later, UNITY, 1);
        assert_eq!(at_marker.position(), (2, 0, 0), "a marker is skipped");
    }

    #[test]
    fn playback_runs_on_through_section_markers_as_fmod_does() {
        // FMOD drops 254 and 255 from the order list (the game numbers orders for
        // FMUSIC_SetOrder without them), so a pattern before a 255 is followed by the next
        // section, not by the song's start.
        let mut base = module(&[], 1, 125);
        base.orders = vec![0, s3m::ORDER_END, s3m::ORDER_SKIP, 1];
        let mut music = Music::new(&base, UNITY, 0);
        play(&mut music, 960 * s3m::ROWS);
        assert_eq!(music.position(), (3, 0, 0));
    }

    /// A module whose one sample rises steadily (0, 8, 16, ...), so the output shows how far
    /// into the sample a voice is.
    fn rising(rows: &[(usize, usize, Cell)], speed: u8) -> Module {
        let mut rising = module(rows, speed, 125);
        rising.samples[0].data = (0..4000).map(|i| i16::try_from(i * 8).unwrap()).collect();
        rising.samples[0].looped = None;
        rising
    }

    #[test]
    fn the_sample_offset_starts_a_note_further_into_its_sample() {
        // O (545 times in TR5) starts notes part way into their sample; ignoring it would play
        // the sample's beginning instead.
        let mut plain = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 6),
            UNITY,
            0,
        );
        let mut offset = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), 'O', 8))], 6),
            UNITY,
            0,
        );
        let (plain, offset) = (play(&mut plain, 300), play(&mut offset, 300));
        // 8 * 256 = 2048 samples in: about 2100 instead of about 50 at frame 299.
        assert!(plain[2 * 299] > 0);
        assert!(
            offset[2 * 299] > 20 * plain[2 * 299],
            "{} vs {}",
            offset[2 * 299],
            plain[2 * 299]
        );
    }

    #[test]
    fn vibrato_with_volume_slide_does_both() {
        // K (277 times in TR1) keeps an earlier H's vibrato going while it slides the volume;
        // doing only one of the two freezes the note or its level.
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, Some(10), 'H', 0x48)),
                    (1, 0, cell(NO_NOTE, 0, None, 'K', 0x20)),
                ],
                4,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        let position = music.channels[0].vibrato_position;
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        assert_eq!(
            music.channels[0].vibrato_position,
            position + 3 * 4,
            "the vibrato goes on at speed 4"
        );
    }

    #[test]
    fn portamento_up_lowers_the_period_and_its_fine_form_acts_once() {
        // F (252 times in TR5) slides notes up; the wrong direction, or FFx on every tick,
        // would detune whole phrases.
        let mut up = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut up, 960 * 3);
        assert_eq!(up.channels[0].period, 1712 - 2 * 8, "2 ticks of 4 * 2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 0xF3))], 3, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 3);
        assert_eq!(
            fine.channels[0].period,
            1712 - 3 * 4,
            "FF3: once, on the first tick"
        );
    }

    #[test]
    fn retrigger_restarts_the_note_at_its_interval_and_changes_its_volume() {
        // Q (90 times in TR9) drums a note several times per row; without the restarts, or the
        // volume change, a roll becomes one long note.
        let mut music = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(20), 'Q', 0xA3))], 7),
            UNITY,
            0,
        );
        let out = play(&mut music, 960 * 7);
        // Restarted at tick 3: 200 frames later a voice plays again, near the sample's start.
        let (restarted, before) = (out[2 * (3 * 960 + 200)], out[2 * (3 * 960 - 1)]);
        assert!(
            restarted > 0 && restarted < before / 4,
            "{restarted} vs {before}"
        );
        assert_eq!(
            music.channels[0].volume, 24,
            "+2 at each restart, on ticks 3 and 6"
        );
    }

    #[test]
    fn a_retrigger_without_an_interval_never_overflows() {
        // Q00 before any Q with an interval repeats nothing; counting its ticks anyway overflowed
        // after 255 of them and crashed the game in the middle of the music.
        let mut long = module(
            &[
                (0, 0, cell(C4, 1, Some(64), 'Q', 0)),
                (1, 0, cell(NO_NOTE, 0, None, 'Q', 0)),
            ],
            200,
            125,
        );
        long.orders = vec![0];
        let mut music = Music::new(&long, UNITY, 0);
        play(&mut music, 960 * 400);
        assert_eq!(music.position(), (0, 2, 0));
    }

    #[test]
    fn a_song_of_markers_only_stays_silent() {
        // An order list without a pattern must not hang the player looking for one.
        let mut base = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        base.orders = vec![s3m::ORDER_SKIP, s3m::ORDER_END];
        let mut music = Music::new(&base, UNITY, 0);
        assert!(play(&mut music, 960 * 4).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn volume_slides_act_after_the_first_tick_and_fine_ones_on_it() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x20))], 4, 125),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x3F))], 4, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 4);
        assert_eq!(fine.channels[0].volume, 13, "one fine step of +3");
    }

    #[test]
    fn portamentos_move_the_period() {
        let mut down = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'E', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut down, 960 * 3);
        assert_eq!(down.channels[0].period, 1712 + 2 * 8);
        let mut towards = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(0x50, 1, None, 'G', 0xFF)),
                ],
                3,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut towards, 960 * 6);
        assert_eq!(
            towards.channels[0].period, 856,
            "the tone portamento stops at its target"
        );
    }

    #[test]
    fn vibrato_wobbles_around_the_note() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'H', 0x48))], 6, 125),
            UNITY,
            0,
        );
        let mut deltas = Vec::new();
        for _ in 0..6 {
            play(&mut music, 960);
            deltas.push(music.channels[0].period_delta);
        }
        assert_eq!(deltas[0], 0, "no vibrato on the first tick");
        assert!(
            deltas[1..].iter().all(|&delta| delta >= 0) && deltas[5] > 0,
            "{deltas:?}"
        );
        assert_eq!(
            music.channels[0].period, 1712,
            "the note itself does not move"
        );
    }

    #[test]
    fn a_note_of_an_empty_sample_slot_silences_the_channel() {
        // The menu music's order 47 starts with notes of a sample slot its author emptied;
        // FMOD plays them as silence. Ignoring them left the channel's looping note playing,
        // brought back up by their volume: a stray tone in the menu.
        let mut emptied = module(
            &[
                (0, 0, cell(C4, 1, Some(64), ' ', 0)),
                (1, 0, cell(C4, 2, Some(32), ' ', 0)),
            ],
            1,
            125,
        );
        emptied.samples.push(Sample::default());
        let mut music = Music::new(&emptied, UNITY, 0);
        play(&mut music, 960);
        let out = play(&mut music, 960);
        assert!(out[2 * 900..].iter().all(|&sample| sample == 0));
    }

    #[test]
    fn setting_the_order_goes_on_at_its_first_row() {
        // The Hall of Fame moves the menu music to its own part (order 81) this way.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 5000);
        music.set_order(1);
        assert_eq!(music.position(), (1, 0, 0));
        assert_eq!(music.order(), 1);
    }

    #[test]
    fn note_delay_and_cut_and_retrigger() {
        let mut delayed = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'S', 0xD2))], 4, 125),
            UNITY,
            0,
        );
        play(&mut delayed, 960 * 2);
        assert!(delayed.channels[0].voice.is_none(), "not before tick 2");
        play(&mut delayed, 960);
        assert!(delayed.channels[0].voice.is_some());
        let mut cut = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(NOTE_CUT, 0, None, ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut cut, 960 * 2);
        assert!(cut.channels[0].voice.is_none());
        assert_eq!(retrigger_volume(40, 0xF), 64);
        assert_eq!(retrigger_volume(40, 0x7), 20);
        assert_eq!(retrigger_volume(40, 0x3), 36);
    }

    #[test]
    fn the_same_module_always_renders_the_same_samples() {
        let rows = [
            (0, 0, cell(C4, 1, Some(40), 'H', 0x46)),
            (8, 1, cell(0x45, 1, None, 'Q', 0x93)),
        ];
        let render = || {
            let mut music = Music::new(&module(&rows, 3, 131), UNITY, 0);
            play(&mut music, 48_000)
        };
        assert_eq!(render(), render());
    }
}
```

<!-- write: crates/core/src/audio/mod.rs -->
```rust
//! Sound: the music and effect players and the mixer they share (spec M1b §4.2).

pub(crate) mod effects;
pub(crate) mod mixer;
pub(crate) mod music;
pub(crate) mod tables;

use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::xm::Bank;

use self::effects::Effects;
use self::mixer::{UNITY, Voice, clip};
use self::music::Music;
use crate::AUDIO_CHANNELS;

/// FMOD's master volume for music, 0..=256, at a music volume of the game's configuration
/// (0..=0x10000): `mask * (volume >> 8) >> 9`, as `musicSetmusicVolume` (0x43C280) sets it,
/// with the game's volume mask (0x456A34) at 255 unless the end screen lowers it. A volume
/// past full, from a damaged `dr.cfg`, counts as full.
fn music_master(volume: u32, mask: u32) -> i64 {
    (i64::from(mask) * i64::from(volume.min(effects::FULL) >> 8)) >> 9
}

/// The volume mask's normal value.
const FULL_MASK: u32 = 255;

/// The music volume the intro plays at, whatever `dr.cfg` says: the game's volume globals
/// start at 255 and take `dr.cfg`'s values only when the menu music starts.
pub(crate) const FULL_VOLUME: u32 = 0xFF00;

/// `dr.cfg`'s default music volume, 50 % (`defaultConfig`, 0x426700). The menus play at it
/// until M2's Configure menu can change it.
pub(crate) const DEFAULT_MUSIC_VOLUME: u32 = 0x8000;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug)]
pub(crate) struct Sound {
    music: Option<Music>,
    /// Voices of music that newer music replaced, fading out.
    fading: Vec<Voice>,
    effects: Option<Effects>,
    /// Voices of a bank that a newer bank replaced, fading out.
    fading_effects: Vec<Voice>,
    mix: Vec<i64>,
    /// The original's volume globals: the mask (0x456A34), the music's (0x456A30) and the
    /// effects' (0x456A2C) volumes as `dr.cfg` gives them; all full until the intro ends.
    mask: u32,
    music_volume: u32,
    effects_volume: u32,
}

impl Default for Sound {
    fn default() -> Sound {
        Sound {
            music: None,
            fading: Vec::new(),
            effects: None,
            fading_effects: Vec::new(),
            mix: Vec::new(),
            mask: FULL_MASK,
            music_volume: FULL_VOLUME,
            effects_volume: FULL_VOLUME,
        }
    }
}

impl Sound {
    /// Starts `module` at order `first_order` (counted as the game counts them) and the
    /// configured music `volume` (0..=0x10000), fading out any music playing.
    pub(crate) fn play_music(&mut self, module: &Module, first_order: usize, volume: u32) {
        if let Some(old) = self.music.take() {
            self.fading.extend(old.into_fading());
        }
        self.music_volume = volume;
        self.music = Some(Music::new(module, self.music_gain(), first_order));
    }

    fn music_gain(&self) -> i64 {
        UNITY * music_master(self.music_volume, self.mask) / 256
    }

    /// The effects stream's share: `mask * (volume >> 8) >> 8` of 255 (`musicSetVolume`,
    /// 0x43C250), 254 of 255 while the intro plays.
    fn effects_gain(&self) -> i64 {
        let volume = self.effects_volume.min(effects::FULL);
        UNITY * ((i64::from(self.mask) * i64::from(volume >> 8)) >> 8) / 255
    }

    /// The configured effects volume (0..=0x10000) for the effects stream.
    pub(crate) fn set_effects_volume(&mut self, volume: u32) {
        self.effects_volume = volume;
    }

    /// The volume mask (`setMusicVolume`, 0x43C2B0): 0..=255 over music and effects alike.
    pub(crate) fn set_mask(&mut self, mask: u32) {
        self.mask = mask;
        self.apply_music_gain();
    }

    /// The configured music volume (0..=0x10000) for the music playing, as Configure's popup
    /// sets it (`musicSetmusicVolume`, 0x43C280).
    pub(crate) fn set_music_volume(&mut self, volume: u32) {
        self.music_volume = volume;
        self.apply_music_gain();
    }

    /// The music's order (0 without music).
    pub(crate) fn music_order(&self) -> usize {
        self.music.as_ref().map_or(0, Music::order)
    }

    /// The music on to order `order` at its first row.
    pub(crate) fn set_music_order(&mut self, order: usize) {
        if let Some(music) = &mut self.music {
            music.set_order(order);
        }
    }

    fn apply_music_gain(&mut self) {
        let gain = self.music_gain();
        if let Some(music) = &mut self.music {
            music.set_gain(gain);
        }
    }

    /// Makes `bank` the source of [`Sound::trigger`]; the old bank's effects fade out.
    pub(crate) fn load_effects(&mut self, bank: &Bank) {
        if let Some(old) = self.effects.take() {
            self.fading_effects.extend(old.into_fading());
        }
        self.effects = Some(Effects::new(bank));
    }

    /// Plays effect `effect` of the loaded bank on `channel` (both 1-based) at full volume and
    /// normal pitch, as the intro does.
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8) {
        self.trigger_at(channel, effect, effects::FULL, effects::FULL);
    }

    /// Plays effect `effect` on `channel` at `volume` and `pitch` (16.16, 0x10000 full and
    /// normal), as `loadMenuSoundEffect` (0x43C380) does.
    pub(crate) fn trigger_at(&mut self, channel: usize, effect: u8, volume: u32, pitch: u32) {
        if let Some(effects) = &mut self.effects {
            effects.trigger(channel, effect, volume, pitch);
        }
    }

    /// Stops the music and every effect, with a short fade so nothing clicks.
    pub(crate) fn stop(&mut self) {
        if let Some(music) = &mut self.music {
            music.stop();
        }
        if let Some(effects) = &mut self.effects {
            effects.stop_all();
        }
    }

    /// Appends `frames` interleaved stereo frames to `out`.
    pub(crate) fn render(&mut self, frames: usize, out: &mut Vec<i16>) {
        self.mix.clear();
        self.mix.resize(frames * AUDIO_CHANNELS, 0);
        if let Some(music) = &mut self.music {
            music.mix_into(&mut self.mix);
        }
        for voice in &mut self.fading {
            voice.mix_into(&mut self.mix);
        }
        self.fading.retain(|voice| !voice.finished());
        let mut part = vec![0; self.mix.len()];
        if let Some(effects) = &mut self.effects {
            effects.mix_into(&mut part);
        }
        for voice in &mut self.fading_effects {
            voice.mix_into(&mut part);
        }
        self.fading_effects.retain(|voice| !voice.finished());
        let gain = self.effects_gain();
        for (sum, effect) in self.mix.iter_mut().zip(part) {
            *sum += (effect * gain) >> 16;
        }
        out.extend(self.mix.iter().map(|&value| clip(value)));
    }
}

/// Renders `frames` stereo frames of `module` from its first order at the default music
/// volume, mixed as the game plays music in its menus and races.
#[must_use]
pub fn render_music(module: &Module, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.play_music(module, 0, DEFAULT_MUSIC_VOLUME);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

/// Renders effect `effect` (1-based) of `bank` at full volume and normal pitch, as the game
/// triggers it, for `frames` stereo frames.
#[must_use]
pub fn render_effect(bank: &Bank, effect: u8, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.load_effects(bank);
    sound.trigger(1, effect);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{self, Cell, Channel, Pattern, Sample};
    use deadrally_gamedata::xm::{Instrument, Looping};

    fn bank() -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![Some(Instrument {
                name: "Loud".into(),
                data: vec![i16::MAX; 20_000],
                looping: Looping::None,
                volume: 64,
                finetune: 0,
                relative_note: 0,
                panning: 255,
                fadeout: 0,
            })],
        }
    }

    #[test]
    fn nothing_loaded_is_silence_of_the_right_length() {
        // The frontend paces itself on the audio queue; every tick must deliver its samples.
        let mut out = Vec::new();
        Sound::default().render(672, &mut out);
        assert_eq!(out.len(), 672 * AUDIO_CHANNELS);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn overlapping_effects_clip_instead_of_wrapping() {
        // Four full-scale effects add up to twice the 16-bit range; wrapping would turn the
        // overload into noise, clipping only flattens it.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        for channel in 1..=4 {
            sound.trigger(channel, 1);
        }
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 900], i16::MAX, "left, panned hard left");
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn new_music_fades_the_old_out_instead_of_cutting_it() {
        // The intro's music gives way to the menu music; a cut at full level would click.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut silent = loud.clone();
        silent.orders.clear();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        assert!(before > 0);
        sound.play_music(&silent, 0, FULL_VOLUME);
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_configured_music_volume_sets_fmods_master_volume() {
        // musicSetmusicVolume (0x43C280): 255 * (volume >> 8) >> 9.
        assert_eq!(music_master(FULL_VOLUME, FULL_MASK), 127);
        assert_eq!(music_master(DEFAULT_MUSIC_VOLUME, FULL_MASK), 63);
        assert_eq!(music_master(0x1_0000, FULL_MASK), 127);
        assert_eq!(music_master(0, FULL_MASK), 0);
    }

    #[test]
    fn a_new_bank_lets_the_old_banks_effects_fade_out() {
        // The intro's effects stop as the menu's bank is loaded; cutting them at full level
        // would click.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        sound.stop();
        sound.load_effects(&bank());
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_effects_volume_and_the_mask_scale_the_effects_stream() {
        // The menus play effects at dr.cfg's 75 %: the stream at 255 * 192 >> 8 = 191 of 255
        // instead of the intro's 254. The end screen's mask lowers everything.
        let level = |sound: &mut Sound| {
            sound.load_effects(&bank());
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(1000, &mut out);
            i64::from(out[2 * 999])
        };
        let full = level(&mut Sound::default());
        let mut menu = Sound::default();
        menu.set_effects_volume(0xC000);
        let at_75 = level(&mut menu);
        assert!(
            (at_75 * 254 - full * 191).abs() <= 254 * 2,
            "{at_75} vs {full}"
        );
        let mut quiet = Sound::default();
        quiet.set_mask(0);
        assert_eq!(level(&mut quiet), 0);
    }

    /// One endless loud note on the first channel.
    fn loud() -> Module {
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        loud
    }

    #[test]
    fn the_mask_scales_the_music_while_it_plays() {
        // The end screen fades the music out through the mask, 255 down to 0.
        let loud = loud();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        sound.set_mask(0x80);
        out.clear();
        sound.render(2000, &mut out);
        // 255 * 255 >> 9 = 127 against 128 * 255 >> 9 = 63.
        assert!(
            (i64::from(out[2 * 1999]) * 127 - full * 63).abs() <= 127 * 2,
            "{} vs {full}",
            out[2 * 1999]
        );
    }

    #[test]
    fn the_music_volume_changes_the_music_while_it_plays() {
        // Configure's popup applies each step at once (`musicSetmusicVolume`): 50 % plays at
        // master volume 63, 100 % at 127.
        let mut sound = Sound::default();
        sound.play_music(&loud(), 0, DEFAULT_MUSIC_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let half = i64::from(out[2 * 1999]);
        sound.set_music_volume(0x1_0000);
        out.clear();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        assert!(
            (full * 63 - half * 127).abs() <= 127 * 2,
            "{half} vs {full}"
        );
    }

    #[test]
    fn a_volume_beyond_full_plays_at_full() {
        // dr.cfg can hold any number; the music and the effects must not blare at many times
        // full volume from the moment the menu music starts.
        let level = |volume: u32| {
            let mut sound = Sound::default();
            sound.play_music(&loud(), 0, volume);
            sound.load_effects(&bank());
            sound.set_effects_volume(volume);
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(2000, &mut out);
            out[2 * 1999]
        };
        assert_eq!(level(0xFFFF_FFFF), level(0x1_0000));
    }

    #[test]
    fn stopping_fades_everything_out() {
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        sound.stop();
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 999], 0);
    }
}
```

<!-- write: crates/core/src/menu/draw.rs -->
```rust
//! The menu screen's drawing, as `dr.exe` does it into its screen buffer and copies to the
//! shown buffer (spec M2a §3.3, §3.4): popups (`createPopup` 0x41A530), menus (`drawMenu`
//! 0x41A880), the cursor (`updateCursor` 0x41AB50), the highlight's moves (`refreshMenuUp`
//! 0x41AF40, `refreshMenuDown` 0x41B1A0, 0x41ACF0) and the bottom panel (0x41A7A0, 0x41E810).

use deadrally_gamedata::assets::MenuAssets;
use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::Texts;

use crate::canvas::{Canvas, at};
use crate::font::Font;

/// The fill colour of popups and of the cursor's box.
pub(crate) const POPUP_FILL: u8 = 0xC4;
/// Popup lines: a focused popup's, an unfocused one's.
const LINE_FOCUSED: u8 = 7;
const LINE_UNFOCUSED: u8 = 4;
/// Corner pictures are 32x20, the cursor 20x20, a big glyph 32 high.
const CORNER_WIDTH: usize = 32;
const CORNER_HEIGHT: usize = 20;
const CURSOR_SIZE: usize = 20;
pub(crate) const CURSOR_FRAMES: usize = 50;

/// One menu of the table at 0x4456F0 and its active rows (0x4457F0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuTable {
    /// Which menu of `dr.exe`'s text table its rows are.
    pub(crate) text: usize,
    pub(crate) rows: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) row_height: usize,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) selected: usize,
    pub(crate) active: [bool; 9],
}

/// The main menu: start, multiplayer (inactive), configure, hall of fame, credits, exit.
pub(crate) const MAIN_MENU: MenuTable = MenuTable {
    text: 0,
    rows: 6,
    x: 145,
    y: 124,
    row_height: 28,
    width: 349,
    height: 192,
    selected: 0,
    active: [true, false, true, true, true, true, false, false, false],
};

/// The start submenu at the first start: rows 0, 3 and 5 active.
pub(crate) const START_MENU: MenuTable = MenuTable {
    text: 1,
    rows: 6,
    x: 109,
    y: 171,
    row_height: 28,
    width: 421,
    height: 192,
    selected: 0,
    active: [true, false, false, true, false, true, false, false, false],
};

/// Configure (menu 3): music volume, effect volume, define keyboard, define gamepad, the
/// gamepad switch, previous menu.
pub(crate) const CONFIGURE_MENU: MenuTable = MenuTable {
    text: 3,
    rows: 6,
    x: 95,
    y: 146,
    row_height: 28,
    width: 485,
    height: 192,
    selected: 0,
    active: [true, true, true, true, true, true, false, false, false],
};

/// Define Keyboard (menu 6): the eight controls and previous menu.
pub(crate) const KEYBOARD_MENU: MenuTable = MenuTable {
    text: 6,
    rows: 9,
    x: 50,
    y: 93,
    row_height: 28,
    width: 532,
    height: 278,
    selected: 0,
    active: [true; 9],
};

/// Define Gamepad (menu 8): seven controls (no horn) and previous menu.
pub(crate) const PAD_MENU: MenuTable = MenuTable {
    text: 8,
    rows: 8,
    x: 50,
    y: 113,
    row_height: 28,
    width: 532,
    height: 250,
    selected: 0,
    active: [true, true, true, true, true, true, true, true, false],
};

/// How a menu is drawn: unfocused (mode 0) or focused (mode 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Unfocused,
    Focused,
}

/// The menu's pictures, fonts and rows.
#[derive(Clone, Debug)]
pub(crate) struct Graphics {
    pub(crate) background: Image,
    panel_line: Image,
    corners_focused: Vec<Image>,
    corners_unfocused: Vec<Image>,
    cursor: Vec<Image>,
    pub(crate) big_a: Font,
    pub(crate) big_b: Font,
    pub(crate) big_d: Font,
    pub(crate) small: [Font; 3],
    /// The Hall of Fame's font.
    pub(crate) medium: Font,
    /// `dr.exe`'s menu text table: `menus[m][r]` is row `r` of menu `m`. The original rewrites
    /// some rows as settings change.
    menus: Vec<Vec<Vec<u8>>>,
    /// The volume popups' slider and its knob.
    pub(crate) slider: Image,
    pub(crate) knob: Image,
}

impl Graphics {
    pub(crate) fn new(assets: &MenuAssets) -> Graphics {
        let texts = &assets.texts;
        Graphics {
            background: assets.background.clone(),
            panel_line: assets.panel_line.clone(),
            corners_focused: assets.corners_focused.clone(),
            corners_unfocused: assets.corners_unfocused.clone(),
            cursor: assets.cursor.clone(),
            big_a: Font::new(assets.big_a.clone(), &texts.big),
            big_b: Font::new(assets.big_b.clone(), &texts.big),
            big_d: Font::new(assets.big_d.clone(), &texts.big),
            small: [
                Font::new(assets.small_a.clone(), &texts.small),
                Font::new(assets.small_b.clone(), &texts.small),
                Font::new(assets.small_c.clone(), &texts.small),
            ],
            medium: Font::new(assets.medium.clone(), &texts.medium),
            menus: texts.menus.clone(),
            slider: assets.slider.clone(),
            knob: assets.knob.clone(),
        }
    }

    /// Rewrites row `row` of menu `menu`, as the original copies a setting's text into its
    /// table.
    pub(crate) fn set_row(&mut self, menu: usize, row: usize, text: Vec<u8>) {
        self.menus[menu][row] = text;
    }

    pub(crate) fn cursor(&self, frame: usize) -> &Image {
        &self.cursor[frame % self.cursor.len()]
    }

    /// `createPopup(x, y, w, h, focus)`: fill, corners, then lines; nothing outside is
    /// cleared, so a popup drawn over another blends their corners.
    pub(crate) fn popup(
        &self,
        screen: &mut Canvas,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        focus: Focus,
    ) {
        if h > 8 {
            screen.fill(at(x + 2, y + 2), w - 6, h - 8, POPUP_FILL);
        }
        let (corners, line) = match focus {
            Focus::Unfocused => (&self.corners_unfocused, LINE_UNFOCUSED),
            Focus::Focused => (&self.corners_focused, LINE_FOCUSED),
        };
        let right = x + w - CORNER_WIDTH;
        let bottom = y + h - CORNER_HEIGHT;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            screen.draw(corner, offset, true);
        }
        if w > 64 {
            screen.fill(at(x + 32, y + 1), w - 64, 1, line);
            screen.fill(at(x + 32, y + h - 7), w - 64, 1, line);
        }
        if h > 40 {
            screen.fill(at(x + 1, y + 20), 1, h - 40, line);
            screen.fill(at(x + w - 5, y + 20), 1, h - 40, line);
        }
    }

    /// `drawMenu(menu, focus)`: its popup and rows; the selected row with the cursor when the
    /// menu has focus. Nothing reaches the shown buffer.
    pub(crate) fn menu(&self, screen: &mut Canvas, menu: &MenuTable, focus: Focus, cursor: usize) {
        self.popup(screen, menu.x, menu.y, menu.width, menu.height, focus);
        for row in 0..menu.rows {
            let text = &self.menus[menu.text][row];
            let at_text = at(menu.x + 32, menu.y + 5 + row * menu.row_height);
            let font = if row == menu.selected {
                if focus == Focus::Focused {
                    screen.draw(self.cursor(cursor), self.cursor_at(menu), true);
                    &self.big_a
                } else {
                    &self.big_d
                }
            } else if menu.active[row] && focus == Focus::Focused {
                &self.big_b
            } else {
                &self.big_d
            };
            font.draw(screen, text, at_text);
        }
    }

    /// Where the selected row's cursor goes.
    fn cursor_at(&self, menu: &MenuTable) -> usize {
        at(menu.x + 9, menu.y + 11 + menu.selected * menu.row_height)
    }

    /// `updateCursor`: the cursor's box refilled, frame `frame` drawn, the box copied to the
    /// shown buffer.
    pub(crate) fn update_cursor(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &MenuTable,
        frame: usize,
    ) {
        let offset = self.cursor_at(menu);
        screen.fill(offset, CURSOR_SIZE, CURSOR_SIZE, POPUP_FILL);
        screen.draw(self.cursor(frame), offset, true);
        shown.copy_from(screen, offset, CURSOR_SIZE, CURSOR_SIZE);
    }

    /// Moves the highlight to row `to` as `refreshMenuUp`/`refreshMenuDown` and 0x41ACF0 do:
    /// both rows' areas refilled and redrawn, the cursor drawn with frame `frame`, both
    /// copied to the shown buffer. `base` is 6 for Up and the jump to the last row, 5 for Down.
    pub(crate) fn move_highlight(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &mut MenuTable,
        to: usize,
        base: usize,
        frame: usize,
    ) {
        let rows_at = |row: usize| menu.y + base + row * menu.row_height;
        let text_at = |row: usize| at(menu.x + 32, menu.y + 5 + row * menu.row_height);
        let old = menu.selected;
        screen.fill(
            at(menu.x + 9, rows_at(old) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_b
            .draw(screen, &self.menus[menu.text][old], text_at(old));
        menu.selected = to;
        screen.fill(
            at(menu.x + 9, rows_at(to) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_a
            .draw(screen, &self.menus[menu.text][to], text_at(to));
        screen.draw(self.cursor(frame), self.cursor_at(menu), true);
        shown.copy_from(screen, at(menu.x + 7, rows_at(old)), menu.width - 10, 32);
        shown.copy_from(screen, at(menu.x + 7, rows_at(to)), menu.width - 10, 32);
    }

    /// `drawTransparentBlock(x, y, w, h)`: the background restored, then the panel's two
    /// frame lines.
    pub(crate) fn panel_frame(&self, screen: &mut Canvas, x: usize, y: usize, w: usize, h: usize) {
        screen.restore(&self.background, at(x + 2, y - 4), w - 6, h);
        screen.draw(&self.panel_line, at(0, y + 1), true);
        screen.draw(&self.panel_line, at(0, y + h - 9), true);
    }

    /// `drawBottomMenuText`: rows 380..=468 restored, then the panel's last six lines at
    /// (12, 378 + 15k), each in its own small font.
    pub(crate) fn panel_text(&self, screen: &mut Canvas, panel: &Panel) {
        screen.copy_rows(&self.background, 380, 89);
        for (k, line) in panel.lines[16..].iter().enumerate() {
            if let Some(font) = self.small.get(usize::from(line.font)) {
                font.draw(screen, &line.text, at(12, 378 + 15 * k));
            }
        }
    }
}

/// One line of the bottom panel and its font (0, 1, 2: small A, B, C).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: Vec<u8>,
    pub(crate) font: u8,
}

/// The bottom message panel: 22 lines, new ones pushed in at the bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Panel {
    lines: Vec<PanelLine>,
}

impl Panel {
    /// The panel as `mainMenu` fills it at start-up: the four start-up lines in small B, an
    /// empty line before the last.
    pub(crate) fn startup(texts: &Texts) -> Panel {
        let mut panel = Panel {
            lines: vec![PanelLine::default(); 22],
        };
        let [first, second, third, last] = [0, 1, 2, 3].map(|i| texts.panel[i].clone());
        for text in [first, second, third, Vec::new(), last] {
            panel.push(text, 1);
        }
        panel
    }

    fn push(&mut self, text: Vec<u8>, font: u8) {
        self.lines.remove(0);
        self.lines.push(PanelLine { text, font });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::{HEIGHT, WIDTH};

    /// Graphics where every picture has its own colour: corners 11..=14 (focused) and 21..=24,
    /// cursor frame k colour 100 + k with a transparent top-left pixel, glyphs as in
    /// `font::tests::font` but 32x32 (big) or 16x16 (small) and colour 50 + font.
    pub(crate) fn graphics() -> Graphics {
        let solid = |w: u32, h: u32, colour: u8| Image::new(w, h, vec![colour; (w * h) as usize]);
        let glyphs = |size: u32, colour: u8| {
            (0..96)
                .map(|_| solid(size, size, colour))
                .collect::<Vec<_>>()
        };
        let metrics = |size: u8| deadrally_gamedata::text::Metrics {
            width: size,
            height: size,
            advances: vec![size; 96],
        };
        Graphics {
            background: Image::new(
                640,
                480,
                (0..WIDTH * HEIGHT).map(|i| (i % 7) as u8 + 1).collect(),
            ),
            panel_line: solid(640, 10, 99),
            corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
            corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
            cursor: (0..50)
                .map(|k| {
                    let mut frame = solid(20, 20, 100 + k);
                    frame.pixels[0] = 0;
                    frame
                })
                .collect(),
            big_a: Font::new(glyphs(32, 50), &metrics(32)),
            big_b: Font::new(glyphs(32, 51), &metrics(32)),
            big_d: Font::new(glyphs(32, 52), &metrics(32)),
            small: [
                Font::new(glyphs(16, 60), &metrics(16)),
                Font::new(glyphs(16, 61), &metrics(16)),
                Font::new(glyphs(16, 62), &metrics(16)),
            ],
            medium: Font::new(glyphs(9, 63), &metrics(9)),
            menus: texts().menus,
            slider: solid(172, 24, 70),
            knob: solid(10, 24, 71),
        }
    }

    pub(crate) fn texts() -> Texts {
        let metrics = deadrally_gamedata::text::Metrics {
            width: 32,
            height: 32,
            advances: vec![32; 96],
        };
        Texts {
            menus: (0..9)
                .map(|m| {
                    (0..9)
                        .map(|r| {
                            if r < 6 {
                                vec![b'A' + m as u8]
                            } else {
                                Vec::new()
                            }
                        })
                        .collect()
                })
                .collect(),
            panel: (0..4).map(|i| vec![b'a' + i]).collect(),
            exit_question: b"?".to_vec(),
            yes: b"Y".to_vec(),
            no: b"N".to_vec(),
            big: metrics.clone(),
            small: metrics.clone(),
            medium: metrics,
            configure: configure_texts(),
            hall_of_fame: deadrally_gamedata::text::HallOfFameTexts {
                circuits: vec![b"C".to_vec(); 18],
                cars: vec![b"V".to_vec(); 6],
                difficulties: vec![b"D".to_vec(); 4],
                circuit_order: (0..18).collect(),
            },
        }
    }

    /// Configure's texts: one letter each, `k` for key and pad names.
    pub(crate) fn configure_texts() -> deadrally_gamedata::text::ConfigureTexts {
        let one = |c: u8| vec![c];
        deadrally_gamedata::text::ConfigureTexts {
            adjust_music: one(b'm'),
            adjust_effects: one(b'e'),
            gamepad_on: one(b'+'),
            gamepad_off: one(b'-'),
            not_detected: one(b'!'),
            press_any_key: one(b'.'),
            controls: (0..8).map(|i| one(b'0' + i)).collect(),
            key_prompts: (0..8).map(|_| one(b'k')).collect(),
            pad_prompts: (0..7).map(|_| one(b'p')).collect(),
            key_names: (0..256).map(|_| one(b'k')).collect(),
            pad_names: (0..9).map(|_| one(b'p')).collect(),
        }
    }

    #[test]
    fn a_popup_fills_exactly_w_minus_6_columns() {
        // DreeRally fills two columns short on the main menu; the original fills x+2..=x+w-5.
        let mut screen = Canvas::default();
        graphics().popup(&mut screen, 145, 124, 349, 192, Focus::Focused);
        let p = screen.pixels();
        assert_eq!(p[at(147, 200)], POPUP_FILL);
        assert_eq!(
            p[at(489 - 1, 200)],
            POPUP_FILL,
            "x + w - 5 is the right line"
        );
        assert_eq!(p[at(489, 200)], LINE_FOCUSED);
        assert_eq!(p[at(146, 200)], LINE_FOCUSED, "left line at x + 1");
        assert_eq!(p[at(490, 200)], 0, "the shadow columns are left alone");
        assert_eq!(p[at(177, 125)], LINE_FOCUSED, "top line from x + 32");
        assert_eq!(
            p[at(461, 309)],
            LINE_FOCUSED,
            "bottom line at y + h - 7 to x + w - 33"
        );
        assert_eq!(p[at(145, 124)], 11, "top-left corner");
        assert_eq!(p[at(462, 124)], 12, "top-right corner");
        assert_eq!(p[at(145, 296)], 13, "bottom-left corner");
        assert_eq!(p[at(462 + 31, 296 + 19)], 14, "bottom-right corner");
    }

    #[test]
    fn a_focused_menu_shows_the_cursor_and_its_rows_in_three_fonts() {
        let mut screen = Canvas::default();
        let mut menu = MAIN_MENU;
        menu.selected = 2;
        graphics().menu(&mut screen, &menu, Focus::Focused, 7);
        let p = screen.pixels();
        let text_row = |row: usize| p[at(177, 129 + 28 * row)];
        assert_eq!(text_row(2), 50, "selected: big A");
        assert_eq!(text_row(0), 51, "active: big B");
        assert_eq!(text_row(1), 52, "inactive: big D");
        assert_eq!(
            p[at(155, 135 + 56)],
            107,
            "cursor frame 7 at (x + 9, y + 11 + 28 * 2)"
        );
        let mut dim = Canvas::default();
        graphics().menu(&mut dim, &menu, Focus::Unfocused, 7);
        assert_eq!(
            dim.pixels()[at(177, 129 + 56)],
            52,
            "unfocused: everything big D"
        );
        assert_eq!(dim.pixels()[at(155, 191)], POPUP_FILL, "no cursor");
        assert_eq!(dim.pixels()[at(145, 124)], 21, "unfocused corners");
    }

    #[test]
    fn the_cursor_update_copies_its_box_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        graphics().update_cursor(&mut screen, &mut shown, &MAIN_MENU, 3);
        assert_eq!(
            shown.pixels()[at(154, 135)],
            POPUP_FILL,
            "transparent pixel over the fill"
        );
        assert_eq!(shown.pixels()[at(155, 135)], 103);
        assert_eq!(shown.pixels()[at(174, 135)], 0, "only the 20x20 box");
    }

    #[test]
    fn moving_the_highlight_redraws_both_rows_and_copies_both_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        let mut menu = MAIN_MENU;
        graphics().move_highlight(&mut screen, &mut shown, &mut menu, 2, 5, 9);
        assert_eq!(menu.selected, 2);
        let s = shown.pixels();
        assert_eq!(s[at(177, 129)], 51, "the old row in big B");
        assert_eq!(s[at(177, 129 + 56)], 50, "the new row in big A");
        assert_eq!(s[at(155, 135 + 56)], 109, "cursor frame 9");
        assert_eq!(s[at(152, 129 + 28)], 0, "row 1 is not copied");
    }

    #[test]
    fn the_panel_shows_its_last_six_lines_with_the_startup_text_in_small_b() {
        let panel = Panel::startup(&texts());
        let mut screen = Canvas::default();
        graphics().panel_text(&mut screen, &panel);
        let p = screen.pixels();
        assert_eq!(
            p[at(12, 380)],
            graphics().background.pixels[at(12, 380)],
            "line 16 is empty; the restore starts at row 380"
        );
        for k in [1, 2, 3, 5] {
            assert_eq!(p[at(12, 378 + 15 * k)], 61, "line {}: small B", 16 + k);
        }
        assert_eq!(
            p[at(12, 445)],
            graphics().background.pixels[at(12, 445)],
            "line 20 is empty (the glyphs above reach row 438)"
        );
    }
}
```

<!-- write: crates/core/src/menu/hall_of_fame.rs -->
```rust
//! The Hall of Fame (spec M2c §3): the best ten (`seeHallOfFame`, 0x431510), the records by
//! circuit (`drawRecordByCircuit`, 0x41E490) and the wipes between them and the main menu
//! (`sub_42C560`, `sub_42C4A0`).

use deadrally_gamedata::image::Image;

use super::draw::Focus;
use super::{Menu, State};
use crate::canvas::{Canvas, at};
use crate::keys;

/// The wipe: 43 steps of 15 pixels, a band of 10 tile columns and 22 tile rows of 15 x 15,
/// copied to the screen as a 150 x 330 window from row 75.
const WIPE_STEPS: u32 = 43;
const WIPE_START: usize = at(0, 75);
const TILE: usize = 15;
const WIPE_ROWS: usize = 22;
const WIPE_COLUMNS: usize = 10;
/// The music's volume mask falls from here by this much a step while the menu wipes away.
const WIPE_VOLUME: u32 = 65_532;
const WIPE_VOLUME_STEP: u32 = 1524;
/// The Hall of Fame's music starts at this order; the volume mask is then 0x10000 >> 8.
const FAME_ORDER: usize = 81;
const FULL_MASK: u32 = 0x1_0000 >> 8;
/// Effect 26 sounds as Left or Right change the circuit; the arrow stays lit for 8 waits.
const STEP_SOUND: u8 = 26;
const ARROW_WAITS: u32 = 8;
/// The snapshots are drawn 98 rows high.
const SNAPSHOT_ROWS: u32 = 98;

/// What a wipe brings in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Wipe {
    Fame,
    Records,
    Menu,
}

fn upper(text: &[u8]) -> Vec<u8> {
    text.to_ascii_uppercase()
}

impl Menu {
    /// The best ten drawn into the second buffer, then wiped in with the music falling.
    pub(super) fn open_hall_of_fame(&mut self) -> State {
        self.config.upper_case_hall_of_fame();
        self.back = self.screen.clone();
        self.back.copy_rows(&self.graphics.background, 105, 262);
        let menu = &self.assets.menu;
        self.back.draw(&menu.fame_title, at(0, 84), true);
        let medium = &self.graphics.medium;
        let difficulties = &menu.texts.hall_of_fame.difficulties;
        for rank in 0..10 {
            let y = 144 + 22 * rank;
            let rank_at = if rank == 9 { at(28, y) } else { at(36, y) };
            medium.draw(&mut self.back, format!("{}.", rank + 1).as_bytes(), rank_at);
            let (name, races, difficulty) = self.config.hall_of_fame(rank);
            medium.draw(&mut self.back, name, at(137, y));
            if races >= 0 {
                let x = match races {
                    0..10 => 344,
                    10..100 => 336,
                    _ => 328,
                };
                medium.draw(&mut self.back, races.to_string().as_bytes(), at(x, y));
            }
            let level = difficulties
                .get(difficulty as usize)
                .map_or(&[][..], Vec::as_slice);
            medium.draw(&mut self.back, &upper(level), at(429, y));
        }
        State::Wipe {
            wipe: Wipe::Fame,
            step: 0,
        }
    }

    /// One step of a wipe after its wait: the band of masked tiles from the second buffer,
    /// its window shown, the music falling when the menu goes or comes back.
    pub(super) fn wipe_tick(&mut self, wipe: Wipe, step: u32) -> State {
        self.palette.after_wait();
        let base = WIPE_START + TILE * step as usize;
        for row in 0..WIPE_ROWS {
            for column in 0..WIPE_COLUMNS {
                let offset = base + TILE * (row * crate::canvas::WIDTH + column);
                self.screen
                    .blit_mask(&self.assets.menu.wipe[column], &self.back, offset);
            }
        }
        self.shown.copy_from(&self.screen, base, 150, 330);
        if wipe != Wipe::Records {
            self.sound
                .set_mask((WIPE_VOLUME - WIPE_VOLUME_STEP * step) >> 8);
        }
        if step + 1 < WIPE_STEPS {
            return State::Wipe {
                wipe,
                step: step + 1,
            };
        }
        self.keys.take();
        self.keys.take();
        match wipe {
            Wipe::Fame => {
                self.music_order = self.sound.music_order();
                self.sound.set_music_order(FAME_ORDER);
                self.sound.set_mask(FULL_MASK);
                self.keys.take();
                self.keys.take();
                State::FameWait
            }
            Wipe::Records => State::Records { index: 0 },
            Wipe::Menu => {
                let volume = self.config.music_volume();
                self.sound
                    .play_music(&self.assets.menu_music, self.music_order, volume);
                self.sound.set_mask(FULL_MASK);
                self.shown = self.screen.clone();
                State::Main { second: false }
            }
        }
    }

    /// The best ten wait for any key.
    pub(super) fn fame_wait(&mut self) -> State {
        self.palette.after_wait();
        if self.keys.take() == 0 {
            return State::FameWait;
        }
        self.keys.take();
        self.keys.take();
        self.back = self.screen.clone();
        self.back.copy_rows(&self.graphics.background, 84, 283);
        let menu = &self.assets.menu;
        self.back.draw(&menu.records_title, at(0, 92), true);
        let circuit = usize::from(menu.texts.hall_of_fame.circuit_order[0]);
        let mut back = std::mem::take(&mut self.back);
        self.draw_records(&mut back, circuit);
        back.draw(&self.snapshot(circuit), at(40, 214), false);
        back.draw(&self.assets.menu.arrows[0], at(24, 228), false);
        back.draw(&self.assets.menu.arrows[1], at(168, 228), false);
        self.border(&mut back, 15, 204, 178, 117);
        self.back = back;
        State::Wipe {
            wipe: Wipe::Records,
            step: 0,
        }
    }

    /// The records screen's key after its wait: Left and Right step through the circuits,
    /// Enter, keypad Enter and Escape leave.
    pub(super) fn records_tick(&mut self, index: usize) -> State {
        self.palette.after_wait();
        let order = self.assets.menu.texts.hall_of_fame.circuit_order.clone();
        let circuits = order.len();
        let (index, right) = match self.keys.take() {
            keys::LEFT | keys::PAD_LEFT => ((index + circuits - 1) % circuits, false),
            keys::RIGHT | keys::PAD_RIGHT => ((index + 1) % circuits, true),
            keys::ENTER | keys::ESCAPE | 0x9C => return self.leave_hall_of_fame(),
            // F1 opens the chat in a network game; nothing else does anything here.
            _ => return State::Records { index },
        };
        self.sound(STEP_SOUND);
        let (frame, x) = if right { (3, 168) } else { (2, 24) };
        let mut screen = std::mem::take(&mut self.screen);
        screen.draw(&self.assets.menu.arrows[frame], at(x, 228), false);
        let circuit = usize::from(order[index]);
        screen.draw(&self.snapshot(circuit), at(40, 214), false);
        self.draw_records(&mut screen, circuit);
        self.screen = screen;
        self.shown = self.screen.clone();
        State::RecordsArrow {
            index,
            right,
            waits: 0,
        }
    }

    /// The lit arrow's waits; after the eighth it goes dark again.
    pub(super) fn records_arrow(&mut self, index: usize, right: bool, waits: u32) -> State {
        self.palette.after_wait();
        if waits + 1 < ARROW_WAITS {
            return State::RecordsArrow {
                index,
                right,
                waits: waits + 1,
            };
        }
        let (frame, x) = if right { (1, 168) } else { (0, 24) };
        self.screen
            .draw(&self.assets.menu.arrows[frame], at(x, 228), false);
        self.shown = self.screen.clone();
        State::Records { index }
    }

    /// Back to the main menu: drawn into the second buffer and wiped in with the music
    /// falling; the menu music then starts again at the order it had.
    fn leave_hall_of_fame(&mut self) -> State {
        self.back = self.screen.clone();
        self.back.copy_rows(&self.graphics.background, 84, 283);
        self.graphics
            .menu(&mut self.back, &self.main, Focus::Focused, self.cursor);
        State::Wipe {
            wipe: Wipe::Menu,
            step: 0,
        }
    }

    /// Circuit `circuit`'s records (0x41E490): two areas restored, the bar, the circuit's name
    /// centred, and car 5 down to car 0 with their record's driver and time.
    fn draw_records(&self, canvas: &mut Canvas, circuit: usize) {
        let background = &self.graphics.background;
        canvas.restore(background, at(225, 133), 381, 29);
        canvas.restore(background, at(224, 206), 368, 130);
        let menu = &self.assets.menu;
        canvas.draw(&menu.records_bar, at(0, 132), true);
        let hall = &menu.texts.hall_of_fame;
        let name = &hall.circuits[circuit];
        let width = self.graphics.big_a.width(name);
        self.graphics
            .big_a
            .draw(canvas, name, at(413, 136) - width / 2);
        for row in 0..6 {
            let car = 5 - row;
            let y = 208 + 22 * row;
            let medium = &self.graphics.medium;
            medium.draw(canvas, &upper(&hall.cars[car]), at(228, y));
            let (driver, [minutes, seconds, hundredths]) = self.config.record(circuit, car);
            medium.draw(canvas, &upper(driver), at(360, y));
            let time = format!("{minutes:02}:{seconds:02}.{hundredths:02}");
            medium.draw(canvas, time.as_bytes(), at(514, y));
        }
    }

    /// Circuit `circuit`'s snapshot, 98 rows of its frame.
    fn snapshot(&self, circuit: usize) -> Image {
        let frame = &self.assets.menu.snapshots[circuit];
        let rows = (frame.width * SNAPSHOT_ROWS) as usize;
        Image::new(frame.width, SNAPSHOT_ROWS, frame.pixels[..rows].to_vec())
    }

    /// `drawBorder` (0x421AE0): `CHOO2`'s four corners and lines of colour 22 between them.
    fn border(&self, canvas: &mut Canvas, x: usize, y: usize, width: usize, height: usize) {
        const CORNER: usize = 24;
        const LINE: u8 = 0x16;
        let corners = &self.assets.menu.border_corners;
        let right = x + width - CORNER;
        let bottom = y + height - CORNER;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            canvas.draw(corner, offset, true);
        }
        let across = width - 2 * CORNER;
        let down = height - 2 * CORNER;
        canvas.fill(at(x + CORNER, y + 2), across, 1, LINE);
        canvas.fill(at(x + CORNER, y + height - 3), across, 1, LINE);
        canvas.fill(at(x + 2, y + CORNER), 1, down, LINE);
        canvas.fill(at(x + width - 3, y + CORNER), 1, down, LINE);
    }
}
```

<!-- write: crates/core/src/menu/mod.rs -->
```rust
//! The main menu (spec M2a §3.2–§3.5, M2b §3.2, M2c §3), from the title's fade to black to
//! the end screen, with Configure and the Hall of Fame.
//!
//! The original runs this as straight code with waits in it (`waitWithRefresh`, 0x43D870); the
//! screen shown during a tick is what the shown buffer and the palette hold when that tick's
//! wait starts. Here [`State`] names the wait the menu stands at, and [`Menu::tick`] runs the
//! code from it to the next one.

mod configure;
pub(crate) mod draw;
mod hall_of_fame;
pub(crate) mod palette;

use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::dr_cfg::DrCfg;

use self::draw::{
    CONFIGURE_MENU, CURSOR_FRAMES, Focus, Graphics, KEYBOARD_MENU, MAIN_MENU, MenuTable, PAD_MENU,
    POPUP_FILL, Panel, START_MENU,
};
use self::palette::MenuPalette;
use crate::audio::Sound;
use crate::canvas::{Canvas, HEIGHT, WIDTH, at};
use crate::keys::{self, Keys};
use crate::{AUDIO_FRAMES_PER_TICK, Frame};

/// The menus' sounds (`loadMenuSoundEffect`, 0x43C380): channel 1, at the configured effects
/// volume and pitch 0x28000.
const SOUND_CHANNEL: usize = 1;
const SOUND_PITCH: u32 = 0x2_8000;
const MOVE_SOUND: u8 = 25;
const BACK_SOUND: u8 = 22;
const CHOOSE_SOUND: u8 = 28;

/// The player's colour at the first start: driver 19's, 0 until a game sets it.
const PLAYER_COLOUR: usize = 0;
/// The main menu's rows: 0 start, 2 configure, 3 Hall of Fame, 4 credits, 5 exit.
const START_ROW: usize = 0;
const CONFIGURE_ROW: usize = 2;
const HALL_OF_FAME_ROW: usize = 3;
const CREDITS_ROW: usize = 4;
const EXIT_ROW: usize = 5;
/// The start submenu's last row returns to the main menu.
const START_MENU_BACK: usize = 5;
/// The exit question's popup and its yes/no at (x, y) = (180, 238).
const YES_NO_X: usize = 180;
const YES_NO_Y: usize = 238;
/// The end screen shows for at most 560 ticks; its fade-out lowers the music from 65500 in
/// steps of 2620.
const END_HOLD_TICKS: u32 = 560;
const END_VOLUME: u32 = 65_500;
const END_VOLUME_STEP: u32 = 2620;

/// Fades: 4 % a tick (`fadeIn` 0x427280 25 steps to 96 %, `transitionToBlack` 0x427300 26
/// steps from 100 % to 0), the menu's own fades 2 % a tick over 50 steps.
const FADE_IN_STEPS: u32 = 25;
const FADE_OUT_STEPS: u32 = 26;
const MENU_FADE_STEPS: u32 = 50;

/// The wait the menu stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// `transitionToBlack` on the title: wait `step` of 26.
    TitleToBlack {
        step: u32,
    },
    /// The menu's fade-in, wait `step` of 50.
    FadeIn {
        step: u32,
    },
    /// `readEventInMenu`: the first or second wait of a pass, in the main menu or a submenu.
    Main {
        second: bool,
    },
    Submenu {
        menu: Submenu,
        second: bool,
    },
    /// A volume popup's loop (`showAdjustOptions`, 0x4309A0): the level 0..=128 and the key
    /// read last.
    Volume {
        music: bool,
        level: i32,
        last: u8,
    },
    /// Define Keyboard waiting for control `control`'s key; `key` is the one read last.
    KeyWait {
        control: usize,
        key: u8,
    },
    /// Define Gamepad waiting for control `control`'s input (0x42CBF0): the polls so far.
    PadWait {
        control: usize,
        polls: u32,
    },
    /// The popup when the gamepad switch finds no gamepad (0x41E3B0), until a key.
    NotDetected,
    /// A wipe's wait before step `step`.
    Wipe {
        wipe: hall_of_fame::Wipe,
        step: u32,
    },
    /// The best ten, waiting for a key.
    FameWait,
    /// The records, circuit `index` of the circuit order; an arrow lit for 8 waits.
    Records {
        index: usize,
    },
    RecordsArrow {
        index: usize,
        right: bool,
        waits: u32,
    },
    /// `drawYesNoMenu` for the exit question; `yes` is the side selected.
    Exit {
        second: bool,
        yes: bool,
    },
    /// `showEndScreen`: the menu to black, `END.BMP` in, held, out with the music.
    EndToBlack {
        step: u32,
    },
    EndIn {
        step: u32,
    },
    EndHold {
        ticks: u32,
    },
    EndOut {
        step: u32,
    },
    /// The game has ended.
    Ended,
    /// `showCredits`: the menu out (50 down to 0), each credits screen in, held, out, the
    /// menu back in.
    CreditsOut {
        step: u32,
    },
    CreditsIn {
        screen: usize,
        step: u32,
    },
    CreditsHold {
        screen: usize,
    },
    CreditsToBlack {
        screen: usize,
        step: u32,
    },
    CreditsBack {
        step: u32,
    },
}

/// The menus below the main menu, each read by `readEventInMenu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Submenu {
    Start,
    Configure,
    Keyboard,
    Pad,
}

#[derive(Debug)]
pub(crate) struct Menu {
    assets: Assets,
    graphics: Graphics,
    /// The original's screen buffer, the shown buffer, and the credits' copy of the screen.
    screen: Canvas,
    shown: Canvas,
    saved: Canvas,
    palette: MenuPalette,
    keys: Keys,
    sound: Sound,
    audio: Vec<i16>,
    main: MenuTable,
    /// Start, Configure, Define Keyboard, Define Gamepad, by [`Submenu`].
    submenus: [MenuTable; 4],
    panel: Panel,
    /// The player's `dr.cfg`, and whether the original would write it now.
    config: DrCfg,
    save: bool,
    /// The second screen buffer the Hall of Fame is drawn into before its wipe, and the
    /// menu music's order while the Hall of Fame plays its own.
    back: Canvas,
    music_order: usize,
    /// The cursor's frame (0x45FBF8).
    cursor: usize,
    state: State,
}

impl Menu {
    /// Takes over from the startup when the title has faded in: the title is shown at its
    /// last fade step, `title_shown`, and the menu stands at `transitionToBlack`'s first wait.
    pub(crate) fn new(
        assets: Assets,
        sound: Sound,
        keys: Keys,
        audio: Vec<i16>,
        title_shown: &deadrally_gamedata::image::Palette,
        (config, save): (DrCfg, bool),
    ) -> Menu {
        let menu_assets = &assets.menu;
        let colour = menu_assets.copper.0[PLAYER_COLOUR];
        let mut palette =
            MenuPalette::new(&menu_assets.palette, colour, &menu_assets.background_copper);
        palette.show(title_shown, 100);
        let mut graphics = Graphics::new(menu_assets);
        graphics.set_row(
            CONFIGURE_MENU.text,
            configure::SWITCH_ROW,
            configure::switch_text(&menu_assets.texts.configure, &config),
        );
        let panel = Panel::startup(&menu_assets.texts);
        let mut shown = Canvas::default();
        shown.copy_all(&assets.title.image);
        Menu {
            graphics,
            screen: Canvas::default(),
            shown,
            saved: Canvas::default(),
            palette,
            keys,
            sound,
            audio,
            main: MAIN_MENU,
            submenus: [START_MENU, CONFIGURE_MENU, KEYBOARD_MENU, PAD_MENU],
            panel,
            config,
            save,
            back: Canvas::default(),
            music_order: 0,
            cursor: 0,
            state: State::TitleToBlack { step: 0 },
            assets,
        }
    }

    pub(crate) fn input(&mut self, event: crate::InputEvent) {
        self.keys.event(event);
    }

    /// The player chose to exit and the end screen is over.
    pub(crate) fn quit_requested(&self) -> bool {
        self.state == State::Ended
    }

    pub(crate) fn tick(&mut self) {
        self.keys.tick();
        self.state = self.run();
        self.sound.render(AUDIO_FRAMES_PER_TICK, &mut self.audio);
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            pixels: self.shown.pixels(),
            palette: &self.palette.shown().0,
            aspect: (4, 3),
        }
    }

    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }

    /// The code from the wait at `self.state` to the next wait.
    fn run(&mut self) -> State {
        match self.state {
            State::TitleToBlack { step } => {
                let title = self.assets.title.palette.clone();
                self.palette.show(&title, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::TitleToBlack { step: step + 1 };
                }
                self.set_up();
                State::FadeIn { step: 0 }
            }
            State::FadeIn { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    return State::FadeIn { step: step + 1 };
                }
                self.shown = self.screen.clone();
                State::Main { second: false }
            }
            State::Main { second: false } => {
                self.palette.after_wait();
                State::Main { second: true }
            }
            State::Main { second: true } => {
                self.palette.after_wait();
                self.update_cursor_main();
                self.main_key()
            }
            State::Submenu {
                menu,
                second: false,
            } => {
                self.palette.after_wait();
                State::Submenu { menu, second: true }
            }
            State::Submenu { menu, second: true } => {
                self.palette.after_wait();
                self.graphics.update_cursor(
                    &mut self.screen,
                    &mut self.shown,
                    &self.submenus[menu as usize],
                    self.cursor,
                );
                self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
                self.submenu_key(menu)
            }
            State::Volume { music, level, last } => self.volume_tick(music, level, last),
            State::KeyWait { control, key } => self.key_wait(control, key),
            State::PadWait { control, polls } => self.pad_wait(control, polls),
            State::NotDetected => self.not_detected(),
            State::Wipe { wipe, step } => self.wipe_tick(wipe, step),
            State::FameWait => self.fame_wait(),
            State::Records { index } => self.records_tick(index),
            State::RecordsArrow {
                index,
                right,
                waits,
            } => self.records_arrow(index, right, waits),
            State::Exit { second: false, yes } => {
                self.palette.after_wait();
                State::Exit { second: true, yes }
            }
            State::Exit { second: true, yes } => {
                self.palette.after_wait();
                self.exit_key(yes)
            }
            State::EndToBlack { step } => {
                self.palette.fade(100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::EndToBlack { step: step + 1 };
                }
                self.screen.copy_all(&self.assets.menu.end.image);
                self.shown = self.screen.clone();
                State::EndIn { step: 0 }
            }
            State::EndIn { step } => {
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    State::EndIn { step: step + 1 }
                } else {
                    State::EndHold { ticks: 0 }
                }
            }
            State::EndHold { ticks } => {
                // `do { wait; i++ } while (!eventDetected() && i < 560)`.
                if self.keys.take() != 0 || ticks + 1 >= END_HOLD_TICKS {
                    State::EndOut { step: 0 }
                } else {
                    State::EndHold { ticks: ticks + 1 }
                }
            }
            State::EndOut { step } => {
                self.sound
                    .set_mask((END_VOLUME - END_VOLUME_STEP * step) >> 8);
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    State::EndOut { step: step + 1 }
                } else {
                    // `mainMenu` writes `dr.cfg` after the end screen.
                    self.save = true;
                    State::Ended
                }
            }
            State::Ended => State::Ended,
            State::CreditsOut { step } => {
                // `for (e = 50; e >= 0; e--)`: here `step` counts e down.
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step > 0 {
                    return State::CreditsOut { step: step - 1 };
                }
                self.show_credits(0);
                State::CreditsIn { screen: 0, step: 0 }
            }
            State::CreditsIn { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    return State::CreditsIn {
                        screen,
                        step: step + 1,
                    };
                }
                // A key is checked before the first wait: one pressed during the fade-in moves
                // on at once.
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsHold { screen } => {
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsToBlack { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::CreditsToBlack {
                        screen,
                        step: step + 1,
                    };
                }
                if screen == 0 {
                    self.show_credits(1);
                    return State::CreditsIn { screen: 1, step: 0 };
                }
                self.palette.compose();
                self.screen = self.saved.clone();
                self.shown = self.screen.clone();
                State::CreditsBack { step: 0 }
            }
            State::CreditsBack { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    State::CreditsBack { step: step + 1 }
                } else {
                    self.main_pass()
                }
            }
        }
    }

    /// `mainMenu` after the title: the background, the bottom panel, the main menu, the
    /// palette composed; the menu's fade-in follows.
    fn set_up(&mut self) {
        self.screen.copy_all(&self.graphics.background);
        self.graphics
            .panel_frame(&mut self.screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut self.screen, &self.panel);
        self.draw_main();
        self.shown = self.screen.clone();
        self.palette.compose();
    }

    /// The top of `mainMenu`'s loop: rows 84..=366 restored, the main menu drawn with focus.
    fn draw_main(&mut self) {
        self.screen.copy_rows(&self.graphics.background, 84, 283);
        self.main.active[1] = false;
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Focused, self.cursor);
    }

    /// A pass of `mainMenu`'s loop without the fade: drawn, shown, waiting for a key.
    fn main_pass(&mut self) -> State {
        self.draw_main();
        self.shown = self.screen.clone();
        State::Main { second: false }
    }

    fn update_cursor_main(&mut self) {
        self.graphics
            .update_cursor(&mut self.screen, &mut self.shown, &self.main, self.cursor);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
    }

    fn sound(&mut self, effect: u8) {
        self.sound.trigger_at(
            SOUND_CHANNEL,
            effect,
            self.config.effects_volume(),
            SOUND_PITCH,
        );
    }

    /// Moves the highlight of `menu` (the main menu when `None`) for Up, Down or Escape.
    fn move_highlight(&mut self, menu: Option<Submenu>, key: u8) {
        let menu = match menu {
            None => &mut self.main,
            Some(submenu) => &mut self.submenus[submenu as usize],
        };
        let (to, base) = match key {
            keys::UP | keys::PAD_UP => {
                let mut row = menu.selected;
                loop {
                    row = if row == 0 { menu.rows - 1 } else { row - 1 };
                    if menu.active[row] {
                        break (row, 6);
                    }
                }
            }
            keys::DOWN | keys::PAD_DOWN => {
                let mut row = menu.selected;
                loop {
                    row = if row + 1 >= menu.rows { 0 } else { row + 1 };
                    if menu.active[row] {
                        break (row, 5);
                    }
                }
            }
            _ => (menu.rows - 1, 6),
        };
        self.graphics.move_highlight(
            &mut self.screen,
            &mut self.shown,
            menu,
            to,
            base,
            self.cursor,
        );
    }

    /// The key read at the end of a main menu pass.
    fn main_key(&mut self) -> State {
        match self.keys.take() {
            keys::ESCAPE => {
                if self.main.selected != self.main.rows - 1 {
                    self.move_highlight(None, keys::ESCAPE);
                    self.sound(MOVE_SOUND);
                }
            }
            keys::ENTER | keys::SPACE | 0x9C => {
                self.sound(CHOOSE_SOUND);
                return self.choose(self.main.selected);
            }
            key @ (keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN) => {
                self.move_highlight(None, key);
                self.sound(MOVE_SOUND);
            }
            _ => {}
        }
        State::Main { second: false }
    }

    /// What a main menu row does.
    fn choose(&mut self, row: usize) -> State {
        match row {
            START_ROW => self.submenu_pass(Submenu::Start),
            CONFIGURE_ROW => self.submenu_pass(Submenu::Configure),
            HALL_OF_FAME_ROW => self.open_hall_of_fame(),
            CREDITS_ROW => {
                self.saved = self.screen.clone();
                self.palette.compose();
                State::CreditsOut {
                    step: MENU_FADE_STEPS,
                }
            }
            EXIT_ROW => self.ask_exit(),
            // Multiplayer is never active.
            _ => self.main_pass(),
        }
    }

    /// `mainMenu`'s exit question: the main menu dimmed over itself, the question's popup,
    /// "no" selected.
    fn ask_exit(&mut self) -> State {
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics
            .popup(&mut self.screen, 170, 200, 300, 80, Focus::Focused);
        let question = &self.assets.menu.texts.exit_question;
        self.graphics.small[0].draw(&mut self.screen, question, at(253, 208));
        self.draw_yes_no(false);
        self.shown = self.screen.clone();
        State::Exit {
            second: false,
            yes: false,
        }
    }

    /// The two answers, the selected one in big A.
    fn draw_yes_no(&mut self, yes: bool) {
        let texts = &self.assets.menu.texts;
        let (yes_font, no_font) = if yes {
            (&self.graphics.big_a, &self.graphics.big_b)
        } else {
            (&self.graphics.big_b, &self.graphics.big_a)
        };
        yes_font.draw(
            &mut self.screen,
            &texts.yes,
            at(YES_NO_X + 30, YES_NO_Y - 7),
        );
        no_font.draw(
            &mut self.screen,
            &texts.no,
            at(YES_NO_X + 200, YES_NO_Y - 7),
        );
    }

    fn exit_key(&mut self, yes: bool) -> State {
        let cursor_x = if yes { YES_NO_X + 7 } else { YES_NO_X + 177 };
        let cursor_at = at(cursor_x, YES_NO_Y);
        self.screen.fill(cursor_at, 20, 20, POPUP_FILL);
        let cursor = self.graphics.cursor(self.cursor).clone();
        self.screen.draw(&cursor, cursor_at, true);
        self.shown
            .copy_from(&self.screen, at(YES_NO_X + 2, YES_NO_Y), 240, 28);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
        let key = match self.keys.take() {
            keys::Y => keys::PAD_LEFT,
            keys::N => keys::PAD_RIGHT,
            key => key,
        };
        let answer = match key {
            keys::LEFT | keys::PAD_LEFT | keys::RIGHT | keys::PAD_RIGHT => {
                let left = matches!(key, keys::LEFT | keys::PAD_LEFT);
                if left != yes {
                    self.sound(MOVE_SOUND);
                }
                self.screen
                    .fill(at(YES_NO_X + 2, YES_NO_Y), 240, 25, POPUP_FILL);
                self.draw_yes_no(left);
                return State::Exit {
                    second: false,
                    yes: left,
                };
            }
            keys::ESCAPE => false,
            keys::ENTER | 0x9C => yes,
            _ => {
                return State::Exit { second: false, yes };
            }
        };
        self.sound(CHOOSE_SOUND);
        if answer {
            self.palette.compose();
            State::EndToBlack { step: 0 }
        } else {
            self.main_pass()
        }
    }

    /// Credits screen `screen` drawn and shown, its palette black.
    fn show_credits(&mut self, screen: usize) {
        self.screen
            .copy_all(&self.assets.menu.credits[screen].image);
        self.shown = self.screen.clone();
    }
}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (305 passed, 18 ignored).

Check that the tests can fail, one mutation at a time, each undone after (`hall_of_fame.rs` is not in Git yet: undo by hand): dropping `self.sound.set_mask(FULL_MASK)` after the wipe back to the menu, Left stepping like Right, `ARROW_WAITS` 4, dropping `self.config.upper_case_hall_of_fame()`, "10." at x 36, `TILE` 16; each fails one test.

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (18 passed): the menu and Configure runs never open the Hall of Fame.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat: add the Hall of Fame"
```

---

### Task 3: Checking against the original, and the record

**Files:**
- Create: `scripts/reference/menu-hall-of-fame.scenario`, `crates/headless/tests/hall-of-fame-run.sha256`, `docs/verification/m2c.md`
- Modify: `crates/headless/tests/menu_run.rs`, `README.md`, `CLAUDE.md`

- [ ] **Step 1: Screenshots and the recording**

<!-- write: scripts/reference/menu-hall-of-fame.scenario -->
```text
# The Hall of Fame (spec M2c section 5): the best ten, the records by circuit with Left and
# Right, and the way back to the main menu, with shots through every change of screen.
at 89500 shot idle
at 90000 key Down
at 90500 key Down
at 91000 key Return
at 91100 shot fame-91100
at 91200 shot fame-91200
at 91300 shot fame-91300
at 91400 shot fame-91400
at 91500 shot fame-91500
at 91600 shot fame-91600
at 91700 shot fame-91700
at 91800 shot fame-91800
at 91900 shot fame-91900
at 92000 shot fame-92000
at 92100 shot fame-92100
at 92200 shot fame-92200
at 92300 shot fame-92300
at 92400 shot fame-92400
at 92500 shot fame-92500
at 92600 shot fame-92600
at 92700 shot fame-92700
at 92800 shot fame-92800
at 92900 shot fame-92900
at 94000 shot fame
at 95000 key space
at 95100 shot records-95100
at 95200 shot records-95200
at 95300 shot records-95300
at 95400 shot records-95400
at 95500 shot records-95500
at 95600 shot records-95600
at 95700 shot records-95700
at 95800 shot records-95800
at 95900 shot records-95900
at 96000 shot records-96000
at 96100 shot records-96100
at 96200 shot records-96200
at 96300 shot records-96300
at 96400 shot records-96400
at 96500 shot records-96500
at 96600 shot records-96600
at 96700 shot records-96700
at 96800 shot records-96800
at 96900 shot records-96900
at 97500 shot records
at 98000 key Right
at 98050 shot right-98050
at 98100 shot right-98100
at 98150 shot right-98150
at 98200 shot right-98200
at 98250 shot right-98250
at 98300 shot right-98300
at 98350 shot right-98350
at 99000 shot after-right
at 99500 key Left
at 100000 shot after-left
at 100500 key Left
at 100550 shot left-100550
at 100600 shot left-100600
at 100650 shot left-100650
at 100700 shot left-100700
at 100750 shot left-100750
at 100800 shot left-100800
at 100850 shot left-100850
at 101500 shot after-left-2
at 102000 key Escape
at 102100 shot back-102100
at 102200 shot back-102200
at 102300 shot back-102300
at 102400 shot back-102400
at 102500 shot back-102500
at 102600 shot back-102600
at 102700 shot back-102700
at 102800 shot back-102800
at 102900 shot back-102900
at 103000 shot back-103000
at 103100 shot back-103100
at 103200 shot back-103200
at 103300 shot back-103300
at 103400 shot back-103400
at 103500 shot back-103500
at 103600 shot back-103600
at 103700 shot back-103700
at 103800 shot back-103800
at 103900 shot back-103900
at 105000 shot menu-back
at 108000 shot menu-later
```

Run:
```bash
scripts/reference-run.sh scripts/reference/menu-hall-of-fame.scenario captures/hof
/tmp/guard-real-sink.sh scripts/reference-run.sh --sound scripts/reference/menu-hall-of-fame.scenario captures/hof-sound
cargo build --release -p deadrally-headless
```
Expected: `done: 79 shots` twice; `guard: command exit 0, leaks 0`. Find the keys' ticks as `docs/verification/m2c.md` says (the shot before the first key anchors a line of 14 ms a tick; the Space and Escape keys may need to move a tick or two, which the wipes after them show), then `find` prints a `ticks` line for all 79 shots, and `render-audio --startup` with the sound run's keys, `--seconds 112`, against `captures/hof-sound/sound.wav` with `--min-overlap 100` gives `result: PASS`.

- [ ] **Step 2: Pin the run and write the record**

<!-- write: crates/headless/tests/menu_run.rs -->
```rust
//! A run through the main menu against the developer's real game data. Run with
//! `cargo test-data`; the test reads DEADRALLY_DATA and fails (never passes silently) when it is
//! unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{Game, InputEvent, Key};
use deadrally_gamedata::assets::Assets;
use sha2::{Digest, Sha256};

/// The keys of `scripts/reference/menu-keys.scenario`, at the ticks where the original read
/// them in the run its screenshots come from (docs/verification/m2a.md): the highlight up and
/// down, Escape, the start submenu, the exit question, the credits and the end screen.
const KEYS: [(u64, Key); 30] = [
    (6423, Key::Down),
    (6495, Key::Down),
    (6566, Key::Up),
    (6638, Key::Up),
    (6709, Key::Down),
    (6780, Key::Escape),
    (6852, Key::Up),
    (6923, Key::Up),
    (6995, Key::Up),
    (7066, Key::Up),
    (7137, Key::Enter),
    (7209, Key::Down),
    (7280, Key::Down),
    (7352, Key::Enter),
    (7423, Key::Enter),
    (7495, Key::Escape),
    (7566, Key::Escape),
    (7637, Key::Enter),
    (7709, Key::Left),
    (7780, Key::Right),
    (7852, Key::Escape),
    (7923, Key::Up),
    (7995, Key::Enter),
    (8138, Key::Space),
    (8280, Key::Space),
    (8423, Key::Down),
    (8495, Key::Enter),
    (8566, Key::Left),
    (8638, Key::Enter),
    (8923, Key::Space),
];

/// The frame after each of these ticks equals the scenario's screenshot of that name.
const SHOTS: [(u64, &str); 47] = [
    (6388, "idle"),
    (6459, "after-90000"),
    (6530, "after-91000"),
    (6602, "after-92000"),
    (6673, "after-93000"),
    (6745, "after-94000"),
    (6816, "after-95000"),
    (6888, "after-96000"),
    (6959, "after-97000"),
    (7030, "after-98000"),
    (7102, "after-99000"),
    (7173, "after-100000"),
    (7245, "after-101000"),
    (7316, "after-102000"),
    (7387, "after-103000"),
    (7459, "after-104000"),
    (7530, "after-105000"),
    (7602, "after-106000"),
    (7673, "after-107000"),
    (7745, "after-108000"),
    (7816, "after-109000"),
    (7887, "after-110000"),
    (7959, "after-111000"),
    (8009, "credits-112200"),
    (8023, "credits-112400"),
    (8045, "credits-112700"),
    (8066, "credits-113000"),
    (8101, "credits-113500"),
    (8151, "credits-114200"),
    (8172, "credits-114500"),
    (8194, "credits-114800"),
    (8223, "credits-115200"),
    (8294, "credits-116200"),
    (8315, "credits-116500"),
    (8344, "credits-116900"),
    (8387, "credits-117500"),
    (8458, "after-118000"),
    (8530, "after-119000"),
    (8601, "after-120000"),
    (8651, "end-121200"),
    (8672, "end-121500"),
    (8694, "end-121800"),
    (8723, "end-122200"),
    (8780, "end-123000"),
    (8851, "end-124000"),
    (8929, "end-125100"),
    (8937, "end-125200"),
];

/// The menu run ends when the game asks to quit, a little after its last screenshot.
const MAX_TICKS: u64 = 9_000;

/// The keys of `scripts/reference/menu-configure.scenario` in the run of
/// docs/verification/m2b.md: both volume popups, Define Keyboard with Q for accelerate, Define
/// Gamepad, the gamepad switch (no gamepad under Wine), Escape and "previous menu".
const CONFIGURE_KEYS: [(u64, Key); 24] = [
    (6420, Key::Down),
    (6491, Key::Enter),
    (6563, Key::Enter),
    (6634, Key::Left),
    (6656, Key::Left),
    (6677, Key::Left),
    (6742, Key::Enter),
    (6813, Key::Down),
    (6849, Key::Enter),
    (6920, Key::Right),
    (6992, Key::Enter),
    (7027, Key::Down),
    (7063, Key::Enter),
    (7134, Key::Enter),
    (7206, Key::Q),
    (7277, Key::Escape),
    (7349, Key::Down),
    (7385, Key::Enter),
    (7456, Key::Escape),
    (7528, Key::Down),
    (7563, Key::Enter),
    (7635, Key::Space),
    (7706, Key::Escape),
    (7778, Key::Enter),
];

const CONFIGURE_SHOTS: [(u64, &str); 17] = [
    (6384, "idle"),
    (6527, "configure"),
    (6598, "music"),
    (6705, "music-left"),
    (6777, "after-music"),
    (6884, "effects"),
    (6955, "effects-right"),
    (7098, "keyboard"),
    (7170, "press-key"),
    (7241, "after-q"),
    (7312, "after-keyboard"),
    (7419, "gamepad"),
    (7491, "after-gamepad"),
    (7598, "not-detected"),
    (7670, "after-not-detected"),
    (7741, "escape"),
    (7812, "previous"),
];

/// The keys of `scripts/reference/menu-hall-of-fame.scenario` in the run of
/// docs/verification/m2c.md: into the Hall of Fame, on to the records, Right, Left twice and
/// Escape.
const HALL_OF_FAME_KEYS: [(u64, Key); 8] = [
    (6455, Key::Down),
    (6491, Key::Down),
    (6527, Key::Enter),
    (6814, Key::Space),
    (7027, Key::Right),
    (7134, Key::Left),
    (7205, Key::Left),
    (7314, Key::Escape),
];

const HALL_OF_FAME_SHOTS: [(u64, &str); 78] = [
    (6420, "idle"),
    (6534, "fame-91100"),
    (6541, "fame-91200"),
    (6548, "fame-91300"),
    (6556, "fame-91400"),
    (6563, "fame-91500"),
    (6570, "fame-91600"),
    (6577, "fame-91700"),
    (6584, "fame-91800"),
    (6591, "fame-91900"),
    (6598, "fame-92000"),
    (6606, "fame-92100"),
    (6613, "fame-92200"),
    (6620, "fame-92300"),
    (6627, "fame-92400"),
    (6634, "fame-92500"),
    (6641, "fame-92600"),
    (6648, "fame-92700"),
    (6655, "fame-92800"),
    (6663, "fame-92900"),
    (6741, "fame"),
    (6820, "records-95100"),
    (6827, "records-95200"),
    (6834, "records-95300"),
    (6841, "records-95400"),
    (6848, "records-95500"),
    (6856, "records-95600"),
    (6863, "records-95700"),
    (6870, "records-95800"),
    (6877, "records-95900"),
    (6884, "records-96000"),
    (6891, "records-96100"),
    (6898, "records-96200"),
    (6906, "records-96300"),
    (6913, "records-96400"),
    (6920, "records-96500"),
    (6927, "records-96600"),
    (6934, "records-96700"),
    (6941, "records-96800"),
    (6948, "records-96900"),
    (6991, "records"),
    (7031, "right-98050"),
    (7034, "right-98100"),
    (7038, "right-98150"),
    (7041, "right-98200"),
    (7045, "right-98250"),
    (7049, "right-98300"),
    (7052, "right-98350"),
    (7099, "after-right"),
    (7170, "after-left"),
    (7209, "left-100550"),
    (7213, "left-100600"),
    (7216, "left-100650"),
    (7220, "left-100700"),
    (7224, "left-100750"),
    (7227, "left-100800"),
    (7231, "left-100850"),
    (7277, "after-left-2"),
    (7320, "back-102100"),
    (7327, "back-102200"),
    (7334, "back-102300"),
    (7341, "back-102400"),
    (7349, "back-102500"),
    (7356, "back-102600"),
    (7363, "back-102700"),
    (7370, "back-102800"),
    (7377, "back-102900"),
    (7384, "back-103000"),
    (7392, "back-103100"),
    (7399, "back-103200"),
    (7406, "back-103300"),
    (7413, "back-103400"),
    (7420, "back-103500"),
    (7427, "back-103600"),
    (7434, "back-103700"),
    (7442, "back-103800"),
    (7449, "back-103900"),
    (7527, "menu-back"),
];

/// One line per screenshot (the frame's pixels and palette), one for the run's sound and one
/// for the last `dr.cfg` it wrote. The run stops at `ticks`, or earlier when the game quits.
fn manifest(keys: &[(u64, Key)], shots: &[(u64, &str)], ticks: u64) -> String {
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let config = assets.menu.default_config.clone();
    let mut game = Game::new(assets, config);
    let mut lines = String::new();
    let mut audio = Vec::new();
    let mut written = None;
    let mut done = 0;
    while !game.quit_requested() && done < ticks {
        for &(_, key) in keys.iter().filter(|(at, _)| *at == done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        game.tick();
        game.take_audio(&mut audio);
        written = game.take_config().or(written);
        done += 1;
        for &(_, name) in shots.iter().filter(|(at, _)| *at == done) {
            let frame = game.frame();
            let mut hasher = Sha256::new();
            hasher.update(frame.pixels);
            hasher.update(frame.palette.as_flattened());
            writeln!(lines, "{}  frame after tick {done} ({name})", hex(hasher)).unwrap();
        }
    }
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  sound of {done} ticks", hex(hasher)).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(written.expect("dr.cfg is written at start-up"));
    writeln!(lines, "{}  dr.cfg written last", hex(hasher)).unwrap();
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_menu_run_matches_the_committed_manifest() {
    // The manifest was written after every screenshot of the run equalled our frame at its
    // tick and the sound was measured against the recording (docs/verification/m2a.md); a
    // change to the menu must not alter a frame or a sound unnoticed.
    let lines = manifest(&KEYS, &SHOTS, MAX_TICKS);
    assert!(
        lines.contains("after tick 8937"),
        "the end screen asks to quit after its fade"
    );
    check_manifest("menu-run.sha256", &lines, "the menu run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_configure_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick, and the
    // dr.cfg written last equalled the original's but for its random byte
    // (docs/verification/m2b.md).
    let lines = manifest(&CONFIGURE_KEYS, &CONFIGURE_SHOTS, 7_900);
    check_manifest("configure-run.sha256", &lines, "the configure run");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_hall_of_fame_run_matches_the_committed_manifest() {
    // Written after every screenshot of the run equalled our frame at its tick and the
    // recording of the run compared as M1b's (docs/verification/m2c.md).
    let lines = manifest(&HALL_OF_FAME_KEYS, &HALL_OF_FAME_SHOTS, 7_600);
    check_manifest("hall-of-fame-run.sha256", &lines, "the hall of fame run");
}
```

<!-- write: crates/headless/tests/hall-of-fame-run.sha256 -->
```text
d5c3994be4827e14e45f02da976eb44d869ee8fa6135a45c78af5459aab8b2c7  frame after tick 6420 (idle)
ec4fb530bd5da08207ef1418a71935ab0ee062ac2a15ad1c772f17363a4bfaaa  frame after tick 6534 (fame-91100)
315ec1e51454e343c9fbabda33ad6ee447657ccd9314884b54c2a0f897de0074  frame after tick 6541 (fame-91200)
1dbd035bc63ef999d166dafe8bf7389e9b834ed079b2e5b4a3bffc6c61a552e7  frame after tick 6548 (fame-91300)
a98ee2030389920f0e2d71fe1351a684f7fe6f0fceb5928b6059265e01f42523  frame after tick 6556 (fame-91400)
49524ffd3cc3c55c2575482294498860158c7c5dafee13299b4d942cfbc4a87b  frame after tick 6563 (fame-91500)
0e86f907f89155b9e6605e51f7c76e85e4123bb3e6689c39c756f2ab50613e9e  frame after tick 6570 (fame-91600)
3f68b52c2fe77e26220c9f2cfce39bf5e4e46413b95139e898c05ddf16446cd4  frame after tick 6577 (fame-91700)
9a3b9495edcde93f3abb6c3ef50d5e30b6b5831bcd15b34bfd4a85c323ed8216  frame after tick 6584 (fame-91800)
574c7028ef71503bd4f6c3a0d0598b3bb9516c2b0ecd0ca8c75bbf093975445f  frame after tick 6591 (fame-91900)
730eb114a4645e6da8edae8a21293954a8be87ab2287984aa5fb0dd6e9b88f97  frame after tick 6598 (fame-92000)
730eb114a4645e6da8edae8a21293954a8be87ab2287984aa5fb0dd6e9b88f97  frame after tick 6606 (fame-92100)
574c7028ef71503bd4f6c3a0d0598b3bb9516c2b0ecd0ca8c75bbf093975445f  frame after tick 6613 (fame-92200)
9a3b9495edcde93f3abb6c3ef50d5e30b6b5831bcd15b34bfd4a85c323ed8216  frame after tick 6620 (fame-92300)
3f68b52c2fe77e26220c9f2cfce39bf5e4e46413b95139e898c05ddf16446cd4  frame after tick 6627 (fame-92400)
6c0535bc4c6609cca7139756f140dfe95b01e9fdfeb946c59dbba3da92898644  frame after tick 6634 (fame-92500)
1d20d568dfe0fc410d12182527822c017f5b93ac4036aeb73ad0e21f392e3535  frame after tick 6641 (fame-92600)
65a122873bb48c9d645a2b16080814d389e366da66336ab732a4b2dc54a96ae4  frame after tick 6648 (fame-92700)
ba8260e98badb3577a8d98ab83426593e65477baf0fb884019af696e5f908de3  frame after tick 6655 (fame-92800)
f91d3b48acdd624a0d03d4f6bba00bd61189c9f3b256e88cd1e8c70e4d79dde3  frame after tick 6663 (fame-92900)
adb8708700579020a9495b3c097ff270dc6fbc4187b5a27ac2f320b44459b507  frame after tick 6741 (fame)
3f4b5bda5d4cced5145097e1b67bcdb366bf91e4a3c8ac4d9f8a968d97a2437e  frame after tick 6820 (records-95100)
8fe0118ffc6a3a68c218800b5265f4fc693e88cce72d31724be617013dc1a982  frame after tick 6827 (records-95200)
6f6645afb700584c4a751b65ce20f180f2ff117e299ea4e0abed5fc27275f08c  frame after tick 6834 (records-95300)
e0538aa7480429679a1955dcd2390ce68ed0143c080baeb67e829950052d6026  frame after tick 6841 (records-95400)
e0f563886b7c99bab08c2c67c16b29ccd663e30aab3cf07bc9a5fad2942c0e16  frame after tick 6848 (records-95500)
d9fd6734f4e0d93379fdb7a1cdadffd527298dd0a694a8faff6a9c76c4d1f52e  frame after tick 6856 (records-95600)
870888ef842236adaeca89dc4aa35572c50c12adde19153e9498be1b81af3a8c  frame after tick 6863 (records-95700)
5d6e3ee40bd1f94d9505a3faadff1fc64499857d844e087b28bbaacb3b5222fc  frame after tick 6870 (records-95800)
0cb991a61d6362c4574eb0ebcf232806760cf549e9f7ed6bb712e920b2f0dc4b  frame after tick 6877 (records-95900)
10899d8bad89efb06912945d01ca1d55c6245cc7408abec37130bca4fa0b5daf  frame after tick 6884 (records-96000)
9d20078cf80ff143fd7c3726bef3476131484010e15ecee2f28eb09322604091  frame after tick 6891 (records-96100)
10899d8bad89efb06912945d01ca1d55c6245cc7408abec37130bca4fa0b5daf  frame after tick 6898 (records-96200)
98bce12b0d7e2db8f1545e1f27f17ba4a6855603adcd90d59b3928b49f8c5c48  frame after tick 6906 (records-96300)
7124dadb9fe16d0e2c9e2225d93fa9c155856b1381d65dc755734c8438dbe544  frame after tick 6913 (records-96400)
eddfe7877b2e2272aca5aeb24ab0e766d9da34681e7ed19b469bfcf4afe90fd8  frame after tick 6920 (records-96500)
8fcae1924c0f44ab271c379d35853fbeaa87815f33a3a60e2be5c5180f5e0d9d  frame after tick 6927 (records-96600)
9c799aaef524d2102a0f2b59789e7e332792befa0ca7f4bc37c6aba85fba3f19  frame after tick 6934 (records-96700)
60d81efe3d03933023d9f81c4903ebabe04b4e06316ecde77b9a7c153d9b058f  frame after tick 6941 (records-96800)
4003545eef04c7c3f7e3b4ed75ee864bd63e03b35572e63a74e27a63fefcf715  frame after tick 6948 (records-96900)
8fcae1924c0f44ab271c379d35853fbeaa87815f33a3a60e2be5c5180f5e0d9d  frame after tick 6991 (records)
53f7f2e532cd99a5b0f3e8b3cdc69bcd368631a993f3f3d619cb347cc1e4365c  frame after tick 7031 (right-98050)
91fec71b8eda63bbbe03bfe2119d5df8facbdd6ebe4317360addb63f674b5258  frame after tick 7034 (right-98100)
511ddf272f49a225e152130b19ab8ca946efc1f9709d6b1ab97d52f105370658  frame after tick 7038 (right-98150)
50dcff6d97655ecc53bcb1a127ec7712c5b1b8b24dbd061911e61770a7a09e06  frame after tick 7041 (right-98200)
95d8808a23af435057cf42525ae3f9c15f7fc0c94f8d142b9df801b9239b1dd6  frame after tick 7045 (right-98250)
cc8f1d64e3a3fab32e586aea9888428182071ae0ba895036d034f421292ca2ff  frame after tick 7049 (right-98300)
493f90aa04537d2067b1e8cea3124697e79cb0b241856df43e751a118f4a5f7c  frame after tick 7052 (right-98350)
014fe56630731b5888f660ce5b8a5a13042e76feeb2d60959e7344ecd7b34511  frame after tick 7099 (after-right)
8ca3c15d18fbdea7e8870ad777326c66c1884650fac13513473b5ea04aecccce  frame after tick 7170 (after-left)
bd91bbbb39f5a1d8043b2561e8a8eb93e62ab1dc0ce89c9d6b98a9fae6008c7d  frame after tick 7209 (left-100550)
37019ee3a8fa46a06d4e2f3f10951b50d7fe5b715fb6af23267546ce3f39285e  frame after tick 7213 (left-100600)
20616abe562b9d4117adef91b75900bd911f697a47aac46c7f7c0519e63fb731  frame after tick 7216 (left-100650)
7d69c515adc4484d5aa3d3441e51d5a813a2da32f60686f74ec152ea89bbd0d2  frame after tick 7220 (left-100700)
af11d14e120442273288e06621a5633494ff9119842f6eb536b06e719b7c2ed9  frame after tick 7224 (left-100750)
37c09c44fa7b57530ed20c9896ff7828f304b0b2ac0991c5c125dc2b6edce2b3  frame after tick 7227 (left-100800)
735c6ebda08c437405aa6a44962403fd4312e08210dc839afcce7cc1cac6b921  frame after tick 7231 (left-100850)
fe7313b0dabfa60f9551f0d2c4fe5516ce70de56c862a766ef6fd83c78310dcf  frame after tick 7277 (after-left-2)
1ad139f84cdfd8658c9e972c568404fb6aa67537862cc0b521489f24808737a3  frame after tick 7320 (back-102100)
3a7c45dfebe077347a85051a37380d806f4323d974bfcc7a916984d7a4c3d504  frame after tick 7327 (back-102200)
ca0fc8544fbbbf322401d68e154d00caee082770ac84e1b43761dfc8f1d12f41  frame after tick 7334 (back-102300)
659885f01bbcec76a5fd3c5bba8fb8bb181d2de1ead14f522003d95d83dd9dd4  frame after tick 7341 (back-102400)
08f8a78e73aad5a922169590c2fdf44482fb39533f58c0f370100c88d9f02445  frame after tick 7349 (back-102500)
eaa9661c93415b693a0f003762b76d534b29bd174c415d2c385f647720a5701c  frame after tick 7356 (back-102600)
f774fc1629a660b73662d2287e7eb24da3e4ec8544621bc472558741b2928100  frame after tick 7363 (back-102700)
cfbfee6a540e66487ae8334ea5dda413fb20aa231357db13156fb4556c607c3a  frame after tick 7370 (back-102800)
5419f8c4e959e530cdea74460cb661672db226d702a150450bd452fc8dca36c1  frame after tick 7377 (back-102900)
e58e344c9f3c6f445dd0a78cded7f240e56a5ac99dc9a370fda672f72baf655e  frame after tick 7384 (back-103000)
97c8c8cb67d2f3d8d1cd66d4a9096392145810e0c0ab4f76f1063c54d3d0526a  frame after tick 7392 (back-103100)
18f09b6bf18748548f440340ec8f75064e974d3c2f2b9b6db88fe2d2af98d080  frame after tick 7399 (back-103200)
669ca4e6c647d213e8924e96735ab96695e321cdabd810aa5300e7da6d9c7410  frame after tick 7406 (back-103300)
06fdcd19c1d5454fcdcf2ca0738261603a0e14277676564a4faf8abdd1d3b22a  frame after tick 7413 (back-103400)
ddd60346a55e5d90d678f898c37e0addf86da8b4116859a75cc93d976361eed8  frame after tick 7420 (back-103500)
83f5b7e1119189ad74fa9cec8fac8e8f6dfb35406a56cdf23f42527795b49ac4  frame after tick 7427 (back-103600)
c51e5256bd14a46195434014b537ff3485a2c4e61990029994070588c809593c  frame after tick 7434 (back-103700)
65f9bcd00016d3e321cc44e5a8fa934eb00cad319ea5c516f2d126b3bd400de0  frame after tick 7442 (back-103800)
02342bdddbfbe2ea02a4b1cbbb56c64a18e874dfa1fedbab818ae4e51d3f0dc6  frame after tick 7449 (back-103900)
6ff99620e9252d43a93778ca654db2963fb3d161de176d685cbbf841de373ee5  frame after tick 7527 (menu-back)
060bd73946789ecad1753d1472460c1f4e475e720cc522795479b38ee3fa3964  sound of 7600 ticks
5045e30a6992de3656ed1881731431a4096885fa2fbcf79a032a8e5fd639d366  dr.cfg written last
```

<!-- write: docs/verification/m2c.md -->
````markdown
# M2c: verification against the original

The checks of the M2c spec (section 5), run against the original `dr.exe` under Wine as in [M2a](m2a.md). Screenshots and recordings stay under `captures/`.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| The Hall of Fame's screens | `menu-hall-of-fame` | **Pass.** All 78 shots equal one of our frames: the best ten wiping in over the menu every 100 ms, held, the records wiping in, Right and Left with the arrow lit, and the main menu wiping back. |
| Its sound | `menu-hall-of-fame`, recorded | **Pass.** `compare-audio` over the whole run: loudness per second within 0.34 dB (median) and 1.34 dB (largest), bands within 0.6 dB, tempo +0.080 % over 10 pieces, pitch +0 cents, balance +0.11 dB. Every second of the Hall of Fame, the music's fall, its jump to order 81 and the menu music starting again, is within 1.13 dB. |

## What the original does that the spec did not say

- **The difficulty's name is upper-cased too** before it is drawn: the medium font has no small letters.
- **Frame 0 of the wipe's masks covers its whole tile:** the band's leading tile column shows the least of the new screen, and every column ends covered.
- **Notes keep sounding across the jump to order 81**, as in a tracker; the Hall of Fame's own notes start over them.

## Key ticks

The screenshot run's keys were put on a line of 14 ms a tick from the shot before the first key. Two keys, Space on the best ten and Escape on the records, had to move two ticks later: the wipes that follow them only match when the highlights' pulse is at the original's phase.

## The manifest

`crates/headless/tests/hall-of-fame-run.sha256` holds our frames at the 78 shots' ticks, the run's sound and the last `dr.cfg` written.

## Runs

```
$ deadrally-headless find --ticks 7700 --key-at 6455:down --key-at 6491:down --key-at 6527:enter --key-at 6814:space --key-at 7027:right --key-at 7134:left --key-at 7205:left --key-at 7314:escape captures/hof/*.png
(78 lines, each "ticks ..."; exit 0)
$ deadrally-headless compare-audio captures/hof-sound/sound.wav captures/hof-sound/ours.wav --min-overlap 100
lag: 1910 ms (envelope correlation 0.900)
loudness per second, ours - original: median 0.34 dB, largest 1.34 dB
tempo, ours - original: +0.080 % (over 10 pieces of 10 s)
stereo balance (left - right), ours - original: largest +0.11 dB (over 10 pieces of 10 s)
result: PASS
```

The sound run's scenario has one more shot at 108 s than the screenshot run's, so the recording covers the menu music starting again.
````

<!-- write: README.md -->
````markdown
# DeadRally

Original Death Rally reincarnation for modern systems: a clean, native, 64-bit reimplementation of *Death Rally for Windows* (Remedy, 2009) for Windows, macOS and Linux, written in Rust.

**Status:** M2, the menus. The game starts like the original: the intro with its music and effects, the Apogee and Remedy logos, the title screen, then the main menu with Configure (volumes, keys, gamepad), the Hall of Fame, the credits and the exit, under the menu music. Settings and records are kept in a `dr.cfg` like the original's. Racing comes next. [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) covers the goal, the approach and the roadmap.

The repository contains no game data. You need your own copy of the game: Death Rally (Classic) on Steam (free) or Remedy's 2009 freeware release.

## Quick start

```
scripts/install-linux-deps.sh               # Linux; see CONTRIBUTING.md for macOS and Windows
export DEADRALLY_DATA=~/games/DeathRally    # your copy of the game
cargo run -p deadrally-headless -- check-data
cargo run --release -p deadrally -- -window
```

[CONTRIBUTING.md](CONTRIBUTING.md) has the full setup, the data configuration and the rules.

Licence: GPL-3.0-or-later, see [LICENSE](LICENSE).
````

<!-- write: CLAUDE.md -->
```markdown
# DeadRally: instructions for AI agents

DeadRally is a clean, native reimplementation of *Death Rally* (Remedy, 2009) in Rust. Read `docs/PROJECT_BRIEF.md` for the goal and `docs/superpowers/specs/` for the current design. `CONTRIBUTING.md` has the setup.

## Ground rules (brief §2)

1. **Faithfulness first.** Anything that changes how the game plays (timings, physics, prices, AI) must match the original first. Improvements come later, as options that default to the original behaviour.
2. **Never commit game data:** BPA, HAF, the original exe or DLLs, saves, sound, music, or screenshots that are mostly original art. `.gitignore` and `scripts/check-no-game-data.sh` (run in CI) enforce this. Never `git add -f` such files.
3. **Provenance.** New code is ours (GPL-3.0-or-later). Facts, file formats and constants from DreeRally or dRally are fine: describe them in your own words and credit them. Code copied from dRally (MIT) keeps its notice. Do not paste decompiled DreeRally code; re-implement from understanding. When unsure, ask the owner.
4. **Evidence for every gameplay claim:** a parity log, a side-by-side screenshot, or a reference to the original's code (a DreeRally function with its original address).

## Determinism (`crates/core`)

- No clocks, threads, environment reads, `HashMap`/`HashSet` or libm transcendental functions: `crates/core/clippy.toml` bans them. Frontends pace ticks with `deadrally_core::host::Pacer`.
- Overflow checks are on in every profile. Write intentional wrap-around as `wrapping_*`.
- `unsafe` is forbidden in the whole workspace.
- `deadrally-core` must not depend on platform crates; CI's `core-purity` job checks it.
- Everything a frontend shares (pacing, audio gate, letterbox, stats) belongs in `deadrally_core::host`, not in a frontend.

## Commands

| Command | What it does |
|---|---|
| `cargo fmt --all` | format |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint; CI denies warnings |
| `cargo test --workspace` | tests that need no game data |
| `DEADRALLY_DATA=~/games/DeathRally cargo test-data` | tests that need the original data; they fail when it is unset |
| `cargo run -p deadrally-headless -- check-data` | where the data was found and whether it is a known release |
| `cargo run --release -p deadrally-headless -- run --ticks 7000` | determinism hashes; CI compares them across OSes |
| `cargo run --release -p deadrally -- -window` | the game: the original's startup sequence with its sound, then the main menu (`-testscene`: the M0 test scene) |
| `cargo run --release -p deadrally-headless -- dump-assets` | every catalogued image as PNG under `dumps/` (ignored) |
| `scripts/reference-run.sh scripts/reference/startup.scenario captures/startup` | screenshots of the original under Wine on a virtual display |
| `target/release/deadrally-headless find captures/startup/*.png` | the ticks of our startup sequence that match each screenshot exactly; `--key-at TICK:KEY` presses keys, `--ticks N` runs on into the menus |
| `target/release/deadrally-headless render-audio --startup --seconds 100 --out captures/startup.wav` | the startup's sound as the game plays it, the intro and then the menu music; also `--music NAME`, `--effect BANK --number K` |
| `scripts/reference-run.sh --sound scripts/reference/startup-sound.scenario captures/startup-sound` | the original's sound, recorded from a null sink (nothing reaches the speakers); `--cfg FILE` starts it with another `dr.cfg` |
| `target/release/deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115` | does our render sound like the recording; PASS or FAIL against spec M1b §5 |
| `DEADRALLY_BLESS=1 cargo test-data` | rewrite the manifests `crates/gamedata/tests/decoded-images.sha256`, `crates/headless/tests/rendered-audio.sha256`, `menu-run.sha256`, `configure-run.sha256` and `hall-of-fame-run.sha256`, only after checking the pictures and the sound against the original again |
| `scripts/spike-check.sh screens target/release/deadrally captures/x 10` | screenshots and stats without a monitor (Xvfb; sound to a file) |
| `scripts/fullscreen-check.sh target/release/deadrally captures/fs` | four fullscreen toggles on the real GPU without a monitor (headless Weston) |

## Tests

- Tests encode **why**: the name or a comment says what goes wrong for a player if the behaviour changes.
- `#[ignore]` is only for tests that need game data: `#[ignore = "needs game data (DEADRALLY_DATA)"]`. They read the data through `DEADRALLY_DATA` and fail when it is unset.
- Fixtures are generated by the tests in temporary directories. Never commit files derived from game data; hashes of decoded data are facts and may be committed.
- "Done" means verified. Say which checks ran, and say so when one could not run (CI never runs `cargo test-data`).

## Commits

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`.
- Subject at most 50 characters, imperative, English. A body only when several things changed, as a `- ` list.
- No Co-Authored-By or any other attribution.

## Working as an agent (brief §11)

- Work from a written task: goal, files, evidence required.
- One git worktree per task (under `.worktrees/`, which is ignored). Merge only after an independent review.
- Re-run the key checks yourself before reporting success. Checks that silently did not run are the most common false "done".
```

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && scripts/check-no-game-data.sh && DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (305 passed, 19 ignored; data 19 passed), among them `the_hall_of_fame_run_matches_the_committed_manifest`.

- [ ] **Step 3: Commit**

```bash
git add crates/headless/tests scripts/reference/menu-hall-of-fame.scenario docs README.md CLAUDE.md
git commit -m "test: check the Hall of Fame against the original" -m "- a scenario and a manifest of a run through the Hall of Fame
- the verification record; the README's status"
```

- [ ] **Step 4: Final review, push, CI, merge**

As M2b's plan (Task 5, Steps 4 and 5) with the branch `m2c-hall-of-fame` and the merge message `feat: merge M2c hall of fame`.
