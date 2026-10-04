//! Voices that play 16-bit samples at any pitch into a 48 kHz stereo sum (spec M1b §4.2):
//! linear interpolation and short volume ramps as in the original's minifmod mixer, in
//! integers so every OS produces the same samples.

use std::sync::Arc;

use crate::AUDIO_SAMPLE_RATE;

/// Unity gain for voice volumes: 1.0 in 16.16 fixed point.
pub(crate) const UNITY: i64 = 1 << 16;

/// Volume ramp length: minifmod's 128 samples at 44.1 kHz, at 48 kHz.
pub(crate) const RAMP_FRAMES: i64 = 128 * AUDIO_SAMPLE_RATE as i64 / 44_100;

/// How a sample repeats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Loop {
    None,
    /// Plays `start..end` again and again after reaching `end`.
    Forward {
        start: u32,
        end: u32,
    },
    /// Plays `start..end` forwards, then backwards, then forwards.
    PingPong {
        start: u32,
        end: u32,
    },
}

/// One sound being played.
#[derive(Clone, Debug)]
pub(crate) struct Voice {
    data: Arc<[i16]>,
    looping: Loop,
    /// Sample index in the high 32 bits, fraction in the low 32 bits.
    position: i64,
    /// Samples advanced per output frame, in the same 32.32 format.
    step: i64,
    backwards: bool,
    /// Target gains, [`UNITY`] = 1.0.
    left: i64,
    right: i64,
    /// Current gains while ramping towards the target.
    ramp_left: i64,
    ramp_right: i64,
    ramp_frames: i64,
    /// Set when the voice is fading out to make room; it stops at silence.
    releasing: bool,
}

impl Voice {
    /// A voice at the start of `data` (or at `offset` samples), silent until a volume is set.
    pub(crate) fn new(data: Arc<[i16]>, looping: Loop, offset: u32) -> Voice {
        let offset = i64::from(offset).min(data.len() as i64);
        Voice {
            data,
            looping,
            position: offset << 32,
            step: 0,
            backwards: false,
            left: 0,
            right: 0,
            ramp_left: 0,
            ramp_right: 0,
            ramp_frames: 0,
            releasing: false,
        }
    }

    /// Pitch as the sample rate at which the sample plays, in Hz.
    pub(crate) fn set_frequency(&mut self, hz: u32) {
        self.step = (i64::from(hz) << 32) / i64::from(AUDIO_SAMPLE_RATE);
    }

    /// Gains in 16.16 fixed point; the change ramps over [`RAMP_FRAMES`].
    pub(crate) fn set_volume(&mut self, left: i64, right: i64) {
        if (left, right) != (self.left, self.right) {
            self.left = left;
            self.right = right;
            self.ramp_frames = RAMP_FRAMES;
        }
    }

    /// Fades the voice to silence over [`RAMP_FRAMES`], then ends it.
    pub(crate) fn release(&mut self) {
        self.releasing = true;
        self.set_volume(0, 0);
    }

    pub(crate) fn finished(&self) -> bool {
        (self.releasing && self.ramp_frames == 0) || (self.position >> 32) >= self.data.len() as i64
    }

    /// The sample after `index`, following the loop, for interpolation.
    fn next_sample(&self, index: i64) -> i64 {
        let next = match self.looping {
            Loop::Forward { start, end } if index + 1 >= i64::from(end) => i64::from(start),
            Loop::PingPong { end, .. } if index + 1 >= i64::from(end) => index,
            _ => index + 1,
        };
        self.data
            .get(next as usize)
            .map_or(0, |&value| i64::from(value))
    }

