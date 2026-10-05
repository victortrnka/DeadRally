//! Rendered sound against the developer's real game data. Run with `cargo test-data`; the
//! test reads DEADRALLY_DATA and fails (never passes silently) when it is unset.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use deadrally_core::{AUDIO_SAMPLE_RATE, Game, render_effect, render_music};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::sound::{self, EFFECTS, MUSIC};
use deadrally_gamedata::{DATA_ENV_VAR, Located, locate};
use sha2::{Digest, Sha256};

/// Seconds of each module: several patterns, so most of each module's commands play.
const MUSIC_SECONDS: usize = 30;
/// Seconds of each effect: all of most samples, and a few loops of the looped ones.
const EFFECT_SECONDS: usize = 2;
/// Ticks of the startup after the intro: 10 s of the menu music.
const MENU_MUSIC_TICKS: u32 = 714;

fn located() -> Located {
    let dir = match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    };
    locate(Some(&dir), None, None).unwrap_or_else(|error| panic!("{error}"))
}

fn hash(samples: &[i16], hasher: &mut Sha256) {
    for sample in samples {
        hasher.update(sample.to_le_bytes());
    }
}

fn hex(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// One line per sound: the startup as the game plays it (the whole intro and the first 10 s of
/// the menu music after it), the start of every module and every effect of every bank.
fn manifest(located: &Located) -> String {
    let mut lines = String::new();
    let assets = Assets::load(&located.validation).unwrap_or_else(|error| panic!("{error}"));
    let ticks = assets
        .intro
        .delays
        .iter()
        .map(|&delay| u32::from(delay))
        .sum::<u32>()
        + MENU_MUSIC_TICKS;
    let mut game = Game::new(assets);
    let mut audio = Vec::new();
    for _ in 0..ticks {
        game.tick();
        game.take_audio(&mut audio);
    }
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  startup {ticks} ticks", hex(hasher)).unwrap();

    let archive = Archive::open(&located.validation.dir.join(sound::ARCHIVE))
        .unwrap_or_else(|error| panic!("{error}"));
    let frames = |seconds: usize| seconds * AUDIO_SAMPLE_RATE as usize;
    for name in MUSIC {
        let module = sound::load_music(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        let mut hasher = Sha256::new();
        hash(&render_music(&module, frames(MUSIC_SECONDS)), &mut hasher);
        writeln!(
            lines,
            "{}  {}/{name} first {MUSIC_SECONDS} s",
            hex(hasher),
            sound::ARCHIVE
        )
        .unwrap();
    }
    for name in EFFECTS {
        let bank = sound::load_effects(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        let mut hasher = Sha256::new();
        for (index, instrument) in bank.instruments.iter().enumerate() {
            if instrument.is_some() {
                let effect = u8::try_from(index + 1).expect("banks have under 256 instruments");
                hash(
                    &render_effect(&bank, effect, frames(EFFECT_SECONDS)),
                    &mut hasher,
                );
            }
        }
        writeln!(
            lines,
            "{}  {}/{name} every effect, {EFFECT_SECONDS} s each",
            hex(hasher),
            sound::ARCHIVE
        )
        .unwrap();
    }
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn rendered_sound_matches_the_committed_manifest() {
    // The manifest was written after the intro and the menu music were measured against the
    // original (docs/verification/m1b.md); a player change must not alter any sound unnoticed.
    let actual = manifest(&located());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/rendered-audio.sha256");
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
        "rendered sound differs from {}:\nrendered now:\n{}\nin the manifest:\n{}\nIf the \
         change is intended, measure it against the original again and rewrite the manifest \
         with DEADRALLY_BLESS=1 cargo test-data",
        path.display(),
        new.join("\n"),
        gone.join("\n")
    );
}
