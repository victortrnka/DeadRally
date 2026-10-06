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
/// (0..=0x10000): `mask * (volume >> 8) >> 9`, as `musicSetmusicVolume` (0x43C280) sets it,
/// with the game's volume mask (0x456A34) at 255 unless the end screen lowers it. A volume
/// past full, from a damaged `dr.cfg`, counts as full.
fn music_master(volume: u32, mask: u32) -> i64 {
    (i64::from(mask) * i64::from(volume.min(effects::FULL) >> 8)) >> 9
}

/// The volume mask's normal value.
const FULL_MASK: u32 = 255;

/// The music volume the intro plays at, whatever `dr.cfg` says: the game's volume globals
/// start at 255 and take `dr.cfg`'s values only when the menu music starts.
pub(crate) const FULL_VOLUME: u32 = 0xFF00;

/// `dr.cfg`'s default music volume, 50 % (`defaultConfig`, 0x426700). The menus play at it
/// until M2's Configure menu can change it.
pub(crate) const DEFAULT_MUSIC_VOLUME: u32 = 0x8000;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug)]
pub(crate) struct Sound {
    music: Option<Music>,
    /// Voices of music that newer music replaced, fading out.
    fading: Vec<Voice>,
    effects: Option<Effects>,
    /// Voices of a bank that a newer bank replaced, fading out.
    fading_effects: Vec<Voice>,
    mix: Vec<i64>,
    /// The original's volume globals: the mask (0x456A34), the music's (0x456A30) and the
    /// effects' (0x456A2C) volumes as `dr.cfg` gives them; all full until the intro ends.
    mask: u32,
    music_volume: u32,
    effects_volume: u32,
}

impl Default for Sound {
    fn default() -> Sound {
        Sound {
            music: None,
            fading: Vec::new(),
            effects: None,
            fading_effects: Vec::new(),
            mix: Vec::new(),
            mask: FULL_MASK,
            music_volume: FULL_VOLUME,
            effects_volume: FULL_VOLUME,
        }
    }
}

impl Sound {
    /// Starts `module` at order `first_order` (counted as the game counts them) and the
    /// configured music `volume` (0..=0x10000), fading out any music playing.
    pub(crate) fn play_music(&mut self, module: &Module, first_order: usize, volume: u32) {
        if let Some(old) = self.music.take() {
            self.fading.extend(old.into_fading());
        }
        self.music_volume = volume;
        self.music = Some(Music::new(module, self.music_gain(), first_order));
    }

    fn music_gain(&self) -> i64 {
        UNITY * music_master(self.music_volume, self.mask) / 256
    }

    /// The effects stream's share: `mask * (volume >> 8) >> 8` of 255 (`musicSetVolume`,
    /// 0x43C250), 254 of 255 while the intro plays.
    fn effects_gain(&self) -> i64 {
        let volume = self.effects_volume.min(effects::FULL);
        UNITY * ((i64::from(self.mask) * i64::from(volume >> 8)) >> 8) / 255
    }

    /// The configured effects volume (0..=0x10000) for the effects stream.
    pub(crate) fn set_effects_volume(&mut self, volume: u32) {
        self.effects_volume = volume;
    }

    /// The volume mask (`setMusicVolume`, 0x43C2B0): 0..=255 over music and effects alike.
    pub(crate) fn set_mask(&mut self, mask: u32) {
        self.mask = mask;
        self.apply_music_gain();
    }

    /// The configured music volume (0..=0x10000) for the music playing, as Configure's popup
    /// sets it (`musicSetmusicVolume`, 0x43C280).
    pub(crate) fn set_music_volume(&mut self, volume: u32) {
        self.music_volume = volume;
        self.apply_music_gain();
    }

    /// The music's order (0 without music).
    pub(crate) fn music_order(&self) -> usize {
        self.music.as_ref().map_or(0, Music::order)
    }

    /// The music on to order `order` at its first row.
    pub(crate) fn set_music_order(&mut self, order: usize) {
        if let Some(music) = &mut self.music {
            music.set_order(order);
        }
    }

    fn apply_music_gain(&mut self) {
        let gain = self.music_gain();
        if let Some(music) = &mut self.music {
            music.set_gain(gain);
        }
    }

    /// Makes `bank` the source of [`Sound::trigger`]; the old bank's effects fade out.
    pub(crate) fn load_effects(&mut self, bank: &Bank) {
        if let Some(old) = self.effects.take() {
            self.fading_effects.extend(old.into_fading());
        }
        self.effects = Some(Effects::new(bank));
    }

