//! The sound decoders against the developer's real game data. Run with `cargo test-data`; the
//! tests read DEADRALLY_DATA and fail (never pass silently) when it is unset.

use std::collections::BTreeSet;
use std::path::PathBuf;

use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::sound::{self, EFFECTS, MUSIC};
use deadrally_gamedata::{DATA_ENV_VAR, locate};
use sha2::{Digest, Sha256};

fn musics() -> Archive {
    let dir = match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    };
    let dir = locate(Some(&dir), None, None)
        .unwrap_or_else(|error| panic!("{error}"))
        .validation
        .dir;
    Archive::open(&dir.join(sound::ARCHIVE)).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_sound_file_is_music_or_a_bank_of_effects() {
    // An unlisted file could never be played; a misfiled one would be parsed as the wrong kind.
    let archive = musics();
    let listed: BTreeSet<&str> = MUSIC.iter().chain(&EFFECTS).copied().collect();
    let present: BTreeSet<&str> = archive.names().collect();
    assert_eq!(present, listed);
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_music_parses_and_uses_only_the_supported_commands() {
    // The player implements exactly these; any other command would play wrong without notice.
    let archive = musics();
    let supported: BTreeSet<char> = "ABCDEFGHKOQST".chars().collect();
    for name in MUSIC {
        let module = sound::load_music(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        assert!(!module.orders.is_empty(), "{name}");
        for pattern in &module.patterns {
            for cell in pattern.rows.iter().flatten() {
                if cell.command == 0 {
                    continue;
                }
                let letter = char::from(b'@' + cell.command);
                assert!(supported.contains(&letter), "{name} uses {letter}");
                if letter == 'S' {
                    assert_eq!(cell.info >> 4, 0xD, "{name} uses S{:X}", cell.info >> 4);
                }
            }
        }
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_banks_hold_their_documented_instruments() {
    // Effect numbers in the game's tables index these lists; a shifted list plays wrong effects.
    let archive = musics();
    for (name, count, empty) in [
        ("SANIM-E.CMF", 40, 13),
        ("ENDANI-E.CMF", 40, 33),
        ("ENDANI0E.CMF", 5, 0),
        ("GEN-EFE.CMF", 44, 9),
        ("MEN-SAM.CMF", 31, 16),
    ] {
        let bank = sound::load_effects(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(bank.instruments.len(), count, "{name}");
        let empties = bank.instruments.iter().filter(|i| i.is_none()).count();
        assert_eq!(empties, empty, "{name}");
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_race_record_call_is_made_of_the_known_release_s_calls() {
    // "It's a new race record" in the announcer's voice: the cuts are sample numbers of the
    // known release's lap record and race over calls, so other samples would cut words in
    // half. The call's samples were heard by the owner and by a speech recogniser before
    // their hash was written down; DEADRALLY_DUMP_CALL names a file for its raw samples.
    let bank = sound::load_effects(&musics(), "GEN-EFE.CMF").unwrap_or_else(|e| panic!("{e}"));
    let call = sound::race_record_call(&bank).expect("the known release's calls");
    if let Some(path) = std::env::var_os("DEADRALLY_DUMP_CALL") {
        let bytes: Vec<u8> = call.data.iter().flat_map(|v| v.to_le_bytes()).collect();
        std::fs::write(path, bytes).unwrap();
    }
    assert_eq!(call.relative_note, -19);
    let mut hasher = Sha256::new();
    for sample in &call.data {
        hasher.update(sample.to_le_bytes());
    }
    let hash: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hash,
        "44cb38c7a38b0b26401fb69f28b9541a244aafe6a741dcc5f2429c662e78fc53"
    );
}
