//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 1/70 s of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

/// Simulation ticks per second. The original runs its logic at 70 Hz and stores lap times in
/// 1/70 s units.
pub const TICKS_PER_SECOND: u32 = 70;

/// Output sample rate in Hz.
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 44 100 / 70, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 630;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK * TICKS_PER_SECOND as usize == AUDIO_SAMPLE_RATE as usize);
