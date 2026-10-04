//! The headless binary as CI and developers use it (spec section 8).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::{TempDir, tempdir};

const REQUIRED_FILES: [&str; 19] = deadrally_gamedata::REQUIRED_FILES;

/// The binary with an empty, private config directory and no DEADRALLY_DATA, so the
/// developer's own settings cannot leak into a test.
fn headless(home: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_deadrally-headless"));
    command
        .env_remove("DEADRALLY_DATA")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"));
    command
}

fn fake_install(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    for name in REQUIRED_FILES {
        fs::write(dir.join(name), "placeholder").unwrap();
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn run(home: &TempDir, args: &[&str]) -> Output {
    headless(home).args(args).output().unwrap()
}

#[test]
fn run_prints_the_same_hashes_every_time() {
    // CI compares this line across three operating systems; it must be stable within one.
    let home = tempdir().unwrap();
    let first = run(&home, &["run", "--ticks", "140"]);
    let second = run(&home, &["run", "--ticks", "140"]);
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);

    let line = text(&first.stdout);
    let fields: Vec<&str> = line.trim().split(' ').collect();
    assert_eq!(fields[0], "ticks=140");
    for (field, prefix) in fields[1..].iter().zip(["frames_sha256=", "audio_sha256="]) {
        let hash = field
            .strip_prefix(prefix)
            .unwrap_or_else(|| panic!("{line}"));
        assert_eq!(hash.len(), 64, "{line}");
    }
}

#[test]
fn run_hashes_depend_on_the_number_of_ticks() {
    // A hash that ignored the frames would make the determinism job pass vacuously.
    let home = tempdir().unwrap();
    let short = text(&run(&home, &["run", "--ticks", "1"]).stdout);
    let long = text(&run(&home, &["run", "--ticks", "2"]).stdout);
    let hashes = |line: &str| line.split_once(' ').unwrap().1.to_owned();
    assert_ne!(hashes(&short), hashes(&long));
}

#[test]
fn bad_arguments_print_usage_and_fail() {
    let home = tempdir().unwrap();
    for args in [&[][..], &["run", "--ticks", "many"], &["fly"]] {
        let output = run(&home, args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(text(&output.stderr).contains("usage:"), "{args:?}");
    }
}

#[test]
fn check_data_reports_an_unknown_version_with_exit_status_2() {
    // Usable but unrecognised data must be distinguishable from both success and failure.
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let output = run(&home, &["check-data", "--data", data.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("source: command line (--data)"), "{stdout}");
    assert!(stdout.contains("UNKNOWN VERSION"), "{stdout}");
    assert!(text(&output.stderr).contains("warning: unknown version"));
}

#[test]
fn check_data_names_a_missing_file_and_fails() {
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    fs::remove_file(data.join("SANIM.HAF")).unwrap();
    let output = run(&home, &["check-data", "--data", data.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("SANIM.HAF"));
}

#[test]
fn check_data_uses_the_environment_when_no_option_is_given() {
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let output = headless(&home)
        .arg("check-data")
        .env("DEADRALLY_DATA", &data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stdout).contains("source: environment (DEADRALLY_DATA)"));
}

#[test]
fn check_data_does_not_fall_back_from_a_wrong_option_to_the_environment() {
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let wrong = home.path().join("wrong");
    fs::create_dir(&wrong).unwrap();
    let output = headless(&home)
        .args(["check-data", "--data", wrong.to_str().unwrap()])
        .env("DEADRALLY_DATA", &data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("command line"));
}

#[test]
fn check_data_without_any_source_explains_all_three() {
    let home = tempdir().unwrap();
    let output = run(&home, &["check-data"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    for needle in ["--data", "DEADRALLY_DATA", "data_path"] {
        assert!(stderr.contains(needle), "{stderr}");
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn check_data_recognises_the_developers_install() {
    let data = std::env::var_os("DEADRALLY_DATA")
        .filter(|value| !value.is_empty())
        .expect(
            "DEADRALLY_DATA is not set: point it at your Death Rally data to run `cargo test-data`",
        );
    let home = tempdir().unwrap();
    let output = headless(&home)
        .arg("check-data")
        .env("DEADRALLY_DATA", data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("outcome: known version"));
}

fn write_png(path: &Path, width: u32, height: u32, rgb: &[u8]) {
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgb).unwrap();
    writer.finish().unwrap();
}

#[test]
fn compare_succeeds_only_for_identical_pictures() {
    // The verification scripts rely on the exit status; "close enough" must fail.
    let home = tempdir().unwrap();
    let path = |name: &str| home.path().join(name);
    write_png(&path("a.png"), 2, 1, &[0, 0, 0, 10, 10, 10]);
    write_png(&path("b.png"), 2, 1, &[0, 0, 0, 10, 11, 10]);
    write_png(&path("c.png"), 1, 2, &[0, 0, 0, 10, 10, 10]);
    let compare = |a: &str, b: &str| {
        run(
            &home,
            &[
                "compare",
                path(a).to_str().unwrap(),
                path(b).to_str().unwrap(),
            ],
        )
    };

    let same = compare("a.png", "a.png");
    assert_eq!(same.status.code(), Some(0));
    assert!(text(&same.stdout).contains("0 pixels differ"));

    let close = compare("a.png", "b.png");
    assert_eq!(close.status.code(), Some(1));
    assert!(
        text(&close.stdout).contains("1 pixels differ, largest channel difference 1"),
        "{}",
        text(&close.stdout)
    );

    let other_size = compare("a.png", "c.png");
    assert_eq!(other_size.status.code(), Some(1));
    assert!(text(&other_size.stdout).contains("sizes differ"));
}

/// A 48 kHz stereo WAV of a 440 Hz tone whose loudness changes every 200 ms without
/// repeating, so `compare-audio` can line two of them up in one place only.
fn write_tone(path: &Path, seconds: u32, gain: f64) {
    const RATE: u32 = 48_000;
    let mut data = Vec::new();
    for i in 0..seconds * RATE {
        let t = f64::from(i) / f64::from(RATE);
        let segment = (t * 5.0) as u64;
        let mixed = segment
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407)
            >> 33;
        let level = 0.1 + 0.8 * (mixed % 1000) as f64 / 1000.0;
        let value =
            (gain * level * (2.0 * std::f64::consts::PI * 440.0 * t).sin() * 32767.0) as i16;
        data.extend_from_slice(&value.to_le_bytes());
        data.extend_from_slice(&value.to_le_bytes());
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    for field in [16u32, 1 | 2 << 16, RATE, RATE * 4, 4 | 16 << 16] {
        bytes.extend_from_slice(&field.to_le_bytes());
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&data);
    fs::write(path, bytes).unwrap();
}

#[test]
fn compare_audio_passes_only_within_the_tolerances() {
    // The sound checks rely on the exit status: a quieter render, too short an overlap or a
    // measure that could not be taken must fail, not pass with a remark.
    let home = tempdir().unwrap();
    let path = |name: &str| home.path().join(name);
    write_tone(&path("original.wav"), 40, 1.0);
    write_tone(&path("quieter.wav"), 40, 0.5);
    write_tone(&path("brief.wav"), 15, 1.0);
    let compare = |ours: &str, options: &[&str]| {
        let (original, ours) = (path("original.wav"), path(ours));
        let mut args = vec![
            "compare-audio",
            original.to_str().unwrap(),
            ours.to_str().unwrap(),
        ];
        args.extend_from_slice(options);
        run(&home, &args)
    };

    let same = compare("original.wav", &[]);
    assert_eq!(same.status.code(), Some(0), "{}", text(&same.stdout));
    assert!(text(&same.stdout).contains("result: PASS"));

    let quieter = compare("quieter.wav", &[]);
    assert_eq!(quieter.status.code(), Some(1), "{}", text(&quieter.stdout));
    assert!(text(&quieter.stdout).contains("median loudness difference above"));

    let short = compare("original.wav", &["--min-overlap", "50"]);
    assert_eq!(short.status.code(), Some(1), "{}", text(&short.stdout));
    assert!(text(&short.stdout).contains("overlap below 50 s"));

    // 15 s hold one 10 s piece: too few to measure a tempo on.
    let brief = compare("brief.wav", &["--min-overlap", "10"]);
    assert_eq!(brief.status.code(), Some(1), "{}", text(&brief.stdout));
    assert!(text(&brief.stdout).contains("tempo measured on fewer than 3 pieces"));
}

#[test]
fn find_rejects_a_screenshot_that_is_not_window_sized() {
    // A shot of the whole virtual screen or with window decorations can never match; say so
    // instead of searching for minutes.
    let home = tempdir().unwrap();
    let shot = home.path().join("shot.png");
    write_png(&shot, 2, 1, &[0; 6]);
    let output = run(&home, &["find", shot.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        text(&output.stderr).contains("the original's window is 640x480"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn dump_assets_refuses_to_write_into_the_game_data() {
    // The install must stay exactly as the player's copy is; dumps go elsewhere.
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let out = data.join("dumps");
    let output = run(
        &home,
        &[
            "dump-assets",
            "--data",
            data.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("inside the game data directory"));
    assert!(!out.exists());
}

#[test]
fn dump_assets_refuses_the_steam_folder_above_the_data() {
    // With Steam's layout the data sits one level down; the folder the player named is still
    // the game's install and must not fill up with dumps.
    let home = tempdir().unwrap();
    let named = home.path().join("Death Rally");
    fake_install(&named.join("Death Rally"));
    let out = named.join("dumps");
    let output = run(
        &home,
        &[
            "dump-assets",
            "--data",
            named.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    assert!(
        text(&output.stderr).contains("game's install"),
        "{}",
        text(&output.stderr)
    );
    assert!(!out.exists());
}

fn data_env() -> std::ffi::OsString {
    std::env::var_os("DEADRALLY_DATA")
        .filter(|value| !value.is_empty())
        .expect(
            "DEADRALLY_DATA is not set: point it at your Death Rally data to run `cargo test-data`",
        )
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn dump_assets_writes_one_png_per_catalogued_image() {
    let home = tempdir().unwrap();
    let out = home.path().join("dumps");
    let output = headless(&home)
        .args(["dump-assets", "--out", out.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let count = deadrally_gamedata::catalog::IMAGES.len();
    assert!(text(&output.stdout).contains(&format!("wrote {count} images")));
    let written = walk_pngs(&out);
    assert_eq!(written, count);
    assert!(out.join("MENU/APOGEE.png").is_file());
}

fn walk_pngs(dir: &Path) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|path| {
            if path.is_dir() {
                walk_pngs(&path)
            } else {
                usize::from(path.extension().is_some_and(|ext| ext == "png"))
            }
        })
        .sum()
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn find_locates_a_rendered_frame_in_the_startup_sequence() {
    // find is how screenshots of the original are matched; it must at least find our own
    // frames, at the tick they were rendered.
    let home = tempdir().unwrap();
    let shot = home.path().join("tick-1000.png");
    let render = headless(&home)
        .args(["render", "--tick", "1000", "--out", shot.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(render.status.code(), Some(0), "{}", text(&render.stderr));
    let found = headless(&home)
        .args(["find", "--ticks", "1100", shot.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(found.status.code(), Some(0), "{}", text(&found.stdout));
    let line = text(&found.stdout);
    let ticks = line.split("ticks ").nth(1).unwrap().trim();
    let (first, last) = ticks.split_once('-').unwrap_or((ticks, ticks));
    let (first, last): (u64, u64) = (first.parse().unwrap(), last.parse().unwrap());
    assert!((first..=last).contains(&1000), "{line}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn render_audio_writes_the_whole_intro_and_the_music_after_it_as_a_48_khz_wav() {
    // compare-audio and the owner's listening use this file: another rate would shift every
    // pitch, and a short file would hide how the intro ends and the menu music starts.
    let home = tempdir().unwrap();
    let out = home.path().join("startup.wav");
    let output = headless(&home)
        .args(["render-audio", "--startup", "--out", out.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let bytes = fs::read(&out).unwrap();
    let field = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2, "channels");
    assert_eq!(field(24), 48_000, "rate");
    // The intro's 5732 ticks and 2 s (143 ticks) after it, 672 frames of 4 bytes each.
    assert_eq!(field(40), (5732 + 143) * 672 * 4, "data bytes");
}
