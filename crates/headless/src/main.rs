//! Runs the core without a window (spec section 8, spec M1a section 7).
//!
//! `run --ticks N` hashes every frame and every audio sample, so two runs, or the same run on
//! two operating systems, can be compared with one line. `check-data` reports where the game
//! data was found and whether it is a known release. `dump-assets` writes every catalogued
//! image as a PNG. `render`, `compare` and `find` check the startup sequence against
//! screenshots of the original (scripts/reference-run.sh).

mod dump;
mod rgb;
mod window;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deadrally_core::{Game, InputEvent, Key};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::{DATA_ENV_VAR, Located, Outcome, config_path, locate};
use sha2::{Digest, Sha256};

use crate::rgb::{Difference, Rgb};

const USAGE: &str = "usage:
  deadrally-headless run --ticks N
  deadrally-headless check-data [--data PATH]
  deadrally-headless dump-assets [--data PATH] [--out DIR]
  deadrally-headless render [--data PATH] --tick T [--key-at T]... --out FILE.png
  deadrally-headless compare A.png B.png
  deadrally-headless find [--data PATH] [--key-at T]... [--ticks N] SHOT.png...";

/// Exit status of `check-data` when the data is usable but not a known release.
const EXIT_UNKNOWN_VERSION: u8 = 2;

/// `find` runs this many ticks by default: the whole startup sequence of the known version
/// (6219 ticks) and then some.
const FIND_TICKS: u64 = 7_000;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run {
        ticks: u64,
    },
    CheckData {
        data: Option<PathBuf>,
    },
    DumpAssets {
        data: Option<PathBuf>,
        out: PathBuf,
    },
    Render {
        data: Option<PathBuf>,
        tick: u64,
        keys: Vec<u64>,
        out: PathBuf,
    },
    Compare {
        a: PathBuf,
        b: PathBuf,
    },
    Find {
        data: Option<PathBuf>,
        keys: Vec<u64>,
        ticks: u64,
        shots: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("error: {message}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let result = match command {
        Command::Run { ticks } => {
            println!("{}", run(ticks));
            Ok(ExitCode::SUCCESS)
        }
        Command::CheckData { data } => Ok(check_data(data.as_deref())),
        Command::DumpAssets { data, out } => locate_data(data.as_deref()).and_then(|located| {
            let count = dump::dump_assets(&located.validation, &out)?;
            println!("wrote {count} images to {}", out.display());
            Ok(ExitCode::SUCCESS)
        }),
        Command::Render {
            data,
            tick,
            keys,
            out,
        } => render(data.as_deref(), tick, &keys, &out).map(|()| ExitCode::SUCCESS),
        Command::Compare { a, b } => compare(&a, &b),
        Command::Find {
            data,
            keys,
            ticks,
            shots,
        } => find(data.as_deref(), &keys, ticks, &shots),
    };
    result.unwrap_or_else(|message| {
        eprintln!("error: {message}");
        ExitCode::FAILURE
    })
}

