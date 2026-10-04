//! Sound: the music and effect players and the mixer they share (spec M1b §4.2).

pub(crate) mod effects;
pub(crate) mod mixer;
pub(crate) mod tables;

use deadrally_gamedata::xm::Bank;

use self::effects::Effects;
use self::mixer::{UNITY, clip};
use crate::AUDIO_CHANNELS;

/// The effects' share: the volume of the stream minifmod mixes into while the intro plays,
/// 254 of 255 (`255 * 255 >> 8`).
pub(crate) const EFFECTS_GAIN: i64 = UNITY * 254 / 255;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug, Default)]
pub(crate) struct Sound {
    effects: Option<Effects>,
    mix: Vec<i64>,
}

impl Sound {
    /// Makes `bank` the source of [`Sound::trigger`], silencing the old bank's effects.
    pub(crate) fn load_effects(&mut self, bank: &Bank) {
        self.effects = Some(Effects::new(bank));
    }

    /// Plays effect `effect` of the loaded bank on `channel` (both 1-based).
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8) {
        if let Some(effects) = &mut self.effects {
            effects.trigger(channel, effect, effects::FULL, effects::FULL);
        }
    }

    /// Appends `frames` interleaved stereo frames to `out`.
    pub(crate) fn render(&mut self, frames: usize, out: &mut Vec<i16>) {
        self.mix.clear();
        self.mix.resize(frames * AUDIO_CHANNELS, 0);
        if let Some(effects) = &mut self.effects {
            let mut part = vec![0; self.mix.len()];
            effects.mix_into(&mut part);
            for (sum, effect) in self.mix.iter_mut().zip(part) {
                *sum += (effect * EFFECTS_GAIN) >> 16;
            }
        }
        out.extend(self.mix.iter().map(|&value| clip(value)));
    }
}

/// Renders effect `effect` (1-based) of `bank` at full volume and normal pitch, as the game
/// triggers it, for `frames` stereo frames.
#[must_use]
pub fn render_effect(bank: &Bank, effect: u8, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.load_effects(bank);
    sound.trigger(1, effect);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::xm::{Instrument, Looping};

    fn bank() -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![Some(Instrument {
                name: "Loud".into(),
                data: vec![i16::MAX; 20_000],
                looping: Looping::None,
                volume: 64,
                finetune: 0,
                relative_note: 0,
                panning: 255,
                fadeout: 0,
            })],
        }
    }

    #[test]
    fn nothing_loaded_is_silence_of_the_right_length() {
        // The frontend paces itself on the audio queue; every tick must deliver its samples.
        let mut out = Vec::new();
        Sound::default().render(672, &mut out);
        assert_eq!(out.len(), 672 * AUDIO_CHANNELS);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn overlapping_effects_clip_instead_of_wrapping() {
        // Four full-scale effects add up to twice the 16-bit range; wrapping would turn the
        // overload into noise, clipping only flattens it.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        for channel in 1..=4 {
            sound.trigger(channel, 1);
        }
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 900], i16::MAX, "left, panned hard left");
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }
}
