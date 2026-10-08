//! Runs the core without a window (spec section 8, spec M1a section 7).
//!
//! `run --ticks N` hashes every frame and every audio sample, so two runs, or the same run on
//! two operating systems, can be compared with one line. `check-data` reports where the game
//! data was found and whether it is a known release. `dump-assets` writes every catalogued
//! image as a PNG. `render`, `compare` and `find` check the startup sequence and the menus
//! against screenshots of the original (scripts/reference-run.sh). `render-audio` writes what
//! the game plays as a WAV, and `compare-audio` checks it against a recording of the original
//! (spec M1b sections 4.4 and 5).

mod audio_compare;
mod dump;
mod rgb;
mod wav;
mod window;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deadrally_core::{
    AUDIO_SAMPLE_RATE, Game, InputEvent, Key, TICK_NANOS, render_effect, render_music,
};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::dr_cfg::DrCfg;
use deadrally_gamedata::{DATA_ENV_VAR, Located, Outcome, config_path, locate};
use deadrally_gamedata::{save_game, sound};
use sha2::{Digest, Sha256};

use crate::rgb::{Difference, Rgb};
use crate::wav::Wav;

const USAGE: &str = "usage:
  deadrally-headless run --ticks N
  deadrally-headless check-data [--data PATH]
  deadrally-headless dump-assets [--data PATH] [--out DIR]
  deadrally-headless render [--data PATH] --tick T [--key-at T[:KEY[+N]]]... [--smooth] --out FILE.png
  deadrally-headless trace [--data PATH] --tick T [--key-at T[:KEY[+N]]]... [--menus]
  deadrally-headless compare A.png B.png
  deadrally-headless find [--data PATH] [--key-at T[:KEY[+N]]]... [--sabotage-clock MS] [--no-ai] [--smooth] [--ticks N] SHOT.png...
  deadrally-headless render-audio [--data PATH] --startup [--key-at T[:KEY[+N]]]... [--save SLOT:FILE]... [--no-ai] [--cfg DR.CFG] [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --music NAME [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --effect BANK --number K --out FILE.wav
  deadrally-headless compare-audio ORIGINAL.wav OURS.wav [--min-overlap S]";

/// Exit status of `check-data` when the data is usable but not a known release.
const EXIT_UNKNOWN_VERSION: u8 = 2;

/// `find` runs this many ticks by default: the whole startup sequence of the known version
/// (6219 ticks) and then some.
const FIND_TICKS: u64 = 7_000;

/// `render-audio --startup` renders the whole intro and then this many ticks (2 s) by default:
/// the menu music starting.
const STARTUP_AFTER_INTRO_TICKS: u64 = 143;

/// `render-audio --music` renders this many seconds by default.
const MUSIC_SECONDS: u64 = 30;
/// `render-audio --effect` renders at most this many seconds, then trims the silence.
const EFFECT_SECONDS: u64 = 10;

/// `compare-audio` tolerances (spec M1b §5).
const LOUDNESS_MEDIAN_DB: f64 = 1.5;
const LOUDNESS_MAX_DB: f64 = 4.0;
const BAND_DB: f64 = 3.0;
const TEMPO_PERCENT: f64 = 0.15;
const PITCH_CENTS: f64 = 10.0;
const BALANCE_DB: f64 = 1.0;
const MIN_OVERLAP_SECONDS: u64 = 25;

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
        keys: Vec<Press>,
        seed: u32,
        saves: Vec<(usize, PathBuf)>,
        clock: Option<u32>,
        still: bool,
        smooth: bool,
        out: PathBuf,
    },
    Compare {
        a: PathBuf,
        b: PathBuf,
    },
    Trace {
        data: Option<PathBuf>,
        tick: u64,
        keys: Vec<Press>,
        seed: u32,
        saves: Vec<(usize, PathBuf)>,
        clock: Option<u32>,
        still: bool,
        menus: bool,
    },
    Find {
        data: Option<PathBuf>,
        keys: Vec<Press>,
        seed: u32,
        saves: Vec<(usize, PathBuf)>,
        clock: Option<u32>,
        still: bool,
        smooth: bool,
        ticks: u64,
        shots: Vec<PathBuf>,
    },
    RenderAudio {
        data: Option<PathBuf>,
        source: AudioSource,
        keys: Vec<Press>,
        seed: u32,
        saves: Vec<(usize, PathBuf)>,
        still: bool,
        cfg: Option<PathBuf>,
        seconds: Option<u64>,
        out: PathBuf,
    },
    CompareAudio {
        original: PathBuf,
        ours: PathBuf,
        min_overlap: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum AudioSource {
    Startup,
    Music(String),
    Effect { bank: String, number: u8 },
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
            seed,
            saves,
            clock,
            still,
            smooth,
            out,
        } => render(
            data.as_deref(),
            tick,
            &keys,
            (seed, &saves, clock, still),
            smooth,
            &out,
        )
        .map(|()| ExitCode::SUCCESS),
        Command::Compare { a, b } => compare(&a, &b),
        Command::Trace {
            data,
            tick,
            keys,
            seed,
            saves,
            clock,
            still,
            menus,
        } => trace(
            data.as_deref(),
            tick,
            &keys,
            (seed, &saves, clock, still),
            menus,
        )
        .map(|()| ExitCode::SUCCESS),
        Command::Find {
            data,
            keys,
            seed,
            saves,
            clock,
            still,
            smooth,
            ticks,
            shots,
        } => find(
            data.as_deref(),
            &keys,
            (seed, &saves, clock, still),
            smooth,
            ticks,
            &shots,
        ),
        Command::RenderAudio {
            data,
            source,
            keys,
            seed,
            saves,
            still,
            cfg,
            seconds,
            out,
        } => render_audio(
            data.as_deref(),
            &source,
            &keys,
            (seed, &saves, None, still),
            cfg.as_deref(),
            seconds,
            &out,
        )
        .map(|()| ExitCode::SUCCESS),
        Command::CompareAudio {
            original,
            ours,
            min_overlap,
        } => compare_audio(&original, &ours, min_overlap),
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
        "render" => &[
            "--data",
            "--tick",
            "--key-at",
            "--seed",
            "--save",
            "--sabotage-clock",
            "--out",
        ],
        "compare" => &[],
        "trace" => &[
            "--data",
            "--tick",
            "--key-at",
            "--seed",
            "--save",
            "--sabotage-clock",
        ],
        "find" => &[
            "--data",
            "--key-at",
            "--seed",
            "--save",
            "--sabotage-clock",
            "--ticks",
        ],
        "render-audio" => &[
            "--data",
            "--seed",
            "--save",
            "--cfg",
            "--music",
            "--effect",
            "--number",
            "--seconds",
            "--key-at",
            "--out",
        ],
        "compare-audio" => &["--min-overlap"],
        _ => return Err(format!("unknown command: {command}")),
    };
    let takes_files = matches!(command, "compare" | "find" | "compare-audio");
    let (mut data, mut out, mut ticks, mut tick) = (None, None, None, None);
    let (mut startup, mut music, mut effect, mut effect_number, mut seconds, mut min_overlap) =
        (false, None, None, None, None, None);
    let mut keys = Vec::new();
    let mut seed = 0;
    let mut saves = Vec::new();
    let mut clock = None;
    let mut still = false;
    let mut smooth = false;
    let mut menus = false;
    let mut cfg = None;
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        let name = arg.to_str().unwrap_or_default();
        if command == "render-audio" && name == "--startup" {
            startup = true;
        } else if matches!(command, "render" | "trace" | "find" | "render-audio")
            && name == "--no-ai"
        {
            still = true;
        } else if matches!(command, "render" | "find") && name == "--smooth" {
            smooth = true;
        } else if command == "trace" && name == "--menus" {
            menus = true;
        } else if options.contains(&name) {
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
                "--cfg" => cfg = Some(PathBuf::from(value)),
                "--ticks" => ticks = Some(number()?),
                "--tick" => tick = Some(number()?),
                "--key-at" => keys.push(press(&value.to_string_lossy())?),
                "--save" => {
                    let text = value.to_string_lossy();
                    let (slot, path) = text
                        .split_once(':')
                        .ok_or("--save takes SLOT:FILE, a slot 0 to 7")?;
                    let slot = slot
                        .parse::<usize>()
                        .ok()
                        .filter(|&slot| slot < save_game::SLOTS)
                        .ok_or("--save: the slot is 0 to 7")?;
                    saves.push((slot, PathBuf::from(path)));
                }
                "--seed" => {
                    seed = u32::try_from(number()?).map_err(|_| "--seed: at most 4294967295")?;
                }
                "--sabotage-clock" => {
                    clock = Some(
                        u32::try_from(number()?)
                            .map_err(|_| "--sabotage-clock: at most 4294967295")?,
                    );
                }
                "--music" => music = Some(value.to_string_lossy().into_owned()),
                "--effect" => effect = Some(value.to_string_lossy().into_owned()),
                "--number" => {
                    effect_number = Some(
                        u8::try_from(number()?).map_err(|_| "--number: an effect is 1 to 255")?,
                    );
                }
                "--seconds" => seconds = Some(number()?),
                "--min-overlap" => min_overlap = Some(number()?),
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
            seed,
            saves,
            clock,
            still,
            smooth,
            out: out.ok_or("render needs --out FILE.png")?,
        }),
        "trace" => Ok(Command::Trace {
            data,
            tick: tick.ok_or("trace needs --tick T")?,
            keys,
            seed,
            saves,
            clock,
            still,
            menus,
        }),
        "compare" => match <[PathBuf; 2]>::try_from(files) {
            Ok([a, b]) => Ok(Command::Compare { a, b }),
            Err(_) => Err("compare needs exactly two PNG files".into()),
        },
        "compare-audio" => match <[PathBuf; 2]>::try_from(files) {
            Ok([original, ours]) => Ok(Command::CompareAudio {
                original,
                ours,
                min_overlap: min_overlap.unwrap_or(MIN_OVERLAP_SECONDS),
            }),
            Err(_) => Err("compare-audio needs exactly two WAV files".into()),
        },
        "render-audio" => {
            let source = match (startup, music, effect, effect_number) {
                (true, None, None, None) => AudioSource::Startup,
                (false, Some(name), None, None) => AudioSource::Music(name),
                (false, None, Some(bank), Some(number)) if number > 0 => {
                    AudioSource::Effect { bank, number }
                }
                _ => {
                    return Err("render-audio needs one of --startup, --music NAME, or --effect BANK --number K".into());
                }
            };
            if source != AudioSource::Startup && !keys.is_empty() {
                return Err("--key-at only applies to --startup".into());
            }
            if source != AudioSource::Startup && !saves.is_empty() {
                return Err("--save only applies to --startup".into());
            }
            if source != AudioSource::Startup && still {
                return Err("--no-ai only applies to --startup".into());
            }
            if source != AudioSource::Startup && cfg.is_some() {
                return Err("--cfg only applies to --startup".into());
            }
            Ok(Command::RenderAudio {
                data,
                source,
                keys,
                seed,
                saves,
                still,
                cfg,
                seconds,
                out: out.ok_or("render-audio needs --out FILE.wav")?,
            })
        }
        _ => {
            if files.is_empty() {
                return Err("find needs at least one screenshot".into());
            }
            Ok(Command::Find {
                data,
                keys,
                seed,
                saves,
                clock,
                still,
                smooth,
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

/// A key pressed after a number of ticks (0: before the first tick) and released then or
/// after the ticks it is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Press {
    tick: u64,
    key: Key,
    held: u64,
}

/// The keys `--key-at T:KEY` can name; `T` alone presses Space.
const KEY_NAMES: [(&str, Key); 22] = [
    ("space", Key::Space),
    ("ctrl", Key::LeftCtrl),
    ("alt", Key::LeftAlt),
    ("enter", Key::Enter),
    ("escape", Key::Escape),
    ("up", Key::Up),
    ("down", Key::Down),
    ("left", Key::Left),
    ("right", Key::Right),
    ("y", Key::Y),
    ("n", Key::N),
    ("q", Key::Q),
    ("a", Key::A),
    ("d", Key::D),
    ("e", Key::E),
    ("i", Key::I),
    ("l", Key::L),
    ("o", Key::O),
    ("p", Key::P),
    ("r", Key::R),
    ("v", Key::V),
    ("w", Key::W),
];

/// The saved games given with `--save`, slot by slot.
fn read_saves(saves: &[(usize, PathBuf)]) -> Result<Vec<Option<Vec<u8>>>, String> {
    let mut slots = vec![None; save_game::SLOTS];
    for (slot, path) in saves {
        slots[*slot] =
            Some(std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?);
    }
    Ok(slots)
}

/// Parses `T`, `T:KEY` or `T:KEY+N` (held for N ticks, as the quick save's F2 must be).
fn press(text: &str) -> Result<Press, String> {
    let (tick, name) = text.split_once(':').unwrap_or((text, "space"));
    let (name, held) = match name.split_once('+') {
        Some((name, held)) => (
            name,
            held.parse()
                .map_err(|_| format!("--key-at: not a number of ticks: {held}"))?,
        ),
        None => (name, 0),
    };
    let tick = tick
        .parse()
        .map_err(|_| format!("--key-at: not a number: {tick}"))?;
    // The short names, then any key by its name in `Key` (letters, digits, Backspace, ...).
    let key = KEY_NAMES
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|&(_, key)| key)
        .or_else(|| {
            Key::ALL
                .iter()
                .copied()
                .find(|key| format!("{key:?}").eq_ignore_ascii_case(name))
        })
        .ok_or_else(|| {
            let names: Vec<&str> = KEY_NAMES.iter().map(|(known, _)| *known).collect();
            format!(
                "--key-at: unknown key {name}; known keys: {}",
                names.join(", ")
            )
        })?;
    Ok(Press { tick, key, held })
}

/// Presses the keys due before tick `done + 1` and releases those whose time is up, in the
/// order given.
fn press_due(game: &mut Game, keys: &[Press], done: u64) {
    for press in keys {
        if press.tick == done {
            game.input(InputEvent::Key {
                key: press.key,
                pressed: true,
            });
        }
        if press.tick + press.held == done {
            game.input(InputEvent::Key {
                key: press.key,
                pressed: false,
            });
        }
    }
}

/// Runs `ticks` ticks, pressing and releasing the keys in `keys`, and calls `each` with the tick
/// count and the game after every tick.
fn play(game: &mut Game, ticks: u64, keys: &[Press], mut each: impl FnMut(u64, &Game)) {
    for done in 0..ticks {
        press_due(game, keys, done);
        game.tick();
        each(done + 1, game);
    }
}

/// Writes the frame after `tick` ticks as the original's window would show it.
/// How a run starts: `rand()`'s seed, the saved games given, the sabotage's clock if fixed,
/// and whether the opponents stay still (`--no-ai`).
type Start<'a> = (u32, &'a [(usize, PathBuf)], Option<u32>, bool);

