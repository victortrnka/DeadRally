//! Pure helpers that every frontend shares, so all frontends pace ticks, bound audio latency,
//! letterbox and report in exactly the same way. Nothing here reads a clock: the frontend
//! measures time and passes it in.

use crate::{AUDIO_FRAMES_PER_TICK, AUDIO_SAMPLE_RATE, TICKS_PER_SECOND};

/// The most ticks a frontend runs before presenting a frame. After a stall (a dragged window,
/// a breakpoint, a laptop waking up) the game skips ahead instead of fast-forwarding.
pub const MAX_CATCH_UP_TICKS: u32 = 5;

/// Audio latency the frontend aims for, in ticks (about 43 ms).
pub const AUDIO_TARGET_QUEUE_TICKS: usize = 3;

/// Queued audio above this many ticks is dropped until the queue is back at the target.
pub const AUDIO_MAX_QUEUE_TICKS: usize = 8;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Turns elapsed wall-clock time into a number of ticks to run.
#[derive(Debug, Default)]
pub struct Pacer {
    /// Time not yet turned into ticks, in units of 1/(70 * 10^9) s, so one tick is exactly
    /// 10^9 units and no rounding error accumulates.
    backlog: u64,
    dropped_ticks: u64,
}

impl Pacer {
    #[must_use]
    pub fn new() -> Pacer {
        Pacer::default()
    }

    /// Adds `elapsed_nanos` of wall-clock time and returns how many ticks to run now, at most
    /// [`MAX_CATCH_UP_TICKS`]. Ticks beyond that are dropped and counted.
    pub fn advance(&mut self, elapsed_nanos: u64) -> u32 {
        self.backlog = self
            .backlog
            .saturating_add(elapsed_nanos.saturating_mul(u64::from(TICKS_PER_SECOND)));
        let due = self.backlog / NANOS_PER_SECOND;
        self.backlog %= NANOS_PER_SECOND;
        let max = u64::from(MAX_CATCH_UP_TICKS);
        if due > max {
            self.dropped_ticks += due - max;
            MAX_CATCH_UP_TICKS
        } else {
            u32::try_from(due).expect("due <= MAX_CATCH_UP_TICKS")
        }
    }

    #[must_use]
    pub fn dropped_ticks(&self) -> u64 {
        self.dropped_ticks
    }
}

/// What to do with one tick's samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDecision {
    /// Queue `silence_frames` stereo frames of silence, then the tick's samples.
    Queue { silence_frames: usize },
    /// Drop the tick's samples: the device queue is too long.
    Drop,
}

/// Keeps the audio device queue near [`AUDIO_TARGET_QUEUE_TICKS`], so latency is low but
/// stable and identical in every frontend.
///
/// - An empty queue (at start, or after the device ran dry) is first padded with silence up to
///   one tick below the target, so ordinary jitter does not starve the device. Running dry
///   after the start counts as an underrun.
/// - A queue longer than [`AUDIO_MAX_QUEUE_TICKS`] (the wall clock runs faster than the sound
///   card) drops incoming audio until it has drained back to the target, so latency cannot
///   drift.
#[derive(Debug, Default)]
pub struct AudioGate {
    started: bool,
    draining: bool,
    underruns: u64,
    discarded_ticks: u64,
}

impl AudioGate {
    #[must_use]
    pub fn new() -> AudioGate {
        AudioGate::default()
    }

    /// `queued_frames` is the number of stereo frames still waiting to be played. Call once per
    /// tick, just before queueing that tick's samples.
    pub fn decide(&mut self, queued_frames: usize) -> AudioDecision {
        let queued_ticks = queued_frames / AUDIO_FRAMES_PER_TICK;
        if queued_ticks > AUDIO_MAX_QUEUE_TICKS {
            self.draining = true;
        } else if queued_ticks <= AUDIO_TARGET_QUEUE_TICKS {
            self.draining = false;
        }
        if self.draining {
            self.discarded_ticks += 1;
            return AudioDecision::Drop;
        }
        if queued_frames > 0 {
            return AudioDecision::Queue { silence_frames: 0 };
        }
        if self.started {
            self.underruns += 1;
        }
        self.started = true;
        AudioDecision::Queue {
            silence_frames: (AUDIO_TARGET_QUEUE_TICKS - 1) * AUDIO_FRAMES_PER_TICK,
        }
    }

    /// Times the queue ran dry after audio had started.
    #[must_use]
    pub fn underruns(&self) -> u64 {
        self.underruns
    }

    #[must_use]
    pub fn discarded_ticks(&self) -> u64 {
        self.discarded_ticks
    }