    /// Adds `frames` stereo frames of this voice to `out` (interleaved left, right).
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        for frame in out.as_chunks_mut::<2>().0 {
            if self.finished() {
                return;
            }
            let index = self.position >> 32;
            let current = i64::from(self.data[index as usize]);
            let next = self.next_sample(index);
            let fraction = (self.position & 0xFFFF_FFFF) >> 16;
            let value = current + (((next - current) * fraction) >> 16);
            if self.ramp_frames > 0 {
                self.ramp_left += (self.left - self.ramp_left) / self.ramp_frames;
                self.ramp_right += (self.right - self.ramp_right) / self.ramp_frames;
                self.ramp_frames -= 1;
            } else {
                (self.ramp_left, self.ramp_right) = (self.left, self.right);
            }
            frame[0] += (value * self.ramp_left) >> 16;
            frame[1] += (value * self.ramp_right) >> 16;
            self.advance();
        }
    }

    fn advance(&mut self) {
        match self.looping {
            Loop::None => self.position += self.step,
            Loop::Forward { start, end } => {
                self.position += self.step;
                let (start, end) = (i64::from(start) << 32, i64::from(end) << 32);
                if end > start {
                    while self.position >= end {
                        self.position -= end - start;
                    }
                }
            }
            Loop::PingPong { start, end } => {
                // Turns at the loop's first and last sample, playing each of them once.
                let (first, last) = (i64::from(start) << 32, (i64::from(end) - 1) << 32);
                if last <= first {
                    self.position = first;
                    return;
                }
                if self.backwards {
                    self.position -= self.step;
                    if self.position < first {
                        self.position = 2 * first - self.position;
                        self.backwards = false;
                    }
                } else {
                    self.position += self.step;
                    if self.position > last {
                        self.position = 2 * last - self.position;
                        self.backwards = true;
                    }
                }
                self.position = self.position.clamp(first, last);
            }
        }
    }
}

/// Clips a mixed value to the 16-bit output, as the original's mixer does.
pub(crate) fn clip(value: i64) -> i16 {
    value.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(data: &[i16], looping: Loop) -> Voice {
        let mut voice = Voice::new(Arc::from(data), looping, 0);
        voice.set_frequency(AUDIO_SAMPLE_RATE);
        voice.left = UNITY;
        voice.right = UNITY / 2;
        voice.ramp_left = UNITY;
        voice.ramp_right = UNITY / 2;
        voice
    }

    fn render(voice: &mut Voice, frames: usize) -> Vec<i64> {
        let mut out = vec![0; 2 * frames];
        voice.mix_into(&mut out);
        out.chunks(2).map(|frame| frame[0]).collect()
    }

    #[test]
    fn a_sample_at_the_output_rate_plays_sample_for_sample() {
        let mut v = voice(&[100, 200, 300], Loop::None);
        let mut out = vec![0; 8];
        v.mix_into(&mut out);
        assert_eq!(
            out,
            [100, 50, 200, 100, 300, 150, 0, 0],
            "left full, right half, then silence"
        );
        assert!(v.finished());
    }

    #[test]
    fn half_speed_interpolates_between_neighbours() {
        // Without interpolation a slowed sample buzzes; minifmod interpolates linearly.
        let mut v = voice(&[0, 1000, 2000], Loop::None);
        v.set_frequency(AUDIO_SAMPLE_RATE / 2);
        assert_eq!(render(&mut v, 5), [0, 500, 1000, 1500, 2000]);
    }

    #[test]
    fn forward_loops_repeat_their_section() {
        let mut v = voice(&[1, 2, 3, 4], Loop::Forward { start: 1, end: 3 });
        assert_eq!(render(&mut v, 7), [1, 2, 3, 2, 3, 2, 3]);
        assert!(!v.finished());
    }

    #[test]
    fn ping_pong_loops_turn_at_both_ends() {
        let mut v = voice(&[1, 2, 3, 4, 5], Loop::PingPong { start: 1, end: 4 });
        assert_eq!(render(&mut v, 9), [1, 2, 3, 4, 3, 2, 3, 4, 3]);
    }

    #[test]
    fn volume_changes_ramp_instead_of_jumping() {
        // A jump in gain on a loud sample clicks; the original ramps over about 3 ms.
        let mut v = voice(&[10_000; 400], Loop::None);
        v.set_volume(0, 0);
        let out = render(&mut v, RAMP_FRAMES as usize + 2);
        assert!(out[0] < 10_000 && out[0] > 9_000, "{}", out[0]);
        assert!(out.windows(2).all(|pair| pair[1] <= pair[0]));
        assert_eq!(out[RAMP_FRAMES as usize], 0);
    }

    #[test]
    fn a_released_voice_fades_out_and_ends() {
        let mut v = voice(
            &[10_000; 1000],
            Loop::Forward {
                start: 0,
                end: 1000,
            },
        );
        v.release();
        render(&mut v, RAMP_FRAMES as usize);
        assert!(v.finished());
    }

    #[test]
    fn the_output_is_clipped_not_wrapped() {
        // Wrapping turns an overload into full-scale noise; clipping only flattens the peak.
        assert_eq!(clip(40_000), i16::MAX);
        assert_eq!(clip(-40_000), i16::MIN);
        assert_eq!(clip(1234), 1234);
    }
}
