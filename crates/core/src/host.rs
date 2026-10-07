//! Pure helpers that every frontend shares, so all frontends pace ticks, bound audio latency,
//! letterbox and report in exactly the same way. Nothing here reads a clock: the frontend
//! measures time and passes it in.

use crate::{AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, AUDIO_SAMPLE_RATE, TICK_NANOS};

/// The most ticks a frontend runs before presenting a frame. After a stall (a dragged window,
/// a breakpoint, a laptop waking up) the game skips ahead instead of fast-forwarding.
pub const MAX_CATCH_UP_TICKS: u32 = 5;

/// Audio latency the frontend aims for, in ticks (about 43 ms).
pub const AUDIO_TARGET_QUEUE_TICKS: usize = 3;

/// Queued audio above this many ticks is dropped until the queue is back at the target.
pub const AUDIO_MAX_QUEUE_TICKS: usize = 8;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Ticks after the first one before the audio gate locks onto the queue's settled length.
const AUDIO_SETTLE_TICKS: u64 = 2 * NANOS_PER_SECOND / TICK_NANOS;

/// The queue length is averaged over roughly this many ticks, which smooths out the sawtooth
/// of a device that takes its audio in large chunks.
const QUEUE_AVERAGE_TICKS: i64 = 64;

/// Drift correction starts when the average strays this far from the held length: half a tick.
const DRIFT_BAND_FRAMES: i64 = (AUDIO_FRAMES_PER_TICK / 2) as i64;

/// Turns elapsed wall-clock time into a number of ticks to run.
#[derive(Debug, Default)]
pub struct Pacer {
    /// Time not yet turned into ticks, in nanoseconds.
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
        self.backlog = self.backlog.saturating_add(elapsed_nanos);
        let due = self.backlog / TICK_NANOS;
        self.backlog %= TICK_NANOS;
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

/// Keeps the audio device queue near [`AUDIO_TARGET_QUEUE_TICKS`], so latency is low but
/// stable and identical in every frontend.
///
/// - An empty queue (at start, or after the device ran dry) is first padded with silence up to
///   one tick below the target, so ordinary jitter does not starve the device. Running dry
///   after the start counts as an underrun.
/// - Sound cards run a little fast or slow against the system clock (tens of ppm). Two seconds
///   after the start the gate notes where the queue's average has settled (at least the
///   target) and holds it there by repeating or dropping one stereo frame in a tick when the
///   average strays by more than half a tick. That changes the speed by at most 1/630
///   (0.16 %), which nobody hears, and stops the queue from slowly running dry or filling up.
/// - A queue longer than [`AUDIO_MAX_QUEUE_TICKS`] (after a stall, say) drops incoming audio
///   until it has drained back to the target.
#[derive(Debug, Default)]
pub struct AudioGate {
    started: bool,
    draining: bool,
    underruns: u64,
    discarded_ticks: u64,
    /// Ticks fed so far.
    fed_ticks: u64,
    /// Exponential moving average of the queue length, times `QUEUE_AVERAGE_TICKS`.
    average_scaled: i64,
    /// The queue length the average is held at, once settled.
    hold: Option<i64>,
    /// Frames repeated (positive) minus frames dropped (negative) to correct drift.
    drift_frames: i64,
}

impl AudioGate {
    #[must_use]
    pub fn new() -> AudioGate {
        AudioGate::default()
    }