    /// The counters for a stats line.
    #[must_use]
    pub fn report(&self, queued_frames: usize) -> AudioReport {
        AudioReport {
            underruns: self.underruns,
            discarded_ticks: self.discarded_ticks,
            queued_frames,
        }
    }
}

/// Where to draw the frame inside the output, in output pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The largest rectangle with display aspect `aspect` that fits an output of
/// `output_width` x `output_height`, centred; the rest becomes black bars. A zero-sized output
/// (a minimised window) gives a zero-sized viewport.
#[must_use]
pub fn letterbox(output_width: u32, output_height: u32, aspect: (u32, u32)) -> Viewport {
    let (aspect_width, aspect_height) = (u64::from(aspect.0), u64::from(aspect.1));
    let (out_w, out_h) = (u64::from(output_width), u64::from(output_height));
    let (width, height) = if aspect_width == 0 || aspect_height == 0 {
        (out_w, out_h)
    } else if out_w * aspect_height >= out_h * aspect_width {
        (out_h * aspect_width / aspect_height, out_h)
    } else {
        (out_w, out_w * aspect_height / aspect_width)
    };
    let width = u32::try_from(width).expect("width <= output width");
    let height = u32::try_from(height).expect("height <= output height");
    Viewport {
        x: (output_width - width) / 2,
        y: (output_height - height) / 2,
        width,
        height,
    }
}

/// Audio counters a frontend reports.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioReport {
    pub underruns: u64,
    pub discarded_ticks: u64,
    pub queued_frames: usize,
}

/// Counters a frontend prints once per second and once at exit.
#[derive(Debug, Default)]
pub struct RunStats {
    ticks: u64,
    present_micros: Vec<u32>,
}

impl RunStats {
    #[must_use]
    pub fn new() -> RunStats {
        RunStats::default()
    }

    pub fn add_ticks(&mut self, ticks: u32) {
        self.ticks += u64::from(ticks);
    }

    /// Records how long one present took (upload, draw and present), in microseconds.
    pub fn add_present(&mut self, micros: u32) {
        self.present_micros.push(micros);
    }

    /// One cumulative log line, for example
    /// `t=60.000s ticks=4200 rate=70.00/s frames=3600 dropped_ticks=0 underruns=0 discarded_ticks=0 queue_ms=42 present_avg_us=850 present_p99_us=1200`.
    #[must_use]
    pub fn line(&self, elapsed_nanos: u64, dropped_ticks: u64, audio: AudioReport) -> String {
        let millis = elapsed_nanos / 1_000_000;
        let rate_centi = if elapsed_nanos == 0 {
            0
        } else {
            u128::from(self.ticks) * 100 * u128::from(NANOS_PER_SECOND) / u128::from(elapsed_nanos)
        };
        let queue_ms = audio.queued_frames as u64 * 1000 / u64::from(AUDIO_SAMPLE_RATE);
        format!(
            "t={}.{:03}s ticks={} rate={}.{:02}/s frames={} dropped_ticks={} underruns={} discarded_ticks={} queue_ms={} present_avg_us={} present_p99_us={}",
            millis / 1000,
            millis % 1000,
            self.ticks,
            rate_centi / 100,
            rate_centi % 100,
            self.present_micros.len(),
            dropped_ticks,
            audio.underruns,
            audio.discarded_ticks,
            queue_ms,
            self.present_average(),
            self.present_percentile(99),
        )
    }

    fn present_average(&self) -> u64 {
        let count = self.present_micros.len() as u64;
        if count == 0 {
            return 0;
        }
        self.present_micros
            .iter()
            .map(|&m| u64::from(m))
            .sum::<u64>()
            / count
    }

