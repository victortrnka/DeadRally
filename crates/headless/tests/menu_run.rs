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
