//! Sound effects: instruments of an XM bank triggered one at a time, as the original's
//! modified minifmod does (`FMUSIC_UpdateXMNote`, 0x43EC40; spec M1b §3.3).

use std::sync::Arc;

use deadrally_gamedata::xm::{Bank, Looping};

use super::mixer::{Loop, UNITY, Voice};
use super::tables::scale_by_exp2;

/// Effect channels: the banks have 8 or 16; the game uses 1-6 in the intro.
pub(crate) const CHANNELS: usize = 16;

/// Full volume and normal pitch in the game's calls (`0x10000`).
pub(crate) const FULL: u32 = 0x1_0000;

/// The original's lowest playback rate.
const MIN_HZ: u32 = 100;

#[derive(Debug)]
struct Sound {
    data: Arc<[i16]>,
    looping: Loop,
    relative_note: i32,
    finetune: i32,
    panning: i64,
}

#[derive(Debug)]
pub(crate) struct Effects {
    sounds: Vec<Option<Sound>>,
    channels: [Option<Voice>; CHANNELS],
    /// Voices cut off by a new effect on their channel, fading out.
    fading: Vec<Voice>,
}

impl Effects {
    pub(crate) fn new(bank: &Bank) -> Effects {
        let sounds = bank
            .instruments
            .iter()
            .map(|instrument| {
                instrument.as_ref().map(|instrument| Sound {
                    data: Arc::from(instrument.data.as_slice()),
                    looping: match instrument.looping {
                        Looping::None => Loop::None,
                        Looping::Forward { start, length } => Loop::Forward {
                            start,
                            end: start + length,
                        },
                        Looping::PingPong { start, length } => Loop::PingPong {
                            start,
                            end: start + length,
                        },
                    },
                    relative_note: i32::from(instrument.relative_note),
                    finetune: i32::from(instrument.finetune),
                    panning: i64::from(instrument.panning),
                })
            })
            .collect();
        Effects {
            sounds,
            channels: std::array::from_fn(|_| None),
            fading: Vec::new(),
        }
    }

    /// Plays effect `effect` (1-based) on `channel` (1-based) at `volume` and `pitch`, where
    /// [`FULL`] is full volume and normal pitch. A sound still playing there fades out.
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8, volume: u32, pitch: u32) {
        let slot = channel - 1;
        if let Some(mut old) = self.channels[slot].take() {
            old.release();
            self.fading.push(old);
        }
        let Some(Some(sound)) = usize::from(effect)
            .checked_sub(1)
            .and_then(|index| self.sounds.get(index))
        else {
            return;
        };
        // The volume byte (volume * 64 >> 16) + 16 sets the channel volume 0..=64; the final
        // volume is 64 * volume * 255 / (64 * 64) / 2 of 255 (fadeout and global volume full).
        let channel_volume = i64::from((volume.min(FULL) * 64) >> 16);
        let final_volume = channel_volume * 255 / 128;
        let (left, right) = pan(final_volume, sound.panning);
        let mut voice = Voice::new(Arc::clone(&sound.data), sound.looping, 0);
        voice.set_frequency(frequency(sound, pitch));
        voice.set_volume(left, right);
        self.channels[slot] = Some(voice);
    }

    /// Silences `channel` (1-based) with a short fade.
    pub(crate) fn stop(&mut self, channel: usize) {
        if let Some(mut voice) = self.channels[channel - 1].take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    pub(crate) fn stop_all(&mut self) {
        for channel in 1..=CHANNELS {
            self.stop(channel);
        }
    }

    /// Adds every playing effect to `out` (interleaved stereo).
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        for slot in &mut self.channels {
            if let Some(voice) = slot {
                voice.mix_into(out);
                if voice.finished() {
                    *slot = None;
                }
            }
        }
        for voice in &mut self.fading {
            voice.mix_into(out);
        }
        self.fading.retain(|voice| !voice.finished());
    }
}

/// minifmod's linear frequency: period `7680 - (6 * pitch / 65536 + 46 + relative) * 64 -
/// finetune / 2`, rounded down, then `8363 * 2^((4608 - period) / 768)` Hz, rounded down.
fn frequency(sound: &Sound, pitch: u32) -> u32 {
    // In 1/512 period units, so the fractions of the original's double arithmetic are exact.
    let period_512 = 7680 * 512
        - 3 * i64::from(pitch)
        - i64::from(46 + sound.relative_note) * 32768
        - i64::from(sound.finetune) * 256;
    let period = period_512.div_euclid(512);
    let steps = i32::try_from(4608 - period).unwrap_or(i32::MAX);
    scale_by_exp2(8363, steps).max(MIN_HZ)
}