fn parse(args: &[OsString]) -> Result<Command, String> {
    let mut args = args.iter();
    let command = args.next().ok_or("missing command")?.to_string_lossy();
    let command = &*command;
    let options: &[&str] = match command {
        "run" => &["--ticks"],
        "check-data" => &["--data"],
        "dump-assets" => &["--data", "--out"],
        "render" => &["--data", "--tick", "--key-at", "--out"],
        "compare" => &[],
        "find" => &["--data", "--key-at", "--ticks"],
        _ => return Err(format!("unknown command: {command}")),
    };
    let takes_files = matches!(command, "compare" | "find");
    let (mut data, mut out, mut ticks, mut tick) = (None, None, None, None);
    let mut keys = Vec::new();
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        let name = arg.to_str().unwrap_or_default();
        if options.contains(&name) {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            let number = || {
                value
                    .to_str()
                    .and_then(|text| text.parse::<u64>().ok())
                    .ok_or(format!("{name}: not a number: {}", value.to_string_lossy()))
            };
            match name {
                "--data" => data = Some(PathBuf::from(value)),
                "--out" => out = Some(PathBuf::from(value)),
                "--ticks" => ticks = Some(number()?),
                "--tick" => tick = Some(number()?),
                "--key-at" => keys.push(number()?),
                _ => unreachable!("every option is handled"),
            }
        } else if takes_files && !name.starts_with("--") {
            files.push(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument: {}", arg.to_string_lossy()));
        }
    }
    match command {
        "run" => Ok(Command::Run {
            ticks: ticks.ok_or("run needs --ticks N")?,
        }),
        "check-data" => Ok(Command::CheckData { data }),
        "dump-assets" => Ok(Command::DumpAssets {
            data,
            out: out.unwrap_or_else(|| PathBuf::from("dumps")),
        }),
        "render" => Ok(Command::Render {
            data,
            tick: tick.ok_or("render needs --tick T")?,
            keys,
            out: out.ok_or("render needs --out FILE.png")?,
        }),
        "compare" => match <[PathBuf; 2]>::try_from(files) {
            Ok([a, b]) => Ok(Command::Compare { a, b }),
            Err(_) => Err("compare needs exactly two PNG files".into()),
        },
        _ => {
            if files.is_empty() {
                return Err("find needs at least one screenshot".into());
            }
            Ok(Command::Find {
                data,
                keys,
                ticks: ticks.unwrap_or(FIND_TICKS),
                shots: files,
            })
        }
    }
}

/// Runs `ticks` ticks without input and hashes, per tick in order: width, height and both
/// aspect terms as little-endian u32, the 768 palette bytes, the pixels; and every audio sample
/// as little-endian i16.
fn run(ticks: u64) -> String {
    let mut game = Game::test_scene();
    let mut frames = Sha256::new();
    let mut audio = Sha256::new();
    let mut samples = Vec::new();
    let mut sample_bytes = Vec::new();
    for _ in 0..ticks {
        game.tick();
        let frame = game.frame();
        for value in [frame.width, frame.height, frame.aspect.0, frame.aspect.1] {
            frames.update(value.to_le_bytes());
        }
        frames.update(frame.palette.as_flattened());
        frames.update(frame.pixels);

        samples.clear();
        game.take_audio(&mut samples);
        sample_bytes.clear();
        sample_bytes.extend(samples.iter().flat_map(|sample| sample.to_le_bytes()));
        audio.update(&sample_bytes);
    }
    format!(
        "ticks={ticks} frames_sha256={} audio_sha256={}",
        hex(&frames.finalize()),
        hex(&audio.finalize())
    )
}

fn check_data(cli: Option<&Path>) -> ExitCode {
    let config = config_path();
    match &config {
        Some(path) => println!("config file: {}", path.display()),
        None => println!("config file: unavailable (this system has no config directory)"),
    }
    let env = std::env::var_os(DATA_ENV_VAR);
    let located = match locate(cli, env.as_deref(), config.as_deref()) {
        Ok(located) => located,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    println!("source: {}", located.source);
    println!("directory: {}", located.validation.dir.display());
    for file in &located.validation.files {
        println!("  {:<12} {:>9}  {}", file.name, file.size, file.sha256);
    }
    match &located.validation.outcome {
        Outcome::Known { version } => {
            println!("outcome: known version: {version}");
            ExitCode::SUCCESS
        }
        Outcome::Unknown { closest, differing } => {
            println!("outcome: UNKNOWN VERSION (closest: {closest})");
            eprintln!(
                "warning: unknown version, the game may behave differently; files that differ from {closest}: {}",
                differing.join(", ")
            );
            ExitCode::from(EXIT_UNKNOWN_VERSION)
        }
    }
}

/// Finds and validates the data like `check-data`, warning instead of failing on an unknown
/// version.
fn locate_data(cli: Option<&Path>) -> Result<Located, String> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let located =
        locate(cli, env.as_deref(), config.as_deref()).map_err(|error| error.to_string())?;
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    if let Outcome::Unknown { closest, .. } = &located.validation.outcome {
        eprintln!(
            "warning: unknown data version (closest: {closest}); it may not match the original"
        );
    }
    Ok(located)
}

