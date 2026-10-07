//! DeadRally: the game's frontend (see docs/adr/0001-platform-layer.md). It plays the
//! original's startup sequence on SDL3, silently until M1b.
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-smooth` starts with the 320x200 screens smoothed; `-nogl` shows the original's
//! software picture (640x480, at a whole scale) instead of scaling the screens to the desktop;
//! `-novsync` turns vsync off, for measuring present cost; `-testscene` runs the M0 test scene,
//! which needs no game data; `--data <dir>` names the game data directory (else
//! `DEADRALLY_DATA`, else `data_path` in the config file, else the folder the player chooses
//! in the system's dialog, which the config file then keeps). The config file can also keep
//! `window`, `smooth`, `nogl` and `vsync`. Alt+Enter toggles fullscreen, F12 toggles the
//! smoothing, closing the window quits. One stats line per second goes to stdout.

// A player's Windows build opens no console window; a debug build keeps it for the stats.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod keymap;

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioGate, Pacer, RunStats, letterbox, whole_scale};
use deadrally_core::{
    AUDIO_CHANNELS, AUDIO_SAMPLE_RATE, Game, InputEvent, PadAxis, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::{
    Config, DATA_ENV_VAR, DataSource, LocateError, Outcome, config_path, load_config, locate,
    save_data_path,
};
use deadrally_gamedata::{dr_cfg, save_game};
use sdl3::Sdl;
use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::dialog::show_open_folder_dialog;
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Gamepad};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FRect, ScaleMode};
use sdl3::video::{FullscreenType, Window};

const BYTES_PER_SAMPLE: usize = 2;

#[derive(Debug, PartialEq, Eq)]
struct Options {
    windowed: bool,
    smooth: bool,
    nogl: bool,
    vsync: bool,
    test_scene: bool,
    data: Option<PathBuf>,
}

fn parse_options(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut options = Options {
        windowed: false,
        smooth: false,
        nogl: false,
        vsync: true,
        test_scene: false,
        data: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-window") => options.windowed = true,
            Some("-smooth") => options.smooth = true,
            Some("-nogl") => options.nogl = true,
            Some("-novsync") => options.vsync = false,
            Some("-testscene") => options.test_scene = true,
            // macOS's Finder may add its process serial number on a first launch.
            Some(arg) if arg.starts_with("-psn_") => {}
            Some("--data") => {
                options.data = Some(PathBuf::from(
                    args.next().ok_or("--data needs a directory")?,
                ));
            }
            _ => {
                return Err(format!(
                    "unknown option {}; known: -window, -smooth, -nogl, -novsync, -testscene, \
                     --data <dir>",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(options)
}

impl Options {
    /// The options with the config file's where the command line gave none: the command line's
    /// flags only turn an option on (or vsync off), so either source can.
    fn with_config(self, config: &Config) -> Options {
        Options {
            windowed: self.windowed || config.window.unwrap_or(false),
            smooth: self.smooth || config.smooth.unwrap_or(false),
            nogl: self.nogl || config.nogl.unwrap_or(false),
            vsync: self.vsync && config.vsync.unwrap_or(true),
            ..self
        }
    }
}

/// The startup sequence on the player's data and `dr.cfg`, a warning to show when the data is
/// not a known release, and where DeadRally keeps its `dr.cfg`.
type Loaded = (Game, Option<String>, Option<PathBuf>);

/// Why the game did not load, and whether choosing its folder in the dialog could help.
struct NotLoaded {
    message: String,
    choosing_helps: bool,
}

impl From<String> for NotLoaded {
    fn from(message: String) -> NotLoaded {
        NotLoaded {
            message,
            choosing_helps: false,
        }
    }
}

/// Whether the system's folder dialog could mend `error`: no folder given anywhere, or the
/// config file's not holding the game. A wrong `--data` or `DEADRALLY_DATA` is the player's
/// to change, and a broken config file is reported as it is.
fn offers_dialog(error: &LocateError) -> bool {
    matches!(
        error,
        LocateError::NotSpecified { .. }
            | LocateError::Invalid {
                source: DataSource::ConfigFile(_),
                ..
            }
    )
}

fn load_game(data: Option<&Path>) -> Result<Loaded, NotLoaded> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let hint = || {
        let file = config.as_deref().map_or_else(
            || "the config file".to_owned(),
            |path| path.display().to_string(),
        );
        format!(
            "Point DeadRally at your copy of Death Rally (the folder that holds MENU.BPA) with \
             --data <dir>, the {DATA_ENV_VAR} environment variable, or data_path in {file}."
        )
    };
    let located = locate(data, env.as_deref(), config.as_deref()).map_err(|error| NotLoaded {
        choosing_helps: offers_dialog(&error),
        message: match error {
            // This one already names all three ways.
            LocateError::NotSpecified { .. } => error.to_string(),
            _ => format!("{error}\n\n{}", hint()),
        },
    })?;
    let dir = &located.validation.dir;
    let warning = match &located.validation.outcome {
        Outcome::Known { .. } => None,
        Outcome::Unknown { closest, differing } => Some(format!(
            "The game data in {} is not a release DeadRally knows (closest: {closest}; \
             different: {}). The game starts anyway, but it may not match the original.",
            dir.display(),
            differing.join(", ")
        )),
    };
    let assets = Assets::load(&located.validation).map_err(|error| {
        format!(
            "cannot read the game data in {}: {error}\n\n{}",
            dir.display(),
            hint()
        )
    })?;
    let own = dr_cfg::own_path();
    let config = dr_cfg::load(own.as_deref(), dir, &assets.menu.default_config)
        .map_err(|error| format!("cannot read dr.cfg: {error}"))?;
    // The original seeds its random numbers with the milliseconds since its start; any
    // number that changes from run to run does as well.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u32);
    let mut game = Game::with_seed(assets, config, seed);
    // Saved games next to DeadRally's own dr.cfg, else the game folder's (only read).
    let own_dir = own.as_deref().and_then(Path::parent);
    game.set_saved_games(save_game::load_slots(own_dir, dir));
    Ok((game, warning, own))
}

/// Shows `message` in a dialog as well as on stderr; the dialog is best effort (there may be
/// no display at all).
fn tell(flag: MessageBoxFlag, title: &str, message: &str) {
    eprintln!("{}: {message}", title.to_lowercase());
    let _ = show_simple_message_box(flag, &format!("DeadRally: {title}"), message, None);
}

/// How a screen `width` pixels wide is filtered when scaled: the original filters its 640x480
/// screens always (0x43B898) and its 320x200 ones when smoothing is on (0x43B6A2).
fn scale_mode(width: u32, smooth: bool) -> ScaleMode {
    if smooth || width != 320 {
        ScaleMode::Linear
    } else {
        ScaleMode::Nearest
    }
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

/// The game on the data the player named or the config file holds; or, when they named none
/// and none was usable, on the folder they then choose in the system's dialog, which the
/// config file keeps from then on (spec M7b).
fn load_or_choose(sdl: &Sdl, data: Option<&Path>) -> Result<Loaded, String> {
    let failure = match load_game(data) {
        Ok(loaded) => return Ok(loaded),
        Err(failure) => failure,
    };
    if !failure.choosing_helps {
        return Err(failure.message);
    }
    tell(
        MessageBoxFlag::INFORMATION,
        "Choose the game",
        &format!(
            "{}\n\nChoose the folder of your copy of Death Rally next.",
            failure.message
        ),
    );
    let Some(dir) = choose_folder(sdl) else {
        return Err(failure.message);
    };
    // Kept first and then read back, so that a folder that does not hold the game is reported
    // as the config file's and is asked for again at the next start.
    match config_path().map(|path| save_data_path(&path, &dir)) {
        Some(Ok(())) => load_game(None).map_err(|failure| NotLoaded {
            message: format!(
                "{}\n\nThe next start asks for the folder again.",
                failure.message
            ),
            ..failure
        }),
        Some(Err(error)) => {
            tell(
                MessageBoxFlag::WARNING,
                "Warning",
                &format!("{error}\n\nDeadRally will ask for the folder again at the next start."),
            );
            load_game(Some(&dir))
        }
        None => load_game(Some(&dir)),
    }
    .map_err(|failure| failure.message)
}

/// The folder the player chooses in the system's dialog; `None` when they cancel or the
/// system has no dialog.
fn choose_folder(sdl: &Sdl) -> Option<PathBuf> {
    let chosen: Arc<Mutex<Option<Option<PathBuf>>>> = Arc::default();
    let answer = Arc::clone(&chosen);
    show_open_folder_dialog(
        None::<&Path>,
        false,
        None::<&Window>,
        Box::new(move |result, _| {
            let dir = result.ok().and_then(|dirs| dirs.into_iter().next());
            *answer.lock().unwrap_or_else(PoisonError::into_inner) = Some(dir);
        }),
    );
    let mut events = sdl.event_pump().ok()?;
    loop {
        if let Some(dir) = chosen.lock().unwrap_or_else(PoisonError::into_inner).take() {
            return dir;
        }
        // Ctrl+C or a stopped dialog must not leave the game waiting with no window.
        if let Some(Event::Quit { .. }) = events.wait_event_timeout(Duration::from_millis(50)) {
            return None;
        }
    }
}

fn main() {
    // Started from Finder or Explorer, a player sees no stderr.
    if let Err(error) = run() {
        tell(MessageBoxFlag::ERROR, "Error", &error.to_string());
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_options(std::env::args_os().skip(1))?;
    let config = match config_path().as_deref().map(load_config) {
        Some(Ok(Some(config))) => config,
        Some(Err(error)) => {
            tell(MessageBoxFlag::WARNING, "Warning", &error.to_string());
            Config::default()
        }
        _ => Config::default(),
    };
    if !config.warnings.is_empty() {
        tell(
            MessageBoxFlag::WARNING,
            "Warning",
            &config.warnings.join("\n"),
        );
    }
    let options = options.with_config(&config);
    sdl3::hint::set("SDL_RENDER_VSYNC", if options.vsync { "1" } else { "0" });

    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let (mut game, dr_cfg_path) = if options.test_scene {
        (Game::test_scene(), None)
    } else {
        match load_or_choose(&sdl, options.data.as_deref()) {
            Ok((game, warning, own)) => {
                if let Some(warning) = warning {
                    tell(MessageBoxFlag::WARNING, "Warning", &warning);
                }
                (game, own)
            }
            Err(message) => {
                tell(MessageBoxFlag::ERROR, "Error", &message);
                std::process::exit(1);
            }
        }
    };
    let gamepads = sdl.gamepad()?;
    let audio = sdl.audio()?;

    let mut window = video.window("DR", 640, 480);
    window.resizable();
    if !options.windowed {
        window.fullscreen();
    }
    let mut canvas = window.build()?.into_canvas();
    let texture_creator = canvas.texture_creator();

    let spec = AudioSpec {
        freq: Some(i32::try_from(AUDIO_SAMPLE_RATE)?),
        channels: Some(i32::try_from(AUDIO_CHANNELS)?),
        format: Some(AudioFormat::s16_sys()),
    };
    let stream = audio
        .open_playback_device(&spec)?
        .open_device_stream(Some(&spec))?;
    stream.resume()?;

    let mut texture_size = (0, 0);
    let mut texture = None;
    let mut rgba = Vec::new();
    let mut samples = Vec::new();
    let mut outgoing = Vec::new();
    let mut smooth = options.smooth;
    let mut open_pads: Vec<Gamepad> = Vec::new();

    let mut pacer = Pacer::new();
    let mut gate = AudioGate::new();
    let mut stats = RunStats::new();
    let start = Instant::now();
    let mut last = start;
    let mut last_report = start;

    let mut events = sdl.event_pump()?;
    'running: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    scancode: Some(Scancode::Return),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) => {
                    // Alt's own press still reaches the game before this, so Alt+Enter skips
                    // the intro or a logo. The original does the same: refreshScreen (0x43B580)
                    // remembers every key press except F12 and Enter with Alt.
                    let window = canvas.window_mut();
                    let fullscreen = window.fullscreen_state() != FullscreenType::Off;
                    window.set_fullscreen(!fullscreen)?;
                }
                Event::KeyDown {
                    scancode: Some(Scancode::F12),
                    repeat: false,
                    ..
                } => smooth = !smooth,
                Event::KeyUp {
                    scancode: Some(Scancode::F12),
                    ..
                } => {}
                Event::KeyDown {
                    scancode: Some(scancode),
                    repeat: false,
                    ..
                } => {
                    if let Some(key) = keymap::key(scancode) {
                        game.input(InputEvent::Key { key, pressed: true });
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = keymap::key(scancode) {
                        game.input(InputEvent::Key {
                            key,
                            pressed: false,
                        });
                    }
                }
                Event::GamepadAdded { which, .. } => {
                    match gamepads.open(which) {
                        Ok(pad) => open_pads.push(pad),
                        Err(error) => eprintln!("cannot open gamepad: {error}"),
                    }
                    game.input(InputEvent::PadConnected {
                        connected: !open_pads.is_empty(),
                    });
                }
                Event::GamepadRemoved { which, .. } => {
                    open_pads.retain(|pad| pad.id().ok() != Some(which));
                    game.input(InputEvent::PadConnected {
                        connected: !open_pads.is_empty(),
                    });
                }
                Event::GamepadButtonDown { button, .. } => {
                    if let Some(button) = keymap::pad_button(button) {
                        game.input(InputEvent::PadButton {
                            button,
                            pressed: true,
                        });
                    }
                }
                Event::GamepadButtonUp { button, .. } => {
                    if let Some(button) = keymap::pad_button(button) {
                        game.input(InputEvent::PadButton {
                            button,
                            pressed: false,
                        });
                    }
                }
                Event::GamepadAxisMotion {
                    axis: Axis::LeftX,
                    value,
                    ..
                } => {
                    game.input(InputEvent::PadAxis {
                        axis: PadAxis::StickX,
                        value,
                    });
                }
                Event::GamepadAxisMotion {
                    axis: Axis::LeftY,
                    value,
                    ..
                } => {
                    game.input(InputEvent::PadAxis {
                        axis: PadAxis::StickY,
                        value,
                    });
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let ticks = pacer.advance(nanos(now - last));
        last = now;
        for _ in 0..ticks {
            game.tick();
            samples.clear();
            game.take_audio(&mut samples);
            let queued_frames =
                usize::try_from(stream.queued_bytes()?)? / (AUDIO_CHANNELS * BYTES_PER_SAMPLE);
            outgoing.clear();
            gate.feed(queued_frames, &samples, &mut outgoing);
            if !outgoing.is_empty() {
                stream.put_data_i16(&outgoing)?;
            }
        }
        stats.add_ticks(ticks);
        if let (Some(bytes), Some(path)) = (game.take_config(), &dr_cfg_path)
            && let Err(error) = dr_cfg::save(path, &bytes)
        {
            eprintln!("warning: cannot write {}: {error}", path.display());
        }
        if let Some((slot, bytes)) = game.take_saved_game() {
            match dr_cfg_path.as_deref().and_then(Path::parent) {
                Some(dir) => {
                    if let Err(error) = save_game::write_slot(dir, slot, &bytes) {
                        eprintln!(
                            "warning: cannot write {}: {error}",
                            dir.join(save_game::file_name(slot)).display()
                        );
                    }
                }
                None => eprintln!(
                    "warning: no configuration folder, so the game saved in slot {slot} is lost"
                ),
            }
        }
        if game.quit_requested() {
            break 'running;
        }

        let present_start = Instant::now();
        let frame = game.frame();
        // -nogl: the original's software picture, always 640x480, its pixels kept whole. Only
        // the test scene has screens of other sizes, which are scaled as without it.
        let software =
            options.nogl && matches!((frame.width, frame.height), (640, 480) | (320, 200));
        let size = if software {
            (WINDOW_WIDTH, WINDOW_HEIGHT)
        } else {
            (frame.width, frame.height)
        };
        if texture.is_none() || texture_size != size {
            texture = Some(texture_creator.create_texture_streaming(
                PixelFormat::RGBA32,
                size.0,
                size.1,
            )?);
            texture_size = size;
            rgba.resize(size.0 as usize * size.1 as usize * 4, 0);
        }
        let texture = texture.as_mut().expect("created above");
        let (output_width, output_height) = canvas.output_size()?;
        let viewport = if software {
            frame.write_window_rgba(smooth, &mut rgba)?;
            texture.set_scale_mode(ScaleMode::Nearest);
            whole_scale(output_width, output_height, size)
        } else {
            frame.write_rgba(&mut rgba);
            texture.set_scale_mode(scale_mode(frame.width, smooth));
            letterbox(output_width, output_height, frame.aspect)
        };
        texture.update(None, &rgba, size.0 as usize * 4)?;
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        if viewport.width > 0 && viewport.height > 0 {
            let target = FRect::new(
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
            );
            canvas.copy(texture, None, Some(target))?;
        }
        canvas.present();
        stats.add_present(u32::try_from(present_start.elapsed().as_micros()).unwrap_or(u32::MAX));

        if now - last_report >= Duration::from_secs(1) {
            last_report = now;
            let queued_frames =
                usize::try_from(stream.queued_bytes()?)? / (AUDIO_CHANNELS * BYTES_PER_SAMPLE);
            let audio = gate.report(queued_frames);
            println!(
                "{}",
                stats.line(nanos(now - start), pacer.dropped_ticks(), audio)
            );
        }
    }

    let audio = gate.report(0);
    println!(
        "final {}",
        stats.line(nanos(start.elapsed()), pacer.dropped_ticks(), audio)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use deadrally_gamedata::{ConfigError, ValidationError};

    fn parse(list: &[&str]) -> Result<Options, String> {
        parse_options(list.iter().map(OsString::from))
    }

    #[test]
    fn options_select_the_scene_and_the_data() {
        let options = parse(&["-window", "-testscene", "--data", "/games/dr"]).unwrap();
        assert!(options.windowed && options.test_scene && options.vsync);
        assert_eq!(options.data, Some(PathBuf::from("/games/dr")));
        assert_eq!(parse(&[]).unwrap().data, None);
    }

    #[test]
    fn the_originals_display_options_are_known() {
        // The 2009 readme tells players to add -smooth and -nogl; refusing them would stop
        // the game for a player who follows it.
        let options = parse(&["-smooth", "-nogl"]).unwrap();
        assert!(options.smooth && options.nogl);
        let plain = parse(&[]).unwrap();
        assert!(!plain.smooth && !plain.nogl);
    }

    #[test]
    fn the_config_file_keeps_the_options_and_the_command_line_wins() {
        // A player who wrote smooth = true once gets it every time; -novsync on the command
        // line still turns vsync off for one run.
        let config = Config {
            window: Some(true),
            smooth: Some(true),
            vsync: Some(true),
            ..Config::default()
        };
        let options = parse(&["-novsync"]).unwrap().with_config(&config);
        assert!(options.windowed && options.smooth && !options.nogl && !options.vsync);
        let off = Config {
            nogl: Some(false),
            vsync: Some(false),
            ..Config::default()
        };
        let options = parse(&["-nogl"]).unwrap().with_config(&off);
        assert!(options.nogl && !options.vsync && !options.windowed);
    }

    #[test]
    fn smoothing_is_for_the_low_resolution_screens_and_the_menus_are_always_filtered() {
        // The original filters its 640x480 screens whatever F12 says (0x43B898) and its
        // 320x200 ones only when smoothing is on (0x43B6A2).
        for smooth in [false, true] {
            assert_eq!(scale_mode(640, smooth), ScaleMode::Linear);
        }
        assert_eq!(scale_mode(320, false), ScaleMode::Nearest);
        assert_eq!(scale_mode(320, true), ScaleMode::Linear);
    }

    #[test]
    fn unusable_data_says_how_to_point_at_other_data() {
        // A player whose copy is incomplete or damaged must learn how to choose another one.
        let empty = tempfile::tempdir().unwrap();
        let Err(NotLoaded { message, .. }) = load_game(Some(empty.path())) else {
            panic!("an empty folder is no game data");
        };
        for needle in ["--data", "DEADRALLY_DATA", "data_path"] {
            assert!(message.contains(needle), "{needle} missing in: {message}");
        }
    }

    #[test]
    fn the_folder_dialog_is_offered_only_when_choosing_can_help() {
        // A broken config file or a wrong --data must be reported as such: a dialog there
        // would ask for a folder the player already has, and rewrite their config file.
        let unreadable = || ValidationError::DirUnreadable {
            dir: PathBuf::from("/nowhere"),
            source: std::io::Error::other("gone"),
        };
        assert!(offers_dialog(&LocateError::NotSpecified {
            config_path: None
        }));
        assert!(offers_dialog(&LocateError::Invalid {
            source: DataSource::ConfigFile(PathBuf::from("config.toml")),
            error: unreadable(),
        }));
        assert!(!offers_dialog(&LocateError::Invalid {
            source: DataSource::CommandLine,
            error: unreadable(),
        }));
        assert!(!offers_dialog(&LocateError::Invalid {
            source: DataSource::Environment,
            error: unreadable(),
        }));
        assert!(!offers_dialog(&LocateError::Config(ConfigError::Parse {
            path: PathBuf::from("config.toml"),
            message: "line 1".to_owned(),
        })));
    }

    #[test]
    fn macos_launch_arguments_are_ignored() {
        // Finder may pass -psn_0_NNN on a first launch; refusing it would stop the game
        // before it shows anything.
        let options = parse(&["-psn_0_1234567", "-window"]).unwrap();
        assert!(options.windowed);
    }

    #[test]
    fn unknown_or_incomplete_options_are_errors() {
        // A typo such as -testcsene must not silently start the real game instead.
        assert!(parse(&["-testcsene"]).unwrap_err().contains("-testscene"));
        assert!(parse(&["--data"]).is_err());
    }
}
