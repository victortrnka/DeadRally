//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 14 ms of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod audio;
mod campaign;
mod canvas;
mod fade;
mod font;
mod frame;
mod game;
pub mod host;
mod input;
mod keys;
mod menu;
mod race;
mod startup;
mod test_scene;
mod trig;

pub use audio::{render_effect, render_music};
pub use frame::{Frame, expand_6bit};
pub use game::Game;
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Length of one simulation tick: 14 ms, as in the Windows version, which counts time as
/// `SDL_GetTicks() / 14` (about 71.43 ticks per second; DOS ran at 70 Hz).
pub const TICK_NANOS: u64 = 14_000_000;

/// Output sample rate in Hz. At 48 kHz a 14 ms tick is a whole number of frames.
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 48 000 × 0.014, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 672;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK as u64 * 1_000_000_000 == AUDIO_SAMPLE_RATE as u64 * TICK_NANOS);
