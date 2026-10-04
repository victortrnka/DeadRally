//! DeadRally: the game's frontend (see docs/adr/0001-platform-layer.md). In M0 it runs the test
//! scene on SDL3.
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-novsync` turns vsync off, for measuring present cost. Alt+Enter toggles
//! fullscreen, F12 toggles bilinear smoothing, closing the window quits. One stats line per
//! second goes to stdout.

mod keymap;

use std::error::Error;
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioGate, Pacer, RunStats, letterbox};
use deadrally_core::{AUDIO_CHANNELS, AUDIO_SAMPLE_RATE, Game, InputEvent, PadAxis};
use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Gamepad};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FRect, ScaleMode};
use sdl3::video::FullscreenType;

const BYTES_PER_SAMPLE: usize = 2;

struct Options {
    windowed: bool,
    vsync: bool,
}

fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        windowed: false,
        vsync: true,
    };
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-window" => options.windowed = true,
            "-novsync" => options.vsync = false,
            other => return Err(format!("unknown option {other}; known: -window, -novsync")),
        }
    }
    Ok(options)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options()?;
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

    let mut game = Game::test_scene();
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