/// A game started as `start` says.
/// `cfg` is a `dr.cfg` to start from instead of the defaults a fresh one has.
fn started(
    located: &Located,
    (seed, saves, clock, still): Start,
    cfg: Option<&Path>,
) -> Result<Game, String> {
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let config = match cfg {
        Some(path) => {
            let bytes =
                std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
            DrCfg::parse(&bytes).ok_or(format!("{}: not a dr.cfg", path.display()))?
        }
        None => assets.menu.default_config.clone(),
    };
    let mut game = Game::with_seed(assets, config, seed);
    // Our runs are checked against the original, which keeps the race's last key.
    game.keep_the_race_s_last_key();
    game.set_saved_games(read_saves(saves)?);
    if let Some(ms) = clock {
        game.fix_sabotage_clock(ms);
    }
    if still {
        game.keep_opponents_still();
    }
    Ok(game)
}

fn render(
    data: Option<&Path>,
    tick: u64,
    keys: &[Press],
    start: Start,
    smooth: bool,
    out: &Path,
) -> Result<(), String> {
    let located = locate_data(data)?;
    let mut game = started(&located, start, None)?;
    play(&mut game, tick, keys, |_, _| {});
    window::present(&game.frame(), smooth)?.write_png(out)
}

/// The race's state after every tick up to `tick` (`tick race-frame | car | ...`), to compare
/// with the original's memory as `scripts/reference-watch.py` logs it; with `menus`, instead
/// the menus' count of waits, copper row and pulse (`tick count row pulse`) each time the
/// count or the pulse moves, as its `.menus` log has them.
fn trace(
    data: Option<&Path>,
    tick: u64,
    keys: &[Press],
    start: Start,
    menus: bool,
) -> Result<(), String> {
    let located = locate_data(data)?;
    let mut game = started(&located, start, None)?;
    let mut last = None;
    play(&mut game, tick, keys, |done, game| {
        if menus {
            if let Some((count, row, pulse)) = game.menu_waits()
                && last != Some((count, pulse))
            {
                println!("{done} {count} {row} {pulse}");
                last = Some((count, pulse));
            }
        } else if let Some(state) = game.race_trace() {
            println!("{done} {state}");
        }
    });
    Ok(())
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
    keys: &[Press],
    start: Start,
    smooth: bool,
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
    timeline(
        &located,
        keys,
        start,
        smooth,
        ticks,
        |tick, window, changed| {
            for (index, picture) in pictures.iter().enumerate() {
                if changed {
                    equal[index] = window.pixels == picture.pixels;
                }
                if equal[index] {
                    matches[index].push(tick);
                }
            }
        },
    )?;
    // Then, for screenshots without a match, the nearest picture, to help find out why.
    let unmatched: Vec<usize> = (0..shots.len())
        .filter(|&index| matches[index].is_empty())
        .collect();
    let mut closest: Vec<Option<(Difference, u64)>> = vec![None; shots.len()];
    if !unmatched.is_empty() {
        timeline(
            &located,
            keys,
            start,
            smooth,
            ticks,
            |tick, window, changed| {
                if !changed {
                    return;
                }
                for &index in &unmatched {
                    let difference = window.difference(&pictures[index]).expect("window-sized");
                    if closest[index].is_none_or(|(best, _)| difference < best) {
                        closest[index] = Some((difference, tick));
                    }
                }
            },
        )?;
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
    keys: &[Press],
    start: Start,
    smooth: bool,
    ticks: u64,
    mut each: impl FnMut(u64, &Rgb, bool),
) -> Result<(), String> {
    let mut game = started(located, start, None)?;
    let mut previous: Option<(Vec<u8>, Vec<[u8; 3]>, Rgb)> = None;
    let mut failure = None;
    let mut visit = |tick: u64, game: &Game| {
        let frame = game.frame();
        let changed = previous
            .as_ref()
            .is_none_or(|(pixels, palette, _)| pixels != frame.pixels || palette != frame.palette);
        if changed {
            match window::present(&frame, smooth) {
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

/// A sound file's entry name from `TR0-MUS` or `tr0-mus.cmf`.
fn sound_entry(name: &str) -> String {
    let upper = name.to_ascii_uppercase();
    if upper.ends_with(".CMF") {
        upper
    } else {
        format!("{upper}.CMF")
    }
}

/// Writes the startup's sound (the intro, then the menu music), a piece of music or one effect
/// as a 48 kHz 16-bit WAV.
fn render_audio(
    data: Option<&Path>,
    source: &AudioSource,
    keys: &[Press],
    start: Start,
    cfg: Option<&Path>,
    seconds: Option<u64>,
    out: &Path,
) -> Result<(), String> {
    let located = locate_data(data)?;
    let frames = |seconds: u64| {
        usize::try_from(seconds * u64::from(AUDIO_SAMPLE_RATE)).unwrap_or(usize::MAX)
    };
    let musics = || {
        let file = located
            .validation
            .files
            .iter()
            .find(|file| file.name == sound::ARCHIVE)
            .ok_or("MUSICS.BPA is not among the validated files")?;
        Archive::open(&file.path).map_err(|error| error.to_string())
    };
    let samples = match source {
        AudioSource::Startup => {
            let ticks = match seconds {
                Some(seconds) => seconds * 1_000_000_000 / TICK_NANOS,
                None => {
                    let assets =
                        Assets::load(&located.validation).map_err(|error| error.to_string())?;
                    let intro_ticks = assets.intro.delays.iter().map(|&delay| u64::from(delay));
                    intro_ticks.sum::<u64>() + STARTUP_AFTER_INTRO_TICKS
                }
            };
            let mut game = started(&located, start, cfg)?;
            let mut audio = Vec::new();
            for done in 0..ticks {
                press_due(&mut game, keys, done);
                game.tick();
                game.take_audio(&mut audio);
            }
            audio
        }
        AudioSource::Music(name) => {
            let module = sound::load_music(&musics()?, &sound_entry(name))
                .map_err(|error| error.to_string())?;
            render_music(&module, frames(seconds.unwrap_or(MUSIC_SECONDS)))
        }
        AudioSource::Effect { bank, number } => {
            let bank = sound::load_effects(&musics()?, &sound_entry(bank))
                .map_err(|error| error.to_string())?;
            let mut samples =
                render_effect(&bank, *number, frames(seconds.unwrap_or(EFFECT_SECONDS)));
            let end = samples
                .iter()
                .rposition(|&sample| sample != 0)
                .map_or(0, |last| (last / 2 + 1) * 2);
            samples.truncate(end);
            samples
        }
    };
    Wav {
        rate: AUDIO_SAMPLE_RATE,
        channels: 2,
        samples,
    }
    .write(out)
}

/// Exit status 0 only when every measure is within the spec's tolerances.
fn compare_audio(original: &Path, ours: &Path, min_overlap: u64) -> Result<ExitCode, String> {
    let report = audio_compare::compare(&Wav::read(original)?, &Wav::read(ours)?);
    println!(
        "lag: {:.0} ms (envelope correlation {:.3})",
        report.lag_ms, report.correlation
    );
    println!("overlap: {:.1} s", report.overlap_seconds);
    println!(
        "loudness per second, ours - original: median {:.2} dB, largest {:.2} dB",
        report.loudness_median_db, report.loudness_max_db
    );
    let bands: Vec<String> = report
        .bands
        .iter()
        .map(|(hz, difference)| format!("{hz:.0} Hz {difference:+.1} dB"))
        .collect();
    println!("octave bands, ours - original: {}", bands.join(", "));
    println!(
        "tempo, ours - original: {:+.3} % (over {} pieces of 10 s)",
        report.tempo_percent, report.tempo_pieces
    );
    println!("pitch, ours - original: {:+.0} cents", report.pitch_cents);
    println!(
        "stereo balance (left - right), ours - original: largest {:+.2} dB (over {} pieces of 10 s)",
        report.balance_db, report.balance_pieces
    );
    let mut failures = Vec::new();
    if report.overlap_seconds < min_overlap as f64 {
        failures.push(format!("overlap below {min_overlap} s"));
    }
    if report.loudness_median_db > LOUDNESS_MEDIAN_DB {
        failures.push(format!(
            "median loudness difference above {LOUDNESS_MEDIAN_DB} dB"
        ));
    }
    if report.loudness_max_db > LOUDNESS_MAX_DB {
        failures.push(format!(
            "largest loudness difference above {LOUDNESS_MAX_DB} dB"
        ));
    }
    for (hz, difference) in &report.bands {
        if difference.abs() > BAND_DB {
            failures.push(format!("{hz:.0} Hz band off by more than {BAND_DB} dB"));
        }
    }
    // A measure that could not be taken fails: passing it would hide the difference it exists
    // to find.
    if report.tempo_pieces < audio_compare::MIN_TEMPO_PIECES {
        failures.push(format!(
            "tempo measured on fewer than {} pieces of 10 s",
            audio_compare::MIN_TEMPO_PIECES
        ));
    } else if report.tempo_percent.abs() > TEMPO_PERCENT {
        failures.push(format!("tempo off by more than {TEMPO_PERCENT} %"));
    }
    if report.pitch_cents.abs() > PITCH_CENTS {
        failures.push(format!("pitch off by more than {PITCH_CENTS} cents"));
    }
    if report.balance_pieces == 0 {
        failures.push("stereo balance not measured (a mono or silent file)".to_owned());
    } else if report.balance_db.abs() > BALANCE_DB {
        failures.push(format!("stereo balance off by more than {BALANCE_DB} dB"));
    }
    if failures.is_empty() {
        println!("result: PASS");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("result: FAIL ({})", failures.join("; "));
        Ok(ExitCode::FAILURE)
    }
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
    fn a_key_name_the_scenarios_do_not_use_is_an_error_that_lists_the_known_ones() {
        // A typo would otherwise press nothing, and a shot taken after it would match the
        // wrong frame.
        let error = parse(&args(&[
            "render", "--tick", "3", "--key-at", "2:enetr", "--out", "a.png",
        ]))
        .unwrap_err();
        assert!(error.contains("unknown key enetr"), "{error}");
        assert!(error.contains("escape"), "{error}");
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
                "render", "--tick", "300", "--key-at", "10", "--key-at", "20:Down", "--key-at",
                "30:F2+12", "--out", "a.png"
            ])),
            Ok(Command::Render {
                data: None,
                tick: 300,
                // F2 is held 12 ticks: the quick save looks at the keys held, not pressed.
                keys: vec![
                    Press {
                        tick: 10,
                        key: Key::Space,
                        held: 0
                    },
                    Press {
                        tick: 20,
                        key: Key::Down,
                        held: 0
                    },
                    Press {
                        tick: 30,
                        key: Key::F2,
                        held: 12
                    }
                ],
                seed: 0,
                saves: vec![],
                clock: None,
                still: false,
                smooth: false,
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
                seed: 0,
                saves: vec![],
                clock: None,
                still: false,
                smooth: false,
                ticks: FIND_TICKS,
                shots: vec![PathBuf::from("a.png"), PathBuf::from("b.png")]
            })
        );
        // The reference runner's --smooth starts the original with -smooth; find must
        // smooth our picture the same way to compare.
        assert!(matches!(
            parse(&args(&["find", "--smooth", "a.png"])),
            Ok(Command::Find { smooth: true, .. })
        ));
    }

    /// The race's sound is recorded from runs of the original with `--no-ai`; our render of
    /// such a run must keep the opponents still too, or their crashes and horns would not be
    /// the recording's.
    #[test]
    fn render_audio_keeps_the_opponents_still_with_no_ai() {
        assert_eq!(
            parse(&args(&[
                "render-audio",
                "--startup",
                "--no-ai",
                "--out",
                "a.wav"
            ])),
            Ok(Command::RenderAudio {
                data: None,
                source: AudioSource::Startup,
                keys: vec![],
                seed: 0,
                saves: vec![],
                still: true,
                cfg: None,
                seconds: None,
                out: PathBuf::from("a.wav")
            })
        );
    }

    /// The original's effects are recorded under a `dr.cfg` with the music off
    /// (`reference-run.sh --cfg`), so that short sounds can be heard on their own; our render
    /// of such a run must start from the same `dr.cfg`.
    #[test]
    fn render_audio_starts_from_the_dr_cfg_given() {
        assert_eq!(
            parse(&args(&[
                "render-audio",
                "--startup",
                "--cfg",
                "quiet.cfg",
                "--out",
                "a.wav"
            ])),
            Ok(Command::RenderAudio {
                data: None,
                source: AudioSource::Startup,
                keys: vec![],
                seed: 0,
                saves: vec![],
                still: false,
                cfg: Some(PathBuf::from("quiet.cfg")),
                seconds: None,
                out: PathBuf::from("a.wav")
            })
        );
        assert!(
            parse(&args(&[
                "render-audio",
                "--music",
                "X",
                "--cfg",
                "a",
                "--out",
                "a.wav"
            ]))
            .is_err()
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