/// Runs `ticks` ticks, pressing and releasing a key after each tick count in `keys` (0: before
/// the first tick), and calls `each` with the tick count and the game after every tick.
fn play(game: &mut Game, ticks: u64, keys: &[u64], mut each: impl FnMut(u64, &Game)) {
    for done in 0..ticks {
        if keys.contains(&done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key {
                    key: Key::Space,
                    pressed,
                });
            }
        }
        game.tick();
        each(done + 1, game);
    }
}

/// Writes the frame after `tick` ticks as the original's window would show it.
fn render(data: Option<&Path>, tick: u64, keys: &[u64], out: &Path) -> Result<(), String> {
    let located = locate_data(data)?;
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let mut game = Game::new(assets);
    play(&mut game, tick, keys, |_, _| {});
    window::present(&game.frame())?.write_png(out)
}

/// Exit status 0 only when the pictures are identical.
fn compare(a: &Path, b: &Path) -> Result<ExitCode, String> {
    let (first, second) = (Rgb::read_png(a)?, Rgb::read_png(b)?);
    let Some(difference) = first.difference(&second) else {
        println!(
            "sizes differ: {}x{} and {}x{}",
            first.width, first.height, second.width, second.height
        );
        return Ok(ExitCode::FAILURE);
    };
    println!(
        "{}x{}: {} pixels differ, largest channel difference {}",
        first.width, first.height, difference.pixels, difference.max_channel
    );
    Ok(if difference.pixels == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// For each screenshot of the original, the ticks of our startup sequence that show exactly
/// the same picture. Exit status 0 only when every screenshot has a match.
fn find(
    data: Option<&Path>,
    keys: &[u64],
    ticks: u64,
    shots: &[PathBuf],
) -> Result<ExitCode, String> {
    let pictures = shots
        .iter()
        .map(|path| Rgb::read_png(path))
        .collect::<Result<Vec<_>, _>>()?;
    for (path, picture) in shots.iter().zip(&pictures) {
        if (picture.width, picture.height) != (window::WIDTH, window::HEIGHT) {
            return Err(format!(
                "{}: {}x{}, the original's window is {}x{}",
                path.display(),
                picture.width,
                picture.height,
                window::WIDTH,
                window::HEIGHT
            ));
        }
    }
    let located = locate_data(data)?;
    // First only exact matches, which fail fast on the first differing byte.
    let mut matches = vec![Vec::new(); shots.len()];
    let mut equal = vec![false; shots.len()];
    timeline(&located, keys, ticks, |tick, window, changed| {
        for (index, picture) in pictures.iter().enumerate() {
            if changed {
                equal[index] = window.pixels == picture.pixels;
            }
            if equal[index] {
                matches[index].push(tick);
            }
        }
    })?;
    // Then, for screenshots without a match, the nearest picture, to help find out why.
    let unmatched: Vec<usize> = (0..shots.len())
        .filter(|&index| matches[index].is_empty())
        .collect();
    let mut closest: Vec<Option<(Difference, u64)>> = vec![None; shots.len()];
    if !unmatched.is_empty() {
        timeline(&located, keys, ticks, |tick, window, changed| {
            if !changed {
                return;
            }
            for &index in &unmatched {
                let difference = window.difference(&pictures[index]).expect("window-sized");
                if closest[index].is_none_or(|(best, _)| difference < best) {
                    closest[index] = Some((difference, tick));
                }
            }
        })?;
    }
    for (index, path) in shots.iter().enumerate() {
        match closest[index] {
            None => println!("{}: ticks {}", path.display(), ranges(&matches[index])),
            Some((difference, tick)) => println!(
                "{}: no exact match; closest is tick {tick}: {} pixels differ, largest channel difference {}",
                path.display(),
                difference.pixels,
                difference.max_channel
            ),
        }
    }
    Ok(if unmatched.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Plays the startup sequence for `ticks` ticks and calls `each` with every tick count from 0,
/// the window picture and whether it changed since the previous call. Most ticks repeat the
/// previous picture, so callers can skip work on those.
fn timeline(
    located: &Located,
    keys: &[u64],
    ticks: u64,
    mut each: impl FnMut(u64, &Rgb, bool),
) -> Result<(), String> {
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let mut game = Game::new(assets);
    let mut previous: Option<(Vec<u8>, Vec<[u8; 3]>, Rgb)> = None;
    let mut failure = None;
    let mut visit = |tick: u64, game: &Game| {
        let frame = game.frame();
        let changed = previous
            .as_ref()
            .is_none_or(|(pixels, palette, _)| pixels != frame.pixels || palette != frame.palette);
        if changed {
            match window::present(&frame) {
                Ok(window) => {
                    previous = Some((frame.pixels.to_vec(), frame.palette.to_vec(), window));
                }
                Err(error) => {
                    failure.get_or_insert(format!("tick {tick}: {error}"));
                    return;
                }
            }
        }
        if let Some((_, _, window)) = &previous {
            each(tick, window, changed);
        }
    };
    visit(0, &game);
    play(&mut game, ticks, keys, &mut visit);
    failure.map_or(Ok(()), Err)
}

/// `[3, 4, 5, 9]` as `3-5, 9`.
fn ranges(ticks: &[u64]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0;
    for index in 1..=ticks.len() {
        if index == ticks.len() || ticks[index] != ticks[index - 1] + 1 {
            parts.push(if index - 1 == start {
                ticks[start].to_string()
            } else {
                format!("{}-{}", ticks[start], ticks[index - 1])
            });
            start = index;
        }
    }
    parts.join(", ")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_both_commands() {
        assert_eq!(
            parse(&args(&["run", "--ticks", "7000"])),
            Ok(Command::Run { ticks: 7000 })
        );
        assert_eq!(
            parse(&args(&["check-data"])),
            Ok(Command::CheckData { data: None })
        );
        assert_eq!(
            parse(&args(&["check-data", "--data", "/x"])),
            Ok(Command::CheckData {
                data: Some(PathBuf::from("/x"))
            })
        );
    }

    #[test]
    fn rejects_options_of_the_other_command() {
        // A misplaced option must not be silently ignored.
        assert!(parse(&args(&["run", "--ticks", "1", "--data", "/x"])).is_err());
        assert!(parse(&args(&["check-data", "--ticks", "1"])).is_err());
    }

    #[test]
    fn rejects_bad_tick_counts() {
        for bad in [
            &["run"][..],
            &["run", "--ticks"],
            &["run", "--ticks", "-1"],
            &["run", "--ticks", "ten"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn parses_the_asset_commands() {
        assert_eq!(
            parse(&args(&["dump-assets"])),
            Ok(Command::DumpAssets {
                data: None,
                out: PathBuf::from("dumps")
            })
        );
        assert_eq!(
            parse(&args(&[
                "render", "--tick", "300", "--key-at", "10", "--key-at", "20", "--out", "a.png"
            ])),
            Ok(Command::Render {
                data: None,
                tick: 300,
                keys: vec![10, 20],
                out: PathBuf::from("a.png")
            })
        );
        assert_eq!(
            parse(&args(&["compare", "a.png", "b.png"])),
            Ok(Command::Compare {
                a: PathBuf::from("a.png"),
                b: PathBuf::from("b.png")
            })
        );
        assert_eq!(
            parse(&args(&["find", "--data", "/x", "a.png", "b.png"])),
            Ok(Command::Find {
                data: Some(PathBuf::from("/x")),
                keys: vec![],
                ticks: FIND_TICKS,
                shots: vec![PathBuf::from("a.png"), PathBuf::from("b.png")]
            })
        );
    }

    #[test]
    fn rejects_incomplete_asset_commands() {
        for bad in [
            &["render", "--out", "a.png"][..],
            &["render", "--tick", "3"],
            &["render", "--tick", "3", "--out", "a.png", "extra.png"],
            &["compare", "a.png"],
            &["compare", "a.png", "b.png", "c.png"],
            &["find"],
            &["find", "--ticks", "many", "a.png"],
            &["dump-assets", "--tick", "3"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn tick_lists_are_printed_as_ranges() {
        assert_eq!(ranges(&[3, 4, 5, 9]), "3-5, 9");
        assert_eq!(ranges(&[7]), "7");
        assert_eq!(ranges(&[1, 3]), "1, 3");
    }
}