    /// Nearest-rank percentile of the recorded present times; 0 when nothing was presented.
    fn present_percentile(&self, percent: usize) -> u32 {
        if self.present_micros.is_empty() {
            return 0;
        }
        let mut sorted = self.present_micros.clone();
        sorted.sort_unstable();
        let rank = (sorted.len() * percent).div_ceil(100).max(1);
        sorted[rank - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_60HZ_NANOS: u64 = 16_666_667;

    #[test]
    fn pacer_runs_exactly_70_ticks_per_second_at_60_hz() {
        // Drift here would desynchronise lap times and music over a long race.
        let mut pacer = Pacer::new();
        let ticks: u32 = (0..36_000).map(|_| pacer.advance(FRAME_60HZ_NANOS)).sum();
        // 36 000 frames of 16 666 667 ns are 600.000012 s: exactly 42 000 ticks.
        assert_eq!(ticks, 42_000);
        assert_eq!(pacer.dropped_ticks(), 0);
    }

    #[test]
    fn pacer_caps_catch_up_after_a_stall_and_counts_the_rest() {
        // After a one-second stall the game must not fast-forward 70 ticks in one frame.
        let mut pacer = Pacer::new();
        assert_eq!(pacer.advance(NANOS_PER_SECOND), MAX_CATCH_UP_TICKS);
        assert_eq!(pacer.dropped_ticks(), 70 - u64::from(MAX_CATCH_UP_TICKS));
    }

    #[test]
    fn pacer_survives_absurd_elapsed_times() {
        // A clock jump (suspend, debugger) must not panic on overflow.
        let mut pacer = Pacer::new();
        assert_eq!(pacer.advance(u64::MAX), MAX_CATCH_UP_TICKS);
        assert_eq!(pacer.advance(0), 0);
    }

    #[test]
    fn audio_gate_primes_an_empty_queue_with_silence() {
        // Queueing one tick at a time into an empty device starves it on the first jitter;
        // starting two ticks ahead keeps the queue near the ~43 ms target.
        let mut gate = AudioGate::new();
        let prime = AudioDecision::Queue {
            silence_frames: 2 * AUDIO_FRAMES_PER_TICK,
        };
        assert_eq!(gate.decide(0), prime);
        assert_eq!(gate.underruns(), 0, "starting is not an underrun");
        assert_eq!(
            gate.decide(AUDIO_FRAMES_PER_TICK),
            AudioDecision::Queue { silence_frames: 0 }
        );
        assert_eq!(gate.decide(0), prime, "after running dry, prime again");
        assert_eq!(gate.underruns(), 1);
    }

    #[test]
    fn audio_gate_drains_to_target_then_queues_again() {
        // Latency must come back down to ~43 ms, not hover just under the maximum.
        let mut gate = AudioGate::new();
        let tick = AUDIO_FRAMES_PER_TICK;
        let queue = AudioDecision::Queue { silence_frames: 0 };
        assert_eq!(gate.decide(AUDIO_MAX_QUEUE_TICKS * tick), queue);
        assert_eq!(
            gate.decide((AUDIO_MAX_QUEUE_TICKS + 1) * tick),
            AudioDecision::Drop
        );
        assert_eq!(gate.decide(5 * tick), AudioDecision::Drop);
        assert_eq!(
            gate.decide((AUDIO_TARGET_QUEUE_TICKS + 1) * tick),
            AudioDecision::Drop
        );
        assert_eq!(gate.decide(AUDIO_TARGET_QUEUE_TICKS * tick), queue);
        assert_eq!(gate.discarded_ticks(), 3);
        assert_eq!(gate.report(7).queued_frames, 7);
    }

    #[test]
    fn letterbox_adds_side_bars_for_4_3_on_16_9() {
        assert_eq!(
            letterbox(1920, 1080, (4, 3)),
            Viewport {
                x: 240,
                y: 0,
                width: 1440,
                height: 1080
            }
        );
    }

    #[test]
    fn letterbox_adds_top_and_bottom_bars_for_16_9_on_4_3() {
        assert_eq!(
            letterbox(1024, 768, (16, 9)),
            Viewport {
                x: 0,
                y: 96,
                width: 1024,
                height: 576
            }
        );
    }

    #[test]
    fn letterbox_of_a_minimised_window_is_empty() {
        // Some platforms report a 0x0 drawable while minimised; that must not panic.
        assert_eq!(
            letterbox(0, 0, (4, 3)),
            Viewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0
            }
        );
    }

    #[test]
    fn stats_line_reports_rate_and_percentiles() {
        let mut stats = RunStats::new();
        stats.add_ticks(70);
        for micros in 1..=100 {
            stats.add_present(micros);
        }
        let audio = AudioReport {
            underruns: 1,
            discarded_ticks: 2,
            queued_frames: 1890,
        };
        assert_eq!(
            stats.line(NANOS_PER_SECOND, 3, audio),
            "t=1.000s ticks=70 rate=70.00/s frames=100 dropped_ticks=3 underruns=1 discarded_ticks=2 queue_ms=42 present_avg_us=50 present_p99_us=99"
        );
    }

    #[test]
    fn stats_line_before_anything_happened_is_all_zero() {
        let line = RunStats::new().line(0, 0, AudioReport::default());
        assert_eq!(
            line,
            "t=0.000s ticks=0 rate=0.00/s frames=0 dropped_ticks=0 underruns=0 discarded_ticks=0 queue_ms=0 present_avg_us=0 present_p99_us=0"
        );
    }
}