    /// Plays effect `effect` of the loaded bank on `channel` (both 1-based) at full volume and
    /// normal pitch, as the intro does.
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8) {
        self.trigger_at(channel, effect, effects::FULL, effects::FULL);
    }

    /// Plays effect `effect` on `channel` at `volume` and `pitch` (16.16, 0x10000 full and
    /// normal), as `loadMenuSoundEffect` (0x43C380) does.
    pub(crate) fn trigger_at(&mut self, channel: usize, effect: u8, volume: u32, pitch: u32) {
        if let Some(effects) = &mut self.effects {
            effects.trigger(channel, effect, volume, pitch);
        }
    }

    /// The volume and pitch of what plays on `channel` changed in place (`sub_43C1B0`).
    pub(crate) fn set_channel(&mut self, channel: usize, volume: u32, pitch: u32) {
        if let Some(effects) = &mut self.effects {
            effects.set(channel, volume, pitch);
        }
    }

    /// Silences `channel` (1-based) with a short fade (`stopSoundChannel`, 0x43C3E0).
    pub(crate) fn stop_channel(&mut self, channel: usize) {
        if let Some(effects) = &mut self.effects {
            effects.stop(channel);
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
        let mut part = vec![0; self.mix.len()];
        if let Some(effects) = &mut self.effects {
            effects.mix_into(&mut part);
        }
        for voice in &mut self.fading_effects {
            voice.mix_into(&mut part);
        }
        self.fading_effects.retain(|voice| !voice.finished());
        let gain = self.effects_gain();
        for (sum, effect) in self.mix.iter_mut().zip(part) {
            *sum += (effect * gain) >> 16;
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
        assert_eq!(music_master(FULL_VOLUME, FULL_MASK), 127);
        assert_eq!(music_master(DEFAULT_MUSIC_VOLUME, FULL_MASK), 63);
        assert_eq!(music_master(0x1_0000, FULL_MASK), 127);
        assert_eq!(music_master(0, FULL_MASK), 0);
    }

    #[test]
    fn a_new_bank_lets_the_old_banks_effects_fade_out() {
        // The intro's effects stop as the menu's bank is loaded; cutting them at full level
        // would click.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        sound.stop();
        sound.load_effects(&bank());
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
    fn the_effects_volume_and_the_mask_scale_the_effects_stream() {
        // The menus play effects at dr.cfg's 75 %: the stream at 255 * 192 >> 8 = 191 of 255
        // instead of the intro's 254. The end screen's mask lowers everything.
        let level = |sound: &mut Sound| {
            sound.load_effects(&bank());
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(1000, &mut out);
            i64::from(out[2 * 999])
        };
        let full = level(&mut Sound::default());
        let mut menu = Sound::default();
        menu.set_effects_volume(0xC000);
        let at_75 = level(&mut menu);
        assert!(
            (at_75 * 254 - full * 191).abs() <= 254 * 2,
            "{at_75} vs {full}"
        );
        let mut quiet = Sound::default();
        quiet.set_mask(0);
        assert_eq!(level(&mut quiet), 0);
    }

    /// One endless loud note on the first channel.
    fn loud() -> Module {
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
        loud
    }

    #[test]
    fn the_mask_scales_the_music_while_it_plays() {
        // The end screen fades the music out through the mask, 255 down to 0.
        let loud = loud();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        sound.set_mask(0x80);
        out.clear();
        sound.render(2000, &mut out);
        // 255 * 255 >> 9 = 127 against 128 * 255 >> 9 = 63.
        assert!(
            (i64::from(out[2 * 1999]) * 127 - full * 63).abs() <= 127 * 2,
            "{} vs {full}",
            out[2 * 1999]
        );
    }

    #[test]
    fn the_music_volume_changes_the_music_while_it_plays() {
        // Configure's popup applies each step at once (`musicSetmusicVolume`): 50 % plays at
        // master volume 63, 100 % at 127.
        let mut sound = Sound::default();
        sound.play_music(&loud(), 0, DEFAULT_MUSIC_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let half = i64::from(out[2 * 1999]);
        sound.set_music_volume(0x1_0000);
        out.clear();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        assert!(
            (full * 63 - half * 127).abs() <= 127 * 2,
            "{half} vs {full}"
        );
    }

    #[test]
    fn a_volume_beyond_full_plays_at_full() {
        // dr.cfg can hold any number; the music and the effects must not blare at many times
        // full volume from the moment the menu music starts.
        let level = |volume: u32| {
            let mut sound = Sound::default();
            sound.play_music(&loud(), 0, volume);
            sound.load_effects(&bank());
            sound.set_effects_volume(volume);
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(2000, &mut out);
            out[2 * 1999]
        };
        assert_eq!(level(0xFFFF_FFFF), level(0x1_0000));
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