    /// Appends to `out` what to queue at the audio device for one tick's `samples`
    /// (interleaved stereo), given `queued_frames` stereo frames still waiting to be played.
    /// Call once per tick and queue `out` as it is; it may be empty.
    pub fn feed(&mut self, queued_frames: usize, samples: &[i16], out: &mut Vec<i16>) {
        let queued = i64::try_from(queued_frames).unwrap_or(i64::MAX);
        if self.fed_ticks == 0 {
            self.average_scaled = queued.saturating_mul(QUEUE_AVERAGE_TICKS);
        } else {
            self.average_scaled = self
                .average_scaled
                .saturating_add(queued - self.average_scaled / QUEUE_AVERAGE_TICKS);
        }
        self.fed_ticks += 1;
        let average = self.average_scaled / QUEUE_AVERAGE_TICKS;
        if self.fed_ticks == AUDIO_SETTLE_TICKS {
            let target =
                i64::try_from(AUDIO_TARGET_QUEUE_TICKS * AUDIO_FRAMES_PER_TICK).expect("fits");
            self.hold = Some(average.max(target));
        }

        let queued_ticks = queued_frames / AUDIO_FRAMES_PER_TICK;
        if queued_ticks > AUDIO_MAX_QUEUE_TICKS {
            self.draining = true;
        } else if queued_ticks <= AUDIO_TARGET_QUEUE_TICKS {
            self.draining = false;
        }
        if self.draining {
            self.discarded_ticks += 1;
            return;
        }
        if queued_frames == 0 {
            if self.started {
                self.underruns += 1;
            }
            self.started = true;
            let silence = (AUDIO_TARGET_QUEUE_TICKS - 1) * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS;
            out.extend(std::iter::repeat_n(0, silence));
            out.extend_from_slice(samples);
            return;
        }
        let last_frame = samples.len().saturating_sub(AUDIO_CHANNELS);
        match self.hold {
            Some(hold) if average < hold - DRIFT_BAND_FRAMES && !samples.is_empty() => {
                out.extend_from_slice(samples);
                out.extend_from_slice(&samples[last_frame..]);
                self.drift_frames += 1;
            }
            Some(hold) if average > hold + DRIFT_BAND_FRAMES && !samples.is_empty() => {
                out.extend_from_slice(&samples[..last_frame]);
                self.drift_frames -= 1;
            }
            _ => out.extend_from_slice(samples),
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
            drift_frames: self.drift_frames,
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

/// A picture of `size` at the largest whole scale that fits the output, centred, as `-nogl`
/// shows the original's software picture (spec M7); an output smaller than the picture gets
/// [`letterbox`]'s fit instead.
#[must_use]
pub fn whole_scale(output_width: u32, output_height: u32, size: (u32, u32)) -> Viewport {
    let scale = match size {
        (0, _) | (_, 0) => 0,
        (width, height) => (output_width / width).min(output_height / height),
    };
    if scale == 0 {
        return letterbox(output_width, output_height, size);
    }
    let (width, height) = (size.0 * scale, size.1 * scale);
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
    /// Frames repeated minus frames dropped to correct sound-card clock drift.
    pub drift_frames: i64,
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
    /// `t=60.000s ticks=4286 rate=71.43/s frames=3600 dropped_ticks=0 underruns=0 discarded_ticks=0 drift_frames=+3 queue_ms=39 present_avg_us=850 present_p99_us=1200`.
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
            "t={}.{:03}s ticks={} rate={}.{:02}/s frames={} dropped_ticks={} underruns={} discarded_ticks={} drift_frames={:+} queue_ms={} present_avg_us={} present_p99_us={}",
            millis / 1000,
            millis % 1000,
            self.ticks,
            rate_centi / 100,
            rate_centi % 100,
            self.present_micros.len(),
            dropped_ticks,
            audio.underruns,
            audio.discarded_ticks,
            audio.drift_frames,
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
    fn pacer_turns_wall_clock_time_into_14_ms_ticks_without_drift() {
        // Drift here would desynchronise lap times and music over a long race.
        let mut pacer = Pacer::new();
        let ticks: u32 = (0..36_000).map(|_| pacer.advance(FRAME_60HZ_NANOS)).sum();
        // 36 000 frames of 16 666 667 ns are 600.000012 s: 42 857 whole ticks of 14 ms.
        assert_eq!(ticks, 42_857);
        assert_eq!(pacer.dropped_ticks(), 0);
    }

    #[test]
    fn pacer_caps_catch_up_after_a_stall_and_counts_the_rest() {
        // After a one-second stall the game must not fast-forward 71 ticks in one frame.
        let mut pacer = Pacer::new();
        assert_eq!(pacer.advance(NANOS_PER_SECOND), MAX_CATCH_UP_TICKS);
        assert_eq!(pacer.dropped_ticks(), 71 - u64::from(MAX_CATCH_UP_TICKS));
    }

    #[test]
    fn pacer_survives_absurd_elapsed_times() {
        // A clock jump (suspend, debugger) must not panic on overflow.
        let mut pacer = Pacer::new();
        assert_eq!(pacer.advance(u64::MAX), MAX_CATCH_UP_TICKS);
        assert_eq!(pacer.advance(0), 0);
    }

    /// One tick of recognisable, non-silent samples.
    fn tick_samples() -> Vec<i16> {
        (1..=AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS)
            .map(|i| i16::try_from(i).expect("fits"))
            .collect()
    }

    #[test]
    fn audio_gate_primes_an_empty_queue_with_silence() {
        // Queueing one tick at a time into an empty device starves it on the first jitter;
        // starting two ticks ahead keeps the queue near the ~43 ms target.
        let mut gate = AudioGate::new();
        let tick = tick_samples();
        let silence = 2 * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS;
        let mut out = Vec::new();
        gate.feed(0, &tick, &mut out);
        assert!(out[..silence].iter().all(|&sample| sample == 0));
        assert_eq!(out[silence..], tick[..]);
        assert_eq!(gate.underruns(), 0, "starting is not an underrun");

        out.clear();
        gate.feed(AUDIO_FRAMES_PER_TICK, &tick, &mut out);
        assert_eq!(out, tick);

        out.clear();
        gate.feed(0, &tick, &mut out);
        assert_eq!(
            out.len(),
            silence + tick.len(),
            "after running dry, prime again"
        );
        assert_eq!(gate.underruns(), 1);
    }

    #[test]
    fn audio_gate_drains_to_target_then_queues_again() {
        // Latency must come back down to ~43 ms, not hover just under the maximum.
        let mut gate = AudioGate::new();
        let tick = tick_samples();
        let frames = AUDIO_FRAMES_PER_TICK;
        let mut fed = |queued: usize| {
            let mut out = Vec::new();
            gate.feed(queued, &tick, &mut out);
            out.len()
        };
        assert_eq!(fed(AUDIO_MAX_QUEUE_TICKS * frames), tick.len());
        assert_eq!(fed((AUDIO_MAX_QUEUE_TICKS + 1) * frames), 0);
        assert_eq!(fed(5 * frames), 0);
        assert_eq!(fed((AUDIO_TARGET_QUEUE_TICKS + 1) * frames), 0);
        assert_eq!(fed(AUDIO_TARGET_QUEUE_TICKS * frames), tick.len());
        assert_eq!(gate.discarded_ticks(), 3);
        assert_eq!(gate.report(7).queued_frames, 7);
    }

    /// Plays `minutes` of game audio into a simulated sound card whose clock runs `ppm` parts
    /// per million fast (positive) or slow (negative) against the game's clock and which takes
    /// `chunk` frames at a time. Returns the gate, how often the card found too few frames, and
    /// the longest queue seen.
    fn simulate_sound_card(
        ppm: u128,
        fast: bool,
        chunk: usize,
        minutes: u64,
    ) -> (AudioGate, u64, usize) {
        let mut gate = AudioGate::new();
        let tick = tick_samples();
        let mut out = Vec::new();
        let (mut queued, mut starved, mut longest) = (0usize, 0u64, 0usize);
        let card_rate = u128::from(AUDIO_SAMPLE_RATE)
            * if fast {
                1_000_000 + ppm
            } else {
                1_000_000 - ppm
            };
        let tick_time = |n: u64| n * TICK_NANOS;
        let pull_time = |n: u64| {
            let nanos = u128::from(n) * chunk as u128 * u128::from(NANOS_PER_SECOND) * 1_000_000
                / card_rate;
            u64::try_from(nanos).expect("fits")
        };
        let end = minutes * 60 * NANOS_PER_SECOND;
        let (mut ticks, mut pulls) = (0, 0);
        while tick_time(ticks).min(pull_time(pulls)) <= end {
            if pull_time(pulls) <= tick_time(ticks) {
                if queued >= chunk {
                    queued -= chunk;
                } else {
                    starved += u64::from(pulls > 0);
                    queued = 0;
                }
                pulls += 1;
            } else {
                out.clear();
                gate.feed(queued, &tick, &mut out);
                queued += out.len() / AUDIO_CHANNELS;
                longest = longest.max(queued);
                ticks += 1;
            }
        }
        (gate, starved, longest)
    }

    #[test]
    fn audio_queue_neither_runs_dry_nor_overflows_when_the_sound_card_clock_drifts() {
        // Sound cards run tens of ppm off the system clock (the M0 soak measured about 60 ppm).
        // Uncorrected, the queue slowly empties (a gap every few minutes) or fills up (a dropped
        // tick). Fifteen simulated minutes at 100 ppm either way must stay clean.
        for fast in [true, false] {
            for chunk in [512, 1024, 2048] {
                let (gate, starved, longest) = simulate_sound_card(100, fast, chunk, 15);
                let case = format!(
                    "100 ppm {}, {chunk}-frame pulls",
                    if fast { "fast" } else { "slow" }
                );
                assert_eq!(starved, 0, "{case}: the card ran out of audio");
                assert_eq!(gate.underruns(), 0, "{case}: underruns");
                assert_eq!(gate.discarded_ticks(), 0, "{case}: ticks dropped");
                assert!(
                    longest <= AUDIO_MAX_QUEUE_TICKS * AUDIO_FRAMES_PER_TICK,
                    "{case}: queue {longest}"
                );
            }
        }
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
    fn the_software_picture_is_shown_at_the_largest_whole_scale() {
        // A whole scale keeps the doubled pixels even; a fractional one would make some of
        // the picture's rows and columns wider than others.
        let shown = |width, height| whole_scale(width, height, (640, 480));
        assert_eq!(
            shown(1920, 1080),
            Viewport {
                x: 320,
                y: 60,
                width: 1280,
                height: 960
            }
        );
        assert_eq!(
            shown(640, 480),
            Viewport {
                x: 0,
                y: 0,
                width: 640,
                height: 480
            }
        );
        // A window too small for it is fitted as the scaled picture is.
        assert_eq!(shown(320, 240), letterbox(320, 240, (640, 480)));
        assert_eq!(shown(0, 0), letterbox(0, 0, (640, 480)));
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
            drift_frames: -4,
            queued_frames: 1890,
        };
        assert_eq!(
            stats.line(NANOS_PER_SECOND, 3, audio),
            "t=1.000s ticks=70 rate=70.00/s frames=100 dropped_ticks=3 underruns=1 discarded_ticks=2 drift_frames=-4 queue_ms=39 present_avg_us=50 present_p99_us=99"
        );
    }

    #[test]
    fn stats_line_before_anything_happened_is_all_zero() {
        let line = RunStats::new().line(0, 0, AudioReport::default());
        assert_eq!(
            line,
            "t=0.000s ticks=0 rate=0.00/s frames=0 dropped_ticks=0 underruns=0 discarded_ticks=0 drift_frames=+0 queue_ms=0 present_avg_us=0 present_p99_us=0"
        );
    }
}
