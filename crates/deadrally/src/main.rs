//! DeadRally: the game's frontend (see docs/adr/0001-platform-layer.md). It plays the
//! original's startup sequence on SDL3, silently until M1b.
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-novsync` turns vsync off, for measuring present cost; `-testscene` runs the
//! M0 test scene, which needs no game data; `--data <dir>` names the game data directory (else
//! `DEADRALLY_DATA`, else `data_path` in the config file). Alt+Enter toggles fullscreen, F12
//! toggles bilinear smoothing, closing the window quits. One stats line per second goes to
//! stdout.

mod keymap;

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioGate, Pacer, RunStats, letterbox};
use deadrally_core::{AUDIO_CHANNELS, AUDIO_SAMPLE_RATE, Game, InputEvent, PadAxis};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::{DATA_ENV_VAR, Outcome, config_path, locate};
use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Gamepad};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FRect, ScaleMode};
use sdl3::video::FullscreenType;

const BYTES_PER_SAMPLE: usize = 2;

#[derive(Debug, PartialEq, Eq)]
struct Options {
    windowed: bool,
    vsync: bool,
    test_scene: bool,
    data: Option<PathBuf>,
}

fn parse_options(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut options = Options {
        windowed: false,
        vsync: true,
        test_scene: false,
        data: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-window") => options.windowed = true,
            Some("-novsync") => options.vsync = false,
            Some("-testscene") => options.test_scene = true,
            Some("--data") => {
                options.data = Some(PathBuf::from(
                    args.next().ok_or("--data needs a directory")?,
                ));
            }
            _ => {
                return Err(format!(
                    "unknown option {}; known: -window, -novsync, -testscene, --data <dir>",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(options)
}

/// The startup sequence on the player's data, plus a warning to show when the data is not a
/// known release.
fn load_game(data: Option<&Path>) -> Result<(Game, Option<String>), String> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let located =
        locate(data, env.as_deref(), config.as_deref()).map_err(|error| error.to_string())?;
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
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
    let assets = Assets::load(&located.validation)
        .map_err(|error| format!("cannot read the game data in {}: {error}", dir.display()))?;
    Ok((Game::new(assets), warning))
}

/// Shows `message` in a dialog as well as on stderr; the dialog is best effort (there may be
/// no display at all).
fn tell(flag: MessageBoxFlag, title: &str, message: &str) {
    eprintln!("{}: {message}", title.to_lowercase());
    let _ = show_simple_message_box(flag, &format!("DeadRally: {title}"), message, None);
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options(std::env::args_os().skip(1))?;
    let mut game = if options.test_scene {
        Game::test_scene()
    } else {
        match load_game(options.data.as_deref()) {
            Ok((game, warning)) => {
                if let Some(warning) = warning {
                    tell(MessageBoxFlag::WARNING, "Warning", &warning);
                }
                game
            }
            Err(message) => {
                tell(MessageBoxFlag::ERROR, "Error", &message);
                std::process::exit(1);
            }
        }
    };
    sdl3::hint::set("SDL_RENDER_VSYNC", if options.vsync { "1" } else { "0" });

    let sdl = sdl3::init()?;
    let video = sdl.video()?;
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
    let mut smooth = false;
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
                Event::GamepadAdded { which, .. } => match gamepads.open(which) {
                    Ok(pad) => open_pads.push(pad),
                    Err(error) => eprintln!("cannot open gamepad: {error}"),
                },
                Event::GamepadRemoved { which, .. } => {
                    open_pads.retain(|pad| pad.id().ok() != Some(which))
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

        let present_start = Instant::now();
        let frame = game.frame();
        if texture.is_none() || texture_size != (frame.width, frame.height) {
            texture = Some(texture_creator.create_texture_streaming(
                PixelFormat::RGBA32,
                frame.width,
                frame.height,
            )?);
            texture_size = (frame.width, frame.height);
            rgba.resize(frame.pixels.len() * 4, 0);
        }
        let texture = texture.as_mut().expect("created above");
        frame.write_rgba(&mut rgba);
        texture.update(None, &rgba, frame.width as usize * 4)?;
        texture.set_scale_mode(if smooth {
            ScaleMode::Linear
        } else {
            ScaleMode::Nearest
        });

        let (output_width, output_height) = canvas.output_size()?;
        let viewport = letterbox(output_width, output_height, frame.aspect);
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
    fn unknown_or_incomplete_options_are_errors() {
        // A typo such as -testcsene must not silently start the real game instead.
        assert!(parse(&["-testcsene"]).unwrap_err().contains("-testscene"));
        assert!(parse(&["--data"]).is_err());
    }
}
