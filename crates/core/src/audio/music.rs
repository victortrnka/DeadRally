//! The music player: Scream Tracker 3 modules with the 13 commands the game's music uses
//! (spec M1b §3.2, §4.2). Timing is counted in output samples, so the tempo never drifts.

use std::sync::Arc;

use deadrally_gamedata::s3m::{self, Cell, Module, NO_NOTE, NOTE_CUT, ORDER_SKIP};

use super::mixer::{Loop, Voice};
use super::tables::{S3M_PERIODS, vibrato};
use crate::AUDIO_SAMPLE_RATE;

/// Scream Tracker's clock: frequency = 14317056 / period.
const CLOCK: u32 = 14_317_056;
/// FMOD mixes at this rate and counts a tick as a whole number of its samples:
/// `44100 * 5 / (2 * tempo)`, rounded down. At tempo 141 that is 781 samples, not 781.9, so
/// the original plays such music 0.1 % fast; at tempo 125 it is exactly 882 (20 ms).
const FMOD_RATE: u32 = 44_100;
/// Period limits of Scream Tracker 3.
const MIN_PERIOD: i32 = 64;
const MAX_PERIOD: i32 = 32_767;

#[derive(Clone, Debug)]
struct SampleData {
    data: Arc<[i16]>,
    looping: Loop,
    c2spd: u32,
    volume: i32,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    enabled: bool,
    /// 0..=255.
    pan: i64,
    voice: Option<Voice>,
    /// Current sample (1-based instrument number), 0 = none yet.
    instrument: u8,
    period: i32,
    target_period: i32,
    /// 0..=64.
    volume: i32,
    /// Vibrato offset of this tick, in period units.
    period_delta: i32,
    command: u8,
    info: u8,
    // Effect memories.
    volume_slide: u8,
    porta: u8,
    tone_porta: u8,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_position: u8,
    offset: u8,
    retrigger: u8,
    retrigger_count: u8,
    /// A row's note held back by SDx until this tick.
    delayed: Option<(u8, Cell)>,
}

#[derive(Debug)]
pub(crate) struct Music {
    orders: Vec<u8>,
    patterns: Vec<s3m::Pattern>,
    samples: Vec<Option<SampleData>>,
    global_volume: i32,
    channels: Vec<Channel>,
    fading: Vec<Voice>,
    speed: u8,
    tempo: u8,
    order: usize,
    row: usize,
    tick: u8,
    /// Output frames left in the current tick, and the carried fraction of a frame.
    frames_left: u32,
    remainder: u32,
    /// Where the next row comes from after a B or C command.
    jump: Option<(usize, usize)>,
    /// Gain applied to every channel, in 16.16 (the original's master volume).
    gain: i64,
}

impl Music {
    /// A player at order `first_order` (counted as the game counts them, markers included),
    /// row 0 and tick 0. `gain` scales the whole module ([`UNITY`] = as loud as the module
    /// asks). As FMOD does (measured), the module's own master volume scales it by
    /// `master / 64` (the game's music has 48, 2.5 dB below full), and a stereo module plays
    /// at twice a mono module's level.
    pub(crate) fn new(module: &Module, gain: i64, first_order: usize) -> Music {
        let stereo = if module.stereo { 2 } else { 1 };
        let gain = gain * i64::from(module.master_volume) * stereo / 64;
        let samples = module
            .samples
            .iter()
            .map(|sample| {
                (!sample.data.is_empty()).then(|| SampleData {
                    data: Arc::from(sample.data.as_slice()),
                    looping: sample
                        .looped
                        .map_or(Loop::None, |(start, end)| Loop::Forward { start, end }),
                    c2spd: sample.c2spd,
                    volume: i32::from(sample.volume),
                })
            })
            .collect();
        let channels = module
            .channels
            .iter()
            .map(|channel| Channel {
                enabled: channel.enabled,
                // FMOD 3 puts a stereo module's channels fully on their side (measured on the
                // menu music); mono modules play in the centre.
                pan: match (module.stereo, channel.pan < 8) {
                    (false, _) => 128,
                    (true, true) => 0,
                    (true, false) => 255,
                },
                ..Channel::default()
            })
            .collect();
        let mut music = Music {
            orders: module.orders.clone(),
            patterns: module.patterns.clone(),
            samples,
            global_volume: i32::from(module.global_volume),
            channels,
            fading: Vec::new(),
            speed: module.initial_speed.max(1),
            tempo: module.initial_tempo.max(32),
            order: first_order,
            row: 0,
            tick: 0,
            frames_left: 0,
            remainder: 0,
            jump: None,
            gain,
        };
        music.skip_marker_orders();
        music
    }