/// minifmod's pan law on a 0..=255 volume: left gets `pan / 255`, right `(255 - pan) / 255`.
/// The gains are scaled so 255 becomes [`UNITY`].
fn pan(volume: i64, panning: i64) -> (i64, i64) {
    let left = volume * panning / 255;
    let right = volume * (255 - panning) / 255;
    (left * UNITY / 255, right * UNITY / 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::xm::Instrument;

    fn bank(relative_note: i8, panning: u8) -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![
                None,
                Some(Instrument {
                    name: "Boom".into(),
                    data: vec![1000; 48_000],
                    looping: Looping::None,
                    volume: 64,
                    finetune: 0,
                    relative_note,
                    panning,
                    fadeout: 0,
                }),
            ],
        }
    }

    #[test]
    fn normal_pitch_plays_note_52_plus_the_relative_note() {
        // Period 7680 - 52 * 64 = 4352 is 256 steps above XM's 8363 Hz: 2^(1/3) higher.
        let effects = Effects::new(&bank(0, 128));
        assert_eq!(frequency(effects.sounds[1].as_ref().unwrap(), FULL), 10_536);
        let lower = Effects::new(&bank(-12, 128));
        assert_eq!(
            frequency(lower.sounds[1].as_ref().unwrap(), FULL),
            5268,
            "an octave down"
        );
    }

    #[test]
    fn full_volume_is_127_of_255_split_by_the_pan_law() {
        // The original halves every effect (the 0.5 in its volume formula); playing them at
        // full scale would drown the music.
        assert_eq!(pan(127, 255), (127 * UNITY / 255, 0));
        assert_eq!(pan(127, 0), (0, 127 * UNITY / 255));
        let (left, right) = pan(127, 128);
        assert_eq!(left, 63 * UNITY / 255);
        assert_eq!(right, 63 * UNITY / 255);
    }

    #[test]
    fn a_triggered_effect_plays_and_an_empty_one_is_silent() {
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(3, 2, FULL, FULL);
        let mut out = vec![0i64; 400];
        effects.mix_into(&mut out);
        assert!(out[398] > 0, "{}", out[398]);
        effects.trigger(3, 1, FULL, FULL);
        let mut later = vec![0i64; 2 * 1000];
        effects.mix_into(&mut later);
        assert_eq!(
            later[1998], 0,
            "the old effect faded out, the empty one is silent"
        );
    }

    #[test]
    fn a_retriggered_channel_hands_its_old_sound_over_instead_of_cutting_it() {
        // minifmod moves the old voice to a spare channel and ramps it out while the new one
        // ramps in; a hard cut would click.
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(1, 2, FULL, FULL);
        let mut before = vec![0i64; 2 * 400];
        effects.mix_into(&mut before);
        let level = before[2 * 399];
        effects.trigger(1, 2, FULL, FULL);
        let mut after = vec![0i64; 2 * 20];
        effects.mix_into(&mut after);
        // Ten frames in, the old voice has lost what the new one has gained.
        assert!(
            (after[2 * 10] - level).abs() < level / 10,
            "{} after {level}",
            after[2 * 10]
        );
    }

    #[test]
    fn effect_numbers_outside_the_bank_are_silent() {
        // The HAF tables and the game's calls name effects by number; one the bank lacks must
        // play nothing rather than crash.
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(1, 0, FULL, FULL);
        effects.trigger(2, 3, FULL, FULL);
        effects.trigger(3, 255, FULL, FULL);
        let mut out = vec![0i64; 2 * 1000];
        effects.mix_into(&mut out);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn stopping_all_channels_silences_everything() {
        // The intro's end stops channels 1-6; a sound that kept playing would bleed into the logos.
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(1, 2, FULL, FULL);
        effects.trigger(6, 2, FULL, FULL);
        effects.stop_all();
        let mut out = vec![0i64; 2 * 1000];
        effects.mix_into(&mut out);
        assert_eq!(out[1998], 0);
    }
}
