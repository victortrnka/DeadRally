//! Sound: the music and effect players and the mixer they share (spec M1b §4.2).

pub(crate) mod effects;
pub(crate) mod mixer;
pub(crate) mod music;
pub(crate) mod tables;

use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::xm::Bank;

use self::effects::Effects;
use self::mixer::{UNITY, Voice, clip};
use self::music::Music;
use crate::AUDIO_CHANNELS;

/// FMOD's master volume for music, 0..=256, at a music volume of the game's configuration
/// (0..=0x10000): `255 * (volume >> 8) >> 9`, as `musicSetmusicVolume` (0x43C280) sets it with
/// the game's volume mask at 255.
pub(crate) fn music_master(volume: u32) -> i64 {
    (255 * i64::from(volume >> 8)) >> 9
}

/// The music volume the intro plays at, whatever `dr.cfg` says: the game's volume globals
/// start at 255 and take `dr.cfg`'s values only when the menu music starts.
pub(crate) const FULL_VOLUME: u32 = 0xFF00;

/// `dr.cfg`'s default music volume, 50 % (`defaultConfig`, 0x426700). The menus play at it
/// until M2's Configure menu can change it.
pub(crate) const DEFAULT_MUSIC_VOLUME: u32 = 0x8000;

/// The effects' share: the volume of the stream minifmod mixes into while the intro plays,
/// 254 of 255 (`255 * 255 >> 8`).
pub(crate) const EFFECTS_GAIN: i64 = UNITY * 254 / 255;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug, Default)]
pub(crate) struct Sound {
    music: Option<Music>,
    /// Voices of music that newer music replaced, fading out.
    fading: Vec<Voice>,
    effects: Option<Effects>,
    mix: Vec<i64>,
}

impl Sound {
    /// Starts `module` at order `first_order` (counted as the game counts them) and the
    /// configured music `volume` (0..=0x10000), fading out any music playing.
    pub(crate) fn play_music(&mut self, module: &Module, first_order: usize, volume: u32) {
        if let Some(old) = self.music.take() {
            self.fading.extend(old.into_fading());
        }
        let gain = UNITY * music_master(volume) / 256;
        self.music = Some(Music::new(module, gain, first_order));
    }

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

    /// Stops the music and every effect, with a short fade so nothing clicks.
    pub(crate) fn stop(&mut self) {
        if let Some(music) = &mut self.music {
            music.stop();
        }
        if let Some(effects) = &mut self.effects {
            effects.stop_all();
        }
    }

    /// Appends `frames` interleaved stereo frames to `out`.
    pub(crate) fn render(&mut self, frames: usize, out: &mut Vec<i16>) {
        self.mix.clear();
        self.mix.resize(frames * AUDIO_CHANNELS, 0);
        if let Some(music) = &mut self.music {
            music.mix_into(&mut self.mix);
        }
        for voice in &mut self.fading {
            voice.mix_into(&mut self.mix);
        }
        self.fading.retain(|voice| !voice.finished());
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

/// Renders `frames` stereo frames of `module` from its first order at the default music
/// volume, mixed as the game plays music in its menus and races.
#[must_use]
pub fn render_music(module: &Module, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.play_music(module, 0, DEFAULT_MUSIC_VOLUME);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
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

    use deadrally_gamedata::s3m::{self, Cell, Channel, Pattern, Sample};
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

    #[test]
    fn new_music_fades_the_old_out_instead_of_cutting_it() {
        // The intro's music gives way to the menu music; a cut at full level would click.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut silent = loud.clone();
        silent.orders.clear();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        assert!(before > 0);
        sound.play_music(&silent, 0, FULL_VOLUME);
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_configured_music_volume_sets_fmods_master_volume() {
        // musicSetmusicVolume (0x43C280): 255 * (volume >> 8) >> 9.
        assert_eq!(music_master(FULL_VOLUME), 127);
        assert_eq!(music_master(DEFAULT_MUSIC_VOLUME), 63);
        assert_eq!(music_master(0x1_0000), 127);
        assert_eq!(music_master(0), 0);
    }

    #[test]
    fn stopping_fades_everything_out() {
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        sound.stop();
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 999], 0);
    }
}