    /// Fades every channel out, as `FMUSIC_StopSong`.
    pub(crate) fn stop(&mut self) {
        for channel in &mut self.channels {
            if let Some(mut voice) = channel.voice.take() {
                voice.release();
                self.fading.push(voice);
            }
        }
        self.orders.clear();
    }

    /// Fades every channel out and hands the fading voices over, for music being replaced.
    pub(crate) fn into_fading(mut self) -> Vec<Voice> {
        self.stop();
        self.fading
    }

    /// Adds the music to `out` (interleaved stereo), advancing ticks exactly on time.
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        let mut done = 0;
        let frames = out.len() / 2;
        while done < frames {
            if self.frames_left == 0 {
                if !self.orders.is_empty() {
                    self.process_tick();
                }
                self.start_tick_timer();
            }
            let count = (frames - done).min(self.frames_left as usize);
            let part = &mut out[2 * done..2 * (done + count)];
            for channel in &mut self.channels {
                if let Some(voice) = &mut channel.voice {
                    voice.mix_into(part);
                    if voice.finished() {
                        channel.voice = None;
                    }
                }
            }
            for voice in &mut self.fading {
                voice.mix_into(part);
            }
            self.fading.retain(|voice| !voice.finished());
            done += count;
            self.frames_left -= u32::try_from(count).expect("at most a tick");
        }
    }

    /// The next tick's length: FMOD's whole number of 44.1 kHz samples, converted to our rate
    /// exactly by carrying the remainder.
    fn start_tick_timer(&mut self) {
        let fmod_samples = FMOD_RATE * 5 / (2 * u32::from(self.tempo));
        let scaled = fmod_samples * AUDIO_SAMPLE_RATE + self.remainder;
        self.frames_left = scaled / FMOD_RATE;
        self.remainder = scaled % FMOD_RATE;
    }

    /// Moves past 254 and 255, which FMOD drops from the order list.
    fn skip_marker_orders(&mut self) {
        while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
            self.order += 1;
        }
        if self.order >= self.orders.len() {
            // The song loops from its start, as FMOD's looping music does.
            self.order = 0;
            while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
                self.order += 1;
            }
        }
    }

    fn process_tick(&mut self) {
        if self.tick == 0 {
            self.process_row();
        } else {
            for index in 0..self.channels.len() {
                self.channel_tick(index);
            }
        }
        for index in 0..self.channels.len() {
            self.update_voice(index);
        }
        self.tick += 1;
        if self.tick >= self.speed {
            self.tick = 0;
            self.next_row();
        }
    }

    fn next_row(&mut self) {
        if let Some((order, row)) = self.jump.take() {
            self.order = order;
            self.row = row;
            self.skip_marker_orders();
            return;
        }
        self.row += 1;
        if self.row >= s3m::ROWS {
            self.row = 0;
            self.order += 1;
            self.skip_marker_orders();
        }
    }

    fn process_row(&mut self) {
        let Some(&pattern) = self.orders.get(self.order) else {
            return;
        };
        let cells = self.patterns[usize::from(pattern)].rows[self.row];
        for (index, cell) in cells.iter().enumerate() {
            if !self.channels[index].enabled {
                continue;
            }
            let channel = &mut self.channels[index];
            channel.command = cell.command;
            channel.info = cell.info;
            channel.period_delta = 0;
            if cell.command == command('S') && cell.info >> 4 == 0xD && cell.info & 0xF > 0 {
                channel.delayed = Some((cell.info & 0xF, *cell));
                continue;
            }
            channel.delayed = None;
            self.start_cell(index, cell);
            self.row_effect(index, cell);
        }
    }

    /// The note, instrument and volume of a cell.
    fn start_cell(&mut self, index: usize, cell: &Cell) {
        let tone_porta = cell.command == command('G');
        if cell.instrument != 0 {
            let channel = &mut self.channels[index];
            channel.instrument = cell.instrument;
            if let Some(Some(sample)) = self.samples.get(usize::from(cell.instrument) - 1) {
                channel.volume = sample.volume;
            }
        }
        if cell.note == NOTE_CUT {
            self.cut(index);
        } else if cell.note != NO_NOTE {
            let instrument = self.channels[index].instrument;
            if let Some(Some(sample)) = usize::from(instrument)
                .checked_sub(1)
                .and_then(|i| self.samples.get(i))
            {
                let period = note_period(cell.note, sample.c2spd);
                let channel = &mut self.channels[index];
                if tone_porta && channel.voice.is_some() {
                    channel.target_period = period;
                } else {
                    let offset = if cell.command == command('O') {
                        if cell.info != 0 {
                            channel.offset = cell.info;
                        }
                        u32::from(channel.offset) * 256
                    } else {
                        0
                    };
                    let voice = Voice::new(Arc::clone(&sample.data), sample.looping, offset);
                    if let Some(mut old) = channel.voice.replace(voice) {
                        old.release();
                        self.fading.push(old);
                    }
                    let channel = &mut self.channels[index];
                    channel.period = period;
                    channel.target_period = period;
                    channel.vibrato_position = 0;
                    channel.retrigger_count = 0;
                }
            } else if instrument != 0 && !(tone_porta && self.channels[index].voice.is_some()) {
                // FMOD plays a note of an empty sample slot as silence: the note sounding stops.
                self.cut(index);
            }
        }
        if let Some(volume) = cell.volume {
            self.channels[index].volume = i32::from(volume);
        }
    }

    fn cut(&mut self, index: usize) {
        if let Some(mut voice) = self.channels[index].voice.take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    /// Commands that act on the row's first tick.
    fn row_effect(&mut self, index: usize, cell: &Cell) {
        let info = cell.info;
        let channel = &mut self.channels[index];
        match letter(cell.command) {
            'A' if info > 0 => self.speed = info,
            'T' if info >= 0x20 => self.tempo = info,
            'B' => {
                let row = self.jump.map_or(0, |(_, row)| row);
                self.jump = Some((usize::from(info), row));
            }
            'C' => {
                let row = usize::from((info >> 4) * 10 + (info & 0xF)).min(s3m::ROWS - 1);
                let order = self.jump.map_or(self.order + 1, |(order, _)| order);
                self.jump = Some((order, row));
            }
            'D' | 'K' => {
                if info != 0 {
                    channel.volume_slide = info;
                }
                let slide = channel.volume_slide;
                // Fine slides (DxF up, DFy down) act once, on this tick.
                if slide & 0x0F == 0x0F && slide >> 4 != 0 {
                    channel.volume = (channel.volume + i32::from(slide >> 4)).min(64);
                } else if slide >> 4 == 0x0F && slide & 0x0F != 0 {
                    channel.volume = (channel.volume - i32::from(slide & 0x0F)).max(0);
                }
            }
            'E' | 'F' => {
                if info != 0 {
                    channel.porta = info;
                }
                let porta = channel.porta;
                let sign = if letter(cell.command) == 'E' { 1 } else { -1 };
                // EFx fine (x * 4), EEx extra fine (x), on this tick only.
                match porta >> 4 {
                    0xF => channel.period += sign * 4 * i32::from(porta & 0xF),
                    0xE => channel.period += sign * i32::from(porta & 0xF),
                    _ => {}
                }
                channel.period = channel.period.clamp(MIN_PERIOD, MAX_PERIOD);
            }
            'G' => {
                if info != 0 {
                    channel.tone_porta = info;
                }
            }
            'H' => {
                if info >> 4 != 0 {
                    channel.vibrato_speed = info >> 4;
                }
                if info & 0xF != 0 {
                    channel.vibrato_depth = info & 0xF;
                }
            }
            'Q' if info != 0 => channel.retrigger = info,
            _ => {}
        }
    }

    /// Commands that act on every tick but the first.
    fn channel_tick(&mut self, index: usize) {
        if !self.channels[index].enabled {
            return;
        }
        if let Some((at, cell)) = self.channels[index].delayed {
            if self.tick == at {
                self.channels[index].delayed = None;
                self.start_cell(index, &cell);
            }
            return;
        }
        let channel = &mut self.channels[index];
        channel.period_delta = 0;
        match letter(channel.command) {
            'D' => volume_slide(channel),
            'K' => {
                volume_slide(channel);
                vibrato_tick(channel);
            }
            'E' | 'F' => {
                let porta = channel.porta;
                if porta >> 4 < 0xE {
                    let sign = if letter(channel.command) == 'E' {
                        1
                    } else {
                        -1
                    };
                    channel.period = (channel.period + sign * 4 * i32::from(porta))
                        .clamp(MIN_PERIOD, MAX_PERIOD);
                }
            }
            'G' => {
                let speed = 4 * i32::from(channel.tone_porta);
                if channel.period < channel.target_period {
                    channel.period = (channel.period + speed).min(channel.target_period);
                } else {
                    channel.period = (channel.period - speed).max(channel.target_period);
                }
            }
            'H' => vibrato_tick(channel),
            // Without an interval nothing repeats, and nothing is counted.
            'Q' if channel.retrigger & 0xF != 0 => {
                let interval = channel.retrigger & 0xF;
                channel.retrigger_count += 1;
                if channel.retrigger_count >= interval {
                    channel.retrigger_count = 0;
                    channel.volume = retrigger_volume(channel.volume, channel.retrigger >> 4);
                    let sample = usize::from(channel.instrument)
                        .checked_sub(1)
                        .and_then(|i| self.samples.get(i))
                        .and_then(Option::as_ref);
                    if let Some(sample) = sample {
                        let voice = Voice::new(Arc::clone(&sample.data), sample.looping, 0);
                        if let Some(mut old) = channel.voice.replace(voice) {
                            old.release();
                            self.fading.push(old);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Pushes the channel's pitch and volume into its voice.
    fn update_voice(&mut self, index: usize) {
        let global = i64::from(self.global_volume);
        let gain = self.gain;
        let channel = &mut self.channels[index];
        let Some(voice) = &mut channel.voice else {
            return;
        };
        let period = (channel.period + channel.period_delta).clamp(MIN_PERIOD, MAX_PERIOD);
        voice.set_frequency(CLOCK / u32::try_from(period).expect("positive"));
        let volume = i64::from(channel.volume) * global * gain / (64 * 64);
        let left = volume * (255 - channel.pan) / 255;
        let right = volume * channel.pan / 255;
        voice.set_volume(left, right);
    }

    #[cfg(test)]
    fn position(&self) -> (usize, usize, u8) {
        (self.order, self.row, self.tick)
    }
}

fn command(letter: char) -> u8 {
    letter as u8 - b'@'
}

fn letter(command: u8) -> char {
    if (1..=26).contains(&command) {
        char::from(b'@' + command)
    } else {
        ' '
    }
}

/// Scream Tracker's period of a note byte (octave in the high nibble) for a sample's C2SPD.
/// The octave shift comes last, so high notes keep their precision.
fn note_period(note: u8, c2spd: u32) -> i32 {
    let (octave, semitone) = (u32::from(note >> 4), usize::from(note & 0xF).min(11));
    let period =
        (8363 * 16 * u64::from(S3M_PERIODS[semitone]) / u64::from(c2spd.max(1))) >> octave.min(9);
    i32::try_from(period)
        .unwrap_or(MAX_PERIOD)
        .clamp(MIN_PERIOD, MAX_PERIOD)
}

fn volume_slide(channel: &mut Channel) {
    let slide = channel.volume_slide;
    let (up, down) = (slide >> 4, slide & 0x0F);
    if down == 0 && up != 0 {
        channel.volume = (channel.volume + i32::from(up)).min(64);
    } else if up == 0 && down != 0 {
        channel.volume = (channel.volume - i32::from(down)).max(0);
    }
}

fn vibrato_tick(channel: &mut Channel) {
    let wave = vibrato(channel.vibrato_position);
    channel.period_delta = (wave * i32::from(channel.vibrato_depth)) >> 5;
    channel.vibrato_position = (channel.vibrato_position + channel.vibrato_speed) & 63;
}

/// Qxy's volume change `x` applied to a volume 0..=64.
fn retrigger_volume(volume: i32, change: u8) -> i32 {
    let changed = match change {
        1..=5 => volume - (1 << (change - 1)),
        6 => volume * 2 / 3,
        7 => volume / 2,
        9..=13 => volume + (1 << (change - 9)),
        14 => volume * 3 / 2,
        15 => volume * 2,
        _ => volume,
    };
    changed.clamp(0, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{Channel as S3mChannel, Pattern, Sample};

    use crate::audio::mixer::UNITY;

    const C4: u8 = 0x40;

    fn module(rows: &[(usize, usize, Cell)], speed: u8, tempo: u8) -> Module {
        let mut pattern = Pattern {
            rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
        };
        for &(row, channel, cell) in rows {
            pattern.rows[row][channel] = cell;
        }
        let mut channels = [S3mChannel::default(); s3m::CHANNELS];
        channels[0] = S3mChannel {
            enabled: true,
            pan: 3,
        };
        channels[1] = S3mChannel {
            enabled: true,
            pan: 12,
        };
        Module {
            title: "Test".into(),
            orders: vec![0, 1],
            initial_speed: speed,
            initial_tempo: tempo,
            global_volume: 64,
            master_volume: 48,
            stereo: false,
            channels,
            samples: vec![Sample {
                name: "Tone".into(),
                c2spd: 8363,
                volume: 32,
                looped: Some((0, 100)),
                data: vec![10_000; 100],
            }],
            patterns: vec![pattern.clone(), pattern],
        }
    }

    fn cell(note: u8, instrument: u8, volume: Option<u8>, command: char, info: u8) -> Cell {
        Cell {
            note,
            instrument,
            volume,
            command: if command == ' ' {
                0
            } else {
                super::command(command)
            },
            info,
        }
    }

    fn play(music: &mut Music, frames: usize) -> Vec<i64> {
        let mut out = vec![0; 2 * frames];
        music.mix_into(&mut out);
        out
    }

    #[test]
    fn a_tick_lasts_fmods_whole_number_of_samples() {
        // At tempo 125 a tick is 882 samples at 44.1 kHz, exactly 20 ms: 960 of ours.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 960 * 6);
        assert_eq!(music.position(), (0, 1, 0));
        // At tempo 141 FMOD counts 781 samples (not 781.9) per tick, which is why the menu
        // music runs 0.1 % fast in the original: 147 ticks are exactly 124 960 of our samples.
        let mut faster = Music::new(&module(&[], 1, 141), UNITY, 0);
        play(&mut faster, 124_960);
        assert_eq!(
            faster.position(),
            (0, 19, 0),
            "147 rows: both patterns, then the song loops to row 19 of its start"
        );
    }

    #[test]
    fn middle_c_plays_at_the_samples_c2spd() {
        assert_eq!(note_period(C4, 8363), 1712);
        assert_eq!(CLOCK / 1712, 8362);
        assert_eq!(
            note_period(0x50, 8363),
            856,
            "an octave up halves the period"
        );
        assert_eq!(
            note_period(C4, 16_726),
            856,
            "a doubled C2SPD sounds an octave up"
        );
    }

    #[test]
    fn notes_play_with_the_samples_volume_and_the_volume_column_overrides_it() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(C4, 1, Some(64), ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        let out = play(&mut music, 960);
        // Mono: centred, 32 / 64 of full volume on each side, and the module's master volume
        // 48 / 64 on top.
        let expected = 10_000 * 32 / 64 * 127 / 255 * 48 / 64;
        assert!(
            (out[2 * 900] - expected).abs() <= 2,
            "{} vs {expected}",
            out[2 * 900]
        );
        let louder = play(&mut music, 960);
        assert!(
            (louder[2 * 900] - 2 * expected).abs() <= 4,
            "{}",
            louder[2 * 900]
        );
    }

    #[test]
    fn speed_tempo_jump_and_break_commands_steer_the_song() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(NO_NOTE, 0, None, 'A', 2)),
                    (0, 1, cell(NO_NOTE, 0, None, 'C', 0x10)),
                ],
                6,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 2);
        assert_eq!(
            music.position(),
            (1, 10, 0),
            "speed 2, then a break to row 10 of the next order"
        );
        let mut jumping = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'B', 0))], 1, 125),
            UNITY,
            0,
        );
        play(&mut jumping, 960);
        assert_eq!(jumping.position(), (0, 0, 0), "B00 loops the first order");
        // As in minifmod, a new tempo already sets the length of the tick that sets it.
        let mut tempo = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'T', 250))], 1, 125),
            UNITY,
            0,
        );
        play(&mut tempo, 480 * 3);
        assert_eq!(
            tempo.position(),
            (0, 3, 0),
            "tempo 250: ticks of 480 samples"
        );
    }

    #[test]
    fn stereo_modules_play_each_channel_fully_on_its_side_and_twice_as_loud() {
        // FMOD 3 pans a stereo module's channels hard left or right, and plays them at twice a
        // mono module's level: only that matches the stereo image and the loudness of the
        // original's menu music, recorded at two music volumes (docs/verification/m1b.md).
        let mut stereo = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        stereo.stereo = true;
        let mut music = Music::new(&stereo, UNITY, 0);
        let out = play(&mut music, 960);
        let full = 10_000 * 48 / 64 * 2;
        assert!((out[2 * 900] - full).abs() <= 4, "left {}", out[2 * 900]);
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn a_song_can_start_at_a_later_order() {
        // The game starts the menu music at order 45 (musicSetOrder), not at its beginning.
        let mut later = module(&[], 1, 125);
        later.orders = vec![0, s3m::ORDER_END, 1];
        let music = Music::new(&later, UNITY, 2);
        assert_eq!(music.position(), (2, 0, 0));
        let at_marker = Music::new(&later, UNITY, 1);
        assert_eq!(at_marker.position(), (2, 0, 0), "a marker is skipped");
    }

    #[test]
    fn playback_runs_on_through_section_markers_as_fmod_does() {
        // FMOD drops 254 and 255 from the order list (the game numbers orders for
        // FMUSIC_SetOrder without them), so a pattern before a 255 is followed by the next
        // section, not by the song's start.
        let mut base = module(&[], 1, 125);
        base.orders = vec![0, s3m::ORDER_END, s3m::ORDER_SKIP, 1];
        let mut music = Music::new(&base, UNITY, 0);
        play(&mut music, 960 * s3m::ROWS);
        assert_eq!(music.position(), (3, 0, 0));
    }

    /// A module whose one sample rises steadily (0, 8, 16, ...), so the output shows how far
    /// into the sample a voice is.
    fn rising(rows: &[(usize, usize, Cell)], speed: u8) -> Module {
        let mut rising = module(rows, speed, 125);
        rising.samples[0].data = (0..4000).map(|i| i16::try_from(i * 8).unwrap()).collect();
        rising.samples[0].looped = None;
        rising
    }

    #[test]
    fn the_sample_offset_starts_a_note_further_into_its_sample() {
        // O (545 times in TR5) starts notes part way into their sample; ignoring it would play
        // the sample's beginning instead.
        let mut plain = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 6),
            UNITY,
            0,
        );
        let mut offset = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), 'O', 8))], 6),
            UNITY,
            0,
        );
        let (plain, offset) = (play(&mut plain, 300), play(&mut offset, 300));
        // 8 * 256 = 2048 samples in: about 2100 instead of about 50 at frame 299.
        assert!(plain[2 * 299] > 0);
        assert!(
            offset[2 * 299] > 20 * plain[2 * 299],
            "{} vs {}",
            offset[2 * 299],
            plain[2 * 299]
        );
    }

    #[test]
    fn vibrato_with_volume_slide_does_both() {
        // K (277 times in TR1) keeps an earlier H's vibrato going while it slides the volume;
        // doing only one of the two freezes the note or its level.
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, Some(10), 'H', 0x48)),
                    (1, 0, cell(NO_NOTE, 0, None, 'K', 0x20)),
                ],
                4,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        let position = music.channels[0].vibrato_position;
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        assert_eq!(
            music.channels[0].vibrato_position,
            position + 3 * 4,
            "the vibrato goes on at speed 4"
        );
    }

    #[test]
    fn portamento_up_lowers_the_period_and_its_fine_form_acts_once() {
        // F (252 times in TR5) slides notes up; the wrong direction, or FFx on every tick,
        // would detune whole phrases.
        let mut up = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut up, 960 * 3);
        assert_eq!(up.channels[0].period, 1712 - 2 * 8, "2 ticks of 4 * 2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 0xF3))], 3, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 3);
        assert_eq!(
            fine.channels[0].period,
            1712 - 3 * 4,
            "FF3: once, on the first tick"
        );
    }

    #[test]
    fn retrigger_restarts_the_note_at_its_interval_and_changes_its_volume() {
        // Q (90 times in TR9) drums a note several times per row; without the restarts, or the
        // volume change, a roll becomes one long note.
        let mut music = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(20), 'Q', 0xA3))], 7),
            UNITY,
            0,
        );
        let out = play(&mut music, 960 * 7);
        // Restarted at tick 3: 200 frames later a voice plays again, near the sample's start.
        let (restarted, before) = (out[2 * (3 * 960 + 200)], out[2 * (3 * 960 - 1)]);
        assert!(
            restarted > 0 && restarted < before / 4,
            "{restarted} vs {before}"
        );
        assert_eq!(
            music.channels[0].volume, 24,
            "+2 at each restart, on ticks 3 and 6"
        );
    }

    #[test]
    fn a_retrigger_without_an_interval_never_overflows() {
        // Q00 before any Q with an interval repeats nothing; counting its ticks anyway overflowed
        // after 255 of them and crashed the game in the middle of the music.
        let mut long = module(
            &[
                (0, 0, cell(C4, 1, Some(64), 'Q', 0)),
                (1, 0, cell(NO_NOTE, 0, None, 'Q', 0)),
            ],
            200,
            125,
        );
        long.orders = vec![0];
        let mut music = Music::new(&long, UNITY, 0);
        play(&mut music, 960 * 400);
        assert_eq!(music.position(), (0, 2, 0));
    }

    #[test]
    fn a_song_of_markers_only_stays_silent() {
        // An order list without a pattern must not hang the player looking for one.
        let mut base = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        base.orders = vec![s3m::ORDER_SKIP, s3m::ORDER_END];
        let mut music = Music::new(&base, UNITY, 0);
        assert!(play(&mut music, 960 * 4).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn volume_slides_act_after_the_first_tick_and_fine_ones_on_it() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x20))], 4, 125),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x3F))], 4, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 4);
        assert_eq!(fine.channels[0].volume, 13, "one fine step of +3");
    }

    #[test]
    fn portamentos_move_the_period() {
        let mut down = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'E', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut down, 960 * 3);
        assert_eq!(down.channels[0].period, 1712 + 2 * 8);
        let mut towards = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(0x50, 1, None, 'G', 0xFF)),
                ],
                3,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut towards, 960 * 6);
        assert_eq!(
            towards.channels[0].period, 856,
            "the tone portamento stops at its target"
        );
    }

    #[test]
    fn vibrato_wobbles_around_the_note() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'H', 0x48))], 6, 125),
            UNITY,
            0,
        );
        let mut deltas = Vec::new();
        for _ in 0..6 {
            play(&mut music, 960);
            deltas.push(music.channels[0].period_delta);
        }
        assert_eq!(deltas[0], 0, "no vibrato on the first tick");
        assert!(
            deltas[1..].iter().all(|&delta| delta >= 0) && deltas[5] > 0,
            "{deltas:?}"
        );
        assert_eq!(
            music.channels[0].period, 1712,
            "the note itself does not move"
        );
    }

    #[test]
    fn a_note_of_an_empty_sample_slot_silences_the_channel() {
        // The menu music's order 47 starts with notes of a sample slot its author emptied;
        // FMOD plays them as silence. Ignoring them left the channel's looping note playing,
        // brought back up by their volume: a stray tone in the menu.
        let mut emptied = module(
            &[
                (0, 0, cell(C4, 1, Some(64), ' ', 0)),
                (1, 0, cell(C4, 2, Some(32), ' ', 0)),
            ],
            1,
            125,
        );
        emptied.samples.push(Sample::default());
        let mut music = Music::new(&emptied, UNITY, 0);
        play(&mut music, 960);
        let out = play(&mut music, 960);
        assert!(out[2 * 900..].iter().all(|&sample| sample == 0));
    }

    #[test]
    fn note_delay_and_cut_and_retrigger() {
        let mut delayed = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'S', 0xD2))], 4, 125),
            UNITY,
            0,
        );
        play(&mut delayed, 960 * 2);
        assert!(delayed.channels[0].voice.is_none(), "not before tick 2");
        play(&mut delayed, 960);
        assert!(delayed.channels[0].voice.is_some());
        let mut cut = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(NOTE_CUT, 0, None, ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut cut, 960 * 2);
        assert!(cut.channels[0].voice.is_none());
        assert_eq!(retrigger_volume(40, 0xF), 64);
        assert_eq!(retrigger_volume(40, 0x7), 20);
        assert_eq!(retrigger_volume(40, 0x3), 36);
    }

    #[test]
    fn the_same_module_always_renders_the_same_samples() {
        let rows = [
            (0, 0, cell(C4, 1, Some(40), 'H', 0x46)),
            (8, 1, cell(0x45, 1, None, 'Q', 0x93)),
        ];
        let render = || {
            let mut music = Music::new(&module(&rows, 3, 131), UNITY, 0);
            play(&mut music, 48_000)
        };
        assert_eq!(render(), render());
    }
}
