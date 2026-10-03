//! Platform spike candidate: the DeadRally test scene on winit, wgpu, cpal and gilrs (spec
//! section 7).
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-novsync` turns vsync off, for measuring present cost. Alt+Enter toggles
//! fullscreen, F12 toggles bilinear smoothing, closing the window quits. One stats line per
//! second goes to stdout.

mod audio;
mod keymap;
mod present;

use std::error::Error;
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioGate, Pacer, RunStats};
use deadrally_core::{AUDIO_CHANNELS, Game, InputEvent, PadAxis};
use gilrs::{Axis, EventType, Gilrs};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::audio::SampleQueue;
use crate::present::Gpu;

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

/// Converts a gilrs stick value (-1.0..=1.0, up is positive) to the core's range (up is
/// negative).
fn stick(value: f32, invert: bool) -> i16 {
    let value = if invert { -value } else { value };
    (value.clamp(-1.0, 1.0) * 32_767.0) as i16
}

/// The window and its GPU side, created once the event loop is running.
struct Display {
    window: Arc<Window>,
    gpu: Gpu,
    rgba: Vec<u8>,
}

struct App {
    options: Options,
    display: Option<Display>,
    error: Option<Box<dyn Error>>,
    game: Game,
    gilrs: Option<Gilrs>,
    queue: SampleQueue,
    _stream: Option<cpal::Stream>,
    samples: Vec<i16>,
    outgoing: Vec<i16>,
    smooth: bool,
    alt_held: bool,
    pacer: Pacer,
    gate: AudioGate,
    stats: RunStats,
    start: Instant,
    last: Instant,
    last_report: Instant,
}

impl App {
    fn create_display(&self, event_loop: &ActiveEventLoop) -> Result<Display, Box<dyn Error>> {
        let mut attributes = Window::default_attributes()
            .with_title("DR")
            .with_inner_size(LogicalSize::new(640, 480));
        if !self.options.windowed {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = Arc::new(event_loop.create_window(attributes)?);
        let gpu = Gpu::new(Arc::clone(&window), self.options.vsync)?;
        Ok(Display {
            window,
            gpu,
            rgba: Vec::new(),
        })
    }

    fn poll_gamepads(&mut self) {
        let Some(gilrs) = &mut self.gilrs else { return };
        while let Some(event) = gilrs.next_event() {
            let input = match event.event {
                EventType::ButtonPressed(button, _) => {
                    keymap::pad_button(button).map(|button| InputEvent::PadButton {
                        button,
                        pressed: true,
                    })
                }
                EventType::ButtonReleased(button, _) => {
                    keymap::pad_button(button).map(|button| InputEvent::PadButton {
                        button,
                        pressed: false,
                    })
                }
                EventType::AxisChanged(Axis::LeftStickX, value, _) => Some(InputEvent::PadAxis {
                    axis: PadAxis::StickX,
                    value: stick(value, false),
                }),
                EventType::AxisChanged(Axis::LeftStickY, value, _) => Some(InputEvent::PadAxis {
                    axis: PadAxis::StickY,
                    value: stick(value, true),
                }),
                _ => None,
            };
            if let Some(input) = input {
                self.game.input(input);
            }
        }
    }

    fn advance(&mut self) {
        let now = Instant::now();
        let ticks = self.pacer.advance(nanos(now - self.last));
        self.last = now;
        for _ in 0..ticks {
            self.game.tick();
            self.samples.clear();
            self.game.take_audio(&mut self.samples);
            let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
            self.outgoing.clear();
            let queued_frames = queue.len() / AUDIO_CHANNELS;
            self.gate
                .feed(queued_frames, &self.samples, &mut self.outgoing);
            queue.extend(&self.outgoing);
        }
        self.stats.add_ticks(ticks);

        if now - self.last_report >= Duration::from_secs(1) {
            self.last_report = now;
            println!("{}", self.stats_line(now));
        }
    }

    fn stats_line(&self, now: Instant) -> String {
        let queued_frames = self
            .queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
            / AUDIO_CHANNELS;
        self.stats.line(
            nanos(now - self.start),
            self.pacer.dropped_ticks(),
            self.gate.report(queued_frames),
        )
    }

    fn present(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(display) = &mut self.display else {
            return Ok(());
        };
        let present_start = Instant::now();
        let frame = self.game.frame();
        display.rgba.resize(frame.pixels.len() * 4, 0);
        frame.write_rgba(&mut display.rgba);
        display.gpu.present(&frame, &display.rgba, self.smooth)?;
        self.stats
            .add_present(u32::try_from(present_start.elapsed().as_micros()).unwrap_or(u32::MAX));
        Ok(())
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Box<dyn Error>) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.display.is_none() {
            match self.create_display(event_loop) {
                Ok(display) => self.display = Some(display),
                Err(error) => self.fail(event_loop, error),
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(modifiers) => self.alt_held = modifiers.state().alt_key(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                match code {
                    KeyCode::Enter if pressed && self.alt_held => {
                        if !repeat && let Some(display) = &self.display {
                            let fullscreen = display.window.fullscreen().is_some();
                            display.window.set_fullscreen(
                                (!fullscreen).then_some(Fullscreen::Borderless(None)),
                            );
                        }
                    }
                    KeyCode::F12 => {
                        if pressed && !repeat {
                            self.smooth = !self.smooth;
                        }
                    }
                    _ if repeat => {}
                    _ => {
                        if let Some(key) = keymap::key(code) {
                            self.game.input(InputEvent::Key { key, pressed });
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.present() {
                    self.fail(event_loop, error);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.poll_gamepads();
        self.advance();
        if let Some(display) = &self.display {
            display.window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options()?;
    let queue = SampleQueue::default();
    let stream = audio::start(Arc::clone(&queue))
        .inspect_err(|error| eprintln!("no audio: {error}"))
        .ok();
    let gilrs = Gilrs::new()
        .inspect_err(|error| eprintln!("no gamepads: {error}"))
        .ok();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let now = Instant::now();
    let mut app = App {
        options,
        display: None,
        error: None,
        game: Game::new(),
        gilrs,
        queue,
        _stream: stream,
        samples: Vec::new(),
        outgoing: Vec::new(),
        smooth: false,
        alt_held: false,
        pacer: Pacer::new(),
        gate: AudioGate::new(),
        stats: RunStats::new(),
        start: now,
        last: now,
        last_report: now,
    };
    event_loop.run_app(&mut app)?;
    println!("final {}", app.stats_line(Instant::now()));
    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
